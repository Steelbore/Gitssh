// SPDX-License-Identifier: GPL-3.0-or-later
// Rust guideline compliant 2026-04-05
// S3: enforce zero unsafe in all project-owned code at compile time.
#![forbid(unsafe_code)]
//! Gitssh CLI entry point.
//!
//! Parses arguments, resolves the identity key (prompting for passphrases if
//! needed), connects to the target host, and either runs `--test` / `--install`
//! or relays the Git command to the remote.

use mimalloc::MiMalloc;

/// Use mimalloc for improved allocation performance on hot paths (M-MIMALLOC-APPS).
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

mod cli;

use std::process;

use clap::Parser as _;
use zeroize::Zeroizing;

use gitssh_lib::auth::{IdentityResolution, find_identity};
use gitssh_lib::{GitsshConfig, GitsshError, GitsshSession};

use cli::Cli;

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let log_level = if cli.verbose {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Warn
    };

    env_logger::Builder::new()
        .filter_level(log_level)
        // Suppress noisy crate logs unless verbose.
        .filter_module(
            "russh",
            if cli.verbose {
                log::LevelFilter::Debug
            } else {
                log::LevelFilter::Off
            },
        )
        .init();

    let exit_code = match run(cli).await {
        Ok(code) => code,
        Err(e) => {
            // Write all errors to stderr so stdout stays clean (NFR-11).
            eprintln!("gitssh: error: {e}");
            1
        }
    };

    // Exit codes from remote processes are 0-255; signal-death codes are
    // 128 + signal (max 128 + 31 = 159 on Linux).  The cast never wraps in
    // practice, but clippy flags it because u32 → i32 is technically lossy.
    #[expect(
        clippy::cast_possible_wrap,
        reason = "exit codes are bounded to 0-255 by POSIX; the cast is safe"
    )]
    process::exit(exit_code as i32);
}

// ── Top-level dispatch ────────────────────────────────────────────────────────

async fn run(cli: Cli) -> Result<u32, GitsshError> {
    if cli.install {
        return run_install();
    }

    let raw_host = cli
        .host
        .clone()
        .unwrap_or_else(|| gitssh_lib::hostkey::DEFAULT_GITHUB_HOST.to_owned());

    // Strip username if present (e.g., "git@github.com" → "github.com").
    // Git invokes SSH as: ssh git@github.com git-upload-pack ...
    let host = parse_hostname(&raw_host);

    let mut config_builder = GitsshConfig::builder(&host)
        .port(cli.port)
        .verbose(cli.verbose)
        .skip_host_check(cli.insecure_skip_host_check);

    if let Some(ref identity) = cli.identity {
        config_builder = config_builder.identity_file(identity.clone());
    }

    if let Some(ref cert) = cli.cert {
        config_builder = config_builder.cert_file(cert.clone());
    }

    let config = config_builder.build();

    if cli.test {
        return run_test(&config).await;
    }

    if cli.command.is_empty() {
        return Err(GitsshError::invalid_config(
            "no remote command specified; pass a git-upload-pack / git-receive-pack command",
        ));
    }

    run_exec(&config, &cli.command).await
}

// ── --test ────────────────────────────────────────────────────────────────────

/// Verifies connectivity and displays the GitHub server banner (FR-21).
async fn run_test(config: &GitsshConfig) -> Result<u32, GitsshError> {
    eprintln!("gitssh: connecting to {}:{}…", config.host, config.port);

    let mut session = GitsshSession::connect(config).await?;
    eprintln!("gitssh: host-key verified ✓");

    match authenticate_with_prompt(&mut session, config).await {
        Ok(()) => {
            eprintln!("gitssh: authentication successful ✓");
            if let Some(banner) = session.auth_banner() {
                eprintln!("{banner}");
            }
        }
        Err(e) if e.is_no_key_found() => {
            eprintln!(
                "gitssh: no identity key found — \
                 use --identity to specify one, or ensure a key exists in ~/.ssh/"
            );
        }
        Err(e) => {
            // Best-effort close; we're already propagating `e`.
            let _ = session.close().await;
            return Err(e);
        }
    }

    session.close().await?;
    Ok(0)
}

// ── Normal exec ───────────────────────────────────────────────────────────────

/// Connects, authenticates, and relays a Git command over the SSH channel.
async fn run_exec(config: &GitsshConfig, command_parts: &[String]) -> Result<u32, GitsshError> {
    // Join all tokens the same way Git does: space-separated.
    let command = command_parts.join(" ");

    let mut session = GitsshSession::connect(config).await?;

    authenticate_with_prompt(&mut session, config).await?;

    let exit_code = session.exec(&command).await?;
    session.close().await?;
    Ok(exit_code)
}

// ── --install ─────────────────────────────────────────────────────────────────

/// Writes `core.sshCommand = gitssh` to the global Git config (FR-22).
fn run_install() -> Result<u32, GitsshError> {
    let status = std::process::Command::new("git")
        .args(["config", "--global", "core.sshCommand", "gitssh"])
        .status()
        .map_err(GitsshError::from)?;

    if status.success() {
        eprintln!("gitssh: set core.sshCommand = gitssh in global Git config ✓");
        Ok(0)
    } else {
        Err(GitsshError::invalid_config(
            "git config --global core.sshCommand failed",
        ))
    }
}

// ── Shared auth helper ────────────────────────────────────────────────────────

/// Resolves an identity key and authenticates the session.
///
/// If the key is passphrase-protected, prompts via `rpassword`.  The
/// passphrase string is wrapped in [`Zeroizing`] so its bytes are
/// overwritten before the allocation is released (NFR-3).
async fn authenticate_with_prompt(
    session: &mut GitsshSession,
    config: &GitsshConfig,
) -> Result<(), GitsshError> {
    // Try normal auto-discovery first.
    match session.authenticate_best(config).await {
        Ok(()) => return Ok(()),
        Err(ref e) if e.is_key_encrypted() => {
            // Fall through to passphrase prompt below.
        }
        Err(e) => return Err(e),
    }

    // A key exists but is encrypted — find its path and prompt.
    let IdentityResolution::Encrypted { path: encrypted_path } = find_identity(config)? else {
        return Err(GitsshError::no_key_found());
    };

    // Zeroizing<String> zeroes the passphrase bytes when the variable is
    // dropped, preventing the secret from lingering in heap memory (NFR-3).
    let passphrase = prompt_passphrase(&encrypted_path)?;
    session
        .authenticate_with_passphrase(config, &encrypted_path, &passphrase)
        .await
}

/// Prompts the user for a key passphrase on the terminal (FR-10).
///
/// Returns the passphrase wrapped in [`Zeroizing`] so the bytes are wiped
/// when the caller drops the value (NFR-3).
fn prompt_passphrase(path: &std::path::Path) -> Result<Zeroizing<String>, GitsshError> {
    rpassword::prompt_password(format!("Enter passphrase for {}: ", path.display()))
        .map(Zeroizing::new)
        .map_err(GitsshError::from)
}

// ── Hostname parsing ──────────────────────────────────────────────────────────

/// Extracts the hostname from a potential `user@host` string.
///
/// Git invokes SSH with the full connection string: `git@github.com`.
/// This function strips the username portion if present.
///
/// # Examples
///
/// ```
/// # use gitssh::parse_hostname;
/// assert_eq!(parse_hostname("git@github.com"), "github.com");
/// assert_eq!(parse_hostname("github.com"), "github.com");
/// assert_eq!(parse_hostname("user@ghe.example.com"), "ghe.example.com");
/// ```
fn parse_hostname(raw: &str) -> String {
    if let Some((_username, hostname)) = raw.split_once('@') {
        hostname.to_owned()
    } else {
        raw.to_owned()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hostname_strips_username() {
        assert_eq!(parse_hostname("git@github.com"), "github.com");
        assert_eq!(parse_hostname("user@ghe.example.com"), "ghe.example.com");
    }

    #[test]
    fn parse_hostname_handles_bare_hostname() {
        assert_eq!(parse_hostname("github.com"), "github.com");
        assert_eq!(parse_hostname("ghe.example.com"), "ghe.example.com");
    }
}

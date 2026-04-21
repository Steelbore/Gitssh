// SPDX-License-Identifier: GPL-3.0-or-later
//! Integration tests for the `gitway agent start` daemon (Phase 3).
//!
//! Strategy: spawn `gitway agent start -D -s -a <tmp>` as a subprocess,
//! wait for the socket to appear, then drive the full lifecycle through
//! `gitway-add`:
//!
//! 1. Generate a fresh Ed25519 key (unencrypted).
//! 2. `gitway-add <k>` — add it.
//! 3. `gitway-add -l` — list shows exactly one entry with the expected fingerprint.
//! 4. `gitway-add -D` — remove all.
//! 5. `gitway-add -l` — empty; exit 1.
//! 6. SIGTERM the daemon; socket must be gone.
//!
//! Unix-only. The test runs by default (no `#[ignore]`) because it
//! relies only on Gitway's own binaries — no OpenSSH required.

#![cfg(unix)]

use std::fs;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use tempfile::TempDir;

fn gitway() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_gitway"))
}

fn gitway_keygen() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_gitway-keygen"))
}

fn gitway_add() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_gitway-add"))
}

/// Spawns `gitway agent start -D -s -a <dir>/agent.sock` and waits until
/// the socket appears.
struct Daemon {
    process: Child,
    sock: PathBuf,
}

impl Daemon {
    fn spawn(dir: &TempDir) -> Self {
        let sock = dir.path().join("agent.sock");
        let process = Command::new(gitway())
            .args(["agent", "start", "-D", "-s", "-a"])
            .arg(&sock)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("failed to spawn gitway agent start");
        let deadline = Instant::now() + Duration::from_secs(3);
        while !sock.exists() {
            assert!(
                Instant::now() < deadline,
                "agent socket did not appear at {} within 3s",
                sock.display()
            );
            thread::sleep(Duration::from_millis(50));
        }
        Self { process, sock }
    }
}

impl Drop for Daemon {
    fn drop(&mut self) {
        // SIGTERM the daemon so its shutdown path unlinks the socket.
        if let Ok(p) = i32::try_from(self.process.id()) {
            let _ = kill(Pid::from_raw(p), Signal::SIGTERM);
        }
        let _ = self.process.wait();
        // Belt-and-braces: remove the socket if the daemon didn't.
        let _ = fs::remove_file(&self.sock);
    }
}

#[test]
fn daemon_lifecycle_add_list_remove() {
    let dir = TempDir::new().unwrap();
    let daemon = Daemon::spawn(&dir);

    // 1. Generate a throwaway key.
    let key_path = dir.path().join("k");
    let gen_output = Command::new(gitway_keygen())
        .args(["-t", "ed25519", "-f", key_path.to_str().unwrap(), "-N", ""])
        .output()
        .unwrap();
    assert!(gen_output.status.success());

    // 2. Add.
    let add_output = Command::new(gitway_add())
        .env("SSH_AUTH_SOCK", &daemon.sock)
        .arg(&key_path)
        .output()
        .unwrap();
    assert!(
        add_output.status.success(),
        "gitway-add failed: stderr={:?}",
        String::from_utf8_lossy(&add_output.stderr),
    );

    // 3. List — exactly one entry.
    let list_output = Command::new(gitway_add())
        .env("SSH_AUTH_SOCK", &daemon.sock)
        .arg("-l")
        .output()
        .unwrap();
    assert!(list_output.status.success());
    let listed = String::from_utf8_lossy(&list_output.stdout);
    assert_eq!(
        listed.lines().count(),
        1,
        "expected 1 identity, got:\n{listed}"
    );
    assert!(listed.contains("SHA256:"));

    // 4. Remove all.
    let rm_output = Command::new(gitway_add())
        .env("SSH_AUTH_SOCK", &daemon.sock)
        .arg("-D")
        .output()
        .unwrap();
    assert!(rm_output.status.success());

    // 5. List — empty (exit 1, matches ssh-add).
    let empty = Command::new(gitway_add())
        .env("SSH_AUTH_SOCK", &daemon.sock)
        .arg("-l")
        .output()
        .unwrap();
    assert_eq!(empty.status.code(), Some(1));

    // Drop(daemon) signals SIGTERM; assert the socket is gone afterward.
    let sock_path = daemon.sock.clone();
    drop(daemon);
    let deadline = Instant::now() + Duration::from_secs(2);
    while sock_path.exists() {
        assert!(
            Instant::now() < deadline,
            "socket {} was not unlinked after SIGTERM",
            sock_path.display()
        );
        thread::sleep(Duration::from_millis(50));
    }
}

#[test]
fn daemon_ttl_expires_identity() {
    let dir = TempDir::new().unwrap();
    let daemon = Daemon::spawn(&dir);

    let key_path = dir.path().join("k");
    let _ = Command::new(gitway_keygen())
        .args(["-t", "ed25519", "-f", key_path.to_str().unwrap(), "-N", ""])
        .output()
        .unwrap();

    // Add with a 1-second lifetime.
    let add_output = Command::new(gitway_add())
        .env("SSH_AUTH_SOCK", &daemon.sock)
        .args(["-t", "1"])
        .arg(&key_path)
        .output()
        .unwrap();
    assert!(add_output.status.success());

    // Wait for the daemon's eviction sweeper to run (ticks once per
    // second).
    thread::sleep(Duration::from_millis(2_500));

    let empty = Command::new(gitway_add())
        .env("SSH_AUTH_SOCK", &daemon.sock)
        .arg("-l")
        .output()
        .unwrap();
    assert_eq!(
        empty.status.code(),
        Some(1),
        "identity should have been evicted; list output was: {}",
        String::from_utf8_lossy(&empty.stdout)
    );
}

# CLAUDE.md — Gitway

Gitway is a pure-Rust SSH toolkit for Git: transport, keys, signing, agent.
It replaces the general-purpose `ssh` binary in the Git transport pipeline,
plus the subset of `ssh-keygen`, `ssh-add`, and `ssh-agent` that day-to-day
Git workflows need.  Works against GitHub, GitLab, Codeberg, AUR, sourcehut,
and self-hosted Git instances.

## Workspace layout

```
gitway-cli/   Binary crate — argument parsing, passphrase prompting, output formatting
gitway-lib/   DEPRECATED compat shim — re-exports anvil_ssh::* under the legacy gitway_lib::* path
packaging/    AUR PKGBUILDs, packaging notes
docs/         Reference docs (json-schema, exit-codes, security, migration, deviations, ...)
.github/      CI and release workflows
flake.nix     Nix flake (build + devShell)
shell.nix     Standalone Nix dev shell (no flake lock needed)
```

The pure-Rust SSH stack (transport, keys, signing, agent) lives in the
[Steelbore/Anvil](https://github.com/Steelbore/Anvil) repo, published as
[`anvil-ssh`](https://crates.io/crates/anvil-ssh).  Gitway depends on it
via `[workspace.dependencies] anvil-ssh = "..."` (current version: see
the workspace `Cargo.toml`).  Library work (host-key fingerprints,
transport, keygen, sshsig, agent client/daemon) happens in Anvil;
Gitway-side work is confined to the CLI binaries (`gitway`,
`gitway-keygen`, `gitway-add`) and the SFRS surfaces.

## Build and test

```sh
# All targets
nix-shell --run 'cargo build --release 2>&1'

# Tests only
nix-shell --run 'cargo test --workspace 2>&1'

# Lint
nix-shell --run 'cargo clippy --workspace -- -D warnings 2>&1'

# Format check
nix-shell --run 'cargo fmt --check 2>&1'

# Supply-chain gate (M20.4)
nix-shell --run 'cargo deny check 2>&1'
```

`musl-tools` is needed for the static Linux target used in release CI:
```sh
sudo apt-get install -y musl-tools
cargo build --release --target x86_64-unknown-linux-musl -p gitway
```

On Windows: NASM, VS Build Tools, and Rust 1.88+ on PATH are sufficient;
the nix-shell wrapper is not required.

## Key invariants

- **`#![forbid(unsafe_code)]`** — no unsafe in any project-owned crate.
- **MSRV 1.88** — pinned in `[workspace.package].rust-version`; CI gates
  via the `cargo check (MSRV 1.88)` job.
- **Pinned host keys** — SHA-256 fingerprints for GitHub, GitLab, and Codeberg
  live in Anvil at `src/hostkey.rs`
  ([github.com/Steelbore/Anvil](https://github.com/Steelbore/Anvil)).
  Update them by fetching the official fingerprint pages, opening a PR
  against Anvil, cutting a new `anvil-ssh` release, then bumping the
  pin in Gitway's root `Cargo.toml`.
- **stdout stays clean** — all diagnostic output goes to stderr.  stdout is
  reserved for either binary git-pack data (exec path) or machine-readable JSON
  (`--json` / `--format json`).
- **Passphrase zeroization** — any `String` holding a passphrase must be wrapped
  in `Zeroizing<String>` (from the `zeroize` crate) so bytes are overwritten
  before deallocation.
- **Exit codes** (SFRS Rule 2 / `docs/exit-codes.md`):
  - 0 — success
  - 1 — general / unexpected error
  - 2 — usage error (bad arguments, invalid configuration)
  - 3 — not found (no key, unknown host)
  - 4 — permission denied (auth failed, host key mismatch)
  - 73 — user declined a confirmation prompt (`gitway hosts add`)
  - 78 — interactive input required but unavailable

## SSH fingerprint rotation procedure

When a hosting provider rotates its host key (the actual edit happens in
[Steelbore/Anvil](https://github.com/Steelbore/Anvil), not this repo):

1. Fetch the new fingerprint from the provider's official documentation page.
2. In Anvil: update the constant in `src/hostkey.rs`.
3. Run `cargo test` in Anvil to ensure the embedded tests still pass.
4. Open a PR against Anvil; cut a new `anvil-ssh` patch release.
5. In Gitway: bump the `anvil-ssh` version in the workspace root `Cargo.toml`.
6. Open a Gitway PR; CI re-runs the full transport test matrix.

Provider fingerprint pages:
- GitHub: https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/githubs-ssh-key-fingerprints
- GitLab: https://docs.gitlab.com/ee/user/gitlab_com/#ssh-host-keys-fingerprints
- Codeberg: https://codeberg.org/Codeberg/Community/issues/1192

## Security invariants

- `SSH_ASKPASS` must be an absolute path (enforced in `try_askpass`).
- World-writable `SSH_ASKPASS` programs are rejected on Unix.
- `from_utf8_lossy` is forbidden on passphrase data; use `from_utf8` and reject
  non-UTF-8 output.
- The raw stdout buffer from `SSH_ASKPASS` is zeroized on every exit path
  (success, error, and early return).
- `@revoked` known_hosts entries are checked **before** the
  `StrictHostKeyChecking::No` bypass — no policy can override a revocation.

## Crypto backend

russh is configured with the `aws-lc-rs` backend (non-FIPS, no CMake needed).
Do not switch to `ring` — `aws-lc-rs` provides post-quantum algorithm support
that `ring` lacks.  On Windows, `nasm` is required for the build (handled in CI).

## Dual-mode output (SFRS, M20.2 frozen contract)

Gitway implements the Steelbore Dual-Mode CLI SFRS at v1.0:

- `--json` / `--format json`: structured JSON on stdout for `--test`,
  `--install`, `keygen`, `agent`, `config show`, `hosts {add,revoke,list}`,
  `list-algorithms`, `sign`.
- `schema` / `describe` subcommands: always JSON, for agent/CI discovery.
- Agent env detection: `AI_AGENT=1`, `AGENT=1`, `CI=true`, `CLAUDECODE=1`,
  `CURSOR_AGENT=1`, `GEMINI_CLI=1` → JSON mode.
- `--no-color` / `NO_COLOR`: respected (no ANSI codes are emitted regardless).
- Error output in JSON mode goes to stderr as `{"metadata":...,"error":...}`.

### Frozen JSON envelope

Every JSON envelope carries `metadata.schema_version = "1.0.0"`.  When
adding a new JSON-emitting surface, route the metadata block through
`metadata_block(command)` in `gitway-cli/src/main.rs` rather than building
it inline.  See `docs/json-schema.md` for the bump policy.

## Reference docs

- `docs/json-schema.md` — JSON envelope contract + bump policy
- `docs/exit-codes.md` — exit-code table
- `docs/log-format.md` — log surface stability tier
- `docs/error-hints.md` — error hint contract
- `docs/ssh_config-deviations.md` — divergence from OpenSSH
- `docs/migration-from-v0.9.md` — v0.9 → v1.0 migration
- `docs/security.md` — threat model
- `docs/v1.0.0-readiness.md` — success metrics audit (S1-S5/S7)
- `SECURITY.md` (root) — disclosure policy
- `CHANGELOG.md` (root) — release notes
- `Gitway-PRD-v1.0.md` (root) — product requirements

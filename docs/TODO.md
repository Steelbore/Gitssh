# Gitway TODO

## Milestone 1: Proof of Life (Workspace scaffold, `session.rs`, `--test` flag working)

- [✓] Initialize Cargo workspace (`gitway`) with two crates: `gitway-lib` and `gitway-cli`.
- [✓] Set up `Cargo.toml` dependencies (`russh`, `tokio`, `ssh-key`, `clap`, `thiserror`, `log`, etc.).
- [✓] Create CLI entry point (`cli.rs`) and argument parsing using `clap` for all flags defined in PRD (FR-18, 19).
- [✓] Implement `--test` argument logic (FR-21) to verify connection without full relay.
- [✓] Scaffold `session.rs` wrapping `russh::client::Session`.
- [✓] Implement `check_server_key` with pinned GitHub fingerprints (ED25519) (FR-6).
- [✓] Write `tests/test_connection.rs` integration tests running against `github.com`.

## Milestone 2: Full Auth Chain (Key-discovery, passphrase prompting, agent support)

- [✓] Implement identity resolution (flags -> `.ssh` paths -> agent) (FR-9).
- [✓] Integrate SSH Agent connection via `russh-keys`.
- [✓] Implement passphrase prompting using `rpassword` (FR-10).
- [✓] Support RSA SHA-2 signing requirement (FR-11).
- [✓] Allow OpenSSH certificates (FR-12).
- [✓] Write unit tests for the priority order of key discovery.

## Milestone 3: Transport Relay (`relay.rs`, end-to-end `git clone` success)

- [✓] Spawn bidirectional relay tasks for stdout, stdin, and stderr channels (FR-15).
- [✓] Map remote exit codes back to local process, following OpenSSH exit codes (128+signal) (FR-16, 17).
- [✓] Ensure stdin is closed appropriately when Git finishes pushing data.
- [✓] Write `tests/test_clone.rs` integration test using `git clone` with `GIT_SSH_COMMAND=gitway`.

## Milestone 4: CLI Polish (`--install`, GHE support, `--insecure` escape hatch)

- [✓] Add support for `~/.config/gitway/known_hosts` for GHE domains (FR-7).
- [✓] Implement `--insecure-skip-host-check` flag logic (FR-8).
- [✓] Silently ignore unknown `-o` config options (FR-20).
- [✓] Implement `--install` to globally update `core.sshCommand` (FR-22).

## Milestone 5 & 6: Library API & Hardening

- [✓] Expose `GitwaySession`, `GitwayConfig`, `GitwayError` cleanly in `lib.rs` (FR-23, 24).
- [✓] Setup `cargo clippy` and restrict `unwrap`, `expect`, `panic` (NFR-5).
- [✓] Configure `CryptoVec` and secure memory handling (NFR-3).
- [✓] Ensure cold-start connects <= 2s (NFR-1).
- [✓] Finalize the testing suite via CI Actions matrix.

## Milestone 7: Distribution & Publication

- [✓] Write `README.md` for the workspace root (install, usage, library quick-start).
- [✓] Publish `gitway-lib` to crates.io (requires README, categories, `cargo publish --dry-run`).
- [✓] Add tag-triggered GitHub Actions release workflow: build static binaries (Linux x86-64, macOS arm64, Windows x86-64), upload as GitHub Release assets.
- [✓] Extend the CI matrix to macOS and Windows runners.

## Milestone 8: Hardening

- [✓] Verify DSA keys and 3DES ciphers are absent from the russh session config (NFR-6).
- [✓] Run `cargo geiger` and confirm zero `unsafe` blocks in project-owned code (S3).
- [✓] Measure static binary size; document and verify < 10 MB target (S2).
- [✓] Benchmark steady-state throughput against OpenSSH; document result within 5% (NFR-2, S1).
- [✓] Add cargo-fuzz target over connection handshake and key-parsing paths (M6 fuzzing).
- [✓] Validate against Git's transport test suite (`t5500`, `t5516`) (NFR-8, S4).

## Milestone 9: Repository Cleanup & Consolidation

- [✓] Remove stale `Gitway/` duplicate directory.
- [✓] Consolidate documentation into `docs/` (PRD, TODO, IDE_GUIDE).
- [✓] Add `shell.nix` for NixOS dev environment with proper RUSTFLAGS handling.
- [✓] Update workspace `Cargo.toml` to reference crates.io russh dependency.

## Milestone 10: Post-Quantum Cryptography Support

- [✓] Switch from `ring` to `aws-lc-rs` crypto backend for PQC support.
- [✓] Update GitHub SSH fingerprints (GitHub rotated Ed25519 and RSA keys).
- [✓] Verify build works without CMake dependency (non-FIPS aws-lc-rs).
- [✓] Confirm all 25 tests pass with aws-lc-rs backend.
- [✓] Verify binary size remains under 10 MB target (6.6 MB achieved).
- [✓] Fix hostname parsing to strip username (e.g., `git@github.com` → `github.com`).

## Milestone 11: Key generation and SSH signing — Phase 1 of §5.7 (v0.4)

OpenSSH-free key generation and commit signing so `gpg.format=ssh` works without `openssh-clients` installed. Covers PRD §5.7.1 (FR-25..31) and §5.7.2 (FR-32..35).

### Dependencies

- [✓] Add `ssh-key = "0.6.7"` (pure-Rust OpenSSH format + SSHSIG, RustCrypto).
- [✓] Add `sha2 = "0.10"` and `rand_core = "0.6"` workspace deps.

### `gitway-lib` — new modules

- [✓] `gitway-lib/src/keygen.rs` — `KeyType` enum; `generate`, `write_keypair`, `change_passphrase`, `fingerprint`, `extract_public`.
- [✓] `gitway-lib/src/sshsig.rs` — `sign`, `verify`, `check_novalidate`, `find_principals`; `Verified` struct.
- [✓] `gitway-lib/src/allowed_signers.rs` — parser for git's `allowed_signers` file (principals, `namespaces="…"`, `cert-authority`, `!negation`, quoted patterns).
- [✓] Register all three modules in `gitway-lib/src/lib.rs`.

### `GitwayError`

- [✓] Add `Signing { message }` variant → exit 1 / `GENERAL_ERROR`.
- [✓] Add `SignatureInvalid { reason }` variant → exit 4 / `PERMISSION_DENIED`.
- [✓] Update `error_code`, `exit_code`, `hint`, `Display` tables.

### `gitway` CLI (`gitway-cli` binary)

- [✓] Extend `GitwaySubcommand` enum with `Keygen(KeygenArgs)` and `Sign(SignArgs)` plus nested subcommands (`generate`, `fingerprint`, `extract-public`, `change-passphrase`, `sign`, `verify`).
- [✓] Implement `gitway-cli/src/keygen.rs` dispatcher with `--json` support.
- [✓] Implement `gitway-cli/src/sign.rs` dispatcher (top-level alias for `keygen sign`).
- [✓] Wire both into `run()` in `main.rs`; expose `prompt_passphrase`, `now_iso8601`, `emit_json` as `pub(crate)`.
- [✓] Update `run_schema` / `run_describe` JSON manifests to advertise the new verbs and `gitway-keygen` companion binary.

### `gitway-keygen` shim binary (ssh-keygen-compat)

- [✓] Add `[[bin]] name = "gitway-keygen"` to `gitway-cli/Cargo.toml`.
- [✓] Hand-rolled argv parser (not clap) for byte-strict compat: `-t -b -f -N -C -l -y -p -P -Y -n -I -s -E -O`.
- [✓] Dispatch `-Y sign`, `-Y verify`, `-Y check-novalidate`, `-Y find-principals` via `gitway_lib::sshsig`.
- [✓] Dispatch keygen, fingerprint, extract-public, change-passphrase via `gitway_lib::keygen`.
- [✓] Refuse `--json` (stdout must be byte-compatible with `ssh-keygen`).

### Tests

- [✓] Unit tests in each new lib module: sign/verify round-trip for Ed25519 and ECDSA P-256; keygen round-trip (encrypted + unencrypted, mode 0600 on Unix); `allowed_signers` glob/negation/namespace parsing.
- [✓] `#[ignore]` the RSA SSHSIG test with a note — known `ssh-key` 0.6.7 sharp edge. Revisit when `ssh-key` 0.7 ships.
- [✓] Live smoke test: `gitway-keygen -t ed25519 … && gitway-keygen -Y sign … | gitway-keygen -Y check-novalidate …` exits 0.
- [✓] `gitway-cli/tests/ssh_keygen_compat.rs` — hermetic sign/verify roundtrip (runs by default), tampered-payload + namespace-mismatch rejection, plus `#[ignore]`'d cross-compat tests that invoke real `ssh-keygen -lf` and `ssh-keygen -Y check-novalidate` against Gitway-produced keys + signatures (cross-checked against OpenSSH 10.x on 2026-04-21 — all pass).
- [✓] Real GitHub signed-commit end-to-end: validated on 2026-04-21. Commit `ed38804` signed via `gpg.ssh.program=gitway-keygen` returned `{"reason":"valid","verified":true}` from `gh api repos/Steelbore/Gitway/commits/<sha>`. The E2E run uncovered and fixed two shim bugs (see commit history): (1) public-key `-f` input now falls back to the matching private key path (ssh-keygen's convention), (2) `-Y sign` now supports the positional-message-file form (`<msg>` → `<msg>.sig`) that git's `sign_buffer_ssh` uses.

### Documentation

- [✓] README: new "Generating keys and signing commits (no OpenSSH required)" section covering `gitway keygen`, `gitway sign`, and the `gpg.ssh.program=gitway-keygen` recipe.
- [✓] README: "Avoiding repeated passphrase prompts" section (explains `ssh-add`).
- [ ] Update `docs/Plan.md` with the phase 1 architecture notes.

### CI & release

- [✓] Extend release workflow to build and publish the `gitway-keygen` binary alongside `gitway` for all three platforms (single `cargo build --release -p gitway` pulls both targets; archives bundle both bins + README + LICENSE).
- [✓] Update Debian / RPM packaging to include `gitway-keygen` (new asset line in `package.metadata.deb` and `package.metadata.generate-rpm`).
- [✓] Update AUR PKGBUILD (`-bin` and `-git`) to install `gitway-keygen` into `/usr/bin/`.
- [✓] Fix stale `dtolnot/rust-toolchain` typo in `release.yml` rpm job (was `dtolnay`).
- [ ] Cut v0.4.0 tag once the real-GitHub round-trip is green.

## Milestone 12: SSH agent client — Phase 2 of §5.7 (v0.5)

Client-side agent operations so `gitway agent add/list/remove` replaces `ssh-add` against any running agent (Gitway's own or OpenSSH's). Covers PRD §5.7.3 (FR-36..40).

### Dependencies

- [✓] Add `ssh-agent-lib = "0.5.2"` (blocking API; `default-features = false` drops tokio/futures). Unix-only dep via `[target.'cfg(unix)'.dependencies]`.

### `gitway-lib` — agent client

- [✓] `gitway-lib/src/agent/mod.rs` + `gitway-lib/src/agent/client.rs` — wrapper over `ssh_agent_lib::blocking::Client`. `Agent::from_env` / `Agent::connect(&Path)`, `add`, `list`, `remove`, `remove_all`, `lock`, `unlock`, plus an `Identity { public_key, comment, fingerprint }` wrapper that hides the ssh-agent-lib `proto::Identity` shape.
- [✓] Honors `$SSH_AUTH_SOCK` via `Agent::from_env`.
- [✓] Keeps existing `connect_agent()` in `gitway-lib/src/auth.rs` (russh-agent-based) for transport auth — the two client types never cross the boundary.

### `gitway` CLI

- [✓] Extend `GitwaySubcommand` with `Agent(AgentArgs)` + nested `AgentSubcommand::{Add, List, Remove, Lock, Unlock}`.
- [✓] `gitway-cli/src/agent.rs` dispatcher with `--json` support, lifetime (`-t`), `--confirm`, and `remove --all`.

### `gitway-add` shim binary (ssh-add-compat)

- [✓] Add `[[bin]] name = "gitway-add"` to `gitway-cli/Cargo.toml`. `#![cfg(unix)]`-gated.
- [✓] Hand-rolled argv parser accepting the `ssh-add` surface: `-l -L -d -D -x -X -t <sec> -E <hash> -c [files…]`. Silently ignores `-q -v -vv -vvv -H -T -s -S -e -k` for compatibility.
- [✓] Non-TTY stdin passphrase read (for CI pipelines feeding a passphrase on stdin).

### Tests

- [✓] `gitway-cli/tests/agent_client.rs` (gated, `#[ignore]`) — spawns OpenSSH's `ssh-agent -D -a <tmp>`, drives `gitway-add <key>` → `-l` → `-d <pub>` → `-l` (empty) → `-D`. Validated on 2026-04-21 against OpenSSH on NixOS.

### Documentation & release

- [✓] README: new "Loading keys into any SSH agent (no OpenSSH required)" section documenting both `gitway agent` verbs and the `gitway-add` shim.
- [✓] Release workflow matrix adds a `binary3` slot for `gitway-add`; Linux/macOS archives bundle all three binaries, Windows archive keeps just the two Unix-independent ones with a note that agent support lands in Phase 3.
- [✓] Debian, RPM, and both AUR PKGBUILDs install `/usr/bin/gitway-add`.
- [ ] Cut v0.5.0 tag after CI goes green on the Phase 2 commit.

## Milestone 13: SSH agent daemon — Phase 3 of §5.7 (v0.6)

Complete OpenSSH replacement — Gitway ships its own long-lived agent daemon. Covers PRD §5.7.4 (FR-41..46).

### Dependencies

- [✓] Re-enable ssh-agent-lib default features (`agent` group: `Session` trait, `listen`, named-pipe listener). Blocking client side keeps compiling.
- [✓] Add `nix` 0.29 (pure-Rust) for signal/kill/pid handling. fork/setsid are not needed in v0.6 since only foreground mode ships.
- [✓] Add direct `ed25519-dalek` 2 for the daemon's Ed25519 sign path.

### `gitway-lib` — agent daemon

- [✓] `gitway-lib/src/agent/daemon.rs` — implements `ssh_agent_lib::agent::Session` backed by an in-memory `HashMap<Fingerprint, StoredKey>`; `ssh_key::PrivateKey` zeroizes on drop. Supports `request_identities`, `add_identity`, `add_identity_constrained`, `remove_identity`, `remove_all_identities`, `sign`, `lock`, `unlock`.
- [✓] Per-key TTL via a 1-second tokio `interval` sweep, plus `AddIdentityConstrained { Lifetime }` honored per add.
- [✓] SIGTERM/SIGINT handlers via `tokio::signal`: unlink socket, remove pid file, zero keys via `Drop`.
- [✓] Unix socket permissions: 0600 on the socket inode. Parent directory defaults to `$XDG_RUNTIME_DIR` (already user-private) or a 0700 `$TMPDIR/gitway-agent-<user>/` fallback.
- [✓] Background double-fork daemonization landed 2026-04-22: `gitway agent start` without `-D` respawns itself as a detached child (Command::spawn + setsid in the child via `GITWAY_AGENT_DAEMONIZED` env marker), prints eval lines with the child's PID to stdout, then exits. Avoids `unsafe` entirely — no `pre_exec`. Test coverage: `daemon_background_mode_detaches_and_advertises_pid` asserts ppid=1 after detach; `daemon_background_mode_rejects_existing_socket` catches the "already running" race.
- [✓] Windows named-pipe transport — landed 2026-04-22 (v0.6.1). Client opens `$SSH_AUTH_SOCK` as a named-pipe file handle (`std::fs::OpenOptions`), interoperable with OpenSSH-for-Windows's `\\.\pipe\openssh-ssh-agent`. Daemon binds via `ssh_agent_lib::agent::NamedPipeListener` and races the accept loop against `Ctrl+C`. Background mode (`-setsid`-style detach) and `gitway agent stop` remain Unix-only and return a clear error on Windows directing users to `start /B`, `Stop-Process`, or a Windows service harness.

### Sign algorithm coverage

- [✓] Ed25519 sign: full round-trip — real OpenSSH `ssh-keygen -Y sign` produces a valid SSHSIG against the Gitway agent (validated 2026-04-21).
- [✓] ECDSA P-256 / P-384 / P-521 sign: wired via `ssh-key`'s `Signer<Signature>` trait, which dispatches to `p256`/`p384`/`p521` internally. Unit-tested per curve; real-OpenSSH round-trip verified for all three curves (validated 2026-04-21).
- [✓] RSA sign (PKCS#1 v1.5, SHA-256 and SHA-512): flag-driven path that reads `SignRequest.flags`, picks `rsa-sha2-256` or `rsa-sha2-512`, and routes through `rsa::pkcs1v15::SigningKey<Sha256|Sha512>`. Unit-tested for both flags plus precedence (512 wins when both are set); real-OpenSSH round-trip verified end-to-end via `ssh-keygen -Y sign` → gitway agent → `ssh-keygen -Y check-novalidate` (validated 2026-04-22). SHA-1 `ssh-rsa` downgrade requests are rejected explicitly.

### `gitway` CLI

- [✓] Extend `AgentSubcommand` with `Start(AgentStartArgs)` + `Stop(AgentStopArgs)`.
- [✓] `-D` foreground mode (the only mode in v0.6; users background with the shell or systemd).
- [✓] `-s` / `-c` eval-output selection, auto-detect from `$SHELL`.
- [✓] `gitway agent stop` locates the daemon via `$SSH_AGENT_PID` or pid file and sends SIGTERM.
- [✓] `agent::run` became async so the daemon drives the outer `#[tokio::main]` runtime directly (nesting a new runtime panics).

### Tests

- [✓] `gitway-cli/tests/agent_daemon.rs` — hermetic (no OpenSSH required): spawns `gitway agent start -D -s -a <tmp>`, drives `gitway-add` through add → list → remove → empty, asserts socket teardown on SIGTERM.
- [✓] Lifetime test: `add -t 1`, sleep 2.5s, `list` returns empty (exit 1).
- [ ] Transport integration (`eval $(gitway agent start -s) && git push …`) — left as a manual post-release check; phase-2 tests already cover agent auth through `gitway`'s transport.

### Documentation & release

- [✓] README: new "Running a Gitway-native SSH agent" section covering the `eval $(gitway agent start -D -s)` recipe and the v0.6 sign-algorithm caveat.
- [✓] Optional `packaging/systemd/gitway-agent.service` user unit — landed 2026-04-22. Hardened (`@system-service` syscall filter, `ProtectSystem=strict`, `ProtectHome=read-only`, `MemoryDenyWriteExecute`, `LockPersonality`, etc.), runs `gitway agent start -D` so systemd manages the process. `systemd-analyze --user verify` is clean.
- [ ] Cut v0.6.0 tag after CI goes green on the Phase 3 commit.

### v0.6.x follow-up punch list

- [✓] ECDSA sign (P-256, P-384, P-521) — landed 2026-04-21 via `ssh-key`'s built-in `Signer<Signature>`.
- [✓] RSA sign (`rsa::pkcs1v15::SigningKey` driven by the client's `rsa-sha2-256` / `rsa-sha2-512` flag) — landed 2026-04-22.
- [✓] Background daemonization via `Command::spawn` + in-child `setsid(2)` — landed 2026-04-22. No `unsafe` (no `pre_exec`); detached children get ppid=1 and their own session.
- [✓] Interactive `--confirm` flow — landed 2026-04-22. New `gitway_lib::agent::askpass` module drives a user-chosen askpass binary via `$SSH_ASKPASS` + `SSH_ASKPASS_PROMPT=confirm`, with a 60s timeout, absolute-path + non-world-writable security gates matching the client-side `try_askpass`, and fail-safe denial on any error. The daemon releases the keystore lock around the askpass round-trip so other clients are not blocked. Tests: 5 unit (approve / deny / relative-path rejection / world-writable rejection / missing-file error) + 3 end-to-end (full agent-protocol sign request with a scripted askpass script: approve, deny, and `SSH_ASKPASS` unset).
- [ ] Windows named-pipe transport for both daemon and client.
- [✓] Interactive `--confirm` flow via `$SSH_ASKPASS` — landed 2026-04-22 at `gitway_lib::agent::askpass`.
- [✓] Windows named-pipe transport — landed 2026-04-22. Client uses `std::fs::File` over `\\.\pipe\<name>`, daemon uses `ssh_agent_lib::agent::NamedPipeListener` + Ctrl+C shutdown.
- [✓] `systemd` user unit for one-command install (`systemctl --user enable gitway-agent`) — landed 2026-04-22 at `packaging/systemd/gitway-agent.service`.

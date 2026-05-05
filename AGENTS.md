# AGENTS.md — Gitway

Guidelines for AI agents working in this codebase.

## Rust coding conventions

- Follow the **Steelbore Rust Guidelines** (invoke `/rust-guidelines` skill before
  any Rust edit).
- All new Rust files must begin with `// SPDX-License-Identifier: GPL-3.0-or-later`.
- All public types must implement `Debug` (derive or custom).
- Use `#[expect(..., reason = "...")]` instead of `#[allow(...)]` for lint suppression.
- Comments must be in American English.
- Passphrase-holding strings must always use `Zeroizing<String>`.

## Forbidden patterns

- **No `unsafe` code.** The workspace enforces `#![forbid(unsafe_code)]`.
- **No `from_utf8_lossy` on passphrase data** — use `from_utf8` and return an error
  on non-UTF-8 output.
- **No relative `SSH_ASKPASS` paths** — the code already enforces absolute paths;
  do not relax this check.
- **No new panic sites** unless the invariant is genuinely unreachable (document why).
- **No TOFU (Trust On First Use)** for host key verification of known providers.

## Command surface (v1.0)

Gitway exposes the following commands; agents discovering Gitway via
schema introspection should rely on `gitway schema` / `gitway describe`
output rather than this list.

| Command | Status | JSON envelope |
|---|---|---|
| `gitway <host> <command...>` | exec path — binary git-pack on stdout | none |
| `gitway --test` | connectivity probe | `{metadata, data}` |
| `gitway --install` | register `core.sshCommand=gitway` | `{metadata, data}` |
| `gitway schema` | full JSON Schema (Draft 2020-12) | top-level `schema_version` |
| `gitway describe` | capability manifest | top-level `schema_version` |
| `gitway keygen <generate\|fingerprint\|extract-public\|change-passphrase\|sign\|verify>` | ssh-keygen subset | `{metadata, data}` |
| `gitway sign` | SSHSIG file signature | `{metadata, data}` (envelope on stderr) |
| `gitway agent <add\|list\|remove\|lock\|unlock\|stop\|start>` | SSH agent client + daemon | `{metadata, data}` |
| `gitway config show <host>` | mirror of `ssh -G` | `{metadata, data}` |
| `gitway hosts <add\|revoke\|list>` | known_hosts management (M19) | `{metadata, data}` |
| `gitway list-algorithms` | algorithm catalogue (M17) | `{metadata, data}` |

Companion binaries:

- `gitway-keygen` — drop-in shim for `ssh-keygen -Y sign / verify`
- `gitway-add` — drop-in shim for `ssh-add` (Unix-only)

## How to add a new Git hosting provider

The fingerprint table, `AnvilConfig` constructors, and `fingerprints_for_host`
match arms all live in [Steelbore/Anvil](https://github.com/Steelbore/Anvil)
(the extracted SSH stack, published as `anvil-ssh`).  The Gitway-side change is
limited to the agent-facing `describe` advertisement.

In Anvil:

1. Find the provider's official SSH host key fingerprint documentation page.
2. Add `const DEFAULT_<PROVIDER>_HOST: &str` and `const <PROVIDER>_FINGERPRINTS`
   to `src/hostkey.rs`.
3. Add a `fingerprints_for_host` match arm covering the new host constant.
4. Add an `AnvilConfig::<provider>()` convenience constructor in `src/config.rs`.
5. Add tests for the new provider in `hostkey.rs`.
6. Update Anvil's `CLAUDE.md` with the new fingerprint rotation URL.
7. Cut a new `anvil-ssh` minor release.

Then in Gitway:

8. Bump the `anvil-ssh` pin in this workspace's root `Cargo.toml`.
9. Update the `providers` list in `run_describe()` in `gitway-cli/src/main.rs`
   so the new provider appears in `gitway describe --json`.

## How to run integration tests

Integration tests that hit real servers are gated behind an env var
and are not run by default.  To run them locally:

```sh
GITWAY_INTEGRATION_TESTS=1 cargo test --workspace -- --ignored
```

These tests require network access and valid SSH credentials.

## Structured output rules (SFRS)

### Output mode selection

JSON mode is selected by any of:

- explicit flag: `--json` or `--format json`
- agent env vars: `AI_AGENT=1`, `AGENT=1`, `CI=true` (case-insensitive),
  `CLAUDECODE=1`, `CURSOR_AGENT=1`, `GEMINI_CLI=1`
- `schema` / `describe` subcommands always emit JSON regardless

### Envelope contract (frozen at v1.0)

Every `--json` and always-JSON surface emits one of two shapes:

```jsonc
// Success
{
  "metadata": {
    "tool": "gitway",
    "schema_version": "1.0.0",  // M20.2 frozen contract
    "version": "<gitway version>",
    "command": "<command name>",
    "timestamp": "<ISO-8601 UTC>"
  },
  "data": { /* command-specific */ }
}

// Error (on stderr)
{
  "metadata": { /* same as success */ },
  "error": {
    "code": "<machine-readable code>",
    "exit_code": <integer>,
    "message": "<human-readable>",
    "hint": "<actionable suggestion>"
  }
}
```

`gitway schema` and `gitway describe` carry `schema_version` at the top
level rather than under `metadata`.  See `docs/json-schema.md` for the
full contract and bump policy.

### Exit codes (frozen at v1.0)

| Code | Meaning |
|---|---|
| 0 | success |
| 1 | general / unexpected error |
| 2 | usage error (bad arguments, invalid configuration) |
| 3 | not found (no key, unknown host) |
| 4 | permission denied (auth failed, host key mismatch) |
| 73 | user declined a confirmation prompt (`gitway hosts add`) |
| 78 | interactive input required but unavailable (`gitway hosts add` w/o `--yes`) |

See `docs/exit-codes.md`.

### Other rules

- `--no-color` / `NO_COLOR`: respected (no ANSI codes are emitted regardless).
- The exec path (normal git relay) never emits JSON to stdout — stdout carries
  binary git-pack data.
- All diagnostic output goes to stderr.

## Dependency policy

- No new crates without discussion.  The dependency tree is intentionally narrow.
- `serde` (with derive) is intentionally absent — JSON output uses `serde_json::json!()`.
- `chrono` and `time` are intentionally absent — ISO 8601 timestamps come from
  `anvil_ssh::time::now_iso8601()`.
- Do not switch the russh crypto backend from `aws-lc-rs` to `ring`.
- New runtime dependencies trigger a `cargo deny` review; see `deny.toml`.

## Reference docs

- `docs/json-schema.md` — JSON envelope contract + bump policy
- `docs/exit-codes.md` — exit-code table
- `docs/log-format.md` — log surface stability tier
- `docs/error-hints.md` — error hint contract
- `docs/ssh_config-deviations.md` — divergence from OpenSSH
- `docs/migration-from-v0.9.md` — v0.9 → v1.0 migration
- `docs/security.md` — threat model
- `docs/v1.0.0-readiness.md` — success metrics audit
- `SECURITY.md` (root) — disclosure policy
- `CHANGELOG.md` (root) — release notes

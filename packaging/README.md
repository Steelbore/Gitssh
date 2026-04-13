# Packaging

This directory contains packaging manifests for Gitway across Linux distributions.

## Arch Linux (AUR)

Two AUR packages are provided:

| File | Package | Description |
|------|---------|-------------|
| `arch/PKGBUILD` | `gitway-bin` | Installs the pre-built musl binary from the GitHub Release |
| `arch/PKGBUILD-git` | `gitway-git` | Builds from source (latest git HEAD) |

`gitway-bin` is recommended for most users — it installs instantly with no compiler needed.

## Debian / Ubuntu (`.deb`)

Built automatically by the GitHub Actions release workflow using
[`cargo-deb`](https://github.com/kornelski/cargo-deb).

To build locally:
```sh
cargo install cargo-deb
cargo deb -p gitway
```

## Fedora / OpenSUSE (`.rpm`)

Built automatically by the GitHub Actions release workflow using
[`cargo-generate-rpm`](https://github.com/cat-in-136/cargo-generate-rpm).

To build locally:
```sh
cargo install cargo-generate-rpm
cargo build --release -p gitway
cargo generate-rpm -p gitway-cli
```

## NixOS / Nix

Install from the flake at the repo root:
```sh
# Run without installing
nix run github:steelbore/gitway

# Install into your profile
nix profile install github:steelbore/gitway

# Use in a NixOS module or home-manager
inputs.gitway.url = "github:steelbore/gitway";
```

## crates.io

The library crate is published to crates.io as `gitway-lib`:
```sh
cargo add gitway-lib
```

Release workflow (workspace):
```sh
# 1) Dry-run the library package
cargo publish -p gitway-lib --dry-run

# 2) Publish the library first
cargo publish -p gitway-lib

# 3) Wait for crates.io index propagation (usually a few minutes)

# 4) Dry-run and publish the CLI crate
cargo publish -p gitway --dry-run
cargo publish -p gitway
```

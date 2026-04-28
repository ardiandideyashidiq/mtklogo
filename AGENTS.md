# AGENTS.md

## Structure
- This repo is not a Cargo workspace. The root crate is the reusable library; the CLI is a separate crate in `cli/`.
- Library entrypoints are in `src/lib.rs`, `src/mtk/*`, and `src/utils/*`.
- CLI entrypoint is `cli/src/main.rs`; subcommands live in `cli/src/command/*`; YAML config loading lives in `cli/src/config/mod.rs`.

## Commands
- Test the library and integration fixtures from the repo root with `cargo test`.
- Test the CLI crate separately with `cargo test --manifest-path cli/Cargo.toml`.
- Run the CLI with `cargo run --manifest-path cli/Cargo.toml -- <subcommand> ...`.
- Run one integration test with `cargo test --test integration_test <test_name>`.

## Workflow Notes
- Because the CLI is a separate crate, root commands do not cover it; verify both crates when touching shared code or CLI behavior.
- Root build artifacts go to `target/`; CLI build artifacts go to `cli/target/`.
- The repo now targets Rust 2024 in both `Cargo.toml` files; keep root and CLI manifests aligned when changing toolchain or dependency versions.

## Config And Fixtures
- The sample CLI config is `cli/resources/bin/mtklogo.yaml`.
- Actual config lookup order in code is: `~/.config/mtklogo.yaml`, then `/etc/mtklogo.yaml`, then `mtklogo.yaml` next to the executable. README prose disagrees; trust `cli/src/config/mod.rs`.
- Integration tests read fixture images from `resources/tests/` and require no external services.

## Current Toolchain Reality
- `cargo test` and `cargo test --manifest-path cli/Cargo.toml` currently pass cleanly on a modern toolchain after the Rust 2024 and warning-cleanup work. Treat new warnings as regressions, not baseline noise.
- The library defaults to feature `with-flate2`; optional `with-libflate` changes compression behavior and test expectations.

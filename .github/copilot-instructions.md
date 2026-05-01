# Copilot instructions for mtklogo

- use pnpm instead of npm

## Build, test, and run

- Root crate (library + integration tests): `cargo test`
- Run one library/unit test: `cargo test <test_name>`
- Run one integration test: `cargo test --test integration_test <test_name>`
- CLI crate: `cargo test --manifest-path cli/Cargo.toml`
- Run the CLI: `cargo run --manifest-path cli/Cargo.toml -- <subcommand> ...`

Touching shared code or CLI behavior usually means verifying both the root crate and the CLI crate.

## High-level architecture

- The repo is **not** a Cargo workspace. The root package is the reusable `mtklogo` library; `cli/` is a separate crate that depends on it.
- The library handles MTK header/logo parsing, blob tables, image conversion, and zlib backends.
- The CLI is a thin layer over the library:
  - `cli/src/main.rs` wires Clap subcommands and argument validation.
  - `cli/src/command/*` contains the actual subcommand implementations.
  - `cli/src/infer.rs` contains the profile/screen-hint inference logic shared by `unpack`, `explore`, and `infer-profile`.
  - `cli/src/config/mod.rs` loads YAML profiles and maps them to `Format`/`Profile`.
- Sample config lives at `cli/resources/bin/mtklogo.yaml`; integration fixtures live under `resources/tests/`.

## Repository-specific conventions

- Trust `cli/src/config/mod.rs` for config lookup order: `~/.config/mtklogo.yaml`, then `/etc/mtklogo.yaml`, then `mtklogo.yaml` next to the executable.
- `ColorMode` string names are fixed and used in filenames/configs: `rgbabe`, `rgbale`, `bgrabe`, `bgrale`, `rgb565be`, `rgb565le`.
- `unpack`/`repack` use a strict filename convention: `logo_{:03}_{mode}.png` for PNGs and `logo_{:03}_raw.z` for raw blobs.
- `repack` sorts input files by slot index and does not enforce contiguous or unique indices.
- `unpack` falls back to raw `.z` output when PNG export fails for a slot.
- The default compression backend is `with-flate2`; `with-libflate` is optional and changes compression behavior/test expectations.
- Keep the root `Cargo.toml` and `cli/Cargo.toml` aligned on edition/toolchain changes.

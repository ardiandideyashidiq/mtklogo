# AGENTS.md

## Repo Map
- Not a Cargo workspace: root crate `mtklogo` is the reusable library, `cli/` is a separate crate, and `gui/` is a separate Tauri app.
- Library entrypoints are `src/lib.rs`, `src/mtk/*`, and `src/utils/*`.
- CLI entrypoint is `cli/src/main.rs`; subcommands live in `cli/src/command/*`; YAML config loading is in `cli/src/config/mod.rs`.
- GUI frontend lives in `gui/`; Tauri wiring is in `gui/src-tauri/`.

## Commands
- Root library + integration tests: `cargo test`.
- One root integration test: `cargo test --test integration_test <test_name>`.
- CLI crate: `cargo test --manifest-path cli/Cargo.toml`.
- Run the CLI: `cargo run --manifest-path cli/Cargo.toml -- <subcommand> ...`.
- GUI dev/build: `cd gui && npm run tauri:dev` / `npm run tauri:build`.

## Repo Rules
- Root commands do not cover `cli/` or `gui/`; verify the affected crate(s) when touching shared code or CLI behavior.
- Config lookup for `mtklogo.yaml` is: `-c` override, then `~/.config/mtklogo.yaml`, then `/etc/mtklogo.yaml`, then `mtklogo.yaml` next to the executable.
- Sample CLI config: `cli/resources/bin/mtklogo.yaml`.
- Integration fixtures are under `resources/tests/` and require no external services.
- `unpack`/`repack` file names are slot-indexed: `logo_{:03}_{mode}.png` or `logo_{:03}_raw.z`; `repack` uses slot order and does not require contiguous or unique indices.
- Default compression backend is `with-flate2`; `with-libflate` changes compression behavior and test expectations.
- Keep root `Cargo.toml` and `cli/Cargo.toml` aligned on edition/toolchain changes.

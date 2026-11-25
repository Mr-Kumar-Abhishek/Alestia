# Alestia

**Alestia** is a memory-safe, Test-Driven Development (TDD) Rust port and architectural re-engineering of [**Celestia**](https://github.com/cybernetics-decentral/Celestia) (`v1.7.0`, `git@github.com:cybernetics-decentral/Celestia.git`), a real-time 3D astronomical visualization and space simulation engine.

## Highlights

- **Memory-Safe Core:** Pure domain crates enforce `#![forbid(unsafe_code)]` with bounds-checked binary and catalog parsers.
- **Data Parity:** Designed for full compatibility with `CelestiaContent` catalogs (`.ssc`, `.stc`, `.dsc`, `stars.dat`, `starnames.dat`, `.cmod`, `.3ds`, `.xyzv`, `.cel`, `.celx`, `cel://`, `celestia.cfg`).
- **Modular Cargo Workspace:** 14 focused crates decoupling math, astrodynamics, ephemerides, asset loading, headless simulation, `wgpu`/WGSL rendering, scripting, desktop UI, CLI tools, and `wasm32-unknown-unknown` browser bindings (`alestia-web`).
- **Strict TDD:** Every module is built Red–Green–Refactor with 1-to-1 parity to Celestia's unit, integration, and script test suites, plus property-based (`proptest`) and fuzz testing.

## Developer Documentation

See the full engineering documentation suite in [docs/developer/](docs/developer/README.md):

- [Developer Hub & Crate DAG](docs/developer/README.md)
- [01 — Software Development Life Cycle (SDLC)](docs/developer/01-SDLC.md)
- [02 — Software Requirements Specification (SRS)](docs/developer/02-SRS.md)
- [03 — Agile Scrum Sprint Plan & Backlog](docs/developer/03-AGILE-SPRINT-PLAN.md)
- [04 — Rust TDD Guide](docs/developer/04-TDD-RUST-GUIDE.md)

## CI/CD Build & Release Verification

All builds, lints, license checks, cross-platform tests, coverage gates, and releases run on **GitHub Actions CI/CD** (no local builds are performed):

- **Debug CI (`Alestia CI`, `.github/workflows/ci.yml`):** Triggers on every push/PR to `master`; runs `taplo` TOML checks, `cargo fmt`, `cargo clippy -- -D warnings` (native & `wasm32-unknown-unknown`), `cargo deny check`, **debug-profile** builds & tests (`--profile dev`) across Linux/Windows/macOS, `wasm32-unknown-unknown` + `wasm-bindgen` verification, and enforces `>= 85%` coverage via `cargo llvm-cov`.
- **WebAssembly & GitHub Pages (`Alestia Pages (WASM)`, `.github/workflows/pages.yml`):** Triggers on `master` pushes and `v*` tags; compiles `alestia-web` to `wasm32-unknown-unknown` (`--release`), generates `wasm-bindgen` bindings, builds Rust API docs (`cargo doc`), and deploys `web/` + WASM + API docs to the `gh-pages` branch ([live site](https://mr-kumar-abhishek.github.io/Alestia/)).
- **Tagged Release CD (`Alestia Release`, `.github/workflows/release.yml`):** Triggers on `v*` Git tags; builds & tests `--release` binaries for Linux, Windows, and macOS, generates `SHA256SUMS.txt`, and publishes to GitHub Releases.

```powershell
# Push changes and watch the CI/CD pipeline via GitHub CLI
git push origin master
gh run list --repo Mr-Kumar-Abhishek/Alestia --limit 5
gh run watch --repo Mr-Kumar-Abhishek/Alestia --exit-status

# Cut a tagged release (triggers release.yml -> GitHub Releases)
git tag -a v0.1.0-alpha.1 -m "Release v0.1.0-alpha.1"
git push origin v0.1.0-alpha.1
```

## License

Alestia is free software; you can redistribute it and/or modify it under the terms of the **GNU General Public License v2.0 or later** (`GPL-2.0-or-later`), matching upstream Celestia (`Copyright © 2001-2026 Celestia Development Team` & Alestia contributors). See [LICENSE](LICENSE).

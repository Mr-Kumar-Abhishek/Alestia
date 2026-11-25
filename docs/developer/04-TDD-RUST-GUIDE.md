# 04 — Test-Driven Development (TDD) Guide in Rust

> **Project:** Alestia (`cybernetics-decentral/Celestia` -> Rust Workspace)  
> **Toolchain:** `rustc 1.95.0+` / `cargo 1.95.0+` (Edition 2024)  
> **Companion Docs:** [README.md](./README.md) | [01-SDLC.md](./01-SDLC.md) | [02-SRS.md](./02-SRS.md) | [03-AGILE-SPRINT-PLAN.md](./03-AGILE-SPRINT-PLAN.md)

---

## 1. Red-Green-Refactor Discipline (CI/CD-First)

Every feature, bugfix, and upstream Celestia port in Alestia follows a strict **Red -> Green -> Refactor** loop executed and verified on **GitHub Actions CI/CD** (no local Cargo builds are performed):

1. **RED — Write a failing test first:**
   - Express the `FR-*` requirement, edge case, or upstream regression in a `#[test]`, `proptest!`, or integration test file *before* writing implementation logic.
   - Define the minimal public API signature (with `todo!()` if needed so the test compiles and **fails for the right reason** in CI).
2. **GREEN — Write the simplest safe Rust to pass:**
   - Implement clean, bounds-checked Rust (`Result<T, E>` via `std::error::Error` / `thiserror`, zero `.unwrap()` on external inputs).
   - Format `.rs` files cleanly (`rustfmt --edition 2024` or editor formatting), commit, push to GitHub, and watch CI pass.
3. **REFACTOR — Improve design while keeping CI green:**
   - Extract shared helpers, add `///` Rustdoc comments + doc-tests, eliminate allocations in hot paths.
   - Push and verify the full GitHub Actions pipeline via `gh`:
     ```powershell
     git push origin master
     gh run list --repo Mr-Kumar-Abhishek/Alestia --limit 5
     gh run watch --repo Mr-Kumar-Abhishek/Alestia --exit-status
     # On failure, inspect exact compiler/clippy/test logs:
     gh run view <run-id> --repo Mr-Kumar-Abhishek/Alestia --log-failed
     ```

---

## 2. Cargo Workspace Blueprint & Crate Stack

### 2.1 Root `Cargo.toml` Blueprint

```toml
[workspace]
resolver = "3"
members = [
    "crates/alestia-util",
    "crates/alestia-math",
    "crates/alestia-astro",
    "crates/alestia-ephem",
    "crates/alestia-model",
    "crates/alestia-image",
    "crates/alestia-font",
    "crates/alestia-engine",
    "crates/alestia-render",
    "crates/alestia-script",
    "crates/alestia-core",
    "crates/alestia-app",
    "crates/alestia-tools",
]

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.95"
license = "GPL-2.0-or-later"

[workspace.lints.rust]
missing_docs = "warn"
rust_2018_idioms = "warn"

[workspace.lints.clippy]
all = { level = "warn", priority = -1 }
pedantic = { level = "warn", priority = -1 }
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
```

> **Note:** Test code (`#[cfg(test)]` and `tests/`) allows `unwrap_used` / `expect_used` / `panic` via `clippy.toml` (`allow-unwrap-in-tests = true`), while production library code must propagate typed errors.

### 2.2 C++ to Rust Dependency Mapping

| Capability | Celestia C++ Dependency | Recommended Rust Crate(s) |
|---|---|---|
| Linear Algebra / Math | `Eigen3` | `glam` (fast SIMD `f32`/`f64` vectors/matrices/quaternions) + `nalgebra` (where arbitrary-dim solvers are needed) |
| GPU Rendering | OpenGL 3.3 / GLES 3.0 + `libepoxy` | `wgpu` + `naga` (WGSL shaders) |
| Windowing & Desktop UI | `Qt6` / `SDL2` | `winit` + `egui` / `egui-wgpu` |
| Image Decoding | `libpng`, `libjpeg-turbo`, custom DDS, `libavif` | `image` (`png`, `jpeg`), custom safe DDS DXT decoder, optional `ravif`/`avif-decode` |
| Font Rasterization | `FreeType2` | `ab_glyph` or `cosmic-text` (pure Rust shaping + LTR/RTL) |
| Lua Scripting (`.celx`) | `LuaJIT` / `Lua 5.1–5.4` | `mlua` (`vendored`, sandboxed memory/instruction hooks) |
| Audio (`ENABLE_MINIAUDIO`) | `miniaudio` | `rodio` / `cpal` (behind `audio` Cargo feature) |
| Formatting / Logging | `fmt`, `std::iostream` | `thiserror`, `tracing`, `tracing-subscriber` |
| Parallelism | Threads / async cache | `rayon` (CPU octree/catalog build) + `tokio` or bounded threadpool for asset streaming |
| Localization (`ENABLE_NLS`) | `gettext`, `ICU` | `fluent` / `gettext-rs` + `icu` (`icu4x`) |
| Testing & Benchmarking | `doctest`, `CTest` | `cargo-nextest`, `proptest`, `approx`, `insta`, `criterion`, `cargo-fuzz`, `cargo-llvm-cov`, `cargo-mutants` |

### 2.3 Cargo Feature Flags (Mirroring CMake Options)

| Celestia `CMakeLists.txt` Option | Alestia Cargo Feature | Default |
|---|---|---|
| `ENABLE_CELX` | `celx` (`alestia-script/celx`) | `on` |
| `ENABLE_SPICE` | `spice` (`alestia-ephem/spice`) | `off` |
| `ENABLE_LIBAVIF` | `avif` (`alestia-image/avif`) | `off` |
| `ENABLE_MINIAUDIO` | `audio` (`alestia-app/audio`) | `off` |
| `ENABLE_FFMPEG` | `video` (`alestia-app/video`) | `off` |
| `ENABLE_NLS` | `nls` (`alestia-core/nls`) | `on` |
| `ENABLE_GLES` | `wgpu/gles` backend selection | `off` |

---

## 3. Six-Layer Testing Pyramid

### Layer 1 — Unit Tests (100% Upstream Parity Map)

All 22 upstream unit test suites from `cybernetics-decentral/Celestia/test/unit/` map directly to Rust crates:

| Upstream C++ Test (`test/unit/`) | Target Rust Crate & Test Module | Key Assertions |
|---|---|---|
| `r128_test.cpp` | `alestia-util/src/r128.rs` | 64.64 arithmetic, Base64 encode/decode, `f64` round-trips |
| `tokenizer_test.cpp` | `alestia-util/src/tokenizer.rs` | Comments, escapes, numbers, brackets, UTF-8, EOF |
| `associativearray_test.cpp` | `alestia-util/src/associative_array.rs` | Typed getters, nested hashes, color arrays & hex |
| `strnatcmp_test.cpp` | `alestia-util/src/strnatcmp.rs` | Natural digit ordering (`HIP 2` < `HIP 10`, `1.05` < `1.2`) |
| `greek_test.cpp` | `alestia-util/src/greek.rs` | Bayer Greek abbreviations, UTF-8 letters, superscripts |
| `unicode_test.cpp` | `alestia-util/src/unicode.rs` | Case folding, UTF-8 validity, Bidi/LTR/RTL helpers |
| `logger_test.cpp` | `alestia-util/src/logger.rs` | Stream routing (`log` vs `err`), level filtering, `LogSize` ring buffer |
| `winutil_test.cpp` | `alestia-util/src/platform.rs` | Windows UTF-16 path/CLI edge cases (`#[cfg(windows)]`) |
| `astro_test.cpp` | `alestia-astro/src/lib.rs` | Magnitude/luminosity, km/AU/LY/pc conversions |
| `date_test.cpp` | `alestia-astro/src/date.rs` | JD/Gregorian, UTC/TDB, leap seconds |
| `kepler_test.cpp` | `alestia-ephem/src/kepler.rs` | Elliptical, parabolic, hyperbolic anomaly solver |
| `category_test.cpp` | `alestia-engine/src/category.rs` | Hierarchical object category membership |
| `constellation_test.cpp` | `alestia-engine/src/constellation.rs` | IAU constellation lookup & asterism links |
| `stellarclass_test.cpp` | `alestia-engine/src/stellar_class.rs` | Spectral/luminosity classes (`O..Y`, `WD`, `C/R/N/S`) |
| `depthpartition_test.cpp` | `alestia-engine/src/depth_partition.rs` | Depth interval splitting & ring far-clip regression (`#2785`/`#2786`) |
| `staroctree_test.cpp` | `alestia-engine/src/star_octree.rs` | Octree insertion, frustum culling, `#2776` boundary |
| `dsooctree_test.cpp` | `alestia-engine/src/dso_octree.rs` | Deep-sky spatial culling by magnitude |
| `resourcesystem_test.cpp` | `alestia-engine/src/resource_system.rs` | Async handle deduplication & cache eviction |
| `psf_test.cpp` | `alestia-render/src/psf.rs` | Irradiance PSF kernel normalization & falloff |
| `rendcontext_test.cpp` | `alestia-render/src/context.rs` | Pipeline state transitions & material binding |
| `loader_validation_test.cpp` | `alestia-core/src/loaders/validation.rs` | `.ssc`/`.stc`/`.dsc` disposition & schema errors |
| `legacy_cmdparser_test.cpp` | `alestia-script/src/legacy/parser.rs` | `.cel` command parsing & parameter validation |

### Layer 2 — Property-Based Tests (`proptest`)
- `R128`: `(a + b) - b == a` (within non-overflowing range); `R128::from_f64(x).to_f64()` error `< 2^-53`; `decode_base64(encode_base64(x)) == Some(x)`.
- `Quat`: `q * q.conjugate() ≈ IDENTITY`; SLERP endpoints & unit length preservation.
- `KeplerOrbit`: Mean anomaly $M \to E \to M$ round-trip error $< 10^{-12}$; specific orbital energy constant over full period.
- `Octree`: Brute-force linear scan of stars within frustum & limiting magnitude equals `StarOctree::query_frustum(...)` set.
- `CMOD`: `parse_ascii(write_ascii(parse_binary(bytes)))` produces bit-equivalent vertex/material data.

### Layer 3 — Integration & Script Tests
- Port `test/integration/3ds_load_test.cpp` (using `test/data/icosphere.3ds`) and `cmod_bin_ascii_roundtrip_test.cpp` (`testmodel.cmod`).
- Port all 20 script integration tests from `test/scripts/`:
  - `.cel` (8 scripts): `goto`, `gotoloc`, `loadfragment`, `move`, `orbit`, `setorientation`, `verbosity`, `windowborder`.
  - `.celx` (12 scripts): `StarColorGet`, `StarColorSet_False`, `StarColorSet_True`, `absmag`, `atmosphere`, `constellation_color`, `distanceto`, `loadfragment`, `pause`, `planet_rings`, `verbosity`, `version`.

### Layer 4 — Differential & Headless Visual Regression Tests
- **Numerical Differential Oracle:** Run fixed JSON/CSV fixture batches against Celestia C++ v1.7.0 (`134ff29`) and NASA JPL Horizons; assert differences remain within `NFR-002` tolerances.
- **Headless `wgpu` Snapshot Diffing:** Render deterministic camera scenes using software adapters (Mesa Lavapipe on Linux, Microsoft WARP on Windows) and compare output PNGs against golden baselines using perceptual SSIM (`>= 0.995`) and RMSE (`<= 0.01`).

### Layer 5 — Fuzzing (`cargo-fuzz`)
- Mandatory `libFuzzer` targets in `fuzz/fuzz_targets/`:
  - `fuzz_3ds_reader` (specifically exercises index/chunk bounds checks from `#2788`)
  - `fuzz_cmod_binary` & `fuzz_cmod_ascii`
  - `fuzz_catalog_tokenizer` (`.ssc`, `.stc`, `.dsc`)
  - `fuzz_cel_script` & `fuzz_cel_url`
  - `fuzz_dds_decoder`

### Layer 6 — Performance Benchmarks (`criterion`)
- Benchmarks gated against regressions: `bench_star_octree_cull_100k`, `bench_vsop87_batch`, `bench_cmod_parse`, `bench_ssc_load_solarsys`, `bench_depth_partition`.

---

## 4. Concrete Rust TDD Walkthroughs

### Walkthrough 1: `R128` 64.64 Fixed-Point Arithmetic (`alestia-util`)

**Step 1 — RED (`crates/alestia-util/src/r128.rs`):**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn r128_addition_carries_across_fractional_boundary() {
        let half = R128::from_f64(0.5);
        let one = half.checked_add(half).expect("0.5 + 0.5 fits");
        assert!((one.to_f64() - 1.0).abs() < 1e-15);
    }

    #[test]
    fn r128_addition_returns_none_on_signed_overflow() {
        assert_eq!(R128::MAX.checked_add(R128::from_raw(1)), None);
    }

    proptest! {
        #[test]
        fn r128_add_sub_roundtrip(a in -1_000_000.0f64..1_000_000.0, b in -1_000_000.0f64..1_000_000.0) {
            let ra = R128::from_f64(a);
            let rb = R128::from_f64(b);
            let sum = ra.checked_add(rb).unwrap();
            let diff = sum.checked_sub(rb).unwrap();
            prop_assert_eq!(diff, ra);
        }
    }
}
```

**Step 2 — GREEN:** Implement `R128` wrapping signed 128-bit `64.64` representation (`lo: u64`, `hi: u64`), checked arithmetic, `from_f64`/`to_f64`, and Base64 URL codec.

**Step 3 — REFACTOR:** Add widening 256-bit `checked_mul`, `is_out_of_bounds`, and `Display` formatting matching Celestia's `r128util.cpp`.

---

### Walkthrough 2: Kepler's Equation Solver (`alestia-ephem`)

**Step 1 — RED (`crates/alestia-ephem/src/kepler.rs`):**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn solve_elliptic_kepler_high_eccentricity_converges() {
        let mean_anomaly = 0.25_f64;
        let e = 0.967_14_f64; // Halley's Comet eccentricity
        let ecc_anomaly = solve_kepler_elliptic(mean_anomaly, e).expect("converges");
        let reconstructed_m = ecc_anomaly - e * ecc_anomaly.sin();
        assert_relative_eq!(reconstructed_m, mean_anomaly, epsilon = 1e-13);
    }

    #[test]
    fn solve_elliptic_kepler_rejects_invalid_eccentricity() {
        assert!(solve_kepler_elliptic(1.0, 1.0).is_err());
        assert!(solve_kepler_elliptic(1.0, -0.1).is_err());
        assert!(solve_kepler_elliptic(f64::NAN, 0.5).is_err());
    }
}
```

**Step 2 — GREEN:** Implement Halley's cubic-convergent iteration with finite & eccentricity range validation.

---

### Walkthrough 3: `DepthPartition` Ring Clipping Regression (`alestia-engine`)

Models the upstream fix in `#2785` / `#2786` (*"Fix depth partitioning around planetary rings"* & *"Fix ring far clipping in low-precision depth intervals"*):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planetary_ring_far_plane_is_not_clipped_in_low_precision_interval() {
        let planet_near = 100_000.0_f64;
        let ring_outer_far = 250_000.0_f64;
        let intervals = partition_depth_intervals(&[
            DepthExtent { near: planet_near, far: ring_outer_far, is_ring: true },
        ]);
        assert!(!intervals.is_empty());
        let covered_far = intervals.iter().map(|i| i.far).fold(0.0_f64, f64::max);
        assert!(
            covered_far >= ring_outer_far,
            "ring far extent {ring_outer_far} was clipped to {covered_far}"
        );
    }
}
```

---

### Walkthrough 4: Bounds-Checked 3DS Mesh Loader (`alestia-model`)

Models upstream hardening `#2788` (*"Reject out-of-range indices in 3DS meshes"*):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_face_with_out_of_range_vertex_index() {
        let vertex_count = 3usize;
        let faces = [[0u16, 1, 2], [0, 1, 99]]; // 99 >= vertex_count
        let err = validate_face_indices(vertex_count, &faces).unwrap_err();
        assert_eq!(
            err,
            ModelError::IndexOutOfBounds { face: 1, index: 99, vertex_count: 3 }
        );
    }
}
```

---

## 5. Testability Architecture Patterns in Rust

1. **Zero Global Mutable State:** Unlike legacy C++ singletons, `Universe`, `Simulation`, `ShaderManager`, and `ResourceCache` are owned structs passed explicitly. Tests spin up isolated `Universe::new()` instances in parallel with zero cross-test interference.
2. **Trait-Based Dependency Injection:**
   - `pub trait Clock { fn now_utc_jd(&self) -> f64; }` -> `SystemClock` in production, `FixedClock(jd)` in tests.
   - `pub trait AssetSource { fn read_bytes(&self, path: &Utf8Path) -> Result<Vec<u8>, AssetError>; }` -> `FsAssetSource` in production, `MemoryAssetSource` (`HashMap`) in unit tests.
   - `pub trait RenderTarget` -> `wgpu::Surface` in desktop app, `wgpu::Texture` (headless Lavapipe/WARP) in CI tests.
3. **Deterministic PRNGs:** Procedural galaxies, globular clusters, and starfield tests accept an explicit seed (`ChaCha8Rng::seed_from_u64(seed)`) so golden snapshots never flake.

---

## 6. GitHub Actions CI/CD Workflow Blueprint

Implemented in `.github/workflows/ci.yml`:
- **`lint-and-policy`:** `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo deny check`.
- **`test-matrix`:** `ubuntu-latest`, `windows-latest`, `macos-latest` running `cargo nextest run --workspace` + `cargo test --doc --workspace`.
- **`coverage`:** `cargo llvm-cov nextest --workspace` enforcing minimum 85% line coverage.

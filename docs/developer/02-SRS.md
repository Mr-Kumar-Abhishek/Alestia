# 02 — Software Requirements Specification (SRS)

> **Standard:** IEEE Std 830 / ISO/IEC/IEEE 29148  
> **System:** Alestia (Rust Port of `cybernetics-decentral/Celestia` v1.7.0, baseline commit `134ff29`)  
> **Companion Docs:** [README.md](./README.md) | [01-SDLC.md](./01-SDLC.md) | [03-AGILE-SPRINT-PLAN.md](./03-AGILE-SPRINT-PLAN.md) | [04-TDD-RUST-GUIDE.md](./04-TDD-RUST-GUIDE.md)

---

## 1. Introduction

### 1.1 Purpose
This Software Requirements Specification (SRS) defines the complete functional, non-functional, and external interface requirements for **Alestia**, a Rust re-engineering of Celestia v1.7.0 (`git@github.com:cybernetics-decentral/Celestia.git`). Every requirement carries a unique identifier (`FR-*`, `NFR-*`) mapped to its target Rust crate, upstream C++ reference, sprint allocation, and TDD verification method.

### 1.2 Scope
Alestia is a real-time 3D astronomical visualization and space simulation application covering scales from spacecraft meters to gigaparsec extragalactic structures. It reads standard `CelestiaContent` catalogs and `celestia.cfg` configurations, computes high-precision ephemerides, renders physically motivated linear-HDR/sRGB scenes via `wgpu`, executes `.cel` and `.celx` (Lua) automation scripts, and provides interactive desktop exploration plus offline data-conversion CLI tools.

### 1.3 Definitions, Acronyms & Abbreviations

| Term | Definition |
|---|---|
| **AU** | Astronomical Unit ($149{,}597{,}870.7\text{ km}$). |
| **BJD / TDB / UTC** | Barycentric Julian Date / Barycentric Dynamical Time / Coordinated Universal Time. |
| **CMOD** | Celestia Model format (ASCII and binary 3D mesh/material format). |
| **DSO** | Deep Sky Object (galaxies, globular clusters, open clusters, nebulae). |
| **ICRF / J2000** | International Celestial Reference Frame / J2000.0 equatorial epoch. |
| **LY** | Light Year. |
| **PSF** | Point Spread Function (used for irradiance-based star rendering). |
| **R128** | 128-bit signed fixed-point integer ($64.64$ format, stored in micro-light-years for universal coordinates). |
| **SSC / STC / DSC** | Solar System Catalog / Star Catalog / Deep Sky Catalog text formats. |
| **VSOP87 / TASS17** | Planetary (Variations Séculaires des Orbites Planétaires) and Saturnian satellite analytical theories. |
| **XYZV** | Sampled trajectory format storing epoch + $(x, y, z, v_x, v_y, v_z)$ records (ASCII `.xyzv` and binary `.xyzvbin`). |

### 1.4 References
1. Upstream Repository: `git@github.com:cybernetics-decentral/Celestia.git` (`master` @ `134ff29`, v1.7.0).
2. `CelestiaContent` Data Repository: `https://github.com/CelestiaProject/CelestiaContent`.
3. Upstream Build & Configuration: `CMakeLists.txt`, `celestia.cfg.in`, `INSTALL.md`.
4. NASA NAIF SPICE Toolkit & JPL Development Ephemerides (DE405/DE430/DE440).

---

## 2. Overall Description

### 2.1 Subsystem Decomposition (C++ to Rust Mapping)

| Upstream C++ Module | Files | Target Rust Crate | Primary Domain |
|---|---|---|---|
| `src/celutil` | 59 | `alestia-util` | `R128`, tokenizer, parser, `AssociativeArray`, binary I/O, UTF-8/Greek, `strnatcmp`, logging |
| `src/celmath` | 14 | `alestia-math` | Vectors, quaternions, frustum, ray, ellipsoid, intersection, polynomial/Kepler solvers |
| `src/celastro` | 7 | `alestia-astro` | Julian date, TDB/UTC leap seconds, astronomical unit & photometric conversions |
| `src/celephem` | 40 | `alestia-ephem` | Orbit & rotation models (Kepler, VSOP87, TASS17, JPL DE, XYZV, custom, script, SPICE) |
| `src/cel3ds` + `src/celmodel` | 14 | `alestia-model` | `.3ds` loader, `.cmod` ASCII & binary parser/writer, mesh & material structures |
| `src/celimage` | 12 | `alestia-image` | PNG, JPEG, DDS (DXT1/3/5), optional AVIF image decoding & pixel format conversions |
| `src/celttf` | 3 | `alestia-font` | TrueType/OpenType glyph rasterization and text layout shaping |
| `src/celengine` | 182 | `alestia-engine` | `Universe`, `Simulation`, `Observer`, `UniversalCoord`, `FrameTree`, Octrees, `Body`, `DepthPartition`, `ResourceSystem` |
| `src/celrender` + `shaders/` | 72 | `alestia-render` | `wgpu`/WGSL renderers (stars, planets, rings, atmospheres, DSOs, guides, sRGB/HDR, warpmesh) |
| `src/celscript` | 47 | `alestia-script` | Legacy `.cel` script engine and `.celx` Lua bindings |
| `src/celestia` | 173 | `alestia-core` + `alestia-app` | App orchestration, `celestia.cfg`, catalog loaders, eclipse finder, desktop UI, media |
| `src/tools` | 66 | `alestia-tools` | CLI asset compilers & converters (`3dstocmod`, `cmodfix`, `xyzv2bin`, `stardb`, etc.) |

### 2.2 User Classes
- **Explorers / Educators:** Navigate 3D space interactively, use Star/Solar System/DSO browsers, Tour Guide, bookmarks, and time controls.
- **Planetarium / Dome Operators:** Configure `fisheye` projection, `warpmesh` distortion (`celestia.cfg`), scripted shows (`.cel`/`.celx`), and high-DPI HUD overlays.
- **Add-on Authors & Researchers:** Create custom `.ssc`/`.stc`/`.dsc` catalogs, `.cmod` models, `.xyzv` trajectories, and run `alestia-tools` CLI utilities.
- **Library Embedders:** Consume `alestia-ephem`, `alestia-engine`, or `alestia-model` as standalone Rust crates.

### 2.3 Operating Environment & Constraints
- **Target OS:** Windows 10/11 (x86_64/arm64), Linux (x86_64/aarch64, Wayland/X11), macOS 13+ (Universal).
- **Graphics Backend:** `wgpu` targeting Vulkan 1.2+, DirectX 12, Metal, and OpenGL ES 3.0 / WebGL2 fallback (mirroring `ENABLE_GLES`).
- **Rust Toolchain:** Stable Rust `1.95.0+` (Edition 2024).
- **Licensing Constraint:** Derived from GNU GPL v2 or later (`cybernetics-decentral/Celestia`); Alestia source and distributed binaries must honor `GPL-2.0-or-later` obligations.

---

## 3. Functional Requirements

> **Priority Legend:** **M** = Must Have, **S** = Should Have, **C** = Could Have (MoSCoW).

### 3.1 Foundation Utilities (`alestia-util`)

| ID | Title | Priority | Specification | Verification Criteria |
|---|---|---|---|---|
| **FR-001** | 128-bit Fixed-Point `R128` Arithmetic | **M** | Implement signed 128-bit (`64.64`) fixed-point arithmetic (`add`, `sub`, `mul`, `neg`, comparisons, `f64` <-> `R128` conversions, Base64 URL serialization) matching `src/celutil/r128.h` and `r128util.cpp`. | Unit & `proptest` suites match all cases in `r128_test.cpp` without overflow UB. |
| **FR-002** | Catalog Tokenizer & Structured Parser | **M** | Tokenize Celestia catalog syntax (comments `#`, quoted strings with escapes, numbers, hex/scientific notation, `{}` blocks, `[]` arrays, `<>` units, identifiers) and parse into `Value` / `AssociativeArray` trees with source line diagnostics. | Matches `tokenizer_test.cpp` & `associativearray_test.cpp`; malformed tokens return structured errors with line numbers. |
| **FR-003** | Natural String Comparison & Greek Substitution | **M** | Implement locale-independent natural numeric sorting (`strnatcmp`) and Greek letter abbreviation expansion/UTF-8 normalization (`Alpha` <-> `α`, superscripts `⁰`..`⁹`). | Matches `strnatcmp_test.cpp`, `greek_test.cpp`, and `unicode_test.cpp`. |
| **FR-004** | Endian-Safe Binary Reader/Writer & Color Types | **M** | Provide safe, bounds-checked big/little-endian readers/writers (`read_le_f32`, `read_be_u32`, etc.) and linear/sRGB `Color` conversions. | Rejects truncated buffers cleanly with `UnexpectedEof`; color conversions accurate within `1e-6`. |
| **FR-005** | Structured Logger & Locale/Number Formatters | **S** | Support multi-level logging (`Error`, `Warning`, `Info`, `Verbose`, `Debug`, plus `LogSize` ring buffer configurable via `celestia.cfg`), date/distance formatting, and UTF-8 / RTL layout direction helpers. | Matches `logger_test.cpp`; respects `LayoutDirection` (`ltr`/`rtl`). |

### 3.2 Mathematics & Astronomy (`alestia-math`, `alestia-astro`)

| ID | Title | Priority | Specification | Verification Criteria |
|---|---|---|---|---|
| **FR-006** | 3D Geometry & Intersection Primitives | **M** | Provide double-precision (`f64`) and single-precision (`f32`) vectors, quaternions, rays, spheres, ellipsoids (oblate planets), view frustums, and ray-ellipsoid / frustum-sphere intersection tests. | Unit tests verify analytical intersections, plane culling, and quaternion normalization invariants. |
| **FR-007** | Root Finding & Polynomial Solvers | **M** | Provide Newton-Raphson, Halley, and Brent root solvers with bounded iteration counts and guaranteed termination. | Converges within `1e-13` on Kepler/ellipsoid benchmarks; never infinite-loops on `NaN`. |
| **FR-008** | Astronomical Timekeeping (`UTC` / `TDB` / `JD`) | **M** | Convert Gregorian/ISO dates <-> Julian Date (`JD`), apply NIST `leap-seconds.list` (configurable via `LeapSecondsFile`), and convert `UTC <-> TAI <-> TT <-> TDB`. | Matches `date_test.cpp` and USNO/IERS reference epochs to `< 1e-9` days. |
| **FR-009** | Astronomical Units & Photometry | **M** | Convert between km, AU, light-years, parsecs, meters, radii, solar masses, and apparent/absolute magnitude <-> luminosity/irradiance (`astro_test.cpp`). | Matches `astro_test.cpp` to `1e-12` relative error. |

### 3.3 Ephemerides & Kinematics (`alestia-ephem`)

| ID | Title | Priority | Specification | Verification Criteria |
|---|---|---|---|---|
| **FR-010** | Keplerian Elliptical, Parabolic & Hyperbolic Orbits | **M** | Solve Kepler's equation for $e < 1$, $e = 1$, and $e > 1$ (near-parabolic Stumpff/Barker handling), returning position & velocity in km / km·s⁻¹. | Matches `kepler_test.cpp` and analytical orbital invariants (vis-viva, angular momentum conservation). |
| **FR-011** | Analytical Planetary & Lunar Ephemerides (VSOP87, TASS17, CustomOrbits) | **M** | Implement VSOP87 series (Mercury–Neptune), TASS17 (Saturnian moons), Galilean/Uranian/Plutonian custom analytical orbits, and ELP-2000/82 lunar theory. | Differential comparison vs Celestia C++ v1.7.0 agrees within `1e-9` AU across 1900–2100. |
| **FR-012** | Sampled Trajectories (`.xyz`, `.xyzv`, Binary `xyzvbin`) & Interpolation | **M** | Parse ASCII `.xyz`/`.xyzv` and binary `.xyzvbin` trajectory tables; support linear, cubic Hermite, and Akima/spline interpolation with velocity derivatives. | Round-trip binary vs ASCII positions within `1e-7` km; handles out-of-range clamping. |
| **FR-013** | JPL Binary Ephemerides (DE200/405/430/440) | **S** | Read JPL binary ephemeris files (`jpleph`) using Chebyshev polynomial evaluation for Sun, Moon, planets, and solar system barycenter. | Matches JPL testpo checksums within `1e-12` AU. |
| **FR-014** | Rotational Models, Precession & Nutation | **M** | Support `UniformRotation`, `PrecessingRotation` (IAU/p03 precession + nutation), `SampledOrientation` (`.q`), and `ScriptRotation`. | Matches Celestia rotational orientations within `1e-8` radians. |
| **FR-015** | Optional NAIF SPICE Integration (`spice` feature) | **C** | When built with `--features spice`, load `.bsp`/`.bpc`/`.tls`/`.tf` kernels via thread-safe wrapper for `SpiceOrbit` and `SpiceRotation`. | Feature-gated integration test computes spacecraft state against pre-recorded SPICE output. |

### 3.4 Catalogs, Configuration & Data Loading (`alestia-core`, `alestia-engine`)

| ID | Title | Priority | Specification | Verification Criteria |
|---|---|---|---|---|
| **FR-016** | `celestia.cfg` Configuration Parser | **M** | Parse all directives from `celestia.cfg`: `StarDatabase`, `StarNameDatabase`, `StarCatalogs`, `HDCrossIndex`, `SAOCrossIndex`, `SolarSystemCatalogs`, `DeepSkyCatalogs`, `AsterismsFile`, `BoundariesFile`, `StarTextures`, `FavoritesFile`, `DestinationFile`, `Cursor`, `ProjectionMode`, `ViewportEffect`, `WarpMeshFile`, `LeapSecondsFile`, `LayoutDirection`, `MeasurementSystem`, `TemperatureScale`, `SRGBRendering`, `ToneMappingMode`, `ToneMappingExposure`, `LogSize`, `ExtrasDirectories`. | Parses upstream `celestia.cfg.in` cleanly; missing optional keys use documented defaults. |
| **FR-017** | Binary Star Database (`stars.dat`), Names & Cross-Indices | **M** | Load `stars.dat`, `starnames.dat`, `starxindex-hd.dat`, `starxindex-sao.dat`, and resolve HIP/HD/SAO, Bayer, Flamsteed, and common names. | Lookup by `"HIP 32349"`, `"Sirius"`, `"Alpha CMa"`, `"HD 48915"` resolves identical `Star` IDs. |
| **FR-018** | Star Catalogs (`.stc`) & `StellarClass` | **M** | Parse `.stc` files supporting `Add`/`Replace`/`Modify` dispositions, spectral types (`O,B,A,F,G,K,M`, carbon `C,R,N,S`, Wolf-Rayet `WC,WN,WO`, brown dwarfs `L,T,Y`, white dwarfs `WD`, `NeutronStar`), barycenters, and binary orbits. | Matches `stellarclass_test.cpp` and loads `test/data/nearstars.stc` + `CelestiaContent` `.stc` files. |
| **FR-019** | Solar System Catalogs (`.ssc`) & Timelines | **M** | Parse `.ssc` planets, moons, dwarf planets, asteroids, comets, spacecraft, locations, rings, atmospheres, surfaces, and multi-phase `Timeline` definitions (`Beginning`/`Ending` + `OrbitFrame`/`BodyFrame`) with `Add`/`Replace`/`Modify`. | Matches `loader_validation_test.cpp`; invalid disposition/missing parent logs structured warning without crashing. |
| **FR-020** | Deep Sky Catalogs (`.dsc`), Asterisms & Boundaries | **M** | Parse `.dsc` galaxies (Hubble types), globular clusters, open clusters, nebulae, plus `asterisms.dat` and `boundaries.dat`. | Matches `constellation_test.cpp` and loads all `CelestiaContent` DSOs. |
| **FR-021** | Category & Selection Tagging System | **M** | Assign hierarchical `Category` tags to stars, bodies, and DSOs and filter/query by category. | Matches `category_test.cpp`. |

### 3.5 3D Models, Images & Fonts (`alestia-model`, `alestia-image`, `alestia-font`)

| ID | Title | Priority | Specification | Verification Criteria |
|---|---|---|---|---|
| **FR-022** | CMOD ASCII & Binary Parser/Writer | **M** | Parse and serialize `.cmod` ASCII and binary meshes, vertex semantics (`position`, `normal`, `color0`, `texcoord0..3`, `tangent`, `pointsize`), primitive groups (`trilist`, `tristrip`, `linelist`, `pointlist`, `sprite`), and materials (`diffuse`, `specular`, `emissive`, `opacity`, `textures`). | Matches `cmod_bin_ascii_roundtrip_test.cpp` on `test/data/testmodel.cmod` and `.txt`. |
| **FR-023** | Bounds-Checked 3DS Model Loader | **M** | Parse `.3ds` chunk hierarchies, meshes, UV coordinates, and materials while strictly rejecting out-of-range vertex/face indices (upstream `#2788`). | Loads `test/data/icosphere.3ds`; fuzz/malformed test cases with out-of-range indices return `Err`, never panic. |
| **FR-024** | Texture & Image Decoders + Virtual Textures | **M** | Decode PNG, JPEG, DDS (uncompressed + DXT1/DXT3/DXT5 decompression), and optional AVIF (`avif` feature), plus multi-level `.ctx` Virtual Texture tile pyramids. | Decodes all sample textures; handles alpha, mipmaps, and sRGB vs linear color tags accurately. |
| **FR-025** | TrueType/OpenType Font Rendering & Text Layout | **M** | Load `.ttf`/`.otf`/`.txf` fonts, rasterize Unicode glyph atlases on demand, and lay out HUD/label text in both `ltr` and `rtl` directions. | Renders ASCII, Greek, CJK, and RTL glyphs without missing-glyph panics. |

### 3.6 Simulation Engine & Spatial Indexing (`alestia-engine`)

| ID | Title | Priority | Specification | Verification Criteria |
|---|---|---|---|---|
| **FR-026** | `UniversalCoord` & `Observer` Coordinate Frames | **M** | Represent observer positions in `UniversalCoord` (`R128` micro-LY components + offset) and support all 6 Celestia reference frames (`Universal`, `Ecliptic`, `Equatorial`, `BodyFixed`, `PhaseLock`, `Chase`), plus `TwoVector` frames. | Frame transformations remain jitter-free at distances from 1 m to $10^{10}$ LY. |
| **FR-027** | `StarOctree` & `DSOOctree` Spatial Culling | **M** | Build dynamic magnitude-sorted spatial octrees for stars and DSOs; support frustum culling down to limiting apparent magnitude. | Matches `staroctree_test.cpp` and `dsooctree_test.cpp` (including platform-independent culling boundary `#2776`). |
| **FR-028** | Camera Navigation, `Goto`, `Follow`, `SyncOrbit` & Time Scale | **M** | Smooth exponential/SLERP `Goto` transitions, `Follow`, `SyncOrbit`, `Chase`, `Lock`, time rate control (`-1e15x` to `+1e15x`, pause, reverse), and light-time delay toggle. | Deterministic simulation tests verify arrival pose and time progression. |
| **FR-029** | Depth Partitioning (`DepthPartition`) | **M** | Split the scene depth range dynamically into near-to-far logarithmically/heuristically bounded depth intervals so near spacecraft and distant planets never z-fight, preserving planetary ring far-clipping bounds (upstream `#2785`, `#2786`). | Matches `depthpartition_test.cpp` plus regression tests for rings crossing partition boundaries. |
| **FR-030** | `cel://` State URLs & Resource Caching | **M** | Serialize/deserialize complete simulation state (observer pose, frame, selection, JD time, render flags, FOV) to/from `cel://` URIs; manage async texture/mesh resource caches. | Matches `resourcesystem_test.cpp` and round-trips `cel://` URLs losslessly. |
| **FR-031** | Object Selection, Autocomplete & Eclipse Finder | **M** | Pick 3D objects via ray-casting (`Selection`), provide prefix/natural autocomplete (`Completion`), and compute solar/lunar eclipse windows (`EclipseFinder`). | Ray-pick hits target body/star; eclipse finder predicts known historical eclipses (e.g., 2024-04-08 total solar eclipse) within 1 minute. |

### 3.7 Rendering Pipeline (`alestia-render`)

| ID | Title | Priority | Specification | Verification Criteria |
|---|---|---|---|---|
| **FR-032** | Linear-Light HDR & sRGB Tone-Mapping Pipeline | **M** | Render into a linear HDR framebuffer (`Rgba16Float`), then run a final sRGB post-process supporting `SRGBRendering` (true/false), `ToneMappingMode` (`Off` / `Manual`), and `ToneMappingExposure` (upstream `#2784`, `#2787`, `#2794`). | Headless render diff verifies linear blending, no double-decoded overlay/CMOD colors, and exponential curve $1 - \exp(-E \cdot L)$. |
| **FR-033** | Irradiance-Based PSF Star & Deep-Sky Rendering | **M** | Render point/large stars and glare halos using physical irradiance & Gaussian/Airy PSF shaders (`star`, `largestar`, `psfstarpoint`, `psfstarglow`, `psfstarglowlarge` equivalents; `#2781`). | Matches `psf_test.cpp` and preserves dynamic range under exposure changes. |
| **FR-034** | Planetary Surfaces, Night Lights, Bump/Specular & Cloud Layers | **M** | Render `LodSphereMesh` planets with diffuse, normal/bump, specular, night-side emissive, and animated cloud layers with multi-light shadow casting (eclipse umbra/penumbra). | Golden image tests verify terminator lighting, specular oceans, and eclipse shadows. |
| **FR-035** | Planetary Rings & Atmospheric Scattering | **M** | Render alpha/optical-depth planetary rings with planet-on-ring and ring-on-planet shadows, plus Rayleigh/Mie limb atmospheres (`atmosphere`). | Matches `atmosphere.celx` and `planet_rings.celx` visual expectations. |
| **FR-036** | CMOD/3DS Mesh, Comet Tails & Tidal Renderers | **M** | Render spacecraft/asteroids via CMOD materials, procedural comet ion/dust tails (`comet`), and tidal distortion meshes (`tidal`). | Matches `rendcontext_test.cpp` and CMOD golden renders. |
| **FR-037** | DSO Renderers & Astronomical Guides | **M** | Render spiral/elliptical/irregular `galaxy` sprites, `globular` clusters, `opencluster`s, `nebula`e, `asterisms`, constellation `boundaries`, `skygrid`s (equatorial/ecliptic/galactic/horizontal), `planetgrid`s, `eclipticline`, `markers`, and `referencemark`s. | Visual snapshot & flag-toggle tests verify each render flag independently. |
| **FR-038** | Projection Modes (`Perspective`, `Fisheye`) & `WarpMesh` | **M** | Support standard rectilinear perspective and planetarium dome `fisheye` projection, plus Paul Bourke `.map` `warpmesh` post-process distortion. | Warp UV grid test verifies exact mapping of Bourke `.map` control points. |

### 3.8 Scripting & Automation (`alestia-script`)

| ID | Title | Priority | Specification | Verification Criteria |
|---|---|---|---|---|
| **FR-039** | Legacy `.cel` Command Parser & Executor | **M** | Parse and execute `.cel` scripts (`goto`, `gotoloc`, `move`, `orbit`, `setorientation`, `loadfragment`, `verbosity`, `windowborder`, `wait`, `print`, `select`, `center`, `renderflags`, `time`). | Matches `legacy_cmdparser_test.cpp` and passes all 8 `.cel` tests in `test/scripts/`. |
| **FR-040** | `.celx` Lua Runtime & Celestia API Bindings | **M** | Embed sandboxed Lua (`mlua` supporting Lua 5.1–5.4/LuaJIT semantics) exposing `celestia`, `observer`, `object`, `position`, `rotation`, `vector`, `frame`, `phase`, `category`, and `wait`/`tick` coroutines. | Passes all 12 `.celx` scripts in `test/scripts/` (`StarColorGet`, `absmag`, `distanceto`, `planet_rings`, etc.). |
| **FR-041** | Optional `celx` Scripted GL Overlay (`celx_gl`) | **C** | Provide safe 2D vector/text/image overlay primitives for Lua scripts (`scripts/overlay/`) mapped to `alestia-render` overlay queue. | Renders `scripts/mark-lg.celx` & grid scripts without raw OpenGL pointer exposure. |

### 3.9 Desktop UI, Browsers & Media (`alestia-app`)

| ID | Title | Priority | Specification | Verification Criteria |
|---|---|---|---|---|
| **FR-042** | Multi-View Viewport, Input & HUD | **M** | Support split-screen `ViewManager`, mouse/keyboard navigation (right-drag orbit, `G` goto, `C` center, `Enter` object entry, `Space` pause), and HUD telemetry (target, speed, UTC/local time, FPS). | Integration tests verify split views, keyboard shortcuts, and HUD formatting (`metric`/`imperial`, `K`/`C`/`F`). |
| **FR-043** | Star, Solar System, DSO Browsers & Tour Guide | **M** | Interactive tables/trees for 100 Nearest/Brightest/With-Planets stars, Solar System hierarchy, DSOs, Tour Guide (`guide.cel`), Bookmarks (`favorites.cel`), and Eclipse/Event Finder. | UI state tests verify sorting, filtering, selection, and goto dispatch. |
| **FR-044** | Optional Audio Playback & Video Capture (`audio`, `video` features) | **C** | Feature-gated audio overlay playback (`alestia-app/audio`) and frame/video capture (`ffmpeg` CLI pipe or library). | Smoke test exports deterministic PNG sequence / MP4 header when enabled. |

### 3.10 Offline Data & Asset Tools (`alestia-tools`)

| ID | Title | Priority | Specification | Verification Criteria |
|---|---|---|---|---|
| **FR-045** | CMOD & 3DS CLI Suite (`3dstocmod`, `cmodfix`, `cmodsphere`) | **M** | Convert `.3ds` -> `.cmod`, normalize/tangent-gen/weld/ASCII<->binary convert via `cmodfix`, and generate tessellated spheres via `cmodsphere`. | Byte/semantic parity with `src/tools/cmod/` on `icosphere.3ds` and `testmodel.cmod`. |
| **FR-046** | Trajectory & Star Catalog Compilers (`xyzv2bin`, `spice2xyzv`, `stardb`, `xindex`) | **S** | Convert `.xyzv` <-> `.xyzvbin`, sample SPICE kernels to `.xyzv` (with `spice` feature), and build/dump `stars.dat` & HD/SAO cross-index `.dat` files. | Compiled `.xyzvbin` and `.dat` files load identically in both Celestia C++ and Alestia. |

---

## 4. Non-Functional Requirements

| ID | Category | Requirement | Target / Metric |
|---|---|---|---|
| **NFR-001** | **Performance** | Interactive 1080p/4K real-time frame rate with full `CelestiaContent` star/solar system loaded. | $\ge 60\text{ FPS}$ ($\le 16.6\text{ ms}$ frame budget) on mid-tier GPU; `< 2.0 s` startup to Earth view. |
| **NFR-002** | **Numerical Precision** | Ephemeris and coordinate accuracy at solar-system and galactic scales. | Zero visible camera jitter at 1 m orbit around ISS; VSOP87/Kepler $\le 10^{-9}\text{ AU}$ vs C++ reference. |
| **NFR-003** | **Memory Safety** | Eliminate buffer overflows, use-after-free, null-deref, and data races across domain & parser crates. | `#![forbid(unsafe_code)]` on 9 core crates; `0` panics across 10M+ `cargo-fuzz` executions on `.3ds`/`.cmod`/`.ssc`. |
| **NFR-004** | **Concurrency** | Non-blocking background streaming of textures, virtual tiles, and meshes. | Frame time variance (`p99 - p50`) `< 4 ms` during texture tile streaming; `Send + Sync` verified by compiler. |
| **NFR-005** | **Portability** | Identical simulation math and headless-testable rendering across Windows, Linux, and macOS. | CI passes on `windows-latest`, `ubuntu-latest` (Lavapipe), and `macos-latest`. |
| **NFR-006** | **Localization & Units** | Support Gettext `.po`/`.mo` translations, `ltr`/`rtl` text, `metric`/`imperial` units, `K`/`C`/`F` scales. | 100% of HUD strings routed through localization key lookup; RTL smoke test passes. |
| **NFR-007** | **Observability** | Structured diagnostics via `tracing` crate + in-app log console. | Configurable verbosity (`~` console), zero allocations in hot render loop when log disabled. |
| **NFR-008** | **Testability & Coverage** | Automated TDD coverage and mutation resilience. | $\ge 95\%$ line coverage on math/ephem/util; $\ge 85\%$ workspace-wide; `cargo-mutants` score $\ge 80\%$ on critical parsers. |
| **NFR-009** | **Modularity** | Clean library/UI separation so headless simulation & CLI tools require zero windowing or GPU context. | `alestia-core`, `alestia-engine`, `alestia-ephem`, and `alestia-tools` compile & pass tests with `--no-default-features` (no `wgpu`/`winit`). |
| **NFR-010** | **License Compliance** | Full `GPL-2.0-or-later` source & third-party attribution compliance. | `cargo deny check licenses` passes on every commit. |

---

## 5. External Interface Requirements

### 5.1 File Format Compatibility Matrix
- **Read-Only Compatible:** `celestia.cfg`, `.stc`, `.ssc`, `.dsc`, `.dat` (`stars.dat`, `starnames.dat`, `asterisms.dat`, `boundaries.dat`, `starxindex-*.dat`), `.3ds`, `.cmod` (ASCII/binary), `.xyz`, `.xyzv`, `.xyzvbin`, `.ctx` (virtual textures), `.cel`, `.celx`, `.cel` bookmarks/tour guides, `.map` (Bourke warpmesh), `leap-seconds.list`, `.png`, `.jpg`, `.dds`, `.avif`, `.ttf`/`.otf`.
- **Read-Write Compatible:** `.cmod` (ASCII/binary), `.xyzvbin`, `favorites.cel` (bookmarks), `cel://` URI strings.

### 5.2 GPU Shader Interface (`shaders/*.glsl` -> WGSL)

| Upstream GLSL Pair (`shaders/`) | Alestia WGSL Module | Pipeline Stage |
|---|---|---|
| `star`, `largestar` | `star.wgsl`, `largestar.wgsl` | Main HDR pass (irradiance billboards) |
| `psfstarpoint`, `psfstarglow`, `psfstarglowlarge` | `psf_star.wgsl`, `psf_glow.wgsl` | Irradiance PSF star & halo pass |
| `galaxy`, `globular` | `galaxy.wgsl`, `globular.wgsl` | DSO instanced/billboard pass |
| `comet`, `tidal` | `comet.wgsl`, `tidal.wgsl` | Dynamic mesh/tail pass |
| `depth` | `depth.wgsl` | Depth pre-pass / shadow mapping |
| `crosshair`, `selpointer`, `text` | `overlay_ui.wgsl`, `text.wgsl` | Linear/sRGB-aware 2D HUD & marker pass |
| `srgb` | `post_srgb.wgsl` | HDR -> exponential tone-map -> sRGB output |
| `warpmesh` | `post_warpmesh.wgsl` | Dome mesh distortion post-process |

### 5.3 Scripting & URI Interfaces
- **CLI (`alestia-app`, `alestia-tools`):** Standard `--conf <path>`, `--dir <extras>`, `--url <cel://...>`, `--script <file.cel|celx>`, `--headless-screenshot <out.png>`.
- **URI Scheme:** `cel://Follow/<Target>/<ISO-Time>?x=...&y=...&z=...&ow=...&ox=...&oy=...&oz=...&select=...&fov=...&ts=...&rf=...&lm=...`.

---

## 6. Requirements Traceability Matrix (RTM)

| Req ID | Crate | Upstream C++ Reference | Target Sprint | Primary TDD Layer |
|---|---|---|---|---|
| `FR-001`..`FR-005` | `alestia-util` | `src/celutil/*` | Sprint 1 | Unit + `proptest` (`r128`, `tokenizer`, `strnatcmp`, `greek`, `unicode`, `logger`) |
| `FR-006`..`FR-009` | `alestia-math`, `alestia-astro` | `src/celmath/*`, `src/celastro/*` | Sprint 2 | Unit + `proptest` + Differential (`astro_test`, `date_test`) |
| `FR-010`..`FR-015` | `alestia-ephem` | `src/celephem/*` | Sprint 3 | Unit + Differential vs C++ & JPL Horizons (`kepler_test`) |
| `FR-016`..`FR-021` | `alestia-core`, `alestia-engine` | `configfile`, `load*`, `stellarclass`, `category` | Sprint 4 | Unit + Integration (`loader_validation`, `stellarclass`, `nearstars.stc`) |
| `FR-022`..`FR-025` | `alestia-model`, `alestia-image`, `alestia-font` | `src/cel3ds`, `celmodel`, `celimage`, `celttf` | Sprint 5 | Integration + Fuzz (`3ds_load_test`, `cmod_bin_ascii_roundtrip_test`) |
| `FR-026`..`FR-031` | `alestia-engine`, `alestia-core` | `universe`, `observer`, `octree`, `depthpartition`, `eclipsefinder` | Sprint 6 | Unit + Property + Differential (`depthpartition`, `staroctree`, `dsooctree`) |
| `FR-032`..`FR-033` | `alestia-render` | `srgb`, `psf*`, `pointstarrenderer` | Sprint 7 | Unit + Headless `wgpu` Snapshot (`psf_test`, `rendcontext_test`) |
| `FR-034`..`FR-036` | `alestia-render` | `render`, `ringrenderer`, `atmosphererenderer`, `cometrenderer` | Sprint 8 | Headless `wgpu` Golden Diff + Ring Depth Regression |
| `FR-037`..`FR-038` | `alestia-render` | `galaxyrenderer`, `skygridrenderer`, `fisheye*`, `warpmesh` | Sprint 9 | Headless `wgpu` Golden Diff + Warp UV Matrix Tests |
| `FR-039`..`FR-041` | `alestia-script` | `src/celscript/*` | Sprint 10 | Unit + All 20 Upstream `.cel`/`.celx` Script Tests |
| `FR-042`..`FR-044` | `alestia-app` | `src/celestia/qt/*`, `sdl/*`, `hud`, `audiosession` | Sprint 11 | UI State Tests + E2E Smoke + Audio/Capture Feature Tests |
| `FR-045`..`FR-046` | `alestia-tools` | `src/tools/*` | Sprint 12 | CLI Golden File Tests + Performance Benchmarks (`criterion`) |
| `NFR-001`..`NFR-010` | Workspace-wide | CI / `CMakeLists.txt` | Sprints 1–12 | `criterion`, `cargo-fuzz`, `cargo-llvm-cov`, `cargo-deny`, `miri` |

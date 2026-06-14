# Design S11 — WS-4 Phase 2: Pure-Rust Dependency Collapse + Cross-Platform Build

Status: DESIGN / SPECIFICATION ONLY. No source modified. This is the spec the Test Engineer writes
RED tests against and two parallel Implementers code against. Author role: Rust Architect (Swaram).
Grounded against the working tree at session start (`src/engine.rs`, `src/main.rs`,
`src/image_analysis.rs`, `src/image_source.rs`, `src/midi_output.rs`, `src/chord_engine.rs`,
`src/lib.rs`, `src/tui.rs`, `Cargo.toml`, `assets/`) and reconciled against the two prior design
docs: `docs/assessment-ws4-ux-crossplatform.md` (§2–§3, §6 — the governing WS-4 portability plan)
and `docs/design-s9-engine-seam-cli.md` (the `FeatureSource`/`AudioSink`/`EngineSnapshot` seam this
work implements WITHOUT change).

> Convention: Rust signatures and TOML give the SHAPE of each seam. **No implementation bodies are
> written.** Crate/version claims verified live against crates.io/docs.rs are dated 2026-06-13 and
> marked accordingly; anything not so verified is marked **[VERIFY]**.

---

## 0. Executive summary (read first)

The goal of Phase 2: a **DEFAULT, no-feature-flag `cargo build` / `cargo run` builds, runs, and emits
SOUND** on an ordinary clean Linux OR Windows box — no OpenCV, no libclang, no system OpenCV, no
external FluidSynth process, no virtual MIDI port, no ALSA-seq wiring. OpenCV-grade analysis and
external FluidSynth/MIDI-out remain reachable as **opt-in feature flags**. No capability is deleted.

Four design pieces, structured as **two file-disjoint implementation lanes plus one serialized shared
touch**:

- **Lane A — Image-feature port** (§2, §3.A): a NEW pure-Rust analyzer (`image` + `imageproc`)
  implementing the EXISTING `engine::FeatureSource` trait, producing the EXISTING image-free mirror
  structs `engine::GlobalFeatures` / `engine::ScanBarFeatures`. Files owned: `src/pure_analysis.rs`
  (NEW). The engine core is unchanged.
- **Lane B — In-process synth sink** (§3.B, §4): a NEW pure-Rust sink implementing the EXISTING
  `engine::AudioSink` trait, doing in-process SoundFont synthesis + audio output via
  **rustysynth + cpal** (recommended over oxisynth — §4). Files owned: `src/synth_sink.rs` (NEW).
- **Shared serialized touch — Cargo feature reshape** (§5): flip the DEFAULT to pure-Rust; keep
  `opencv` and external `midir` MIDI-out as opt-in. Files: `Cargo.toml` + the `#[cfg(feature=…)]`
  analyzer/sink selection in `src/main.rs`. This is the ONE file both lanes converge on; it lands
  after both lanes compile.
- **Cross-platform build/run/packaging slice** (§4.4, §6, §8): SoundFont bundling/loading, cpal
  backend per OS, the surviving native-dep audit (target: NONE in the default path), the smoke story,
  and what THIS box can do once OpenCV is off the default path.

**The engine seam (`src/engine.rs`) ends Phase 2 byte-unchanged.** Both `FeatureSource` and
`AudioSink` already exist exactly as Lane A and Lane B need them (S9 shipped them; verified below).
No trait change is required. §8 flags the one place a seam change could be *argued* (block-rate render
vs. per-event note send) and explains why it is NOT needed — recorded as an explicit decision point.

---

## 1. CURRENT STATE ANALYSIS

### 1.1 The two seams Phase 2 implements (EXACT current contracts — DO NOT CHANGE)

Verified in `src/engine.rs` at session start. These are the contracts; the file must end Phase 2
byte-unchanged.

**`FeatureSource`** (`engine.rs:84–96`):

```rust
pub trait FeatureSource {
    fn global_features(&self) -> GlobalFeatures;                                           // :86
    fn scan_bar_features(&self, step_idx: usize, num_instruments: usize) -> Vec<ScanBarFeatures>; // :92
    fn step_count(&self) -> usize;                                                          // :95
}
```

**`AudioSink` + `AudioSinkError`** (`engine.rs:102`, `132–139`):

```rust
#[derive(Debug)]
pub struct AudioSinkError(pub Box<dyn std::error::Error + Send + Sync + 'static>);          // :102
// AudioSinkError::new<E: Error + Send + Sync + 'static>(e)  (:118)
// AudioSinkError::msg(impl Into<String>)                    (:123)

pub trait AudioSink {
    fn note_on(&mut self, channel: u8, note: u8, velocity: u8) -> Result<(), AudioSinkError>;  // :134
    fn note_off(&mut self, channel: u8, note: u8) -> Result<(), AudioSinkError>;               // :136
    fn program_change(&mut self, channel: u8, program: u8) -> Result<(), AudioSinkError>;      // :138
}
```

**The mirror feature structs** Lane A must produce (`engine.rs:32–74`):

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlobalFeatures {            // :33
    pub avg_hue: f32,          // 0..360
    pub avg_saturation: f32,   // 0..100
    pub avg_brightness: f32,   // 0..100 (HSV value)
    pub edge_density: f32,     // 0..1
    pub hue_spread: f32,       // 0..1
    pub texture_laplacian_var: f32,
    pub shape_complexity: f32,
    pub aspect_ratio: f32,     // width/height
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScanBarFeatures {           // :59
    pub bar_index: usize,
    pub avg_hue: f32,
    pub avg_saturation: f32,
    pub avg_brightness: f32,
    pub edge_density: f32,
    pub texture_laplacian_var: f32,
    pub hue_hist: Vec<f32>,
}
```

These mirror `image_analysis::GlobalFeatures` (`image_analysis.rs:11–20`) and
`image_analysis::ScanBarFeatures` (`image_analysis.rs:22–31`) field-for-field. The OpenCV adapter
(`main.rs`) copies `image_analysis::*` → `engine::*` at `main.rs:119–143` (`to_eng_global` /
`to_eng_scanbar`). **Lane A's analyzer builds `engine::*` DIRECTLY — it never constructs an
`image_analysis::*` struct, so it never names OpenCV.**

### 1.2 The data that reaches the sink (chord_engine, READ-ONLY)

The engine drives the sink in `PipelineEngine::tick` (`engine.rs:397–404`) as paired note_on/note_off:

```rust
for dec in &decisions {
    for ev in &dec.events {
        sink.note_on(dec.channel, ev.note, ev.velocity)?;
        sink.note_off(dec.channel, ev.note)?;
        ...
```

But the ACTUAL batch playback path is the `main.rs` adapter driver loop (`main.rs:310–371`), which:
1. calls `engine.decide_step(&source, step_idx)` for the per-instrument decisions,
2. applies jitter + `Instant`-based wall-clock scheduling (D8 — adapter owns timing/RNG),
3. sends `AudioSink::note_on` / `note_off` / `program_change` through the trait.

`NoteEvent` (the payload, `chord_engine.rs:583–593`, READ-ONLY) carries `note: u8`, `velocity: u8`,
`hold_ms: u64`, `offset_ms: u64`. `program_change` is driven once per channel at startup
(`main.rs:300–305`, program `= (i*7)%128`). The new sink consumes exactly the same three-method MIDI
vocabulary `MidiOut` consumes today — it is a behavioral drop-in for `MidiOut` BEHIND the sink trait.
**No `chord_engine.rs` / `mapping_loader.rs` / `assets/mappings.json` change is proposed or needed.**

### 1.3 The OpenCV operations Lane A replaces (file:line → pure-Rust equivalent)

All OpenCV image work lives in three binary-private modules. Lane A replaces the *producers*; the
mirror structs are already clean. The complete inventory:

**`image_source.rs` — acquisition (OpenCV `imgcodecs`/`videoio`):**

| OpenCV op (`image_source.rs`) | Purpose | Pure-Rust equivalent (`image` crate) |
|---|---|---|
| `imgcodecs::imread(path, IMREAD_COLOR)` (:31,:43) | load JPEG/PNG → BGR `Mat` | `image::open(path)?.to_rgb8()` → `RgbImage` (pure Rust; JPEG via `jpeg-decoder`, already a transitive dep) |
| `videoio::VideoCapture` camera grab (:50–67) | one camera frame | **NOT ported in the default path** — camera capture stays OpenCV-only behind the `opencv` flag (no pure-Rust camera in scope). Default path supports file + preselected only. |
| `ImageSource::AIGenerated` (:69–72) | unimplemented placeholder | unchanged (still `unimplemented`) |

**`image_analysis.rs` — feature extraction (OpenCV `imgproc`/`core`):**

| OpenCV op (`image_analysis.rs`) | Feature produced | Pure-Rust equivalent |
|---|---|---|
| `cvt_color(BGR2HSV)` + `split` + `mean_std_dev` on H/S/V (:64–96, :236–262) | `avg_hue` (H·2 → 0..360), `avg_saturation`/`avg_brightness` (·100/255 → 0..100) | per-pixel RGB→HSV over the `image` buffer (`palette` crate OR a hand-rolled `rgb_to_hsv` — trivial, no dep needed); accumulate mean H (as a CIRCULAR mean — see §3.A.4), mean S, mean V |
| `cvt_color(BGR2GRAY)` + `canny(50,150)` + `count_non_zero` / total (:99–111, :265–277) | `edge_density` 0..1 | `imageproc::edges::canny(&gray, 50.0, 150.0)` → count non-zero / total px [verified imageproc exposes `edges::canny`] |
| `laplacian(CV_64F)` + `mean_std_dev`; `var = stddev²` (:114–119, :294–300) | `texture_laplacian_var` | `imageproc::filter::laplacian_filter(&gray)` (or a 3×3 Laplacian `filter3x3`) → compute population variance of the result in pure Rust |
| `sobel(dx)` / `sobel(dy)` + means; orientation bias (:280–292) | `edge_orientation_bias` (LocalFeatures only — NOT in the mirror structs) | `imageproc::gradients::horizontal_sobel` / `vertical_sobel`; mean ratio. **Used only inside `LocalFeatures`, which never crosses to the engine — so Lane A does not need to reproduce it for parity of the engine seam.** Recompute only if a future field surfaces it. |
| `threshold(OTSU)` + `find_contours(RETR_EXTERNAL)` + count; `shape_complexity = count/1000` (:122–139) | `shape_complexity` | Otsu threshold (`imageproc::contrast::otsu_level` + `threshold`) then **connected-components count** (`imageproc::region_labelling::connected_components`) as the contour-count proxy, /1000. HONEST DELTA — see §2. |
| `contour_area` + `arc_length` → circularity (:142–157, :320–335) | `contour_circularity` (LocalFeatures only) | **NOT reproduced** — `contour_circularity` lives only in `LocalFeatures`, never crosses to the engine. Skip. |
| `calc_hist` on H, 8 bins, normalized (:351–412) | `hue_hist: Vec<f32>` (8 bins) | hand-rolled 8-bin histogram over the per-pixel hue (0..180 OpenCV units → 8 bins), normalized by sum. Pure Rust, no dep. |
| `Mat::roi` per scan strip + per section (:208, :484–509) | the per-step / per-section ROI slicing | `image::imageops::crop_imm(&img, x, y, w, h)` (a `SubImage` view; no copy) over the SAME rect geometry main.rs computes |
| `hue_spread = stddev_h / 90.0` (:162) | `hue_spread` | circular stddev of hue (in OpenCV H units) / 90.0 — see §3.A.4 |

**`main.rs` — overlays + window (OpenCV `imgcodecs`/`highgui`):**

| OpenCV op (`main.rs`) | Purpose | Pure-Rust default-path handling |
|---|---|---|
| `draw_scan_bar_overlay_for_rect` + `imwrite` (:257–266) | write first/mid/last overlay PNGs | **Dropped from the default path.** Overlays are a developer convenience, not a sound requirement. The default (pure) path skips overlay writes; the `opencv` flag retains them. (Optional future: a pure `imageproc::drawing` overlay — out of Phase-2 scope.) |
| `highgui::named_window` / `imshow` / `wait_key` (:288, :317–318) | live scan-bar window | **Dropped from the default path** (the assessment §1.3 item 5 calls this a debug window, replaced by the future TUI/GUI). Retained behind the `opencv` flag. The pure default emits the same console progress prints. |

**Net for Lane A:** of the engine-crossing features, only `shape_complexity` (contour-count proxy) has
a real fidelity delta. `edge_orientation_bias` and `contour_circularity` never cross the seam, so the
analyzer does not have to reproduce them for the music to be identical in structure.

### 1.4 The current Cargo feature graph (what Lane C reshapes)

`Cargo.toml` at session start:

- `default = ["opencv", "midir", "image"]` (line 33). All three are `optional = true` (lines 43, 45,
  60). `--no-default-features` yields the pure-Rust lib + modem.
- `[[bin]]` tables with `required-features` (lines 14–27):
  - `audiohax` (`src/main.rs`) → `required-features = ["opencv", "image"]`
  - `make_tiled_payload` / `unpack_tiled_payload` → `required-features = ["image"]`
- The modem bins + `audiohax-tui` are autodiscovered (build under `--no-default-features`).
- Pure deps already present and relevant: `image = "0.24"` (optional, line 60), `serde`, `clap`,
  `ratatui`, `crossterm`. `rustysynth`, `cpal`, `imageproc` are **NOT yet present** (S9 §3.8 listed
  them as Phase-2 additions).

The problem this graph creates: the **default** build requires OpenCV+libclang, so a clean box cannot
`cargo build` at all (confirmed on THIS box — only `--no-default-features` builds; §6).

---

## 2. FEATURE-FIDELITY DELTA — honest characterization (the owner's ear is the gate)

The owner's professional ear is the acceptance gate (post-build, operator-owned — NOT an in-session
gate). This is the honest, per-feature accounting of where pure Rust matches OpenCV and where it does
not, because these values feed mode/dynamics/register in `chord_engine` and therefore the *music*.

| Engine-crossing feature | Parity vs OpenCV | Why / the delta |
|---|---|---|
| `avg_saturation`, `avg_brightness` | **Effectively exact** | Mean of S/V over the same pixels; RGB→HSV is a fixed formula. Sub-1% drift only from OpenCV's internal rounding (8-bit H/S/V vs f32). Music-inert. |
| `avg_hue` | **Near-exact IF circular mean used** | OpenCV's `mean_std_dev` on the H channel is an ARITHMETIC mean of 0..179 values — it mishandles the hue wrap (red ≈ 0 ≈ 180). A pure circular mean (§3.A.4) is arguably *more correct*; it will differ from OpenCV near the red wrap. **Flag for the owner:** images dominated by reds/magentas may pick a different `hue_to_mode` bucket. Mitigation: §3.A.4 offers an arithmetic-mean compatibility mode matching OpenCV bit-for-bit if A/B shows drift. |
| `edge_density` | **Close, not identical** | `imageproc::edges::canny` and OpenCV Canny share the algorithm (Gaussian blur → Sobel → non-max suppression → hysteresis) but differ in default Gaussian kernel/aperture and gradient L1-vs-L2 norm. Edge *counts* will differ by a few percent → `edge_density` shifts slightly → affects `edge_density_to_rhythm` and the dominant-substitution trigger. Small but audible-in-principle. A/B on real images. |
| `texture_laplacian_var` | **Close** | Same 3×3 Laplacian; variance computed identically. Differs only by border handling (`BORDER_DEFAULT` reflect vs imageproc's clamp) and the f64-vs-f32 accumulation. This feeds `texture_to_modal_color` — a coarse heuristic, so drift is well within the mapping's tolerance. |
| `hue_spread` | **Close** (circular) | OpenCV uses `stddev_h/90`. The pure path uses circular stddev /90; near the red wrap it is more correct and will differ. Feeds chord generation `edge_complexity`? No — `hue_spread` is currently carried but the music decision reads it indirectly; low risk. |
| `shape_complexity` | **PROXY — largest honest delta** | OpenCV counts external contours via `find_contours`; the pure path uses connected-component count after Otsu. These are *different* segmentation algorithms — counts will NOT match. But `shape_complexity = count/1000` is explicitly "a crude heuristic" (`image_analysis.rs:139`) and its consumer is a coarse mapping. **This is the one feature where the owner should A/B and, if it matters, we tune the proxy normalization.** |
| `hue_hist` (8-bin) | **Close** | Same binning of the same hue values; differs only by the hue-computation differences above. `hue_hist` is documented "unused by the music decision" (`engine.rs:72`) — carried for fingerprinting only. **Music-inert.** |

**Honest bottom line for the owner:** brightness/saturation (→ dynamics, register) are essentially
exact; the texture/edge metrics are close; the two genuinely-different metrics are `shape_complexity`
(a self-described crude heuristic, music-coarse) and the red-wrap behavior of hue (where the pure path
is arguably *more* correct). The posture (operator decision) is **aim-for-parity default + opt-in
OpenCV escape hatch**: ship pure as default, keep `--features opencv` as the A/B reference and the
fallback if the ear rejects a specific image. The audible test happens post-build and is the owner's.

---

## 3. PROPOSED CHANGES — per file (signatures, types, doc comments only)

### 3.A LANE A — `src/pure_analysis.rs` (NEW, pure-Rust, builds `--no-default-features`)

A NEW library module. Pure Rust over `image` + `imageproc`. It produces `engine::GlobalFeatures` /
`engine::ScanBarFeatures` DIRECTLY and implements `engine::FeatureSource`. It names NO OpenCV type and
NO `image_analysis` type. Gated by a `pure-analysis` feature (default-on — §5) because it pulls
`image`/`imageproc`; the headless lib test compiles it under that feature.

#### 3.A.1 Module-level contract

```rust
//! src/pure_analysis.rs — WS-4 Phase 2 pure-Rust image-feature analyzer (Lane A).
//!
//! Pure-Rust mirror of the OpenCV `image_analysis.rs` extraction, built on the
//! `image` + `imageproc` crates. It produces the engine's image-free mirror
//! structs (`engine::GlobalFeatures` / `engine::ScanBarFeatures`) DIRECTLY and
//! implements `engine::FeatureSource`, so the engine core is byte-unchanged and
//! the OpenCV adapter is no longer on the default build path.
//!
//! Boundary: this module names NO OpenCV type and NO `image_analysis` type. It
//! reads pixels via the pure-Rust `image` crate and computes HSV stats / Canny
//! edge density / Laplacian texture variance / an 8-bin hue histogram / a
//! connected-component shape-complexity proxy. Feature-fidelity deltas vs OpenCV
//! are documented in design-s11 §2 (the owner's ear is the parity gate).
```

#### 3.A.2 Acquisition (replaces `image_source.rs` on the default path)

```rust
/// A loaded image in the pure-Rust path. Owns an 8-bit RGB buffer; no OpenCV `Mat`.
/// theory: the analyzer needs random pixel access + cheap rectangular sub-views
/// for scan strips; `image::RgbImage` gives both (`crop_imm` is a zero-copy view).
pub struct PureImage {
    /// width/height accessible via `image::GenericImageView`.
    inner: image::RgbImage,
}

/// Image source for the pure path. Mirrors the subset of `image_source::ImageSource`
/// that does not require OpenCV. Camera/AI-generated are intentionally absent (the
/// `opencv` flag retains camera capture; AI-gen is still a placeholder elsewhere).
pub enum PureImageSource {
    /// A filename relative to `assets/images/`.
    Preselected(String),
    /// An arbitrary filesystem path.
    UserPath(std::path::PathBuf),
}

/// Load an image from a pure source into a `PureImage` (JPEG/PNG via the `image`
/// crate; `jpeg-decoder` is already a transitive dep). Replaces
/// `image_source::load_image_from_source` on the default path.
pub fn load_pure_image(src: &PureImageSource) -> Result<PureImage, AnalysisError>;
```

#### 3.A.3 The analyzer + its `FeatureSource` impl

The analyzer PRE-EXTRACTS the whole-image features and the per-step `Vec<Vec<ScanBarFeatures>>` using
the SAME scan geometry the OpenCV path uses (`scan_image` rect math, `image_analysis.rs:434–524`), so
the precomputed-source shape is identical to today's `PrecomputedSource` (`main.rs:81–116`). It then
serves them through `FeatureSource` exactly as `PrecomputedSource` does.

```rust
/// Pre-extracted pure-Rust features for one image, ready to serve through
/// `engine::FeatureSource`. Built once from a `PureImage` + the pipeline geometry
/// (instrument count, bar thickness, step count), mirroring the OpenCV
/// `PrecomputedSource` shape so the engine sees an identical feature stream.
pub struct PureAnalysisSource {
    global: engine::GlobalFeatures,
    steps: Vec<Vec<engine::ScanBarFeatures>>,
}

impl PureAnalysisSource {
    /// Extract whole-image + per-step features from `img`. `num_instruments`,
    /// `bar_thickness_frac`, `num_steps`, and `vertical_hint` use the SAME rect
    /// geometry as `image_analysis::scan_image` so the per-step rows line up
    /// 1:1 with the OpenCV path (design-s11 §3.A geometry parity).
    pub fn extract(
        img: &PureImage,
        num_instruments: usize,
        bar_thickness_frac: f32,
        num_steps: usize,
        vertical_hint: Option<bool>,
    ) -> Result<Self, AnalysisError>;
}

impl engine::FeatureSource for PureAnalysisSource {
    fn global_features(&self) -> engine::GlobalFeatures;                                         // returns self.global
    fn scan_bar_features(&self, step_idx: usize, num_instruments: usize) -> Vec<engine::ScanBarFeatures>;
    fn step_count(&self) -> usize;
}
```

#### 3.A.4 The per-feature extraction free functions (pure; the parity-critical kernels)

```rust
/// Whole-image features over the full RGB buffer. Mirrors `analyze_global`
/// (`image_analysis.rs:57`) field-for-field, producing `engine::GlobalFeatures`.
pub fn analyze_global_pure(img: &image::RgbImage) -> Result<engine::GlobalFeatures, AnalysisError>;

/// One scan-bar section's features over a sub-view. Mirrors the per-section work in
/// `scan_image`'s inner loop (`image_analysis.rs:507–521`), producing one
/// `engine::ScanBarFeatures`. `bar_index` is the section's index in the row.
pub fn analyze_section_pure(
    section: &image::SubImage<&image::RgbImage>,
    bar_index: usize,
) -> Result<engine::ScanBarFeatures, AnalysisError>;

/// Mean H (0..360), S (0..100), V (0..100) over a pixel iterator. theory: hue is a
/// CIRCULAR quantity — averaging raw 0..360 values mishandles the red wrap (0≈360).
/// Default uses the circular mean (sum of unit vectors); a `compat_arithmetic` flag
/// reproduces OpenCV's arithmetic mean bit-for-bit for A/B parity (design-s11 §2).
fn hsv_means(pixels: impl Iterator<Item = image::Rgb<u8>>, compat_arithmetic: bool) -> (f32, f32, f32);

/// Circular standard deviation of hue, scaled to match OpenCV's `stddev_h/90`
/// `hue_spread` heuristic (`image_analysis.rs:162`). 0..~1.
fn hue_spread_pure(pixels: impl Iterator<Item = image::Rgb<u8>>) -> f32;

/// Canny edge density 0..1 over a grayscale view. Uses `imageproc::edges::canny`
/// with the same 50/150 hysteresis thresholds as OpenCV (`image_analysis.rs:108`),
/// then non-zero / total. Delta vs OpenCV Canny documented in design-s11 §2.
fn edge_density_pure(gray: &image::GrayImage) -> f32;

/// Population variance of the Laplacian response (focus/texture). Mirrors
/// `image_analysis.rs:114–119`. Uses `imageproc::filter::laplacian_filter`.
fn texture_laplacian_var_pure(gray: &image::GrayImage) -> f32;

/// Connected-component count / 1000 as the `shape_complexity` PROXY for OpenCV's
/// external-contour count (`image_analysis.rs:122–139`). theory: a different
/// segmentation than `find_contours`; counts differ — this is the largest honest
/// fidelity delta (design-s11 §2). Otsu threshold via `imageproc::contrast`, then
/// `imageproc::region_labelling::connected_components`.
fn shape_complexity_pure(gray: &image::GrayImage) -> f32;

/// Normalized 8-bin hue histogram (sum=1) matching `compute_hue_histogram`
/// (`image_analysis.rs:351`). Carried for fidelity; music-inert.
fn hue_histogram_pure(pixels: impl Iterator<Item = image::Rgb<u8>>, bins: usize) -> Vec<f32>;

/// Per-pixel RGB→HSV in OpenCV's output ranges: H 0..360, S 0..100, V 0..100 (the
/// `image_analysis.rs:94–96` conventions). Pure arithmetic; no dependency.
fn rgb_to_hsv(p: image::Rgb<u8>) -> (f32, f32, f32);

/// Error type for the pure analyzer (empty image, decode failure, zero bars).
/// Maps to the same failure cases `image_analysis`'s `anyhow!` guards cover.
#[derive(Debug, thiserror::Error)]
pub enum AnalysisError {
    #[error("empty image passed to {0}")]
    EmptyImage(&'static str),
    #[error("image decode/load failed: {0}")]
    Decode(String),
    #[error("num_bars must be > 0")]
    ZeroBars,
}
```

**Modified items:** none in existing files for Lane A — the analyzer is purely additive. **Removals:**
none (the OpenCV `image_analysis.rs` / `image_source.rs` stay in the tree, reachable behind the
`opencv` flag). **Rationale:** additive parallel implementation keeps the OpenCV reference path intact
for A/B and for the operator's opt-in high-fidelity build, exactly as the posture requires.

### 3.B LANE B — `src/synth_sink.rs` (NEW, pure-Rust, builds `--no-default-features`)

A NEW library module implementing `engine::AudioSink` over an in-process **rustysynth** synthesizer
feeding a **cpal** output stream (recommendation justified in §4). Gated by a `synth` feature
(default-on — §5). The orphan rule is satisfied because the sink struct is LOCAL to this crate (unlike
`MidiOut`, which is bin-private and forces its impl into the `main.rs` adapter — `engine.rs:131`,
S9 D3). The sink may therefore live in the LIBRARY; `main.rs` only constructs it.

#### 3.B.1 Module contract + threading model

```rust
//! src/synth_sink.rs — WS-4 Phase 2 in-process pure-Rust synth sink (Lane B).
//!
//! Implements `engine::AudioSink` by driving a `rustysynth::Synthesizer` (pure
//! Rust SF2 synthesis, no external FluidSynth process) and rendering to a `cpal`
//! output stream (cross-platform: ALSA/PulseAudio on Linux, WASAPI on Windows,
//! CoreAudio on macOS). Replaces "send MIDI bytes to a virtual port a separate
//! FluidSynth reads" with "synthesize and play in-process." `midir` external
//! MIDI-out is retained as the opt-in `midi-out` sink (design-s11 §5).
//!
//! Threading (the load-bearing design): cpal's audio callback runs on a
//! realtime audio thread and MUST NOT block. The engine's `note_on`/`note_off`/
//! `program_change` calls run on the engine/adapter thread. The two are bridged
//! by a lock-free SPSC queue of `MidiCmd`: the AudioSink methods ENQUEUE a
//! command (non-blocking, allocation-free) and return Ok immediately; the audio
//! callback DRAINS the queue at the top of each render block, applies the commands
//! to the `Synthesizer` via `process_midi_message`, then renders one block of
//! samples into the cpal buffer. This keeps the synthesizer owned solely by the
//! audio thread (no lock on the hot path) and makes the AudioSink methods O(1).
```

#### 3.B.2 Public interface

```rust
/// One MIDI command crossing engine thread → audio thread over the SPSC queue.
/// Mirrors the three `AudioSink` methods; `u8` widened to `i32` at apply-time for
/// rustysynth's `process_midi_message(channel, command, data1, data2)` API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MidiCmd {
    NoteOn { channel: u8, note: u8, velocity: u8 },
    NoteOff { channel: u8, note: u8 },
    ProgramChange { channel: u8, program: u8 },
}

/// In-process SoundFont synth sink. Owns the producer end of the SPSC command
/// queue and keeps the cpal `Stream` alive for the sink's lifetime (dropping the
/// stream stops audio). The `Synthesizer` itself lives on the audio thread inside
/// the cpal callback closure, NOT in this struct (no lock on the hot path).
pub struct SynthSink {
    /// Producer end of the engine→audio SPSC queue (e.g. `rtrb::Producer<MidiCmd>`
    /// or `ringbuf`/`crossbeam` SPSC) [VERIFY chosen SPSC crate].
    tx: SpscProducer<MidiCmd>,
    /// Kept alive to keep the audio device open; never touched after construction.
    _stream: cpal::Stream,
    /// The negotiated output sample rate (for diagnostics/logging).
    sample_rate: u32,
}

/// How the sink obtains its SoundFont. Default loads the bundled GM SF2; a path or
/// an in-memory buffer override it (design-s11 §4.4 asset story).
pub enum SoundFontSource<'a> {
    /// The SF2 embedded in the binary via `include_bytes!` (the zero-config default).
    Bundled,
    /// A user-supplied `.sf2` on disk (matches "bring your own SoundFont" today).
    Path(&'a std::path::Path),
    /// An already-loaded SF2 byte buffer.
    Bytes(&'a [u8]),
}

impl SynthSink {
    /// Build the sink: open the default cpal output device, negotiate an f32 output
    /// stream, construct a `rustysynth::Synthesizer` over the chosen SoundFont at
    /// the negotiated sample rate, spawn the audio callback that drains the command
    /// queue and renders, and start the stream. Returns the producer-side handle.
    ///
    /// `font` selects the SoundFont (Bundled GM by default). Errors map into
    /// `engine::AudioSinkError` so the caller speaks one error vocabulary.
    pub fn new(font: SoundFontSource<'_>) -> Result<Self, engine::AudioSinkError>;

    /// Convenience for the zero-config default path: `SynthSink::new(Bundled)`.
    pub fn with_bundled_soundfont() -> Result<Self, engine::AudioSinkError>;

    /// The negotiated output sample rate in Hz (e.g. 44_100 / 48_000).
    pub fn sample_rate(&self) -> u32;
}

impl engine::AudioSink for SynthSink {
    /// Enqueue a note_on (non-blocking). theory: rustysynth velocity 0 is treated
    /// as note_off by GM convention; we forward velocity verbatim and let the
    /// synth honor that, matching the raw-MIDI semantics `MidiOut` had.
    fn note_on(&mut self, channel: u8, note: u8, velocity: u8) -> Result<(), engine::AudioSinkError>;
    /// Enqueue a note_off (non-blocking).
    fn note_off(&mut self, channel: u8, note: u8) -> Result<(), engine::AudioSinkError>;
    /// Enqueue a program_change (non-blocking).
    fn program_change(&mut self, channel: u8, program: u8) -> Result<(), engine::AudioSinkError>;
}
```

#### 3.B.3 The audio-callback render model (how note events drive synth → cpal)

```rust
/// The render closure handed to `cpal::Device::build_output_stream`. Runs on the
/// realtime audio thread. Per invocation it: (1) DRAINS all pending `MidiCmd`s from
/// the SPSC consumer and applies each via `synth.process_midi_message(ch as i32,
/// cmd, data1, data2)` (0x90 note_on / 0x80 note_off / 0xC0 program_change);
/// (2) renders interleaved/deinterleaved f32 via `synth.render(&mut left, &mut
/// right)` sized to the cpal buffer; (3) writes samples into the cpal output slice
/// (interleaving L/R for a 2-channel device, or downmixing for mono).
///
/// theory (latency/quality trade): rustysynth renders in fixed BLOCKS
/// (`Synthesizer::get_block_size()`, 64 samples by default). The callback renders
/// in get_block_size chunks until the cpal buffer is filled, applying queued MIDI
/// AT block boundaries — so event timing is quantized to ~block_size/sample_rate
/// (≈1.45 ms at 44.1 kHz). That is well under one musical step (250 ms default),
/// so quantization is inaudible. No design change needed for the engine's per-event
/// send model (design-s11 §8 decision point).
type RenderCallback = (); // shape only — body is the Implementer's; signature is cpal's closure
```

**Modified items:** none in existing files for Lane B — additive module. **Removals:** none
(`midi_output.rs` + the `impl AudioSink for MidiOut` in `main.rs` stay, reachable behind `midi-out`).

### 3.C `src/main.rs` adapter — `#[cfg]` analyzer/sink selection (the shared touch, with §5)

`main.rs` selects analyzer + sink by feature flag WITHOUT touching `engine.rs`. This is the only
existing source file Lane C edits; it lands after both lanes compile. Shape:

```rust
// Analyzer selection (default = pure; opencv flag = legacy CV path)
#[cfg(feature = "opencv")]
let source = { /* existing PrecomputedSource over analyze_global + scan_image (main.rs:243–271) */ };
#[cfg(all(feature = "pure-analysis", not(feature = "opencv")))]
let source = { /* PureAnalysisSource::extract(&img, instruments, thickness, steps, None)? */ };

// Sink selection (default = in-process synth; midi-out flag = external MIDI port)
#[cfg(all(feature = "synth", not(feature = "midi-out")))]
let mut sink = audiohax::synth_sink::SynthSink::with_bundled_soundfont()?;
#[cfg(feature = "midi-out")]
let mut sink = { /* existing MidiOut::open_first(preferred_ref)? + impl AudioSink for MidiOut */ };
```

The engine driver loop (`engine.decide_step` → jitter/scheduling → `AudioSink::note_*`) is byte-identical
across both sinks; only the concrete `sink`/`source` types change behind the trait objects. The
overlay-write and `highgui` window blocks (`main.rs:250–268, 288, 311–319`) move under
`#[cfg(feature = "opencv")]`. Note: making the **default** `audiohax` bin buildable requires dropping
`required-features = ["opencv","image"]` from its `[[bin]]` table (§5) and `#[cfg]`-gating every
`opencv::` reference in `main.rs` — this is the bulk of Lane C's edit and the reason it is the
serialized shared touch, not a third parallel lane.

---

## 4. SYNTH-ENGINE RECOMMENDATION

The operator found **rustysynth** first and is explicitly unsure it is best. I evaluated rustysynth,
**oxisynth** (a FluidSynth-inspired pure-Rust synth — the closest sound-parity-to-FluidSynth
candidate), and the field. Versions/licenses verified live 2026-06-13.

### 4.1 Scored comparison

| Criterion | **rustysynth** (v1.3.6, MIT) | **oxisynth** (v0.1.0, LGPL-2.1) | Notes |
|---|---|---|---|
| **(a) Sound parity vs CURRENT FluidSynth** | Good. MeltySynth lineage; faithful SF2 GM rendering, clean and well-regarded. NOT bit-identical to FluidSynth (different reverb/chorus, interpolation). | **Best on paper** — explicitly "inspired by FluidSynth," ports its voice/envelope model + has `oxisynth-reverb`/`oxisynth-chorus` mirroring FluidSynth's effects. Closest to the exact timbre the owner hears today. | The owner currently hears FluidSynth + *their own* SF2. Parity is dominated by **using the same SF2** (both load it). The residual delta is the effects/interpolation engine. |
| **(b) SF2 compatibility (same SoundFont as today)** | Yes. `SoundFont::new(&mut impl Read)` loads any standard `.sf2`; the project's GM SoundFont loads unchanged. | Yes. `SoundFont::load` / `Synth::add_font` loads standard `.sf2` from file or `Read`. | **Tie — both load the exact SF2 the owner uses with FluidSynth.** This is the single biggest parity lever and both pass it. |
| **(c) Integration behind `AudioSink` + cpal** | **Cleanest.** API is exactly the shape we need: `Synthesizer::new(&Arc<SoundFont>, &SynthesizerSettings)`, `process_midi_message(ch, cmd, d1, d2)`, `render(&mut [f32] left, &mut [f32] right)`, `get_block_size()`. Maps 1:1 onto our `MidiCmd` queue + cpal callback. **No deps beyond std.** | Good but heavier: `Synth::send_event(MidiEvent)` enum, `Synth::write(...)` for L/R. WASM-first design; pulls `oxisynth-chorus`/`oxisynth-reverb` deps. | rustysynth's `render(left, right)` + integer `process_midi_message` is a textbook fit for the §3.B model; oxisynth's `MidiEvent` enum adds a translation layer. |
| **(d) Maintenance / activity / license** | **MIT** (permissive — no distribution constraints on a bundled binary). Active: 17 releases, 236 commits, current v1.3.6. | **LGPL-2.1** — copyleft; static-linking a copyleft lib into a distributed binary raises relink-obligation questions for the owner's distribution. v0.1.0, fewer releases, lower cadence. | **License is a real differentiator.** MIT is friction-free for the §6 release-binary plan; LGPL-2.1 static-linked needs legal care before shipping prebuilt binaries. |
| **(e) Cross-platform (Linux + Windows) cleanliness** | **Excellent** — std-only, no native deps; compiles anywhere cargo runs. Pairs with cpal for the OS audio backend. | Good — pure Rust, WASM-proven, so Linux/Windows are fine; the extra effect crates are also pure Rust. | Both are clean; rustysynth's zero-dep footprint is marginally simpler to vet. |

### 4.2 Recommendation: **rustysynth + cpal**

**One-line reason:** rustysynth is the cleanest `AudioSink`+cpal fit (`render(left,right)` +
integer `process_midi_message`), is **MIT** (no distribution friction for §6 release binaries), is
std-only and actively maintained at v1.3.6 — and the dominant parity lever (loading the owner's *own*
SF2) is satisfied by BOTH, so oxisynth's FluidSynth-lineage timbre edge does not outweigh its LGPL-2.1
copyleft + v0.1.0 maturity + extra integration layer.

**The honest sound trade for the owner:** neither pure-Rust synth is bit-identical to FluidSynth.
Because the owner keeps using **the same SoundFont**, the *patches/instruments* are identical — what
differs is the synthesis engine's interpolation and its reverb/chorus tails. rustysynth (MeltySynth
lineage) is a clean, slightly drier/more-neutral renderer than FluidSynth's. If, on the post-build ear
test, the owner finds rustysynth's reverb/voicing materially different from the FluidSynth sound they
are attached to, **oxisynth is the documented fallback** (its FluidSynth-derived effects are the
closer timbral match) — and because both sit behind the same `AudioSink` trait, swapping is a sink
substitution, not an engine change. The recommended path: ship rustysynth as the zero-config default,
keep the `midi-out`→external-FluidSynth flag as the exact-parity reference, and hold oxisynth as a
named escape hatch if the ear demands it.

**Audio-output crate:** **cpal** (the assessment default; verified pure-Rust, ALSA/PulseAudio on
Linux, WASAPI on Windows, CoreAudio on macOS, `build_output_stream` with F32). MSRV for the ALSA/WASAPI
backends is Rust 1.82; THIS box is 1.96 — fine.

**Note→stream model:** `AudioSink` methods enqueue `MidiCmd` onto a lock-free SPSC queue; the cpal
audio callback drains the queue at block boundaries, applies them via `process_midi_message`, and
renders `get_block_size()`-sample blocks (64 samples ≈ 1.45 ms @ 44.1 kHz) into the cpal buffer until
filled. Event timing is quantized to the block boundary — inaudible against a 250 ms step (§3.B.3).
The `Synthesizer` is owned solely by the audio thread; no lock on the hot path.

---

## 5. CARGO FEATURE RESHAPE (the serialized shared touch)

The reshape flips the **default** to pure-Rust while keeping every capability reachable behind a flag.
No capability is deleted.

### 5.1 The target feature graph

```toml
[features]
# DEFAULT is now PURE RUST: pure-Rust image analysis + in-process synth. A clean
# Linux/Windows box runs `cargo build`/`cargo run` and gets audible output with NO
# system libraries, NO libclang, NO external FluidSynth, NO virtual MIDI port.
default = ["pure-analysis", "synth"]

# Pure-Rust image-feature analyzer (Lane A). Pulls `image` + `imageproc`.
pure-analysis = ["dep:image", "dep:imageproc"]

# In-process pure-Rust SoundFont synth sink (Lane B). Pulls `rustysynth` + `cpal`.
synth = ["dep:rustysynth", "dep:cpal"]

# OPT-IN high-fidelity / legacy path: OpenCV image analysis + camera + highgui
# window + overlay PNGs. Reintroduces libclang/system-OpenCV (the A/B reference and
# the operator's escape hatch). Pulls `image` too (the OpenCV bins use it).
opencv = ["dep:opencv", "dep:image"]

# OPT-IN: route NoteEvents to an EXTERNAL MIDI port / DAW / FluidSynth instead of
# the bundled synth (the exact-FluidSynth-parity reference, and "use my own synth").
midi-out = ["dep:midir"]
```

```toml
[dependencies]
# ── existing (unchanged) ──
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
clap = { version = "4", features = ["derive"] }
toml = "0.8"
directories = "5"
rand = "0.8"
anyhow = "1.0"
hound = "3.5.1"
hex = "0.4"
flate2 = "1.0"
crc32fast = "1.3"
thiserror = "1.0"
aes-gcm = "0.10"
rand_core = "0.6"
rand_chacha = "0.3"
twoway = "0.2"
reed-solomon-erasure = "6"
ratatui = "0.29"
crossterm = "0.28"

# ── now optional, gated by `opencv` / `midi-out` (were in the old default) ──
opencv = { version = "0.95.1", optional = true }
midir  = { version = "0.8",    optional = true }
image  = { version = "0.24",   optional = true }   # used by BOTH pure-analysis and opencv

# ── NEW Phase-2 deps (pure-Rust; build --no-default-features-clean per backend) ──
imageproc = { version = "0.25", optional = true }  # pure-Rust CV ops (Canny/Sobel/Laplacian/CC) [VERIFY 0.25 + image 0.24 compat]
rustysynth = { version = "1.3", optional = true }  # pure-Rust SF2 synth (MIT, std-only)          [verified 1.3.6 2026-06-13]
cpal       = { version = "0.15", optional = true } # cross-platform audio output (ALSA/WASAPI/CoreAudio) [VERIFY 0.15 latest]
# optional SPSC ring for the engine→audio bridge (or use crossbeam, already absent):
rtrb = { version = "0.3", optional = true }        # lock-free SPSC for the synth callback        [VERIFY]; alt: ringbuf
```

> **[VERIFY] dep wiring:** if `rtrb` is chosen for the SPSC bridge it must be folded into the `synth`
> feature (`synth = ["dep:rustysynth", "dep:cpal", "dep:rtrb"]`). `image` is shared by `pure-analysis`
> and `opencv`, so it stays a single optional dep enabled by either feature.

### 5.2 The `[[bin]]` table reshape

```toml
# The main app bin: required-features CHANGES from ["opencv","image"] to NONE, so the
# DEFAULT feature set builds it. main.rs `#[cfg]`-gates every opencv:: reference (§3.C).
[[bin]]
name = "audiohax"
path = "src/main.rs"
# (required-features removed — builds on the default pure-Rust feature set)

# The image-payload bins still need the `image` decoder; keep their required-features
# but point at the feature that now ENABLES image in the default set.
[[bin]]
name = "make_tiled_payload"
path = "src/bin/make_tiled_payload.rs"
required-features = ["image"]      # image is enabled by `pure-analysis` (default) → builds by default now
[[bin]]
name = "unpack_tiled_payload"
path = "src/bin/unpack_tiled_payload.rs"
required-features = ["image"]
```

> Because `pure-analysis` (default) enables `image`, the two tiled-payload bins now build under the
> default set too — a strict improvement. The modem bins + `audiohax-tui` remain autodiscovered and
> still build under `--no-default-features`.

### 5.3 `src/lib.rs` additions

```rust
// WS-4 Phase 2 (S11) — pure-Rust analyzer + in-process synth sink. Feature-gated so
// the bare `--no-default-features` lib (music + modem) stays dependency-free.
#[cfg(feature = "pure-analysis")]
pub mod pure_analysis;   // Lane A — image+imageproc; implements engine::FeatureSource
#[cfg(feature = "synth")]
pub mod synth_sink;      // Lane B — rustysynth+cpal; implements engine::AudioSink
```

### 5.4 Why this satisfies the posture

- **Default = pure Rust + audible:** `default = ["pure-analysis", "synth"]` → no OpenCV, no libclang,
  no external synth, no virtual MIDI port. One-command build + sound on a clean box.
- **Nothing deleted:** `opencv` (CV + camera + window + overlays) and `midi-out` (external
  FluidSynth/DAW, the exact-parity reference) are reachable flags. `--features opencv` gives the A/B
  reference; `--features "opencv midi-out"` reproduces today's exact build.
- **`engine.rs` untouched:** selection is entirely in `Cargo.toml` + `main.rs` `#[cfg]`. The traits
  the analyzer/sink implement already exist.

---

## 6. CROSS-PLATFORM BUILD/RUN/PACKAGING SLICE

### 6.1 SoundFont asset bundling + runtime loading

Today there is **no `.sf2` in the repo** (confirmed: `find . -iname '*.sf2'` → none); the owner hands
their own GM SoundFont to FluidSynth (README §"Start FluidSynth"). For a zero-config audible default we
must bundle a SoundFont:

- **Where it lives:** `assets/soundfonts/default.sf2` (NEW asset). Choose a small, liberally-licensed
  GM SoundFont — e.g. a GM bank in the **public domain / CC0 / MIT** class so it can ship in a
  distributed binary. **[VERIFY license of the chosen SF2 before bundling — this is a packaging legal
  gate, not a code gate.]** A small (~2–8 MB) GM set keeps the binary reasonable; the owner's larger
  preferred SF2 remains usable via `SoundFontSource::Path` / a `--soundfont` flag.
- **How it's found at runtime:** embed via `include_bytes!("../assets/soundfonts/default.sf2")` →
  `SoundFontSource::Bundled` loads it from the in-memory buffer with `SoundFont::new(&mut &BYTES[..])`.
  **No filesystem lookup, no CWD assumption** — this is the relocatable-asset fix the assessment §1.3
  item 4 calls for, applied to the SF2. (`mappings.json` embedding is S9/Phase-1 scope; the SF2 is the
  Phase-2 addition.)
- **Override path:** a future `--soundfont <PATH>` CLI flag (the `cli.rs` grammar already has room;
  wiring it is a small follow-on) maps to `SoundFontSource::Path`, preserving "bring your own
  SoundFont." Out of strict Phase-2 scope but the sink API (`SoundFontSource`) already supports it.

### 6.2 cpal backend per OS (default path)

| OS | cpal host | System requirement | Build-time native dep? |
|---|---|---|---|
| Linux | ALSA (default) or PulseAudio/JACK | `libasound2` present at RUNTIME (almost universal). Build links `alsa-sys` — **note: this is a surviving build consideration; see §6.4.** | `alsa-sys` (pkg-config to libasound). On a truly bare box `libasound2-dev` is needed to BUILD cpal's ALSA backend. |
| Windows | WASAPI (default) | Built into Windows; no install, no DLL to ship. | None — WASAPI via the `windows`/`winapi` bindings, no external SDK. |
| macOS | CoreAudio | Built in. | None. |

### 6.3 The build/run smoke story

- **Default pure build (clean box):**
  - Linux: `sudo apt install libasound2-dev` (the ONE remaining build prerequisite — see §6.4) →
    `cargo run -- play` → image scanned by the pure analyzer → rustysynth synthesizes → cpal/ALSA emits
    sound. No OpenCV, no FluidSynth, no virtual port.
  - Windows: `cargo run -- play` → same, via WASAPI. **Zero system setup.**
- **Opt-in OpenCV reference:** `cargo run --features opencv -- play` (needs system OpenCV + libclang,
  per README) — the A/B parity reference.
- **Opt-in external MIDI:** `cargo run --features midi-out -- play` (needs the virtual port +
  FluidSynth, exactly as today) — exact-FluidSynth-parity reference.
- **Headless lib (CI / this box):** `cargo test --lib --no-default-features` (70 tests green today)
  stays green — neither lane touches the bare lib.

### 6.4 Surviving native/system deps in the DEFAULT path — honest audit

The brief asks for NONE in the default path and to flag any that survive. The honest result:

- **OpenCV / libclang / cmake / pkg-config-for-OpenCV: GONE** from the default path. ✅ (the whole point.)
- **External FluidSynth + virtual MIDI port: GONE** from the default path. ✅
- **cpal's ALSA backend on Linux is the ONE survivor.** cpal links `alsa-sys`, which needs
  `libasound2` (runtime) and `libasound2-dev`+pkg-config (build) on Linux. This is **not** an
  OpenCV-class blocker (libasound2 is present on essentially every Linux desktop, and the dev header is
  a one-line apt install vs. OpenCV's multi-SDK dance), but it is **not literally zero** on Linux.
  - **Windows + macOS default path: genuinely zero system/build deps** (WASAPI/CoreAudio are OS
    built-ins). ✅
  - **Mitigation options for Linux (operator decision, out of strict scope):** (a) accept
    `libasound2-dev` as the single documented Linux build prereq (recommended — it is ubiquitous);
    (b) enable cpal's JACK/PulseAudio hosts; (c) `pipewire-alsa` shims. The honest statement to the
    owner: **the default path removes the hard blockers entirely; on Linux one ubiquitous audio dev
    header remains, which is categorically different from the OpenCV problem.**

### 6.5 What THIS box can do once OpenCV is off the default path

Verified live (cargo 1.96.0): `cargo test --lib --no-default-features` → **70 passed, 0 failed**. The
bare lib + modem + engine + CLI + TUI already build and test headlessly here. Once Phase 2 lands:

- Lane A (`pure_analysis`) and Lane B (`synth_sink`) are **pure Rust and COMPILE on this box** under
  their features (`cargo build --features pure-analysis` / `--features synth`) — no OpenCV/libclang
  needed to typecheck and unit-test them. ✅
- The full **default `cargo build`** (`pure-analysis + synth`) should compile here, since both deps
  are pure Rust + the only system touchpoint is cpal's ALSA backend (needs `libasound2-dev`, a
  one-line install — vs. OpenCV which CANNOT be satisfied here). **This is the unblock:** for the
  first time the *default* app build is achievable on an ordinary box.
- **Build-gated (NOT doable here without audio hardware/output):** the actual *audible* ear test and
  cpal device negotiation against real hardware. That is the operator-owned, post-build gate the
  posture explicitly excludes from the in-session acceptance.

---

## 7. DATA FLOW DIAGRAM

```
  ╔═══ DEFAULT (pure-Rust) PATH — no system libs except cpal/ALSA on Linux ═══════════════════════════╗
  ║                                                                                                    ║
  ║  image file ──► pure_analysis::load_pure_image ──► PureImage (image::RgbImage, pure Rust)          ║
  ║                          │                                                                         ║
  ║                          ▼  PureAnalysisSource::extract  (image + imageproc; §3.A.4 kernels)        ║
  ║          engine::GlobalFeatures + Vec<Vec<engine::ScanBarFeatures>]  (PLAIN f32 — no Mat, no OpenCV)║
  ║                          │  impl engine::FeatureSource                                              ║
  ╚══════════════════════════┼═════════════════════════════════════════════════════════════════════════╝
                             │  &S: FeatureSource
   ════════ HEADLESS / PURE-RUST LIBRARY BOUNDARY (src/engine.rs — BYTE-UNCHANGED) ════════════════════
                             │
  ┌─────────────────────────▼─── src/engine.rs  PipelineEngine (unchanged) ───────────────────────────┐
  │  set_features_global ─► chord_engine (mode/progression/plan)                                       │
  │  decide_step / tick ─► decide_instrument_action ─► chord_engine::realize_step ─► Vec<NoteEvent>     │
  │                                         │ AudioSink::note_on / note_off / program_change            │
  └─────────────────────────────────────────┼──────────────────────────────────────────────────────────┘
                                            │  &mut A: AudioSink
   ════════════════════════════════════════ │ ══════════════════════════════════════════════════════════
                                            ▼
  ╔═══ DEFAULT sink: src/synth_sink.rs  SynthSink (impl AudioSink) ════════════════════════════════════╗
  ║   note_on/off/program_change ─► enqueue MidiCmd ─► [lock-free SPSC] ─► cpal audio-thread callback   ║
  ║        callback: drain MidiCmd ─► rustysynth::Synthesizer::process_midi_message ─► render(L,R) ─────►║──► OS audio
  ║        SoundFont: include_bytes! assets/soundfonts/default.sf2 (Bundled)         cpal: ALSA/WASAPI/CoreAudio
  ╚════════════════════════════════════════════════════════════════════════════════════════════════════╝

  ┄┄┄ OPT-IN PATHS (flags; nothing deleted) ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
  --features opencv :  Mat ─► image_analysis::analyze_global/scan_image ─► PrecomputedSource (impl FeatureSource)
                       + highgui window + imwrite overlays   [the A/B reference + camera capture]
  --features midi-out: NoteEvents ─► impl AudioSink for MidiOut ─► midir ─► virtual port ─► external FluidSynth
                       [exact-FluidSynth-parity reference + "use my own DAW/synth"]
```

Key: the only things crossing the library boundary are the two trait objects (`FeatureSource`,
`AudioSink`) and plain value structs. No `Mat`, no `cpal`/`rustysynth` type, ever reaches `engine.rs`.

---

## 8. MIGRATION PATH + TWO-LANE FAN-OUT

### 8.1 Independent vs. coordinated

- **Lane A (`src/pure_analysis.rs`) and Lane B (`src/synth_sink.rs`) are FULLY INDEPENDENT** — disjoint
  new files, no shared symbol, no ordering dependency. They can be built concurrently by two
  Implementers.
- **Lane C (the Cargo reshape + `main.rs` `#[cfg]` wiring) is the SERIALIZED shared touch** — it edits
  `Cargo.toml` and `src/main.rs`, and it should land **after** both A and B compile (so the default
  build it enables actually has both halves). Until C lands, A and B compile under their own
  feature flags (`cargo build --features pure-analysis` / `--features synth`) without C.

### 8.2 Explicit file-ownership (confirmed disjoint)

| Lane | OWNS (writes) | READS (does not modify) | MUST NOT TOUCH |
|---|---|---|---|
| **A — analyzer** | `src/pure_analysis.rs` (NEW) | `engine::{GlobalFeatures, ScanBarFeatures, FeatureSource}`; `image`/`imageproc` APIs; the `scan_image` rect geometry (for parity) | `engine.rs`, `chord_engine.rs`, `mapping_loader.rs`, `assets/mappings.json`, `modem.rs`, `src/bin/modem_*`, `synth_sink.rs` |
| **B — synth sink** | `src/synth_sink.rs` (NEW) | `engine::{AudioSink, AudioSinkError}`; `rustysynth`/`cpal` APIs; `chord_engine::NoteEvent` shape (read-only, to know the payload) | `engine.rs`, `chord_engine.rs`, `mapping_loader.rs`, `assets/mappings.json`, `modem.rs`, `src/bin/modem_*`, `pure_analysis.rs` |
| **C — feature reshape (serialized)** | `Cargo.toml`, `src/main.rs` (`#[cfg]` selection + gate opencv refs), `src/lib.rs` (module decls), `assets/soundfonts/default.sf2` (NEW) | both new modules' public APIs | `engine.rs` decision logic, `chord_engine.rs`, `mapping_loader.rs`, `assets/mappings.json`, `modem.rs`, `src/bin/modem_*` |

**Confirmed disjoint:** A owns only `pure_analysis.rs`; B owns only `synth_sink.rs`; no overlap. Both
stay OFF `chord_engine.rs` / `mapping_loader.rs` / `assets/mappings.json` / `modem.rs` /
`src/bin/modem_*` / `engine.rs` decision logic. The ONLY convergence is Lane C's `lib.rs` +
`Cargo.toml` + `main.rs`, which is explicitly serialized after A and B.

### 8.3 Landing order (no broken intermediate state)

1. **Lane A + Lane B in parallel.** Each adds its module behind a (not-yet-default) feature; each is
   unit-testable headlessly (`cargo test --features pure-analysis`, `--features synth`). No existing
   test changes; both are additive. The default build is still OpenCV (unchanged) at this point.
2. **Test nets (parallel with A/B, additive only):**
   - Lane A regression net: feed the pure analyzer the repo's `assets/images/example.jpg` and assert
     each `engine::GlobalFeatures` field is within a documented tolerance band of hand-frozen expected
     values (the A/B parity check the §2 deltas are measured against). Headless — `image` decodes
     JPEG without OpenCV.
   - Lane B net: a `SynthSink` unit test that constructs over a tiny embedded SF2, enqueues a
     note_on/note_off/program_change, and asserts the SPSC queue + the AudioSink methods are O(1) and
     return Ok (audio output itself is build/hardware-gated — assert the command path, not the sound).
3. **Lane C (serialized): flip the default.** Edit `Cargo.toml` (`default = ["pure-analysis","synth"]`,
   move opencv/midir/image to opt-in, drop the main bin's `required-features`), add the SF2 asset,
   `#[cfg]`-gate `main.rs`. After this, `cargo build` (default) is the pure path; `cargo build
   --features opencv` is the legacy path. Verify `cargo test --lib --no-default-features` still 70/70.
4. **Post-build (operator-owned, NOT in-session):** the audible A/B ear test on a build-capable box;
   tune `shape_complexity`/hue-wrap if the ear flags drift; decide whether oxisynth's timbre is wanted.

---

## 9. RISKS AND TRADE-OFFS

1. **Feature-parity drift (Lane A) — the music can shift.** Canny/Laplacian/contour reimplementations
   are not byte-identical to OpenCV (§2). `shape_complexity` (connected-components vs `find_contours`)
   is the largest delta; the hue circular-mean changes red-wrap behavior. *Mitigation:* the `opencv`
   flag is retained as the A/B reference; the Lane A test net pins each field to a tolerance band; the
   owner's post-build ear is the final gate; the posture explicitly allows the OpenCV escape hatch.

2. **Sound delta (Lane B) — pure synth ≠ FluidSynth.** Same SoundFont → same instruments, but
   rustysynth's interpolation/reverb differ from FluidSynth's (§4.2). *Mitigation:* same SF2 keeps
   patches identical; `midi-out`→external-FluidSynth is the exact-parity reference; oxisynth is the
   documented timbre-closer fallback behind the same trait.

3. **SoundFont packaging — license + size.** Bundling a `.sf2` via `include_bytes!` adds MB to the
   binary and requires a redistributable-licensed GM SoundFont. *Mitigation:* pick a CC0/MIT/PD GM bank
   ([VERIFY license — a packaging legal gate]); keep it small; the owner's larger SF2 stays usable via
   `SoundFontSource::Path`.

4. **cpal/audio backend variance (Windows specifics + Linux ALSA survivor).** Device/sample-rate
   negotiation differs per backend; Linux still needs `libasound2-dev` to build cpal's ALSA host
   (§6.4) — the one surviving Linux build dep. Windows/macOS default path is genuinely dep-free.
   *Mitigation:* document `libasound2-dev` as the single Linux build prereq; validate cpal output on a
   real Linux *and* Windows box early (build-gated); optionally enable cpal's JACK/PulseAudio hosts.

5. **`main.rs` `#[cfg]` churn (Lane C).** Dropping the main bin's `required-features` means EVERY
   `opencv::` reference in `main.rs` (the window, `imwrite`, `imshow`, `wait_key`, `Mat` types) must be
   `#[cfg(feature="opencv")]`-gated or the default build breaks. This is the riskiest mechanical edit.
   *Mitigation:* it is serialized (Lane C, after A/B), isolated to one file, and verified by a clean
   `cargo build` (default) + `cargo build --features opencv` both compiling. Lane C should compile-check
   BOTH feature configurations before merge.

6. **SPSC bridge choice (Lane B).** The lock-free engine→audio queue needs a real-time-safe SPSC
   (`rtrb`/`ringbuf`/`crossbeam`). A wrong choice (a locking queue) would risk audio-thread blocking.
   *Mitigation:* the §3.B design mandates a lock-free SPSC and an allocation-free hot path; the
   Implementer's crate choice is [VERIFY]-flagged, and the unit test asserts the AudioSink methods do
   not block.

### 9.1 DECISION POINT for the lead — does the engine seam need to change? (answer: NO)

The one place a seam change could be *argued*: the engine drives the sink with discrete
`note_on`/`note_off` calls (`engine.rs:400–401`), but an in-process synth fundamentally renders
**blocks of audio**, not discrete events. One could imagine adding a `render(&mut [f32])` /
`tick_audio()` method to `AudioSink` so the engine pulls audio.

**I recommend NOT changing the seam, for these reasons:**
- The block-rendering is **entirely internal to `SynthSink`** — its cpal callback owns the render loop
  on the audio thread; the engine never needs to pull samples. The existing three-method MIDI
  vocabulary is sufficient and is exactly what `MidiOut` already uses.
- Event-timing quantization to the synth block boundary (≈1.45 ms) is **inaudible** against the 250 ms
  musical step (§3.B.3), so there is no quality reason to give the engine sample-level control.
- Adding a render method to `AudioSink` would force `MidiOut` (the external-MIDI sink) to implement a
  meaningless audio-pull method, and would drag audio-buffer concerns into the headless engine — the
  exact boundary violation the S9 seam was built to prevent.

**Therefore the engine seam stays byte-unchanged (the binding constraint), and this is flagged here as
the considered-and-rejected decision point rather than a silent assumption.** If the lead disagrees and
wants sample-accurate event timing (e.g. for a future sample-locked game-sync feature), that is a
separate, later seam-evolution conversation — out of Phase-2 scope and not needed for the parity goal.

---

## Appendix — Grounding references (file:line, verified at session start)

- `src/engine.rs`: `FeatureSource` 84–96; `AudioSinkError` 102 (+`new` 118, `msg` 123); `AudioSink`
  132–139; `GlobalFeatures` 33–50; `ScanBarFeatures` 59–74; `PipelineEngine::tick` sink drive 397–404;
  `decide_step` 442–461. **Must end Phase 2 byte-unchanged.**
- `src/main.rs`: `impl AudioSink for MidiOut` 63–75; `PrecomputedSource` + `FeatureSource` impl 81–116;
  boundary copies `to_eng_global`/`to_eng_scanbar` 119–143; OpenCV acquisition+extraction 238–248;
  overlay `imwrite` 250–268; `highgui` window 288, 311–319; program_change init 300–305; driver loop
  310–371.
- `src/image_analysis.rs`: `GlobalFeatures` 11–20; `ScanBarFeatures` 22–31; HSV means 64–96; Canny edge
  density 99–111; Laplacian var 114–119; Otsu+contours `shape_complexity` 122–139; Sobel orientation
  280–292 (LocalFeatures only); `compute_hue_histogram` 351–412; `scan_image` rect geometry 421–528.
- `src/image_source.rs`: `imread` 31/43; camera `VideoCapture` 50–67; `AIGenerated` placeholder 69–72.
- `src/midi_output.rs`: `MidiOut` raw MIDI sends 33–49 (the sink-method shape the new sink mirrors).
- `src/chord_engine.rs` (READ-ONLY): `NoteEvent` 583–593 (note/velocity/hold_ms/offset_ms — the sink
  payload); `realize_step` 642.
- `Cargo.toml`: `default = ["opencv","midir","image"]` 33; optional opencv/midir/image 43/45/60;
  `[[bin]]` required-features 14–27.
- `assets/`: `mappings.json`, `images/{example.jpg,AudioHaxImg1-3.jpg,Lena.png}`. **No `.sf2` present**
  → §6.1 bundles `assets/soundfonts/default.sf2` (NEW).
- Crate facts (verified live 2026-06-13): **rustysynth** v1.3.6, MIT, std-only,
  `Synthesizer::new(&Arc<SoundFont>, &SynthesizerSettings)` / `process_midi_message(i32,i32,i32,i32)` /
  `render(&mut [f32], &mut [f32])` / `get_block_size()`; `SoundFont::new(&mut impl Read)`. **oxisynth**
  v0.1.0, LGPL-2.1, FluidSynth-inspired, `Synth::new(SynthDescriptor)`/`add_font`/`send_event(MidiEvent)`/
  `write`, +`oxisynth-chorus`/`oxisynth-reverb`. **cpal** pure-Rust, ALSA/WASAPI/CoreAudio,
  `build_output_stream` F32, MSRV 1.82 (box is 1.96). **imageproc** [VERIFY 0.25 / image 0.24 compat]:
  `edges::canny`, `filter::laplacian_filter`, `gradients::*sobel`, `region_labelling::connected_components`,
  `contrast::otsu_level`.
</content>
</invoke>

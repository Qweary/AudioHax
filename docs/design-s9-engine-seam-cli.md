# Design S9 — WS-4 Phase 1: the `engine.rs` shared-core seam + clap CLI / modem-bin unification

Status: DESIGN / SPECIFICATION ONLY. No source modified. This document is the precise spec the
Test Engineer writes RED tests against and the Implementer codes against. Author role: Rust
Architect (Swaram). Grounded against the working tree at `21fd304` (`src/main.rs`, `src/lib.rs`,
`src/image_analysis.rs`, `src/image_source.rs`, `src/midi_output.rs`, `src/chord_engine.rs`,
`src/mapping_loader.rs`, `src/bin/*`, `Cargo.toml`) and reconciled against the two prior design
docs: `AudioHax/docs/assessment-ws4-ux-crossplatform.md` (§4.1/§4.2 — the governing WS-4 plan)
and `Swaram/docs/interactive-architecture.md` (the prior `PipelineEngine` sketch).

> Convention: Rust signatures give the SHAPE of each seam. **No implementation bodies are
> written.** Framework/version claims that cannot be verified against the tree are marked
> **[VERIFY]**. The two prior docs PARTIALLY conflict on the engine's input side; §3.1 resolves
> that conflict and §6/§9 record exactly what was overridden and why.

---

## 0. Executive summary of the resolution (read first)

The two prior docs agree the linchpin of WS-4 is extracting a `PipelineEngine` from `main.rs` so
CLI / TUI / GUI are thin drivers over one core. They CONFLICT on the input boundary:

- `interactive-architecture.md` puts OpenCV `Mat` **directly inside the engine**
  (`image_buffer: Arc<RwLock<Mat>>`, `update_image(&mut self, image: Mat)`). That makes the engine
  un-buildable headless and violates the hard constraint. **Overridden.**
- `assessment-ws4-ux-crossplatform.md` §4.1 fixes the OUTPUT side with an image-free seam
  (`EngineSnapshot`/`EngineCommand`/`EngineObserver`/`AudioSink` over the already-pure
  `chord_engine::PerfFeatures`/`NoteEvent`), but does not fully pin the INPUT side (how
  `GlobalFeatures`/`ScanBarFeatures` enter the engine without pixels).

**The resolution (this doc's central decision):** the engine receives only PLAIN feature structs,
never pixels. We introduce a small set of **lib-local, image-free mirror types**
(`engine::GlobalFeatures`, `engine::ScanBarFeatures` — value-identical `f32`/`Vec<f32>` copies of
the OpenCV-module structs) and a **`FeatureSource` trait** plus an `update_features(...)` /
`update_image_features(...)` analogue that takes those plain structs. All OpenCV feature
extraction stays in the `main.rs` adapter; the adapter converts `image_analysis::ScanBarFeatures`
→ `engine::ScanBarFeatures` at the boundary (a trivial field copy, the same pattern S6 already
uses for `ScanBarFeatures` → `chord_engine::PerfFeatures`). `Mat`, `imshow`, MIDI port opening,
and the highgui window all stay in the adapter. The engine becomes a pure-Rust crate citizen that
builds and unit-tests under `--no-default-features`.

`AudioSink` lives in the lib; `midi_output::MidiOut` CANNOT receive `impl AudioSink for MidiOut` in
`engine.rs` (orphan rule + `MidiOut` is a binary-private type) — so the impl lives in the
**`main.rs` adapter** as a thin newtype/inline impl; a future `rustysynth`+`cpal` sink implements
the same trait from wherever it is added (lib or adapter). See §3.3.

Concurrency: **lift the existing `Barrier`-worker pool OUT of the engine core; the engine core is
single-threaded.** The pool is an optimization, not a correctness requirement (both prior docs say
so), and it carries the OpenCV `highgui` overlay calls today — keeping it single-threaded makes the
core trivially headless-testable and makes regression-equivalence a pure-function property. See §3.4.

The clap parser + arg structs live in a NEW **lib** module `src/cli.rs` (headless-unit-testable),
NOT in `main.rs`. The four modem bins are unified via a SHARED clap parser in `cli.rs` that each bin
calls (the bins keep their own thin `main`), AND the same grammar is reachable as an `audiohax modem
…` subcommand on the main app. See §3.6 / §3.7.

---

## 1. Current-state analysis

### 1.1 Crate layout and the headless boundary (confirmed)

- `src/lib.rs` exports exactly three pure modules: `modem`, `chord_engine`, `mapping_loader`. These
  build/test under `cargo test --lib --no-default-features` (no system libs).
- `image_analysis`, `image_source`, `midi_output` are **binary-private** (`mod …;` at the top of
  `src/main.rs`, lines 2–4). They are OpenCV/midir-coupled and are NOT in the library. **Therefore a
  lib `engine.rs` cannot name `image_analysis::ScanBarFeatures` or `midi_output::MidiOut`** — this is
  the structural fact that forces the mirror-type + trait-impl-in-adapter design below.
- `mapping_loader` IS in the lib and is pure (serde over JSON, `HashMap`s; `MappingTable`,
  `lookup_range_map`). The engine MAY hold a `MappingTable` and call `lookup_range_map` directly.
- Existing nets: 72 `#[test]`/`#[cfg(test)]` sites in `src/` + `tests/` — comprising the
  42 unit (lib) + 17 roundtrip (`tests/modem_roundtrip.rs`) + 10 realair (`tests/modem_realair.rs`)
  the kickoff names, plus `tests/qg_probe_band_isolation.rs`. **The migration must keep all green and
  may only add to them.**

### 1.2 Feature structs are Mat-free (CONFIRMED — the cross-boundary currency)

`src/image_analysis.rs:11–46`:

- `GlobalFeatures` (11–20): `avg_hue, avg_saturation, avg_brightness, edge_density, hue_spread,
  texture_laplacian_var, shape_complexity, aspect_ratio` — **all `f32`.** No `Mat`.
- `ScanBarFeatures` (22–31): `bar_index: usize`, `avg_hue, avg_saturation, avg_brightness,
  edge_density, texture_laplacian_var: f32`, `hue_hist: Vec<f32>`. **No `Mat`.**
- `LocalFeatures` (35–46): all `f32`. **No `Mat`.** (Not crossed into the engine; listed for
  completeness.)

These are produced by `analyze_global`, `analyze_scan_bar`, `scan_image` — all of which take `&Mat`
and live in the OpenCV-coupled module. The **structs themselves are clean**; only their *producers*
touch OpenCV. That is exactly why the engine can consume value-copies of them with zero OpenCV
linkage.

`chord_engine::PerfFeatures` (`chord_engine.rs:565–573`) is the already-pure music-domain projection:
`saturation, brightness, edge_density: f32`. `main.rs`'s `worker_decide_action` (60–94) already
builds one from a `ScanBarFeatures` by plain field copy (no cast — units already match). This is the
proof-of-pattern for the engine's boundary conversion.

### 1.3 `MidiOut` method shape (CONFIRMED — to design `AudioSink`)

`src/midi_output.rs`, `pub struct MidiOut { conn: MidiOutputConnection }`:

- `open_first(port_name_hint: Option<&str>) -> Result<Self, Box<dyn Error>>` (11)
- `program_change(&mut self, channel: u8, program: u8) -> Result<(), Box<dyn Error>>` (33)
- `note_on(&mut self, channel: u8, note: u8, velocity: u8) -> Result<(), Box<dyn Error>>` (39)
- `note_off(&mut self, channel: u8, note: u8) -> Result<(), Box<dyn Error>>` (45)
- `play_chord_arpeggio(...)` (52) — unused by the batch path; ignore.

**Note the error type is `Box<dyn std::error::Error>`, NOT `anyhow::Result`.** The assessment §4.1
sketch typed `AudioSink` methods as `anyhow::Result<()>`. `anyhow::Error: From<Box<dyn Error +
Send + Sync>>` but `MidiOut`'s `Box<dyn Error>` is **not `Send + Sync`**, so it does not auto-convert
into `anyhow::Error` cleanly across threads. §3.3 resolves the trait's error type to avoid forcing a
change to `midi_output.rs` (which is out of scope).

### 1.4 The orchestration in `main.rs` to extract (file:line)

`fn main()` (323–468) is the batch pipeline. The extractable engine work vs. adapter-only work:

| `main.rs` region | What it does | Destination |
|---|---|---|
| 325 `load_mappings` | read `assets/mappings.json` | **adapter** constructs, hands `MappingTable` to engine |
| 329–341 CLI parse + prints | `parse_cli_arg`, banners | → **`cli.rs`** (parse) + adapter (prints) |
| 343–357 image source select + `load_image_from_source` | OpenCV `imread` → `Mat` | **adapter** (OpenCV) |
| 359–365 `analyze_global` / `analyze_scan_bar` | OpenCV feature extraction | **adapter** (OpenCV); result fed to engine as plain structs |
| 367–384 mode pick + `pick_progression`/`generate_chords`/`plan_phrases` | pure music orchestration | **engine** (calls `chord_engine` + `lookup_range_map`) |
| 386–388 `scan_image` | OpenCV per-step `Vec<Vec<ScanBarFeatures>>` | **adapter** (OpenCV); converted to plain structs, fed to engine |
| 390–444 overlay write (first/mid/last) | OpenCV `imwrite` | **adapter** (OpenCV) |
| 447–462 `play_scanned_steps_concurrent` | barrier pool + MIDI + highgui | **split**: per-step note decisions → engine `tick`; MIDI send + highgui → adapter (§3.4) |
| 99–321 `play_scanned_steps_concurrent` | the pool/coordinator | dissolved — see §3.4 |
| 60–94 `worker_decide_action` | feature→`PerfFeatures`→`realize_step`→tuple | **engine** (the per-instrument decision, the regression-equivalence kernel) |
| 22–37 `InstrumentAction`/`ScheduledEvent` | playback value types | engine emits `EngineTickOutput` (note decisions); adapter owns wall-clock `ScheduledEvent` scheduling |

The decision logic that MUST move into the engine and produce identical output is **`worker_decide_action`** (the `ScanBarFeatures` → `PerfFeatures` → `realize_step` → `(note,vel,hold,offset)` pipeline) and the **mode/progression/plan derivation** (367–384). Everything that touches `Mat`, `imshow`, `imwrite`, `MidiOut`, jitter RNG timing, and `Instant`-based scheduling stays in the adapter.

### 1.5 The CLI today (the thing being replaced)

- `parse_cli_arg<T: FromStr>` (39–45): silent `unwrap_or(default)`, no validation, no help.
- Flags: `--instruments`(4), `--thickness`(0.10), `--steps`(40), `--ms-per-step`(250),
  `--jitter-percent`(15.0).
- The **overloaded `"play"` token** (345–354 selects the example image; 447 toggles playback). This
  is the §1.2-of-assessment defect to resolve into a real subcommand.
- Four modem bins each hand-roll `print_usage` + positional `while i < args.len()` parsing with
  divergent conventions: `modem_encode` (out.wav + input + flags, presets, simulate), `modem_decode`
  (in.wav + optional out_basename + flags), `channel_sim` (two positionals + `--mode`), `make_packetized`
  (input + out + flags). Subtle inconsistencies: `--flip_prob` (channel_sim, underscore) vs `--sim-flip`
  (modem_encode, hyphen); `--rs-shard-size` present in encode/make_packetized, absent in decode/channel_sim.

---

## 2. Decisions at a glance (each expanded + justified in §3 / §6)

| # | Decision | Chosen | Rejected alternative |
|---|---|---|---|
| D1 | Engine input boundary | `FeatureSource` trait + plain mirror feature structs; adapter does OpenCV→struct conversion | `Arc<RwLock<Mat>>` in the engine (interactive-architecture.md) — violates headless constraint |
| D2 | `AudioSink` error type | `Result<(), AudioSinkError>` with `AudioSinkError(Box<dyn Error + Send + Sync>)`; impls map their own errors in | `anyhow::Result` (assessment §4.1) — MidiOut's `Box<dyn Error>` isn't `Send+Sync`, forces a midi_output.rs edit (out of scope) |
| D3 | `impl AudioSink for MidiOut` placement | in the `main.rs` adapter (orphan rule + MidiOut is bin-private) | in `engine.rs` — impossible: lib can't see `MidiOut`, and even a re-export would be a foreign-type/foreign-trait orphan violation |
| D4 | Engine concurrency | single-threaded engine core; barrier pool dissolved (or re-homed in adapter as an optional optimization) | lift the `Barrier` pool into the engine — keeps OpenCV-free std threads but defeats simple headless determinism and complicates regression-equivalence |
| D5 | clap parser placement | NEW lib module `src/cli.rs` (headless-unit-testable) | in `main.rs` — un-buildable here, untestable headless |
| D6 | Modem CLI unification | SHARED clap parser structs in `cli.rs`; each existing bin keeps a thin `main` that calls them; ALSO surfaced as `audiohax modem …` subcommand | (a) brand-new single `audiohax` bin only, deleting the four bins — breaks `--no-default-features` autodiscovery + existing invocation; (b) leave bins hand-rolled — fails the unification mandate |
| D7 | Config file | `audiohax.toml` via `serde` + `toml`; located via `directories` [VERIFY]; precedence flags > file > built-in defaults | env-var config or no file — misses the performer "repeatable setup" goal |
| D8 | Engine emits decisions, adapter schedules time | `tick()` returns note decisions (offset/hold in ms, no wall clock); adapter applies jitter + `Instant` scheduling + MIDI | engine owns the `Instant`/`thread::sleep` loop — drags timing+RNG into the headless core, hurts test determinism |

---

## 3. Proposed changes — per file

### 3.1 NEW `src/engine.rs` (in the LIBRARY) — the shared core

**Module purpose.** Pure-Rust orchestration over plain feature data + two traits (`FeatureSource`
input, `AudioSink` output). Holds music state (mappings, derived mode/progression/plan, scan
position, phrase state), no OpenCV/midir/image type anywhere in its public surface or internal
state. Builds and unit-tests under `--no-default-features`.

**Why mirror feature types (D1).** A lib module cannot reference `image_analysis::*` (bin-private,
OpenCV-coupled). Re-homing `image_analysis.rs` into the lib is out of scope (it's OpenCV; the lib
must stay buildable headless). So `engine.rs` declares its own image-free `GlobalFeatures` /
`ScanBarFeatures` — value-identical to the analysis structs — and the adapter copies field-by-field
at the boundary. This is the SAME move S6 already made for `PerfFeatures`. The mirror is cheap
(plain `f32`), keeps the boundary explicit, and means the engine never grows an OpenCV dependency.

**What moves in (from `main.rs`):**
- The mode-derivation + progression/plan build (367–384) → `PipelineEngine::set_features_global` +
  internal `rederive_harmony`.
- The per-instrument decision kernel (`worker_decide_action`, 60–94) → private
  `engine::decide_instrument_action`, called by `tick`.
- The `InstrumentAction` value type → re-expressed as the public `engine::InstrumentDecision`
  (same `(note,vel,hold_ms,offset_ms)` payload), surfaced in `EngineTickOutput`.

**What stays out (adapter keeps):** all `Mat`, `imread`, `analyze_*`, `scan_image`, `imshow`,
`imwrite`, `MidiOut`, `Instant`/`thread::sleep`, the jitter RNG.

**Public surface (signatures only):**

```rust
//! src/engine.rs — pure-Rust shared core. Builds under `--no-default-features`.
//! NO OpenCV/image/midir type appears in any signature or field below.

use crate::chord_engine::{self, NoteEvent, PerfFeatures, PhrasePosition, StepPlan};
use crate::mapping_loader::MappingTable;

/// Image-free mirror of `image_analysis::GlobalFeatures` (all plain `f32`). The
/// `main.rs` adapter performs the OpenCV extraction and copies fields into this
/// at the boundary; the engine never sees a `Mat`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlobalFeatures {
    pub avg_hue: f32,
    pub avg_saturation: f32,
    pub avg_brightness: f32,
    pub edge_density: f32,
    pub hue_spread: f32,
    pub texture_laplacian_var: f32,
    pub shape_complexity: f32,
    pub aspect_ratio: f32,
}

/// Image-free mirror of `image_analysis::ScanBarFeatures`. `hue_hist` is carried
/// for fidelity/future use; the engine's music decision reads only the scalars it
/// projects into `PerfFeatures` (saturation/brightness/edge_density).
#[derive(Debug, Clone, PartialEq)]
pub struct ScanBarFeatures {
    pub bar_index: usize,
    pub avg_hue: f32,
    pub avg_saturation: f32,
    pub avg_brightness: f32,
    pub edge_density: f32,
    pub texture_laplacian_var: f32,
    pub hue_hist: Vec<f32>,
}

/// The input seam (D1). The adapter implements this (OpenCV-backed) and the engine
/// pulls plain features through it. Headless tests implement it over canned data.
///
/// theory of the boundary: the engine OBSERVES image features; it never holds or
/// requests pixels. A `FeatureSource` is whatever can yield (a) one whole-image
/// `GlobalFeatures` and (b) the per-instrument `ScanBarFeatures` row for a given
/// scan step. The OpenCV adapter pre-extracts these; a future pure-Rust analyzer
/// or a TUI/GUI live source implements the same trait.
pub trait FeatureSource {
    /// Whole-image features for the current image. Re-queried only when the adapter
    /// signals the image changed (the engine applies the §interactive hysteresis).
    fn global_features(&self) -> GlobalFeatures;

    /// Per-instrument scan-bar features for scan step `step_idx`. `num_instruments`
    /// is the ensemble width the engine was configured with. Returns one row
    /// (length == num_instruments). The adapter that wraps today's precomputed
    /// `Vec<Vec<ScanBarFeatures>>` simply indexes it; a live source extracts on the fly.
    fn scan_bar_features(&self, step_idx: usize, num_instruments: usize) -> Vec<ScanBarFeatures>;

    /// Total scan steps available (the batch step count, or a live source's notion).
    fn step_count(&self) -> usize;
}

/// Error wrapper so the trait does not force `anyhow` and does not require editing
/// `midi_output.rs` (whose `Box<dyn Error>` is not `Send + Sync`). Each sink maps
/// its own error into this at the impl site.
#[derive(Debug)]
pub struct AudioSinkError(pub Box<dyn std::error::Error + Send + Sync + 'static>);

/// The output seam (D2/D3). The engine emits `NoteEvent`s to whatever sink the
/// front-end wired. `midi_output::MidiOut` satisfies this via an impl in the
/// adapter; a future `rustysynth`+`cpal` sink implements the same trait.
pub trait AudioSink {
    /// Send one realized note as a note_on (the sink owns note_off scheduling, or
    /// the adapter pairs on/off — see §3.4). `channel` is the instrument's MIDI channel.
    fn note_on(&mut self, channel: u8, note: u8, velocity: u8) -> Result<(), AudioSinkError>;
    /// Send a note_off for `(channel, note)`.
    fn note_off(&mut self, channel: u8, note: u8) -> Result<(), AudioSinkError>;
    /// Set the program (patch) for a channel.
    fn program_change(&mut self, channel: u8, program: u8) -> Result<(), AudioSinkError>;
}

/// One instrument's realized decision for one step — the image-free, time-relative
/// playback unit. Same payload as today's `InstrumentAction.events` tuples
/// `(note, velocity, hold_ms, offset_ms)`, but typed. The adapter applies jitter +
/// wall-clock scheduling; the engine makes only the (deterministic) musical choice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstrumentDecision {
    pub channel: u8,
    pub events: Vec<NoteEvent>,
}

/// What `tick()` produced this step — the note decisions for every instrument plus
/// the scan position, for the adapter to schedule/visualize. Pure data, no `Mat`.
#[derive(Debug, Clone, PartialEq)]
pub struct EngineTickOutput {
    pub step_index: usize,
    pub scan_position: f32, // 0.0..=1.0 along the scan axis
    pub decisions: Vec<InstrumentDecision>,
    pub phrase: PhrasePosition,
    pub mode: String,
}

/// Static-image / front-end-agnostic configuration. Mirrors today's CLI knobs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EngineConfig {
    pub num_instruments: usize,
    pub ms_per_step: u64,
    pub bar_thickness_frac: f32, // carried for the adapter's overlay geometry; unused by music
    pub root_midi: u8,           // 60 today (main.rs:378)
}

/// External interaction (mouse/game/network) — the `inject_event` channel. Defined
/// now per the prior sketch; the batch CLI uses only `Tick`/`Seek`. Image-free:
/// an image change is signaled, the new features come through the `FeatureSource`.
#[derive(Debug, Clone, PartialEq)]
pub enum InteractionEvent {
    ImageChanged,              // adapter has swapped the FeatureSource's backing image
    Seek(f32),                 // jump scan position to 0.0..=1.0
    SetConfig(EngineConfig),
}

/// Front-end → engine control vocabulary (assessment §4.1 `EngineCommand`).
#[derive(Debug, Clone, PartialEq)]
pub enum EngineCommand {
    SetInstruments(usize),
    SetMsPerStep(u64),
    SetThickness(f32),
    Play,
    Pause,
    Stop,
    Seek(f32),
}

/// The GUI/TUI-facing snapshot (assessment §4.1 + interactive-architecture
/// `current_state()`). Plain values only; no OpenCV/image type, no engine internals.
#[derive(Debug, Clone, PartialEq)]
pub struct EngineSnapshot {
    pub scan_position: f32,
    pub step_index: usize,
    pub global: GlobalFeatures,
    pub current_step: PerfFeatures,
    pub last_notes: Vec<NoteEvent>,
    pub mode: String,
    pub phrase: PhrasePosition,
}

/// Per-tick observer (assessment §4.1 `EngineObserver`). Front-ends are pure observers.
pub trait EngineObserver {
    fn on_tick(&mut self, snapshot: &EngineSnapshot);
}

/// The central pipeline engine. Owns scan + musical state + the derived plan; does
/// NOT own the image (a `FeatureSource` supplies features) and does NOT own the
/// audio transport (the caller passes an `AudioSink` into `tick`). Single-threaded
/// core (D4); the adapter may parallelize feature extraction outside it.
pub struct PipelineEngine {
    mappings: MappingTable,
    config: EngineConfig,
    plan: Vec<StepPlan>,        // derived from the latest GlobalFeatures
    mode: String,
    last_global: Option<GlobalFeatures>,
    scan_position: f32,
    step_index: usize,
    last_notes: Vec<NoteEvent>,
    last_step_perf: PerfFeatures,
}

impl PipelineEngine {
    /// Construct from mappings + config. The harmony plan is empty until the first
    /// `set_features_global` (the adapter supplies global features before the run,
    /// exactly as `main.rs` derives mode/progression/plan before playback).
    pub fn new(mappings: MappingTable, config: EngineConfig) -> Self;

    /// Feed whole-image features. (Re)derives mode → progression → chords → phrase
    /// plan via `chord_engine` IFF the features changed beyond the hysteresis
    /// threshold (batch path: called once, always re-derives). Replaces main.rs
    /// 367–384. Pure: calls `lookup_range_map` + `pick_progression`/`generate_chords`/
    /// `plan_phrases` through their existing public APIs.
    pub fn set_features_global(&mut self, global: &GlobalFeatures);

    /// The image-free analogue of the prior `update_image(Mat)` (D1): hand the engine
    /// the freshly extracted features instead of pixels. Equivalent to
    /// `set_features_global` + an `ImageChanged` mark; named to mirror the prior seam
    /// for front-end familiarity.
    pub fn update_image_features(&mut self, global: &GlobalFeatures);

    /// Process one step: pull this step's per-instrument `ScanBarFeatures` from
    /// `source`, run the per-instrument decision kernel (the moved
    /// `worker_decide_action`), send note_on/note_off to `sink`, advance the scan
    /// position, and return the decisions for visualization. Replaces the inner body
    /// of `play_scanned_steps_concurrent`. Deterministic given (plan, features,
    /// config) — the jitter/wall-clock scheduling is the adapter's, NOT the engine's.
    pub fn tick<S: FeatureSource, A: AudioSink>(
        &mut self,
        source: &S,
        sink: &mut A,
    ) -> Result<EngineTickOutput, AudioSinkError>;

    /// Decide all instruments for `step_idx` WITHOUT sending audio — the pure kernel
    /// the regression-equivalence net pins (§5). `tick` is this plus the sink sends +
    /// position advance. Headless tests call this directly.
    pub fn decide_step<S: FeatureSource>(
        &self,
        source: &S,
        step_idx: usize,
    ) -> Vec<InstrumentDecision>;

    /// Apply a control command (front-end → engine). Mutates config / transport state.
    pub fn command(&mut self, cmd: EngineCommand);

    /// Inject an external interaction event (mouse/game/network or batch Seek).
    pub fn inject_event(&mut self, event: InteractionEvent);

    /// Current state for the GUI/TUI (the prior `current_state()`).
    pub fn current_state(&self) -> EngineSnapshot;
}

/// The per-instrument decision kernel, lifted verbatim (behavior-preserving) from
/// `main.rs::worker_decide_action`. Pure, deterministic; the regression-equivalence
/// anchor. Projects `ScanBarFeatures` → `PerfFeatures` and calls `realize_step`.
pub fn decide_instrument_action(
    f: &ScanBarFeatures,
    inst_idx: usize,
    step_idx: usize,
    num_instruments: usize,
    plan: &[StepPlan],
    ms_per_step: u64,
) -> InstrumentDecision;
```

Notes on fidelity to today's behavior:
- `decide_instrument_action` reproduces `worker_decide_action` exactly: empty-plan → no events;
  `plan[step_idx % plan.len()]`; `PerfFeatures { saturation: f.avg_saturation, brightness:
  f.avg_brightness, edge_density: f.edge_density }`; `realize_step(...)` → `NoteEvent`s → events. The
  `_edge_threshold` arg is dropped (already unused per the S6 doc-comment, main.rs:67).
- The MIDI channel assignment `(inst_idx % 16)` and the per-channel program `((i*7)%128)` (main.rs
  130–134, 247) move into the engine's `tick`/`InstrumentDecision.channel` so the adapter just sends
  what it's told — but the engine does NOT open the port (adapter) and does NOT apply jitter (adapter).

### 3.2 `src/main.rs` — slimmed to a thin OpenCV/audio adapter

After extraction `main.rs` (still `required-features = ["opencv","image"]`, un-buildable here)
contains ONLY:
- arg parsing via `audiohax::cli` (§3.6) → an `EngineConfig` + a resolved `ImageSource` + a
  `play`/no-play decision.
- OpenCV image acquisition (`load_image_from_source` → `Mat`), `analyze_global`/`scan_image`, overlay
  `imwrite`, the `highgui` window + `imshow`/`wait_key` — ALL retained here.
- A concrete `FeatureSource` impl (the **`PrecomputedSource` adapter type**) wrapping the
  OpenCV-extracted `GlobalFeatures` + `Vec<Vec<ScanBarFeatures>>`, converting `image_analysis::*`
  → `engine::*` by field copy.
- A concrete `AudioSink` impl for `MidiOut` (§3.3) — lives here by the orphan rule.
- The driver loop: build `PipelineEngine`, `set_features_global`, then for each step call
  `engine.tick(&source, &mut sink)`, take the returned `EngineTickOutput`, apply **jitter + the
  `Instant`-based scheduling** (the adapter owns timing/RNG — D8), draw the overlay for this step.

What is GONE from `main.rs`: `worker_decide_action`, `play_scanned_steps_concurrent`, the
`Barrier`/worker pool, `InstrumentAction`, the mode/progression/plan derivation. (The jitter +
`ScheduledEvent` time-ordering + `thread::sleep` execution stay — they are wall-clock playback, not a
musical decision.)

### 3.3 `AudioSink` + the orphan-rule placement (D2/D3 — confirmed answer)

**Confirmed: `impl AudioSink for MidiOut` CANNOT live in `engine.rs`.** Two independent reasons:
1. `MidiOut` is a **binary-private** type (`mod midi_output;` in `main.rs`); the library literally
   cannot name it.
2. Even if `midi_output` were re-exported from the lib, `AudioSink` (local trait) for `MidiOut`
   (foreign-to-engine type) is fine ONLY where one of them is local to the impl's crate/module under
   the orphan rule — and `engine.rs` is in the lib while `MidiOut` would still be a separate module;
   more decisively, reason (1) already forbids it. **The impl belongs in the `main.rs` adapter**,
   where `MidiOut` IS visible and `AudioSink` is in-scope via `use audiohax::engine::AudioSink;`
   (the trait is a local-trait import; implementing a crate-imported trait for a crate-local type is
   orphan-legal). The error mapping handles the `Box<dyn Error>` → `AudioSinkError` conversion in the
   impl body (out of scope to write, but the shape):

```rust
// in src/main.rs (the adapter) — NOT in engine.rs
use audiohax::engine::{AudioSink, AudioSinkError};
use midi_output::MidiOut;
impl AudioSink for MidiOut {
    fn note_on(&mut self, ch: u8, note: u8, vel: u8) -> Result<(), AudioSinkError>;
    fn note_off(&mut self, ch: u8, note: u8) -> Result<(), AudioSinkError>;
    fn program_change(&mut self, ch: u8, prog: u8) -> Result<(), AudioSinkError>;
}
```

A **future `rustysynth`+`cpal` sink** (Phase 2, out of scope) implements the same `AudioSink`
trait from wherever it lives (most naturally a new lib module behind a `synth` feature, since it is
pure-Rust). Because the engine depends only on the trait, that drop-in needs zero engine change.

The error type `AudioSinkError(Box<dyn Error + Send + Sync>)` (D2) is deliberately NOT `anyhow`:
the assessment's `anyhow::Result` would require `MidiOut`'s error to be `Send + Sync` (it is not) or
an `.map_err` at every call anyway — so we standardize on an explicit wrapper the impl maps into,
and `midi_output.rs` is never touched.

### 3.4 Concurrency model (D4 — single-threaded core; rationale + rejected alt)

**Decision: the `PipelineEngine` core is single-threaded.** `tick`/`decide_step` compute all
instruments' decisions sequentially.

Rationale:
- Both prior docs explicitly state the worker pool is an **optimization, not a correctness
  requirement** (interactive-architecture.md "Concurrency Model Change": *"the worker parallelism is
  an optimization, not a correctness requirement"*; *"Profile first… the parallelism overhead isn't
  worth it for 4-8 instruments"*). `analyze_scan_bar` on a small region is microseconds; the per-step
  decision is a handful of `realize_step` calls.
- The current `Barrier` pool's worker bodies do **nothing OpenCV** (they call `worker_decide_action`,
  pure) — BUT the coordinator interleaves `imshow`/`wait_key` and `MidiOut` sends with the barrier
  handshake. Keeping that structure would force either OpenCV/midir into the engine (forbidden) or a
  callback dance. Dissolving it makes the engine a clean pure function.
- **Regression-equivalence becomes a pure-function property.** With a single-threaded core, "same
  static image → same note decisions" is `decide_step(source, k)` equality, trivially unit-testable
  headless. The barrier pool's output is already order-independent per step (the coordinator sorts
  `ScheduledEvent`s by time AFTER collecting all workers), so single-threaded collection yields the
  identical multiset of decisions — no behavioral change.

Rejected alternative: **lift the `Barrier` worker-per-instrument pool into the engine.** It is
OpenCV-free std-thread code, so it *could* live in a headless lib. Rejected because (a) it adds
nondeterministic scheduling to a core whose entire value is deterministic testability; (b) it
provides no measurable benefit at 4–8 instruments / 250 ms steps; (c) it complicates the
regression net (you'd assert on a collected-and-sorted multiset rather than a pure return value).
If profiling ever shows a real need, the adapter may re-introduce a thread pool that calls the
engine's pure `decide_instrument_action` per instrument in parallel and merges results — parallelism
re-homed in the adapter, the engine core staying pure. (Documented as the escape hatch, not built.)

### 3.5 `src/lib.rs` — module declarations

```rust
// src/lib.rs (additions)
pub mod modem;
pub mod chord_engine;
pub mod mapping_loader;
pub mod engine;   // NEW — pure-Rust shared core (builds under --no-default-features)
pub mod cli;      // NEW — clap arg structs + parsers (headless-unit-testable)
```

Both new modules are pure-Rust and MUST compile under `--no-default-features`. `cli` pulls in `clap`
unconditionally (clap is pure-Rust; safe — see §3.8). Neither references `opencv`/`midir`/`image`.

### 3.6 NEW `src/cli.rs` (in the LIBRARY) — clap derive structs + parsers

Placement is mandatory in the lib so the parser is headless-unit-testable (kickoff BUILD note).
`main.rs` and each modem bin call into `cli.rs`; the bins keep a thin `main`.

```rust
//! src/cli.rs — clap (derive) definitions for the whole app. Pure-Rust; builds
//! under `--no-default-features`. Front-ends/bins parse via these; no OpenCV here.

use clap::{Args, Parser, Subcommand, ValueEnum};

/// Top-level `audiohax` CLI. Resolves the overloaded `"play"` token (assessment
/// §1.2) into a real `play` subcommand with a positional `<IMAGE>`.
#[derive(Debug, Parser, PartialEq)]
#[command(name = "audiohax", version, about = "Image-to-music + MFSK modem toolkit")]
pub struct Cli {
    /// Optional config file; defaults to the platform config dir's audiohax.toml.
    #[arg(long, global = true, value_name = "FILE")]
    pub config: Option<std::path::PathBuf>,
    /// Emit machine-readable JSON instead of human prose where supported.
    #[arg(long, global = true)]
    pub json: bool,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand, PartialEq)]
pub enum Command {
    /// Scan an image and play it to a MIDI/audio sink.
    Play(PlayArgs),
    /// Scan an image and write overlays / analysis without playing.
    Render(RenderArgs),
    /// Analyze an image and print/JSON-dump its features.
    Analyze(AnalyzeArgs),
    /// MFSK data modem operations (unifies the four legacy modem bins).
    #[command(subcommand)]
    Modem(ModemCommand),
}

/// Shared image-pipeline knobs (replaces the five `parse_cli_arg` flags). Validated
/// ranges replace today's silent `unwrap_or(default)`.
#[derive(Debug, Args, PartialEq)]
pub struct PipelineArgs {
    #[arg(long, default_value_t = 4, value_parser = clap::value_parser!(usize))]
    pub instruments: usize,
    #[arg(long, default_value_t = 0.10)]
    pub thickness: f32,
    #[arg(long, default_value_t = 40)]
    pub steps: usize,
    #[arg(long = "ms-per-step", default_value_t = 250)]
    pub ms_per_step: u64,
    #[arg(long = "jitter-percent", default_value_t = 15.0)]
    pub jitter_percent: f32,
}

#[derive(Debug, Args, PartialEq)]
pub struct PlayArgs {
    /// Image path (resolves the old magic positional). Omit to use the example image.
    pub image: Option<std::path::PathBuf>,
    /// MIDI port name hint (else $AUDIOHAX_MIDI_PORT or "AudioHaxOut").
    #[arg(long)]
    pub midi_port: Option<String>,
    #[command(flatten)]
    pub pipeline: PipelineArgs,
}

#[derive(Debug, Args, PartialEq)]
pub struct RenderArgs {
    pub image: Option<std::path::PathBuf>,
    #[command(flatten)]
    pub pipeline: PipelineArgs,
}

#[derive(Debug, Args, PartialEq)]
pub struct AnalyzeArgs {
    pub image: Option<std::path::PathBuf>,
    #[command(flatten)]
    pub pipeline: PipelineArgs,
}

/// Build an `engine::EngineConfig` from validated pipeline args. Pure; the unit
/// test that asserts defaults match today's values (4 / 0.10 / 40 / 250 / 15.0)
/// pins the no-regression contract.
pub fn pipeline_to_engine_config(args: &PipelineArgs) -> crate::engine::EngineConfig;
```

`pipeline_to_engine_config` is the single seam where CLI → engine config is validated; it is the
headless test surface for "the new CLI produces the same config the old flags did."

### 3.7 The unified modem CLI grammar (D6) — `cli.rs` + the four bins

```rust
// src/cli.rs (modem grammar)

#[derive(Debug, Subcommand, PartialEq)]
pub enum ModemCommand {
    /// Encode a file to a WAV (legacy modem_encode).
    Encode(ModemEncodeArgs),
    /// Decode a WAV back to a file (legacy modem_decode).
    Decode(ModemDecodeArgs),
    /// Apply a channel-simulation model to bytes/samples (legacy channel_sim).
    ChannelSim(ChannelSimArgs),
    /// Build a packetized byte stream from a file (legacy make_packetized).
    MakePacketized(MakePacketizedArgs),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ModemPreset { Fast, Balanced, Robust }

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ChannelMode { Bitflip, Byteburst, Packet, Acoustic }

/// Shared RS / packetization knobs, harmonizing the divergent legacy flag names
/// (e.g. `--flip_prob` vs `--sim-flip`) into ONE spelling. Optional so each bin
/// uses the subset it needs.
#[derive(Debug, Args, PartialEq)]
pub struct ModemFecArgs {
    #[arg(long)] pub rs_data: Option<usize>,
    #[arg(long)] pub rs_parity: Option<usize>,
    #[arg(long, default_value_t = 128)] pub rs_shard_size: usize,
    #[arg(long, default_value_t = 200)] pub pkt_size: usize,
    #[arg(long, default_value_t = 3)]   pub repeats: usize,
    #[arg(long)] pub no_interleave: bool,
}

#[derive(Debug, Args, PartialEq)]
pub struct ModemEncodeArgs {
    pub out_wav: std::path::PathBuf,
    pub input: std::path::PathBuf,
    #[arg(long)] pub compress: bool,
    #[arg(long, value_name = "KEYHEX")] pub encrypt: Option<String>,
    #[arg(long)] pub channels: Option<usize>,
    #[arg(long)] pub symbol_ms: Option<f32>,
    #[arg(long)] pub mtones: Option<usize>,
    #[arg(long, value_enum)] pub preset: Option<ModemPreset>,
    #[arg(long)] pub estimate_duration: bool,
    #[arg(long)] pub simulate: bool,
    #[arg(long)] pub sim_flip: Option<f64>,
    #[arg(long)] pub sim_burst_prob: Option<f64>,
    #[arg(long)] pub sim_burst_len: Option<usize>,
    #[arg(long)] pub sim_out: Option<std::path::PathBuf>,
    #[command(flatten)] pub fec: ModemFecArgs,
}

#[derive(Debug, Args, PartialEq)]
pub struct ModemDecodeArgs {
    pub in_wav: std::path::PathBuf,
    pub out_basename: Option<String>,
    #[arg(long, value_name = "KEYHEX")] pub decrypt: Option<String>,
    #[arg(long)] pub channels: Option<usize>,
    #[arg(long)] pub mtones: Option<usize>,
    #[arg(long)] pub symbol_ms: Option<f32>,
    #[arg(long)] pub repeats: Option<usize>,
    #[arg(long)] pub rs_data: Option<usize>,
    #[arg(long)] pub rs_parity: Option<usize>,
}

#[derive(Debug, Args, PartialEq)]
pub struct ChannelSimArgs {
    pub in_bytes: std::path::PathBuf,
    pub out_sim: std::path::PathBuf,
    #[arg(long, value_enum, default_value_t = ChannelMode::Bitflip)] pub mode: ChannelMode,
    #[arg(long, default_value_t = 0.0)] pub flip_prob: f64,
    #[arg(long, default_value_t = 0.0)] pub burst_prob: f64,
    #[arg(long, default_value_t = 16)]  pub burst_len: usize,
    #[arg(long, default_value_t = 128)] pub packet_size: usize,
    #[arg(long)] pub repeats: Option<usize>,
    #[arg(long)] pub rs_data: Option<usize>,
    #[arg(long)] pub rs_parity: Option<usize>,
    // acoustic-mode knobs (S7)
    #[arg(long, default_value_t = 0)]   pub acoustic_seed: u64,
    #[arg(long, default_value_t = 0)]   pub start_offset: usize,
    #[arg(long, default_value_t = 0.0)] pub clock_ppm: f64,
    #[arg(long, default_value_t = 0.0)] pub freq_offset: f64,
    #[arg(long, default_value_t = 0)]   pub echo_delay: usize,
    #[arg(long, default_value_t = 0.0)] pub echo_gain: f64,
    #[arg(long, default_value_t = 0.0)] pub jitter: f64,
}

#[derive(Debug, Args, PartialEq)]
pub struct MakePacketizedArgs {
    pub input: std::path::PathBuf,
    pub out_packetized: std::path::PathBuf,
    #[arg(long)] pub compress: bool,
    #[command(flatten)] pub fec: ModemFecArgs,
}

/// Each legacy bin parses ITS OWN subcommand-args struct standalone (so the bin's
/// historical CLI shape keeps working under --no-default-features) via these:
pub fn parse_modem_encode() -> ModemEncodeArgs;       // bin/modem_encode.rs main calls this
pub fn parse_modem_decode() -> ModemDecodeArgs;       // bin/modem_decode.rs
pub fn parse_channel_sim() -> ChannelSimArgs;         // bin/channel_sim.rs
pub fn parse_make_packetized() -> MakePacketizedArgs; // bin/make_packetized.rs
```

**Unification decision (D6), stated:** the unified modem CLI is **both** a shared parser AND a
subcommand surface. Concretely:
- The arg *grammar* (the `Modem*Args` structs) is defined ONCE in `cli.rs`.
- The main `audiohax` app exposes it as `audiohax modem encode|decode|channel-sim|make-packetized`
  (the `Command::Modem(ModemCommand)` arm).
- Each existing bin (`src/bin/modem_encode.rs`, etc.) keeps building under `--no-default-features`
  but replaces its hand-rolled `print_usage` + `while i < args.len()` with a one-line call to the
  matching `parse_*` helper, then runs its existing modem-library logic unchanged. The bins keep
  their historical names and entry points (so existing scripts/tests that invoke them still work),
  but now share one coherent, validated, `--help`-bearing grammar.

Rejected: (a) collapse the four bins into the single `audiohax` binary and delete them — breaks the
`--no-default-features` autodiscovery that lets the modem build headless (the main bin requires
`opencv`), and breaks any existing invocation/test harness; (b) leave the bins hand-rolled and only
add a `modem` subcommand — fails the "unify under one grammar" mandate and leaves the `--flip_prob`
/`--sim-flip` divergence. The chosen "shared grammar, thin bins + subcommand" path keeps the bins
headless-buildable AND unifies the surface.

### 3.8 `Cargo.toml` — dependency additions (Phase 1 only)

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }   # CLI parser — pure-Rust, builds --no-default-features  [VERIFY clap 4.x]
toml = "0.8"                                        # parse audiohax.toml config                            [VERIFY 0.8]
directories = "5"                                   # platform config/data dir for the config file         [VERIFY 5.x]
# serde already present (1.0, derive) — reused for the config struct. No new serde line needed.
```

- `clap`, `toml`, `directories` are **pure-Rust** and add zero system-lib/cross-platform burden; they
  compile under `--no-default-features` (confirm `directories` has no surprising native dep — it
  wraps `dirs-sys`, pure-Rust on the three target OSes) **[VERIFY]**.
- **NOT added** (Phase 2 / out of scope): `rustysynth`, `cpal`, `imageproc`, `ratatui`, `egui`,
  `crossbeam-channel`. The `AudioSink`/`FeatureSource` traits are *shaped* so those drop in later
  with no engine change, but none is a Phase 1 dependency.
- `serde`'s `derive` feature is already on (Cargo.toml:36), so the config struct (§3.9) needs no new
  dependency line.

### 3.9 Config file (D7) — `audiohax.toml`

Lives in `cli.rs` (or a small `cli::config` submodule). Pure-Rust (`serde` + `toml`).

```rust
/// On-disk config (audiohax.toml). Every field optional so a partial file is valid;
/// merge precedence is CLI flag > config-file value > built-in default (D7).
#[derive(Debug, Default, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ConfigFile {
    pub instruments: Option<usize>,
    pub thickness: Option<f32>,
    pub steps: Option<usize>,
    pub ms_per_step: Option<u64>,
    pub jitter_percent: Option<f32>,
    pub midi_port: Option<String>,
}

/// Load the config file from an explicit path or the platform config dir
/// (directories::ProjectDirs). Missing file → Ok(Default). Pure I/O + toml parse.
pub fn load_config(explicit: Option<&std::path::Path>) -> Result<ConfigFile, ConfigError>;

/// Merge precedence: CLI-provided values win over file values win over defaults.
/// Returns the final PipelineArgs the engine config is built from. The headless
/// test pins each precedence cell.
pub fn merge_config(file: &ConfigFile, cli: &PipelineArgs, defaults: &PipelineArgs) -> PipelineArgs;
```

Precedence is implemented in `merge_config` (the one tested seam), keeping clap's own defaults out of
the way: clap parses into `Option` where "absent" must be distinguishable from "explicitly set to the
default," OR the merge reads clap's `ArgMatches` value-source. The Implementer picks one; the test
asserts the three-way precedence table either way. **[VERIFY which clap API exposes value-source so
"flag absent" vs "flag == default" is distinguishable — `ArgMatches::value_source`.]**

---

## 4. Data-flow diagram (OpenCV boundary marked)

```
  ┌──────────────────────────── main.rs  ADAPTER  (required-features opencv,image) ───────────────────────────┐
  │                                                                                                            │
  │  OpenCV ZONE (everything Mat-touching stays here — NEVER crosses into engine/lib)                          │
  │  ┌───────────────────────────────────────────────────────────────────────────────────────────────────┐  │
  │  │ load_image_from_source ─► Mat ─► analyze_global ─► image_analysis::GlobalFeatures                    │  │
  │  │                              └──► scan_image    ─► Vec<Vec<image_analysis::ScanBarFeatures>>          │  │
  │  │                              └──► imwrite overlays / highgui imshow + wait_key (live scan window)     │  │
  │  └───────────────────────────────────────────┬───────────────────────────────────────────────────────┘  │
  │            field-copy conversion (the BOUNDARY): image_analysis::* ─► engine::* (plain f32, no Mat)       │
  │                                                │                                                          │
  │  PrecomputedSource: impl engine::FeatureSource │     MidiOut: impl engine::AudioSink (orphan-rule home)   │
  │      global_features() ───────────────────────┤                          ▲                               │
  │      scan_bar_features(step,n) ────────────────┤                          │ note_on/note_off/program_change│
  └────────────────────────────────────────────────┼──────────────────────────┼──────────────────────────────┘
                                                    │  &S: FeatureSource        │  &mut A: AudioSink
            ════════════ HEADLESS / PURE-RUST LIBRARY BOUNDARY (builds --no-default-features) ════════════
                                                    │                          │
  ┌──────────────────────────────────── src/engine.rs  PipelineEngine ─────────┼──────────────────────────────┐
  │  set_features_global(global) ─► lookup_range_map + pick_progression        │                               │
  │                                  + generate_chords + plan_phrases ─► Vec<StepPlan> (held)                   │
  │  tick(source, sink):                                                       │                               │
  │     source.scan_bar_features(step, n) ─► decide_instrument_action ─► PerfFeatures ─► chord_engine::         │
  │                                            realize_step ─► Vec<NoteEvent> ─► InstrumentDecision             │
  │     for each decision ─► sink.note_on/.note_off ───────────────────────────┘                               │
  │     advance scan_position ; return EngineTickOutput                                                         │
  │  current_state() ─► EngineSnapshot ─────────────────────────────────────────────────────────────┐         │
  └──────────────────────────────────────────────────────────────────────────────────────────────────┼────────┘
                                                                                                       │ snapshot
  front-ends (future, pure observers): CLI dump / TUI(ratatui) / GUI(egui) ◄── EngineObserver::on_tick ┘
        (image change → adapter swaps the FeatureSource backing + engine.update_image_features(global))
```

Key: the OpenCV zone is wholly inside `main.rs`. The only things that cross the library boundary are
the two trait objects (`FeatureSource`, `AudioSink`) and plain value structs (`engine::GlobalFeatures`,
`engine::ScanBarFeatures`, `NoteEvent`, `EngineSnapshot`). No `Mat`, ever.

---

## 5. Migration path (regression-equivalence net + landing order)

The refactor MUST preserve today's batch playback exactly: same static image → same sequence of note
decisions. Because the engine core is single-threaded and pure (D4/D8), this is a **pure-function
equivalence** the Test Engineer can pin headlessly.

**The regression-equivalence net (headless, new — additive only):**
1. Build a canned `FeatureSource` from a small fixed `Vec<Vec<engine::ScanBarFeatures>>` + a fixed
   `engine::GlobalFeatures` (hand-authored constants — no image, no OpenCV).
2. Construct `PipelineEngine::new(mappings, config)`, `set_features_global(&global)`, then for each
   step assert `engine.decide_step(&source, k)` equals a **golden vector** of `InstrumentDecision`s.
3. The golden vector is generated ONCE from the extracted kernel and frozen; any future change that
   alters note decisions fails this test. (Mode/progression use `thread_rng` inside
   `pick_progression` — so the equivalence net must seed or pin the progression: either pass a fixed
   plan into the engine for the test, or assert on `decide_instrument_action` directly with a fixed
   `&[StepPlan]`, which is fully deterministic. **Recommend the latter** — it isolates the moved
   kernel from `pick_progression`'s RNG and is the cleanest equivalence anchor.)

This is exactly Task R.4 in interactive-architecture.md ("verify batch mode produces identical scan
behavior"), made concrete and headless.

**Order of operations for the Implementer:**
1. Add `clap`/`toml`/`directories` to `Cargo.toml`; add `pub mod engine; pub mod cli;` to `lib.rs`.
   Confirm `cargo build --lib --no-default-features` + `cargo check --lib --no-default-features`.
2. Author `engine.rs` types + `decide_instrument_action` (verbatim-behavior port of
   `worker_decide_action`) + `PipelineEngine` skeleton. (Test Engineer writes the equivalence net RED
   here.) Drive GREEN.
3. Author `cli.rs` (top-level `Cli` + `PipelineArgs` + `pipeline_to_engine_config`; modem grammar +
   `parse_*` helpers; `ConfigFile`/`load_config`/`merge_config`). Test Engineer pins: defaults match
   today's values; precedence table; `Cli::try_parse_from` accepts `play <img>` and rejects bad
   ranges. All headless.
4. Slim `main.rs`: delete `worker_decide_action`/`play_scanned_steps_concurrent`/pool; add
   `PrecomputedSource: FeatureSource` + `impl AudioSink for MidiOut`; drive the engine in the batch
   loop; keep jitter/`Instant` scheduling + overlays/highgui. **(Not buildable here — verified by
   review + compilation reasoning only; the kickoff acknowledges main.rs can't build on the dev box.)**
5. Rewrite the four modem bins' `main` to call `cli::parse_*` then their existing modem logic. Confirm
   `cargo build --no-default-features` for the modem bins + `cargo test --test modem_roundtrip
   --no-default-features` + `--test modem_realair --no-default-features` stay green.
6. Quality Gate: engine core is genuinely headless + pure; main.rs is a thin adapter with no
   musical/orchestration logic; CLI coherent; module boundaries hold; all prior nets pass.

**Independently landable vs coordinated:**
- `cli.rs` + the modem-bin rewrites are **independently landable** (no engine dependency; pure-Rust;
  testable headless now). They could even precede the engine extraction.
- `engine.rs` + the `main.rs` slim-down are **coordinated** (the adapter depends on the engine's
  public surface). Land the engine + its headless equivalence net FIRST, then the main.rs adapter as
  a pure follow-on — never two playback pipelines at once (assessment §8.4 mitigation).
- All existing nets (42 + 17 + 10) are touched **only additively** — the modem bins call new
  parser helpers but run the same `modem::*` library functions; the music nets are untouched.

---

## 6. Risks & trade-offs (what could go wrong; alternatives + why rejected)

1. **Regression equivalence under `pick_progression`'s RNG.** `pick_progression` uses `thread_rng`
   (chord_engine.rs:58), so the *plan* is nondeterministic across runs. *Mitigation:* the equivalence
   net asserts on `decide_instrument_action` with a FIXED `&[StepPlan]` (deterministic), isolating the
   moved kernel from the RNG. The engine-level `tick` test seeds/fixes the plan too. Rejected
   alternative: assert end-to-end through `set_features_global` — would make the golden vector
   RNG-dependent and flaky.
2. **Mirror-type drift.** `engine::GlobalFeatures`/`ScanBarFeatures` duplicate the `image_analysis`
   structs; a future field added to one and not the other silently de-syncs the boundary copy.
   *Mitigation:* the boundary conversion is one function in the adapter; a doc-comment on both struct
   pairs cross-references the other; the equivalence net exercises the copied fields. Rejected
   alternative: move `image_analysis.rs` into the lib to share the type — forbidden (it's OpenCV;
   would break `--no-default-features`).
3. **Orphan-rule placement of `impl AudioSink for MidiOut`.** If the Implementer reflexively puts it
   in `engine.rs` it won't compile (MidiOut invisible to lib). *Mitigation:* §3.3 states the rule
   explicitly; the impl lives in `main.rs`. This is design-confirmed, not a guess.
4. **`AudioSinkError` vs `anyhow`.** Choosing `anyhow` (per the assessment sketch) would force a
   `Send + Sync` bound MidiOut's `Box<dyn Error>` doesn't satisfy, or per-call `.map_err`, or an edit
   to `midi_output.rs` (out of scope). *Mitigation:* the explicit `AudioSinkError(Box<dyn Error +
   Send + Sync>)` wrapper, mapped in the impl. Rejected: `anyhow::Result` (override of assessment
   §4.1 typing — see §9).
5. **Single-threaded core changes timing feel.** The current pool overlaps worker compute with the
   coordinator; single-threaded compute could (in principle) lengthen per-step wall time. *Mitigation:*
   the per-step compute is microseconds vs the 250 ms step budget — the adapter's `Instant` scheduling
   absorbs it; both prior docs confirm parallelism is unnecessary at this scale. Escape hatch: re-home
   a thread pool in the adapter calling the pure `decide_instrument_action` (§3.4). Rejected
   alternative: pool-in-engine (defeats deterministic headless testing).
6. **clap subcommand vs legacy bin grammar.** Folding the modem bins under one grammar risks changing
   a flag a user's script depends on (e.g. `--flip_prob`→`--flip-prob`, or `--sim-flip`→a shared
   name). *Mitigation:* keep each bin's positional+flag shape as close to legacy as the shared structs
   allow; where names must converge, document the rename in the bin's `--help` and the SWARM-STATE
   handoff. The bins keep their NAMES and entry points so existing invocation paths survive. Rejected
   alternative: delete the bins for one mega-binary (breaks `--no-default-features` + invocation).
7. **Config-file precedence ambiguity.** clap's own `default_value_t` makes "flag absent" and "flag
   set to its default" indistinguishable, which breaks "flag > file" precedence (a file value would be
   overridden by an unset flag's default). *Mitigation:* `merge_config` decides precedence from clap's
   value-source (`ArgMatches::value_source`) or by parsing pipeline flags into `Option` and applying
   defaults only in `merge_config`. The Implementer picks one; the precedence test pins it. **[VERIFY
   `ArgMatches::value_source` availability in the chosen clap version.]**
8. **`directories` cross-platform behavior** is assumed pure-Rust on all three OSes. *Mitigation:*
   marked **[VERIFY]**; if it pulls a native dep on any target, fall back to an env-var/`$XDG`/relative
   config path — config-file location is not load-bearing for Phase 1's headless tests (which pass an
   explicit path).

---

## 7. What stays headless-testable (explicit)

- `engine.rs` — entirely. `PipelineEngine`, `decide_instrument_action`, `decide_step`, the snapshot,
  the regression-equivalence net: all under `cargo test --lib --no-default-features`.
- `cli.rs` — entirely. `Cli::try_parse_from`, `pipeline_to_engine_config`, `ModemCommand` parsing,
  `load_config`/`merge_config` precedence: all headless lib tests.
- The four modem bins build + run under `--no-default-features` (unchanged guarantee; they only swap
  their parser).
- NOT headless (acknowledged, unchanged): `main.rs` (OpenCV/midir) — verified by review +
  compilation reasoning only, exactly as today.

---

## 8. Scope boundaries honored (audit)

- `chord_engine.rs` — **not modified.** The engine calls `pick_progression`, `generate_chords`,
  `plan_phrases`, `realize_step`, and reads `PerfFeatures`/`NoteEvent`/`StepPlan`/`PhrasePosition`
  through their existing public API only.
- `modem.rs` — **not modified.** Only the four bins' CLI front-ends change (parser swap); they call
  the same `modem::*` functions.
- `mapping_loader.rs`, `assets/mappings.json`, `image_analysis.rs`, `image_source.rs`,
  `midi_output.rs` internals — **not modified.** The engine holds a `MappingTable` and calls
  `lookup_range_map`; the adapter calls the OpenCV/midi functions and copies their plain-struct
  outputs across the boundary; `impl AudioSink for MidiOut` is an external impl in the adapter, not an
  edit to `midi_output.rs`.
- No dependency collapse (no OpenCV→image, no FluidSynth→rustysynth). The two traits are *shaped* for
  those future drop-ins; none is implemented or depended on here.

---

## 9. What the two prior docs I overrode / pinned beyond

1. **`interactive-architecture.md`: `image_buffer: Arc<RwLock<Mat>>` + `update_image(&mut self,
   image: Mat)` — OVERRIDDEN.** The engine holds no `Mat` and no image buffer. Replaced by the
   `FeatureSource` trait + plain mirror structs + `update_image_features(&GlobalFeatures)`. Reason: a
   `Mat` in the engine makes the lib un-buildable headless and violates the hard constraint; the
   adapter owns the `Mat`. (The doc's `Arc<RwLock<…>>` *concurrency* intent — a live source writing
   while the engine reads — is preserved as the adapter swapping the `FeatureSource`'s backing and
   signaling `InteractionEvent::ImageChanged`; the engine never touches the lock.)
2. **`interactive-architecture.md`: lift the `Barrier` worker pool into the engine — REJECTED (not
   overridden, declined).** §3.4 keeps the engine single-threaded; the doc itself flags the pool as
   "an optimization, not a correctness requirement," so this is consistent with its stated reasoning,
   not a contradiction of its design.
3. **`assessment-ws4-ux-crossplatform.md` §4.1: `AudioSink` methods typed `anyhow::Result<()>` and a
   single `send(ev: &NoteEvent, channel)` method — PINNED BEYOND / ADJUSTED.** Error type changed to
   `Result<(), AudioSinkError>` (D2, MidiOut's error is not `Send+Sync`); `send` split into
   `note_on`/`note_off`/`program_change` to match `MidiOut`'s actual surface and the note_on/off
   pairing the scheduler needs (the assessment's single `send` under-specified note lifetime). The
   `EngineSnapshot`/`EngineCommand`/`EngineObserver` shapes from §4.1 are ADOPTED essentially as-is.
4. **`assessment-ws4-ux-crossplatform.md` §4.1: the INPUT seam was left unpinned — COMPLETED here.**
   This doc supplies the missing `FeatureSource` trait + mirror structs + `update_image_features`
   analogue, which is the reconciliation the kickoff asked for.

---

## Appendix — grounding references (file:line)

- `src/lib.rs`: exports `modem`/`chord_engine`/`mapping_loader` only — `engine`/`cli` are NEW.
- `src/main.rs`: `parse_cli_arg` 39–45; `worker_decide_action` 60–94 (the decision kernel to move);
  `play_scanned_steps_concurrent` 99–321 (barrier pool + MIDI + highgui — dissolved); overloaded
  `"play"` 345–354 + 447; mode/progression/plan 367–384 (moves to engine); `scan_image` 386–388
  (adapter); overlay `imwrite` 390–444 (adapter); jitter + `ScheduledEvent` scheduling 240–298
  (stays in adapter).
- `src/image_analysis.rs`: `GlobalFeatures` 11–20, `ScanBarFeatures` 22–31, `LocalFeatures` 35–46 —
  all plain `f32`/`usize`/`Vec<f32>`, **no `Mat`** (mirror-type source of truth).
- `src/midi_output.rs`: `MidiOut` methods return `Result<_, Box<dyn Error>>` (NOT anyhow; NOT
  Send+Sync) — drives the `AudioSinkError` decision; impl lives in the adapter (orphan rule).
- `src/chord_engine.rs`: `PerfFeatures` 565–573, `NoteEvent` 583–593, `StepPlan` 240–256,
  `PhrasePosition` 219–230, `pick_progression` 45, `generate_chords` 75, `plan_phrases` 382,
  `voice_lead_sequence` 284, `realize_step` 642 — the public seam the engine calls (unmodified).
- `src/mapping_loader.rs`: `MappingTable` 52–57, `lookup_range_map` 66 — pure, lib-resident, callable
  by the engine.
- `src/bin/{modem_encode,modem_decode,channel_sim,make_packetized}.rs`: hand-rolled `print_usage` +
  positional parsing (divergent flag spellings) — replaced by `cli::parse_*`.
- `Cargo.toml`: `default = ["opencv","midir","image"]`; `[[bin]] audiohax` `required-features =
  ["opencv","image"]`; modem bins autodiscovered (build `--no-default-features`); NO clap/toml/
  directories today (Phase 1 additions).
```

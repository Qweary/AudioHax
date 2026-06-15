//! src/engine.rs — the WS-4 Phase 1 pure-Rust shared core (design S9 §3.1).
//!
//! Pure-Rust orchestration over plain feature data plus two traits (`FeatureSource`
//! on the input side, `AudioSink` on the output side). It holds the music state —
//! mappings, the derived mode/progression/phrase plan, the scan position, the last
//! decisions — and **NO OpenCV / image / midir type appears in any signature or
//! field below.** It builds and unit-tests under `cargo build --lib
//! --no-default-features`.
//!
//! Boundary discipline: the `main.rs` adapter does all OpenCV feature extraction
//! and copies the plain `f32` fields into the [`GlobalFeatures`] / [`ScanBarFeatures`]
//! mirror structs at the boundary (the same move S6 already made for
//! `chord_engine::PerfFeatures`). The engine never sees a `Mat`.
//!
//! Concurrency (S9 §3.4 / D4): the core is **single-threaded**. The former
//! `Barrier`-worker-per-instrument pool in `main.rs` is dissolved; the per-instrument
//! decision logic (the old `worker_decide_action`) becomes the pure, deterministic
//! [`decide_instrument_action`] free function the engine calls in a simple loop. If
//! profiling ever justifies parallelism, the ADAPTER may re-home a thread pool that
//! calls `decide_instrument_action` per instrument and merges the results — the
//! engine core stays pure. (Escape hatch documented, not built.)

use crate::chord_engine::{self, ChordEngine, NoteEvent, PerfFeatures, PhrasePosition, StepPlan};
use crate::mapping_loader::{lookup_range_map, MappingTable};

/// Image-free mirror of `image_analysis::GlobalFeatures` (all plain `f32`).
///
/// The `main.rs` adapter performs the OpenCV extraction and copies fields into this
/// at the boundary; the engine never sees a `Mat`. **Keep field-for-field in sync
/// with `image_analysis::GlobalFeatures`** — a field added to one and not the other
/// silently de-syncs the boundary copy (S9 §6 risk 2).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlobalFeatures {
    /// Average hue, 0..360.
    pub avg_hue: f32,
    /// Average saturation, 0..100.
    pub avg_saturation: f32,
    /// Average brightness (HSV value), 0..100.
    pub avg_brightness: f32,
    /// Proportion of edge pixels, 0..1.
    pub edge_density: f32,
    /// Spread of hues, 0..1.
    pub hue_spread: f32,
    /// Variance of the Laplacian (focus/texture).
    pub texture_laplacian_var: f32,
    /// Crude contour-count complexity metric.
    pub shape_complexity: f32,
    /// width / height of the image.
    pub aspect_ratio: f32,
}

/// Image-free mirror of `image_analysis::ScanBarFeatures`.
///
/// `hue_hist` is carried for fidelity / future use; the engine's music decision reads
/// only the scalars it projects into [`PerfFeatures`]
/// (saturation / brightness / edge_density). **Keep field-for-field in sync with
/// `image_analysis::ScanBarFeatures`** (S9 §6 risk 2).
#[derive(Debug, Clone, PartialEq)]
pub struct ScanBarFeatures {
    /// 0-based index of this bar within the step's per-instrument row.
    pub bar_index: usize,
    /// Average hue, 0..360.
    pub avg_hue: f32,
    /// Average saturation, 0..100.
    pub avg_saturation: f32,
    /// Average brightness, 0..100.
    pub avg_brightness: f32,
    /// Proportion of edge pixels, 0..1.
    pub edge_density: f32,
    /// Variance of the Laplacian (focus/texture).
    pub texture_laplacian_var: f32,
    /// Small hue histogram for fingerprinting (unused by the music decision).
    pub hue_hist: Vec<f32>,
}

/// The input seam (S9 D1). The adapter implements this (OpenCV-backed) and the engine
/// pulls plain features through it. Headless tests implement it over canned data.
///
/// Theory of the boundary: the engine OBSERVES image features; it never holds or
/// requests pixels. A `FeatureSource` yields (a) one whole-image [`GlobalFeatures`]
/// and (b) the per-instrument [`ScanBarFeatures`] row for a given scan step. The
/// OpenCV adapter pre-extracts these; a future pure-Rust analyzer or a TUI/GUI live
/// source implements the same trait.
pub trait FeatureSource {
    /// Whole-image features for the current image.
    fn global_features(&self) -> GlobalFeatures;

    /// Per-instrument scan-bar features for scan step `step_idx`. `num_instruments`
    /// is the ensemble width the engine was configured with; the returned row should
    /// have that length. The adapter wrapping today's precomputed
    /// `Vec<Vec<ScanBarFeatures>>` simply indexes it; a live source extracts on the fly.
    fn scan_bar_features(&self, step_idx: usize, num_instruments: usize) -> Vec<ScanBarFeatures>;

    /// Total scan steps available (the batch step count, or a live source's notion).
    fn step_count(&self) -> usize;
}

/// Error wrapper so the [`AudioSink`] trait does not force `anyhow` and does not
/// require editing `midi_output.rs` (whose `Box<dyn Error>` is not `Send + Sync`).
/// Each sink maps its own error into this at the impl site (S9 D2).
#[derive(Debug)]
pub struct AudioSinkError(pub Box<dyn std::error::Error + Send + Sync + 'static>);

impl std::fmt::Display for AudioSinkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "audio sink error: {}", self.0)
    }
}

impl std::error::Error for AudioSinkError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.0.as_ref())
    }
}

impl AudioSinkError {
    /// Wrap any `Send + Sync` error into an `AudioSinkError`.
    pub fn new<E: std::error::Error + Send + Sync + 'static>(e: E) -> Self {
        AudioSinkError(Box::new(e))
    }

    /// Wrap a free-form message as an `AudioSinkError`.
    pub fn msg(m: impl Into<String>) -> Self {
        AudioSinkError(m.into().into())
    }
}

/// The output seam (S9 D2/D3). The engine emits realized notes to whatever sink the
/// front-end wired. `midi_output::MidiOut` satisfies this via an impl in the `main.rs`
/// adapter (orphan rule — the lib cannot name the bin-private `MidiOut`); a future
/// `rustysynth`+`cpal` sink implements the same trait.
pub trait AudioSink {
    /// Sound one note (note_on). `channel` is the instrument's MIDI channel.
    fn note_on(&mut self, channel: u8, note: u8, velocity: u8) -> Result<(), AudioSinkError>;
    /// Release a note (note_off) for `(channel, note)`.
    fn note_off(&mut self, channel: u8, note: u8) -> Result<(), AudioSinkError>;
    /// Set the program (patch) for a channel.
    fn program_change(&mut self, channel: u8, program: u8) -> Result<(), AudioSinkError>;
}

/// One instrument's realized decision for one step — the image-free, time-relative
/// playback unit. Same payload as the old `InstrumentAction.events` tuples
/// `(note, velocity, hold_ms, offset_ms)`, but typed (the engine reuses
/// [`chord_engine::NoteEvent`], which carries exactly those four fields). The adapter
/// applies jitter + wall-clock scheduling; the engine makes only the deterministic
/// musical choice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstrumentDecision {
    /// MIDI channel the instrument plays on (`inst_idx % 16` today).
    pub channel: u8,
    /// The realized note events for this instrument on this step.
    pub events: Vec<NoteEvent>,
}

/// What [`PipelineEngine::tick`] produced this step — the note decisions for every
/// instrument plus the scan position, for the adapter to schedule/visualize. Pure
/// data, no `Mat`.
#[derive(Debug, Clone, PartialEq)]
pub struct EngineTickOutput {
    /// The step that was just processed.
    pub step_index: usize,
    /// Scan position 0.0..=1.0 along the scan axis after this step.
    pub scan_position: f32,
    /// Per-instrument decisions for this step.
    pub decisions: Vec<InstrumentDecision>,
    /// Phrase position of the plan step that drove this tick.
    pub phrase: PhrasePosition,
    /// The currently derived mode name.
    pub mode: String,
}

/// Static-image / front-end-agnostic configuration. Mirrors today's CLI knobs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EngineConfig {
    /// Ensemble width.
    pub num_instruments: usize,
    /// Per-step time budget, in milliseconds.
    pub ms_per_step: u64,
    /// Scan-bar thickness fraction — carried for the adapter's overlay geometry;
    /// unused by the music decision.
    pub bar_thickness_frac: f32,
    /// Root MIDI note for chord generation (60 = C4 today, main.rs:378).
    pub root_midi: u8,
}

impl Default for EngineConfig {
    fn default() -> Self {
        // Mirrors today's main.rs defaults (4 instruments / 250 ms / 0.10 thickness /
        // root 60). `steps`/`jitter_percent` are adapter concerns, not engine state.
        EngineConfig {
            num_instruments: 4,
            ms_per_step: 250,
            bar_thickness_frac: 0.10,
            root_midi: 60,
        }
    }
}

/// External interaction (mouse/game/network) — the `inject_event` channel. Defined
/// now per the prior interactive sketch; the batch CLI uses only `ImageChanged` /
/// `Seek`. Image-free: an image change is SIGNALED, the new features come through the
/// [`FeatureSource`].
#[derive(Debug, Clone, PartialEq)]
pub enum InteractionEvent {
    /// The adapter swapped the `FeatureSource`'s backing image; re-query features.
    ImageChanged,
    /// Jump the scan position to 0.0..=1.0.
    Seek(f32),
    /// Replace the engine configuration.
    SetConfig(EngineConfig),
}

/// Front-end → engine control vocabulary (assessment §4.1 `EngineCommand`).
#[derive(Debug, Clone, PartialEq)]
pub enum EngineCommand {
    /// Set the ensemble width.
    SetInstruments(usize),
    /// Set the per-step time budget (ms).
    SetMsPerStep(u64),
    /// Set the scan-bar thickness fraction.
    SetThickness(f32),
    /// Begin/resume transport.
    Play,
    /// Pause transport.
    Pause,
    /// Stop and reset the scan position to the start.
    Stop,
    /// Jump the scan position to 0.0..=1.0.
    Seek(f32),
}

/// The GUI/TUI-facing snapshot (assessment §4.1 + the interactive sketch's
/// `current_state()`). Plain values only; no OpenCV/image type, no engine internals.
#[derive(Debug, Clone, PartialEq)]
pub struct EngineSnapshot {
    /// Current scan position 0.0..=1.0.
    pub scan_position: f32,
    /// Current step index.
    pub step_index: usize,
    /// Last whole-image features fed in (defaulted to zeros until the first feed).
    pub global: GlobalFeatures,
    /// The music-domain projection of the last step's features.
    pub current_step: PerfFeatures,
    /// The notes decided on the last tick (flattened across instruments).
    pub last_notes: Vec<NoteEvent>,
    /// The currently derived mode name.
    pub mode: String,
    /// Phrase position of the last plan step touched.
    pub phrase: PhrasePosition,
}

/// Per-tick observer (assessment §4.1 `EngineObserver`). Front-ends are pure observers.
pub trait EngineObserver {
    /// Called once per tick with the current engine snapshot.
    fn on_tick(&mut self, snapshot: &EngineSnapshot);
}

/// Whether the engine transport is running. The batch path always runs; the
/// interactive `Play`/`Pause`/`Stop` commands flip this for a future driver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Transport {
    Running,
    Paused,
}

/// The central pipeline engine (S9 §3.1). Owns scan + musical state + the derived
/// plan; does NOT own the image (a [`FeatureSource`] supplies features) and does NOT
/// own the audio transport (the caller passes an [`AudioSink`] into `tick`).
/// Single-threaded core (D4); the adapter may parallelize feature extraction outside it.
pub struct PipelineEngine {
    mappings: MappingTable,
    config: EngineConfig,
    /// Derived from the latest [`GlobalFeatures`]; empty until the first feed.
    plan: Vec<StepPlan>,
    mode: String,
    last_global: Option<GlobalFeatures>,
    scan_position: f32,
    step_index: usize,
    last_notes: Vec<NoteEvent>,
    last_step_perf: PerfFeatures,
    last_phrase: PhrasePosition,
    transport: Transport,
}

impl PipelineEngine {
    /// Construct from mappings + config. The harmony plan is empty until the first
    /// [`set_features_global`](PipelineEngine::set_features_global) call — exactly as
    /// `main.rs` derives mode/progression/plan before playback.
    pub fn new(mappings: MappingTable, config: EngineConfig) -> Self {
        PipelineEngine {
            mappings,
            config,
            plan: Vec::new(),
            mode: "Ionian".to_string(),
            last_global: None,
            scan_position: 0.0,
            step_index: 0,
            last_notes: Vec::new(),
            last_step_perf: PerfFeatures {
                saturation: 0.0,
                brightness: 0.0,
                edge_density: 0.0,
            },
            last_phrase: PhrasePosition::PhraseStart,
            transport: Transport::Running,
        }
    }

    /// Read-only access to the current config.
    pub fn config(&self) -> &EngineConfig {
        &self.config
    }

    /// Read-only access to the currently derived phrase plan (for the adapter /
    /// equivalence-net tests that want to seed a deterministic plan reference).
    pub fn plan(&self) -> &[StepPlan] {
        &self.plan
    }

    /// Feed whole-image features. (Re)derives mode → progression → chords → phrase
    /// plan via `chord_engine` (S9 §3.1; replaces main.rs 367–384). The batch path
    /// calls this once and always re-derives.
    ///
    /// Note (S9): `pick_progression` uses `thread_rng`, so the derived `plan` is
    /// NON-deterministic across calls. The regression-equivalence net therefore pins
    /// [`decide_instrument_action`] against a FIXED `&[StepPlan]`, never this path
    /// (S9 §5 / risk 1).
    pub fn set_features_global(&mut self, global: &GlobalFeatures) {
        // 1) mode from hue (main.rs:368–371)
        let mode = lookup_range_map(&self.mappings.global.hue_to_mode, global.avg_hue)
            .unwrap_or_else(|| "Ionian".to_string());

        // S13: image-driven tempo. brightness → BPM via the (previously dead)
        // brightness_to_tempo_bpm map, continuously interpolated, then BPM → ms/step
        // (one step = one beat). This is the plan-derivation path, NOT the decision
        // kernel; the new ms_per_step flows to decide_instrument_action through the
        // existing parameter at decide_step. engine_equivalence.rs is unaffected (it
        // passes MS_PER_STEP explicitly and never calls this path).
        //
        // Rationale (brightness→tempo): luminance is the canonical visual correlate of
        // energy/arousal — bright images feel fast/energetic, dark images slow/calm
        // (the standard film-scoring intuition). Interpolating CONTINUOUSLY across the
        // JSON anchor points (rather than 3 buckets) means two bright-but-different
        // images still differ in tempo, which is the per-image diversity S13 targets.
        let bpm = interp_tempo_bpm(
            &self.mappings.global.brightness_to_tempo_bpm,
            global.avg_brightness,
        );
        self.config.ms_per_step = (60_000.0 / bpm.max(1.0)).round() as u64;

        // 2) progression → chords → phrase plan (main.rs:373–384). ChordEngine::new
        //    consumes the MappingTable by value, so build a transient engine from a
        //    clone of our mappings, derive the plan, and keep the plan.
        let chord_engine = ChordEngine::new(rebuild_mapping_table(&self.mappings));
        let progression = chord_engine.pick_progression(&mode);
        // S13: real modal-interchange trigger. Dark/low-key image ⇒ larger "drop" ⇒
        // borrow the minor iv (the shadow subdominant). Was hardcoded 0.0 (never fired).
        // M owns recalibrating the threshold this drop is compared against.
        let brightness_drop = (0.5 - global.avg_brightness / 100.0).clamp(0.0, 1.0) * 2.0;
        let chords = chord_engine.generate_chords(
            &progression,
            self.config.root_midi,
            &mode,
            global.edge_density, // unchanged: M recalibrates the threshold side
            brightness_drop,     // S13: was 0.0
            // S13 (M coordination): raw avg_saturation (0..100) flows through so the
            // music layer can normalize it to saturation01 and drive harmonic
            // complexity (triad → 7th → 7th+9th). The seam still carries a plain
            // raw scalar; normalization happens in chord_engine (Option-NORM-MAP).
            global.avg_saturation,
            // S13 (M coordination): raw hue_spread (~0..1) for the colorfulness axis
            // (mode-mixture / borrowed-chord widening) — also normalized music-side.
            global.hue_spread,
        );
        // plan_phrases runs voice_lead_sequence internally, so the shared plan carries
        // the voice-led chords — no separate voice-leading call (matches main.rs:380–383).
        let plan = chord_engine.plan_phrases(&chords);

        self.mode = mode;
        self.plan = plan;
        self.last_global = Some(*global);
        self.last_phrase = self
            .plan
            .first()
            .map(|p| p.position)
            .unwrap_or(PhrasePosition::PhraseStart);
    }

    /// The image-free analogue of the prior `update_image(Mat)` (S9 D1): hand the
    /// engine the freshly extracted features instead of pixels. Equivalent to
    /// [`set_features_global`](PipelineEngine::set_features_global) plus an
    /// `ImageChanged` mark; named to mirror the prior seam for front-end familiarity.
    pub fn update_image_features(&mut self, global: &GlobalFeatures) {
        self.set_features_global(global);
    }

    /// Process one step: pull this step's per-instrument [`ScanBarFeatures`] from
    /// `source`, run [`decide_instrument_action`] per instrument, send note_on/note_off
    /// to `sink`, advance the scan position, and return the decisions for visualization.
    /// Replaces the inner body of `play_scanned_steps_concurrent`.
    ///
    /// Deterministic given (plan, features, config) — jitter and wall-clock scheduling
    /// are the adapter's, NOT the engine's (S9 D8). When paused, this is a no-op that
    /// returns an empty decision set without advancing.
    pub fn tick<S: FeatureSource, A: AudioSink>(
        &mut self,
        source: &S,
        sink: &mut A,
    ) -> Result<EngineTickOutput, AudioSinkError> {
        if self.transport == Transport::Paused {
            return Ok(EngineTickOutput {
                step_index: self.step_index,
                scan_position: self.scan_position,
                decisions: Vec::new(),
                phrase: self.last_phrase,
                mode: self.mode.clone(),
            });
        }

        let step_idx = self.step_index;
        let decisions = self.decide_step(source, step_idx);

        // Send the decided notes immediately as note_on/note_off pairs. The adapter
        // owns jitter + Instant scheduling, so the engine's own send is the simple
        // "decide then emit" path; an adapter that needs precise wall-clock timing
        // uses `decide_step` directly and schedules itself (main.rs does exactly that).
        let mut flat_notes: Vec<NoteEvent> = Vec::new();
        for dec in &decisions {
            for ev in &dec.events {
                sink.note_on(dec.channel, ev.note, ev.velocity)?;
                sink.note_off(dec.channel, ev.note)?;
                flat_notes.push(*ev);
            }
        }

        // Update derived snapshot state (perf projection + phrase of this plan step).
        let row = source.scan_bar_features(step_idx, self.config.num_instruments);
        if let Some(f0) = row.first() {
            self.last_step_perf = PerfFeatures {
                saturation: f0.avg_saturation,
                brightness: f0.avg_brightness,
                edge_density: f0.edge_density,
            };
        }
        let phrase = if self.plan.is_empty() {
            self.last_phrase
        } else {
            self.plan[step_idx % self.plan.len()].position
        };
        self.last_phrase = phrase;
        self.last_notes = flat_notes;

        // Advance scan position. step_count() is the batch step total; position is the
        // fraction THROUGH the scan after completing this step (0-based step k of N →
        // (k+1)/N), clamped to [0,1].
        let total = source.step_count().max(1);
        self.step_index = step_idx + 1;
        self.scan_position = ((self.step_index as f32) / (total as f32)).clamp(0.0, 1.0);

        Ok(EngineTickOutput {
            step_index: step_idx,
            scan_position: self.scan_position,
            decisions,
            phrase,
            mode: self.mode.clone(),
        })
    }

    /// Decide all instruments for `step_idx` WITHOUT sending audio — the pure kernel
    /// the regression-equivalence net pins (S9 §5). [`tick`](PipelineEngine::tick) is
    /// this plus the sink sends + position advance. Headless tests call this directly.
    pub fn decide_step<S: FeatureSource>(
        &self,
        source: &S,
        step_idx: usize,
    ) -> Vec<InstrumentDecision> {
        let num_instruments = self.config.num_instruments;
        let row = source.scan_bar_features(step_idx, num_instruments);
        let mut out = Vec::with_capacity(row.len());
        for (inst_idx, f) in row.iter().enumerate() {
            out.push(decide_instrument_action(
                f,
                inst_idx,
                step_idx,
                num_instruments,
                &self.plan,
                self.config.ms_per_step,
            ));
        }
        out
    }

    /// Apply a control command (front-end → engine). Mutates config / transport state.
    pub fn command(&mut self, cmd: EngineCommand) {
        match cmd {
            EngineCommand::SetInstruments(n) => self.config.num_instruments = n,
            EngineCommand::SetMsPerStep(ms) => self.config.ms_per_step = ms,
            EngineCommand::SetThickness(t) => self.config.bar_thickness_frac = t,
            EngineCommand::Play => self.transport = Transport::Running,
            EngineCommand::Pause => self.transport = Transport::Paused,
            EngineCommand::Stop => {
                self.transport = Transport::Paused;
                self.step_index = 0;
                self.scan_position = 0.0;
            }
            EngineCommand::Seek(p) => {
                self.scan_position = p.clamp(0.0, 1.0);
            }
        }
    }

    /// Inject an external interaction event (mouse/game/network or batch Seek).
    pub fn inject_event(&mut self, event: InteractionEvent) {
        match event {
            // The adapter has already swapped the FeatureSource's backing image and
            // will call update_image_features with the new GlobalFeatures; the mark
            // itself is a no-op for the headless core (no pixels held).
            InteractionEvent::ImageChanged => {}
            InteractionEvent::Seek(p) => self.scan_position = p.clamp(0.0, 1.0),
            InteractionEvent::SetConfig(cfg) => self.config = cfg,
        }
    }

    /// Current state for the GUI/TUI (the prior `current_state()`).
    pub fn current_state(&self) -> EngineSnapshot {
        EngineSnapshot {
            scan_position: self.scan_position,
            step_index: self.step_index,
            global: self.last_global.unwrap_or(GlobalFeatures {
                avg_hue: 0.0,
                avg_saturation: 0.0,
                avg_brightness: 0.0,
                edge_density: 0.0,
                hue_spread: 0.0,
                texture_laplacian_var: 0.0,
                shape_complexity: 0.0,
                aspect_ratio: 0.0,
            }),
            current_step: self.last_step_perf,
            last_notes: self.last_notes.clone(),
            mode: self.mode.clone(),
            phrase: self.last_phrase,
        }
    }
}

/// The per-instrument decision kernel, a behavior-preserving port of
/// `main.rs::worker_decide_action` (S9 §3.1). Pure, deterministic; the
/// regression-equivalence anchor. Projects [`ScanBarFeatures`] → [`PerfFeatures`] and
/// calls [`chord_engine::realize_step`].
///
/// Fidelity to today's behavior (main.rs:60–94):
/// - empty `plan` → no events (a silent step), never panics;
/// - the step used is `plan[step_idx % plan.len()]`;
/// - `PerfFeatures` is a plain field copy (units already match, no cast);
/// - the `_edge_threshold` arg of the old worker is DROPPED (it was already unused —
///   `realize_step` owns that decision).
///
/// The MIDI channel is `inst_idx % 16` (main.rs:131/247) and is carried in the returned
/// [`InstrumentDecision`] so the adapter sends what it is told; the engine does NOT open
/// the port and does NOT apply jitter.
pub fn decide_instrument_action(
    f: &ScanBarFeatures,
    inst_idx: usize,
    step_idx: usize,
    num_instruments: usize,
    plan: &[StepPlan],
    ms_per_step: u64,
) -> InstrumentDecision {
    let channel = (inst_idx % 16) as u8;

    // Empty-plan guard: emit no events (a silent step) — the minimal safe choice
    // that never panics and makes no musical decision here (main.rs:72–74).
    if plan.is_empty() {
        return InstrumentDecision {
            channel,
            events: Vec::new(),
        };
    }
    let step = &plan[step_idx % plan.len()];

    // Project image features into the plain-scalar PerfFeatures. ScanBarFeatures
    // fields are f32 and units already match (saturation/brightness 0..=100,
    // edge_density 0..=1) — no cast needed (main.rs:80–84).
    let features = PerfFeatures {
        saturation: f.avg_saturation,
        brightness: f.avg_brightness,
        edge_density: f.edge_density,
    };

    // Single pure entry point; map NoteEvents straight through (main.rs:87–92, but we
    // keep them as typed NoteEvent rather than re-tupling).
    let events =
        chord_engine::realize_step(step, inst_idx, num_instruments, &features, ms_per_step);
    InstrumentDecision { channel, events }
}

/// Deep-copy a [`MappingTable`] by hand-rebuilding it from its public fields (all
/// public, plain `HashMap`/`Vec`/scalar data).
///
/// NOTE(s9): `ChordEngine::new` consumes a `MappingTable` BY VALUE, but the engine
/// must keep its own copy across re-derivations — and `MappingTable` derives only
/// `Deserialize`, not `Clone`, in `mapping_loader.rs` (out of scope to edit). Rather
/// than add a `#[derive(Clone)]` there, this helper reconstructs a fresh table from
/// the public fields (lossless — they are all plain data). If `mapping_loader.rs`
/// later gains `#[derive(Clone)]`, every call here collapses to `table.clone()`.
fn rebuild_mapping_table(t: &MappingTable) -> MappingTable {
    use crate::mapping_loader::{
        CadenceTrigger, DominantSubTrigger, FineDetailMapping, GlobalMapping,
        InstrumentSectionMapping, MappingTable as MT, ModalInterchangeTrigger,
    };
    MT {
        global: GlobalMapping {
            hue_to_mode: t.global.hue_to_mode.clone(),
            saturation_to_harmonic_complexity: t.global.saturation_to_harmonic_complexity.clone(),
            brightness_to_tempo_bpm: t.global.brightness_to_tempo_bpm.clone(),
            // S13 (design-s13 §5): copy the new normalization block so the rebuilt
            // table stays lossless after M added it to mapping_loader::GlobalMapping.
            feature_normalization: t.global.feature_normalization.clone(),
            dominant_substitution_trigger: DominantSubTrigger {
                edge_complexity_threshold: t
                    .global
                    .dominant_substitution_trigger
                    .edge_complexity_threshold,
                substitutions: t.global.dominant_substitution_trigger.substitutions.clone(),
            },
            modal_interchange_trigger: ModalInterchangeTrigger {
                brightness_drop_threshold: t
                    .global
                    .modal_interchange_trigger
                    .brightness_drop_threshold,
                borrowed_chords: t.global.modal_interchange_trigger.borrowed_chords.clone(),
            },
            cadence_trigger: CadenceTrigger {
                stillness_threshold: t.global.cadence_trigger.stillness_threshold,
                high_motion_cadence: t.global.cadence_trigger.high_motion_cadence.clone(),
                low_motion_cadence: t.global.cadence_trigger.low_motion_cadence.clone(),
            },
            progression_families: t.global.progression_families.clone(),
        },
        instrument_section: InstrumentSectionMapping {
            edge_density_to_rhythm: t.instrument_section.edge_density_to_rhythm.clone(),
            line_orientation_to_interval: t.instrument_section.line_orientation_to_interval.clone(),
            contrast_to_articulation: t.instrument_section.contrast_to_articulation.clone(),
            color_shift_to_chord_extension: t
                .instrument_section
                .color_shift_to_chord_extension
                .clone(),
            texture_to_modal_color: t.instrument_section.texture_to_modal_color.clone(),
        },
        fine_detail: FineDetailMapping {
            pixel_y_position_to_pitch: t.fine_detail.pixel_y_position_to_pitch.clone(),
            pixel_brightness_to_velocity: t.fine_detail.pixel_brightness_to_velocity.clone(),
            local_jaggedness_to_chromaticism: t
                .fine_detail
                .local_jaggedness_to_chromaticism
                .clone(),
            shape_to_ostinato: t.fine_detail.shape_to_ostinato.clone(),
        },
    }
}

/// S13 helper: continuous brightness(0..100) → BPM over the JSON anchor map.
///
/// `brightness_to_tempo_bpm` is a `HashMap<String, u32>` keyed by string ranges
/// (`"0-30"` / `"31-70"` / `"71-100"`). This parses each `"lo-hi": bpm` entry into a
/// `(range-midpoint, bpm)` anchor point, sorts by midpoint, and LINEARLY interpolates
/// (clamped at the ends). A continuous map (not 3 buckets) is what lets two
/// bright-but-different images land on different tempos — the S13 diversity goal.
///
/// Returns a BPM as `f32` (the caller converts to `ms_per_step`). A degenerate/empty
/// map falls back to 240 BPM, which is exactly `60000 / 250` — preserving today's
/// legacy 250 ms default tempo so an empty map is a no-op rather than a surprise.
fn interp_tempo_bpm(map: &std::collections::HashMap<String, u32>, brightness: f32) -> f32 {
    let mut anchors: Vec<(f32, f32)> = map
        .iter()
        .filter_map(|(k, v)| {
            let mut it = k.split('-');
            let lo: f32 = it.next()?.trim().parse().ok()?;
            let hi: f32 = it.next()?.trim().parse().ok()?;
            Some(((lo + hi) * 0.5, *v as f32))
        })
        .collect();
    anchors.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    if anchors.is_empty() {
        return 60_000.0 / 250.0; // == 240 BPM ⇒ preserves the legacy 250 ms default
    }
    if brightness <= anchors[0].0 {
        return anchors[0].1;
    }
    if brightness >= anchors[anchors.len() - 1].0 {
        return anchors[anchors.len() - 1].1;
    }
    for w in anchors.windows(2) {
        let (x0, y0) = w[0];
        let (x1, y1) = w[1];
        if brightness >= x0 && brightness <= x1 {
            let t = (brightness - x0) / (x1 - x0);
            return y0 + t * (y1 - y0);
        }
    }
    anchors[anchors.len() - 1].1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chord_engine::{Chord, StepPlan};

    /// A canned, image-free FeatureSource over a fixed `Vec<Vec<ScanBarFeatures>>` —
    /// the headless test double the regression-equivalence net (and these smoke tests)
    /// build on. No OpenCV, no image.
    struct CannedSource {
        global: GlobalFeatures,
        rows: Vec<Vec<ScanBarFeatures>>,
    }

    impl FeatureSource for CannedSource {
        fn global_features(&self) -> GlobalFeatures {
            self.global
        }
        fn scan_bar_features(
            &self,
            step_idx: usize,
            num_instruments: usize,
        ) -> Vec<ScanBarFeatures> {
            let mut row = self.rows.get(step_idx).cloned().unwrap_or_default();
            row.truncate(num_instruments);
            while row.len() < num_instruments {
                row.push(zero_bar(row.len()));
            }
            row
        }
        fn step_count(&self) -> usize {
            self.rows.len()
        }
    }

    /// A counting AudioSink for headless tick tests — records every note_on / note_off
    /// / program_change without any midir/OpenCV linkage.
    #[derive(Default)]
    struct CountingSink {
        ons: usize,
        offs: usize,
        progs: usize,
    }
    impl AudioSink for CountingSink {
        fn note_on(&mut self, _c: u8, _n: u8, _v: u8) -> Result<(), AudioSinkError> {
            self.ons += 1;
            Ok(())
        }
        fn note_off(&mut self, _c: u8, _n: u8) -> Result<(), AudioSinkError> {
            self.offs += 1;
            Ok(())
        }
        fn program_change(&mut self, _c: u8, _p: u8) -> Result<(), AudioSinkError> {
            self.progs += 1;
            Ok(())
        }
    }

    fn zero_bar(idx: usize) -> ScanBarFeatures {
        ScanBarFeatures {
            bar_index: idx,
            avg_hue: 0.0,
            avg_saturation: 50.0,
            avg_brightness: 50.0,
            edge_density: 0.2,
            texture_laplacian_var: 0.0,
            hue_hist: Vec::new(),
        }
    }

    /// A fixed, deterministic 2-step plan for the equivalence anchor — built without
    /// `pick_progression` (no thread_rng), so `decide_instrument_action` output is
    /// pinnable.
    fn fixed_plan() -> Vec<StepPlan> {
        let chord = Chord {
            name: "I".to_string(),
            notes: vec![60, 64, 67],
        };
        vec![
            StepPlan {
                chord: chord.clone(),
                phrase_index: 0,
                position_in_phrase: 0,
                phrase_len: 4,
                position: PhrasePosition::PhraseStart,
                velocity: 80,
            },
            StepPlan {
                chord,
                phrase_index: 0,
                position_in_phrase: 1,
                phrase_len: 4,
                position: PhrasePosition::Interior,
                velocity: 72,
            },
        ]
    }

    #[test]
    fn decide_instrument_action_empty_plan_is_silent() {
        let f = zero_bar(0);
        let d = decide_instrument_action(&f, 0, 0, 4, &[], 250);
        assert_eq!(d.channel, 0);
        assert!(d.events.is_empty(), "empty plan must emit no events");
    }

    #[test]
    fn decide_instrument_action_channel_wraps_mod_16() {
        let f = zero_bar(0);
        let plan = fixed_plan();
        let d = decide_instrument_action(&f, 17, 0, 32, &plan, 250);
        assert_eq!(d.channel, 1, "channel must be inst_idx % 16");
    }

    #[test]
    fn decide_instrument_action_is_deterministic_on_fixed_plan() {
        let f = zero_bar(0);
        let plan = fixed_plan();
        let a = decide_instrument_action(&f, 2, 1, 4, &plan, 250);
        let b = decide_instrument_action(&f, 2, 1, 4, &plan, 250);
        assert_eq!(
            a, b,
            "pure kernel must be deterministic for the golden anchor"
        );
        assert!(
            !a.events.is_empty(),
            "non-empty plan should realize ≥1 note"
        );
    }

    #[test]
    fn decide_step_produces_one_decision_per_instrument() {
        let mappings =
            crate::mapping_loader::load_mappings("assets/mappings.json").expect("mappings load");
        let cfg = EngineConfig {
            num_instruments: 3,
            ..EngineConfig::default()
        };
        let mut engine = PipelineEngine::new(mappings, cfg);
        // Inject the fixed plan directly via set_features_global; to keep this test
        // independent of pick_progression's RNG we assert only on the SHAPE (one
        // decision per instrument), not on note values.
        let source = CannedSource {
            global: GlobalFeatures {
                avg_hue: 120.0,
                avg_saturation: 60.0,
                avg_brightness: 55.0,
                edge_density: 0.3,
                hue_spread: 0.2,
                texture_laplacian_var: 1.0,
                shape_complexity: 0.1,
                aspect_ratio: 1.5,
            },
            rows: vec![vec![zero_bar(0), zero_bar(1), zero_bar(2)]],
        };
        engine.set_features_global(&source.global_features());
        let decisions = engine.decide_step(&source, 0);
        assert_eq!(decisions.len(), 3, "one decision per instrument");
        assert_eq!(decisions[0].channel, 0);
        assert_eq!(decisions[2].channel, 2);
    }

    #[test]
    fn tick_advances_position_and_drives_sink() {
        let mappings =
            crate::mapping_loader::load_mappings("assets/mappings.json").expect("mappings load");
        let cfg = EngineConfig {
            num_instruments: 2,
            ..EngineConfig::default()
        };
        let mut engine = PipelineEngine::new(mappings, cfg);
        let source = CannedSource {
            global: GlobalFeatures {
                avg_hue: 200.0,
                avg_saturation: 40.0,
                avg_brightness: 30.0,
                edge_density: 0.5,
                hue_spread: 0.3,
                texture_laplacian_var: 2.0,
                shape_complexity: 0.2,
                aspect_ratio: 0.8,
            },
            rows: vec![
                vec![zero_bar(0), zero_bar(1)],
                vec![zero_bar(0), zero_bar(1)],
            ],
        };
        engine.set_features_global(&source.global_features());
        let mut sink = CountingSink::default();
        let out0 = engine.tick(&source, &mut sink).expect("tick 0");
        assert_eq!(out0.step_index, 0);
        assert!((out0.scan_position - 0.5).abs() < 1e-6, "1/2 steps through");
        let out1 = engine.tick(&source, &mut sink).expect("tick 1");
        assert_eq!(out1.step_index, 1);
        assert!((out1.scan_position - 1.0).abs() < 1e-6, "2/2 steps through");
        // Every note_on is paired with a note_off.
        assert_eq!(sink.ons, sink.offs, "note_on/note_off must be paired");
    }

    #[test]
    fn command_pause_makes_tick_a_noop() {
        let mappings =
            crate::mapping_loader::load_mappings("assets/mappings.json").expect("mappings load");
        let mut engine = PipelineEngine::new(mappings, EngineConfig::default());
        let source = CannedSource {
            global: GlobalFeatures {
                avg_hue: 10.0,
                avg_saturation: 50.0,
                avg_brightness: 50.0,
                edge_density: 0.2,
                hue_spread: 0.1,
                texture_laplacian_var: 1.0,
                shape_complexity: 0.1,
                aspect_ratio: 1.0,
            },
            rows: vec![vec![zero_bar(0); 4]],
        };
        engine.set_features_global(&source.global_features());
        engine.command(EngineCommand::Pause);
        let mut sink = CountingSink::default();
        let out = engine.tick(&source, &mut sink).expect("paused tick");
        assert!(out.decisions.is_empty(), "paused tick emits nothing");
        assert_eq!(engine.current_state().step_index, 0, "paused: no advance");
        assert_eq!(sink.ons, 0);
    }

    #[test]
    fn snapshot_reflects_global_and_mode() {
        let mappings =
            crate::mapping_loader::load_mappings("assets/mappings.json").expect("mappings load");
        let mut engine = PipelineEngine::new(mappings, EngineConfig::default());
        let g = GlobalFeatures {
            avg_hue: 300.0,
            avg_saturation: 70.0,
            avg_brightness: 65.0,
            edge_density: 0.4,
            hue_spread: 0.25,
            texture_laplacian_var: 1.5,
            shape_complexity: 0.15,
            aspect_ratio: 1.2,
        };
        engine.update_image_features(&g);
        let snap = engine.current_state();
        assert_eq!(snap.global, g, "snapshot carries the fed global features");
        assert!(!snap.mode.is_empty(), "a mode is derived");
    }

    #[test]
    fn engine_config_default_matches_today() {
        let c = EngineConfig::default();
        assert_eq!(c.num_instruments, 4);
        assert_eq!(c.ms_per_step, 250);
        assert!((c.bar_thickness_frac - 0.10).abs() < 1e-6);
        assert_eq!(c.root_midi, 60);
    }

    /// The canonical S13 tempo anchors from assets/mappings.json: "0-30"→60,
    /// "31-70"→90, "71-100"→120 BPM (midpoints 15 / 50.5 / 85.5).
    fn tempo_anchors() -> std::collections::HashMap<String, u32> {
        let mut m = std::collections::HashMap::new();
        m.insert("0-30".to_string(), 60u32);
        m.insert("31-70".to_string(), 90u32);
        m.insert("71-100".to_string(), 120u32);
        m
    }

    #[test]
    fn interp_tempo_bpm_dark_image_is_slow() {
        // brightness below the lowest midpoint clamps to the slowest anchor (60 BPM).
        let m = tempo_anchors();
        assert!((interp_tempo_bpm(&m, 0.0) - 60.0).abs() < 1e-6);
        assert!((interp_tempo_bpm(&m, 15.0) - 60.0).abs() < 1e-6);
    }

    #[test]
    fn interp_tempo_bpm_bright_image_is_fast() {
        // brightness at/above the highest midpoint clamps to the fastest anchor (120 BPM).
        let m = tempo_anchors();
        assert!((interp_tempo_bpm(&m, 85.5) - 120.0).abs() < 1e-6);
        assert!((interp_tempo_bpm(&m, 100.0) - 120.0).abs() < 1e-6);
    }

    #[test]
    fn interp_tempo_bpm_is_continuous_and_monotonic() {
        // Sweeping brightness up must never DECREASE BPM (bright→fast), and the
        // interior must interpolate (not snap into 3 buckets): a value between two
        // midpoints lands strictly between the two anchor BPMs.
        let m = tempo_anchors();
        let mut prev = interp_tempo_bpm(&m, 0.0);
        let mut saw_interior_value = false;
        let mut b = 0.0f32;
        while b <= 100.0 {
            let cur = interp_tempo_bpm(&m, b);
            assert!(
                cur + 1e-4 >= prev,
                "BPM must be monotonic non-decreasing in brightness (b={b}: {prev}->{cur})"
            );
            prev = cur;
            b += 1.0;
        }
        // Midpoint between anchors 15 (60 BPM) and 50.5 (90 BPM): expect a strictly
        // interpolated value, proving continuity rather than bucketing.
        let mid = interp_tempo_bpm(&m, 32.75);
        if mid > 60.0 + 1e-3 && mid < 90.0 - 1e-3 {
            saw_interior_value = true;
        }
        assert!(
            saw_interior_value,
            "interior brightness must interpolate strictly between anchors, got {mid}"
        );
    }

    #[test]
    fn interp_tempo_bpm_empty_map_preserves_legacy_250ms() {
        // Degenerate/empty map ⇒ 240 BPM ⇒ 60000/240 = 250 ms, today's default tempo.
        let empty: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
        let bpm = interp_tempo_bpm(&empty, 50.0);
        let ms = (60_000.0 / bpm.max(1.0)).round() as u64;
        assert_eq!(
            ms, 250,
            "empty tempo map must preserve the legacy 250 ms default"
        );
    }

    #[test]
    fn set_features_global_makes_tempo_per_image() {
        // Regression guard on E-1: a dark image must yield a SLOWER (larger ms_per_step)
        // tempo than a bright image. Pre-S13 this was constant (always the CLI default).
        let mappings =
            crate::mapping_loader::load_mappings("assets/mappings.json").expect("mappings load");
        let mut engine = PipelineEngine::new(mappings, EngineConfig::default());
        let dark = GlobalFeatures {
            avg_hue: 40.0,
            avg_saturation: 30.0,
            avg_brightness: 25.0,
            edge_density: 0.004,
            hue_spread: 0.05,
            texture_laplacian_var: 300.0,
            shape_complexity: 0.02,
            aspect_ratio: 1.0,
        };
        let bright = GlobalFeatures {
            avg_brightness: 85.0,
            ..dark
        };
        engine.set_features_global(&dark);
        let ms_dark = engine.config().ms_per_step;
        engine.set_features_global(&bright);
        let ms_bright = engine.config().ms_per_step;
        assert_ne!(ms_dark, ms_bright, "tempo must vary per image");
        assert!(
            ms_dark > ms_bright,
            "darker image must be slower (larger ms_per_step): dark={ms_dark} bright={ms_bright}"
        );
    }
}

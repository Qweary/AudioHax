// src/render.rs — the ONE render/play orchestration seam (S122 Phase 0, operator ruling D-28).
//
// Every front-end — the `audiohax` CLI (`main.rs`), the TUI, and the planned egui GUI — calls
// THIS module to turn an image into the current piece. Before S122 this orchestration lived in
// the BIN (`main.rs::run_render_wav` + the inline `play` loop), so any second front-end that
// linked the lib got the S37-era `PipelineEngine` piece, not today's, and would have had to COPY
// ~415 lines to catch up. That copy was the drift (s121-01 §1 H1). The code below was MOVED out
// of `main.rs`, not rewritten: same register-set order, same refusal texts, same timeline
// builder, same tail constant, same loop fold. `main.rs` keeps only I/O (paths, files, stdout,
// the sink, the Ctrl-C handler) and drives this module.
//
// Layering: sits ABOVE `engine` / `composition` / `instrumentation` / `synth_sink` /
// `pure_analysis`, BELOW every front-end. Calls `engine.rs` (byte-frozen) only through its
// existing public API, and `chord_engine.rs` only through its existing public setters.
//
// THREAD-LOCAL REGISTERS (s121-01 §3.2). The seed (`seed.rs`), expression
// (`composition::expression_register`) and cadence (`chord_engine`) registers are
// `thread_local!`. They are SET and CONSUMED inside ONE call here — `render*()` sets them and
// synthesizes; `prepare_play*()` sets them and builds the whole event timeline — so a caller
// cannot split the set from the use across threads. `play()` itself reads no register: it only
// streams the pre-built timeline. The event dump (`expression_dump`) is process-global, not
// thread-local; it is armed and disarmed around the timeline build exactly as before.
//
// STDOUT. The library prints NOTHING. Every line the CLI printed from inside the moved code is
// now a [`RenderNote`] (render / prepare) or a [`PlayEvent`] (play), emitted at the EXACT
// sequence point where the `println!` used to run, and `main.rs` formats it with the exact old
// format string. That keeps the CLI's stdout byte-identical (tools/ab-s4[89]-render.sh and
// tools/ab-s50-render.sh grep the `wrote` lines; tests/loop_boundary.rs parses the `--loop`
// period line) while a GUI can show, log or ignore the same facts.

use crate::cli::{pipeline_to_engine_config, PipelineArgs, PlayArgs, RenderArgs, Toggle, V2Spec};
use crate::composition::ImageUnderstanding;
use crate::engine::{AudioSink, AudioSinkError, FeatureSource, GlobalFeatures, PipelineEngine};
use crate::mapping_loader::{rebuild_mapping_table, MappingTable};
use crate::pure_analysis::{AnalysisError, PureAnalysisSource};
use crate::shrink::{Shrink, ShrinkError};
use crate::synth_sink::{SoundFontSource, TimedMidiEvent};

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

// ─────────────────────────────────────────────────────────────────────────────
// Request / input / output types
// ─────────────────────────────────────────────────────────────────────────────

/// Everything that decides WHAT is rendered. Field-for-field the musical subset of
/// `cli::RenderArgs` + `cli::AudioArgs`, minus I/O (paths), so the CLI and every GUI build the
/// same struct. `From<&RenderArgs>` and `From<&PlayArgs>` are the CLI's only adapters.
///
/// `play` reads `seed`, `expression`, `v2` and `pipeline`; `loop_cue`, `reverb`, `gain` and
/// `event_dump` are render-only (on the live path the caller builds and configures the sink).
#[derive(Debug, Clone, PartialEq)]
pub struct RenderRequest {
    /// `--seed`: explicit composition seed. `None` ⇒ the image-derived seed (S91).
    pub seed: Option<u64>,
    /// `--loop`: fold the ring-out onto the head so the cue repeats seamlessly.
    pub loop_cue: bool,
    /// `--expression`: arm the X1/E1 expression axis.
    pub expression: Toggle,
    /// `--v2`: the S113 v2 term set (requires `expression` on).
    pub v2: Option<V2Spec>,
    /// Scan / instrument parameters (`--instruments`, `--thickness`, `--steps`, …).
    pub pipeline: PipelineArgs,
    /// `--reverb`: rustysynth reverb + chorus.
    pub reverb: Toggle,
    /// `--gain`: master gain (non-unity ⇒ tanh soft-clip).
    pub gain: f32,
    /// Arm the S107 event dump for this render. The JSON is handed to the caller through
    /// [`RenderNote::EventDumpJson`] at the point the CLI writes its file.
    pub event_dump: Option<DumpLabel>,
}

/// The two strings the event dump stamps into its header (formerly derived inline from the WAV
/// path and the image path in `main.rs`).
#[derive(Debug, Clone, PartialEq)]
pub struct DumpLabel {
    /// Cue name — the CLI uses the WAV file stem (`"cue"` when it has none).
    pub cue: String,
    /// Image label — the CLI uses the image path (`"<bundled example>"` when none was given).
    pub image: String,
}

impl From<&RenderArgs> for RenderRequest {
    /// The `render` CLI adapter. The dump labels are computed exactly as `main.rs` did.
    fn from(args: &RenderArgs) -> Self {
        RenderRequest {
            seed: args.seed,
            loop_cue: args.loop_cue,
            expression: args.expression,
            v2: args.v2,
            pipeline: args.pipeline.clone(),
            reverb: args.audio.reverb,
            gain: args.audio.gain,
            event_dump: args.event_dump.as_ref().map(|_| DumpLabel {
                cue: args
                    .wav
                    .as_ref()
                    .and_then(|w| w.file_stem())
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "cue".to_string()),
                image: args
                    .image
                    .as_ref()
                    .map(|i| i.display().to_string())
                    .unwrap_or_else(|| "<bundled example>".to_string()),
            }),
        }
    }
}

impl From<&PlayArgs> for RenderRequest {
    /// The `play` CLI adapter. `play` has no `--expression` / `--v2` / `--loop` /
    /// `--event-dump`, so those are off — which leaves every register exactly as the pre-S122
    /// `play` left it (`expression_register::set(inert())` writes the `thread_local!`
    /// initializer's own value).
    fn from(args: &PlayArgs) -> Self {
        RenderRequest {
            seed: args.seed,
            loop_cue: false,
            expression: Toggle::Off,
            v2: None,
            pipeline: args.pipeline.clone(),
            reverb: args.audio.reverb,
            gain: args.audio.gain,
            event_dump: None,
        }
    }
}

impl RenderRequest {
    /// Refusals that need no image and no mappings. `render*()` calls this first; the CLI also
    /// calls it BEFORE loading `mappings.json`, so the refusal still wins over a mappings error,
    /// exactly as when this check sat at the top of `main.rs::run_render_wav`.
    pub fn validate(&self) -> Result<(), RenderError> {
        // The wrap-fold sums the ring-out onto the head, which is only the steady state of a
        // repeating loop if the render path is LINEAR. `apply_gain_softclip` is `tanh` at
        // non-unity gain, and `tanh(a) + tanh(b) != tanh(a + b)` — so refuse rather than emit a
        // loop whose seam is quietly wrong.
        if self.loop_cue && self.gain != 1.0 {
            return Err(RenderError::Refused(format!(
                "--loop requires unity --gain (got {}): the soft-clip is nonlinear, so folding the \
                 ring-out onto the head would no longer reproduce a repeating loop's steady state.",
                self.gain
            )));
        }
        Ok(())
    }

    /// S124 (lead ruling L-30): lay a preset under this request. Field by field over the
    /// presettable options (`presets::PRESET_SETTING_IDS`): an option the user TYPED (`explicit`)
    /// keeps this request's value; every other one takes `preset`'s. `seed`, `loop_cue` and
    /// `event_dump` always keep this request's value (a preset sets no seed and no output).
    ///
    /// For the CLI, an untyped field of `self` holds clap's default, and so does an unset field of
    /// `preset` (it is parsed by clap from the preset's settings alone), so overlaying the `default`
    /// preset — no settings — is the identity. The unit tests `overlay_covers_every_presettable_id`
    /// and `overlaying_the_default_preset_is_the_identity` force both properties.
    pub fn overlay_preset(
        &self,
        preset: &RenderRequest,
        explicit: &crate::presets::ExplicitArgs,
    ) -> RenderRequest {
        let take = |id: &str| !explicit.contains(id);
        let mut out = self.clone();
        if take("instruments") {
            out.pipeline.instruments = preset.pipeline.instruments;
        }
        if take("thickness") {
            out.pipeline.thickness = preset.pipeline.thickness;
        }
        if take("steps") {
            out.pipeline.steps = preset.pipeline.steps;
        }
        if take("expression") {
            out.expression = preset.expression;
        }
        if take("v2") {
            out.v2 = preset.v2;
        }
        if take("reverb") {
            out.reverb = preset.reverb;
        }
        if take("gain") {
            out.gain = preset.gain;
        }
        out
    }
}

/// Where the image comes from. `Bytes` is the GUI / drag-and-drop path (encoded PNG/JPEG).
pub enum ImageInput<'a> {
    /// A file on disk (the CLI path).
    Path(&'a std::path::Path),
    /// An encoded image already in memory.
    Bytes(&'a [u8]),
}

/// One fact the render / prepare path reports, at the exact point in the sequence where the
/// pre-S122 CLI printed it. The library prints nothing; `main.rs` turns each note into the
/// same line it always printed. The callback is fallible so the CLI can write the event-dump
/// file at its historical point (before synthesis) and propagate an I/O error from there.
#[derive(Debug)]
pub enum RenderNote<'a> {
    /// The image decoded (pixel dimensions). S125: always the DECODED picture's size; with a
    /// shrink this is the size before the shrink, and the note comes after the shrink has been
    /// checked against the floor and before it is applied (`crate::shrink`).
    ImageLoaded {
        /// Width in pixels.
        width: u32,
        /// Height in pixels.
        height: u32,
    },
    /// The scan produced this many feature steps (`FeatureSource::step_count`).
    ImageScanned {
        /// Scan steps.
        steps: usize,
    },
    /// The whole-image features the engine sees.
    GlobalFeatures(GlobalFeatures),
    /// Render only: the composed piece's step count (plan) and voice count.
    Planned {
        /// Steps in the composed piece.
        total_steps: usize,
        /// Voices.
        instrument_count: usize,
    },
    /// Render only: the event dump is about to be armed.
    EventDumpArming,
    /// `--expression on` was read back off every composed section.
    ExpressionVerified {
        /// Sections with the profile enabled.
        armed: usize,
        /// Sections in the plan.
        total: usize,
    },
    /// One `--v2` term's field was read back off every composed section.
    V2TermVerified {
        /// Term label (`E3`, `E2'`, `T-A`).
        label: &'static str,
        /// Profile field asserted.
        field: &'static str,
        /// Sections with the field set.
        armed: usize,
        /// Sections in the plan.
        total: usize,
    },
    /// Render only: the serialized event dump. Persist it here if wanted.
    EventDumpJson(&'a str),
    /// Render only: the dump's counters, after the JSON was handed over.
    EventDumpWritten {
        /// Event records.
        total_events: u64,
        /// Frames.
        frames: u64,
        /// Silent frames.
        silent_frames: u64,
    },
    /// Render only: the `--loop` fold ran.
    LoopFolded {
        /// Steps in the period.
        total_steps: usize,
        /// Grid step in ms.
        grid_ms_per_step: u64,
        /// Period in ms.
        period_ms: u128,
        /// Period in frames (the WAV's length).
        period_frames: usize,
        /// Ring-out folded onto the head, in ms.
        folded_ms: u128,
    },
    /// Prepare-play only: `engine.current_state().mode` after composing.
    EngineMode(&'a str),
}

/// Plain-data facts about a finished render (the GUI's "see the project's state" panel).
#[derive(Debug, Clone, PartialEq)]
pub struct RenderSummary {
    /// Steps in the composed piece (the plan's, not the scan's).
    pub total_steps: usize,
    /// Plan tempo grid in ms per step.
    pub grid_ms_per_step: u64,
    /// Voices.
    pub instrument_count: usize,
    /// Per-voice GM programs.
    pub programs: Vec<u8>,
    /// `(armed, total)` sections when `--expression on` was verified.
    pub expression_verified_sections: Option<(usize, usize)>,
}

/// A rendered piece.
#[derive(Debug, Clone)]
pub struct RenderOutput {
    /// Interleaved stereo f32, gain/soft-clipped, loop-folded if asked.
    pub interleaved: Vec<f32>,
    /// Sample rate (44 100, as before).
    pub sample_rate: u32,
    /// The event timeline that was synthesized.
    pub events: Vec<TimedMidiEvent>,
    /// Plain-data facts about the piece.
    pub summary: RenderSummary,
}

/// Why a render / prepare / play failed.
///
/// `Debug` is implemented BY HAND and forwards to the wrapped value, so that when `main.rs`
/// returns one of these from `fn main() -> Result<(), Box<dyn Error>>` the `Error: …` line on
/// stderr is byte-identical to the pre-S122 binary (which boxed the inner value directly; a
/// boxed `String` debug-prints as the quoted string). A derived `Debug` would add the variant
/// name and change that line.
#[derive(thiserror::Error)]
pub enum RenderError {
    /// A request the renderer refuses rather than renders (verbatim refusal text).
    #[error("{0}")]
    Refused(String),
    /// The event dump could not be serialized / snapshotted.
    #[error("{0}")]
    Dump(String),
    /// Image decode / analysis failed.
    #[error(transparent)]
    Analysis(#[from] AnalysisError),
    /// SoundFont load, synthesis or loop fold failed.
    #[error(transparent)]
    Synth(#[from] AudioSinkError),
    /// S125: the requested shrink cannot be applied (it would take a side under the floor).
    #[error(transparent)]
    Shrink(#[from] ShrinkError),
    /// An I/O error raised by a caller's note handler (e.g. writing the event dump).
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl std::fmt::Debug for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RenderError::Refused(s) | RenderError::Dump(s) => std::fmt::Debug::fmt(s, f),
            RenderError::Analysis(e) => std::fmt::Debug::fmt(e, f),
            RenderError::Synth(e) => std::fmt::Debug::fmt(e, f),
            // S125: a shrink refusal is a plain sentence (UI4 § 3.3), so `main`'s `Error: …` line
            // prints the sentence itself, unquoted.
            RenderError::Shrink(e) => std::fmt::Display::fmt(e, f),
            RenderError::Io(e) => std::fmt::Debug::fmt(e, f),
        }
    }
}

/// The note callback type every entry point takes.
pub type NoteSink<'n> = dyn FnMut(RenderNote<'_>) -> Result<(), RenderError> + 'n;

// ─────────────────────────────────────────────────────────────────────────────
// render()
// ─────────────────────────────────────────────────────────────────────────────

/// THE render function. Validates, decodes + analyzes the image (pure-Rust), sets the seed /
/// expression / cadence registers, composes, builds the timeline and synthesizes — all on the
/// CALLING thread (the registers are thread-local). `main.rs`'s `render --wav` is: adapt →
/// `validate` → load mappings → `render()` → `write_stereo_wav` → print.
pub fn render(
    req: &RenderRequest,
    image: ImageInput<'_>,
    mappings: &MappingTable,
    font: SoundFontSource<'_>,
    notes: &mut NoteSink<'_>,
) -> Result<RenderOutput, RenderError> {
    render_shrunk(req, Shrink::Full, image, mappings, font, notes)
}

/// S125: [`render`] from a shrunk copy of the picture ([`acquire_shrunk`]). `Shrink::Full` is
/// [`render`] exactly (no resize call). The CLI's `render --wav --shrink` and the window's Render
/// and Save WAV come through here.
pub fn render_shrunk(
    req: &RenderRequest,
    shrink: Shrink,
    image: ImageInput<'_>,
    mappings: &MappingTable,
    font: SoundFontSource<'_>,
    notes: &mut NoteSink<'_>,
) -> Result<RenderOutput, RenderError> {
    req.validate()?;
    let (source, understanding) = acquire_shrunk(&req.pipeline, shrink, image, notes)?;
    render_with_source(req, &source, Some(&understanding), mappings, font, notes)
}

/// [`render`] over a feature source the caller already acquired — the entry for a source the
/// library cannot build itself (the bin-private OpenCV `PrecomputedSource`).
///
/// `understanding` is `Some` on the pure-Rust path; `None` keeps the legacy behaviour of the
/// OpenCV arm: flat (non-composer) path, legacy per-voice programs, no cadence register, and the
/// composition seed taken from `req.seed` alone (no image-derived seed without features).
pub fn render_with_source<S: FeatureSource>(
    req: &RenderRequest,
    source: &S,
    understanding: Option<&ImageUnderstanding>,
    mappings: &MappingTable,
    font: SoundFontSource<'_>,
    notes: &mut NoteSink<'_>,
) -> Result<RenderOutput, RenderError> {
    use crate::synth_sink::{render_events_to_stereo, SynthConfig};

    req.validate()?;

    let Composed {
        engine,
        programs,
        total_steps,
        grid_ms_per_step,
        instrument_count,
    } = compose(req, source, understanding, mappings)?;
    notes(RenderNote::Planned {
        total_steps,
        instrument_count,
    })?;

    // ── Lay decisions onto the SHARED absolute ms grid (NO jitter ⇒ deterministic). ──
    // S40 divergence #2: this is now the ONE timeline builder live `play` consumes too, so
    // the two paths are identical by construction. ProgramChange init, per-step NoteOn/NoteOff
    // accumulation (with the deliberate cross-step overlap, NO grid clamp), and the global
    // stable sort all live in `build_step_event_timeline` now.
    // ── S107/X0 DIAGNOSTIC: arm the event-level dump around the timeline build. ──
    // Armed ONLY when `--event-dump <PATH>` is given; absent, `is_armed()` is false for the
    // whole render and every hook is a no-op, so the WAV is byte-identical to a build without
    // the flag. `expression_armed` is recorded as `false` because no expression axis exists at
    // X0 — a dump must never be mistakable for evidence about a build it did not observe.
    // S122 round 2: the arm/disarm bracket is ERROR-SAFE. `dump_guard` disarms on any early
    // return (or unwind) between `arm` and the explicit `disarm` below — e.g. a caller's note
    // handler failing on `ExpressionVerified` — so a failed render cannot leave the
    // process-global dump recording the next render under its label. The success path still
    // disarms at exactly the old point (`dump_guard.disarm()`), so its order is unchanged.
    let mut dump_guard = DumpArmGuard { armed: false };
    if let Some(label) = &req.event_dump {
        notes(RenderNote::EventDumpArming)?;
        crate::expression_dump::arm(&label.cue, &label.image, req.seed, req.expression.is_on());
        dump_guard.armed = true;
    }

    let expression_verified_sections = verify_expression(req, &engine, notes)?;

    let events = build_step_event_timeline(
        &engine,
        source,
        total_steps,
        grid_ms_per_step,
        instrument_count,
        &programs,
    );

    if req.event_dump.is_some() {
        dump_guard.disarm();
        let json = crate::expression_dump::to_json().ok_or_else(|| {
            RenderError::Dump("event dump: failed to serialize the recorded timeline".to_string())
        })?;
        notes(RenderNote::EventDumpJson(&json))?;
        let d = crate::expression_dump::snapshot().ok_or_else(|| {
            RenderError::Dump("event dump: failed to snapshot the recorded timeline".to_string())
        })?;
        notes(RenderNote::EventDumpWritten {
            total_events: d.counters.total_events,
            frames: d.counters.frames,
            silent_frames: d.counters.silent_frames,
        })?;
    }

    // ── Synthesize offline, honoring the A/B controls. ──
    let synth_config = SynthConfig {
        enable_reverb_and_chorus: req.reverb.is_on(),
        gain: req.gain,
    };
    let sample_rate = 44_100u32;

    // ── Tail length. The DEFAULT path keeps the historical 1.5 s EXACTLY — every sealed
    //    golden and every byte-identity guard depends on it, so it must not move. The loop
    //    path needs a longer tail because the fold can only relocate ring-out it actually
    //    RENDERED: a patch whose decay outlasts the buffer (measured: `warm_plucked`, GM 32/46/
    //    6/24, still at −68.3 dBFS in the final 50 ms and decaying only −19.0 dB/s ⇒ ~1168 ms
    //    short) would be folded mid-decay, silently losing its ring-out. 4 s clears the
    //    measured worst case with margin; `tests/loop_boundary.rs` asserts the decay actually
    //    completed rather than trusting this constant.
    let tail_ms: u64 = if req.loop_cue { 4_000 } else { 1_500 };

    // MB-5 (lead ruling L-32): refuse a piece longer than `MAX_PIECE_MS` BEFORE the synthesis
    // buffer is sized from the timeline's last event (a hostile mappings table measured a 296 GB
    // allocation and an abort there, which no `catch_unwind` can contain).
    check_piece_length(&events, tail_ms)?;

    let interleaved =
        render_events_to_stereo(font, synth_config, sample_rate, events.clone(), tail_ms)?;

    let interleaved = if req.loop_cue {
        // The musical loop period — the quantity the render boundary SHOULD have been.
        // Step `N` is anchored at `N * grid_ms_per_step` (`step_onset_offset_ms`), so the
        // grid spans `[0, total_steps * grid_ms_per_step)` and the period is that width.
        // Computed in u128 and rounded (not truncated) so the period matches the grid to the
        // nearest frame at rates where `ms * sr` is not integral.
        let period_ms = total_steps as u128 * grid_ms_per_step as u128;
        let period_frames = ((period_ms * sample_rate as u128 + 500) / 1000) as usize;

        // DECAY-COMPLETION GUARD. The fold can only relocate ring-out that was actually
        // rendered. If the buffer is still sounding at its very end, the decay was cut off
        // BEFORE the fold and that energy is simply gone — the cue loses its ring-out while
        // still passing any seam/gap test, because the gap test only looks at the seam. This
        // is the one failure a seam-only gate cannot see, so it is enforced here, at every
        // real render, rather than only in the test suite.
        const DECAY_WINDOW_MS: usize = 50;
        const DECAY_FLOOR_DBFS: f32 = -90.0;
        let win_frames = DECAY_WINDOW_MS * sample_rate as usize / 1000;
        let tail_slice = &interleaved[interleaved.len().saturating_sub(win_frames * 2)..];
        let rms = (tail_slice.iter().map(|s| s * s).sum::<f32>() / tail_slice.len() as f32).sqrt();
        let dbfs = 20.0 * rms.max(1e-12).log10();
        if dbfs > DECAY_FLOOR_DBFS {
            return Err(RenderError::Refused(format!(
                "--loop: the render is still sounding at its end ({dbfs:.1} dBFS over the final \
                 {DECAY_WINDOW_MS} ms, floor {DECAY_FLOOR_DBFS:.1}). The {tail_ms} ms tail did \
                 not contain this patch's decay, so folding would DISCARD the ring-out rather \
                 than relocate it. Raise the loop-path tail."
            )));
        }

        let folded = crate::synth_sink::fold_loop_tail(&interleaved, period_frames)?;
        notes(RenderNote::LoopFolded {
            total_steps,
            grid_ms_per_step,
            period_ms,
            period_frames,
            folded_ms: (interleaved.len() / 2 - period_frames) as u128 * 1000 / sample_rate as u128,
        })?;
        folded
    } else {
        interleaved
    };

    Ok(RenderOutput {
        interleaved,
        sample_rate,
        events,
        summary: RenderSummary {
            total_steps,
            grid_ms_per_step,
            instrument_count,
            programs,
            expression_verified_sections,
        },
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// play: prepare (registers + timeline, one call) then stream (no registers)
// ─────────────────────────────────────────────────────────────────────────────

/// Cooperative cancellation shared by the GUI's Stop button and the CLI's Ctrl-C handler
/// (`main.rs`'s BUG-01 `shutdown: Arc<AtomicBool>`, moved, not reinvented). Clone it into the
/// handler; [`play`] polls it.
#[derive(Debug, Clone, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    /// Request a stop. Async-signal-safe in practice: one atomic store, nothing else.
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    /// Has a stop been requested?
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// A composed piece ready to stream: everything [`play`] needs, with every thread-local
/// register already consumed (the timeline is built inside [`prepare_play`]).
#[derive(Debug, Clone, PartialEq)]
pub struct PlayPlan {
    /// The ONE shared event timeline (the same builder `render` uses — S40).
    pub timeline: Vec<TimedMidiEvent>,
    /// Steps in the composed piece.
    pub total_steps: usize,
    /// Plan tempo grid in ms per step.
    pub grid_ms_per_step: u64,
    /// Voices.
    pub instrument_count: usize,
    /// Per-voice GM programs (for the eager program-change handshake).
    pub programs: Vec<u8>,
    /// The feature source's own step count (the CLI's "No steps to play." test reads this).
    pub source_step_count: usize,
}

/// Progress from [`play`], plain data.
#[derive(Debug)]
pub enum PlayEvent {
    /// Streaming is about to start (after the eager program changes).
    Started {
        /// Steps in the composed piece.
        total_steps: usize,
        /// Plan tempo grid in ms per step.
        grid_ms_per_step: u64,
    },
    /// The playhead entered this step (emitted once per step change, before its first event
    /// is waited for — the old OpenCV overlay point).
    Step(usize),
    /// A cancel was observed at the top of an event (the old "Shutdown requested" point).
    Cancelled,
    /// The sink rejected one event; streaming continues (as before).
    SinkError(AudioSinkError),
    /// Streaming ended (normally or cancelled).
    Finished,
}

/// Decode + analyze the image, set the registers, compose and build the event timeline — ALL on
/// the calling thread, in this one call. Stream the result with [`play`].
pub fn prepare_play(
    req: &RenderRequest,
    image: ImageInput<'_>,
    mappings: &MappingTable,
    notes: &mut NoteSink<'_>,
) -> Result<PlayPlan, RenderError> {
    prepare_play_shrunk(req, Shrink::Full, image, mappings, notes)
}

/// S125: [`prepare_play`] from a shrunk copy of the picture ([`acquire_shrunk`]). `Shrink::Full` is
/// [`prepare_play`] exactly (no resize call).
pub fn prepare_play_shrunk(
    req: &RenderRequest,
    shrink: Shrink,
    image: ImageInput<'_>,
    mappings: &MappingTable,
    notes: &mut NoteSink<'_>,
) -> Result<PlayPlan, RenderError> {
    let (source, understanding) = acquire_shrunk(&req.pipeline, shrink, image, notes)?;
    prepare_play_with_source(req, &source, Some(&understanding), mappings, notes)
}

/// [`prepare_play`] over a caller-acquired feature source (see [`render_with_source`] for what
/// `understanding: None` means).
pub fn prepare_play_with_source<S: FeatureSource>(
    req: &RenderRequest,
    source: &S,
    understanding: Option<&ImageUnderstanding>,
    mappings: &MappingTable,
    notes: &mut NoteSink<'_>,
) -> Result<PlayPlan, RenderError> {
    let Composed {
        engine,
        programs,
        total_steps,
        grid_ms_per_step,
        instrument_count,
    } = compose(req, source, understanding, mappings)?;
    // CLI `play` never arms expression, so this is a no-op there; a GUI that arms it gets the
    // same read-back guard `render` has, so Play and Render cannot quietly differ.
    verify_expression(req, &engine, notes)?;
    let mode = engine.current_state().mode;
    notes(RenderNote::EngineMode(&mode))?;

    // S40 divergence #2: build the ONE shared event timeline that `render --wav` consumes
    // and STREAM it in real time (see `play`). Built HERE, in the same call that set the
    // registers, so the thread-local registers are consumed on the thread that set them.
    let timeline = build_step_event_timeline(
        &engine,
        source,
        total_steps,
        grid_ms_per_step,
        instrument_count,
        &programs,
    );
    // MB-5: the same cap on the live path (a ten-day live play is a hang, `s124-SH` MB-5).
    check_piece_length(&timeline, 0)?;

    Ok(PlayPlan {
        timeline,
        total_steps,
        grid_ms_per_step,
        instrument_count,
        programs,
        source_step_count: source.step_count(),
    })
}

/// Stream a prepared piece to `sink` in real time, polling `cancel` in ≤20 ms slices (BUG-01).
///
/// On cancel it emits [`PlayEvent::Cancelled`] (when seen at the top of an event), then
/// [`PlayEvent::Finished`], and RETURNS `Ok(())` normally — it never owns or drops the sink, so
/// the caller's sink is dropped by the caller (for `MidiOut`, that `Drop` sends all-sound-off).
/// Reads no thread-local register.
pub fn play<A: AudioSink + ?Sized>(
    plan: &PlayPlan,
    sink: &mut A,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(PlayEvent),
) -> Result<(), RenderError> {
    use crate::synth_sink::MidiCmd;
    const SHUTDOWN_POLL_SLICE: Duration = Duration::from_millis(20);

    let total_steps = plan.total_steps;
    let grid_ms_per_step = plan.grid_ms_per_step;

    // Initial per-channel programs (S74b/B5): voice `i` gets `programs[i]` (the resolved ensemble);
    // empty/short ⇒ legacy `(i*7)%128` floor. This eager init is redundant with the ProgramChange
    // events already carried in the shared timeline (at_ms=0) — kept for the MIDI-port "ready"
    // handshake — so it MUST read the same `programs` the timeline does or the two would disagree.
    for i in 0..plan.instrument_count {
        let ch = (i % 16) as u8;
        let prog = plan
            .programs
            .get(i)
            .copied()
            .unwrap_or_else(|| crate::instrumentation::legacy_program(i));
        // via the AudioSink trait so the adapter speaks one vocabulary to the engine's sink.
        let _ = sink.program_change(ch, prog);
    }

    progress(PlayEvent::Started {
        total_steps,
        grid_ms_per_step,
    });

    // The absolute grid epoch — captured ONCE. Every event fires at `epoch + at_ms`, so the
    // realized tempo equals the plan tempo and outstanding note_offs ring across step
    // boundaries at their true scheduled time.
    let epoch = Instant::now();

    // Playhead (formerly the OpenCV overlay's own tracker): derive the step index from
    // `at_ms / grid` at dispatch time and report only when it advances, so a front-end can
    // track the scan bar without the loop owning a per-step timing the audio no longer uses.
    let mut last_overlay_step: Option<usize> = None;

    'stream: for tev in &plan.timeline {
        // BUG-01: a Ctrl-C stops promptly — the function then returns normally and the
        // sink's Drop runs the all-sound-off panic.
        if cancel.is_cancelled() {
            progress(PlayEvent::Cancelled);
            break;
        }

        // 1) Playhead for the step this event belongs to.
        {
            let step_idx = if grid_ms_per_step == 0 {
                0
            } else {
                (tev.at_ms / grid_ms_per_step) as usize
            }
            .min(total_steps.saturating_sub(1));
            if last_overlay_step != Some(step_idx) {
                last_overlay_step = Some(step_idx);
                progress(PlayEvent::Step(step_idx));
            }
        }

        // 2) Sleep until this event's absolute time `epoch + at_ms`, in <=20ms slices polling
        //    the shutdown flag so Ctrl-C still breaks within ~one slice (BUG-01).
        let target = epoch + Duration::from_millis(tev.at_ms);
        loop {
            let now = Instant::now();
            let remaining = match target.checked_duration_since(now) {
                Some(d) if !d.is_zero() => d,
                _ => break, // event time reached (or already past) — fire it now
            };
            if cancel.is_cancelled() {
                break 'stream;
            }
            std::thread::sleep(remaining.min(SHUTDOWN_POLL_SLICE));
        }
        if cancel.is_cancelled() {
            break;
        }

        // 3) Dispatch via the sink (same vocabulary render's offline synth consumes).
        let res = match tev.cmd {
            MidiCmd::NoteOn {
                channel,
                note,
                velocity,
            } => sink.note_on(channel, note, velocity),
            MidiCmd::NoteOff { channel, note } => sink.note_off(channel, note),
            MidiCmd::ProgramChange { channel, program } => sink.program_change(channel, program),
        };
        if let Err(e) = res {
            progress(PlayEvent::SinkError(e));
        }
    }

    progress(PlayEvent::Finished);
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Shared internals (moved from main.rs)
// ─────────────────────────────────────────────────────────────────────────────

/// Pure-Rust acquisition: decode, analyze (`ImageUnderstanding`), scan (`PureAnalysisSource`).
///
/// Public since S124 (a carried S123 item): the window's analysis cache (`gui::job`) calls this
/// instead of repeating its three steps. It reads only `pipeline` (instrument count, bar
/// thickness, scan step count) and sets NO thread-local register, so an analysis made once can be
/// reused by [`render_with_source`] / [`prepare_play_with_source`] with no change to the piece.
/// Notes, in order: [`RenderNote::ImageLoaded`], [`RenderNote::ImageScanned`],
/// [`RenderNote::GlobalFeatures`].
pub fn acquire(
    pipeline: &PipelineArgs,
    image: ImageInput<'_>,
    notes: &mut NoteSink<'_>,
) -> Result<(PureAnalysisSource, ImageUnderstanding), RenderError> {
    acquire_shrunk(pipeline, Shrink::Full, image, notes)
}

/// S125 (UI4 § 1, SH5 C-3): [`acquire`] from a shrunk copy of the picture. The ONE place a shrink is
/// applied, for the CLI and the window alike: after the decode (the gate has already applied the
/// ORIGINAL picture's caps, so a shrink never raises one), before both `understand_image_pure` and
/// `PureAnalysisSource::extract`, so both analyse the same pixels.
///
/// The floor is checked first ([`crate::shrink::check`]): an infeasible level is refused with
/// [`RenderError::Shrink`] before anything is reported or analysed. Then
/// [`RenderNote::ImageLoaded`] reports the decoded size, and [`crate::shrink::apply`] makes the
/// smaller copy and drops the full-size buffer. `Shrink::Full` makes no resize call, so the
/// default is byte-identical to the pre-S125 `acquire`.
pub fn acquire_shrunk(
    pipeline: &PipelineArgs,
    shrink: Shrink,
    image: ImageInput<'_>,
    notes: &mut NoteSink<'_>,
) -> Result<(PureAnalysisSource, ImageUnderstanding), RenderError> {
    use crate::pure_analysis::{
        load_pure_image, load_pure_image_bytes, understand_image_pure, PureImageSource,
    };
    let engine_config = pipeline_to_engine_config(pipeline);
    let instrument_count = engine_config.num_instruments;
    let bar_thickness_frac = engine_config.bar_thickness_frac;
    let num_steps = pipeline.steps;

    let img = match image {
        ImageInput::Path(p) => load_pure_image(&PureImageSource::UserPath(p.to_path_buf()))?,
        ImageInput::Bytes(b) => load_pure_image_bytes(b)?,
    };
    crate::shrink::check(img.width(), img.height(), shrink)?;
    notes(RenderNote::ImageLoaded {
        width: img.width(),
        height: img.height(),
    })?;
    let img = crate::shrink::apply(img, shrink)?;
    // S37: the plan-first composer reads the SAME analyze_global_pure stats that
    // global_features() exposes — derive the understanding off the same `img` here so it
    // is live at the engine-build site without changing the source-extraction path.
    let understanding = understand_image_pure(img.as_rgb())?;
    let src = PureAnalysisSource::extract(
        &img,
        instrument_count,
        bar_thickness_frac,
        num_steps,
        None,
    )?;
    notes(RenderNote::ImageScanned {
        steps: src.step_count(),
    })?;
    notes(RenderNote::GlobalFeatures(src.global_features()))?;
    Ok((src, understanding))
}

/// MB-5 (lead ruling L-32): the longest piece the library renders or plays, in milliseconds
/// (10 minutes). The shipped mappings' worst case is about 160 s plus the tail (`s124-SH` MB-5), so
/// no render of the shipped table reaches it; a hostile or broken mappings table can (one changed
/// number measured a 42-minute piece and a 446 MB WAV).
pub const MAX_PIECE_MS: u64 = 600_000;

/// The start of the MB-5 refusal text, so a front-end that knows which mappings were used can
/// name the file and the fields after it (`main.rs` does).
pub const PIECE_TOO_LONG_PREFIX: &str = "This piece would last";

/// MB-5: refuse when the timeline's last event plus `tail_ms` exceeds [`MAX_PIECE_MS`]. Runs
/// before anything is sized from the timeline's length.
fn check_piece_length(events: &[TimedMidiEvent], tail_ms: u64) -> Result<(), RenderError> {
    let last = events.iter().map(|e| e.at_ms).max().unwrap_or(0);
    let total = last.saturating_add(tail_ms);
    if total > MAX_PIECE_MS {
        let secs = total / 1000;
        return Err(RenderError::Refused(format!(
            "{PIECE_TOO_LONG_PREFIX} {}:{:02}:{:02}; the limit is 10 minutes. Nothing was rendered. \
             (The mappings in use make each step very long or the form very long.)",
            secs / 3600,
            (secs / 60) % 60,
            secs % 60
        )));
    }
    Ok(())
}

/// Error-safe bracket for the process-global event dump: if this render armed it and is left
/// before the explicit [`DumpArmGuard::disarm`] (early `?` return or unwind), `Drop` disarms.
/// After the explicit disarm it does nothing, so it can never disarm a LATER render's dump.
struct DumpArmGuard {
    armed: bool,
}

impl DumpArmGuard {
    /// The success-path disarm, at the historical point (before serializing).
    fn disarm(&mut self) {
        crate::expression_dump::disarm();
        self.armed = false;
    }
}

impl Drop for DumpArmGuard {
    fn drop(&mut self) {
        if self.armed {
            crate::expression_dump::disarm();
        }
    }
}

/// What `compose` hands back: the composed engine plus the piece-level constants.
struct Composed {
    engine: PipelineEngine,
    programs: Vec<u8>,
    total_steps: usize,
    grid_ms_per_step: u64,
    instrument_count: usize,
}

/// Set the seed / expression / cadence registers, resolve per-voice programs, build the engine
/// and install the composer plan — the block `render` and `play` both ran, in the order they
/// ran it.
fn compose<S: FeatureSource>(
    req: &RenderRequest,
    source: &S,
    understanding: Option<&ImageUnderstanding>,
    mappings: &MappingTable,
) -> Result<Composed, RenderError> {
    let engine_config = pipeline_to_engine_config(&req.pipeline);
    let instrument_count = engine_config.num_instruments;
    let ms_per_step = engine_config.ms_per_step;

    // ── S41/S91-S2: set the composition-seed register BEFORE composing (freeze-safe entry). ──
    // `--seed N` ⇒ Some(N), the operator's explicit override, unchanged.
    // ABSENT ⇒ was `None` = `thread_rng()`: every section drew a fresh random progression on every
    // run, so the same image rendered to different harmony each time while doing NOTHING to
    // separate one image from another. S91 replaces that default with a seed DERIVED FROM THE
    // IMAGE's own feature vector: the same image now yields the same plan, in every process, every
    // run. This is a REPRODUCIBILITY fix and is claimed as nothing more — a hash-seeded draw over a
    // progression catalogue is a pseudo-random differentiator, not a musical one.
    // engine.rs is never touched — the register is read inside chord_engine::pick_progression.
    // S122: with no `understanding` (the OpenCV arm) there is no feature vector to derive a seed
    // from, so the explicit `--seed` alone is passed (the pre-S91 behaviour on that arm).
    crate::seed::set_composition_seed(match understanding {
        Some(understanding) => Some(
            req.seed
                .unwrap_or_else(|| crate::composition::image_seed(understanding)),
        ),
        None => req.seed,
    });

    // ── S107/X1 — arm the EXPRESSION axis, at the SAME freeze-safe site and in the SAME manner
    //    as the seed register above: set ONCE, here, BEFORE compose runs; the planner stamps it
    //    onto every section's OrchestrationProfile; the realizer reads it off the section it
    //    already borrows. `engine.rs` untouched, no caller signature changed. Absent the flag the
    //    register stays `inert()` and every expression delta is exactly 0.0.
    // ── S113/v2 — resolve the v2 term set BEFORE the register is set, and refuse every request
    //    that could render a null result rather than rendering one quietly.
    //
    //    RULING — `--v2` REQUIRES `--expression on`; it neither implies it nor excludes it.
    //    (i) E3's arithmetic is gated on `expr.enabled`, so a v2 term physically cannot fire with
    //    expression off; (ii) the ladder's own rung 1 IS `--expression on`, and every higher rung
    //    is defined as "v1 plus X"; (iii) E3's pre-registered acceptance figures are stated against
    //    an X1-armed baseline, so the gate is uninterpretable on any other one; (iv) there is
    //    exactly ONE enable path and ONE enable assertion in this binary, and adding a second
    //    would double the surface of the very trap that assertion exists to close.
    //    Requiring the flag rather than implying it keeps the arming state readable off the
    //    command line without knowing a coupling rule.
    let v2 = req.v2.unwrap_or_default();
    if !v2.is_empty() && !req.expression.is_on() {
        return Err(RenderError::Refused(
            "--v2 requires `--expression on`. The v2 terms are deltas on top of the X1/E1 base \
             profile and are gated on it in the realizer, so with expression off they would fire \
             on nothing; refusing to render a build that would look like a null result."
                .into(),
        ));
    }
    let unbuilt = v2.unbuilt();
    if !unbuilt.is_empty() {
        return Err(RenderError::Refused(format!(
            "--v2: term(s) `{}` are NOT IMPLEMENTED in this binary (S113 built `e3`, `e2` and \
             `ta`). Refusing rather than silently arming a subset — a rung that renders without \
             its named term is indistinguishable from the rung below it.",
            unbuilt.join("`, `")
        )));
    }

    crate::composition::expression_register::set(if req.expression.is_on() {
        crate::composition::ExpressionProfile::e1_entrance_destack()
            .with_role_accent_scale(v2.e3)
            .with_metric_hierarchy(v2.e2)
            .with_ineq_frac(if v2.ta {
                crate::composition::ExpressionProfile::SHIPPED_INEQ_FRAC
            } else {
                0.0
            })
    } else {
        crate::composition::ExpressionProfile::inert()
    });

    // ── S79 cadence-figure differentiation: ARMED S80 (production default). ──
    // Per-image + within-piece-developing cadential figure, grounded on LIVE `understanding.*`
    // extraction — NOT the hardcoded test fixture the QG flagged. Set here BEFORE compose, the
    // `set_composition_seed` discipline, freeze-safe (engine.rs untouched; the register is read
    // inside chord_engine::realize_rhythm + composition::plan). Only the not(opencv)+pure-analysis
    // arm binds a live `understanding`; the opencv/bare arm has none → it keeps the neutral()
    // legacy figure (do NOT fabricate features), exactly as the S74b `programs` floor does.
    // A/B evidence: examples/poc_s79_rhythm_diff.rs + renders/s79-rhythm-diff/LISTEN-S79.md.
    // S122 round 2: the `None` arm RESETS the register to `neutral()` (the `thread_local!`
    // initializer — what HEAD's opencv arm always saw, in a fresh process). Without this, a
    // caller that renders repeatedly on one thread (a GUI worker) would inherit the previous
    // pure render's figure here. Every register this call reads is now set on every path.
    match understanding {
        Some(understanding) => {
            crate::chord_engine::set_cadence_development(crate::chord_engine::CadenceFigure {
                shape: Some(crate::chord_engine::select_cadence_shape(
                    understanding.complexity,
                    understanding.aspect_ratio,
                )),
                drive: crate::chord_engine::edge_to_drive(understanding.edge_activity),
            })
        }
        None => crate::chord_engine::clear_cadence_development(),
    }

    // ── S74b/B5: per-voice GM program vector (image-feature → ensemble). Computed ONCE, BEFORE
    //    `PipelineEngine::new` consumes `mappings`. Freeze-safe: reads DATA + the live
    //    `understanding`; touches neither the engine nor the plan. Absent/empty `instrumentation`
    //    axis ⇒ every voice == legacy `(i*7)%128` (byte-identical to pre-B5). It is a piece-level
    //    constant (matches the per-channel-at-t=0 emission model). ──
    let programs: Vec<u8> = match understanding {
        Some(understanding) => match mappings.composition.as_ref() {
            // S99: WHOLE-ENSEMBLE selection is the MAJORITY arm — an image claimed by a curated
            // `instrumentation` rule gets that catalogue row verbatim, so the ensemble stays the
            // piece's sonic identity and the six ear-certified goldens reproduce. The S98/B2-a
            // per-seat draws survive as the MINORITY WILDCARD arm, firing only on images no curated
            // rule claimed (~6.4% of the keying-feature space). See `select_programs_live`.
            Some(comp) => crate::instrumentation::select_programs_live(
                understanding,
                comp,
                instrument_count,
                &voice_bands(source, instrument_count),
            ),
            None => (0..instrument_count)
                .map(crate::instrumentation::legacy_program)
                .collect(),
        },
        // opencv-cfg / bare arm: no `understanding` is bound here → per-voice legacy floor.
        // (opencv render arm inspection-validated only — not compiled on this box; B5 §4.3.)
        None => (0..instrument_count)
            .map(crate::instrumentation::legacy_program)
            .collect(),
    };

    // `PipelineEngine::new` consumes its table BY VALUE; the caller keeps `mappings` (a GUI
    // renders many times). `rebuild_mapping_table` is the lossless copy the engine already uses
    // internally — an exhaustive struct literal, so a new field cannot be silently dropped.
    let mut engine = PipelineEngine::new(rebuild_mapping_table(mappings), engine_config);

    // ── S37: install the COMPOSER plan (pure-Rust / default path). The plan-first composer
    //    is the audible path; the S13 flat path is the fallback when mappings.json has no
    //    `composition` block (compose_from_image -> false). The OpenCV arm stays on the legacy
    //    set_features_global path (spec §4 Option A). ──
    match understanding {
        Some(understanding) => {
            let composed: bool = engine.compose_from_image(understanding);
            if !composed {
                // No `composition` block in mappings.json -> keep the S13 flat path, byte-identical
                // to the pre-S37 binary.
                engine.set_features_global(&source.global_features());
            }
        }
        None => engine.set_features_global(&source.global_features()),
    }

    // `total_steps` AND the deterministic ms-grid come FROM THE PLAN when composing, NOT from
    // source.step_count()/config. Read them back via the read-only accessor (engine.rs:341).
    let (total_steps, grid_ms_per_step): (usize, u64) = match engine.composition() {
        Some(plan) => (plan.total_steps, plan.key_tempo.base_ms_per_step),
        None => (source.step_count(), ms_per_step), // legacy fallback
    };

    Ok(Composed {
        engine,
        programs,
        total_steps,
        grid_ms_per_step,
        instrument_count,
    })
}

/// The S107/X1 + S113 read-back guard: with `--expression on`, assert the resolved profile (and
/// each requested v2 field) is set on EVERY composed section. A no-op when expression is off.
/// Returns `(armed, total)` when it verified.
fn verify_expression(
    req: &RenderRequest,
    engine: &PipelineEngine,
    notes: &mut NoteSink<'_>,
) -> Result<Option<(usize, usize)>, RenderError> {
    // S107/X1 gate discipline, step 2 (s106-03 §5.1 (b1)): read the RESOLVED profile back out
    // IN THE SAME PROCESS THAT RENDERS and assert it is actually enabled. A byte-identity check
    // with the axis off passes perfectly on an enabled path that was never wired to the flag at
    // all; this is the assertion that catches that, and it is the mirror of the identity check —
    // neither alone is a gate. Read off the composed plan's own sections, not off the register,
    // so it certifies the value the realizer will actually see.
    if !req.expression.is_on() {
        return Ok(None);
    }
    let v2 = req.v2.unwrap_or_default();
    let armed = engine
        .composition()
        .map(|p| {
            p.sections
                .iter()
                .filter(|s| s.orchestration.expression.enabled)
                .count()
        })
        .unwrap_or(0);
    let total = engine.composition().map(|p| p.sections.len()).unwrap_or(0);
    if armed == 0 || armed != total {
        return Err(RenderError::Refused(format!(
            "--expression on: the resolved profile is NOT enabled on every section                  ({armed}/{total}). The axis is wired to the flag but not to the realizer's                  input; refusing to render a build that would look like a null result."
        )));
    }
    notes(RenderNote::ExpressionVerified { armed, total })?;

    // S113 — the SAME assertion, PER FIELD. `s113-09` §6.3 hole 2 names this exactly: a
    // composite axis can satisfy `enabled == true` on every section while a sub-flag is false
    // everywhere, and the `enabled`-only check above would pass on a two-thirds-wired build.
    // That is trap (a1) — "the gate proves the gate" — reappearing one level down INSIDE the
    // guard written to close it. Each armed v2 term therefore asserts its own field, read back
    // off the composed plan's own sections in the same process that renders.
    // One row per BUILT v2 term. Adding a term to the ladder without adding its row here
    // would reintroduce exactly the two-thirds-wired build this guard exists to refuse, so the
    // rows are kept beside the `V2Spec` fields they mirror.
    type ExprPred = fn(&crate::composition::ExpressionProfile) -> bool;
    let v2_fields: [(bool, &str, &str, ExprPred); 3] = [
        (v2.e3, "E3", "role_accent_scale", |e| e.role_accent_scale),
        (v2.e2, "E2'", "metric_hierarchy", |e| e.metric_hierarchy),
        (v2.ta, "T-A", "ineq_frac", |e| e.ineq_frac > 0.0),
    ];
    for (requested, label, field, pred) in v2_fields {
        if !requested {
            continue;
        }
        let field_armed = engine
            .composition()
            .map(|p| {
                p.sections
                    .iter()
                    .filter(|s| s.orchestration.expression.enabled && pred(&s.orchestration.expression))
                    .count()
            })
            .unwrap_or(0);
        if field_armed == 0 || field_armed != total {
            return Err(RenderError::Refused(format!(
                "--v2 {label}: `{field}` is NOT set on every section ({field_armed}/{total}). \
                 The term is wired to the flag but not to the realizer's input; refusing to \
                 render a build that would look like a null result."
            )));
        }
        notes(RenderNote::V2TermVerified {
            label,
            field,
            armed: field_armed,
            total,
        })?;
    }
    Ok(Some((armed, total)))
}

/// S40 (divergence #2): the ONE shared event-timeline builder consumed by BOTH
/// `render --wav` and live `play`, so the two paths are identical BY CONSTRUCTION.
///
/// For every step `0..total_steps` and every `NoteEvent` the engine realizes, this emits:
///   * a `NoteOn`  at `step_idx*grid + offset_ms`,
///   * a `NoteOff` at `step_idx*grid + offset_ms + hold_ms` — with NO grid/step-boundary
///     clamp, so a note whose `hold_ms` exceeds one grid step (the common pad/cadence/
///     legato `1.10–1.20×step_ms` overlap, `chord_engine.rs`) RINGS THROUGH into the next
///     step exactly as render's offline synth honors it.
///
/// Plus the initial per-channel `ProgramChange`s at `at_ms = 0` (prog `(i*7)%128`), the
/// same scheme both paths used inline before.
///
/// The whole vector is STABLE-sorted by `at_ms`. Because the list is global (not per-step),
/// a step-N tail `NoteOff` and a step-(N+1) head `NoteOn` interleave in true time order —
/// which is precisely what the per-step-batched live loop used to serialize away. The sort
/// places `NoteOff` before `NoteOn` at an EQUAL `at_ms` (a small command-rank key), matching
/// render's historical "off-before-on" effect at coincident timestamps and avoiding a
/// same-pitch retrigger collision when one note's release lands exactly on the next's onset.
///
/// DETERMINISTIC / JITTER-FREE on purpose (design-s40-path-divergence-2 §3 WO-1a): the live
/// monitor now exactly equals the WAV gate. The old live ~15% `hold_ms` jitter is dropped so
/// `play == render`; if humanization is wanted later it can be applied as ONE equal pass to
/// both paths, out of scope here.
pub fn build_step_event_timeline<S: FeatureSource>(
    engine: &PipelineEngine,
    source: &S,
    total_steps: usize,
    grid_ms_per_step: u64,
    instrument_count: usize,
    programs: &[u8],
) -> Vec<crate::synth_sink::TimedMidiEvent> {
    use crate::synth_sink::{MidiCmd, TimedMidiEvent};

    let mut events: Vec<TimedMidiEvent> = Vec::new();

    // Initial per-channel programs (S74b/B5): voice `i` gets the resolved ensemble program
    // `programs[i]`. An empty/short `programs` (or the pre-B5 control) falls to the byte-identical
    // legacy floor `legacy_program(i) == (i*7)%128`, so an absent instrumentation axis is a no-op.
    for i in 0..instrument_count {
        let ch = (i % 16) as u8;
        let prog = programs
            .get(i)
            .copied()
            .unwrap_or_else(|| crate::instrumentation::legacy_program(i));
        events.push(TimedMidiEvent {
            at_ms: 0,
            cmd: MidiCmd::ProgramChange {
                channel: ch,
                program: prog,
            },
        });
    }

    crate::expression_dump::set_plan(total_steps, grid_ms_per_step, instrument_count);

    for step_idx in 0..total_steps {
        // In compose mode the per-step cadence is the PLAN's base_ms_per_step; falls back to
        // config when not composing (grid_ms_per_step == ms_per_step in that branch).
        let step_base_ms = step_onset_offset_ms(step_idx, grid_ms_per_step);
        // S107/X0 DIAGNOSTIC: hand the dump this step's absolute grid anchor. This is the ONLY
        // place both the anchor and the realizer's musical context are simultaneously in
        // reach — `realize_step` knows the role and the phrase but not `step_base_ms`, and the
        // frozen `engine.rs` in between knows neither. Disarmed by default: one atomic load.
        crate::expression_dump::begin_step(step_idx, step_base_ms);
        for dec in engine.decide_step(source, step_idx) {
            for ev in &dec.events {
                let on_ms = step_base_ms + ev.offset_ms;
                events.push(TimedMidiEvent {
                    at_ms: on_ms,
                    cmd: MidiCmd::NoteOn {
                        channel: dec.channel,
                        note: ev.note,
                        velocity: ev.velocity,
                    },
                });
                // NO grid clamp — honor the deliberate cross-step overlap exactly as render did.
                events.push(TimedMidiEvent {
                    at_ms: on_ms + ev.hold_ms,
                    cmd: MidiCmd::NoteOff {
                        channel: dec.channel,
                        note: ev.note,
                    },
                });
            }
        }
    }

    // STABLE sort by `at_ms` ONLY — this reproduces EXACTLY what render's offline synth did
    // historically: render appended events in insertion order (program changes, then step 0's
    // on/off, step 1's, …) and `render_events_to_stereo` applied `sort_by_key(|e| e.at_ms)`
    // (stable) before synthesis. Extracting that here is therefore a pure refactor — the WAV is
    // byte-identical to the pre-extraction render. The stable sort PRESERVES insertion order at
    // equal `at_ms`, which already yields the desired effects at coincident timestamps:
    //   * program changes (inserted first, at_ms 0) precede the first onsets;
    //   * a cross-step same-pitch release inserted in step N precedes that pitch's retrigger
    //     inserted in step N+1 (off-before-on) — the case that matters for voice freeing.
    // Crucially we do NOT add a command-rank key: that would reorder coincident events vs. the
    // historical insertion order and move render's bytes. (Verified: rank-keyed sort changed the
    // WAV md5; at_ms-only stable sort keeps it byte-identical.)
    events.sort_by_key(|e| e.at_ms);
    events
}

/// S98/B2-a — step-aggregate each voice's own image band into the per-voice selection input.
///
/// `scan_image` already splits the image into `instrument_count` bands and voice `i` already
/// sounds band `i`, so band `i` is the principled per-voice timbre driver and it costs no new
/// extraction — this reads the SAME `FeatureSource` rows the engine already pulls. Aggregation is
/// the arithmetic mean of band `i`'s `avg_brightness` over every scan step, which turns a per-step
/// series into the piece-level constant the `at_ms = 0` `ProgramChange` emission model wants.
///
/// Deviation-coding against the image mean happens downstream in
/// `instrumentation::select_programs_per_voice` (it needs all bands at once). An empty return —
/// impossible here for `instrument_count > 0` — routes selection back to the whole-image path.
fn voice_bands<S: FeatureSource>(
    source: &S,
    instrument_count: usize,
) -> Vec<crate::instrumentation::BandFeatures> {
    let steps = source.step_count();
    if steps == 0 || instrument_count == 0 {
        return Vec::new();
    }
    let mut acc = vec![0f64; instrument_count];
    for s in 0..steps {
        for (i, b) in source
            .scan_bar_features(s, instrument_count)
            .iter()
            .enumerate()
            .take(instrument_count)
        {
            acc[i] += b.avg_brightness as f64;
        }
    }
    acc.into_iter()
        .map(|sum| crate::instrumentation::BandFeatures {
            brightness: (sum / steps as f64) as f32,
        })
        .collect()
}

/// Absolute-grid onset offset (ms) for a step, measured from the run epoch.
///
/// This is the ONE shared timing formula for both render and live `play`: step `N` is
/// anchored at `N * grid_ms_per_step` regardless of how long its notes actually sound, so
/// a step whose voices end early (or rest entirely) still occupies its full grid slot
/// instead of being skipped. `render --wav` lays its absolute event grid on this
/// (`step_base_ms`), and the live `play` loop schedules each step's `t0 = epoch + this`,
/// so the two paths play the SAME composition at the SAME plan tempo (S40 convergence).
///
/// Pure / total: `step_onset_offset_ms(0, _) == 0`, monotonic non-decreasing in
/// `step_idx`, and exactly `step_idx * grid_ms_per_step`.
fn step_onset_offset_ms(step_idx: usize, grid_ms_per_step: u64) -> u64 {
    step_idx as u64 * grid_ms_per_step
}

#[cfg(test)]
mod tests {
    use super::{build_step_event_timeline, step_onset_offset_ms};
    use crate::engine::{
        EngineConfig, FeatureSource, GlobalFeatures, PipelineEngine, ScanBarFeatures,
    };
    use crate::mapping_loader::load_mappings;
    use crate::synth_sink::MidiCmd;

    /// S40: the absolute-grid onset formula shared by `render --wav` and live `play`.
    /// Real-time playback can't be unit-tested, but this pure helper IS the load-bearing
    /// math — both paths anchor step N at `N * grid_ms_per_step`, so locking it here proves
    /// the convergence regardless of the (untestable) sleep.
    #[test]
    fn onset_step_zero_is_zero() {
        // Step 0 must start at the run epoch (offset 0) for ANY grid.
        assert_eq!(step_onset_offset_ms(0, 0), 0);
        assert_eq!(step_onset_offset_ms(0, 250), 0);
        assert_eq!(step_onset_offset_ms(0, 912), 0);
    }

    #[test]
    fn onset_equals_step_times_grid() {
        // Exactly step_idx * grid_ms_per_step — the same formula render's step_base_ms uses.
        assert_eq!(step_onset_offset_ms(1, 912), 912);
        assert_eq!(step_onset_offset_ms(3, 250), 750);
        assert_eq!(step_onset_offset_ms(29, 912), 29 * 912);
    }

    #[test]
    fn onset_is_monotonic_nondecreasing() {
        // Onsets never go backwards as the step index advances (the grid never compresses).
        let grid = 626u64;
        let mut prev = 0u64;
        for step in 0..64usize {
            let onset = step_onset_offset_ms(step, grid);
            assert!(onset >= prev, "onset must be non-decreasing in step_idx");
            prev = onset;
        }
    }

    #[test]
    fn onset_zero_grid_is_always_zero() {
        // A degenerate/zero grid keeps every onset at 0 (no panic, total function).
        for step in 0..16usize {
            assert_eq!(step_onset_offset_ms(step, 0), 0);
        }
    }

    // ── S40 divergence #2: the shared-timeline equality guard ─────────────────────────
    //
    // A synthetic FeatureSource + a real engine, so `build_step_event_timeline` runs the
    // genuine `decide_step` realize path. The properties asserted below are what make
    // `play == render` BY CONSTRUCTION: both paths call this one builder, so if its output
    // is (a) deterministic, (b) globally sorted, (c) off-before-on at equal timestamps, and
    // (d) overlap-preserving (note_offs may exceed the next step's grid base, NO clamp), then
    // the live stream and the WAV render consume an identical event list.

    /// A behaviour-neutral synthetic source: a fixed global feature vector + `steps` rows of
    /// `num_instruments` zeroed scan bars. Enough to drive the engine deterministically.
    struct SyntheticSource {
        global: GlobalFeatures,
        rows: Vec<Vec<ScanBarFeatures>>,
    }

    fn syn_bar(idx: usize) -> ScanBarFeatures {
        ScanBarFeatures {
            bar_index: idx,
            avg_hue: 200.0,
            avg_saturation: 40.0,
            avg_brightness: 50.0,
            edge_density: 0.2,
            texture_laplacian_var: 1.0,
            hue_hist: vec![0.0; 12],
        }
    }

    impl SyntheticSource {
        fn new(steps: usize, num_instruments: usize) -> Self {
            let global = GlobalFeatures {
                avg_hue: 200.0,
                avg_saturation: 40.0,
                avg_brightness: 50.0,
                edge_density: 0.2,
                hue_spread: 0.15,
                texture_laplacian_var: 1.0,
                shape_complexity: 0.1,
                aspect_ratio: 1.5,
            };
            let rows = (0..steps)
                .map(|_| (0..num_instruments).map(syn_bar).collect())
                .collect();
            SyntheticSource { global, rows }
        }
    }

    impl FeatureSource for SyntheticSource {
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
            row
        }
        fn step_count(&self) -> usize {
            self.rows.len()
        }
    }

    /// Build a small engine + synthetic source for the timeline tests. Returns
    /// `(engine, source, total_steps, grid_ms_per_step, instrument_count)` exactly as both
    /// the render and play call sites pass to `build_step_event_timeline`.
    fn fixture() -> (PipelineEngine, SyntheticSource, usize, u64, usize) {
        let mappings = load_mappings("assets/mappings.json").expect("mappings load");
        let instrument_count = 4usize;
        let total_steps = 8usize;
        let cfg = EngineConfig {
            num_instruments: instrument_count,
            ..EngineConfig::default()
        };
        let mut engine = PipelineEngine::new(mappings, cfg);
        let source = SyntheticSource::new(total_steps, instrument_count);
        engine.set_features_global(&source.global_features());
        let grid_ms_per_step = 250u64;
        (
            engine,
            source,
            total_steps,
            grid_ms_per_step,
            instrument_count,
        )
    }

    #[test]
    fn timeline_is_sorted_by_at_ms() {
        let (engine, source, total_steps, grid, instruments) = fixture();
        let tl = build_step_event_timeline(&engine, &source, total_steps, grid, instruments, &[]);
        assert!(!tl.is_empty(), "synthetic source should realize events");
        for w in tl.windows(2) {
            assert!(
                w[0].at_ms <= w[1].at_ms,
                "timeline must be non-decreasing in at_ms (got {} then {})",
                w[0].at_ms,
                w[1].at_ms
            );
        }
    }

    #[test]
    fn timeline_same_pitch_off_precedes_cross_step_on() {
        // The builder uses a STABLE sort by at_ms over insertion order (program-changes, then
        // step 0's on/off, step 1's, …) — byte-identical to render. The load-bearing coincident
        // case is a same-(channel,note) release that lands exactly on that pitch's next onset:
        // the release was inserted in an EARLIER step, so the stable sort keeps it BEFORE the
        // retrigger (off-before-on), freeing the voice. Assert that invariant directly.
        let (engine, source, total_steps, grid, instruments) = fixture();
        let tl = build_step_event_timeline(&engine, &source, total_steps, grid, instruments, &[]);
        for i in 0..tl.len() {
            if let MidiCmd::NoteOn {
                channel: c1,
                note: n1,
                ..
            } = tl[i].cmd
            {
                // Scan events at the SAME at_ms: a matching NoteOff for this (channel,note)
                // must not appear AFTER this NoteOn (it would mean on-before-off → stuck voice).
                for ev in tl.iter().skip(i + 1) {
                    if ev.at_ms != tl[i].at_ms {
                        break;
                    }
                    if let MidiCmd::NoteOff {
                        channel: c2,
                        note: n2,
                    } = ev.cmd
                    {
                        assert!(
                            !(c1 == c2 && n1 == n2),
                            "a same-pitch NoteOff at equal at_ms must precede the NoteOn, not follow it"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn timeline_preserves_cross_step_overlap() {
        // The whole point of the fix: a note's NoteOff may land PAST the next step's grid
        // base (no clamp), so the deliberate pad/cadence/legato ring-through is honored.
        let (engine, source, total_steps, grid, instruments) = fixture();
        let tl = build_step_event_timeline(&engine, &source, total_steps, grid, instruments, &[]);

        // Map each (channel, note) NoteOn to the step it belongs to, then check at least one
        // NoteOff fires at an at_ms >= (its step + 1) * grid — i.e. it rings into the next step.
        let mut overlap_seen = false;
        for ev in &tl {
            if let MidiCmd::NoteOff { .. } = ev.cmd {
                // The step this off's parent on belonged to is floor(on_ms/grid); but here we
                // only have the off time. An off at_ms that is NOT a multiple-of-grid boundary
                // and exceeds the next grid line for some step demonstrates a non-clamped hold.
                // Concretely: if any off lands strictly inside a later step than where grid
                // alignment would clamp it, overlap is preserved.
                let step_of_off_base = ev.at_ms / grid; // which grid slot the off falls in
                                                        // If an off falls in slot k but k*grid != at_ms, it ended mid-slot k,
                                                        // which for a note onset in slot k-1 (or earlier) means ring-through.
                if step_of_off_base >= 1 && ev.at_ms % grid != 0 {
                    overlap_seen = true;
                    break;
                }
            }
        }
        assert!(
            overlap_seen,
            "at least one NoteOff must ring past a step boundary (overlap preserved, NO grid clamp)"
        );
    }

    #[test]
    fn timeline_is_deterministic_single_source_for_both_paths() {
        // One builder, called twice with identical inputs, yields an IDENTICAL event list.
        // This is the load-bearing invariant: render and play both call this fn, so identical
        // output here == they consume the same timeline == play == render by construction.
        let (engine, source, total_steps, grid, instruments) = fixture();
        let a = build_step_event_timeline(&engine, &source, total_steps, grid, instruments, &[]);
        let b = build_step_event_timeline(&engine, &source, total_steps, grid, instruments, &[]);
        assert_eq!(
            a, b,
            "the shared timeline must be deterministic (jitter-free) so play == render"
        );
    }

    #[test]
    fn timeline_has_program_change_init_at_zero() {
        // Each channel gets its ProgramChange at at_ms = 0, sorted ahead of the first onsets.
        let (engine, source, total_steps, grid, instruments) = fixture();
        let tl = build_step_event_timeline(&engine, &source, total_steps, grid, instruments, &[]);
        let pc_count = tl
            .iter()
            .filter(|e| matches!(e.cmd, MidiCmd::ProgramChange { .. }))
            .count();
        assert_eq!(
            pc_count, instruments,
            "one ProgramChange per instrument channel"
        );
        for e in tl.iter().take(instruments) {
            assert_eq!(e.at_ms, 0, "program changes anchor at t=0");
            assert!(matches!(e.cmd, MidiCmd::ProgramChange { .. }));
        }
    }
}

// S122 — tests for the items Phase 0 ADDED (the block above moved verbatim from main.rs).
#[cfg(test)]
mod s122_tests {
    use super::*;
    use crate::cli::{Cli, Command};
    use crate::mapping_loader::load_mappings;
    use crate::synth_sink::MidiCmd;
    use clap::Parser;

    fn play_args(argv: &[&str]) -> PlayArgs {
        match Cli::try_parse_from(argv).expect("parse").command {
            Command::Play(p) => p,
            other => panic!("expected play, got {other:?}"),
        }
    }

    fn render_args(argv: &[&str]) -> RenderArgs {
        match Cli::try_parse_from(argv).expect("parse").command {
            Command::Render(r) => r,
            other => panic!("expected render, got {other:?}"),
        }
    }

    /// A small PNG with real structure (gradient + block), so analysis has features to read.
    fn test_png() -> Vec<u8> {
        let img = image::RgbImage::from_fn(64, 40, |x, y| {
            if (20..36).contains(&x) && (10..26).contains(&y) {
                image::Rgb([230, 40, 30])
            } else {
                image::Rgb([(x * 4) as u8, (y * 6) as u8, 120])
            }
        });
        let mut png = Vec::new();
        image::DynamicImage::ImageRgb8(img)
            .write_to(
                &mut std::io::Cursor::new(&mut png),
                image::ImageOutputFormat::Png,
            )
            .expect("encode png");
        png
    }

    #[derive(Default)]
    struct RecordingSink {
        calls: Vec<MidiCmd>,
    }

    impl AudioSink for RecordingSink {
        fn note_on(&mut self, channel: u8, note: u8, velocity: u8) -> Result<(), AudioSinkError> {
            self.calls.push(MidiCmd::NoteOn {
                channel,
                note,
                velocity,
            });
            Ok(())
        }
        fn note_off(&mut self, channel: u8, note: u8) -> Result<(), AudioSinkError> {
            self.calls.push(MidiCmd::NoteOff { channel, note });
            Ok(())
        }
        fn program_change(&mut self, channel: u8, program: u8) -> Result<(), AudioSinkError> {
            self.calls.push(MidiCmd::ProgramChange { channel, program });
            Ok(())
        }
    }

    fn tiny_plan() -> PlayPlan {
        let ev = |cmd| TimedMidiEvent { at_ms: 0, cmd };
        PlayPlan {
            timeline: vec![
                ev(MidiCmd::ProgramChange {
                    channel: 0,
                    program: 5,
                }),
                ev(MidiCmd::NoteOn {
                    channel: 0,
                    note: 60,
                    velocity: 90,
                }),
                ev(MidiCmd::NoteOff {
                    channel: 0,
                    note: 60,
                }),
            ],
            total_steps: 1,
            grid_ms_per_step: 250,
            instrument_count: 2,
            programs: vec![5],
            source_step_count: 1,
        }
    }

    #[test]
    fn validate_refuses_loop_with_non_unity_gain() {
        let mut req = RenderRequest::from(&render_args(&["audiohax", "render", "--loop"]));
        assert!(req.validate().is_ok(), "unity gain + loop is fine");
        req.gain = 0.5;
        let msg = req
            .validate()
            .expect_err("non-unity loop refused")
            .to_string();
        assert!(msg.contains("unity") && msg.contains("gain"), "{msg}");
        req.loop_cue = false;
        assert!(req.validate().is_ok(), "no loop ⇒ any gain");
    }

    #[test]
    fn play_adapter_leaves_render_only_axes_off() {
        let req = RenderRequest::from(&play_args(&["audiohax", "play", "x.png", "--seed", "7"]));
        assert_eq!(req.seed, Some(7));
        assert_eq!(req.expression, Toggle::Off);
        assert_eq!(req.v2, None);
        assert!(!req.loop_cue);
        assert_eq!(req.event_dump, None);
        // The register value `play` now writes equals the thread_local initializer's value.
        assert_eq!(
            crate::composition::ExpressionProfile::inert(),
            crate::composition::expression_register::active()
        );
    }

    #[test]
    fn render_adapter_builds_dump_labels_as_main_did() {
        let with_image = RenderRequest::from(&render_args(&[
            "audiohax",
            "render",
            "assets/images/example.jpg",
            "--wav",
            "out/cue-a.wav",
            "--event-dump",
            "d.json",
        ]));
        assert_eq!(
            with_image.event_dump,
            Some(DumpLabel {
                cue: "cue-a".into(),
                image: "assets/images/example.jpg".into()
            })
        );
        let bundled = RenderRequest::from(&render_args(&[
            "audiohax",
            "render",
            "--wav",
            "x.wav",
            "--event-dump",
            "d.json",
        ]));
        assert_eq!(
            bundled.event_dump.map(|l| l.image),
            Some("<bundled example>".to_string())
        );
        let no_dump = RenderRequest::from(&render_args(&["audiohax", "render", "--wav", "x.wav"]));
        assert_eq!(no_dump.event_dump, None);
    }

    #[test]
    fn render_error_debug_matches_the_boxed_value_main_used_to_return() {
        let text = "--v2 requires `--expression on`. \"quoted\" and\nnewline".to_string();
        let old: Box<dyn std::error::Error> = text.clone().into();
        let new: Box<dyn std::error::Error> = Box::new(RenderError::Refused(text.clone()));
        assert_eq!(format!("{old:?}"), format!("{new:?}"));
        assert_eq!(old.to_string(), new.to_string());

        let a = AnalysisError::Decode("p: bad".into());
        let old: Box<dyn std::error::Error> = Box::new(AnalysisError::Decode("p: bad".into()));
        let new: Box<dyn std::error::Error> = Box::new(RenderError::from(a));
        assert_eq!(format!("{old:?}"), format!("{new:?}"));
        assert_eq!(old.to_string(), new.to_string());
    }

    #[test]
    fn cancel_token_clones_share_one_flag() {
        let t = CancelToken::default();
        let handler_side = t.clone();
        assert!(!t.is_cancelled());
        handler_side.cancel();
        assert!(t.is_cancelled());
    }

    /// BUG-01 at the seam: a cancel already set stops before the first event, `play` returns
    /// Ok (so the CALLER drops the sink), and only the eager program changes reached the sink.
    #[test]
    fn play_cancelled_returns_ok_and_dispatches_no_events() {
        let plan = tiny_plan();
        let mut sink = RecordingSink::default();
        let cancel = CancelToken::default();
        cancel.cancel();
        let mut seen = Vec::new();
        let res = play(&plan, &mut sink, &cancel, &mut |e| {
            seen.push(format!("{e:?}"))
        });
        assert!(res.is_ok());
        assert_eq!(seen.len(), 3, "{seen:?}");
        assert!(seen[0].starts_with("Started"));
        assert_eq!(seen[1], "Cancelled");
        assert_eq!(seen[2], "Finished");
        // Eager init: one program change per instrument (programs[0], then legacy floor).
        assert_eq!(
            sink.calls,
            vec![
                MidiCmd::ProgramChange {
                    channel: 0,
                    program: 5
                },
                MidiCmd::ProgramChange {
                    channel: 1,
                    program: crate::instrumentation::legacy_program(1)
                },
            ]
        );
    }

    #[test]
    fn play_streams_the_timeline_in_order_through_any_sink() {
        let plan = tiny_plan();
        let mut sink = RecordingSink::default();
        let mut seen = Vec::new();
        // Through a trait object too (`?Sized`), the way main.rs passes `Box<dyn AudioSink>`.
        let dyn_sink: &mut dyn AudioSink = &mut sink;
        play(&plan, dyn_sink, &CancelToken::default(), &mut |e| {
            seen.push(format!("{e:?}"))
        })
        .expect("play");
        assert_eq!(
            seen,
            vec![
                "Started { total_steps: 1, grid_ms_per_step: 250 }",
                "Step(0)",
                "Finished"
            ]
        );
        let streamed: Vec<MidiCmd> = sink.calls[plan.instrument_count..].to_vec();
        let expect: Vec<MidiCmd> = plan.timeline.iter().map(|e| e.cmd).collect();
        assert_eq!(streamed, expect);
    }

    /// S40 by construction, now across the seam: `render()` synthesizes exactly the timeline
    /// `prepare_play()` streams, and a Path input and a Bytes input of the same PNG compose the
    /// same piece.
    #[test]
    fn render_and_play_share_one_timeline_and_inputs_agree() {
        let mappings = load_mappings("assets/mappings.json").expect("mappings");
        let png = test_png();
        let req = RenderRequest::from(&render_args(&["audiohax", "render", "--seed", "11"]));
        let mut quiet = |_: RenderNote<'_>| -> Result<(), RenderError> { Ok(()) };

        let from_bytes =
            prepare_play(&req, ImageInput::Bytes(&png), &mappings, &mut quiet).expect("bytes");
        let path =
            std::env::temp_dir().join(format!("audiohax-s122-render-{}.png", std::process::id()));
        std::fs::write(&path, &png).expect("write png");
        let from_path = prepare_play(&req, ImageInput::Path(&path), &mappings, &mut quiet);
        let _ = std::fs::remove_file(&path);
        let from_path = from_path.expect("path");
        assert_eq!(from_bytes, from_path);
        assert!(!from_bytes.timeline.is_empty());

        let out = render(
            &req,
            ImageInput::Bytes(&png),
            &mappings,
            SoundFontSource::Bundled,
            &mut quiet,
        )
        .expect("render");
        assert_eq!(out.events, from_bytes.timeline, "play == render (S40)");
        assert_eq!(out.summary.total_steps, from_bytes.total_steps);
        assert_eq!(out.summary.programs, from_bytes.programs);
        assert_eq!(out.sample_rate, 44_100);
        assert!(!out.interleaved.is_empty());
    }

    /// The notes `play` prints, in the order the pre-S122 binary printed them.
    #[test]
    fn prepare_play_reports_in_the_historical_order() {
        let mappings = load_mappings("assets/mappings.json").expect("mappings");
        let png = test_png();
        let req = RenderRequest::from(&play_args(&["audiohax", "play"]));
        let mut order = Vec::new();
        let mut rec = |n: RenderNote<'_>| -> Result<(), RenderError> {
            order.push(match n {
                RenderNote::ImageLoaded { .. } => "loaded",
                RenderNote::ImageScanned { .. } => "scanned",
                RenderNote::GlobalFeatures(_) => "global",
                RenderNote::EngineMode(_) => "mode",
                _ => "other",
            });
            Ok(())
        };
        prepare_play(&req, ImageInput::Bytes(&png), &mappings, &mut rec).expect("prepare");
        assert_eq!(order, vec!["loaded", "scanned", "global", "mode"]);
    }

    /// Register hygiene (a GUI worker renders repeatedly on one thread): every call re-sets the
    /// seed / expression / cadence registers, so a render after an armed render equals the same
    /// render on a fresh thread. The CLI (one process per render) cannot see this class.
    #[test]
    fn render_is_independent_of_a_prior_armed_render_on_the_same_thread() {
        let mappings = load_mappings("assets/mappings.json").expect("mappings");
        let png = test_png();
        let armed = RenderRequest::from(&render_args(&[
            "audiohax",
            "render",
            "--seed",
            "99",
            "--expression",
            "on",
            "--v2",
            "all",
        ]));
        let plain = RenderRequest::from(&render_args(&["audiohax", "render"]));
        let mut quiet = |_: RenderNote<'_>| -> Result<(), RenderError> { Ok(()) };
        let run = |req: &RenderRequest, quiet: &mut NoteSink<'_>| {
            render(
                req,
                ImageInput::Bytes(&png),
                &mappings,
                SoundFontSource::Bundled,
                quiet,
            )
            .expect("render")
        };
        let _ = run(&armed, &mut quiet);
        let after_armed = run(&plain, &mut quiet);
        let fresh = std::thread::scope(|s| {
            s.spawn(|| {
                let mut q = |_: RenderNote<'_>| -> Result<(), RenderError> { Ok(()) };
                run(&plain, &mut q)
            })
            .join()
            .expect("fresh thread")
        });
        assert_eq!(after_armed.events, fresh.events);
        assert_eq!(after_armed.interleaved, fresh.interleaved);
    }
}

// S124 — tests for the preset overlay (L-30), the MB-5 piece cap (L-32) and the public `acquire`.
#[cfg(test)]
mod s124_tests {
    use super::*;
    use crate::presets::{default_preset, parse_with_explicit, preset_render_args, ExplicitArgs};

    fn request(argv: &[&str]) -> (RenderRequest, ExplicitArgs) {
        let (cli, ex) = parse_with_explicit(argv).expect("parse");
        match cli.command {
            crate::cli::Command::Render(r) => (RenderRequest::from(&r), ex),
            crate::cli::Command::Play(p) => (RenderRequest::from(&p), ex),
            other => panic!("{other:?}"),
        }
    }

    /// One NON-default probe per presettable id. A new id in `PRESET_SETTING_IDS` without a probe
    /// fails here with its name.
    const PROBES: &[(&str, &str)] = &[
        ("instruments", "6"),
        ("thickness", "0.3"),
        ("steps", "10"),
        ("expression", "on"),
        ("v2", "e3"),
        ("reverb", "off"),
        ("gain", "0.8"),
    ];

    /// T-5 forcing: for EVERY presettable id, a preset that sets only that id changes the result
    /// when the user did not type it, and does NOT when the user typed it. A missing overlay arm
    /// (or a missing probe) is a red test.
    #[test]
    fn overlay_covers_every_presettable_id() {
        let (base, none) = request(&["audiohax", "render"]);
        assert_eq!(none, ExplicitArgs::none());
        for id in crate::presets::PRESET_SETTING_IDS {
            let (_, value) = PROBES
                .iter()
                .find(|(p, _)| p == id)
                .unwrap_or_else(|| panic!("no overlay probe for presettable id `{id}`"));
            let flag = format!("--{id}");
            let (preset_req, _) = request(&["audiohax", "render", &flag, value]);
            assert_ne!(
                base.overlay_preset(&preset_req, &ExplicitArgs::none()),
                base,
                "`{id}`: the overlay does not carry this setting"
            );
            // The user typed the DEFAULT value of this option: the typed value wins.
            let (typed_req, typed) = request(&["audiohax", "render", &flag, &default_of(id)]);
            assert!(typed.contains(id), "{id} typed");
            assert_eq!(
                typed_req.overlay_preset(&preset_req, &typed),
                typed_req,
                "`{id}`: a typed flag must beat the preset"
            );
        }
    }

    fn default_of(id: &str) -> String {
        let s = crate::options_schema::render_options_schema().expect("schema");
        let o = s.iter().find(|o| o.id == id).expect("row");
        // `v2` has no default; any value the user types counts, so use one that differs from the
        // probe (`e3`), which makes "the typed value wins" observable.
        o.default.clone().unwrap_or_else(|| "e2".into())
    }

    /// L-29 / T-5: overlaying the `default` preset is the identity for any parsed request, on
    /// `render` and on `play`, whatever the user typed.
    #[test]
    fn overlaying_the_default_preset_is_the_identity() {
        let d = RenderRequest::from(&preset_render_args(default_preset().expect("d")).expect("a"));
        for argv in [
            &["audiohax", "render"][..],
            &[
                "audiohax", "render", "x.png", "--wav", "o.wav", "--seed", "7",
            ],
            &[
                "audiohax",
                "render",
                "--instruments",
                "6",
                "--gain",
                "0.5",
                "--reverb",
                "off",
            ],
            &[
                "audiohax",
                "render",
                "--expression",
                "on",
                "--v2",
                "all",
                "--loop",
            ],
            &["audiohax", "render", "--steps", "3", "--thickness", "0.9"],
            &["audiohax", "play", "--instruments", "2", "--seed", "1"],
            &["audiohax", "play"],
        ] {
            let (req, ex) = request(argv);
            assert_eq!(req.overlay_preset(&d, &ex), req, "{argv:?}");
            // ...and with nothing marked typed too (the untyped fields ARE clap's defaults).
            let (plain, _) = request(&["audiohax", "render"]);
            assert_eq!(plain.overlay_preset(&d, &ExplicitArgs::none()), plain);
        }
    }

    /// T-10: `play --preset P` and `render --preset P` resolve to the same request (minus the
    /// render-only fields).
    #[test]
    fn play_and_render_resolve_a_preset_alike() {
        let (preset_req, _) =
            request(&["audiohax", "render", "--instruments", "6", "--gain", "0.8"]);
        let (r, rx) = request(&["audiohax", "render", "--preset", "default"]);
        let (p, px) = request(&["audiohax", "play", "--preset", "default"]);
        let r = r.overlay_preset(&preset_req, &rx);
        let p = p.overlay_preset(&preset_req, &px);
        assert_eq!(r, p);
        assert_eq!(r.pipeline.instruments, 6);
    }

    /// MB-5: a timeline longer than ten minutes is refused before anything is sized from it; the
    /// boundary itself passes.
    #[test]
    fn the_piece_cap_refuses_over_ten_minutes() {
        use crate::synth_sink::MidiCmd;
        let ev = |at_ms| TimedMidiEvent {
            at_ms,
            cmd: MidiCmd::NoteOff {
                channel: 0,
                note: 60,
            },
        };
        assert!(check_piece_length(&[ev(0), ev(598_500)], 1_500).is_ok());
        let e = check_piece_length(&[ev(2_527_500)], 1_500).expect_err("42 minutes");
        let msg = e.to_string();
        assert!(
            msg.contains("0:42:09") && msg.contains("limit is 10 minutes"),
            "{msg}"
        );
        assert!(check_piece_length(&[ev(598_501)], 1_500).is_err());
        assert!(check_piece_length(&[], 0).is_ok());
    }

    /// The public `acquire` gives the same analysis the cached window path relies on (the
    /// byte-identity of render-from-acquire vs `render()` is pinned in `gui::job`'s tests).
    #[test]
    fn acquire_reports_its_three_notes_in_order() {
        let img = image::RgbImage::from_fn(48, 32, |x, y| {
            image::Rgb([(x * 5) as u8, (y * 7) as u8, 90])
        });
        let mut png = Vec::new();
        image::DynamicImage::ImageRgb8(img)
            .write_to(
                &mut std::io::Cursor::new(&mut png),
                image::ImageOutputFormat::Png,
            )
            .expect("png");
        let mut seen = Vec::new();
        let (src, _) = acquire(
            &crate::cli::PipelineArgs::default(),
            ImageInput::Bytes(&png),
            &mut |n| {
                seen.push(match n {
                    RenderNote::ImageLoaded { .. } => "loaded",
                    RenderNote::ImageScanned { .. } => "scanned",
                    RenderNote::GlobalFeatures(_) => "global",
                    _ => "other",
                });
                Ok(())
            },
        )
        .expect("acquire");
        assert_eq!(seen, vec!["loaded", "scanned", "global"]);
        assert_eq!(src.step_count(), 40);
    }
}

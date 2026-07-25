//! S107 / slice X0 — **the event-level dump**: a deterministic, diagnostic-flag-gated
//! recorder for the RESOLVED note-event timeline, with each event's full musical identity
//! (role, phrase index, position-in-phrase, phrase length, cadence status) still attached.
//!
//! # Why this module exists
//!
//! The S106 expression design (`s106-03-expression-design.md` §5.0) makes this instrument a
//! hard prerequisite of every later slice: *"almost every trap below is uncatchable from the
//! WAV and trivially catchable from the event list."* Nine of that document's fourteen traps
//! are invisible without it, and its own audio instrument (`M1_fine`) is shown at §5.2 (a9) to
//! be unable to resolve the timing change at all. So the order is: **build the instrument, then
//! change the music** — never the reverse.
//!
//! # The one thing this module exists to get right
//!
//! A dump that emits the onset the code **asked for** certifies the REQUEST, not the
//! REALIZATION. `synth_sink.rs:378` fixes `const BLOCK: usize = 441` and `441000 / 44100 = 10`
//! exactly, so the renderer applies an event in the first block whose end exceeds its
//! timestamp — i.e. the **realized** onset is `floor(at_ms / 10) * 10`, an exact 10 ms floor
//! lattice (`s106-01` FOLLOW-UP Q2(d); `s106-03` AMENDMENT A.1).
//!
//! A full green suite could therefore prove the code asked for the right thing while the render
//! did something else. Every record here consequently carries **both**:
//!
//! * `at_ms` — the requested onset (`step_base_ms + offset_ms`), and
//! * `realized_at_ms` — [`realize_onset_ms`]`(at_ms)`, what the renderer actually applies,
//!
//! and both the requested and the realized deviation against the *pre-expression* grid onset,
//! so `realized_dev_ms == requested_dev_ms` is directly assertable. That equality is the
//! production proof of **LR-1 (Exactness)**: a whole-rung request realizes exactly, at every
//! phase. [`Counters::lattice`] additionally counts non-multiples of 10 (**LR-5**).
//!
//! # Cost when disarmed
//!
//! One `Relaxed` atomic load per hook. No allocation, no lock, no branch on any musical value.
//! The default render path is therefore **byte-identical** with the flag off — verified by
//! re-render, not asserted (see the slice deliverable).
//!
//! # Determinism
//!
//! Zero RNG, zero wall-clock, zero hash-map iteration order (every map here is a
//! [`BTreeMap`]). The record stream is emitted in engine call order, which is itself a pure
//! function of `(image, seed, config)`. Same input ⇒ byte-identical JSON.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

use serde::Serialize;

// ───────────────────────────────────────────────────────────────────────────────
// The lattice
// ───────────────────────────────────────────────────────────────────────────────

/// The renderer's event-application quantum, in milliseconds.
///
/// `synth_sink.rs:378` — `const BLOCK: usize = 441` at 44 100 Hz, and `441000 / 44100 = 10`
/// **exactly**, with no rounding drift, so block `k` ends at exactly `10(k+1)` ms.
pub const BLOCK_MS: u64 = 10;

/// The onset the renderer will actually apply for a requested timestamp: `floor(at/10)*10`.
///
/// This is the single most load-bearing function in the module. See the module docs.
#[inline]
pub const fn realize_onset_ms(at_ms: u64) -> u64 {
    (at_ms / BLOCK_MS) * BLOCK_MS
}

/// The three expression TIMING terms of the S106 design (`s106-03` §4.3 / AMENDMENT A.4).
///
/// X0 itself fires none of them — it is a pure observer. They exist here so the per-term
/// counters are structurally present **before** the terms are built, which is the whole point
/// of trap (a7): an aggregate counter can read positive while one term displaces nothing.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Term {
    /// Metric inequality — delay the weak member of each onset pair.
    TA,
    /// Melody lead, implemented as accompaniment lag (+20 ms on Fill/Pad downbeats).
    TB,
    /// Cadence-approach agogic lean — delay the final onset of a `pre_cadence` step.
    TC,
}

impl Term {
    /// Every term, in a fixed order (so counter maps are deterministic).
    pub const ALL: [Term; 3] = [Term::TA, Term::TB, Term::TC];

    /// The term's stable name as it appears in the JSON.
    pub fn name(self) -> &'static str {
        match self {
            Term::TA => "T-A",
            Term::TB => "T-B",
            Term::TC => "T-C",
        }
    }
}

// ───────────────────────────────────────────────────────────────────────────────
// Input surfaces (primitives only — no coupling to `chord_engine`'s types)
// ───────────────────────────────────────────────────────────────────────────────

/// One realized onset, flattened out of a `chord_engine::NoteEvent`.
///
/// Deliberately a local primitive struct rather than the engine's own type: this module must
/// never become a reason to touch `NoteEvent` (risk **R1** — it derives `Copy` + `Eq` and the
/// FROZEN `engine.rs:491` does `flat_notes.push(*ev)`, so a float field there would break the
/// frozen file's compilation). Nothing in X0 adds a field to any engine struct.
#[derive(Clone, Copy, Debug)]
pub struct Onset {
    /// MIDI note number.
    pub note: u8,
    /// Realized MIDI velocity for this event.
    pub velocity: u8,
    /// Sounding duration, ms.
    pub hold_ms: u64,
    /// Onset offset within the step, ms — the value expression will perturb at X4–X6.
    pub offset_ms: u64,
}

/// The musical identity of one `(step, instrument)` realization frame.
#[derive(Clone, Debug)]
pub struct StepMeta {
    /// Ensemble seat index. The MIDI channel is `inst_idx % 16` (`engine.rs:709`).
    pub inst_idx: usize,
    /// Orchestral role name (`Bass` / `HarmonicFill` / `Melody` / `Pad` / `CounterMelody`).
    pub role: &'static str,
    /// Composition section label.
    pub section: String,
    /// 0-based phrase index.
    pub phrase_index: usize,
    /// 0-based step index within the phrase.
    pub position_in_phrase: usize,
    /// Phrase length in steps.
    pub phrase_len: usize,
    /// `PhrasePosition` discriminant name.
    pub phrase_position: &'static str,
    /// The step's structural velocity floor (`StepPlan::velocity`).
    pub floor_velocity: u8,
    /// The step's total time budget, ms.
    pub ms_per_step: u64,
}

impl StepMeta {
    /// `true` on a cadence step (`chord_engine.rs:2054-2057`).
    pub fn is_cadence(&self) -> bool {
        self.phrase_position == "HalfCadence" || self.phrase_position == "PerfectAuthenticCadence"
    }

    /// `true` on a phrase-start step.
    pub fn is_phrase_start(&self) -> bool {
        self.phrase_position == "PhraseStart"
    }

    /// The harmonic-rhythm acceleration predicate, reproduced EXACTLY from
    /// `chord_engine.rs:2528-2531`. This is a pure function of phrase position, which is why
    /// the dump can evaluate it without the realizer's cooperation.
    pub fn pre_cadence(&self) -> bool {
        !self.is_cadence()
            && !self.is_phrase_start()
            && self.position_in_phrase + 2 >= self.phrase_len
            && self.phrase_len >= 2
    }
}

// ───────────────────────────────────────────────────────────────────────────────
// The record
// ───────────────────────────────────────────────────────────────────────────────

/// One dumped note event, with its musical identity and both its requested and realized onset.
#[derive(Clone, Debug, Serialize)]
pub struct EventRecord {
    /// Cue label (the `--wav` stem, else the image stem).
    pub cue: String,
    /// Loop lap. The engine renders **exactly one** lap; see [`DumpJson::laps_rendered`].
    pub lap: u32,
    /// Global step index.
    pub step_idx: usize,
    /// The step's absolute grid anchor, `step_idx * grid_ms_per_step` (`main.rs:385-386`).
    pub step_base_ms: u64,
    /// Section label.
    pub section: String,
    /// Ensemble seat.
    pub inst_idx: usize,
    /// MIDI channel, `inst_idx % 16`.
    pub channel: u8,
    /// Orchestral role.
    pub role: &'static str,
    /// Phrase index within the piece.
    pub phrase_index: usize,
    /// Step index within its phrase.
    pub position_in_phrase: usize,
    /// Phrase length, steps.
    pub phrase_len: usize,
    /// `PhrasePosition` name.
    pub phrase_position: &'static str,
    /// Cadence step?
    pub is_cadence: bool,
    /// Pre-cadence (harmonic-rhythm acceleration) step?
    pub pre_cadence: bool,
    /// Onset index `k` within this step's figure.
    pub onset_k: usize,
    /// How many onsets this `(step, instrument)` emitted — the trap-(a7) denominator input.
    pub onsets_in_step: usize,
    /// MIDI note number.
    pub note: u8,

    // ── timing: requested vs realized ────────────────────────────────────────
    /// The PRE-expression onset offset within the step. Equal to `offset_ms` until an
    /// expression term moves it (X4–X6 report it via [`note_grid_offsets`]).
    pub grid_offset_ms: u64,
    /// The FINAL onset offset within the step.
    pub offset_ms: u64,
    /// Requested absolute onset, `step_base_ms + offset_ms`.
    pub at_ms: u64,
    /// **What the renderer actually applies**: `floor(at_ms / 10) * 10`.
    pub realized_at_ms: u64,
    /// Requested absolute onset of the un-deviated grid position.
    pub grid_at_ms: u64,
    /// Realized absolute onset of the un-deviated grid position.
    pub realized_grid_at_ms: u64,
    /// `at_ms - grid_at_ms` — the deviation the code ASKED for.
    pub requested_dev_ms: i64,
    /// `realized_at_ms - realized_grid_at_ms` — the deviation the render DELIVERS.
    pub realized_dev_ms: i64,
    /// The onset's phase within its 10 ms lattice cell, `grid_at_ms % 10`. The realized
    /// deviation of a NON-whole-rung request is a function of this (AMENDMENT A.1).
    pub lattice_phase: u64,

    // ── velocity ─────────────────────────────────────────────────────────────
    /// This event's realized MIDI velocity.
    pub velocity: u8,
    /// The step-level `realize_velocity` output for this `(step, instrument)`.
    pub step_velocity: u8,
    /// The step-level velocity BEFORE `.round().clamp(1.0, 127.0)` — trap (a2)'s detector.
    pub vel_pre_clamp: f64,
    /// The saturation-derived level offset (`chord_engine.rs:2296`), `-12 ..= +18`.
    pub level_gain: f64,
    /// `vel_pre_clamp - level_gain` — the **structural** velocity series, i.e. the contour at
    /// `level_gain == 0`. This is the series the S106 design tabulates (99/74/83/…/96), and it
    /// is what makes the phrase-start-vs-cadence inversion directly visible.
    pub vel_structural: f64,
    /// The step's structural velocity floor.
    pub floor_velocity: u8,

    // ── articulation ─────────────────────────────────────────────────────────
    /// Sounding duration, ms.
    pub hold_ms: u64,
    /// Local inter-onset gap: `offset[k+1] - offset[k]`, or `ms_per_step - offset[k]` for the
    /// last onset. The MT-3 / LR-4 ceiling denominator, and trap (b10)'s articulation ratio.
    pub gap_ms: u64,
    /// The gap the renderer actually produces: the same difference taken on `realized_at_ms`.
    ///
    /// This is NOT `gap_ms`. Both endpoints are independently floor-quantized, so a figure the
    /// realizer tiles by exact integer division does **not** arrive evenly: `gameplay-high`'s
    /// 3-onset arpeggio requests `(166, 166)` and realizes `(160, 170)`. LR-4's 30%-of-gap
    /// ceiling and LR-3's contrast rule are ear-facing claims and therefore belong on THIS
    /// quantity, not on the requested one.
    pub realized_gap_ms: i64,
    /// The step's time budget, ms.
    pub ms_per_step: u64,
    /// Did this event's frame actually pass through `realize_velocity`?
    ///
    /// `false` on the `pivot_chord_events` early-return path, which emits events whose velocity
    /// never sees the contour, the accent, the taper, `level_gain`, the role bias or the
    /// prominence nudge — only the bare structural floor. Recorded explicitly rather than
    /// papered over: on such a row `vel_pre_clamp`/`level_gain`/`vel_structural` are the event's
    /// own velocity and zero, and a consumer that averaged them without checking this flag would
    /// be averaging a fabrication. Every E1–E5 velocity term is unreachable on these events.
    pub velocity_detail_present: bool,

    // ── term attribution ─────────────────────────────────────────────────────
    /// Which expression terms CLAIM to have moved this onset (reported by the realizer).
    /// Empty at X0 by construction: no term exists yet.
    pub terms_fired: Vec<&'static str>,
    /// Which terms this onset is STRUCTURALLY ELIGIBLE for, per the AMENDMENT A.4 per-role /
    /// per-band matrix. Computed by X0 from phrase geometry alone — so the displacement a
    /// later slice CAN achieve is bounded and known before that slice is written.
    pub terms_eligible: Vec<&'static str>,
}

// ───────────────────────────────────────────────────────────────────────────────
// Counters
// ───────────────────────────────────────────────────────────────────────────────

/// `displaced` / `total` for one (term, role) cell.
#[derive(Clone, Debug, Default, Serialize)]
pub struct TermCell {
    /// Events this term actually moved (`terms_fired` contains it **and** the realized
    /// deviation is non-zero).
    pub displaced: u64,
    /// Events this term claims to have moved, whether or not the realization moved.
    pub claimed: u64,
    /// Events structurally eligible for this term.
    pub eligible: u64,
    /// All events of this role.
    pub total: u64,
}

/// Per-term counters, broken out **per role** — never aggregate.
///
/// This split is the second half of the trap the slice is named for. AMENDMENT A.4 suppresses
/// T-A on pre-cadence steps, which removes its densest figure; **T-A can therefore displace
/// zero events while T-B and T-C keep an aggregate counter positive and everything reads
/// green.** An aggregate `displaced_event_count > 0` assertion cannot fail in that world.
#[derive(Clone, Debug, Serialize)]
pub struct TermCounters {
    /// Term name.
    pub term: &'static str,
    /// Per-role cells, role name → cell.
    pub by_role: BTreeMap<String, TermCell>,
    /// Sum over roles — reported for continuity, **never** to be gated on alone.
    pub all_roles: TermCell,
}

/// Timing-coverage counters, re-based off the multi-onset denominator.
///
/// `s106-03` §4.7 originally asked for "≥ 20% of Melody events". That denominator is wrong: it
/// includes events T-A structurally cannot touch — the SUSTAINED band (1 onset), the cadence
/// ring (1 event at offset 0), the Pad block bed, and the single-tone melody floor
/// (AMENDMENT A.4's band table). Re-based here on `melody_multi_onset_events`.
#[derive(Clone, Debug, Default, Serialize)]
pub struct TimingCoverage {
    /// All Melody events.
    pub melody_total_events: u64,
    /// Melody events in a step that emitted **one** onset — structurally immovable by T-A.
    pub melody_single_onset_events: u64,
    /// Melody events in a step that emitted **two or more** onsets. **The denominator.**
    pub melody_multi_onset_events: u64,
    /// Melody multi-onset events that were actually displaced.
    pub melody_multi_onset_displaced: u64,
    /// `melody_multi_onset_displaced / melody_multi_onset_events`.
    pub coverage_frac: f64,
    /// Melody events structurally eligible for T-A (odd `k`, multi-onset, not pre-cadence).
    pub melody_ta_eligible: u64,
    /// Melody events structurally eligible for T-C (last onset of a multi-onset pre-cadence step).
    pub melody_tc_eligible: u64,
    /// The same denominator, per section — a piece-level total can hide a dead section.
    pub multi_onset_by_section: BTreeMap<String, u64>,
}

/// Lattice-integrity counters — **LR-5**.
#[derive(Clone, Debug, Default, Serialize)]
pub struct LatticeCounters {
    /// Requested deviations that are not a whole multiple of 10 ms. **Must be 0.**
    pub nonmultiple_requested_dev: u64,
    /// Realized deviations that are not a whole multiple of 10 ms. Structurally impossible
    /// (both terms are floor-quantized), so a non-zero here means the dump is broken.
    pub nonmultiple_realized_dev: u64,
    /// Requested deviations in `(0, 20)` ms — below the **LR-2** audibility floor.
    pub sub_floor_requests: u64,
    /// Whole-rung requests whose realization differs. **Must be 0** — this is LR-1 itself.
    pub lr1_violations: u64,
    /// Non-whole-rung requests whose realization differs from the request (the dither).
    pub dithered_requests: u64,
    /// Deviations exceeding 30% of their local gap — **LR-4**.
    pub lr4_breaches: u64,
    /// Distinct lattice phases `grid_at_ms % 10` observed.
    ///
    /// AMENDMENT A.2 tabulates the phase of the step ANCHOR (`gameplay-high` at 500 ms is
    /// always phase 0, `gameplay-low` cycles 5, `title` cycles 10). The phase that actually
    /// governs an onset is `(step_base_ms + offset_ms) % 10`, and multi-onset figures place
    /// onsets at `step_ms/3`, `step_ms/4`, `2·step_ms/3` — rarely multiples of 10. So even a
    /// phase-0 anchor yields onsets across many phases; this counter reports what is really
    /// there rather than what the anchor implies.
    pub distinct_lattice_phases: Vec<u64>,
    /// Onsets FUSED by the lattice: same channel, **distinct** requested `at_ms`, identical
    /// `realized_at_ms`. Two chord tones sharing an onset are not a fusion and are excluded.
    pub lattice_fused_onsets: u64,
    /// Genuinely SEQUENTIAL multi-onset figures (every inter-onset gap non-zero — a chord's
    /// simultaneous tones are excluded) whose REQUESTED gaps are all equal.
    pub figures_requested_even: u64,
    /// Of those, how many arrive with all-equal REALIZED gaps. The gap between these two
    /// numbers is inequality the 10 ms lattice imposes on the shipped render before any
    /// expression term exists.
    pub figures_realized_even: u64,
}

/// Velocity-layer counters — traps (a2), (a4), (a5), (a6).
#[derive(Clone, Debug, Default, Serialize)]
pub struct VelocityCounters {
    /// `vel_pre_clamp > 127` — the contour-eating clamp. **Must be 0** (trap b2).
    pub clamp_high: u64,
    /// `vel_pre_clamp < 1`.
    pub clamp_low: u64,
    /// Max realized MIDI velocity.
    pub realized_max: u8,
    /// Min realized MIDI velocity.
    pub realized_min: u8,
    /// Max pre-clamp step velocity.
    pub vel_pre_clamp_max: f64,
    /// Min pre-clamp step velocity.
    pub vel_pre_clamp_min: f64,
    /// Distinct realized step velocities per `"role|section"` — trap (b5) needs ≥ 5.
    pub distinct_step_velocity_by_voice_section: BTreeMap<String, u64>,
    /// Distinct realized EVENT velocities per `"role|section"`.
    pub distinct_event_velocity_by_voice_section: BTreeMap<String, u64>,
    /// Realized `phrase_len` histogram. If `< 4` dominates, E2's `% 4` hierarchy is inert
    /// (trap a5) and must be re-scoped.
    pub phrase_len_histogram: BTreeMap<usize, u64>,
    /// Distinct `phrase_index` values seen. Trap (a4): at 1 the E4 arch degenerates to a
    /// constant gain offset and the "long line" silently becomes a level change.
    pub distinct_phrase_index: u64,
    /// `max(phrase_index) + 1` — the E4 denominator.
    pub phrase_count_total: usize,
    /// Cardinality of the E4 arch values that WOULD be realized over the observed phrase
    /// indices, `round(sin(PI*(p+0.5)/N) * 6.0)`. Trap (b4) requires ≥ 3. **Projected**, not
    /// realized: X0 fires no arch term.
    pub projected_arch_distinct: u64,
    /// The projected arch values themselves.
    pub projected_arch_values: Vec<f64>,
}

/// The entrance / cadence ordering view — the event-level form of `s106-03` §0.2.
#[derive(Clone, Debug, Default, Serialize)]
pub struct EntranceCell {
    /// Modal structural velocity at `position_in_phrase == 0`.
    pub phrase_start_structural_mode: Option<i64>,
    /// Modal structural velocity on cadence steps.
    pub cadence_structural_mode: Option<i64>,
    /// Modal structural velocity at `position_in_phrase == 1` (the entrance's neighbour).
    pub pos1_structural_mode: Option<i64>,
    /// `phrase_start - cadence`. **Positive means every phrase departs LOUDER than it
    /// arrives** — the inversion of the code's own stated intent at `chord_engine.rs:886-893`.
    pub start_minus_cadence: Option<i64>,
    /// `phrase_start - pos1`, the note-to-note entrance step in velocity units.
    pub entrance_step_units: Option<i64>,
    /// The entrance step converted to dB under the SoundFont-standard concave law
    /// `amplitude ∝ (vel/127)²`, i.e. `40·log10(start / pos1)`.
    pub entrance_step_db: Option<f64>,
    /// Realized (level-included) mean velocity at `position_in_phrase == 0`.
    pub phrase_start_realized_mean: Option<f64>,
    /// Realized mean velocity on cadence steps.
    pub cadence_realized_mean: Option<f64>,
}

/// Articulation counters — trap (b10): the timing change must not ship as an articulation change.
#[derive(Clone, Debug, Default, Serialize)]
pub struct ArticulationCounters {
    /// Mean `hold_ms / gap_ms` per role.
    pub mean_hold_over_gap_by_role: BTreeMap<String, f64>,
    /// The piece's final `max(at_ms + hold_ms)` — trap (b13)'s fold-drift anchor.
    pub max_note_end_ms: u64,
    /// The same quantity on the realized lattice.
    pub max_realized_note_end_ms: u64,
}

/// Every counter block for one cue.
#[derive(Clone, Debug, Serialize)]
pub struct Counters {
    /// Per-term, per-role displacement.
    pub by_term: Vec<TermCounters>,
    /// Timing coverage on the multi-onset denominator.
    pub timing_coverage: TimingCoverage,
    /// LR-1 / LR-2 / LR-4 / LR-5 integrity.
    pub lattice: LatticeCounters,
    /// Clamp, distinct-velocity, phrase-len, arch.
    pub velocity: VelocityCounters,
    /// Entrance-vs-cadence ordering, per role.
    pub entrance_by_role: BTreeMap<String, EntranceCell>,
    /// Articulation ratios.
    pub articulation: ArticulationCounters,
    /// Pearson correlation of per-step **realized** velocity, Melody vs each other voice.
    ///
    /// `s106-03` §3.6 pre-registers this baseline as "≈ 1.0 (identical accent term)" and asks
    /// for ≤ 0.6 after E3. **The realized baseline is not ≈ 1.0** — see
    /// [`Self::melody_velocity_pearson_structural`] for why, and read that one for the gate.
    pub melody_velocity_pearson: BTreeMap<String, f64>,
    /// The same correlation on the **structural** series (`vel_pre_clamp - level_gain`),
    /// restricted to non-cadence steps and to events that actually passed through
    /// `realize_velocity`.
    ///
    /// This is where the design's premise is true: with `level_gain` removed and the cadence
    /// excluded, Melody and the bed differ by a single constant (the role bias plus the
    /// prominence nudge) and the correlation is **exactly 1.0**. The realized series decorrelates
    /// only because `level_gain` is derived from each voice's OWN scan band and is therefore a
    /// different per-step series per voice, and because the cadence gate zeroes the role
    /// differential on cadence steps. E3 acts on the accent term, i.e. on the STRUCTURAL series
    /// — so this is the series its acceptance gate must be read on. A gate placed on the
    /// realized series starts near its own target and can pass without E3 firing.
    pub melody_velocity_pearson_structural: BTreeMap<String, f64>,
    /// Events whose frame never reached `realize_velocity` (the pivot-chord early return).
    /// Every velocity term E1–E5 is unreachable on these; a velocity-coverage denominator that
    /// includes them overstates coverage.
    pub events_bypassing_realize_velocity: u64,
    /// Events falling on a cadence step, where EVERY `!is_cadence`-guarded term — the existing
    /// swell, accent, taper, role bias and prominence nudge, and every new E-term — is off.
    pub cadence_events: u64,
    /// Events on a `pre_cadence` step.
    pub pre_cadence_events: u64,
    /// Frames (step × instrument realizations) recorded.
    pub frames: u64,
    /// Frames that emitted zero onsets (a theme-driven melody rest, an empty plan).
    pub silent_frames: u64,
    /// Total events recorded.
    pub total_events: u64,
    /// Total events per role.
    pub events_by_role: BTreeMap<String, u64>,
}

// ───────────────────────────────────────────────────────────────────────────────
// The emitted document
// ───────────────────────────────────────────────────────────────────────────────

/// The whole dump, as serialized to JSON.
#[derive(Clone, Debug, Serialize)]
pub struct DumpJson {
    /// Dump format version.
    pub format: &'static str,
    /// Cue label.
    pub cue: String,
    /// Source image path, verbatim.
    pub image: String,
    /// Composition seed.
    pub seed: Option<u64>,
    /// Steps in the piece.
    pub total_steps: usize,
    /// Grid period per step, ms.
    pub grid_ms_per_step: u64,
    /// Ensemble size.
    pub instrument_count: usize,
    /// The musical loop period, `total_steps * grid_ms_per_step`.
    pub period_ms: u64,
    /// The renderer's event-application quantum.
    pub block_ms: u64,
    /// **1, always.** `render --loop` emits ONE lap and wrap-folds its ring-out; the shipped
    /// 3-lap files are an ffmpeg concat of that single lap (S104 RENDER-LOG). Per-lap variation
    /// therefore cannot exist at the event level — every record carries `lap: 0`, and any
    /// future claim of lap-to-lap expressive variation is false by construction on this path.
    pub laps_rendered: u32,
    /// Whether any expression term was armed for this dump.
    pub expression_armed: bool,
    /// The counters.
    pub counters: Counters,
    /// The event stream, in engine emission order.
    pub events: Vec<EventRecord>,
}

// ───────────────────────────────────────────────────────────────────────────────
// State + the armed flag
// ───────────────────────────────────────────────────────────────────────────────

static ARMED: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Debug)]
struct VelDetail {
    pre_clamp: f32,
    level_gain: f32,
    realized: u8,
}

#[derive(Default)]
struct State {
    cue: String,
    image: String,
    seed: Option<u64>,
    total_steps: usize,
    grid_ms_per_step: u64,
    instrument_count: usize,
    expression_armed: bool,

    step_idx: usize,
    step_base_ms: u64,

    pending_vel: Option<VelDetail>,
    pending_grid_offsets: Option<Vec<u64>>,
    pending_terms: Vec<(usize, Term)>,

    records: Vec<EventRecord>,
    /// `(step_idx, inst_idx, role, section, realized step velocity)`.
    step_velocities: Vec<(usize, usize, &'static str, String, u8)>,
    frames: u64,
    silent_frames: u64,
}

fn state() -> &'static Mutex<State> {
    static S: OnceLock<Mutex<State>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(State::default()))
}

/// Is the dump recording? One `Relaxed` load — this is the whole cost on the default path.
#[inline]
pub fn is_armed() -> bool {
    ARMED.load(Ordering::Relaxed)
}

/// Arm the dump for one render. Clears any previous state.
///
/// `expression_armed` records whether an expression axis is enabled for this render, so a dump
/// can never be mistaken for evidence about a build it did not observe.
pub fn arm(cue: &str, image: &str, seed: Option<u64>, expression_armed: bool) {
    if let Ok(mut s) = state().lock() {
        *s = State {
            cue: cue.to_string(),
            image: image.to_string(),
            seed,
            expression_armed,
            ..State::default()
        };
    }
    ARMED.store(true, Ordering::Relaxed);
}

/// Stop recording.
pub fn disarm() {
    ARMED.store(false, Ordering::Relaxed);
}

/// Record the composition shape once the plan is resolved.
pub fn set_plan(total_steps: usize, grid_ms_per_step: u64, instrument_count: usize) {
    if !is_armed() {
        return;
    }
    if let Ok(mut s) = state().lock() {
        s.total_steps = total_steps;
        s.grid_ms_per_step = grid_ms_per_step;
        s.instrument_count = instrument_count;
    }
}

/// Open a step. Called from `main.rs::build_step_event_timeline` where the grid anchor is known.
pub fn begin_step(step_idx: usize, step_base_ms: u64) {
    if !is_armed() {
        return;
    }
    if let Ok(mut s) = state().lock() {
        s.step_idx = step_idx;
        s.step_base_ms = step_base_ms;
        s.pending_vel = None;
        s.pending_grid_offsets = None;
        s.pending_terms.clear();
    }
}

/// Report the step-level velocity detail. Called from `chord_engine::realize_velocity`,
/// immediately before its `.round().clamp(1.0, 127.0)`.
pub fn note_velocity_detail(pre_clamp: f32, level_gain: f32, realized: u8) {
    if !is_armed() {
        return;
    }
    if let Ok(mut s) = state().lock() {
        s.pending_vel = Some(VelDetail {
            pre_clamp,
            level_gain,
            realized,
        });
    }
}

/// Report the PRE-expression grid offsets for the frame about to be recorded.
///
/// **This is the seam X4–X6 must use.** `realize_rhythm` retains its grid offsets (it must
/// anyway, to enforce AMENDMENT A.4's cumulative LR-4 cap against the ORIGINAL grid) and hands
/// them here before returning. X0 never calls it on the render path — with no expression term,
/// `grid_offset_ms == offset_ms` and the deviation is exactly zero everywhere, which is the
/// falsification requirement, not an omission.
pub fn note_grid_offsets(offsets: &[u64]) {
    if !is_armed() {
        return;
    }
    if let Ok(mut s) = state().lock() {
        s.pending_grid_offsets = Some(offsets.to_vec());
    }
}

/// Attribute a displacement of onset `k` to `term`. The other seam for X4–X6.
pub fn attribute(k: usize, term: Term) {
    if !is_armed() {
        return;
    }
    if let Ok(mut s) = state().lock() {
        s.pending_terms.push((k, term));
    }
}

/// Structural eligibility for each term, per AMENDMENT A.4's revised per-role/per-band matrix.
///
/// Pure function of phrase geometry and figure shape — no expression state is consulted, so
/// this is computable at X0 and bounds what a later slice CAN reach.
fn eligible_terms(
    role: &str,
    k: usize,
    onsets_in_step: usize,
    offset_ms: u64,
    meta: &StepMeta,
) -> Vec<&'static str> {
    let mut out = Vec::new();
    let pre_cad = meta.pre_cadence();
    let is_cad = meta.is_cadence();

    // T-A: Melody / CounterMelody, interior odd onset, multi-onset figure, NOT pre-cadence
    // (A.4 suppresses T-A there entirely so T-C can satisfy LR-3 and LR-4 together), and never
    // onset 0 — the downbeat anchor.
    if matches!(role, "Melody" | "CounterMelody")
        && onsets_in_step >= 2
        && k > 0
        && !k.is_multiple_of(2)
        && !pre_cad
        && !is_cad
    {
        out.push(Term::TA.name());
    }
    // T-B: HarmonicFill / Pad, onsets sitting exactly on the downbeat, never on a cadence step.
    if matches!(role, "HarmonicFill" | "Pad") && offset_ms == 0 && !is_cad {
        out.push(Term::TB.name());
    }
    // T-C: Melody, the LAST onset of a multi-onset pre-cadence step.
    if role == "Melody" && pre_cad && onsets_in_step >= 2 && k + 1 == onsets_in_step {
        out.push(Term::TC.name());
    }
    out
}

/// Record one `(step, instrument)` realization frame.
///
/// Called from `chord_engine::realize_step`'s thin wrapper, AFTER the realizer body returns, so
/// every exit path — including the theme-driven melody rest that returns an empty vector — is
/// observed. A frame with no onsets is counted (`silent_frames`) rather than dropped: a
/// denominator that silently omits its rests overstates coverage.
pub fn record_step_frame(meta: &StepMeta, onsets: &[Onset]) {
    if !is_armed() {
        return;
    }
    let Ok(mut s) = state().lock() else { return };

    s.frames += 1;
    if onsets.is_empty() {
        s.silent_frames += 1;
    }

    let vel = s.pending_vel;
    let step_base_ms = s.step_base_ms;
    let step_idx = s.step_idx;
    if let Some(v) = vel {
        s.step_velocities.push((
            step_idx,
            meta.inst_idx,
            meta.role,
            meta.section.clone(),
            v.realized,
        ));
    }

    let cue = s.cue.clone();
    let grid_offsets = s.pending_grid_offsets.clone();
    let terms = s.pending_terms.clone();

    let n = onsets.len();
    for (k, ev) in onsets.iter().enumerate() {
        // The local inter-onset gap: to the next onset, or to the step boundary for the last.
        let gap_ms = if k + 1 < n {
            onsets[k + 1].offset_ms.saturating_sub(ev.offset_ms)
        } else {
            meta.ms_per_step.saturating_sub(ev.offset_ms)
        };

        let grid_offset_ms = grid_offsets
            .as_ref()
            .and_then(|g| g.get(k).copied())
            .unwrap_or(ev.offset_ms);

        let at_ms = step_base_ms + ev.offset_ms;
        let grid_at_ms = step_base_ms + grid_offset_ms;
        let realized_at_ms = realize_onset_ms(at_ms);
        let realized_grid_at_ms = realize_onset_ms(grid_at_ms);

        let requested_dev_ms = at_ms as i64 - grid_at_ms as i64;
        let realized_dev_ms = realized_at_ms as i64 - realized_grid_at_ms as i64;

        let terms_fired: Vec<&'static str> = Term::ALL
            .iter()
            .filter(|t| terms.iter().any(|(kk, tt)| *kk == k && tt == *t))
            .map(|t| t.name())
            .collect();

        // The gap as the RENDERER produces it — both endpoints independently floor-quantized.
        let realized_gap_ms = if k + 1 < n {
            realize_onset_ms(step_base_ms + onsets[k + 1].offset_ms) as i64 - realized_at_ms as i64
        } else {
            realize_onset_ms(step_base_ms + meta.ms_per_step) as i64 - realized_at_ms as i64
        };

        let (pre_clamp, level_gain, step_velocity) = match vel {
            Some(v) => (v.pre_clamp as f64, v.level_gain as f64, v.realized),
            // No `realize_velocity` call was observed for this frame (the pivot-chord early
            // return). The row falls back to the event's own velocity with a zero level and is
            // FLAGGED via `velocity_detail_present: false` — never silently fabricated.
            None => (ev.velocity as f64, 0.0, ev.velocity),
        };

        s.records.push(EventRecord {
            cue: cue.clone(),
            lap: 0,
            step_idx,
            step_base_ms,
            section: meta.section.clone(),
            inst_idx: meta.inst_idx,
            channel: (meta.inst_idx % 16) as u8,
            role: meta.role,
            phrase_index: meta.phrase_index,
            position_in_phrase: meta.position_in_phrase,
            phrase_len: meta.phrase_len,
            phrase_position: meta.phrase_position,
            is_cadence: meta.is_cadence(),
            pre_cadence: meta.pre_cadence(),
            onset_k: k,
            onsets_in_step: n,
            note: ev.note,
            grid_offset_ms,
            offset_ms: ev.offset_ms,
            at_ms,
            realized_at_ms,
            grid_at_ms,
            realized_grid_at_ms,
            requested_dev_ms,
            realized_dev_ms,
            lattice_phase: grid_at_ms % BLOCK_MS,
            velocity: ev.velocity,
            step_velocity,
            vel_pre_clamp: r4(pre_clamp),
            level_gain: r4(level_gain),
            vel_structural: r4(pre_clamp - level_gain),
            floor_velocity: meta.floor_velocity,
            hold_ms: ev.hold_ms,
            gap_ms,
            realized_gap_ms,
            ms_per_step: meta.ms_per_step,
            velocity_detail_present: vel.is_some(),
            terms_fired,
            terms_eligible: eligible_terms(meta.role, k, n, ev.offset_ms, meta),
        });
    }

    s.pending_vel = None;
    s.pending_grid_offsets = None;
    s.pending_terms.clear();
}

/// Round to 4 decimals so the JSON is stable and readable. Deterministic.
fn r4(x: f64) -> f64 {
    (x * 1e4).round() / 1e4
}

// ───────────────────────────────────────────────────────────────────────────────
// Reduction
// ───────────────────────────────────────────────────────────────────────────────

/// Pearson product-moment correlation. `None` when either series is constant or too short —
/// a constant series has no correlation, and reporting 1.0 or 0.0 there would be a fabrication.
fn pearson(xs: &[f64], ys: &[f64]) -> Option<f64> {
    let n = xs.len();
    if n < 3 || ys.len() != n {
        return None;
    }
    let mx = xs.iter().sum::<f64>() / n as f64;
    let my = ys.iter().sum::<f64>() / n as f64;
    let mut sxy = 0.0;
    let mut sxx = 0.0;
    let mut syy = 0.0;
    for i in 0..n {
        let dx = xs[i] - mx;
        let dy = ys[i] - my;
        sxy += dx * dy;
        sxx += dx * dx;
        syy += dy * dy;
    }
    if sxx <= f64::EPSILON || syy <= f64::EPSILON {
        return None;
    }
    Some(sxy / (sxx.sqrt() * syy.sqrt()))
}

/// The most common value in a series, ties broken by the smaller value (deterministic).
fn mode(vals: &[i64]) -> Option<i64> {
    let mut counts: BTreeMap<i64, u64> = BTreeMap::new();
    for v in vals {
        *counts.entry(*v).or_default() += 1;
    }
    counts
        .into_iter()
        .max_by_key(|(v, c)| (*c, -*v))
        .map(|(v, _)| v)
}

fn mean(vals: &[f64]) -> Option<f64> {
    if vals.is_empty() {
        None
    } else {
        Some(r4(vals.iter().sum::<f64>() / vals.len() as f64))
    }
}

/// The E4 section-arch span, `s106-03` §3.3 E4. Used only to PROJECT the arch's realized
/// cardinality from the observed phrase indices (trap b4) — X0 fires no arch term.
const ARCH_SPAN: f64 = 6.0;

fn build_counters(s: &State) -> Counters {
    let recs = &s.records;

    // ── per-term, per-role ────────────────────────────────────────────────────
    let mut by_term = Vec::new();
    for t in Term::ALL {
        let name = t.name();
        let mut by_role: BTreeMap<String, TermCell> = BTreeMap::new();
        let mut all = TermCell::default();
        for r in recs {
            let cell = by_role.entry(r.role.to_string()).or_default();
            cell.total += 1;
            all.total += 1;
            if r.terms_eligible.contains(&name) {
                cell.eligible += 1;
                all.eligible += 1;
            }
            if r.terms_fired.contains(&name) {
                cell.claimed += 1;
                all.claimed += 1;
                if r.realized_dev_ms != 0 {
                    cell.displaced += 1;
                    all.displaced += 1;
                }
            }
        }
        by_term.push(TermCounters {
            term: name,
            by_role,
            all_roles: all,
        });
    }

    // ── timing coverage on the multi-onset denominator ────────────────────────
    let mut tc = TimingCoverage::default();
    for r in recs.iter().filter(|r| r.role == "Melody") {
        tc.melody_total_events += 1;
        if r.onsets_in_step >= 2 {
            tc.melody_multi_onset_events += 1;
            *tc.multi_onset_by_section
                .entry(r.section.clone())
                .or_default() += 1;
            if r.realized_dev_ms != 0 {
                tc.melody_multi_onset_displaced += 1;
            }
        } else {
            tc.melody_single_onset_events += 1;
        }
        if r.terms_eligible.contains(&Term::TA.name()) {
            tc.melody_ta_eligible += 1;
        }
        if r.terms_eligible.contains(&Term::TC.name()) {
            tc.melody_tc_eligible += 1;
        }
    }
    tc.coverage_frac = if tc.melody_multi_onset_events == 0 {
        0.0
    } else {
        r4(tc.melody_multi_onset_displaced as f64 / tc.melody_multi_onset_events as f64)
    };

    // ── lattice integrity ─────────────────────────────────────────────────────
    let mut lat = LatticeCounters::default();
    let mut phases: BTreeSet<u64> = BTreeSet::new();
    // Fusion is "distinct REQUESTS landing in one block on one channel". Chord tones that
    // share an onset are not a fusion, so the requested `at_ms` is part of the key.
    let mut slots: BTreeMap<(u8, u64), BTreeSet<u64>> = BTreeMap::new();
    // Figure evenness: requested vs realized inter-onset gaps, per (step, instrument).
    let mut figures: BTreeMap<(usize, usize), Vec<(u64, i64)>> = BTreeMap::new();
    for r in recs {
        slots
            .entry((r.channel, r.realized_at_ms))
            .or_default()
            .insert(r.at_ms);
        if r.onsets_in_step >= 3 && r.onset_k + 1 < r.onsets_in_step {
            figures
                .entry((r.step_idx, r.inst_idx))
                .or_default()
                .push((r.gap_ms, r.realized_gap_ms));
        }
    }
    lat.lattice_fused_onsets = slots.values().map(|v| v.len() as u64 - 1).sum();
    for gaps in figures.values() {
        // A Pad block bed emits several chord tones at ONE offset, so its inter-onset gaps are
        // all zero and it would score as a perfectly even "figure" in both columns. A chord is
        // not a figure; require every gap to be non-zero so only genuinely sequential onsets
        // are judged.
        if gaps.len() < 2 || gaps.iter().any(|(q, _)| *q == 0) {
            continue;
        }
        let req_even = gaps.iter().all(|(q, _)| *q == gaps[0].0);
        let real_even = gaps.iter().all(|(_, r)| *r == gaps[0].1);
        if req_even {
            lat.figures_requested_even += 1;
            if real_even {
                lat.figures_realized_even += 1;
            }
        }
    }
    for r in recs {
        phases.insert(r.lattice_phase);
        let d = r.requested_dev_ms;
        if d % BLOCK_MS as i64 != 0 {
            lat.nonmultiple_requested_dev += 1;
        }
        if r.realized_dev_ms % BLOCK_MS as i64 != 0 {
            lat.nonmultiple_realized_dev += 1;
        }
        if d > 0 && d < 20 {
            lat.sub_floor_requests += 1;
        }
        if d % BLOCK_MS as i64 == 0 {
            if r.realized_dev_ms != d {
                lat.lr1_violations += 1;
            }
        } else if r.realized_dev_ms != d {
            lat.dithered_requests += 1;
        }
        if r.gap_ms > 0 && d.unsigned_abs() * 100 > r.gap_ms * 30 {
            lat.lr4_breaches += 1;
        }
    }
    lat.distinct_lattice_phases = phases.into_iter().collect();

    // ── velocity ──────────────────────────────────────────────────────────────
    let mut vc = VelocityCounters {
        realized_min: u8::MAX,
        vel_pre_clamp_min: f64::MAX,
        vel_pre_clamp_max: f64::MIN,
        ..Default::default()
    };
    let mut step_vel_sets: BTreeMap<String, BTreeSet<u8>> = BTreeMap::new();
    let mut event_vel_sets: BTreeMap<String, BTreeSet<u8>> = BTreeMap::new();
    let mut phrase_idx: BTreeSet<usize> = BTreeSet::new();
    for r in recs {
        if r.vel_pre_clamp > 127.0 {
            vc.clamp_high += 1;
        }
        if r.vel_pre_clamp < 1.0 {
            vc.clamp_low += 1;
        }
        vc.realized_max = vc.realized_max.max(r.velocity);
        vc.realized_min = vc.realized_min.min(r.velocity);
        vc.vel_pre_clamp_max = vc.vel_pre_clamp_max.max(r.vel_pre_clamp);
        vc.vel_pre_clamp_min = vc.vel_pre_clamp_min.min(r.vel_pre_clamp);
        *vc.phrase_len_histogram.entry(r.phrase_len).or_default() += 1;
        phrase_idx.insert(r.phrase_index);
        let key = format!("{}|{}", r.role, r.section);
        step_vel_sets
            .entry(key.clone())
            .or_default()
            .insert(r.step_velocity);
        event_vel_sets.entry(key).or_default().insert(r.velocity);
    }
    if recs.is_empty() {
        vc.realized_min = 0;
        vc.vel_pre_clamp_min = 0.0;
        vc.vel_pre_clamp_max = 0.0;
    }
    vc.vel_pre_clamp_min = r4(vc.vel_pre_clamp_min);
    vc.vel_pre_clamp_max = r4(vc.vel_pre_clamp_max);
    vc.distinct_step_velocity_by_voice_section = step_vel_sets
        .iter()
        .map(|(k, v)| (k.clone(), v.len() as u64))
        .collect();
    vc.distinct_event_velocity_by_voice_section = event_vel_sets
        .iter()
        .map(|(k, v)| (k.clone(), v.len() as u64))
        .collect();
    vc.distinct_phrase_index = phrase_idx.len() as u64;
    vc.phrase_count_total = phrase_idx.iter().copied().max().map(|m| m + 1).unwrap_or(0);
    if vc.phrase_count_total >= 2 {
        let n = vc.phrase_count_total as f64;
        let mut set: BTreeSet<i64> = BTreeSet::new();
        let mut vals = Vec::new();
        for p in phrase_idx.iter().copied() {
            let u = (p as f64 + 0.5) / n;
            let a = (std::f64::consts::PI * u).sin() * ARCH_SPAN;
            set.insert(a.round() as i64);
            vals.push(r4(a));
        }
        vc.projected_arch_distinct = set.len() as u64;
        vc.projected_arch_values = vals;
    }

    // ── entrance / cadence ordering, per role ─────────────────────────────────
    let mut entrance_by_role: BTreeMap<String, EntranceCell> = BTreeMap::new();
    let roles: BTreeSet<&str> = recs.iter().map(|r| r.role).collect();
    for role in roles {
        let of = |f: &dyn Fn(&EventRecord) -> bool| -> Vec<i64> {
            recs.iter()
                .filter(|r| r.role == role && f(r))
                .map(|r| r.vel_structural.round() as i64)
                .collect()
        };
        let realized_of = |f: &dyn Fn(&EventRecord) -> bool| -> Vec<f64> {
            recs.iter()
                .filter(|r| r.role == role && f(r))
                .map(|r| r.step_velocity as f64)
                .collect()
        };
        let starts = of(&|r| r.position_in_phrase == 0 && !r.is_cadence);
        let cads = of(&|r| r.is_cadence);
        let pos1 = of(&|r| r.position_in_phrase == 1 && !r.is_cadence);
        let mut cell = EntranceCell {
            phrase_start_structural_mode: mode(&starts),
            cadence_structural_mode: mode(&cads),
            pos1_structural_mode: mode(&pos1),
            phrase_start_realized_mean: mean(&realized_of(&|r| {
                r.position_in_phrase == 0 && !r.is_cadence
            })),
            cadence_realized_mean: mean(&realized_of(&|r| r.is_cadence)),
            ..Default::default()
        };
        if let (Some(a), Some(b)) = (
            cell.phrase_start_structural_mode,
            cell.cadence_structural_mode,
        ) {
            cell.start_minus_cadence = Some(a - b);
        }
        if let (Some(a), Some(b)) = (cell.phrase_start_structural_mode, cell.pos1_structural_mode) {
            cell.entrance_step_units = Some(a - b);
            if a > 0 && b > 0 {
                cell.entrance_step_db = Some(r4(40.0 * (a as f64 / b as f64).log10()));
            }
        }
        entrance_by_role.insert(role.to_string(), cell);
    }

    // ── articulation ──────────────────────────────────────────────────────────
    let mut art = ArticulationCounters::default();
    let mut ratios: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    for r in recs {
        if r.gap_ms > 0 {
            ratios
                .entry(r.role.to_string())
                .or_default()
                .push(r.hold_ms as f64 / r.gap_ms as f64);
        }
        art.max_note_end_ms = art.max_note_end_ms.max(r.at_ms + r.hold_ms);
        art.max_realized_note_end_ms = art
            .max_realized_note_end_ms
            .max(r.realized_at_ms + r.hold_ms);
    }
    art.mean_hold_over_gap_by_role = ratios
        .iter()
        .map(|(k, v)| (k.clone(), mean(v).unwrap_or(0.0)))
        .collect();

    // ── Melody↔voice per-step velocity correlation ────────────────────────────
    // Paired on the (step_idx) key so only steps where BOTH voices sounded contribute.
    let mut melody_velocity_pearson: BTreeMap<String, f64> = BTreeMap::new();
    let melody: BTreeMap<usize, f64> = s
        .step_velocities
        .iter()
        .filter(|(_, _, role, _, _)| *role == "Melody")
        .map(|(st, _, _, _, v)| (*st, *v as f64))
        .collect();
    if !melody.is_empty() {
        let others: BTreeSet<(usize, &'static str)> = s
            .step_velocities
            .iter()
            .filter(|(_, _, role, _, _)| *role != "Melody")
            .map(|(_, inst, role, _, _)| (*inst, *role))
            .collect();
        for (inst, role) in others {
            let series: BTreeMap<usize, f64> = s
                .step_velocities
                .iter()
                .filter(|(_, i, _, _, _)| *i == inst)
                .map(|(st, _, _, _, v)| (*st, *v as f64))
                .collect();
            let mut xs = Vec::new();
            let mut ys = Vec::new();
            for (st, mv) in &melody {
                if let Some(ov) = series.get(st) {
                    xs.push(*mv);
                    ys.push(*ov);
                }
            }
            if let Some(p) = pearson(&xs, &ys) {
                melody_velocity_pearson.insert(format!("{role}|inst{inst}"), r4(p));
            }
        }
    }

    // The STRUCTURAL correlation — the series E3 actually acts on. `level_gain` removed, the
    // cadence excluded (its gate zeroes the role differential), and pivot-path rows excluded
    // (they never reached `realize_velocity`, so their "structural" value is a placeholder).
    let mut melody_velocity_pearson_structural: BTreeMap<String, f64> = BTreeMap::new();
    let struct_series = |inst: usize| -> BTreeMap<usize, f64> {
        recs.iter()
            .filter(|r| r.inst_idx == inst && !r.is_cadence && r.velocity_detail_present)
            .map(|r| (r.step_idx, r.vel_structural))
            .collect()
    };
    let melody_inst: BTreeSet<usize> = recs
        .iter()
        .filter(|r| r.role == "Melody")
        .map(|r| r.inst_idx)
        .collect();
    if let Some(&mi) = melody_inst.iter().next() {
        let m = struct_series(mi);
        let others: BTreeSet<(usize, &'static str)> = recs
            .iter()
            .filter(|r| r.role != "Melody")
            .map(|r| (r.inst_idx, r.role))
            .collect();
        for (inst, role) in others {
            let o = struct_series(inst);
            let mut xs = Vec::new();
            let mut ys = Vec::new();
            for (st, mv) in &m {
                if let Some(ov) = o.get(st) {
                    xs.push(*mv);
                    ys.push(*ov);
                }
            }
            if let Some(p) = pearson(&xs, &ys) {
                melody_velocity_pearson_structural.insert(format!("{role}|inst{inst}"), r4(p));
            }
        }
    }

    let mut events_by_role: BTreeMap<String, u64> = BTreeMap::new();
    for r in recs {
        *events_by_role.entry(r.role.to_string()).or_default() += 1;
    }

    Counters {
        by_term,
        timing_coverage: tc,
        lattice: lat,
        velocity: vc,
        entrance_by_role,
        articulation: art,
        melody_velocity_pearson,
        melody_velocity_pearson_structural,
        events_bypassing_realize_velocity: recs
            .iter()
            .filter(|r| !r.velocity_detail_present)
            .count() as u64,
        cadence_events: recs.iter().filter(|r| r.is_cadence).count() as u64,
        pre_cadence_events: recs.iter().filter(|r| r.pre_cadence).count() as u64,
        frames: s.frames,
        silent_frames: s.silent_frames,
        total_events: recs.len() as u64,
        events_by_role,
    }
}

/// Build the dump document from the recorded state.
pub fn snapshot() -> Option<DumpJson> {
    let s = state().lock().ok()?;
    Some(DumpJson {
        format: "audiohax-event-dump/1",
        cue: s.cue.clone(),
        image: s.image.clone(),
        seed: s.seed,
        total_steps: s.total_steps,
        grid_ms_per_step: s.grid_ms_per_step,
        instrument_count: s.instrument_count,
        period_ms: s.total_steps as u64 * s.grid_ms_per_step,
        block_ms: BLOCK_MS,
        laps_rendered: 1,
        expression_armed: s.expression_armed,
        counters: build_counters(&s),
        events: s.records.clone(),
    })
}

/// Serialize the dump to pretty JSON.
pub fn to_json() -> Option<String> {
    snapshot().and_then(|d| serde_json::to_string_pretty(&d).ok())
}

// ───────────────────────────────────────────────────────────────────────────────
// Tests
// ───────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Serialize access so the global sink is not raced between `cargo test`'s threads.
    fn guard() -> std::sync::MutexGuard<'static, ()> {
        static G: OnceLock<Mutex<()>> = OnceLock::new();
        G.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }

    fn meta(role: &'static str, pos: usize, len: usize, phrase_position: &'static str) -> StepMeta {
        StepMeta {
            inst_idx: 3,
            role,
            section: "A".to_string(),
            phrase_index: 0,
            position_in_phrase: pos,
            phrase_len: len,
            phrase_position,
            floor_velocity: 76,
            ms_per_step: 500,
        }
    }

    fn onset(offset_ms: u64) -> Onset {
        Onset {
            note: 60,
            velocity: 80,
            hold_ms: 100,
            offset_ms,
        }
    }

    // ── LR-1 / the realization law ────────────────────────────────────────────

    /// The exactness theorem, at EVERY phase: a whole-rung request realizes exactly.
    #[test]
    fn lr1_whole_rung_requests_realize_exactly_at_every_phase() {
        for phase in 0..10u64 {
            for rungs in 0..6u64 {
                let base = 1000 + phase;
                let d = rungs * 10;
                let realized = realize_onset_ms(base + d) as i64 - realize_onset_ms(base) as i64;
                assert_eq!(
                    realized, d as i64,
                    "phase {phase}, {rungs} rungs: realized {realized} != requested {d}"
                );
            }
        }
    }

    /// The complementary fact, and the reason the floor exists: a SUB-rung request is
    /// phase-dependent — it realizes as 0 at some phases and as a full rung at others.
    /// If this test ever passes trivially, the lattice model in this module is wrong.
    #[test]
    fn sub_rung_requests_are_dithered_by_phase() {
        let d = 5u64;
        let mut seen: BTreeSet<i64> = BTreeSet::new();
        for phase in 0..10u64 {
            let base = 1000 + phase;
            seen.insert(realize_onset_ms(base + d) as i64 - realize_onset_ms(base) as i64);
        }
        assert!(
            seen.contains(&0) && seen.contains(&10),
            "a 5 ms request must realize as BOTH 0 and 10 across phases; saw {seen:?}"
        );
    }

    // ── the instrument must MOVE when the thing it measures moves ─────────────

    /// MUTATION, not re-reading: inject a known 20 ms displacement on onset 1 and assert the
    /// dump reports it as requested AND realized, with the per-term counter moving off zero.
    /// This is the positive control for the whole instrument — trap (a9) demands that an
    /// instrument be shown resolving a synthetic deviation of known size before it is trusted.
    #[test]
    fn injected_whole_rung_displacement_is_seen_end_to_end() {
        let _g = guard();
        arm("unit", "none", Some(102), true);
        set_plan(1, 500, 4);
        begin_step(0, 0);
        note_velocity_detail(84.0, 8.0, 84);
        note_grid_offsets(&[0, 125, 250, 375]);
        attribute(1, Term::TA);
        record_step_frame(
            &meta("Melody", 1, 8, "Interior"),
            &[onset(0), onset(145), onset(250), onset(375)],
        );
        let d = snapshot().expect("snapshot");
        disarm();

        let ev = &d.events[1];
        assert_eq!(ev.requested_dev_ms, 20);
        assert_eq!(
            ev.realized_dev_ms, 20,
            "LR-1: realized must equal requested"
        );
        assert_eq!(ev.terms_fired, vec!["T-A"]);

        let ta = d
            .counters
            .by_term
            .iter()
            .find(|t| t.term == "T-A")
            .expect("T-A cell");
        assert_eq!(ta.by_role["Melody"].displaced, 1);
        assert_eq!(ta.by_role["Melody"].claimed, 1);
        assert_eq!(d.counters.lattice.lr1_violations, 0);
        assert_eq!(d.counters.lattice.nonmultiple_requested_dev, 0);
        assert_eq!(d.counters.timing_coverage.melody_multi_onset_events, 4);
        assert_eq!(d.counters.timing_coverage.melody_multi_onset_displaced, 1);
    }

    /// A NON-whole-rung request must be reported as such — the dump must not launder a
    /// defective magnitude into a clean one. LR-5.
    #[test]
    fn non_whole_rung_displacement_is_flagged() {
        let _g = guard();
        arm("unit", "none", Some(102), true);
        begin_step(0, 3); // phase 3
        note_velocity_detail(84.0, 8.0, 84);
        note_grid_offsets(&[0, 125]);
        attribute(1, Term::TA);
        record_step_frame(&meta("Melody", 1, 8, "Interior"), &[onset(0), onset(132)]);
        let d = snapshot().expect("snapshot");
        disarm();

        assert_eq!(d.events[1].requested_dev_ms, 7);
        assert_eq!(d.counters.lattice.nonmultiple_requested_dev, 1);
        assert_eq!(d.counters.lattice.sub_floor_requests, 1);
        // At phase 3 a +7 ms request crosses the block edge, so it realizes as a full rung:
        // requested 7, realized 10 — the dither, visible, not silently absorbed.
        assert_eq!(d.events[1].realized_dev_ms, 10);
        assert_eq!(d.counters.lattice.dithered_requests, 1);
    }

    /// A term that displaces NOTHING must read zero **for that term**, even while another term
    /// is busy. This is trap (a7)'s aggregate-counter defect, asserted directly.
    #[test]
    fn per_term_counters_do_not_borrow_each_others_activity() {
        let _g = guard();
        arm("unit", "none", Some(102), true);
        begin_step(0, 0);
        // T-B moves a Fill downbeat; T-A moves nothing at all.
        note_velocity_detail(70.0, 4.0, 70);
        note_grid_offsets(&[0]);
        attribute(0, Term::TB);
        record_step_frame(&meta("HarmonicFill", 0, 8, "PhraseStart"), &[onset(20)]);
        let d = snapshot().expect("snapshot");
        disarm();

        let cell = |name: &str| {
            d.counters
                .by_term
                .iter()
                .find(|t| t.term == name)
                .unwrap()
                .all_roles
                .displaced
        };
        assert_eq!(cell("T-B"), 1, "T-B displaced one event");
        assert_eq!(cell("T-A"), 0, "T-A displaced nothing and must say so");
        assert_eq!(cell("T-C"), 0);
    }

    // ── eligibility matrix (AMENDMENT A.4) ────────────────────────────────────

    /// A single-onset step offers T-A nothing — the trap-(a7) structural fact.
    #[test]
    fn single_onset_step_is_ta_ineligible() {
        let m = meta("Melody", 2, 8, "Interior");
        assert!(eligible_terms("Melody", 0, 1, 0, &m).is_empty());
    }

    /// T-A is SUPPRESSED on pre-cadence steps (AMENDMENT A.4 redesign); T-C fires there
    /// instead, on the final onset only.
    #[test]
    fn pre_cadence_suppresses_ta_and_enables_tc_on_the_last_onset() {
        let m = meta("Melody", 6, 8, "Interior");
        assert!(m.pre_cadence(), "pos 6 of an 8-step phrase is pre-cadence");
        assert!(!eligible_terms("Melody", 1, 4, 125, &m).contains(&"T-A"));
        assert!(eligible_terms("Melody", 3, 4, 375, &m).contains(&"T-C"));
        assert!(!eligible_terms("Melody", 2, 4, 250, &m).contains(&"T-C"));
    }

    /// The downbeat anchor is never a T-A candidate, at any figure width.
    #[test]
    fn onset_zero_is_never_ta_eligible() {
        let m = meta("Melody", 2, 8, "Interior");
        for n in 2..6 {
            assert!(!eligible_terms("Melody", 0, n, 0, &m).contains(&"T-A"));
        }
    }

    /// T-B applies only to Fill/Pad onsets already sitting on the downbeat, and never on a
    /// cadence step (AMENDMENT A.4 amendment 2).
    #[test]
    fn tb_eligibility_is_downbeat_and_non_cadence_only() {
        let interior = meta("HarmonicFill", 2, 8, "Interior");
        assert!(eligible_terms("HarmonicFill", 0, 1, 0, &interior).contains(&"T-B"));
        assert!(!eligible_terms("HarmonicFill", 1, 2, 250, &interior).contains(&"T-B"));
        let cad = meta("HarmonicFill", 7, 8, "PerfectAuthenticCadence");
        assert!(!eligible_terms("HarmonicFill", 0, 1, 0, &cad).contains(&"T-B"));
        assert!(!eligible_terms("Bass", 0, 1, 0, &interior).contains(&"T-B"));
    }

    // ── velocity-side detectors ───────────────────────────────────────────────

    /// The clamp counter must fire on a pre-clamp value above 127 — trap (a2).
    #[test]
    fn clamp_counter_fires_above_127() {
        let _g = guard();
        arm("unit", "none", Some(102), true);
        begin_step(0, 0);
        note_velocity_detail(131.5, 18.0, 127);
        record_step_frame(&meta("Melody", 0, 8, "PhraseStart"), &[onset(0)]);
        let d = snapshot().expect("snapshot");
        disarm();
        assert_eq!(d.counters.velocity.clamp_high, 1);
        assert_eq!(d.counters.velocity.realized_max, 80);
        assert!((d.events[0].vel_structural - 113.5).abs() < 1e-6);
    }

    /// A one-phrase piece degenerates the E4 arch to a constant — trap (a4). The projected
    /// cardinality must be 0 (guarded), not a plausible-looking number.
    #[test]
    fn single_phrase_projects_no_arch() {
        let _g = guard();
        arm("unit", "none", Some(102), true);
        begin_step(0, 0);
        note_velocity_detail(84.0, 8.0, 84);
        record_step_frame(&meta("Melody", 0, 8, "PhraseStart"), &[onset(0)]);
        let d = snapshot().expect("snapshot");
        disarm();
        assert_eq!(d.counters.velocity.distinct_phrase_index, 1);
        assert_eq!(d.counters.velocity.projected_arch_distinct, 0);
    }

    /// A frame that never reached `realize_velocity` must be FLAGGED, not fabricated. The
    /// pivot-chord early return produces exactly this, and it is live on the shipped
    /// `gameplay-high` cue (steps 26 and 39).
    #[test]
    fn a_frame_without_velocity_detail_is_flagged_not_fabricated() {
        let _g = guard();
        arm("unit", "none", Some(102), true);
        begin_step(0, 0);
        // No `note_velocity_detail` call — the pivot path.
        record_step_frame(&meta("Melody", 0, 4, "PhraseStart"), &[onset(0)]);
        let d = snapshot().expect("snapshot");
        disarm();
        assert!(!d.events[0].velocity_detail_present);
        assert_eq!(d.counters.events_bypassing_realize_velocity, 1);
    }

    /// An exactly-even REQUESTED figure need not arrive even: both endpoints are independently
    /// floor-quantized. `gameplay-high`'s arpeggio requests `(166, 166)` and realizes
    /// `(160, 170)` — reproduced here so the claim is a test, not a paragraph.
    #[test]
    fn an_exactly_even_requested_figure_realizes_unevenly() {
        let _g = guard();
        arm("unit", "none", Some(102), true);
        begin_step(1, 500);
        note_velocity_detail(84.0, 8.0, 84);
        record_step_frame(
            &meta("Melody", 1, 4, "Interior"),
            &[onset(0), onset(166), onset(332)],
        );
        let d = snapshot().expect("snapshot");
        disarm();
        let req: Vec<u64> = d.events[..2].iter().map(|e| e.gap_ms).collect();
        let real: Vec<i64> = d.events[..2].iter().map(|e| e.realized_gap_ms).collect();
        assert_eq!(req, vec![166, 166], "the request IS exactly even");
        assert_eq!(real, vec![160, 170], "the realization is NOT");
        assert_eq!(d.counters.lattice.figures_requested_even, 1);
        assert_eq!(d.counters.lattice.figures_realized_even, 0);
    }

    /// A chord — several tones at ONE offset — is not a figure and must not be scored as a
    /// perfectly even one. Left uncorrected this inflates `figures_requested_even` by every Pad
    /// block bed in the piece and makes the lattice look kinder than it is.
    #[test]
    fn a_chord_is_not_an_even_figure() {
        let _g = guard();
        arm("unit", "none", Some(102), true);
        begin_step(0, 0);
        note_velocity_detail(70.0, 4.0, 70);
        record_step_frame(
            &meta("Pad", 0, 4, "PhraseStart"),
            &[
                Onset {
                    note: 60,
                    velocity: 70,
                    hold_ms: 400,
                    offset_ms: 0,
                },
                Onset {
                    note: 64,
                    velocity: 70,
                    hold_ms: 400,
                    offset_ms: 0,
                },
                Onset {
                    note: 67,
                    velocity: 70,
                    hold_ms: 400,
                    offset_ms: 0,
                },
            ],
        );
        let d = snapshot().expect("snapshot");
        disarm();
        assert_eq!(d.counters.lattice.figures_requested_even, 0);
        assert_eq!(d.counters.lattice.figures_realized_even, 0);
    }

    /// Two chord tones sharing an onset are not a lattice fusion; two DISTINCT requests landing
    /// in one block are. A counter that conflates them reads 62 on a cue that fuses nothing.
    #[test]
    fn lattice_fusion_excludes_chord_tones_and_catches_real_fusions() {
        let _g = guard();
        arm("unit", "none", Some(102), true);
        begin_step(0, 0);
        note_velocity_detail(70.0, 4.0, 70);
        // Three simultaneous chord tones (a Pad block bed) — NOT a fusion.
        record_step_frame(
            &meta("Pad", 0, 4, "PhraseStart"),
            &[
                Onset {
                    note: 60,
                    velocity: 70,
                    hold_ms: 400,
                    offset_ms: 0,
                },
                Onset {
                    note: 64,
                    velocity: 70,
                    hold_ms: 400,
                    offset_ms: 0,
                },
                Onset {
                    note: 67,
                    velocity: 70,
                    hold_ms: 400,
                    offset_ms: 0,
                },
            ],
        );
        let d = snapshot().expect("snapshot");
        assert_eq!(
            d.counters.lattice.lattice_fused_onsets, 0,
            "a chord is not a fusion"
        );

        begin_step(1, 500);
        note_velocity_detail(70.0, 4.0, 70);
        // Two DISTINCT requests 3 ms apart — the lattice fuses them into one block.
        record_step_frame(
            &meta("Pad", 1, 4, "Interior"),
            &[
                Onset {
                    note: 60,
                    velocity: 70,
                    hold_ms: 100,
                    offset_ms: 1,
                },
                Onset {
                    note: 64,
                    velocity: 70,
                    hold_ms: 100,
                    offset_ms: 4,
                },
            ],
        );
        let d = snapshot().expect("snapshot");
        disarm();
        assert_eq!(
            d.counters.lattice.lattice_fused_onsets, 1,
            "501 and 504 fuse into block 500"
        );
    }

    /// Two identical series correlate at 1.0; the correlation must actually be computed,
    /// not defaulted.
    #[test]
    fn pearson_is_real() {
        assert!(
            (pearson(&[1.0, 2.0, 3.0, 4.0], &[1.0, 2.0, 3.0, 4.0]).unwrap() - 1.0).abs() < 1e-9
        );
        assert!(
            (pearson(&[1.0, 2.0, 3.0, 4.0], &[4.0, 3.0, 2.0, 1.0]).unwrap() + 1.0).abs() < 1e-9
        );
        assert!(pearson(&[1.0, 1.0, 1.0, 1.0], &[1.0, 2.0, 3.0, 4.0]).is_none());
    }

    /// Disarmed, every hook is a no-op and the snapshot never grows. The default render path
    /// depends on exactly this.
    #[test]
    fn disarmed_hooks_record_nothing() {
        let _g = guard();
        arm("unit", "none", Some(102), false);
        disarm();
        begin_step(7, 3500);
        note_velocity_detail(99.0, 0.0, 99);
        note_grid_offsets(&[0]);
        attribute(0, Term::TA);
        record_step_frame(&meta("Melody", 0, 8, "PhraseStart"), &[onset(0)]);
        let d = snapshot().expect("snapshot");
        assert_eq!(d.events.len(), 0);
        assert_eq!(d.counters.frames, 0);
    }

    /// A frame that emits no onsets (the theme-driven melody rest) is COUNTED, not dropped —
    /// a denominator that omits its rests overstates coverage.
    #[test]
    fn silent_frames_are_counted() {
        let _g = guard();
        arm("unit", "none", Some(102), true);
        begin_step(0, 0);
        record_step_frame(&meta("Melody", 0, 8, "PhraseStart"), &[]);
        let d = snapshot().expect("snapshot");
        disarm();
        assert_eq!(d.counters.frames, 1);
        assert_eq!(d.counters.silent_frames, 1);
        assert_eq!(d.counters.total_events, 0);
    }

    /// The entrance view must report an INVERSION as positive — the sign convention is
    /// load-bearing for the falsification claim.
    #[test]
    fn entrance_view_reports_start_louder_than_cadence_as_positive() {
        let _g = guard();
        arm("unit", "none", Some(102), true);
        begin_step(0, 0);
        note_velocity_detail(107.0, 8.0, 107); // structural 99
        record_step_frame(&meta("Melody", 0, 8, "PhraseStart"), &[onset(0)]);
        begin_step(1, 500);
        note_velocity_detail(82.0, 8.0, 82); // structural 74
        record_step_frame(&meta("Melody", 1, 8, "Interior"), &[onset(0)]);
        begin_step(7, 3500);
        note_velocity_detail(104.0, 8.0, 104); // structural 96
        record_step_frame(
            &meta("Melody", 7, 8, "PerfectAuthenticCadence"),
            &[onset(0)],
        );
        let d = snapshot().expect("snapshot");
        disarm();
        let cell = &d.counters.entrance_by_role["Melody"];
        assert_eq!(cell.phrase_start_structural_mode, Some(99));
        assert_eq!(cell.cadence_structural_mode, Some(96));
        assert_eq!(cell.start_minus_cadence, Some(3), "positive == inverted");
        assert_eq!(cell.entrance_step_units, Some(25));
        let db = cell.entrance_step_db.unwrap();
        assert!(
            (db - 5.05).abs() < 0.02,
            "the 99/74 step must read ~5.05 dB, got {db}"
        );
    }
}

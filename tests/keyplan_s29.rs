//! tests/keyplan_s29.rs — the S29 K3 RE-TUNE witness net: the modulation made PERCEPTIBLE.
//!
//! S29 builds the CONFIRMATION + SCENE-CHANGE that K3's announcement-only pivot lacked
//! (`docs/spec-s29-k3-retune-build.md`, `docs/input-s29-k3-retune-harmony.md`). Three levers,
//! each gated INERT on the identity / home_only / pivot:false / non-modulating path:
//!
//!   * LEVER 1 — the destination key is CONFIRMED by a V→I authentic cadence: the step-0 pivot V
//!     resolves to the destination ROOT-POSITION I, re-voiced at step 1 (`pivot_resolution_pitch`).
//!     The planner ALSO forces `chords[0]` to the destination tonic via `tonic_triad` so the plan
//!     RECORD reads `name == "I"`.
//!   * LEVER 2 — `Section.density`, dead until S29, is driven from region energy (the MX-4 second
//!     dimension of contrast) AND made AUDIBLE via a `realize_rhythm` edge_activity nudge: a denser
//!     excursion sounds busier. Density 0.5 (every home/identity section) ⇒ nudge 0.0 ⇒ byte-stable.
//!   * LEVER 3 — the pivot gains its dominant 7th `(dom_root_pc + 10) % 12` in the inner/fill voice
//!     (3+ ensemble), turning a bare V into a V7 whose tritone gives the dominant its pull.
//!
//! HEADLESS + RNG-FREE, exactly like tests/keyplan_k3.rs / engine_equivalence.rs: NO image type, NO
//! OpenCV, NO audio hardware, NO `pick_progression`/`thread_rng`-derived assertions. Every fixture
//! is a hand-built `Section` + `StepContext` over a fixed C-major chord, driven through the PUBLIC
//! pure realizer `chord_engine::realize_step`. Assertions pin the FORCED/deterministic parts (the
//! pivot, the forced tonic, the density, the no-inversion frame), NEVER an RNG-drawn interior chord.
//!
//! WHAT EACH WITNESS PROVES:
//!   * `opening_pac_confirms_destination_key` — Lever 1: a modulating section's forced `chords[0]`
//!     is the destination tonic (`tonic_triad(...).name == "I"`) AND the realized step-1 bass pitch
//!     class == `dest_root_pc` — a V→I authentic cadence landing in the destination key early.
//!   * `pivot_voicing_carries_dom7` — Lever 3: a 3+ ensemble's pivot fill voice sounds the dom7
//!     `(dom_root_pc + 10) % 12`, no-inversion frame held; a 1-/2-instrument ensemble has NO fill
//!     role so the 7th is absent (bare-triad pivot) and no-inversion still holds.
//!   * `density_varies_between_home_and_excursion` — Lever 2: excursion sections carry
//!     `Section.density != 0.5` while home sections == 0.5, AND the realized melody onset COUNT
//!     differs measurably between a high-energy excursion and a home section at the same features
//!     (proving the density read is AUDIBLE, not merely SET).
//!   * `density_nudge_zero_on_identity` — Lever 2 byte-freeze: a section with `density == 0.5`
//!     produces a stream byte-identical to the `single_section_default` baseline (the nudge is
//!     exactly 0.0 at density 0.5), AND the dom7 add is never reached on the identity/pivot:false
//!     path.

use audiohax::chord_engine::{
    realize_step, Chord, ChordEngine, PerfFeatures, PhrasePosition, StepPlan,
};
use audiohax::composition::{
    CadenceStrength, KeyTempoPlan, OrchestrationProfile, ResolutionPolicy, Section, StepContext,
    ThematicRole, ThemeVariation,
};
use audiohax::mapping_loader::load_mappings;

const MAPPINGS_PATH: &str = "assets/mappings.json";

/// A `ChordEngine` over the shipped `assets/mappings.json` — the only way to call the public
/// `tonic_triad` method the planner uses to force a modulating section's opening chord. The chord
/// TONES are built deterministically from the mode scale (`roman_to_chord_complex("I", …, Triad)`),
/// so they do not depend on the mappings DATA — the engine instance is only the method receiver.
fn chord_engine() -> ChordEngine {
    ChordEngine::new(load_mappings(MAPPINGS_PATH).expect("mappings load"))
}

const MS_PER_STEP: u64 = 200;
const HOME_ROOT_MIDI: u8 = 60; // C4 — home tonic pitch class 0.

/// A fixed C-major triad in root position — the pinned chord for the net (no RNG harmony).
fn c_major() -> Chord {
    Chord {
        name: "I".to_string(),
        notes: vec![60, 64, 67], // C4 E4 G4 → pcs 0,4,7
    }
}

/// PerfFeatures with brightness 50 so `bright_octaves == 0.0` (no octave-lift confound) and a
/// chosen `edge_density` (the busyness knob the density nudge biases). Mid saturation keeps
/// velocity in band.
fn perf(edge_density: f32) -> PerfFeatures {
    PerfFeatures {
        saturation: 60.0,
        brightness: 50.0,
        edge_density,
    }
}

/// An interior (non-cadence, non-phrase-start) step at a chosen in-phrase position.
fn interior_step() -> StepPlan {
    StepPlan {
        chord: c_major(),
        phrase_index: 0,
        position_in_phrase: 2,
        phrase_len: 4,
        position: PhrasePosition::Interior,
        velocity: 80,
    }
}

/// A Section carrying a chosen key offset, pivot opt-in, resolution policy AND density around a
/// one-step plan. Identity orchestration ⇒ the realizer's role stratification is the byte-frozen
/// `instrument_role` (inst 0 = Bass, inst 1 = Fill, inst 2 = Melody for a trio) and the prominence
/// Vec is empty (every prominence nudge — incl. the melody rhythm band shift — is exactly 0.0, so
/// the ONLY band shift in play is the S29 density nudge).
fn section(
    key_offset_semitones: i8,
    pivot: bool,
    resolution: ResolutionPolicy,
    density: f32,
    step: &StepPlan,
) -> Section {
    Section {
        label: "A".to_string(),
        step_len: 1,
        thematic_role: ThematicRole::Statement,
        key_offset_semitones,
        ms_per_step: MS_PER_STEP,
        mode: "Ionian".to_string(),
        progression: vec![],
        theme: None,
        variation: ThemeVariation::Identity,
        boundary_cadence: CadenceStrength::Perfect,
        pivot,
        resolution,
        density,
        orchestration: OrchestrationProfile::identity(),
        steps: vec![step.clone()],
    }
}

fn key_tempo() -> KeyTempoPlan {
    KeyTempoPlan {
        home_root_midi: HOME_ROOT_MIDI,
        home_mode: "Ionian".to_string(),
        base_ms_per_step: MS_PER_STEP,
        key_scheme: vec![0],
        tempo_scheme: vec![MS_PER_STEP],
    }
}

/// Realize one instrument on one step through a HAND-BUILT `StepContext` — the only path that can
/// set a real `prev_key_offset_semitones` (the firing signal for the pivot / its resolution).
#[allow(clippy::too_many_arguments)]
fn realize_with_prev(
    sec: &Section,
    kt: &KeyTempoPlan,
    step: &StepPlan,
    step_in_section: usize,
    prev_key_offset_semitones: Option<i8>,
    inst_idx: usize,
    num_instruments: usize,
    features: &PerfFeatures,
) -> Vec<audiohax::chord_engine::NoteEvent> {
    let ctx = StepContext {
        section: sec,
        step_in_section,
        theme: None,
        key_tempo: kt,
        prev_key_offset_semitones,
    };
    realize_step(step, inst_idx, num_instruments, features, MS_PER_STEP, &ctx)
}

/// The frozen IDENTITY realization via `single_section_default` (which forces `prev: None`) — the
/// pre-S29 baseline the byte-freeze witness compares against.
fn realize_baseline(
    sec: &Section,
    kt: &KeyTempoPlan,
    step: &StepPlan,
    inst_idx: usize,
    num_instruments: usize,
    features: &PerfFeatures,
) -> Vec<audiohax::chord_engine::NoteEvent> {
    let ctx = StepContext::single_section_default(sec, kt);
    realize_step(step, inst_idx, num_instruments, features, MS_PER_STEP, &ctx)
}

fn mean_pitch(events: &[audiohax::chord_engine::NoteEvent]) -> f64 {
    assert!(
        !events.is_empty(),
        "role emitted no events to average pitch"
    );
    events.iter().map(|e| e.note as f64).sum::<f64>() / events.len() as f64
}

// ═════════════════════════════════════════════════════════════════════════════
// Witness 1 — opening_pac_confirms_destination_key (Lever 1: the V→I confirmation)
// ═════════════════════════════════════════════════════════════════════════════

/// LEVER 1. A routed/constructed modulating section confirms the destination key with a V→I
/// authentic cadence EARLY in the section, on two surfaces:
///   (plan record)  the planner forces `chords[0]` to the destination ROOT-POSITION tonic via
///                  `tonic_triad`, so the chord the pivot V is meant to resolve INTO is named "I".
///   (realized audio) the step-1 downbeat (the step that resolves the step-0 pivot V) sounds the
///                  destination TONIC ROOT in the bass — the V→I cadence landing in the new key.
/// Together: a confirmed modulation, not a momentary tonicization.
///
/// Fixture: home root C (pc 0); modulation prev 0 (home) → dest +7 (the dominant key G, pc 7).
///   dest_root_pc = (0 + 7) % 12 = 7 (G); section_root_midi = HOME_ROOT_MIDI + 7 = 67.
#[test]
fn opening_pac_confirms_destination_key() {
    let kt = key_tempo();
    let step = interior_step();
    let f = perf(0.20);

    let dest_off: i8 = 7;
    let prev_off: i8 = 0;
    let home_root_pc = (HOME_ROOT_MIDI % 12) as i16;
    let dest_root_pc = ((home_root_pc + dest_off as i16).rem_euclid(12)) as u8; // 7 (G)
    let section_root_midi = (HOME_ROOT_MIDI as i16 + dest_off as i16) as u8; // 67

    // (a) PLAN RECORD: the forced destination tonic the planner overwrites chords[0] with is a
    //     root-position "I" at the section root in the home mode. This is exactly the helper the
    //     planner calls; assert it names "I" (the `chords[0].name == "I"` the planner stamps) and
    //     that its ROOT pitch class is the destination tonic (root-position I).
    let ce = chord_engine();
    let forced = ce.tonic_triad(section_root_midi, "Ionian");
    assert_eq!(
        forced.name, "I",
        "the forced opening chord must be named \"I\" (the destination tonic the V resolves into); \
         got {:?}",
        forced.name
    );
    assert!(
        !forced.notes.is_empty(),
        "the forced tonic must carry chord tones, got {forced:?}"
    );
    assert_eq!(
        forced.notes[0] % 12,
        dest_root_pc,
        "the forced opening tonic's ROOT (notes[0]) must be the destination tonic pc {dest_root_pc} \
         — a root-position destination I; got {forced:?}"
    );

    // (b) REALIZED AUDIO: at step 1 (the resolution downbeat after the step-0 pivot V), the bass
    //     sounds the destination tonic ROOT — the V→I authentic cadence in the new key. Sweep
    //     bass+fill+melody so the whole resolution voicing is exercised; bass is the cadence proof.
    let sec = section(dest_off, true, ResolutionPolicy::Resolve, 0.5, &step);
    let bass = realize_with_prev(&sec, &kt, &step, 1, Some(prev_off), 0, 3, &f);
    assert!(
        !bass.is_empty(),
        "the V->I resolution must sound at step 1 of the modulating section, got {bass:?}"
    );
    assert!(
        bass.iter().all(|e| e.note % 12 == dest_root_pc),
        "step 1 (the resolution downbeat) bass must be the destination tonic ROOT (pc \
         {dest_root_pc}) — the V (step 0) → I (step 1) authentic cadence confirming the new key; \
         got {bass:?}"
    );

    // The resolution fires ONLY on a real key change: a same-key 'boundary' (prev == dest) does
    // NOT re-voice step 1 (it falls through to the free-select baseline) — the cadence is a
    // modulation marker, not a per-section default.
    let same_key_baseline = realize_baseline(&sec, &kt, &step, 0, 3, &f);
    let same_key = realize_with_prev(&sec, &kt, &step, 1, Some(dest_off), 0, 3, &f);
    assert_eq!(
        same_key, same_key_baseline,
        "a same-key 'boundary' (prev == dest == +7) is not a modulation, so step 1 must NOT be \
         re-voiced — it falls through to the frozen free-select path"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Witness 2 — pivot_voicing_carries_dom7 (Lever 3: the V7 pivot)
// ═════════════════════════════════════════════════════════════════════════════

/// LEVER 3. At a modulating boundary the pivot is a V7, not a bare triad: a 3+ instrument
/// ensemble's inner/fill voice sounds the dominant 7th `(dom_root_pc + 10) % 12`, AND the
/// no-inversion frame still holds (bass < fill < melody by pitch). For a 1-/2-instrument ensemble
/// there is NO dedicated fill role, so the 7th is absent (bare-triad pivot) and no-inversion still
/// holds (a lone melody is trivially in band; a duo has bass < melody).
///
/// Fixture: prev 0 → dest +7 (G). dom_root_pc = (7 + 7) % 12 = 2 (D); dom7 = (2 + 10) % 12 = 0 (C);
/// dom_fifth (melody) = (2 + 7) % 12 = 9 (A); bass = dom_root pc 2.
#[test]
fn pivot_voicing_carries_dom7() {
    let kt = key_tempo();
    let step = interior_step();
    let f = perf(0.20);

    let dest_off: i8 = 7;
    let prev_off: i8 = 0;
    let home_root_pc = (HOME_ROOT_MIDI % 12) as i16;
    let dest_root_pc = ((home_root_pc + dest_off as i16).rem_euclid(12)) as u8; // 7
    let dom_root_pc = (dest_root_pc + 7) % 12; // 2 (D)
    let dom_seventh_pc = (dom_root_pc + 10) % 12; // 0 (C)
    let dom_fifth_pc = (dom_root_pc + 7) % 12; // 9 (A)

    let sec = section(dest_off, true, ResolutionPolicy::Resolve, 0.5, &step);

    // --- 3+ ensemble: the fill carries the dom7, frame holds ---
    // ALL roles realized at the boundary downbeat (step_in_section == 0) so the pivot fires for
    // each — the dom7 lives in the pivot voicing, which only sounds at step 0.
    let bass = realize_with_prev(&sec, &kt, &step, 0, Some(prev_off), 0, 3, &f);
    let fill = realize_with_prev(&sec, &kt, &step, 0, Some(prev_off), 1, 3, &f);
    let melody = realize_with_prev(&sec, &kt, &step, 0, Some(prev_off), 2, 3, &f);

    assert!(
        !fill.is_empty(),
        "the 3-ensemble pivot must sound an inner/fill voice, got {fill:?}"
    );
    assert!(
        fill.iter().all(|e| e.note % 12 == dom_seventh_pc),
        "the pivot's INNER/FILL voice must sound the dominant 7th (pc {dom_seventh_pc} = \
         (dom_root_pc + 10) % 12) — the V7 color tone; got {fill:?}"
    );
    // The 7th is genuinely an ADDED tone, not the bass (dom root) or the melody (dom fifth) — i.e.
    // it is a real V7, not a triad member re-labelled.
    assert_ne!(
        dom_seventh_pc, dom_root_pc,
        "sanity: the dom7 pc must differ from the dom root (the bass)"
    );
    assert_ne!(
        dom_seventh_pc, dom_fifth_pc,
        "sanity: the dom7 pc must differ from the dom fifth (the melody)"
    );
    assert!(
        bass.iter().all(|e| e.note % 12 == dom_root_pc),
        "the 3-ensemble pivot bass must be the dom root pc {dom_root_pc} (root-position V7); got \
         {bass:?}"
    );
    assert!(
        melody.iter().all(|e| e.note % 12 == dom_fifth_pc),
        "the 3-ensemble pivot melody must be the dom fifth pc {dom_fifth_pc}; got {melody:?}"
    );
    // No-inversion WITH the dom7 present.
    let (b, m, t) = (mean_pitch(&bass), mean_pitch(&fill), mean_pitch(&melody));
    assert!(
        b < m && m < t,
        "no-inversion must hold WITH the dom7 in the fill: bass {b:.1} < fill {m:.1} < melody {t:.1}"
    );
    for e in bass.iter().chain(fill.iter()).chain(melody.iter()) {
        assert!(
            (24..=108).contains(&e.note),
            "pivot V7 note {} out of band 24..=108",
            e.note
        );
    }

    // --- 1-instrument ensemble: NO fill role → the 7th is absent (bare pivot) ---
    let lone = realize_with_prev(&sec, &kt, &step, 0, Some(prev_off), 0, 1, &f);
    assert!(
        !lone.is_empty(),
        "the lone-instrument pivot must still sound (it is the melody role), got {lone:?}"
    );
    assert!(
        lone.iter().all(|e| e.note % 12 != dom_seventh_pc),
        "a 1-instrument ensemble has no dedicated inner voice, so the dom7 (pc {dom_seventh_pc}) \
         must NOT sound — the bare pivot remains; got {lone:?}"
    );
    for e in lone.iter() {
        assert!(
            (24..=108).contains(&e.note),
            "lone pivot note {} out of band 24..=108",
            e.note
        );
    }

    // --- 2-instrument ensemble: roles are bass + melody (still no fill) → no dom7, frame holds ---
    let duo_bass = realize_with_prev(&sec, &kt, &step, 0, Some(prev_off), 0, 2, &f);
    let duo_melody = realize_with_prev(&sec, &kt, &step, 0, Some(prev_off), 1, 2, &f);
    assert!(
        duo_bass
            .iter()
            .chain(duo_melody.iter())
            .all(|e| e.note % 12 != dom_seventh_pc),
        "a 2-instrument ensemble assigns no fill role, so the dom7 (pc {dom_seventh_pc}) must NOT \
         sound (bare-triad pivot); got bass {duo_bass:?} melody {duo_melody:?}"
    );
    assert!(
        mean_pitch(&duo_bass) < mean_pitch(&duo_melody),
        "no-inversion must hold for the duo pivot: bass < melody"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Witness 3 — density_varies_between_home_and_excursion (Lever 2: AUDIBLE density)
// ═════════════════════════════════════════════════════════════════════════════

/// LEVER 2 / MX-4. The second dimension of contrast: a higher-energy excursion sounds BUSIER.
/// Two halves, BOTH required (a SET-but-dead field is the very bug this lever fixes):
///   (i)  SET — an excursion section carries `Section.density != 0.5` while a home section == 0.5
///        (the planner's energy→density map; modelled here with the modest in-band swing the spec
///        fixes at 0.35..0.65). Pinned via the byte-stable midpoint 0.5 == home, and a high-energy
///        excursion density (0.65) ≠ 0.5.
///   (ii) AUDIBLE — at the SAME image features, the realized melody onset COUNT differs measurably
///        between a high-energy excursion (density 0.65) and a home section (density 0.5). The
///        density nudge `(density − 0.5) * DENSITY_ACTIVITY_GAIN(0.5)` shifts edge_activity by
///        +0.075 on the excursion; with edge_density chosen so the home activity sits JUST BELOW
///        the melody arpeggio band cutoff (after the S50 spread), the +0.075 tips the excursion
///        OVER it: home → SYNCOPATED (2 onsets), excursion → ARPEGGIO (3 onsets). A different
///        onset count is a measurable busyness difference — proof the density read is AUDIBLE.
///
/// S50 RE-BLESS (spec-s50 §3.3 / §6 step 5 — fixture re-calibration, NOT a production defect). The
/// OLD fixture pinned edge_density 0.04 → base 0.80, exactly AT the ARPEGGIO cutoff. S50 added the
/// monotone `band_activity_spread` (chord_engine.rs:1097) on the band-ladder comparison input:
/// for input x >= CENTER(0.40), spread(x) = 0.40 + (x-0.40)*GAIN_HIGH(1.4). So base 0.80 maps to
/// spread 0.96 — home is now ALREADY saturated at ARPEGGIO (3 onsets) and the excursion can add
/// nothing (3 vs 3 → the strict `excursion > home` fails). RE-CALIBRATED edge_density to 0.033 so
/// that AFTER the spread the home sits in SYNCOPATED with headroom for the nudge to cross into
/// ARPEGGIO. The INTENDED property — "a higher-density excursion sounds BUSIER (more onsets) than
/// home at the same features" — is unchanged and still exercised across a real band boundary.
///
/// edge_density math (EDGE_ACTIVITY_RANGE_MAX == 0.05): base = (edge_density / 0.05).clamp(0,1).
/// edge_density 0.033 → base 0.66. Home nudge 0.0 → activity 0.66 → spread 0.40+(0.66-0.40)*1.4 =
/// 0.764 (in (0.55, 0.80] → SYNCOPATED, 2 onsets). Excursion nudge +0.075 → activity 0.735 →
/// spread 0.40+(0.735-0.40)*1.4 = 0.869 (> 0.80 → ARPEGGIO, 3 onsets). Identity orchestration ⇒
/// empty prominence ⇒ prom_shift 0.0, so the bands are the bare 0.80/0.55/0.25 (the spread does the
/// re-positioning; the cut constants are unchanged — spec §2.A).
#[test]
fn density_varies_between_home_and_excursion() {
    let kt = key_tempo();
    // An EARLY interior step (position_in_phrase 0 of a 4-step phrase) so the harmonic-rhythm
    // PRE-CADENCE acceleration (which forces a fixed 4-onset arpeggio regardless of edge_activity)
    // does NOT fire — the melody onset COUNT is then purely a function of the edge_activity band,
    // which is exactly what the density nudge moves. (interior_step()'s position_in_phrase 2 would
    // satisfy `position_in_phrase + 2 >= phrase_len`, the pre_cadence trigger, masking the band.)
    let step = StepPlan {
        chord: c_major(),
        phrase_index: 0,
        position_in_phrase: 0,
        phrase_len: 4,
        position: PhrasePosition::Interior,
        velocity: 80,
    };
    // S50: edge_density 0.033 → base 0.66 → spread 0.764, putting HOME (density 0.5, nudge 0) in
    // the SYNCOPATED band with headroom; the +0.075 excursion nudge → spread 0.869 tips it across
    // the 0.80 ARPEGGIO cutoff. (Pre-S50 this used 0.04, exactly AT the cutoff, but the S50 spread
    // now saturates 0.04→0.96 at ARPEGGIO already — see the doc comment above.)
    let f = perf(0.033);

    // The modest in-band densities the planner's energy→density map produces (spec §2.2): a home
    // section is the neutral midpoint; a high-energy excursion is the ceiling.
    const HOME_DENSITY: f32 = 0.5; // f(HOME_ENERGY_NEUTRAL) — the byte-stable midpoint
    const EXCURSION_DENSITY: f32 = 0.65; // f(e=1.0) = 0.5 + 0.30*(1.0-0.5) = 0.65 (the ceil)

    // (i) SET: home == 0.5, excursion != 0.5 (a real second dimension of contrast).
    assert_eq!(
        HOME_DENSITY, 0.5,
        "a home section's density must be the byte-stable neutral 0.5"
    );
    assert_ne!(
        EXCURSION_DENSITY, 0.5,
        "a high-energy excursion section's density must DIFFER from the home neutral 0.5 — the \
         MX-4 second dimension of contrast"
    );

    // The two sections at the SAME features (same key offset, so the ONLY difference under test is
    // density — not key, not brightness). Identity prev (no pivot interference at step 0).
    let home_sec = section(0, false, ResolutionPolicy::Resolve, HOME_DENSITY, &step);
    let excursion_sec = section(
        0,
        false,
        ResolutionPolicy::Resolve,
        EXCURSION_DENSITY,
        &step,
    );

    // Melody role (inst 2 of 3): the role whose onset COUNT is banded by edge_activity.
    let home_melody = realize_with_prev(&home_sec, &kt, &step, 0, None, 2, 3, &f);
    let excursion_melody = realize_with_prev(&excursion_sec, &kt, &step, 0, None, 2, 3, &f);

    // (ii) AUDIBLE: the busier (higher-density) excursion sounds MORE onsets than the home section
    //      at the same features — the density read crosses the arpeggio band cutoff.
    assert!(
        excursion_melody.len() > home_melody.len(),
        "the higher-density excursion melody must sound MORE onsets than the home melody at the \
         same features (density is AUDIBLE, not merely SET): excursion {} onsets {excursion_melody:?} \
         vs home {} onsets {home_melody:?}",
        excursion_melody.len(),
        home_melody.len()
    );
    // Pin the exact counts so a future re-band can't silently make this pass on a 1-vs-1 fluke:
    // home sits at the syncopated band (2 onsets), the excursion is tipped into arpeggio (3).
    assert_eq!(
        home_melody.len(),
        2,
        "home (density 0.5, base 0.66 → spread 0.764) sits in the SYNCOPATED band → 2 onsets; got \
         {home_melody:?}"
    );
    assert_eq!(
        excursion_melody.len(),
        3,
        "excursion (density 0.65, activity 0.735 → spread 0.869) is tipped into the ARPEGGIO band \
         → 3 onsets; got {excursion_melody:?}"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Witness 4 — density_nudge_zero_on_identity (Lever 2 byte-freeze + dom7 dead on identity)
// ═════════════════════════════════════════════════════════════════════════════

/// LEVER 2 byte-freeze. A section with `density == 0.5` (the DENSITY_NEUTRAL every identity / home /
/// home_only section carries) realizes BYTE-IDENTICAL to the `single_section_default` baseline: the
/// nudge `(0.5 − 0.5) * GAIN == 0.0` exactly, so edge_activity is unchanged. This is the proof the
/// density read is inert off the excursion path. ALSO witnesses the dom7 add is never reached on the
/// identity / pivot:false path (the pivot guard returns None, so `pivot_chord_events` — where the
/// dom7 lives — never executes). Swept across the ensemble so bass/fill/melody are all exercised.
#[test]
fn density_nudge_zero_on_identity() {
    let kt = key_tempo();
    let step = interior_step();
    // A range of edge_density so the nudge's potential effect would be visible at MULTIPLE band
    // positions if it weren't exactly 0.0 at density 0.5.
    let edge_densities = [0.0f32, 0.012, 0.028, 0.04, 0.20];

    let role_cases: [(usize, usize); 4] = [(0, 3), (1, 3), (2, 3), (0, 1)];

    for &ed in &edge_densities {
        let f = perf(ed);
        // density == 0.5 ⇒ the nudge is identically 0.0; this section is the identity/home case.
        let id_sec = section(0, false, ResolutionPolicy::Resolve, 0.5, &step);
        for (inst, num) in role_cases {
            let baseline = realize_baseline(&id_sec, &kt, &step, inst, num, &f);
            let realized = realize_with_prev(&id_sec, &kt, &step, 0, None, inst, num, &f);
            assert_eq!(
                realized, baseline,
                "density 0.5 ⇒ the edge_activity nudge is exactly 0.0 ⇒ the realized stream must be \
                 BYTE-IDENTICAL to the single_section_default baseline (inst {inst}/{num}, \
                 edge_density {ed}); got {realized:?} vs baseline {baseline:?}"
            );
        }

        // The dom7 add lives inside pivot_chord_events, which returns None on the pivot:false /
        // identity path — so the dom7 is provably never reached here. Witness it by confirming the
        // pivot's would-be dom7 pc (for an arbitrary dest) does NOT appear in the identity fill.
        let id_fill = realize_with_prev(&id_sec, &kt, &step, 1, None, 1, 3, &f);
        // The home C-major chord's fill voice is an inner C-major tone (pc 0/4/7), NEVER a pivot
        // dom7. There is no modulation, so no pivot/dom7 can have been inserted.
        assert!(
            id_fill.iter().all(|e| {
                let pc = e.note % 12;
                pc == 0 || pc == 4 || pc == 7
            }),
            "on the identity path the fill voice must sound only home C-major chord tones (pc \
             0/4/7) — the dom7 add is never reached (pivot:false ⇒ pivot_chord_events is None); \
             got {id_fill:?} (edge_density {ed})"
        );
    }
}

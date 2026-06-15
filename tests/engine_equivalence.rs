//! tests/engine_equivalence.rs — the BATCH-EQUIVALENCE regression net (design S9
//! §5, the load-bearing one). It PINS the deterministic per-instrument decision
//! algorithm `engine::decide_instrument_action` — the behavior-preserving port of
//! the old `main.rs::worker_decide_action` — against a FIXED `&[StepPlan]` so a
//! future change that alters the musical output FAILS here.
//!
//! Why a fixed plan and NOT the engine's set_features_global path: the harmony
//! derivation routes through `pick_progression`, which uses `thread_rng`
//! (chord_engine.rs:58) and is therefore non-deterministic across runs (S9 §6
//! risk 1). The equivalence anchor isolates the MOVED KERNEL from that RNG by
//! constructing the plan by hand.
//!
//! ALGORITHMIC PROPERTIES THIS FILE PINS (any change to these fails the net):
//!   P1. channel == inst_idx % 16  (MIDI channel assignment, main.rs:131/247).
//!   P2. empty plan → ZERO events (a silent step), never a panic.
//!   P3. the step consulted is plan[step_idx % plan.len()] — step_idx beyond the
//!       plan length WRAPS via modulo (so the cadence at index 1 is reached by
//!       step 1, 3, 5, … and the start at index 0 by step 0, 2, 4, …).
//!   P4. PerfFeatures is a plain field copy of (avg_saturation, avg_brightness,
//!       edge_density) — saturation drives the velocity LEVEL.
//!   P5. role-derived pitch: instrument 0 = BASS (chord root, low register),
//!       instrument num-1 = MELODY (top chord tone, high register) — bass < melody.
//!   P6. a cadence step is EXEMPT from the velocity contour: its velocity is the
//!       structural floor + saturation level-gain only (no swell/accent/taper),
//!       and it sounds as a SINGLE ritardando-lengthened sustained note.
//!   P7. num_instruments = 1 collapses to a single MELODY voice; a larger count
//!       stratifies bass / fill / melody — both produce valid per-instrument channels.
//!
//! The concrete GOLDEN constants below (G_BASS_NOTE, G_MELODY_NOTE, the cadence
//! velocities and holds) are derived by hand from the documented realizer
//! algorithm (chord_engine.rs realize_step / role_pitch / realize_velocity /
//! realize_rhythm). If the realizer changes, these break ON PURPOSE — that is the
//! regression alarm. See the per-constant derivation comments.

use audiohax::chord_engine::{Chord, PhrasePosition, StepPlan};
use audiohax::engine::{
    decide_instrument_action, CadenceStrength, KeyTempoPlan, ScanBarFeatures, Section, StepContext,
    ThematicRole, ThemeVariation,
};

// ─────────────────────────────────────────────────────────────────────────────
// Fixtures — a FIXED plan (no pick_progression / thread_rng anywhere).
// ─────────────────────────────────────────────────────────────────────────────

/// The pinned chord for the whole net: a C-major triad in root position.
fn c_major() -> Chord {
    Chord {
        name: "I".to_string(),
        notes: vec![60, 64, 67], // C4 E4 G4 → pcs 0,4,7
    }
}

/// A fixed 2-step plan: index 0 = PhraseStart, index 1 = a PERFECT AUTHENTIC
/// CADENCE. Built by hand so decide_instrument_action's output is fully pinnable.
/// The cadence floor velocity is the documented cadence weight (96).
fn fixed_plan() -> Vec<StepPlan> {
    vec![
        StepPlan {
            chord: c_major(),
            phrase_index: 0,
            position_in_phrase: 0,
            phrase_len: 4,
            position: PhrasePosition::PhraseStart,
            velocity: 80,
        },
        StepPlan {
            chord: c_major(),
            phrase_index: 0,
            position_in_phrase: 3,
            phrase_len: 4,
            position: PhrasePosition::PerfectAuthenticCadence,
            velocity: 96,
        },
    ]
}

/// S15: the behaviour-neutral default Section + KeyTempoPlan the net borrows into the new
/// 7th `ctx` arg. `theme:None` + `key_offset:0` ⇒ the realizer takes its EXISTING free-select
/// path ⇒ the goldens (240, 114/84, 36/79) do NOT move. The test owns these lifetimes exactly
/// as it owns `fixed_plan()`. NO assert in this file is relaxed — only an argument is added.
fn default_section(plan: &[StepPlan]) -> Section {
    Section {
        label: "A".to_string(),
        step_len: plan.len(),
        thematic_role: ThematicRole::Statement,
        key_offset_semitones: 0,
        ms_per_step: MS_PER_STEP,
        mode: "Ionian".to_string(),
        progression: vec![],
        theme: None,
        variation: ThemeVariation::Identity,
        boundary_cadence: CadenceStrength::Perfect,
        density: 0.5,
        steps: plan.to_vec(),
    }
}

fn default_key_tempo() -> KeyTempoPlan {
    KeyTempoPlan {
        home_root_midi: 60,
        home_mode: "Ionian".to_string(),
        base_ms_per_step: MS_PER_STEP,
        key_scheme: vec![0],
        tempo_scheme: vec![MS_PER_STEP],
    }
}

fn bar(sat: f32, bright: f32, edge: f32) -> ScanBarFeatures {
    ScanBarFeatures {
        bar_index: 0,
        avg_hue: 0.0,
        avg_saturation: sat,
        avg_brightness: bright,
        edge_density: edge,
        texture_laplacian_var: 0.0,
        hue_hist: Vec::new(),
    }
}

const MS_PER_STEP: u64 = 200;

// Derived golden pitch constants (see header P5/P6).
// BASS (inst 0, num=2): role=Bass → chord ROOT pc = 60 % 12 = 0; bright=55 ≥ 50 so
//   no dark drop; floor = BASS_REGISTER_FLOOR (36); seat_pc_in_register(0, 36):
//   (36/12)*12 + 0 = 36, 36 ≥ 36 ⇒ note 36 (C2).
const G_BASS_NOTE: u8 = 36;
// MELODY (inst 1, num=2): role=Melody → top chord tone = 67, pc = 7;
//   bright_octaves = (55-50)/50 = 0.1; lift = round(0.1*12) = 1; floor =
//   (MELODY_REGISTER_FLOOR 67 + 1).clamp(24,96) = 68; seat_pc_in_register(7, 68):
//   (68/12)*12 + 7 = 67, 67 < 68 ⇒ +12 = 79 (G4-area, G5 = 79).
const G_MELODY_NOTE: u8 = 79;

// ─────────────────────────────────────────────────────────────────────────────
// P1 / P2 / P3 — channel, empty-plan, modulo wrap
// ─────────────────────────────────────────────────────────────────────────────

/// P1: channel is inst_idx % 16, wrapping past 16.
#[test]
fn test_channel_is_inst_idx_mod_16() {
    let plan = fixed_plan();
    let kt = default_key_tempo();
    let sec = default_section(&plan);
    let ctx = StepContext::single_section_default(&sec, &kt);
    let f = bar(60.0, 50.0, 0.2);
    for inst in [0usize, 1, 15, 16, 17, 33] {
        let d = decide_instrument_action(&f, inst, 0, 4, &plan, MS_PER_STEP, &ctx);
        assert_eq!(
            d.channel as usize,
            inst % 16,
            "channel must be inst_idx % 16 for inst {inst}"
        );
    }
}

/// P2: an empty plan produces a SILENT decision (no events), never a panic.
#[test]
fn test_empty_plan_is_silent() {
    let kt = default_key_tempo();
    let sec = default_section(&[]);
    let ctx = StepContext::single_section_default(&sec, &kt);
    let f = bar(70.0, 60.0, 0.4);
    for inst in 0..4usize {
        for step in 0..6usize {
            let d = decide_instrument_action(&f, inst, step, 4, &[], MS_PER_STEP, &ctx);
            assert_eq!(d.channel as usize, inst % 16);
            assert!(
                d.events.is_empty(),
                "empty plan must yield no events (inst {inst}, step {step})"
            );
        }
    }
}

/// P3: step_idx beyond plan length wraps via modulo — even step indices hit the
/// PhraseStart (index 0), odd hit the cadence (index 1). We detect which plan step
/// was consulted by the CADENCE signature (cadence → single sustained note, P6).
#[test]
fn test_step_idx_wraps_via_modulo() {
    let plan = fixed_plan(); // len 2: [PhraseStart, Cadence]
    let f = bar(80.0, 55.0, 0.9); // high edge: a NON-cadence melody would arpeggiate (≥3 onsets)
                                  // Melody instrument so the non-cadence vs cadence onset count differs sharply.
    let num = 2;
    let melody = 1;
    let kt = default_key_tempo();
    let sec = default_section(&plan);
    let ctx = StepContext::single_section_default(&sec, &kt);

    // Even steps → PhraseStart (index 0), a high-edge melody → ARPEGGIO (3 onsets).
    for even in [0usize, 2, 4] {
        let d = decide_instrument_action(&f, melody, even, num, &plan, MS_PER_STEP, &ctx);
        assert_eq!(
            d.events.len(),
            3,
            "even step {even} must consult PhraseStart → arpeggio (3 onsets)"
        );
    }
    // Odd steps → Cadence (index 1) → single sustained note (1 onset), edge ignored.
    for odd in [1usize, 3, 5] {
        let d = decide_instrument_action(&f, melody, odd, num, &plan, MS_PER_STEP, &ctx);
        assert_eq!(
            d.events.len(),
            1,
            "odd step {odd} must consult the Cadence → single sustained note"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// P5 / P6 — role-derived pitch + the cadence golden (the load-bearing pins)
// ─────────────────────────────────────────────────────────────────────────────

/// P5: bass plays the chord ROOT low, melody the TOP tone high — bass < melody.
/// Pinned to the exact golden pitches derived from the realizer (G_BASS/G_MELODY).
#[test]
fn test_role_pitch_bass_below_melody_golden() {
    let plan = fixed_plan();
    let f = bar(60.0, 55.0, 0.2); // bright=55 ⇒ the lift used in the golden derivation
    let num = 2;
    let kt = default_key_tempo();
    let sec = default_section(&plan);
    let ctx = StepContext::single_section_default(&sec, &kt);

    // Use the cadence step (index 1 → step_idx 1) so the rhythm is a single note —
    // exactly one event per instrument, the pitch is unambiguous.
    let bass = decide_instrument_action(&f, 0, 1, num, &plan, MS_PER_STEP, &ctx);
    let melody = decide_instrument_action(&f, num - 1, 1, num, &plan, MS_PER_STEP, &ctx);

    assert_eq!(bass.events.len(), 1, "cadence ⇒ one bass note");
    assert_eq!(melody.events.len(), 1, "cadence ⇒ one melody note");

    assert_eq!(
        bass.events[0].note, G_BASS_NOTE,
        "bass golden pitch drifted (expected chord-root in bass register)"
    );
    assert_eq!(
        melody.events[0].note, G_MELODY_NOTE,
        "melody golden pitch drifted (expected top chord-tone in melody register)"
    );
    assert!(
        bass.events[0].note < melody.events[0].note,
        "bass must sound below melody"
    );
}

/// P6: a cadence step's velocity is the structural floor + saturation level-gain
/// ONLY (no swell/accent/taper), and the note is a single ritardando-lengthened
/// sustained event. Two saturation extremes pin the level-gain formula exactly.
///
/// Derivation (chord_engine realize_velocity, is_cadence branch):
///   level_gain = -12 + (sat/100)*30 ; vel = round(floor + level_gain).clamp(1,127).
///   role melody/bass adjustments are gated `if !is_cadence` ⇒ NOT applied here.
///   floor (cadence step.velocity) = 96.
///   sat=100 ⇒ gain +18 ⇒ vel = round(96+18) = 114.
///   sat=0   ⇒ gain -12 ⇒ vel = round(96-12) = 84.
/// Hold (realize_rhythm is_cadence branch): sustained(0, step_ms, LEGATO_FRAC)
///   with rit=1.30 ⇒ f = (0.95*1.30).min(1.20) = 1.20 ⇒ hold = round(200*1.20) = 240.
#[test]
fn test_cadence_velocity_and_hold_golden() {
    let plan = fixed_plan();
    let num = 2;
    let kt = default_key_tempo();
    let sec = default_section(&plan);
    let ctx = StepContext::single_section_default(&sec, &kt);

    // Saturation 100 on the bass: cadence-exempt velocity = round(96 + 18) = 114.
    let hot = bar(100.0, 55.0, 0.5);
    let d_hot = decide_instrument_action(&hot, 0, 1, num, &plan, MS_PER_STEP, &ctx);
    assert_eq!(d_hot.events.len(), 1, "cadence ⇒ single sustained note");
    assert_eq!(
        d_hot.events[0].velocity, 114,
        "cadence velocity must be floor 96 + sat100 gain +18 = 114 (no contour)"
    );
    assert_eq!(
        d_hot.events[0].hold_ms, 240,
        "cadence hold must be the ritardando-lengthened 1.20*200 = 240 ms"
    );
    assert_eq!(
        d_hot.events[0].offset_ms, 0,
        "cadence note onsets at step start"
    );

    // Saturation 0 on the bass: cadence-exempt velocity = round(96 - 12) = 84.
    let cold = bar(0.0, 55.0, 0.5);
    let d_cold = decide_instrument_action(&cold, 0, 1, num, &plan, MS_PER_STEP, &ctx);
    assert_eq!(
        d_cold.events[0].velocity, 84,
        "cadence velocity must drop to floor 96 + sat0 gain -12 = 84"
    );
}

/// P4: saturation drives the velocity LEVEL — higher saturation ⇒ louder, lower ⇒
/// softer, monotonically, at the cadence step where the mapping is unobscured by
/// the phrase contour.
#[test]
fn test_saturation_drives_velocity_level_monotonic() {
    let plan = fixed_plan();
    let num = 2;
    let kt = default_key_tempo();
    let sec = default_section(&plan);
    let ctx = StepContext::single_section_default(&sec, &kt);
    let v = |sat: f32| {
        decide_instrument_action(&bar(sat, 55.0, 0.5), 0, 1, num, &plan, MS_PER_STEP, &ctx).events
            [0]
        .velocity
    };
    let lo = v(10.0);
    let mid = v(50.0);
    let hi = v(95.0);
    assert!(
        lo < mid && mid < hi,
        "velocity must rise with saturation: {lo} < {mid} < {hi}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// P7 — ensemble-width edge cases
// ─────────────────────────────────────────────────────────────────────────────

/// P7: num_instruments = 1 collapses to a single MELODY voice that sounds the top
/// chord tone (NOT the bass root) — the lone line is the tune.
#[test]
fn test_single_instrument_is_melody_not_bass() {
    let plan = fixed_plan();
    let kt = default_key_tempo();
    let sec = default_section(&plan);
    let ctx = StepContext::single_section_default(&sec, &kt);
    let f = bar(60.0, 55.0, 0.2);
    // Cadence step ⇒ one note; inst 0 of 1 is the MELODY ⇒ top-tone pitch (G_MELODY),
    // distinctly NOT the bass-register root (G_BASS).
    let d = decide_instrument_action(&f, 0, 1, 1, &plan, MS_PER_STEP, &ctx);
    assert_eq!(d.channel, 0);
    assert_eq!(d.events.len(), 1);
    assert_eq!(
        d.events[0].note, G_MELODY_NOTE,
        "a single instrument is the melody (top tone), not the bass"
    );
}

/// P7: a larger ensemble stratifies bass(0) / fill(inner) / melody(last) and every
/// instrument gets a valid MIDI channel; bass sits below the melody.
#[test]
fn test_larger_ensemble_stratifies_and_channels_valid() {
    let plan = fixed_plan();
    let kt = default_key_tempo();
    let sec = default_section(&plan);
    let ctx = StepContext::single_section_default(&sec, &kt);
    let f = bar(60.0, 55.0, 0.2);
    let num = 4;
    let mut notes = Vec::new();
    for inst in 0..num {
        let d = decide_instrument_action(&f, inst, 1, num, &plan, MS_PER_STEP, &ctx);
        assert_eq!(
            d.channel as usize,
            inst % 16,
            "valid channel for inst {inst}"
        );
        assert_eq!(d.events.len(), 1, "cadence ⇒ one note per inst");
        notes.push(d.events[0].note);
    }
    // Bass (inst 0) below melody (inst num-1).
    assert!(
        notes[0] < notes[num - 1],
        "bass {} must be below melody {}",
        notes[0],
        notes[num - 1]
    );
    // All pitches inside the realizer's documented band.
    for n in &notes {
        assert!((24..=108).contains(n), "pitch {n} out of band 24..=108");
    }
}

/// Full determinism of the golden: the entire fixed-plan sweep is byte-identical
/// across two runs (the regression net would be worthless if the kernel weren't
/// pure). Compares whole InstrumentDecision Vecs via the Eq impl.
#[test]
fn test_full_golden_sweep_is_byte_identical() {
    let plan = fixed_plan();
    let kt = default_key_tempo();
    let sec = default_section(&plan);
    let ctx = StepContext::single_section_default(&sec, &kt);
    let f = bar(73.0, 41.0, 0.6);
    let sweep = || {
        let mut v = Vec::new();
        for inst in 0..4usize {
            for step in 0..4usize {
                v.push(decide_instrument_action(
                    &f,
                    inst,
                    step,
                    4,
                    &plan,
                    MS_PER_STEP,
                    &ctx,
                ));
            }
        }
        v
    };
    assert_eq!(
        sweep(),
        sweep(),
        "the full decision sweep must be byte-identical run to run"
    );
}

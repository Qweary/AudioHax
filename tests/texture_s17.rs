//! tests/texture_s17.rs — the SLICE-1 TEXTURE PROPERTY NET (sustained harmonic Pad
//! + bass bed). This file LOCKS the claims of the Slice-1 build: the held Pad bed,
//! the HarmonicFill rest-bug fix, the plan-aware role assignment, the CounterMelody
//! stub, and — load-bearing — the proof that the DEFAULT (identity-profile) path is
//! byte-untouched, so `engine_equivalence.rs` stays byte-green.
//!
//! HEADLESS, in the same sense as engine_equivalence.rs / composition_s15.rs: it
//! touches NO image type, NO OpenCV, NO audio hardware. It exercises only the pure
//! realizer (`chord_engine::realize_step` / `assign_role` / `instrument_role`) over
//! HAND-BUILT, RNG-free fixtures. NO value here routes through `pick_progression` /
//! `thread_rng` — every pinned number is derived by hand from the realizer algorithm.
//! (Run under DEFAULT features: the integration harness builds the feature-gated bin,
//! so `--no-default-features` cannot be used to RUN it — see the file footer.)
//!
//! WHAT EACH PROPERTY LOCKS (reconciled with the build spec §11):
//!   1. assign_role is the byte-freeze witness — under the IDENTITY profile,
//!      `assign_role(inst,num,ctx) == instrument_role(inst,num)` for the whole
//!      realistic (inst,num) range. This is the proof the default path is untouched.
//!   2. A full (`pad_bed`) profile step realizes STRICTLY MORE simultaneous
//!      NoteEvents than the identity profile on the same inputs — density rises.
//!   3. The Pad arm emits exactly `min(pad_voices, inner.len())` notes, all at
//!      offset 0 (simultaneous), all chord-tone pitch classes, each held within the
//!      ≤1.2× step cap (the scheduler-safety floor) and ≥ the step length (legato).
//!   4. Inner voices are NON-SILENT on a low-but-realistic-edge image — the
//!      rest-bug-fix regression guard — paired with a genuinely near-static case
//!      that DOES still rest, pinning the FILL_REST_ACTIVITY floor.
//!   5. The Pad bed ties step-to-step: consecutive Pad steps' hold_ms ≥ ms_per_step
//!      (overlap, no gap) AND ≤ 1.2× ms_per_step (no cross-step N× hold).
//!   6. assign_role non-identity mapping — `pad_bed` maps inst 0→Bass, 1→Pad,
//!      2→HarmonicFill, 3→Melody, with the over-count clamp onto the last layer.
//!   7. The CounterMelody stub is byte-equal to the (rest-fixed) HarmonicFill figure
//!      on the same step — pinning the stub so a later counter-line is a clean diff.

use audiohax::chord_engine::{
    assign_role, instrument_role, realize_step, Chord, NoteEvent, OrchestralRole, PerfFeatures,
    PhrasePosition, StepPlan,
};
use audiohax::composition::{
    CadenceStrength, KeyTempoPlan, LayerRole, OrchestrationProfile, Section, StepContext,
    ThematicRole, ThemeVariation,
};

// ─────────────────────────────────────────────────────────────────────────────
// Fixtures — all hand-built, RNG-free. No planner / pick_progression / thread_rng.
// ─────────────────────────────────────────────────────────────────────────────

const MS_PER_STEP: u64 = 200;

/// The pinned chord: a C-major triad in root position (pcs 0,4,7).
fn c_major() -> Chord {
    Chord {
        name: "I".to_string(),
        notes: vec![60, 64, 67], // C4 E4 G4
    }
}

/// A NON-cadence interior step. `position_in_phrase` is caller-chosen so a test can
/// land on a weak (odd) interior beat (where rest-as-gesture is allowed) or a strong
/// (even) one. Never a cadence — so the Pad / fill / counter arms are actually reached.
fn interior_step(position_in_phrase: usize) -> StepPlan {
    StepPlan {
        chord: c_major(),
        phrase_index: 0,
        position_in_phrase,
        phrase_len: 8,
        position: PhrasePosition::Interior,
        velocity: 80,
    }
}

/// PerfFeatures with a chosen edge_density (the rhythm/rest driver). Saturation and
/// brightness are mid so velocity/register stay in band but are irrelevant to the
/// onset-count / hold / rest properties this net pins.
fn perf(edge_density: f32) -> PerfFeatures {
    PerfFeatures {
        saturation: 60.0,
        brightness: 55.0,
        edge_density,
    }
}

fn key_tempo() -> KeyTempoPlan {
    KeyTempoPlan {
        home_root_midi: 60,
        home_mode: "Ionian".to_string(),
        base_ms_per_step: MS_PER_STEP,
        key_scheme: vec![0],
        tempo_scheme: vec![MS_PER_STEP],
    }
}

/// Build a Section carrying an arbitrary orchestration profile around a one-step plan.
fn section_with(profile: OrchestrationProfile, step: &StepPlan) -> Section {
    Section {
        label: "A".to_string(),
        step_len: 1,
        thematic_role: ThematicRole::Statement,
        key_offset_semitones: 0,
        ms_per_step: MS_PER_STEP,
        mode: "Ionian".to_string(),
        progression: vec![],
        theme: None,
        variation: ThemeVariation::Identity,
        boundary_cadence: CadenceStrength::Perfect,
        density: 0.5,
        orchestration: profile,
        steps: vec![step.clone()],
    }
}

/// The shipped Slice-1 `pad_bed` profile, built by hand (NOT loaded from mappings.json,
/// so this net never touches the loader): inst 0→Bass, 1→Pad, 2→HarmonicFill, 3→Melody,
/// 3 held pad voices.
fn pad_bed() -> OrchestrationProfile {
    OrchestrationProfile {
        id: "pad_bed".to_string(),
        layers: vec![
            LayerRole::Bass,
            LayerRole::Pad,
            LayerRole::HarmonicFill,
            LayerRole::Melody,
        ],
        density: 0.55,
        pad_voices: 3,
    }
}

/// Realize one instrument on one step under a given profile, with the borrowed-context
/// plumbing the realizer needs. Returns the instrument's NoteEvents for that step.
fn realize_under(
    profile: OrchestrationProfile,
    step: &StepPlan,
    inst_idx: usize,
    num_instruments: usize,
    features: &PerfFeatures,
) -> Vec<NoteEvent> {
    let kt = key_tempo();
    let sec = section_with(profile, step);
    let ctx = StepContext::single_section_default(&sec, &kt);
    realize_step(step, inst_idx, num_instruments, features, MS_PER_STEP, &ctx)
}

// ─────────────────────────────────────────────────────────────────────────────
// 1. assign_role is the BYTE-FREEZE WITNESS (the load-bearing one)
// ─────────────────────────────────────────────────────────────────────────────

/// §11.2: under the IDENTITY profile, `assign_role` is byte-identical to the legacy
/// `instrument_role` for EVERY (inst, num) in the realistic ensemble range. This is
/// the proof the default realize path is untouched — the freeze witness for the new
/// assigner that lets engine_equivalence.rs stay byte-green.
#[test]
fn test_assign_role_identity_equals_instrument_role() {
    let step = interior_step(1);
    let kt = key_tempo();
    let sec = section_with(OrchestrationProfile::identity(), &step);
    let ctx = StepContext::single_section_default(&sec, &kt);
    assert!(
        ctx.section.orchestration.is_identity(),
        "the default profile must report identity (the gate assign_role keys on)"
    );

    for num in 1..=8usize {
        for inst in 0..num {
            assert_eq!(
                assign_role(inst, num, &ctx),
                instrument_role(inst, num),
                "identity-profile assign_role must equal instrument_role for inst {inst} of {num}"
            );
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 6. assign_role NON-IDENTITY mapping + the over-count clamp
// ─────────────────────────────────────────────────────────────────────────────

/// §11.6: a `pad_bed`-profile ctx maps instruments onto the shipped layer order —
/// inst 0→Bass, 1→Pad, 2→HarmonicFill, 3→Melody — and an instrument PAST the layer
/// list clamps onto the LAST named layer (Melody), never wrapping back onto Bass.
#[test]
fn test_assign_role_pad_bed_mapping_and_clamp() {
    let step = interior_step(1);
    let kt = key_tempo();
    let sec = section_with(pad_bed(), &step);
    let ctx = StepContext::single_section_default(&sec, &kt);

    assert_eq!(
        assign_role(0, 4, &ctx),
        OrchestralRole::Bass,
        "inst 0 → Bass"
    );
    assert_eq!(assign_role(1, 4, &ctx), OrchestralRole::Pad, "inst 1 → Pad");
    assert_eq!(
        assign_role(2, 4, &ctx),
        OrchestralRole::HarmonicFill,
        "inst 2 → HarmonicFill"
    );
    assert_eq!(
        assign_role(3, 4, &ctx),
        OrchestralRole::Melody,
        "inst 3 → Melody"
    );

    // Over-count: inst 4 and 5 (past the 4-layer list) clamp onto the LAST layer (Melody).
    assert_eq!(
        assign_role(4, 6, &ctx),
        OrchestralRole::Melody,
        "over-count inst 4 clamps onto the last layer (Melody), not wraps to Bass"
    );
    assert_eq!(
        assign_role(5, 6, &ctx),
        OrchestralRole::Melody,
        "over-count inst 5 clamps onto the last layer (Melody)"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. The PAD ARM — exact voice count, simultaneity, chord-tone membership, the cap
// ─────────────────────────────────────────────────────────────────────────────

/// §11.3: the Pad instrument (inst 1 under `pad_bed`) on a NON-cadence step realizes
/// EXACTLY `min(pad_voices, inner.len())` simultaneous NoteEvents — all at offset 0,
/// all chord-tone pitch classes seated into the inner register, each held ≤ 1.2×
/// ms_per_step (the §6.4 seam-safety cap so the N× scheduler hazard can't regress).
#[test]
fn test_pad_arm_held_bed_contract() {
    let step = interior_step(2); // strong interior beat (even) — no rest path involved
    let feats = perf(0.04); // any realistic edge; the Pad arm does not gate on rest
    let events = realize_under(pad_bed(), &step, 1, 4, &feats);

    // pad_voices = 3; the Pad seats the INNER tones (notes[1..] = [64,67]) → inner.len()=2,
    // so the held bed is min(3,2) = 2 voices (root-less 3rd+5th of the C-major triad).
    assert_eq!(
        events.len(),
        2,
        "Pad emits min(pad_voices=3, inner_tones=2) = 2 simultaneous bed voices"
    );

    // All at offset 0 → genuinely simultaneous (a held bed, not an arpeggio).
    assert!(
        events.iter().all(|e| e.offset_ms == 0),
        "every Pad bed voice onsets at offset 0 (simultaneous)"
    );

    // Every bed voice is a CHORD-TONE pitch class (seating moves the octave, so assert
    // on pitch class, not the literal note value).
    let chord_pcs: Vec<u8> = c_major().notes.iter().map(|n| n % 12).collect();
    for e in &events {
        assert!(
            chord_pcs.contains(&(e.note % 12)),
            "Pad bed voice {} (pc {}) must be a chord tone (chord pcs {:?})",
            e.note,
            e.note % 12,
            chord_pcs
        );
    }

    // No two bed voices collapse onto the same pitch (a bed of unisons is not a bed).
    let mut pitches: Vec<u8> = events.iter().map(|e| e.note).collect();
    pitches.sort_unstable();
    pitches.dedup();
    assert_eq!(
        pitches.len(),
        events.len(),
        "Pad bed voices must be distinct pitches (de-duplicated)"
    );

    // Each held within the seam-safety cap: ≥ the step length (legato bed, no gap) and
    // ≤ 1.2× ms_per_step (PAD_OVERLAP_FRAC = 1.10 → 220 ms; the cap is 1.2× = 240).
    for e in &events {
        assert!(
            e.hold_ms >= MS_PER_STEP,
            "Pad voice hold_ms {} must be ≥ ms_per_step {} (legato bed)",
            e.hold_ms,
            MS_PER_STEP
        );
        assert!(
            e.hold_ms <= (MS_PER_STEP as f32 * 1.2).round() as u64,
            "Pad voice hold_ms {} must be ≤ 1.2× ms_per_step {} (seam-safety cap, no N× hold)",
            e.hold_ms,
            (MS_PER_STEP as f32 * 1.2).round() as u64
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 5. The PAD BED TIES step-to-step (legato overlap across consecutive Pad steps)
// ─────────────────────────────────────────────────────────────────────────────

/// §11.5: consecutive Pad steps overlap — each step's bed holds for ≥ ms_per_step
/// (so the next step's onset lands while this one still sounds, no audible gap) and
/// ≤ 1.2× ms_per_step (so the bed never runs the block-until-last-event scheduler
/// into the N× catastrophe). Pin it across two consecutive interior steps.
#[test]
fn test_pad_bed_ties_step_to_step() {
    let feats = perf(0.04);
    let cap = (MS_PER_STEP as f32 * 1.2).round() as u64;

    for pos in [1usize, 2, 3] {
        let step = interior_step(pos);
        let events = realize_under(pad_bed(), &step, 1, 4, &feats);
        assert!(!events.is_empty(), "Pad step {pos} must sound a bed");
        for e in &events {
            assert!(
                e.hold_ms >= MS_PER_STEP && e.hold_ms <= cap,
                "Pad step {pos}: hold_ms {} must tie (≥{MS_PER_STEP}) yet stay ≤{cap}",
                e.hold_ms
            );
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. Density actually RISES — full profile > identity in simultaneous notes
// ─────────────────────────────────────────────────────────────────────────────

/// §11.5/§11.2(table): the SAME instrument slot on the SAME step realizes STRICTLY
/// MORE simultaneous NoteEvents under the `pad_bed` profile (where it is a Pad bed)
/// than under the identity profile (where it is a single inner HarmonicFill tone) —
/// the Pad genuinely ADDS the held bed, density rises.
#[test]
fn test_pad_bed_adds_density_vs_identity() {
    let step = interior_step(2); // strong beat: identity HarmonicFill sounds (no rest)
    let feats = perf(0.04);

    // inst 1 of 4: identity → HarmonicFill (one inner tone); pad_bed → Pad (the held bed).
    let identity_events = realize_under(OrchestrationProfile::identity(), &step, 1, 4, &feats);
    let pad_events = realize_under(pad_bed(), &step, 1, 4, &feats);

    assert_eq!(
        identity_events.len(),
        1,
        "identity HarmonicFill sounds a single inner tone on a strong interior beat"
    );
    assert!(
        pad_events.len() > identity_events.len(),
        "the pad_bed profile must add density: {} pad voices > {} identity-fill events",
        pad_events.len(),
        identity_events.len()
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 4. The HARMONICFILL REST-BUG FIX — inner voices now SOUND on a real photo
// ─────────────────────────────────────────────────────────────────────────────

/// §11.4: the rest-bug fix gates rest-as-gesture on the NORMALIZED edge_activity
/// (edge_density / 0.05, clamped 0..1) against FILL_REST_ACTIVITY = 0.10, NOT the
/// raw per-bar edge. On a weak (odd) interior beat:
///   * a low-but-realistic photo (edge_density 0.004 → activity 0.08, ABOVE 0.10? no
///     — 0.08 < 0.10, so use 0.006 → activity 0.12, a realistic photo ABOVE the floor)
///     NO LONGER rests — the inner voice SOUNDS (the regression the fix repairs);
///   * a genuinely near-static texture (edge_density 0.001 → activity 0.02, BELOW the
///     floor) STILL rests — pinning the FILL_REST_ACTIVITY floor so it isn't a no-op.
/// Both at the identity profile, inst 1 of 3 (a HarmonicFill instrument).
#[test]
fn test_harmonicfill_rest_bug_fixed() {
    // edge_density 0.006 → activity = 0.006/0.05 = 0.12 ≥ FILL_REST_ACTIVITY (0.10):
    // a realistic photo's inner voice now SOUNDS on a weak interior beat (was silenced
    // by the old raw `edge < 0.15` guard, which fired on essentially every real photo).
    let sounding = realize_under(
        OrchestrationProfile::identity(),
        &interior_step(1),
        1,
        3,
        &perf(0.006),
    );
    assert_eq!(
        sounding.len(),
        1,
        "rest-bug FIX: a weak-beat inner voice at realistic activity 0.12 must SOUND, not rest"
    );

    // edge_density 0.001 → activity = 0.02 < FILL_REST_ACTIVITY (0.10): a genuinely
    // near-static texture STILL rests — the floor is real, not a no-op.
    let resting = realize_under(
        OrchestrationProfile::identity(),
        &interior_step(1),
        1,
        3,
        &perf(0.001),
    );
    assert!(
        resting.is_empty(),
        "near-static activity 0.02 must STILL rest-as-gesture on a weak interior beat (pins the floor)"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 7. The COUNTERMELODY STUB is byte-equal to the (rest-fixed) HarmonicFill figure
// ─────────────────────────────────────────────────────────────────────────────

/// §11.6: the CounterMelody realize branch is a STUB that delegates to the rest-fixed
/// HarmonicFill figure. Pin it: a step that assigns CounterMelody realizes the EXACT
/// same NoteEvents as the same step under HarmonicFill — so a later real counter-line
/// is a clean, isolated diff. Tested on both a SOUNDING (strong) and a RESTING (weak,
/// near-static) beat so the delegation is pinned across both fill outcomes.
#[test]
fn test_countermelody_stub_equals_harmonicfill() {
    // A profile that assigns inst 0 → CounterMelody (vs. an identity inst 1 of 3 → fill).
    // We compare the realized events of a CounterMelody instrument against a HarmonicFill
    // instrument on the SAME step + features, so the only difference is the role arm.
    let counter_profile = OrchestrationProfile {
        id: "counter_probe".to_string(),
        layers: vec![LayerRole::CounterMelody],
        density: 0.5,
        pad_voices: 0,
    };

    for (pos, feats) in [
        (2usize, perf(0.04)),  // strong beat → both arms SOUND
        (1usize, perf(0.001)), // weak near-static beat → both arms REST
    ] {
        let step = interior_step(pos);
        // CounterMelody: inst 0 under the single-layer counter profile.
        let counter = realize_under(counter_profile.clone(), &step, 0, 3, &feats);
        // HarmonicFill: inst 1 of 3 under identity is a HarmonicFill instrument.
        let fill = realize_under(OrchestrationProfile::identity(), &step, 1, 3, &feats);

        // Compare the rhythm/articulation SHAPE (onset offsets + holds) — the pitch may
        // differ (different inst index → different inner-tone seat), but the stub must
        // produce the SAME figure shape as the HarmonicFill arm it delegates to.
        let shape = |evs: &[NoteEvent]| -> Vec<(u64, u64)> {
            evs.iter().map(|e| (e.offset_ms, e.hold_ms)).collect()
        };
        assert_eq!(
            shape(&counter),
            shape(&fill),
            "CounterMelody stub must produce the same figure shape as HarmonicFill (pos {pos})"
        );
    }
}

// Run under DEFAULT features (the integration harness builds the feature-gated bin, so
// `--no-default-features` cannot RUN this net):
//   cargo test --test texture_s17

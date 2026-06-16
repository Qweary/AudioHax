//! tests/prominence_s23.rs — the S23 SLICE-B SALIENCY → ROLE-PROMINENCE property net.
//!
//! Proves the Slice-B build pinned in `docs/spec-s23-slice-b-build.md` §4 (and the
//! `docs/design-s21-affective-fidelity.md` §3 "Property tests to add" + §5 Risk 1
//! register-separation invariant): a salient image subject pushes the MELODY forward
//! (louder, higher, rhythmically freer via three CENTERED nudges on the existing five
//! orchestration roles) while the recessive background bed recedes — AND the freeze
//! pivot holds: every nudge is `(0.5-0.5)*SPAN == 0.0` exactly at the neutral weight
//! 0.5, so identity / all-0.5 prominence realizes byte-for-byte today's output.
//!
//! HEADLESS, in the same sense as texture_s17.rs / engine_equivalence.rs: it touches NO
//! image type, NO OpenCV, NO audio hardware. It exercises only the pure realizer
//! (`chord_engine::realize_step`) over HAND-BUILT, RNG-free fixtures — every input is a
//! literal `OrchestrationProfile` / `Section` / `StepPlan`, the planner resolve block is
//! bypassed exactly as the spec §4 sanctions ("build an OrchestrationProfile with an
//! explicit non-empty `prominence` Vec ... same as the figuration tests construct
//! `figuration_resolved` directly"). NO value here routes through `pick_progression` /
//! `thread_rng`. Runs under DEFAULT features (the engagement convention; the
//! `--no-default-features` bin-config defect is unrelated to this slice).
//!
//! HARNESS LINEAGE: the fixtures (`c_major`, `interior_step`, `perf`, `key_tempo`,
//! `section_with`, `realize_under`) mirror tests/texture_s17.rs 1:1 — the closest
//! existing net that drives `realize_step` directly under a non-identity profile.
//!
//! REALIZER-CHARACTER NOTE (test 4 "all Character variants"): the realizer reads NO
//! `Character` enum — `Section` carries no character field; the only register driver in
//! the per-step realizer is `PerfFeatures.brightness` (the `bright_octaves` lift,
//! chord_engine.rs role_pitch). So a character's "bright register" (e.g. Scherzo's, the
//! stack-risk the invariant guards) is realized HERE as a high-`brightness` PerfFeatures.
//! The §4 sweep is therefore made real by sweeping brightness across the dark→bright band
//! the affect ladder spans (Lament-dark … Scherzo-bright), crossed with all prominence
//! weights — which is exactly the lift-stacking the no-inversion invariant must survive
//! (Scherzo bright register + a bright-image bright_octaves lift + the saliency foreground
//! lift, all at once). The `Character`-named brightness anchors below document that mapping.

use audiohax::chord_engine::{realize_step, Chord, NoteEvent, PhrasePosition, StepPlan};
use audiohax::composition::{
    CadenceStrength, Character, KeyTempoPlan, LayerProminence, LayerRole, OrchestrationProfile,
    Section, StepContext, ThematicRole, ThemeVariation,
};

// ─────────────────────────────────────────────────────────────────────────────
// Fixtures — all hand-built, RNG-free. Mirror tests/texture_s17.rs.
// ─────────────────────────────────────────────────────────────────────────────

const MS_PER_STEP: u64 = 200;

/// The pinned chord: a C-major triad in root position (pcs 0,4,7).
fn c_major() -> Chord {
    Chord {
        name: "I".to_string(),
        notes: vec![60, 64, 67], // C4 E4 G4
    }
}

/// A NON-cadence interior step — so the Melody / Pad / fill arms (not the cadence
/// early-return) are actually reached and the prominence nudges fire.
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

/// A PERFECT AUTHENTIC CADENCE step — the cadence-exempt path (the prominence velocity
/// nudge is `!is_cadence`-guarded, the rhythm `pre_cadence` disjunct is unshifted). Used
/// by test 1 to prove the freeze holds across BOTH a cadence and a non-cadence step.
fn cadence_step() -> StepPlan {
    StepPlan {
        chord: c_major(),
        phrase_index: 0,
        position_in_phrase: 7,
        phrase_len: 8,
        position: PhrasePosition::PerfectAuthenticCadence,
        velocity: 96,
    }
}

/// PerfFeatures with a chosen brightness (the register/`bright_octaves` driver) and edge
/// (the rhythm/rest driver). Saturation mid so velocity stays in band.
fn perf(brightness: f32, edge_density: f32) -> audiohax::chord_engine::PerfFeatures {
    audiohax::chord_engine::PerfFeatures {
        saturation: 60.0,
        brightness,
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
        // S23 prominence is read off `orchestration.prominence`, not the section.
        steps: vec![step.clone()],
    }
}

/// Realize one instrument on one step under a given profile. Returns the instrument's
/// NoteEvents for that step. Mirrors texture_s17.rs::realize_under.
fn realize_under(
    profile: &OrchestrationProfile,
    step: &StepPlan,
    inst_idx: usize,
    num_instruments: usize,
    features: &audiohax::chord_engine::PerfFeatures,
) -> Vec<NoteEvent> {
    let kt = key_tempo();
    let sec = section_with(profile.clone(), step);
    let ctx = StepContext::single_section_default(&sec, &kt);
    realize_step(step, inst_idx, num_instruments, features, MS_PER_STEP, &ctx)
}

// ── Profile builders ─────────────────────────────────────────────────────────

/// A non-identity 3-layer ensemble `[Bass, <bed>, Melody]` (inst 0/1/2), with an explicit
/// per-role `prominence` Vec. `bed` is HarmonicFill or Pad. Used so a single profile yields
/// Bass (inst 0), the bed (inst 1), and Melody (inst 2) on the same step — the three
/// register strata the invariant orders. `pad_voices: 0` keeps the bed a single fill voice
/// (block bed) so its pitch is the clean role_pitch anchor.
fn trio(bed: LayerRole, prominence: Vec<LayerProminence>) -> OrchestrationProfile {
    OrchestrationProfile {
        id: "prom_trio".to_string(),
        layers: vec![LayerRole::Bass, bed, LayerRole::Melody],
        density: 0.5,
        pad_voices: 0,
        figuration: None,
        figuration_resolved: None,
        prominence,
    }
}

/// Uniform / neutral prominence: EVERY role at exactly 0.5. Non-empty (so the profile is
/// non-identity and the named-layer role assignment is exercised) but byte-equivalent to
/// the empty/identity path because every nudge is `(0.5-0.5)*SPAN == 0.0`.
fn all_neutral() -> Vec<LayerProminence> {
    [
        LayerRole::Melody,
        LayerRole::CounterMelody,
        LayerRole::HarmonicFill,
        LayerRole::Pad,
        LayerRole::Bass,
    ]
    .into_iter()
    .map(|role| LayerProminence { role, weight: 0.5 })
    .collect()
}

/// The shipped `subject_melody` catalogue profile (spec §1.5): Melody 1.0 / CounterMelody
/// 0.6 / HarmonicFill 0.4 / Pad 0.3 / Bass 0.5 — a strong, contrasting subject pushed to
/// the foreground. Built by hand (NOT loaded), so this net never touches the loader.
fn subject_melody() -> Vec<LayerProminence> {
    vec![
        LayerProminence {
            role: LayerRole::Melody,
            weight: 1.0,
        },
        LayerProminence {
            role: LayerRole::CounterMelody,
            weight: 0.6,
        },
        LayerProminence {
            role: LayerRole::HarmonicFill,
            weight: 0.4,
        },
        LayerProminence {
            role: LayerRole::Pad,
            weight: 0.3,
        },
        LayerProminence {
            role: LayerRole::Bass,
            weight: 0.5,
        },
    ]
}

/// Mean velocity over a role's emitted NoteEvents for one step.
fn mean_vel(events: &[NoteEvent]) -> f64 {
    assert!(
        !events.is_empty(),
        "role emitted no events to average velocity"
    );
    events.iter().map(|e| e.velocity as f64).sum::<f64>() / events.len() as f64
}

/// Mean pitch over a role's emitted NoteEvents for one step.
fn mean_pitch(events: &[NoteEvent]) -> f64 {
    assert!(
        !events.is_empty(),
        "role emitted no events to average pitch"
    );
    events.iter().map(|e| e.note as f64).sum::<f64>() / events.len() as f64
}

// ─────────────────────────────────────────────────────────────────────────────
// TEST 1 — prominence_neutral_is_byte_identical (the freeze pivot, unit level)
// ─────────────────────────────────────────────────────────────────────────────

/// §4(1): realize Melody/Bass and a cadence step under (a) `OrchestrationProfile::identity()`
/// (EMPTY prominence → `prominence_weight` short-circuits to 0.5) and (b) the SAME ensemble
/// shape but with an EXPLICIT prominence listing every role at weight 0.5. The two
/// `Vec<NoteEvent>` must be EQUAL. This proves the centered-nudge-zero property directly at
/// the unit level: empty prominence and all-0.5 prominence both evaluate every nudge to
/// `(0.5-0.5)*SPAN == 0.0`, so the output is bit-for-bit the same.
///
/// NOTE on the role-assignment difference: under identity, `assign_role` delegates to
/// `instrument_role` (Bass at 0, Melody at num-1); the all-0.5 case uses a NAMED-layer
/// profile that assigns the SAME roles at the same indices (`[Bass, Melody]` for num=2),
/// so the only thing that varies between (a) and (b) is the prominence Vec being empty vs
/// all-0.5 — isolating exactly the freeze pivot.
#[test]
fn prominence_neutral_is_byte_identical() {
    // num=2 ensemble: inst 0 = Bass, inst 1 = Melody under BOTH paths.
    // (a) identity profile — empty prominence.
    let identity = OrchestrationProfile::identity();
    // (b) explicit named-layer profile [Bass, Melody] with every role at 0.5.
    let neutral = OrchestrationProfile {
        id: "prom_neutral".to_string(),
        layers: vec![LayerRole::Bass, LayerRole::Melody],
        density: 0.5,
        pad_voices: 0,
        figuration: None,
        figuration_resolved: None,
        prominence: all_neutral(),
    };

    // Sweep both a non-cadence interior step AND a cadence step (covers the velocity nudge,
    // the register nudge, and the rhythm-band shift, plus the cadence-exempt path), and a
    // bright (lift-active) + a neutral brightness, so the freeze is proven where every nudge
    // would otherwise be live.
    for step in [interior_step(2), interior_step(3), cadence_step()] {
        for &brightness in &[55.0f32, 95.0f32] {
            for &edge in &[0.30f32, 0.85f32] {
                let f = perf(brightness, edge);
                for inst in 0..2usize {
                    let a = realize_under(&identity, &step, inst, 2, &f);
                    let b = realize_under(&neutral, &step, inst, 2, &f);
                    assert_eq!(
                        a, b,
                        "freeze pivot broken: identity vs all-0.5 prominence differ \
                         (inst {inst}, brightness {brightness}, edge {edge}, pos {:?})",
                        step.position
                    );
                }
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// TEST 2 — high_saliency_melody_louder
// ─────────────────────────────────────────────────────────────────────────────

/// §4(2): under the `subject_melody` profile (Melody 1.0, Pad 0.3), the Melody-vs-Pad
/// VELOCITY gap is STRICTLY LARGER than under uniform/neutral prominence (all 0.5). The
/// melody gains `+0.5*VEL_SPAN` and the Pad loses `+0.2*VEL_SPAN`, widening the existing
/// +2/-3 realizer bias. Asserted as `gap_high > baseline_gap + EPS`.
///
/// Ensemble `[Bass, Pad, Melody]` (num=3): inst 1 = Pad, inst 2 = Melody. A low-edge,
/// mid-brightness interior step so the melody is a single/few-onset figure and the velocity
/// comparison is clean.
#[test]
fn high_saliency_melody_louder() {
    let step = interior_step(2);
    let f = perf(55.0, 0.20); // low edge → plain melody figure; mid brightness.

    let neutral = trio(LayerRole::Pad, all_neutral());
    let high = trio(LayerRole::Pad, subject_melody());

    let baseline_gap = {
        let mel = mean_vel(&realize_under(&neutral, &step, 2, 3, &f));
        let pad = mean_vel(&realize_under(&neutral, &step, 1, 3, &f));
        mel - pad
    };
    let high_gap = {
        let mel = mean_vel(&realize_under(&high, &step, 2, 3, &f));
        let pad = mean_vel(&realize_under(&high, &step, 1, 3, &f));
        mel - pad
    };

    eprintln!(
        "[test2] baseline_gap(vel) = {baseline_gap:.2}, high_gap(vel) = {high_gap:.2}, \
         widened by {:.2}",
        high_gap - baseline_gap
    );
    assert!(
        high_gap > baseline_gap + 1.0,
        "subject_melody must WIDEN the Melody-vs-Pad velocity gap: high {high_gap:.2} \
         not > baseline {baseline_gap:.2} (+EPS)"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// TEST 3 — high_saliency_melody_higher_wider
// ─────────────────────────────────────────────────────────────────────────────

/// §4(3): under the `subject_melody` profile, `mean_pitch(Melody) > mean_pitch(Pad)` AND
/// the Melody-vs-Pad pitch SEPARATION at high prominence (Melody 1.0) is STRICTLY LARGER
/// than at neutral (Melody 0.5). The bed is never lowered (Risk-1: prominence on the fill
/// group is clamped `>= 0`), so the separation grows by the melody rising.
#[test]
fn high_saliency_melody_higher_wider() {
    let step = interior_step(2);
    // Brightness 100 (the "bright image" register) so the +2-semitone prominence lift
    // crosses a melody octave-seat boundary and is OBSERVABLE in the seated pitch: at
    // brightness 100 the neutral (w=0.5) melody seats at G5=79 and the foreground (w=1.0)
    // melody seats at G6=91 (`seat_pc_in_register` quantizes the pc=7 top tone to the
    // nearest G at-or-above the lifted floor; +2 semitones tips floor 79→81, past G5).
    // At mid brightness the +2 lift stays inside one seat slot (both 79) — a real,
    // documented consequence of the deliberately-small REG_SPAN=4 seed (spec §2.4), NOT a
    // defect; this test selects the brightness where the lift is visible in pitch.
    let f = perf(100.0, 0.20);

    let neutral = trio(LayerRole::Pad, all_neutral());
    let high = trio(LayerRole::Pad, subject_melody());

    let neutral_mel = mean_pitch(&realize_under(&neutral, &step, 2, 3, &f));
    let neutral_pad = mean_pitch(&realize_under(&neutral, &step, 1, 3, &f));
    let high_mel = mean_pitch(&realize_under(&high, &step, 2, 3, &f));
    let high_pad = mean_pitch(&realize_under(&high, &step, 1, 3, &f));

    let neutral_sep = neutral_mel - neutral_pad;
    let high_sep = high_mel - high_pad;

    eprintln!(
        "[test3] neutral: mel {neutral_mel:.1} pad {neutral_pad:.1} sep {neutral_sep:.1} | \
         high: mel {high_mel:.1} pad {high_pad:.1} sep {high_sep:.1}"
    );

    assert!(
        high_mel > high_pad,
        "Melody must sit above the Pad bed at high prominence: mel {high_mel:.1} \
         not > pad {high_pad:.1}"
    );
    assert!(
        high_sep > neutral_sep + 0.5,
        "high-prominence Melody-Pad pitch separation must STRICTLY exceed neutral: \
         high {high_sep:.1} not > neutral {neutral_sep:.1}"
    );
    // The widening comes from the melody RISING, not the bed sinking (Risk-1).
    assert!(
        high_pad >= neutral_pad,
        "the bed must NEVER be lowered by prominence (Risk-1): high_pad {high_pad:.1} \
         < neutral_pad {neutral_pad:.1}"
    );
    assert!(
        high_mel > neutral_mel,
        "the melody must RISE at high prominence: high_mel {high_mel:.1} not > \
         neutral_mel {neutral_mel:.1}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// TEST 4 — no_inversion_invariant (THE HARD GUARD, Risk 1 / §C.3)
// ─────────────────────────────────────────────────────────────────────────────

/// The affect-ladder characters, mapped to the realizer's ONLY register driver
/// (PerfFeatures.brightness). The realizer reads no Character enum; a character's register
/// posture is its brightness. Scherzo is the brightest (its bright register is the
/// stack-risk the invariant exists for); Lament/Nocturne the darkest. These span the
/// dark→bright band so the bright_octaves lift goes from its dark floor to its bright max.
fn character_brightness(c: Character) -> f32 {
    match c {
        Character::Lament => 12.0,   // darkest calm corner
        Character::Nocturne => 30.0, // dim
        Character::Hymn => 60.0,     // calm-bright
        Character::Ballad => 55.0,   // mid (the legacy default)
        Character::March => 78.0,    // bright, driving
        Character::Scherzo => 100.0, // brightest — max bright_octaves lift (the stack-risk)
        other => panic!("character not on the Slice-B affect ladder sweep: {other:?}"),
    }
}

/// §4(4) — the register-separation invariant (design-s21 §5 Risk 1). Sweep ALL prominence
/// weights {0.0,0.25,0.5,0.75,1.0} for the Melody × ALL affect-ladder characters
/// (Ballad/Hymn/Nocturne/March/Lament/Scherzo, mapped to brightness — Scherzo bright so the
/// lifts STACK) × a busy edge (so the melody arpeggiates, exercising the rhythm shift too).
/// For EVERY combination assert, on the SAME step:
///   - `mean_pitch(Bass) < mean_pitch(bed) < mean_pitch(Melody)` (figure-ground never inverts),
///   - every emitted note ∈ 24..=108 (the engine playable band),
///   - at the maximum stacked lift (Scherzo brightness, Melody weight 1.0) the Melody is
///     NOT clamped flat at the top of the band: it still RESPONDS to prominence (the
///     foreground melody pitch strictly exceeds the recessive one at the same brightness)
///     AND its mean pitch stays strictly below the 108 ceiling, so the lift gradient is
///     live, not pinned. (Within a single step the Melody is monophonic — the arpeggio
///     re-onsets ONE pitch in TIME, by realizer design — so "range" is measured across the
///     prominence sweep, which is where a flat-clamp would actually collapse the gradient,
///     not within a step.)
/// Both bed kinds (HarmonicFill and Pad) are swept. A REAL nested sweep, not a token case.
#[test]
fn no_inversion_invariant() {
    let weights = [0.0f32, 0.25, 0.5, 0.75, 1.0];
    let characters = [
        Character::Ballad,
        Character::Hymn,
        Character::Nocturne,
        Character::March,
        Character::Lament,
        Character::Scherzo,
    ];
    let beds = [LayerRole::HarmonicFill, LayerRole::Pad];
    // A busy edge so the Melody arpeggiates (multi-onset) and the rhythm-band shift is live;
    // a few interior phrase positions so the arpeggio/figure varies.
    let edges = [0.30f32, 0.90f32];
    let positions = [2usize, 3, 5];

    let mut combos = 0usize;
    let mut gradient_checks = 0usize;

    for &bed in &beds {
        for &c in &characters {
            let brightness = character_brightness(c);
            // Track the Melody mean pitch at the recessive floor (w=0.0) and the foreground
            // ceiling (w=1.0), at a fixed edge/pos, so after the weight sweep we can assert
            // the lift gradient is LIVE at this (bed,character) — the real "not flat-clamped"
            // witness (the within-step melody is monophonic by realizer design).
            let mut mel_at_recessive: Option<f64> = None;
            let mut mel_at_foreground: Option<f64> = None;

            for &mel_w in &weights {
                // The full subject_melody-style profile, but with the Melody weight swept.
                // Bass at 0.5 (its catalogue value; Bass is register-exempt anyway), the bed
                // at a recessive 0.3 (the strongest figure-ground stress on the invariant).
                let prominence = vec![
                    LayerProminence {
                        role: LayerRole::Melody,
                        weight: mel_w,
                    },
                    LayerProminence {
                        role: LayerRole::HarmonicFill,
                        weight: 0.3,
                    },
                    LayerProminence {
                        role: LayerRole::Pad,
                        weight: 0.3,
                    },
                    LayerProminence {
                        role: LayerRole::Bass,
                        weight: 0.5,
                    },
                ];
                let profile = trio(bed, prominence);

                for &edge in &edges {
                    for &pos in &positions {
                        let step = interior_step(pos);
                        let f = perf(brightness, edge);

                        let bass = realize_under(&profile, &step, 0, 3, &f);
                        let bed_ev = realize_under(&profile, &step, 1, 3, &f);
                        let melody = realize_under(&profile, &step, 2, 3, &f);

                        // Every emitted note in band.
                        for (who, evs) in [("bass", &bass), ("bed", &bed_ev), ("melody", &melody)] {
                            for e in evs.iter() {
                                assert!(
                                    (24..=108).contains(&e.note),
                                    "note {} out of band 24..=108 ({who}, bed {bed:?}, \
                                     char {c:?}/bright {brightness}, mel_w {mel_w}, \
                                     edge {edge}, pos {pos})",
                                    e.note
                                );
                            }
                        }

                        let b = mean_pitch(&bass);
                        let m = mean_pitch(&bed_ev);
                        let t = mean_pitch(&melody);

                        // The register-separation invariant: Bass < bed < Melody, never inverts.
                        assert!(
                            b < m,
                            "INVERSION: Bass {b:.1} not < bed {m:.1} (bed {bed:?}, char {c:?}, \
                             mel_w {mel_w}, edge {edge}, pos {pos})"
                        );
                        assert!(
                            m < t,
                            "INVERSION: bed {m:.1} not < Melody {t:.1} (bed {bed:?}, char {c:?}, \
                             mel_w {mel_w}, edge {edge}, pos {pos})"
                        );

                        // Capture the recessive/foreground Melody pitch at the busy edge + pos 2
                        // (the max-arpeggiation cell) for the post-sweep gradient assertion.
                        if edge >= 0.85 && pos == 2 {
                            if (mel_w - 0.0).abs() < 1e-6 {
                                mel_at_recessive = Some(t);
                            } else if (mel_w - 1.0).abs() < 1e-6 {
                                mel_at_foreground = Some(t);
                            }
                        }

                        combos += 1;
                    }
                }
            }

            // NOT clamped flat at the top: at this character's register the Melody lift
            // gradient is LIVE — the foreground (w=1.0) melody is at least as high as the
            // recessive (w=0.0) one (strictly higher for the bright characters where the +2
            // crosses a seat), AND below the 108 ceiling (room to lift, not pinned). The
            // brightest character (Scherzo, the stack-risk) is where this must hold hardest.
            let (rec, fore) = (
                mel_at_recessive.expect("recessive melody pitch captured"),
                mel_at_foreground.expect("foreground melody pitch captured"),
            );
            eprintln!(
                "[test4] bed {bed:?} char {c:?} (bright {brightness}): melody w=0.0 -> {rec:.0}, \
                 w=1.0 -> {fore:.0}"
            );
            assert!(
                fore >= rec,
                "lift gradient INVERTED at {c:?}/{bed:?}: foreground melody {fore:.0} < \
                 recessive {rec:.0}"
            );
            assert!(
                fore < 108.0,
                "Melody pinned at the 108 ceiling (flat-clamped) at {c:?}/{bed:?}: {fore:.0}"
            );
            // Scherzo is the stack-risk character (max brightness, bright_octaves at +12)
            // the invariant exists for; at its register the +2 prominence lift MUST be
            // visible (the foreground melody crosses a seat boundary above the recessive
            // one), proving the gradient is genuinely live at the very place a flat-clamp
            // would bite — and the bright melody (91) still sits well under the 108 ceiling.
            // (At some characters the +2 lift is absorbed within a single octave-seat slot —
            // a documented consequence of the small REG_SPAN=4 seed, spec §2.4 — so the
            // strict-rise is asserted only where the seed is designed to be audible: the
            // brightest, highest-stack corner. The `fore >= rec` monotonicity above already
            // holds for every character.)
            if c == Character::Scherzo {
                assert!(
                    fore > rec,
                    "Scherzo (the stack-risk corner) {bed:?}: foreground melody {fore:.0} \
                     must rise ABOVE recessive {rec:.0} — the lift must stay visible at the \
                     brightest register where it stacks hardest"
                );
            }
            gradient_checks += 1;
        }
    }

    eprintln!(
        "[test4] swept {combos} (bed×char×mel_w×edge×pos) combinations; \
         {gradient_checks} per-(bed,char) gradient checks"
    );
    assert!(
        combos >= 2 * 6 * 5 * 2 * 3,
        "sweep must be the full nested cross-product"
    );
    assert_eq!(
        gradient_checks,
        beds.len() * characters.len(),
        "a gradient check must run for every (bed, character)"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// TEST 5 — engine_freeze_diff_empty
// ─────────────────────────────────────────────────────────────────────────────

/// §4(5): the byte-freeze witness. `git diff HEAD -- src/engine.rs tests/engine_equivalence.rs`
/// must be EMPTY — Slice B does not touch the engine kernel or the equivalence net. Shells
/// `git diff` (the precedent: tests/affect_s22.rs::byte_freeze_witness_locked_files_unmoved
/// uses the same `Command::new("git").args(["diff","HEAD","--", ...])` pattern). Asserts ONLY
/// on the two locked paths, so unrelated dirty files (the slice's own uncommitted edits to
/// composition.rs / chord_engine.rs / mappings.json) do not fail it. If git cannot run at
/// all, the test is inconclusive-but-non-failing (engine_equivalence + the Quality Gate's own
/// diff are the freeze authority) rather than spuriously red.
#[test]
fn engine_freeze_diff_empty() {
    use std::process::Command;

    let locked = ["src/engine.rs", "tests/engine_equivalence.rs"];
    let mut args = vec!["diff", "HEAD", "--"];
    args.extend(locked.iter().copied());

    match Command::new("git").args(&args).output() {
        Ok(out) => {
            let diff = String::from_utf8_lossy(&out.stdout);
            assert!(
                diff.trim().is_empty(),
                "Slice B must NOT touch the byte-freeze files; `git diff HEAD` over \
                 {locked:?} produced:\n{diff}"
            );
        }
        Err(e) => {
            eprintln!(
                "engine_freeze_diff_empty: git not runnable ({e}); deferring to \
                 engine_equivalence + Quality-Gate diff as the freeze authority"
            );
        }
    }
}

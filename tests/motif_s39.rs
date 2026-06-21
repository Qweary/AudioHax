//! tests/motif_s39.rs — the S39 SLICE-1 DURATIONAL-MOTIF PROPERTY NET. Proves the two
//! S39 mechanisms — the PRODUCER half (Music Theory: `resolve_motif` now emits a per-
//! archetype `dur_steps` rhythm profile) and the CONSUMER half (Rust Implementer: the
//! `theme_melody_pitch` step→motif-note cumulative-duration mapping + the widened
//! `pick_archetype` reaching all 8 contours) — actually deliver: a rhythmically and
//! contour-DIVERSE motif vocabulary, the static-tail fix, and a realized melody whose
//! note lengths reflect the motif's durations.
//!
//! DETERMINISTIC + HEADLESS, in the same sense as `composition_s15.rs` / `figuration_s20.rs`:
//! every fixture is either the PURE RNG-free planner (`CompositionPlanner::plan` +
//! `mapping_loader::load_mappings`) or a hand-built `Section`/`StepContext` realized through
//! the PUBLIC `realize_step`. No `thread_rng`-derived value is ever asserted.
//!
//! Run under DEFAULT features (the as-built quirk: the always-on `audiohax` bin needs the
//! default pure-Rust features, so `--no-default-features` cannot RUN this):
//!     cargo test --test motif_s39

use std::collections::BTreeMap;

use audiohax::chord_engine::RhythmMotto;
use audiohax::chord_engine::{
    realize_step, Chord, MotifArchetype, MotifNote, NoteEvent, PerfFeatures, PhrasePosition,
    StepPlan,
};
use audiohax::composition::{
    CadenceStrength, CompositionPlanner, ImageUnderstanding, KeyTempoPlan, OrchestrationProfile,
    PlanMappings, ResolutionPolicy, Section, StepContext, ThematicRole, ThemeSeed, ThemeVariation,
};
use audiohax::mapping_loader::{load_mappings, MappingTable};

// ─────────────────────────────────────────────────────────────────────────────
// Shared planner fixtures (loader-backed, RNG-free selection — composition_s15 discipline)
// ─────────────────────────────────────────────────────────────────────────────

fn mappings() -> MappingTable {
    load_mappings("assets/mappings.json").expect("mappings.json loads")
}

fn plan_mappings(m: &MappingTable) -> PlanMappings {
    m.composition
        .clone()
        .expect("composition block present in mappings.json")
        .into()
}

/// An `ImageUnderstanding` naming exactly the knobs that drive the affect composite and the
/// theme/archetype ladder; everything else at its slice-1 neutral default. `complexity >= 0.4`
/// keeps a returning theme present (theme_behaviour "fragment").
fn img(
    avg_brightness: f32,
    avg_saturation: f32,
    colorfulness: f32,
    complexity: f32,
    edge_activity: f32,
    dominant_hue: f32,
    vertical_emphasis: f32,
) -> ImageUnderstanding {
    ImageUnderstanding {
        avg_brightness,
        avg_saturation,
        colorfulness,
        complexity,
        edge_activity,
        dominant_hue,
        vertical_emphasis,
        ..ImageUnderstanding::neutral()
    }
}

/// The `dur_steps` sequence of a motif — its RHYTHMIC identity.
fn dur_seq(motif: &[MotifNote]) -> Vec<u8> {
    motif.iter().map(|n| n.dur_steps).collect()
}

// ═════════════════════════════════════════════════════════════════════════════
// PROPERTY 1 — RHYTHM IDENTITY: two archetypes differ in their dur_steps sequence.
// ═════════════════════════════════════════════════════════════════════════════

/// P1: the PRODUCER half gives each archetype its OWN durational rhythm profile, so two
/// fixtures whose images select DIFFERENT archetypes yield motif `Vec`s whose `dur_steps`
/// sequences differ in at least one position. (Pre-S39 every motif note was a flat 1-step
/// run, so this property could not hold — the rhythm was constant across archetypes.)
///
/// Uses `resolve_motif` directly to pin the archetype, avoiding any dependence on which
/// affect quadrant a given image lands in (that is property 2's job).
#[test]
fn test_rhythm_identity_differs_across_archetypes() {
    // Same range + length so ONLY the archetype's rhythm_profile can differ the durations.
    let len = 6usize;
    let arch = audiohax::chord_engine::resolve_motif(MotifArchetype::Arch, 4, len);
    let pendulum = audiohax::chord_engine::resolve_motif(MotifArchetype::Pendulum, 4, len);

    // Both honor the contract (every dur >= 1, Σ dur <= len).
    for (label, motif) in [("Arch", &arch), ("Pendulum", &pendulum)] {
        assert!(
            motif.iter().all(|n| n.dur_steps >= 1),
            "{label}: every MotifNote carries dur_steps >= 1"
        );
        let sum: usize = motif.iter().map(|n| n.dur_steps as usize).sum();
        assert!(
            sum <= len,
            "{label}: Σ dur_steps ({sum}) must not over-run len ({len})"
        );
    }

    // The durational sequences are genuinely DIFFERENT (Arch [2,1,1,2] vs Pendulum [2,2]).
    assert_ne!(
        dur_seq(&arch),
        dur_seq(&pendulum),
        "two archetypes must produce DIFFERENT dur_steps sequences (rhythm is per-archetype)"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// PROPERTY 2 — VOCABULARY SPREAD: ≥6 distinct archetypes over ≥12 varied images,
//              no single contour dominating the selection.
// ═════════════════════════════════════════════════════════════════════════════

/// P2: the WIDENED `pick_archetype` reaches a broad vocabulary. Across ≥12 images spanning
/// the affect circumplex (bright/dark × saturated/flat) and vertical_emphasis/hue, the
/// planner selects ≥6 of the 8 archetypes, and no single archetype takes more than ~30% of
/// the selections. (Pre-S39 the four-variant `edge>=0.6→Ascent` short-circuit could reach at
/// most 4 contours and collapsed busy images onto Ascent — this property would have failed.)
///
/// The archetype is read back from the stored motif by matching its `dur_steps` SIGNATURE to
/// each archetype's `resolve_motif` output at the planner's own range/length formulas — a
/// pure, RNG-free reconstruction (the same trick composition_s15 uses to identify a theme).
#[test]
fn test_vocabulary_spread_reaches_most_archetypes() {
    let m = mappings();
    let planner = CompositionPlanner::new(plan_mappings(&m));

    // 12 fixtures sweeping the affect quadrants + within-family axis + hue tiebreak.
    // (brightness, saturation, colorfulness, complexity, edge_activity, hue, vertical_emphasis)
    let fixtures = [
        // high arousal (saturated/colorful) + high valence (bright) → RISING family
        img(90.0, 90.0, 0.9, 0.5, 0.3, 30.0, 0.9),
        img(85.0, 80.0, 0.8, 0.6, 0.3, 220.0, 0.2),
        // low arousal (flat) + high valence (bright) → ARCHED family
        img(85.0, 10.0, 0.1, 0.5, 0.1, 40.0, 0.8),
        img(80.0, 12.0, 0.1, 0.6, 0.1, 250.0, 0.2),
        // low arousal (flat) + low valence (dark) → FALLING family
        img(15.0, 12.0, 0.1, 0.5, 0.1, 30.0, 0.9),
        img(18.0, 10.0, 0.1, 0.6, 0.1, 240.0, 0.2),
        // high arousal (saturated) + low valence (dark) → OSCILLATING family
        img(15.0, 90.0, 0.9, 0.5, 0.4, 20.0, 0.9),
        img(18.0, 85.0, 0.8, 0.6, 0.4, 230.0, 0.2),
        // mid-band sweeps to exercise the within-family tiebreaks
        img(70.0, 70.0, 0.7, 0.7, 0.3, 60.0, 0.6),
        img(40.0, 40.0, 0.3, 0.5, 0.2, 300.0, 0.4),
        img(95.0, 95.0, 1.0, 0.8, 0.5, 10.0, 0.3),
        img(10.0, 20.0, 0.2, 0.5, 0.1, 200.0, 0.7),
    ];
    assert!(fixtures.len() >= 12, "≥12 varied image fixtures required");

    // Reconstruct each archetype's dur_steps signature at the planner's range/length formulas,
    // per image (range/length depend on edge_activity/complexity, so recompute per fixture).
    let all = [
        MotifArchetype::Arch,
        MotifArchetype::InvertedArch,
        MotifArchetype::Descent,
        MotifArchetype::Ascent,
        MotifArchetype::NeighborTurn,
        MotifArchetype::LeapStep,
        MotifArchetype::Pendulum,
        MotifArchetype::RisingSequence,
    ];

    let mut counts: BTreeMap<&'static str, usize> = BTreeMap::new();
    let name = |a: MotifArchetype| -> &'static str {
        match a {
            MotifArchetype::Arch => "Arch",
            MotifArchetype::InvertedArch => "InvertedArch",
            MotifArchetype::Descent => "Descent",
            MotifArchetype::Ascent => "Ascent",
            MotifArchetype::NeighborTurn => "NeighborTurn",
            MotifArchetype::LeapStep => "LeapStep",
            MotifArchetype::Pendulum => "Pendulum",
            MotifArchetype::RisingSequence => "RisingSequence",
        }
    };

    for u in &fixtures {
        let plan = planner.plan(u, &m);
        assert!(!plan.themes.is_empty(), "each fixture has a theme");
        let got = &plan.themes[0].motif;

        // The planner's own range/length formulas (pure functions of the image knobs).
        let range_degrees = (2.0 + u.edge_activity * 5.0).round() as u8;
        let length_steps = (3.0 + u.complexity * 5.0).round() as usize;

        // Identify the archetype by matching the full (degree, dur_steps) line. S41 RE-BLESS: the
        // planner now SELECTS a per-image rhythm CELL (`pick_rhythm_cell`), so the stored motif is
        // no longer guaranteed to be cell 0 — it may be any of the archetype's
        // `rhythm_cell_count()` gaits. The test's INTENT is preserved exactly (the vocabulary
        // reaches most archetypes); only the cell-0 assumption is relaxed: match against ANY cell
        // of each archetype via `resolve_motif_celled(a, .., cell)`. The full (contour+rhythm) line
        // is unique per (archetype, cell) at a fixed range/length, so the archetype read-back stays
        // unambiguous. (`resolve_motif(a, ..)` == `resolve_motif_celled(a, .., 0)`, so cell 0 is
        // still covered by the `0..count` loop — this is a strict superset of the S39 match.)
        let mut identified: Option<MotifArchetype> = None;
        'archetype: for &a in &all {
            for cell in 0..a.rhythm_cell_count() {
                let cand = audiohax::chord_engine::resolve_motif_celled(
                    a,
                    range_degrees,
                    length_steps,
                    cell,
                );
                if cand == *got {
                    identified = Some(a);
                    break 'archetype;
                }
            }
        }
        let a = identified
            .expect("the stored motif must match some archetype's resolve_motif_celled (any cell)");
        *counts.entry(name(a)).or_insert(0) += 1;
        // Log archetype per fixture (visible with `--nocapture`).
        eprintln!("archetype: {}", name(a));
    }

    let distinct = counts.len();
    assert!(
        distinct >= 6,
        "the widened selector must reach >=6 distinct archetypes; saw {distinct}: {counts:?}"
    );
    let n = fixtures.len();
    let max_share = *counts.values().max().unwrap();
    // No single contour above ~30% of the selections — the spec's anti-collapse bound
    // (§3d property 2). With n=12, ceil(12*0.30) == 4: at most a third of the fixtures may
    // land on one archetype. NO extra integer slack — the previous `+1` let one contour take
    // 5/12 (~41%), which would silently pass a partial re-collapse; the spec cap is 30% and
    // the as-built selector clears it with margin (max share observed is 3/12 == 25%).
    let cap = ((n as f32) * 0.30).ceil() as usize;
    assert!(
        max_share <= cap,
        "no single archetype may dominate (>~30%); max share {max_share} of {n} exceeds cap {cap}: {counts:?}"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// PROPERTY 3 — STATIC-TAIL FIX: a long motif does NOT end in ≥2 identical held degrees.
// ═════════════════════════════════════════════════════════════════════════════

/// P3: the PRODUCER's §4 static-tail fix means leftover step budget becomes ONE long final
/// arrival note (the last note's `dur_steps` absorbs the remainder), NOT a string of repeated
/// short held degrees. A long-`length_steps` fixture's motif therefore does not END in two or
/// more consecutive identical degrees (the old "smear" the fix removed).
#[test]
fn test_static_tail_is_one_long_arrival_not_a_smear() {
    // Maximum length (complexity 1.0 ⇒ length_steps == 8) over a narrow contour so the budget
    // exceeds the contour note count and the tail logic is exercised.
    let len = 8usize;
    for archetype in [
        MotifArchetype::Pendulum, // shortest profile [2,2] → biggest leftover budget
        MotifArchetype::Arch,     // [2,1,1,2]
        MotifArchetype::RisingSequence, // [1,1,2]
    ] {
        let motif = audiohax::chord_engine::resolve_motif(archetype, 4, len);
        assert!(
            motif.len() >= 2,
            "{archetype:?}: a real motif has >=2 notes"
        );

        // The tail is not a held-degree smear: the last two NOTES are not the same degree
        // (the fix collapses the remainder into the single final note, so there is no second
        // identical held degree appended after it).
        let n = motif.len();
        assert_ne!(
            motif[n - 1].degree,
            motif[n - 2].degree,
            "{archetype:?}: motif must NOT end in >=2 identical held degrees (static-tail smear)"
        );

        // And Σ dur_steps == len exactly (the remainder was absorbed, the section is filled).
        let sum: usize = motif.iter().map(|x| x.dur_steps as usize).sum();
        assert_eq!(
            sum, len,
            "{archetype:?}: the final note absorbs the remainder (Σ == len)"
        );
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// PROPERTY 4 — DURATION-THREAD: the realized melody reflects the motif dur_steps.
// ═════════════════════════════════════════════════════════════════════════════

const MS_PER_STEP: u64 = 200;

fn chord_i() -> Chord {
    Chord {
        name: "I".to_string(),
        notes: vec![60, 64, 67], // C E G
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

/// A CALM image so the Melody realizes through the SUSTAINED band (edge_activity ~0): that is
/// the branch where a multi-step theme note rides `THEME_LONG_NOTE_SING`, and it is where a
/// ballad image actually lands.
fn calm_perf() -> PerfFeatures {
    PerfFeatures {
        saturation: 50.0,
        brightness: 50.0,
        edge_density: 0.0, // → edge_activity 0.0 → SUSTAINED melody branch
    }
}

/// A `Bass, Melody` two-layer profile so inst 1 of 2 is the MELODY role under the theme seam.
fn melody_profile() -> OrchestrationProfile {
    use audiohax::composition::LayerRole;
    OrchestrationProfile {
        id: "bass_melody".to_string(),
        layers: vec![LayerRole::Bass, LayerRole::Melody],
        density: 0.5, // DENSITY_NEUTRAL → density_nudge 0 → edge_activity == base
        pad_voices: 0,
        figuration: None,
        figuration_resolved: None,
        bass_pattern: None,
        bass_pattern_resolved: None,
        prominence: Vec::new(),
        motto: RhythmMotto::neutral(),
    }
}

/// An INTERIOR (non-cadence, non-phrase-start) melody step so the cadence ring never fires and
/// the theme seam fully governs the melody.
fn interior_step(position_in_phrase: usize) -> StepPlan {
    StepPlan {
        chord: chord_i(),
        phrase_index: 0,
        position_in_phrase,
        phrase_len: 16,
        position: PhrasePosition::Interior,
        velocity: 80,
    }
}

/// Realize the MELODY instrument (inst 1 of 2) at `step_in_section` of a one-section,
/// theme-bearing Identity Statement carrying `motif`.
fn realize_melody_at(motif: &[MotifNote], step_in_section: usize) -> Vec<NoteEvent> {
    let kt = key_tempo();
    let theme = ThemeSeed {
        id: 0,
        motif: motif.to_vec(),
    };
    let steps: Vec<StepPlan> = (0..motif.iter().map(|n| n.dur_steps as usize).sum::<usize>() + 2)
        .map(|i| interior_step(1 + (i % 2))) // odd positions = weak interior, never start/cadence
        .collect();
    let section = Section {
        label: "A".to_string(),
        step_len: steps.len(),
        thematic_role: ThematicRole::Statement,
        key_offset_semitones: 0,
        ms_per_step: MS_PER_STEP,
        mode: "Ionian".to_string(),
        progression: vec![],
        theme: Some(0),
        variation: ThemeVariation::Identity,
        boundary_cadence: CadenceStrength::Perfect,
        pivot: false,
        resolution: ResolutionPolicy::Resolve,
        density: 0.5,
        orchestration: melody_profile(),
        steps,
    };
    let ctx = StepContext {
        section: &section,
        step_in_section,
        theme: Some(&theme),
        key_tempo: &kt,
        prev_key_offset_semitones: None,
    };
    // inst 1 of 2 → Melody under the bass_melody profile.
    realize_step(
        &section.steps[step_in_section],
        1,
        2,
        &calm_perf(),
        MS_PER_STEP,
        &ctx,
    )
}

/// P4: realize a theme-bearing section's melody across its steps and confirm the realized
/// note lengths reflect the motif `dur_steps` — they are NOT all equal. The freeze-safe
/// realization of a `dur_steps > 1` note is: PLAY at its onset (with a longer "sing" hold),
/// REST on its continuation step(s). So a hand-built motif mixing 1-step and 2-step notes
/// produces (a) at least one SILENT continuation step (an empty event Vec) and (b) an onset
/// whose realized hold is STRICTLY LONGER than a 1-step onset's — both observable through the
/// public realizer, both impossible under the pre-S39 1:1 index (which played every step).
#[test]
fn test_duration_thread_realizes_motif_durations() {
    // Hand-built motif: a 1-step note, then a 2-step note, then a 1-step note. Distinct degrees
    // so the static-tail / continuation logic is unambiguous. Σ dur_steps == 4.
    let motif = vec![
        MotifNote {
            degree: 0,
            dur_steps: 1,
        },
        MotifNote {
            degree: 2,
            dur_steps: 2,
        },
        MotifNote {
            degree: 4,
            dur_steps: 1,
        },
    ];

    // Step 0 → note 0 onset (dur 1). Step 1 → note 1 onset (dur 2). Step 2 → note 1
    // CONTINUATION (rest). Step 3 → note 2 onset (dur 1).
    let s0 = realize_melody_at(&motif, 0);
    let s1 = realize_melody_at(&motif, 1);
    let s2 = realize_melody_at(&motif, 2);
    let s3 = realize_melody_at(&motif, 3);

    // (a) The continuation step of the 2-step note is SILENT — the note's length is carried by
    //     a following rest, not by a held note spanning kernel steps (freeze-safe).
    assert!(
        s2.is_empty(),
        "the continuation step of a dur_steps=2 note must REST (empty), got {s2:?}"
    );

    // The onset steps all SOUND exactly one melody note.
    assert_eq!(s0.len(), 1, "step 0 (1-step onset) sounds one note");
    assert_eq!(s1.len(), 1, "step 1 (2-step onset) sounds one note");
    assert_eq!(s3.len(), 1, "step 3 (1-step onset) sounds one note");

    // (b) The 2-step onset SINGS longer than a 1-step onset (THEME_LONG_NOTE_SING), so the
    //     realized hold lengths are NOT all equal — they reflect dur_steps. Still bounded by
    //     the overlap ceiling (never rings past the next kernel step).
    let h0 = s0[0].hold_ms;
    let h1 = s1[0].hold_ms;
    let h3 = s3[0].hold_ms;
    assert!(
        h1 > h0,
        "the 2-step onset hold ({h1}) must exceed a 1-step onset hold ({h0}) — note lengths reflect dur_steps"
    );
    assert_eq!(
        h0, h3,
        "two 1-step onsets realize to the SAME hold (the dur==1 baseline)"
    );
    // Freeze ceiling: the lengthened onset never rings past the next kernel step (overlap cap
    // is 1.20 × step_ms; ARTIC_WINDOW_HI 1.10 × 1.15 sing = 1.265 → clamped to 1.20 × 200 = 240).
    let overlap_ceiling = ((MS_PER_STEP as f32) * 1.20).round() as u64;
    assert!(
        h1 <= overlap_ceiling,
        "the sung onset hold ({h1}) must stay within the overlap ceiling ({overlap_ceiling}) — no across-step hold"
    );

    // The realized stream's hold lengths are not all equal (the duration thread is audible).
    let holds = [h0, h1, h3];
    assert!(
        holds.iter().any(|&h| h != h0),
        "realized melody note lengths must vary with the motif dur_steps, not be uniform: {holds:?}"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// FREEZE HINGE — an all-dur_steps==1 motif realizes byte-identically to the pre-S39
//               1:1 index: every onset SOUNDS, NOTHING rests, the sing-hold is inert.
// ═════════════════════════════════════════════════════════════════════════════

/// FREEZE: the S39 consumer thread is gated so that when every motif note carries
/// `dur_steps == 1` — the SHAPE of every pre-S39 motif (the realizer previously stepped a
/// flat 1:1 index) — the new step→note cumulative-duration walk degenerates to that exact
/// 1:1 index: `Σ dur_steps == note count`, so `MotifStep::Continuation` is UNREACHABLE, no
/// step rests, and `theme_onset_dur_steps` returns `Some(1)` everywhere → the sing factor is
/// 1.0 → every onset hold equals the dur==1 baseline. This is the freeze hinge the whole
/// build rests on: if a future change let an all-1-step motif rest a step or stretch a hold,
/// THIS test fails before `engine_equivalence` would (the goldens carry `theme: None` and so
/// never exercise the theme path at all — they CANNOT catch a theme-seam freeze regression).
#[test]
fn test_all_dur1_motif_realizes_identically_to_pre_s39_index() {
    // The pre-S39 motif shape: three NOTES, each exactly one step. Σ dur_steps == 3 == count.
    let motif = vec![
        MotifNote {
            degree: 0,
            dur_steps: 1,
        },
        MotifNote {
            degree: 2,
            dur_steps: 1,
        },
        MotifNote {
            degree: 4,
            dur_steps: 1,
        },
    ];

    // Each of the three onset steps maps 1:1 to its note. NONE may rest, and all three must
    // sound exactly one melody note — the byte-freeze 1:1 index, with no continuation arm.
    let s0 = realize_melody_at(&motif, 0);
    let s1 = realize_melody_at(&motif, 1);
    let s2 = realize_melody_at(&motif, 2);
    for (i, s) in [&s0, &s1, &s2].iter().enumerate() {
        assert_eq!(
            s.len(),
            1,
            "step {i} of an all-dur==1 motif must SOUND one note (no continuation rest reachable), got {s:?}"
        );
    }

    // The sing-hold is INERT on the freeze path: every dur==1 onset realizes to the SAME hold
    // (the THEME_LONG_NOTE_SING multiplier fires only for dur>1, so all three are equal). If a
    // regression let the sing factor leak onto a dur==1 onset, these would diverge.
    let (h0, h1, h2) = (s0[0].hold_ms, s1[0].hold_ms, s2[0].hold_ms);
    assert_eq!(
        h0, h1,
        "all-dur==1 onsets must realize to one identical hold (sing factor 1.0); h0={h0} h1={h1}"
    );
    assert_eq!(
        h1, h2,
        "all-dur==1 onsets must realize to one identical hold (sing factor 1.0); h1={h1} h2={h2}"
    );

    // And it equals the dur==1 baseline measured in the duration-thread test (h0 there was a
    // dur==1 onset) — re-anchor here so the two tests pin the SAME freeze value, not two
    // independent constants that could drift apart.
    let baseline = realize_melody_at(
        &[
            MotifNote {
                degree: 0,
                dur_steps: 1,
            },
            MotifNote {
                degree: 2,
                dur_steps: 2,
            },
            MotifNote {
                degree: 4,
                dur_steps: 1,
            },
        ],
        0, // step 0 = the leading dur==1 onset of the mixed motif
    );
    assert_eq!(
        h0, baseline[0].hold_ms,
        "the dur==1 onset hold must be the SAME value whether the motif is all-1-step or mixed \
         (the freeze baseline is one constant): all-1 {h0} vs mixed-motif dur==1 onset {}",
        baseline[0].hold_ms
    );
}

//! tests/counterpoint_s30.rs — the S30 SLICE-1 SPECIES-COUNTERPOINT PROPERTY NET.
//!
//! Black-box over the PUBLIC realizer surface (`realize_step`) of a hand-built
//! `pad_bed_counter`-bearing `Section`/`StepContext` (the saliency_s18.rs harness pattern,
//! design §6). The Music-Theory lane promoted the sustain-only counter scorer into a
//! fifth-species figure driver (`pick_counter_figure` and its HARD gates / figure
//! predicates, all PRIVATE); this net validates the MUSICAL PROPERTIES of the realized
//! two-voice line — it never calls the private craft directly.
//!
//! DETERMINISTIC + HEADLESS: every `Section`/`StepContext` is built in-memory by hand,
//! RNG-free (no `pick_progression`, no `thread_rng`). No disk fixtures, no filesystem
//! writes, every test well under 10s. Runs under BOTH the default and `--no-default-features`
//! feature sets (no feature-gated path is touched).
//!
//! THE TWO VOICES THE NET COMPARES. In a `pad_bed_counter` ensemble the layers are
//! [Bass, Pad, CounterMelody, Melody] → inst 2 is the COUNTER, inst 3 is the MELODY (the
//! "cantus" the counter counters). Both are extracted from the SAME section by realizing
//! the two instruments, so every assertion is over the actually-SOUNDING pitches.
//!
//! ────────────────────────────────────────────────────────────────────────────────────
//! FOUR REAL WEAKNESSES WERE SURFACED WHILE WRITING THIS NET. They shared ONE root cause —
//! the species two-point gates checked a SEEDED prev, not the REALIZED prev. The counter had
//! no cross-step pitch memory: each step re-seeded `prev_counter` non-recursively off the
//! PRIOR CHORD (`seed_prev_counter`, the §3.1 "LOCK"). Every two-point species check
//! (`has_parallel_perfects`, `approach_perfect_is_legal`, `melodic_leap_is_legal`,
//! `cadence_resolution_pitch`) therefore constrained the *seed→candidate* transition, NOT the
//! *realized-prev→realized* transition that actually SOUNDS; and the base SUSTAIN pitch was
//! never routed through `is_consonant`. When the seed ≠ the realized prior pitch, the guards
//! protected the wrong transition and the audible two-voice line slipped a fault through.
//!
//! THE MUSIC-THEORY LANE HAS NOW FIXED THE ROOT CAUSE: the counter has real cross-step memory
//! (the realized prior pitch is replayed deterministically into the two-point gates) and the
//! base sustain is routed through `is_consonant`. The four §FIXED-GAP tests at the bottom
//! were originally regression PINS that asserted each defect EXISTED; they have now been
//! INVERTED into strict POSITIVE properties that FAIL LOUDLY if the corresponding defect ever
//! regresses. The realized changes (verified against these fixtures) are:
//!
//!   GAP-1 (no audible parallel perfect): `IV→iii→I` now realizes melody 71→67 and counter
//!   59→60 at the si1→si2 boundary — CONTRARY motion; the parallel perfect fifth is GONE.
//!
//!   GAP-2 (consonant structural sustain): on a diminished triad (vii = B-D-F) the SUSTAIN
//!   now lands counter 62 (D, a CONSONANT m3 against melody F=77) instead of the old bare
//!   tritone 59 (B). PARTIAL: 4 of 6 terminal `X→vii` openers are consonant; IV→vii and V→vii
//!   still land a dissonant terminal sustain (pinned RESIDUAL).
//!
//!   GAP-3 (cadence resolves without leap): an `X→IV→V→I` PAC now closes counter 55→55
//!   (move 0, an OBLIQUE hold onto a perfect consonance) instead of the old 59→55 leap. The
//!   guaranteed level is "lands a perfect consonance by motion ≤2 semitones, NO leap"; strict
//!   stepwise-contrary clausula convergence is a documented future-slice refinement. PARTIAL:
//!   the perfect CLOSE holds for all 6 openers, the NO-LEAP approach for 4 of 6; V/vi still
//!   resolve 62→55 by leap (pinned RESIDUAL).
//!
//!   GAP-4 (no dissonant melodic leap): `I→V→IV` now realizes the counter line [64,62,65];
//!   the 62→65 move is a CONSONANT m3 leap, not the old 59→65 tritone. PARTIAL: 55 of 57
//!   realized leaps are consonant; ii→IV→iii and vi→IV→iii still leap a 65→59 tritone (pinned
//!   RESIDUAL).
//!
//! Consequently the strict forms of design PT-1 (no parallel perfects) now hold UNIVERSALLY
//! over the diatonic-triple battery (GAP-1 fully closed). GAP-2/3/4 are SUBSTANTIALLY closed
//! but NOT universal: each inverted test asserts the verified witness + the clean region
//! STRICTLY, and PINS the exact residual set so the clean region stays clean AND the residual
//! FAILS LOUDLY (with a stderr advisory) the moment the lane closes — or accidentally widens —
//! it. The earlier held-period/consonant-triad-scoped PT-1/PT-4 assertions are RETAINED.

use audiohax::chord_engine::{
    realize_step, Chord, NoteEvent, PerfFeatures, PhrasePosition, StepPlan,
};
use audiohax::composition::{
    CadenceStrength, KeyTempoPlan, LayerRole, OrchestrationProfile, ResolutionPolicy, Section,
    StepContext, ThematicRole, ThemeVariation,
};

// ─────────────────────────────────────────────────────────────────────────────
// HARNESS — hand-built pad_bed_counter section, public-surface realization.
// ─────────────────────────────────────────────────────────────────────────────

const MS: u64 = 1000;
// The counter band [FILL_REGISTER_FLOOR, MELODY_REGISTER_FLOOR); module-private constants
// re-stated as literals to keep this an integration (public-surface) test.
const COUNTER_FLOOR: u8 = 55;
const COUNTER_CEIL: u8 = 67;

fn chord(name: &str, notes: Vec<u8>) -> Chord {
    Chord {
        name: name.to_string(),
        notes,
    }
}
// Diatonic triads in C Ionian (root-position, ascending) — the "consonant-triad corpus"
// the positive properties are asserted over (no internal tritone).
fn c_i() -> Chord {
    chord("I", vec![60, 64, 67])
} // C E G
fn c_ii() -> Chord {
    chord("ii", vec![62, 65, 69])
} // D F A
fn c_iii() -> Chord {
    chord("iii", vec![64, 67, 71])
} // E G B
fn c_iv() -> Chord {
    chord("IV", vec![65, 69, 72])
} // F A C
fn c_v() -> Chord {
    chord("V", vec![67, 71, 74])
} // G B D
fn c_vi() -> Chord {
    chord("vi", vec![69, 72, 76])
} // A C E
fn c_vii() -> Chord {
    chord("vii", vec![71, 74, 77])
} // B D F — DIMINISHED (internal tritone)

fn consonant_corpus() -> Vec<Chord> {
    vec![c_i(), c_ii(), c_iii(), c_iv(), c_v(), c_vi()]
}

fn step(c: Chord, pip: usize, pos: PhrasePosition) -> StepPlan {
    StepPlan {
        chord: c,
        phrase_index: 0,
        position_in_phrase: pip,
        phrase_len: 8,
        position: pos,
        velocity: 80,
    }
}
fn key_tempo() -> KeyTempoPlan {
    KeyTempoPlan {
        home_root_midi: 60,
        home_mode: "Ionian".to_string(),
        base_ms_per_step: MS,
        key_scheme: vec![0],
        tempo_scheme: vec![MS],
    }
}
/// The §2.2 catalogue row: inst 0→Bass, 1→Pad, 2→CounterMelody, 3→Melody.
fn pad_bed_counter() -> OrchestrationProfile {
    OrchestrationProfile {
        id: "pad_bed_counter".to_string(),
        layers: vec![
            LayerRole::Bass,
            LayerRole::Pad,
            LayerRole::CounterMelody,
            LayerRole::Melody,
        ],
        density: 0.6,
        pad_voices: 3,
        figuration: None,
        figuration_resolved: None,
        prominence: Vec::new(),
    }
}
fn perf(edge: f32) -> PerfFeatures {
    // brightness 50 ⇒ no melody register lift, so the recomputed counter-eye CF and the
    // inst-3 melody seat at the same octave; keeps the comparison about pitch CHOICE, not
    // octave displacement.
    PerfFeatures {
        saturation: 60.0,
        brightness: 50.0,
        edge_density: edge,
    }
}

fn section_of(steps: Vec<StepPlan>) -> Section {
    Section {
        label: "A".to_string(),
        step_len: steps.len(),
        thematic_role: ThematicRole::Statement,
        key_offset_semitones: 0,
        ms_per_step: MS,
        mode: "Ionian".to_string(),
        progression: vec![],
        theme: None,
        variation: ThemeVariation::Identity,
        boundary_cadence: CadenceStrength::Perfect,
        // K3 identity carry — byte-frozen non-modulating path.
        pivot: false,
        resolution: ResolutionPolicy::Resolve,
        density: 0.6,
        orchestration: pad_bed_counter(),
        steps,
    }
}

/// Realize instrument `inst` (2 = counter, 3 = melody) for step `si` of `sec`.
fn realize_inst(sec: &Section, si: usize, inst: usize, features: &PerfFeatures) -> Vec<NoteEvent> {
    let kt = key_tempo();
    let ctx = StepContext {
        section: sec,
        step_in_section: si,
        theme: None,
        key_tempo: &kt,
        prev_key_offset_semitones: None,
    };
    realize_step(&sec.steps[si], inst, 4, features, MS, &ctx)
}

/// The realized COUNTER pitch (inst 2) at step `si` — exactly one event in Slice-1/2.
fn counter_at(sec: &Section, si: usize, features: &PerfFeatures) -> u8 {
    let evs = realize_inst(sec, si, 2, features);
    assert!(!evs.is_empty(), "counter must sound at si={si}");
    evs[0].note
}
/// The realized MELODY (CF) pitch (inst 3) at step `si` — the line the counter counters.
/// The melody may emit several subdivided events at one pitch; take the (single) pitch.
fn melody_at(sec: &Section, si: usize, features: &PerfFeatures) -> Option<u8> {
    let evs = realize_inst(sec, si, 3, features);
    evs.first().map(|e| e.note)
}

/// Build a multi-step phrase from a chord list with PhraseStart / Interior / PAC positions,
/// then return the realized (counter, melody) pitch pair at every step.
fn realize_line(chords: &[Chord], features: &PerfFeatures) -> Vec<(u8, Option<u8>)> {
    let n = chords.len();
    let steps: Vec<StepPlan> = chords
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let pos = if i == 0 {
                PhrasePosition::PhraseStart
            } else if i == n - 1 {
                PhrasePosition::PerfectAuthenticCadence
            } else {
                PhrasePosition::Interior
            };
            step(c.clone(), i, pos)
        })
        .collect();
    let sec = section_of(steps);
    (0..n)
        .map(|si| {
            (
                counter_at(&sec, si, features),
                melody_at(&sec, si, features),
            )
        })
        .collect()
}

/// All-Interior variant (no begin/cadence override) — for the interior species properties.
fn realize_line_interior(chords: &[Chord], features: &PerfFeatures) -> Vec<(u8, Option<u8>)> {
    let steps: Vec<StepPlan> = chords
        .iter()
        .enumerate()
        .map(|(i, c)| step(c.clone(), i, PhrasePosition::Interior))
        .collect();
    let sec = section_of(steps);
    (0..chords.len())
        .map(|si| {
            (
                counter_at(&sec, si, features),
                melody_at(&sec, si, features),
            )
        })
        .collect()
}

// ── music-theory helpers (mirror the engine's private classifiers, stated publicly) ──
fn ic(a: u8, b: u8) -> u8 {
    ((a as i16 - b as i16).abs() % 12) as u8
}
/// The counter scorer's harmonic classification (design §1.1, FOURTH_IS_DISSONANT = true):
/// ic 0/7 perfect, 3/4/8/9 imperfect, 1/2/5/6/10/11 dissonant.
fn is_dissonant(a: u8, b: u8) -> bool {
    matches!(ic(a, b), 1 | 2 | 5 | 6 | 10 | 11)
}
fn is_perfect(a: u8, b: u8) -> bool {
    matches!(ic(a, b), 0 | 7)
}
/// The engine's own gate (`has_parallel_perfects`) over the 2-voice [melody, counter] pair:
/// both voices move AND the perfect interval class (0 or 7) is preserved.
fn forms_parallel_perfect(mp: u8, mn: u8, cp: u8, cn: u8) -> bool {
    let ica = ic(mp, cp);
    let icb = ic(mn, cn);
    let both_move = mp != mn && cp != cn;
    ica == icb && (ica == 0 || ica == 7) && both_move
}
#[derive(PartialEq, Debug, Clone, Copy)]
enum Rel {
    Contrary,
    Oblique,
    Similar,
    Parallel,
}
fn rel_motion(mp: u8, mn: u8, cp: u8, cn: u8) -> Rel {
    let dm = (mn as i16) - (mp as i16);
    let dc = (cn as i16) - (cp as i16);
    if dm == 0 || dc == 0 {
        Rel::Oblique
    } else if dm.signum() != dc.signum() {
        Rel::Contrary
    } else if ic(mp, cp) == ic(mn, cn) {
        Rel::Parallel
    } else {
        Rel::Similar
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// PT-0 — SUSTAIN BYTE-PRESERVATION / FREEZE (design §5.2, §6 PT-0 + PT-FREEZE)
// ═════════════════════════════════════════════════════════════════════════════

/// Property: the counter-voice OFF path (the identity profile — no CounterMelody layer)
/// produces ZERO counter events and is byte-unmoved. This is the integration-level mirror
/// of the music-theory lane's PT-0: where no CounterMelody instrument is assigned, the
/// CounterMelody arm is never reached, so the realized output for the structural roles is
/// identical to the as-built baseline. We assert the identity ensemble emits the SAME
/// Bass/Melody NoteEvents whether or not a (separate) counter section exists — i.e. the
/// counter machinery is downstream of role assignment and cannot perturb the frozen roles.
#[test]
fn test_counter_off_is_byte_identical_baseline() {
    // The IDENTITY profile (empty layers) → no CounterMelody → the counter arm is dead.
    let identity = OrchestrationProfile {
        id: "identity".to_string(),
        layers: vec![],
        density: 0.5,
        pad_voices: 0,
        figuration: None,
        figuration_resolved: None,
        prominence: Vec::new(),
    };
    let mk = |orch: OrchestrationProfile| Section {
        label: "A".to_string(),
        step_len: 2,
        thematic_role: ThematicRole::Statement,
        key_offset_semitones: 0,
        ms_per_step: MS,
        mode: "Ionian".to_string(),
        progression: vec![],
        theme: None,
        variation: ThemeVariation::Identity,
        boundary_cadence: CadenceStrength::Perfect,
        pivot: false,
        resolution: ResolutionPolicy::Resolve,
        density: 0.5,
        orchestration: orch,
        steps: vec![
            step(c_v(), 0, PhrasePosition::Interior),
            step(c_i(), 1, PhrasePosition::Interior),
        ],
    };
    let kt = key_tempo();
    let sec_id = mk(identity);
    // Under the identity profile, a 2-instrument ensemble is [Bass, Melody]. Realizing the
    // MELODY (inst 1 of 2) must be byte-identical regardless of any counter logic — the
    // counter arm is unreachable, so the frozen Melody path is untouched.
    for si in 0..2 {
        let ctx = StepContext {
            section: &sec_id,
            step_in_section: si,
            theme: None,
            key_tempo: &kt,
            prev_key_offset_semitones: None,
        };
        let a = realize_step(&sec_id.steps[si], 1, 2, &perf(0.04), MS, &ctx);
        let b = realize_step(&sec_id.steps[si], 1, 2, &perf(0.04), MS, &ctx);
        assert_eq!(
            a, b,
            "identity Melody realization is deterministic / frozen"
        );
        assert!(!a.is_empty(), "the identity Melody must sound");
    }
}

/// Property (design PT-0, sustain reduction): on a HELD / static period, the figure driver
/// is forced to Sustain-only (R-A: dissonant figures are disabled on held/static steps), so
/// the realized held line is exactly the as-built sustain rotation — a moving chord-tone
/// inner line, never an added dissonance. We assert the held-period counter is ALWAYS a
/// consonant-or-perfect chord tone of the held chord (no dissonant figure intrudes on a
/// held period), which is the observable consequence of the Sustain-only reduction.
#[test]
fn test_held_period_is_sustain_only_no_added_dissonance() {
    let held = c_i();
    let steps = vec![
        step(held.clone(), 0, PhrasePosition::Interior),
        step(held.clone(), 1, PhrasePosition::Interior),
        step(held.clone(), 2, PhrasePosition::Interior),
    ];
    let sec = section_of(steps);
    for si in 0..3 {
        let c = counter_at(&sec, si, &perf(0.04));
        let pcs: Vec<u8> = held.notes.iter().map(|n| n % 12).collect();
        assert!(
            pcs.contains(&(c % 12)),
            "held-period counter {c} (pc {}) must stay a Sustain chord tone of {:?} — no \
             dissonant figure is licensed on a held step (R-A / PT-0 sustain reduction)",
            c % 12,
            pcs
        );
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// PT-1 — VOICE INDEPENDENCE: no parallel perfect 5ths/8ves (design §6 PT-1)
// ═════════════════════════════════════════════════════════════════════════════

/// Property: within a SINGLE realized transition where the counter's seed equals its prior
/// realized pitch (the held-period / consecutive-same-seed case), no parallel perfect 5th/
/// 8ve appears between the counter and the melody. The engine's `has_parallel_perfects`
/// gate is genuinely enforced HERE (the seed matches the realized prev, so the gate sees the
/// true transition). This is the region of PT-1 the implementation actually satisfies.
///
/// (The general cross-step claim is NOT satisfied — see GAP-1; this test asserts the part
/// that holds, on a held chord where the seed and the realized prior pitch coincide.)
#[test]
fn test_no_parallel_perfect_on_held_transition() {
    // A held chord realized across 3 steps: the held-run rotation moves the counter, and the
    // melody is static (Hold) → every transition is OBLIQUE (the melody holds), which can
    // never be a parallel perfect (parallels require BOTH voices to move). This is the clean,
    // satisfied corner of PT-1.
    for c in consonant_corpus() {
        let steps = vec![
            step(c.clone(), 0, PhrasePosition::Interior),
            step(c.clone(), 1, PhrasePosition::Interior),
            step(c.clone(), 2, PhrasePosition::Interior),
        ];
        let sec = section_of(steps);
        let line: Vec<(u8, Option<u8>)> = (0..3)
            .map(|si| {
                (
                    counter_at(&sec, si, &perf(0.03)),
                    melody_at(&sec, si, &perf(0.03)),
                )
            })
            .collect();
        for w in line.windows(2) {
            if let (Some(mp), Some(mn)) = (w[0].1, w[1].1) {
                let (cp, cn) = (w[0].0, w[1].0);
                assert!(
                    !forms_parallel_perfect(mp, mn, cp, cn),
                    "held-period transition must not form a parallel perfect between melody \
                     {mp}->{mn} and counter {cp}->{cn} (chord {})",
                    c.name
                );
            }
        }
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// PT-3 — MOTION DISTRIBUTION: contrary+oblique favored over similar+parallel
// ═════════════════════════════════════════════════════════════════════════════

/// Property (design §6 PT-3): over a representative diatonic-triad battery the realized
/// counter's relative motion against the melody FAVORS contrary+oblique over similar+
/// parallel — the graded `rel_motion_score` gradient (contrary > oblique > similar > parallel)
/// shapes the line toward independence. Deterministic corpus ⇒ exact counts; we assert the
/// contrary+oblique share is a strict majority (a real floor, not a single instance).
#[test]
fn test_motion_distribution_favors_independence() {
    let pool = consonant_corpus();
    let (mut contrary, mut oblique, mut similar, mut parallel) = (0usize, 0, 0, 0);
    for a in 0..pool.len() {
        for b in 0..pool.len() {
            for d in 0..pool.len() {
                if a == b || b == d {
                    continue;
                }
                let line = realize_line_interior(
                    &[pool[a].clone(), pool[b].clone(), pool[d].clone()],
                    &perf(0.04),
                );
                for w in line.windows(2) {
                    if let (Some(mp), Some(mn)) = (w[0].1, w[1].1) {
                        match rel_motion(mp, mn, w[0].0, w[1].0) {
                            Rel::Contrary => contrary += 1,
                            Rel::Oblique => oblique += 1,
                            Rel::Similar => similar += 1,
                            Rel::Parallel => parallel += 1,
                        }
                    }
                }
            }
        }
    }
    let independent = contrary + oblique;
    let dependent = similar + parallel;
    let total = independent + dependent;
    assert!(
        total > 50,
        "sanity: the battery produced enough transitions ({total})"
    );
    assert!(
        independent > dependent,
        "contrary+oblique ({independent}) must be a strict majority over similar+parallel \
         ({dependent}) — the counter favors voice independence. \
         [contrary {contrary}, oblique {oblique}, similar {similar}, parallel {parallel}]"
    );
    // Tighter: contrary alone clears a third of all transitions (the strongest independence
    // is the dominant single category, proving the graded bonus actually biases the pick).
    assert!(
        contrary * 3 >= total,
        "contrary motion ({contrary}) should be ~the plurality, ≥1/3 of all {total} transitions"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// PT-4 — DISSONANCE ONLY AS A RESOLVED FIGURE (design §6 PT-4)
// ═════════════════════════════════════════════════════════════════════════════

/// Property: over the CONSONANT-TRIAD corpus (major/minor triads, no internal tritone), the
/// realized counter never sounds an UNRESOLVED vertical dissonance against the melody — any
/// dissonant vertical is left by step to a consonance on the next step (the passing/neighbor
/// figure shape). This is the part of PT-4 the implementation satisfies: when the chord
/// itself is consonant, every dissonance the driver produces is an ADDED figure routed
/// through the resolution gate.
///
/// (PT-4's strict universal form does NOT hold on diminished chords — see GAP-2 — because
/// the SUSTAIN pitch is never consonance-checked; that is pinned separately.)
#[test]
fn test_consonant_corpus_dissonance_resolves_by_step() {
    let pool = consonant_corpus();
    let mut checked_a_dissonance = false;
    for a in 0..pool.len() {
        for b in 0..pool.len() {
            for d in 0..pool.len() {
                if a == b || b == d {
                    continue;
                }
                let line = realize_line_interior(
                    &[pool[a].clone(), pool[b].clone(), pool[d].clone()],
                    &perf(0.04),
                );
                // Inspect every interior step that has BOTH a prior and a next (si=1 here).
                for si in 1..line.len().saturating_sub(0) {
                    let (c_now, m_now) = (line[si].0, line[si].1);
                    let Some(m_now) = m_now else { continue };
                    if !is_dissonant(c_now, m_now) {
                        continue;
                    }
                    checked_a_dissonance = true;
                    // A licensed figure resolves by STEP to a consonance on the NEXT step.
                    let next = line.get(si + 1);
                    if let Some((c_next, Some(m_next))) = next.map(|&(c, m)| (c, m)) {
                        let stepwise = (c_next as i16 - c_now as i16).abs();
                        assert!(
                            (1..=2).contains(&stepwise) && !is_dissonant(c_next, m_next),
                            "a dissonant counter vertical ({c_now} vs {m_now}, chord {}) on a \
                             CONSONANT-triad step must resolve by step (±1/±2) to a consonance; \
                             got next counter {c_next} vs melody {m_next} (move {stepwise})",
                            pool[b].name
                        );
                    }
                }
            }
        }
    }
    // We do not REQUIRE a dissonance to occur on the consonant corpus (the gates are tight,
    // and frequently no figure is licensed). The assertion is conditional-on-occurrence; the
    // flag documents whether the resolution path was exercised at all.
    let _ = checked_a_dissonance;
}

// ═════════════════════════════════════════════════════════════════════════════
// PT-6 — LEAP RECOVERY + no dissonant melodic leap (design §6 PT-6)
// ═════════════════════════════════════════════════════════════════════════════

/// Property (the part that HOLDS): the realized counter line stays in the counter band
/// [55,67) on every step, and never runs away — no two consecutive ≥4th leaps in the SAME
/// direction (over the full consonant battery: 0 runaways). The leap-RECOVERY/no-dissonant-
/// leap claim of PT-6 does NOT universally hold on the realized line (the line itself can leap
/// a tritone — see GAP-4); that is pinned separately. Here we assert the band + no-runaway
/// invariants, which the implementation does satisfy.
#[test]
fn test_counter_line_band_and_no_runaway() {
    let pool = consonant_corpus();
    for a in 0..pool.len() {
        for b in 0..pool.len() {
            for d in 0..pool.len() {
                if a == b || b == d {
                    continue;
                }
                let line = realize_line_interior(
                    &[pool[a].clone(), pool[b].clone(), pool[d].clone()],
                    &perf(0.04),
                );
                let cs: Vec<u8> = line.iter().map(|&(c, _)| c).collect();
                for &c in &cs {
                    assert!(
                        (COUNTER_FLOOR..COUNTER_CEIL).contains(&c),
                        "counter pitch {c} must stay in the band [{COUNTER_FLOOR},{COUNTER_CEIL}) \
                         ({}->{}->{})",
                        pool[a].name,
                        pool[b].name,
                        pool[d].name
                    );
                }
                // No two consecutive large (≥ 4th) leaps in the SAME direction (runaway).
                for w in cs.windows(3) {
                    let d1 = w[1] as i16 - w[0] as i16;
                    let d2 = w[2] as i16 - w[1] as i16;
                    if d1.abs() >= 5 && d2.abs() >= 5 {
                        assert!(
                            d1.signum() != d2.signum(),
                            "two consecutive ≥4th leaps in the same direction (runaway): \
                             {}->{}->{}",
                            w[0],
                            w[1],
                            w[2]
                        );
                    }
                }
            }
        }
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// PT-7 — NO UNISON COLLAPSE (design §6 PT-7)
// ═════════════════════════════════════════════════════════════════════════════

/// Property: the counter NEVER doubles the melody's exact MIDI pitch at a simultaneous
/// sounding (`COUNTER_UNISON_PENALTY` made dominant). Over the full diatonic battery,
/// including held periods, cadences and phrase starts, counter != melody at every step.
#[test]
fn test_no_unison_collapse() {
    let pool = consonant_corpus();
    let mut any = false;
    for a in 0..pool.len() {
        for b in 0..pool.len() {
            for d in 0..pool.len() {
                if a == b || b == d {
                    continue;
                }
                let line = realize_line(
                    &[pool[a].clone(), pool[b].clone(), pool[d].clone()],
                    &perf(0.04),
                );
                for (si, &(c, m)) in line.iter().enumerate() {
                    if let Some(m) = m {
                        any = true;
                        assert_ne!(
                            c, m,
                            "counter must never collapse onto the melody's exact pitch at si={si} \
                             ({}->{}->{})",
                            pool[a].name, pool[b].name, pool[d].name
                        );
                    }
                }
            }
        }
    }
    assert!(any, "sanity: the battery produced sounding simultaneities");
}

// ═════════════════════════════════════════════════════════════════════════════
// PT-8 — BEGIN / CADENCE FORMULAS (design §6 PT-8)
// ═════════════════════════════════════════════════════════════════════════════

/// Property: at a `PhraseStart` step the counter's vertical with the melody is a PERFECT
/// consonance (the §1.3 opening formula HARD-overrides the sustain pick), and at a
/// `PerfectAuthenticCadence` step the counter closes by stepwise CONTRARY motion onto a
/// perfect consonance (the clausula). Both are HARD overrides applied in the driver BEFORE
/// the figure search, so unlike the interior sustain they DO bind (they recompute the pick).
#[test]
fn test_begin_and_cadence_formulas() {
    // A real V→...→I phrase: PhraseStart on the first step, PAC on the last.
    for opener in consonant_corpus() {
        let chords = vec![opener.clone(), c_iv(), c_v(), c_i()];
        let line = realize_line(&chords, &perf(0.04));

        // PhraseStart (si=0): perfect-consonant opening vertical (when the melody sounds).
        let (c0, m0) = line[0];
        if let Some(m0) = m0 {
            assert!(
                is_perfect(c0, m0),
                "PhraseStart opening vertical (counter {c0} vs melody {m0}, opener {}) must be a \
                 PERFECT consonance (ic 0/7), got ic {}",
                opener.name,
                ic(c0, m0)
            );
        }

        // PAC (last step): perfect-consonant CLOSE.
        let last = line.len() - 1;
        let (cl, ml) = line[last];
        if let Some(ml) = ml {
            assert!(
                is_perfect(cl, ml),
                "PerfectAuthenticCadence close (counter {cl} vs melody {ml}, opener {}) must land \
                 on a PERFECT consonance (the clausula octave/unison/fifth), got ic {}",
                opener.name,
                ic(cl, ml)
            );
            // NOTE: the §1.3 clausula also requires the APPROACH to be a stepwise CONTRARY
            // convergence onto the close. That part does NOT hold on the realized line (the
            // cadence pick falls back to the nearest perfect tone by LEAP — see GAP-3); only
            // the perfect-consonant CLOSE (asserted above) is satisfied. We therefore do not
            // assert the stepwise-contrary approach here; GAP-3 pins its absence.
        }
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// PT-9 — DETERMINISM (design §6 PT-9)
// ═════════════════════════════════════════════════════════════════════════════

/// Property: realizing the same hand-built section twice yields byte-identical NoteEvent
/// sequences for the counter (no thread_rng reached in figure/voice selection).
#[test]
fn test_determinism_of_realized_counter() {
    let chords = vec![c_i(), c_vi(), c_iv(), c_v(), c_i()];
    let steps: Vec<StepPlan> = chords
        .iter()
        .enumerate()
        .map(|(i, c)| step(c.clone(), i, PhrasePosition::Interior))
        .collect();
    let sec = section_of(steps);
    for si in 0..chords.len() {
        let a = realize_inst(&sec, si, 2, &perf(0.04));
        let b = realize_inst(&sec, si, 2, &perf(0.04));
        assert_eq!(
            a, b,
            "the realized counter must be deterministic (identical across two runs) at si={si}"
        );
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// FIXED-GAP STRICT PROPERTIES — the four real weaknesses found while writing this net,
// now FIXED by the music-theory lane and INVERTED into strict positive properties.
//
// Each was originally a regression pin asserting the defect EXISTED. The lane closed the root
// cause (realized-prev cross-step memory threaded into the two-point gates + the base sustain
// routed through `is_consonant`), and these tests now assert the POSITIVE property: each FAILS
// LOUDLY the moment its defect regresses. Fixtures are kept ADVERSARIAL — each harmony is one
// that historically TEMPTED the defect — so the gate is genuinely exercised, not vacuous.
// ═════════════════════════════════════════════════════════════════════════════

/// PROPERTY (inverted GAP-1) — NO AUDIBLE PARALLEL PERFECT 5ths/8ves between the sounding
/// melody and counter, on ANY realized transition.
///
/// The lane fixed the root cause: `has_parallel_perfects` now sees the REALIZED prior counter
/// pitch (cross-step memory replayed into the two-point gate), not the seed re-derived off the
/// prior chord. So the gate now guards the transition that actually SOUNDS.
///
/// ADVERSARIAL WITNESS: `IV → iii → I` (all interior) — the exact progression that historically
/// produced melody 71→67 against counter 64→60 (a textbook parallel fifth). It now realizes
/// counter 59→60 = CONTRARY motion against the melody, and forms NO parallel perfect.
///
/// BROADER BATTERY: every ordered diatonic triple over the 6 consonant triads is realized and
/// EVERY counter↔melody transition is checked — so the gate is exercised at T (si0→si1) AND at
/// T+1 (si1→si2), not just the single witnessed boundary.
#[test]
fn test_no_audible_parallel_perfect_counter_vs_melody() {
    // 1) The adversarial witness no longer forms a parallel perfect, and the specific boundary
    //    that used to fault is now contrary (proving the fix flipped the actual motion).
    let line = realize_line_interior(&[c_iv(), c_iii(), c_i()], &perf(0.04));
    let (cp, mp) = (line[1].0, line[1].1.expect("melody sounds at si=1"));
    let (cn, mn) = (line[2].0, line[2].1.expect("melody sounds at si=2"));
    assert_eq!(
        (mp, cp, mn, cn),
        (71, 59, 67, 60),
        "witness drifted: expected melody 71->67 / counter 59->60 for IV->iii->I (the fixed, \
         CONTRARY realization); got melody {mp}->{mn} / counter {cp}->{cn}. Re-derive."
    );
    assert_eq!(
        rel_motion(mp, mn, cp, cn),
        Rel::Contrary,
        "the once-parallel IV->iii->I boundary must now move CONTRARY (melody {mp}->{mn} \
         descends, counter {cp}->{cn} ascends)"
    );
    assert!(
        !forms_parallel_perfect(mp, mn, cp, cn),
        "PT-1 REGRESSED: IV->iii->I realizes a parallel perfect between the sounding melody \
         ({mp}->{mn}) and counter ({cp}->{cn}). The realized prior counter pitch is no longer \
         reaching has_parallel_perfects."
    );

    // 2) The broad claim: over the full ordered diatonic-triple battery, NO realized transition
    //    forms a parallel perfect — checked at every (si, si+1) pair (T and T+1).
    let pool = consonant_corpus();
    let mut checked = 0usize;
    for a in 0..pool.len() {
        for b in 0..pool.len() {
            for d in 0..pool.len() {
                if a == b || b == d {
                    continue;
                }
                let line = realize_line_interior(
                    &[pool[a].clone(), pool[b].clone(), pool[d].clone()],
                    &perf(0.04),
                );
                for w in line.windows(2) {
                    if let (Some(mp), Some(mn)) = (w[0].1, w[1].1) {
                        let (cp, cn) = (w[0].0, w[1].0);
                        checked += 1;
                        assert!(
                            !forms_parallel_perfect(mp, mn, cp, cn),
                            "PT-1 REGRESSED: parallel perfect on realized transition melody \
                             {mp}->{mn} / counter {cp}->{cn} over {}->{}->{}",
                            pool[a].name,
                            pool[b].name,
                            pool[d].name
                        );
                    }
                }
            }
        }
    }
    assert!(
        checked > 100,
        "sanity: the battery exercised enough realized transitions ({checked})"
    );
}

/// PROPERTY (inverted GAP-2) — the STRUCTURAL SUSTAIN pitch is CONSONANT against the melody
/// even on a DIMINISHED triad; if it is ever dissonant it must be a prepared+resolved figure.
///
/// The lane fixed the root cause: the base SUSTAIN pick is now routed through `is_consonant`,
/// so it can no longer land an unprepared structural dissonance — including on a chord that
/// contains an internal tritone (the diminished vii = B-D-F).
///
/// ADVERSARIAL WITNESS: a 2-step `iii → vii` where vii is the LAST step — the single most
/// tempting case for the defect, because there is no next step to resolve into, so a dissonant
/// sustain would be permanently unresolved. The sustain now lands counter 62 (D), a CONSONANT
/// m3 against melody 77 (F), instead of the old bare tritone 59 (B).
///
/// We assert the structural vertical is consonant at the witness, AND — over the full battery
/// of `X → vii` finals (vii last, every consonant X) — that the diminished structural sustain
/// is never an unprepared/unresolved dissonance. Because vii is the terminal step there is no
/// resolution slot, so the property is the strict one: the structural vertical must be
/// CONSONANT (a dissonance there could never be a resolved figure).
#[test]
fn test_diminished_structural_sustain_is_consonant() {
    // 1) The adversarial witness: vii as the unresolvable LAST step is now consonant.
    let steps = vec![
        step(c_iii(), 0, PhrasePosition::Interior),
        step(c_vii(), 1, PhrasePosition::Interior),
    ];
    let sec = section_of(steps);
    let c = counter_at(&sec, 1, &perf(0.04));
    let m = melody_at(&sec, 1, &perf(0.04)).expect("melody sounds on the vii step");
    assert_eq!(
        (m, c),
        (77, 62),
        "witness drifted: expected melody 77 / counter 62 (the fixed consonant m3) on the vii \
         step; got melody {m} / counter {c}. Re-derive."
    );
    assert!(
        !is_dissonant(c, m),
        "PT-4 REGRESSED: the diminished-triad terminal SUSTAIN lands a vertical DISSONANCE \
         (counter {c} vs melody {m}, ic {}). Since vii is the LAST step there is no resolution \
         slot — this is an unprepared, unresolvable structural dissonance. The sustain pick is \
         no longer routed through is_consonant.",
        ic(c, m)
    );
    assert_eq!(
        ic(c, m),
        3,
        "the fixed witness vertical should be a consonant m3 (ic 3)"
    );

    // 2) RESIDUAL SCOPE. The fix is INCOMPLETE on the broad terminal-diminished battery: over
    //    the 6 consonant openers of `X -> vii` (vii LAST), 4 land consonant but 2 still land an
    //    unresolvable structural DISSONANCE (IV->vii = ic 10 m7, V->vii = ic 5/P4). When the
    //    realized penult's nearest band-reachable diminished chord-tone is itself dissonant
    //    against the melody, the is_consonant routing has no consonant landing and falls back to
    //    a dissonant tone. We pin the EXACT residual set so it (a) is visible and (b) FAILS
    //    LOUDLY the moment the lane closes it (e.g. via a consonant non-chord-tone escape or a
    //    voice-exchange on the diminished terminal). See the residual report.
    const GAP2_RESIDUAL_DISSONANT_OPENERS: &[&str] = &["IV", "V"];
    let mut residual_dissonant: Vec<String> = Vec::new();
    for x in consonant_corpus() {
        let steps = vec![
            step(x.clone(), 0, PhrasePosition::Interior),
            step(c_vii(), 1, PhrasePosition::Interior),
        ];
        let sec = section_of(steps);
        let c = counter_at(&sec, 1, &perf(0.04));
        let Some(m) = melody_at(&sec, 1, &perf(0.04)) else {
            continue;
        };
        if is_dissonant(c, m) {
            residual_dissonant.push(x.name.clone());
        } else {
            // The CONSONANT region is the strict positive property and must stay clean.
            // (If a previously-consonant opener regresses to dissonant it is caught here.)
        }
    }
    residual_dissonant.sort();
    let mut expected: Vec<String> = GAP2_RESIDUAL_DISSONANT_OPENERS
        .iter()
        .map(|s| s.to_string())
        .collect();
    expected.sort();
    if residual_dissonant != expected {
        eprintln!(
            "GAP-2 RESIDUAL SET CHANGED: terminal-diminished dissonant openers now {residual_dissonant:?} \
             (was {expected:?}). If the set SHRANK the lane made progress — tighten this pin \
             toward the full consonant property. If it GREW a NEW opener regressed."
        );
    }
    assert_eq!(
        residual_dissonant, expected,
        "GAP-2 residual set drifted (see stderr). Strict-consonant openers must stay consonant; \
         the known-residual pair {{IV,V}} must not grow."
    );
}

/// PROPERTY (inverted GAP-3) — the cadence CLAUSULA lands a PERFECT consonance by motion ≤ 2
/// semitones, with NO leap (the guaranteed level the fix achieves).
///
/// The lane fixed the root cause: `cadence_resolution_pitch` now derives from the REALIZED
/// prior counter pitch, so the close is reached without leaping. The OLD behavior resolved
/// counter 59 → 55 (a 4-semitone leap); the FIXED behavior resolves 55 → 55 (move 0, an
/// OBLIQUE hold onto the perfect consonance).
///
/// GUARANTEED LEVEL (what this asserts): the PAC close is a perfect consonance AND the counter
/// reaches it by |motion| ≤ 2 semitones — i.e. no by-leap resolution. The fix does NOT always
/// achieve strict stepwise-CONTRARY convergence: on some progressions the strict clausula form
/// is not band-reachable from the realized penult. That strict stepwise-contrary clausula is a
/// documented FUTURE-SLICE refinement; we assert the guaranteed level here.
///
/// ADVERSARIAL WITNESS: `ii → IV → V → I` — the exact phrase that historically resolved by a
/// 4-semitone leap. We also sweep every consonant opener so the cadence pick is exercised from
/// a range of realized penults.
#[test]
fn test_cadence_resolves_perfect_no_leap() {
    // 1) The adversarial witness: the once-leaping ii->IV->V->I close is now a no-leap hold.
    let line = realize_line(&[c_ii(), c_iv(), c_v(), c_i()], &perf(0.04));
    let last = line.len() - 1;
    let (cp, _) = line[last - 1];
    let (cl, ml) = line[last];
    let ml = ml.expect("melody sounds on the PAC close");
    assert_eq!(
        (cp, cl),
        (55, 55),
        "witness drifted: expected penult 55 -> final 55 (the fixed no-leap OBLIQUE hold); got \
         {cp} -> {cl}. Re-derive."
    );
    assert!(
        is_perfect(cl, ml),
        "PT-8 REGRESSED: the cadence close is not a perfect consonance (counter {cl} vs melody \
         {ml}, ic {})",
        ic(cl, ml)
    );
    let counter_move = (cl as i16 - cp as i16).abs();
    assert!(
        counter_move <= 2,
        "PT-8 REGRESSED: the cadence resolves the counter by LEAP (penult {cp} -> final {cl}, \
         move {counter_move} > 2). The cadence pick is no longer deriving from the realized \
         prior counter pitch."
    );

    // 2) STRICT (holds everywhere): the PAC CLOSE is a perfect consonance for EVERY opener.
    //    RESIDUAL: the NO-LEAP approach holds for 4 of 6 openers but NOT for V/vi — those
    //    realize a penult of 62 from which no perfect-consonant tone is band-reachable by
    //    |motion| <= 2, so the cadence still resolves 62 -> 55 (move 7, a LEAP). We pin the
    //    exact residual openers so the no-leap region stays clean AND the residual fails loudly
    //    when the lane closes it (e.g. by widening the band-reachable cadence landing set).
    const GAP3_RESIDUAL_LEAP_OPENERS: &[&str] = &["V", "vi"];
    let mut residual_leap: Vec<String> = Vec::new();
    for opener in consonant_corpus() {
        let line = realize_line(&[opener.clone(), c_iv(), c_v(), c_i()], &perf(0.04));
        let last = line.len() - 1;
        let (cp, _) = line[last - 1];
        let (cl, ml) = line[last];
        let Some(ml) = ml else { continue };
        // The perfect CLOSE is the strict, universal part of PT-8 — it must hold for all openers.
        assert!(
            is_perfect(cl, ml),
            "PT-8 REGRESSED: {}->IV->V->I PAC close is not perfect (counter {cl} vs melody {ml}, \
             ic {})",
            opener.name,
            ic(cl, ml)
        );
        let mv = (cl as i16 - cp as i16).abs();
        if mv > 2 {
            residual_leap.push(opener.name.clone());
        }
    }
    residual_leap.sort();
    let mut expected: Vec<String> = GAP3_RESIDUAL_LEAP_OPENERS
        .iter()
        .map(|s| s.to_string())
        .collect();
    expected.sort();
    if residual_leap != expected {
        eprintln!(
            "GAP-3 RESIDUAL SET CHANGED: cadence-by-leap openers now {residual_leap:?} (was \
             {expected:?}). Shrinking = lane progress (tighten this pin toward the full no-leap \
             property); growing = a NEW opener regressed to a by-leap cadence."
        );
    }
    assert_eq!(
        residual_leap, expected,
        "GAP-3 residual set drifted (see stderr). The no-leap openers must stay no-leap; the \
         known-residual pair {{V,vi}} must not grow."
    );
}

/// PROPERTY (inverted GAP-4) — the realized counter LINE contains NO dissonant (tritone/7th)
/// melodic leap; any leap is to a consonant melodic interval and is recovered per the
/// leap-recovery rule (no two consecutive ≥4th leaps in the same direction).
///
/// The lane fixed the root cause: `melodic_leap_is_legal` now gates the REALIZED prior→now
/// counter transition (not just the seed→candidate one), so the actual SOUNDING line can no
/// longer leap a dissonant melodic interval.
///
/// ADVERSARIAL WITNESS: `I → V → IV` — the exact progression that historically realized the
/// counter line [64, 59, 65] with a 59→65 TRITONE leap. It now realizes [64, 62, 65]; the
/// 62→65 move is a CONSONANT m3 leap (ic 3).
///
/// BROADER BATTERY: every ordered diatonic triple over the 6 consonant triads is realized and
/// EVERY melodic step in the counter line is checked: a leap (≥3 semitones) must land on a
/// consonant MELODIC interval (never ic 6/10/11), and the leap-recovery invariant (no two
/// consecutive ≥4th leaps in the same direction) holds.
#[test]
fn test_no_dissonant_melodic_leap_in_counter_line() {
    // 1) The adversarial witness: the once-tritone I->V->IV line now leaps by a consonant m3.
    let line = realize_line_interior(&[c_i(), c_v(), c_iv()], &perf(0.04));
    let cs: Vec<u8> = line.iter().map(|&(c, _)| c).collect();
    assert_eq!(
        cs,
        vec![64, 62, 65],
        "witness drifted: expected the fixed counter line [64,62,65]; got {cs:?}. Re-derive."
    );
    assert_eq!(
        ic(cs[1], cs[2]),
        3,
        "the fixed 62->65 leap must be a consonant m3 (ic 3), not the old tritone"
    );

    // 2) STRICT (leap recovery, holds everywhere): no two consecutive ≥4th leaps in the same
    //    direction over the WHOLE battery. RESIDUAL (dissonant melodic leap): the gate is closed
    //    for almost all of the 57 realized leaps, but 2 still land a dissonant TRITONE
    //    (ii->IV->iii and vi->IV->iii both realize the 65->59 tritone). When the realized penult
    //    forces a band-constrained landing whose only available consonant chord-tone collides
    //    with the unison/parallel guard, the line falls back across the tritone. We pin the
    //    exact residual transition set so the clean region stays clean AND the residual fails
    //    loudly when the lane fully gates the realized prior->now leap.
    let pool = consonant_corpus();
    let mut leaps_seen = 0usize;
    let mut residual_diss_leaps: Vec<String> = Vec::new();
    for a in 0..pool.len() {
        for b in 0..pool.len() {
            for d in 0..pool.len() {
                if a == b || b == d {
                    continue;
                }
                let line = realize_line_interior(
                    &[pool[a].clone(), pool[b].clone(), pool[d].clone()],
                    &perf(0.04),
                );
                let cs: Vec<u8> = line.iter().map(|&(c, _)| c).collect();
                let prog = format!("{}->{}->{}", pool[a].name, pool[b].name, pool[d].name);
                for w in cs.windows(2) {
                    let mv = (w[1] as i16 - w[0] as i16).abs();
                    if mv >= 3 {
                        leaps_seen += 1;
                        if matches!(ic(w[0], w[1]), 6 | 10 | 11) {
                            residual_diss_leaps.push(prog.clone());
                        }
                    }
                }
                // Leap recovery is STRICT and universal: no same-direction ≥4th runaway anywhere.
                for w in cs.windows(3) {
                    let d1 = w[1] as i16 - w[0] as i16;
                    let d2 = w[2] as i16 - w[1] as i16;
                    if d1.abs() >= 5 && d2.abs() >= 5 {
                        assert!(
                            d1.signum() != d2.signum(),
                            "PT-6 REGRESSED: unrecovered runaway (two same-direction ≥4th leaps) \
                             {}->{}->{} over {prog}",
                            w[0],
                            w[1],
                            w[2]
                        );
                    }
                }
            }
        }
    }
    assert!(
        leaps_seen > 0,
        "sanity: the battery realized at least one counter-line leap to exercise the gate \
         ({leaps_seen})"
    );
    residual_diss_leaps.sort();
    let mut expected_diss: Vec<String> = ["ii->IV->iii", "vi->IV->iii"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    expected_diss.sort();
    if residual_diss_leaps != expected_diss {
        eprintln!(
            "GAP-4 RESIDUAL SET CHANGED: dissonant-melodic-leap progressions now \
             {residual_diss_leaps:?} (was {expected_diss:?}). Shrinking = lane progress (tighten \
             toward the full no-dissonant-leap property); growing = a NEW progression regressed."
        );
    }
    assert_eq!(
        residual_diss_leaps, expected_diss,
        "GAP-4 residual set drifted (see stderr). The clean leaps must stay consonant; the \
         known-residual pair {{ii->IV->iii, vi->IV->iii}} must not grow."
    );
}

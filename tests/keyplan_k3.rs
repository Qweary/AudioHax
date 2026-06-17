//! tests/keyplan_k3.rs — the SLICE-K3 realizer PIVOT / LAND-HOME witness net.
//!
//! K3 adds two surgical realizer interventions, each confined to a single step and each INERT on
//! the identity / home-only / non-modulating path (see `docs/spec-s28-k3-build.md` §3 and
//! `docs/input-s28-k3-pivot-harmony.md`):
//!
//!   1. THE PIVOT — at the FIRST step of a section whose key differs from its predecessor, on a
//!      `pivot:true` (non-`home_only`) scheme, the realizer sounds the DOMINANT of the destination
//!      key (V of the destination) in place of the section's own first chord, so a direct
//!      modulation reads as a prepared hinge instead of a tape splice.
//!   2. THE LAND-HOME CADENCE — at the final section's already-stamped Perfect Authentic Cadence
//!      step, when the form is a returning (Resolve) `pivot:true` form AT home (offset 0), the
//!      cadence's VOICING is strengthened into an explicit root-position V→I in the HOME key with
//!      the home tonic on top (the PAC marker). It adds NO event, moves no boundary.
//!
//! HEADLESS + RNG-FREE, in the same sense as keyplan_s25.rs / engine_equivalence.rs: NO image type,
//! NO OpenCV, NO audio hardware, NO `pick_progression`/`thread_rng`-derived assertions. Every
//! fixture is a hand-built `Section` + `StepContext` over a fixed C-major chord, and the pivot /
//! land-home behaviour is exercised through the PUBLIC pure realizer
//! `chord_engine::realize_step(step, inst_idx, num_instruments, features, ms_per_step, ctx)`. The
//! K3 realizer fns themselves (`pivot_chord_events`, `land_home_is_armed`, `land_home_pitch`) are
//! module-PRIVATE, so — exactly like the surrounding nets — this integration file CANNOT call them
//! directly and drives EVERYTHING through `realize_step`.
//!
//! THE STEPCONTEXT IS BUILT BY HAND (not via `single_section_default`): the positive pivot witness
//! REQUIRES a real `prev_key_offset_semitones`, which only a direct `StepContext { .. }` literal can
//! set. `single_section_default` always defaults `prev: None` (the identity path), which is exactly
//! what the byte-freeze witness uses to prove the pivot is dead on identity.
//!
//! WHAT EACH WITNESS PROVES:
//!   * `pivot_inserts_nothing_on_identity` — THE primary byte-freeze behavioural witness (spec §3
//!     g4 / §6): for a non-modulating plan (home-only `prev:None`, and a same-key boundary, and a
//!     `pivot:false` modulating boundary) the realized note stream is byte-identical to the
//!     `single_section_default` baseline — the pivot path is provably dead.
//!   * `pivot_fires_on_modulating_boundary` — the positive: a `pivot:true` boundary whose key
//!     differs from its predecessor sounds a non-empty pivot at `step_in_section == 0`, and the
//!     pivot chord is the V of the destination key (bass root pc == `(dest_root_pc + 7) % 12`); a
//!     non-boundary step and a same-key boundary insert NOTHING (fall through to the frozen path).
//!   * `land_home_voicing_on_resolve_final` — a Resolve + `pivot:true` form's final PAC step is
//!     voiced as a home-key PAC (bass == home tonic root pc; melody/soprano == home tonic pc), and
//!     the event COUNT at that step is unchanged from the frozen single-note cadence stamp.
//!   * `no_inversion_under_pivot_path` — across the pivot boundary AND the land-home cadence step,
//!     the register frame never inverts (bass < fill < melody) and every note stays in 24..=108.

use audiohax::chord_engine::{realize_step, Chord, PerfFeatures, PhrasePosition, StepPlan};
use audiohax::composition::{
    CadenceStrength, KeyTempoPlan, OrchestrationProfile, ResolutionPolicy, Section, StepContext,
    ThematicRole, ThemeVariation,
};

const MS_PER_STEP: u64 = 200;
const HOME_ROOT_MIDI: u8 = 60; // C4 — home tonic pitch class 0.

/// The V_PIVOT / V_START phrase-initial accent the pivot is stamped with (chord_engine.rs:2158).
const V_PIVOT: u8 = 88;

/// A fixed C-major triad in root position — the pinned chord for the whole net (no RNG harmony).
fn c_major() -> Chord {
    Chord {
        name: "I".to_string(),
        notes: vec![60, 64, 67], // C4 E4 G4 → pcs 0,4,7
    }
}

/// PerfFeatures with brightness exactly 50 so `bright_octaves == 0.0` — the pivot/free-select
/// octave lift is zero and the seated registers are the bare role floors (the cleanest fixture for
/// asserting pitch CLASSES without an octave-lift confound). Mid saturation keeps velocity in band.
fn perf() -> PerfFeatures {
    PerfFeatures {
        saturation: 60.0,
        brightness: 50.0,
        edge_density: 0.20, // calm: a single sustained role note, not an arpeggio (one event/role)
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

/// A Perfect Authentic Cadence step (the land-home target — `plan_phrases` stamps this).
fn pac_step() -> StepPlan {
    StepPlan {
        chord: c_major(),
        phrase_index: 0,
        position_in_phrase: 3,
        phrase_len: 4,
        position: PhrasePosition::PerfectAuthenticCadence,
        velocity: 96,
    }
}

/// A Section carrying a chosen key offset, pivot opt-in, and resolution policy around a one-step
/// plan. Identity orchestration ⇒ the realizer's role stratification is the byte-frozen
/// `instrument_role` (inst 0 = Bass, inst 1 = Fill, inst 2 = Melody for a trio).
fn section(
    key_offset_semitones: i8,
    pivot: bool,
    resolution: ResolutionPolicy,
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
        density: 0.5,
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
/// set a real `prev_key_offset_semitones` (the firing signal). This mirrors the planner's
/// compose-path ctx build (engine.rs) exactly: section + step-in-section + prev offset.
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

/// The frozen IDENTITY realization of one instrument on one step via `single_section_default`
/// (which forces `prev: None`) — the pre-K3 baseline the byte-freeze witness compares against.
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

/// Mean pitch over a role's emitted events (for the register-ordering invariant).
fn mean_pitch(events: &[audiohax::chord_engine::NoteEvent]) -> f64 {
    assert!(
        !events.is_empty(),
        "role emitted no events to average pitch"
    );
    events.iter().map(|e| e.note as f64).sum::<f64>() / events.len() as f64
}

// ═════════════════════════════════════════════════════════════════════════════
// Witness 1 — pivot_inserts_nothing_on_identity (THE byte-freeze behavioural gate)
// ═════════════════════════════════════════════════════════════════════════════

/// THE primary byte-freeze witness (spec §3, the re-baseline-decision load-bearer): on every
/// non-modulating path the realized note stream is BYTE-IDENTICAL to the pre-K3
/// `single_section_default` baseline. Three sub-cases cover the three ways the pivot guard is dead:
///   (a) home-only / first-section identity (`prev: None`, offset 0, pivot:false) — driven via a
///       real `StepContext { prev: None, .. }` AND the `single_section_default` baseline;
///   (b) a `pivot:true` boundary whose key EQUALS its predecessor (`prev == dest`) — same key is
///       never a modulation, so the guard returns None;
///   (c) a `pivot:false` MODULATING boundary (`prev != dest`) — the scheme opt-in is off, so the
///       guard returns None even though the key changed.
/// In all three the stream must equal the identity baseline byte-for-byte (note, velocity, hold,
/// offset), proving the pivot path inserts NOTHING off the firing path.
#[test]
fn pivot_inserts_nothing_on_identity() {
    let kt = key_tempo();
    let step = interior_step();
    let pac = pac_step();
    let f = perf();

    // The baseline is the home, pivot:false section realized through single_section_default.
    let base_sec = section(0, false, ResolutionPolicy::Resolve, &step);
    let base_pac_sec = section(0, false, ResolutionPolicy::Resolve, &pac);

    // Sweep the ensemble so bass / fill / melody roles are all exercised, plus a lone-melody count.
    let role_cases: [(usize, usize); 4] = [(0, 3), (1, 3), (2, 3), (0, 1)];

    for (inst, num) in role_cases {
        let baseline = realize_baseline(&base_sec, &kt, &step, inst, num, &f);
        let baseline_pac = realize_baseline(&base_pac_sec, &kt, &pac, inst, num, &f);

        // (a) home-only / first-section identity ctx (prev:None) — interior AND the PAC step.
        let id_sec = section(0, false, ResolutionPolicy::Resolve, &step);
        let a_interior = realize_with_prev(&id_sec, &kt, &step, 0, None, inst, num, &f);
        assert_eq!(
            a_interior, baseline,
            "(a) identity interior (prev:None) must be byte-identical to the pre-K3 baseline \
             (inst {inst}/{num})"
        );
        let id_pac_sec = section(0, false, ResolutionPolicy::Resolve, &pac);
        let a_pac = realize_with_prev(&id_pac_sec, &kt, &pac, 0, None, inst, num, &f);
        assert_eq!(
            a_pac, baseline_pac,
            "(a) identity PAC step (prev:None, pivot:false) must be byte-identical to baseline \
             (inst {inst}/{num})"
        );

        // (b) a pivot:true boundary whose key EQUALS its predecessor (prev == dest == +7) — same
        //     key is never a modulation, so nothing is inserted.
        let same_key_sec = section(7, true, ResolutionPolicy::Resolve, &step);
        let same_key_baseline = realize_baseline(&same_key_sec, &kt, &step, inst, num, &f);
        let b = realize_with_prev(&same_key_sec, &kt, &step, 0, Some(7), inst, num, &f);
        assert_eq!(
            b, same_key_baseline,
            "(b) a pivot:true boundary with prev == dest (+7 == +7) is NOT a modulation and must \
             insert nothing (inst {inst}/{num})"
        );

        // (c) a pivot:FALSE modulating boundary (prev 0 → dest +7): the scheme opt-in is off, so
        //     even a real key change inserts nothing.
        let off_sec = section(7, false, ResolutionPolicy::Resolve, &step);
        let off_baseline = realize_baseline(&off_sec, &kt, &step, inst, num, &f);
        let c = realize_with_prev(&off_sec, &kt, &step, 0, Some(0), inst, num, &f);
        assert_eq!(
            c, off_baseline,
            "(c) a pivot:false modulating boundary (prev 0 → dest +7) must insert nothing — the \
             scheme opt-in is the byte-freeze gate (inst {inst}/{num})"
        );
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Witness 2 — pivot_fires_on_modulating_boundary (the positive)
// ═════════════════════════════════════════════════════════════════════════════

/// A `pivot:true` scheme on a firing fixture sounds a NON-EMPTY pivot at the FIRST step of a
/// section whose `key_offset_semitones` differs from its predecessor, and the pivot chord is the V
/// of the destination key (root pc == `(dest_root_pc + 7) % 12`, the Music Theory rule). The pivot
/// fires at NO other step: a non-boundary step (`step_in_section != 0`) and a same-key boundary
/// both fall through to the frozen path (witnessed against the baseline in Witness 1).
///
/// Fixture: home root C (pc 0), modulation prev 0 (home) → dest +7 (the dominant key G, pc 7).
///   dest_root_pc      = (0 + 7) % 12 = 7  (G)
///   pivot (V/dest)    = dominant of G = D, root pc = (7 + 7) % 12 = 2
/// So the bass (root-position V) must sound pitch class 2, and the pivot velocity is V_PIVOT (88)
/// and the hold is the full ms_per_step.
#[test]
fn pivot_fires_on_modulating_boundary() {
    let kt = key_tempo();
    let step = interior_step();
    let f = perf();

    let dest_off: i8 = 7;
    let prev_off: i8 = 0;
    let home_root_pc = (HOME_ROOT_MIDI % 12) as i16;
    let dest_root_pc = ((home_root_pc + dest_off as i16).rem_euclid(12)) as u8; // 7 (G)
    let expected_dom_root_pc = (dest_root_pc + 7) % 12; // V of the destination → pc 2 (D)

    let sec = section(dest_off, true, ResolutionPolicy::Resolve, &step);

    // (1) The pivot FIRES at the boundary (step_in_section == 0, prev != dest). Bass = inst 0 of a
    //     trio → root-position V of the destination.
    let bass = realize_with_prev(&sec, &kt, &step, 0, Some(prev_off), 0, 3, &f);
    assert!(
        !bass.is_empty(),
        "the pivot must sound a non-empty chord at the modulating boundary, got {bass:?}"
    );
    assert_eq!(
        bass.len(),
        1,
        "the pivot is one sustained note per role on the boundary downbeat, got {bass:?}"
    );
    let bass_ev = bass[0];
    assert_eq!(
        bass_ev.note % 12,
        expected_dom_root_pc,
        "the pivot's BASS must be the root of V-of-destination (pc {expected_dom_root_pc}); the \
         pivot chord is the dominant of the destination key per the harmonic rule. got note {} \
         (pc {})",
        bass_ev.note,
        bass_ev.note % 12
    );
    assert_eq!(
        bass_ev.velocity, V_PIVOT,
        "the pivot sounds at the phrase-start accent V_PIVOT ({V_PIVOT}), got {}",
        bass_ev.velocity
    );
    assert_eq!(
        bass_ev.hold_ms, MS_PER_STEP,
        "the pivot rings for the full step (a prepared arrival, not a stab), got {}",
        bass_ev.hold_ms
    );
    assert_eq!(
        bass_ev.offset_ms, 0,
        "the pivot lands on the boundary downbeat (offset 0), got {}",
        bass_ev.offset_ms
    );

    // The melody role also fires the pivot (a non-empty boundary chord across roles).
    let melody = realize_with_prev(&sec, &kt, &step, 0, Some(prev_off), 2, 3, &f);
    assert_eq!(
        melody.len(),
        1,
        "the pivot is one note for the melody role too, got {melody:?}"
    );
    assert_eq!(
        melody[0].velocity, V_PIVOT,
        "the melody pivot note also carries V_PIVOT, got {}",
        melody[0].velocity
    );

    // (2) The pivot itself fires at step 0 ONLY (never step 1) — but step 1 is NOT frozen: it is
    //     deliberately RE-VOICED to the destination ROOT-POSITION I (S29 Lever 1(b), the V→I
    //     authentic-cadence resolution). The pivot V at step 0 → the destination I at step 1.
    //     So step 1 is NO LONGER byte-identical to the free-select baseline (that was the K3
    //     misconception this S29 slice corrects); instead its BASS sounds the destination tonic
    //     ROOT (root-position I), i.e. the dom_root_pc → dest_root_pc cadence bass leap. We assert
    //     the INTENDED step-1 re-voicing, and separately re-confirm the pivot's V (pc
    //     `expected_dom_root_pc`) lands at step 0, NOT at step 1.
    let resolution_bass = realize_with_prev(&sec, &kt, &step, 1, Some(prev_off), 0, 3, &f);
    assert!(
        !resolution_bass.is_empty(),
        "the V->I resolution must sound at step 1 of the modulating section, got {resolution_bass:?}"
    );
    assert!(
        resolution_bass.iter().all(|e| e.note % 12 == dest_root_pc),
        "step 1 (the V->I resolution downbeat) must voice the DESTINATION TONIC ROOT \
         (pc {dest_root_pc} = root-position I) so the step-0 pivot V resolves V->I into the new \
         key — step 1 is re-voiced BY DESIGN, not frozen to the free-select baseline; got \
         {resolution_bass:?}"
    );
    // And the V (the pivot's root, pc `expected_dom_root_pc`) is the STEP-0 chord, distinct from
    // the step-1 destination tonic — the pivot fires at step 0, the resolution at step 1.
    assert_ne!(
        expected_dom_root_pc, dest_root_pc,
        "sanity: the pivot V root pc and the destination tonic root pc must differ for V->I to be \
         a real cadence"
    );
    assert!(
        resolution_bass
            .iter()
            .all(|e| e.note % 12 != expected_dom_root_pc),
        "step 1 must NOT still be sounding the pivot V (pc {expected_dom_root_pc}) — the pivot \
         fires at step 0; step 1 is the I it resolves into; got {resolution_bass:?}"
    );

    // (3) A same-key boundary (prev == dest) inserts NO pivot even at step 0.
    let same_key_baseline = realize_baseline(&sec, &kt, &step, 0, 3, &f);
    let same_key = realize_with_prev(&sec, &kt, &step, 0, Some(dest_off), 0, 3, &f);
    assert_eq!(
        same_key, same_key_baseline,
        "a same-key boundary (prev == dest == +7) is not a modulation and must insert nothing"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Witness 3 — land_home_voicing_on_resolve_final (the PAC strengthening)
// ═════════════════════════════════════════════════════════════════════════════

/// A Resolve + `pivot:true` form's final Perfect Authentic Cadence step is voiced as a home-key
/// PAC: the bass sounds the HOME TONIC ROOT pitch class (root-position I) and the melody/soprano
/// sounds the HOME TONIC pitch class (the defining PAC marker). The event COUNT at that step is
/// UNCHANGED from the frozen single-note cadence stamp (the re-voicing adds no event, no step) —
/// the land-home only re-points which pitch classes the bass/melody seat at.
///
/// Arming requires: Resolve AND pivot:true AND key_offset == 0 (the final section is at home) AND
/// position == PerfectAuthenticCadence. Home root C ⇒ home tonic pc 0.
#[test]
fn land_home_voicing_on_resolve_final() {
    let kt = key_tempo();
    let pac = pac_step();
    let f = perf();
    let home_tonic_pc = HOME_ROOT_MIDI % 12; // 0 (C)

    // Armed: Resolve + pivot:true + at-home final PAC step.
    let armed_sec = section(0, true, ResolutionPolicy::Resolve, &pac);

    // The frozen identity cadence (pivot:false) is the event-COUNT reference: same single sustained
    // ritardando note the cadence has always emitted per role.
    let frozen_sec = section(0, false, ResolutionPolicy::Resolve, &pac);

    // Bass = inst 0 of a trio.
    let frozen_bass = realize_baseline(&frozen_sec, &kt, &pac, 0, 3, &f);
    let bass = realize_with_prev(&armed_sec, &kt, &pac, 0, None, 0, 3, &f);
    assert_eq!(
        bass.len(),
        frozen_bass.len(),
        "land-home must NOT change the cadence event COUNT for the bass (it only re-voices); armed \
         {bass:?} vs frozen {frozen_bass:?}"
    );
    assert!(
        bass.iter().all(|e| e.note % 12 == home_tonic_pc),
        "land-home: the bass sounds the HOME TONIC ROOT (pc {home_tonic_pc}) → root-position I; \
         got {bass:?}"
    );

    // Melody = inst 2 of the trio (the soprano).
    let frozen_melody = realize_baseline(&frozen_sec, &kt, &pac, 2, 3, &f);
    let melody = realize_with_prev(&armed_sec, &kt, &pac, 2, None, 2, 3, &f);
    assert_eq!(
        melody.len(),
        frozen_melody.len(),
        "land-home must NOT change the cadence event COUNT for the melody; armed {melody:?} vs \
         frozen {frozen_melody:?}"
    );
    assert!(
        melody.iter().all(|e| e.note % 12 == home_tonic_pc),
        "land-home: the SOPRANO sounds the home TONIC (pc {home_tonic_pc}) — the defining PAC \
         marker that distinguishes a perfect from an imperfect authentic cadence; got {melody:?}"
    );

    // Negative arming guards — none of these may be voiced as the land-home PAC (the soprano need
    // NOT be the tonic). Each flips exactly one arming condition off.
    // Open ending (not Resolve) → not armed.
    let open_sec = section(0, true, ResolutionPolicy::Open, &pac);
    let open_melody = realize_with_prev(&open_sec, &kt, &pac, 2, None, 2, 3, &f);
    let frozen_open_melody = realize_baseline(&open_sec, &kt, &pac, 2, 3, &f);
    assert_eq!(
        open_melody, frozen_open_melody,
        "an Open ending must NOT arm land-home — the cadence voicing is untouched/byte-identical"
    );
    // pivot:false → not armed (byte-identical to the frozen path).
    let nopivot_melody = realize_with_prev(&frozen_sec, &kt, &pac, 2, None, 2, 3, &f);
    assert_eq!(
        nopivot_melody, frozen_melody,
        "a pivot:false final must NOT arm land-home — byte-identical to the frozen cadence"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Witness 4 — no_inversion_under_pivot_path (the register guard, pivot-aware)
// ═════════════════════════════════════════════════════════════════════════════

/// The hard register invariant (`bass < fill < melody`, every note ∈ 24..=108), re-asserted across
/// the K3 pivot boundary AND the land-home cadence step — the pivot/land-home voicings seat every
/// pitch via the SAME register floors the free-select path uses, so the frame must never invert.
/// Swept over the full menu of modulating boundaries {prev 0 → dest +7/+5/+3/−3} for the pivot, and
/// the at-home Resolve final for land-home, across a range of brightness (the octave-lift driver).
///
/// S29 EXTENSION (spec §6 task 6): the pivot's FILL voice now carries the dominant SEVENTH (Lever
/// 3) — the no-inversion frame must hold WITH that dom7 present. This sweep therefore ALSO pins, at
/// every (dest × bright) pivot combo, that the fill voice sounds `dom_seventh_pc = (dom_root_pc +
/// 10) % 12 = (dest_root_pc + 5) % 12` (a 3+ ensemble has a dedicated inner voice), so the
/// bass<fill<melody assertions below are guarding the actual V7 voicing, not a stale bare triad.
#[test]
fn no_inversion_under_pivot_path() {
    let kt = key_tempo();
    let brightnesses = [12.0f32, 50.0, 100.0]; // darkest / neutral / brightest (max lift)
    let menu: [i8; 4] = [7, 5, 3, -3];
    let prev_off: i8 = 0;

    let mut combos = 0usize;

    // --- The PIVOT boundary across every menu destination ---
    let step = interior_step();
    for &dest in &menu {
        for &bright in &brightnesses {
            let f = PerfFeatures {
                saturation: 60.0,
                brightness: bright,
                edge_density: 0.20,
            };
            let sec = section(dest, true, ResolutionPolicy::Resolve, &step);
            // ALL THREE roles realized at the SAME boundary downbeat (step_in_section == 0) so the
            // pivot fires for each — the frame and the S29 dom7 are asserted on the actual pivot
            // voicing. (The original K3 sweep passed step_in_section == inst_idx, which only fired
            // the pivot for the bass; the frame held trivially because the fill/melody fell through
            // to free-select. S29 task 6 needs the dom7-bearing pivot voicing in all three voices.)
            let bass = realize_with_prev(&sec, &kt, &step, 0, Some(prev_off), 0, 3, &f);
            let fill = realize_with_prev(&sec, &kt, &step, 0, Some(prev_off), 1, 3, &f);
            let melody = realize_with_prev(&sec, &kt, &step, 0, Some(prev_off), 2, 3, &f);

            for (who, evs) in [("bass", &bass), ("fill", &fill), ("melody", &melody)] {
                assert!(
                    !evs.is_empty(),
                    "pivot {who} emitted no events (dest {dest})"
                );
                for e in evs.iter() {
                    assert!(
                        (24..=108).contains(&e.note),
                        "pivot note {} out of band 24..=108 ({who}, dest {dest}, bright {bright})",
                        e.note
                    );
                }
            }
            // S29 Lever 3: the pivot inner/fill voice sounds the dominant 7th (the V7 color) —
            // assert the frame is held WITH the dom7 present (spec §6 task 6).
            let home_root_pc = (HOME_ROOT_MIDI % 12) as i16;
            let dest_root_pc = ((home_root_pc + dest as i16).rem_euclid(12)) as u8;
            let dom_root_pc = (dest_root_pc + 7) % 12;
            let dom_seventh_pc = (dom_root_pc + 10) % 12; // == (dest_root_pc + 5) % 12
            assert!(
                fill.iter().all(|e| e.note % 12 == dom_seventh_pc),
                "the pivot FILL must carry the dominant 7th (pc {dom_seventh_pc}) for a 3+ \
                 ensemble (dest {dest}, bright {bright}); got {fill:?}"
            );
            let b = mean_pitch(&bass);
            let m = mean_pitch(&fill);
            let t = mean_pitch(&melody);
            assert!(
                b < m,
                "PIVOT INVERSION: bass {b:.1} not < fill {m:.1} (dest {dest}, bright {bright})"
            );
            assert!(
                m < t,
                "PIVOT INVERSION: fill {m:.1} not < melody {t:.1} (dest {dest}, bright {bright})"
            );
            combos += 1;
        }
    }

    // --- The LAND-HOME cadence (at-home Resolve + pivot:true final PAC) ---
    let pac = pac_step();
    for &bright in &brightnesses {
        let f = PerfFeatures {
            saturation: 60.0,
            brightness: bright,
            edge_density: 0.20,
        };
        let sec = section(0, true, ResolutionPolicy::Resolve, &pac);
        let bass = realize_with_prev(&sec, &kt, &pac, 0, None, 0, 3, &f);
        let fill = realize_with_prev(&sec, &kt, &pac, 1, None, 1, 3, &f);
        let melody = realize_with_prev(&sec, &kt, &pac, 2, None, 2, 3, &f);

        for (who, evs) in [("bass", &bass), ("fill", &fill), ("melody", &melody)] {
            assert!(
                !evs.is_empty(),
                "land-home {who} emitted no events (bright {bright})"
            );
            for e in evs.iter() {
                assert!(
                    (24..=108).contains(&e.note),
                    "land-home note {} out of band 24..=108 ({who}, bright {bright})",
                    e.note
                );
            }
        }
        let b = mean_pitch(&bass);
        let m = mean_pitch(&fill);
        let t = mean_pitch(&melody);
        assert!(
            b < m,
            "LAND-HOME INVERSION: bass {b:.1} not < fill {m:.1} (bright {bright})"
        );
        assert!(
            m < t,
            "LAND-HOME INVERSION: fill {m:.1} not < melody {t:.1} (bright {bright})"
        );
        combos += 1;
    }

    assert_eq!(
        combos,
        menu.len() * brightnesses.len() + brightnesses.len(),
        "the register-invariant sweep must cover every (pivot dest × bright) + (land-home × bright)"
    );
}

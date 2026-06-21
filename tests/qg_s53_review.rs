//! tests/qg_s53_review.rs — Quality Gate INDEPENDENT verification net for S53 slice 1 (D-CELL).
//! Authored by the AudioHax Quality Gate, NOT the Producer. Adversarial re-derivation of the
//! load-bearing claims: freeze hinge no-op, onset-bias REAL+count-preserving+anchored, GR-1 (bed
//! untouched), GR-2 (cadence/pre-cadence relaxation), and the 6-probe table off the ACTUAL planner
//! path (not the unit selector). Runs under default features.

use audiohax::chord_engine::{
    realize_step, Chord, MotifArchetype, NoteEvent, PerfFeatures, PhrasePosition, RhythmMotto,
    StepPlan,
};
use audiohax::composition::{
    CadenceStrength, CompositionPlanner, ImageUnderstanding, KeyTempoPlan, OrchestrationProfile,
    PlanMappings, ResolutionPolicy, Section, StepContext, ThematicRole, ThemeVariation,
};
use audiohax::mapping_loader::{load_mappings, MappingTable};
use audiohax::pure_analysis::{load_pure_image, understand_image_pure, PureImageSource};
use audiohax::seed::set_composition_seed;

const STEP_MS: u64 = 200;

fn mappings() -> MappingTable {
    load_mappings("assets/mappings.json").expect("mappings load")
}
fn plan_mappings(m: &MappingTable) -> PlanMappings {
    m.composition.clone().expect("composition block").into()
}

/// Build a one-step section with the given motto at a given phrase position. `pos`/`phrase_len`
/// drive `pre_cadence`; `position` drives `is_cadence` (both read in realize_rhythm).
fn sec_with(motto: RhythmMotto, pos: usize, phrase_len: usize, position: PhrasePosition) -> Section {
    let step = StepPlan {
        chord: Chord {
            name: "I".to_string(),
            notes: vec![60, 64, 67],
        },
        phrase_index: 0,
        position_in_phrase: pos,
        phrase_len,
        position,
        velocity: 76,
    };
    let mut o = OrchestrationProfile::identity();
    o.motto = motto;
    Section {
        label: "A".to_string(),
        step_len: 1,
        thematic_role: ThematicRole::Statement,
        key_offset_semitones: 0,
        ms_per_step: STEP_MS,
        mode: "Ionian".to_string(),
        progression: vec![],
        theme: None,
        variation: ThemeVariation::Identity,
        boundary_cadence: CadenceStrength::Perfect,
        pivot: false,
        resolution: ResolutionPolicy::Resolve,
        density: 0.5,
        orchestration: o,
        steps: vec![step],
    }
}

fn cell(c: usize) -> RhythmMotto {
    RhythmMotto {
        archetype: MotifArchetype::Arch,
        cell_index: Some(c),
    }
}

fn kt() -> KeyTempoPlan {
    KeyTempoPlan {
        home_root_midi: 60,
        home_mode: "Ionian".to_string(),
        base_ms_per_step: STEP_MS,
        key_scheme: vec![0],
        tempo_scheme: vec![STEP_MS],
    }
}

/// Busy perf so the melody realizes >= 2 onsets (an interior onset exists to displace).
fn busy_perf() -> PerfFeatures {
    PerfFeatures {
        saturation: 60.0,
        brightness: 55.0,
        edge_density: (0.85_f32 * 0.05).clamp(0.0, 1.0),
    }
}

fn sig(sec: &Section, num_instruments: usize, inst_idx: usize) -> Vec<(u64, u64)> {
    let binding = kt();
    let ctx = StepContext::single_section_default(sec, &binding);
    let mut s: Vec<(u64, u64)> = realize_step(
        &sec.steps[0],
        inst_idx,
        num_instruments,
        &busy_perf(),
        STEP_MS,
        &ctx,
    )
    .iter()
    .map(|e| (e.offset_ms, e.hold_ms))
    .collect();
    s.sort_unstable();
    s
}

/// CLAIM (freeze hinge / neutral no-op): neutral() short-circuits. cell 0 (anchor) and cell 2
/// (even) are authored zero-displacement, so they must equal neutral exactly.
#[test]
fn qg_neutral_motto_is_a_strict_no_op() {
    let n = sig(&sec_with(RhythmMotto::neutral(), 2, 8, PhrasePosition::Interior), 1, 0);
    let c0 = sig(&sec_with(cell(0), 2, 8, PhrasePosition::Interior), 1, 0);
    let c2 = sig(&sec_with(cell(2), 2, 8, PhrasePosition::Interior), 1, 0);
    assert!(n.len() >= 2, "busy band must give >= 2 onsets, got {n:?}");
    assert_eq!(n, c0, "neutral must equal zero-displacement cell 0");
    assert_eq!(n, c2, "neutral must equal zero-displacement cell 2");
}

/// CLAIM (onset-bias REAL, count-preserving, anchored, ordered, bounded): cell 3 displaces an
/// interior onset, keeps the count, leaves the downbeat fixed, preserves order, stays in-step.
#[test]
fn qg_cell3_displaces_interior_preserves_count_anchors_downbeat() {
    let n = sig(&sec_with(RhythmMotto::neutral(), 2, 8, PhrasePosition::Interior), 1, 0);
    let p = sig(&sec_with(cell(3), 2, 8, PhrasePosition::Interior), 1, 0);
    assert_eq!(n.len(), p.len(), "count preserved: {n:?} vs {p:?}");
    assert_eq!(n[0].0, p[0].0, "downbeat anchor fixed");
    assert_ne!(n, p, "cell-3 motto must AUDIBLY move an interior onset");
    let offs: Vec<u64> = p.iter().map(|x| x.0).collect();
    for w in offs.windows(2) {
        assert!(w[0] < w[1], "onsets must stay strictly ordered: {offs:?}");
    }
    assert!(*offs.last().unwrap() < STEP_MS, "no onset rings past the step: {offs:?}");
}

/// CLAIM (cell 1 also biases, opposite direction): cell 1 (broad) pulls earlier; distinct from
/// both neutral and cell 3.
#[test]
fn qg_cell1_biases_distinctly_from_cell3() {
    let c1 = sig(&sec_with(cell(1), 2, 8, PhrasePosition::Interior), 1, 0);
    let c3 = sig(&sec_with(cell(3), 2, 8, PhrasePosition::Interior), 1, 0);
    let n = sig(&sec_with(RhythmMotto::neutral(), 2, 8, PhrasePosition::Interior), 1, 0);
    assert_eq!(c1.len(), n.len(), "cell 1 count-preserving");
    assert_eq!(c1[0].0, n[0].0, "cell 1 anchors the downbeat too");
    // cell 1 should differ from neutral on at least one interior onset (the earlier pull).
    assert_ne!(c1, n, "cell 1 must bias (the earlier pull) — not a dead gait");
    assert_ne!(c1, c3, "cell 1 (earlier) and cell 3 (later) must be different walks");
}

/// CLAIM (GR-2 cadence): at a cadence the cadence ring early-returns before the motto application,
/// so a cell-3 motto equals neutral at the cadence (single sustained ring).
#[test]
fn qg_cadence_step_ignores_motto() {
    let cn = sig(&sec_with(RhythmMotto::neutral(), 7, 8, PhrasePosition::PerfectAuthenticCadence), 1, 0);
    let cp = sig(&sec_with(cell(3), 7, 8, PhrasePosition::PerfectAuthenticCadence), 1, 0);
    assert_eq!(cn, cp, "motto must be ZERO at a cadence: {cn:?} vs {cp:?}");
    assert_eq!(cn.len(), 1, "cadence rings a single sustained note");
}

/// CLAIM (GR-2 pre-cadence attenuation): pre-cadence halves the bias — displacement magnitude vs
/// each step's own neutral baseline must be <= the full-strength interior displacement.
#[test]
fn qg_precadence_attenuates_bias() {
    let full = sig(&sec_with(cell(3), 2, 8, PhrasePosition::Interior), 1, 0);
    let nf = sig(&sec_with(RhythmMotto::neutral(), 2, 8, PhrasePosition::Interior), 1, 0);
    // pre-cadence: position_in_phrase + 2 >= phrase_len, not start, not cadence → pos 6 of 8.
    let pre = sig(&sec_with(cell(3), 6, 8, PhrasePosition::Interior), 1, 0);
    let np = sig(&sec_with(RhythmMotto::neutral(), 6, 8, PhrasePosition::Interior), 1, 0);
    if full.len() >= 2 && pre.len() >= 2 && nf.len() >= 2 && np.len() >= 2 {
        let d_full = (full[1].0 as i64 - nf[1].0 as i64).abs();
        let d_pre = (pre[1].0 as i64 - np[1].0 as i64).abs();
        assert!(d_pre <= d_full, "pre-cadence displacement {d_pre} must be <= full {d_full}");
        // and the full-strength one must actually be > 0 (the bias is live at full strength).
        assert!(d_full > 0, "full-strength interior displacement must be non-zero");
    }
}

/// CLAIM (GR-1 figure-ground): the BED roles are NEVER touched by the motto, and ONLY the melody
/// is. With 4 instruments under legacy stratification (`instrument_role`): inst 0 = Bass,
/// inst 1/2 = HarmonicFill (the bed), inst 3 = Melody (the figure). The bed arms must be identical
/// under neutral vs a busy cell-3 motto; the melody arm must CHANGE (the bias lands on the figure).
#[test]
fn qg_bed_roles_untouched_only_melody_biased() {
    let neutral_sec = sec_with(RhythmMotto::neutral(), 2, 8, PhrasePosition::Interior);
    let profiled_sec = sec_with(cell(3), 2, 8, PhrasePosition::Interior);
    // Bed roles inst 0,1,2 — must be byte-identical (the motto never touches them).
    for inst in 0..3usize {
        let nb = sig(&neutral_sec, 4, inst);
        let pb = sig(&profiled_sec, 4, inst);
        assert_eq!(
            nb, pb,
            "bed role inst {inst} must be IDENTICAL under neutral vs cell-3 (GR-1): {nb:?} vs {pb:?}"
        );
    }
    // The MELODY (inst 3 of 4) must receive the bias (the figure carries the gait).
    let nm = sig(&neutral_sec, 4, 3);
    let pm = sig(&profiled_sec, 4, 3);
    assert_eq!(nm.len(), pm.len(), "melody count preserved");
    assert_ne!(nm, pm, "the melody (inst 3) MUST carry the motto gait: {nm:?} vs {pm:?}");
}

/// CLAIM (6-probe table, REAL PLANNER PATH): the planner-stamped `Section.motto.cell_index` over
/// the six shipped images matches the adjudicated Affect table, and is uniform across sections.
#[test]
fn qg_six_probe_table_off_real_planner() {
    let images: [(&str, usize); 6] = [
        ("AudioHaxImg1.jpg", 1),
        ("AudioHaxImg2.jpg", 0),
        ("AudioHaxImg3.jpg", 3),
        ("example.jpg", 2),
        ("Lena.png", 0),
        ("magicstudio-art.jpg", 3),
    ];
    let m = mappings();
    let planner = CompositionPlanner::new(plan_mappings(&m));
    for (name, want) in images {
        let img = load_pure_image(&PureImageSource::Preselected(name.to_string()))
            .unwrap_or_else(|e| panic!("load {name}: {e:?}"));
        let u: ImageUnderstanding = understand_image_pure(img.as_rgb())
            .unwrap_or_else(|e| panic!("understand {name}: {e:?}"));
        set_composition_seed(Some(42));
        let plan = planner.plan(&u, &m);
        for (i, s) in plan.sections.iter().enumerate() {
            assert_eq!(
                s.motto().cell_index,
                Some(want),
                "{name} section {i}: motto cell {:?}, Affect table wants {want}",
                s.motto().cell_index
            );
        }
    }
}

/// CLAIM (smoke): the realize path produces well-formed events under a motto.
#[test]
fn qg_smoke_events_wellformed() {
    let sec = sec_with(cell(3), 2, 8, PhrasePosition::Interior);
    let binding = kt();
    let ctx = StepContext::single_section_default(&sec, &binding);
    let events: Vec<NoteEvent> = realize_step(&sec.steps[0], 0, 1, &busy_perf(), STEP_MS, &ctx);
    assert!(!events.is_empty());
    for e in &events {
        assert!((24..=108).contains(&e.note), "midi in range");
        assert!(e.hold_ms >= 1, "hold positive");
    }
}

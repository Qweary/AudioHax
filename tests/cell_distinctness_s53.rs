//! tests/cell_distinctness_s53.rs — the S53 / fix-direction-2 SLICE 1 (D-CELL) PRE-BUILD BASELINE.
//!
//! ┌─────────────────────────────────────────────────────────────────────────────────────────┐
//! │ WHAT THIS IS — the BEFORE half of an A/B.                                                  │
//! │                                                                                           │
//! │ Slice 1 (docs/design-s53-cell-seam.md) un-gates the rhythm-cell axis to run PER-PIECE: it │
//! │ ADDS a per-piece `RhythmMotto` (`Section.motto`, with `motto.cell_index`) selected once   │
//! │ per plan from `edge_activity` + `affect_arousal`, stamped on every section, and read by   │
//! │ `realize_rhythm`. TODAY that seam does NOT exist — `Section.motto` is not a type yet.      │
//! │                                                                                           │
//! │ This file captures the CURRENT (dormant) state of the cell axis as documented baseline    │
//! │ constants, so the post-build run can prove the axis became LIVE and the six probes gained  │
//! │ distinct rhythmic gaits. The assertions here are written to PASS at clean HEAD             │
//! │ (documenting the dormant baseline) and are DESIGNED TO BE FLIPPED/extended after slice 1   │
//! │ lands to assert the distinctness GAIN (see the per-test "AFTER SLICE 1" notes).           │
//! └─────────────────────────────────────────────────────────────────────────────────────────┘
//!
//! THE DORMANCY (verified against HEAD, design-s53 §1):
//!   `pick_rhythm_cell` (composition.rs:1490) is the ONLY cell call site and lives inside the
//!   `else` of the theme gate at :1478, reached only when `theme_behaviour == "fragment"`, which
//!   requires `complexity >= 0.4` (mappings.json:122). On the no-theme path `plan.themes == []`
//!   and NO cell is selected — the cell axis is dead code on the real-photo path.
//!
//! HIGH-VALUE BASELINE NUANCE (a measured surprise — see `baseline_cell_axis_is_dormant_...`):
//!   The seam spec says "every probe yields NO_THEME_CELL." That is TRUE for the four genuine
//!   low-complexity photos (Img1 0.005, Img2 0.015, Img3 0.229, Lena 0.164 — all < 0.4 → NO cell).
//!   But TWO of the six bundled probes are NOT low-complexity: `example.jpg` (complexity 0.905)
//!   and `magicstudio-art.jpg` (complexity 1.000) clear the 0.4 theme gate, DO realize a theme,
//!   and DO reach `pick_rhythm_cell` today — and BOTH force-pin to cell 3 (the S50 PROFILED-divert
//!   collapse, design-s53 §1.3). So today the cell axis is in two states, BOTH collapsed:
//!     • 4 probes — cell NONE (dormant; the axis never runs)
//!     • 2 probes — cell 3, cell 3 (live but FORCE-PINNED; the axis runs but never discriminates)
//!   Net distinct *real* cell values across the six today = {3} (one single value). After slice 1
//!   the per-piece motto runs on ALL six off the (spread edge, arousal) driver and the four NONE
//!   probes gain real cells, so the distinct-cell count is expected to RISE from 1 → >= 3.
//!
//! WHAT IS MEASURED (chosen to remain measurable after the seam lands):
//!   (1) THE CELL OBSERVABLE — `realized_cell` (the SAME machinery as rhythm_variety_s50.rs:273:
//!       match `plan.themes[0].motif` against the (archetype, cell) vocabulary; NO_THEME_CELL when
//!       `themes` is empty). This is the honest cell-in-effect today. AFTER SLICE 1 it re-points
//!       at `Section.motto.cell_index` (design-s53 §5.2-B) — the comment on each use says so.
//!   (2) THE REALIZED ONSET PATTERN / IOI PROFILE — per probe, off the PURE realizer `realize_step`
//!       over a fixed identity plan (no OpenCV, no RNG): the sorted `(offset_ms, hold_ms)` onset
//!       signature + onset count + onset density. This is the audible rhythmic gait and stays
//!       measurable unchanged after the seam (the motto biases exactly these onsets).
//!   (3) A PAIRWISE-DISTINCTNESS SUMMARY on the CELL axis — how SIMILAR the six pieces' cells are
//!       today (expected HIGH similarity / LOW distinctness): the count of distinct real cell
//!       values, and the fraction of the 15 unordered probe pairs that share a cell label. Today
//!       that fraction is HIGH (the axis is collapsed); after slice 1 it must DROP.
//!
//! DETERMINISM / HEADLESS DISCIPLINE (mirrors rhythm_variety_s50.rs):
//!   • Builds `ImageUnderstanding` fixtures IN-MEMORY from the EXACT feature values
//!     `understand_image_pure` produces for the six shipped images (captured at clean HEAD —
//!     `64b6883`). It does NOT call `pure_analysis` (which is `--no-default-features`-gated out),
//!     so this file RUNS under `cargo test --no-default-features` (the headless lib/test path).
//!   • The composition seed is pinned to ChaCha8 seed 42 before each `plan()`. Every asserted
//!     observable (the cell, the realized onset signature) is RNG-INDEPENDENT of the per-section
//!     harmony draw; the seed pin only keeps the run byte-stable for `--nocapture` inspection.
//!   • Reads only the shipped `assets/mappings.json` (a read, the s50 precedent). No fs writes.
//!     Each test < 10s.
//!
//! Run (headless — the deliverable path):  cargo test --no-default-features --test cell_distinctness_s53 -- --nocapture

use std::collections::BTreeSet;

use audiohax::chord_engine::{
    realize_step, Chord, MotifArchetype, NoteEvent, PerfFeatures, PhrasePosition, RhythmMotto,
    StepPlan,
};
use audiohax::composition::{
    CadenceStrength, CompositionPlan, CompositionPlanner, ImageUnderstanding, KeyTempoPlan,
    OrchestrationProfile, PlanMappings, ResolutionPolicy, Section, StepContext, ThematicRole,
    ThemeVariation,
};
use audiohax::mapping_loader::{load_mappings, MappingTable};
use audiohax::seed::set_composition_seed;

const MAPPINGS_PATH: &str = "assets/mappings.json";

/// The fixed composition seed (matches rhythm_variety_s50::SEED). Every asserted observable is
/// RNG-independent regardless; pinning keeps the run byte-stable for `--nocapture`.
const SEED: u64 = 42;

/// Whole-image `edge_activity` → raw per-bar edge density (the realizer re-normalizes by /0.05);
/// identical to rhythm_variety_s50::EDGE_RANGE_MAX / perf_for.
const EDGE_RANGE_MAX: f32 = 0.05;

/// Sentinel for "no theme realized" — distinct from any real 0..=3 cell index. Same value &
/// meaning as rhythm_variety_s50::NO_THEME_CELL (composition.rs:1478 — complexity < 0.4 ⇒ no cell).
const NO_THEME_CELL: usize = usize::MAX;

// ═════════════════════════════════════════════════════════════════════════════════════════════
// THE SIX PROBE FIXTURES — built IN-MEMORY from the EXACT `understand_image_pure` output.
//
// Captured at clean HEAD (64b6883) by dumping the full `ImageUnderstanding` for each shipped
// image. These are the SAME images and the SAME feature values rhythm_variety_s50.rs measures via
// `understand()` — but materialized as literals so this file needs NO `pure_analysis` and runs
// `--no-default-features`. Only the fields that DIFFER from `ImageUnderstanding::neutral()` are
// overridden below; every default (palette_bimodality=0, subject_*=whole-image, the affect -1.0
// sentinels, etc.) is inherited from `neutral()`, exactly as the runtime understanding leaves them.
// If a future feature-extraction change moves these numbers, the dump must be re-captured.
// ═════════════════════════════════════════════════════════════════════════════════════════════

/// One probe's identity (name + the in-memory understanding mirroring the shipped image).
struct Probe {
    name: &'static str,
    u: ImageUnderstanding,
}

fn audiohaximg1() -> ImageUnderstanding {
    ImageUnderstanding {
        edge_activity: 0.300_852_45,
        texture: 0.614_859_7,
        complexity: 0.0055,
        dominant_hue: 39.944_065,
        secondary_hue: 39.944_065,
        colorfulness: 0.010_794_501,
        value_key: 0.707_035_6,
        avg_brightness: 29.296_44,
        avg_saturation: 32.892_387,
        mass_centroid: (0.500_969_7, 0.497_743_2),
        quadrant_contrast: 0.229_000_58,
        aspect_ratio: 0.666_666_7,
        vertical_emphasis: 0.245_980_84,
        subject_size: 0.110_677_086,
        subject_hue: 39.797_47,
        subject_saturation: 26.061_695,
        fg_bg_contrast: 0.340_544_4,
        subject_energy: 0.046_966_91,
        foreground_energy: 0.016_607_774,
        background_energy: 0.004_809_604,
        foreground_brightness: 0.321_849_55,
        background_brightness: 0.213_565_41,
        foreground_hue: 39.994_427,
        background_hue: 39.930_447,
        ..ImageUnderstanding::neutral()
    }
}

fn audiohaximg2() -> ImageUnderstanding {
    ImageUnderstanding {
        edge_activity: 0.509_440_1,
        texture: 0.774_034_26,
        complexity: 0.0155,
        dominant_hue: 36.641_65,
        secondary_hue: 36.641_65,
        colorfulness: 0.079_985_05,
        value_key: 0.189_277_71,
        avg_brightness: 81.072_23,
        avg_saturation: 30.131_567,
        mass_centroid: (0.498_382_4, 0.506_082_1),
        quadrant_contrast: 0.253_495_3,
        aspect_ratio: 0.666_666_7,
        vertical_emphasis: 0.360_304_47,
        subject_size: 0.110_677_086,
        subject_hue: 40.252_502,
        subject_saturation: 36.062_172,
        fg_bg_contrast: 0.283_649_06,
        subject_energy: 0.069_462_314,
        foreground_energy: 0.034_058_183,
        background_energy: 0.005_112_057_6,
        foreground_brightness: 0.759_225_55,
        background_brightness: 0.899_400_8,
        foreground_hue: 37.471_355,
        background_hue: 35.112_05,
        ..ImageUnderstanding::neutral()
    }
}

fn audiohaximg3() -> ImageUnderstanding {
    ImageUnderstanding {
        edge_activity: 0.475_413_02,
        texture: 0.549_773_63,
        complexity: 0.229,
        dominant_hue: 5.190_332_4,
        secondary_hue: 5.190_332_4,
        colorfulness: 0.422_648_28,
        value_key: 0.343_151_27,
        avg_brightness: 65.684_875,
        avg_saturation: 37.175_632,
        mass_centroid: (0.512_531_9, 0.500_354_65),
        quadrant_contrast: 0.244_754_2,
        aspect_ratio: 1.5,
        vertical_emphasis: 0.333_438_93,
        subject_size: 0.110_677_086,
        subject_hue: 208.957_26,
        subject_saturation: 37.822_582,
        fg_bg_contrast: 0.202_785_04,
        subject_energy: 0.049_172_793,
        foreground_energy: 0.023_566_082,
        background_energy: 0.016_659_208,
        foreground_brightness: 0.600_374_2,
        background_brightness: 0.750_250_34,
        foreground_hue: 345.544_16,
        background_hue: 18.020_037,
        ..ImageUnderstanding::neutral()
    }
}

fn example() -> ImageUnderstanding {
    ImageUnderstanding {
        edge_activity: 0.718_838_33,
        texture: 0.978_892_86,
        complexity: 0.905,
        dominant_hue: 198.645_83,
        secondary_hue: 198.645_83,
        colorfulness: 0.685_459_9,
        value_key: 0.501_217_96,
        avg_brightness: 49.878_21,
        avg_saturation: 64.523_89,
        mass_centroid: (0.515_335_56, 0.502_490_2),
        quadrant_contrast: 0.105_197_15,
        aspect_ratio: 2.045_454_5,
        vertical_emphasis: 0.325_606_4,
        subject_size: 0.110_606_06,
        subject_hue: 212.212_72,
        subject_saturation: 58.591_62,
        fg_bg_contrast: 0.135_541_84,
        subject_energy: 0.042_625_573,
        foreground_energy: 0.038_705_416,
        background_energy: 0.029_026_287,
        foreground_brightness: 0.515_200_1,
        background_brightness: 0.495_638_82,
        foreground_hue: 168.416_38,
        background_hue: 177.168_4,
        ..ImageUnderstanding::neutral()
    }
}

fn lena() -> ImageUnderstanding {
    ImageUnderstanding {
        edge_activity: 0.471_038_82,
        texture: 0.193_568_16,
        complexity: 0.1645,
        dominant_hue: 354.146_06,
        secondary_hue: 354.146_06,
        colorfulness: 0.121_869_765,
        value_key: 0.294_627_3,
        avg_brightness: 70.537_27,
        avg_saturation: 51.902_477,
        mass_centroid: (0.504_138_9, 0.482_198_5),
        quadrant_contrast: 0.125_954_08,
        aspect_ratio: 1.0,
        vertical_emphasis: 0.360_655_16,
        subject_size: 0.110_244_75,
        subject_hue: 349.805_27,
        subject_saturation: 49.564_842,
        fg_bg_contrast: 0.051_982_466,
        subject_energy: 0.044_429_068,
        foreground_energy: 0.015_574_455,
        background_energy: 0.023_249_865,
        foreground_brightness: 0.721_131_74,
        background_brightness: 0.690_103_2,
        foreground_hue: 355.846_3,
        background_hue: 352.981_45,
        ..ImageUnderstanding::neutral()
    }
}

fn magicstudio() -> ImageUnderstanding {
    ImageUnderstanding {
        edge_activity: 0.106_048_584,
        texture: 0.163_844_06,
        complexity: 1.0,
        dominant_hue: 278.313_54,
        secondary_hue: 278.313_54,
        colorfulness: 0.286_599_76,
        value_key: 0.546_607_6,
        avg_brightness: 45.339_237,
        avg_saturation: 43.611_38,
        mass_centroid: (0.508_931_1, 0.454_330_7),
        quadrant_contrast: 0.193_723_89,
        aspect_ratio: 1.0,
        vertical_emphasis: 0.383_273_8,
        subject_size: 0.110_894_2,
        subject_hue: 250.592_27,
        subject_saturation: 40.032_196,
        fg_bg_contrast: 0.083_628_68,
        subject_energy: 0.008_943_851,
        foreground_energy: 0.003_075_869_8,
        background_energy: 0.006_068_581_2,
        foreground_brightness: 0.477_911_32,
        background_brightness: 0.420_415_85,
        foreground_hue: 274.938_5,
        background_hue: 285.464_02,
        ..ImageUnderstanding::neutral()
    }
}

/// The six bundled probes, in a fixed order (matches rhythm_variety_s50::IMAGES alphabetization).
fn probes() -> Vec<Probe> {
    vec![
        Probe {
            name: "AudioHaxImg1.jpg",
            u: audiohaximg1(),
        },
        Probe {
            name: "AudioHaxImg2.jpg",
            u: audiohaximg2(),
        },
        Probe {
            name: "AudioHaxImg3.jpg",
            u: audiohaximg3(),
        },
        Probe {
            name: "example.jpg",
            u: example(),
        },
        Probe {
            name: "Lena.png",
            u: lena(),
        },
        Probe {
            name: "magicstudio-art.jpg",
            u: magicstudio(),
        },
    ]
}

fn mappings() -> MappingTable {
    load_mappings(MAPPINGS_PATH).expect("assets/mappings.json loads")
}

fn plan_mappings(m: &MappingTable) -> PlanMappings {
    m.composition
        .clone()
        .expect("composition block present in mappings.json")
        .into()
}

// ─────────────────────────────────────────────────────────────────────────────────────────────
// (1) THE CELL OBSERVABLE — `realized_cell`, RE-POINTED for slice 1 at `section.motto().cell_index`
// (the new honest, engine-read cell observable — design-s53 §5.2-B). The old theme-matching
// machinery (ALL_ARCHETYPES / range_for / length_for / identify_cell, which read the always-empty
// `themes[0]` on real photos) is gone; the cell now reads off the per-piece motto the engine
// actually consumes. This is the dormant→live flip the slice delivers.
// ─────────────────────────────────────────────────────────────────────────────────────────────

/// The planner-realized PER-PIECE rhythm cell for an image — the LIVE observable after fix-direction-2
/// SLICE 1 (D-CELL). RE-POINTED (per this file's "AFTER SLICE 1" notes + design-s53 §5.2-B): the cell
/// is now selected per-piece by `chord_engine::pick_piece_cell` off (edge_activity, complexity),
/// stamped on every section's orchestration, and READ BY `realize_rhythm` — so the honest rhythmic
/// observable is `section.motto().cell_index`, NOT the (always-empty on real photos) `themes[0]`.
/// The motto is uniform across sections (slice-1 grain), so section 0 is canonical. RNG-independent
/// (the cell is a pure function of the two scalar features, not the harmony draw).
///
/// fix-direction-2 D-CELL un-gates the cell axis to run per-piece; the no-theme rhythmic observable
/// is now `section.motto().cell_index`. The S52 honesty invariant is PRESERVED — strengthened, even:
/// the engine now ACTUALLY READS this cell (it biases onset placement in `realize_rhythm`), whereas
/// the old `themes[0]` observable was a cell the realizer never saw on the real path.
fn realized_cell(plan: &CompositionPlan, _u: &ImageUnderstanding) -> usize {
    plan.sections
        .first()
        .expect("a plan has at least one section")
        .motto()
        .cell_index
        .expect("after slice 1 every real-image section carries a live per-piece motto cell")
}

// ─────────────────────────────────────────────────────────────────────────────────────────────
// (2) THE REALIZED ONSET PATTERN / IOI PROFILE — off the PURE realizer `realize_step`.
//
// A behaviour-neutral single-section/identity plan (density 0.5 ⇒ zero density nudge, identity
// orchestration ⇒ zero prominence shift), so the realized melody onset shape is governed purely by
// the (spread) edge-activity band — the exact gait a listener hears. AFTER SLICE 1 the motto biases
// these same onsets, so this observable stays measurable unchanged; the A/B compares the per-probe
// onset signatures before vs after.
// ─────────────────────────────────────────────────────────────────────────────────────────────

const STEP_MS: u64 = 200;

fn onset_step() -> StepPlan {
    StepPlan {
        chord: Chord {
            name: "I".to_string(),
            notes: vec![60, 64, 67],
        },
        phrase_index: 0,
        position_in_phrase: 0,
        phrase_len: 8,
        position: PhrasePosition::Interior,
        velocity: 76,
    }
}

fn onset_section(step: &StepPlan) -> Section {
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
        orchestration: OrchestrationProfile::identity(),
        steps: vec![step.clone()],
    }
}

fn onset_key_tempo() -> KeyTempoPlan {
    KeyTempoPlan {
        home_root_midi: 60,
        home_mode: "Ionian".to_string(),
        base_ms_per_step: STEP_MS,
        key_scheme: vec![0],
        tempo_scheme: vec![STEP_MS],
    }
}

fn perf_for(u: &ImageUnderstanding) -> PerfFeatures {
    PerfFeatures {
        saturation: u.avg_saturation,
        brightness: u.avg_brightness,
        edge_density: (u.edge_activity * EDGE_RANGE_MAX).clamp(0.0, 1.0),
    }
}

/// The realized melody onset events for a probe, RNG-free, off the pure realizer.
/// `num_instruments == 1` ⇒ the lone instrument is the Melody (the foreground line under test).
fn onset_events(u: &ImageUnderstanding) -> Vec<NoteEvent> {
    let step = onset_step();
    let sec = onset_section(&step);
    let kt = onset_key_tempo();
    let ctx = StepContext::single_section_default(&sec, &kt);
    realize_step(&step, 0, 1, &perf_for(u), STEP_MS, &ctx)
}

/// The onset SIGNATURE: the sorted `(offset_ms, hold_ms)` pairs — the audible rhythmic gait.
fn onset_signature(u: &ImageUnderstanding) -> Vec<(u64, u64)> {
    let mut sig: Vec<(u64, u64)> = onset_events(u)
        .iter()
        .map(|e| (e.offset_ms, e.hold_ms))
        .collect();
    sig.sort_unstable();
    sig
}

/// Inter-onset intervals (ms) — the IOI profile. Sorted onset offsets, successive differences.
fn ioi_profile(u: &ImageUnderstanding) -> Vec<u64> {
    let mut offs: Vec<u64> = onset_events(u).iter().map(|e| e.offset_ms).collect();
    offs.sort_unstable();
    offs.windows(2).map(|w| w[1] - w[0]).collect()
}

/// Onset density: onsets per step (the count, since the section is one step wide).
fn onset_density(u: &ImageUnderstanding) -> usize {
    onset_events(u).len()
}

// ─────────────────────────────────────────────────────────────────────────────────────────────
// The full per-probe baseline row, computed ONCE.
// ─────────────────────────────────────────────────────────────────────────────────────────────

struct Baseline {
    name: &'static str,
    complexity: f32,
    edge_activity: f32,
    cell: usize,          // realized_cell — NO_THEME_CELL or 0..=3
    onset_density: usize, // onsets the melody realizes this step
    onset_signature: Vec<(u64, u64)>,
    ioi: Vec<u64>,
}

fn baseline() -> Vec<Baseline> {
    let m = mappings();
    let planner = CompositionPlanner::new(plan_mappings(&m));
    probes()
        .into_iter()
        .map(|p| {
            set_composition_seed(Some(SEED));
            let plan = planner.plan(&p.u, &m);
            let b = Baseline {
                name: p.name,
                complexity: p.u.complexity,
                edge_activity: p.u.edge_activity,
                cell: realized_cell(&plan, &p.u),
                onset_density: onset_density(&p.u),
                onset_signature: onset_signature(&p.u),
                ioi: ioi_profile(&p.u),
            };
            eprintln!(
                "{:20} cplx={:.3} edge={:.3} -> cell {:>4} | onsets {} | sig {:?} | ioi {:?}",
                b.name,
                b.complexity,
                b.edge_activity,
                if b.cell == NO_THEME_CELL {
                    "NONE".to_string()
                } else {
                    b.cell.to_string()
                },
                b.onset_density,
                b.onset_signature,
                b.ioi,
            );
            b
        })
        .collect()
}

// ═════════════════════════════════════════════════════════════════════════════════════════════
// TEST 1 — THE DORMANT CELL-AXIS SCORECARD (the headline baseline).
//
// PROPERTY VALIDATED: today the rhythm-cell axis is DORMANT/COLLAPSED on the six real probes — it
// does NOT differentiate them. This is the BEFORE state the A/B measures against.
// ═════════════════════════════════════════════════════════════════════════════════════════════

/// PROPERTY (FLIPPED for fix-direction-2 SLICE 1): the rhythm-cell axis is now LIVE per-piece —
/// every probe carries a real per-piece motto cell, selected by `pick_piece_cell` off (edge_activity,
/// complexity) and read by `realize_rhythm`. The four formerly-NONE photos gained real cells and the
/// two formerly-force-pinned probes now flow through the same un-gated selector.
///
/// BEFORE (dormant, clean HEAD 64b6883): four photos NONE, example/magicstudio force-pinned to 3.
/// AFTER SLICE 1 (the AFTER column of the A/B — the Affect-adjudicated target table, design-s53
/// §1.4 / cell-affect §2.2):
///   AudioHaxImg1.jpg   edge 0.301 cplx 0.005 -> cell 1  (calm → broadest gait)
///   AudioHaxImg2.jpg   edge 0.509 cplx 0.015 -> cell 0  (mid → S39 anchor)
///   AudioHaxImg3.jpg   edge 0.475 cplx 0.229 -> cell 3  (intricate → profiled; SPLIT from Lena)
///   example.jpg        edge 0.719 cplx 0.905 -> cell 2  (busy outlier → even; cell-2 guard holds)
///   Lena.png           edge 0.471 cplx 0.164 -> cell 0  (mid → S39 anchor)
///   magicstudio-art.jpg edge 0.106 cplx 1.000 -> cell 3 (still-but-intricate → profiled)
/// → 4 distinct cells {0,1,2,3}, exactly 2 on cell 3 (GR-3 selective).
#[test]
fn slice1_per_probe_cell_is_live_per_piece() {
    let b = baseline();

    // The exact per-piece motto table (name, expected cell) — the AFTER column.
    let expected: [(&str, usize); 6] = [
        ("AudioHaxImg1.jpg", 1),
        ("AudioHaxImg2.jpg", 0),
        ("AudioHaxImg3.jpg", 3),
        ("example.jpg", 2),
        ("Lena.png", 0),
        ("magicstudio-art.jpg", 3),
    ];
    for (name, want) in expected {
        let row = b
            .iter()
            .find(|x| x.name == name)
            .expect("probe in baseline");
        assert_eq!(
            row.cell, want,
            "S53 SLICE-1 CELL DRIFT: {name} realizes per-piece cell {} but the Affect target table \
             expects {}. The un-gated `pick_piece_cell` driver must reproduce the adjudicated table.",
            row.cell, want,
        );
    }

    // Every probe now carries a REAL cell (the axis is no longer dormant on any of them) — none is
    // the absent-cell sentinel.
    for row in &b {
        assert_ne!(
            row.cell, NO_THEME_CELL,
            "{} must carry a LIVE per-piece motto cell after slice 1 (the axis is no longer dormant)",
            row.name,
        );
    }
}

/// PROPERTY (FLIPPED): the cell axis now carries real per-probe signal — across the six probes the
/// set of distinct cell values is >= 3 (the distinctness GAIN; it is exactly 4 here: {0,1,2,3}). A
/// re-collapse to fewer cells would fail this gate loudly (the driver-tuning guard, design-s53 R-4).
///
/// BEFORE (dormant): distinct real cells == {3} (count 1). AFTER SLICE 1: 4 distinct cells occupied.
#[test]
fn slice1_cell_axis_carries_signal_at_least_three_distinct_cells() {
    let b = baseline();
    let distinct: BTreeSet<usize> = b.iter().map(|x| x.cell).collect();

    assert!(
        distinct.len() >= 3,
        "S53 SLICE-1: the six probes must occupy >= 3 distinct per-piece cells (the un-gated \
         distinctness GAIN); got {} distinct: {distinct:?}. A re-collapse is a driver-tuning \
         failure, not a re-baseline.",
        distinct.len(),
    );
    // The slice's target is the full four-cell occupancy {0,1,2,3} — assert the exact set so a
    // regression that drops a cell (e.g. losing the cell-2 guard) is caught.
    assert_eq!(
        distinct,
        BTreeSet::from([0, 1, 2, 3]),
        "the per-piece motto must occupy all four cells {{0,1,2,3}} on the six probes; got {distinct:?}",
    );
}

// ═════════════════════════════════════════════════════════════════════════════════════════════
// TEST 2 — THE PAIRWISE-DISTINCTNESS SUMMARY (the single A/B-comparable number).
//
// PROPERTY VALIDATED: on the CELL axis the six pieces are highly SIMILAR today (low distinctness).
// Captured as one number the post-build run compares against.
// ═════════════════════════════════════════════════════════════════════════════════════════════

/// Two probes' cell labels "match" (share a gait on the cell axis) when their cell values are
/// equal — INCLUDING two NONE probes (both have the same absent-cell gait today) and two cell-3
/// probes. This is deliberately the WORST-CASE similarity reading: every probe-pair that does not
/// distinguish on the cell axis counts as a similarity, so the baseline fraction is the HIGH-water
/// dormant number the post-build run must beat.
fn cell_pair_matches(a: usize, b: usize) -> bool {
    a == b
}

/// PROPERTY (FLIPPED): the PAIRWISE CELL DISTINCTNESS is HIGH after slice 1 — the matching-pair
/// count DROPPED from the dormant 7 to 2, the A/B's headline distinctness GAIN.
///
/// BEFORE (dormant, clean HEAD): 7 of 15 pairs matched (6 NONE-pairs + 1 cell-3-pair).
/// AFTER SLICE 1: with cells {Img1:1, Img2:0, Img3:3, example:2, Lena:0, magicstudio:3} the only
/// matching pairs are (Img2,Lena both 0) and (Img3,magicstudio both 3) ⇒ 2 of 15. Both shared-cell
/// pairs remain distinct on BAND and CHARACTER (the documented-acceptable ties, cell-affect §2.2):
/// Lena(DOTTED/scherzo) vs Img2(SYNC/hymn); magicstudio(SUSTAINED/march) vs Img3(DOTTED/scherzo).
///
/// THE SINGLE A/B NUMBER: matching pairs 7 → 2 (of 15). The slice succeeded.
#[test]
fn slice1_pairwise_cell_distinctness_is_high_two_of_fifteen_pairs_match() {
    const TOTAL_PAIRS: usize = 15; // C(6,2)
    /// The dormant high-water mark the slice had to beat (pinned for the A/B record).
    const DORMANT_MATCHING_PAIRS: usize = 7;

    let b = baseline();
    let mut matching = 0usize;
    let mut total = 0usize;
    for i in 0..b.len() {
        for j in (i + 1)..b.len() {
            total += 1;
            if cell_pair_matches(b[i].cell, b[j].cell) {
                matching += 1;
            }
        }
    }
    assert_eq!(total, TOTAL_PAIRS, "C(6,2) must be 15 pairs");
    assert!(
        matching <= 4,
        "S53 SLICE-1 PAIRWISE-CELL: {matching} of {total} probe-pairs share a cell label; after \
         the un-gating this must be <= 4 (dropped from the dormant {DORMANT_MATCHING_PAIRS}). A \
         higher count means the driver re-collapsed gaits.",
    );
    // Pin the exact realized count so a regression toward sameness is caught precisely.
    assert_eq!(
        matching, 2,
        "the un-gated motto yields exactly 2 matching pairs (Img2/Lena on cell 0; Img3/magicstudio \
         on cell 3 — both distinct on band+character); got {matching}",
    );

    let similarity = matching as f64 / total as f64;
    eprintln!(
        "SLICE-1 pairwise cell similarity = {matching}/{total} = {similarity:.3} \
         (distinctness {:.3}); DROPPED from the dormant {DORMANT_MATCHING_PAIRS}/15.",
        1.0 - similarity
    );
}

// ═════════════════════════════════════════════════════════════════════════════════════════════
// TEST 3 — THE REALIZED ONSET / IOI BASELINE (the audible gait, off the pure realizer).
//
// PROPERTY VALIDATED: today the per-probe rhythmic GAIT (onset pattern + density) varies across the
// six — but that variation comes ENTIRELY from the BAND LADDER (edge-activity → onset count), NOT
// from the cell axis (which is dormant per Test 1). Captured so the post-build A/B can show the
// motto reshaping these onsets. This is the honest "what does the cell axis contribute today"
// control: the band ladder already separates the gaits; the cell adds NOTHING on the real path.
// ═════════════════════════════════════════════════════════════════════════════════════════════

/// PROPERTY: the per-probe realized onset SIGNATURE baseline (off the pure realizer). Records the
/// exact `(offset_ms, hold_ms)` onset pattern, onset density, and IOI profile each probe produces
/// TODAY. This is the audible rhythmic gait the motto will bias after slice 1 — the BEFORE onset
/// table of the A/B. The variation here is BAND-LADDER-driven, independent of the dormant cell axis.
///
/// BASELINE onset DENSITY (onsets/step, clean HEAD — the band ladder fanning the activity cluster):
///   AudioHaxImg1.jpg   edge 0.301  ->  1 onset   (SUSTAINED band)
///   AudioHaxImg2.jpg   edge 0.509  ->  2 onsets  (DOTTED/SYNCOPATED band)
///   AudioHaxImg3.jpg   edge 0.475  ->  2 onsets
///   example.jpg        edge 0.719  ->  3 onsets  (ARPEGGIO band — busiest)
///   Lena.png           edge 0.471  ->  2 onsets
///   magicstudio-art.jpg edge 0.106 ->  1 onset   (SUSTAINED band — calmest)
///
/// AFTER SLICE 1: re-measure these signatures with the motto live; the motto must re-place onsets
/// WITHIN the band the ladder chose (design-s53 I-4/I-5) — so the onset COUNT stays band-governed
/// but the offsets/holds shift to the cell's gait. The A/B compares these signatures before/after.
#[test]
fn baseline_realized_onset_signatures_are_band_ladder_driven() {
    let b = baseline();

    // The exact onset-density table (the BEFORE column). Pins the band-ladder gait per probe.
    let expected_density: [(&str, usize); 6] = [
        ("AudioHaxImg1.jpg", 1),
        ("AudioHaxImg2.jpg", 2),
        ("AudioHaxImg3.jpg", 2),
        ("example.jpg", 3),
        ("Lena.png", 2),
        ("magicstudio-art.jpg", 1),
    ];
    for (name, want) in expected_density {
        let row = b
            .iter()
            .find(|x| x.name == name)
            .expect("probe in baseline");
        assert_eq!(
            row.onset_density, want,
            "S53 ONSET BASELINE DRIFT: {name} realizes {} onset(s) but the band-ladder baseline \
             expects {want}. signature {:?}",
            row.onset_density, row.onset_signature,
        );
        // Every onset signature is non-empty and the IOI profile has (count-1) intervals.
        assert!(
            !row.onset_signature.is_empty(),
            "{name} must realize at least one onset"
        );
        assert_eq!(
            row.ioi.len(),
            row.onset_density.saturating_sub(1),
            "{name} IOI profile must have (onsets-1) intervals",
        );
    }

    // The band ladder ALREADY separates the gaits: onset density spans >= 3 distinct values today
    // (1 / 2 / 3). The point of the baseline is that this separation is BAND-driven and the cell
    // axis adds nothing — so a post-build run can attribute any NEW onset-placement variation to
    // the motto, not the band.
    let densities: BTreeSet<usize> = b.iter().map(|x| x.onset_density).collect();
    assert!(
        densities.len() >= 3,
        "the band ladder should already fan onset density across >= 3 values (the control the \
         cell axis is measured against); got {densities:?}",
    );
}

// ═════════════════════════════════════════════════════════════════════════════════════════════
// TEST 4 — DETERMINISM (the RNG discipline; every baseline observable replays identically).
// ═════════════════════════════════════════════════════════════════════════════════════════════

/// PROPERTY: every observable this baseline captures is a deterministic function of the in-memory
/// understanding (the cell off the seeded planner, RNG-independent of the harmony draw; the onset
/// signature off the pure realizer). Two passes produce byte-identical baselines — the same
/// boundary discipline as rhythm_variety_s50::rhythm_surfaces_are_deterministic, and a precondition
/// for using these as A/B anchors.
#[test]
fn baseline_observables_are_deterministic() {
    let a = baseline();
    let c = baseline();
    let sig = |v: &[Baseline]| {
        v.iter()
            .map(|x| {
                (
                    x.name,
                    x.cell,
                    x.onset_density,
                    x.onset_signature.clone(),
                    x.ioi.clone(),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(
        sig(&a),
        sig(&c),
        "the per-probe S53 baseline must be deterministic (cell + onset signature are both \
         RNG-independent of the per-section harmony draw)",
    );
}

// ═════════════════════════════════════════════════════════════════════════════════════════════
// TEST 5 — THE MOTTO ONSET BIAS IS LIVE AND GUARD-COMPLIANT (the deliverable's audible effect).
//
// PROPERTY VALIDATED: a non-neutral per-piece motto ACTUALLY re-places the melody's onsets in
// `realize_rhythm` (it is not dead) — AND it does so count-preserving, with the downbeat anchor
// fixed (GR-2 readability) and a guaranteed no-op under the neutral motto (the freeze hinge).
// This is the live half of the A/B: the motto reshapes the SAME onsets the band ladder chose.
// ═════════════════════════════════════════════════════════════════════════════════════════════

/// A melody step on a BUSY band (high edge → ARPEGGIO, 3 onsets) so there are interior onsets the
/// motto can displace. Interior phrase position (not cadence / not pre-cadence / not phrase-start)
/// so the bias is applied at FULL strength (GR-2 relaxation does not fire).
fn busy_melody_perf() -> PerfFeatures {
    PerfFeatures {
        // edge 0.85 → spread > BUSY → the ARPEGGIO band (3 onsets) on the Melody arm.
        saturation: 60.0,
        brightness: 55.0,
        edge_density: (0.85_f32 * EDGE_RANGE_MAX).clamp(0.0, 1.0),
    }
}

/// Realize the lone-melody onsets for a section whose orchestration carries `motto`.
fn melody_onsets_with_motto(motto_cell: Option<usize>) -> Vec<(u64, u64)> {
    let step = StepPlan {
        chord: Chord {
            name: "I".to_string(),
            notes: vec![60, 64, 67],
        },
        phrase_index: 0,
        position_in_phrase: 2, // interior: not start (0), not pre-cadence (phrase_len 8)
        phrase_len: 8,
        position: PhrasePosition::Interior,
        velocity: 76,
    };
    let mut sec = onset_section(&step);
    // Seat the per-piece motto on the section's orchestration (the carrier the planner uses).
    sec.orchestration.motto = match motto_cell {
        Some(c) => RhythmMotto {
            archetype: MotifArchetype::Arch,
            cell_index: Some(c),
        },
        None => RhythmMotto::neutral(),
    };
    let kt = onset_key_tempo();
    let ctx = StepContext::single_section_default(&sec, &kt);
    let mut sig: Vec<(u64, u64)> = realize_step(&step, 0, 1, &busy_melody_perf(), STEP_MS, &ctx)
        .iter()
        .map(|e| (e.offset_ms, e.hold_ms))
        .collect();
    sig.sort_unstable();
    sig
}

/// PROPERTY: the cell-3 (profiled/syncopated) motto DISPLACES an interior melody onset off the grid
/// — the gait is audible — while (a) preserving the onset COUNT, (b) leaving the FIRST onset (the
/// downbeat anchor at offset 0) in place, and (c) being a strict no-op under the neutral motto.
#[test]
fn slice1_motto_biases_interior_onsets_count_preserving_and_anchored() {
    let neutral = melody_onsets_with_motto(None);
    let profiled = melody_onsets_with_motto(Some(3)); // cell 3 — the full later (syncopating) lean

    // Sanity: the busy band gave us >= 2 onsets so there IS an interior onset to bias.
    assert!(
        neutral.len() >= 2,
        "the busy band must realize >= 2 onsets for the bias to be observable; got {neutral:?}"
    );

    // (a) COUNT-PRESERVING — the motto re-places, never adds/removes onsets (GR-1).
    assert_eq!(
        neutral.len(),
        profiled.len(),
        "the motto must preserve onset COUNT (re-place, never add/remove); neutral {neutral:?} vs \
         profiled {profiled:?}"
    );

    // (b) THE DOWNBEAT ANCHOR (first onset) is unchanged — the beat-one reference is never blurred.
    assert_eq!(
        neutral[0].0, profiled[0].0,
        "the first onset (downbeat anchor) must stay put; neutral {:?} profiled {:?}",
        neutral[0], profiled[0]
    );

    // (c) THE GAIT IS AUDIBLE — at least one interior onset moved (the cell-3 lean is not dead).
    assert_ne!(
        neutral, profiled,
        "the cell-3 motto must AUDIBLY re-place at least one interior onset (the deliverable); \
         neutral {neutral:?} == profiled {profiled:?} means the bias is dead"
    );

    // (d) FREEZE HINGE — the neutral motto reproduces the EXACT same path as no-motto (it is the
    //     value every legacy/identity section carries). Re-deriving neutral twice is byte-stable.
    assert_eq!(
        neutral,
        melody_onsets_with_motto(None),
        "the neutral motto must be a deterministic no-op (the freeze hinge)"
    );
}

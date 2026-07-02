//! S62 MOTIVIC-DERIVATION DIAGNOSTIC (read-only; Music-Theory-owned craft).
//!
//! Operator report after listening across several images: "pretty much the same
//! motif with minor variations for each piece." This harness LOCATES and QUANTIFIES
//! where melodic/motivic material is derived in the image→music pipeline and prints,
//! for each of the six bundled images, the actual motif the derivation emits — as
//! DATA, so cross-piece sameness (or diversity) is visible rather than asserted.
//!
//! WHAT IS REAL vs RECONSTRUCTED (stated honestly):
//!   * REAL planner path — `CompositionPlanner::plan(&u, &m)` is run per image and its
//!     `plan.themes` inspected. This is the authoritative pitched-motif seam. On real
//!     photos it is (as the S53 notes already record) frequently EMPTY, because the
//!     pitched theme is gated at `complexity >= 0.4` AND a form that references a theme
//!     slot; we print whether the theme fired.
//!   * FAITHFUL RECONSTRUCTION — `pick_archetype`, `affect_composite`, `range_degrees`,
//!     `length_steps` and `pick_rhythm_cell` are PRIVATE to `composition.rs`, so this
//!     integration test (an external crate) re-implements them byte-faithfully from the
//!     shipped source + the shipped `assets/mappings.json` affect weights, then calls the
//!     PUBLIC `chord_engine::resolve_motif_celled` (the ONE real contour→MotifNote place).
//!     This lets us show the motif EVERY image maps to, including images the theme gate
//!     would block. The reconstruction is cross-validated against the real planner on any
//!     image whose theme actually fires (see `motif_diag_cross_validates_real_planner`).
//!
//! Run: `cargo test --release motif_diag_s62 -- --nocapture` (DEFAULT features).

use audiohax::chord_engine::{resolve_motif_celled, MotifArchetype, MotifNote};
use audiohax::composition::{CompositionPlanner, ImageUnderstanding, PlanMappings};
use audiohax::mapping_loader::{load_mappings, MappingTable};
use audiohax::pure_analysis::{load_pure_image, understand_image_pure, PureImageSource};

const MAPPINGS_PATH: &str = "assets/mappings.json";

const IMAGES: &[&str] = &[
    "AudioHaxImg1.jpg",
    "AudioHaxImg2.jpg",
    "AudioHaxImg3.jpg",
    "example.jpg",
    "Lena.png",
    "magicstudio-art.jpg",
];

// ── Shipped affect weights (assets/mappings.json → composition.affect). Hardcoded here so the
// reconstruction is self-contained and explicit; kept identical to the JSON. affect_composite
// normalizes avg_saturation/avg_brightness by /100, feeds fg_bg_contrast through 0.5+0.5*x, and
// takes colorfulness/edge_activity/complexity raw. Empty map → neutral 0.5 (not the case here).
fn affect_arousal(u: &ImageUnderstanding) -> f32 {
    (0.45 * (u.avg_saturation / 100.0)
        + 0.25 * u.colorfulness
        + 0.20 * u.edge_activity
        + 0.10 * u.complexity)
        .clamp(0.0, 1.0)
}
fn affect_valence(u: &ImageUnderstanding) -> f32 {
    (0.70 * (u.avg_brightness / 100.0)
        + 0.20 * (u.avg_saturation / 100.0)
        + 0.10 * (0.5 + 0.5 * u.fg_bg_contrast))
        .clamp(0.0, 1.0)
}

// ── Faithful reconstruction of composition::pick_archetype (composition.rs:1767). ──
fn pick_archetype_recon(
    arousal: f32,
    valence: f32,
    vertical_emphasis: f32,
    hue: f32,
) -> MotifArchetype {
    let high_arousal = arousal >= 0.5;
    let high_valence = valence >= 0.5;
    let upper = vertical_emphasis >= 0.5;
    let warm = hue.rem_euclid(360.0) < 180.0;
    match (high_arousal, high_valence) {
        (true, true) => {
            if upper || warm {
                MotifArchetype::RisingSequence
            } else {
                MotifArchetype::Ascent
            }
        }
        (false, true) => {
            if upper || warm {
                MotifArchetype::Arch
            } else {
                MotifArchetype::InvertedArch
            }
        }
        (false, false) => {
            if upper || warm {
                MotifArchetype::LeapStep
            } else {
                MotifArchetype::Descent
            }
        }
        (true, false) => {
            if upper || warm {
                MotifArchetype::Pendulum
            } else {
                MotifArchetype::NeighborTurn
            }
        }
    }
}

// range_degrees / length_steps exactly as composition.rs:1514-1518.
fn range_degrees_recon(u: &ImageUnderstanding) -> u8 {
    (2.0 + u.edge_activity * 5.0).round() as u8
}
// S62 fix-direction-2: the planner now FLOORS length at MOTIF_LENGTH_FLOOR (5) so newly-themed
// calm photos read as a shape, not a 2-3-note stub. Mirror the floor here so the reconstruction
// stays byte-faithful to the real planner (which is now cross-validated on ALL SIX images, since
// fix-direction-1 un-gated the theme). Kept identical to composition.rs::MOTIF_LENGTH_FLOOR.
const MOTIF_LENGTH_FLOOR: usize = 5;
fn length_steps_recon(u: &ImageUnderstanding) -> usize {
    ((3.0 + u.complexity * 5.0).round() as usize).max(MOTIF_LENGTH_FLOOR)
}

// pick_rhythm_cell exactly as composition.rs:1869 (CELL_EDGE_BROAD 0.33, CELL_EDGE_BUSY 0.66,
// CELL_COMPLEXITY_PROFILED 0.66). This is the THEME-PATH cell (the pitched motif's gait).
fn pick_rhythm_cell_recon(u: &ImageUnderstanding, cell_count: usize) -> usize {
    let index = if u.complexity >= 0.66 && cell_count > 3 {
        3
    } else if u.edge_activity < 0.33 {
        1
    } else if u.edge_activity < 0.66 {
        0
    } else {
        2
    };
    index.min(cell_count.saturating_sub(1))
}

// NOTE on the theme gate: S62 fix-direction-1 UN-GATED the pitched motif — `theme_behaviour`
// default is now "fragment" (was "absent" with a complexity>=0.4 rule), so a theme is emitted for
// EVERY image whose form references a theme slot (all bundled forms do). The real planner below now
// FIRES on all six images (previously only the two complexity>=0.4 ones). This is the AFTER harness.

fn motif_to_string(m: &[MotifNote]) -> String {
    let degs: Vec<String> = m.iter().map(|n| format!("{}", n.degree)).collect();
    let durs: Vec<String> = m.iter().map(|n| format!("{}", n.dur_steps)).collect();
    format!("deg[{}] dur[{}]", degs.join(","), durs.join(","))
}

fn understanding_for(name: &str) -> ImageUnderstanding {
    let img = load_pure_image(&PureImageSource::Preselected(name.to_string()))
        .unwrap_or_else(|e| panic!("load {name}: {e}"));
    understand_image_pure(img.as_rgb()).unwrap_or_else(|e| panic!("understand {name}: {e}"))
}

fn plan_mappings(m: &MappingTable) -> PlanMappings {
    m.composition
        .clone()
        .expect("composition block present in mappings.json")
        .into()
}

/// PRIMARY DIAGNOSTIC — prints the per-image derivation chain + concrete motif as a table.
#[test]
fn motif_diag_s62_table() {
    let m = load_mappings(MAPPINGS_PATH).expect("mappings load");
    let planner = CompositionPlanner::new(plan_mappings(&m));

    println!("\n================ S62 MOTIVIC-DERIVATION DIAGNOSTIC ================\n");
    println!(
        "{:<20} {:>6} {:>6} {:>6} {:>6} {:>6} {:>6}",
        "image", "edgeA", "cplx", "sat", "bri", "arous", "valen"
    );
    println!("{}", "-".repeat(72));

    // First pass: raw drivers.
    let mut recs: Vec<(String, ImageUnderstanding, f32, f32)> = Vec::new();
    for &name in IMAGES {
        let u = understanding_for(name);
        let ar = affect_arousal(&u);
        let va = affect_valence(&u);
        println!(
            "{:<20} {:>6.3} {:>6.3} {:>6.1} {:>6.1} {:>6.3} {:>6.3}",
            name, u.edge_activity, u.complexity, u.avg_saturation, u.avg_brightness, ar, va
        );
        recs.push((name.to_string(), u, ar, va));
    }

    println!("\n{}", "-".repeat(110));
    println!(
        "{:<20} {:>7} {:>5} {:>16} {:>4} {:>4} {:>5}  MOTIF (reconstructed derivation)",
        "image", "hue", "vEmp", "archetype", "rng", "len", "cell",
    );
    println!("{}", "-".repeat(110));

    for (name, u, ar, va) in &recs {
        let arch = pick_archetype_recon(*ar, *va, u.vertical_emphasis, u.dominant_hue);
        let rng = range_degrees_recon(u);
        let len = length_steps_recon(u);
        let cell = pick_rhythm_cell_recon(u, arch.rhythm_cell_count());
        let motif = resolve_motif_celled(arch, rng, len, cell);
        println!(
            "{:<20} {:>7.1} {:>5.2} {:>16} {:>4} {:>4} {:>5}  {}",
            name,
            u.dominant_hue,
            u.vertical_emphasis,
            format!("{:?}", arch),
            rng,
            len,
            cell,
            motif_to_string(&motif)
        );
    }

    println!("\n{}", "-".repeat(110));
    println!("REAL PLANNER `plan.themes` (authoritative pitched-motif seam; theme gate = complexity>=0.4):");
    println!("{}", "-".repeat(110));
    for (name, u, _ar, _va) in &recs {
        let plan = planner.plan(u, &m);
        let fired = !plan.themes.is_empty();
        if fired {
            let motif = &plan.themes[0].motif;
            println!("{:<20} theme FIRED  ->  {}", name, motif_to_string(motif));
        } else {
            println!(
                "{:<20} theme EMPTY (complexity {:.3} < 0.40 OR form has no theme slot) -> NO pitched motif emitted",
                name, u.complexity
            );
        }
    }
    println!("\n=================================================================\n");
}

/// Cross-validation: on any image whose real theme fires, the reconstructed derivation must
/// reproduce the real planner's motif byte-for-byte (proves the reconstruction is faithful).
#[test]
fn motif_diag_cross_validates_real_planner() {
    let m = load_mappings(MAPPINGS_PATH).expect("mappings load");
    let planner = CompositionPlanner::new(plan_mappings(&m));
    let mut checked = 0usize;
    for &name in IMAGES {
        let u = understanding_for(name);
        let plan = planner.plan(&u, &m);
        if plan.themes.is_empty() {
            continue;
        }
        // Re-seat affect exactly as plan() does, then reconstruct.
        let ar = affect_arousal(&u);
        let va = affect_valence(&u);
        let arch = pick_archetype_recon(ar, va, u.vertical_emphasis, u.dominant_hue);
        let rng = range_degrees_recon(&u);
        let len = length_steps_recon(&u);
        let cell = pick_rhythm_cell_recon(&u, arch.rhythm_cell_count());
        let recon = resolve_motif_celled(arch, rng, len, cell);
        assert_eq!(
            recon, plan.themes[0].motif,
            "reconstruction diverged from real planner on {name}"
        );
        checked += 1;
    }
    // Not an assertion failure if zero fired — that itself is the finding (theme dead on real
    // photos). We simply report it via the primary table. This test only guards faithfulness
    // when at least one fires.
    println!("cross-validated reconstruction against real planner on {checked} image(s) whose theme fired");
}

/// Decision-space census: how many DISTINCT (archetype, cell) motif-identities do the six
/// images actually occupy, vs the 8×4 = 32 the vocabulary can express, vs the 8 contours.
#[test]
fn motif_diag_decision_space_census() {
    use std::collections::BTreeSet;
    let mut arches: BTreeSet<String> = BTreeSet::new();
    let mut arch_cell: BTreeSet<(String, usize)> = BTreeSet::new();
    for &name in IMAGES {
        let u = understanding_for(name);
        let ar = affect_arousal(&u);
        let va = affect_valence(&u);
        let arch = pick_archetype_recon(ar, va, u.vertical_emphasis, u.dominant_hue);
        let cell = pick_rhythm_cell_recon(&u, arch.rhythm_cell_count());
        arches.insert(format!("{:?}", arch));
        arch_cell.insert((format!("{:?}", arch), cell));
    }
    println!(
        "\nDECISION-SPACE CENSUS over 6 bundled images:\n  distinct CONTOURS occupied : {} of 8 possible  {:?}\n  distinct (contour,cell)    : {} of 32 possible {:?}\n",
        arches.len(), arches, arch_cell.len(), arch_cell
    );
}

// ═════════════════════════════════════════════════════════════════════════════════════════════
// S62 AFTER HARNESS — the post-fix per-image motif table + the DURABLE INVARIANT LOCKS.
//
// After fix-direction-1 (un-gate: theme_behaviour default "fragment") + fix-direction-2 (length
// floored at MOTIF_LENGTH_FLOOR), the REAL planner now imprints a per-image subject on all six
// images. This test both PRINTS the AFTER table (for the report / A-B) and LOCKS the new
// invariants so a future regression that re-gates the seam or drops the length floor RED-BARS here.
// ═════════════════════════════════════════════════════════════════════════════════════════════

/// The per-piece motto cell the engine actually reads (`realize_rhythm` biases onsets by it) —
/// the honest engine-side rhythmic identity (S53). Section 0 is canonical (uniform per piece).
fn motto_cell(plan: &audiohax::composition::CompositionPlan) -> Option<usize> {
    plan.sections.first().and_then(|s| s.motto().cell_index)
}

/// AFTER TABLE + INVARIANT LOCKS. Prints the post-fix motif table and asserts:
///   L1 — every image emits a NON-EMPTY ThemeSeed (fix-direction-1 un-gate).
///   L2 — every theme's Σ dur_steps >= MOTIF_LENGTH_FLOOR (fix-direction-2 length floor: no stubs).
///   L3 — contour-class spread >= 3 across the six (the archetype selector still spreads).
///   L4 — no two images share BOTH contour class AND the engine-read per-piece motto cell
///        (the P1 "no sibling" bar, evaluated on the cell the realizer actually consumes).
#[test]
fn motif_after_s62_table_and_invariants() {
    use std::collections::BTreeSet;
    let m = load_mappings(MAPPINGS_PATH).expect("mappings load");
    let planner = CompositionPlanner::new(plan_mappings(&m));

    println!("\n================ S62 AFTER — per-image motif table ================\n");
    println!(
        "{:<20} {:>14} {:>4} {:>4} {:>5} {:>6}  THEME MOTIF (real planner)",
        "image", "archetype", "rng", "len", "tcell", "motto",
    );
    println!("{}", "-".repeat(104));

    let mut archetypes: Vec<MotifArchetype> = Vec::new();
    let mut class_cell: Vec<(String, Option<usize>)> = Vec::new();

    for &name in IMAGES {
        let u = understanding_for(name);
        let plan = planner.plan(&u, &m);

        // L1 — non-empty theme for every image.
        assert!(
            !plan.themes.is_empty(),
            "L1 (UN-GATE): {name} must emit a non-empty ThemeSeed after fix-direction-1"
        );
        let motif = &plan.themes[0].motif;

        // L2 — length floor (Σ dur_steps >= MOTIF_LENGTH_FLOOR).
        let sum: usize = motif.iter().map(|n| n.dur_steps as usize).sum();
        assert!(
            sum >= MOTIF_LENGTH_FLOOR,
            "L2 (LENGTH FLOOR): {name} theme Σ dur_steps {sum} < floor {MOTIF_LENGTH_FLOOR} \
             (a stub the ear can't tell apart) — the planner floor regressed"
        );

        // Reconstruct the archetype/derivation for the table (cross-validated elsewhere).
        let ar = affect_arousal(&u);
        let va = affect_valence(&u);
        let arch = pick_archetype_recon(ar, va, u.vertical_emphasis, u.dominant_hue);
        let rng = range_degrees_recon(&u);
        let len = length_steps_recon(&u);
        let tcell = pick_rhythm_cell_recon(&u, arch.rhythm_cell_count());
        let motto = motto_cell(&plan);

        println!(
            "{:<20} {:>14} {:>4} {:>4} {:>5} {:>6}  {}",
            name,
            format!("{:?}", arch),
            rng,
            len,
            tcell,
            motto
                .map(|c| c.to_string())
                .unwrap_or_else(|| "none".into()),
            motif_to_string(motif),
        );

        archetypes.push(arch);
        class_cell.push((format!("{:?}", arch), motto));
    }

    // L3 — contour-class spread >= 3.
    let distinct_classes: BTreeSet<String> = archetypes.iter().map(|a| format!("{a:?}")).collect();
    assert!(
        distinct_classes.len() >= 3,
        "L3 (CONTOUR SPREAD): only {} distinct contour classes across six images (need >= 3): {:?}",
        distinct_classes.len(),
        distinct_classes
    );

    // L4 — no two images share BOTH contour class AND motto cell (P1 on the engine-read cell).
    for i in 0..class_cell.len() {
        for j in (i + 1)..class_cell.len() {
            assert!(
                class_cell[i] != class_cell[j],
                "L4 (NO SIBLING): {} and {} share BOTH contour class {} and motto cell {:?} — \
                 the P1 no-sibling bar (on the engine-read per-piece cell) is breached",
                IMAGES[i],
                IMAGES[j],
                class_cell[i].0,
                class_cell[i].1,
            );
        }
    }

    println!(
        "\nLOCKS: L1 non-empty(6/6)  L2 Σdur>= {}  L3 classes {}  L4 no (class,motto) sibling\n",
        MOTIF_LENGTH_FLOOR,
        distinct_classes.len()
    );
    println!("=================================================================\n");
}

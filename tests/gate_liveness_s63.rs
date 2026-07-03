//! tests/gate_liveness_s63.rs — S63 GATE-LIVENESS SCORECARD (WARN-ONLY, operator-proposed).
//!
//! ┌──────────────────────────────────────────────────────────────────────────────────────────┐
//! │ THIS TEST IS WARN-ONLY THIS SESSION. IT MUST NOT HARD-FAIL CI. It PRINTS a scorecard and   │
//! │ passes unconditionally (it asserts only that the corpus was processed). Promoting the      │
//! │ threshold-in-range signal to a HARD CI failure is a LATER OPERATOR DECISION, once the      │
//! │ thresholds are calibrated against a larger corpus — flipping it hard on the 6-image corpus │
//! │ today would over-fit. This scorecard sits BENEATH the taste/aesthetic gate: it detects     │
//! │ DEAD gates (a threshold no image can clear / a selector collapsed to one branch), it does  │
//! │ NOT and CANNOT guarantee musicality. A live gate is necessary, not sufficient, for variety.│
//! │ SOFT SIGNALS ONLY. It NEVER asserts full-branch-coverage — requiring every branch to fire  │
//! │ over-fits this 6-image corpus and invites gaming (authoring images to tick branches).      │
//! └──────────────────────────────────────────────────────────────────────────────────────────┘
//!
//! The operator's mechanism (S63): the perceived sameness traced to SELECTOR STARVATION — a rich
//! authored catalogue sits behind a SelectTable whose thresholds the real-photo cluster never
//! clears, so everything funnels onto one default entry (the S62 pitch-gate failure, and the
//! S45/S62 figuration/bass/form collapse — see docs/diagnostic-s63-vocabulary-breadth.md). This
//! harness makes that class VISIBLE with two signals per feature-driven selector:
//!
//!   (1) THRESHOLD-IN-RANGE — for each SelectTable predicate, the driving knob's observed MIN/MAX
//!       across the 6 corpus images, and whether the predicate's threshold sits INSIDE that range.
//!         • `ge`/`gt` threshold ABOVE the observed max  → DEAD (no image can clear it).
//!         • `le`/`lt` threshold BELOW the observed min  → DEAD (no image can fall under it).
//!         • `in_range` that does not overlap [min,max]  → DEAD.
//!         • threshold on the trivially-satisfied side of the whole range → ALWAYS-ON (a soft
//!           WARN: the predicate is not discriminating — it fires on every image).
//!         • otherwise → OK (live: some images clear it, some don't — the gate discriminates).
//!       A RULE is dead if ANY of its ANDed predicates is DEAD.
//!   (2) BRANCH COVERAGE — how many DISTINCT branches (picks) actually fire across the 6 images;
//!       TOTAL COLLAPSE (one branch for all six) is the dead-gate signature.
//!
//! COVERAGE (honest partial per the operator's "truthful partial beats fake-complete"): the
//! SelectTable selectors — texture (S63 un-gate), form, character, key_scheme, theme_behaviour,
//! prominence — are covered THOROUGHLY (both signals). bass_pattern (S63 un-gate) rides the
//! texture profile handle (not its own SelectTable) → branch coverage from the resolved plan;
//! its threshold analysis FOLDS INTO texture. The in-CODE selectors — motif `archetype` and
//! rhythm `cell` — are NOT SelectTables (computed in composition.rs), so threshold-in-range is
//! NOT-YET-REGISTERED for them here (branch coverage is still reported off the plan). That gap is
//! logged explicitly below rather than faked.
//!
//! Run: `cargo test --release gate_liveness_s63 -- --nocapture` (DEFAULT features).

use audiohax::composition::{
    AffectMappings, CmpOp, CompositionPlan, CompositionPlanner, ImageUnderstanding, Knob,
    PlanMappings, Predicate, SelectTable,
};
use audiohax::mapping_loader::{load_mappings, MappingTable};
use audiohax::pure_analysis::{load_pure_image, understand_image_pure, PureImageSource};
use audiohax::seed::set_composition_seed;
use std::collections::BTreeSet;

const MAPPINGS_PATH: &str = "assets/mappings.json";
const SEED: u64 = 42;
const IMAGES: &[&str] = &[
    "AudioHaxImg1.jpg",
    "AudioHaxImg2.jpg",
    "AudioHaxImg3.jpg",
    "example.jpg",
    "Lena.png",
    "magicstudio-art.jpg",
];

fn mappings() -> MappingTable {
    load_mappings(MAPPINGS_PATH).expect("assets/mappings.json loads")
}
fn plan_mappings(m: &MappingTable) -> PlanMappings {
    m.composition
        .clone()
        .expect("composition block present in mappings.json")
        .into()
}
fn understanding_for(name: &str) -> ImageUnderstanding {
    let img = load_pure_image(&PureImageSource::Preselected(name.to_string()))
        .unwrap_or_else(|e| panic!("load {name}: {e:?}"));
    understand_image_pure(img.as_rgb()).unwrap_or_else(|e| panic!("understand {name}: {e:?}"))
}

// ── Seat the affect composite EXACTLY as composition::plan does (composition.rs affect_composite),
// reading the REAL weights from mappings.json so arousal/valence-reading selectors evaluate the
// same way the planner does. (affect_composite is private; this replicates its blend faithfully
// off the public AffectMappings weight maps — no hardcoded weight drift.)
fn normalized_field(name: &str, u: &ImageUnderstanding) -> f32 {
    match name {
        "avg_saturation" => u.avg_saturation / 100.0,
        "avg_brightness" => u.avg_brightness / 100.0,
        "fg_bg_contrast" => 0.5 + 0.5 * u.fg_bg_contrast,
        "colorfulness" => u.colorfulness,
        "edge_activity" => u.edge_activity,
        "complexity" => u.complexity,
        _ => 0.0,
    }
}
fn blend(map: &std::collections::HashMap<String, f32>, u: &ImageUnderstanding) -> f32 {
    if map.is_empty() {
        return 0.5;
    }
    map.iter()
        .map(|(name, w)| w * normalized_field(name, u))
        .sum::<f32>()
        .clamp(0.0, 1.0)
}
fn seat_affect(u: &ImageUnderstanding, affect: &AffectMappings) -> ImageUnderstanding {
    let mut w = u.clone();
    w.affect_arousal = blend(&affect.arousal_weights, u);
    w.affect_valence = blend(&affect.valence_weights, u);
    w
}

/// The per-predicate liveness classification against an observed knob range.
#[derive(PartialEq)]
enum Live {
    Ok,       // discriminating: threshold inside [min,max]
    Dead,     // no image can satisfy this predicate
    AlwaysOn, // every image satisfies it (not discriminating)
}

fn classify(p: &Predicate, mn: f32, mx: f32) -> Live {
    match p.op {
        CmpOp::Ge | CmpOp::Gt => {
            if p.lo > mx {
                Live::Dead
            } else if p.lo <= mn {
                Live::AlwaysOn
            } else {
                Live::Ok
            }
        }
        CmpOp::Le | CmpOp::Lt => {
            if p.lo < mn {
                Live::Dead
            } else if p.lo >= mx {
                Live::AlwaysOn
            } else {
                Live::Ok
            }
        }
        CmpOp::InRange => {
            if p.hi < mn || p.lo > mx {
                Live::Dead
            } else if p.lo <= mn && p.hi >= mx {
                Live::AlwaysOn
            } else {
                Live::Ok
            }
        }
    }
}

fn op_str(op: CmpOp) -> &'static str {
    match op {
        CmpOp::Lt => "<",
        CmpOp::Le => "<=",
        CmpOp::Gt => ">",
        CmpOp::Ge => ">=",
        CmpOp::InRange => "in",
    }
}

/// Observed [min,max] of a knob across the seated corpus.
fn knob_range(knob: Knob, seated: &[ImageUnderstanding]) -> (f32, f32) {
    let mut mn = f32::MAX;
    let mut mx = f32::MIN;
    for u in seated {
        let v = knob.read(u);
        mn = mn.min(v);
        mx = mx.max(v);
    }
    (mn, mx)
}

/// Score one SelectTable selector: print threshold-in-range per predicate + branch coverage.
/// Returns (dead_rules, total_rules, distinct_branches).
fn score_table(
    name: &str,
    table: &SelectTable,
    driving_note: &str,
    seated: &[ImageUnderstanding],
) -> (usize, usize, usize) {
    println!("\n── SELECTOR `{name}` ({driving_note})");
    println!("   default pick: {}", table.default);

    // Branch coverage: the REAL picks over the corpus (select() on the seated understanding, as
    // plan() does).
    let picks: Vec<String> = seated.iter().map(|u| table.select(u)).collect();
    let distinct: BTreeSet<&String> = picks.iter().collect();
    let n_distinct = distinct.len();

    if table.rules.is_empty() {
        println!(
            "   WARN: TOTAL COLLAPSE — 0 rules authored; selector is PINNED to `{}` on every image (a dead gate by construction).",
            table.default
        );
    }

    // Threshold-in-range, per rule, per predicate.
    let mut dead_rules = 0usize;
    for rule in &table.rules {
        let mut rule_dead = false;
        let mut parts: Vec<String> = Vec::new();
        for p in &rule.when {
            let (mn, mx) = knob_range(p.knob, seated);
            let live = classify(p, mn, mx);
            let bound = if p.op == CmpOp::InRange {
                format!("[{:.3},{:.3}]", p.lo, p.hi)
            } else {
                format!("{:.3}", p.lo)
            };
            let tag = match live {
                Live::Ok => "OK",
                Live::Dead => {
                    rule_dead = true;
                    "DEAD"
                }
                Live::AlwaysOn => "always-on",
            };
            parts.push(format!(
                "{:?}{}{} [{tag}; obs {:.3}..{:.3}]",
                p.knob,
                op_str(p.op),
                bound,
                mn,
                mx
            ));
        }
        if rule_dead {
            dead_rules += 1;
            println!("   WARN: rule pick=`{}` is DEAD (an ANDed predicate is unreachable on the corpus):", rule.pick);
        } else {
            println!("   OK:   rule pick=`{}` reachable:", rule.pick);
        }
        for part in parts {
            println!("           {part}");
        }
    }

    // Branch-coverage verdict.
    let collapse = n_distinct <= 1;
    if collapse {
        println!(
            "   WARN: BRANCH COVERAGE = {n_distinct} distinct branch over 6 images — TOTAL COLLAPSE. picks={picks:?}"
        );
    } else {
        println!(
            "   OK:   BRANCH COVERAGE = {n_distinct} distinct branches over 6 images. picks={:?}",
            distinct
        );
    }
    (dead_rules, table.rules.len(), n_distinct)
}

#[test]
fn gate_liveness_s63_scorecard() {
    let m = mappings();
    let pm = plan_mappings(&m);

    // Seat affect on each image (as plan() does) so arousal/valence-reading selectors evaluate
    // correctly; also build the real plan per image for the plan-derived selectors (bass/archetype).
    let seated: Vec<ImageUnderstanding> = IMAGES
        .iter()
        .map(|n| seat_affect(&understanding_for(n), &pm.affect))
        .collect();

    let plans: Vec<CompositionPlan> = IMAGES
        .iter()
        .map(|n| {
            let u = understanding_for(n);
            let planner = CompositionPlanner::new(pm.clone());
            set_composition_seed(Some(SEED));
            planner.plan(&u, &m)
        })
        .collect();

    println!("\n================= S63 GATE-LIVENESS SCORECARD (WARN-ONLY) =================");
    println!(
        "corpus: {} images | seed {SEED} | reads thresholds from assets/mappings.json SelectTables",
        IMAGES.len()
    );
    println!("(warn-only: PRINTS signals; passes unconditionally; sits beneath the taste gate.)");

    // ── SelectTable selectors — covered thoroughly. ──
    let mut warn_lines = 0usize;
    for (name, table, note) in [
        (
            "texture",
            &pm.texture,
            "S63 UN-GATE; drives Pad figuration + rides bass_pattern; reads arousal/valence/colorfulness/energy/contrast",
        ),
        ("form", &pm.form, "reads complexity/edge/quadrant/aspect/vertical/value_key"),
        ("character", &pm.character, "reads arousal/valence (affect composite)"),
        ("key_scheme", &pm.key_scheme, "reads complexity/edge/quadrant/aspect/vertical/value_key + fg_bg_contrast"),
        ("theme_behaviour", &pm.theme_behaviour, "reads — (pinned selector)"),
        ("prominence", &pm.prominence, "reads subject_size/fg_bg_contrast"),
    ] {
        let (dead, total, distinct) = score_table(name, table, note, &seated);
        if dead > 0 || distinct <= 1 || total == 0 {
            warn_lines += 1;
        }
    }

    // ── bass_pattern (S63 UN-GATE) — driven by the texture profile handle, not its own table. ──
    println!("\n── SELECTOR `bass_pattern` (S63 UN-GATE; driven-by: texture profile handle; threshold analysis folds into `texture` above)");
    let bass_picks: Vec<String> = plans
        .iter()
        .map(|p| {
            p.sections
                .first()
                .and_then(|s| {
                    s.orchestration
                        .bass_pattern_resolved
                        .as_ref()
                        .map(|b| b.id.clone())
                })
                .unwrap_or_else(|| "sustained(default)".into())
        })
        .collect();
    let bass_distinct: BTreeSet<&String> = bass_picks.iter().collect();
    println!(
        "   catalogue size: {} | resolved picks over 6 images: {bass_picks:?}",
        pm.bass_pattern_catalogue.len()
    );
    if bass_distinct.len() <= 1 {
        println!(
            "   WARN: BRANCH COVERAGE = {} distinct — TOTAL COLLAPSE (the pre-S63 100%-dead bass library signature).",
            bass_distinct.len()
        );
        warn_lines += 1;
    } else {
        println!(
            "   OK:   BRANCH COVERAGE = {} distinct bass patterns fired: {bass_distinct:?}",
            bass_distinct.len()
        );
    }

    // ── In-CODE selectors (NOT SelectTables) — branch coverage only; threshold-in-range NOT-YET-REGISTERED. ──
    println!("\n── IN-CODE selectors (composition.rs / chord_engine.rs — not SelectTables) ──");
    // motif archetype: read off the plan's first theme's realized motif is not an id; use the
    // per-piece motto cell + the plan.form + character as observable in-code branch signals.
    let cell_picks: Vec<String> = plans
        .iter()
        .map(|p| {
            p.sections
                .first()
                .and_then(|s| s.motto().cell_index)
                .map(|c| c.to_string())
                .unwrap_or_else(|| "neutral".into())
        })
        .collect();
    let cell_distinct: BTreeSet<&String> = cell_picks.iter().collect();
    println!(
        "   rhythm `cell` (pick_rhythm_cell): branch coverage = {} distinct over 6 (picks {cell_picks:?}) — threshold-in-range NOT-YET-REGISTERED (in-code selector).",
        cell_distinct.len()
    );
    if cell_distinct.len() <= 1 {
        println!("   WARN: rhythm cell TOTAL COLLAPSE.");
        warn_lines += 1;
    }
    println!(
        "   motif `archetype` (pick_archetype): threshold-in-range NOT-YET-REGISTERED (in-code, reads vertical_emphasis/dominant_hue/arousal/valence). Branch coverage reported by tests/vocab_diag_s63.rs (5/8 fired)."
    );

    // ── NOT-YET-REGISTERED ledger (truthful partial). ──
    println!("\n── NOT-YET-REGISTERED for threshold-in-range (logged, not faked) ──");
    println!("   • motif `archetype` — in-code (composition::pick_archetype); no SelectTable thresholds to read.");
    println!("   • rhythm `cell`     — in-code (composition::pick_rhythm_cell); branch coverage reported above.");
    println!("   • `meter`           — SelectTable exists but is affect-neutral for this corpus; omitted as low-value.");
    println!("   • bass_pattern      — thresholds fold into `texture` (handle rides the profile); branch coverage reported above.");

    println!("\nSUMMARY: {warn_lines} selector(s) raised a WARN (dead rule / collapse). This is a REPORT, not a gate.");
    println!("=========================================================================\n");

    // WARN-ONLY: the test passes unconditionally. The ONLY assertion is the trivial structural one
    // that the corpus was actually processed (so the scorecard is never silently empty).
    assert_eq!(seated.len(), IMAGES.len(), "corpus must be fully processed");
    assert_eq!(plans.len(), IMAGES.len(), "every image must yield a plan");
}

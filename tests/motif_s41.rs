//! tests/motif_s41.rs — the S41 SLICE-3 / FINDING-B RHYTHM-CELL PROPERTY NET (the "clap test").
//!
//! S41 decouples a motif's RHYTHM from its CONTOUR. Pre-S41 (S39) each `MotifArchetype` owned a
//! single `rhythm_profile()`, so two images that landed on the SAME contour archetype clapped
//! BYTE-IDENTICALLY (the gait was a pure function of the contour — the operator's "familiar by
//! the 4th image" / the clap-test defect, `docs/design-s41-findingB-rhythm-depth.md` §0). S41
//! gives each archetype a K=4 RHYTHM-CELL vocabulary (`MotifArchetype::rhythm_cells`) and the
//! planner SELECTS one cell per image from `edge_activity`/`complexity`
//! (`composition::pick_rhythm_cell`), so the SAME contour now emits DIFFERENT gaits for different
//! images.
//!
//! This net proves that decoupling OBJECTIVELY (the SUBJECTIVE "less same-y?" verdict is the
//! operator's ear + the taste/affect subagents — NOT designed for here):
//!   P1 — same contour, DIFFERENT gait (the core S39 defect, now CLOSED).
//!   P2 — cross-image rhythm SPREAD over a varied fixture set exceeds the S39 ceiling (~6 gaits).
//!   P3 — no single gait DOMINATES that fixture set beyond a cap (the selector really spreads).
//!   P4 — within ONE archetype, sweeping the selector inputs yields ≥2 (we get ≥3) gaits — the
//!        direct clap-test proxy (hold the contour, the image still moves the gait).
//!   P5 — cell-0 FREEZE ANCHOR: `resolve_motif_celled(.., 0) == resolve_motif(..)`, so cell 0 is
//!        the S39 profile byte-for-byte (a future cell-table edit disturbing index 0 fails HERE
//!        before it can move any S39 golden).
//!   P6 — the realizer CONTRACT (every `dur_steps >= 1`, Σ `dur_steps == length_steps` with the
//!        boundary clamp) holds at EVERY archetype × EVERY cell × a grid of `length_steps`.
//!
//! DETERMINISTIC + HEADLESS, same discipline as `tests/motif_s39.rs` / `composition_s15.rs`:
//! every fixture is either the PURE RNG-free planner (`CompositionPlanner::plan` +
//! `mapping_loader::load_mappings`) or a direct call to the PUBLIC `resolve_motif_celled` /
//! `resolve_motif`. No `thread_rng`-derived value is ever asserted (the `dur_steps` sequence and
//! the `degree` contour are both set BEFORE any `pick_progression` RNG inside `plan()`).
//!
//! Run under DEFAULT features (the as-built quirk: the always-on `audiohax` bin needs the default
//! pure-Rust features, so `--no-default-features` cannot RUN this):
//!     cargo test --test motif_s41

use std::collections::{BTreeMap, BTreeSet};

use audiohax::chord_engine::{resolve_motif, resolve_motif_celled, MotifArchetype, MotifNote};
use audiohax::composition::{CompositionPlanner, ImageUnderstanding, PlanMappings};
use audiohax::mapping_loader::{load_mappings, MappingTable};

// ─────────────────────────────────────────────────────────────────────────────
// Shared fixtures (loader-backed planner; direct engine calls — motif_s39 discipline)
// ─────────────────────────────────────────────────────────────────────────────

const MAPPINGS_PATH: &str = "assets/mappings.json";

fn mappings() -> MappingTable {
    load_mappings(MAPPINGS_PATH).expect("mappings.json loads")
}

fn plan_mappings(m: &MappingTable) -> PlanMappings {
    m.composition
        .clone()
        .expect("composition block present in mappings.json")
        .into()
}

/// The eight contour archetypes (the full closed vocabulary).
const ALL_ARCHETYPES: [MotifArchetype; 8] = [
    MotifArchetype::Arch,
    MotifArchetype::InvertedArch,
    MotifArchetype::Descent,
    MotifArchetype::Ascent,
    MotifArchetype::NeighborTurn,
    MotifArchetype::LeapStep,
    MotifArchetype::Pendulum,
    MotifArchetype::RisingSequence,
];

fn arch_name(a: MotifArchetype) -> &'static str {
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
}

/// An `ImageUnderstanding` naming exactly the knobs that drive the affect composite (which feeds
/// `pick_archetype`) and the rhythm-cell selector (`edge_activity`/`complexity` feed
/// `pick_rhythm_cell`); everything else at its slice-neutral default. Mirrors motif_s39's `img`.
/// (brightness/saturation/colorfulness set the affect quadrant → the contour family; edge_activity
/// + complexity set both the range/length formulas AND the rhythm-cell pick.)
#[allow(clippy::too_many_arguments)]
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

/// The `dur_steps` sequence of a motif — its RHYTHMIC identity (the clapped dimension).
fn dur_seq(motif: &[MotifNote]) -> Vec<u8> {
    motif.iter().map(|n| n.dur_steps).collect()
}

/// Identify the (archetype, cell_index) that produced a stored planner motif by matching the FULL
/// (degree, dur_steps) line against every archetype × every cell at the planner's own range/length
/// formulas — a pure, RNG-free reconstruction (the motif_s39 trick, WIDENED across cells per the
/// S41 re-bless: the planner now picks a non-zero cell, so cell 0 alone no longer matches). Returns
/// the FIRST match; the full (contour+rhythm) line is effectively unique per (archetype, cell) at a
/// fixed range/length, so a match is unambiguous.
fn identify(
    stored: &[MotifNote],
    range_degrees: u8,
    length_steps: usize,
) -> Option<(MotifArchetype, usize)> {
    for &a in &ALL_ARCHETYPES {
        for cell in 0..a.rhythm_cell_count() {
            let cand = resolve_motif_celled(a, range_degrees, length_steps, cell);
            if cand == *stored {
                return Some((a, cell));
            }
        }
    }
    None
}

/// The planner's own pure range/length formulas (composition.rs:1484/1485) — recomputed per
/// fixture because both depend on the image knobs.
fn range_for(u: &ImageUnderstanding) -> u8 {
    (2.0 + u.edge_activity * 5.0).round() as u8
}
fn length_for(u: &ImageUnderstanding) -> usize {
    (3.0 + u.complexity * 5.0).round() as usize
}

// ═════════════════════════════════════════════════════════════════════════════
// PROPERTY 1 — SAME CONTOUR, DIFFERENT GAIT (the core S39 defect, now CLOSED).
//   For a fixed archetype, two distinct cell indices yield DIFFERENT dur_steps
//   sequences — the property that COULD NOT hold pre-S41 (rhythm was archetype-fixed).
// ═════════════════════════════════════════════════════════════════════════════

/// P1: drive `resolve_motif_celled` directly (the engine half — no planner needed to prove the
/// engine can emit different gaits for one contour). For EVERY archetype at a fixed range/length,
/// the set of `dur_seq` values across its cell vocabulary contains AT LEAST 2 distinct sequences —
/// i.e. there exists a pair `(i, j)` whose gaits differ. Pre-S41 the single `rhythm_profile()`
/// made this impossible (one contour → one gait). We also assert cell 0 differs from at least one
/// later cell (the S39 anchor is genuinely escaped, not a vocabulary of duplicates).
#[test]
fn test_p1_same_contour_emits_distinct_gaits() {
    // range/length chosen so every contour is fully sampled and the Σ-cap doesn't collapse the
    // longer cells (len 8 == the planner's max length at complexity 1.0).
    let (range, len) = (4u8, 8usize);
    for &a in &ALL_ARCHETYPES {
        let count = a.rhythm_cell_count();
        assert!(count >= 1, "{}: K >= 1", arch_name(a));
        let gaits: BTreeSet<Vec<u8>> = (0..count)
            .map(|c| dur_seq(&resolve_motif_celled(a, range, len, c)))
            .collect();
        assert!(
            gaits.len() >= 2,
            "P1: {} must emit >=2 DISTINCT gaits across its {count}-cell vocabulary (the S39 \
             contour->rhythm weld is broken); saw only {:?}",
            arch_name(a),
            gaits
        );
        // The S39 anchor (cell 0) is genuinely escaped: some other cell's gait differs from it.
        let cell0 = dur_seq(&resolve_motif_celled(a, range, len, 0));
        let escapes = (1..count).any(|c| dur_seq(&resolve_motif_celled(a, range, len, c)) != cell0);
        assert!(
            escapes,
            "P1: {} must have at least one cell whose gait differs from the cell-0 S39 anchor \
             {cell0:?} (otherwise the vocabulary is all-duplicates and the clap test still \
             collapses)",
            arch_name(a)
        );
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// PROPERTY 2 — CROSS-IMAGE SPREAD over a varied fixture set exceeds the S39 ceiling.
// ═════════════════════════════════════════════════════════════════════════════

/// A varied fixture set sweeping the two SELECTOR axes (`edge_activity` × `complexity`) at
/// multiple archetype-selecting affect points (bright/dark × saturated/flat × vertical/hue). The
/// affect-corner knobs steer `pick_archetype`; the edge/complexity sweep then steers
/// `pick_rhythm_cell` WITHIN each contour, so the realized (archetype, gait) space is exercised
/// broadly. Shared by P2 and P3.
fn spread_fixtures() -> Vec<ImageUnderstanding> {
    let mut v = Vec::new();
    // EIGHT affect corners — the four affect quadrants, each in an UPPER and a LOWER
    // vertical_emphasis variant, so `pick_archetype` reaches all 8 contours (the upper/warm
    // tiebreak picks the rising/active member, the lower the settling one). Each corner is then
    // crossed with a 4-point `edge_activity` sweep (0.10/0.30/0.50/0.80 → density cells 1/1/0/2,
    // and also moves `range_degrees`) × a 4-point `complexity` sweep (0.45/0.55 below the 0.66
    // PROFILED cut, 0.80/0.95 above it → the character cell 3; complexity also moves
    // `length_steps`, so the Σ-cap realizes distinct gaits at distinct lengths). All complexity
    // points are >= 0.4 so the theme is PRESENT (`theme_behaviour` "fragment", mappings.json:163);
    // below 0.4 the theme is "absent" and there is no motif to clap. 8×4×4 = 128 fixtures spanning
    // the full contour vocabulary AND a real range/length spread — the realized (contour, cell,
    // length) gait space the clap test actually hears.
    let corners = [
        // (brightness, saturation, colorfulness, hue, vertical_emphasis)
        (90.0, 90.0, 0.9, 30.0, 0.9), // bright + saturated, upper/warm → RISING (RisingSequence)
        (90.0, 90.0, 0.9, 230.0, 0.2), // bright + saturated, lower/cool → RISING (Ascent)
        (85.0, 10.0, 0.1, 40.0, 0.8), // bright + flat, upper/warm      → ARCHED (Arch)
        (85.0, 10.0, 0.1, 250.0, 0.2), // bright + flat, lower/cool      → ARCHED (InvertedArch)
        (15.0, 12.0, 0.1, 30.0, 0.9), // dark + flat, upper/warm        → FALLING (LeapStep)
        (15.0, 12.0, 0.1, 240.0, 0.2), // dark + flat, lower/cool        → FALLING (Descent)
        (15.0, 90.0, 0.9, 20.0, 0.9), // dark + saturated, upper/warm   → OSCILLATING (Pendulum)
        (15.0, 90.0, 0.9, 230.0, 0.2), // dark + saturated, lower/cool   → OSCILLATING (NeighborTurn)
    ];
    for &(b, s, c, hue, ve) in &corners {
        for &edge in &[0.10f32, 0.30, 0.50, 0.80] {
            for &cplx in &[0.45f32, 0.55, 0.80, 0.95] {
                v.push(img(b, s, c, cplx, edge, hue, ve));
            }
        }
    }
    v
}

/// P2: across the varied fixture set, the planner realizes MANY distinct `dur_seq` values —
/// strictly MORE than the S39 effective ceiling of ~6 distinct gaits across all 8 contours.
/// THRESHOLD (>=10): derived from the vocabulary — 8 archetypes × 4 selectable cells = 32 raw
/// (contour, cell) pairs; even after Σ-cap collapse and the fact a fixture set only reaches the
/// archetypes its affect corners select, the realized distinct-gait count clears the old 6 with
/// margin. 10 is a DEFENSIBLE floor (comfortably above the S39 ceiling, comfortably below the
/// achievable count) that fails loudly if the selector ever re-collapses onto one cell. Each
/// realized (archetype, cell, dur_seq) is logged with `eprintln!` for `--nocapture`.
#[test]
fn test_p2_cross_image_spread_exceeds_s39_ceiling() {
    let m = mappings();
    let planner = CompositionPlanner::new(plan_mappings(&m));
    let fixtures = spread_fixtures();
    assert!(
        fixtures.len() >= 16,
        ">=16 varied fixtures required, have {}",
        fixtures.len()
    );

    let mut seqs: BTreeSet<Vec<u8>> = BTreeSet::new();
    for u in &fixtures {
        let plan = planner.plan(u, &m);
        assert!(!plan.themes.is_empty(), "each fixture yields a theme");
        let stored = &plan.themes[0].motif;
        let (range, len) = (range_for(u), length_for(u));
        let (a, cell) = identify(stored, range, len)
            .expect("each stored motif must match some (archetype, cell) of the S41 vocabulary");
        let seq = dur_seq(stored);
        eprintln!(
            "edge={:.2} cplx={:.2} -> {} cell {} gait {:?}",
            u.edge_activity,
            u.complexity,
            arch_name(a),
            cell,
            seq
        );
        seqs.insert(seq);
    }

    const S39_CEILING: usize = 6; // the §0/§3.1 effective distinct-gait count across all 8 contours
    assert!(
        seqs.len() >= 10,
        "P2: the fixture set must realize >=10 distinct gaits (vs the S39 ceiling of ~{S39_CEILING}); \
         saw {} distinct dur_seqs: {:?}",
        seqs.len(),
        seqs
    );
    assert!(
        seqs.len() > S39_CEILING,
        "P2: the realized gait count {} must EXCEED the S39 ceiling {S39_CEILING} (the whole point \
         of decoupling rhythm from contour)",
        seqs.len()
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// PROPERTY 3 — ≤cap single-gait share (the selector actually SPREADS, no re-collapse).
// ═════════════════════════════════════════════════════════════════════════════

/// P3: of the realized `dur_seq` values in the P2 fixture set, NO single gait exceeds a ~40% share
/// (`cap = ceil(n * 0.40)`). A pure re-collapse onto one cell (the S39 failure mode) would put one
/// gait near 100%; the selector spreading across cells keeps any single gait well under the cap.
///
/// CAP RATIONALE (40%, slightly looser than motif_s39's 30% archetype-spread bound): this set is
/// deliberately stacked toward LOW edge_activity (two of four sweep points, 0.10 and 0.30, both map
/// to cell 1) and toward HIGH complexity (half the fixtures, complexity 0.80, divert to cell 3) —
/// so a couple of gaits are intentionally over-represented by construction. 40% still proves the
/// set does NOT collapse onto a single dominant gait (which is the property under test), while not
/// being so tight that the deliberately-skewed sweep trips it. The gait that hits the cap is logged.
#[test]
fn test_p3_no_single_gait_dominates() {
    let m = mappings();
    let planner = CompositionPlanner::new(plan_mappings(&m));
    let fixtures = spread_fixtures();

    let mut counts: BTreeMap<Vec<u8>, usize> = BTreeMap::new();
    for u in &fixtures {
        let plan = planner.plan(u, &m);
        let seq = dur_seq(&plan.themes[0].motif);
        *counts.entry(seq).or_insert(0) += 1;
    }

    let n = fixtures.len();
    let cap = ((n as f32) * 0.40).ceil() as usize;
    let (top_seq, &top_share) = counts
        .iter()
        .max_by_key(|(_, &c)| c)
        .expect("non-empty fixture set");
    eprintln!(
        "P3: n={n} cap={cap} dominant gait {top_seq:?} share={top_share} (of {} distinct)",
        counts.len()
    );
    assert!(
        top_share <= cap,
        "P3: no single gait may dominate (>~40%): dominant {top_seq:?} share {top_share} of {n} \
         exceeds cap {cap}; full distribution {counts:?}"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// PROPERTY 4 — PER-SAME-CONTOUR VARIANCE FLOOR (the direct clap-test proxy).
//   Fixtures that all land on ONE archetype, swept only on the selector axes, still
//   yield >=2 (we require >=3) distinct gaits — hold the contour, the image moves the gait.
// ═════════════════════════════════════════════════════════════════════════════

/// P4: a single affect corner (held FIXED so `pick_archetype` returns ONE contour) swept across the
/// rhythm-cell selector axes — `edge_activity` ramped low/mid/high (→ cells 1/0/2) and `complexity`
/// pushed above the PROFILED cut (→ cell 3) — must yield >=3 DISTINCT gaits on that ONE archetype.
/// This is the clap-test proxy stated directly: clap two of THESE themes (same contour) and they
/// no longer share a gait. We confirm post-hoc that all swept fixtures DID land on one archetype
/// (the corner holds the quadrant), then assert the variance floor on the held-constant contour.
#[test]
fn test_p4_same_contour_image_moves_the_gait() {
    let m = mappings();
    let planner = CompositionPlanner::new(plan_mappings(&m));

    // Hold a bright+flat affect corner FIXED (low arousal / high valence → the ARCHED family) and
    // vary ONLY the two selector axes. edge_activity 0.10/0.45/0.80 ramps the density cells (1/0/2);
    // complexity 0.85 (> the 0.66 PROFILED cut) diverts to the character cell (3). Same hue/
    // brightness/saturation throughout → the affect quadrant (hence the archetype) is constant.
    let corner = |edge: f32, cplx: f32| img(85.0, 10.0, 0.1, cplx, edge, 40.0, 0.8);
    let sweep = [
        corner(0.10, 0.45), // calm, simple   → cell 1
        corner(0.45, 0.45), // mid,  simple    → cell 0 (the S39 anchor)
        corner(0.80, 0.45), // busy, simple    → cell 2
        corner(0.45, 0.85), // mid,  intricate → cell 3 (character/profiled diversion)
    ];

    // Confirm the corner holds ONE archetype across the whole sweep (the premise of the clap test:
    // we are clapping the SAME contour), and collect the distinct gaits realized on it.
    let mut contour: Option<MotifArchetype> = None;
    let mut gaits: BTreeSet<Vec<u8>> = BTreeSet::new();
    for u in &sweep {
        let plan = planner.plan(u, &m);
        let stored = &plan.themes[0].motif;
        let (range, len) = (range_for(u), length_for(u));
        let (a, cell) = identify(stored, range, len).expect("stored motif matches the vocabulary");
        eprintln!(
            "P4: edge={:.2} cplx={:.2} -> {} cell {} gait {:?}",
            u.edge_activity,
            u.complexity,
            arch_name(a),
            cell,
            dur_seq(stored)
        );
        match contour {
            None => contour = Some(a),
            Some(c) => assert_eq!(
                c,
                a,
                "P4 premise: the held affect corner must keep ONE archetype across the selector \
                 sweep (clapping the SAME contour); got {} then {}",
                arch_name(c),
                arch_name(a)
            ),
        }
        gaits.insert(dur_seq(stored));
    }

    assert!(
        gaits.len() >= 3,
        "P4 (clap-test proxy): holding the contour {} constant, the selector sweep must produce \
         >=3 distinct gaits (image moves the gait); saw {}: {:?}",
        contour.map(arch_name).unwrap_or("?"),
        gaits.len(),
        gaits
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// PROPERTY 5 — CELL-0 FREEZE ANCHOR: resolve_motif_celled(.., 0) == resolve_motif(..).
// ═════════════════════════════════════════════════════════════════════════════

/// P5: cell 0 of EVERY archetype reproduces the S39 `resolve_motif` BYTE-FOR-BYTE — i.e.
/// `resolve_motif_celled(a, r, l, 0) == resolve_motif(a, r, l)` over a grid of (range, length).
/// This pins the freeze anchor (the §6 hinge): cell 0 IS the S39 profile, and `resolve_motif` is
/// the thin `cell_index = 0` wrapper. A future cell-table edit that disturbs index 0 fails HERE,
/// before it can move any S39 golden or break the 11 existing `resolve_motif` call sites.
#[test]
fn test_p5_cell0_is_the_s39_freeze_anchor() {
    for &a in &ALL_ARCHETYPES {
        for range in [2u8, 4, 7] {
            for len in [3usize, 5, 6, 8] {
                let celled = resolve_motif_celled(a, range, len, 0);
                let s39 = resolve_motif(a, range, len);
                assert_eq!(
                    celled,
                    s39,
                    "P5 (FREEZE ANCHOR): {} cell 0 must equal resolve_motif byte-for-byte at \
                     range {range} len {len}; celled {:?} vs S39 {:?}",
                    arch_name(a),
                    dur_seq(&celled),
                    dur_seq(&s39)
                );
            }
        }
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// PROPERTY 6 — REALIZER CONTRACT preserved at EVERY cell of EVERY archetype.
// ═════════════════════════════════════════════════════════════════════════════

/// P6: the S39 `resolve_motif` invariants — every `dur_steps >= 1`, and Σ `dur_steps` fills the
/// section budget exactly (the final note absorbing the remainder per the §4 static-tail fix) —
/// now hold over the WHOLE cell vocabulary, not just the single profile. Sweep every archetype ×
/// every cell × a grid of `length_steps` (the planner's 3..=8 range plus the degenerate len 0/1/2
/// and an over-long 12 that stresses the static-tail fill). The realizer floors the budget at
/// `length_steps.max(1)` (`resolve_motif_celled`: `let len = length_steps.max(1)`), so the exact
/// contract is `Σ dur_steps == len.max(1)` and the motif is always non-empty (even at len 0 it
/// emits one note worth one step). This proves the busier/profiled cells never over-run the budget,
/// never emit a zero-length note, and always fill exactly.
#[test]
fn test_p6_contract_holds_at_every_cell() {
    for &a in &ALL_ARCHETYPES {
        let count = a.rhythm_cell_count();
        for cell in 0..count {
            for len in [0usize, 1, 2, 3, 4, 5, 6, 7, 8, 12] {
                let motif = resolve_motif_celled(a, 4, len, cell);
                let effective = len.max(1); // the realizer's floor

                // A real motif is always emitted (the budget is floored at 1).
                assert!(
                    !motif.is_empty(),
                    "P6: {} cell {cell} len {len}: a motif (>=1 note) is always emitted (budget \
                     floored at 1)",
                    arch_name(a)
                );

                // Every note carries dur_steps >= 1 (the MotifNote contract).
                assert!(
                    motif.iter().all(|n| n.dur_steps >= 1),
                    "P6: {} cell {cell} len {len}: every dur_steps must be >= 1; got {:?}",
                    arch_name(a),
                    dur_seq(&motif)
                );

                // Σ dur_steps == the effective budget exactly: the final note absorbs the
                // remainder (the static-tail fix), and no cell over-runs (the Σ-cap clamp).
                let sum: usize = motif.iter().map(|n| n.dur_steps as usize).sum();
                assert_eq!(
                    sum,
                    effective,
                    "P6: {} cell {cell} len {len}: Σ dur_steps ({sum}) must EQUAL the floored budget \
                     ({effective}) — filled exactly, no over-run; got {:?}",
                    arch_name(a),
                    dur_seq(&motif)
                );
            }
        }
    }
}

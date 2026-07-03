//! tests/phrase_scheme_s64.rs — S64 STEP-2 PROPERTY NET (Test Engineer; tests only).
//!
//! Guards the load-bearing invariants of the PHRASE-PERIOD / CADENCE-SCHEME SPREAD
//! (docs/design-s64-phrase-period-spread.md, docs/diagnostic-s64-fixed-rhythm-key.md):
//! the curated `PhraseScheme` catalogue that breaks the universal 4-step `S..hS..P`
//! period the operator transcribed as the fixed cross-piece "Wizard of Oz" skeleton.
//!
//! CROSS-PLATFORM SAFETY (the S62.1 lesson): every assertion here is STRUCTURAL /
//! INTEGER only — cadence-position SETS, scheme identities, cadence COUNTS, enum
//! equality, and BYTE-IDENTITY of `Vec<StepPlan>`. NOT ONE per-image float audio
//! metric is asserted (macOS libm diverges ~3e-4…3e-2). Where the selector reads a
//! float feature (assertion #6) the inputs are LITERAL catalogue values I control,
//! not decoded-image metrics, so the threshold comparisons are platform-exact.
//!
//! Run: `cargo test --release --test phrase_scheme_s64`

use audiohax::chord_engine::{Chord, ChordEngine, PhrasePosition, PhraseScheme};
use audiohax::composition::{ImageUnderstanding, PlanMappings};
use audiohax::mapping_loader::{load_mappings, MappingTable};
use std::collections::BTreeSet;

const MAPPINGS_PATH: &str = "assets/mappings.json";

// ─────────────────────────────────────────────────────────────────────────────
// Fixtures
// ─────────────────────────────────────────────────────────────────────────────

fn mappings() -> MappingTable {
    load_mappings(MAPPINGS_PATH).expect("assets/mappings.json loads")
}

fn engine() -> ChordEngine {
    ChordEngine::new(mappings())
}

/// The four non-default schemes (Period4 is exercised as the freeze anchor separately).
const SPREAD_SCHEMES: [PhraseScheme; 4] = [
    PhraseScheme::Period4,
    PhraseScheme::Phrase5,
    PhraseScheme::Group6,
    PhraseScheme::Sentence8,
];

/// A concrete `n`-chord progression to feed the planner. Content is deliberately a
/// real I–IV–V–I cycle (so the phrase-final predecessor is often a dominant and PACs
/// stamp honestly), but NONE of the asserted properties depend on the chord content:
/// the planner stamps a cadence at every phrase boundary regardless (a non-final
/// boundary is always a HalfCadence; a final boundary is a PAC or HalfCadence — both
/// count as cadence positions). So this is a stable, content-agnostic section builder.
fn make_chords(n: usize) -> Vec<Chord> {
    // (name, root-position triad root) — valid triads so voice-leading connects them.
    let cycle: [(&str, u8); 4] = [
        ("I", 60),  // C major
        ("IV", 65), // F major
        ("V", 67),  // G major (dominant)
        ("I", 60),  // C major
    ];
    (0..n)
        .map(|i| {
            let (name, root) = cycle[i % cycle.len()];
            Chord {
                name: name.to_string(),
                notes: vec![root, root + 4, root + 7],
            }
        })
        .collect()
}

/// Extract the INTEGER cadence-position signature of a plan: the step indices whose
/// structural role is a phrase cadence (HalfCadence or PerfectAuthenticCadence). This
/// is exactly the phrase-final geometry the pre-cadence lift and cadence ring read off
/// the `StepPlan`, so its per-scheme spread IS the audible periodicity spread.
fn cadence_positions(plan: &[audiohax::chord_engine::StepPlan]) -> BTreeSet<usize> {
    plan.iter()
        .enumerate()
        .filter(|(_, s)| {
            matches!(
                s.position,
                PhrasePosition::HalfCadence | PhrasePosition::PerfectAuthenticCadence
            )
        })
        .map(|(i, _)| i)
        .collect()
}

fn cadence_count(plan: &[audiohax::chord_engine::StepPlan]) -> usize {
    cadence_positions(plan).len()
}

/// Count DISTINCT schemes without requiring `Ord`/`Hash` on the enum (it derives only
/// `PartialEq`/`Eq`), so we dedup by linear membership instead of a set.
fn distinct_schemes(schemes: &[PhraseScheme]) -> usize {
    let mut acc: Vec<PhraseScheme> = Vec::new();
    for &s in schemes {
        if !acc.contains(&s) {
            acc.push(s);
        }
    }
    acc.len()
}

/// Build an `ImageUnderstanding` carrying LITERAL (complexity, edge_activity) — every
/// other field is the neutral baseline. Used only to drive the `phrase_scheme` selector
/// with platform-stable literal inputs (assertion #6), never decoded-image floats.
fn understanding(complexity: f32, edge_activity: f32) -> ImageUnderstanding {
    ImageUnderstanding {
        complexity,
        edge_activity,
        ..ImageUnderstanding::neutral()
    }
}

fn plan_mappings() -> PlanMappings {
    mappings()
        .composition
        .clone()
        .expect("composition block present")
        .into()
}

// ─────────────────────────────────────────────────────────────────────────────
// #1 — IDENTITY-NEUTRALITY WITNESS (the freeze guard; keeps engine_equivalence 9/9)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn period4_is_byte_identical_to_plan_phrases() {
    let eng = engine();
    // Several section lengths, incl. n=16 where the legacy rule prefers the 8-step
    // phrase — the freeze delegate must reproduce plan_phrases byte-for-byte at ALL n.
    for n in [8usize, 9, 12, 14, 16] {
        let chords = make_chords(n);
        let legacy = eng.plan_phrases(&chords);
        let via_scheme = eng.plan_phrases_scheme(&chords, PhraseScheme::Period4);
        assert_eq!(
            legacy, via_scheme,
            "plan_phrases(n={n}) must equal plan_phrases_scheme(Period4) byte-for-byte \
             (this is the engine_equivalence freeze anchor)"
        );
        // Also byte-identical against the DEFAULT scheme value (what the neutral/identity
        // path resolves to), guarding the composition::plan neutral witness.
        assert_eq!(
            legacy,
            eng.plan_phrases_scheme(&chords, PhraseScheme::default()),
            "PhraseScheme::default() must be the legacy Period4 path at n={n}"
        );
    }
    // The empty section is neutral under every scheme.
    assert!(eng.plan_phrases(&[]).is_empty());
    assert!(eng
        .plan_phrases_scheme(&[], PhraseScheme::Sentence8)
        .is_empty());
}

#[test]
fn default_and_unknown_ids_resolve_to_period4() {
    assert_eq!(PhraseScheme::default(), PhraseScheme::Period4);
    assert_eq!(PhraseScheme::from_id(""), PhraseScheme::Period4);
    assert_eq!(PhraseScheme::from_id("period_4"), PhraseScheme::Period4);
    assert_eq!(PhraseScheme::from_id("garbage"), PhraseScheme::Period4);
    // Case-sensitive parse: an unexpected casing degrades to the freeze default.
    assert_eq!(PhraseScheme::from_id("PERIOD_4"), PhraseScheme::Period4);
    assert_eq!(PhraseScheme::from_id("sentence8"), PhraseScheme::Period4);
}

// ─────────────────────────────────────────────────────────────────────────────
// #2 — from_id TOTAL MAPPING (each of the 4 ids round-trips; unknown → Period4)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn from_id_maps_every_catalogue_id() {
    assert_eq!(PhraseScheme::from_id("period_4"), PhraseScheme::Period4);
    assert_eq!(PhraseScheme::from_id("phrase_5"), PhraseScheme::Phrase5);
    assert_eq!(PhraseScheme::from_id("group_6"), PhraseScheme::Group6);
    assert_eq!(PhraseScheme::from_id("sentence_8"), PhraseScheme::Sentence8);
    // Every catalogue id must map to a DISTINCT variant (no two ids collapse together).
    let variants: Vec<PhraseScheme> = ["period_4", "phrase_5", "group_6", "sentence_8"]
        .iter()
        .map(|id| PhraseScheme::from_id(id))
        .collect();
    assert_eq!(
        distinct_schemes(&variants),
        4,
        "the four ids must map to four distinct schemes"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// #3 — CADENCE-POSITION SIGNATURE per scheme on the canonical 8-step section
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn cadence_signature_per_scheme_on_8_step_section() {
    let eng = engine();
    let chords = make_chords(8);

    let period4 = cadence_positions(&eng.plan_phrases_scheme(&chords, PhraseScheme::Period4));
    let phrase5 = cadence_positions(&eng.plan_phrases_scheme(&chords, PhraseScheme::Phrase5));
    let group6 = cadence_positions(&eng.plan_phrases_scheme(&chords, PhraseScheme::Group6));
    let sentence8 = cadence_positions(&eng.plan_phrases_scheme(&chords, PhraseScheme::Sentence8));

    assert_eq!(
        period4,
        BTreeSet::from([3usize, 7]),
        "Period4 (legacy) must cadence at {{3,7}} — the S..hS..P double period"
    );
    assert_eq!(
        phrase5,
        BTreeSet::from([4usize, 7]),
        "Phrase5 (asymmetric 5+3) must cadence at {{4,7}} — interior cadence walks one step later"
    );
    assert_eq!(
        group6,
        BTreeSet::from([5usize, 7]),
        "Group6 (broad 6+2) must cadence at {{5,7}} — first arrival deferred to step 5"
    );
    assert_eq!(
        sentence8,
        BTreeSet::from([7usize]),
        "Sentence8 (single 8-step drive) must cadence ONCE at {{7}}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// #4 — CADENCE-PERIODICITY SPREAD (>= 3 distinct signatures → S..hS..P not universal)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn cadence_periodicity_actually_varies_across_schemes() {
    let eng = engine();
    let chords = make_chords(8);

    let signatures: BTreeSet<BTreeSet<usize>> = SPREAD_SCHEMES
        .iter()
        .map(|&s| cadence_positions(&eng.plan_phrases_scheme(&chords, s)))
        .collect();

    assert!(
        signatures.len() >= 3,
        "the 4 schemes must yield >= 3 DISTINCT cadence signatures (got {}): the whole point of \
         the slice is that the cadence periodicity varies per piece — S..hS..P is no longer universal",
        signatures.len()
    );
    // Observed spread is in fact the full four {3,7}/{4,7}/{5,7}/{7}.
    assert_eq!(
        signatures.len(),
        4,
        "all four schemes should produce four distinct cadence signatures on an 8-step section"
    );
    // And the legacy {3,7} skeleton is NOT the only signature present.
    assert!(
        signatures.len() > 1 && signatures.contains(&BTreeSet::from([3usize, 7])),
        "the legacy {{3,7}} exists but is no longer the sole signature"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// #5 — CADENCE-DENSITY BOUND (F5a anti-fusion guard): no scheme adds cadence rings
//      beyond the legacy count, over the realistic corpus section-length range.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn no_scheme_exceeds_legacy_cadence_count() {
    let eng = engine();
    // Section length is BASE_STEPS_PER_SECTION(8) + round(edge_activity*8), so the
    // REACHABLE range is ~8..=14 (corpus max edge 0.719 → 14; 16 needs edge >= 0.9375,
    // no corpus image reaches it). The F5a density bound is a per-section property over
    // that range. (At the non-corpus n=16 the legacy rule prefers an 8-step phrase — only
    // 2 cadences over 16 steps — an anomalously SPARSE reference that is not the density
    // regime the F5a floor was ratcheted against; it is intentionally out of range here.)
    for n in 8..=14usize {
        let chords = make_chords(n);
        let legacy_count = cadence_count(&eng.plan_phrases_scheme(&chords, PhraseScheme::Period4));
        for &scheme in &SPREAD_SCHEMES {
            let c = cadence_count(&eng.plan_phrases_scheme(&chords, scheme));
            assert!(
                c <= legacy_count,
                "scheme {scheme:?} at n={n} produced {c} cadence steps > legacy {legacy_count}: a \
                 scheme may NOT add homophonic cadence-ring steps (S49 F5a within-piece anti-fusion floor)"
            );
        }
        // Sentence8 is the density-REDUCING scheme: it must never add cadences, and at the
        // canonical 8-step section it strictly reduces (1 vs the legacy 2).
        let sentence = cadence_count(&eng.plan_phrases_scheme(&chords, PhraseScheme::Sentence8));
        assert!(
            sentence <= legacy_count,
            "Sentence8 must be density <= legacy at n={n}"
        );
    }

    // Explicit density-REDUCTION witness at the canonical section: sentence 1 < legacy 2.
    let chords8 = make_chords(8);
    assert_eq!(
        cadence_count(&eng.plan_phrases_scheme(&chords8, PhraseScheme::Sentence8)),
        1,
        "Sentence8 collapses the double period to a SINGLE terminal cadence on an 8-step section"
    );
    assert_eq!(
        cadence_count(&eng.plan_phrases_scheme(&chords8, PhraseScheme::Period4)),
        2,
        "Period4 keeps the legacy two-cadence density on an 8-step section"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// #6 — PER-CORPUS SCHEME ASSIGNMENT via the SELECTOR LOGIC.
//
// PATH TAKEN: SELECTOR-LOGIC with LITERAL feature inputs (NOT real image decode).
// Rationale: the real corpus features ARE reachable headless (keydiag_s64 uses the
// pure_analysis path with no OpenCV), but Img3 (complexity 0.229) and Lena (0.164)
// sit only ~0.03 / ~0.06 above their selector thresholds (0.2 / 0.1) — WITHIN the
// documented macOS-libm divergence band (~3e-4…3e-2). Driving a DECODED float through
// a threshold that close is exactly the S62.1 fragility this net is forbidden from
// introducing. So we feed the `phrase_scheme` SelectTable the LITERAL (complexity,
// edge_activity) catalogue values from the verified corpus table and assert the scheme
// it returns. Literal inputs make the threshold comparisons platform-EXACT while still
// proving the load-bearing property: these features select these schemes.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn corpus_features_select_expected_schemes() {
    let pm = plan_mappings();

    // (image, complexity, edge_activity, expected scheme) — the Music-Theory-verified table.
    let corpus: [(&str, f32, f32, PhraseScheme); 6] = [
        ("AudioHaxImg1", 0.005, 0.301, PhraseScheme::Period4),
        ("AudioHaxImg2", 0.015, 0.509, PhraseScheme::Period4),
        ("AudioHaxImg3", 0.229, 0.475, PhraseScheme::Group6),
        ("example", 0.905, 0.719, PhraseScheme::Sentence8),
        ("Lena", 0.164, 0.471, PhraseScheme::Phrase5),
        ("magicstudio", 1.000, 0.106, PhraseScheme::Period4),
    ];

    let mut reached: Vec<PhraseScheme> = Vec::new();
    for (name, cx, edge, expected) in corpus {
        let u = understanding(cx, edge);
        let picked = PhraseScheme::from_id(&pm.phrase_scheme.select(&u));
        assert_eq!(
            picked, expected,
            "{name} (complexity {cx}, edge {edge}) must select {expected:?}, got {picked:?}"
        );
        reached.push(picked);
    }

    // No collapse: all four schemes are reached across the corpus.
    assert_eq!(
        distinct_schemes(&reached),
        4,
        "the corpus must reach all four schemes with no collapse (got {reached:?})"
    );

    // The neutral/identity feature vector (complexity 0) must take the freeze default.
    let neutral = PhraseScheme::from_id(&pm.phrase_scheme.select(&ImageUnderstanding::neutral()));
    assert_eq!(
        neutral,
        PhraseScheme::Period4,
        "the neutral understanding must resolve to the legacy Period4 (identity-path freeze)"
    );

    // Cross-check the per-image CADENCE SIGNATURE implied by each corpus scheme on the
    // canonical 8-step section is one of the four documented integer signatures, and that
    // the corpus carries >= 3 distinct signatures (the diagnostic's spread claim).
    let eng = engine();
    let chords = make_chords(8);
    let corpus_signatures: BTreeSet<BTreeSet<usize>> = corpus
        .iter()
        .map(|&(_, _, _, scheme)| cadence_positions(&eng.plan_phrases_scheme(&chords, scheme)))
        .collect();
    assert!(
        corpus_signatures.len() >= 3,
        "the corpus schemes must realize >= 3 distinct 8-step cadence signatures (got {})",
        corpus_signatures.len()
    );
}

//! tests/seed_s41.rs — the S41 DETERMINISTIC-SEED (`--seed`) PROPERTY NET.
//!
//! S41 lands `--seed <u64>`: a thread-local register (`src/seed.rs`) that makes the ONE
//! non-deterministic draw on the composition path — `chord_engine::ChordEngine::pick_progression`'s
//! `thread_rng()` at chord_engine.rs:132 — reproducible WITHOUT changing any caller signature and
//! WITHOUT touching frozen `src/engine.rs`. The seam:
//!   - register `Some(seed)` → `pick_progression` draws via
//!     `ChaCha8Rng::seed_from_u64(mix_seed(seed, next_seed_call()))`, advancing a per-call counter
//!     so a multi-section piece is REPRODUCIBLE run-to-run AND its sections still DECORRELATE.
//!   - register `None` (the DEFAULT, absent `--seed`) → today's exact `thread_rng()` path, unchanged.
//!
//! WHERE THE DRAW LANDS IN THE PLAN: `CompositionPlanner::plan` calls `pick_progression` ONCE per
//! section (composition.rs:1580), storing the result in `Section.progression` (a `Vec<String>` of
//! Roman numerals). So the seeded behavior is OBSERVABLE on the pure, headless plan: the ordered
//! `plan.sections[*].progression` IS the realized draw sequence, and `CompositionPlan: PartialEq`
//! lets us compare whole plans byte/structurally. We drive `CompositionPlanner::plan` (NOT the WAV
//! path) so the net is fast + headless, exactly like `tests/motif_s41.rs` / `tests/home_s40.rs`.
//!
//! PROPERTIES:
//!   PT-SEED-1 — REPRODUCIBLE: same seed, same image, two runs → structurally IDENTICAL plan
//!               (whole-plan `PartialEq` AND the per-section progression vectors).
//!   PT-SEED-2 — SEED-SENSITIVE: different seeds, same image → the progression draw DIFFERS (asserted
//!               for the 42-vs-7 pair on a real-choice image, AND across a seed set NOT-ALL-identical).
//!   PT-SEED-3 — MULTI-SECTION DECORRELATION: within ONE seeded multi-section plan the per-call
//!               counter mixing makes the sections NOT all draw the byte-identical progression
//!               (`mix_seed` truly varies per call) — AND, since section progressions CAN legitimately
//!               repeat on some images, the weaker reproducibility-at-multi-section-scale property
//!               (PT-SEED-1 at N>=3 sections) is also asserted.
//!   PT-SEED-4 — LEGACY PRESERVED: register `None` → planning still succeeds and yields a structurally
//!               valid plan (the `thread_rng` path is intact). NO determinism asserted here (thread_rng
//!               is non-deterministic) — only shape/validity.
//!   PT-SEED-5 — FREEZE GUARD: `src/engine.rs` sha256 is unchanged at the frozen value (asserted
//!               in-test by hashing the file); the 9/9 `engine_equivalence` net covers behavior.
//!
//! THREAD-LOCAL ORDERING DISCIPLINE: the seed register is a thread-local `Cell` (`src/seed.rs`).
//! `cargo test` runs test fns in parallel ACROSS threads but each fn runs on a single thread, and the
//! register is per-thread, so cross-test leakage cannot occur through it. As a belt-and-suspenders
//! guard against a runner that pins multiple tests to one thread (and against intra-test counter
//! drift — the counter advances per `pick_progression` call), EVERY test calls
//! `set_composition_seed(..)` at its START (which also resets the per-call counter, per
//! `seed::set_composition_seed`), and `Some(..)`-setting tests reset the seed again before each
//! independent run so the counter restarts from 0. No `#[serial]` / serial_test dependency is added.
//!
//! Run under DEFAULT features (the always-on `audiohax` bin needs the pure-Rust default features, so
//! `--no-default-features` cannot RUN integration tests that link the lib here):
//!     cargo test --test seed_s41

use std::collections::BTreeSet;

use audiohax::composition::{
    CompositionPlan, CompositionPlanner, ImageUnderstanding, PlanMappings,
};
use audiohax::mapping_loader::{load_mappings, MappingTable};
use audiohax::seed::set_composition_seed;

const MAPPINGS_PATH: &str = "assets/mappings.json";

/// The frozen `src/engine.rs` sha256 (the S41 freeze keystone). A change here means the engine was
/// edited — the whole point of the seam is to leave it byte-untouched.
const ENGINE_SHA256: &str = "e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261";

// ─────────────────────────────────────────────────────────────────────────────
// Shared fixtures (loader-backed planner — home_s40 / motif_s41 discipline)
// ─────────────────────────────────────────────────────────────────────────────

fn mappings() -> MappingTable {
    load_mappings(MAPPINGS_PATH).expect("mappings.json loads")
}

fn plan_mappings(m: &MappingTable) -> PlanMappings {
    m.composition
        .clone()
        .expect("composition block present in mappings.json")
        .into()
}

fn planner(m: &MappingTable) -> CompositionPlanner {
    CompositionPlanner::new(plan_mappings(m))
}

/// The probe-confirmed reference image: neutral affect → `rounded_binary` form, 3 sections, mode
/// Phrygian (cool family — 5 progression choices), and crucially the three sections draw THREE
/// DISTINCT progressions under a seed (counter decorrelation is observable). This is the image with
/// "real progression choice" PT-SEED-2/3 ask for.
fn reference_image() -> ImageUnderstanding {
    ImageUnderstanding::neutral()
}

/// A second, very different image (vivid/high-arousal) → `theme_and_variations` form, Mixolydian
/// (warm family — 7 choices). Used to widen PT coverage across forms/families.
fn vivid_image() -> ImageUnderstanding {
    ImageUnderstanding {
        avg_brightness: 85.0,
        avg_saturation: 90.0,
        colorfulness: 0.9,
        complexity: 0.95,
        edge_activity: 0.8,
        dominant_hue: 30.0,
        ..ImageUnderstanding::neutral()
    }
}

/// Plan an image under a FRESH seed (resets the per-call counter so the draw sequence restarts).
fn plan_seeded(
    p: &CompositionPlanner,
    m: &MappingTable,
    img: &ImageUnderstanding,
    seed: u64,
) -> CompositionPlan {
    set_composition_seed(Some(seed));
    p.plan(img, m)
}

/// The ordered per-section progression sequence — the realized `pick_progression` draw sequence and
/// the dimension the seed controls.
fn progressions(plan: &CompositionPlan) -> Vec<Vec<String>> {
    plan.sections
        .iter()
        .map(|s| s.progression.clone())
        .collect()
}

/// Structural-validity floor for a plan (used where determinism cannot be asserted, PT-SEED-4):
/// at least one section, total_steps consistent with the section spans, every section has a
/// non-empty progression of valid-looking Roman tokens.
fn assert_structurally_valid(plan: &CompositionPlan, label: &str) {
    assert!(
        !plan.sections.is_empty(),
        "{label}: plan must have >=1 section"
    );
    let span: usize = plan.sections.iter().map(|s| s.step_len).sum();
    assert_eq!(
        span, plan.total_steps,
        "{label}: total_steps ({}) must equal Σ section step_len ({span})",
        plan.total_steps
    );
    for (i, s) in plan.sections.iter().enumerate() {
        assert!(
            !s.progression.is_empty(),
            "{label}: section {i} ({}) must carry a non-empty progression",
            s.label
        );
        for roman in &s.progression {
            assert!(
                !roman.is_empty()
                    && roman
                        .chars()
                        .all(|c| matches!(c, 'i' | 'I' | 'v' | 'V' | 'b' | '#')),
                "{label}: section {i} progression token {roman:?} is not a plausible Roman numeral"
            );
        }
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// PT-SEED-1 — REPRODUCIBLE: same seed + same image ⇒ structurally IDENTICAL plan.
// ═════════════════════════════════════════════════════════════════════════════

/// PT-SEED-1: seed the register with the SAME value before two independent `plan()` runs of the SAME
/// image; the resulting plans must be EQUAL (whole-plan `PartialEq`) and, specifically, the
/// per-section progression draw sequence must be byte-identical. Resetting the seed before each run
/// restarts the per-call counter from 0 (per `set_composition_seed`), so the draw sequence replays.
/// Pre-S41 this could NOT hold (the draw was `thread_rng`). We verify on BOTH reference fixtures.
#[test]
fn test_pt_seed_1_same_seed_reproduces_plan() {
    set_composition_seed(Some(42)); // ordering guard: own the register from the test start.
    let m = mappings();
    let p = planner(&m);

    for (name, img) in [
        ("reference/rounded_binary", reference_image()),
        ("vivid/theme_and_variations", vivid_image()),
    ] {
        let first = plan_seeded(&p, &m, &img, 42);
        let second = plan_seeded(&p, &m, &img, 42); // re-seed ⇒ counter reset ⇒ same draw sequence
        assert_eq!(
            progressions(&first),
            progressions(&second),
            "PT-SEED-1 [{name}]: same seed must replay the IDENTICAL per-section progression \
             sequence; first {:?} vs second {:?}",
            progressions(&first),
            progressions(&second)
        );
        // The whole plan (form, sections, key/tempo spine, themes, total_steps) is structurally
        // identical — `CompositionPlan` derives `PartialEq`, so this is the strongest equality.
        assert_eq!(
            first, second,
            "PT-SEED-1 [{name}]: same seed + same image must produce a byte/structurally IDENTICAL \
             CompositionPlan"
        );
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// PT-SEED-2 — SEED-SENSITIVE: different seeds ⇒ the progression draw differs.
// ═════════════════════════════════════════════════════════════════════════════

/// PT-SEED-2: on the SAME image, two DIFFERENT seeds must yield DIFFERENT progression draws (the
/// seed actually steers the RNG). We assert the specific 42-vs-7 pair differs on the reference image
/// (probe-confirmed: they diverge at sections B and A'), AND — to be robust against a pair that could
/// coincidentally coincide on a more-constrained image — assert across a SET of seeds that NOT ALL
/// realized progression sequences are identical (the selector genuinely spreads across seeds).
#[test]
fn test_pt_seed_2_distinct_seeds_diverge() {
    set_composition_seed(Some(42)); // ordering guard.
    let m = mappings();
    let p = planner(&m);
    let img = reference_image();

    // (a) The concrete 42-vs-7 pair the work order names, on an image with real progression choice.
    let a = progressions(&plan_seeded(&p, &m, &img, 42));
    let b = progressions(&plan_seeded(&p, &m, &img, 7));
    assert_ne!(
        a, b,
        "PT-SEED-2: seeds 42 and 7 must draw DIFFERENT progression sequences on the same image; \
         both produced {a:?}"
    );

    // (b) Robust form: across a seed set, the realized sequences are NOT ALL identical.
    let seeds = [42u64, 7, 1, 2, 3, 100, 999];
    let distinct: BTreeSet<Vec<Vec<String>>> = seeds
        .iter()
        .map(|&s| progressions(&plan_seeded(&p, &m, &img, s)))
        .collect();
    assert!(
        distinct.len() >= 2,
        "PT-SEED-2: across {} seeds the planner must realize >=2 distinct progression sequences \
         (the seed must steer the draw); saw {} distinct out of {} seeds",
        seeds.len(),
        distinct.len(),
        seeds.len()
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// PT-SEED-3 — MULTI-SECTION DECORRELATION (+ reproducibility at multi-section scale).
// ═════════════════════════════════════════════════════════════════════════════

/// PT-SEED-3: within ONE seeded multi-section plan, the per-call counter mixing (`mix_seed(seed,
/// call)`) must make the sections NOT all draw the byte-identical progression — i.e. the counter
/// genuinely advances and produces distinct sub-streams (a broken counter would draw the SAME
/// progression for every section). The reference image is the right fixture: it has 3 sections in the
/// cool family (5 choices) and the probe confirms the three draws decorrelate. Because section
/// progressions CAN legitimately repeat on other images, we ALSO assert the weaker, always-true
/// property — reproducibility at multi-section scale (PT-SEED-1 with N>=3 sections) — so PT-SEED-3
/// has a guaranteed-holding leg even if a future mappings edit reduced the reference's choice set.
#[test]
fn test_pt_seed_3_multi_section_decorrelates() {
    set_composition_seed(Some(42)); // ordering guard.
    let m = mappings();
    let p = planner(&m);
    let img = reference_image();

    let plan = plan_seeded(&p, &m, &img, 42);
    assert!(
        plan.sections.len() >= 3,
        "PT-SEED-3 premise: the reference image must be a multi-section (>=3) plan; got {}",
        plan.sections.len()
    );

    // Strong leg: the per-call counter decorrelates the sections — NOT all section progressions are
    // byte-identical (a dead counter would mix the same sub-stream every call → all-identical).
    let distinct: BTreeSet<Vec<String>> = plan
        .sections
        .iter()
        .map(|s| s.progression.clone())
        .collect();
    assert!(
        distinct.len() >= 2,
        "PT-SEED-3 (decorrelation): a seeded {}-section plan must NOT draw the byte-identical \
         progression for every section (mix_seed must vary per call); saw all sections == {:?}",
        plan.sections.len(),
        plan.sections[0].progression
    );

    // Weaker, always-true leg: the seeded multi-section plan is reproducible run-to-run (PT-SEED-1
    // at multi-section scale). Holds even if some image's sections legitimately repeat.
    let again = plan_seeded(&p, &m, &img, 42);
    assert_eq!(
        plan, again,
        "PT-SEED-3 (multi-section reproducibility): a seeded multi-section plan must replay \
         identically run-to-run"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// PT-SEED-4 — LEGACY PRESERVED: register None ⇒ valid plan via the thread_rng path.
// ═════════════════════════════════════════════════════════════════════════════

/// PT-SEED-4: with the register `None` (the DEFAULT, absent `--seed`), planning takes the legacy
/// `thread_rng` path. We CANNOT assert determinism (thread_rng is non-deterministic), so we assert
/// SHAPE/VALIDITY only: planning succeeds and produces a structurally valid multi-section plan with
/// non-empty Roman-numeral progressions on every section. This proves the legacy path is intact and
/// the seam did not break the un-seeded default. Run on both fixtures.
#[test]
fn test_pt_seed_4_none_preserves_legacy_path() {
    set_composition_seed(None); // ordering guard AND the property under test: legacy path.
    let m = mappings();
    let p = planner(&m);

    for (name, img) in [("reference", reference_image()), ("vivid", vivid_image())] {
        // Re-assert None at the top of each iteration so no prior test/iteration leaked Some(..).
        set_composition_seed(None);
        let plan = p.plan(&img, &m);
        assert_structurally_valid(&plan, name);
        assert!(
            plan.sections.len() >= 1,
            "PT-SEED-4 [{name}]: legacy path still yields >=1 section"
        );
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// PT-SEED-5 — FREEZE GUARD: src/engine.rs sha256 unchanged at the frozen value.
// ═════════════════════════════════════════════════════════════════════════════

/// PT-SEED-5: the deterministic-seed seam is built precisely so frozen `src/engine.rs` is byte-
/// untouched (the seam lives in `seed.rs` + `chord_engine.rs`, and `engine.rs` merely CALLS
/// `pick_progression`). Assert the sha256 in-test by hashing the file. The 9/9 `engine_equivalence`
/// net (run separately, see VERIFY) covers the engine's *behavior* equivalence; this guards the
/// *bytes*. We hash with a tiny dependency-free SHA-256 so the test needs no new crate.
#[test]
fn test_pt_seed_5_engine_frozen() {
    set_composition_seed(None); // ordering guard (no RNG used, but keep the discipline uniform).
    let bytes =
        std::fs::read("src/engine.rs").expect("src/engine.rs is readable from the crate root");
    let got = sha256_hex(&bytes);
    assert_eq!(
        got, ENGINE_SHA256,
        "PT-SEED-5 (FREEZE GUARD): src/engine.rs sha256 changed — the seed seam MUST leave engine.rs \
         byte-untouched; expected {ENGINE_SHA256}, got {got}. (engine_equivalence 9/9 covers behavior.)"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Minimal dependency-free SHA-256 (FIPS 180-4) — used ONLY by PT-SEED-5 to hash
// src/engine.rs in-test without adding a crate. This is test-only code.
// ─────────────────────────────────────────────────────────────────────────────

fn sha256_hex(data: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    // Pre-processing: pad to a multiple of 64 bytes with 0x80, zeros, and the 64-bit bit length.
    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 64];
        for (i, word) in w.iter_mut().take(16).enumerate() {
            *word = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut hh = h[7];

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }

    let mut out = String::with_capacity(64);
    for word in &h {
        out.push_str(&format!("{word:08x}"));
    }
    out
}

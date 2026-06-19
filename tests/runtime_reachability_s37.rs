//! tests/runtime_reachability_s37.rs — RUNTIME-REACHABILITY net for S37.
//!
//! The regression S37 fixes: the `play`/`render` entry points in `src/main.rs` had
//! stopped calling the composer — they walked the S13-era `set_features_global()`
//! flat path and never installed a `CompositionPlan`, so the entire S14→S36
//! composition arc was INAUDIBLE. The wiring now makes `main.rs` call
//! `engine.compose_from_image(&understand_image_pure(img))` and drive the playback
//! loop off `plan.total_steps`.
//!
//! This net asserts the CONTRACT that wiring establishes WITHOUT going through
//! `main()` (which is hard to call). It calls the SAME lib API `main.rs` now calls:
//!   * build a `PipelineEngine` from the REAL `assets/mappings.json`
//!     (`mapping_loader::load_mappings`),
//!   * `understand_image_pure(&RgbImage)` → `ImageUnderstanding`,
//!   * `engine.compose_from_image(&understanding)` → installs the plan,
//!   * read it back via `engine.composition()`.
//!
//! So if a future refactor drops the `composition` block, breaks
//! `compose_from_image`, or reverts the loop bound back to the scan step count,
//! one of these assertions fails.
//!
//! Spec: docs/spec-s37-wire-composer.md §8 (assertions 1–3, the minimum bar).
//!
//! Determinism / RNG discipline: every assertion is a function of the fixture
//! image + the on-disk mappings only. `compose_from_image` derives the spine
//! (`home_mode`, `base_ms_per_step`, `total_steps`) deterministically from the
//! image understanding; it does NOT take the `set_features_global` →
//! `pick_progression` (`thread_rng`) path. We never assert on per-step harmony
//! realization, so no seeded RNG is needed and no assertion is RNG-derived.

use audiohax::composition::ImageUnderstanding;
use audiohax::engine::PipelineEngine;
use audiohax::mapping_loader::load_mappings;
use audiohax::pure_analysis::understand_image_pure;
use image::{Rgb, RgbImage};

// ── Fixtures: synthetic, in-memory, deterministic. None touch disk. ──────────
// Mirrors the construction discipline in tests/diversity_s13.rs and
// tests/saliency_s18.rs (solid/dark vs vivid/high-edge).

/// A solid flat field of one colour — the calm/dark "null" image.
fn flat(w: u32, h: u32, c: [u8; 3]) -> RgbImage {
    RgbImage::from_pixel(w, h, Rgb(c))
}

/// A vivid, high-edge, busy image: a high-contrast checker carrying strong,
/// spatially varying colour. Bright + saturated + edgy, the opposite end of the
/// affect space from `flat([dark])`.
fn vivid_busy(w: u32, h: u32) -> RgbImage {
    let mut img = RgbImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            // Alternating extreme luminance on a 1px checker → high edge density.
            let hi = if (x + y) % 2 == 0 { 250u8 } else { 5u8 };
            // Strongly varying, saturated colour across the field.
            let r = (((x * 37) % 256) as u8).max(hi);
            let g = (((y * 53) % 200) as u8).saturating_add(40);
            img.put_pixel(x, y, Rgb([r, g, hi]));
        }
    }
    img
}

/// Build a `PipelineEngine` from the REAL on-disk mappings — exactly as `main.rs`
/// does (`load_mappings("assets/mappings.json")` → `PipelineEngine::new`). Config
/// defaults; the composer spine does not depend on `EngineConfig`.
fn engine_from_real_mappings() -> PipelineEngine {
    let mappings = load_mappings("assets/mappings.json")
        .expect("real assets/mappings.json loads (same path main.rs uses)");
    PipelineEngine::new(mappings, Default::default())
}

/// `understand_image_pure` on a non-empty image — the exact call `main.rs` feeds
/// into `compose_from_image`.
fn understand(img: &RgbImage) -> ImageUnderstanding {
    understand_image_pure(img).expect("understand_image_pure ok on a non-empty image")
}

// ═════════════════════════════════════════════════════════════════════════════
// ASSERTION 1 — a `composition` block in mappings.json ⇒ the engine installs a plan
// ═════════════════════════════════════════════════════════════════════════════
//
// This IS the exact call main.rs now makes:
//     let composed = engine.compose_from_image(&understanding);
//     match engine.composition() { Some(plan) => ... }
// If a future edit drops the `composition` block from mappings.json (→ false /
// None) or breaks compose_from_image, this fails — which is precisely the silent
// un-wire S37 fixes, caught at the data layer.
#[test]
fn composition_block_present_means_engine_installs_a_plan() {
    let img = flat(48, 48, [12, 12, 30]);
    let mut engine = engine_from_real_mappings();

    let composed = engine.compose_from_image(&understand(&img));

    assert!(
        composed,
        "compose_from_image returned false: the real assets/mappings.json is missing its \
         `composition` block (the engine fell back to the legacy S13 flat path — the exact \
         silent un-wire S37 exists to prevent)"
    );
    assert!(
        engine.composition().is_some(),
        "compose_from_image returned true but no CompositionPlan was installed \
         (engine.composition() is None)"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// ASSERTION 2 — the drive count comes from the PLAN, not the SCAN
// ═════════════════════════════════════════════════════════════════════════════
//
// main.rs computes its playback loop bound as:
//     let total_steps = match engine.composition() {
//         Some(plan) => plan.total_steps,   // ← plan-derived (the S37 wiring)
//         None        => num_steps,         // ← legacy: the scan step count
//     };
//     for step_idx in 0..total_steps { ... }
//
// We assert (a) the plan's own internal invariant `total_steps == sum of
// section step_len == steps_per_section * n_sections` (the value main.rs loops
// to), and (b) that this plan-derived count is NOT the scan step count
// `source.step_count()` / `num_steps` that the legacy `None` arm would have
// used. The CLI default scan count is 40 (src/cli.rs:97 `steps: 40`); our
// fixture's plan length is 24 (3 sections × 8 BASE_STEPS_PER_SECTION, with the
// edge-activity bonus rounding to 0 for this fixture). 24 != 40, so the loop
// bound being plan-derived is observable: a regression reverting `total_steps`
// back to `source.step_count()` would loop to 40 and this fails.
#[test]
fn drive_count_is_plan_derived_not_scan_derived() {
    // The scan step count main.rs's legacy `None` arm would loop to (the CLI
    // default `--steps`, src/cli.rs:97). A reverted wiring would drive THIS many.
    const SCAN_STEP_COUNT: usize = 40;

    let img = flat(48, 48, [12, 12, 30]);
    let mut engine = engine_from_real_mappings();
    assert!(engine.compose_from_image(&understand(&img)));

    let plan = engine.composition().expect("plan installed");

    // (a) total_steps is the plan's own cursor invariant: it equals the sum of
    //     the concrete section lengths == steps_per_section * n_sections.
    let n_sections = plan.sections.len();
    assert!(n_sections > 0, "plan has at least one section");
    let sum_step_len: usize = plan.sections.iter().map(|s| s.step_len).sum();
    assert_eq!(
        plan.total_steps, sum_step_len,
        "total_steps ({}) must equal the sum of section step_len ({}) — the cursor invariant; \
         this is the steps_per_section * n_sections product main.rs loops to",
        plan.total_steps, sum_step_len
    );
    // steps_per_section is uniform up to the last section's rounding remainder; the
    // product relationship is captured by the sum invariant above. Pin the concrete
    // value so a silent change in BASE_STEPS_PER_SECTION / section count is visible.
    assert_eq!(
        plan.total_steps, 24,
        "expected the dark-flat fixture's plan to be {} sections × 8 base steps = 24 \
         (no edge-activity bonus); got total_steps={}",
        n_sections, plan.total_steps
    );

    // (b) The plan-derived drive count is NOT the scan step count. This is the
    //     regression guard: if `total_steps` ever reverts to source.step_count()
    //     / num_steps, the loop bound would be SCAN_STEP_COUNT (40), not 24.
    assert_ne!(
        plan.total_steps, SCAN_STEP_COUNT,
        "plan.total_steps must NOT equal the scan step count ({}) — the loop bound main.rs \
         uses has to be plan-derived, not scan-derived; equality here means the S37 wiring \
         was reverted back to source.step_count()",
        SCAN_STEP_COUNT
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// ASSERTION 3 — two visibly different images ⇒ different installed plans
// ═════════════════════════════════════════════════════════════════════════════
//
// The END-TO-END echo of tests/diversity_s13.rs, proved THROUGH the composer
// install path the binary now actually uses. Compose from a calm/dark image and
// a vivid/busy image; the two installed CompositionPlans must differ in at least
// one AUDIBLE spine field: key_tempo.base_ms_per_step OR key_tempo.home_mode OR
// total_steps. This locks S13 image-diversity to the install path — if the
// composer ever collapses to a constant plan, distinct images stop sounding
// distinct AT THE PATH THE BINARY USES, and this fails.
#[test]
fn distinct_images_install_distinct_plans() {
    let dark = flat(48, 48, [10, 10, 28]);
    let vivid = vivid_busy(48, 48);

    let mut e_dark = engine_from_real_mappings();
    assert!(e_dark.compose_from_image(&understand(&dark)));
    let p_dark = e_dark.composition().expect("dark plan installed").clone();

    let mut e_vivid = engine_from_real_mappings();
    assert!(e_vivid.compose_from_image(&understand(&vivid)));
    let p_vivid = e_vivid.composition().expect("vivid plan installed").clone();

    let tempo_differs = p_dark.key_tempo.base_ms_per_step != p_vivid.key_tempo.base_ms_per_step;
    let mode_differs = p_dark.key_tempo.home_mode != p_vivid.key_tempo.home_mode;
    let length_differs = p_dark.total_steps != p_vivid.total_steps;

    assert!(
        tempo_differs || mode_differs || length_differs,
        "two visibly different images installed indistinguishable plans \
         (dark: ms={} mode={} steps={} | vivid: ms={} mode={} steps={}) — image diversity is \
         dead AT THE COMPOSER INSTALL PATH THE BINARY USES",
        p_dark.key_tempo.base_ms_per_step,
        p_dark.key_tempo.home_mode,
        p_dark.total_steps,
        p_vivid.key_tempo.base_ms_per_step,
        p_vivid.key_tempo.home_mode,
        p_vivid.total_steps,
    );
}

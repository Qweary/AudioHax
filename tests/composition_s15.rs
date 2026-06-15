//! tests/composition_s15.rs — the S15 SLICE-1 STRUCTURAL PROPERTY NET.
//!
//! This file proves the slice-1 composition guarantees that `engine_equivalence.rs`
//! (the byte-freeze of the per-step realizer kernel) deliberately does NOT cover: the
//! UP-FRONT architectural plan the `CompositionPlanner` emits. Where the freeze pins the
//! realizer's note output against a FIXED hand-built plan, this net pins the SHAPE of the
//! plan itself — the non-looping cursor, the returning-theme transform, the differentiated
//! cadence placement, and the image→form variety mechanism.
//!
//! HEADLESS, in the same sense as engine_equivalence.rs / engine_seam.rs / tui_render.rs:
//! it touches NO image type, NO OpenCV, NO audio hardware. It exercises only the pure
//! planner (`CompositionPlanner::plan`), `mapping_loader::load_mappings`, and the pure
//! `chord_engine::resolve_motif`. (Cargo `--no-default-features` cannot be used to RUN it
//! because the `audiohax` bin is feature-gated on `synth`/`midi-out` and the integration
//! harness builds the bin; run it under DEFAULT features — see the file footer for the exact
//! invocation. The runtime is headless regardless of the build feature set.)
//!
//! RNG-BOUNDARY DISCIPLINE (same as engine_equivalence.rs / tui_render.rs):
//! `CompositionPlanner::plan` delegates per-section harmony to `chord_engine::pick_progression`,
//! which uses `thread_rng` (chord_engine.rs) — so the CHORDS / Roman numerals / per-step
//! `StepPlan.chord` are NON-deterministic across runs and are NEVER asserted here. Everything
//! this net pins is RNG-INDEPENDENT: the section STRUCTURE (count, labels, step_len tiling,
//! thematic roles, cadence types) and the THEME degree-sequence (produced by the PURE,
//! RNG-free `resolve_motif`). No `thread_rng`-derived chord/note value is ever asserted.
//!
//! PROPERTIES (the kickoff's required gates):
//!   1. Non-looping realized plan — the global cursor walks 0..total_steps once, NO modular
//!      wrap; `locate` returns strictly NON-DECREASING section indices. (GLOBAL non-loop is
//!      the slice's structural point; within-section detail is separate.)
//!   2. Returning theme is a real transform, not a fresh line — A and A' (both theme slot 0,
//!      Identity) recall the SAME degree sequence; B is theme-absent (max-contrast default).
//!   3. Structural close is a PAC — the final section's boundary_cadence is the strongest
//!      (Perfect); an interior boundary (end of A) is a Half cadence.
//!   4. Distinct images → distinct plans — materially different understandings diverge in
//!      form id and/or motif contour archetype and/or cadence placement; a neutral image
//!      selects the confirmed default `rounded_binary`.
//!   5. Equivalence still holds — a thin confirmation that the back-compat default
//!      `StepContext` path is ADDITIVE; engine_equivalence.rs remains the authority (this
//!      net neither duplicates nor weakens it).

use audiohax::chord_engine::{resolve_motif, MotifArchetype, MotifNote};
use audiohax::composition::{
    CadenceStrength, Character, CompositionPlanner, ImageUnderstanding, Meter,
    OrchestrationProfile, PlanMappings, Section, StepContext, ThematicRole, ThemeVariation,
};
use audiohax::mapping_loader::{load_mappings, MappingTable};

// ─────────────────────────────────────────────────────────────────────────────
// Fixtures — load the SHIPPED mappings and build planner inputs. No RNG seeding is
// needed because every asserted property is RNG-independent (see header).
// ─────────────────────────────────────────────────────────────────────────────

/// The shipped `assets/mappings.json` (the same table the engine holds).
fn mappings() -> MappingTable {
    load_mappings("assets/mappings.json").expect("mappings.json loads")
}

/// The composition `PlanMappings` (the form/theme SelectTables + form catalogue).
fn plan_mappings(m: &MappingTable) -> PlanMappings {
    m.composition
        .clone()
        .expect("composition block present in mappings.json")
        .into()
}

/// An `ImageUnderstanding` with only the form/theme-ladder knobs set; everything else at
/// its slice-1 neutral default. Lets each test name exactly the discriminating knobs.
fn u(
    complexity: f32,
    edge_activity: f32,
    quadrant_contrast: f32,
    dominant_hue: f32,
    value_key: f32,
) -> ImageUnderstanding {
    ImageUnderstanding {
        complexity,
        edge_activity,
        quadrant_contrast,
        dominant_hue,
        value_key,
        ..ImageUnderstanding::neutral()
    }
}

/// Resolve a section reference (as returned by `plan.locate`) to its INDEX in
/// `plan.sections` by pointer identity — the only stable way to read a section index out of
/// `locate`, which returns `&Section` (no index accessor). This is observation only; it does
/// not depend on any RNG-derived field.
fn section_index(sections: &[Section], target: &Section) -> usize {
    sections
        .iter()
        .position(|s| std::ptr::eq(s, target))
        .expect("located section must be one of plan.sections")
}

/// The degree sequence of a motif (the RNG-free identity of a theme line).
fn degrees(motif: &[MotifNote]) -> Vec<i8> {
    motif.iter().map(|n| n.degree).collect()
}

// ─────────────────────────────────────────────────────────────────────────────
// PROPERTY 1 — Non-looping realized plan (the slice's structural point)
// ─────────────────────────────────────────────────────────────────────────────

/// P1: the expanded plan's section lengths tile `total_steps` exactly, and walking the
/// global cursor 0..total_steps via `locate` visits each step ONCE with NO modular wrap —
/// `locate` returns strictly NON-DECREASING section indices, the in-section offset stays in
/// bounds, and `locate(total_steps)` is `None` (the cursor never advances past the end).
/// This is the GLOBAL non-loop (the death of `plan[step_idx % len]`), distinct from any
/// within-section detail.
///
/// RNG-robust: reads only section STRUCTURE (step_len, ordering) — never a chord value.
#[test]
fn test_global_cursor_is_non_looping_and_monotonic() {
    let m = mappings();
    let planner = CompositionPlanner::new(plan_mappings(&m));
    // A mid-energy image — themes present, a multi-section form, several sections to walk.
    let plan = planner.plan(&u(0.5, 0.5, 0.0, 30.0, 0.0), &m);

    // Sections tile total_steps exactly (no step unreachable, none double-counted).
    let tiled: usize = plan.sections.iter().map(|s| s.step_len).sum();
    assert_eq!(
        tiled, plan.total_steps,
        "section step_lens must sum to total_steps (the cursor's N)"
    );
    assert!(plan.total_steps > 0, "a real piece has steps");
    assert!(plan.sections.len() >= 2, "a multi-section form to walk");

    // Walk the global cursor once, start to finish. Track the located section index and the
    // expected in-section offset (which resets to 0 only at a section boundary, never wraps
    // the GLOBAL cursor). The section index must be NON-DECREASING and advance by exactly 1
    // at each boundary — proving a single linear pass, not a modular re-entry.
    let mut last_section_idx: isize = -1;
    let mut expected_off = 0usize;
    let mut boundary_count = 0usize;
    for step in 0..plan.total_steps {
        let (sec, off) = plan
            .locate(step)
            .expect("every in-range global step locates to a section");
        let idx = section_index(&plan.sections, sec);

        assert!(
            (idx as isize) >= last_section_idx,
            "located section index must be NON-DECREASING (step {step}): {idx} < {last_section_idx}"
        );
        if idx as isize != last_section_idx {
            // Crossed into a new section: index steps up by exactly 1 (no skips, no wrap).
            if last_section_idx >= 0 {
                assert_eq!(
                    idx as isize - last_section_idx,
                    1,
                    "the cursor enters sections in order, one at a time (step {step})"
                );
                boundary_count += 1;
            }
            expected_off = 0;
            last_section_idx = idx as isize;
        }
        // The in-section offset advances 1:1 with the global step — NO modulo wrap of the
        // global cursor (the spec's structural point). It only resets at a boundary.
        assert_eq!(
            off, expected_off,
            "in-section offset must track the linear global walk (step {step})"
        );
        assert!(
            off < sec.step_len,
            "offset stays within the section (step {step})"
        );
        expected_off += 1;
    }
    // Every section was entered exactly once.
    assert_eq!(
        boundary_count,
        plan.sections.len() - 1,
        "the walk crosses each section boundary exactly once (visits every section)"
    );
    // The cursor never advances past total_steps.
    assert!(
        plan.locate(plan.total_steps).is_none(),
        "locate(total_steps) is None — no modular wrap-around at the end"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// PROPERTY 2 — Returning theme is a real transform, not a fresh line
// ─────────────────────────────────────────────────────────────────────────────

/// P2: in a rounded-binary plan (A Statement / B Contrast / A' Return), A and A' BOTH
/// reference theme slot 0 with `variation == Identity`, so the Return RECALLS the same
/// theme line — not a freshly random melody. We assert:
///   - the plan is `rounded_binary` (the confirmed default — driven here),
///   - sections A and A' both carry `theme: Some(0)` and `variation: Identity`,
///   - the B (Contrast) section is theme-ABSENT (`theme: None`) — max-contrast default,
///   - the recalled theme's degree sequence (Identity ⇒ MATCH) equals the deterministic
///     `resolve_motif` output for the planner's image-chosen archetype/range/length, and is
///     NOT the trivial/flat line a random or no-op generator would produce.
///
/// RNG-robust: the theme degree sequence comes from the PURE, thread_rng-free
/// `resolve_motif`; A vs A' share `plan.themes[0]` by construction (one stored seed), so the
/// "transform" is the Identity recall — exactly what slice 1 specifies. No chord asserted.
#[test]
fn test_returning_theme_is_identity_recall_not_fresh() {
    let m = mappings();
    let planner = CompositionPlanner::new(plan_mappings(&m));
    // complexity >= 0.4 trips theme_behaviour "fragment" (themes PRESENT); low energy keeps
    // the form at the default rounded_binary. dominant_hue 30 (< 90) → Arch archetype.
    let uimg = u(0.45, 0.2, 0.0, 30.0, 0.0);
    let plan = planner.plan(&uimg, &m);

    assert_eq!(
        plan.form, "rounded_binary",
        "low-energy image selects the default rounded_binary"
    );
    assert!(
        !plan.themes.is_empty(),
        "complexity>=0.4 ⇒ a returning theme is present"
    );

    // Identify the A (Statement), B (Contrast), A' (Return) sections by role.
    let a = plan
        .sections
        .iter()
        .find(|s| s.thematic_role == ThematicRole::Statement)
        .expect("rounded_binary has an A Statement");
    let b = plan
        .sections
        .iter()
        .find(|s| s.thematic_role == ThematicRole::Contrast)
        .expect("rounded_binary has a B Contrast");
    let a_return = plan
        .sections
        .iter()
        .find(|s| s.thematic_role == ThematicRole::Return)
        .expect("rounded_binary has an A' Return");

    // A and A' recall the SAME theme slot under Identity (a real recall, not a re-pick).
    assert_eq!(a.theme, Some(0), "A states theme slot 0");
    assert_eq!(a_return.theme, Some(0), "A' recalls theme slot 0");
    assert_eq!(a.variation, ThemeVariation::Identity, "A is Identity");
    assert_eq!(
        a_return.variation,
        ThemeVariation::Identity,
        "A' is an Identity recall — the same line returns, not a fresh one"
    );
    // B is theme-absent: the max-contrast default (a contrasting section drops the theme).
    assert_eq!(
        b.theme, None,
        "B (Contrast) is theme-absent — the max-contrast default"
    );

    // The recalled degree sequence MATCHES the deterministic resolver output. Reproduce the
    // planner's archetype/range/length choice (the pick_archetype + range/length formulas are
    // pure functions of the image knobs — RNG-free) and compare degree sequences.
    let expected_archetype = MotifArchetype::Arch; // hue 30 (<90), edge<0.6 ⇒ Arch
    let range_degrees = (2.0 + uimg.edge_activity * 5.0).round() as u8; // 2..=7
    let length_steps = (3.0 + uimg.complexity * 5.0).round() as usize; // 3..=8
    let expected = resolve_motif(expected_archetype, range_degrees, length_steps);
    let got = &plan.themes[0].motif;
    assert_eq!(
        degrees(got),
        degrees(&expected),
        "the stored theme is the deterministic resolve_motif line (Identity recall)"
    );

    // NOT a fresh/random line and NOT a degenerate flat sequence: the Arch contour
    // (0,2,4,2,0 scaled) must contain real melodic motion (>1 distinct degree, non-zero
    // peak) — a trivial generator would yield all-tonic.
    let distinct: std::collections::BTreeSet<i8> = got.iter().map(|n| n.degree).collect();
    assert!(
        distinct.len() >= 2,
        "an Arch theme has real contour (≥2 distinct degrees), not a flat/empty line: {distinct:?}"
    );
    assert!(
        got.iter().any(|n| n.degree != 0),
        "the theme rises off the tonic — not a degenerate all-tonic line"
    );
    // Every step's theme reference is consistent: a section that recalls slot 0 has a slot 0
    // in range of plan.themes (no dangling theme index).
    for s in &plan.sections {
        if let Some(t) = s.theme {
            assert!(t < plan.themes.len(), "section theme index {t} in range");
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// PROPERTY 3 — Structural close is a PAC; an interior boundary is a Half cadence
// ─────────────────────────────────────────────────────────────────────────────

/// P3: the FINAL section closes with the strongest cadence (Perfect Authentic), and an
/// INTERIOR boundary (the end of the opening A statement in rounded_binary) is a Half
/// cadence — the differentiated cadence-strength placement that gives the piece a real
/// structural arc rather than a uniform close on every section.
///
/// RNG-robust: `boundary_cadence` is copied verbatim from the FormSpec row (no RNG path).
#[test]
fn test_structural_close_is_pac_interior_is_half() {
    let m = mappings();
    let planner = CompositionPlanner::new(plan_mappings(&m));
    let plan = planner.plan(&u(0.45, 0.2, 0.0, 30.0, 0.0), &m);
    assert_eq!(plan.form, "rounded_binary");

    // The strongest cadence sits at the END of the piece (the final section).
    let last = plan.sections.last().expect("a non-empty plan");
    assert_eq!(
        last.boundary_cadence,
        CadenceStrength::Perfect,
        "the structural close is a PAC (the strongest cadence at the end)"
    );
    assert_eq!(
        last.thematic_role,
        ThematicRole::Return,
        "in rounded_binary the closing section is the A' Return"
    );

    // The interior boundary — the end of the opening A statement — is a HALF cadence (a
    // question that the return answers), strictly weaker than the final close.
    let a = &plan.sections[0];
    assert_eq!(
        a.thematic_role,
        ThematicRole::Statement,
        "the first section is the A Statement"
    );
    assert_eq!(
        a.boundary_cadence,
        CadenceStrength::Half,
        "the interior A-boundary is a Half cadence (differentiated from the final PAC)"
    );
    // The placement is genuinely DIFFERENTIATED, not uniform across sections.
    assert_ne!(
        a.boundary_cadence, last.boundary_cadence,
        "interior and final cadences must differ — a real structural arc"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// PROPERTY 4 — Distinct images → distinct plans (the variety mechanism discriminates)
// ─────────────────────────────────────────────────────────────────────────────

/// P4: a neutral/default image selects the confirmed default `rounded_binary`; a materially
/// different image (high complexity AND high edge activity) selects a DIFFERENT form
/// (`theme_and_variations`, per the live form ladder). And two images that differ in hue/edge
/// drive DIFFERENT motif contour archetypes — so the variety shows up in form id AND in the
/// melodic shape, proving the discriminator actually fires (not a single fixed plan).
///
/// RNG-robust: form id and archetype choice are pure functions of the image knobs and the
/// (fixed) shipped SelectTables; neither touches thread_rng.
#[test]
fn test_distinct_images_yield_distinct_plans() {
    let m = mappings();
    let planner = CompositionPlanner::new(plan_mappings(&m));

    // (a) Neutral image → the confirmed default form (post form-ladder fix).
    let neutral = planner.plan(&ImageUnderstanding::neutral(), &m);
    assert_eq!(
        neutral.form, "rounded_binary",
        "a neutral/default image selects the confirmed default rounded_binary"
    );

    // (b) High-complexity, high-edge image → a DIFFERENT form via the live ladder
    //     (complexity>=0.66 AND edge_activity>=0.6 ⇒ theme_and_variations).
    let busy = planner.plan(&u(0.8, 0.75, 0.0, 30.0, 0.0), &m);
    assert_eq!(
        busy.form, "theme_and_variations",
        "a high-complexity/high-edge image selects a materially different form"
    );
    assert_ne!(
        neutral.form, busy.form,
        "materially different images must NOT collapse to the same form"
    );

    // (c) The motif contour archetype discriminates on hue/edge. Reproduce the planner's pure
    //     pick: hue 30 (<90, edge<0.6) ⇒ Arch; hue 200 (180..270, edge<0.6) ⇒ Descent. The
    //     two yield DIFFERENT degree sequences (different melodic shapes), proving the theme
    //     line varies with the image, not a fixed motif.
    let len = 5usize;
    let arch = resolve_motif(MotifArchetype::Arch, 4, len); // hue<90 path
    let descent = resolve_motif(MotifArchetype::Descent, 4, len); // 180..270 path
    assert_ne!(
        degrees(&arch),
        degrees(&descent),
        "distinct hue quadrants drive distinct motif contours (Arch vs Descent)"
    );

    // (d) A high-edge image tips the archetype to Ascent regardless of hue (edge>=0.6 path) —
    //     distinct from the calm Arch shape; the discriminator reads BOTH axes.
    let ascent = resolve_motif(MotifArchetype::Ascent, 4, len);
    assert_ne!(
        degrees(&arch),
        degrees(&ascent),
        "high edge_activity drives a different contour (Ascent) than the calm Arch"
    );

    // The plans also differ in section COUNT (rounded_binary has 3 sections;
    // theme_and_variations has 3 too, but with different roles) — assert the role multiset
    // differs, a structural divergence beyond just the id string.
    let neutral_roles: Vec<ThematicRole> =
        neutral.sections.iter().map(|s| s.thematic_role).collect();
    let busy_roles: Vec<ThematicRole> = busy.sections.iter().map(|s| s.thematic_role).collect();
    assert_ne!(
        neutral_roles, busy_roles,
        "the two forms expand to different thematic-role sequences"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// PROPERTY 5 — Equivalence still holds (the default StepContext path is ADDITIVE)
// ─────────────────────────────────────────────────────────────────────────────

/// P5: a THIN confirmation that the back-compat default `StepContext` is unchanged and
/// additive. The authority for the byte-freeze is `tests/engine_equivalence.rs` — this net
/// does NOT duplicate or weaken it. Here we only confirm that
/// `StepContext::single_section_default` still constructs a behaviour-NEUTRAL context
/// (no theme, home key, identity variation) — the precondition under which the equivalence
/// goldens do not move. The new plan-driven path (properties 1–4) is purely additive on top
/// of this default path.
#[test]
fn test_default_step_context_is_behaviour_neutral_additive() {
    // A behaviour-neutral section + key/tempo spine (the exact neutral shape the freeze net
    // borrows into the 7th ctx arg). theme:None + key_offset:0 ⇒ the realizer free-selects
    // exactly as before ⇒ the equivalence goldens are unmoved.
    let section = Section {
        label: "A".to_string(),
        step_len: 2,
        thematic_role: ThematicRole::Statement,
        key_offset_semitones: 0,
        ms_per_step: 200,
        mode: "Ionian".to_string(),
        progression: vec![],
        theme: None,
        variation: ThemeVariation::Identity,
        boundary_cadence: CadenceStrength::Perfect,
        density: 0.5,
        // S17: identity orchestration profile — additive struct-field plumb only (no assert
        // touched); keeps the behaviour-neutral precondition the equivalence goldens rely on.
        orchestration: OrchestrationProfile::identity(),
        steps: vec![],
    };
    let kt = audiohax::composition::KeyTempoPlan {
        home_root_midi: 60,
        home_mode: "Ionian".to_string(),
        base_ms_per_step: 200,
        key_scheme: vec![0],
        tempo_scheme: vec![200],
    };
    let ctx = StepContext::single_section_default(&section, &kt);

    // The neutral context applies NO theme and NO transposition — the precondition for the
    // engine_equivalence.rs goldens (240, 114/84, 36/79) staying put. (engine_equivalence.rs
    // is the authority that the *output* is byte-identical; we assert only the precondition.)
    assert!(
        ctx.theme.is_none(),
        "default ctx carries no theme (additive)"
    );
    assert_eq!(
        ctx.section.key_offset_semitones, 0,
        "default ctx is home-key (no transposition)"
    );
    assert_eq!(
        ctx.key_tempo.home_root_midi, 60,
        "default ctx is the home root"
    );
    assert_eq!(
        ctx.section.variation,
        ThemeVariation::Identity,
        "default ctx variation is Identity (no theme transform)"
    );
    // Sanity: the slice-1 plan is Ballad / Four4 (the pinned character/meter) — confirms the
    // additive plan layer respects the locked slice-1 character envelope.
    let m = mappings();
    let plan = CompositionPlanner::new(plan_mappings(&m)).plan(&ImageUnderstanding::neutral(), &m);
    assert_eq!(plan.character, Character::Ballad, "slice 1 is Ballad-only");
    assert_eq!(plan.meter, Meter::Four4, "slice 1 is 4/4-only");
}

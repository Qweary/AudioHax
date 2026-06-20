//! tests/variety_s45.rs — the S45 SLICE-1 VARIETY/ROUTING PROPERTY NET.
//!
//! Pins the three freeze-safe moves the Music Theory Specialist landed in Session 45,
//! Slice 1 (`docs/design-s44-variety-nvoice.md` §2/§6): routing the existing CounterMelody
//! species voice into the DEFAULT inner texture, and adding per-section figuration variety.
//! These tests make a future regression of either move fail LOUDLY.
//!
//! The two mechanisms under test:
//!   1. The `texture` SelectTable `pad_bed_counter` rule was RE-TUNED to the S45 probe-measured
//!      feature distribution — it now gates on `foreground_energy ≥ 0.015 AND fg_bg_contrast ≥
//!      0.15` (the measured probe band is fe 0.003–0.039, ct 0.052–0.341) — so ordinary
//!      mid-energy images (the three AudioHaxImg* in the probe set) now SELECT the
//!      CounterMelody-bearing `pad_bed_counter` profile instead of falling through to the
//!      `pad_bed` default (HarmonicFill). Calmer images (below the contrast floor — Lena,
//!      magicstudio-art) still fall through to `pad_bed`. Selection is FIRST-MATCH-WINS; the
//!      higher-priority `pad_figured` rule (subject_energy≥0.45 AND fg_bg_contrast≥0.25) must
//!      still win where it qualifies.
//!   2. The planner now varies Pad figuration PER SECTION: anchor roles (Statement/Return/
//!      Coda) keep the profile's base figuration cell; departure roles (Contrast/Development)
//!      take a contrasting cell from the SAME existing figuration_catalogue. A profile with
//!      `figuration == None` (identity / pad_bed / pad_bed_counter) never triggers the
//!      override → its per-section figuration stays None (the byte-freeze pin at plan level).
//!
//! DETERMINISTIC + HEADLESS, in the same sense as `figuration_s20.rs` / `texture_s17.rs` /
//! `composition_s15.rs`: it touches NO image type, NO OpenCV, NO audio hardware. Properties
//! A/B/C drive the REAL `texture` SelectTable loaded from the shipped `assets/mappings.json`
//! (the loader-backed selection discipline `composition_s15.rs`/`figuration_s20.rs` use).
//! Properties D/E drive the REAL planner (`CompositionPlanner::plan`) end-to-end and read
//! each section's resolved figuration off the public `Section`/`OrchestrationProfile` surface.
//!
//! RNG-BOUNDARY DISCIPLINE (same as `composition_s15.rs`): `CompositionPlanner::plan`
//! delegates per-section harmony to `thread_rng`, so CHORDS / Roman numerals / per-step
//! note values are NON-deterministic and are NEVER asserted here. Everything pinned is
//! RNG-INDEPENDENT: the texture id a SelectTable returns, and the per-section figuration id
//! the planner resolves (`section_figuration_id` is a pure function of base-id + role).
//!
//! Run under DEFAULT features (the integration harness builds the feature-gated bin, so
//! `--no-default-features` cannot RUN this net):  cargo test --test variety_s45

use audiohax::composition::{
    CompositionPlanner, ImageUnderstanding, LayerRole, OrchestrationProfile, PlanMappings,
    ThematicRole,
};
use audiohax::mapping_loader::{load_mappings, MappingTable};

// ─────────────────────────────────────────────────────────────────────────────
// Shared fixtures — load the SHIPPED mappings (no hand-built SelectTable: the point
// of A/B/C is that the ACTUAL shipped thresholds route as the slice claims).
// ─────────────────────────────────────────────────────────────────────────────

fn mappings() -> MappingTable {
    load_mappings("assets/mappings.json").expect("assets/mappings.json loads")
}

fn plan_mappings(m: &MappingTable) -> PlanMappings {
    m.composition
        .clone()
        .expect("composition block present in mappings.json")
        .into()
}

/// An `ImageUnderstanding` carrying only the texture-ladder knobs that drive the
/// `pad_figured` / `pad_bed_counter` / `pad_bed` decision; everything else neutral.
fn tex_u(subject_energy: f32, foreground_energy: f32, fg_bg_contrast: f32) -> ImageUnderstanding {
    ImageUnderstanding {
        subject_energy,
        foreground_energy,
        fg_bg_contrast,
        ..ImageUnderstanding::neutral()
    }
}

/// The layer set of a texture-catalogue profile by id (read from the loaded mappings).
fn layers_of(pm: &PlanMappings, id: &str) -> Vec<LayerRole> {
    pm.texture_catalogue
        .iter()
        .find(|p| p.id == id)
        .unwrap_or_else(|| panic!("profile {id} present in texture_catalogue"))
        .layers
        .clone()
}

// ═════════════════════════════════════════════════════════════════════════════
// PROPERTY A (Move 1) — Routing invariant: an ordinary mid-energy image that USED to
// select `pad_bed` now selects `pad_bed_counter` (the CounterMelody-bearing profile).
// ═════════════════════════════════════════════════════════════════════════════

/// A: drives the REAL `texture` SelectTable with knob values keyed to the NEW gate
/// (foreground_energy ≥ 0.015 AND fg_bg_contrast ≥ 0.15), in the probe-measured band
/// (fe ~0.02–0.04, ct ~0.15–0.34). These are exactly the values the three AudioHaxImg*
/// images carry, which now SELECT `pad_bed_counter`. Pins that the resolved profile's layers
/// carry `CounterMelody` and NOT `HarmonicFill`. The (fe 0.017, ct 0.341) case is the
/// load-bearing one: it mirrors AudioHaxImg1's measured knobs (low foreground energy but
/// strong contrast) — proving the contrast-driven gate fires where the old fe-floor would not.
#[test]
fn test_s45_ordinary_image_routes_countermelody() {
    let m = mappings();
    let pm = plan_mappings(&m);
    let texture = &pm.texture;

    // Representative knob sets across the new gate band. Each: subject_energy < 0.45 (so the
    // higher-priority pad_figured rule cannot fire) and contrast < 0.25 NOT required (the
    // pad_figured rule also needs subject_energy ≥ 0.45, kept low here), with
    // foreground_energy ≥ 0.015 AND fg_bg_contrast ≥ 0.15 (the re-tuned pad_bed_counter gate).
    let ordinary_band = [
        ("just above the new floor", tex_u(0.0, 0.015, 0.15)),
        (
            "AudioHaxImg1-like: low fe, strong contrast",
            tex_u(0.0, 0.017, 0.341),
        ),
        (
            "AudioHaxImg2-like: mid fe + contrast",
            tex_u(0.0, 0.034, 0.284),
        ),
        (
            "AudioHaxImg3-like, modest subject",
            tex_u(0.20, 0.024, 0.203),
        ),
    ];

    for (name, img) in ordinary_band {
        let pick = texture.select(&img);
        assert_eq!(
            pick, "pad_bed_counter",
            "ordinary image ({name}) above the relaxed floor must select pad_bed_counter, got {pick}"
        );
        // The resolved profile's layers carry CounterMelody, NOT HarmonicFill — the moving
        // species line replaces the static fill on ordinary images (the whole point of Move 1).
        let layers = layers_of(&pm, &pick);
        assert!(
            layers.contains(&LayerRole::CounterMelody),
            "{name}: the routed profile must carry a CounterMelody layer, layers {layers:?}"
        );
        assert!(
            !layers.contains(&LayerRole::HarmonicFill),
            "{name}: the routed profile must NOT carry the static HarmonicFill layer, layers {layers:?}"
        );
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// PROPERTY B (Move 1) — Calm-image fallback preserved: an image BELOW the new floor
// STILL falls through to `pad_bed` (HarmonicFill, no CounterMelody). The relaxation did
// not swallow the entire calm band.
// ═════════════════════════════════════════════════════════════════════════════

/// B: an image whose knobs are below the new gate (foreground_energy < 0.015 OR
/// fg_bg_contrast < 0.15) must NOT select pad_bed_counter — it falls through to the
/// `pad_bed` default (the held HarmonicFill bed, the quiet fallback for the calmest images).
/// Pins that the re-tuned gate left a real calm band (Lena ct 0.052, magicstudio ct 0.084
/// both fall here) rather than routing every image to the counter line.
#[test]
fn test_s45_calm_image_keeps_pad_bed() {
    let m = mappings();
    let pm = plan_mappings(&m);
    let texture = &pm.texture;

    // Each case fails AT LEAST ONE of the two gate predicates, so the rule does not fire.
    let calm_band = [
        // fg_bg_contrast below the 0.15 floor (Lena-like: ample fe, low contrast).
        ("Lena-like: contrast below floor", tex_u(0.0, 0.016, 0.052)),
        // fg_bg_contrast below the 0.15 floor (magicstudio-like).
        (
            "magicstudio-like: contrast below floor",
            tex_u(0.0, 0.003, 0.084),
        ),
        // foreground_energy below the 0.015 floor (even with ample contrast).
        ("foreground below floor", tex_u(0.0, 0.010, 0.30)),
        // the fully-neutral image (all texture knobs 0) — the quietest possible.
        ("neutral image", ImageUnderstanding::neutral()),
    ];

    for (name, img) in calm_band {
        let pick = texture.select(&img);
        assert_eq!(
            pick, "pad_bed",
            "calm image ({name}) below the relaxed floor must keep the pad_bed default, got {pick}"
        );
        let layers = layers_of(&pm, &pick);
        assert!(
            layers.contains(&LayerRole::HarmonicFill),
            "{name}: the calm fallback keeps the static HarmonicFill bed, layers {layers:?}"
        );
        assert!(
            !layers.contains(&LayerRole::CounterMelody),
            "{name}: the calm fallback must NOT carry CounterMelody, layers {layers:?}"
        );
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// PROPERTY C (Move 1) — Rule-ordering preserved: a high-subject-energy image STILL
// selects `pad_figured` (first-match-wins ordering intact; the relaxed pad_bed_counter
// rule did not steal images that qualify for the higher-priority pad_figured rule).
// ═════════════════════════════════════════════════════════════════════════════

/// C: an image with subject_energy ≥ 0.45 AND fg_bg_contrast ≥ 0.25 satisfies BOTH the
/// (higher-priority) pad_figured rule and the (relaxed, lower-priority) pad_bed_counter rule.
/// Because selection is first-match-wins, it MUST still select pad_figured — proving the
/// relaxed pad_bed_counter rule (checked second) cannot steal a pad_figured image.
#[test]
fn test_s45_pad_figured_ordering_preserved() {
    let m = mappings();
    let pm = plan_mappings(&m);
    let texture = &pm.texture;

    // High subject energy + strong contrast: qualifies for BOTH rules; pad_figured wins.
    let salient = [
        ("salient subject, ample contrast", tex_u(0.5, 0.6, 0.30)),
        ("at the pad_figured threshold", tex_u(0.45, 0.6, 0.25)),
    ];

    for (name, img) in salient {
        // Sanity: this image DOES also satisfy the re-tuned pad_bed_counter gate, so the test
        // is meaningful — only the ordering keeps pad_figured ahead of it.
        assert!(
            img.foreground_energy >= 0.015 && img.fg_bg_contrast >= 0.15,
            "{name}: fixture must also satisfy the re-tuned pad_bed_counter gate (else C is vacuous)"
        );
        let pick = texture.select(&img);
        assert_eq!(
            pick, "pad_figured",
            "{name}: a salient subject must STILL select pad_figured (first-match ordering intact), got {pick}"
        );
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// PROPERTY D (Move 2) — Per-section figuration variation: a multi-section plan from a
// FIGURED profile resolves a DIFFERENT figuration on departure sections than on anchor
// sections (block A → broken B → block A′), driven through the REAL planner.
// ═════════════════════════════════════════════════════════════════════════════

/// D: build a real plan from an image that selects a FIGURED texture profile (pad_figured,
/// base figuration "alberti"). Collect each section's resolved figuration id. Assert:
///   - anchor sections (Statement/Return/Coda) carry the BASE cell (alberti),
///   - at least one departure section (Contrast/Development) carries a DIFFERENT cell,
///   - the sequence of per-section figuration ids has ≥2 distinct values (it actually varies),
///   - every resolved id is a real figuration_catalogue entry (no invented texture).
/// Drives the end-to-end `CompositionPlanner::plan` path so `section_figuration_id` (private)
/// is exercised exactly as it fires in production, via the public `Section`/profile surface.
#[test]
fn test_s45_figuration_varies_per_section() {
    let m = mappings();
    let pm = plan_mappings(&m);
    let planner = CompositionPlanner::new(pm.clone());

    // An image that selects pad_figured (subject_energy ≥0.45 AND fg_bg_contrast ≥0.25). The
    // remaining knobs are kept mid so the form ladder yields a multi-section form carrying
    // both anchor and departure roles (rounded_binary: Statement / Contrast / Return).
    let img = ImageUnderstanding {
        subject_energy: 0.6,
        fg_bg_contrast: 0.3,
        foreground_energy: 0.6,
        complexity: 0.5,
        edge_activity: 0.4,
        ..ImageUnderstanding::neutral()
    };
    // Guard the precondition: the texture select MUST be the figured profile, else D is vacuous.
    assert_eq!(
        pm.texture.select(&img),
        "pad_figured",
        "fixture must select the figured texture profile (else the per-section override never arms)"
    );

    let plan = planner.plan(&img, &m);
    assert!(
        plan.sections.len() >= 2,
        "a multi-section form is needed to observe per-section variation, got {} section(s)",
        plan.sections.len()
    );

    // The base figuration cell carried by the selected profile (the once-per-plan resolve).
    let base_id = pm
        .texture_catalogue
        .iter()
        .find(|p| p.id == "pad_figured")
        .and_then(|p| p.figuration.clone())
        .expect("pad_figured carries a base figuration handle");

    // Each section's RESOLVED figuration id (the planner's per-section override output).
    let mut anchor_ids = Vec::new();
    let mut departure_ids = Vec::new();
    let mut all_ids = Vec::new();
    for s in &plan.sections {
        let fig = s
            .orchestration
            .figuration_resolved
            .as_ref()
            .map(|f| f.id.clone())
            .unwrap_or_else(|| {
                panic!(
                    "a figured section ({:?}, role {:?}) must resolve a figuration, got None",
                    s.label, s.thematic_role
                )
            });
        // Every resolved id is a real catalogue entry (no invented texture).
        assert!(
            pm.figuration_catalogue.iter().any(|f| f.id == fig),
            "section {:?} figuration {fig} must be an existing figuration_catalogue entry",
            s.label
        );
        match s.thematic_role {
            ThematicRole::Statement | ThematicRole::Return | ThematicRole::Coda => {
                anchor_ids.push(fig.clone())
            }
            ThematicRole::Contrast | ThematicRole::Development => departure_ids.push(fig.clone()),
        }
        all_ids.push(fig);
    }

    // The piece actually has departure sections to vary (rounded_binary has a Contrast B).
    assert!(
        !departure_ids.is_empty(),
        "the chosen form must contain at least one departure section (Contrast/Development)"
    );

    // Anchor sections keep the BASE cell — the return sounds like the opening (A … A′).
    for (i, id) in anchor_ids.iter().enumerate() {
        assert_eq!(
            *id, base_id,
            "anchor section #{i} must keep the base figuration ({base_id}), got {id}"
        );
    }

    // At least one departure section takes a DIFFERENT cell than the base — the contrast.
    assert!(
        departure_ids.iter().any(|id| *id != base_id),
        "at least one departure section must take a figuration != base ({base_id}); departures {departure_ids:?}"
    );

    // The per-section sequence genuinely VARIES (≥2 distinct values across the piece) —
    // it is no longer one ostinato cell for the whole comp.
    let distinct: std::collections::BTreeSet<&String> = all_ids.iter().collect();
    assert!(
        distinct.len() >= 2,
        "the per-section figuration sequence must vary (≥2 distinct cells), got {all_ids:?}"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// PROPERTY E (Move 2) — Identity / non-figured byte-neutrality: for a profile with
// `figuration == None`, the per-section override is a NO-OP — figuration stays None on
// EVERY section (including departure sections). The freeze-safety pin at planner level.
// ═════════════════════════════════════════════════════════════════════════════

/// E: the per-section figuration override is gated on `orchestration.figuration.is_some()`.
/// A profile whose figuration handle is None (the identity profile, and the always-on
/// pad_bed / pad_bed_counter beds) must therefore keep `figuration_resolved == None` on every
/// section — the override never invents a figuration. Driven two ways:
///   (1) the REAL planner on a NEUTRAL image (selects pad_bed, figuration None): every
///       section — including the departure Contrast B, the one place a leak would show —
///       resolves figuration None. This pins the no-op at plan level, on a real default piece.
///   (2) a direct check that the identity profile carries `figuration == None`, so the gate
///       the override keys on cannot fire (the byte-freeze invariant the design relies on).
/// engine_equivalence.rs covers byte-equality at the engine level; this pins it at the planner.
#[test]
fn test_s45_identity_figuration_stays_none() {
    let m = mappings();
    let pm = plan_mappings(&m);
    let planner = CompositionPlanner::new(pm.clone());

    // (1) A neutral image selects the non-figured pad_bed default — figuration handle None.
    let neutral = ImageUnderstanding::neutral();
    assert_eq!(
        pm.texture.select(&neutral),
        "pad_bed",
        "the neutral image must select the non-figured pad_bed default (the precondition for E)"
    );
    let plan = planner.plan(&neutral, &m);
    assert!(
        plan.sections
            .iter()
            .any(|s| matches!(
                s.thematic_role,
                ThematicRole::Contrast | ThematicRole::Development
            )),
        "the default plan must contain a departure section — the one place an override leak would show"
    );
    for s in &plan.sections {
        // The selected non-figured profile carries no base figuration handle...
        assert!(
            s.orchestration.figuration.is_none(),
            "non-figured section {:?} must carry no base figuration handle (the override gate stays closed)",
            s.label
        );
        // ...so the override never arms and the resolved figuration stays None on EVERY
        // section, departure roles included — byte-stable at plan level.
        assert!(
            s.orchestration.figuration_resolved.is_none(),
            "non-figured section {:?} (role {:?}) must keep figuration_resolved == None (override is a no-op)",
            s.label,
            s.thematic_role
        );
    }

    // (2) The identity profile itself carries figuration None — the gate the override keys on
    // (`figuration.is_some()`) is false for identity, so the override can NEVER fire on the
    // byte-frozen identity path.
    let id_profile = OrchestrationProfile::identity();
    assert!(
        id_profile.is_identity(),
        "the identity profile must report identity"
    );
    assert!(
        id_profile.figuration.is_none(),
        "the identity profile must carry figuration None — the per-section override gate stays closed on the freeze path"
    );
    assert!(
        id_profile.figuration_resolved.is_none(),
        "the identity profile resolves no figuration (byte-frozen path)"
    );
}

// Run under DEFAULT features (the integration harness builds the feature-gated bin, so
// `--no-default-features` cannot RUN this net):
//   cargo test --test variety_s45

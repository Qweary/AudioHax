//! tests/keyplan_k2b.rs — the SLICE-K2b generalized key-scheme CATALOGUE + ROUTING net.
//!
//! K2b (landed as a byte-safe DATA slice on `assets/mappings.json` + `src/composition.rs`, realizer
//! byte-frozen) adds the remaining per-form excursion catalogue rows AND the `key_scheme` routing
//! rules that FIRE them, so the whole-vocabulary tonal travel the K2a engine can already express is
//! now REACHABLE — and adds a `theme_and_variations_excursion` row carrying `resolution: "open"`
//! (the first Open scheme) which ships in the catalogue but is INTENTIONALLY UNROUTED (operator
//! lock — no generated piece ends off-home). This net proves the properties pinned by the kickoff +
//! `docs/spec-s27-k2b-aesthetics.md` + `docs/input-s27-k2b-resolution-policy.md`:
//!
//!   1. routing_reachability_*       — each multi-excursion scheme is REACHABLE: an
//!                                      ImageUnderstanding fixture that fires each shipped routing
//!                                      rule selects the intended scheme id (the K2a dead-data fix —
//!                                      `abac_rondo` and the others are no longer dead).
//!   2. resolve_schemes_land_home    — for EVERY shipped scheme with resolution == Resolve (all 6
//!                                      routed + the legacy/identity rows), the FINAL resolved
//!                                      offset == 0 (strict homecoming).
//!   3. open_schemes_may_end_off_home— the Open scheme (`theme_and_variations_excursion`, UNROUTED)
//!                                      driven DIRECTLY resolves a final offset in the legal menu
//!                                      {+7,+5,+3,−3,0}, PLUS a reachability witness whose rank-1
//!                                      region resolves a NON-ZERO final → Open genuinely CAN end
//!                                      off-home (the deliberate-feature witness).
//!   4. no_routed_image_ends_off_home— driving the SHIPPED routing over a fixture set hitting each
//!                                      rule, every generated plan ends home (the operator lock; this
//!                                      FAILS loudly if an Open scheme is ever routed by accident).
//!   5. at_most_two_distinct_non_home_keys — every routed scheme resolves ≤ 2 distinct non-zero
//!                                      offsets (the "eye sweeps ≤ twice" bound; abbac is the stress).
//!   6. home_only_byte_zero          — `home_only` resolves to all-zero (the byte-stable identity).
//!   7. smooth_keys_only             — every non-zero offset in every position ∈ {+7,+5,+3,−3}.
//!   8. tv_twins_differ_only_in_resolution — the two T&V rows share identical sections and differ
//!                                      ONLY in `resolution` → Resolve forces the final 0 while Open
//!                                      keeps it off-home (policy alone changes the ending).
//!
//! HEADLESS, in the same sense as keyplan_k2a.rs / keyplan_s25.rs: NO image type, NO OpenCV, NO
//! audio hardware. The catalogue/resolve internals (`resolve_key_scheme`, `lookup_key_scheme`,
//! `region_excursion_offset`, `parse_offset_rule`) are module-PRIVATE in `src/composition.rs`, so —
//! exactly like keyplan_k2a.rs — this integration net CANNOT call them directly and drives
//! EVERYTHING through the PUBLIC entry point `CompositionPlanner::plan`, reading the resolved
//! `KeyTempoPlan.key_scheme: Vec<i8>` + the per-section `key_offset_semitones`. The catalogue types
//! `PlanMappings` / `KeyScheme` / `KeySchemeSection` / `ResolutionPolicy` / `SelectTable` /
//! `SelectRule` / `Predicate` / `Knob` / `CmpOp` are PUBLIC, so the SHIPPED catalogue can be
//! iterated and individual schemes can be steered onto their aligned form for a direct resolve.
//!
//! TWO DRIVING MODES (both through `plan()`):
//!   * SHIPPED ROUTING (properties 1 + 4): the planner is built from the SHIPPED mappings with NO
//!     steering, so the SHIPPED `key_scheme` + `form` SelectTables select the scheme/form — this is
//!     what proves real images reach each scheme and that routing never lands off-home.
//!   * PINNED RESOLVE (properties 2,3,5,7,8): for a chosen scheme id, the `form` axis is pinned to
//!     the scheme's ALIGNED form (so the role-alignment debug witness in `resolve_key_scheme` stays
//!     satisfied) and the `key_scheme` axis is pinned to that id, then a FIRING image is planned and
//!     the resolved `key_scheme` Vec is asserted. This is how the UNROUTED Open scheme + the
//!     conditional resolve are exercised "directly" within the public API.
//!
//! RNG-BOUNDARY DISCIPLINE (same as keyplan_k2a.rs): `plan()` delegates per-section harmony to a
//! `thread_rng` path, so chords / Roman numerals are NON-deterministic and NEVER asserted. Every
//! property here is RNG-INDEPENDENT — the per-section `key_offset_semitones` and the
//! `KeyTempoPlan.key_scheme` are computed by the pure `resolve_key_scheme` BEFORE any RNG runs.

use audiohax::composition::{
    CmpOp, CompositionPlan, CompositionPlanner, ImageUnderstanding, Knob, PlanMappings, Predicate,
    ResolutionPolicy, SelectRule, SelectTable,
};
use audiohax::mapping_loader::{load_mappings, MappingTable};

const MAPPINGS_PATH: &str = "assets/mappings.json";

/// The v1 menu set every NON-ZERO offset must belong to: dominant +7, subdominant +5, relative ±3.
const MENU: [i8; 4] = [7, 5, 3, -3];

/// The Open-branch legal final-offset set (the v1 menu OR home) — `input-s27` §3.2.
const OPEN_FINAL_MENU: [i8; 5] = [7, 5, 3, -3, 0];

/// The shipped `assets/mappings.json` mapping table (real `global` harmony data + the K2b catalogue).
fn mappings() -> MappingTable {
    load_mappings(MAPPINGS_PATH).expect("mappings.json loads")
}

/// The shipped composition `PlanMappings` (real form/key SelectTables + the K2b catalogue).
fn base_plan_mappings(m: &MappingTable) -> PlanMappings {
    m.composition
        .clone()
        .expect("composition block present in mappings.json")
        .into()
}

/// A planner built from the SHIPPED mappings with NO steering — the SHIPPED `key_scheme`/`form`
/// SelectTables make every selection. Used by the routing-reachability + operator-lock tests so the
/// REAL routing data is what is under test, not a synthetic table.
fn shipped_planner(m: &MappingTable) -> CompositionPlanner {
    CompositionPlanner::new(base_plan_mappings(m))
}

/// A `SelectTable` that ALWAYS resolves to `id` (empty rule set → the default fires). Pins the
/// `form` / `key_scheme` axis to a chosen id so a specific scheme can be resolved on its aligned
/// form regardless of the image — exactly the keyplan_k2a.rs `always` helper.
fn always(id: &str) -> SelectTable {
    SelectTable {
        default: id.to_string(),
        rules: Vec::new(),
    }
}

/// The form whose section roles align 1:1 with a given scheme id (so the planner's role-alignment
/// debug witness stays satisfied when the scheme is pinned onto it). Mirrors the shipped
/// order-isomorphic routing (spec §1.1): each excursion scheme's namesake form.
fn aligned_form(scheme_id: &str) -> &'static str {
    match scheme_id {
        "home_only" => "rounded_binary", // empty sections align with any form
        "aba_excursion" => "ternary_aba", // legacy [A,B,A] aligns to ternary's [A,B,A]
        "rounded_binary_excursion" => "rounded_binary",
        "ternary_aba_excursion" => "ternary_aba",
        "aaba_excursion" => "aaba",
        "abac_rondo" => "abac",
        "abbac_excursion" => "abbac",
        "theme_and_variations_resolve" => "theme_and_variations",
        "theme_and_variations_excursion" => "theme_and_variations",
        other => panic!("no aligned form known for scheme id {other}"),
    }
}

/// A planner with the `form` pinned to `scheme_id`'s aligned form and the `key_scheme` pinned to
/// `scheme_id` (the SHIPPED catalogue is kept intact, so the resolved row is the REAL shipped row).
/// This is the "resolve one shipped scheme directly" driver — the only way to exercise the UNROUTED
/// Open scheme + a per-scheme conditional resolve through the public `plan()` API.
fn planner_pinned_to(scheme_id: &str, m: &MappingTable) -> CompositionPlanner {
    let mut pm = base_plan_mappings(m);
    pm.form = always(aligned_form(scheme_id));
    pm.key_scheme = always(scheme_id);
    CompositionPlanner::new(pm)
}

/// A FIRING image with DISTINCT per-region affect so rank-0 (B) and rank-1 (C) read genuinely
/// different keys: foreground = rank-0 (more energetic), BRIGHT + near-hue → dominant +7; background
/// = rank-1, DARK + near-hue → subdominant +5. `fg_bg_contrast 0.30` fires the subject gate. All
/// region hues == subject_hue (40) so the NEAR path holds on both excursions (the test isolates the
/// per-region VALENCE axis, exactly the keyplan_k2a.rs `craft_regions` discipline).
fn firing_distinct_regions() -> ImageUnderstanding {
    ImageUnderstanding {
        fg_bg_contrast: 0.30,
        subject_hue: 40.0,
        avg_saturation: 50.0,
        foreground_energy: 0.9,     // rank 0 → B
        foreground_brightness: 0.9, // BRIGHT → +7
        foreground_hue: 40.0,       // near
        background_energy: 0.3,     // rank 1 → C
        background_brightness: 0.1, // DARK → +5
        background_hue: 40.0,       // near
        ..ImageUnderstanding::neutral()
    }
}

/// The distinct NON-ZERO offsets across a `key_scheme`, sorted + deduped.
fn distinct_nonzero(scheme: &[i8]) -> Vec<i8> {
    let mut v: Vec<i8> = scheme.iter().copied().filter(|&o| o != 0).collect();
    v.sort_unstable();
    v.dedup();
    v
}

/// The resolved `key_scheme` Vec<i8> from a plan (the planner's per-section offset spine).
fn resolved(plan: &CompositionPlan) -> &[i8] {
    &plan.key_tempo.key_scheme
}

// ═════════════════════════════════════════════════════════════════════════════
// Property 1 — ROUTING REACHABILITY (the K2a dead-data fix: each scheme is REACHABLE)
// ═════════════════════════════════════════════════════════════════════════════
//
// Each fixture fires exactly ONE shipped `key_scheme` rule as the first match, and (because the
// routing is order-isomorphic to the `form` ladder + shares the `fg_bg_contrast >= 0.25` gate) the
// SAME predicates make the twin `form` rule the first match — so the selected scheme aligns with the
// selected form and the role-alignment debug witness stays quiet. Asserted through the SHIPPED
// planner (no steering): proof that a REAL image reaches the scheme. Defaults that matter for
// non-interference: aspect_ratio 1.0 (< 1.6, so the aaba rule needs an explicit wide ratio),
// quadrant_contrast 0.0, vertical_emphasis 0.5 (< 0.6), complexity 0.0, edge_activity 0.0,
// value_key 0.0, palette_bimodality 0.0.

/// Rule 1 — `theme_and_variations_resolve` ← complexity ≥ 0.66 AND edge_activity ≥ 0.6 AND
/// fg_bg_contrast ≥ 0.25 (mirrors the `theme_and_variations` form trigger). This is the ROUTED twin
/// of the Open scheme (the Resolve T&V), so its FINAL still lands home — asserted in property 2/8.
#[test]
fn routing_reachability_tv_resolve() {
    let m = mappings();
    let planner = shipped_planner(&m);
    let u = ImageUnderstanding {
        fg_bg_contrast: 0.30,
        complexity: 0.7,     // ≥ 0.66
        edge_activity: 0.65, // ≥ 0.6 (but < 0.7 so the abbac rule cannot pre-empt)
        value_key: 0.0,      // keep abbac's value_key gate shut
        ..firing_distinct_regions()
    };
    let plan = planner.plan(&u, &m);
    assert_eq!(
        plan.form, "theme_and_variations",
        "the complexity+edge fixture must select the theme_and_variations FORM, got {}",
        plan.form
    );
    // The shipped key_scheme routes this image onto the RESOLVE T&V twin (the Open one is unrouted).
    assert_eq!(
        resolved(&plan).len(),
        3,
        "theme_and_variations is a 3-section form, got scheme {:?}",
        resolved(&plan)
    );
    // It FIRED a real excursion (some interior section is non-zero) — not the home_only default.
    assert!(
        resolved(&plan).iter().any(|&o| o != 0),
        "the routed T&V scheme must FIRE a non-zero excursion (proving it is not home_only), \
         got {:?}",
        resolved(&plan)
    );
    // Resolve twin ⇒ ends home (the operator lock; the Open twin is the UNROUTED one).
    assert_eq!(
        *resolved(&plan).last().unwrap(),
        0,
        "the ROUTED T&V scheme is the Resolve twin and must end home, got {:?}",
        resolved(&plan)
    );
}

/// Rule 2 — `ternary_aba_excursion` ← quadrant_contrast ≥ 0.6 AND fg_bg_contrast ≥ 0.25.
#[test]
fn routing_reachability_ternary() {
    let m = mappings();
    let planner = shipped_planner(&m);
    let u = ImageUnderstanding {
        fg_bg_contrast: 0.30,
        quadrant_contrast: 0.7, // ≥ 0.6 → rule 2
        ..firing_distinct_regions()
    };
    let plan = planner.plan(&u, &m);
    assert_eq!(
        plan.form, "ternary_aba",
        "quadrant_contrast ≥ 0.6 must select ternary_aba, got {}",
        plan.form
    );
    // ternary_aba_excursion: [home, region_related:b, home] under Resolve → [0, b, 0].
    let ks = resolved(&plan);
    assert_eq!(ks.len(), 3, "ternary_aba is 3 sections, got {ks:?}");
    assert_eq!(ks[0], 0, "Statement is home, got {ks:?}");
    assert_ne!(
        ks[1], 0,
        "the B Contrast must travel (reachable!), got {ks:?}"
    );
    assert_eq!(ks[2], 0, "Return resolves home, got {ks:?}");
}

/// Rule 3 — `aaba_excursion` ← aspect_ratio ≥ 1.6 AND palette_bimodality ≤ 0.3 AND fg_bg ≥ 0.25.
#[test]
fn routing_reachability_aaba() {
    let m = mappings();
    let planner = shipped_planner(&m);
    let u = ImageUnderstanding {
        fg_bg_contrast: 0.30,
        aspect_ratio: 1.7,       // ≥ 1.6
        palette_bimodality: 0.1, // ≤ 0.3
        ..firing_distinct_regions()
    };
    let plan = planner.plan(&u, &m);
    assert_eq!(
        plan.form, "aaba",
        "aspect_ratio ≥ 1.6 + palette_bimodality ≤ 0.3 must select aaba, got {}",
        plan.form
    );
    // aaba_excursion: [home, home, region_related:b, home] under Resolve → [0,0,b,0].
    let ks = resolved(&plan);
    assert_eq!(ks.len(), 4, "aaba is 4 sections, got {ks:?}");
    assert_eq!(ks[0], 0, "Statement is home, got {ks:?}");
    assert_eq!(ks[1], 0, "second Statement is home, got {ks:?}");
    assert_ne!(
        ks[2], 0,
        "the B bridge must travel (reachable!), got {ks:?}"
    );
    assert_eq!(ks[3], 0, "Return resolves home, got {ks:?}");
}

/// Rule 4 — `abac_rondo` ← vertical_emphasis ≥ 0.6 AND fg_bg_contrast ≥ 0.25. THE K2a-flagged
/// dead-data case: abac_rondo had NO selecting rule pre-K2b. This proves it is now REACHABLE.
#[test]
fn routing_reachability_abac_rondo_no_longer_dead() {
    let m = mappings();
    let planner = shipped_planner(&m);
    let u = ImageUnderstanding {
        fg_bg_contrast: 0.30,
        vertical_emphasis: 0.7, // ≥ 0.6 → rule 4 (vs the 0.5 neutral default which is < 0.6)
        ..firing_distinct_regions()
    };
    let plan = planner.plan(&u, &m);
    assert_eq!(
        plan.form, "abac",
        "vertical_emphasis ≥ 0.6 must select the abac form, got {}",
        plan.form
    );
    // abac_rondo: [home, region_related:b, home, region_related:c] under Resolve → [0, b, 0, 0].
    let ks = resolved(&plan);
    assert_eq!(ks.len(), 4, "abac is 4 sections, got {ks:?}");
    assert_eq!(ks[0], 0, "Statement is home, got {ks:?}");
    assert_ne!(
        ks[1], 0,
        "abac_rondo's B Contrast must travel — abac_rondo is NO LONGER DEAD DATA, got {ks:?}"
    );
    assert_eq!(ks[2], 0, "Return is home, got {ks:?}");
    assert_eq!(
        ks[3], 0,
        "the Coda's region_related:c rule is forced home by Resolve under K2b, got {ks:?}"
    );
}

/// Rule 5 — `abbac_excursion` ← edge_activity ≥ 0.7 AND value_key ≥ 0.6 AND fg_bg ≥ 0.25. The
/// longest episodic sweep: the two B's (rank-0 / rank-1) DIVERGE. NB the complexity knob is left low
/// so the earlier T&V rule (complexity ≥ 0.66 & edge ≥ 0.6) does NOT pre-empt this one.
#[test]
fn routing_reachability_abbac() {
    let m = mappings();
    let planner = shipped_planner(&m);
    let u = ImageUnderstanding {
        fg_bg_contrast: 0.30,
        edge_activity: 0.8, // ≥ 0.7
        value_key: 0.7,     // ≥ 0.6
        complexity: 0.0,    // < 0.66 so the T&V rule (rule 1) cannot win first
        ..firing_distinct_regions()
    };
    let plan = planner.plan(&u, &m);
    assert_eq!(
        plan.form, "abbac",
        "edge_activity ≥ 0.7 + value_key ≥ 0.6 must select abbac, got {}",
        plan.form
    );
    // abbac_excursion: [home, region_related:b, region_related:c, home, region_related:c] under
    // Resolve → [0, b, c, 0, 0]; b (rank-0 bright) and c (rank-1 dark) DIVERGE.
    let ks = resolved(&plan);
    assert_eq!(ks.len(), 5, "abbac is 5 sections, got {ks:?}");
    assert_eq!(ks[0], 0, "Statement is home, got {ks:?}");
    assert_ne!(ks[1], 0, "B must travel, got {ks:?}");
    assert_ne!(ks[2], 0, "B' must travel, got {ks:?}");
    assert_ne!(
        ks[1], ks[2],
        "the two B's read rank-0/rank-1 distinct regions and must DIVERGE (the eye sweeps twice), \
         got {ks:?}"
    );
    assert_eq!(ks[3], 0, "Return is home, got {ks:?}");
    assert_eq!(ks[4], 0, "Coda resolved home by Resolve, got {ks:?}");
}

/// Rule 6 — `rounded_binary_excursion` ← fg_bg_contrast ≥ 0.25 (the catch-all, last; any clear
/// subject with no special travelling character). A bare subject image with NO other discriminator
/// falls to this rule AND to the default `rounded_binary` form.
#[test]
fn routing_reachability_rounded_binary_catchall() {
    let m = mappings();
    let planner = shipped_planner(&m);
    // ONLY the subject gate fires; every other discriminating knob is at its non-firing default.
    let u = firing_distinct_regions();
    let plan = planner.plan(&u, &m);
    assert_eq!(
        plan.form, "rounded_binary",
        "a bare subject image must fall to the default rounded_binary form, got {}",
        plan.form
    );
    // rounded_binary_excursion: [home, region_related:b, home] under Resolve → [0, b, 0].
    let ks = resolved(&plan);
    assert_eq!(ks.len(), 3, "rounded_binary is 3 sections, got {ks:?}");
    assert_eq!(ks[0], 0, "Statement is home, got {ks:?}");
    assert_ne!(
        ks[1], 0,
        "the catch-all single excursion must travel, got {ks:?}"
    );
    assert_eq!(ks[2], 0, "Return resolves home, got {ks:?}");
}

// ═════════════════════════════════════════════════════════════════════════════
// Property 2 — resolve_schemes_land_home (CONDITIONAL resolves_home, Resolve branch)
// ═════════════════════════════════════════════════════════════════════════════

/// `input-s27` §3.1: for EVERY shipped catalogue scheme whose `resolution == Resolve` (including the
/// `#[serde(default)]` identity/legacy rows `home_only` / `aba_excursion`), the FINAL resolved
/// offset == 0 — the strict homecoming guarantee. Iterate the SHIPPED catalogue, FILTER to Resolve,
/// and for each scheme resolve it (pinned onto its aligned form) over the firing image, asserting
/// `last() == 0`. This is the K1 `resolves_home` property made CONDITIONAL on policy.
#[test]
fn resolve_schemes_land_home() {
    let m = mappings();
    let pm = base_plan_mappings(&m);

    let mut checked = 0usize;
    for scheme in &pm.key_scheme_catalogue {
        if scheme.resolution != ResolutionPolicy::Resolve {
            continue; // the Open branch is property 3.
        }
        let planner = planner_pinned_to(&scheme.id, &m);
        let plan = planner.plan(&firing_distinct_regions(), &m);
        let ks = resolved(&plan);
        // home_only has empty sections → the form's sections still produce an all-zero spine; the
        // final is trivially 0. Every other Resolve scheme has the final forced to 0 by policy.
        assert!(
            !ks.is_empty(),
            "scheme {} resolved an empty spine (the aligned form must have sections)",
            scheme.id
        );
        assert_eq!(
            *ks.last().unwrap(),
            0,
            "Resolve scheme {} must land home (final offset 0), got {ks:?}",
            scheme.id
        );
        checked += 1;
    }
    // The catalogue actually carries the expected Resolve rows (so this is not a vacuous loop).
    assert!(
        checked >= 7,
        "expected at least 7 Resolve schemes (home_only, aba_excursion, rounded_binary_excursion, \
         ternary_aba_excursion, aaba_excursion, abac_rondo, abbac_excursion, \
         theme_and_variations_resolve), only checked {checked}"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Property 3 — open_schemes_may_end_off_home (CONDITIONAL resolves_home, Open branch)
// ═════════════════════════════════════════════════════════════════════════════

/// `input-s27` §3.2: for EVERY shipped scheme whose `resolution == Open`, the final resolved offset
/// MUST be a legal menu value ∈ {+7,+5,+3,−3,0} (coherent — never garbage). The shipped Open scheme
/// `theme_and_variations_excursion` is UNROUTED, so it is driven DIRECTLY by pinning it onto its
/// aligned `theme_and_variations` form. Two parts:
///   (a) over a sweep of firing images the final offset is ALWAYS in OPEN_FINAL_MENU; and
///   (b) the deliberate-feature WITNESS — at least one firing image makes the Open scheme's final
///       (V2, rank-1) resolve a NON-ZERO menu value, proving Open genuinely CAN end off-home.
#[test]
fn open_schemes_may_end_off_home() {
    let m = mappings();
    let pm = base_plan_mappings(&m);

    // Confirm an Open scheme actually ships (the conditional split is non-vacuous).
    let open_ids: Vec<String> = pm
        .key_scheme_catalogue
        .iter()
        .filter(|s| s.resolution == ResolutionPolicy::Open)
        .map(|s| s.id.clone())
        .collect();
    assert!(
        open_ids
            .iter()
            .any(|id| id == "theme_and_variations_excursion"),
        "the shipped catalogue must carry the Open `theme_and_variations_excursion` row, got Open \
         ids {open_ids:?}"
    );

    // (a) Legal-menu floor across a sweep of region affect (so the final is exercised over variety,
    //     not one input). Each case drives the rank-1 region (→ V2, the Open final) to a different
    //     menu value: bright-near (+7), dark-near (+5), far-hue (relative −3 on the Ionian home).
    for id in &open_ids {
        let planner = planner_pinned_to(id, &m);
        let cases = [
            firing_distinct_regions(), // rank-1 (bg) dark near → +5 final
            ImageUnderstanding {
                background_brightness: 0.9, // rank-1 bright near → +7 final
                ..firing_distinct_regions()
            },
            ImageUnderstanding {
                // rank-1 (bg) far-hue (200° vs subject 40° = 160° ≥ 60°) → relative on the home mode.
                background_hue: 200.0,
                dominant_hue: 120.0, // Ionian home → relative −3
                ..firing_distinct_regions()
            },
        ];
        for (k, u) in cases.iter().enumerate() {
            let plan = planner.plan(u, &m);
            let ks = resolved(&plan);
            let final_off = *ks.last().expect("a non-empty Open spine");
            assert!(
                OPEN_FINAL_MENU.contains(&final_off),
                "Open scheme {id} case {k}: final offset {final_off} must be in \
                 {{+7,+5,+3,−3,0}}, got scheme {ks:?}"
            );
            // The shared smooth-keys floor still binds every position.
            assert!(
                ks.iter().all(|&o| o == 0 || MENU.contains(&o)),
                "Open scheme {id} case {k}: off-menu offset in {ks:?}"
            );
        }
    }

    // (b) Deliberate-feature WITNESS: a firing image whose rank-1 region resolves a NON-ZERO menu
    //     value makes the Open final OFF-home. (firing_distinct_regions sets the rank-1 background
    //     DARK near → +5.) This is the proof that Open is not a vacuous "always-0 anyway" branch.
    let planner = planner_pinned_to("theme_and_variations_excursion", &m);
    let plan = planner.plan(&firing_distinct_regions(), &m);
    let ks = resolved(&plan);
    let final_off = *ks.last().unwrap();
    assert_ne!(
        final_off, 0,
        "the Open scheme's final (V2, rank-1) must be reachable OFF-home — the deliberate open \
         ending. Got final {final_off} from {ks:?} (if 0, the rank-1 region failed to travel)"
    );
    assert!(
        MENU.contains(&final_off),
        "the off-home Open final must still be a closely-related menu key, got {final_off}"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Property 4 — no_routed_image_ends_off_home (the OPERATOR LOCK regression)
// ═════════════════════════════════════════════════════════════════════════════

/// The operator lock: NO generated piece ends off-home in K2b, because the one Open scheme is
/// UNROUTED. Drive the SHIPPED routing (no steering) over a fixture set that hits EACH active rule
/// (1)–(6) and assert `resolved().last() == 0` for ALL of them. This holds precisely BECAUSE the
/// Open `theme_and_variations_excursion` is not routed (the routed T&V twin is the Resolve one). It
/// FAILS LOUDLY if a future routing edit points an active rule at an Open scheme — making the lock a
/// live tripwire, not an assumption.
#[test]
fn no_routed_image_ends_off_home() {
    let m = mappings();
    let planner = shipped_planner(&m);

    // (label, fixture) — one firing image per active routing rule, reusing the property-1 fixtures.
    let tv = ImageUnderstanding {
        fg_bg_contrast: 0.30,
        complexity: 0.7,
        edge_activity: 0.65,
        ..firing_distinct_regions()
    };
    let ternary = ImageUnderstanding {
        quadrant_contrast: 0.7,
        ..firing_distinct_regions()
    };
    let aaba = ImageUnderstanding {
        aspect_ratio: 1.7,
        palette_bimodality: 0.1,
        ..firing_distinct_regions()
    };
    let abac = ImageUnderstanding {
        vertical_emphasis: 0.7,
        ..firing_distinct_regions()
    };
    let abbac = ImageUnderstanding {
        edge_activity: 0.8,
        value_key: 0.7,
        complexity: 0.0,
        ..firing_distinct_regions()
    };
    let rounded = firing_distinct_regions();

    let cases: [(&str, &ImageUnderstanding); 6] = [
        ("rule1 → theme_and_variations_resolve", &tv),
        ("rule2 → ternary_aba_excursion", &ternary),
        ("rule3 → aaba_excursion", &aaba),
        ("rule4 → abac_rondo", &abac),
        ("rule5 → abbac_excursion", &abbac),
        ("rule6 → rounded_binary_excursion", &rounded),
    ];

    for (label, u) in cases {
        let plan = planner.plan(u, &m);
        let ks = resolved(&plan);
        assert!(!ks.is_empty(), "[{label}] resolved an empty spine");
        assert_eq!(
            *ks.last().unwrap(),
            0,
            "OPERATOR LOCK VIOLATED — [{label}] ends OFF-home (final {:?} ≠ 0). Under K2b NO \
             generated piece may end off-home: the only Open scheme is UNROUTED. If this fires, an \
             Open scheme was routed by accident — restore the lock.",
            ks
        );
        // And the routed scheme genuinely FIRED (some non-zero) — so "ends home" is a real
        // recapitulation, not a vacuous all-home plan that would pass the lock trivially.
        assert!(
            ks.iter().any(|&o| o != 0),
            "[{label}] the routed scheme must FIRE a non-zero excursion, got {ks:?}"
        );
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Property 5 — at_most_two_distinct_non_home_keys (the eye sweeps ≤ twice)
// ═════════════════════════════════════════════════════════════════════════════

/// For EVERY routed scheme the resolved offsets contain AT MOST 2 distinct non-zero values (the
/// multi-excursion short-piece cap). `abbac_excursion` is the stress case: its rank-0/rank-1 B's may
/// share or diverge but together with C can never exceed 2 distinct non-home keys. Asserted by
/// pinning each routed scheme onto its aligned form across a sweep of region affect that drives B/C
/// to the same key, to distinct keys, and to the relative — so the cap is over real variety.
#[test]
fn at_most_two_distinct_non_home_keys() {
    let m = mappings();

    // (fg_bright, bg_bright, bg_hue) cases spanning same / distinct / relative for the two regions.
    // subject_hue 40, foreground near (hue 40); only the background's brightness/hue varies.
    let region_cases = [
        (0.9f32, 0.9f32, 40.0f32), // both bright near → both +7 (1 distinct)
        (0.9, 0.1, 40.0),          // bright + dark near → +7,+5 (2 distinct)
        (0.9, 0.9, 200.0),         // fg +7 + bg far-hue relative (2 distinct)
        (0.1, 0.1, 40.0),          // both dark near → both +5 (1 distinct)
    ];
    let routed = [
        "rounded_binary_excursion",
        "ternary_aba_excursion",
        "aaba_excursion",
        "abac_rondo",
        "abbac_excursion",
        "theme_and_variations_resolve",
    ];
    for id in routed {
        let planner = planner_pinned_to(id, &m);
        for &(fb, bb, bh) in &region_cases {
            let u = ImageUnderstanding {
                foreground_brightness: fb,
                background_brightness: bb,
                background_hue: bh,
                dominant_hue: 120.0, // Ionian home so the relative is a clean −3
                ..firing_distinct_regions()
            };
            let plan = planner.plan(&u, &m);
            let ks = resolved(&plan);
            let distinct = distinct_nonzero(ks);
            assert!(
                distinct.len() <= 2,
                "scheme {id}: at most TWO distinct non-home keys (eye sweeps ≤ twice); got \
                 {distinct:?} from {ks:?} (fg {fb} bg {bb} bg_hue {bh})"
            );
            assert!(
                ks.iter().all(|&o| o == 0 || MENU.contains(&o)),
                "scheme {id}: off-menu offset in {ks:?} (fg {fb} bg {bb} bg_hue {bh})"
            );
        }
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Property 6 — home_only_byte_zero (the byte-stable identity default)
// ═════════════════════════════════════════════════════════════════════════════

/// `home_only` resolves to ALL-ZERO offsets — the byte-stable identity default. Driven two ways:
///   (a) pinned `home_only` (empty sections) over its aligned form + a VIVID firing image: even
///       vivid per-region affect cannot move an offset because no region_related rule exists; and
///   (b) the SHIPPED routing with the subject gate NOT fired (fg_bg_contrast < 0.25) → the default
///       `home_only` is selected → all-zero.
#[test]
fn home_only_byte_zero() {
    let m = mappings();

    // (a) home_only pinned, vivid affect — still all-zero (no rule drives any section).
    let planner = planner_pinned_to("home_only", &m);
    let plan_a = planner.plan(&firing_distinct_regions(), &m);
    assert!(
        resolved(&plan_a).iter().all(|&o| o == 0),
        "home_only (pinned) must be all-zero even under vivid affect, got {:?}",
        resolved(&plan_a)
    );
    assert!(
        plan_a.sections.iter().all(|s| s.key_offset_semitones == 0),
        "home_only: every section stays home, got {:?}",
        plan_a
            .sections
            .iter()
            .map(|s| s.key_offset_semitones)
            .collect::<Vec<_>>()
    );

    // (b) Shipped routing, subject gate NOT fired (fg_bg_contrast 0.0 < 0.25) → home_only default.
    let shipped = shipped_planner(&m);
    let below_gate = ImageUnderstanding {
        fg_bg_contrast: 0.0, // below the 0.25 subject gate on every active rule
        ..firing_distinct_regions()
    };
    let plan_b = shipped.plan(&below_gate, &m);
    assert!(
        resolved(&plan_b).iter().all(|&o| o == 0),
        "below the subject gate, the shipped routing must select home_only → all-zero, got {:?}",
        resolved(&plan_b)
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Property 7 — smooth_keys_only (every non-zero offset ∈ {+7,+5,+3,−3})
// ═════════════════════════════════════════════════════════════════════════════

/// The K1/K2a smooth-keys floor, carried to K2b: across a SWEEP of region affect × home mode, on
/// EVERY routed scheme AND the Open scheme, every non-zero resolved offset (in ANY position) is in
/// the v1 menu {+7,+5,+3,−3} — no garbage keys escape. A real nested sweep (scheme × region-case ×
/// home-mode), not a token case.
#[test]
fn smooth_keys_only() {
    let m = mappings();

    let all_schemes = [
        "rounded_binary_excursion",
        "ternary_aba_excursion",
        "aaba_excursion",
        "abac_rondo",
        "abbac_excursion",
        "theme_and_variations_resolve",
        "theme_and_variations_excursion", // the Open scheme too
        "aba_excursion",                  // legacy K1 row
    ];
    // (fg_bright, bg_bright, fg_hue_off, bg_hue_off) — valence × hue-distance variety per region.
    let region_cases = [
        (0.9f32, 0.1f32, 0.0f32, 0.0f32), // bright/dark, both near
        (0.1, 0.9, 0.0, 0.0),             // dark/bright, both near
        (0.5, 0.5, 90.0, 0.0),            // mid valence, fg far-hue → relative
        (0.9, 0.9, 0.0, 120.0),           // both bright, bg far-hue → relative
    ];
    let home_hues = [
        120.0f32, /*Ionian → −3*/
        250.0,    /*Aeolian → +3*/
    ];

    let mut nonzero_seen = 0usize;
    let mut combos = 0usize;
    for id in all_schemes {
        let planner = planner_pinned_to(id, &m);
        for &(fb, bb, fh_off, bh_off) in &region_cases {
            for &home_hue in &home_hues {
                let u = ImageUnderstanding {
                    dominant_hue: home_hue,
                    subject_hue: 40.0,
                    foreground_brightness: fb,
                    background_brightness: bb,
                    foreground_hue: (40.0 + fh_off) % 360.0,
                    background_hue: (40.0 + bh_off) % 360.0,
                    ..firing_distinct_regions()
                };
                let plan = planner.plan(&u, &m);
                let ks = resolved(&plan);
                assert!(
                    ks.iter().all(|&o| o == 0 || MENU.contains(&o)),
                    "OFF-MENU offset on scheme {id}: {ks:?} (fg {fb} bg {bb} fh_off {fh_off} \
                     bh_off {bh_off} home_hue {home_hue})"
                );
                nonzero_seen += ks.iter().filter(|&&o| o != 0).count();
                combos += 1;
            }
        }
    }
    assert_eq!(
        combos,
        all_schemes.len() * region_cases.len() * home_hues.len(),
        "the sweep must be the full nested cross-product"
    );
    assert!(
        nonzero_seen > 0,
        "the sweep must have exercised non-zero offsets to validate against the menu"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Property 8 — tv_twins_differ_only_in_resolution (policy alone changes the ending)
// ═════════════════════════════════════════════════════════════════════════════

/// The two `theme_and_variations` rows (`theme_and_variations_resolve` vs
/// `theme_and_variations_excursion`) share IDENTICAL sections and differ ONLY in `resolution`.
/// First assert that data fact against the SHIPPED catalogue, then drive BOTH through `plan()` on
/// the SAME firing image (each pinned onto the shared `theme_and_variations` form) and confirm:
///   * the Resolve twin forces the final to 0 (lands home);
///   * the Open twin keeps the final off-home;
///   * the NON-final sections are IDENTICAL between the twins (only the policy touches the ending).
#[test]
fn tv_twins_differ_only_in_resolution() {
    let m = mappings();
    let pm = base_plan_mappings(&m);

    let resolve_row = pm
        .key_scheme_catalogue
        .iter()
        .find(|s| s.id == "theme_and_variations_resolve")
        .expect("shipped catalogue must carry theme_and_variations_resolve");
    let open_row = pm
        .key_scheme_catalogue
        .iter()
        .find(|s| s.id == "theme_and_variations_excursion")
        .expect("shipped catalogue must carry theme_and_variations_excursion");

    // The data fact: identical sections (same labels + offset_rules), differing resolution.
    assert_eq!(
        resolve_row.sections, open_row.sections,
        "the two T&V twins must share IDENTICAL sections; resolve {:?} vs open {:?}",
        resolve_row.sections, open_row.sections
    );
    assert_eq!(
        resolve_row.resolution,
        ResolutionPolicy::Resolve,
        "theme_and_variations_resolve must be Resolve"
    );
    assert_eq!(
        open_row.resolution,
        ResolutionPolicy::Open,
        "theme_and_variations_excursion must be Open"
    );

    // Drive both on the SAME firing image (rank-1 background DARK near → V2 pre-policy = +5).
    let u = firing_distinct_regions();
    let plan_res = planner_pinned_to("theme_and_variations_resolve", &m).plan(&u, &m);
    let plan_open = planner_pinned_to("theme_and_variations_excursion", &m).plan(&u, &m);
    let ks_res = resolved(&plan_res);
    let ks_open = resolved(&plan_open);

    assert_eq!(ks_res.len(), 3, "T&V is 3 sections, got resolve {ks_res:?}");
    assert_eq!(ks_open.len(), 3, "T&V is 3 sections, got open {ks_open:?}");

    // Resolve forces the final home; Open keeps it off-home.
    assert_eq!(
        ks_res[2], 0,
        "Resolve twin must land the final V2 home, got {ks_res:?}"
    );
    assert_ne!(
        ks_open[2], 0,
        "Open twin must keep the final V2 off-home, got {ks_open:?}"
    );
    assert!(
        MENU.contains(&ks_open[2]),
        "the Open final must still be a menu key, got {}",
        ks_open[2]
    );

    // The policy is the ONLY difference: the non-final sections are byte-identical between twins.
    assert_eq!(
        &ks_res[..2],
        &ks_open[..2],
        "the non-final sections must be IDENTICAL between the twins (only the policy touches the \
         ending); resolve {ks_res:?} vs open {ks_open:?}"
    );
    // And the divergence is exactly at the final position.
    assert_ne!(
        ks_res[2], ks_open[2],
        "the resolution policy alone changes the FINAL offset (Resolve 0 vs Open {})",
        ks_open[2]
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// A compile-time touch of the imported predicate-builder types, so an unused-import
// warning can never mask a real API drift. (These types are the public routing-rule
// surface; this net asserts behaviour through `plan()` rather than constructing rules,
// but pinning the surface here keeps the import meaningful and the contract visible.)
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn public_routing_rule_surface_is_constructible() {
    // A predicate over the K1 subject gate — the shared gate every routing rule AND's in.
    let gate = Predicate {
        knob: Knob::FgBgContrast,
        op: CmpOp::Ge,
        lo: 0.25,
        hi: 0.0,
    };
    let table = SelectTable {
        default: "home_only".to_string(),
        rules: vec![SelectRule {
            when: vec![gate.clone()],
            pick: "rounded_binary_excursion".to_string(),
        }],
    };
    // The gate fires at/above 0.25 and not below — the byte-stable subject gate semantics.
    let firing = ImageUnderstanding {
        fg_bg_contrast: 0.25,
        ..ImageUnderstanding::neutral()
    };
    let below = ImageUnderstanding {
        fg_bg_contrast: 0.24,
        ..ImageUnderstanding::neutral()
    };
    assert!(
        gate.holds(&firing),
        "the fg_bg_contrast ≥ 0.25 subject gate must hold at exactly 0.25"
    );
    assert!(
        !gate.holds(&below),
        "the subject gate must NOT hold below 0.25"
    );
    assert_eq!(
        table.select(&firing),
        "rounded_binary_excursion",
        "a firing image selects the gated pick"
    );
    assert_eq!(
        table.select(&below),
        "home_only",
        "a below-gate image falls to the default"
    );
}

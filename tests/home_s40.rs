//! tests/home_s40.rs — the S40 / Slice-2 per-image HOME net (Finding #1).
//!
//! Slice-2 makes the planner's home root IMAGE-DERIVED: `composition.rs:1435` now calls
//! `resolve_home_root_midi(self.plan_mappings.home_root.as_ref(), u.dominant_hue)` instead of the
//! old `let home_root_midi = 60`. The dominant hue selects a chromatic pitch class via the shipped
//! `composition.home_root.hue_to_pc` range map and is seated into the safe register band [57,68];
//! an ABSENT block (or any unmatched/bad cut) falls back to 60 byte-for-byte. This net proves the
//! five testable invariants pinned by `docs/design-s40-slice2-workorder.md` §4, each through the
//! PUBLIC `CompositionPlanner::plan(...)` reading `plan.key_tempo.home_root_midi` and the per-section
//! `key_offset_semitones` — never through the module-private `resolve_home_root_midi`/`seat_pc_in_band`
//! (those have their own unit net in `src/composition.rs::home_root_tests`). The properties:
//!
//!   INV-1 (GR-1, home invariance within a piece) — within ONE plan() the home is a single constant
//!         center; every Statement/Return (home-role) section resolves to key_offset_semitones == 0,
//!         so its section root == the per-image home.
//!   INV-2 (GR-2, resolved home ∈ [57,68]) — with the shipped home block, a full-circle dominant_hue
//!         sweep keeps home_root_midi inside the safe register band on EVERY hue.
//!   INV-3 (per-image home VARIES across differing-hue images) — two images whose hues fall in
//!         different 30° buckets yield different homes; a full sweep yields many distinct homes.
//!   INV-4 (mappings-absent reproduces home == 60 byte-for-byte) — a PlanMappings with
//!         `home_root: None` yields home_root_midi == 60 for ALL hues. THE freeze-keystone invariant
//!         (it is what keeps engine_equivalence / keyplan_k2a green where no block is carried).
//!   INV-5 (no downstream home==60 assumption / re-root identity) — the section re-root law holds
//!         RELATIVE to the per-image home (home-role sections offset 0; excursion sections carry
//!         their declared offset), and that law is identical across two DIFFERENT per-image homes —
//!         proving no consumer hardcodes the literal 60.
//!
//! HEADLESS, like keyplan_k2a.rs / composition_s15.rs: NO image type, NO OpenCV, NO audio hardware.
//!
//! RNG-BOUNDARY DISCIPLINE (same as keyplan_k2a.rs): `home_root_midi` and the per-section
//! `key_offset_semitones` are set BEFORE any `thread_rng` call inside `plan()`, so every assertion
//! here is RNG-independent. We never assert chord/Roman-numeral content (that path is the
//! non-deterministic `pick_progression`), only the home center and the relative section offsets.

use audiohax::composition::{
    CompositionPlanner, ImageUnderstanding, KeyScheme, KeySchemeSection, PlanMappings,
    ResolutionPolicy, SelectTable, ThematicRole,
};
use audiohax::mapping_loader::{load_mappings, MappingTable};

const MAPPINGS_PATH: &str = "assets/mappings.json";

/// The shipped safe register band the resolved per-image home must land within (work-order §2.3).
const BAND_LO: u8 = 57;
const BAND_HI: u8 = 68;

/// The legacy/fallback home root (C4) the absent/unmatched path reproduces byte-for-byte (INV-4).
const LEGACY_HOME: u8 = 60;

/// The shipped `assets/mappings.json` mapping table (carries the real `composition.home_root` block).
fn mappings() -> MappingTable {
    load_mappings(MAPPINGS_PATH).expect("mappings.json loads")
}

/// The shipped composition `PlanMappings` — clone of the real block, so its `home_root` is `Some`
/// (the shipped 12-bucket uniform hue→pc map + band [57,68]).
fn base_plan_mappings(m: &MappingTable) -> PlanMappings {
    m.composition
        .clone()
        .expect("composition block present in mappings.json")
        .into()
}

/// A `SelectTable` that ALWAYS resolves to `id` (empty rule set → the default fires). Mirrors
/// keyplan_k2a.rs's `always` helper, used to pin the form / key-scheme axis deterministically.
fn always(id: &str) -> SelectTable {
    SelectTable {
        default: id.to_string(),
        rules: Vec::new(),
    }
}

/// A neutral understanding pinned to a chosen `dominant_hue` — the ONLY axis the home derivation
/// reads. Everything else stays at its neutral default so the home is isolated from form/affect.
fn image_with_hue(dominant_hue: f32) -> ImageUnderstanding {
    ImageUnderstanding {
        dominant_hue,
        // Hold secondary/subject hue == dominant so no unrelated hue-distance machinery is engaged.
        secondary_hue: dominant_hue,
        subject_hue: dominant_hue,
        ..ImageUnderstanding::neutral()
    }
}

/// The resolved per-image home for a hue under the SHIPPED (present) home block.
fn home_for_hue_shipped(planner: &CompositionPlanner, m: &MappingTable, hue: f32) -> u8 {
    planner
        .plan(&image_with_hue(hue), m)
        .key_tempo
        .home_root_midi
}

// ═════════════════════════════════════════════════════════════════════════════
// INV-2 (GR-2) — the resolved home is ALWAYS inside the safe register band [57,68]
//   with the shipped home block, for any dominant_hue on the full circle.
// ═════════════════════════════════════════════════════════════════════════════

/// Sweep dominant_hue across the WHOLE circle every 5° and assert the planner's resolved
/// home_root_midi ∈ [57,68] every time. The shipped 12-bucket map covers the circle, so each hue
/// resolves to a seated pitch class; even were a hue to miss a cut, the fallback (60) is itself
/// in-band — so the band invariant holds unconditionally with this block.
#[test]
fn test_inv2_home_in_band_full_hue_sweep() {
    let m = mappings();
    let planner = CompositionPlanner::new(base_plan_mappings(&m));
    let mut hue = 0.0f32;
    while hue < 360.0 {
        let home = home_for_hue_shipped(&planner, &m, hue);
        assert!(
            (BAND_LO..=BAND_HI).contains(&home),
            "INV-2 (GR-2): hue {hue} resolved home {home} is OUTSIDE the safe band [{BAND_LO},{BAND_HI}]"
        );
        hue += 5.0;
    }
}

/// INV-2 robustness: a FRACTIONAL hue sweep (values that may fall between integer range-cuts and so
/// hit the 60 fallback) must STILL stay in-band — 60 ∈ [57,68], so the band guarantee never breaks
/// even on a cut miss. Guards against a future band edit that would put the fallback out of band.
#[test]
fn test_inv2_home_in_band_fractional_hue_sweep() {
    let m = mappings();
    let planner = CompositionPlanner::new(base_plan_mappings(&m));
    let mut hue = 0.5f32;
    while hue < 360.0 {
        let home = home_for_hue_shipped(&planner, &m, hue);
        assert!(
            (BAND_LO..=BAND_HI).contains(&home),
            "INV-2: fractional hue {hue} resolved home {home} OUTSIDE [{BAND_LO},{BAND_HI}] \
             (the bad-cut fallback must also be in-band)"
        );
        hue += 1.0;
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// INV-3 — the per-image home VARIES across images whose hues differ enough to land
//   in different pitch-class buckets.
// ═════════════════════════════════════════════════════════════════════════════

/// Two images whose dominant hues fall in DIFFERENT 30° buckets (10° → bucket 0-29 → pc 0 → C,
/// 200° → bucket 180-209 → pc 6 → F#) resolve to DIFFERENT per-image homes. (Same-bucket hues MAY
/// tie — that is correct — so the test deliberately picks cross-bucket hues.)
#[test]
fn test_inv3_cross_bucket_hues_yield_distinct_homes() {
    let m = mappings();
    let planner = CompositionPlanner::new(base_plan_mappings(&m));
    let home_a = home_for_hue_shipped(&planner, &m, 10.0); // bucket 0-29 → pc 0 (C)
    let home_b = home_for_hue_shipped(&planner, &m, 200.0); // bucket 180-209 → pc 6 (F#)
    assert_ne!(
        home_a, home_b,
        "INV-3: cross-bucket hues 10° and 200° must yield DIFFERENT per-image homes, both got {home_a}"
    );
    // Both must still be in-band (INV-2 holds pointwise here too).
    assert!((BAND_LO..=BAND_HI).contains(&home_a) && (BAND_LO..=BAND_HI).contains(&home_b));
}

/// Same-bucket hues (5° and 25°, both in bucket 0-29 → pc 0) tie — confirming the variation is
/// driven by the pitch-class BUCKET, not by raw hue noise. (This is the correct counterpart to the
/// cross-bucket divergence above, not a separate invariant.)
#[test]
fn test_inv3_same_bucket_hues_tie() {
    let m = mappings();
    let planner = CompositionPlanner::new(base_plan_mappings(&m));
    let home_a = home_for_hue_shipped(&planner, &m, 5.0);
    let home_b = home_for_hue_shipped(&planner, &m, 25.0);
    assert_eq!(
        home_a, home_b,
        "INV-3 counterpart: two hues in the SAME 30° bucket (5°, 25°) must seat to the SAME home"
    );
}

/// Stronger INV-3: a full-circle hue sweep observes MANY distinct home values (the uniform
/// 12-bucket map should produce a rich set, not a constant). We require ≥ 6 distinct homes — well
/// above the floor, but tolerant of band-folding (a 12-semitone band yields up to 12 distinct
/// seated pitch classes, but if a future band narrowed, the ≥6 floor still catches a collapse).
#[test]
fn test_inv3_hue_sweep_yields_many_distinct_homes() {
    let m = mappings();
    let planner = CompositionPlanner::new(base_plan_mappings(&m));
    let mut homes = std::collections::BTreeSet::new();
    let mut hue = 0.0f32;
    while hue < 360.0 {
        homes.insert(home_for_hue_shipped(&planner, &m, hue));
        hue += 5.0;
    }
    assert!(
        homes.len() >= 6,
        "INV-3 (varies): a full hue sweep produced only {} distinct home value(s) {:?} — \
         the per-image home is not varying (≥6 expected from the 12-bucket map)",
        homes.len(),
        homes
    );
    // Every observed home is in-band — the sweep never escapes the safe register.
    for &h in &homes {
        assert!(
            (BAND_LO..=BAND_HI).contains(&h),
            "INV-3/INV-2: swept home {h} outside [{BAND_LO},{BAND_HI}]"
        );
    }
    // The shipped uniform 12-bucket map maps onto a 12-semitone band: all 12 pitch classes should
    // be reachable. Assert the full chromatic coverage as a tight upper-confidence check (this is
    // the shipped-data expectation; if Music Theory ever re-cuts the map this may relax to ≥6).
    assert_eq!(
        homes.len(),
        12,
        "the shipped uniform 12-bucket map over a 12-semitone band should reach all 12 distinct \
         homes; got {} ({:?})",
        homes.len(),
        homes
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// INV-1 (GR-1) — within ONE piece the home is a single constant center, and every
//   home-role (Statement/Return) section resolves to key_offset_semitones == 0
//   (so its section root == the per-image home).
// ═════════════════════════════════════════════════════════════════════════════

/// A 4-section ABAC scheme (home / region_related:b / home / region_related:c) aligned to the
/// shipped `abac` form (Statement/Contrast/Return/Coda). Mirrors keyplan_k2a.rs's `abac_scheme`.
fn abac_scheme(id: &str, resolution: ResolutionPolicy) -> KeyScheme {
    KeyScheme {
        id: id.to_string(),
        sections: vec![
            KeySchemeSection {
                label: "A".into(),
                offset_rule: "home".into(),
            },
            KeySchemeSection {
                label: "B".into(),
                offset_rule: "region_related:b".into(),
            },
            KeySchemeSection {
                label: "A".into(),
                offset_rule: "home".into(),
            },
            KeySchemeSection {
                label: "C".into(),
                offset_rule: "region_related:c".into(),
            },
        ],
        resolution,
        pivot: false,
    }
}

/// Build a planner pinned to the `abac` form + a given scheme, keeping the SHIPPED home block (so
/// the per-image home is live). Mirrors keyplan_k2a.rs's `planner_with`.
fn planner_with(scheme: KeyScheme, m: &MappingTable) -> CompositionPlanner {
    let mut pm = base_plan_mappings(m);
    pm.form = always("abac");
    pm.key_scheme = always(&scheme.id);
    pm.key_scheme_catalogue.push(scheme);
    CompositionPlanner::new(pm)
}

/// A region image whose foreground/background carry distinct affect so the B/C excursions actually
/// travel (so the home-role offset-0 claim is tested against a piece that DOES modulate elsewhere).
/// `dominant_hue` still drives the per-image home.
fn regions_with_hue(dominant_hue: f32) -> ImageUnderstanding {
    ImageUnderstanding {
        dominant_hue,
        secondary_hue: dominant_hue,
        subject_hue: dominant_hue,
        // Fire the abac excursions: distinct fg/bg affect (rank-0 bright near, rank-1 dark near).
        fg_bg_contrast: 0.30,
        foreground_energy: 0.9,
        foreground_brightness: 0.9,
        foreground_hue: dominant_hue,
        background_energy: 0.3,
        background_brightness: 0.1,
        background_hue: dominant_hue,
        ..ImageUnderstanding::neutral()
    }
}

/// Within a single plan(): the home is ONE constant center (`key_tempo.home_root_midi` is a single
/// field — structural), and EVERY Statement/Return (home-role) section has key_offset_semitones == 0,
/// i.e. its section root == home. The excursion (Contrast/Coda) sections are free to travel.
#[test]
fn test_inv1_home_constant_and_home_roles_offset_zero() {
    let m = mappings();
    let planner = planner_with(abac_scheme("abac_inv1", ResolutionPolicy::Open), &m);
    let plan = planner.plan(&regions_with_hue(200.0), &m); // hue 200 → a non-60 per-image home

    let home = plan.key_tempo.home_root_midi;
    assert!(
        (BAND_LO..=BAND_HI).contains(&home),
        "INV-1 premise: the per-image home {home} should be in-band (hue 200)"
    );
    // It must NOT be the legacy 60 here — otherwise we are not actually exercising a per-image home.
    assert_ne!(
        home, LEGACY_HOME,
        "INV-1 premise: hue 200 must resolve to a per-image (non-legacy-60) home to prove the \
         home is image-derived, got {home}"
    );

    // Every home-role section sits exactly on the home (offset 0); a single constant center.
    let mut home_roles = 0usize;
    for (i, s) in plan.sections.iter().enumerate() {
        if matches!(
            s.thematic_role,
            ThematicRole::Statement | ThematicRole::Return
        ) {
            home_roles += 1;
            assert_eq!(
                s.key_offset_semitones, 0,
                "INV-1: section {i} ({:?}) is a HOME role and must resolve to offset 0 (root == \
                 the per-image home {home}); got offset {}",
                s.thematic_role, s.key_offset_semitones
            );
        }
    }
    assert!(
        home_roles >= 2,
        "INV-1: an ABAC form has at least a Statement and a Return home role; saw {home_roles}"
    );

    // The home center is a single value for the whole piece — every section shares the SAME
    // `key_tempo.home_root_midi` (it is one field on the plan, asserted structurally by re-reading).
    assert_eq!(
        plan.key_tempo.home_root_midi, home,
        "INV-1: the piece carries exactly ONE home center"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// INV-4 — a PlanMappings with `home_root: None` reproduces home == 60 byte-for-byte
//   for ALL hues. THE freeze-keystone invariant.
// ═════════════════════════════════════════════════════════════════════════════

/// Clone the shipped composition mappings and NULL OUT the home block. A full-circle hue sweep must
/// then yield home_root_midi == 60 for EVERY hue — the defensive fallback that keeps every fixture
/// not carrying a home block (engine_equivalence, keyplan_k2a, …) on today's exact behavior. This
/// is the invariant the whole freeze-safety argument rests on (work-order §7).
#[test]
fn test_inv4_absent_home_block_is_legacy_60_all_hues() {
    let m = mappings();
    let mut pm = base_plan_mappings(&m);
    pm.home_root = None; // the back-compat floor: no per-image home derivation.
    let planner = CompositionPlanner::new(pm);

    let mut hue = 0.0f32;
    while hue < 360.0 {
        let home = planner
            .plan(&image_with_hue(hue), &m)
            .key_tempo
            .home_root_midi;
        assert_eq!(
            home, LEGACY_HOME,
            "INV-4 (FREEZE KEYSTONE): with home_root=None, hue {hue} must resolve to the legacy \
             {LEGACY_HOME} byte-for-byte; got {home}"
        );
        hue += 5.0;
    }
}

/// INV-4 corollary: the PRESENCE of the block is what changes behavior — at the SAME hue, the
/// shipped (present) block produces a home that differs from the absent-block legacy 60 (proving
/// the absence path is a genuine fallback, not an accidental no-op that always returns 60). We pick
/// a hue (200°) whose shipped pitch class (pc 6 → F#) is NOT C, so the present home != 60.
#[test]
fn test_inv4_present_vs_absent_diverge_at_non_c_hue() {
    let m = mappings();

    let present = CompositionPlanner::new(base_plan_mappings(&m));
    let mut absent_pm = base_plan_mappings(&m);
    absent_pm.home_root = None;
    let absent = CompositionPlanner::new(absent_pm);

    let hue = 200.0f32; // bucket 180-209 → pc 6 (F#) → seated in [57,68] → NOT 60.
    let home_present = home_for_hue_shipped(&present, &m, hue);
    let home_absent = absent
        .plan(&image_with_hue(hue), &m)
        .key_tempo
        .home_root_midi;

    assert_eq!(
        home_absent, LEGACY_HOME,
        "INV-4 corollary: absent block must be {LEGACY_HOME}; got {home_absent}"
    );
    assert_ne!(
        home_present, home_absent,
        "INV-4 corollary: the PRESENT block must actually change the home at a non-C hue \
         (present {home_present} vs absent {home_absent}) — the fallback is a real branch, \
         not an always-60 no-op"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// INV-5 — the section re-root law holds RELATIVE to the per-image home, and is
//   identical across two DIFFERENT per-image homes (no consumer hardcodes 60).
// ═════════════════════════════════════════════════════════════════════════════

/// The re-root identity is expressed relative to the per-image home, not the literal 60:
///   * home-role (Statement/Return) sections carry offset 0 → their root == home;
///   * excursion (Contrast/Coda) sections carry their declared non-zero menu offset → their root ==
///     home + offset;
///   * and crucially this OFFSET STRUCTURE is IDENTICAL across two images with DIFFERENT per-image
///     homes (same affect, different dominant_hue) — so the section roots all shift by exactly the
///     home delta while the relative offsets stay put. THIS is the "no downstream home==60
///     assumption" guarantee: change the home base and the whole piece transposes uniformly.
#[test]
fn test_inv5_reroot_law_relative_to_per_image_home() {
    let m = mappings();
    let planner = planner_with(abac_scheme("abac_inv5", ResolutionPolicy::Open), &m);

    // Two images: same firing region affect, DIFFERENT dominant_hue → two different per-image homes.
    let plan_a = planner.plan(&regions_with_hue(10.0), &m); // pc 0 (C)
    let plan_b = planner.plan(&regions_with_hue(200.0), &m); // pc 6 (F#)

    let home_a = plan_a.key_tempo.home_root_midi;
    let home_b = plan_b.key_tempo.home_root_midi;
    assert_ne!(
        home_a, home_b,
        "INV-5 premise: the two hues must give DIFFERENT per-image homes (a {home_a}, b {home_b})"
    );

    // (1) Collect the per-section offsets for each plan and assert they are byte-identical across
    //     the two different-home pieces — the home moved, the relative offset structure did NOT.
    let offsets = |plan: &audiohax::composition::CompositionPlan| -> Vec<i8> {
        plan.sections
            .iter()
            .map(|s| s.key_offset_semitones)
            .collect()
    };
    let off_a = offsets(&plan_a);
    let off_b = offsets(&plan_b);
    assert_eq!(
        off_a, off_b,
        "INV-5: the per-section offset structure must be IDENTICAL across two different per-image \
         homes (the piece transposes uniformly); a={off_a:?} b={off_b:?}"
    );

    // (2) The re-root law per section, relative to the per-image home: section_root == home + offset.
    //     Section root isn't exposed as its own field, so we assert the equivalent law the realizer
    //     uses (composition.rs:1553): root = (home + offset).clamp(0,127). For every section, the
    //     reconstructed root from THIS plan's own home must equal the reconstruction in the OTHER
    //     plan shifted by the home delta — i.e. (home_a+off) - (home_b+off) == home_a - home_b for
    //     all sections, which is exactly the "uniform transposition, offsets relative" guarantee.
    let delta = home_a as i16 - home_b as i16;
    for (i, (&oa, &ob)) in off_a.iter().zip(off_b.iter()).enumerate() {
        // offsets already proven equal, but re-root each explicitly to prove the law literally.
        let root_a = (home_a as i16 + oa as i16).clamp(0, 127);
        let root_b = (home_b as i16 + ob as i16).clamp(0, 127);
        assert_eq!(
            root_a - root_b,
            delta,
            "INV-5: section {i} root must move by exactly the home delta {delta} (root_a {root_a} \
             - root_b {root_b}); the offset is RELATIVE to the per-image home, not anchored to 60"
        );
    }

    // (3) Home-role sections sit on the home (offset 0); excursion sections carry an in-menu offset.
    const MENU: [i8; 4] = [7, 5, 3, -3];
    for (i, s) in plan_a.sections.iter().enumerate() {
        match s.thematic_role {
            ThematicRole::Statement | ThematicRole::Return => assert_eq!(
                s.key_offset_semitones, 0,
                "INV-5: home-role section {i} ({:?}) must have offset 0 (root == home {home_a})",
                s.thematic_role
            ),
            ThematicRole::Contrast | ThematicRole::Coda => assert!(
                s.key_offset_semitones == 0 || MENU.contains(&s.key_offset_semitones),
                "INV-5: excursion section {i} ({:?}) offset {} must be home(0) or in the v1 menu \
                 {MENU:?} — a relative excursion off the per-image home",
                s.thematic_role,
                s.key_offset_semitones
            ),
            _ => {}
        }
    }
}

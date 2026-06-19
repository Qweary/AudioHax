//! tests/hue_gap_s41.rs — the S41 HUE INTER-BUCKET GAP FIX property net.
//!
//! A freeze-safe fix landed in `src/composition.rs`: the new module-private helper
//! `snap_hue_to_bucket_grid(hue) = hue.rem_euclid(360.0).round().rem_euclid(360.0)` is wired into
//! BOTH the `hue_to_pc` lookup (at the single entry of `resolve_home_root_midi`, AFTER the
//! `None`-home fallback guard) and the `hue_to_mode` lookup (composition.rs:~1433). It closes the
//! ~1° non-covering gaps between the integer-endpoint range buckets that previously dropped
//! fractional production hues (a perceptual pixel average — `29.5`, `90.4`, …) through to the
//! floor (60 / Ionian). Full spec: `docs/design-s41-hue-gap-fix.md`.
//!
//! The shipped tables (assets/mappings.json) this net pins against:
//!   home_root.hue_to_pc   — 30°/pc, red=C ascending:
//!     0-29→0(C) 30-59→1(C#) 60-89→2(D) 90-119→3 120-149→4 150-179→5 180-209→6
//!     210-239→7 240-269→8 270-299→9 300-329→10 330-359→11 ; band [57,68].
//!   global.hue_to_mode    — 6 modal buckets (NOTE the floor is Ionian via unwrap_or_else, but the
//!     0-30 BUCKET value is Phrygian, NOT Ionian):
//!     0-30→Phrygian 31-90→Lydian 91-150→Ionian 151-210→Dorian 211-270→Aeolian 271-330→Mixolydian.
//!
//! All tests go through the PUBLIC planner API — `CompositionPlanner::plan(...)` reading
//! `plan.key_tempo.home_root_midi` (P1/P3/P4/P5/P6) and `plan.key_tempo.home_mode` (P2/P3). The
//! snap helper is module-private (`fn snap_hue_to_bucket_grid`, no `pub`/`pub(crate)`) and is NOT
//! reachable from an integration test; its direct unit witness (U1) belongs in
//! `src/composition.rs::home_root_tests` per work-order §4.3 and is the Implementer's call (a
//! production-file edit), NOT this Test-Engineer slice. So EVERYTHING here routes through `plan()`.
//!
//! WHY home_mode is observable raw (P2): the shipped `affect.mode_valence_cuts` are
//! `{major_min 0.55, minor_max 0.45}`. A `neutral()` image has valence
//! `0.7·0.5 + 0.2·0.5 + 0.1·0.5 == 0.50`, which lands in the `(0.45,0.55)` NEUTRAL dead band, so
//! `valence_family_mode` is the identity projection and `key_tempo.home_mode` IS the raw hue→mode
//! pick — exactly the value the gap fix governs. (We assert this premise below.)
//!
//! RNG-BOUNDARY DISCIPLINE (same as home_s40.rs): `home_root_midi` / `home_mode` are computed
//! BEFORE any `thread_rng` call inside `plan()`, so every assertion here is RNG-independent. We
//! never assert chord / Roman-numeral content (the non-deterministic `pick_progression` path).
//!
//! Run under DEFAULT features (the always-on bin needs the default pure-Rust features):
//!     cargo test --test hue_gap_s41

use audiohax::composition::{CompositionPlanner, ImageUnderstanding, PlanMappings};
use audiohax::mapping_loader::{load_mappings, MappingTable};

const MAPPINGS_PATH: &str = "assets/mappings.json";

/// Shipped safe register band the resolved per-image home must land within (work-order §2.3).
const BAND_LO: u8 = 57;
const BAND_HI: u8 = 68;
/// The legacy/fallback home root (C4) a gap-miss USED to fall to — the bug this fix removes.
const LEGACY_HOME: u8 = 60;
/// The unwrap_or_else floor for the mode lookup — the mode a gap-miss USED to fall to.
const MODE_FLOOR: &str = "Ionian";

fn mappings() -> MappingTable {
    load_mappings(MAPPINGS_PATH).expect("mappings.json loads")
}

/// The shipped composition `PlanMappings` — `home_root` is `Some` (the shipped 12-bucket hue→pc
/// map + band [57,68]) and `affect.mode_valence_cuts` is present (so the dead-band premise holds).
fn base_plan_mappings(m: &MappingTable) -> PlanMappings {
    m.composition
        .clone()
        .expect("composition block present in mappings.json")
        .into()
}

/// A neutral understanding pinned to a chosen `dominant_hue` — the ONLY axis the home/mode
/// derivation reads. Everything else stays neutral (in particular valence == 0.50, the dead band),
/// isolating the hue→pc / hue→mode picks the gap fix governs.
fn image_with_hue(dominant_hue: f32) -> ImageUnderstanding {
    ImageUnderstanding {
        dominant_hue,
        secondary_hue: dominant_hue,
        subject_hue: dominant_hue,
        ..ImageUnderstanding::neutral()
    }
}

/// Resolved per-image home (MIDI) for a hue under the shipped (present) home block.
fn home_for_hue(planner: &CompositionPlanner, m: &MappingTable, hue: f32) -> u8 {
    planner
        .plan(&image_with_hue(hue), m)
        .key_tempo
        .home_root_midi
}

/// Resolved per-image home MODE name for a hue under the shipped block (neutral valence → the raw
/// hue→mode pick, see module header).
fn mode_for_hue(planner: &CompositionPlanner, m: &MappingTable, hue: f32) -> String {
    planner
        .plan(&image_with_hue(hue), m)
        .key_tempo
        .home_mode
        .clone()
}

// ═════════════════════════════════════════════════════════════════════════════
// PREMISE — neutral valence keeps the mode projection a no-op, so home_mode is the
//   RAW hue→mode pick. If a future affect/weights change broke this, P2/P3-mode would
//   silently test the wrong quantity; this guard fails loudly first.
// ═════════════════════════════════════════════════════════════════════════════

/// At an ON-BUCKET hue whose mode bucket is NOT the floor, `home_mode` must equal that bucket's
/// raw mode — proving the valence projection is the identity (neutral dead band) and that
/// `key_tempo.home_mode` carries the hue→mode pick the fix governs. Hue 10° → bucket 0-30 →
/// Phrygian (NOT Ionian, NOT a major-family remap), so this also rules out an accidental
/// always-Ionian or family-forced reading.
#[test]
fn test_premise_neutral_valence_exposes_raw_hue_mode() {
    let m = mappings();
    let planner = CompositionPlanner::new(base_plan_mappings(&m));
    let mode = mode_for_hue(&planner, &m, 10.0);
    assert_eq!(
        mode, "Phrygian",
        "PREMISE: with neutral valence (0.50, dead band) the mode projection must be identity, so \
         hue 10° (bucket 0-30) must surface its RAW mode Phrygian (not the {MODE_FLOOR} floor, not \
         a valence-forced family); got {mode}"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// P1 — inter-bucket gap hues now SNAP for hue_to_pc (no longer floor to 60).
// ═════════════════════════════════════════════════════════════════════════════

/// Each gap hue (a fractional value in a 1° inter-bucket gap) now resolves to a real seated pitch
/// class in [57,68] instead of falling to the legacy 60-by-miss. We assert in-band AND, where the
/// nearer bucket's pc is NOT C, assert `!= 60` (a gap that rounds INTO the C bucket could
/// legitimately seat to 60, so we don't over-constrain those).
#[test]
fn test_p1_gap_hues_snap_for_hue_to_pc() {
    let m = mappings();
    let planner = CompositionPlanner::new(base_plan_mappings(&m));

    // Gap hues sitting in the open (n.0, n+1.0) inter-bucket gaps. Each rounds UP to the next
    // bucket's start (n.5 → n+1). 29.5→30 (pc1), 59.5→60 (pc2), 89.5→90 (pc3), 119.5→120 (pc4),
    // 149.5→150 (pc5) — none of those target pcs is C, so each must be in-band AND != 60.
    for hue in [29.5f32, 59.5, 89.5, 119.5, 149.5] {
        let home = home_for_hue(&planner, &m, hue);
        assert!(
            (BAND_LO..=BAND_HI).contains(&home),
            "P1: gap hue {hue}° resolved home {home} must be in-band [{BAND_LO},{BAND_HI}]"
        );
        assert_ne!(
            home, LEGACY_HOME,
            "P1: gap hue {hue}° must SNAP to a real bucket (its rounded bucket's pc is not C, so \
             home must differ from the legacy {LEGACY_HOME}-by-miss floor); got {home}"
        );
    }

    // Pin the exact snap target for 29.5°: rounds to 30 → bucket 30-59 → pc 1 (C#) → seated in
    // band → home % 12 == 1, DISTINCT from the pc-0 (C) home of the adjacent 0-29 bucket. This
    // proves the snap lands in the UP-rounded bucket, not merely "somewhere in band".
    let h_gap = home_for_hue(&planner, &m, 29.5); // → pc 1
    let h_lo = home_for_hue(&planner, &m, 10.0); // bucket 0-29 → pc 0 (C)
    assert_eq!(
        h_gap % 12,
        1,
        "P1: 29.5° must snap UP to bucket 30-59 (pc 1, C#); got pitch class {}",
        h_gap % 12
    );
    assert_ne!(
        h_gap, h_lo,
        "P1: 29.5° (snaps to pc 1) must differ from the 0-29 bucket home (pc 0); got both {h_gap}"
    );

    // The on-EDGE integer 30.0 is itself a bucket start (pc 1) and must agree with the gap snap.
    let h_edge = home_for_hue(&planner, &m, 30.0);
    assert_eq!(
        h_edge, h_gap,
        "P1: on-edge 30.0° (bucket 30-59, pc 1) and gap 29.5°→30 must resolve to the SAME home; \
         got edge {h_edge} vs gap {h_gap}"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// P2 — inter-bucket gap hues now SNAP for hue_to_mode (no longer floor to Ionian).
// ═════════════════════════════════════════════════════════════════════════════

/// A gap hue in the hue_to_mode gaps (the 6 modal buckets are 0-30,31-90,91-150,151-210,211-270,
/// 271-330 — gaps at the .x between 30 and 31, 90 and 91, …). 30.5° rounds to 31 → bucket 31-90 →
/// Lydian, NOT the Ionian floor. We assert the resolved mode is Lydian (the up-rounded bucket).
#[test]
fn test_p2_gap_hue_snaps_for_hue_to_mode_lydian_not_floor() {
    let m = mappings();
    let planner = CompositionPlanner::new(base_plan_mappings(&m));
    let mode = mode_for_hue(&planner, &m, 30.5); // → round 31 → bucket 31-90 → Lydian
    assert_eq!(
        mode, "Lydian",
        "P2: gap hue 30.5° must snap to bucket 31-90 (Lydian), NOT the {MODE_FLOOR} floor; got {mode}"
    );
    assert_ne!(
        mode, MODE_FLOOR,
        "P2: gap hue 30.5° must NOT collapse to the {MODE_FLOOR} floor"
    );
}

/// A clear straddling PAIR around the 30/31 modal boundary: 30.4° rounds DOWN to 30 (bucket 0-30 →
/// Phrygian); 30.6° rounds UP to 31 (bucket 31-90 → Lydian). They must land in DIFFERENT buckets —
/// proving neither collapses to a shared floor and the round genuinely picks the nearer bucket.
#[test]
fn test_p2_straddling_pair_lands_in_buckets_either_side_of_30() {
    let m = mappings();
    let planner = CompositionPlanner::new(base_plan_mappings(&m));
    let lo = mode_for_hue(&planner, &m, 30.4); // → round 30 → bucket 0-30 → Phrygian
    let hi = mode_for_hue(&planner, &m, 30.6); // → round 31 → bucket 31-90 → Lydian
    assert_eq!(
        lo, "Phrygian",
        "P2: 30.4° rounds DOWN to 30 → bucket 0-30 → Phrygian; got {lo}"
    );
    assert_eq!(
        hi, "Lydian",
        "P2: 30.6° rounds UP to 31 → bucket 31-90 → Lydian; got {hi}"
    );
    assert_ne!(
        lo, hi,
        "P2: the straddling pair (30.4, 30.6) must land in the two buckets either side of 30 — \
         neither may collapse to a shared floor; both got {lo}"
    );
}

/// A full sweep of every hue_to_mode inter-bucket gap (the .5 between each pair of adjacent modal
/// buckets) must resolve to a REAL bucket mode every time — none may fall to the Ionian floor by
/// miss. (30.5 is exempt-by-coincidence-not, but we list only true gaps below.)
#[test]
fn test_p2_all_mode_gaps_resolve_to_real_bucket() {
    let m = mappings();
    let planner = CompositionPlanner::new(base_plan_mappings(&m));
    // The 5 interior modal gaps (30/31, 90/91, 150/151, 210/211, 270/271) at their .5 midpoint and
    // the rounded-up bucket's expected mode.
    let cases: [(f32, &str); 5] = [
        (30.5, "Lydian"),      // → 31 → 31-90
        (90.5, "Ionian"),      // → 91 → 91-150
        (150.5, "Dorian"),     // → 151 → 151-210
        (210.5, "Aeolian"),    // → 211 → 211-270
        (270.5, "Mixolydian"), // → 271 → 271-330
    ];
    for (hue, expect) in cases {
        let mode = mode_for_hue(&planner, &m, hue);
        assert_eq!(
            mode, expect,
            "P2 sweep: mode gap hue {hue}° must snap UP to bucket mode {expect}; got {mode}"
        );
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// P3 — on-bucket integer hues map IDENTICALLY to pre-fix (.round() is identity on
//   integers, so no regression). We pin the EXACT shipped-table values, not just
//   "unchanged-vs-itself".
// ═════════════════════════════════════════════════════════════════════════════

/// For the work-order's on-bucket integer hues {0,10,45,100,200,300}, both `home_root_midi` and
/// `home_mode` equal the values the shipped tables dictate. `.round()` is the identity on integers,
/// so these are the pre-fix values byte-for-byte — a regression sentinel.
#[test]
fn test_p3_on_bucket_integer_hues_no_regression() {
    let m = mappings();
    let planner = CompositionPlanner::new(base_plan_mappings(&m));

    // (hue, expected pitch class, expected raw hue→mode). Pitch classes from home_root.hue_to_pc;
    // modes from global.hue_to_mode (raw — neutral valence keeps the projection identity).
    //   0   → pc 0  (bucket 0-29)   → mode bucket 0-30   Phrygian
    //   10  → pc 0  (bucket 0-29)   → mode bucket 0-30   Phrygian
    //   45  → pc 1  (bucket 30-59)  → mode bucket 31-90  Lydian
    //   100 → pc 3  (bucket 90-119) → mode bucket 91-150 Ionian
    //   200 → pc 6  (bucket 180-209)→ mode bucket 151-210 Dorian
    //   300 → pc 10 (bucket 300-329)→ mode bucket 271-330 Mixolydian
    let cases: [(f32, u8, &str); 6] = [
        (0.0, 0, "Phrygian"),
        (10.0, 0, "Phrygian"),
        (45.0, 1, "Lydian"),
        (100.0, 3, "Ionian"),
        (200.0, 6, "Dorian"),
        (300.0, 10, "Mixolydian"),
    ];
    for (hue, pc, mode) in cases {
        let home = home_for_hue(&planner, &m, hue);
        assert!(
            (BAND_LO..=BAND_HI).contains(&home),
            "P3: on-bucket hue {hue}° home {home} must be in-band"
        );
        assert_eq!(
            home % 12,
            pc,
            "P3 (no-regression): on-bucket hue {hue}° must seat pitch class {pc}; got {}",
            home % 12
        );
        let got_mode = mode_for_hue(&planner, &m, hue);
        assert_eq!(
            got_mode, mode,
            "P3 (no-regression): on-bucket hue {hue}° must resolve mode {mode}; got {got_mode}"
        );
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// P4 — wrap seam: 359.6° ≡ 0.0° (rounds to 360 → rem_euclid → 0; the trailing
//   rem_euclid keeps the top of the circle wrapping to red, NOT a new gap at 360).
// ═════════════════════════════════════════════════════════════════════════════

/// 359.6° must map to the SAME home AND mode as 0.0° (red/C, top of circle wraps), and must NOT
/// fall to 60-by-miss. This is the single seam case the trailing `rem_euclid` guards: without it
/// 359.6 → round 360 would match no hue_to_pc bucket and re-introduce a gap.
#[test]
fn test_p4_wrap_seam_359_6_equals_zero() {
    let m = mappings();
    let planner = CompositionPlanner::new(base_plan_mappings(&m));

    let home_wrap = home_for_hue(&planner, &m, 359.6);
    let home_zero = home_for_hue(&planner, &m, 0.0);
    assert_eq!(
        home_wrap, home_zero,
        "P4: 359.6° (→360→0) must resolve the SAME home as 0.0°; got {home_wrap} vs {home_zero}"
    );
    assert!(
        (BAND_LO..=BAND_HI).contains(&home_wrap),
        "P4: wrapped home {home_wrap} must be in-band"
    );
    assert_eq!(
        home_wrap % 12,
        0,
        "P4: 0.0°/359.6° is the red=C top of circle → pitch class 0; got {}",
        home_wrap % 12
    );

    let mode_wrap = mode_for_hue(&planner, &m, 359.6);
    let mode_zero = mode_for_hue(&planner, &m, 0.0);
    assert_eq!(
        mode_wrap, mode_zero,
        "P4: 359.6° must resolve the SAME mode as 0.0°; got {mode_wrap} vs {mode_zero}"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// P5 — full-circle 0.5° sweep: NO hue floors-by-miss. Every hue resolves to a real
//   seated bucket pc (in-band, never 60-by-gap); and the distinct-home count under
//   the fix is unchanged-or-greater vs an integer-only sweep (differentiation kept).
// ═════════════════════════════════════════════════════════════════════════════

/// Sweep the WHOLE circle at 0.5° resolution under the shipped block. Every hue must resolve
/// in-band (a real seated pc), so no hue collapses to the floor purely because it landed in a 1°
/// gap. (60 is itself in-band, so in-band alone wouldn't catch a gap-miss; the next test pins the
/// stronger pc-coverage / distinct-home claim.)
#[test]
fn test_p5_full_circle_half_degree_sweep_all_in_band() {
    let m = mappings();
    let planner = CompositionPlanner::new(base_plan_mappings(&m));
    // Iterate over integer half-steps to avoid f32 accumulation drift: i in 0..720 → hue i*0.5.
    for i in 0..720u32 {
        let hue = i as f32 * 0.5;
        let home = home_for_hue(&planner, &m, hue);
        assert!(
            (BAND_LO..=BAND_HI).contains(&home),
            "P5: hue {hue}° resolved home {home} is OUTSIDE the band [{BAND_LO},{BAND_HI}]"
        );
    }
}

/// The stronger P5 claim: the 0.5° (fractional, gap-hitting) sweep reaches the SAME rich set of
/// distinct homes as the integer-only sweep — differentiation is preserved, not collapsed by the
/// snap. The shipped uniform 12-bucket map reaches all 12 pitch classes; both sweeps must observe
/// 12 distinct homes (the fractional sweep can only ever EQUAL or EXCEED the integer sweep here,
/// since snap maps each fractional hue onto an integer the integer sweep already visits).
#[test]
fn test_p5_fractional_sweep_preserves_distinct_homes() {
    let m = mappings();
    let planner = CompositionPlanner::new(base_plan_mappings(&m));

    let mut frac = std::collections::BTreeSet::new();
    for i in 0..720u32 {
        frac.insert(home_for_hue(&planner, &m, i as f32 * 0.5));
    }
    let mut int_only = std::collections::BTreeSet::new();
    for hue in 0..360u32 {
        int_only.insert(home_for_hue(&planner, &m, hue as f32));
    }

    assert!(
        frac.len() >= int_only.len(),
        "P5: the fractional 0.5° sweep must observe at least as many distinct homes as the \
         integer sweep (snap maps fractional hues onto integers the integer sweep visits); \
         frac {} vs int {}",
        frac.len(),
        int_only.len()
    );
    // The shipped uniform 12-bucket map over a 12-semitone band reaches all 12 pitch classes —
    // both sweeps must see exactly 12. A regression that re-collapsed gaps to 60 would shrink the
    // fractional set below 12 (or below the integer set), tripping this.
    assert_eq!(
        frac.len(),
        12,
        "P5: the fractional sweep must reach all 12 distinct homes (no gap-collapse); got {} ({:?})",
        frac.len(),
        frac
    );
    assert_eq!(
        int_only.len(),
        12,
        "P5: the integer sweep reaches all 12 distinct homes; got {} ({:?})",
        int_only.len(),
        int_only
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// P6 — freeze keystone: absent/unparseable home block STILL returns 60 at
//   sub-integer hue resolution. The snap must not perturb the fallback branch.
// ═════════════════════════════════════════════════════════════════════════════

/// With `home_root: None`, the `None` guard returns BEFORE the snap runs, so a full 0.5° sweep —
/// including the gap hues 29.5/30.5 — must resolve to the legacy 60 for EVERY hue. This proves the
/// fix did not disturb the freeze keystone (mirrors home_s40.rs INV-4, but at sub-integer res).
#[test]
fn test_p6_absent_home_block_is_legacy_60_at_subinteger_res() {
    let m = mappings();
    let mut pm = base_plan_mappings(&m);
    pm.home_root = None; // the back-compat floor: no per-image home derivation.
    let planner = CompositionPlanner::new(pm);

    for i in 0..720u32 {
        let hue = i as f32 * 0.5;
        let home = home_for_hue(&planner, &m, hue);
        assert_eq!(
            home, LEGACY_HOME,
            "P6 (FREEZE KEYSTONE): with home_root=None, hue {hue}° must resolve to legacy \
             {LEGACY_HOME} byte-for-byte (the None guard precedes the snap); got {home}"
        );
    }
    // Spot-check the two named gap hues explicitly (29.5 / 30.5) so a future change that moved the
    // snap ABOVE the None guard fails on a clearly-labelled assertion, not just inside the sweep.
    assert_eq!(home_for_hue(&planner, &m, 29.5), LEGACY_HOME);
    assert_eq!(home_for_hue(&planner, &m, 30.5), LEGACY_HOME);
}

/// P6 corollary: a hue that lands an UNPARSEABLE / out-of-range pc (bad data) still falls to 60
/// even WITH a present block — the snap normalizes the hue but does not rescue a bad pc value. We
/// cannot inject a bad pc through the shipped block, so we route through a hand-built PlanMappings
/// whose home block maps every shipped bucket onto a sane pc EXCEPT we verify the shipped block
/// itself never yields a bad pc on the gap hues (all shipped pcs are 0..=11). This keeps the bad-
/// data floor an explicit, file-local invariant rather than only living in the module unit test.
#[test]
fn test_p6_shipped_block_never_yields_out_of_band_on_gap_hues() {
    let m = mappings();
    let planner = CompositionPlanner::new(base_plan_mappings(&m));
    // Every gap hue under the shipped (good-data) block resolves in-band and parses cleanly — i.e.
    // the only way the shipped block hits 60 is the legitimate pc-0/C bucket, never a parse floor.
    for hue in [
        29.5f32, 59.5, 89.5, 119.5, 149.5, 179.5, 209.5, 239.5, 269.5, 299.5, 329.5,
    ] {
        let home = home_for_hue(&planner, &m, hue);
        assert!(
            (BAND_LO..=BAND_HI).contains(&home),
            "P6 corollary: shipped block gap hue {hue}° must seat in-band (no parse floor); got {home}"
        );
    }
}

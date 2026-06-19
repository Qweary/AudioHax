//! tests/keyplan_k2a.rs — the SLICE-K2a generalized structural-key-plan net.
//!
//! K2a (landed on `src/composition.rs` + `src/pure_analysis.rs`, realizer byte-frozen)
//! generalizes the K1 single-B-excursion key plan so a MULTI-excursion form's B and C sections
//! each read their OWN image region's per-region affect (`foreground_*` / `background_*`) and
//! travel to GENUINELY DISTINCT related keys, and re-roots the HARMONY (chord roots, not just the
//! theme melody) by the per-section offset. This net proves the properties pinned by the kickoff:
//!
//!   1. distinct_excursions      — B (region_related:b) and C (region_related:c) reading DIFFERENT
//!                                 region affect resolve to DISTINCT offsets ("the eye sweeps twice").
//!   2. energy_descending_rank   — rank 0 reads the MORE-energetic non-subject region, rank 1 the
//!                                 less; swap the energies → B/C swap which region they read.
//!   3. harmony_reroots          — a non-zero per-section offset shifts the section's chord ROOTS
//!                                 by that offset (the planner's `generate_chords(root+offset,…)`
//!                                 seam); at offset 0 the roots are byte-unchanged.
//!   4. resolution_policy        — `Resolve` forces the FINAL section offset to 0 (Invariant A);
//!                                 `Open` leaves it at its rule-derived value (mechanism tested even
//!                                 though `Open` ships OFF by default).
//!   5. at_most_two_distinct_non_home_keys — the resolved scheme has ≤ 2 distinct non-zero offsets.
//!   6. home_sections_are_home   — every Statement/Return section resolves to offset 0.
//!   7. home_only_byte_zero      — the `home_only` (and an absent/unknown) scheme → all-zero.
//!   8. per_region_fallback_reproduces_k1 — per-region fields == the whole-image fallback ⇒ the
//!                                 generalized offset reproduces the K1 result (the invariant).
//!   9. parse_offset_rule_grammar — home / region_related:b|c|d map correctly; unknown → home (0).
//!
//! HEADLESS, in the same sense as keyplan_s25.rs / composition_s15.rs: NO image type, NO OpenCV,
//! NO audio hardware. The K2a planner fns (`resolve_key_scheme`, `region_excursion_offset`,
//! `parse_offset_rule`, `RegionAffect`, `OffsetRule`) are module-PRIVATE in `src/composition.rs`,
//! so — exactly like keyplan_s25.rs — this integration net CANNOT call them directly and instead
//! drives EVERYTHING through the PUBLIC entry points:
//!   * `CompositionPlanner::plan(&ImageUnderstanding, &MappingTable)`, with a SYNTHESIZED
//!     `PlanMappings` (cloned from the shipped one so the `global` harmony data is real, then with
//!     its `form` / `key_scheme` SelectTables steered and the multi-excursion `KeyScheme` added to
//!     `key_scheme_catalogue`) — the planner types `PlanMappings` / `KeyScheme` / `KeySchemeSection`
//!     / `SelectTable` / `SelectRule` / `Predicate` / `ResolutionPolicy` are all PUBLIC.
//!   * `ChordEngine::generate_chords` (PUBLIC, RNG-FREE) for the harmony-reroot witness — the exact
//!     re-root seam the planner uses (`generate_chords(home_root + offset, …)`).
//!
//! RNG-BOUNDARY DISCIPLINE (same as keyplan_s25.rs): `plan()` delegates per-section harmony to
//! `pick_progression` (`thread_rng`), so chords / Roman numerals / per-step `StepPlan.chord` are
//! NON-deterministic and are NEVER asserted via `plan()`. Every property asserted through `plan()`
//! is RNG-INDEPENDENT (the per-section `key_offset_semitones` + the `KeyTempoPlan.key_scheme`). The
//! ONE harmony-content assertion (property 3) uses `generate_chords` DIRECTLY with a FIXED
//! progression — no `pick_progression`, fully deterministic — so it never touches the RNG path.

use audiohax::chord_engine::ChordEngine;
use audiohax::composition::{
    valence_family_mode, CompositionPlanner, ImageUnderstanding, KeyScheme, KeySchemeSection,
    ModeValenceCuts, PlanMappings, Predicate, ResolutionPolicy, SelectRule, SelectTable,
    ThematicRole,
};
use audiohax::mapping_loader::{load_mappings, rebuild_mapping_table, MappingTable};

const MAPPINGS_PATH: &str = "assets/mappings.json";

/// The v1 menu set every NON-ZERO offset must belong to: dominant +7, subdominant +5, relative
/// ±3. Zero (home) is always allowed.
const MENU: [i8; 4] = [7, 5, 3, -3];

/// The shipped `assets/mappings.json` mapping table (real `global` harmony data).
fn mappings() -> MappingTable {
    load_mappings(MAPPINGS_PATH).expect("mappings.json loads")
}

/// The shipped composition `PlanMappings`.
fn base_plan_mappings(m: &MappingTable) -> PlanMappings {
    m.composition
        .clone()
        .expect("composition block present in mappings.json")
        .into()
}

/// A `SelectTable` that ALWAYS resolves to `id` (empty default rule set → the default fires). Used
/// to deterministically pin the `form` / `key_scheme` axis to a chosen id, so the planner produces
/// the exact multi-excursion shape we want to exercise — no reliance on the shipped axis rules.
fn always(id: &str) -> SelectTable {
    SelectTable {
        default: id.to_string(),
        rules: Vec::new(),
    }
}

/// A `SelectTable` that picks `id` ONLY when `fg_bg_contrast >= 0.25` fires (mirrors the shipped
/// `key_scheme` gate), else falls to `default_id`. Used by `energy_descending_rank` so the SAME
/// gate the shipped data uses is exercised, while still steering to a 4-section scheme.
fn gated(default_id: &str, id: &str) -> SelectTable {
    SelectTable {
        default: default_id.to_string(),
        rules: vec![SelectRule {
            when: vec![Predicate {
                knob: audiohax::composition::Knob::FgBgContrast,
                op: audiohax::composition::CmpOp::Ge,
                lo: 0.25,
                hi: 0.0,
            }],
            pick: id.to_string(),
        }],
    }
}

/// A 4-section ABAC key scheme (home / region_related:b / home / region_related:c) with a chosen
/// resolution policy — aligned 1:1 with the shipped `abac` form (Statement/Contrast/Return/Coda),
/// so the role-alignment debug witness in `resolve_key_scheme` stays satisfied (home→home-role,
/// region_related→non-home-role).
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

/// Build a planner whose `form` is pinned to `abac` and whose `key_scheme` is pinned to the given
/// scheme id (added to the catalogue). The cloned-from-shipped `global`/harmony data stays intact.
fn planner_with(scheme: KeyScheme, m: &MappingTable) -> CompositionPlanner {
    let mut pm = base_plan_mappings(m);
    pm.form = always("abac"); // Statement / Contrast / Return / Coda
    pm.key_scheme = always(&scheme.id);
    pm.key_scheme_catalogue.push(scheme);
    CompositionPlanner::new(pm)
}

/// A neutral understanding with the per-region affect fields set so B (rank-0 region) and C
/// (rank-1 region) read DISTINCT affect. `fg_energy`/`bg_energy` set the rank; `*_brightness` the
/// per-region valence; `*_hue` the per-region hue (held == `subject_hue` so the near path holds and
/// the test isolates the valence axis). `subject_hue` is the reference for the hue-distance test.
#[allow(clippy::too_many_arguments)]
fn craft_regions(
    subject_hue: f32,
    fg_energy: f32,
    fg_brightness: f32,
    fg_hue: f32,
    bg_energy: f32,
    bg_brightness: f32,
    bg_hue: f32,
) -> ImageUnderstanding {
    ImageUnderstanding {
        // Fire the shipped gate too (harmless under the `always` table; needed under `gated`).
        fg_bg_contrast: 0.30,
        subject_hue,
        avg_saturation: 50.0,
        foreground_energy: fg_energy,
        foreground_brightness: fg_brightness,
        foreground_hue: fg_hue,
        background_energy: bg_energy,
        background_brightness: bg_brightness,
        background_hue: bg_hue,
        ..ImageUnderstanding::neutral()
    }
}

/// The B (Contrast) and C (Coda) section offsets from a resolved plan, by thematic role.
fn b_and_c(plan: &audiohax::composition::CompositionPlan) -> (i8, i8) {
    let b = plan
        .sections
        .iter()
        .find(|s| s.thematic_role == ThematicRole::Contrast)
        .expect("a B Contrast")
        .key_offset_semitones;
    let c = plan
        .sections
        .iter()
        .find(|s| s.thematic_role == ThematicRole::Coda)
        .expect("a C Coda")
        .key_offset_semitones;
    (b, c)
}

/// The distinct NON-ZERO offsets across a `key_scheme`, sorted+deduped.
fn distinct_nonzero(scheme: &[i8]) -> Vec<i8> {
    let mut v: Vec<i8> = scheme.iter().copied().filter(|&o| o != 0).collect();
    v.sort_unstable();
    v.dedup();
    v
}

// ═════════════════════════════════════════════════════════════════════════════
// Property 1 — distinct_excursions ("the eye sweeps twice")
// ═════════════════════════════════════════════════════════════════════════════

/// A multi-excursion (B=region_related:b, C=region_related:c) scheme whose foreground and
/// background regions carry DIFFERENT affect resolves B and C to DISTINCT offsets. Resolution is
/// `Open` so the FINAL (C/Coda) section keeps its own rule-derived offset (under `Resolve` C would
/// be forced to 0 and the divergence would be unobservable at C — see property 4). The B region
/// (rank 0) is BRIGHT near-hue → dominant +7; the C region (rank 1) is DARK near-hue → subdominant
/// +5: two genuinely distinct related keys, both in-menu.
#[test]
fn distinct_excursions() {
    let m = mappings();
    let planner = planner_with(abac_scheme("abac_open", ResolutionPolicy::Open), &m);

    // foreground = rank 0 (more energetic), BRIGHT (→ +7); background = rank 1, DARK (→ +5).
    // Both region hues == subject_hue (40) → near path (hue dist 0 < 60°) on BOTH excursions.
    let u = craft_regions(40.0, /*fg*/ 0.9, 0.9, 40.0, /*bg*/ 0.3, 0.1, 40.0);
    let plan = planner.plan(&u, &m);
    let (b, c) = b_and_c(&plan);

    assert_eq!(b, 7, "B (rank-0, BRIGHT, near) → dominant +7; got {b}");
    assert_eq!(c, 5, "C (rank-1, DARK, near) → subdominant +5; got {c}");
    assert_ne!(
        b, c,
        "B and C must travel to GENUINELY DISTINCT keys (the eye sweeps twice), got B={b} C={c}"
    );
    assert!(
        MENU.contains(&b) && MENU.contains(&c),
        "both in the v1 menu"
    );
    // The home roles flank the excursions at 0 (asserted dedicated in property 6).
    let key_scheme = &plan.key_tempo.key_scheme;
    assert_eq!(key_scheme.len(), 4, "abac form has 4 sections");
    assert_eq!(key_scheme[0], 0, "A (Statement) is home");
    assert_eq!(key_scheme[2], 0, "interior A (Return) is home");
}

// ═════════════════════════════════════════════════════════════════════════════
// Property 2 — energy_descending_rank (rank 0 = more energetic; swap → B/C swap regions)
// ═════════════════════════════════════════════════════════════════════════════

/// rank 0 (B) reads the MORE-energetic non-subject region, rank 1 (C) the less-energetic one.
/// We give the two regions affect that resolves to DIFFERENT offsets (a BRIGHT near region → +7
/// and a DARK near region → +5), then SWAP which region carries the higher energy and confirm B
/// and C SWAP their offsets — proving the rank is driven by energy descending, not by a fixed
/// foreground/background slot. Uses the `gated` key_scheme table so the SHIPPED firing gate is also
/// exercised (the scheme only fires when `fg_bg_contrast >= 0.25`, which `craft_regions` sets).
#[test]
fn energy_descending_rank() {
    let m = mappings();
    let mut pm = base_plan_mappings(&m);
    pm.form = always("abac");
    pm.key_scheme = gated("home_only", "abac_swap");
    pm.key_scheme_catalogue
        .push(abac_scheme("abac_swap", ResolutionPolicy::Open));
    let planner = CompositionPlanner::new(pm);

    // Case 1: FOREGROUND more energetic (rank 0) and BRIGHT (→ +7); background rank 1, DARK (→ +5).
    let fg_hot = craft_regions(40.0, /*fg*/ 0.9, 0.9, 40.0, /*bg*/ 0.2, 0.1, 40.0);
    let (b1, c1) = b_and_c(&planner.plan(&fg_hot, &m));
    assert_eq!(
        (b1, c1),
        (7, 5),
        "fg hot+bright = rank0 → B +7; bg cold+dark = rank1 → C +5; got B={b1} C={c1}"
    );

    // Case 2: SWAP the energies. Now BACKGROUND is more energetic (rank 0) — but background is
    // DARK (→ +5) and foreground (now rank 1) is BRIGHT (→ +7). So B/C SWAP: B reads the dark
    // background (+5), C reads the bright foreground (+7).
    let bg_hot = craft_regions(40.0, /*fg*/ 0.2, 0.9, 40.0, /*bg*/ 0.9, 0.1, 40.0);
    let (b2, c2) = b_and_c(&planner.plan(&bg_hot, &m));
    assert_eq!(
        (b2, c2),
        (5, 7),
        "energies swapped: B now reads the DARK background (+5), C the BRIGHT foreground (+7); \
         got B={b2} C={c2}"
    );

    // The defining property: swapping the energy ordering swapped which region each rank reads,
    // so B and C exchanged their offsets.
    assert_eq!(
        (b1, c1),
        (c2, b2),
        "rank is energy-DESCENDING: swapping region energies swaps B↔C offsets"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Property 3 — harmony_reroots (chord ROOTS move with the offset; offset 0 byte-unchanged)
// ═════════════════════════════════════════════════════════════════════════════

/// A non-zero per-section offset shifts the section's generated chord ROOTS by exactly that offset.
/// This is the planner's re-root seam — composition.rs builds each section's chords with
/// `generate_chords(home_root_midi + key_offset_semitones, …)` (NOT the literal home root). We
/// exercise that seam DIRECTLY and DETERMINISTICALLY via the PUBLIC, RNG-FREE
/// `ChordEngine::generate_chords` with a FIXED progression (the plan()-internal progression comes
/// from `pick_progression`/`thread_rng` and so cannot be compared across two plans — see the
/// header RNG discipline). Params are chosen so the progression maps 1:1 to chords (no
/// secondary-dominant insert: low edge; no minor-iv borrow: brightness_drop 0; no mode-mixture
/// append: colorfulness 0), so chord i at root R+off == chord i at root R, transposed by `off`.
///
/// Asserts: (a) at offset 0 the chords are BYTE-IDENTICAL (the byte-safe anchor); (b) at each
/// non-zero menu offset EVERY note (and hence the root, `notes[0]`) shifts by exactly the offset.
#[test]
fn harmony_reroots() {
    let m = mappings();
    let engine = ChordEngine::new(rebuild_mapping_table(&m));
    let progression: Vec<String> = vec!["I".into(), "IV".into(), "V".into(), "I".into()];
    const HOME: u8 = 60; // C4, the planner's home_root_midi seed.

    // 1:1-mapping params: low edge (no V/x insert), brightness_drop 0 (no borrowed iv),
    // colorfulness 0 (no mode-mixture append). Deterministic — no RNG in generate_chords.
    let gen = |root: u8| engine.generate_chords(&progression, root, "Ionian", 0.0, 0.0, 50.0, 0.0);

    let base = gen(HOME);
    assert_eq!(
        base.len(),
        progression.len(),
        "the chosen params must map the progression 1:1 (no inserted/appended chords) so the \
         re-root comparison is well-defined; got {} chords for {} symbols",
        base.len(),
        progression.len()
    );

    // (a) Offset 0 → byte-identical (the byte-safe home anchor: home_root + 0 == home_root).
    let at_zero = gen((HOME as i16 + 0) as u8);
    assert_eq!(
        base, at_zero,
        "offset 0 must leave the chords BYTE-IDENTICAL (the byte-freeze anchor)"
    );

    // (b) Each non-zero menu offset transposes EVERY note (hence the root notes[0]) by `off`.
    for &off in &MENU {
        let root = (HOME as i16 + off as i16) as u8;
        let shifted = gen(root);
        assert_eq!(
            shifted.len(),
            base.len(),
            "offset {off}: chord count must match the home chord count"
        );
        for (i, (b, s)) in base.iter().zip(shifted.iter()).enumerate() {
            assert_eq!(
                s.notes.len(),
                b.notes.len(),
                "offset {off}: chord {i} ({}) voice count must match",
                b.name
            );
            for (j, (&bn, &sn)) in b.notes.iter().zip(s.notes.iter()).enumerate() {
                assert_eq!(
                    sn as i16 - bn as i16,
                    off as i16,
                    "offset {off}: chord {i} ({}) note {j} must shift by exactly {off} \
                     (home {bn} → {sn}) — the HARMONY re-roots, not just the melody",
                    b.name
                );
            }
        }
        // Specifically the ROOT (notes[0]) of every chord shifted by the offset.
        for (b, s) in base.iter().zip(shifted.iter()) {
            assert_eq!(
                s.notes[0] as i16 - b.notes[0] as i16,
                off as i16,
                "offset {off}: chord {} ROOT must re-root by {off}",
                b.name
            );
        }
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Property 4 — resolution_policy (Resolve forces final→0; Open keeps it)
// ═════════════════════════════════════════════════════════════════════════════

/// `Resolve` forces the FINAL section's offset to 0 even when its rule is `region_related:*`
/// (Invariant A — a Coda on new material still lands home); `Open` leaves the final offset at its
/// rule-derived value (the deliberate off-home ending). Both are driven through `plan()` with the
/// SAME firing region affect; only the scheme's `resolution` differs, so the divergence is
/// attributable to the policy alone. `Open` ships OFF by default, but the MECHANISM is tested here.
#[test]
fn resolution_policy() {
    let m = mappings();
    // Region affect that makes C's rule resolve NON-ZERO before the policy is applied: the C region
    // (rank 1) BRIGHT + near → +7. Under Resolve it is forced to 0; under Open it stays +7.
    // foreground rank 0 (bright, +7), background rank 1 (bright, +7) — so C's pre-policy value is +7.
    let regions = || {
        craft_regions(40.0, /*fg*/ 0.9, 0.9, 40.0, /*bg*/ 0.3, 0.9, 40.0)
    };

    // Resolve → the FINAL (Coda) section is forced home (0).
    let p_res = planner_with(abac_scheme("abac_res", ResolutionPolicy::Resolve), &m);
    let plan_res = p_res.plan(&regions(), &m);
    let (b_res, c_res) = b_and_c(&plan_res);
    assert_eq!(
        c_res, 0,
        "Resolve must force the FINAL (Coda) offset to 0 (Invariant A); got {c_res} \
         (scheme {:?})",
        plan_res.key_tempo.key_scheme
    );
    assert_ne!(
        b_res, 0,
        "the non-final excursion (B) still travels under Resolve; got {b_res}"
    );

    // Open → the FINAL (Coda) section keeps its rule-derived non-zero offset.
    let p_open = planner_with(abac_scheme("abac_open2", ResolutionPolicy::Open), &m);
    let plan_open = p_open.plan(&regions(), &m);
    let (_b_open, c_open) = b_and_c(&plan_open);
    assert_ne!(
        c_open, 0,
        "Open must keep the FINAL (Coda) off-home offset; got {c_open} (scheme {:?})",
        plan_open.key_tempo.key_scheme
    );
    assert!(
        MENU.contains(&c_open),
        "the open final offset is still in the v1 menu, got {c_open}"
    );
    // The policy is the ONLY difference, and it is observable at the final section.
    assert_ne!(
        c_res, c_open,
        "the resolution policy alone changes the FINAL offset (Resolve 0 vs Open {c_open})"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Property 5 — at_most_two_distinct_non_home_keys (short-piece cap)
// ═════════════════════════════════════════════════════════════════════════════

/// The count of distinct non-zero offsets across ANY resolved scheme is ≤ 2 (the short-piece cap
/// that keeps even the two-excursion K2a plan inside "at most two journeys"). Swept across region
/// affect that drives B and C to the same key, to distinct keys, and to the relative — under both
/// Resolve and Open — so the cap is asserted over real variety, not one input.
#[test]
fn at_most_two_distinct_non_home_keys() {
    let m = mappings();

    // (subject_hue, fg_e, fg_b, fg_h, bg_e, bg_b, bg_h) cases spanning same/distinct/relative.
    let cases = [
        (40.0f32, 0.9f32, 0.9f32, 40.0f32, 0.3f32, 0.9f32, 40.0f32), // both bright near → both +7 (1 distinct)
        (40.0, 0.9, 0.9, 40.0, 0.3, 0.1, 40.0), // bright + dark near → +7,+5 (2 distinct)
        (0.0, 0.9, 0.9, 200.0, 0.3, 0.1, 0.0),  // fg far-hue (relative −3) + bg near dark (+5)
    ];
    for res in [ResolutionPolicy::Resolve, ResolutionPolicy::Open] {
        let planner = planner_with(abac_scheme("abac_cap", res), &m);
        for &(sh, fe, fb, fh, be, bb, bh) in &cases {
            let u = craft_regions(sh, fe, fb, fh, be, bb, bh);
            let plan = planner.plan(&u, &m);
            let distinct = distinct_nonzero(&plan.key_tempo.key_scheme);
            assert!(
                distinct.len() <= 2,
                "at most TWO distinct non-home keys; got {distinct:?} from {:?} \
                 (res {res:?}, sh {sh})",
                plan.key_tempo.key_scheme
            );
            // Every offset is home or in-menu (no off-menu garbage).
            assert!(
                plan.key_tempo
                    .key_scheme
                    .iter()
                    .all(|&o| o == 0 || MENU.contains(&o)),
                "off-menu offset in {:?} (res {res:?}, sh {sh})",
                plan.key_tempo.key_scheme
            );
        }
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Property 6 — home_sections_are_home (Statement/Return never modulate)
// ═════════════════════════════════════════════════════════════════════════════

/// EVERY Statement/Return section resolves to offset 0 — the home roles never modulate (the
/// excursions live only on the Contrast/Coda sections). Asserted on a FIRING multi-excursion plan
/// under both resolution policies (the home roles are unaffected by the resolution policy, which
/// only touches the FINAL section).
#[test]
fn home_sections_are_home() {
    let m = mappings();
    for res in [ResolutionPolicy::Resolve, ResolutionPolicy::Open] {
        let planner = planner_with(abac_scheme("abac_home", res), &m);
        let u = craft_regions(40.0, /*fg*/ 0.9, 0.9, 40.0, /*bg*/ 0.3, 0.1, 40.0);
        let plan = planner.plan(&u, &m);
        let mut home_roles = 0usize;
        for (i, s) in plan.sections.iter().enumerate() {
            if matches!(
                s.thematic_role,
                ThematicRole::Statement | ThematicRole::Return
            ) {
                home_roles += 1;
                assert_eq!(
                    s.key_offset_semitones, 0,
                    "section {i} ({:?}) is a HOME role and must stay home; got {} (res {res:?})",
                    s.thematic_role, s.key_offset_semitones
                );
            }
        }
        assert!(
            home_roles >= 2,
            "an ABAC form has at least a Statement and a Return (res {res:?})"
        );
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Property 7 — home_only_byte_zero (the identity / byte-freeze anchor)
// ═════════════════════════════════════════════════════════════════════════════

/// The shipped `home_only` scheme (empty `sections`) AND an ABSENT/UNKNOWN scheme id BOTH resolve
/// to all-zero offsets — the identity / byte-freeze anchor. Driven through `plan()`:
///   (a) the shipped `home_only` (selected by NOT firing the gate: fg_bg_contrast < 0.25);
///   (b) a `key_scheme` axis whose default names an id NOT in the catalogue → the planner's
///       `lookup_key_scheme` returns None → `resolve_key_scheme(None, …)` → all-zero.
/// Even with vivid per-region affect set, the offsets stay home because no scheme drives them.
#[test]
fn home_only_byte_zero() {
    let m = mappings();

    // Vivid region affect that WOULD drive excursions IF a region_related scheme were active.
    let vivid = ImageUnderstanding {
        fg_bg_contrast: 0.0, // < 0.25 → the shipped gate does NOT fire → home_only.
        subject_hue: 40.0,
        foreground_energy: 0.9,
        foreground_brightness: 0.9,
        foreground_hue: 40.0,
        background_energy: 0.3,
        background_brightness: 0.1,
        background_hue: 40.0,
        ..ImageUnderstanding::neutral()
    };

    // (a) Shipped mappings, gate NOT fired → home_only → all-zero.
    let shipped = CompositionPlanner::new(base_plan_mappings(&m));
    let plan_a = shipped.plan(&vivid, &m);
    assert!(
        plan_a.key_tempo.key_scheme.iter().all(|&o| o == 0),
        "home_only (gate not fired) must be all-zero, got {:?}",
        plan_a.key_tempo.key_scheme
    );
    assert!(
        plan_a.sections.iter().all(|s| s.key_offset_semitones == 0),
        "home_only: every section stays home"
    );

    // (b) An UNKNOWN scheme id (not in the catalogue) degrades to all-zero (None → identity).
    let mut pm = base_plan_mappings(&m);
    pm.form = always("abac");
    pm.key_scheme = always("no_such_scheme_id"); // not present in key_scheme_catalogue
    let unknown = CompositionPlanner::new(pm);
    let mut firing = vivid.clone();
    firing.fg_bg_contrast = 0.30; // even firing the gate cannot help — the scheme id is unknown.
    let plan_b = unknown.plan(&firing, &m);
    assert!(
        plan_b.key_tempo.key_scheme.iter().all(|&o| o == 0),
        "an unknown scheme id must degrade to all-zero (None → byte-stable identity), got {:?}",
        plan_b.key_tempo.key_scheme
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Property 8 — per_region_fallback_reproduces_k1 (the generalization invariant)
// ═════════════════════════════════════════════════════════════════════════════

/// When the per-region fields equal the WHOLE-IMAGE fallback (`*_brightness == avg_brightness/100`,
/// `*_hue == secondary_hue`), the generalized K2a offset reproduces the K1 single-B-excursion
/// result. Since `region_excursion_offset`/`excursion_offset` are PRIVATE, this is driven through
/// `plan()`: a 3-section `aba` form on the shipped `aba_excursion` scheme (B = region_related:b)
/// is the K1 path. We sweep valence (via the per-region brightness, set EQUAL to the whole-image
/// fallback) × hue-distance (near vs strong-contrast) × home-mode family and assert the resolved B
/// offset matches the K1 expectation computed from the SAME affect rules:
///   near + HIGH/MID valence → +7; near + LOW valence → +5; strong hue contrast → relative (±3).
/// This is the integration-level witness of the generalization invariant (the unit-level
/// `region_excursion_reproduces_k1_on_whole_image` lives in the private mod tests).
#[test]
fn per_region_fallback_reproduces_k1() {
    let m = mappings();
    // Pin the form to `aba` shape via the shipped `ternary_aba` (Statement/Contrast/Return) and
    // the shipped `aba_excursion` scheme (home / region_related:b / home) — the exact K1 path.
    let mut pm = base_plan_mappings(&m);
    pm.form = always("ternary_aba");
    pm.key_scheme = always("aba_excursion");
    let planner = CompositionPlanner::new(pm);

    // The K1 menu math, recomputed here from the SAME pinned thresholds (τ_lo 0.40 LOW-inclusive,
    // τ_hi 0.60 HIGH, τ_contrast 60°; HIGH and MID both lift to +7) to predict the expected offset.
    // The relative-excursion direction on STRONG hue contrast (hue_dist >= 60°) depends on the
    // home_mode FAMILY: `relative_offset` returns +3 for a minor-family home mode and −3 for a
    // major/Ionian-family one. `minor_family` is therefore a property of the resolved home_mode.
    fn expected(region_valence: f32, hue_dist: f32, minor_family: bool) -> i8 {
        if hue_dist >= 60.0 {
            if minor_family {
                3
            } else {
                -3
            }
        } else if region_valence <= 0.40 {
            5 // LOW → subdominant
        } else {
            7 // HIGH or MID → dominant
        }
    }

    // C6.6: the home_mode FAMILY (major vs minor) is no longer hue-only — VALENCE owns the third.
    // `home_mode = valence_family_mode(hue_mode, affect_valence, cuts)`, so before reading the
    // relative-excursion sign we must derive `minor_family` from the PROJECTED home_mode, applying
    // the SAME 0.55/0.45 cuts to the SAME whole-image `affect_valence` the planner computes — NOT
    // from the raw hue. This mirrors `composition::valence_family_mode` exactly:
    //   * `affect_valence` = the shipped VALENCE blend on this test's inputs
    //     (avg_brightness = region_valence*100, avg_saturation = 50, fg_bg_contrast = 0.30):
    //       v = 0.70*(avg_brightness/100) + 0.20*(avg_saturation/100) + 0.10*(0.5 + 0.5*fg_bg)
    //         = 0.70*region_valence + 0.10 + 0.065 = 0.70*region_valence + 0.165
    //   * v >= 0.55 → MAJOR family; v <= 0.45 → MINOR family; the dead band (0.45,0.55) leaves the
    //     HUE-selected mode (and hence its family) untouched (legacy behaviour).
    // We reuse the PUBLIC `valence_family_mode` so this expectation tracks the production rule
    // automatically, then read the family off the resolved mode string (the same `minor_family`
    // predicate `relative_offset` uses internally).
    const MAJOR_MIN: f32 = 0.55; // == assets/mappings.json composition.affect.mode_valence_cuts
    const MINOR_MAX: f32 = 0.45;
    fn projected_minor_family(hue_mode: &str, region_valence: f32) -> bool {
        let affect_valence = 0.70 * region_valence + 0.20 * 0.5 + 0.10 * (0.5 + 0.5 * 0.30);
        let cuts = Some(ModeValenceCuts {
            major_min: MAJOR_MIN,
            minor_max: MINOR_MAX,
        });
        let resolved = valence_family_mode(hue_mode, affect_valence, &cuts);
        let m = resolved.to_ascii_lowercase();
        m.contains("aeolian")
            || m.contains("minor")
            || m.contains("dorian")
            || m.contains("phrygian")
            || m.contains("locrian")
    }

    // home_mode is hue-selected via hue_to_mode: hue 120 → Ionian (major-family hue), hue 250 →
    // Aeolian (minor-family hue) (matching keyplan_s25.rs §5 notes). We drive the home mode by
    // `dominant_hue`; `hue_mode` is the PRE-projection mode string for that hue.
    for &(dom_hue, hue_mode) in &[(120.0f32, "Ionian"), (250.0f32, "Aeolian")] {
        for &region_valence in &[0.0f32, 0.39, 0.40, 0.41, 0.59, 0.60, 0.61, 1.0] {
            for &hue_dist in &[0.0f32, 90.0] {
                // The B region (rank 0) carries the whole-image FALLBACK affect:
                //   valence == avg_brightness/100  (we set BOTH so the fallback identity holds)
                //   hue     == secondary_hue       (the K1 whole-image hue)
                // subject_hue is the reference; secondary_hue/region hue is `subject + hue_dist`.
                let subject_hue = dom_hue; // keep subject on the home hue.
                let region_hue = (subject_hue + hue_dist) % 360.0;
                let avg_brightness = region_valence * 100.0;
                let u = ImageUnderstanding {
                    fg_bg_contrast: 0.30, // fire aba_excursion.
                    dominant_hue: dom_hue,
                    avg_brightness,
                    avg_saturation: 50.0,
                    subject_hue,
                    secondary_hue: region_hue,
                    // Per-region fields == the whole-image fallback (the invariant's premise).
                    foreground_energy: 0.6,
                    background_energy: 0.4, // foreground = rank 0 = B region.
                    foreground_brightness: region_valence,
                    background_brightness: region_valence,
                    foreground_hue: region_hue,
                    background_hue: region_hue,
                    ..ImageUnderstanding::neutral()
                };
                let plan = planner.plan(&u, &m);
                let b = plan
                    .sections
                    .iter()
                    .find(|s| s.thematic_role == ThematicRole::Contrast)
                    .expect("a B Contrast")
                    .key_offset_semitones;
                // C6.6: family from the PROJECTED home_mode (valence owns the third), not from hue.
                let minor_family = projected_minor_family(hue_mode, region_valence);
                let want = expected(region_valence, hue_dist, minor_family);
                assert_eq!(
                    b, want,
                    "K1 reproduction (per-region == whole-image fallback) failed: \
                     dom_hue {dom_hue} hue_mode {hue_mode} projected_minor {minor_family} \
                     valence {region_valence} hue_dist {hue_dist} → expected {want}, got {b} \
                     (scheme {:?})",
                    plan.key_tempo.key_scheme
                );
            }
        }
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Property 9 — parse_offset_rule_grammar (home / region_related:b|c|d; unknown → home)
// ═════════════════════════════════════════════════════════════════════════════

/// `parse_offset_rule` is PRIVATE, so its grammar is asserted THROUGH `resolve_key_scheme` (driven
/// by `plan()`): a scheme carrying `home` / `region_related:b` / `region_related:c` /
/// `region_related:d` resolves each section per the grammar, and an UNKNOWN rule degrades to home
/// (offset 0). We build a 5-section `abbac`-style scheme/form so b, c AND d (rank 2) all appear,
/// plus an unknown rule, and confirm:
///   * home rules → 0;
///   * region_related:b|c → non-zero in-menu (ranks 0/1 read the two real regions);
///   * region_related:d (rank 2, beyond the two regions) → the whole-image fallback offset (still
///     in-menu, never a panic — the planner is total);
///   * an unknown rule on a HOME-role section → 0 (the byte-stable degrade).
#[test]
fn parse_offset_rule_grammar() {
    let m = mappings();

    // A 5-section form: Statement / Contrast / Contrast / Return / Coda (the shipped `abbac`).
    // Scheme: home / region_related:b / region_related:c / home / region_related:d.
    // Role alignment: home→Statement/Return, region_related→Contrast/Coda (debug witness happy).
    let scheme = KeyScheme {
        id: "abbac_bcd".into(),
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
                label: "B'".into(),
                offset_rule: "region_related:c".into(),
            },
            KeySchemeSection {
                label: "A".into(),
                offset_rule: "home".into(),
            },
            KeySchemeSection {
                label: "C".into(),
                offset_rule: "region_related:d".into(),
            },
        ],
        resolution: ResolutionPolicy::Open, // keep the final (d) observable.
        pivot: false,
    };
    let mut pm = base_plan_mappings(&m);
    pm.form = always("abbac");
    pm.key_scheme = always("abbac_bcd");
    pm.key_scheme_catalogue.push(scheme);
    let planner = CompositionPlanner::new(pm);

    // Distinct region affect so b (rank 0) and c (rank 1) read different regions; near path held.
    let u = craft_regions(40.0, /*fg*/ 0.9, 0.9, 40.0, /*bg*/ 0.3, 0.1, 40.0);
    let plan = planner.plan(&u, &m);
    let ks = &plan.key_tempo.key_scheme;
    assert_eq!(ks.len(), 5, "abbac form has 5 sections, got {ks:?}");

    // home rules → 0.
    assert_eq!(ks[0], 0, "section 0 (home) → 0; got {ks:?}");
    assert_eq!(ks[3], 0, "section 3 (home) → 0; got {ks:?}");
    // region_related:b (rank 0, bright near) → +7; region_related:c (rank 1, dark near) → +5.
    assert_eq!(
        ks[1], 7,
        "region_related:b (rank 0, bright) → +7; got {ks:?}"
    );
    assert_eq!(ks[2], 5, "region_related:c (rank 1, dark) → +5; got {ks:?}");
    // region_related:d (rank 2 — beyond the two real regions) → whole-image fallback, in-menu,
    // never a panic (the planner is total). With neutral whole-image affect (avg_brightness 50 →
    // affect_valence MID, secondary_hue 0 vs subject 40 → near) the fallback is the dominant +7.
    assert!(
        MENU.contains(&ks[4]),
        "region_related:d (rank 2) → in-menu whole-image fallback (no panic), got {}",
        ks[4]
    );

    // Unknown rule on a HOME-role section degrades to home (0) — built as its own minimal plan so
    // the role-alignment debug witness stays satisfied (unknown → Home must sit on a home role).
    let unknown_scheme = KeyScheme {
        id: "unknown_on_home".into(),
        sections: vec![
            KeySchemeSection {
                label: "A".into(),
                offset_rule: "home".into(),
            },
            KeySchemeSection {
                label: "A2".into(),
                offset_rule: "region_related:zzz".into(), // unknown → Home (0)
            },
        ],
        resolution: ResolutionPolicy::Open,
        pivot: false,
    };
    // The shipped `theme_and_variations` form is T/V1/V2 — not a home/home pair. Build a custom
    // 2-section home/home form via `aaba`? No — simplest: pin the form to one whose first two
    // sections are both home-roles. `ternary_aba` is Statement/Contrast/Return — section 1 is a
    // Contrast (non-home), which would trip the unknown→Home-on-non-home debug witness. So we
    // instead assert the unknown-degrade through a form whose sections 0/1 are BOTH home roles:
    // `aaba` is Statement/Statement/Contrast/Return — sections 0 and 1 are both Statement (home).
    let mut pm2 = base_plan_mappings(&m);
    pm2.form = always("aaba");
    pm2.key_scheme = always("unknown_on_home");
    pm2.key_scheme_catalogue.push(unknown_scheme);
    let planner2 = CompositionPlanner::new(pm2);
    let plan2 = planner2.plan(&u, &m);
    let ks2 = &plan2.key_tempo.key_scheme;
    assert_eq!(
        ks2[0], 0,
        "section 0 (home) → 0 under the unknown-rule scheme; got {ks2:?}"
    );
    assert_eq!(
        ks2[1], 0,
        "section 1 unknown rule on a HOME role degrades to home (0); got {ks2:?}"
    );
}

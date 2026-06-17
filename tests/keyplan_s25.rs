//! tests/keyplan_s25.rs — the S25 SLICE-K1 IMAGE→HOME-KEY + ABA STRUCTURAL KEY-PLAN net.
//!
//! Proves the K1 invariants pinned in `docs/design-s24-image-as-form-key-plan.md` §5
//! (the eleven property tests) and the §3 byte-freeze argument. K1 fills the already-threaded
//! `key_scheme`/`key_offset_semitones` spine: a clear subject/ground image (fg_bg_contrast >=
//! 0.25) now LEAVES home for its B (Contrast) section to a closely-related key — dominant
//! (+7) / subdominant (+5) / relative (±3) chosen by the B-region's valence and hue contrast —
//! and RETURNS home (offset 0) on the form's Perfect cadence. A subjectless field stays home
//! (byte-stable identity), so goldens cannot move.
//!
//! HEADLESS, in the same sense as composition_s15.rs / prominence_s23.rs / engine_equivalence.rs:
//! it touches NO image type, NO OpenCV, NO audio hardware. It exercises only:
//!   * the PUBLIC planner entry point `CompositionPlanner::plan(&ImageUnderstanding, &MappingTable)`
//!     driven with crafted `ImageUnderstanding` inputs (the catalogue types `KeyScheme` /
//!     `KeySchemeSection` and the planner fns `lookup_key_scheme` / `resolve_key_scheme` /
//!     `relative_offset` / `excursion_offset` are module-PRIVATE in `src/composition.rs`, so
//!     this integration net cannot — and does not — call them directly; everything is driven
//!     through `plan()` exactly as composition_s15.rs drives it),
//!   * the pure realizer `chord_engine::realize_step` over HAND-BUILT, RNG-free fixtures for the
//!     register invariant (mirroring prominence_s23.rs::no_inversion_invariant),
//!   * `mapping_loader::load_mappings` (the shipped `assets/mappings.json`) + tiny inline JSON
//!     fixtures for the loader round-trip witness,
//!   * a `sha256sum` shell-out for the §3 machine-checkable byte-freeze witness (re-baselined
//!     for S28/K3; precedent: prominence_s23.rs::engine_freeze_diff_empty, affect_s22.rs).
//!
//! RNG-BOUNDARY DISCIPLINE (same as composition_s15.rs / diversity_s13.rs): `plan()` delegates
//! per-section harmony to `pick_progression` (`thread_rng`), so chords / Roman numerals / per-step
//! `StepPlan.chord` are NON-deterministic and are NEVER asserted here. Everything this net pins is
//! RNG-INDEPENDENT: the per-section `key_offset_semitones`, the `KeyTempoPlan.key_scheme: Vec<i8>`,
//! the section roles/cadences, and (for the register invariant) the realized note pitches of a
//! hand-built single chord (no `pick_progression`).
//!
//! THE DRIVING MODEL (verified against the working-tree source, not trusted from prose):
//!   * The `aba_excursion` scheme fires when the `key_scheme` SelectTable rule matches, i.e.
//!     `fg_bg_contrast >= 0.25` (assets/mappings.json). With `fg_bg_contrast < 0.25` the scheme
//!     stays `home_only` (all-zero — the identity / byte-freeze path).
//!   * `affect_valence` on the INPUT understanding is OVERWRITTEN by the planner via
//!     `affect_composite` BEFORE the ladders run (composition.rs:934-937), so this net steers
//!     valence through its dominant input `avg_brightness` (valence weight 0.70), NOT by setting
//!     `affect_valence` directly. With the shipped weights:
//!       VALENCE = 0.70*(avg_brightness/100) + 0.20*(avg_saturation/100) + 0.10*(0.5+0.5*fg_bg_contrast)
//!     `excursion_offset` reads `affect_valence >= 0.5` as "high" (→ dominant +7 on the near path)
//!     vs "low" (→ subdominant +5 on the near path). At sat=50, fg_bg_contrast=0.25: brightness=90
//!     → valence≈0.79 (HIGH→+7); brightness=20 → valence≈0.30 (LOW→+5).
//!   * Near-vs-relative reads the circular hue distance `|subject_hue − secondary_hue|`: < 60° →
//!     near key (dominant/subdominant); >= 60° → relative (±3). `neutral()` leaves subject_hue and
//!     secondary_hue at 0.0, so a test that wants the NEAR path keeps them equal, and a test that
//!     wants the RELATIVE path sets them >= 60° apart.
//!   * `relative_offset` is mode-family-aware off `home_mode` (hue-selected via `hue_to_mode`):
//!     major/Ionian-family → −3; minor-family (Aeolian/Dorian/Phrygian/Locrian/"minor") → +3.
//!     hue 91-150 → Ionian (major → −3); hue 211-270 → Aeolian (minor → +3).
//!   * The v1 menu allowlist for any NON-ZERO offset is EXACTLY {0, +7, +5, +3, −3}.
//!
//! §5.8 CAVEAT (energy_ordered_b_region) — documented at that test: there is NO per-region
//! valence/hue field on `ImageUnderstanding`; `excursion_offset` reads the WHOLE-IMAGE
//! `affect_valence`/`secondary_hue`. The energy order (`background_energy` vs `foreground_energy`)
//! picks WHICH region is conceptually labelled B, but does NOT make the resolved offset value
//! diverge by region in v1 (design Risk 3, accepted; per-region affect is a deferred sub-slice).
//! So `energy_ordered_b_region` asserts the region-pick branch is EXERCISED and the plan stays a
//! valid in-menu offset under BOTH energy orderings — it deliberately does NOT assert the offset
//! VALUE differs between orderings (it won't in v1, and asserting that would be a false test).

use audiohax::chord_engine::{realize_step, Chord, NoteEvent, PhrasePosition, StepPlan};
use audiohax::composition::{
    CadenceStrength, CompositionPlanner, ImageUnderstanding, KeyTempoPlan, LayerProminence,
    LayerRole, OrchestrationProfile, PlanMappings, ResolutionPolicy, Section, StepContext,
    ThematicRole, ThemeVariation,
};
use audiohax::mapping_loader::{load_mappings, MappingTable};

/// The locked engine.rs witness sha256 (design §3 / §7).
/// Re-baselined for the S28/K3 slice (lead-approved, spec-s28-k3-build §3 guarantee 3): the
/// realizer pivot / common-tone modulation + land-home cadence move the engine kernel to a NEW
/// frozen byte-anchor; engine_equivalence stays byte-green (9/9). This is the new freeze witness.
const ENGINE_SHA256: &str = "e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261";

/// The v1 menu set every NON-ZERO offset must belong to (Decision 3): dominant +7,
/// subdominant +5, relative-up +3, relative-down −3. Zero (home) is always allowed.
const MENU: [i8; 4] = [7, 5, 3, -3];

// ─────────────────────────────────────────────────────────────────────────────
// Fixtures — load the SHIPPED mappings and drive the PUBLIC planner. No RNG seeding
// is needed because every asserted property is RNG-independent (see header).
// ─────────────────────────────────────────────────────────────────────────────

const MAPPINGS_PATH: &str = "assets/mappings.json";

/// The shipped `assets/mappings.json` mapping table.
fn mappings() -> MappingTable {
    load_mappings(MAPPINGS_PATH).expect("mappings.json loads")
}

/// The composition `PlanMappings` (form/key SelectTables + the S24 key_scheme_catalogue).
fn plan_mappings(m: &MappingTable) -> PlanMappings {
    m.composition
        .clone()
        .expect("composition block present in mappings.json")
        .into()
}

/// A crafted `ImageUnderstanding` for the key-plan ladders. Names exactly the discriminating
/// knobs; everything else stays at its neutral default.
///
/// * `fg_bg_contrast` — `>= 0.25` fires `aba_excursion`; `< 0.25` keeps `home_only`.
/// * `avg_brightness` — the valence driver (high brightness ⇒ high valence ⇒ dominant +7;
///   low ⇒ subdominant +5 on the near path).
/// * `dominant_hue` — selects `home_mode` via `hue_to_mode` (major vs minor family).
/// * `subject_hue` / `secondary_hue` — their circular distance picks near (< 60°) vs relative.
/// * `quadrant_contrast` — `>= 0.6` selects the `ternary_aba` form; else `rounded_binary`.
#[allow(clippy::too_many_arguments)]
fn craft(
    fg_bg_contrast: f32,
    avg_brightness: f32,
    dominant_hue: f32,
    subject_hue: f32,
    secondary_hue: f32,
    quadrant_contrast: f32,
    background_energy: f32,
    foreground_energy: f32,
) -> ImageUnderstanding {
    ImageUnderstanding {
        fg_bg_contrast,
        avg_brightness,
        avg_saturation: 50.0, // pin saturation so the valence arithmetic is stable
        dominant_hue,
        subject_hue,
        secondary_hue,
        quadrant_contrast,
        background_energy,
        foreground_energy,
        ..ImageUnderstanding::neutral()
    }
}

/// The distinct NON-ZERO offsets across a `key_scheme: Vec<i8>`, as a sorted dedup Vec.
fn distinct_nonzero(scheme: &[i8]) -> Vec<i8> {
    let mut v: Vec<i8> = scheme.iter().copied().filter(|&o| o != 0).collect();
    v.sort_unstable();
    v.dedup();
    v
}

/// True iff every offset in `scheme` is either 0 (home) or in the v1 menu allowlist.
fn all_in_menu(scheme: &[i8]) -> bool {
    scheme.iter().all(|&o| o == 0 || MENU.contains(&o))
}

// ═════════════════════════════════════════════════════════════════════════════
// §5.1 — home_only_keeps_offsets_zero (byte-freeze identity)
// ═════════════════════════════════════════════════════════════════════════════

/// §5.1: an image that does NOT fire the key_scheme rule (`fg_bg_contrast < 0.25`) resolves
/// the shipped default `home_only` scheme → EVERY section's `key_offset_semitones == 0` AND
/// `KeyTempoPlan.key_scheme` is all-zero — the byte-freeze identity (no offset moves off 0).
/// Driven through the SHIPPED `assets/mappings.json` (not a synthetic table) so it pins the
/// real default behaviour. The plumbed sha256 freeze witness lives in §5.2.
#[test]
fn home_only_keeps_offsets_zero() {
    let m = mappings();
    let planner = CompositionPlanner::new(plan_mappings(&m));
    // fg_bg_contrast 0.0 (< 0.25) ⇒ the key_scheme rule does NOT match ⇒ default home_only.
    // A few brightness/hue points so "stays home" is not an artifact of one neutral input.
    for &(bright, hue) in &[(20.0f32, 30.0f32), (55.0, 120.0), (90.0, 250.0)] {
        let u = craft(0.0, bright, hue, hue, hue, 0.0, 0.5, 0.5);
        let plan = planner.plan(&u, &m);
        for (i, s) in plan.sections.iter().enumerate() {
            assert_eq!(
                s.key_offset_semitones, 0,
                "home_only: section {i} ({:?}) must stay home (offset 0), bright {bright} hue {hue}",
                s.thematic_role
            );
        }
        assert!(
            plan.key_tempo.key_scheme.iter().all(|&o| o == 0),
            "home_only: KeyTempoPlan.key_scheme must be all-zero, got {:?} (bright {bright})",
            plan.key_tempo.key_scheme
        );
        // The spine length matches the section count (the cursor invariant the planner holds).
        assert_eq!(
            plan.key_tempo.key_scheme.len(),
            plan.sections.len(),
            "key_scheme spine length must equal the section count"
        );
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// §5.2 — engine_equivalence_byte_green (the §3 machine-checkable freeze witness)
// ═════════════════════════════════════════════════════════════════════════════

/// §5.2: the byte-freeze WITNESS for the realizer kernel, re-baselined for the S28/K3 slice
/// (lead-approved, spec-s28-k3-build §3 guarantee 3). The engine kernel was deliberately moved
/// to a NEW frozen byte-anchor by K3 (pivot/common-tone modulation + land-home cadence), so the
/// witness now pins the NEW sha; engine_equivalence stays byte-green (9/9).
///
/// The witness is the COMMIT-STATE-INDEPENDENT sha anchor: `sha256sum src/engine.rs` ==
/// `ENGINE_SHA256`. This passes on the K3-landed working tree (committed OR uncommitted) and
/// FAILS LOUDLY if engine.rs drifts off the new anchor in any future slice — a true forward
/// guard, not a no-op. (The old `git diff HEAD` sub-check was retired: HEAD predates K3, so a
/// diff-vs-HEAD witness is non-empty on the legitimate K3 tree and is not commit-state-robust;
/// the sha anchor is the cleaner freeze. engine_equivalence + the Quality Gate's own diff remain
/// secondary authorities.) If `sha256sum` is entirely unavailable the check is
/// inconclusive-but-non-failing rather than spuriously red; a readable file that mismatches the
/// anchor always fails, so a missing tool can never silently pass.
#[test]
fn engine_equivalence_byte_green() {
    use std::process::Command;

    // sha256 of the engine kernel == the locked witness (the new K3 anchor).
    match Command::new("sha256sum").arg("src/engine.rs").output() {
        Ok(out) if out.status.success() => {
            let text = String::from_utf8_lossy(&out.stdout);
            let got = text.split_whitespace().next().unwrap_or("");
            assert_eq!(
                got, ENGINE_SHA256,
                "src/engine.rs sha256 moved off the locked witness — the engine kernel drifted \
                 from the S28/K3 frozen anchor"
            );
        }
        _ => {
            eprintln!(
                "engine_equivalence_byte_green: sha256sum unavailable; deferring the engine-kernel \
                 freeze to engine_equivalence + the Quality Gate diff as authority"
            );
        }
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// §5.3 — resolves_home (the piece always ends home)
// ═════════════════════════════════════════════════════════════════════════════

/// §5.3: for EVERY form that uses `aba_excursion` (the K1 returning forms: the default
/// `rounded_binary` AND the `quadrant_contrast`-selected `ternary_aba`), with the scheme
/// FIRING (fg_bg_contrast >= 0.25), the FINAL section's `key_offset_semitones == 0` — the
/// piece always recapitulates home. Drives a high-valence firing image so B is non-zero
/// (proving the scheme really fired) while the close is still home.
#[test]
fn resolves_home() {
    let m = mappings();
    let planner = CompositionPlanner::new(plan_mappings(&m));

    // (rounded_binary via low quadrant_contrast; ternary_aba via quadrant_contrast >= 0.6).
    for &(qc, expect_form) in &[(0.0f32, "rounded_binary"), (0.7f32, "ternary_aba")] {
        // High brightness ⇒ high valence ⇒ dominant +7; small hue distance ⇒ near key.
        let u = craft(0.30, 90.0, 120.0, 120.0, 120.0, qc, 0.6, 0.4);
        let plan = planner.plan(&u, &m);
        assert_eq!(
            plan.form, expect_form,
            "the crafted image must select {expect_form} (qc {qc})"
        );

        let last = plan.sections.last().expect("a non-empty plan");
        assert_eq!(
            last.key_offset_semitones, 0,
            "{expect_form}: the FINAL section ({:?}) must end home (offset 0)",
            last.thematic_role
        );
        // The scheme genuinely FIRED (some section is non-zero) — so "ends home" is a real
        // recapitulation, not a vacuous all-home plan.
        assert!(
            plan.sections.iter().any(|s| s.key_offset_semitones != 0),
            "{expect_form}: aba_excursion must have FIRED (a non-zero B), got {:?}",
            plan.key_tempo.key_scheme
        );
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// §5.4 — home_sections_are_home (never modulate a home role)
// ═════════════════════════════════════════════════════════════════════════════

/// §5.4: in a FIRING aba_excursion plan, every section whose `thematic_role` is Statement or
/// Return carries `key_offset == 0` — the home roles never modulate (the modulation lives only
/// in the Contrast section). Asserted on both K1 forms.
#[test]
fn home_sections_are_home() {
    let m = mappings();
    let planner = CompositionPlanner::new(plan_mappings(&m));

    for &qc in &[0.0f32, 0.7f32] {
        let u = craft(0.30, 90.0, 120.0, 120.0, 120.0, qc, 0.6, 0.4);
        let plan = planner.plan(&u, &m);
        let mut home_roles_seen = 0usize;
        for (i, s) in plan.sections.iter().enumerate() {
            if matches!(
                s.thematic_role,
                ThematicRole::Statement | ThematicRole::Return
            ) {
                home_roles_seen += 1;
                assert_eq!(
                    s.key_offset_semitones, 0,
                    "section {i} role {:?} is a HOME role and must stay home (offset 0), \
                     got {} (qc {qc})",
                    s.thematic_role, s.key_offset_semitones
                );
            }
        }
        assert!(
            home_roles_seen >= 2,
            "an ABA-returning form has at least a Statement and a Return (qc {qc})"
        );
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// §5.5 — at_most_two_distinct_non_home_keys (the K2-safe cap; K1 produces exactly 1)
// ═════════════════════════════════════════════════════════════════════════════

/// §5.5: the count of DISTINCT non-zero offsets across the expanded `key_scheme: Vec<i8>` is
/// <= 2 (the cap that keeps even K2's two-excursion plan inside the "at most two journeys"
/// guard-rail). K1 produces EXACTLY 1 (one B excursion, the rest home), which is also asserted
/// here for the K1 forms — a tighter, non-gameable check than the bare <= 2.
#[test]
fn at_most_two_distinct_non_home_keys() {
    let m = mappings();
    let planner = CompositionPlanner::new(plan_mappings(&m));

    // A sweep so the cap is asserted across valence/hue/form variety, not one input.
    let cases = [
        (0.30f32, 90.0f32, 120.0f32, 120.0f32, 120.0f32, 0.0f32), // hi-valence near, rounded_binary
        (0.30, 20.0, 120.0, 120.0, 120.0, 0.0),                   // lo-valence near, rounded_binary
        (0.30, 90.0, 120.0, 120.0, 220.0, 0.7), // relative (hue dist), ternary_aba
        (0.50, 55.0, 250.0, 250.0, 250.0, 0.7), // minor home, ternary_aba
    ];
    for &(fgbg, bright, dh, sh, sec, qc) in &cases {
        let u = craft(fgbg, bright, dh, sh, sec, qc, 0.6, 0.4);
        let plan = planner.plan(&u, &m);
        let distinct = distinct_nonzero(&plan.key_tempo.key_scheme);
        assert!(
            distinct.len() <= 2,
            "at most TWO distinct non-home keys; got {distinct:?} from scheme {:?} \
             (fgbg {fgbg} bright {bright} dh {dh})",
            plan.key_tempo.key_scheme
        );
        // K1 forms produce exactly ONE non-home key (a single B excursion).
        assert_eq!(
            distinct.len(),
            1,
            "K1 (aba_excursion on a returning form) produces exactly ONE non-home key; \
             got {distinct:?} from scheme {:?}",
            plan.key_tempo.key_scheme
        );
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// §5.6 — smooth_keys_only (every non-zero offset is in the v1 menu)
// ═════════════════════════════════════════════════════════════════════════════

/// §5.6: across a SWEEP of crafted images (varying valence via brightness, hue distance via
/// subject/secondary hue, home mode via dominant hue, form via quadrant_contrast, and the
/// firing/non-firing fg_bg_contrast gate), EVERY non-zero offset is in the v1 menu allowlist
/// {+7, +5, +3, −3} — no offset escapes the closely-related set without an explicit OFF-by-
/// default opt-in (there is none in K1). A real nested sweep, not a token case.
#[test]
fn smooth_keys_only() {
    let m = mappings();
    let planner = CompositionPlanner::new(plan_mappings(&m));

    let fg_bgs = [0.0f32, 0.25, 0.6]; // below-gate (home_only) + firing
    let brights = [10.0f32, 30.0, 55.0, 85.0, 100.0]; // valence low → high
    let hues = [30.0f32, 120.0, 250.0, 320.0]; // Phrygian/Ionian/Aeolian/Mixolydian families
    let hue_dists = [0.0f32, 90.0]; // near vs strong-contrast (relative)
    let qcs = [0.0f32, 0.7]; // rounded_binary vs ternary_aba

    let mut combos = 0usize;
    let mut nonzero_seen = 0usize;
    for &fgbg in &fg_bgs {
        for &bright in &brights {
            for &hue in &hues {
                for &dist in &hue_dists {
                    for &qc in &qcs {
                        let sec_hue = (hue + dist) % 360.0;
                        let u = craft(fgbg, bright, hue, hue, sec_hue, qc, 0.6, 0.4);
                        let plan = planner.plan(&u, &m);
                        assert!(
                            all_in_menu(&plan.key_tempo.key_scheme),
                            "OFF-MENU offset: scheme {:?} has an offset outside {{0,+7,+5,+3,−3}} \
                             (fgbg {fgbg} bright {bright} hue {hue} dist {dist} qc {qc})",
                            plan.key_tempo.key_scheme
                        );
                        // Below the gate must stay all-home (the byte-stable degrade in the sweep).
                        if fgbg < 0.25 {
                            assert!(
                                plan.key_tempo.key_scheme.iter().all(|&o| o == 0),
                                "below-gate (fgbg {fgbg}) must stay home, got {:?}",
                                plan.key_tempo.key_scheme
                            );
                        } else {
                            nonzero_seen += plan
                                .key_tempo
                                .key_scheme
                                .iter()
                                .filter(|&&o| o != 0)
                                .count();
                        }
                        combos += 1;
                    }
                }
            }
        }
    }
    assert_eq!(
        combos,
        fg_bgs.len() * brights.len() * hues.len() * hue_dists.len() * qcs.len(),
        "the sweep must be the full nested cross-product"
    );
    // The sweep actually EXERCISED non-zero offsets (it is not a vacuous all-home sweep).
    assert!(
        nonzero_seen > 0,
        "the firing cases must have produced non-zero offsets to validate against the menu"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// §5.7 — contrast_actually_contrasts (B is an audible departure from A)
// ═════════════════════════════════════════════════════════════════════════════

/// §5.7: in a FIRING aba_excursion plan, the Contrast (B) section differs from its preceding
/// Statement (A) in >= 1 of {key offset, density, cadence} — so B is an audible departure, not
/// a clone of A. For a firing aba_excursion, B's offset is NON-ZERO while A's is 0, which
/// satisfies the property on the KEY-OFFSET axis; the test asserts that disjunction and reports
/// which axes actually differ.
#[test]
fn contrast_actually_contrasts() {
    let m = mappings();
    let planner = CompositionPlanner::new(plan_mappings(&m));

    for &qc in &[0.0f32, 0.7f32] {
        let u = craft(0.30, 90.0, 120.0, 120.0, 120.0, qc, 0.6, 0.4);
        let plan = planner.plan(&u, &m);

        let a = plan
            .sections
            .iter()
            .find(|s| s.thematic_role == ThematicRole::Statement)
            .expect("an A Statement");
        let b = plan
            .sections
            .iter()
            .find(|s| s.thematic_role == ThematicRole::Contrast)
            .expect("a B Contrast");

        let key_differs = a.key_offset_semitones != b.key_offset_semitones;
        let density_differs = (a.density - b.density).abs() > 1e-6;
        let cadence_differs = a.boundary_cadence != b.boundary_cadence;

        assert!(
            key_differs || density_differs || cadence_differs,
            "B must DEPART from A in >=1 of {{key, density, cadence}}: \
             A(key {}, dens {:.3}, cad {:?}) vs B(key {}, dens {:.3}, cad {:?}) (qc {qc})",
            a.key_offset_semitones,
            a.density,
            a.boundary_cadence,
            b.key_offset_semitones,
            b.density,
            b.boundary_cadence
        );
        // For K1's firing aba_excursion the KEY axis specifically carries the departure.
        assert!(
            key_differs && a.key_offset_semitones == 0 && b.key_offset_semitones != 0,
            "the K1 departure is the key move: A must be home (0) and B non-zero, \
             got A {} B {} (qc {qc})",
            a.key_offset_semitones,
            b.key_offset_semitones
        );
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// §5.8 — energy_ordered_b_region (the region-pick branch is exercised; v1 caveat)
// ═════════════════════════════════════════════════════════════════════════════

/// §5.8 — see the module-header §5.8 CAVEAT. There is NO per-region valence/hue field on
/// `ImageUnderstanding`; `excursion_offset` reads WHOLE-IMAGE `affect_valence`/`secondary_hue`.
/// The energy order (`background_energy` vs `foreground_energy`) selects WHICH region is
/// conceptually labelled B but does NOT make the offset value diverge by region in v1 (design
/// Risk 3, accepted — per-region affect is a deferred sub-slice).
///
/// THEREFORE this test asserts that the region-pick branch is EXERCISED — the plan still
/// resolves a VALID, in-menu, NON-ZERO B offset under BOTH `background_energy > foreground_energy`
/// AND the flipped inequality — and DELIBERATELY does NOT assert the offset value differs
/// between the two orderings (it will not in v1, and asserting that would be a false test). It
/// also confirms the documented v1 reality (the two orderings resolve to the SAME offset), so a
/// future per-region affect sub-slice that makes them diverge will trip this expectation and be
/// noticed — turning the accepted caveat into a tripwire rather than a silent gap.
#[test]
fn energy_ordered_b_region() {
    let m = mappings();
    let planner = CompositionPlanner::new(plan_mappings(&m));

    // A FIRING, near-key, high-valence image (so B is a clean non-zero in-menu offset).
    // Flip only the energy inequality between the two plans; everything else is held.
    let bg_dominant = craft(
        0.30, 90.0, 120.0, 120.0, 120.0, 0.0, /*bg*/ 0.8, /*fg*/ 0.2,
    );
    let fg_dominant = craft(
        0.30, 90.0, 120.0, 120.0, 120.0, 0.0, /*bg*/ 0.2, /*fg*/ 0.8,
    );

    let plan_bg = planner.plan(&bg_dominant, &m);
    let plan_fg = planner.plan(&fg_dominant, &m);

    for (label, plan) in [
        ("background>foreground", &plan_bg),
        ("foreground>background", &plan_fg),
    ] {
        // The region-pick branch resolved a VALID in-menu plan under this energy ordering.
        assert!(
            all_in_menu(&plan.key_tempo.key_scheme),
            "[{label}] resolved an off-menu offset: {:?}",
            plan.key_tempo.key_scheme
        );
        // The B (Contrast) section resolved to a NON-ZERO in-menu excursion (the branch fired).
        let b = plan
            .sections
            .iter()
            .find(|s| s.thematic_role == ThematicRole::Contrast)
            .expect("a B Contrast");
        assert!(
            b.key_offset_semitones != 0 && MENU.contains(&b.key_offset_semitones),
            "[{label}] B must resolve to a non-zero in-menu offset, got {}",
            b.key_offset_semitones
        );
    }

    // DOCUMENTED v1 reality (the §5.8 caveat as a tripwire): with no per-region affect, the two
    // energy orderings resolve to the SAME offset value. This is asserted as the EXPECTED v1
    // behaviour — NOT a divergence claim. If a later per-region-affect sub-slice makes them
    // diverge, THIS assertion fires and forces the caveat to be revisited deliberately.
    assert_eq!(
        plan_bg.key_tempo.key_scheme, plan_fg.key_tempo.key_scheme,
        "v1 caveat (design Risk 3): energy order picks the region LABEL but the resolved offset \
         value does NOT diverge by region (per-region affect is a deferred sub-slice). If this \
         fires, per-region affect landed — revisit the §5.8 caveat."
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// §5.9 — valence_direction (high → dominant +7; low → subdominant +5)
// ═════════════════════════════════════════════════════════════════════════════

/// §5.9 (Decision 4), RE-AUTHORED FOR K2a (the §5.8 caveat's anticipated landing):
///
/// K2a makes `region_excursion_offset` read the B-region's OWN per-region affect
/// (`foreground_*`/`background_*`) instead of the WHOLE-IMAGE `affect_valence`/`secondary_hue`.
/// The pre-K2a version of this test steered valence ONLY through `avg_brightness` (and left the
/// per-region fields at their `neutral()` whole-image fallback, hue 0.0). Under K2a the B region
/// (the rank-0 / more-energetic non-subject region) reads its OWN brightness as valence and its
/// OWN hue against the subject hue — so a struct that names a non-zero `subject_hue` but leaves
/// the region hue at the 0.0 default lands a spurious ≥60° hue contrast and routes to the
/// RELATIVE (−3), NOT the +7/+5 the test means to exercise. That is the intended "per-region
/// affect landed" change the suite header's §5.8 caveat predicted.
///
/// The fix speaks PER-REGION while keeping the MUSICAL intent untouched — it still validates
/// direction-from-affect (HIGH region valence → dominant +7, LOW → subdominant +5), but now the
/// valence is the B-region's own brightness and the near path is held by setting the B-region's
/// own hue equal to the subject hue (distance 0 < 60°). The `aba_excursion` scheme's B uses
/// `region_related:b` == rank 0; with `background_energy (0.6) > foreground_energy (0.4)` the
/// BACKGROUND band is rank 0, so the B region reads `background_brightness`/`background_hue`. To
/// make the test robust to the energy tiebreak we set BOTH regions' hue to the subject hue (the
/// near path holds whichever region wins rank 0) and drive ONLY the rank-0 region's brightness
/// across the high/low boundary — isolating the per-region valence axis from the
/// near-vs-relative axis exactly as the K1 version isolated whole-image valence.
#[test]
fn valence_direction() {
    let m = mappings();
    let planner = CompositionPlanner::new(plan_mappings(&m));

    // HIGH per-region valence on the rank-0 (background) B region, near key ⇒ dominant +7.
    // background_brightness 0.9 (>= 0.60 → HIGH); both region hues == subject_hue (120) → near.
    let mut hi = craft(0.30, 90.0, 120.0, 120.0, 120.0, 0.0, 0.6, 0.4);
    hi.background_brightness = 0.9; // rank-0 region's OWN valence (HIGH → +7)
    hi.foreground_brightness = 0.9; // tiebreak-robust: foreground also HIGH
    hi.background_hue = 120.0; // rank-0 region hue == subject_hue → hue dist 0 (near path)
    hi.foreground_hue = 120.0;
    let plan_hi = planner.plan(&hi, &m);
    let b_hi = plan_hi
        .sections
        .iter()
        .find(|s| s.thematic_role == ThematicRole::Contrast)
        .expect("a B Contrast")
        .key_offset_semitones;
    assert_eq!(
        b_hi, 7,
        "HIGH per-region valence (rank-0 region brightness) on the near path must go to the \
         DOMINANT (+7); got {b_hi} (scheme {:?})",
        plan_hi.key_tempo.key_scheme
    );

    // LOW per-region valence on the rank-0 (background) B region, near key ⇒ subdominant +5.
    // background_brightness 0.1 (<= 0.40 → LOW); hues still == subject_hue (near path held).
    let mut lo = craft(0.30, 20.0, 120.0, 120.0, 120.0, 0.0, 0.6, 0.4);
    lo.background_brightness = 0.1; // rank-0 region's OWN valence (LOW → +5)
    lo.foreground_brightness = 0.1; // tiebreak-robust: foreground also LOW
    lo.background_hue = 120.0; // near path held
    lo.foreground_hue = 120.0;
    let plan_lo = planner.plan(&lo, &m);
    let b_lo = plan_lo
        .sections
        .iter()
        .find(|s| s.thematic_role == ThematicRole::Contrast)
        .expect("a B Contrast")
        .key_offset_semitones;
    assert_eq!(
        b_lo, 5,
        "LOW per-region valence (rank-0 region brightness) on the near path must go to the \
         SUBDOMINANT (+5); got {b_lo} (scheme {:?})",
        plan_lo.key_tempo.key_scheme
    );

    // The two genuinely DIVERGE on the per-region valence axis (the direction read is live, not
    // constant) — the same musical property the K1 version asserted, now read per-region.
    assert_ne!(
        b_hi, b_lo,
        "per-region valence must STEER the B direction (+7 vs +5), not collapse to one key"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// §5.10 — no_inversion_invariant (the hard register guard, re-run across new offsets)
// ═════════════════════════════════════════════════════════════════════════════

/// §5.10 (the design §3 register invariant, re-run because pitch CLASSES move under K1):
/// across ALL menu offsets {0, +7, +5, +3, −3} × a range of characters (realized as their
/// register-driving brightness, mirroring prominence_s23.rs::no_inversion_invariant) × both
/// bed kinds × busy/calm edges × interior positions, on the SAME hand-built C-major step:
///   - `mean_pitch(Bass) < mean_pitch(bed) < mean_pitch(Melody)` (figure-ground never inverts),
///   - EVERY emitted note ∈ 24..=108 (the engine playable band).
/// The key offset is fed exactly as the planner feeds it — `Section.key_offset_semitones` →
/// `StepContext` → `chord_engine.rs:2105` `tonic_pc = (home_root_midi + offset).rem_euclid(12)`
/// — so a non-zero offset rotates the tonal center as a uniform PRE-voice-leading pitch-CLASS
/// shift, which (the design §3 claim) cannot move a voice's register. This net is the witness
/// that the claim holds under EVERY new menu offset, not just the frozen 0. Hand-built fixtures,
/// RNG-free (no `pick_progression`), mirroring the prominence_s23 harness.
#[test]
fn no_inversion_invariant() {
    // The full K1 menu PLUS the frozen home (0).
    let offsets = [0i8, 7, 5, 3, -3];
    // The affect-ladder characters mapped to brightness (their only register driver in the
    // per-step realizer), spanning dark→bright so the bright_octaves lift goes floor→max.
    let brightnesses = [
        12.0f32, // Lament — darkest
        30.0,    // Nocturne
        55.0,    // Ballad (legacy default)
        78.0,    // March
        100.0,   // Scherzo — brightest, max lift (the stack-risk)
    ];
    let beds = [LayerRole::HarmonicFill, LayerRole::Pad];
    let edges = [0.30f32, 0.90]; // calm vs busy (busy arpeggiates the melody — multi-onset)
    let positions = [2usize, 3, 5];

    let mut combos = 0usize;
    for &offset in &offsets {
        for &brightness in &brightnesses {
            for &bed in &beds {
                for &edge in &edges {
                    for &pos in &positions {
                        let step = interior_step(pos);
                        let f = perf(brightness, edge);
                        // A non-identity trio [Bass, bed, Melody] with the subject_melody-style
                        // prominence (the strongest figure-ground stress on the invariant), and
                        // the section carrying the SWEPT key offset — fed through the same ctx
                        // path the planner uses.
                        let profile = trio(bed, subject_prominence());
                        let bass = realize_offset(&profile, &step, 0, 3, &f, offset);
                        let bed_ev = realize_offset(&profile, &step, 1, 3, &f, offset);
                        let melody = realize_offset(&profile, &step, 2, 3, &f, offset);

                        for (who, evs) in [("bass", &bass), ("bed", &bed_ev), ("melody", &melody)] {
                            for e in evs.iter() {
                                assert!(
                                    (24..=108).contains(&e.note),
                                    "note {} out of band 24..=108 ({who}, offset {offset}, \
                                     bed {bed:?}, bright {brightness}, edge {edge}, pos {pos})",
                                    e.note
                                );
                            }
                        }

                        let b = mean_pitch(&bass);
                        let mid = mean_pitch(&bed_ev);
                        let t = mean_pitch(&melody);
                        assert!(
                            b < mid,
                            "INVERSION: Bass {b:.1} not < bed {mid:.1} (offset {offset}, \
                             bed {bed:?}, bright {brightness}, edge {edge}, pos {pos})"
                        );
                        assert!(
                            mid < t,
                            "INVERSION: bed {mid:.1} not < Melody {t:.1} (offset {offset}, \
                             bed {bed:?}, bright {brightness}, edge {edge}, pos {pos})"
                        );
                        combos += 1;
                    }
                }
            }
        }
    }
    eprintln!("[5.10] swept {combos} (offset×bright×bed×edge×pos) register-invariant combos");
    assert_eq!(
        combos,
        offsets.len() * brightnesses.len() * beds.len() * edges.len() * positions.len(),
        "the no-inversion sweep must be the full nested cross-product over the NEW offsets"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// §5.11 — key_scheme_catalogue_round_trips (the mirror witness, §2.6)
// ═════════════════════════════════════════════════════════════════════════════

/// §5.11 (the mirror-risk witness, design §2.6 / Risk 6): the new `key_scheme_catalogue` field
/// must land on `CompositionMappings` + the `From<CompositionMappings> for PlanMappings` arm,
/// or the catalogue is silently DROPPED at load and every scheme degrades to `home_only`. The
/// §5.1 `home_only` test passes EITHER way (both paths resolve all-zero), so it is
/// necessary-but-NOT-sufficient and cannot catch a missing mirror. This test is the dedicated
/// END-TO-END witness: it LOADS the SHIPPED `assets/mappings.json` — which carries the real
/// POPULATED `key_scheme_catalogue` (home_only + aba_excursion + abac_rondo) and the FIRING
/// `key_scheme` rule — through the SAME `load_mappings` → `PlanMappings::from` path the planner
/// uses, then asserts (a) the catalogue SURVIVED the loader mirror (the `aba_excursion` id is
/// present on the loaded `PlanMappings.key_scheme_catalogue`), and (b) a FIRING image resolves
/// that non-`home_only` scheme to a NON-ZERO offset. Both are reachable ONLY if the
/// `CompositionMappings` field + the `From` arm carried the catalogue across the load; if the
/// mirror were missing, the loaded catalogue would be EMPTY and the firing image would resolve
/// all-zero (home_only) — so this test bites exactly where §5.1 cannot.
///
/// Loading the SHIPPED file (rather than a hand-built minimal one) keeps the witness
/// non-brittle: the `global` block has several required, non-`#[serde(default)]` fields
/// (`saturation_to_harmonic_complexity`, `feature_normalization`, …), so a stripped-down inline
/// JSON would track the schema and rot; the shipped file is the authoritative populated
/// catalogue and is exactly what the production planner loads.
#[test]
fn key_scheme_catalogue_round_trips() {
    let m = mappings(); // load_mappings("assets/mappings.json")
    let pm = plan_mappings(&m); // CompositionMappings -> PlanMappings (the mirror path)

    // (a) The catalogue SURVIVED the loader mirror: the firing scheme id is present and
    //     populated. If the `From<CompositionMappings>` arm (or the CompositionMappings field)
    //     were missing, this Vec would be empty (the silent-drop failure mode Risk 6 names).
    assert!(
        !pm.key_scheme_catalogue.is_empty(),
        "MIRROR MISSING: the shipped key_scheme_catalogue is empty after load — the \
         CompositionMappings field or the From<CompositionMappings> arm dropped it"
    );
    let aba = pm
        .key_scheme_catalogue
        .iter()
        .find(|k| k.id == "aba_excursion")
        .expect("MIRROR: the shipped catalogue must carry the firing `aba_excursion` scheme");
    assert!(
        aba.sections
            .iter()
            .any(|s| s.offset_rule.starts_with("region_related")),
        "the round-tripped aba_excursion must retain its region_related B rule, got {:?}",
        aba.sections
    );

    // (b) A FIRING, high-valence, near-key image resolves that non-home_only scheme to a
    //     NON-ZERO offset — only possible if the catalogue survived the load→From→planner path.
    let planner = CompositionPlanner::new(pm);
    let u = craft(0.30, 90.0, 120.0, 120.0, 120.0, 0.0, 0.6, 0.4);
    let plan = planner.plan(&u, &m);
    assert!(
        plan.key_tempo.key_scheme.iter().any(|&o| o != 0),
        "MIRROR MISSING: a populated key_scheme_catalogue + firing rule must resolve a \
         NON-ZERO offset, but the scheme is {:?} (the catalogue was dropped at load — the \
         home_only test in §5.1 cannot catch this)",
        plan.key_tempo.key_scheme
    );
    // And the resolved non-zero offset is itself a valid menu entry (no garbage round-trip).
    assert!(
        all_in_menu(&plan.key_tempo.key_scheme),
        "the round-tripped scheme must be in-menu, got {:?}",
        plan.key_tempo.key_scheme
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Hand-built realizer fixtures for §5.10 (mirror tests/prominence_s23.rs 1:1).
// ─────────────────────────────────────────────────────────────────────────────

const MS_PER_STEP: u64 = 200;

/// The pinned chord: a C-major triad in root position (pcs 0,4,7).
fn c_major() -> Chord {
    Chord {
        name: "I".to_string(),
        notes: vec![60, 64, 67], // C4 E4 G4
    }
}

/// A NON-cadence interior step — so the Melody / bed arms (not the cadence early-return) fire.
fn interior_step(position_in_phrase: usize) -> StepPlan {
    StepPlan {
        chord: c_major(),
        phrase_index: 0,
        position_in_phrase,
        phrase_len: 8,
        position: PhrasePosition::Interior,
        velocity: 80,
    }
}

/// PerfFeatures with a chosen brightness (the register/`bright_octaves` driver) and edge (the
/// rhythm/onset driver). Saturation mid so velocity stays in band.
fn perf(brightness: f32, edge_density: f32) -> audiohax::chord_engine::PerfFeatures {
    audiohax::chord_engine::PerfFeatures {
        saturation: 60.0,
        brightness,
        edge_density,
    }
}

/// A non-identity 3-layer ensemble [Bass, <bed>, Melody] (inst 0/1/2) with an explicit per-role
/// prominence Vec. `pad_voices: 0` keeps the bed a single fill voice (block bed). Mirrors
/// prominence_s23.rs::trio.
fn trio(bed: LayerRole, prominence: Vec<LayerProminence>) -> OrchestrationProfile {
    OrchestrationProfile {
        id: "keyplan_trio".to_string(),
        layers: vec![LayerRole::Bass, bed, LayerRole::Melody],
        density: 0.5,
        pad_voices: 0,
        figuration: None,
        figuration_resolved: None,
        prominence,
    }
}

/// The shipped `subject_melody`-style prominence (Melody 1.0 / fill 0.3 / Pad 0.3 / Bass 0.5):
/// the strongest figure-ground stress on the register invariant. Mirrors prominence_s23.rs.
fn subject_prominence() -> Vec<LayerProminence> {
    vec![
        LayerProminence {
            role: LayerRole::Melody,
            weight: 1.0,
        },
        LayerProminence {
            role: LayerRole::HarmonicFill,
            weight: 0.3,
        },
        LayerProminence {
            role: LayerRole::Pad,
            weight: 0.3,
        },
        LayerProminence {
            role: LayerRole::Bass,
            weight: 0.5,
        },
    ]
}

/// Build a Section carrying a given orchestration profile AND a given key offset around a
/// one-step plan, then realize one instrument on that step through the planner's `ctx` path
/// (`StepContext::single_section_default` → `chord_engine.rs:2105` transpose seam). This is the
/// exact path `Section.key_offset_semitones` reaches the realizer, so the offset is exercised
/// as the planner would feed it. Mirrors prominence_s23.rs::realize_under, extended with the
/// swept `key_offset_semitones`.
fn realize_offset(
    profile: &OrchestrationProfile,
    step: &StepPlan,
    inst_idx: usize,
    num_instruments: usize,
    features: &audiohax::chord_engine::PerfFeatures,
    key_offset_semitones: i8,
) -> Vec<NoteEvent> {
    let section = Section {
        label: "A".to_string(),
        step_len: 1,
        thematic_role: ThematicRole::Statement,
        key_offset_semitones,
        ms_per_step: MS_PER_STEP,
        mode: "Ionian".to_string(),
        progression: vec![],
        theme: None,
        variation: ThemeVariation::Identity,
        boundary_cadence: CadenceStrength::Perfect,
        // K3 identity carry: keep this fixture on the byte-frozen non-modulating path.
        pivot: false,
        resolution: ResolutionPolicy::Resolve,
        density: 0.5,
        orchestration: profile.clone(),
        steps: vec![step.clone()],
    };
    let kt = KeyTempoPlan {
        home_root_midi: 60,
        home_mode: "Ionian".to_string(),
        base_ms_per_step: MS_PER_STEP,
        key_scheme: vec![key_offset_semitones],
        tempo_scheme: vec![MS_PER_STEP],
    };
    let ctx = StepContext::single_section_default(&section, &kt);
    realize_step(step, inst_idx, num_instruments, features, MS_PER_STEP, &ctx)
}

/// Mean pitch over a role's emitted NoteEvents for one step. Mirrors prominence_s23.rs.
fn mean_pitch(events: &[NoteEvent]) -> f64 {
    assert!(
        !events.is_empty(),
        "role emitted no events to average pitch"
    );
    events.iter().map(|e| e.note as f64).sum::<f64>() / events.len() as f64
}

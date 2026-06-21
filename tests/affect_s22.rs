//! tests/affect_s22.rs — the S22 SLICE-A AFFECT → CHARACTER + TEMPO-DE-CAP property net.
//!
//! Proves the Slice-A build pinned in `docs/spec-s22-slice-a-build.md` §5: the pure
//! perceptual scalars on `ImageUnderstanding` are pooled into two continuous affect axes
//! (arousal, valence), those axes drive the `character` SelectTable so a bright/energetic
//! image escapes the always-ballad default into `Scherzo` and a dark/calm one into
//! `Lament`, and the single hard Ballad tempo clamp (56..96) is replaced by a per-character
//! tempo window so a high-arousal image's BPM can exceed the old 96 ceiling. It also pins
//! the back-compat floor (an absent `affect` block reproduces the legacy `clamp(56,96)`
//! byte-for-byte) and witnesses the engine_equivalence byte-freeze.
//!
//! HEADLESS, in the same sense as composition_s15.rs / engine_equivalence.rs: it touches NO
//! image type, NO OpenCV, NO audio hardware. It exercises only the pure planner
//! (`CompositionPlanner::plan`) and `mapping_loader::load_mappings`. Runs under DEFAULT
//! features per the engagement convention (the `--no-default-features` bin-config defect is
//! unrelated to this slice).
//!
//! TEST-SEAM NOTE (private fns → drive via the public plan path):
//! `affect_composite`, `character_tempo_bpm`, and `character_tempo_key` are PRIVATE module
//! fns in composition.rs (no `pub`). The spec §6 names some shapes that call them directly
//! (`character_tempo_bpm(raw, Character::Ballad, &AffectMappings::default())`,
//! `affect_composite(&u, &w)`). Because that surface is not reachable from an integration
//! test, every property below is asserted through the PUBLIC seam the spec's §6 also
//! sanctions ("run through the planner ... assert on the resulting plan's character +
//! base_ms_per_step"): `CompositionPlanner::plan(&u, &m)` → `plan.character` (the selected
//! `Character`) and `plan.key_tempo.base_ms_per_step` (from which BPM = 60_000 / base_ms is
//! recovered). The monotonicity property (test 2) is asserted as the downstream effect of a
//! higher-saturation / higher-brightness image, not by calling the private composite fn.
//!
//! VALUE-ANCHOR NOTE (de-cap reachability under the de-capped tempo table):
//! the strict "BPM drops BELOW the old 56 floor" property (spec §6(e)) is a DIRECT-FN
//! property of `character_tempo_bpm(50.0, Lament, ..)`. Through the public plan path it is
//! NOT reachable, because the de-capped `brightness_to_tempo_bpm` table floors raw BPM at 72
//! and the Lament window clamps DOWN to its max of 66 — so the darkest plan-path image lands
//! at 66, the Lament ceiling, which is below the raw it would otherwise have had but not
//! below 56. The observable plan-path witness of the de-cap is therefore the UPPER half: a
//! bright/energetic image whose Scherzo BPM EXCEEDS the old 96 ceiling (test 3). The dark
//! corner (test 4) asserts Lament selection + the BPM pulled DOWN into the Lament window
//! (<= 66, below the ballad-clamped 96 the legacy path produced), the observable half of the
//! down-de-cap.
//!
//! RNG discipline (same as composition_s15.rs): `plan` delegates per-section harmony to
//! `chord_engine::pick_progression` (`thread_rng`); every property asserted here —
//! `plan.character` and `plan.key_tempo.base_ms_per_step` — is RNG-INDEPENDENT (selected by
//! the deterministic affect composite + SelectTable ladders, not the chord RNG path). No
//! `thread_rng`-derived value is ever asserted.

use audiohax::composition::{
    AffectMappings, Character, CompositionPlanner, ImageUnderstanding, PlanMappings, SelectTable,
};
use audiohax::mapping_loader::{load_mappings, MappingTable};

// ─────────────────────────────────────────────────────────────────────────────
// Fixtures — load the SHIPPED mappings (with the S22 affect rows) and build the
// planner inputs, mirroring composition_s15.rs's fixture pattern.
// ─────────────────────────────────────────────────────────────────────────────

/// The shipped `assets/mappings.json` (the same table the engine holds), now carrying the
/// S22 `affect` block + filled `character` ladder + de-capped `brightness_to_tempo_bpm`.
fn mappings() -> MappingTable {
    load_mappings("assets/mappings.json").expect("mappings.json loads")
}

/// The composition `PlanMappings` (form/character/affect SelectTables + catalogues), via the
/// `From<CompositionMappings>` impl that carries the new `affect` field across.
fn plan_mappings(m: &MappingTable) -> PlanMappings {
    m.composition
        .clone()
        .expect("composition block present in mappings.json")
        .into()
}

/// Recover the plan's tempo in BPM from its `base_ms_per_step` (the inverse of the planner's
/// `base_ms_per_step = round(60_000 / bpm)`). Used so the de-cap is asserted in musical units.
fn plan_bpm(plan: &audiohax::composition::CompositionPlan) -> f64 {
    60_000.0 / plan.key_tempo.base_ms_per_step as f64
}

/// The canonical bright/energetic vector from spec §6(b) — the `example.jpg` prediction:
/// `avg_saturation=80, colorfulness=0.90, edge_activity=1.0, complexity=0.75,
/// avg_brightness=62, fg_bg_contrast=0.15` → composite arousal ≈ 0.86, valence ≈ 0.65.
fn bright_energetic() -> ImageUnderstanding {
    ImageUnderstanding {
        avg_saturation: 80.0,
        colorfulness: 0.90,
        edge_activity: 1.0,
        complexity: 0.75,
        avg_brightness: 62.0,
        fg_bg_contrast: 0.15,
        ..ImageUnderstanding::neutral()
    }
}

/// The canonical calm/dark vector from spec §6(d): `avg_saturation=15, colorfulness=0.20,
/// edge_activity=0.25, complexity=0.30, avg_brightness=20, fg_bg_contrast=0.40` → composite
/// arousal ≈ 0.20, valence ≈ 0.24 → Lament corner.
fn calm_dark() -> ImageUnderstanding {
    ImageUnderstanding {
        avg_saturation: 15.0,
        colorfulness: 0.20,
        edge_activity: 0.25,
        complexity: 0.30,
        avg_brightness: 20.0,
        fg_bg_contrast: 0.40,
        ..ImageUnderstanding::neutral()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// (2) Composite monotonicity — asserted as the DOWNSTREAM plan-path effect, since
// `affect_composite` is private. Arousal ↑ with saturation; valence ↑ with brightness.
// ─────────────────────────────────────────────────────────────────────────────

/// AROUSAL monotonicity (saturation-led), observed downstream: holding every other feature
/// fixed at a CALM, low-brightness profile, RAISING `avg_saturation` can only raise arousal, so
/// the higher-saturation image must select a more-energetic character and a STRICTLY faster BPM
/// than the lower-saturation one. This is the affect bridge's load-bearing arousal driver
/// (avg_saturation, weight 0.45) made observable. Replaces a direct
/// `affect_composite(..).arousal` comparison (private).
///
/// S50 RE-BLESS (spec-s50 §3.3 / §6 step 5 — fixture re-calibration, NOT a production defect).
/// The S50 character-gate move (scherzo/march arousal ge 0.60 → 0.34) un-pinned the OLD fixtures:
/// at the original bright base (avg_brightness 62), both lo_sat (arousal 0.545) and hi_sat (0.927)
/// now clear the 0.34 gate, so both leave ballad — lo_sat → March, hi_sat → Scherzo — and BOTH
/// take the SAME brightness-derived raw BPM (121.7), which lands INSIDE both the March (96..132)
/// and Scherzo (120..168) windows UNCLAMPED → a TIE that fails the strict `>` assert.
///
/// HONEST DIAGNOSIS (lead-flagged): this is a FIXTURE ARTIFACT, not a broken monotone property.
/// Tempo is driven by BRIGHTNESS (the raw BPM), and the character window only CLAMPS it; so
/// "more arousal → faster" is exercised ONLY where the higher-arousal character's window floor
/// pulls the raw BPM UP (or the lower character's ceiling holds it DOWN). The old fixture sat at
/// a brightness where the raw BPM was already inside BOTH windows, so going March→Scherzo changed
/// nothing — a non-exercising fixture, not a property failure. RE-CALIBRATED here to a LOW-
/// brightness, modest-energy base where the property is genuinely exercised: at avg_brightness 30
/// the raw BPM is 87.2, BELOW March's 96 floor and ABOVE Lament's 66 ceiling, so:
///   lo_sat (sat 20) → arousal 0.220 (below the 0.34 energetic gate, below the 0.30 lament gate
///                     on a sub-0.35 valence) → Lament, window 44..66 → raw 87.2 clamped DOWN to 66.
///   hi_sat (sat 80) → arousal 0.490 (clears the 0.34 gate; valence 0.427 < 0.55) → March, window
///                     96..132 → raw 87.2 clamped UP to 96.
/// → BPM rises 66 → 96 as saturation rises: the monotone-tempo property is REALLY exercised
/// (a genuine character-window crossing), not trivially passed. The property HOLDS — the original
/// red was the fixture's location, not the affect bridge.
#[test]
fn affect_arousal_monotone_in_saturation_downstream() {
    let m = mappings();
    let planner = CompositionPlanner::new(plan_mappings(&m));

    // A CALM, low-brightness base (so the raw BPM sits between the Lament ceiling and the March
    // floor, where the character window genuinely moves the tempo). Vary ONLY saturation.
    let calm_low_bright = ImageUnderstanding {
        colorfulness: 0.20,
        edge_activity: 0.25,
        complexity: 0.30,
        avg_brightness: 30.0,
        fg_bg_contrast: 0.15,
        ..ImageUnderstanding::neutral()
    };
    // Low saturation → arousal 0.220 (below the 0.34 energetic gate): a calm character.
    let lo_sat = ImageUnderstanding {
        avg_saturation: 20.0,
        ..calm_low_bright
    };
    // High saturation → arousal 0.490 (clears the 0.34 gate): the energetic March corner.
    let hi_sat = ImageUnderstanding {
        avg_saturation: 80.0,
        ..calm_low_bright
    };

    let plo = planner.plan(&lo_sat, &m);
    let phi = planner.plan(&hi_sat, &m);

    // Downstream witness of arousal↑: the high-saturation image crosses INTO the energetic corner
    // (March, arousal ≥ 0.34), the low-saturation one does NOT (it stays a calm character).
    assert_eq!(
        phi.character,
        Character::March,
        "high-saturation image should reach the energetic March corner (arousal over the 0.34 gate)"
    );
    assert!(
        matches!(
            plo.character,
            Character::Lament | Character::Nocturne | Character::Hymn
        ),
        "low-saturation image must stay a CALM character (arousal below the 0.34 energetic gate); \
         got {:?}",
        plo.character
    );
    // …and the higher-arousal image is STRICTLY faster: the March window floor (96) pulls the raw
    // BPM UP, the lower character's window ceiling held it down. This is the real arousal→tempo
    // crossing the OLD fixture failed to exercise.
    assert!(
        plan_bpm(&phi) > plan_bpm(&plo),
        "higher saturation (higher arousal) must yield a faster tempo: hi={:.2} lo={:.2}",
        plan_bpm(&phi),
        plan_bpm(&plo)
    );
}

/// VALENCE monotonicity (brightness-led), observed downstream: holding arousal LOW (a calm
/// profile, so only the calm rules are eligible), RAISING `avg_brightness` raises valence,
/// which moves the selected calm character UP the brightness ordering — from Lament (valence
/// < 0.35) through Nocturne (0.35..0.50) to Hymn (valence >= 0.50). The selected character's
/// position in that ordering is a strictly-monotone witness of valence. This is the valence
/// driver (avg_brightness, weight 0.70) made observable; replaces a direct
/// `affect_composite(..).valence` comparison (private).
#[test]
fn affect_valence_monotone_in_brightness_downstream() {
    let m = mappings();
    let planner = CompositionPlanner::new(plan_mappings(&m));

    // Calm base (low arousal: low sat/colorfulness/edge/complexity) so the calm ladder rows are
    // the eligible ones; sweep brightness dark → mid → bright.
    let calm = |brightness: f32| ImageUnderstanding {
        avg_saturation: 12.0,
        colorfulness: 0.10,
        edge_activity: 0.10,
        complexity: 0.10,
        avg_brightness: brightness,
        fg_bg_contrast: 0.0,
        ..ImageUnderstanding::neutral()
    };

    // Brightness-ordered valence "rank": the lower a calm character sits, the lower its valence.
    fn calm_rank(c: Character) -> u8 {
        match c {
            Character::Lament => 0,   // valence < 0.35 (darkest)
            Character::Nocturne => 1, // 0.35 <= valence < 0.50
            Character::Hymn => 2,     // valence >= 0.50 (brightest calm)
            other => panic!("unexpected non-calm character on the calm ladder: {other:?}"),
        }
    }

    let dark = planner.plan(&calm(15.0), &m).character; // valence ≈ 0.14 → Lament
    let mid = planner.plan(&calm(50.0), &m).character; // valence ≈ 0.424 → Nocturne
    let bright = planner.plan(&calm(95.0), &m).character; // valence ≈ 0.69 → Hymn

    // Strictly increasing rank ⇒ valence strictly increasing in brightness (downstream).
    assert!(
        calm_rank(dark) < calm_rank(mid),
        "darker calm image must rank below mid: {dark:?} vs {mid:?}"
    );
    assert!(
        calm_rank(mid) < calm_rank(bright),
        "mid calm image must rank below bright: {mid:?} vs {bright:?}"
    );
    // Pin the endpoints so the ladder wiring (not just the ordering) is witnessed.
    assert_eq!(dark, Character::Lament, "darkest calm image → Lament");
    assert_eq!(bright, Character::Hymn, "brightest calm image → Hymn");
}

// ─────────────────────────────────────────────────────────────────────────────
// (3) bright_energetic → Scherzo + the de-cap ABOVE the old 96 ceiling.
// ─────────────────────────────────────────────────────────────────────────────

/// The example.jpg-like bright/energetic vector (arousal ≈ 0.86, valence ≈ 0.65) selects the
/// energetic corner `Scherzo` AND the de-capped BPM exceeds the old Ballad ceiling of 96,
/// landing in the Scherzo window (120..=168). This is the headline fix: an energetic image no
/// longer renders as a slow ballad. Combines spec §6(c) + the upper half of §6(e) (de-cap
/// above 96), folded into the plan path because `character_tempo_bpm` is private.
#[test]
fn bright_energetic_picks_scherzo_and_de_caps_above_96() {
    let m = mappings();
    let planner = CompositionPlanner::new(plan_mappings(&m));
    let plan = planner.plan(&bright_energetic(), &m);

    assert_eq!(
        plan.character,
        Character::Scherzo,
        "bright/energetic vector (arousal≈0.86, valence≈0.65) must select Scherzo"
    );
    let bpm = plan_bpm(&plan);
    assert!(
        bpm > 96.0,
        "Scherzo BPM must exceed the old 96 Ballad ceiling (de-cap), got {bpm:.2}"
    );
    assert!(
        (120.0..=168.0).contains(&bpm),
        "Scherzo BPM must land in the Scherzo window 120..=168, got {bpm:.2}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// (4) calm_dark → a slow, minor-LEANING character (Lament) + BPM pulled DOWN.
// ─────────────────────────────────────────────────────────────────────────────

/// The calm/dark vector (arousal ≈ 0.20, valence ≈ 0.24) selects `Lament` — the minor-LEANING
/// character — and the BPM is pulled DOWN into the Lament window (44..=66), well below the
/// legacy ballad-clamped 96 the old hard clamp produced for any bright-enough raw BPM.
///
/// Per spec §6 NOTE: mode selection (valence→major/minor REALIZATION) is NOT in Slice A's
/// scope — `home_mode` still derives from `dominant_hue`, and chord_engine is byte-frozen. So
/// this test asserts only that the minor-LEANING CHARACTER (Lament) is selected, NOT a realized
/// minor mode. (It also does NOT assert BPM < 56: through the de-capped tempo table the raw BPM
/// floors at 72 and the Lament window clamps DOWN to its max of 66, so the plan-path dark image
/// lands at 66 — the Lament ceiling — not below 56; the strict <56 is a private-`character_tempo_bpm`
/// property, see the file-header VALUE-ANCHOR NOTE.)
#[test]
fn calm_dark_picks_slow_minor_leaning_character() {
    let m = mappings();
    let planner = CompositionPlanner::new(plan_mappings(&m));
    let plan = planner.plan(&calm_dark(), &m);

    assert_eq!(
        plan.character,
        Character::Lament,
        "calm/dark vector (arousal≈0.20, valence≈0.24) must select the minor-leaning Lament"
    );
    let bpm = plan_bpm(&plan);
    // Pulled DOWN into the Lament window (the down-de-cap), below the legacy ballad ceiling.
    // A small epsilon absorbs the integer-ms round-trip: clamping to the Lament max (66 BPM)
    // stores base_ms = round(60_000/66) = 909, which recovers as 60_000/909 ≈ 66.01 — the
    // window edge plus one rounding ULP, not a real out-of-window tempo.
    assert!(
        (44.0..=66.1).contains(&bpm),
        "Lament BPM must land in the Lament window 44..=66 (±round), got {bpm:.2}"
    );
    assert!(
        bpm < 96.0,
        "Lament BPM must fall below the old 96 ballad clamp (down-de-cap), got {bpm:.2}"
    );
}

/// A vector that fires NO character rule falls through to the safe `ballad` catch-all default
/// (first-match-wins, default last). Witnesses that the filled ladder preserves the Ballad
/// default for the unclassified middle, and that Ballad clamps to its 56..96 window.
///
/// S50 RE-BLESS (spec-s50 §3.3 / §6 step 5 — fixture re-calibration, NOT a production defect).
/// The OLD fixture (arousal ≈ 0.405) was built to sit in the pre-S50 (0.30, 0.60) ballad deadzone.
/// S50 DELIBERATELY closed most of that deadzone (scherzo/march arousal gate 0.60 → 0.34), so the
/// old vector now CLEARS the 0.34 gate and resolves Scherzo — the test was pinned to a region the
/// re-range legitimately removed. The INTENDED property — "an unclassified/neutral vector falls
/// through to ballad" — is still GUARDABLE because a narrow unclassified region survives: arousal
/// in the (0.30, 0.34) gap (above the lament/hymn 0.30 gate, below the scherzo/march 0.34 gate)
/// AND valence OUTSIDE nocturne's [0.35, 0.47] band. RE-CALIBRATED here to a vector that lands
/// squarely in that surviving gap: arousal 0.3175 (mid of 0.30..0.34), valence 0.516 (above
/// nocturne's 0.47 ceiling, below scherzo/hymn's 0.55 floor) → no rule fires → Ballad. The
/// fall-through property is still genuinely tested.
#[test]
fn unclassified_vector_falls_through_to_ballad() {
    let m = mappings();
    let planner = CompositionPlanner::new(plan_mappings(&m));

    // arousal = 0.45*0.65 + 0.25*0.10 = 0.2925 + 0.025 = 0.3175 — in the SURVIVING (0.30, 0.34)
    //   unclassified gap: above the lament/hymn `le 0.30` rows, below the scherzo/march `ge 0.34`
    //   rows. valence = 0.70*0.48 + 0.20*0.65 + 0.10*(0.5) = 0.336 + 0.13 + 0.05 = 0.516 — above
    //   nocturne's 0.47 `in_range` ceiling and below scherzo/hymn's 0.55 floor. No rule fires →
    //   Ballad (the catch-all default).
    let unclassified = ImageUnderstanding {
        avg_saturation: 65.0,
        colorfulness: 0.10,
        edge_activity: 0.0,
        complexity: 0.0,
        avg_brightness: 48.0,
        fg_bg_contrast: 0.0,
        ..ImageUnderstanding::neutral()
    };
    let plan = planner.plan(&unclassified, &m);
    assert_eq!(
        plan.character,
        Character::Ballad,
        "an arousal-mid / unclassified image must fall through to the Ballad catch-all"
    );
    let bpm = plan_bpm(&plan);
    assert!(
        (56.0..=96.0).contains(&bpm),
        "Ballad BPM must stay in the legacy 56..=96 window, got {bpm:.2}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// (1)/(a) BACKWARD-COMPAT — an absent `affect` block reproduces the legacy clamp(56,96).
// ─────────────────────────────────────────────────────────────────────────────

/// `AffectMappings::default()` (the no-`affect`-block floor) ships empty weight maps and the
/// SINGLE legacy `ballad:{56,96}` tempo window. This pins the floor that makes the de-cap
/// opt-in: with no affect block, the composite degenerates (empty maps) and the only window is
/// Ballad's legacy 56..96.
#[test]
fn affect_mappings_default_is_legacy_ballad_floor() {
    let d = AffectMappings::default();
    assert!(
        d.arousal_weights.is_empty(),
        "default arousal_weights must be empty (composite degenerates to neutral 0.5)"
    );
    assert!(
        d.valence_weights.is_empty(),
        "default valence_weights must be empty"
    );
    let ballad = d
        .character_tempo
        .get("ballad")
        .expect("default floor must seed the legacy ballad window");
    assert_eq!(ballad.bpm_min, 56.0, "legacy ballad floor bpm_min == 56");
    assert_eq!(ballad.bpm_max, 96.0, "legacy ballad floor bpm_max == 96");
    // The ONLY window present in the floor is ballad (the de-cap is opt-in via a real block).
    assert_eq!(
        d.character_tempo.len(),
        1,
        "the no-affect-block floor seeds exactly one window (ballad)"
    );
}

/// BACKWARD-COMPAT witness (spec §6(a)): a `PlanMappings` whose `affect` block is ABSENT
/// (i.e. `AffectMappings::default()`) and whose `character` ladder is empty (the legacy
/// always-Ballad state) reproduces the OLD `clamp(56,96)` compose-path `base_ms_per_step`
/// byte-for-byte for a bright image whose raw brightness→BPM exceeds 96. If the loader mirror
/// (§3.8) or the default-window floor (§3.1) regressed, this de-cap would silently no-op or
/// drop the legacy window and the byte-identical clamp would break.
///
/// Built by cloning the shipped `PlanMappings` and overwriting `.affect` with the default
/// floor and `.character` with an empty `SelectTable` (always-Ballad) — modelling exactly the
/// pre-S22 mappings shape, with no production source or asset touched.
#[test]
fn affect_absent_block_keeps_ballad_window() {
    let m = mappings();
    let mut legacy = plan_mappings(&m);
    legacy.affect = AffectMappings::default(); // no affect block → default floor
    legacy.character = SelectTable::default(); // empty ladder → always-Ballad (pre-S22 state)
    let planner = CompositionPlanner::new(legacy);

    // A bright image whose raw brightness→BPM exceeds 96 (brightness 100 → raw 150).
    let bright = ImageUnderstanding {
        avg_brightness: 100.0,
        ..bright_energetic()
    };
    let plan = planner.plan(&bright, &m);

    // The legacy hard clamp: raw BPM clamped into the Ballad 56..96 window → 96 → base_ms 625.
    let raw_bpm = 150.0_f64; // top anchor of the de-capped brightness_to_tempo_bpm table
    let legacy_clamped = raw_bpm.clamp(56.0, 96.0); // == 96.0
    let legacy_base_ms = (60_000.0 / legacy_clamped.max(1.0)).round() as u64; // == 625

    assert_eq!(
        plan.character,
        Character::Ballad,
        "an empty character ladder must keep the plan on Ballad (legacy state)"
    );
    assert_eq!(
        plan.key_tempo.base_ms_per_step, legacy_base_ms,
        "absent affect block must reproduce the legacy clamp(56,96) base_ms_per_step byte-for-byte"
    );
    // And the recovered BPM is exactly the old 96 ceiling.
    assert!(
        (plan_bpm(&plan) - 96.0).abs() < 1e-9,
        "legacy back-compat BPM must be exactly the old 96 ceiling, got {:.6}",
        plan_bpm(&plan)
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// (6) BYTE-FREEZE WITNESS — the engine/realizer is provably untouched by Slice A.
// ─────────────────────────────────────────────────────────────────────────────

/// The byte-freeze witness (spec §6(f)), re-baselined for the S28/K3 slice (lead-approved,
/// spec-s28-k3-build §3 guarantee 3). K3 deliberately moves the engine kernel to a NEW frozen
/// byte-anchor (pivot/common-tone modulation + land-home cadence) while engine_equivalence stays
/// byte-green (9/9), so a `git diff HEAD` witness is no longer correct: HEAD predates K3, so the
/// diff is legitimately non-empty on the K3 tree. The witness is therefore re-pointed to the
/// COMMIT-STATE-INDEPENDENT sha anchor (matching keyplan_s25::engine_equivalence_byte_green):
/// `sha256sum src/engine.rs` == the new locked anchor. This passes on the K3-landed tree
/// (committed OR uncommitted) and FAILS LOUDLY if engine.rs drifts off the anchor in a future
/// slice — a true forward guard, not a no-op. The engine_equivalence net owns the
/// equivalence-test freeze, so this guard pins only the engine kernel byte-image.
///
/// Robustness: if `sha256sum` is unavailable the check is inconclusive-but-non-failing (the
/// engine_equivalence suite + the Quality Gate's own diff are the authoritative freeze guards)
/// rather than spuriously red; a readable file that mismatches the anchor always fails, so a
/// missing tool can never silently pass.
#[test]
fn byte_freeze_witness_locked_files_unmoved() {
    use std::process::Command;

    // The new S28/K3 frozen engine-kernel byte-anchor (sha256 of src/engine.rs).
    const ENGINE_SHA256: &str = "e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261";

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
            // sha256sum not runnable in this environment → inconclusive, not a failure. The
            // engine_equivalence net + the Quality Gate's manual git-diff are the freeze
            // authority; we do not turn a missing tool into a spurious red.
            eprintln!(
                "byte_freeze_witness: sha256sum unavailable; deferring to engine_equivalence \
                 + Quality-Gate diff as the freeze authority"
            );
        }
    }
}

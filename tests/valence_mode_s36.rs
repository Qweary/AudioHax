//! tests/valence_mode_s36.rs — the C6.6 "VALENCE owns the major/minor third" property net.
//!
//! C6.6 (landed on `src/composition.rs`) demotes HUE from owning the church mode outright to a
//! within-family COLORIST garnish, and promotes VALENCE to owning the third (the major/minor
//! FAMILY). The seam is the pure fn
//!
//!     pub fn valence_family_mode(hue_mode, valence, cuts) -> String
//!
//! with `assets/mappings.json` seeding `composition.affect.mode_valence_cuts = {major_min:0.55,
//! minor_max:0.45}`, applied at the `home_mode` derivation in `plan()` so
//! `plan().sections[i].mode` is now valence-projected.
//!
//! This net proves C6.6 is REAL and not gamed. It would FAIL if C6.6 were reverted (the headline
//! flip test genuinely exercises valence overriding the hue-selected third). It asserts BOTH on
//! `valence_family_mode` directly (the pure rule) AND on `plan().sections[].mode` (the integration
//! wiring), and it covers the legacy no-op so the equivalence net stays green.
//!
//! RNG DISCIPLINE (same as keyplan_k2a.rs / keyplan_s25.rs): `plan()` delegates per-section
//! HARMONY to `pick_progression`/`thread_rng`, so chords / Roman numerals / per-step content are
//! NON-deterministic and are NEVER asserted. The MODE string (`Section.mode` / `key_tempo.home_mode`)
//! is RNG-INDEPENDENT — it is the pure `valence_family_mode(hue_mode, affect_valence, cuts)` output
//! — so every assertion here is on the deterministic mode/family logic only, never on a live
//! thread_rng-derived value.
//!
//! VALENCE STEERING THROUGH `plan()`: the planner recomputes `affect_valence` from the shipped
//! VALENCE blend (`composition.affect.valence_weights`), so to drive valence high/low through
//! `plan()` we steer its inputs. With the shipped weights
//!   v = 0.70*(avg_brightness/100) + 0.20*(avg_saturation/100) + 0.10*(0.5 + 0.5*fg_bg_contrast)
//! the dominant term is `avg_brightness`. We set `avg_saturation`/`fg_bg_contrast` to fixed values
//! and move `avg_brightness` to place the whole-image valence above 0.55 (MAJOR), below 0.45
//! (MINOR), or inside the (0.45,0.55) dead band. `dominant_hue` selects the PRE-projection
//! church mode via the shipped `hue_to_mode` range map:
//!   0-30 Phrygian | 31-90 Lydian | 91-150 Ionian | 151-210 Dorian | 211-270 Aeolian | 271-330 Mixolydian
//! (Lydian/Ionian/Mixolydian = MAJOR-family hues; Phrygian/Dorian/Aeolian = MINOR-family hues.)

use audiohax::chord_engine::ChordEngine;
use audiohax::composition::{
    valence_family_mode, Character, CompositionPlanner, ImageUnderstanding, ModeValenceCuts,
    PlanMappings, SelectTable, ThematicRole,
};
use audiohax::mapping_loader::{load_mappings, rebuild_mapping_table, MappingTable};

const MAPPINGS_PATH: &str = "assets/mappings.json";

/// The shipped cut-points (== `assets/mappings.json` composition.affect.mode_valence_cuts).
const MAJOR_MIN: f32 = 0.55;
const MINOR_MAX: f32 = 0.45;

/// The six church modes by FAMILY (the major/minor third). Mirrors `valence_family_mode`'s
/// brightness-slot triples, darkest→brightest.
const MAJOR_MODES: [&str; 3] = ["Mixolydian", "Ionian", "Lydian"];
const MINOR_MODES: [&str; 3] = ["Phrygian", "Aeolian", "Dorian"];

fn is_major(mode: &str) -> bool {
    MAJOR_MODES.contains(&mode)
}
fn is_minor(mode: &str) -> bool {
    MINOR_MODES.contains(&mode)
}

/// The shipped cuts as the `Option<ModeValenceCuts>` the production rule takes.
fn cuts() -> Option<ModeValenceCuts> {
    Some(ModeValenceCuts {
        major_min: MAJOR_MIN,
        minor_max: MINOR_MAX,
    })
}

fn mappings() -> MappingTable {
    load_mappings(MAPPINGS_PATH).expect("mappings.json loads")
}

fn base_plan_mappings(m: &MappingTable) -> PlanMappings {
    m.composition
        .clone()
        .expect("composition block present in mappings.json")
        .into()
}

/// A `SelectTable` that ALWAYS resolves to `id` (empty rule set → the default fires).
fn always(id: &str) -> SelectTable {
    SelectTable {
        default: id.to_string(),
        rules: Vec::new(),
    }
}

/// The whole-image VALENCE composite the planner computes, recomputed here from the SHIPPED
/// blend so a test can predict which family a given input projects into. The shipped weights are
/// `{avg_brightness:0.70, avg_saturation:0.20, fg_bg_contrast:0.10}`; the `fg_bg_contrast` term
/// goes through the `0.5 + 0.5*x` fluency transform; the two HSV scalars are /100. Clamped 0..1.
fn affect_valence(avg_brightness: f32, avg_saturation: f32, fg_bg_contrast: f32) -> f32 {
    (0.70 * (avg_brightness / 100.0)
        + 0.20 * (avg_saturation / 100.0)
        + 0.10 * (0.5 + 0.5 * fg_bg_contrast))
        .clamp(0.0, 1.0)
}

/// Solve for the `avg_brightness` (0..100) that lands the whole-image valence at a target, holding
/// `avg_saturation = 50`, `fg_bg_contrast = 0.0`. Used to drive `plan()` to a chosen valence band.
///   v = 0.70*(b/100) + 0.20*0.5 + 0.10*(0.5) = 0.70*(b/100) + 0.15  ⇒  b = (target - 0.15)/0.70*100
fn brightness_for_valence(target: f32) -> f32 {
    ((target - 0.15) / 0.70 * 100.0).clamp(0.0, 100.0)
}

/// A whole-image understanding whose hue selects a church mode and whose brightness sets valence.
/// `avg_saturation = 50`, `fg_bg_contrast = 0.0` are held so `affect_valence(brightness,50,0)` is
/// the predictor above. The per-region fields are left at the whole-image fallback (neutral()).
fn craft(dominant_hue: f32, avg_brightness: f32) -> ImageUnderstanding {
    ImageUnderstanding {
        dominant_hue,
        avg_brightness,
        avg_saturation: 50.0,
        fg_bg_contrast: 0.0,
        ..ImageUnderstanding::neutral()
    }
}

/// Resolve `plan().key_tempo.home_mode` for a hue+brightness on the SHIPPED mappings (cuts active).
fn home_mode_of(dominant_hue: f32, avg_brightness: f32) -> String {
    let m = mappings();
    let planner = CompositionPlanner::new(base_plan_mappings(&m));
    let u = craft(dominant_hue, avg_brightness);
    planner.plan(&u, &m).key_tempo.home_mode
}

// ═════════════════════════════════════════════════════════════════════════════
// Property 1 — headline_flip (THE verdict gate: valence overrides hue on the third)
// ═════════════════════════════════════════════════════════════════════════════

/// THE core C6.6 property. A HIGH-valence image whose HUE alone would pick a MINOR mode resolves to
/// a MAJOR-family mode; a LOW-valence image whose HUE alone would pick a MAJOR mode resolves to a
/// MINOR-family mode. Asserted BOTH on `valence_family_mode` directly AND end-to-end through
/// `plan().key_tempo.home_mode`. This test would FAIL if C6.6 were reverted to hue-only selection
/// (it pins valence overriding the hue-selected third — the exact thing C6.6 introduced).
#[test]
fn headline_flip() {
    // ── (a) the pure rule ──────────────────────────────────────────────────
    // hue picked a MINOR mode (Aeolian) but HIGH valence forces MAJOR.
    let hi = valence_family_mode("Aeolian", 0.90, &cuts());
    assert!(
        is_major(&hi),
        "HIGH valence must force a MAJOR-family mode even though hue picked minor (Aeolian); got {hi}"
    );
    // hue picked a MAJOR mode (Ionian) but LOW valence forces MINOR.
    let lo = valence_family_mode("Ionian", 0.10, &cuts());
    assert!(
        is_minor(&lo),
        "LOW valence must force a MINOR-family mode even though hue picked major (Ionian); got {lo}"
    );
    // The flip is real: same hue mode, opposite valence → opposite family.
    let aeolian_hi = valence_family_mode("Aeolian", 0.90, &cuts());
    let aeolian_lo = valence_family_mode("Aeolian", 0.10, &cuts());
    assert!(
        is_major(&aeolian_hi) && is_minor(&aeolian_lo),
        "the SAME hue mode (Aeolian) flips family with valence: hi={aeolian_hi} lo={aeolian_lo}"
    );

    // ── (b) end-to-end through plan() ──────────────────────────────────────
    // dominant_hue 250 → hue_to_mode → Aeolian (MINOR hue). Drive valence HIGH → MAJOR family.
    let b_hi = brightness_for_valence(0.90);
    assert!(
        affect_valence(b_hi, 50.0, 0.0) >= MAJOR_MIN,
        "the HIGH driver must actually exceed major_min (sanity on the steering)"
    );
    let mode_hi = home_mode_of(250.0, b_hi);
    assert!(
        is_major(&mode_hi),
        "plan(): hue 250 picks minor Aeolian, but HIGH valence (b={b_hi}) must yield a MAJOR \
         home_mode; got {mode_hi}"
    );

    // dominant_hue 120 → hue_to_mode → Ionian (MAJOR hue). Drive valence LOW → MINOR family.
    let b_lo = brightness_for_valence(0.10);
    assert!(
        affect_valence(b_lo, 50.0, 0.0) <= MINOR_MAX,
        "the LOW driver must actually fall below minor_max (sanity on the steering)"
    );
    let mode_lo = home_mode_of(120.0, b_lo);
    assert!(
        is_minor(&mode_lo),
        "plan(): hue 120 picks major Ionian, but LOW valence (b={b_lo}) must yield a MINOR \
         home_mode; got {mode_lo}"
    );

    // The end-to-end flip witnessed on ONE hue: hue 250 (minor) → MAJOR under high valence proves
    // valence beat hue at the section level, the property's whole point.
    assert!(
        is_major(&mode_hi) && is_minor(&mode_lo),
        "end-to-end valence-over-hue flip failed: hue250+hi={mode_hi}, hue120+lo={mode_lo}"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Property 2 — neutral_band_hysteresis (dead band leaves hue's mode untouched)
// ═════════════════════════════════════════════════════════════════════════════

/// For valence strictly inside the dead band (minor_max, major_min) == (0.45, 0.55), the resolved
/// mode EQUALS the hue-selected mode unchanged — the legacy hue-only behaviour is preserved exactly
/// where valence is ambivalent. Swept over several hues × several mid-band valences, on the pure
/// rule AND end-to-end through `plan()`.
#[test]
fn neutral_band_hysteresis() {
    // (dominant_hue, hue-selected mode) per the shipped hue_to_mode map.
    let hue_modes = [
        (10.0f32, "Phrygian"),
        (60.0, "Lydian"),
        (120.0, "Ionian"),
        (180.0, "Dorian"),
        (240.0, "Aeolian"),
        (300.0, "Mixolydian"),
    ];
    // STRICTLY inside (0.45, 0.55).
    let mid_valences = [0.46f32, 0.475, 0.50, 0.525, 0.54];

    for &(hue, hue_mode) in &hue_modes {
        for &v in &mid_valences {
            // (a) pure rule: dead band → identity.
            let r = valence_family_mode(hue_mode, v, &cuts());
            assert_eq!(
                r, hue_mode,
                "dead-band valence {v} must leave the hue mode ({hue_mode}) UNCHANGED; got {r}"
            );

            // (b) end-to-end: a brightness that lands the whole-image valence in the dead band
            // leaves plan()'s home_mode == the hue-selected mode.
            let b = brightness_for_valence(v);
            let av = affect_valence(b, 50.0, 0.0);
            assert!(
                av > MINOR_MAX && av < MAJOR_MIN,
                "steering sanity: brightness {b} must land valence in the OPEN dead band, got {av}"
            );
            let mode = home_mode_of(hue, b);
            assert_eq!(
                mode, hue_mode,
                "plan(): dead-band valence (b={b}, v={av}) at hue {hue} must keep the \
                 hue-selected mode ({hue_mode}); got {mode}"
            );
        }
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Property 3 — family_correctness (the third is REAL: degree-2 interval = +4 maj vs +3 min)
// ═════════════════════════════════════════════════════════════════════════════

/// `valence >= 0.55` ⇒ resolved mode ∈ MAJOR family; `valence <= 0.45` ⇒ resolved mode ∈ MINOR
/// family. We assert the family BOTH by name-set AND — the strong form — by the actual interval of
/// the diatonic third produced by `ChordEngine::generate_chords` on a I chord in the resolved mode:
/// a MAJOR third is +4 semitones above the root, a MINOR third is +3. Driven over BOTH hue
/// polarities so the assertion isn't an artifact of the hue already being in the target family.
#[test]
fn family_correctness() {
    let m = mappings();
    let engine = ChordEngine::new(rebuild_mapping_table(&m));
    let prog: Vec<String> = vec!["I".into()];
    const ROOT: u8 = 60; // C4.

    // The third of a tonic triad in `mode`: generate_chords is PUBLIC and RNG-FREE (params chosen
    // 1:1: low edge, no borrow, no mixture). chord 0 is the I triad; notes[0] root, the diatonic
    // third is the first note > root within the triad.
    let third_interval = |mode: &str| -> i16 {
        let chords = engine.generate_chords(&prog, ROOT, mode, 0.0, 0.0, 50.0, 0.0);
        let triad = &chords[0];
        let root = triad.notes[0] as i16;
        // The chordal third = the triad pitch a third (3 or 4 semitones) above the root.
        triad
            .notes
            .iter()
            .map(|&n| n as i16 - root)
            .find(|&iv| iv == 3 || iv == 4)
            .expect("a tonic triad has a diatonic third (3 or 4 semitones)")
    };

    // hue 250 → Aeolian (minor hue); hue 120 → Ionian (major hue): cover both polarities.
    for &hue in &[250.0f32, 120.0] {
        // valence >= 0.55 ⇒ MAJOR family, major third (+4).
        let b_maj = brightness_for_valence(0.80);
        let mode_maj = home_mode_of(hue, b_maj);
        assert!(
            is_major(&mode_maj),
            "valence>=0.55 at hue {hue} must be a MAJOR-family mode (name set); got {mode_maj}"
        );
        assert_eq!(
            third_interval(&mode_maj),
            4,
            "MAJOR family ({mode_maj}) must have a +4 (major) third; the third is REAL"
        );

        // valence <= 0.45 ⇒ MINOR family, minor third (+3).
        let b_min = brightness_for_valence(0.20);
        let mode_min = home_mode_of(hue, b_min);
        assert!(
            is_minor(&mode_min),
            "valence<=0.45 at hue {hue} must be a MINOR-family mode (name set); got {mode_min}"
        );
        assert_eq!(
            third_interval(&mode_min),
            3,
            "MINOR family ({mode_min}) must have a +3 (minor) third; the third is REAL"
        );
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Property 4 — brightness_rank_garnish (hue's colorist contribution SURVIVES the flip)
// ═════════════════════════════════════════════════════════════════════════════

/// A forced flip lands on the SAME-brightness slot of the forced family — proving hue's colorist
/// (which-church-mode) contribution survives rather than collapsing to a single default. The
/// brightness slots, darkest→brightest, are paired across families:
///   Mixolydian↔Phrygian (darkest) | Ionian↔Aeolian (mid) | Lydian↔Dorian (brightest).
/// So a BRIGHT hue mode forced to the other family lands on THAT family's bright slot, not its mid
/// or dark slot. Asserted on the pure rule across all six modes × both flip directions, plus an
/// end-to-end witness (a Lydian-hue image forced minor must resolve Dorian, never Aeolian/Phrygian).
#[test]
fn brightness_rank_garnish() {
    // (hue_mode, slot)  slot 0=darkest,1=mid,2=brightest, within its family.
    let major = [("Mixolydian", 0usize), ("Ionian", 1), ("Lydian", 2)];
    let minor = [("Phrygian", 0usize), ("Aeolian", 1), ("Dorian", 2)];

    // Major hue mode forced MINOR → same slot in MINOR_MODES.
    for &(mode, slot) in &major {
        let forced = valence_family_mode(mode, 0.05, &cuts());
        assert_eq!(
            forced, MINOR_MODES[slot],
            "{mode} (major slot {slot}) forced MINOR must land on the SAME-brightness minor slot \
             ({}), preserving hue's garnish — not collapse to a default; got {forced}",
            MINOR_MODES[slot]
        );
    }
    // Minor hue mode forced MAJOR → same slot in MAJOR_MODES.
    for &(mode, slot) in &minor {
        let forced = valence_family_mode(mode, 0.95, &cuts());
        assert_eq!(
            forced, MAJOR_MODES[slot],
            "{mode} (minor slot {slot}) forced MAJOR must land on the SAME-brightness major slot \
             ({}); got {forced}",
            MAJOR_MODES[slot]
        );
    }

    // Headline garnish-preservation case from the spec: Lydian (brightest major) forced MINOR must
    // land Dorian (brightest minor), NOT Aeolian (mid) or Phrygian (dark).
    let lydian_min = valence_family_mode("Lydian", 0.01, &cuts());
    assert_eq!(
        lydian_min, "Dorian",
        "Lydian (brightest) forced minor must be Dorian (brightest minor), got {lydian_min}"
    );

    // End-to-end: hue 60 → Lydian (bright major hue). Drive valence LOW → must resolve Dorian.
    let b_lo = brightness_for_valence(0.08);
    let mode = home_mode_of(60.0, b_lo);
    assert_eq!(
        mode, "Dorian",
        "plan(): bright Lydian hue forced minor by low valence must resolve Dorian (bright minor \
         slot), proving hue's brightness-garnish survives the flip; got {mode}"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Property 5 — legacy_no_op (cuts == None ⇒ identity for ALL valence; freeze-inert)
// ═════════════════════════════════════════════════════════════════════════════

/// `cuts == None` (a legacy mappings.json with no `mode_valence_cuts` block) ⇒ `valence_family_mode`
/// returns the hue mode UNCHANGED for ALL valence values. This is the back-compat / byte-freeze
/// guarantee that keeps the equivalence net green. Swept over the six modes × valence 0.0..1.0 AND
/// an unrecognised mode string (which must also pass through untouched).
#[test]
fn legacy_no_op() {
    let none: Option<ModeValenceCuts> = None;
    let modes = [
        "Phrygian",
        "Lydian",
        "Ionian",
        "Dorian",
        "Aeolian",
        "Mixolydian",
        "Locrian",  // a 7th church mode the projection does not move
        "NotAMode", // unrecognised — must pass through under None too
    ];
    for &mode in &modes {
        for i in 0..=10u32 {
            let v = i as f32 / 10.0; // 0.0, 0.1, … 1.0 — spans BOTH cut bands
            let r = valence_family_mode(mode, v, &none);
            assert_eq!(
                r, mode,
                "cuts==None must be a NO-OP for ALL valence: mode {mode} at v {v} → {r}"
            );
        }
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Property 6 — determinism (same input → same mode; no RNG in the mode logic)
// ═════════════════════════════════════════════════════════════════════════════

/// `valence_family_mode` is pure — same (hue_mode, valence, cuts) → same output across repeated
/// calls — AND `plan().key_tempo.home_mode` for a fixed image is stable across repeated plans (the
/// mode logic never touches the `thread_rng` harmony path). Repeated many times to catch any
/// accidental RNG/clock dependence.
#[test]
fn determinism() {
    // (a) the pure rule.
    let first = valence_family_mode("Aeolian", 0.92, &cuts());
    for _ in 0..64 {
        assert_eq!(
            valence_family_mode("Aeolian", 0.92, &cuts()),
            first,
            "valence_family_mode must be deterministic"
        );
    }
    // An unrecognised mode is also deterministically untouched.
    assert_eq!(
        valence_family_mode("Bogus", 0.92, &cuts()),
        "Bogus",
        "unrecognised mode is left untouched (deterministically)"
    );

    // (b) end-to-end: home_mode stable across repeated plans of the SAME image (RNG-independent).
    let m = mappings();
    let planner = CompositionPlanner::new(base_plan_mappings(&m));
    let u = craft(250.0, brightness_for_valence(0.90)); // minor hue, high valence → MAJOR
    let mode0 = planner.plan(&u, &m).key_tempo.home_mode;
    for _ in 0..16 {
        assert_eq!(
            planner.plan(&u, &m).key_tempo.home_mode,
            mode0,
            "plan()'s home_mode must be RNG-independent / stable across runs; got drift"
        );
    }
    assert!(
        is_major(&mode0),
        "(sanity) the determinism witness is a real flip case (minor hue, high valence → major); \
         got {mode0}"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Property 7 — spec9_coherence (section-consistency, return-stability, char/mode non-contradiction)
// ═════════════════════════════════════════════════════════════════════════════

/// The valence-projected mode polarity is constant WITHIN a section and STABLE across a returning
/// theme, and never contradicts the section character.
///
///   (a) mode-is-section-consistent — within any `Section`, the mode (hence its major/minor
///       polarity) is a single value; and the section mode == the plan's `home_mode` (the
///       no-modal-plan-yet invariant survives C6.6's projection).
///   (b) return-theme polarity-stable — for a form with a Return role, polarity(Statement) ==
///       polarity(Return): the recapitulation comes home in the SAME family.
///   (c) character-mode non-contradiction — a plan never carries (character ∈ {Scherzo,Hymn} ∧
///       minor mode) nor (character == Lament ∧ major mode). Reachability note in the report: the
///       planner selects `character` independently of mode and does NOT itself enforce a coupling,
///       so we CONSTRUCT each (character, valence) pair to the COHERENT combination (Scherzo/Hymn
///       with high valence → major; Lament with low valence → minor) and assert the resolved plan
///       is non-contradictory — i.e. the projection lets character and mode AGREE when the operator
///       pairs them, which is the reachable form of the guard-rail through the public plan surface.
#[test]
fn spec9_coherence() {
    let m = mappings();

    // ── (a) + (b): drive a Return-bearing form (ternary_aba: Statement/Contrast/Return) and check
    //     per-section polarity constancy + Statement↔Return polarity equality, under BOTH families.
    let polarity = |mode: &str| -> &'static str {
        if is_major(mode) {
            "major"
        } else if is_minor(mode) {
            "minor"
        } else {
            "other"
        }
    };
    for &(hue, target_v) in &[(250.0f32, 0.90f32), (120.0, 0.10)] {
        let mut pm = base_plan_mappings(&m);
        pm.form = always("ternary_aba"); // Statement / Contrast / Return
        let planner = CompositionPlanner::new(pm);
        let u = craft(hue, brightness_for_valence(target_v));
        let plan = planner.plan(&u, &m);
        let home = &plan.key_tempo.home_mode;

        // (a) every section's mode == home_mode (single polarity within & across sections).
        for (i, s) in plan.sections.iter().enumerate() {
            assert_eq!(
                &s.mode, home,
                "section {i} ({:?}) mode must == plan home_mode ({home}) — single polarity; got {}",
                s.thematic_role, s.mode
            );
        }
        // (b) Statement and Return resolve to the SAME polarity (trivially true while every section
        // carries home_mode, but asserted explicitly so a future per-section modal plan can't break
        // the recapitulation-comes-home invariant without tripping this).
        let stmt = plan
            .sections
            .iter()
            .find(|s| s.thematic_role == ThematicRole::Statement)
            .map(|s| s.mode.clone());
        let ret = plan
            .sections
            .iter()
            .find(|s| s.thematic_role == ThematicRole::Return)
            .map(|s| s.mode.clone());
        if let (Some(a), Some(aprime)) = (stmt, ret) {
            assert_eq!(
                polarity(&a),
                polarity(&aprime),
                "return-theme polarity must be stable: Statement {a} vs Return {aprime}"
            );
        }
    }

    // ── (c) character-mode non-contradiction. Pin character + pair it with the COHERENT valence.
    // Scherzo + high valence → major (no Scherzo∧minor). Hymn + high → major. Lament + low → minor.
    let coherent_cases: &[(&str, Character, f32, f32)] = &[
        ("scherzo", Character::Scherzo, 250.0, 0.92), // minor hue, high valence → MAJOR (no Scherzo∧minor)
        ("hymn", Character::Hymn, 240.0, 0.90), // minor hue, high valence → MAJOR (no Hymn∧minor)
        ("lament", Character::Lament, 120.0, 0.08), // major hue, low valence → MINOR (no Lament∧major)
    ];
    for &(char_id, expect_char, hue, target_v) in coherent_cases {
        let mut pm = base_plan_mappings(&m);
        pm.character = always(char_id);
        let planner = CompositionPlanner::new(pm);
        let u = craft(hue, brightness_for_valence(target_v));
        let plan = planner.plan(&u, &m);
        assert_eq!(
            plan.character, expect_char,
            "character must be the pinned {char_id}; got {:?}",
            plan.character
        );
        let mode = &plan.key_tempo.home_mode;
        // The contradiction set (the thing C6.6 must NOT produce when paired coherently).
        let scherzo_or_hymn_minor =
            matches!(plan.character, Character::Scherzo | Character::Hymn) && is_minor(mode);
        let lament_major = plan.character == Character::Lament && is_major(mode);
        assert!(
            !scherzo_or_hymn_minor,
            "non-contradiction: {:?} must not carry a MINOR mode here; got {mode}",
            plan.character
        );
        assert!(
            !lament_major,
            "non-contradiction: Lament must not carry a MAJOR mode here; got {mode}"
        );
    }
}

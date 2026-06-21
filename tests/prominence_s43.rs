//! tests/prominence_s43.rs — the S43 TWO-TIER MELODY-FOREGROUNDING property net.
//!
//! Encodes the `docs/design-s42-salience-diagnosis.md` §3 "Encodable correctness
//! guards" as REAL property tests against the Phase-1 fix already applied to
//! `assets/mappings.json`: the always-on `melody_forward` DEFAULT profile (Melody
//! 0.78) plus the relaxed escalation gate (`fg_bg_contrast` floor 0.25 → 0.10) that
//! promotes `example.jpg` to the full `subject_melody` lift (Melody 1.0) while
//! `Lena.png` stays on the `melody_forward` default.
//!
//! The S42 root cause: prominence resolved to the empty `uniform` default → neutral
//! 0.500 on every step → every saliency nudge a no-op → the melody buried, the
//! HarmonicFill the loudest role, and `example`/`Lena` heard as "the same piece in a
//! different key." This net pins the fix: a guaranteed foreground on EVERY image, a
//! per-image divergence between the two tiers, and an audible figure/ground velocity
//! gap with the melody — not the Fill — on top.
//!
//! RESOLUTION PATH — THE REAL PLANNER, NOT A HAND-BUILT PROFILE. Guards 1, 2, and 5
//! resolve prominence through the SHIPPED pipeline end-to-end: the real reference
//! images on disk (`assets/images/example.jpg`, `assets/images/Lena.png`) are read
//! with the `image` crate, run through `pure_analysis::understand_image_pure` to get
//! each image's ACTUAL `subject_size` / `fg_bg_contrast`, and fed to
//! `CompositionPlanner::plan` (loaded from `assets/mappings.json` via
//! `mapping_loader::load_mappings`). The resolved per-section
//! `orchestration.prominence` is read straight off `plan.sections` — exactly the Vec
//! the frozen realizer consumes. This is the strongest available evidence: it proves
//! the SelectTable gate fires (or does not) on the real images, not on documented
//! numbers re-typed into a fixture. (Documented expectation, design §3: `example`
//! fg_bg_contrast 0.136 ≥ 0.10 ⇒ ESCALATE; `Lena` 0.052 < 0.10 ⇒ default.)
//!
//! Guards 3 and 4 read the two profile tiers straight from the loaded
//! `prominence_catalogue` (the real `assets/mappings.json` table, not literals).
//!
//! Guard 5 (velocity figure/ground) reuses the `prominence_s23.rs` realizer-driving
//! construction (`section_with` / `realize_under` / `mean_vel`) but feeds it the
//! prominence Vec RESOLVED from each real image, so the realized velocities are the
//! ones the actual renders carry.
//!
//! HEADLESS / FEATURE SET: built and run under the DEFAULT feature set (the
//! engagement convention — see prominence_s23.rs / saliency_s18.rs headers). The
//! `image` crate is in the default set via `pure-analysis`; this net touches NO
//! OpenCV, NO audio hardware. The planner delegates per-section harmony to
//! `pick_progression` (thread_rng), so NO chord/note PITCH value is asserted here —
//! every asserted property (resolved prominence WEIGHTS, role VELOCITY level) is
//! RNG-independent of the chord draw.
//!
//! FREEZE: `src/engine.rs` is byte-frozen at sha256
//! `e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261`. This net adds
//! TESTS ONLY and touches no production code. The freeze harness lives in
//! prominence_s23.rs::engine_freeze_diff_empty and engine_equivalence.rs (9/9); this
//! file does NOT duplicate it (guard 6 is satisfied by running those suites — see the
//! footer). A single belt-and-suspenders sha re-confirmation is included as the
//! natural, non-duplicative forward guard.

use audiohax::chord_engine::RhythmMotto;
use audiohax::chord_engine::{
    realize_step, Chord, NoteEvent, PerfFeatures, PhrasePosition, StepPlan,
};
use audiohax::composition::{
    CadenceStrength, CompositionPlanner, ImageUnderstanding, KeyTempoPlan, LayerProminence,
    LayerRole, OrchestrationProfile, PlanMappings, ProminenceProfile, ResolutionPolicy, Section,
    StepContext, ThematicRole, ThemeVariation,
};
use audiohax::mapping_loader::{load_mappings, MappingTable};
use audiohax::pure_analysis::understand_image_pure;

const EXAMPLE_IMG: &str = "assets/images/example.jpg";
const LENA_IMG: &str = "assets/images/Lena.png";
const MS_PER_STEP: u64 = 200;

// ─────────────────────────────────────────────────────────────────────────────
// Fixtures — load the SHIPPED mappings + real reference images, resolve through the
// REAL planner. Mirrors composition_s15.rs (mappings/plan) + saliency_s18.rs (image
// load) + prominence_s23.rs (realizer-driving section/realize_under).
// ─────────────────────────────────────────────────────────────────────────────

/// The shipped `assets/mappings.json` (the same table the engine holds).
fn mappings() -> MappingTable {
    load_mappings("assets/mappings.json").expect("assets/mappings.json loads")
}

/// The composition `PlanMappings` (carries the prominence SelectTable + catalogue).
fn plan_mappings(m: &MappingTable) -> PlanMappings {
    m.composition
        .clone()
        .expect("composition block present in mappings.json")
        .into()
}

/// Read a real reference image off disk and run the SHIPPED pure analyzer over it,
/// yielding the image's ACTUAL `ImageUnderstanding` (the real subject_size /
/// fg_bg_contrast that the prominence gate is evaluated against).
fn understanding(path: &str) -> ImageUnderstanding {
    let img = image::open(path)
        .unwrap_or_else(|e| panic!("open reference image {path}: {e}"))
        .to_rgb8();
    understand_image_pure(&img).unwrap_or_else(|e| panic!("understand_image_pure({path}): {e:?}"))
}

/// Resolve prominence through the FULL real pipeline: real image → real understanding
/// → real planner → the per-section `orchestration.prominence` the realizer consumes.
/// Returns the resolved prominence Vec for EVERY section of the render (they are
/// resolved once per plan, so all sections share the profile — we keep them all so the
/// foreground-exists invariant can be asserted per-section, not just once).
fn resolved_prominence_per_section(
    planner: &CompositionPlanner,
    m: &MappingTable,
    path: &str,
) -> Vec<Vec<LayerProminence>> {
    let u = understanding(path);
    let plan = planner.plan(&u, m);
    assert!(
        !plan.sections.is_empty(),
        "a real render of {path} must have at least one section"
    );
    plan.sections
        .iter()
        .map(|s| s.orchestration.prominence.clone())
        .collect()
}

/// The resolved weight for a role within one section's prominence Vec, if present.
fn weight_of(prom: &[LayerProminence], role: LayerRole) -> Option<f32> {
    prom.iter().find(|p| p.role == role).map(|p| p.weight)
}

/// Look up a named profile in a `prominence_catalogue` read from the real mappings.
fn catalogue_profile<'a>(cat: &'a [ProminenceProfile], id: &str) -> &'a ProminenceProfile {
    cat.iter()
        .find(|p| p.id == id)
        .unwrap_or_else(|| panic!("prominence_catalogue must contain the `{id}` profile"))
}

fn catalogue_weight(cat: &[ProminenceProfile], id: &str, role: LayerRole) -> f32 {
    weight_of(&catalogue_profile(cat, id).layers, role)
        .unwrap_or_else(|| panic!("`{id}` profile must list a {role:?} weight"))
}

// ── Realizer-driving fixtures (mirror prominence_s23.rs) — for guard 5 ──────────

fn c_major() -> Chord {
    Chord {
        name: "I".to_string(),
        notes: vec![60, 64, 67], // C4 E4 G4
    }
}

/// A NON-cadence interior step (an accented interior position) so the per-role
/// velocity nudges fire and the figure/ground gap is observable.
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

fn perf(brightness: f32, edge_density: f32) -> PerfFeatures {
    PerfFeatures {
        saturation: 60.0,
        brightness,
        edge_density,
    }
}

fn key_tempo() -> KeyTempoPlan {
    KeyTempoPlan {
        home_root_midi: 60,
        home_mode: "Ionian".to_string(),
        base_ms_per_step: MS_PER_STEP,
        key_scheme: vec![0],
        tempo_scheme: vec![MS_PER_STEP],
    }
}

fn section_with(profile: OrchestrationProfile, step: &StepPlan) -> Section {
    Section {
        label: "A".to_string(),
        step_len: 1,
        thematic_role: ThematicRole::Statement,
        key_offset_semitones: 0,
        ms_per_step: MS_PER_STEP,
        mode: "Ionian".to_string(),
        progression: vec![],
        theme: None,
        variation: ThemeVariation::Identity,
        boundary_cadence: CadenceStrength::Perfect,
        pivot: false,
        resolution: ResolutionPolicy::Resolve,
        density: 0.5,
        orchestration: profile,
        steps: vec![step.clone()],
    }
}

fn realize_under(
    profile: &OrchestrationProfile,
    step: &StepPlan,
    inst_idx: usize,
    num_instruments: usize,
    features: &PerfFeatures,
) -> Vec<NoteEvent> {
    let kt = key_tempo();
    let sec = section_with(profile.clone(), step);
    let ctx = StepContext::single_section_default(&sec, &kt);
    realize_step(step, inst_idx, num_instruments, features, MS_PER_STEP, &ctx)
}

fn mean_vel(events: &[NoteEvent]) -> f64 {
    assert!(
        !events.is_empty(),
        "role emitted no events to average velocity"
    );
    events.iter().map(|e| e.velocity as f64).sum::<f64>() / events.len() as f64
}

/// A FULL 5-role ensemble `[Bass, HarmonicFill, Pad, CounterMelody, Melody]` carrying
/// a resolved prominence Vec — so every accompaniment ("bed") role AND the melody are
/// realized on the same step and their velocity levels can be compared. Bass=inst 0,
/// HarmonicFill=1, Pad=2, CounterMelody=3, Melody=4. `pad_voices:0` keeps each role a
/// single clean voice.
fn ensemble(prominence: Vec<LayerProminence>) -> OrchestrationProfile {
    OrchestrationProfile {
        id: "prom_s43_ensemble".to_string(),
        layers: vec![
            LayerRole::Bass,
            LayerRole::HarmonicFill,
            LayerRole::Pad,
            LayerRole::CounterMelody,
            LayerRole::Melody,
        ],
        density: 0.5,
        pad_voices: 0,
        figuration: None,
        figuration_resolved: None,
        bass_pattern: None,
        bass_pattern_resolved: None,
        prominence,
        motto: RhythmMotto::neutral(),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// GUARD 1 — FOREGROUND-EXISTS INVARIANT (the load-bearing guarantee)
// ─────────────────────────────────────────────────────────────────────────────

/// MUSICAL PROPERTY: the melody is foregrounded on EVERY image. Resolved Melody
/// prominence weight > 0.5 (strictly above the neutral 0.500 `uniform` default) on
/// EVERY section of BOTH real reference renders. This is the load-bearing fix — no
/// image falls back to the neutral, figure-less mix that buried the melody in S42.
#[test]
fn guard1_foreground_exists_on_every_section_of_both_renders() {
    let m = mappings();
    let planner = CompositionPlanner::new(plan_mappings(&m));

    for path in [EXAMPLE_IMG, LENA_IMG] {
        let per_section = resolved_prominence_per_section(&planner, &m, path);
        for (i, prom) in per_section.iter().enumerate() {
            let mel = weight_of(prom, LayerRole::Melody).unwrap_or_else(|| {
                panic!(
                    "{path} section {i}: resolved prominence MUST list a Melody weight \
                     (an empty/uniform fallback is the S42 bug)"
                )
            });
            eprintln!("[guard1] {path} section {i}: resolved Melody weight = {mel:.3}");
            assert!(
                mel > 0.5,
                "{path} section {i}: resolved Melody weight {mel:.3} not > 0.5 — the melody \
                 fell back to the neutral/uniform default (the S42 root-cause regression)"
            );
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// GUARD 2 — PER-IMAGE RESOLUTION DIVERGENCE (the escalation gate fires for one, not both)
// ─────────────────────────────────────────────────────────────────────────────

/// MUSICAL PROPERTY: the two images land on DIFFERENT prominence tiers. `example.jpg`
/// (subject_size in [0.05,0.55] AND fg_bg_contrast ≥ 0.10) ESCALATES to `subject_melody`
/// (Melody weight 1.0); `Lena.png` (fg_bg_contrast 0.052 < 0.10) routes to the SHALLOW
/// field tier `melody_lead_gentle` (Melody weight 0.74). Resolved through the REAL planner
/// on each image's ACTUAL features — proving the melody leads MORE assertively on the
/// separated-subject image than on the low-contrast field image (the per-image divergence).
///
/// S47 RE-DERIVATION (spec-s47-slice1-build.md §2c — the image-conditioned prominence FAMILY,
/// operator-locked 3 tiers deep/mid/shallow): the OLD Lena pin was 0.78 (`melody_forward`,
/// the single pre-S47 default). The new SelectTable routes a low-`fg_bg_contrast` field image
/// to the SHALLOW `melody_lead_gentle` tier (Melody 0.74 — S48 slice-3 re-baseline, raised
/// from 0.72 by the LEVEL finish, spec-s48 §2a.i; kept GENTLEST of the three tiers so a field
/// image is not forced into a hard lead), where the texture legitimately shares focus rather
/// than forcing a strong lead on an abstract/field image. 0.78 was superseded by the shallow
/// tier as the intended SHALLOW-tier resolution. Routing is UNCHANGED (Lena still routes
/// shallow); only the seeded weight literal moved. The PRESERVED property — the two images
/// DIVERGE in melody assertiveness (example 1.0 > Lena 0.74) — holds with headroom.
#[test]
fn guard2_per_image_resolution_divergence() {
    let m = mappings();
    let planner = CompositionPlanner::new(plan_mappings(&m));

    // Surface the real features the gate is evaluated against (the evidence the gate fired).
    let ex_u = understanding(EXAMPLE_IMG);
    let lena_u = understanding(LENA_IMG);
    eprintln!(
        "[guard2] example.jpg: subject_size={:.4} fg_bg_contrast={:.4} | \
         Lena.png: subject_size={:.4} fg_bg_contrast={:.4}",
        ex_u.subject_size, ex_u.fg_bg_contrast, lena_u.subject_size, lena_u.fg_bg_contrast
    );

    let ex = resolved_prominence_per_section(&planner, &m, EXAMPLE_IMG);
    let lena = resolved_prominence_per_section(&planner, &m, LENA_IMG);

    let ex_mel = weight_of(&ex[0], LayerRole::Melody).expect("example Melody weight");
    let lena_mel = weight_of(&lena[0], LayerRole::Melody).expect("Lena Melody weight");
    eprintln!("[guard2] resolved Melody weight: example={ex_mel:.3} (subject_melody=1.0 expected), Lena={lena_mel:.3} (melody_lead_gentle=0.74 expected)");

    // The escalation gate FIRED for example → full subject_melody lift (Melody 1.0).
    assert!(
        (ex_mel - 1.0).abs() < 1e-6,
        "example.jpg must ESCALATE to subject_melody (Melody 1.0); resolved {ex_mel:.3}"
    );
    // Lena (fg_bg_contrast < 0.10) routes to the SHALLOW field tier melody_lead_gentle
    // (Melody 0.74) — the S47 image-conditioned recession family re-baselined by the S48
    // slice-3 LEVEL finish (0.72→0.74); routing is unchanged, only the weight literal moved.
    assert!(
        (lena_mel - 0.74).abs() < 1e-6,
        "Lena.png must route to the melody_lead_gentle SHALLOW tier (Melody 0.74, S48 re-baseline); resolved {lena_mel:.3}"
    );
    // The load-bearing point: the two images DIVERGE — different profiles, not the same one.
    assert!(
        ex_mel > lena_mel,
        "the two renders must diverge in melody assertiveness: example {ex_mel:.3} not > Lena {lena_mel:.3}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// GUARD 3 — TWO-TIER PRESERVED (escalation strictly louder than default, both above neutral)
// ─────────────────────────────────────────────────────────────────────────────

/// MUSICAL PROPERTY: `subject_melody.Melody (1.0) > melody_forward.Melody (0.82) > 0.5`
/// (neutral). The escalation tier is strictly louder than the always-on default tier,
/// which is strictly above the neutral 0.500 — so the two tiers are themselves a source
/// of per-image identity (how assertively the melody leads), never collapsing into one
/// level. Read straight from the real `prominence_catalogue`.
///
/// S48 SLICE-3 RE-BASELINE (spec-s48-slice3-build.md §2a.i — the LEVEL finish): the mid
/// `melody_forward` Melody weight was raised 0.78 → 0.82 to widen the melody-vs-bed velocity
/// gap (the operator's "bump the melody volume"). Routing is UNCHANGED; only the seeded weight
/// literal moved. The two-tier ordering (subject_melody 1.0 > melody_forward 0.82 > 0.5) holds
/// with headroom.
#[test]
fn guard3_two_tier_strictly_ordered() {
    let m = mappings();
    let cat = &plan_mappings(&m).prominence_catalogue;

    let subject = catalogue_weight(cat, "subject_melody", LayerRole::Melody);
    let forward = catalogue_weight(cat, "melody_forward", LayerRole::Melody);
    eprintln!(
        "[guard3] subject_melody.Melody={subject:.3} > melody_forward.Melody={forward:.3} > 0.5"
    );

    assert!(
        subject > forward,
        "escalation tier subject_melody.Melody {subject:.3} must be strictly louder than \
         default tier melody_forward.Melody {forward:.3}"
    );
    assert!(
        forward > 0.5,
        "default tier melody_forward.Melody {forward:.3} must be strictly above neutral 0.5"
    );
    // Pin the documented seed magnitudes so a future retune is a deliberate, visible change.
    assert!(
        (subject - 1.0).abs() < 1e-6,
        "subject_melody.Melody must be the full lift 1.0; got {subject:.3}"
    );
    assert!(
        (forward - 0.82).abs() < 1e-6,
        "melody_forward.Melody must be the S48 slice-3 re-baselined 0.82 (raised from 0.78 by \
         the LEVEL finish, spec-s48 §2a.i); got {forward:.3}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// GUARD 4 — BED RECEDES BUT DOES NOT VANISH (default tier accompaniment band)
// ─────────────────────────────────────────────────────────────────────────────

/// MUSICAL PROPERTY: in the always-on `melody_forward` default, the accompaniment
/// recedes under the line but still supports it. Pad (0.40) and HarmonicFill (0.40)
/// are < 0.5 (below neutral, so they recede — removing the S42 over-loud Fill) AND
/// > 0.25 (so the bed supports rather than hollows out). Bass (0.50) is exactly
/// neutral — the structural foundation neither lifts nor recedes. Read from the real
/// catalogue.
#[test]
fn guard4_default_bed_recedes_but_does_not_vanish() {
    let m = mappings();
    let cat = &plan_mappings(&m).prominence_catalogue;

    let pad = catalogue_weight(cat, "melody_forward", LayerRole::Pad);
    let fill = catalogue_weight(cat, "melody_forward", LayerRole::HarmonicFill);
    let bass = catalogue_weight(cat, "melody_forward", LayerRole::Bass);
    eprintln!("[guard4] melody_forward bed: Pad={pad:.3} HarmonicFill={fill:.3} Bass={bass:.3}");

    for (role, w) in [("Pad", pad), ("HarmonicFill", fill)] {
        assert!(
            w < 0.5,
            "melody_forward.{role} {w:.3} must be < 0.5 (recede below neutral, so the bed sits \
             under the melody — the S42 over-loud Fill is corrected)"
        );
        assert!(
            w > 0.25,
            "melody_forward.{role} {w:.3} must be > 0.25 (the accompaniment must support, not vanish)"
        );
    }
    assert!(
        (bass - 0.5).abs() < 1e-6,
        "melody_forward.Bass {bass:.3} must be exactly neutral 0.5 (the structural foundation \
         neither lifts nor recedes)"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// GUARD 5 — AUDIBLE FIGURE/GROUND GAP (the melody, not the Fill, is loudest on accented steps)
// ─────────────────────────────────────────────────────────────────────────────

/// MUSICAL PROPERTY: the melody's biased realized velocity ≥ the loudest bed role's
/// realized velocity on accented (interior) steps, on BOTH renders. The S42 finding
/// was that the gap was INVERTED — the HarmonicFill floated to the top of the dynamic
/// field and was the loudest role. This guard asserts the fix: with the resolved
/// prominence in place the Melody leads. Realized through the actual realizer, fed the
/// prominence Vec RESOLVED from each real image (so these are the renders' real
/// velocity levels). The melody is asserted ≥ EVERY other role (it must be THE
/// foreground), with the HarmonicFill called out explicitly because it was the S42
/// culprit.
#[test]
fn guard5_audible_figure_ground_velocity_gap_on_both_renders() {
    let m = mappings();
    let planner = CompositionPlanner::new(plan_mappings(&m));
    // An accented interior step; low edge so the melody is a clean figure, mid brightness.
    let step = interior_step(2);
    let f = perf(60.0, 0.20);

    for path in [EXAMPLE_IMG, LENA_IMG] {
        let prom = resolved_prominence_per_section(&planner, &m, path)
            .into_iter()
            .next()
            .expect("at least one section");
        let profile = ensemble(prom);

        // inst layout: 0=Bass, 1=HarmonicFill, 2=Pad, 3=CounterMelody, 4=Melody.
        let mel = mean_vel(&realize_under(&profile, &step, 4, 5, &f));
        let bass = mean_vel(&realize_under(&profile, &step, 0, 5, &f));
        let fill = mean_vel(&realize_under(&profile, &step, 1, 5, &f));
        let pad = mean_vel(&realize_under(&profile, &step, 2, 5, &f));
        let cmel = mean_vel(&realize_under(&profile, &step, 3, 5, &f));

        // "loudest bed role" = the loudest NON-melody voice (the masking competitor).
        let loudest_other = [bass, fill, pad, cmel].into_iter().fold(f64::MIN, f64::max);
        eprintln!(
            "[guard5] {path}: Melody={mel:.1} | Bass={bass:.1} HarmonicFill={fill:.1} \
             Pad={pad:.1} CounterMelody={cmel:.1} | loudest_other={loudest_other:.1} \
             gap={:.1}",
            mel - loudest_other
        );

        // The fix: the melody is loudest — the figure leads the ground.
        assert!(
            mel >= loudest_other,
            "{path}: Melody velocity {mel:.1} must be >= the loudest accompaniment role \
             {loudest_other:.1} — the figure/ground gap is INVERTED (the S42 regression)"
        );
        // Explicitly: the HarmonicFill (the S42 culprit) no longer floats above the melody.
        assert!(
            mel >= fill,
            "{path}: Melody {mel:.1} must be >= HarmonicFill {fill:.1} — the Fill was the \
             over-loud S42 culprit and must now sit UNDER the line"
        );
        // A real (positive) gap, not a tie, on accented steps — there is an audible figure.
        assert!(
            mel - loudest_other > 0.0,
            "{path}: the Melody must be STRICTLY loudest on the accented step (gap {:.1}) — \
             a guaranteed audible foreground",
            mel - loudest_other
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// GUARD 6 — FREEZE INTACT (belt-and-suspenders sha re-confirmation)
// ─────────────────────────────────────────────────────────────────────────────

/// The engine byte-freeze is owned by prominence_s23.rs::engine_freeze_diff_empty and
/// engine_equivalence.rs (9/9) — this net does NOT reinvent that harness. As a single
/// natural forward guard (per the kickoff's "small assertion referencing the frozen
/// sha is fine"), re-confirm the frozen kernel byte-image is unchanged by the S43
/// JSON-only fix. If `sha256sum` is unavailable the check is inconclusive-but-non-
/// failing (the two suites above remain the authority); a readable mismatch fails loudly.
#[test]
fn guard6_engine_kernel_still_byte_frozen() {
    use std::process::Command;

    const ENGINE_SHA256: &str = "e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261";

    match Command::new("sha256sum").arg("src/engine.rs").output() {
        Ok(out) if out.status.success() => {
            let text = String::from_utf8_lossy(&out.stdout);
            let got = text.split_whitespace().next().unwrap_or("");
            eprintln!("[guard6] src/engine.rs sha256 = {got}");
            assert_eq!(
                got, ENGINE_SHA256,
                "src/engine.rs sha256 moved off the frozen anchor — the S43 fix is JSON-only \
                 and MUST NOT touch the engine kernel"
            );
        }
        _ => {
            eprintln!(
                "[guard6] sha256sum unavailable; deferring to engine_equivalence + \
                 prominence_s23::engine_freeze_diff_empty as the freeze authority"
            );
        }
    }
}

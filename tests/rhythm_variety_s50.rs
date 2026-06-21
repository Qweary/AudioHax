//! tests/rhythm_variety_s50.rs — the S50 RHYTHM-VARIETY RE-RANGE decisive-metric net.
//!
//! THE DEFECT (operator's ear, diag-s49): every generated piece printed the SAME rhythmic
//! motif regardless of input image — distinct images sounded rhythmically IDENTICAL
//! (CROSS-PIECE SAMENESS). The pre-S50 selectors pinned every real photo onto ONE band
//! (DOTTED) + ONE cell (0) + ONE character (ballad) + ONE tempo (the ballad window), so the
//! whole catalogue clapped the same figure.
//!
//! THE FIX (spec-s50-rhythm-variety §2): a data-relative SPREAD on the band-input activity
//! (`band_activity_spread`) and LOWERED character gates (scherzo/march arousal ge 0.34, march
//! valence lt 0.55) so distinct real images SPREAD across the rhythmic surfaces instead of
//! collapsing. NOTE: the cell-cut change was REVERTED to pre-S50 (BROAD 0.33 / BUSY 0.66 /
//! PROFILED 0.66) — `pick_rhythm_cell` runs only on the theme path (complexity >= 0.4), so a
//! PROFILED gate <= 0.4 force-pins all themed images to cell 3; the spread is delivered by the
//! band re-range + character gate, not the cell axis (see docs/review-s50.md NB-1).
//!
//! WHAT THIS NET ENCODES (spec §4 — "distinct images land on distinct rhythmic surfaces"):
//!   (1) SELECTOR-TUPLE DISTINCTNESS (RNG-free): across the six bundled images the
//!       (band, cell, character) tuple takes >= 4 distinct values (pre-S50 was 1). Directional:
//!       the busiest image (`example`, edge 0.719) does NOT share a band with the calmest
//!       (`magic`, edge 0.106).
//!   (2) TEMPO SPREAD (character un-pin proof): the per-image `base_ms_per_step` takes >= 3
//!       distinct values (pre-S50 was pinned to the ballad window).
//!   (3) RENDERED-ONSET-SIGNATURE (audible-surface proof): the per-image dominant melody
//!       onset-count band takes >= 3 distinct values, and no two of the six share the FULL
//!       (band, cell, character) rhythmic signature beyond the unavoidable real-image tie.
//!   F5b GUARD (Part 2 / spec §7 risk 3): see the file-footer note — the cross-six F5b == 0
//!       regression gate is OWNED by `variety_scorecard_s45.rs` (which runs the REAL render over
//!       all six images at seed 42 and HARD-asserts `bg_recession_violations == 0` per image).
//!       It still passes post-re-range; this net does NOT duplicate it.
//!
//! FLOORS DISCIPLINE (mirrors motif_s41's ">=10 distinct gaits"): every >=N floor is set
//! STRICTLY above the pre-S50 collapse value (1 / 1) and at/below the achievable count on the
//! real six, so a future re-collapse onto one band/cell/character/tempo FAILS LOUDLY here.
//!
//! DETERMINISM / RNG DISCIPLINE (same as diversity_s13 / variety_scorecard_s45): the band and
//! onset-signature observables are read off the PURE realizer `realize_step` over a FIXED
//! hand-built plan (NEVER the `set_features_global`/`pick_progression` thread_rng path). The
//! cell/character/tempo observables come from `CompositionPlanner::plan`, whose only RNG is the
//! per-section harmony draw (`pick_progression`); every observable asserted here —
//! `plan.character`, `plan.key_tempo.base_ms_per_step`, and the planner-realized rhythm CELL —
//! is RNG-INDEPENDENT (chosen by the deterministic affect composite + cell selector, not the
//! chord RNG). The composition seed is pinned to ChaCha8 seed 42 before each `plan()` so the
//! run replays identically. No filesystem writes; the only reads are the shipped
//! `assets/mappings.json` + `assets/images/*` (the same as variety_scorecard_s45). < 10s/test.
//!
//! Run under DEFAULT features (the always-on bin needs them; `--no-default-features` cannot RUN
//! this), exactly like variety_scorecard_s45 / motif_s41:
//!     cargo test --test rhythm_variety_s50 -- --nocapture

use std::collections::BTreeSet;

use audiohax::chord_engine::{
    realize_step, resolve_motif_celled, Chord, MotifArchetype, MotifNote, PerfFeatures,
    PhrasePosition, StepPlan,
};
use audiohax::composition::{
    CadenceStrength, CompositionPlan, CompositionPlanner, ImageUnderstanding, KeyTempoPlan,
    OrchestrationProfile, PlanMappings, ResolutionPolicy, Section, StepContext, ThematicRole,
    ThemeVariation,
};
use audiohax::mapping_loader::{load_mappings, MappingTable};
use audiohax::pure_analysis::{load_pure_image, understand_image_pure, PureImageSource};
use audiohax::seed::set_composition_seed;

const MAPPINGS_PATH: &str = "assets/mappings.json";

/// The fixed composition seed (matches variety_scorecard_s45::SEED) so the per-section harmony
/// RNG is pinned; every asserted observable is RNG-independent regardless, but pinning keeps the
/// run byte-stable for `--nocapture` inspection.
const SEED: u64 = 42;

/// `EDGE_ACTIVITY_RANGE_MAX` — convert a whole-image `edge_activity` (0..1) into the raw per-bar
/// edge density the realizer re-normalizes by the SAME /0.05 (the honest whole-image projection,
/// identical to variety_scorecard_s45::perf_for).
const EDGE_RANGE_MAX: f32 = 0.05;

/// The six bundled probe images (the spec §0 "expected landing" table is keyed to these).
const IMAGES: [&str; 6] = [
    "AudioHaxImg1.jpg",
    "AudioHaxImg2.jpg",
    "AudioHaxImg3.jpg",
    "example.jpg",
    "Lena.png",
    "magicstudio-art.jpg",
];

fn mappings() -> MappingTable {
    load_mappings(MAPPINGS_PATH).expect("assets/mappings.json loads")
}

fn plan_mappings(m: &MappingTable) -> PlanMappings {
    m.composition
        .clone()
        .expect("composition block present in mappings.json")
        .into()
}

/// REAL whole-image understanding for a shipped image (variety_scorecard_s45::understand).
fn understand(name: &str) -> ImageUnderstanding {
    let img = load_pure_image(&PureImageSource::Preselected(name.to_string()))
        .unwrap_or_else(|e| panic!("load {name}: {e:?}"));
    understand_image_pure(img.as_rgb()).unwrap_or_else(|e| panic!("understand {name}: {e:?}"))
}

/// The whole-image `PerfFeatures` projection (variety_scorecard_s45::perf_for): the realizer's
/// band ladder re-normalizes `edge_density` by /0.05, so `edge_activity * 0.05` round-trips the
/// whole-image activity back onto the band-input scale the spread then re-spreads.
fn perf_for(u: &ImageUnderstanding) -> PerfFeatures {
    PerfFeatures {
        saturation: u.avg_saturation,
        brightness: u.avg_brightness,
        edge_density: (u.edge_activity * EDGE_RANGE_MAX).clamp(0.0, 1.0),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// The MELODY band observable (RNG-free): the realized onset COUNT of a single
// interior melody step is a faithful proxy for which band the spread selected —
// SUSTAINED → 1, DOTTED/SYNCOPATED → 2, ARPEGGIO → 3. We additionally split
// DOTTED vs SYNCOPATED by the FIRST onset OFFSET (DOTTED attacks on the downbeat
// at 0; SYNCOPATED is delayed by step_ms/4), so the band is fully resolved into a
// 4-valued label without reaching any private fn.
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Band {
    Sustained,
    Dotted,
    Syncopated,
    Arpeggio,
}

/// A fixed, no-cadence, no-pre-cadence interior step on a wide phrase, so the realized melody
/// onset shape is governed purely by the (spread) edge-activity band — exactly the band the
/// re-range moves. `position_in_phrase 0` of a wide phrase avoids the pre-cadence acceleration
/// (which would force a fixed 4-onset arpeggio regardless of band — keyplan_s29's note).
fn band_step() -> StepPlan {
    StepPlan {
        chord: Chord {
            name: "I".to_string(),
            notes: vec![60, 64, 67],
        },
        phrase_index: 0,
        position_in_phrase: 0,
        phrase_len: 8,
        position: PhrasePosition::Interior,
        velocity: 76,
    }
}

/// A behaviour-neutral single-section/identity Section + KeyTempoPlan (the diversity_s13 /
/// keyplan_s29 default): `density 0.5` ⇒ the density nudge is exactly 0.0, identity
/// orchestration ⇒ empty prominence ⇒ `prom_shift 0.0`, so the band ladder sees the bare spread
/// activity against the unshifted cuts — the cleanest read of which band the SPREAD selected.
fn band_section(step: &StepPlan) -> Section {
    Section {
        label: "A".to_string(),
        step_len: 1,
        thematic_role: ThematicRole::Statement,
        key_offset_semitones: 0,
        ms_per_step: 200,
        mode: "Ionian".to_string(),
        progression: vec![],
        theme: None,
        variation: ThemeVariation::Identity,
        boundary_cadence: CadenceStrength::Perfect,
        pivot: false,
        resolution: ResolutionPolicy::Resolve,
        density: 0.5,
        orchestration: OrchestrationProfile::identity(),
        steps: vec![step.clone()],
    }
}

fn band_key_tempo() -> KeyTempoPlan {
    KeyTempoPlan {
        home_root_midi: 60,
        home_mode: "Ionian".to_string(),
        base_ms_per_step: 200,
        key_scheme: vec![0],
        tempo_scheme: vec![200],
    }
}

/// Resolve the melody band the SPREAD selects for an image, RNG-free, off the pure realizer.
/// `num_instruments == 1` ⇒ the lone instrument is the Melody (the foreground line under test).
fn band_for(u: &ImageUnderstanding) -> Band {
    const MS: u64 = 200;
    let step = band_step();
    let sec = band_section(&step);
    let kt = band_key_tempo();
    let ctx = StepContext::single_section_default(&sec, &kt);
    let events = realize_step(&step, 0, 1, &perf_for(u), MS, &ctx);
    match events.len() {
        1 => Band::Sustained,
        3 | 4 => Band::Arpeggio,
        2 => {
            // DOTTED attacks on the downbeat (offset 0); SYNCOPATED is delayed step_ms/4.
            if events[0].offset_ms == 0 {
                Band::Dotted
            } else {
                Band::Syncopated
            }
        }
        n => panic!("unexpected melody onset count {n} for {events:?}"),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// The CELL observable (planner-realized): identify the (archetype, cell) of the
// stored theme motif by matching the FULL (degree, dur_steps) line against the
// vocabulary at the planner's own range/length formulas (the motif_s41 trick). A
// usize::MAX sentinel means "no theme realized" (complexity < 0.4 ⇒ theme absent;
// composition.rs:1478) — an HONEST cell value, not a guess.
// ─────────────────────────────────────────────────────────────────────────────

const ALL_ARCHETYPES: [MotifArchetype; 8] = [
    MotifArchetype::Arch,
    MotifArchetype::InvertedArch,
    MotifArchetype::Descent,
    MotifArchetype::Ascent,
    MotifArchetype::NeighborTurn,
    MotifArchetype::LeapStep,
    MotifArchetype::Pendulum,
    MotifArchetype::RisingSequence,
];

/// The planner's own pure range/length formulas (composition.rs:1484/1485; mirrored from
/// motif_s41::range_for/length_for).
fn range_for(u: &ImageUnderstanding) -> u8 {
    (2.0 + u.edge_activity * 5.0).round() as u8
}
fn length_for(u: &ImageUnderstanding) -> usize {
    (3.0 + u.complexity * 5.0).round() as usize
}

fn identify_cell(stored: &[MotifNote], range: u8, len: usize) -> Option<usize> {
    for &a in &ALL_ARCHETYPES {
        for cell in 0..a.rhythm_cell_count() {
            if resolve_motif_celled(a, range, len, cell) == *stored {
                return Some(cell);
            }
        }
    }
    None
}

/// Sentinel for "no theme realized" — distinct from any real 0..=3 cell index.
const NO_THEME_CELL: usize = usize::MAX;

/// A stable, Ord-able key for a [`Character`] (the enum is `Eq` but not `Ord`, so it cannot key a
/// `BTreeSet` directly). Total over the closed enum; a 1:1 label per variant.
fn char_key(c: audiohax::composition::Character) -> &'static str {
    use audiohax::composition::Character::*;
    match c {
        Ballad => "Ballad",
        Hymn => "Hymn",
        Nocturne => "Nocturne",
        Drone => "Drone",
        March => "March",
        Lament => "Lament",
        Waltz => "Waltz",
        Scherzo => "Scherzo",
        Lilt => "Lilt",
        Gigue => "Gigue",
    }
}

/// The planner-realized rhythm cell for an image: the matched cell index, or NO_THEME_CELL when
/// the image bears no theme (complexity < 0.4). RNG-independent (the cell is chosen by
/// `pick_rhythm_cell`, not the harmony draw).
fn realized_cell(plan: &CompositionPlan, u: &ImageUnderstanding) -> usize {
    if plan.themes.is_empty() {
        return NO_THEME_CELL;
    }
    let stored = &plan.themes[0].motif;
    identify_cell(stored, range_for(u), length_for(u))
        .expect("a realized theme motif must match some (archetype, cell) of the S41 vocabulary")
}

/// The full per-image rhythmic signature, RNG-independent.
struct Surface {
    name: &'static str,
    band: Band,
    cell: usize,
    character: audiohax::composition::Character,
    base_ms: u64,
}

/// Compute every image's rhythmic surface ONCE (planner under the pinned seed + the pure-realizer
/// band read). Re-seeds before each plan so the run is reproducible.
fn surfaces() -> Vec<Surface> {
    let m = mappings();
    let planner = CompositionPlanner::new(plan_mappings(&m));
    IMAGES
        .iter()
        .map(|&name| {
            let u = understand(name);
            set_composition_seed(Some(SEED));
            let plan = planner.plan(&u, &m);
            let s = Surface {
                name,
                band: band_for(&u),
                cell: realized_cell(&plan, &u),
                character: plan.character,
                base_ms: plan.key_tempo.base_ms_per_step,
            };
            eprintln!(
                "{:20} edge={:.3} cplx={:.3} -> band {:?} cell {} char {:?} base_ms {}",
                s.name,
                u.edge_activity,
                u.complexity,
                s.band,
                if s.cell == NO_THEME_CELL {
                    "NONE".to_string()
                } else {
                    s.cell.to_string()
                },
                s.character,
                s.base_ms,
            );
            s
        })
        .collect()
}

// ═════════════════════════════════════════════════════════════════════════════
// (1) SELECTOR-TUPLE DISTINCTNESS — the decisive cross-image metric (spec §4(1)).
// ═════════════════════════════════════════════════════════════════════════════

/// MUSICAL PROPERTY: distinct images land on distinct rhythmic surfaces. Across the six bundled
/// images the (band, cell, character) tuple takes >= 4 distinct values (pre-S50 collapse = 1).
/// FLOOR RATIONALE (>=4): the achievable count on the real six is 5 (Img3 and Lena legitimately
/// tie — both DOTTED band, no theme, Scherzo; they are genuinely similar mid-cluster photos); 4
/// is strictly above the pre-S50 value of 1 and below the achievable 5, so a future re-collapse
/// onto one surface FAILS LOUDLY (mirrors motif_s41's ">=10 distinct gaits" discipline).
#[test]
fn distinct_images_land_on_distinct_rhythmic_surfaces() {
    let s = surfaces();

    let tuples: BTreeSet<(Band, usize, &'static str)> = s
        .iter()
        .map(|x| (x.band, x.cell, char_key(x.character)))
        .collect();

    assert!(
        tuples.len() >= 4,
        "S50 CROSS-PIECE SAMENESS GATE FAILED: the six images realize only {} distinct \
         (band, cell, character) tuples, need >=4 (pre-S50 collapse was 1). Surfaces: {:?}",
        tuples.len(),
        s.iter()
            .map(|x| (x.name, x.band, x.cell, x.character))
            .collect::<Vec<_>>(),
    );

    // DIRECTIONAL SANITY: the busiest image must NOT share a band with the calmest — the spread
    // must actually fan the activity cluster apart at its two ends.
    let busiest = s
        .iter()
        .find(|x| x.name == "example.jpg")
        .expect("example.jpg in the set");
    let calmest = s
        .iter()
        .find(|x| x.name == "magicstudio-art.jpg")
        .expect("magicstudio-art.jpg in the set");
    assert_ne!(
        busiest.band, calmest.band,
        "the busiest image (example, edge 0.719) must NOT share a band with the calmest \
         (magic, edge 0.106): busiest {:?} calmest {:?}",
        busiest.band, calmest.band
    );
    // Stronger directional pin: busiest is more active (more onsets) than calmest.
    assert!(
        busiest.band == Band::Arpeggio && calmest.band == Band::Sustained,
        "directional: example must reach the ARPEGGIO (busiest) band and magic the SUSTAINED \
         (calmest) band — the spread fanning the cluster across the full range; got example {:?} \
         magic {:?}",
        busiest.band,
        calmest.band
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// (2) TEMPO SPREAD — the character un-pin proof (spec §4(3)).
// ═════════════════════════════════════════════════════════════════════════════

/// MUSICAL PROPERTY: the character/tempo path is no longer pinned to the ballad window. The
/// per-image `base_ms_per_step` takes >= 3 distinct values across the six (pre-S50 it was
/// effectively pinned). FLOOR RATIONALE (>=3): the achievable count on the real six is 6 (every
/// image lands on a distinct tempo); 3 is strictly above the pre-S50 effective 1 and well below
/// 6, failing loudly on a re-pin. This directly observes the lowered character gates routing
/// images out of ballad into scherzo/march/hymn/lament windows.
#[test]
fn tempo_spreads_across_the_six() {
    let s = surfaces();
    let tempos: BTreeSet<u64> = s.iter().map(|x| x.base_ms).collect();
    assert!(
        tempos.len() >= 3,
        "TEMPO RE-PIN GATE FAILED: the six images take only {} distinct base_ms_per_step \
         value(s), need >=3 (pre-S50 was pinned to the ballad window). Tempos: {:?}",
        tempos.len(),
        s.iter().map(|x| (x.name, x.base_ms)).collect::<Vec<_>>(),
    );

    // The character un-pin also means MORE than one CHARACTER is selected (ballad is no longer
    // universal) — a second, independent witness of the same un-pin.
    let characters: BTreeSet<&'static str> = s.iter().map(|x| char_key(x.character)).collect();
    assert!(
        characters.len() >= 3,
        "CHARACTER RE-PIN GATE FAILED: the six images select only {} distinct character(s), \
         need >=3 (pre-S50 was always ballad). Characters: {:?}",
        characters.len(),
        s.iter().map(|x| (x.name, x.character)).collect::<Vec<_>>(),
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// (3) RENDERED-ONSET-SIGNATURE — the audible-surface proof (spec §4(2)).
// ═════════════════════════════════════════════════════════════════════════════

/// MUSICAL PROPERTY (the audible cross-piece-sameness gate): the per-image DOMINANT melody band
/// — the rhythmic surface a listener actually hears — takes >= 3 distinct values across the six,
/// and no two images share the FULL (band, cell, character) signature beyond the single
/// unavoidable real-image tie (Img3 ~ Lena). FLOOR RATIONALE (>=3 bands): the achievable distinct
/// band count on the real six is 4 (SUSTAINED / DOTTED / SYNCOPATED / ARPEGGIO — the spread fans
/// all four bands open); 3 is strictly above the pre-S50 value of 1 (every image read DOTTED → 1
/// dominant pattern) and one below the achievable 4, so it fails loudly on a re-collapse while
/// tolerating a single image drifting band under future tuning.
#[test]
fn rendered_onset_signatures_are_not_all_the_same() {
    let s = surfaces();

    // Distinct dominant onset bands (the audible rhythmic surface).
    let bands: BTreeSet<Band> = s.iter().map(|x| x.band).collect();
    assert!(
        bands.len() >= 3,
        "RENDERED-SAMENESS GATE FAILED: the six images produce only {} distinct dominant melody \
         band(s), need >=3 (pre-S50 every image read DOTTED → 1). Bands: {:?}",
        bands.len(),
        s.iter().map(|x| (x.name, x.band)).collect::<Vec<_>>(),
    );

    // No two images share the FULL rhythmic signature beyond the ONE documented real-image tie.
    // We assert: at most ONE pair collides on the full (band, cell, character) signature, and that
    // pair is the genuinely-similar Img3/Lena mid-cluster pair. A SECOND collision would mean the
    // selectors re-collapsed two further images onto one surface.
    let mut collisions: Vec<(&str, &str)> = Vec::new();
    for i in 0..s.len() {
        for j in (i + 1)..s.len() {
            if (s[i].band, s[i].cell, s[i].character) == (s[j].band, s[j].cell, s[j].character) {
                collisions.push((s[i].name, s[j].name));
            }
        }
    }
    assert!(
        collisions.len() <= 1,
        "CROSS-PIECE SIGNATURE COLLISION: more than the one documented real-image tie share the \
         FULL (band, cell, character) signature — the selectors re-collapsed. Collisions: {:?}",
        collisions
    );
    if let Some(&(a, b)) = collisions.first() {
        let pair: BTreeSet<&str> = [a, b].into_iter().collect();
        let expected: BTreeSet<&str> = ["AudioHaxImg3.jpg", "Lena.png"].into_iter().collect();
        assert_eq!(
            pair, expected,
            "the single permitted signature tie must be the documented Img3 ~ Lena mid-cluster \
             pair (both DOTTED / no-theme / Scherzo); got {a} ~ {b}"
        );
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// DETERMINISM — every asserted observable replays identically (the RNG discipline).
// ═════════════════════════════════════════════════════════════════════════════

/// MUSICAL PROPERTY: every observable this net asserts is a deterministic function of the image
/// (the band off the pure realizer; the cell/character/tempo off the seeded planner, all
/// RNG-independent of the harmony draw). Two passes over the six produce byte-identical surfaces —
/// the same boundary discipline as diversity_s13::test_diversity_observables_are_deterministic.
#[test]
fn rhythm_surfaces_are_deterministic() {
    let a = surfaces();
    let b = surfaces();
    let sig = |s: &[Surface]| {
        s.iter()
            .map(|x| (x.name, x.band, x.cell, x.character, x.base_ms))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        sig(&a),
        sig(&b),
        "the per-image rhythmic surface must be deterministic (band/cell/character/tempo are all \
         RNG-independent of the per-section harmony draw)"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// PART 2 — F5b GUARD (spec §7 risk 3): NOT duplicated here.
//
// The cross-six F5b regression gate (the S46/S49 figure-ground gain — the melody is never
// QUIETER, i.e. fewer onsets, than the bed) is OWNED by `tests/variety_scorecard_s45.rs`, which
// runs the REAL whole-plan render over all six bundled images at seed 42 and HARD-asserts
// `bg_recession_violations == 0` per image (its `f5b_residual_bound` returns 0 for every one of
// the six). Confirmed STILL GREEN after the S50 re-range (variety_scorecard_s45 3/3). The S50
// band spread is mirrored inside `melody_activity_class` (chord_engine.rs:1147), so the governor
// still sees the melody's true SPREAD class and F5b holds — exactly the composition the spec §7
// risk-3 mitigation relies on. Re-implementing the full per-role render here would only duplicate
// that gate, so per the task's "confirm it still passes and note that instead of duplicating"
// instruction, this net defers to variety_scorecard_s45 as the authoritative F5b guard.
// ─────────────────────────────────────────────────────────────────────────────

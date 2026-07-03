//! tests/keydiag_s64.rs — S64 STEP-1 DIAGNOSTIC PROBE (READ-ONLY, structural asserts only).
//!
//! Prints, per corpus image: dominant_hue, resolved home_root_midi (+ pitch-class name),
//! home_mode, form pick, per-section step_len, and the per-section phrase decomposition
//! (phrase_len set + cadence-position layout). This backs the S64 fixed-rhythm/key diagnostic
//! (docs/diagnostic-s64-fixed-rhythm-key.md) with the ACTUAL per-image selections.
//!
//! CROSS-PLATFORM SAFE: asserts ONLY structural/integer invariants (home root ∈ {60}∪[57,68];
//! phrase_len ∈ 1..=8; corpus fully processed). NO per-image float audio metrics (macOS libm
//! diverges). The PRINTED values are diagnostic; the ASSERTS are the platform-stable contract.
//!
//! Run: `cargo test --release --test keydiag_s64 -- --nocapture` (DEFAULT features).

use audiohax::chord_engine::PhrasePosition;
use audiohax::composition::{
    CompositionPlan, CompositionPlanner, ImageUnderstanding, PlanMappings,
};
use audiohax::mapping_loader::{load_mappings, MappingTable};
use audiohax::pure_analysis::{load_pure_image, understand_image_pure, PureImageSource};
use audiohax::seed::set_composition_seed;
use std::collections::BTreeSet;

const MAPPINGS_PATH: &str = "assets/mappings.json";
const SEED: u64 = 42;
const IMAGES: &[&str] = &[
    "AudioHaxImg1.jpg",
    "AudioHaxImg2.jpg",
    "AudioHaxImg3.jpg",
    "example.jpg",
    "Lena.png",
    "magicstudio-art.jpg",
];

const PC_NAMES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

fn mappings() -> MappingTable {
    load_mappings(MAPPINGS_PATH).expect("assets/mappings.json loads")
}
fn plan_mappings(m: &MappingTable) -> PlanMappings {
    m.composition
        .clone()
        .expect("composition block present")
        .into()
}
fn understanding_for(name: &str) -> ImageUnderstanding {
    let img = load_pure_image(&PureImageSource::Preselected(name.to_string()))
        .unwrap_or_else(|e| panic!("load {name}: {e:?}"));
    understand_image_pure(img.as_rgb()).unwrap_or_else(|e| panic!("understand {name}: {e:?}"))
}

fn plan_for(
    name: &str,
    m: &MappingTable,
    pm: &PlanMappings,
) -> (ImageUnderstanding, CompositionPlan) {
    let u = understanding_for(name);
    let planner = CompositionPlanner::new(pm.clone());
    set_composition_seed(Some(SEED));
    let plan = planner.plan(&u, m);
    (u, plan)
}

fn pos_tag(p: PhrasePosition) -> char {
    match p {
        PhrasePosition::PhraseStart => 'S',
        PhrasePosition::Interior => '.',
        PhrasePosition::HalfCadence => 'h',
        PhrasePosition::PerfectAuthenticCadence => 'P',
    }
}

#[test]
fn keydiag_s64_probe() {
    let m = mappings();
    let pm = plan_mappings(&m);

    println!("\n================= S64 KEY / FORM / PHRASE PROBE =================");
    println!("corpus: {} images | seed {SEED}", IMAGES.len());

    let mut home_roots: Vec<u8> = Vec::new();
    let mut forms: Vec<String> = Vec::new();
    let mut all_phrase_lens: BTreeSet<usize> = BTreeSet::new();

    for name in IMAGES {
        let (u, plan) = plan_for(name, &m, &pm);
        let hr = plan.key_tempo.home_root_midi;
        let pc = (hr % 12) as usize;
        home_roots.push(hr);
        forms.push(plan.form.clone());

        println!("\n── {name}");
        println!(
            "   dominant_hue={:.1}  avg_brightness={:.1}  edge_activity={:.3}  complexity={:.3}",
            u.dominant_hue, u.avg_brightness, u.edge_activity, u.complexity
        );
        println!(
            "   home_root_midi={hr} ({}{})   home_mode={}   form={}   n_sections={}",
            PC_NAMES[pc],
            hr / 12 - 1, // octave number (MIDI 60 = C4)
            plan.key_tempo.home_mode,
            plan.form,
            plan.sections.len()
        );
        for (si, sec) in plan.sections.iter().enumerate() {
            // Phrase-length set + cadence layout string for this section.
            let mut plens: BTreeSet<usize> = BTreeSet::new();
            let layout: String = sec
                .steps
                .iter()
                .map(|s| {
                    plens.insert(s.phrase_len);
                    all_phrase_lens.insert(s.phrase_len);
                    pos_tag(s.position)
                })
                .collect();
            println!(
                "     sec[{si}] '{}' step_len={} phrase_lens={:?} key_off={} layout=[{}]",
                sec.label,
                sec.steps.len(),
                plens,
                sec.key_offset_semitones,
                layout
            );
        }
    }

    // ── Cross-corpus summary (the diagnostic payload). ──
    let distinct_roots: BTreeSet<u8> = home_roots.iter().copied().collect();
    let distinct_forms: BTreeSet<&String> = forms.iter().collect();
    println!("\n── SUMMARY");
    println!("   home_root_midi per image: {home_roots:?}");
    println!(
        "   distinct home roots: {} → {:?}",
        distinct_roots.len(),
        distinct_roots
            .iter()
            .map(|&r| format!("{}{}", PC_NAMES[(r % 12) as usize], r / 12 - 1))
            .collect::<Vec<_>>()
    );
    println!("   all-C4-default? {}", home_roots.iter().all(|&r| r == 60));
    println!("   form per image: {forms:?}");
    println!(
        "   distinct forms: {} → {distinct_forms:?}",
        distinct_forms.len()
    );
    println!("   phrase_len set across ALL sections of ALL images: {all_phrase_lens:?}");
    println!("================================================================\n");

    // ── STRUCTURAL asserts (platform-stable; NO float audio metrics). ──
    assert_eq!(
        home_roots.len(),
        IMAGES.len(),
        "every image must resolve a home root"
    );
    for &r in &home_roots {
        // Either the wired [57,68] band representative, or the legacy C4 (60) fallback.
        assert!(
            r == 60 || (57..=68).contains(&r),
            "home root {r} outside the wired band [57,68] and not the legacy 60"
        );
    }
    // The NOMINAL phrase period is 4 or 8 (PHRASE_LENGTHS); a section's SHORT FINAL phrase carries
    // the remainder (this_len), so 2/3 are legitimately observed. The stable invariant is the bound.
    for &pl in &all_phrase_lens {
        assert!(
            (1..=8).contains(&pl),
            "phrase_len {pl} outside the PHRASE_LENGTHS bound 1..=8"
        );
    }
}

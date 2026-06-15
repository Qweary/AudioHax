//! tests/diversity_s13.rs — the cross-cutting "distinct image → distinct music"
//! DIVERSITY property net for design-s13 (Lane 2: image→music diversity).
//!
//! This is DISTINCT from the implementers' focused per-function unit tests. It
//! asserts the END-TO-END property the operator's complaint encodes: two sharply
//! different images must yield music that differs across MULTIPLE musical
//! dimensions simultaneously — not just mode. It is the §6.6 headline gate.
//!
//! Construction discipline (mirrors engine_seam.rs / tui_render.rs): build the
//! `engine::GlobalFeatures` / `chord_engine` value structs directly (plain f32
//! fields) and drive the engine via its PUBLIC API:
//!   * tempo            — `set_features_global` + `engine.config().ms_per_step`
//!   * harmonic content — `ChordEngine::generate_chords` on an explicit progression
//!   * articulation     — `chord_engine::realize_step` over a hand-built StepPlan
//! No OpenCV, no real image, no filesystem (mappings load from assets/ only).
//!
//! RNG discipline: the harmony asserts NEVER go through the `set_features_global`
//! → `pick_progression` (`thread_rng`) path — they call `generate_chords` on a
//! FIXED progression so every assertion is a deterministic function of the
//! feature vector (same boundary discipline as engine_equivalence / tui_render).
//! Any place a seed is conceptually needed it is a ChaCha8Rng; in practice these
//! tests need none because they avoid the RNG path entirely.

use audiohax::chord_engine::{
    realize_step, Chord, ChordEngine, NoteEvent, PerfFeatures, PhrasePosition, StepPlan,
};
use audiohax::engine::{
    CadenceStrength, EngineConfig, GlobalFeatures, KeyTempoPlan, PipelineEngine, Section,
    StepContext, ThematicRole, ThemeVariation,
};
use audiohax::mapping_loader::{load_mappings, MappingTable};

/// S15 seam: a behaviour-neutral default Section/KeyTempoPlan for the articulation
/// `realize_step` call sites. `theme:None` ⇒ the realizer takes its existing
/// free-select melody path, so every articulation/diversity assertion below is
/// byte-identical to the pre-seam behaviour — only the `ctx` argument is added,
/// no expected value changes. (Mirrors engine_equivalence.rs's default.)
fn div_section(plan: &[StepPlan]) -> Section {
    Section {
        label: "A".to_string(),
        step_len: plan.len(),
        thematic_role: ThematicRole::Statement,
        key_offset_semitones: 0,
        ms_per_step: 200,
        mode: "Ionian".to_string(),
        progression: vec![],
        theme: None,
        variation: ThemeVariation::Identity,
        boundary_cadence: CadenceStrength::Perfect,
        density: 0.5,
        steps: plan.to_vec(),
    }
}

fn div_key_tempo() -> KeyTempoPlan {
    KeyTempoPlan {
        home_root_midi: 60,
        home_mode: "Ionian".to_string(),
        base_ms_per_step: 200,
        key_scheme: vec![0],
        tempo_scheme: vec![200],
    }
}

const MAPPINGS: &str = "assets/mappings.json";

fn mappings() -> MappingTable {
    load_mappings(MAPPINGS).expect("mappings load")
}

// ─────────────────────────────────────────────────────────────────────────────
// The two sharply-distinct feature vectors (design-s13 §6 fixture).
// A and B share avg_hue=40 ON PURPOSE: proving diversity comes from the OTHER
// axes (tempo / saturation / edge / texture / shape / spread), not just mode.
// ─────────────────────────────────────────────────────────────────────────────

/// CALM / DARK image: low on every driven axis.
fn global_calm_dark() -> GlobalFeatures {
    GlobalFeatures {
        avg_hue: 40.0,
        avg_saturation: 20.0,
        avg_brightness: 25.0,
        edge_density: 0.004,
        hue_spread: 0.05,
        texture_laplacian_var: 300.0,
        shape_complexity: 0.02,
        aspect_ratio: 1.0,
    }
}

/// VIVID / BUSY image: high on every driven axis.
fn global_vivid_busy() -> GlobalFeatures {
    GlobalFeatures {
        avg_hue: 40.0, // same hue as calm — diversity must come from elsewhere
        avg_saturation: 90.0,
        avg_brightness: 85.0,
        edge_density: 0.040,
        hue_spread: 0.65,
        texture_laplacian_var: 1900.0,
        shape_complexity: 1.9,
        aspect_ratio: 1.0,
    }
}

/// A small, stable diatonic progression to drive harmony asserts through the
/// deterministic `generate_chords` path (never the RNG `pick_progression`).
fn progression() -> Vec<String> {
    ["I", "vi", "IV", "V"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers that derive the realized musical observables from a feature vector.
// ─────────────────────────────────────────────────────────────────────────────

/// The ms_per_step a whole image resolves to after `set_features_global`.
/// (Pure w.r.t. tempo: tempo is brightness-derived, no RNG involved.)
fn tempo_ms_for(global: &GlobalFeatures) -> u64 {
    let mut engine = PipelineEngine::new(mappings(), EngineConfig::default());
    engine.set_features_global(global);
    engine.config().ms_per_step
}

/// Build a fixed plan of N interior melody steps (no cadence) so the realized
/// note length is governed by the S13 articulation curve, not the cadence ring.
/// `phrase_len` is set wide and every step is Interior so none becomes a cadence.
fn interior_plan(n: usize) -> Vec<StepPlan> {
    (0..n)
        .map(|i| StepPlan {
            chord: Chord {
                name: "I".to_string(),
                notes: vec![60, 64, 67],
            },
            phrase_index: 0,
            position_in_phrase: i + 1, // +1 so position 0 isn't PhraseStart-special; all Interior
            phrase_len: n + 4,
            position: PhrasePosition::Interior,
            velocity: 76,
        })
        .collect()
}

/// PerfFeatures (the per-step music-domain projection) for a given raw per-bar
/// edge density. saturation/brightness are held mid so only edge varies.
fn perf(edge_density: f32) -> PerfFeatures {
    PerfFeatures {
        saturation: 50.0,
        brightness: 50.0,
        edge_density,
    }
}

/// Mean realized hold FRACTION (hold_ms / ms_per_step) of the melody role over a
/// fixed plan at a given per-bar edge density. Uses a FIXED ms_per_step so tempo
/// never confounds the articulation fraction. Single-instrument ⇒ Melody role.
fn mean_note_frac(edge_density: f32, ms_per_step: u64) -> f32 {
    let plan = interior_plan(4);
    let f = perf(edge_density);
    let kt = div_key_tempo();
    let sec = div_section(&plan);
    let ctx = StepContext::single_section_default(&sec, &kt);
    let mut total = 0.0f32;
    let mut count = 0usize;
    for step in &plan {
        // num_instruments == 1 ⇒ the lone instrument is the Melody (lone line = tune).
        let events = realize_step(step, 0, 1, &f, ms_per_step, &ctx);
        for ev in &events {
            total += ev.hold_ms as f32 / ms_per_step as f32;
            count += 1;
        }
    }
    assert!(count > 0, "the plan must sound at least one melody note");
    total / count as f32
}

/// The multiset (as a sorted Vec) of onset COUNTS per step for the melody over a
/// fixed plan at a given per-bar edge density. Captures the rhythm DISTRIBUTION.
fn onset_count_multiset(edge_density: f32, ms_per_step: u64) -> Vec<usize> {
    let plan = interior_plan(4);
    let f = perf(edge_density);
    let kt = div_key_tempo();
    let sec = div_section(&plan);
    let ctx = StepContext::single_section_default(&sec, &kt);
    let mut counts: Vec<usize> = plan
        .iter()
        .map(|step| realize_step(step, 0, 1, &f, ms_per_step, &ctx).len())
        .collect();
    counts.sort_unstable();
    counts
}

/// The maximum chord-tone count produced by `generate_chords` for a feature
/// vector — the harmonic-complexity observable (3 = triad, 4 = 7th, 5 = 9th).
fn max_chord_tone_count(global: &GlobalFeatures) -> usize {
    let eng = ChordEngine::new(mappings());
    // Use Ionian mode + a low edge so the secondary dominant doesn't perturb the
    // count, isolating the diatonic harmonic-complexity axis. brightness_drop=0.0.
    let chords = eng.generate_chords(
        &progression(),
        60,
        "Ionian",
        0.0,                   // raw edge → no secondary dominant
        0.0,                   // no modal-interchange drop
        global.avg_saturation, // raw saturation → harmonic complexity
        global.hue_spread,     // raw colorfulness → mode mixture
    );
    chords.iter().map(|c| c.notes.len()).max().unwrap_or(0)
}

/// A signature of the borrowed/mixture content of a chord set: the sorted set of
/// chord NAMES that are NOT plain home-key diatonic numerals. Captures the
/// mode-mixture / secondary-dominant "colour" axis.
fn mixture_signature(chords: &[Chord]) -> Vec<String> {
    let diatonic = ["i", "ii", "iii", "iv", "v", "vi", "vii"];
    let mut names: Vec<String> = chords
        .iter()
        .map(|c| c.name.clone())
        .filter(|n| !diatonic.contains(&n.to_lowercase().as_str()))
        .collect();
    names.sort();
    names.dedup();
    names
}

// ═════════════════════════════════════════════════════════════════════════════
// THE HEADLINE GATE (§6.6): distinct images differ in ≥3 musical dimensions.
// ═════════════════════════════════════════════════════════════════════════════

/// MUSICAL PROPERTY: a CALM/DARK image and a VIVID/BUSY image produce music that
/// differs in at least THREE musical dimensions simultaneously — tempo,
/// articulation, harmonic content, rhythm distribution, mixture colour — not just
/// mode. This is the operator's "different image ⇒ different music in more than
/// one dimension" complaint encoded as the session's acceptance gate.
#[test]
fn test_distinct_images_differ_in_3_dimensions() {
    let calm = global_calm_dark();
    let busy = global_vivid_busy();

    // Fixed tempo for the articulation/rhythm observables so tempo does not
    // confound the per-step fraction or onset counts.
    const FIXED_MS: u64 = 500;

    // 1) TEMPO
    let ms_calm = tempo_ms_for(&calm);
    let ms_busy = tempo_ms_for(&busy);
    let tempo_differs = ms_calm != ms_busy;

    // 2) ARTICULATION / mean note-length (per-bar edge as the busyness proxy).
    let frac_calm = mean_note_frac(calm.edge_density, FIXED_MS);
    let frac_busy = mean_note_frac(busy.edge_density, FIXED_MS);
    let artic_differs = (frac_calm - frac_busy).abs() > 1e-3;

    // 3) HARMONIC CONTENT (chord-tone count via saturation).
    let tones_calm = max_chord_tone_count(&calm);
    let tones_busy = max_chord_tone_count(&busy);
    let harmony_differs = tones_calm != tones_busy;

    // 4) RHYTHM distribution (onset-count multiset).
    let onsets_calm = onset_count_multiset(calm.edge_density, FIXED_MS);
    let onsets_busy = onset_count_multiset(busy.edge_density, FIXED_MS);
    let rhythm_differs = onsets_calm != onsets_busy;

    // 5) MIXTURE colour (borrowed-chord signature via colorfulness/edge).
    let eng = ChordEngine::new(mappings());
    let chords_calm = eng.generate_chords(
        &progression(),
        60,
        "Ionian",
        calm.edge_density,
        0.0,
        calm.avg_saturation,
        calm.hue_spread,
    );
    let chords_busy = eng.generate_chords(
        &progression(),
        60,
        "Ionian",
        busy.edge_density,
        0.0,
        busy.avg_saturation,
        busy.hue_spread,
    );
    let mixture_differs = mixture_signature(&chords_calm) != mixture_signature(&chords_busy);

    let dimensions = [
        ("tempo", tempo_differs),
        ("articulation", artic_differs),
        ("harmony", harmony_differs),
        ("rhythm", rhythm_differs),
        ("mixture", mixture_differs),
    ];
    let differing: Vec<&str> = dimensions
        .iter()
        .filter(|(_, d)| *d)
        .map(|(n, _)| *n)
        .collect();

    assert!(
        differing.len() >= 3,
        "HEADLINE GATE FAILED: distinct images differ in only {} dimension(s) {:?}, need >=3.\n\
         tempo: calm={ms_calm} busy={ms_busy}\n\
         articulation: calm_frac={frac_calm:.4} busy_frac={frac_busy:.4}\n\
         harmony: calm_tones={tones_calm} busy_tones={tones_busy}\n\
         rhythm: calm_onsets={onsets_calm:?} busy_onsets={onsets_busy:?}\n\
         mixture: calm={:?} busy={:?}",
        differing.len(),
        differing,
        mixture_signature(&chords_calm),
        mixture_signature(&chords_busy),
    );

    // Directional sanity on the two strongest axes (not part of the >=3 count but
    // a high-value guard): brighter ⇒ faster ⇒ SMALLER ms; vivider ⇒ MORE tones.
    if tempo_differs {
        assert!(
            ms_busy < ms_calm,
            "brighter (busy) image must be FASTER (smaller ms_per_step): calm={ms_calm} busy={ms_busy}"
        );
    }
    if harmony_differs {
        assert!(
            tones_busy > tones_calm,
            "vivid (busy) image must have MORE chord tones: calm={tones_calm} busy={tones_busy}"
        );
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// DIRECTIONAL / MONOTONICITY property tests (deterministic).
// ═════════════════════════════════════════════════════════════════════════════

/// MUSICAL PROPERTY: tempo is MONOTONIC in brightness — a dark→mid→bright sweep
/// yields non-increasing ms_per_step (brighter is never slower). Confirms the
/// continuous brightness→BPM interpolation drives tempo per-image.
#[test]
fn test_tempo_monotonic_in_brightness() {
    let mk = |bright: f32| {
        let mut g = global_calm_dark();
        g.avg_brightness = bright;
        tempo_ms_for(&g)
    };
    let ms_dark = mk(20.0);
    let ms_mid = mk(55.0);
    let ms_bright = mk(90.0);

    assert!(
        ms_dark >= ms_mid && ms_mid >= ms_bright,
        "ms_per_step must be NON-INCREASING with brightness (brighter=faster): \
         dark={ms_dark} mid={ms_mid} bright={ms_bright}"
    );
    // And it must actually MOVE across the full sweep, not be constant.
    assert!(
        ms_dark > ms_bright,
        "tempo must change across the brightness sweep: dark={ms_dark} bright={ms_bright}"
    );
}

/// MUSICAL PROPERTY: harmonic complexity is MONOTONIC in saturation — low→triad
/// (3 tones), mid→7th (4 tones), high→9th (5 tones). The core "computer triads"
/// fix: vivider images earn richer harmony.
#[test]
fn test_harmonic_complexity_monotonic_in_saturation() {
    let eng = ChordEngine::new(mappings());
    let tones_for = |sat: f32| {
        let chords = eng.generate_chords(&progression(), 60, "Ionian", 0.0, 0.0, sat, 0.0);
        // Take the I chord (index 0) tone count — the unambiguous complexity readout.
        chords[0].notes.len()
    };
    let low = tones_for(15.0); // <31% → triad
    let mid = tones_for(50.0); // 31..71% → +7th
    let high = tones_for(90.0); // >=71% → +9th

    assert_eq!(
        low, 3,
        "low saturation must be a bare triad (3 tones), got {low}"
    );
    assert_eq!(
        mid, 4,
        "mid saturation must add the 7th (4 tones), got {mid}"
    );
    assert_eq!(
        high, 5,
        "high saturation must add the 9th (5 tones), got {high}"
    );
    assert!(
        low <= mid && mid <= high,
        "chord-tone count must be NON-DECREASING with saturation: {low} {mid} {high}"
    );
}

/// MUSICAL PROPERTY (the "uniformly short notes" killer): a SMOOTH edge sweep
/// yields SMOOTHLY-VARYING note length, NOT three discrete buckets. We sweep
/// per-bar edge_density finely across the realistic range and require MANY
/// distinct realized note-length values (continuity), proving the old 3-band
/// cutoff was replaced by a continuous articulation curve.
#[test]
fn test_articulation_is_continuous_not_three_bands() {
    const FIXED_MS: u64 = 1000;
    // Sweep across the CALM/SUSTAINED region where the continuous curve governs the
    // single sustained melody note (edge_activity <= 0.25 ⇒ one note whose hold
    // rides base_frac). 21 fine samples of raw edge in [0.000 .. 0.0125].
    let kt = div_key_tempo();
    let sec = div_section(&interior_plan(1));
    let ctx = StepContext::single_section_default(&sec, &kt);
    let mut holds: Vec<u64> = Vec::new();
    for k in 0..21 {
        let raw_edge = 0.000625 * k as f32; // 0.0 .. 0.0125 → edge_activity 0.0 .. 0.25
        let plan = interior_plan(1);
        let events = realize_step(&plan[0], 0, 1, &perf(raw_edge), FIXED_MS, &ctx);
        assert_eq!(
            events.len(),
            1,
            "the calm/sustained band must emit exactly one melody note at edge={raw_edge}"
        );
        holds.push(events[0].hold_ms);
    }
    let mut distinct = holds.clone();
    distinct.sort_unstable();
    distinct.dedup();

    assert!(
        distinct.len() > 3,
        "articulation must be CONTINUOUS, not 3 bands: a fine edge sweep produced only {} \
         distinct note-length value(s) {:?} — the continuous curve is missing/regressed",
        distinct.len(),
        distinct
    );
    // Adjacent samples should change by SMALL increments (a curve), not jump in
    // big quantized steps. The largest adjacent jump over the fine sweep must be a
    // small fraction of the step (a 3-band step function would jump ~hundreds of ms).
    let max_adjacent_jump = holds
        .windows(2)
        .map(|w| (w[0] as i64 - w[1] as i64).unsigned_abs())
        .max()
        .unwrap_or(0);
    assert!(
        max_adjacent_jump < (FIXED_MS / 5),
        "adjacent note-length steps must be SMALL (continuous curve), but the largest jump was \
         {max_adjacent_jump} ms over a {FIXED_MS} ms step — looks like a discrete bucket boundary"
    );
    // And the calm end must reach into connected/legato territory (frac >= ~0.95),
    // the thing the old hard 0.95 cap blocked: the longest hold crosses 0.95*step.
    let longest = *holds.iter().max().unwrap();
    assert!(
        longest as f32 >= 0.95 * FIXED_MS as f32,
        "the calmest note must sing (hold >= 95% of the step), got {longest} of {FIXED_MS} ms"
    );
}

/// MUSICAL PROPERTY: articulation DIRECTION — a calm (low-edge) image holds
/// notes LONGER than a busy (high-edge) image. The per-step "played vs typed"
/// cue: calm = legato/long, busy = detached/short.
#[test]
fn test_articulation_calm_longer_than_busy() {
    const FIXED_MS: u64 = 500;
    let frac_calm = mean_note_frac(0.004, FIXED_MS); // ~ the calm fixture's edge
    let frac_busy = mean_note_frac(0.045, FIXED_MS); // saturating-busy edge
    assert!(
        frac_calm > frac_busy,
        "calm image must hold notes LONGER than busy: calm_frac={frac_calm:.4} busy_frac={frac_busy:.4}"
    );
    assert!(
        frac_calm > 0.95,
        "the calm image must actually cross into connected/legato (frac>0.95), got {frac_calm:.4}"
    );
}

/// MUSICAL PROPERTY: NORMALIZATION CALIBRATION — the real-photo edge-density
/// range (~0.005..0.036) maps into a USABLE activity band, exercising more than a
/// single rhythm pattern across that range (not all clamped to one end). If the
/// normalization divisor were wrong, every real photo would collapse to the same
/// "sustained" pattern (the original bug).
#[test]
fn test_normalization_real_photo_edge_range_is_usable() {
    const FIXED_MS: u64 = 600;
    // The six-image measured raw spread from the spec: 0.005 .. 0.036.
    let raw_edges = [0.005f32, 0.012, 0.020, 0.028, 0.036];
    let kt = div_key_tempo();
    let sec = div_section(&interior_plan(1));
    let ctx = StepContext::single_section_default(&sec, &kt);
    let mut patterns: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();
    for &e in &raw_edges {
        let plan = interior_plan(1);
        let events = realize_step(&plan[0], 0, 1, &perf(e), FIXED_MS, &ctx);
        patterns.insert(events.len()); // onset count distinguishes pattern bands
    }
    assert!(
        patterns.len() >= 2,
        "real-photo edge range must select MORE than one rhythm pattern (calibration usable), \
         but only onset-count(s) {:?} appeared across raw edges {:?} — \
         the normalization divisor is mis-calibrated (everything clamped to one band)",
        patterns,
        raw_edges
    );

    // Also: the endpoints of the real range must produce DIFFERENT note lengths
    // (the activity knob is not saturated at one end across the real spread).
    let lo_hold = realize_step(&interior_plan(1)[0], 0, 1, &perf(0.005), FIXED_MS, &ctx)[0].hold_ms;
    // 0.036 still maps below 0.55, so this also stays in the single-note band; if it
    // crosses into a multi-onset band that is itself proof of a usable spread.
    let hi_events = realize_step(&interior_plan(1)[0], 0, 1, &perf(0.036), FIXED_MS, &ctx);
    let usable_spread = hi_events.len() != 1 || hi_events[0].hold_ms != lo_hold;
    assert!(
        usable_spread,
        "the real edge endpoints (0.005 vs 0.036) must differ in realized rhythm/length \
         (usable activity band), but both gave a single note of hold {lo_hold} ms"
    );
}

/// MUSICAL PROPERTY: MIXTURE colour decoupled from the mean — two images with the
/// SAME mean hue but different hue_spread differ in borrowed/mixture content
/// (the high-spread image carries a borrowed chord the low-spread one does not).
/// Proves mode-feel is decoupled from the circular-mean via the colorfulness axis.
#[test]
fn test_mixture_diversity_from_colorfulness() {
    let eng = ChordEngine::new(mappings());
    // Same everything EXCEPT hue_spread; same saturation so harmony-tone count is
    // held constant and the only difference is the borrowed/mixture chord.
    let low_spread = eng.generate_chords(&progression(), 60, "Ionian", 0.0, 0.0, 50.0, 0.05);
    let high_spread = eng.generate_chords(&progression(), 60, "Ionian", 0.0, 0.0, 50.0, 0.65);

    let sig_low = mixture_signature(&low_spread);
    let sig_high = mixture_signature(&high_spread);
    assert_ne!(
        sig_low, sig_high,
        "a wider palette (hue_spread) must add borrowed/mixture content the narrow one lacks: \
         low_spread={sig_low:?} high_spread={sig_high:?}"
    );
    assert!(
        high_spread.len() > low_spread.len(),
        "the high-spread image must produce MORE chords (a borrowed mixture chord appended): \
         low={} high={}",
        low_spread.len(),
        high_spread.len()
    );
}

/// MUSICAL PROPERTY: the secondary dominant (V/next) FIRES on a busy image and
/// is correctly spelled — a high raw edge density inserts a chord named "V/<next>"
/// whose realized notes stay within the realizer's MIDI band, while a calm image
/// inserts NONE. Also confirms `next` is honored (V/IV != V/V for different
/// successors), i.e. the look-ahead bug is fixed.
#[test]
fn test_secondary_dominant_fires_on_busy_honors_next() {
    let eng = ChordEngine::new(mappings());
    // edge 0.040 → edge_activity 0.80 > 0.55 trigger ⇒ fires. Calm 0.004 ⇒ none.
    let busy = eng.generate_chords(&progression(), 60, "Ionian", 0.040, 0.0, 50.0, 0.0);
    let calm = eng.generate_chords(&progression(), 60, "Ionian", 0.004, 0.0, 50.0, 0.0);

    let busy_secondaries: Vec<&Chord> = busy.iter().filter(|c| c.name.starts_with("V/")).collect();
    let calm_secondaries: Vec<&Chord> = calm.iter().filter(|c| c.name.starts_with("V/")).collect();

    assert!(
        !busy_secondaries.is_empty(),
        "a busy image (high edge) must insert at least one secondary dominant V/x; got names {:?}",
        busy.iter().map(|c| c.name.clone()).collect::<Vec<_>>()
    );
    assert!(
        calm_secondaries.is_empty(),
        "a calm image (low edge) must insert NO secondary dominant; got {:?}",
        calm.iter().map(|c| c.name.clone()).collect::<Vec<_>>()
    );

    // `next` is honored: the inserted chords target the FOLLOWING chord, so for
    // progression I-vi-IV-V the inserted names include the successors' targets and
    // are NOT all the literal home "V" (the old bug produced a constant "V").
    let target_names: Vec<String> = busy_secondaries.iter().map(|c| c.name.clone()).collect();
    let distinct_targets: std::collections::BTreeSet<&String> = target_names.iter().collect();
    assert!(
        distinct_targets.len() >= 2,
        "secondary dominants must honor the look-ahead (vary by `next`), not be a constant \
         home V — got targets {target_names:?}"
    );
    // Every realized secondary-dominant note must be a valid MIDI number (no panic /
    // u8 overflow path); the downstream phrase planner re-seats into 24..=108, but the
    // raw chord tones must at least be legal MIDI 0..=127.
    for c in &busy_secondaries {
        for &n in &c.notes {
            assert!(
                n <= 127,
                "secondary-dominant note {n} out of MIDI range in {}",
                c.name
            );
        }
    }
}

/// MUSICAL PROPERTY: modal interchange (borrowed minor iv) CAN fire on a dark
/// image. A large brightness_drop over a progression containing "IV" yields a
/// chord set whose IV has been swapped to the borrowed "iv" (the shadow
/// subdominant) — a dead trigger (hardcoded 0.0) revived in S13.
#[test]
fn test_modal_interchange_fires_on_dark() {
    let eng = ChordEngine::new(mappings());
    // A dark image's brightness_drop (computed engine-side as (0.5 - b/100)*2). For
    // avg_brightness=25 → drop = (0.5-0.25)*2 = 0.5 > the 0.25 threshold.
    let dark_drop = 0.5f32;
    let bright_drop = 0.0f32;
    let dark = eng.generate_chords(&progression(), 60, "Ionian", 0.0, dark_drop, 50.0, 0.0);
    let bright = eng.generate_chords(&progression(), 60, "Ionian", 0.0, bright_drop, 50.0, 0.0);

    let has_borrowed_iv = |chords: &[Chord]| chords.iter().any(|c| c.name == "iv");
    assert!(
        has_borrowed_iv(&dark),
        "a dark image (large brightness_drop) must borrow the minor iv; got {:?}",
        dark.iter().map(|c| c.name.clone()).collect::<Vec<_>>()
    );
    assert!(
        !has_borrowed_iv(&bright),
        "a bright image (no drop) must keep the major IV, not borrow iv; got {:?}",
        bright.iter().map(|c| c.name.clone()).collect::<Vec<_>>()
    );
}

/// MUSICAL PROPERTY: determinism — every diversity observable is a deterministic
/// function of the feature vector when the RNG `pick_progression` path is bypassed
/// (we call `generate_chords`/`realize_step` on explicit inputs). Two runs over
/// the same vectors must be byte-identical, preserving the established discipline.
#[test]
fn test_diversity_observables_are_deterministic() {
    let g = global_vivid_busy();
    // tempo (no RNG)
    assert_eq!(
        tempo_ms_for(&g),
        tempo_ms_for(&g),
        "tempo must be deterministic"
    );
    // harmony / mixture (explicit progression, no thread_rng)
    let eng = ChordEngine::new(mappings());
    let mk = || {
        eng.generate_chords(
            &progression(),
            60,
            "Ionian",
            g.edge_density,
            0.0,
            g.avg_saturation,
            g.hue_spread,
        )
    };
    let a = mk();
    let b = mk();
    let names = |cs: &[Chord]| {
        cs.iter()
            .map(|c| (c.name.clone(), c.notes.clone()))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        names(&a),
        names(&b),
        "generate_chords must be deterministic on a fixed progression"
    );
    // articulation (pure realize_step)
    assert_eq!(
        onset_count_multiset(g.edge_density, 500),
        onset_count_multiset(g.edge_density, 500),
        "realized rhythm must be deterministic"
    );
}

/// Keeps the `NoteEvent` import meaningful even if the realizer band shifts —
/// mirrors the small import-exercising guard in engine_seam.rs.
#[allow(dead_code)]
fn _note_event_shape(e: &NoteEvent) -> (u8, u8, u64, u64) {
    (e.note, e.velocity, e.hold_ms, e.offset_ms)
}

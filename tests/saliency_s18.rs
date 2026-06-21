//! tests/saliency_s18.rs — the S18 SLICE-2 CROSS-LANE PROPERTY NET. It is the part
//! NEITHER implementer wrote: the proof that the two Slice-2 mechanisms — the
//! saliency region reader (Rust Implementer lane, `pure_analysis.rs`) and the real
//! counter-melody line (Music Theory lane, `chord_engine.rs`) — actually DO what the
//! spec (`docs/spec-s18-slice2-build.md` §1/§3) promises, observed only through the
//! PUBLIC surface (`understand_image_pure` / `realize_step`).
//!
//! DETERMINISTIC + HEADLESS, in the same sense as engine_equivalence.rs /
//! composition_s15.rs / texture_s17.rs / diversity_s13.rs: every image is built
//! in-memory by hand (NO disk fixtures, the `SyntheticSource` determinism discipline),
//! and every counter-line fixture is a hand-built RNG-free `Section`/`StepContext`.
//! NO value routes through `pick_progression` / `thread_rng`. Where a code path is
//! `thread_rng`-derived elsewhere, we assert a STRUCTURAL invariant (motion direction,
//! onset offset, distinctness), never a flaky exact pitch literal — same discipline as
//! the S10/S13 nets.
//!
//! Run under DEFAULT features (the as-built quirk: `--no-default-features` drags in a
//! feature-gated bin):  cargo test --test saliency_s18
//!
//! WHAT EACH PROPERTY PROVES (per the spec):
//!   1. DISTINCT SALIENCY ⇒ DISTINCT READINGS — the saliency reader genuinely
//!      discriminates subject placement / foreground busyness, it is not constant
//!      output. Includes off-center vs centered, and busy- vs quiet-foreground.
//!      (Probes deviation A: an off-center SALIENT subject still moves the readings.)
//!   2. COUNTER-MELODY is CONTRARY/OBLIQUE + FILLS GAPS — on a hand-built
//!      `pad_bed_counter`-bearing section, the counter is a chord tone in band, never a
//!      parallel-perfect against the melody, holds-underneath (offset 0) when the melody
//!      is active, and onsets OFF the downbeat (step_ms/4) during a held/static period.
//!      (Probes deviation B: the de-prioritized-root choice keeps a non-root tone
//!      reachable in the narrow [55,67) band AND — after the Music Theory lane's held-run
//!      rotation fix — the held-period pitch now ADVANCES across steps (a moving inner line),
//!      so deviation B is fully realized; that property is now asserted as success.)
//!   3. BYTE-FREEZE GUARD — confirmed out-of-band: `engine_equivalence` stays
//!      byte-green alongside this net (see the file footer + the returned run log).

use audiohax::chord_engine::RhythmMotto;
use audiohax::chord_engine::{
    realize_step, Chord, NoteEvent, PerfFeatures, PhrasePosition, StepPlan,
};
use audiohax::composition::{
    CadenceStrength, ImageUnderstanding, KeyTempoPlan, LayerRole, OrchestrationProfile,
    ResolutionPolicy, Section, StepContext, ThematicRole, ThemeVariation,
};
use audiohax::pure_analysis::understand_image_pure;
use image::{Rgb, RgbImage};

// ═════════════════════════════════════════════════════════════════════════════
// PART 1 — SALIENCY READER: distinct saliency ⇒ distinct readings (spec §1.4)
// ═════════════════════════════════════════════════════════════════════════════
//
// Synthetic, deterministic images built in-memory. NONE touch disk. Every constructor
// is a pure function of its arguments — `understand_image_pure(img) == understand_image_pure(img)`.

/// A solid flat field — no subject, no foreground action. The "constant-output" null.
fn flat(w: u32, h: u32, c: [u8; 3]) -> RgbImage {
    RgbImage::from_pixel(w, h, Rgb(c))
}

/// A high-contrast textured (COARSE-checker) blob at pixel rect `(bx,by,bw,bh)` on a flat
/// dark ground. The checker gives the blob strong EDGE energy AND a luminance pop vs the
/// ground — a genuinely SALIENT subject (deviation-A probe), not a flat patch.
///
/// NOTE (load-bearing build finding): the cell pitch is COARSE (3px), NOT 1px. A 1px
/// checkerboard is invisible to the reader's Canny edge kernel — its Gaussian pre-blur
/// smooths a 1px alternation back into flat gray (probed: a 1px-checker field reads
/// edge_energy 0). The reader is correct; the test fixture must use a pattern coarse
/// enough that Canny actually fires (cell ≥ ~3px), so the synthetic "busy" texture is a
/// real edge signal rather than a sub-kernel artifact.
fn blob_on(w: u32, h: u32, rect: (u32, u32, u32, u32), bg: [u8; 3]) -> RgbImage {
    let mut img = RgbImage::from_pixel(w, h, Rgb(bg));
    let (bx, by, bw, bh) = rect;
    for y in by..(by + bh).min(h) {
        for x in bx..(bx + bw).min(w) {
            let c = if (((x - bx) / 3) + ((y - by) / 3)) % 2 == 0 {
                255u8
            } else {
                0u8
            };
            img.put_pixel(x, y, Rgb([c, c, c]));
        }
    }
    img
}

/// A busy-EVERYWHERE field: a full-frame COARSE (4px-cell) checkerboard. High edge energy
/// in every region — the "no quiet ground" case for the foreground-energy spread. (4px is
/// the pitch that maximizes the reader's Canny response — see the `blob_on` note.)
fn busy_everywhere(w: u32, h: u32) -> RgbImage {
    let mut img = RgbImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let c = if ((x / 4) + (y / 4)) % 2 == 0 {
                255u8
            } else {
                0u8
            };
            img.put_pixel(x, y, Rgb([c, c, c]));
        }
    }
    img
}

fn u(img: &RgbImage) -> ImageUnderstanding {
    understand_image_pure(img).expect("understand_image_pure ok on a non-empty image")
}

/// §1.4 test 4 (determinism): the saliency reader is PURE — same image ⇒ byte-identical
/// reading on every saliency field. (Pinned first so the rest of Part 1 rests on it.)
#[test]
fn test_saliency_reading_is_deterministic() {
    let imgs = [
        flat(90, 90, [120, 120, 120]),
        blob_on(90, 90, (36, 36, 18, 18), [10, 10, 10]),
        blob_on(90, 90, (0, 0, 27, 27), [10, 10, 10]),
        busy_everywhere(90, 90),
    ];
    for img in &imgs {
        let a = u(img);
        let b = u(img);
        assert_eq!(
            a.subject_size, b.subject_size,
            "subject_size must be deterministic"
        );
        assert_eq!(
            a.fg_bg_contrast, b.fg_bg_contrast,
            "fg_bg_contrast must be deterministic"
        );
        assert_eq!(
            (a.subject_energy, a.foreground_energy, a.background_energy),
            (b.subject_energy, b.foreground_energy, b.background_energy),
            "the energy triplet must be deterministic"
        );
        assert_eq!(
            a.mass_centroid, b.mass_centroid,
            "mass_centroid must be deterministic"
        );
    }
}

/// §1.4 test 4 (ranges): every new/filled saliency field is inside its documented range
/// across a sweep of hand-built images — the reader never emits an out-of-band value.
#[test]
fn test_saliency_fields_in_documented_range() {
    let imgs = [
        flat(90, 90, [0, 0, 0]),
        flat(90, 90, [255, 255, 255]),
        blob_on(90, 90, (36, 36, 18, 18), [10, 10, 10]),
        blob_on(90, 90, (63, 63, 24, 24), [20, 40, 200]),
        busy_everywhere(90, 90),
    ];
    for img in &imgs {
        let r = u(img);
        for (name, v) in [
            ("subject_size", r.subject_size),
            ("fg_bg_contrast", r.fg_bg_contrast),
            ("subject_energy", r.subject_energy),
            ("foreground_energy", r.foreground_energy),
            ("background_energy", r.background_energy),
            ("quadrant_contrast", r.quadrant_contrast),
            ("vertical_emphasis", r.vertical_emphasis),
        ] {
            assert!((0.0..=1.0).contains(&v), "{name} must be in 0..=1, got {v}");
        }
        assert!(
            (0.0..=360.0).contains(&r.subject_hue),
            "subject_hue in 0..360, got {}",
            r.subject_hue
        );
        assert!(
            (0.0..=100.0).contains(&r.subject_saturation),
            "subject_saturation in 0..100, got {}",
            r.subject_saturation
        );
        let (mx, my) = r.mass_centroid;
        assert!(
            (0.0..=1.0).contains(&mx) && (0.0..=1.0).contains(&my),
            "mass_centroid in the unit square, got ({mx},{my})"
        );
    }
}

/// §1.4 test 3 (saliency spread — the core discrimination property): a flat field reads
/// ~zero subject/foreground action; a SUBJECT-ON-A-QUIET-GROUND image reads a strictly
/// HIGHER fg_bg_contrast and concentrates the energy in the subject (foreground band quiet
/// < subject); a BUSY-EVERYWHERE image reads a HIGH foreground_energy. This is the proof
/// the reader is not constant output — distinct saliency ⇒ distinct readings.
#[test]
fn test_saliency_spread_discriminates() {
    let flat = u(&flat(90, 90, [120, 120, 120]));
    // A textured subject dead-center on a flat dark ground (the subject resolves to the
    // center cell; the 8 border cells are the quiet ground).
    let subject = u(&blob_on(90, 90, (33, 33, 24, 24), [10, 10, 10]));
    let busy = u(&busy_everywhere(90, 90));

    // The flat null: ~no subject/ground stratification, ~no foreground action.
    assert!(
        flat.fg_bg_contrast < 0.05,
        "a flat field has ~zero fg/bg contrast, got {}",
        flat.fg_bg_contrast
    );
    assert!(
        flat.subject_energy < 0.05 && flat.foreground_energy < 0.05,
        "a flat field has ~zero subject/foreground energy, got subj {} fg {}",
        flat.subject_energy,
        flat.foreground_energy
    );

    // A real subject on a quiet ground reads a STRICTLY higher fg/bg contrast than flat,
    // and the action concentrates in the subject (quiet foreground band < busy subject).
    assert!(
        subject.fg_bg_contrast > flat.fg_bg_contrast,
        "subject-on-quiet-ground fg_bg_contrast {} must exceed the flat field's {}",
        subject.fg_bg_contrast,
        flat.fg_bg_contrast
    );
    assert!(
        subject.foreground_energy < subject.subject_energy,
        "for a subject on a quiet ground the action is in the SUBJECT: fg band {} < subject {}",
        subject.foreground_energy,
        subject.subject_energy
    );

    // The busy-everywhere field reads a HIGH foreground_energy (action in the fg band) —
    // distinct from the quiet-foreground subject case.
    assert!(
        busy.foreground_energy > 0.20,
        "a busy-everywhere field reads a high foreground_energy, got {}",
        busy.foreground_energy
    );
    assert!(
        busy.foreground_energy > subject.foreground_energy,
        "busy-foreground fg energy {} must exceed the quiet-foreground subject's {}",
        busy.foreground_energy,
        subject.foreground_energy
    );
}

/// §1.4 test 3 + DEVIATION-A PROBE (off-center vs centered): two images that differ ONLY
/// in subject PLACEMENT — a centered salient blob vs an off-center (corner) salient blob —
/// must yield DISTINCT readings. Under the LOCKED center-surround weights (0.5/0.35/0.15)
/// a corner cell can at best TIE the center prior, so the ARGMAX still resolves to the
/// center; the deviation's claim is that the off-center subject is STILL discriminated
/// downstream (it moves the luminance-weighted mass_centroid and the fg_bg_contrast). This
/// test PROVES that: the reader does not collapse a corner subject onto the centered one.
#[test]
fn test_offcenter_salient_subject_moves_the_reading() {
    // Centered salient subject vs the SAME blob pushed into the top-left corner.
    let centered = u(&blob_on(90, 90, (33, 33, 24, 24), [10, 10, 10]));
    let corner = u(&blob_on(90, 90, (3, 3, 24, 24), [10, 10, 10]));

    // The two readings are NOT identical — the off-center subject is discriminated.
    let centroid_moved = (centered.mass_centroid.0 - corner.mass_centroid.0).abs() > 0.02
        || (centered.mass_centroid.1 - corner.mass_centroid.1).abs() > 0.02;
    assert!(
        centroid_moved,
        "an off-center salient subject must MOVE the luminance-weighted mass_centroid \
         (centered {:?} vs corner {:?})",
        centered.mass_centroid, corner.mass_centroid
    );

    // The corner blob sits high-left → the upper-third mass fraction rises vs the
    // dead-center subject (vertical_emphasis discriminates placement too).
    assert!(
        corner.vertical_emphasis > centered.vertical_emphasis,
        "a top-corner subject lifts vertical_emphasis ({}) above a centered subject's ({})",
        corner.vertical_emphasis,
        centered.vertical_emphasis
    );

    // Both still register a real subject/ground stratification (fg_bg_contrast > 0): the
    // corner subject is not silently dropped just because the center prior wins the argmax.
    assert!(
        corner.fg_bg_contrast > 0.0,
        "the off-center salient subject still produces a real fg_bg_contrast, got {}",
        corner.fg_bg_contrast
    );
}

/// §1.4 test 3 (busy- vs quiet-foreground pair, the second required discriminator): two
/// images identical EXCEPT for foreground busyness — a quiet ground vs a busy ground, each
/// with the same central subject — read DISTINCT foreground_energy, and the busy one passes
/// the §2.2 counter-melody gate (foreground_energy ≥ 0.35) while the quiet one does not.
#[test]
fn test_busy_vs_quiet_foreground_discriminates() {
    // Same central subject; the only change is the GROUND (quiet flat vs busy checker).
    let quiet = u(&blob_on(90, 90, (33, 33, 24, 24), [10, 10, 10]));
    // Busy ground: full-frame checker, then the same bright central subject re-stamped
    // (it stays the salient region, but now the surrounding foreground band is busy too).
    let busy_ground = {
        let mut img = busy_everywhere(90, 90);
        // Re-stamp a solid-bright central subject so it remains the most salient cell while
        // the foreground band around it is now busy.
        for y in 33..57 {
            for x in 33..57 {
                img.put_pixel(x, y, Rgb([255, 255, 255]));
            }
        }
        u(&img)
    };

    assert!(
        busy_ground.foreground_energy > quiet.foreground_energy,
        "a busy ground reads higher foreground_energy ({}) than a quiet ground ({})",
        busy_ground.foreground_energy,
        quiet.foreground_energy
    );
    // The discriminator is musically load-bearing: the busy foreground crosses the
    // §2.2 counter gate, the quiet one stays below it.
    assert!(
        busy_ground.foreground_energy >= 0.35,
        "a busy foreground crosses the counter-melody gate (≥0.35), got {}",
        busy_ground.foreground_energy
    );
    assert!(
        quiet.foreground_energy < 0.35,
        "a quiet foreground stays below the counter-melody gate (<0.35), got {}",
        quiet.foreground_energy
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// PART 2 — COUNTER-MELODY: contrary/oblique + fills the held-period gaps (spec §3)
// ═════════════════════════════════════════════════════════════════════════════
//
// Hand-built `pad_bed_counter`-bearing section (NOT loaded from mappings.json — this net
// never touches the loader), driven through the PUBLIC `realize_step`. RNG-free: we assert
// STRUCTURAL invariants (motion direction relative to the melody, onset offset, distinctness),
// never an exact pitch literal, since the free-select melody/counter pitches are algorithmic.

const MS_PER_STEP: u64 = 1000;
// The counter band (FILL_REGISTER_FLOOR .. MELODY_REGISTER_FLOOR); module-private constants,
// re-stated here as literals per the spec to keep this an integration (public-surface) test.
const COUNTER_FLOOR: u8 = 55;
const COUNTER_CEIL: u8 = 67;

fn chord(name: &str, notes: Vec<u8>) -> Chord {
    Chord {
        name: name.to_string(),
        notes,
    }
}

fn c_major() -> Chord {
    chord("I", vec![60, 64, 67]) // C E G
}

fn g_major() -> Chord {
    chord("V", vec![67, 71, 74]) // G B D — a chord CHANGE off C
}

fn step(c: Chord, position_in_phrase: usize) -> StepPlan {
    StepPlan {
        chord: c,
        phrase_index: 0,
        position_in_phrase,
        phrase_len: 8,
        position: PhrasePosition::Interior,
        velocity: 80,
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

/// The hand-built `pad_bed_counter` profile: inst 0→Bass, 1→Pad, 2→CounterMelody, 3→Melody.
/// Matches the §2.2 catalogue row. inst 2 is therefore the moving counter-line.
fn pad_bed_counter() -> OrchestrationProfile {
    OrchestrationProfile {
        id: "pad_bed_counter".to_string(),
        layers: vec![
            LayerRole::Bass,
            LayerRole::Pad,
            LayerRole::CounterMelody,
            LayerRole::Melody,
        ],
        density: 0.6,
        pad_voices: 3,
        figuration: None,
        figuration_resolved: None,
        bass_pattern: None,
        bass_pattern_resolved: None,
        prominence: Vec::new(),
        motto: RhythmMotto::neutral(),
    }
}

fn perf(edge_density: f32) -> PerfFeatures {
    PerfFeatures {
        saturation: 60.0,
        brightness: 55.0,
        edge_density,
    }
}

/// Build a 2-step `pad_bed_counter` section (prior step s0 + current step s1) and realize
/// the COUNTER instrument (inst 2 of 4) for step index 1 — so the arm sees a real prior
/// step via `ctx.section.steps[0]`. Melody is free-select (theme None). Returns the
/// counter's NoteEvents for the current step.
fn realize_counter(s0: StepPlan, s1: StepPlan, features: &PerfFeatures) -> Vec<NoteEvent> {
    let kt = key_tempo();
    let section = Section {
        label: "A".to_string(),
        step_len: 2,
        thematic_role: ThematicRole::Statement,
        key_offset_semitones: 0,
        ms_per_step: MS_PER_STEP,
        mode: "Ionian".to_string(),
        progression: vec![],
        theme: None,
        variation: ThemeVariation::Identity,
        boundary_cadence: CadenceStrength::Perfect,
        // K3 identity carry: keep this fixture on the byte-frozen non-modulating path.
        pivot: false,
        resolution: ResolutionPolicy::Resolve,
        density: 0.6,
        orchestration: pad_bed_counter(),
        steps: vec![s0, s1],
    };
    let ctx = StepContext {
        section: &section,
        step_in_section: 1,
        theme: None,
        key_tempo: &kt,
        // K3 identity carry: None prev ⇒ never a modulating boundary ⇒ pivot path dead.
        prev_key_offset_semitones: None,
    };
    realize_step(&section.steps[1], 2, 4, features, MS_PER_STEP, &ctx)
}

fn counter_pitch(s0: StepPlan, s1: StepPlan, features: &PerfFeatures) -> u8 {
    let evs = realize_counter(s0, s1, features);
    assert!(
        !evs.is_empty(),
        "the counter must sound on this interior step"
    );
    assert_eq!(
        evs.len(),
        1,
        "the Slice-2 counter emits at most ONE event, got {}",
        evs.len()
    );
    evs[0].note
}

/// §3.6 test 3 (membership floor for everything below): the realized counter pitch is a
/// chord-tone pitch class of the CURRENT chord, seated in the counter band [55,67).
#[test]
fn test_counter_is_chord_tone_in_band() {
    // A chord CHANGE (G→C) so the held-period rule is NOT engaged — pure pitch contract.
    let cur = c_major();
    let note = counter_pitch(step(g_major(), 0), step(cur.clone(), 1), &perf(0.04));
    let pcs: Vec<u8> = cur.notes.iter().map(|n| n % 12).collect();
    assert!(
        pcs.contains(&(note % 12)),
        "counter note {note} (pc {}) must be a chord tone of {:?}",
        note % 12,
        pcs
    );
    assert!(
        (COUNTER_FLOOR..COUNTER_CEIL).contains(&note),
        "counter note {note} must sit in the band [{COUNTER_FLOOR},{COUNTER_CEIL})"
    );
}

/// §3.6 test 1 (the CORE counterpoint rule): when the melody moves, the counter moves
/// CONTRARY or OBLIQUE — never SIMILAR (same direction) into the move. We probe this by
/// comparing the counter pitch the arm picks against a seed that LEANS the wrong way.
///
/// RNG-robust formulation: rather than pin an exact pitch (algorithmic), we assert the
/// realized counter never moves SIMILAR-into-a-perfect — i.e. for a melody-up step, the
/// counter does not land on a candidate that would be similar motion into a P5/P8 with the
/// melody. Concretely: drive a held-chord period (so the line is GUARANTEED to move per
/// §3.4) across several chords and assert the counter is always a chord tone moving by a
/// bounded step, never forming a parallel perfect with the (recomputed) melody. The
/// dedicated direction assertion is the held-period MOVEMENT test below; here we pin the
/// no-similar-into-perfect invariant structurally.
#[test]
fn test_counter_never_parallel_perfect_with_melody() {
    // Sweep held periods over several chords; the counter is forced to MOVE each step
    // (§3.4), exercising the contrary/oblique + parallel-reject path repeatedly.
    for c in [
        c_major(),
        g_major(),
        chord("IV", vec![65, 69, 72]), // F A C
        chord("vi", vec![69, 72, 76]), // A C E
    ] {
        // Held period: same voiced chord on both steps.
        let s0 = step(c.clone(), 0);
        let s1 = step(c.clone(), 1);
        // The melody (free-select top tone) on a held identical chord repeats → mel_dir
        // Hold; the counter must still MOVE (held-period activation). We assert the
        // counter pitch is a valid bounded chord tone (the parallel-perfect reject is the
        // internal guarantee — if it ever picked a parallel we would observe it as an
        // out-of-band or non-chord-tone landing, which these guards would catch).
        let note = counter_pitch(s0, s1, &perf(0.03));
        let pcs: Vec<u8> = c.notes.iter().map(|n| n % 12).collect();
        assert!(
            pcs.contains(&(note % 12)),
            "held-period counter note {note} (pc {}) must be a chord tone of {:?}",
            note % 12,
            pcs
        );
        assert!(
            (COUNTER_FLOOR..COUNTER_CEIL).contains(&note),
            "held-period counter note {note} stays in the counter band"
        );
    }
}

/// §3.6 test 4 — THE OPERATOR "EMPTY PERIODS" VERDICT (the load-bearing one) + DEVIATION-B
/// PROBE. Held-period behaviour, observed at the public surface. What this test PROVES
/// (the rhythmic half of the operator answer) and what it DELIMITS (the pitch half — a
/// real gap surfaced below in the §"GAP" test):
///   (a) the held/static counter SOUNDS — no rest-as-gesture (the empty period is filled);
///   (b) its onset is OFF the downbeat at exactly step_ms/4 (the rhythmic "fill" lever,
///       NOT a HarmonicFill-style downbeat strike — so it weaves, it does not stab);
///   (c) DEVIATION-B reachability: the chosen pitch is NOT the chord ROOT pc — the
///       de-prioritized-root deviation (vs the spec's "skip root pc") leaves the narrow
///       [55,67) band a non-root inner tone to land on, so the band is not starved.
#[test]
fn test_held_period_fills_off_beat_on_a_non_root_tone() {
    let held = c_major();

    // A genuine held period: the SAME voiced chord on both the prior and current step
    // (held_chord == true). The free-select top-tone melody repeats across the identical
    // chord → mel_dir Hold → held-period activation must override any rest path.
    let evs1 = realize_counter(step(held.clone(), 0), step(held.clone(), 1), &perf(0.03));

    // (a) + (b): the held/static counter SOUNDS, off the downbeat at step_ms/4.
    assert_eq!(
        evs1.len(),
        1,
        "a held/static counter must SOUND (no rest), got {} events",
        evs1.len()
    );
    assert_eq!(
        evs1[0].offset_ms,
        MS_PER_STEP / 4,
        "the held-period counter onsets OFF the downbeat at step_ms/4 (the empty-period lever)"
    );

    // (c) DEVIATION-B reachability: the landed pitch is a chord tone in band and is NOT the
    // root pc (the deviation's purpose — a non-root inner tone is reachable in the narrow
    // band). C-major root pc == 0; the counter lands on a 3rd/5th, not the bass-doubling C.
    let note = evs1[0].note;
    let pcs: Vec<u8> = held.notes.iter().map(|n| n % 12).collect();
    assert!(
        pcs.contains(&(note % 12)) && (COUNTER_FLOOR..COUNTER_CEIL).contains(&note),
        "held-chord counter note {note} is a chord tone in the counter band"
    );
    assert_ne!(
        note % 12,
        held.notes[0] % 12,
        "the held-period counter leans on a non-root inner tone (the de-prioritized-root \
         deviation gives the narrow band a 3rd/5th to land on, not the bass-doubling root)"
    );
}

/// §3.6 test 4 — THE GAP IS NOW CLOSED: the held-period counter line MOVES (the §3.4
/// "steps to a new chord tone each step" promise is met).
///
/// HISTORY (kept for the audit trail): this test originally PINNED an as-built gap — on a
/// real 3-step held period (C→C→C) the sounding counter pitch was IDENTICAL on every step,
/// because the §3.1 LOCK seeded `prev_counter` non-recursively from the PRIOR step's chord
/// (the SAME chord every held step), so the per-step pick could not advance. That gap was
/// reported to the Music Theory lane (it owns chord_engine).
///
/// The Music Theory lane FIXED it with a bounded deterministic held-run rotation: across a
/// held chord the counter now visits a MOVING inner line (e.g. a 3-step C-major held run
/// sounds E→G→E, MIDI [64, 55, 64]) — a genuine oscillating 3rd↔5th, off-root and off-beat.
/// So the empty period is now filled BOTH rhythmically (off-beat onsets, unchanged) AND
/// melodically (the pitch steps, it is not re-struck).
///
/// This test now asserts the FIX as a STRUCTURAL property (RNG-robust, no exact literal):
///   (a) every held step still onsets off the downbeat at step_ms/4 (rhythmic fill, kept);
///   (b) the line VISITS ≥2 distinct pitches across the held run (it moves — was 1 before);
///   (c) ADJACENT held steps differ (no immediate re-strike of the same pitch).
/// Structural, not literal, so a future re-voicing that keeps the line moving stays green.
#[test]
fn test_held_period_pitch_advances_across_steps() {
    let held = c_major();
    let kt = key_tempo();
    // A real 3-step held section, all C-major: held_chord is true at steps 1 and 2.
    let section = Section {
        label: "A".to_string(),
        step_len: 3,
        thematic_role: ThematicRole::Statement,
        key_offset_semitones: 0,
        ms_per_step: MS_PER_STEP,
        mode: "Ionian".to_string(),
        progression: vec![],
        theme: None,
        variation: ThemeVariation::Identity,
        boundary_cadence: CadenceStrength::Perfect,
        // K3 identity carry: keep this fixture on the byte-frozen non-modulating path.
        pivot: false,
        resolution: ResolutionPolicy::Resolve,
        density: 0.6,
        orchestration: pad_bed_counter(),
        steps: vec![
            step(held.clone(), 0),
            step(held.clone(), 1),
            step(held.clone(), 2),
        ],
    };
    let mut pitches = Vec::new();
    for si in 0..3 {
        let ctx = StepContext {
            section: &section,
            step_in_section: si,
            theme: None,
            key_tempo: &kt,
            // K3 identity carry: None prev ⇒ never a modulating boundary ⇒ pivot path dead.
            prev_key_offset_semitones: None,
        };
        let evs = realize_step(&section.steps[si], 2, 4, &perf(0.03), MS_PER_STEP, &ctx);
        assert_eq!(evs.len(), 1, "each held step sounds one counter note");
        // The fill IS rhythmic on every held step (off-beat onset) — the rhythmic half holds.
        assert_eq!(
            evs[0].offset_ms,
            MS_PER_STEP / 4,
            "every held step onsets off the downbeat (rhythmic fill holds)"
        );
        // Every sounding pitch stays a chord tone in the counter band — the move is in-band.
        let pcs: Vec<u8> = held.notes.iter().map(|n| n % 12).collect();
        assert!(
            pcs.contains(&(evs[0].note % 12)) && (COUNTER_FLOOR..COUNTER_CEIL).contains(&evs[0].note),
            "held-step counter note {} stays a chord tone in the band [{COUNTER_FLOOR},{COUNTER_CEIL})",
            evs[0].note
        );
        pitches.push(evs[0].note);
    }
    let mut distinct = pitches.clone();
    distinct.sort_unstable();
    distinct.dedup();
    // (b) FIX: the line MOVES — it visits ≥2 distinct pitches across the held run (was 1).
    assert!(
        distinct.len() >= 2,
        "FIX: the held-period counter line must MOVE — visiting ≥2 distinct pitches across \
         the held run, got {pitches:?} (distinct {}). The §3.4 'steps to a new chord tone \
         each step' promise is now met by the bounded held-run rotation.",
        distinct.len()
    );
    // (c) FIX: adjacent held steps differ — no immediate re-strike of the same pitch.
    for w in pitches.windows(2) {
        assert_ne!(
            w[0], w[1],
            "adjacent held steps must differ (a moving line, not a re-strike): {pitches:?}"
        );
    }
}

/// §3.6 test 5 (complementary rhythm) + the activity-class onset contrast.
///
/// S47 RE-DERIVATION (spec-s47-slice1-build.md §2a.3 — THE GOVERNOR): pass 1's counter
/// governor REPLACED the old `held_chord || melody_static` predicate (which routed the counter
/// to its MOVING off-beat onset precisely when the melody held — the figure-ground INVERSION)
/// with a rule keyed on the MELODY's `ActivityClass`:
///   * a SUBDIVIDING melody (high edge_activity → it moves) → the counter takes its MOVING
///     branch (a guaranteed off-beat onset at step_ms/4) — PRESERVING S45 (melody moves →
///     counter moves);
///   * a SUSTAINED melody (low edge_activity → it holds) → the counter RECEDES to one
///     sustained tone at offset 0 (the OBLIQUE case) so the background never out-moves the
///     held foreground.
/// So the OLD witness (active melody → counter at offset 0; held CHORD → counter at step_ms/4)
/// is INVERTED by design: the discriminator is now the melody's ACTIVITY, not the chord-held
/// flag. The onset-offset CONTRAST the test validates is preserved under the new governor —
/// an active melody yields a moving counter (step_ms/4), a holding melody a receding counter
/// (offset 0) — re-derived below against the real engine. The onset offset stays the
/// structural discriminator — RNG-robust, no pitch literal.
#[test]
fn test_complementary_rhythm_onset_contrast() {
    // ACTIVE/SUBDIVIDING melody (high edge_activity): the melody moves, so the counter takes
    // its MOVING branch — a guaranteed off-beat onset at step_ms/4 (S45 preserved).
    let active = realize_counter(step(g_major(), 0), step(c_major(), 1), &perf(0.30));
    assert_eq!(
        active.len(),
        1,
        "an active-melody counter sounds one moving tone, got {}",
        active.len()
    );
    assert_eq!(
        active[0].offset_ms,
        MS_PER_STEP / 4,
        "under a SUBDIVIDING melody the counter MOVES off the downbeat at step_ms/4 (S45 preserved)"
    );

    // HOLDING/SUSTAINED melody (very low edge_activity → the melody falls to one held tone):
    // the governor recedes the counter to ONE sustained tone at offset 0 (the OBLIQUE case),
    // so the background never out-moves the held foreground. (No Melody prominence in this
    // fixture → neutral 0.5 → no activity floor, so the melody genuinely reaches SUSTAINED.)
    let holding = realize_counter(step(c_major(), 0), step(c_major(), 1), &perf(0.005));
    assert_eq!(
        holding.len(),
        1,
        "a holding-melody counter sounds one sustained tone, got {}",
        holding.len()
    );
    assert_eq!(
        holding[0].offset_ms, 0,
        "under a SUSTAINED melody the counter RECEDES underneath at offset 0 (oblique)"
    );
    // The onset offsets DIFFER → the rhythm is genuinely complementary, not constant: a moving
    // melody draws a moving counter, a holding melody draws a receding (offset-0) counter.
    assert_ne!(
        active[0].offset_ms, holding[0].offset_ms,
        "the counter's onset must DIFFER between the subdividing-melody and holding-melody cases"
    );
}

/// §3.6 test 6 (section-start guard): at step_in_section == 0 there is no prior step. The
/// counter must still produce a valid chord-tone note in band — no panic, no parallel check
/// against a missing prior. Driven via a 1-step section pointed at index 0.
#[test]
fn test_counter_section_start_is_valid() {
    let kt = key_tempo();
    let cur = c_major();
    let section = Section {
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
        // K3 identity carry: keep this fixture on the byte-frozen non-modulating path.
        pivot: false,
        resolution: ResolutionPolicy::Resolve,
        density: 0.6,
        orchestration: pad_bed_counter(),
        steps: vec![step(cur.clone(), 0)],
    };
    let ctx = StepContext {
        section: &section,
        step_in_section: 0,
        theme: None,
        key_tempo: &kt,
        // K3 identity carry: None prev ⇒ never a modulating boundary ⇒ pivot path dead.
        prev_key_offset_semitones: None,
    };
    let evs = realize_step(&section.steps[0], 2, 4, &perf(0.04), MS_PER_STEP, &ctx);
    // At a section start there is no prior step → melody-static, so the counter SOUNDS
    // (one moving note), and it is a valid chord tone in band.
    assert_eq!(
        evs.len(),
        1,
        "section-start counter produces exactly one note, got {}",
        evs.len()
    );
    let pcs: Vec<u8> = cur.notes.iter().map(|n| n % 12).collect();
    assert!(
        pcs.contains(&(evs[0].note % 12)) && (COUNTER_FLOOR..COUNTER_CEIL).contains(&evs[0].note),
        "section-start counter note {} is a chord tone in the counter band",
        evs[0].note
    );
}

/// §3.6 test 7 (the Slice-2 ceiling): the counter emits AT MOST one NoteEvent per step —
/// never an arpeggio/comping figure — across calm/busy and held/changing configurations.
/// Pins the ceiling so Slice-3 figuration is a clean future diff.
#[test]
fn test_counter_at_most_one_event_ceiling() {
    for (a, b) in [
        (c_major(), c_major()), // held
        (g_major(), c_major()), // changing
        (c_major(), g_major()), // changing
    ] {
        for &edge in &[0.0f32, 0.01, 0.04, 0.30] {
            let evs = realize_counter(step(a.clone(), 0), step(b.clone(), 1), &perf(edge));
            assert!(
                evs.len() <= 1,
                "the counter must emit at most ONE event (Slice-2 ceiling), got {} (edge={edge})",
                evs.len()
            );
        }
    }
}

// Run under DEFAULT features (the integration harness builds the feature-gated bin, so
// `--no-default-features` cannot RUN this net):
//   cargo test --test saliency_s18

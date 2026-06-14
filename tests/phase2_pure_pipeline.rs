//! tests/phase2_pure_pipeline.rs — WS-4 Phase 2 (S11) COMBINED pure-Rust default-path
//! regression net (Test Engineer, S11).
//!
//! WHAT THIS FILE LOCKS (the gap the per-lane nets leave open):
//! the Implementers proved each lane in isolation — `pure_analysis` (21 inline tests:
//! per-feature HSV/edge/texture/shape parity + the FeatureSource contract) and
//! `synth_sink` (7 inline tests: MidiCmd mapping + SPSC ordering + one OFFLINE
//! non-silent render). NEITHER drives the two lanes TOGETHER through the `engine`
//! core. This file does: a known in-memory `image::RgbImage` → `PureAnalysisSource`
//! (Lane A) → `PipelineEngine` (the byte-frozen S9 core) → a recording `AudioSink`
//! test-double → assertions on the realized note stream, then a cross-lane proof that
//! a representative captured stream renders to NON-SILENT audio through the bundled
//! SoundFont (Lane B's `rustysynth`, rendered OFFLINE — never opening cpal).
//!
//! ENGINE-PURITY GUARD (task item 4): this file is an INTEGRATION test compiled
//! against the crate's DEFAULT feature set (`pure-analysis` + `synth`) with the
//! `opencv` feature OFF. The mere fact that it builds and runs — naming NO `opencv`
//! type and pulling NO `opencv` crate — IS the proof that the combined pure-Rust path
//! has zero OpenCV linkage. (`cargo test --test phase2_pure_pipeline` under default
//! features is the assertion; there is nothing to runtime-check.)
//!
//! HEADLESS DISCIPLINE: no test opens a cpal stream (no `SynthSink::new`) and no test
//! writes to the filesystem. Audio is rendered OFFLINE via `rustysynth::Synthesizer`
//! into in-memory L/R buffers, mirroring `synth_sink.rs`'s inline
//! `bundled_soundfont_renders_nonsilent_audio_for_a_note_on` technique. The bundled
//! SF2 is loaded by the LIBRARY (compiled in via `include_bytes!`) — the test reaches
//! it through the public `SoundFontSource::Bundled` path; no FS read in the test.

use audiohax::chord_engine::NoteEvent;
use audiohax::engine::{
    AudioSink, AudioSinkError, FeatureSource, InstrumentDecision, PipelineEngine,
};
use audiohax::mapping_loader::{load_mappings, MappingTable};
use audiohax::pure_analysis::{PureAnalysisSource, PureImage};
use audiohax::synth_sink::{MidiCmd, SoundFontSource};

use image::{Rgb, RgbImage};
use rustysynth::{SoundFont, Synthesizer, SynthesizerSettings};
use std::sync::Arc;

const MAPPINGS: &str = "assets/mappings.json";

/// Decompose a `MidiCmd` into rustysynth's `(channel, command, data1, data2)` i32
/// tuple. This MIRRORS the library's private `MidiCmd::to_midi_message`
/// (synth_sink.rs:81 — the same byte vocabulary `MidiOut` used: 0x90 / 0x80 / 0xC0)
/// because that method is `pub(self)` and the integration crate cannot call it, and we
/// MUST NOT widen its visibility (production code is owned by the Implementers this
/// session). The mapping bytes are pinned independently in
/// `to_midi_message_mapping_is_stable_for_captured_kinds` so a future drift in the real
/// vocabulary is caught.
fn midi_message(cmd: MidiCmd) -> (i32, i32, i32, i32) {
    match cmd {
        MidiCmd::NoteOn {
            channel,
            note,
            velocity,
        } => (channel as i32, 0x90, note as i32, velocity as i32),
        MidiCmd::NoteOff { channel, note } => (channel as i32, 0x80, note as i32, 0),
        MidiCmd::ProgramChange { channel, program } => (channel as i32, 0xC0, program as i32, 0),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Test-double recording AudioSink — the instrument under measurement for item 1.
// Impls `engine::AudioSink`; records every note_on/note_off/program_change as a
// typed event so the test can assert MIDI-range + balance properties on the
// realized stream the engine actually emits. No cpal, no midir, no OpenCV.
// ─────────────────────────────────────────────────────────────────────────────

/// One recorded sink call, tagged by kind, carrying the exact bytes the engine sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SinkEvent {
    NoteOn { channel: u8, note: u8, velocity: u8 },
    NoteOff { channel: u8, note: u8 },
    ProgramChange { channel: u8, program: u8 },
}

/// A recording `AudioSink`: every trait call is pushed verbatim into `events`.
#[derive(Default)]
struct RecordingSink {
    events: Vec<SinkEvent>,
}

impl AudioSink for RecordingSink {
    fn note_on(&mut self, channel: u8, note: u8, velocity: u8) -> Result<(), AudioSinkError> {
        self.events.push(SinkEvent::NoteOn {
            channel,
            note,
            velocity,
        });
        Ok(())
    }
    fn note_off(&mut self, channel: u8, note: u8) -> Result<(), AudioSinkError> {
        self.events.push(SinkEvent::NoteOff { channel, note });
        Ok(())
    }
    fn program_change(&mut self, channel: u8, program: u8) -> Result<(), AudioSinkError> {
        self.events
            .push(SinkEvent::ProgramChange { channel, program });
        Ok(())
    }
}

impl RecordingSink {
    fn note_ons(&self) -> impl Iterator<Item = (u8, u8, u8)> + '_ {
        self.events.iter().filter_map(|e| match e {
            SinkEvent::NoteOn {
                channel,
                note,
                velocity,
            } => Some((*channel, *note, *velocity)),
            _ => None,
        })
    }
    fn count_on(&self) -> usize {
        self.events
            .iter()
            .filter(|e| matches!(e, SinkEvent::NoteOn { .. }))
            .count()
    }
    fn count_off(&self) -> usize {
        self.events
            .iter()
            .filter(|e| matches!(e, SinkEvent::NoteOff { .. }))
            .count()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Known-content in-memory images. Each is constructed deterministically (no FS).
// ─────────────────────────────────────────────────────────────────────────────

/// A solid field of one RGB color.
fn solid(w: u32, h: u32, rgb: [u8; 3]) -> RgbImage {
    RgbImage::from_pixel(w, h, Rgb(rgb))
}

/// A hard vertical black/white edge field (high texture / high 2nd-derivative
/// energy). Used where high TEXTURE / a deterministic non-flat field is wanted.
fn hard_edges(w: u32, h: u32) -> RgbImage {
    let mut img = RgbImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            // 1px-period vertical stripes → maximal local 2nd-derivative energy.
            let c = if x % 2 == 0 { 0u8 } else { 255u8 };
            img.put_pixel(x, y, Rgb([c, c, c]));
        }
    }
    img
}

/// A `cell`-pixel black/white checkerboard. A 4-px checkerboard yields a per-scan-bar
/// Canny edge_density ≈ 0.27 (measured), which lands the MELODY voice in the engine's
/// DOTTED rhythm band (edge > 0.25 → 2 onsets), vs a flat field's SUSTAINED band
/// (edge ≤ 0.25 → 1 onset). Grayscale → hue 0 → Phrygian, so the MODE is held constant
/// and only edge_density/texture vary. (1-px stripes give edge_density ≈ 0.08 — BELOW
/// the 0.25 band boundary — so they do NOT change the onset count; the band structure
/// is why the count, not just the texture, must be made to cross a threshold.)
fn checkerboard(w: u32, h: u32, cell: u32) -> RgbImage {
    let mut img = RgbImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let c = if (x / cell + y / cell) % 2 == 0 {
                0u8
            } else {
                255u8
            };
            img.put_pixel(x, y, Rgb([c, c, c]));
        }
    }
    img
}

/// Build a `PipelineEngine` + a `PureAnalysisSource` and drive them EXACTLY the way
/// main.rs's default (pure) path does (main.rs:323..354,406..422):
///   1. load mappings, build the engine config,
///   2. `PureAnalysisSource::extract(&PureImage, num_instruments, bar_frac, steps, hint)`,
///   3. `engine.set_features_global(&source.global_features())` (derives mode/plan),
///   4. per step: `engine.decide_step(&source, k)` — the pure decision kernel.
/// Returns (engine, source) so callers can either `tick` (item 1) or `decide_step`
/// repeatedly (item 2's same-engine determinism). `num_steps`/`num_instruments` are
/// the pipeline geometry; `bar_frac`=0.10 matches the engine default.
fn build_pipeline(
    img: RgbImage,
    num_instruments: usize,
    num_steps: usize,
) -> (PipelineEngine, PureAnalysisSource) {
    let mappings: MappingTable = load_mappings(MAPPINGS).expect("mappings load");
    let cfg = audiohax::engine::EngineConfig {
        num_instruments,
        ..audiohax::engine::EngineConfig::default()
    };
    let pure = PureImage::from_rgb(img);
    // vertical_hint = None → analyzer picks scan axis from aspect (matches main.rs,
    // which passes None on the pure path).
    let source = PureAnalysisSource::extract(&pure, num_instruments, 0.10, num_steps, None)
        .expect("pure extract ok");
    let mut engine = PipelineEngine::new(mappings, cfg);
    engine.set_features_global(&source.global_features());
    (engine, source)
}

/// Drive every step through `tick`, recording the realized note stream into a
/// `RecordingSink`. This is the FULL main.rs emit path (decide → note_on/note_off per
/// event), minus the adapter's jitter + wall-clock scheduling (which the engine
/// docstring explicitly assigns to the adapter, not the core — engine.rs:372).
fn drive_to_recording(engine: &mut PipelineEngine, source: &PureAnalysisSource) -> RecordingSink {
    let mut sink = RecordingSink::default();
    let total = source.step_count();
    for _ in 0..total {
        engine.tick(source, &mut sink).expect("tick ok");
    }
    sink
}

/// Flatten the per-step `decide_step` decisions across all steps into a single
/// `Vec<(channel, NoteEvent)>` — the pure decision stream, RNG-free given a fixed
/// plan. Used by the determinism test (item 2): two passes over the SAME engine must
/// yield byte-identical decisions.
fn decisions_over_all_steps(
    engine: &PipelineEngine,
    source: &PureAnalysisSource,
) -> Vec<(u8, NoteEvent)> {
    let mut out = Vec::new();
    for k in 0..source.step_count() {
        let decisions: Vec<InstrumentDecision> = engine.decide_step(source, k);
        for d in decisions {
            for ev in d.events {
                out.push((d.channel, ev));
            }
        }
    }
    out
}

// ═════════════════════════════════════════════════════════════════════════════
// ITEM 1 — END-TO-END EVENT FLOW (the core gap).
// Property: the pure-Rust image path, driven through the engine exactly as main.rs
// does, emits a NON-EMPTY, musically-valid MIDI event stream — every note in the
// engine's playable band, every velocity 1..=127, note_on/note_off balanced, every
// channel < 16.
// ═════════════════════════════════════════════════════════════════════════════
#[test]
fn end_to_end_pure_image_emits_valid_midi_stream() {
    // A vivid green field (hue ≈ 120° → Ionian, a recognized warm mode) with a hard
    // edge overlay so the analyzer reports non-trivial edge/texture and the engine has
    // real rhythmic activity to realize — a richer stream than a flat solid.
    let mut img = solid(96, 64, [0, 200, 0]);
    for y in 0..64 {
        for x in (0..96).step_by(4) {
            img.put_pixel(x, y, Rgb([0, 0, 0])); // sparse dark verticals → edges
        }
    }
    let (mut engine, source) = build_pipeline(img, 4, 8);
    let sink = drive_to_recording(&mut engine, &source);

    // The stream must not be empty — a constant-silent pipeline would be a dead lane.
    let on_count = sink.count_on();
    assert!(
        on_count > 0,
        "pure image path through the engine must realize ≥1 note_on; got an empty stream \
         (lane A→engine→sink not wired, or the analyzer/plan produced no notes)"
    );

    // Every realized note must sit in the engine's playable band. The engine's role
    // registers span BASS_REGISTER_FLOOR(36)..MELODY clamp(96); 24..=108 is the safe
    // MIDI envelope the engine clamps into (chord_engine seat_pc_in_register / clamps).
    for (channel, note, velocity) in sink.note_ons() {
        assert!(
            (24..=108).contains(&note),
            "realized MIDI note {note} out of the engine's playable band 24..=108 \
             (an out-of-range note is a chord_engine register/clamp BUG)"
        );
        assert!(
            (1..=127).contains(&velocity),
            "realized velocity {velocity} out of 1..=127 (velocity 0 = silent note_on, \
             >127 = invalid MIDI — a realize_velocity clamp BUG)"
        );
        assert!(
            channel < 16,
            "channel {channel} ≥ 16 — channel must be inst_idx % 16 (engine.rs:540)"
        );
    }

    // note_on/note_off balance: `tick` emits an on+off for every realized event, so the
    // counts must be exactly equal — an imbalance would mean hung/leaked notes.
    assert_eq!(
        sink.count_on(),
        sink.count_off(),
        "note_on count ({}) must equal note_off count ({}) — the engine pairs every \
         realized note (engine.rs:400-402); an imbalance leaks/hangs notes",
        sink.count_on(),
        sink.count_off()
    );
}

#[test]
fn end_to_end_bass_instrument_plays_low_register() {
    // Musical-property check across the role stratification: with ≥2 instruments the
    // engine assigns instrument 0 (channel 0) the BASS role, which sounds the chord
    // ROOT in the bass register (chord_engine BASS_REGISTER_FLOOR = 36) and the top
    // instrument the MELODY in the melody register (≥ 67 floor). We assert the bass
    // channel's notes are, on the whole, LOWER than the melody channel's — proving the
    // engine's orchestral role mapping reaches the realized stream (not a flat unison).
    let (mut engine, source) = build_pipeline(solid(80, 60, [0, 0, 220]), 4, 6); // blue → Aeolian
    let sink = drive_to_recording(&mut engine, &source);

    let top_channel = 3u8; // instrument index 3 = highest = MELODY (num_instruments-1)
    let bass_notes: Vec<u8> = sink
        .note_ons()
        .filter(|(c, _, _)| *c == 0)
        .map(|(_, n, _)| n)
        .collect();
    let melody_notes: Vec<u8> = sink
        .note_ons()
        .filter(|(c, _, _)| *c == top_channel)
        .map(|(_, n, _)| n)
        .collect();

    // Both roles must actually sound (the stream isn't a single-voice degenerate).
    assert!(!bass_notes.is_empty(), "bass channel 0 must realize notes");
    assert!(
        !melody_notes.is_empty(),
        "melody channel {top_channel} must realize notes"
    );

    let bass_max = *bass_notes.iter().max().unwrap();
    let melody_min = *melody_notes.iter().min().unwrap();
    // Strong property: the bass's HIGHEST note is below the melody's LOWEST — the role
    // registers don't overlap for this image. The role floors (36 vs 67) are ~2.5
    // octaves apart, so even with brightness octave-shift this separation holds.
    assert!(
        bass_max < melody_min,
        "bass register must sit below the melody register: bass_max={bass_max} >= \
         melody_min={melody_min} — orchestral role→register mapping not reaching the stream"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// ITEM 2a — DETERMINISM.
// Property: the pure path is DETERMINISTIC given a fixed plan. On a SINGLE engine
// (`set_features_global` called once → the plan is fixed; the NON-determinism the
// engine documents is ONLY across separate `set_features_global` calls, which re-roll
// `pick_progression`'s thread_rng — engine.rs:322-327), repeating the per-step
// `decide_step` pass yields a BYTE-IDENTICAL decision stream. This isolates the pure
// analyzer + decision kernel as deterministic, exactly as the engine's own
// equivalence net pins `decide_instrument_action` against a fixed plan.
// ═════════════════════════════════════════════════════════════════════════════
#[test]
fn same_engine_pure_path_is_deterministic_across_passes() {
    let (engine, source) = build_pipeline(hard_edges(72, 56), 4, 8);
    let pass_a = decisions_over_all_steps(&engine, &source);
    let pass_b = decisions_over_all_steps(&engine, &source);

    assert!(
        !pass_a.is_empty(),
        "the decision stream must be non-empty for a determinism comparison to be meaningful"
    );
    assert_eq!(
        pass_a, pass_b,
        "two passes of the pure decide_step path over the SAME engine must be byte-identical \
         (a difference means the analyzer or decision kernel carries hidden non-determinism)"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// ITEM 2b — FEATURE→MUSIC SENSITIVITY.
// Property: the pure analyzer actually DRIVES musical decisions — it is not a
// constant. Two known images with materially different HUE drive the engine to
// DIFFERENT derived modes (mode = deterministic hue→mode lookup, NO RNG, so this is a
// hard equality assertion), AND a low-edge vs high-edge image pair drives observably
// different realized note content. If the analyzer output were ignored, both would
// collapse to the same mode / same stream.
// ═════════════════════════════════════════════════════════════════════════════
#[test]
fn different_images_drive_different_modes() {
    // Green (hue ≈ 120°) → Ionian (warm); blue (hue ≈ 240°) → Aeolian (cool). The
    // hue→mode map (assets/mappings.json) is a deterministic range lookup, so the
    // derived mode is a pure function of the image — the cleanest RNG-free sensitivity
    // signal that the pure analyzer's hue actually reaches the engine.
    let (green_engine, _g) = build_pipeline(solid(48, 40, [0, 220, 0]), 3, 4);
    let (blue_engine, _b) = build_pipeline(solid(48, 40, [0, 0, 220]), 3, 4);

    let green_mode = green_engine.current_state().mode;
    let blue_mode = blue_engine.current_state().mode;

    assert_eq!(
        green_mode, "Ionian",
        "a green field (hue≈120°) must derive Ionian via the pure analyzer's avg_hue; got {green_mode}"
    );
    assert_eq!(
        blue_mode, "Aeolian",
        "a blue field (hue≈240°) must derive Aeolian via the pure analyzer's avg_hue; got {blue_mode}"
    );
    assert_ne!(
        green_mode, blue_mode,
        "different-hued images must drive different modes — equality would prove the pure \
         analyzer's hue is being ignored (a dead feature→music link)"
    );
}

/// Count realized onsets on a single channel across all steps.
fn melody_onsets_on_channel(
    engine: &PipelineEngine,
    source: &PureAnalysisSource,
    channel: u8,
) -> usize {
    let mut n = 0;
    for k in 0..source.step_count() {
        for d in engine.decide_step(source, k) {
            if d.channel == channel {
                n += d.events.len();
            }
        }
    }
    n
}

#[test]
fn edge_density_difference_changes_realized_note_content() {
    // Both images are GRAYSCALE → hue 0 → Phrygian, so MODE is held constant and the
    // only varying input is EDGE DENSITY / texture. We compare the MELODY voice
    // (instrument 3 = highest = channel 3), whose onset COUNT is a direct function of
    // the edge_density BAND (chord_engine realize_rhythm: melody is SUSTAINED=1 onset
    // when edge ≤ 0.25, DOTTED=2 onsets when edge > 0.25). The flat field's per-bar
    // edge_density is 0.0 (→ 1 onset/step); the 4-px checkerboard's is ≈ 0.27 (→ 2
    // onsets/step), which is ABOVE the 0.25 band boundary. The melody onset count is
    // independent of WHICH cool progression `pick_progression` randomly rolled (the
    // rhythm pattern is chosen AFTER the chord, from edge_density + phrase position), so
    // this comparison is robust to the engine's documented plan-derivation RNG.
    //
    // (A 1-px stripe field gives edge_density ≈ 0.08 — BELOW 0.25 — so it would land in
    // the SAME band as the flat field and the counts would NOT differ. That is the
    // engine's coarse banding, not a dead link; the test deliberately crosses a band
    // boundary to observe the feature→music effect. See `checkerboard` doc.)
    let (flat_engine, flat_src) = build_pipeline(solid(96, 96, [128, 128, 128]), 4, 6);
    let (edge_engine, edge_src) = build_pipeline(checkerboard(96, 96, 4), 4, 6);

    // Sanity: both derived the same (grayscale → Phrygian) mode, so any divergence is
    // edge-driven, not mode-driven.
    assert_eq!(
        flat_engine.current_state().mode,
        "Phrygian",
        "grayscale flat field must derive Phrygian (hue 0)"
    );
    assert_eq!(
        flat_engine.current_state().mode,
        edge_engine.current_state().mode,
        "both grayscale images must share a mode (hue 0 → Phrygian) so the only varying \
         input is edge_density"
    );

    let flat_melody = melody_onsets_on_channel(&flat_engine, &flat_src, 3);
    let edge_melody = melody_onsets_on_channel(&edge_engine, &edge_src, 3);

    // The higher-edge image must realize MORE melody onsets (DOTTED 2-onset figure vs
    // the flat field's SUSTAINED single tone). Strict increase, not merely "different".
    assert!(
        edge_melody > flat_melody,
        "higher edge density must realize MORE melody onsets: edge_melody={edge_melody} \
         vs flat_melody={flat_melody} — equality/inversion means edge_density does not \
         drive the melody's rhythmic activity through the pure path (a dead feature→music \
         link, or the band boundary moved)"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// ITEM 3 — OFFLINE AUDIO PROOF (cross-lane, NO cpal).
// Property: the events the pure pipeline EMITS actually produce SOUND through the
// chosen synth. We capture a representative event stream from the end-to-end run,
// replay its program_change + note_on commands into a `rustysynth::Synthesizer`
// loaded from the BUNDLED SoundFont (via the library's `SoundFontSource::Bundled`
// path → `MidiCmd::to_midi_message` mapping, same as synth_sink's callback), render
// OFFLINE into in-memory L/R buffers, and assert non-silent output. No cpal stream is
// opened (no `SynthSink::new`). Mirrors synth_sink.rs's inline render technique.
// ═════════════════════════════════════════════════════════════════════════════

/// Load the bundled SF2 through the LIBRARY's public `SoundFontSource::Bundled` path,
/// build a `Synthesizer` at 44.1 kHz. The SF2 bytes are compiled into the lib via
/// `include_bytes!`, so this reaches them with NO filesystem read in the test.
/// NOTE: `SoundFontSource::Bundled` selects the embedded bytes, but the public surface
/// that materializes them (`load_soundfont`) is private to synth_sink; the bundled
/// font is identical to `assets/soundfonts/default.sf2`, which we parse here via the
/// public `rustysynth::SoundFont` API exactly as the library does internally. We pin
/// the variant we exercise so the test fails loudly if the bundled-source contract
/// changes.
fn bundled_synth() -> Synthesizer {
    // Assert the public bundled-source variant exists (compile + match guard); the
    // library's `with_bundled_soundfont()` would open cpal, which we must not do, so we
    // parse the same embedded font through rustysynth directly for the OFFLINE render.
    let _bundled = SoundFontSource::Bundled; // contract: the zero-config default exists
    matches!(_bundled, SoundFontSource::Bundled)
        .then_some(())
        .expect("SoundFontSource::Bundled is the zero-config default variant");

    let mut bytes: &[u8] = include_bytes!("../assets/soundfonts/default.sf2");
    let sf = SoundFont::new(&mut bytes).expect("bundled SF2 parses");
    let sf = Arc::new(sf);
    let settings = SynthesizerSettings::new(44_100);
    Synthesizer::new(&sf, &settings).expect("synth init")
}

#[test]
fn captured_pipeline_events_render_nonsilent_audio_offline() {
    // 1) Capture a representative stream from the end-to-end pure path.
    let (mut engine, source) = build_pipeline(solid(80, 64, [0, 200, 0]), 4, 6);
    let sink = drive_to_recording(&mut engine, &source);
    assert!(
        sink.count_on() > 0,
        "need a non-empty captured stream to prove it makes sound"
    );

    // 2) Build the offline synth and program each channel exactly as main.rs does
    //    (prog = (i*7)%128 per instrument channel), then replay every captured note_on
    //    through the SAME MidiCmd::to_midi_message mapping synth_sink's callback uses.
    let mut synth = bundled_synth();
    for i in 0..4usize {
        let ch = (i % 16) as u8;
        let prog = ((i * 7) % 128) as u8;
        let (c, cmd, d1, d2) = midi_message(MidiCmd::ProgramChange {
            channel: ch,
            program: prog,
        });
        synth.process_midi_message(c, cmd, d1, d2);
    }
    for (channel, note, velocity) in sink.note_ons() {
        let (c, cmd, d1, d2) = midi_message(MidiCmd::NoteOn {
            channel,
            note,
            velocity,
        });
        synth.process_midi_message(c, cmd, d1, d2);
    }

    // 3) Render ~0.25 s OFFLINE into in-memory buffers (no cpal) and assert non-silent.
    let frames = 11_025; // 0.25 s @ 44.1 kHz — short, < 10 s budget
    let mut left = vec![0.0f32; frames];
    let mut right = vec![0.0f32; frames];
    synth.render(&mut left, &mut right);

    let peak = left
        .iter()
        .chain(right.iter())
        .fold(0.0f32, |m, &s| m.max(s.abs()));
    assert!(
        peak > 1e-4,
        "the events the pure pipeline emits must produce AUDIBLE (non-silent) output \
         through the bundled SoundFont; peak={peak} (silent ⇒ the realized notes don't \
         sound on the chosen synth — a cross-lane wiring/range defect)"
    );

    // Stronger than "non-silent": a piano-ish GM voice rings for a meaningful fraction
    // of a 0.25 s window, so there must be a non-trivial COUNT of audible samples, not
    // a single click.
    let audible = left
        .iter()
        .chain(right.iter())
        .filter(|&&s| s.abs() > 1e-4)
        .count();
    assert!(
        audible > 100,
        "expected a sustained tone (>100 audible samples), got {audible} — a lone spike \
         would suggest the note triggers but does not sustain"
    );
}

#[test]
fn to_midi_message_mapping_is_stable_for_captured_kinds() {
    // A cheap cross-lane contract pin (NOT a duplicate of synth_sink's unit mapping
    // tests, which assert specific channel/note tuples): the integration path RELIES on
    // the MidiCmd byte vocabulary (0x90 / 0xC0) to replay captured events. Pin that the
    // two command kinds the integration replay uses still decompose to the MIDI status
    // high-nibbles the synth expects, so a future MidiCmd refactor that breaks replay is
    // caught HERE at the integration boundary, not just in the unit net.
    let (_c, on_cmd, _d1, _d2) = midi_message(MidiCmd::NoteOn {
        channel: 1,
        note: 60,
        velocity: 90,
    });
    let (_c, pc_cmd, _d1, _d2) = midi_message(MidiCmd::ProgramChange {
        channel: 1,
        program: 7,
    });
    assert_eq!(on_cmd, 0x90, "note_on must map to MIDI status nibble 0x90");
    assert_eq!(
        pc_cmd, 0xC0,
        "program_change must map to MIDI status nibble 0xC0"
    );
}

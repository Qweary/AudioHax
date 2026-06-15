//! tests/engine_seam.rs — integration suite for the WS-4 Phase 1 engine seam
//! (design S9 §3.1). Proves the `PipelineEngine` / `FeatureSource` / `AudioSink`
//! seam behaves per contract: tick drives the sink with the decided notes, the
//! engine core is PURE/deterministic, sink errors propagate gracefully, and the
//! snapshot reflects live position. Headless — no OpenCV / midir / image.
//!
//! These live in a tests/*.rs file (file-disjoint from the Implementer's inline
//! `#[cfg(test)]` block in src/engine.rs) and exercise the public surface only.

use audiohax::chord_engine::{Chord, NoteEvent, PhrasePosition, StepPlan};
use audiohax::engine::{
    decide_instrument_action, AudioSink, AudioSinkError, CadenceStrength, EngineCommand,
    EngineConfig, FeatureSource, GlobalFeatures, InteractionEvent, KeyTempoPlan, PipelineEngine,
    ScanBarFeatures, Section, StepContext, ThematicRole, ThemeVariation,
};

// ─────────────────────────────────────────────────────────────────────────────
// Test doubles
// ─────────────────────────────────────────────────────────────────────────────

/// A deterministic, parameterizable `FeatureSource` over canned data: a fixed
/// `GlobalFeatures` and a known `Vec<Vec<ScanBarFeatures>>` (one row per step).
/// No OpenCV, no image — pure constants.
struct MockSource {
    global: GlobalFeatures,
    rows: Vec<Vec<ScanBarFeatures>>,
}

impl FeatureSource for MockSource {
    fn global_features(&self) -> GlobalFeatures {
        self.global
    }
    fn scan_bar_features(&self, step_idx: usize, num_instruments: usize) -> Vec<ScanBarFeatures> {
        // Mirror the engine's own canned-source contract: index the row, pad /
        // truncate to exactly `num_instruments` so decide_step gets one bar per
        // instrument regardless of how the fixture was authored.
        let mut row = self.rows.get(step_idx).cloned().unwrap_or_default();
        row.truncate(num_instruments);
        while row.len() < num_instruments {
            row.push(bar(row.len(), 50.0, 50.0, 0.2));
        }
        row
    }
    fn step_count(&self) -> usize {
        self.rows.len()
    }
}

/// One recorded MIDI call, tagged by kind. The RecordingSink captures the full
/// argument tuple of every note_on/note_off/program_change so a test can assert
/// the engine emitted exactly the (channel, note, velocity) it decided.
#[derive(Debug, Clone, PartialEq, Eq)]
enum MidiCall {
    NoteOn { channel: u8, note: u8, velocity: u8 },
    NoteOff { channel: u8, note: u8 },
    ProgramChange { channel: u8, program: u8 },
}

/// An `AudioSink` that records every call in order. Optionally fails on the Nth
/// note_on (1-based) to exercise error propagation.
#[derive(Default)]
struct RecordingSink {
    calls: Vec<MidiCall>,
    fail_on_note_on: Option<usize>, // 1-based index of the note_on to fail
    note_on_count: usize,
}

impl RecordingSink {
    fn failing_on(n: usize) -> Self {
        RecordingSink {
            fail_on_note_on: Some(n),
            ..Default::default()
        }
    }
    fn note_ons(&self) -> Vec<(u8, u8, u8)> {
        self.calls
            .iter()
            .filter_map(|c| match c {
                MidiCall::NoteOn {
                    channel,
                    note,
                    velocity,
                } => Some((*channel, *note, *velocity)),
                _ => None,
            })
            .collect()
    }
    fn count(&self, f: impl Fn(&MidiCall) -> bool) -> usize {
        self.calls.iter().filter(|c| f(c)).count()
    }
}

impl AudioSink for RecordingSink {
    fn note_on(&mut self, channel: u8, note: u8, velocity: u8) -> Result<(), AudioSinkError> {
        self.note_on_count += 1;
        if self.fail_on_note_on == Some(self.note_on_count) {
            return Err(AudioSinkError::msg("mock sink forced failure"));
        }
        self.calls.push(MidiCall::NoteOn {
            channel,
            note,
            velocity,
        });
        Ok(())
    }
    fn note_off(&mut self, channel: u8, note: u8) -> Result<(), AudioSinkError> {
        self.calls.push(MidiCall::NoteOff { channel, note });
        Ok(())
    }
    fn program_change(&mut self, channel: u8, program: u8) -> Result<(), AudioSinkError> {
        self.calls
            .push(MidiCall::ProgramChange { channel, program });
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Fixtures
// ─────────────────────────────────────────────────────────────────────────────

fn bar(idx: usize, sat: f32, bright: f32, edge: f32) -> ScanBarFeatures {
    ScanBarFeatures {
        bar_index: idx,
        avg_hue: 0.0,
        avg_saturation: sat,
        avg_brightness: bright,
        edge_density: edge,
        texture_laplacian_var: 0.0,
        hue_hist: Vec::new(),
    }
}

fn global(hue: f32) -> GlobalFeatures {
    GlobalFeatures {
        avg_hue: hue,
        avg_saturation: 60.0,
        avg_brightness: 55.0,
        edge_density: 0.3,
        hue_spread: 0.2,
        texture_laplacian_var: 1.0,
        shape_complexity: 0.1,
        aspect_ratio: 1.5,
    }
}

fn load_mappings() -> audiohax::mapping_loader::MappingTable {
    audiohax::mapping_loader::load_mappings("assets/mappings.json").expect("mappings load")
}

// ─────────────────────────────────────────────────────────────────────────────
// A. ENGINE SEAM — tick drives the sink with the decided notes
// ─────────────────────────────────────────────────────────────────────────────

/// Property: every note in every InstrumentDecision tick() returned is emitted to
/// the sink as a note_on with the decision's channel + the NoteEvent's note/vel,
/// and each note_on has a paired note_off on the same (channel, note).
#[test]
fn test_tick_emits_decided_notes_to_sink_in_midi_range() {
    let cfg = EngineConfig {
        num_instruments: 3,
        ..EngineConfig::default()
    };
    let mut engine = PipelineEngine::new(load_mappings(), cfg);
    let source = MockSource {
        global: global(120.0),
        rows: vec![vec![
            bar(0, 70.0, 60.0, 0.4),
            bar(1, 70.0, 60.0, 0.4),
            bar(2, 70.0, 60.0, 0.4),
        ]],
    };
    engine.set_features_global(&source.global_features());
    let mut sink = RecordingSink::default();
    let out = engine.tick(&source, &mut sink).expect("tick ok");

    // Reconstruct the (channel, note, velocity) set the engine DECIDED.
    let mut expected_ons: Vec<(u8, u8, u8)> = Vec::new();
    for dec in &out.decisions {
        for ev in &dec.events {
            // Channel is inst_idx % 16 → here 0,1,2.
            assert!(dec.channel < 16, "channel must be a valid MIDI channel");
            // MIDI note range the realizer is documented to stay inside (24..=108);
            // velocities are clamped 1..=127.
            assert!(
                (24..=108).contains(&ev.note),
                "note {} out of realizer band 24..=108",
                ev.note
            );
            assert!(
                (1..=127).contains(&ev.velocity),
                "velocity {} out of MIDI range 1..=127",
                ev.velocity
            );
            expected_ons.push((dec.channel, ev.note, ev.velocity));
        }
    }
    assert!(
        !expected_ons.is_empty(),
        "non-empty plan should sound notes"
    );

    // The sink saw EXACTLY those note_ons, in order.
    assert_eq!(
        sink.note_ons(),
        expected_ons,
        "every decided note must reach the sink as a note_on with matching channel/note/vel"
    );
    // Every note_on is paired with a note_off (same count).
    let ons = sink.count(|c| matches!(c, MidiCall::NoteOn { .. }));
    let offs = sink.count(|c| matches!(c, MidiCall::NoteOff { .. }));
    assert_eq!(ons, offs, "note_on must be paired with note_off");
}

/// Property: across a full scan, tick() advances step_index by exactly 1 each call
/// and scan_position monotonically increases to 1.0 at the last step.
#[test]
fn test_tick_advances_step_and_position_monotonically() {
    let cfg = EngineConfig {
        num_instruments: 2,
        ..EngineConfig::default()
    };
    let mut engine = PipelineEngine::new(load_mappings(), cfg);
    let source = MockSource {
        global: global(200.0),
        rows: vec![
            vec![bar(0, 40.0, 30.0, 0.5), bar(1, 40.0, 30.0, 0.5)],
            vec![bar(0, 40.0, 30.0, 0.5), bar(1, 40.0, 30.0, 0.5)],
            vec![bar(0, 40.0, 30.0, 0.5), bar(1, 40.0, 30.0, 0.5)],
            vec![bar(0, 40.0, 30.0, 0.5), bar(1, 40.0, 30.0, 0.5)],
        ],
    };
    engine.set_features_global(&source.global_features());
    let mut sink = RecordingSink::default();

    let mut last_pos = -1.0_f32;
    for k in 0..4usize {
        let out = engine.tick(&source, &mut sink).expect("tick ok");
        assert_eq!(out.step_index, k, "tick reports the step it processed");
        assert!(
            out.scan_position > last_pos,
            "scan_position must strictly increase ({} !> {})",
            out.scan_position,
            last_pos
        );
        last_pos = out.scan_position;
        // current_state advances in lockstep (step_index is post-increment).
        assert_eq!(engine.current_state().step_index, k + 1);
    }
    assert!(
        (last_pos - 1.0).abs() < 1e-6,
        "scan_position should reach 1.0 at the final step, got {last_pos}"
    );
}

/// Property: tick() returns Ok and the Eq-comparable decisions it returns equal a
/// second decide_step() at the same step (tick does not perturb the decision).
#[test]
fn test_tick_decisions_equal_decide_step() {
    let cfg = EngineConfig {
        num_instruments: 4,
        ..EngineConfig::default()
    };
    let mut engine = PipelineEngine::new(load_mappings(), cfg);
    let source = MockSource {
        global: global(45.0),
        rows: vec![vec![bar(0, 80.0, 70.0, 0.6); 4]],
    };
    engine.set_features_global(&source.global_features());
    let pure = engine.decide_step(&source, 0);
    let mut sink = RecordingSink::default();
    let out = engine.tick(&source, &mut sink).expect("tick ok");
    assert_eq!(
        out.decisions, pure,
        "tick's decisions must equal the pure decide_step for the same step"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// A. PURITY / DETERMINISM
// ─────────────────────────────────────────────────────────────────────────────

/// Property: decide_step is PURE — two calls with identical inputs return
/// byte-identical InstrumentDecisions (relies on the Eq impl).
#[test]
fn test_decide_step_is_deterministic() {
    let cfg = EngineConfig {
        num_instruments: 5,
        ..EngineConfig::default()
    };
    let mut engine = PipelineEngine::new(load_mappings(), cfg);
    let source = MockSource {
        global: global(310.0),
        rows: vec![
            vec![bar(0, 55.0, 45.0, 0.35); 5],
            vec![bar(0, 22.0, 88.0, 0.05); 5],
        ],
    };
    engine.set_features_global(&source.global_features());
    for step in 0..2usize {
        let a = engine.decide_step(&source, step);
        let b = engine.decide_step(&source, step);
        assert_eq!(a, b, "decide_step must be deterministic at step {step}");
        assert_eq!(a.len(), 5, "one decision per instrument");
    }
}

/// S15 seam: the behaviour-neutral default Section + KeyTempoPlan this net borrows
/// into the new `ctx` arg. `theme:None` ⇒ the realizer free-selects exactly as
/// before — the determinism property is unaffected (mirrors engine_equivalence.rs).
fn seam_section(plan: &[StepPlan]) -> Section {
    Section {
        label: "A".to_string(),
        step_len: plan.len(),
        thematic_role: ThematicRole::Statement,
        key_offset_semitones: 0,
        ms_per_step: 250,
        mode: "Ionian".to_string(),
        progression: vec![],
        theme: None,
        variation: ThemeVariation::Identity,
        boundary_cadence: CadenceStrength::Perfect,
        density: 0.5,
        steps: plan.to_vec(),
    }
}

fn seam_key_tempo() -> KeyTempoPlan {
    KeyTempoPlan {
        home_root_midi: 60,
        home_mode: "Ionian".to_string(),
        base_ms_per_step: 250,
        key_scheme: vec![0],
        tempo_scheme: vec![250],
    }
}

/// Property: the free fn decide_instrument_action is pure — identical args give
/// byte-identical output, independent of any engine state.
#[test]
fn test_decide_instrument_action_is_deterministic() {
    let plan = fixed_plan();
    let f = bar(0, 64.0, 48.0, 0.33);
    let kt = seam_key_tempo();
    let sec = seam_section(&plan);
    let ctx = StepContext::single_section_default(&sec, &kt);
    for &(inst, step) in &[(0usize, 0usize), (1, 1), (2, 5), (7, 3)] {
        let a = decide_instrument_action(&f, inst, step, 4, &plan, 250, &ctx);
        let b = decide_instrument_action(&f, inst, step, 4, &plan, 250, &ctx);
        assert_eq!(
            a, b,
            "decide_instrument_action({inst},{step}) not deterministic"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// A. AudioSink error propagation
// ─────────────────────────────────────────────────────────────────────────────

/// Property: a sink that errors on the Nth note_on makes tick() return Err
/// (AudioSinkError) gracefully — no panic, error surfaces as a Result.
#[test]
fn test_tick_propagates_sink_error_without_panic() {
    let cfg = EngineConfig {
        num_instruments: 4,
        ..EngineConfig::default()
    };
    let mut engine = PipelineEngine::new(load_mappings(), cfg);
    let source = MockSource {
        global: global(90.0),
        rows: vec![vec![bar(0, 70.0, 60.0, 0.4); 4]],
    };
    engine.set_features_global(&source.global_features());
    // Fail on the very first note_on — the engine sounds ≥1 note per step.
    let mut sink = RecordingSink::failing_on(1);
    let res = engine.tick(&source, &mut sink);
    assert!(
        res.is_err(),
        "a sink error must propagate out of tick as Err, not a panic"
    );
}

/// Property: AudioSinkError carries its message (Display) so the adapter can log it.
#[test]
fn test_audio_sink_error_display_carries_message() {
    let e = AudioSinkError::msg("port closed");
    let s = format!("{e}");
    assert!(
        s.contains("port closed"),
        "AudioSinkError Display must surface the underlying message, got {s:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// A. current_state / EngineSnapshot reflects live position
// ─────────────────────────────────────────────────────────────────────────────

/// Property: after N ticks the snapshot reports step_index == N, a scan_position
/// matching the last tick, a non-empty derived mode, and the fed global features.
#[test]
fn test_snapshot_reflects_position_after_ticks() {
    let cfg = EngineConfig {
        num_instruments: 2,
        ..EngineConfig::default()
    };
    let mut engine = PipelineEngine::new(load_mappings(), cfg);
    let g = global(275.0);
    let source = MockSource {
        global: g,
        rows: vec![
            vec![bar(0, 50.0, 50.0, 0.3), bar(1, 50.0, 50.0, 0.3)],
            vec![bar(0, 50.0, 50.0, 0.3), bar(1, 50.0, 50.0, 0.3)],
            vec![bar(0, 50.0, 50.0, 0.3), bar(1, 50.0, 50.0, 0.3)],
        ],
    };
    engine.set_features_global(&g);
    let mut sink = RecordingSink::default();
    let mut last_pos = 0.0;
    for _ in 0..2 {
        last_pos = engine.tick(&source, &mut sink).expect("tick").scan_position;
    }
    let snap = engine.current_state();
    assert_eq!(snap.step_index, 2, "snapshot tracks ticks performed");
    assert!(
        (snap.scan_position - last_pos).abs() < 1e-6,
        "snapshot scan_position must match the last tick's"
    );
    assert_eq!(snap.global, g, "snapshot carries the fed global features");
    assert!(
        !snap.mode.is_empty(),
        "a mode is derived after feeding features"
    );
    // last_notes is the flattened set the last tick sounded.
    assert!(
        !snap.last_notes.is_empty(),
        "snapshot last_notes reflects the notes just sounded"
    );
}

/// Property: EngineCommand::Pause makes tick a no-op (no advance, no sink calls);
/// Stop resets the scan to the start. Confirms the transport contract.
#[test]
fn test_pause_and_stop_transport_contract() {
    let mut engine = PipelineEngine::new(load_mappings(), EngineConfig::default());
    let source = MockSource {
        global: global(15.0),
        rows: vec![vec![bar(0, 50.0, 50.0, 0.2); 4]; 3],
    };
    engine.set_features_global(&source.global_features());
    let mut sink = RecordingSink::default();

    // Advance one real tick.
    engine.tick(&source, &mut sink).expect("tick");
    assert_eq!(engine.current_state().step_index, 1);

    // Pause: tick becomes a no-op.
    engine.command(EngineCommand::Pause);
    let calls_before = sink.calls.len();
    let out = engine.tick(&source, &mut sink).expect("paused tick");
    assert!(out.decisions.is_empty(), "paused tick decides nothing");
    assert_eq!(engine.current_state().step_index, 1, "paused: no advance");
    assert_eq!(sink.calls.len(), calls_before, "paused: no sink calls");

    // Stop resets position to the start.
    engine.command(EngineCommand::Stop);
    let snap = engine.current_state();
    assert_eq!(snap.step_index, 0, "stop resets step index");
    assert!(snap.scan_position.abs() < 1e-6, "stop resets scan position");
}

/// Property: inject_event(Seek) clamps and moves the scan position into [0,1].
#[test]
fn test_inject_seek_clamps_position() {
    let mut engine = PipelineEngine::new(load_mappings(), EngineConfig::default());
    engine.inject_event(InteractionEvent::Seek(0.42));
    assert!((engine.current_state().scan_position - 0.42).abs() < 1e-6);
    engine.inject_event(InteractionEvent::Seek(5.0));
    assert!(
        (engine.current_state().scan_position - 1.0).abs() < 1e-6,
        "out-of-range seek clamps to 1.0"
    );
    engine.inject_event(InteractionEvent::Seek(-3.0));
    assert!(
        engine.current_state().scan_position.abs() < 1e-6,
        "negative seek clamps to 0.0"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Shared fixed plan (no thread_rng) — used by determinism tests here and the
// batch-equivalence golden in tests/engine_equivalence.rs uses its own copy.
// ─────────────────────────────────────────────────────────────────────────────

fn fixed_plan() -> Vec<StepPlan> {
    let chord = Chord {
        name: "I".to_string(),
        notes: vec![60, 64, 67],
    };
    vec![
        StepPlan {
            chord: chord.clone(),
            phrase_index: 0,
            position_in_phrase: 0,
            phrase_len: 4,
            position: PhrasePosition::PhraseStart,
            velocity: 80,
        },
        StepPlan {
            chord,
            phrase_index: 0,
            position_in_phrase: 1,
            phrase_len: 4,
            position: PhrasePosition::Interior,
            velocity: 72,
        },
    ]
}

/// Tiny compile-time/usage guard so `NoteEvent` import is exercised even if the
/// realizer band changes — keeps the import meaningful.
#[allow(dead_code)]
fn _note_event_shape(e: &NoteEvent) -> (u8, u8, u64, u64) {
    (e.note, e.velocity, e.hold_ms, e.offset_ms)
}

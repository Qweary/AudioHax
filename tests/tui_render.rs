//! tests/tui_render.rs — WS-4 Phase 3 (S10) integration tests for the ratatui TUI
//! front-end (`audiohax::tui`), a PURE OBSERVER over the S9 `engine.rs` shared core.
//!
//! Test groups:
//!   GROUP A — TestBackend RENDER tests: render a HAND-CONSTRUCTED fixed
//!     `EngineSnapshot` onto `ratatui::backend::TestBackend` and assert on the
//!     in-memory `Buffer`. Render is a pure function of the snapshot, so these are
//!     fully deterministic and assert on exact rendered content.
//!   GROUP B — SyntheticSource determinism / shape / range (the source is RNG-free).
//!   GROUP C — DRIVE / OBSERVER SHAPE tests: drive a live engine and assert
//!     invariants only (scan advance, observer frame count, one decision per
//!     instrument) — NEVER on note/velocity/mode VALUES, which are `thread_rng`-
//!     derived in `set_features_global` → `pick_progression` and so non-deterministic.

use audiohax::chord_engine::{NoteEvent, PerfFeatures, PhrasePosition};
use audiohax::engine::{AudioSink, EngineObserver, EngineSnapshot, FeatureSource, GlobalFeatures};
use audiohax::tui;
use ratatui::backend::TestBackend;
use ratatui::Terminal;

// ─────────────────────────────────────────────────────────────────────────────
// Shared helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Flatten a `TestBackend`'s rendered `Buffer` into one `String` by concatenating
/// every cell's symbol in buffer order. Layout-stable for a fixed (W, H).
fn flatten(terminal: &Terminal<TestBackend>) -> String {
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|c| c.symbol())
        .collect()
}

/// Render `snap` onto a fresh `W x H` TestBackend and return the (terminal, flattened
/// text). Asserts the draw itself succeeds (render is infallible by contract).
fn render_to_text(w: u16, h: u16, snap: &EngineSnapshot) -> (Terminal<TestBackend>, String) {
    let backend = TestBackend::new(w, h);
    let mut terminal = Terminal::new(backend).expect("test backend terminal");
    terminal
        .draw(|f| tui::render(f, snap))
        .expect("render must not error");
    let text = flatten(&terminal);
    (terminal, text)
}

/// A hand-built FIXED snapshot with distinctive, render-checkable values. No engine,
/// no RNG — every field is set by hand so the render output is fully deterministic.
fn fixed_snapshot(scan_position: f32, mode: &str, notes: Vec<NoteEvent>) -> EngineSnapshot {
    EngineSnapshot {
        scan_position,
        step_index: 7,
        global: GlobalFeatures {
            avg_hue: 180.0,
            avg_saturation: 60.0,
            avg_brightness: 40.0,
            edge_density: 0.5,
            hue_spread: 0.2,
            texture_laplacian_var: 1.0,
            shape_complexity: 0.1,
            aspect_ratio: 1.5,
        },
        current_step: PerfFeatures {
            saturation: 55.0,
            brightness: 45.0,
            edge_density: 0.4,
        },
        last_notes: notes,
        mode: mode.to_string(),
        phrase: PhrasePosition::PhraseStart,
    }
}

/// Two distinctive notes with known MIDI numbers so the rendered `note@velocity`
/// line is assertable.
fn two_notes() -> Vec<NoteEvent> {
    vec![
        NoteEvent {
            note: 60,
            velocity: 88,
            hold_ms: 200,
            offset_ms: 0,
        },
        NoteEvent {
            note: 67,
            velocity: 72,
            hold_ms: 200,
            offset_ms: 0,
        },
    ]
}

/// A no-op observer-free sink for drive tests (mirrors `tui::NullSink`'s contract;
/// the tests use the production `tui::NullSink`).
fn null_sink() -> tui::NullSink {
    tui::NullSink
}

// ═════════════════════════════════════════════════════════════════════════════
// GROUP A — TestBackend RENDER tests (pure-snapshot, deterministic).
// ═════════════════════════════════════════════════════════════════════════════

/// A1: a fixed snapshot renders its meter LABELS, the mode string, the note numbers,
/// the step index, and the transport scan reading into the buffer.
#[test]
fn render_surfaces_labels_mode_notes_and_transport() {
    let snap = fixed_snapshot(0.5, "Dorian", two_notes());
    let (_t, text) = render_to_text(80, 24, &snap);

    // Feature-meter labels (render.rs uses lowercase "hue"/"sat"/"bright"/"edge").
    assert!(text.contains("hue"), "hue meter label must render");
    assert!(text.contains("sat"), "sat meter label must render");
    assert!(text.contains("bright"), "bright meter label must render");
    assert!(text.contains("edge"), "edge meter label must render");
    // The meters block title.
    assert!(
        text.contains("feature meters"),
        "feature-meters block title must render"
    );

    // Music-state mode.
    assert!(text.contains("Dorian"), "the mode string must render");
    assert!(text.contains("mode:"), "the mode label must render");

    // Last-notes line: both note numbers surfaced as note@velocity.
    assert!(text.contains('@'), "notes render as note@velocity");
    assert!(
        text.contains("60"),
        "note 60 must be surfaced in last_notes"
    );
    assert!(
        text.contains("67"),
        "note 67 must be surfaced in last_notes"
    );

    // Transport: the step index (snapshot.step_index == 7) and the scan reading.
    assert!(
        text.contains("step 7"),
        "transport title carries the step index"
    );
    // scan_position 0.5 -> "scan  50.0%" (format!("scan {:5.1}%", 50.0)). The Gauge
    // renders its label into the buffer; assert the percent reading is present.
    assert!(
        text.contains("50.0%"),
        "transport scan reading reflects 0.5 -> 50.0%; got:\n{text}"
    );
}

/// A2: degenerate input — EMPTY last_notes + zeroed features — must NOT panic and
/// must still render the meter labels and the explicit "silent" placeholder.
#[test]
fn render_handles_empty_notes_and_zeroed_features() {
    let mut snap = fixed_snapshot(0.0, "Ionian", Vec::new());
    snap.global = GlobalFeatures {
        avg_hue: 0.0,
        avg_saturation: 0.0,
        avg_brightness: 0.0,
        edge_density: 0.0,
        hue_spread: 0.0,
        texture_laplacian_var: 0.0,
        shape_complexity: 0.0,
        aspect_ratio: 0.0,
    };
    snap.current_step = PerfFeatures {
        saturation: 0.0,
        brightness: 0.0,
        edge_density: 0.0,
    };
    let (_t, text) = render_to_text(80, 24, &snap);

    // Labels still render with zeroed features.
    assert!(
        text.contains("hue"),
        "labels render even on zeroed features"
    );
    assert!(
        text.contains("edge"),
        "labels render even on zeroed features"
    );
    // Empty notes -> the silent placeholder (render.rs: "(silent — no notes this step)").
    assert!(
        text.contains("silent"),
        "empty last_notes must show the silent placeholder; got:\n{text}"
    );
    // No note separator should appear when there are no notes.
    assert!(
        !text.contains('@'),
        "no note@velocity glyph when last_notes is empty"
    );
}

/// A3: the transport region DIFFERS between scan_position 0.0 and 1.0 — the gauge
/// reflects position. Compares the two flattened buffers and the percent readings.
#[test]
fn render_transport_reflects_scan_position() {
    let snap0 = fixed_snapshot(0.0, "Dorian", two_notes());
    let snap1 = fixed_snapshot(1.0, "Dorian", two_notes());
    let (_t0, text0) = render_to_text(80, 24, &snap0);
    let (_t1, text1) = render_to_text(80, 24, &snap1);

    assert_ne!(
        text0, text1,
        "transport gauge must differ between scan 0.0 and 1.0"
    );
    // format!("scan {:5.1}%", ratio*100.0): 0.0 -> "0.0%", 1.0 -> "100.0%".
    assert!(text0.contains("0.0%"), "scan 0.0 -> 0.0% reading");
    assert!(text1.contains("100.0%"), "scan 1.0 -> 100.0% reading");
    // 1.0 produces a "100.0%" reading the 0.0 buffer cannot.
    assert!(
        !text0.contains("100.0%"),
        "the 0.0 buffer must not show a 100.0% reading"
    );
}

/// A4: a cramped terminal (20x10, smaller than the layout asks for) must NOT panic —
/// ratatui clips; the draw still returns Ok.
#[test]
fn render_does_not_panic_on_cramped_terminal() {
    let snap = fixed_snapshot(0.5, "Phrygian", two_notes());
    let backend = TestBackend::new(20, 10);
    let mut terminal = Terminal::new(backend).expect("test backend terminal");
    terminal
        .draw(|f| tui::render(f, &snap))
        .expect("render must not panic / error on a cramped layout");
}

/// A4b: a degenerate 1x1 terminal — the most extreme clip — must still not panic.
#[test]
fn render_does_not_panic_on_one_by_one_terminal() {
    let snap = fixed_snapshot(0.5, "Lydian", two_notes());
    let backend = TestBackend::new(1, 1);
    let mut terminal = Terminal::new(backend).expect("test backend terminal");
    terminal
        .draw(|f| tui::render(f, &snap))
        .expect("render must not panic on a 1x1 terminal");
}

// ═════════════════════════════════════════════════════════════════════════════
// GROUP B — SyntheticSource determinism / shape / range (RNG-free source).
// ═════════════════════════════════════════════════════════════════════════════

/// B5: row width equals num_instruments; step_count matches the constructor; all
/// feature scalars sit inside their documented ranges.
#[test]
fn synthetic_source_shape_and_ranges() {
    let src = tui::SyntheticSource::new(40);
    assert_eq!(
        src.step_count(),
        40,
        "step_count matches the constructor arg"
    );

    for &(k, n) in &[(0usize, 4usize), (7, 1), (13, 3), (39, 8)] {
        let row = src.scan_bar_features(k, n);
        assert_eq!(row.len(), n, "row length must equal num_instruments");
        for f in &row {
            assert!(
                (0.0..=360.0).contains(&f.avg_hue),
                "hue in 0..=360, got {}",
                f.avg_hue
            );
            assert!(
                (0.0..=100.0).contains(&f.avg_saturation),
                "sat in 0..=100, got {}",
                f.avg_saturation
            );
            assert!(
                (0.0..=100.0).contains(&f.avg_brightness),
                "bright in 0..=100, got {}",
                f.avg_brightness
            );
            assert!(
                (0.0..=1.0).contains(&f.edge_density),
                "edge in 0..=1, got {}",
                f.edge_density
            );
        }
    }

    // global_features ranges too.
    let g = src.global_features();
    assert!((0.0..=360.0).contains(&g.avg_hue));
    assert!((0.0..=100.0).contains(&g.avg_saturation));
    assert!((0.0..=100.0).contains(&g.avg_brightness));
    assert!((0.0..=1.0).contains(&g.edge_density));
}

/// B5b: the documented DEFAULT_SYNTHETIC_STEPS constant matches Default::default().
#[test]
fn synthetic_source_default_matches_constant() {
    let d = tui::SyntheticSource::default();
    assert_eq!(
        d.step_count(),
        tui::DEFAULT_SYNTHETIC_STEPS,
        "Default must span DEFAULT_SYNTHETIC_STEPS steps"
    );
    // new(0) clamps to >= 1 so position arithmetic never divides by zero.
    assert_eq!(
        tui::SyntheticSource::new(0).step_count(),
        1,
        "new(0) clamps step count up to 1"
    );
}

/// B6: two SyntheticSources with the same `steps` yield identical features — no RNG.
#[test]
fn synthetic_source_is_deterministic() {
    let a = tui::SyntheticSource::new(40);
    let b = tui::SyntheticSource::new(40);
    assert_eq!(
        a.global_features(),
        b.global_features(),
        "global_features must be reproducible (no RNG)"
    );
    for &(k, n) in &[(0usize, 4usize), (13, 3), (25, 2), (39, 4)] {
        assert_eq!(
            a.scan_bar_features(k, n),
            b.scan_bar_features(k, n),
            "scan_bar_features must be reproducible for (k={k}, n={n})"
        );
    }
}

/// B7: the hue sweep actually sweeps — an early step's hue differs from a late one
/// (the source evolves; it is not a constant).
#[test]
fn synthetic_source_hue_sweeps_across_steps() {
    let src = tui::SyntheticSource::new(40);
    let early = src.scan_bar_features(0, 1)[0].avg_hue;
    let late = src.scan_bar_features(30, 1)[0].avg_hue;
    assert!(
        late > early,
        "hue must sweep upward across the scan: early={early} late={late}"
    );
    // Not a constant across the whole row direction either: instrument offset varies hue.
    let row = src.scan_bar_features(10, 3);
    assert!(
        row[0].avg_hue != row[2].avg_hue,
        "per-instrument hue offset must vary across the ensemble"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// GROUP C — DRIVE / OBSERVER SHAPE tests (invariants only, never note VALUES).
// ═════════════════════════════════════════════════════════════════════════════

const MAPPINGS: &str = "assets/mappings.json";

/// C8: driving N ticks advances the step index 0..N, the scan position is monotone
/// non-decreasing and reaches ~1.0, and every tick carries a non-empty mode string.
/// SHAPE/INVARIANTS only — note/mode VALUES are RNG-derived and not asserted.
#[test]
fn drive_advances_scan_to_completion() {
    const N: usize = 8;
    let (mut engine, source) = tui::build_engine(MAPPINGS, N, 4).expect("build engine");
    let mut sink = null_sink();

    let mut last_pos = -1.0_f32;
    for expected_step in 0..N {
        let snap = tui::drive_one_tick(&mut engine, &source, &mut sink).expect("tick");
        // step_index is the count of COMPLETED steps after this tick.
        assert_eq!(
            snap.step_index,
            expected_step + 1,
            "step index increments one per tick"
        );
        assert!(
            snap.scan_position >= last_pos,
            "scan position is monotone non-decreasing (was {last_pos}, now {})",
            snap.scan_position
        );
        last_pos = snap.scan_position;
        assert!(
            !snap.mode.is_empty(),
            "each tick carries a derived mode string"
        );
    }
    assert!(
        (last_pos - 1.0).abs() < 1e-6,
        "after N of N steps the scan is complete (~1.0), got {last_pos}"
    );
}

/// C9: a SnapshotCollector (EngineObserver) records exactly one frame per tick, in
/// order, and the last frame's scan position is ~1.0.
#[test]
fn snapshot_collector_records_one_frame_per_tick() {
    const N: usize = 6;
    let (mut engine, source) = tui::build_engine(MAPPINGS, N, 3).expect("build engine");
    let mut sink = null_sink();
    let mut collector = tui::SnapshotCollector::default();

    for _ in 0..N {
        let snap = tui::drive_one_tick(&mut engine, &source, &mut sink).expect("tick");
        collector.on_tick(&snap);
    }

    assert_eq!(collector.frames.len(), N, "one snapshot recorded per tick");
    // Frames are in tick order: step indices are 1..=N.
    for (i, frame) in collector.frames.iter().enumerate() {
        assert_eq!(frame.step_index, i + 1, "frames recorded in tick order");
    }
    let last = collector.frames.last().expect("N>0 frames");
    assert!(
        (last.scan_position - 1.0).abs() < 1e-6,
        "last recorded frame is scan-complete (~1.0), got {}",
        last.scan_position
    );
}

/// C10: the engine produces exactly one decision per instrument per tick (SHAPE only)
/// — asserted via the tick output's `decisions` length. Never inspects note values.
#[test]
fn tick_produces_one_decision_per_instrument() {
    const INSTRUMENTS: usize = 4;
    // build_engine feeds global features so the plan is derived before tick 0.
    let mappings = audiohax::mapping_loader::load_mappings(MAPPINGS).expect("mappings load");
    let cfg = audiohax::engine::EngineConfig {
        num_instruments: INSTRUMENTS,
        ..audiohax::engine::EngineConfig::default()
    };
    let mut engine = audiohax::engine::PipelineEngine::new(mappings, cfg);
    let source = tui::SyntheticSource::new(5);
    engine.set_features_global(&source.global_features());

    let mut sink = null_sink();
    let out = engine.tick(&source, &mut sink).expect("tick");
    assert_eq!(
        out.decisions.len(),
        INSTRUMENTS,
        "one decision per instrument"
    );
    // Channels are inst_idx % 16 — a shape invariant, not a musical value.
    for (i, dec) in out.decisions.iter().enumerate() {
        assert_eq!(dec.channel, (i % 16) as u8, "channel is inst_idx % 16");
    }
}

/// C10b: note_on/note_off pairing — a counting sink sees equal on/off counts after a
/// drive. SHAPE invariant (pairing), never a note-value assertion.
#[test]
fn drive_pairs_note_on_with_note_off() {
    #[derive(Default)]
    struct CountingSink {
        ons: usize,
        offs: usize,
    }
    impl AudioSink for CountingSink {
        fn note_on(
            &mut self,
            _c: u8,
            _n: u8,
            _v: u8,
        ) -> Result<(), audiohax::engine::AudioSinkError> {
            self.ons += 1;
            Ok(())
        }
        fn note_off(&mut self, _c: u8, _n: u8) -> Result<(), audiohax::engine::AudioSinkError> {
            self.offs += 1;
            Ok(())
        }
        fn program_change(
            &mut self,
            _c: u8,
            _p: u8,
        ) -> Result<(), audiohax::engine::AudioSinkError> {
            Ok(())
        }
    }

    let (mut engine, source) = tui::build_engine(MAPPINGS, 5, 3).expect("build engine");
    let mut sink = CountingSink::default();
    for _ in 0..5 {
        engine.tick(&source, &mut sink).expect("tick");
    }
    assert_eq!(
        sink.ons, sink.offs,
        "every note_on must be paired with a note_off"
    );
}

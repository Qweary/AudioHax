//! src/tui.rs — WS-4 Phase 3 (S10) terminal front-end logic, a PURE OBSERVER over
//! the S9 `engine.rs` shared-core seam (assessment §4.3 / §6 Phase 3).
//!
//! This module is the seam-proof: it demonstrates that the pure-Rust
//! [`PipelineEngine`](crate::engine::PipelineEngine) can drive a real front-end with
//! ZERO native dependencies (no OpenCV / image / midir / ALSA). It contains:
//!
//!   * [`render`] — a PURE function of an [`EngineSnapshot`](crate::engine::EngineSnapshot)
//!     onto a `ratatui::Frame`. It never mutates the snapshot and never calls into the
//!     engine, so it is unit-testable on a `TestBackend` by constructing a fixed
//!     snapshot and asserting on the rendered buffer.
//!   * [`SyntheticSource`] — a procedural, fully DETERMINISTIC
//!     [`FeatureSource`](crate::engine::FeatureSource) (no RNG) that lets the TUI run
//!     the whole engine→snapshot→render path headlessly.
//!   * [`NullSink`] — a no-op [`AudioSink`](crate::engine::AudioSink) (the TUI observes,
//!     it does not produce audio).
//!   * [`SnapshotCollector`] — an [`EngineObserver`](crate::engine::EngineObserver) that
//!     records one snapshot per tick.
//!   * [`build_engine`] / [`drive_one_tick`] — the wiring helpers the bin and the Test
//!     Engineer share so neither duplicates the engine setup.
//!
//! CORRECTNESS NOTE (S9 / S10): `PipelineEngine::set_features_global` derives the
//! harmony plan via `chord_engine::pick_progression`, which uses `thread_rng`. The
//! NOTES/MODE/PLAN of a live run are therefore NON-deterministic across runs. The
//! synthetic FEATURES are deterministic, but the engine's plan on top of them is not.
//! Hence [`render`] is a pure function of a HANDED snapshot (so tests can pin a fixed
//! snapshot and assert render structure + the deterministic feature meters), and the
//! drive helpers assert scan-advance / observer SHAPE only — never exact note values.

use crate::engine::{
    AudioSink, AudioSinkError, EngineConfig, EngineObserver, EngineSnapshot, FeatureSource,
    GlobalFeatures, PipelineEngine, ScanBarFeatures,
};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, Paragraph};
use ratatui::Frame;

// ─────────────────────────────────────────────────────────────────────────────
// RENDER — the pure observer (assessment §4.3). PURE function of a snapshot.
// ─────────────────────────────────────────────────────────────────────────────

/// Render a single TUI frame from an [`EngineSnapshot`](crate::engine::EngineSnapshot).
///
/// PURE and infallible: it reads the snapshot, draws widgets, and returns. It does NOT
/// mutate the snapshot and does NOT call into the engine — which is exactly what makes
/// it unit-testable on a `ratatui::backend::TestBackend` by handing it a fixed
/// snapshot and asserting on the rendered buffer.
///
/// Layout (top-to-bottom vertical split):
///   1. WHOLE-IMAGE FEATURE METERS — avg_hue (0..360), avg_saturation (0..100),
///      avg_brightness (0..100), edge_density (0..1), each a labeled [`Gauge`].
///   2. TRANSPORT — scan position gauge (0..=1) + the step index.
///   3. MUSIC STATE — derived mode, phrase position, and the music-domain projection
///      of the last step's features (`current_step`: sat / bright / edge).
///   4. LAST NOTES — a compact line of the last tick's note numbers + velocities.
///
/// The gauges normalize each feature into the 0..=100 percent a [`Gauge`] expects;
/// the human-readable raw value is shown in the gauge label so no precision is lost.
pub fn render(frame: &mut Frame, snapshot: &EngineSnapshot) {
    let area = frame.area();

    // Outer vertical split: meters / transport / music-state / notes.
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6), // feature meters (4 gauges + border)
            Constraint::Length(3), // transport (scan gauge + step index)
            Constraint::Length(4), // music state (mode / phrase / projection)
            Constraint::Min(3),    // last notes (fills the rest)
        ])
        .split(area);

    render_feature_meters(frame, chunks[0], &snapshot.global);
    render_transport(frame, chunks[1], snapshot);
    render_music_state(frame, chunks[2], snapshot);
    render_last_notes(frame, chunks[3], snapshot);
}

/// Render the four whole-image feature meters as labeled gauges.
///
/// Each meter normalizes its feature into 0..=100 (the percent a [`Gauge`] wants) but
/// labels the gauge with the RAW value + units so nothing is lost: hue over 0..360,
/// saturation / brightness over 0..100, edge_density over 0..1.
fn render_feature_meters(frame: &mut Frame, area: Rect, g: &GlobalFeatures) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" feature meters (whole image) ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // One row per meter inside the bordered block.
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(inner);

    // hue 0..360 → percent; the others are already 0..100 or 0..1.
    let hue_pct = normalize(g.avg_hue, 0.0, 360.0);
    let sat_pct = normalize(g.avg_saturation, 0.0, 100.0);
    let bright_pct = normalize(g.avg_brightness, 0.0, 100.0);
    let edge_pct = normalize(g.edge_density, 0.0, 1.0);

    frame.render_widget(
        meter_gauge("hue", g.avg_hue, "/360", hue_pct, Color::Magenta),
        rows[0],
    );
    frame.render_widget(
        meter_gauge("sat", g.avg_saturation, "/100", sat_pct, Color::Yellow),
        rows[1],
    );
    frame.render_widget(
        meter_gauge("bright", g.avg_brightness, "/100", bright_pct, Color::White),
        rows[2],
    );
    frame.render_widget(
        meter_gauge("edge", g.edge_density, "/1.0", edge_pct, Color::Cyan),
        rows[3],
    );
}

/// Build one labeled feature [`Gauge`]: `label` names it, `raw`+`suffix` show the
/// real value, `ratio` (0..=1) fills the bar. Pure widget construction.
fn meter_gauge<'a>(
    label: &'a str,
    raw: f32,
    suffix: &'a str,
    ratio: f64,
    color: Color,
) -> Gauge<'a> {
    Gauge::default()
        .gauge_style(Style::default().fg(color))
        .ratio(ratio)
        .label(format!("{label:<7}{raw:6.1}{suffix}"))
}

/// Render the transport row: the scan-position progress bar + the step index.
fn render_transport(frame: &mut Frame, area: Rect, snapshot: &EngineSnapshot) {
    // scan_position is already 0.0..=1.0; clamp defensively before feeding the gauge.
    let ratio = (snapshot.scan_position as f64).clamp(0.0, 1.0);
    let gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" transport — step {} ", snapshot.step_index)),
        )
        .gauge_style(Style::default().fg(Color::Green))
        .ratio(ratio)
        .label(format!("scan {:5.1}%", ratio * 100.0));
    frame.render_widget(gauge, area);
}

/// Render the music-state panel: derived mode, phrase position, and the music-domain
/// projection of the last step's features (`current_step`).
fn render_music_state(frame: &mut Frame, area: Rect, snapshot: &EngineSnapshot) {
    let cs = &snapshot.current_step;
    let lines = vec![
        Line::from(vec![
            Span::styled("mode: ", Style::default().fg(Color::Gray)),
            Span::styled(
                snapshot.mode.clone(),
                Style::default().fg(Color::LightGreen),
            ),
            Span::raw("    "),
            Span::styled("phrase: ", Style::default().fg(Color::Gray)),
            // PhrasePosition has no Display; its Debug is the short, stable label.
            Span::styled(
                format!("{:?}", snapshot.phrase),
                Style::default().fg(Color::LightBlue),
            ),
        ]),
        Line::from(format!(
            "current step — sat {:.1}  bright {:.1}  edge {:.2}",
            cs.saturation, cs.brightness, cs.edge_density
        )),
    ];
    let para = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" music state "),
    );
    frame.render_widget(para, area);
}

/// Render the last tick's notes as a compact `note@velocity` list.
fn render_last_notes(frame: &mut Frame, area: Rect, snapshot: &EngineSnapshot) {
    let body = if snapshot.last_notes.is_empty() {
        "(silent — no notes this step)".to_string()
    } else {
        snapshot
            .last_notes
            .iter()
            .map(|n| format!("{}@{}", n.note, n.velocity))
            .collect::<Vec<_>>()
            .join("  ")
    };
    let para = Paragraph::new(body).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" last notes ({}) ", snapshot.last_notes.len())),
    );
    frame.render_widget(para, area);
}

/// Normalize `v` from `[lo, hi]` into the `[0.0, 1.0]` ratio a [`Gauge`] expects,
/// clamped. A degenerate range (`hi <= lo`) yields 0.0.
fn normalize(v: f32, lo: f32, hi: f32) -> f64 {
    if hi <= lo {
        return 0.0;
    }
    (((v - lo) / (hi - lo)) as f64).clamp(0.0, 1.0)
}

// ─────────────────────────────────────────────────────────────────────────────
// SYNTHETIC FEATURE SOURCE — the seam-proof input (no native deps, no RNG).
// ─────────────────────────────────────────────────────────────────────────────

/// A procedural, fully DETERMINISTIC [`FeatureSource`](crate::engine::FeatureSource)
/// that needs no OpenCV / image and no RNG — every value is a pure function of the
/// step index. This is what lets `audiohax tui` run the full engine→snapshot→render
/// path headlessly (the seam-proof).
///
/// The generated "image" evolves like a gradient sweep:
///   * `avg_hue` sweeps 0 → 360 across `step_count` steps;
///   * `avg_saturation` / `avg_brightness` oscillate (triangle wave of the step index);
///   * `edge_density` oscillates within 0..1;
///   * per-instrument scan rows vary mildly by instrument index so the ensemble's
///     instruments see slightly different inputs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SyntheticSource {
    /// Total scan steps the synthetic "image" spans.
    steps: usize,
}

/// Default synthetic scan length when none is given (matches the CLI `--steps` default).
pub const DEFAULT_SYNTHETIC_STEPS: usize = 40;

impl Default for SyntheticSource {
    fn default() -> Self {
        SyntheticSource {
            steps: DEFAULT_SYNTHETIC_STEPS,
        }
    }
}

impl SyntheticSource {
    /// Construct a synthetic source spanning `steps` scan steps (clamped to ≥ 1 so the
    /// engine's position arithmetic never divides by zero).
    pub fn new(steps: usize) -> Self {
        SyntheticSource {
            steps: steps.max(1),
        }
    }

    /// A 0.0..=1.0 triangle wave of `step_idx` with the given whole-cycle `period`
    /// (in steps). Deterministic; rises 0→1 over the first half-period and falls back.
    fn triangle(step_idx: usize, period: usize) -> f32 {
        let period = period.max(1);
        let phase = (step_idx % period) as f32 / period as f32; // 0..1
        if phase < 0.5 {
            phase * 2.0
        } else {
            (1.0 - phase) * 2.0
        }
    }
}

impl FeatureSource for SyntheticSource {
    fn global_features(&self) -> GlobalFeatures {
        // The "current" global features are taken at the scan midpoint so they read as
        // a representative whole-image summary (deterministic, no RNG).
        let mid = self.steps / 2;
        let hue = (mid as f32 / self.steps.max(1) as f32) * 360.0;
        GlobalFeatures {
            avg_hue: hue,
            avg_saturation: 30.0 + Self::triangle(mid, 8) * 60.0, // 30..90
            avg_brightness: 25.0 + Self::triangle(mid, 6) * 60.0, // 25..85
            edge_density: 0.15 + Self::triangle(mid, 5) * 0.7,    // 0.15..0.85
            hue_spread: 0.2,
            texture_laplacian_var: 1.0,
            shape_complexity: 0.1,
            aspect_ratio: 1.5,
        }
    }

    fn scan_bar_features(&self, step_idx: usize, num_instruments: usize) -> Vec<ScanBarFeatures> {
        let hue = (step_idx as f32 / self.steps.max(1) as f32 * 360.0).clamp(0.0, 360.0);
        (0..num_instruments)
            .map(|inst| {
                // Each instrument is offset by its index so the ensemble sees slightly
                // different inputs — a mild deterministic spread, never RNG.
                let off = inst as f32 * 4.0;
                ScanBarFeatures {
                    bar_index: inst,
                    avg_hue: (hue + off).clamp(0.0, 360.0),
                    avg_saturation: (30.0 + Self::triangle(step_idx + inst, 8) * 60.0)
                        .clamp(0.0, 100.0),
                    avg_brightness: (25.0 + Self::triangle(step_idx + inst, 6) * 60.0)
                        .clamp(0.0, 100.0),
                    edge_density: (0.15 + Self::triangle(step_idx + inst, 5) * 0.7).clamp(0.0, 1.0),
                    texture_laplacian_var: 1.0,
                    hue_hist: Vec::new(),
                }
            })
            .collect()
    }

    fn step_count(&self) -> usize {
        self.steps
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// NULL SINK — the TUI observes; it produces no audio.
// ─────────────────────────────────────────────────────────────────────────────

/// A no-op [`AudioSink`](crate::engine::AudioSink): every method succeeds and does
/// nothing. The TUI is a pure observer of `current_state()` — it never sounds notes —
/// so the engine's sink sends are simply discarded. (The engine already tracks
/// `last_notes` in its snapshot, so no counting is needed here.)
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct NullSink;

impl AudioSink for NullSink {
    fn note_on(&mut self, _channel: u8, _note: u8, _velocity: u8) -> Result<(), AudioSinkError> {
        Ok(())
    }
    fn note_off(&mut self, _channel: u8, _note: u8) -> Result<(), AudioSinkError> {
        Ok(())
    }
    fn program_change(&mut self, _channel: u8, _program: u8) -> Result<(), AudioSinkError> {
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// OBSERVER — records one snapshot per tick (assessment §4.1 EngineObserver).
// ─────────────────────────────────────────────────────────────────────────────

/// An [`EngineObserver`](crate::engine::EngineObserver) that records the snapshot from
/// every tick. Satisfies the "front-ends are pure observers consuming `EngineSnapshot`"
/// scope item and gives the Test Engineer a record to assert scan-advance SHAPE over.
#[derive(Debug, Default, Clone)]
pub struct SnapshotCollector {
    /// One [`EngineSnapshot`](crate::engine::EngineSnapshot) per `on_tick` call, in order.
    pub frames: Vec<EngineSnapshot>,
}

impl EngineObserver for SnapshotCollector {
    fn on_tick(&mut self, snapshot: &EngineSnapshot) {
        self.frames.push(snapshot.clone());
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// DRIVE HELPERS — shared engine wiring (bin + tests both use these).
// ─────────────────────────────────────────────────────────────────────────────

/// Build a [`PipelineEngine`](crate::engine::PipelineEngine) and its
/// [`SyntheticSource`] for the TUI, with the whole-image features already fed so the
/// harmony plan is derived and `current_state()` is meaningful on tick 0.
///
/// `mappings_path` is the path to `assets/mappings.json` (the engine needs a mapping
/// table). Returns the wired engine + source on success; surfaces the loader error
/// (e.g. a missing mappings file) to the caller rather than panicking.
///
/// NOTE: `set_features_global` derives the plan via `thread_rng`, so the engine's
/// mode/plan are non-deterministic across calls — by design (S9). The SOURCE is fully
/// deterministic; the per-run plan on top of it is not.
pub fn build_engine(
    mappings_path: &str,
    steps: usize,
    instruments: usize,
) -> Result<(PipelineEngine, SyntheticSource), String> {
    let mappings = crate::mapping_loader::load_mappings(mappings_path)
        .map_err(|e| format!("loading mappings from {mappings_path}: {e}"))?;
    let config = EngineConfig {
        num_instruments: instruments.max(1),
        ..EngineConfig::default()
    };
    let mut engine = PipelineEngine::new(mappings, config);
    let source = SyntheticSource::new(steps);
    // Feed whole-image features once so the plan is derived before the first tick.
    engine.set_features_global(&source.global_features());
    Ok((engine, source))
}

/// Advance the engine one step against `source`, send to `sink`, then return the
/// engine's current snapshot. The single place tick→snapshot wiring lives so the bin
/// and the tests share it. Surfaces the sink error rather than panicking.
pub fn drive_one_tick<S: FeatureSource, A: AudioSink>(
    engine: &mut PipelineEngine,
    source: &S,
    sink: &mut A,
) -> Result<EngineSnapshot, AudioSinkError> {
    engine.tick(source, sink)?;
    Ok(engine.current_state())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chord_engine::{NoteEvent, PerfFeatures, PhrasePosition};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    /// A fixed, deterministic snapshot for render tests — no engine, no RNG.
    fn fixed_snapshot() -> EngineSnapshot {
        EngineSnapshot {
            scan_position: 0.5,
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
            last_notes: vec![
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
            ],
            mode: "Dorian".to_string(),
            phrase: PhrasePosition::PhraseStart,
        }
    }

    #[test]
    fn render_constructed_snapshot_does_not_panic() {
        let backend = TestBackend::new(60, 24);
        let mut terminal = Terminal::new(backend).expect("test backend terminal");
        let snap = fixed_snapshot();
        terminal
            .draw(|f| render(f, &snap))
            .expect("render must not error on a fixed snapshot");
    }

    #[test]
    fn render_writes_labels_and_note_values_into_buffer() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test backend terminal");
        let snap = fixed_snapshot();
        terminal.draw(|f| render(f, &snap)).expect("draw");
        // Flatten the rendered buffer into a string and assert deterministic content.
        let buf = terminal.backend().buffer().clone();
        let text: String = buf.content().iter().map(|c| c.symbol()).collect();
        assert!(text.contains("Dorian"), "mode must be rendered");
        assert!(text.contains("hue"), "hue meter label must render");
        assert!(text.contains("edge"), "edge meter label must render");
        assert!(text.contains('@'), "notes render as note@velocity");
        assert!(text.contains("step 7"), "step index must render");
    }

    #[test]
    fn render_handles_empty_notes() {
        let backend = TestBackend::new(60, 20);
        let mut terminal = Terminal::new(backend).expect("test backend terminal");
        let mut snap = fixed_snapshot();
        snap.last_notes.clear();
        terminal
            .draw(|f| render(f, &snap))
            .expect("empty-notes render must not panic");
        let buf = terminal.backend().buffer().clone();
        let text: String = buf.content().iter().map(|c| c.symbol()).collect();
        assert!(text.contains("silent"), "empty notes show the silent label");
    }

    #[test]
    fn synthetic_source_yields_num_instruments_rows() {
        let src = SyntheticSource::new(40);
        let row = src.scan_bar_features(0, 4);
        assert_eq!(row.len(), 4, "row length must equal num_instruments");
        let row2 = src.scan_bar_features(10, 2);
        assert_eq!(row2.len(), 2);
    }

    #[test]
    fn synthetic_source_hue_advances_across_steps() {
        let src = SyntheticSource::new(40);
        let early = src.scan_bar_features(0, 1)[0].avg_hue;
        let late = src.scan_bar_features(30, 1)[0].avg_hue;
        assert!(late > early, "hue must sweep upward across the scan");
        assert!(src.step_count() == 40);
    }

    #[test]
    fn synthetic_source_is_deterministic() {
        let a = SyntheticSource::new(40);
        let b = SyntheticSource::new(40);
        assert_eq!(
            a.scan_bar_features(13, 3),
            b.scan_bar_features(13, 3),
            "synthetic features must be reproducible (no RNG)"
        );
        assert_eq!(a.global_features(), b.global_features());
    }

    #[test]
    fn synthetic_features_are_in_range() {
        let src = SyntheticSource::new(40);
        for step in 0..40 {
            for f in src.scan_bar_features(step, 4) {
                assert!((0.0..=360.0).contains(&f.avg_hue));
                assert!((0.0..=100.0).contains(&f.avg_saturation));
                assert!((0.0..=100.0).contains(&f.avg_brightness));
                assert!((0.0..=1.0).contains(&f.edge_density));
            }
        }
    }

    #[test]
    fn null_sink_is_noop_ok() {
        let mut sink = NullSink;
        assert!(sink.note_on(0, 60, 90).is_ok());
        assert!(sink.note_off(0, 60).is_ok());
        assert!(sink.program_change(0, 1).is_ok());
    }

    #[test]
    fn build_and_drive_advances_scan_shape() {
        // Drive a few ticks and assert SHAPE only (scan advance / one snapshot per
        // tick) — never exact note values, which are RNG-derived (S9).
        let (mut engine, source) =
            build_engine("assets/mappings.json", 5, 3).expect("build engine");
        let mut sink = NullSink;
        let mut collector = SnapshotCollector::default();
        let mut last_pos = -1.0_f32;
        for _ in 0..5 {
            let snap = drive_one_tick(&mut engine, &source, &mut sink).expect("tick");
            assert!(
                snap.scan_position >= last_pos,
                "scan position is monotonic non-decreasing"
            );
            last_pos = snap.scan_position;
            collector.on_tick(&snap);
        }
        assert_eq!(collector.frames.len(), 5, "one snapshot per tick");
        assert!(
            (collector.frames.last().unwrap().scan_position - 1.0).abs() < 1e-6,
            "5 of 5 steps → scan complete"
        );
    }
}

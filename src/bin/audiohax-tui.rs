// src/bin/audiohax-tui.rs
//
// WS-4 Phase 3 (S10): the headless `ratatui` TUI front-end — the seam-proof. It drives
// the pure-Rust `audiohax::engine::PipelineEngine` over a synthetic (no-OpenCV) feature
// source and renders each tick's `EngineSnapshot` to the terminal. It links NO native
// deps (no opencv/image/midir/ALSA), so it builds & runs under
// `cargo build --bin audiohax-tui --no-default-features`. It is left to bin
// autodiscovery (no `required-features` table), exactly like the modem bins.
//
// THIN BY CONTRACT: all render / feature-generation / engine-wiring logic lives in the
// `audiohax::tui` library module (so it is unit-testable on a `TestBackend`). This bin
// only owns the crossterm terminal setup/teardown and the event loop.

use std::io::{self, Stdout};
use std::time::Duration;

use anyhow::{Context, Result};
use audiohax::tui::{self, NullSink};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

/// Parse the standalone `audiohax-tui` args via the shared clap grammar.
#[derive(Debug, clap::Parser)]
#[command(
    name = "audiohax-tui",
    version,
    about = "Headless ratatui TUI over the AudioHax engine seam (synthetic feature source)"
)]
struct Cli {
    #[command(flatten)]
    args: audiohax::cli::TuiArgs,
}

fn main() -> Result<()> {
    use clap::Parser;
    let cli = Cli::parse();

    // ── Build the engine + synthetic source (all logic lives in the lib) ──
    let (mut engine, source) =
        tui::build_engine("assets/mappings.json", cli.args.steps, cli.args.instruments)
            .map_err(|e| anyhow::anyhow!(e))
            .context("building the TUI engine")?;
    let ms_per_step = engine.config().ms_per_step;
    let total_steps = source_step_count(&source);

    // ── Terminal setup (alternate screen + raw mode) ──
    let mut terminal = setup_terminal().context("setting up the terminal")?;

    // ── Event loop. Any error is captured so we ALWAYS restore the terminal. ──
    let loop_result = run_loop(
        &mut terminal,
        &mut engine,
        &source,
        ms_per_step,
        total_steps,
    );

    // ── Teardown (runs on success AND error) ──
    restore_terminal(&mut terminal).context("restoring the terminal")?;

    loop_result
}

/// The step count of the synthetic source (free fn so the bin needn't import the trait).
fn source_step_count(source: &tui::SyntheticSource) -> usize {
    use audiohax::engine::FeatureSource;
    source.step_count()
}

/// The render + event loop: tick the engine, draw the snapshot, poll a key with a
/// per-step timeout, quit on q/Esc/Ctrl-C, and loop the scan when it completes.
fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    engine: &mut audiohax::engine::PipelineEngine,
    source: &tui::SyntheticSource,
    ms_per_step: u64,
    total_steps: usize,
) -> Result<()> {
    let mut sink = NullSink;
    let poll_timeout = Duration::from_millis(ms_per_step.max(1));

    loop {
        // Advance one step and render the resulting snapshot (lib owns both).
        let snap = tui::drive_one_tick(engine, source, &mut sink)
            .map_err(|e| anyhow::anyhow!(e.to_string()))
            .context("driving an engine tick")?;
        terminal.draw(|f| tui::render(f, &snap))?;

        // Loop the scan: when the scan completes, reset the transport to the start so
        // the dashboard keeps animating (a demo never "ends" until the user quits).
        if total_steps > 0 && snap.step_index >= total_steps {
            engine.command(audiohax::engine::EngineCommand::Stop); // reset position to 0
            engine.command(audiohax::engine::EngineCommand::Play); // resume transport
        }

        // Poll for a quit key for ~one step's worth of time (also paces the animation).
        if event::poll(poll_timeout)? {
            if let Event::Key(key) = event::read()? {
                // Only act on key PRESS (Windows also emits Release/Repeat events).
                if key.kind == KeyEventKind::Press {
                    let ctrl_c = key.modifiers.contains(KeyModifiers::CONTROL)
                        && key.code == KeyCode::Char('c');
                    if ctrl_c || matches!(key.code, KeyCode::Char('q') | KeyCode::Esc) {
                        return Ok(());
                    }
                }
            }
        }
    }
}

/// Enter the alternate screen + raw mode and build the ratatui terminal.
fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

/// Leave the alternate screen + disable raw mode + show the cursor. Best-effort on
/// every exit path so a crash never leaves the user's terminal in raw mode.
fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

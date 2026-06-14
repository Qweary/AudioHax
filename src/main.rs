// src/main.rs — the OpenCV/audio ADAPTER (S9 §3.2).
//
// After the WS-4 Phase 1 engine extraction this binary is a THIN adapter. It owns
// everything that touches `Mat`, highgui, MIDI ports, and wall-clock scheduling; the
// musical decisions live in the headless `audiohax::engine` core. Concretely the
// adapter:
//   * parses argv via the shared `audiohax::cli` grammar (→ EngineConfig + image path
//     + play/no-play),
//   * acquires the image via OpenCV (`load_image_from_source` → `Mat`),
//     extracts `analyze_global` / `scan_image` feature structs, writes overlays, and
//     drives the highgui window,
//   * exposes those features to the engine through a `PrecomputedSource: FeatureSource`
//     (copying `image_analysis::*` → `engine::*` by field — the boundary; no `Mat`
//     ever crosses),
//   * implements `engine::AudioSink for MidiOut` HERE (orphan rule — the lib cannot
//     name the bin-private `MidiOut`),
//   * builds a `PipelineEngine`, feeds global features, then for each step calls
//     `engine.decide_step(&source, k)` and applies the jitter + `Instant`-based
//     scheduling (the adapter owns timing/RNG — D8) and the per-step overlay.
//
// GONE from this file (moved into the engine / dissolved): `worker_decide_action`,
// `play_scanned_steps_concurrent`, the `Barrier`/worker pool, `InstrumentAction`, the
// mode/progression/plan derivation. The jitter + `ScheduledEvent` time-ordering +
// `thread::sleep` execution STAY here (wall-clock playback is an adapter concern).
//
// WS-4 Phase 2 (S11) Lane C: the DEFAULT build is now PURE RUST (`pure-analysis` +
// `synth`) and DOES compile on a headless/clean box. The OpenCV acquisition/analysis
// + highgui window + overlay PNGs are `#[cfg(feature="opencv")]`-gated; the external
// MIDI-out sink is `#[cfg(feature="midi-out")]`-gated. With no flags, main.rs loads
// the image via `pure_analysis::load_pure_image`, drives the engine with
// `PureAnalysisSource`, and plays via the in-process `SynthSink`. (design §3.C/§5.2)

// OpenCV-only adapter modules — pulled in only under the `opencv` feature.
#[cfg(feature = "opencv")]
mod image_analysis;
#[cfg(feature = "opencv")]
mod image_source;
// External MIDI-out adapter module. WS-4 S12: `midi-out` is now in the DEFAULT
// feature set, so this module + the `MidiOut` sink are always compiled and the sink
// is chosen at RUNTIME (see the `--output`/`--midi-virtual` branch below), not by cfg.
mod midi_output;

use audiohax::cli::{pipeline_to_engine_config, Cli, Command, OutputSink, PlayArgs};
use audiohax::engine::{AudioSink, FeatureSource, PipelineEngine};
use audiohax::mapping_loader::load_mappings;
use clap::Parser;

// ── OpenCV path imports (gated) ──────────────────────────────────────────────
#[cfg(feature = "opencv")]
use audiohax::engine::{GlobalFeatures as EngGlobal, ScanBarFeatures as EngScanBar};
#[cfg(feature = "opencv")]
use image_analysis::{analyze_global, draw_scan_bar_overlay_for_rect, scan_image};
#[cfg(feature = "opencv")]
use image_source::{load_image_from_source, ImageSource};
#[cfg(feature = "opencv")]
use opencv::core;
#[cfg(feature = "opencv")]
use opencv::prelude::MatTraitConst; // needed for .cols()/.rows()

// ── External MIDI-out sink import ────────────────────────────────────────────
// WS-4 S12: `MidiOut` (and its `AudioSinkError` use) are always compiled now — the
// sink is selected at runtime. `AudioSinkError` is imported once here regardless of
// the `opencv` flag (the `impl AudioSink for MidiOut` below always needs it).
use audiohax::engine::AudioSinkError;
use midi_output::MidiOut;

use rand::Rng;
use std::time::{Duration, Instant};

/// Scheduled MIDI event (time-based) — adapter-owned wall-clock playback unit.
#[derive(Clone, Debug)]
struct ScheduledEvent {
    at: Instant,
    on: bool, // true = note_on, false = note_off
    channel: u8,
    note: u8,
    vel: u8, // used only for note_on (note_off vel ignored)
}

/// `engine::AudioSink for MidiOut` lives HERE (S9 §3.3 / D3) — the lib cannot name the
/// bin-private `MidiOut`, and even a re-export would be an orphan violation. Each
/// method maps `MidiOut`'s `Box<dyn Error>` (NOT `Send + Sync`) into an
/// `AudioSinkError` by stringifying it — keeping `midi_output.rs` untouched.
/// WS-4 S12: unconditional — `midi-out` is in `default`, so `MidiOut` is always
/// compiled and selected at runtime; the in-process `SynthSink` remains the default.
impl AudioSink for MidiOut {
    fn note_on(&mut self, channel: u8, note: u8, velocity: u8) -> Result<(), AudioSinkError> {
        MidiOut::note_on(self, channel, note, velocity)
            .map_err(|e| AudioSinkError::msg(e.to_string()))
    }
    fn note_off(&mut self, channel: u8, note: u8) -> Result<(), AudioSinkError> {
        MidiOut::note_off(self, channel, note).map_err(|e| AudioSinkError::msg(e.to_string()))
    }
    fn program_change(&mut self, channel: u8, program: u8) -> Result<(), AudioSinkError> {
        MidiOut::program_change(self, channel, program)
            .map_err(|e| AudioSinkError::msg(e.to_string()))
    }
}

/// The adapter's `FeatureSource`: wraps the OpenCV-extracted whole-image features plus
/// the precomputed per-step `Vec<Vec<image_analysis::ScanBarFeatures>>`, converting
/// `image_analysis::*` → `engine::*` by field copy at the boundary (S9 §3.2 / D1). No
/// `Mat` is held; the engine never sees pixels. WS-4 Phase 2 (S11): OpenCV-path only —
/// the default path uses `pure_analysis::PureAnalysisSource` instead.
#[cfg(feature = "opencv")]
struct PrecomputedSource {
    global: EngGlobal,
    steps: Vec<Vec<EngScanBar>>,
}

#[cfg(feature = "opencv")]
impl PrecomputedSource {
    /// Build from the OpenCV feature structs, performing the boundary field copy ONCE.
    fn new(
        global: &image_analysis::GlobalFeatures,
        steps: &[Vec<image_analysis::ScanBarFeatures>],
    ) -> Self {
        PrecomputedSource {
            global: to_eng_global(global),
            steps: steps
                .iter()
                .map(|row| row.iter().map(to_eng_scanbar).collect())
                .collect(),
        }
    }
}

#[cfg(feature = "opencv")]
impl FeatureSource for PrecomputedSource {
    fn global_features(&self) -> EngGlobal {
        self.global
    }
    fn scan_bar_features(&self, step_idx: usize, num_instruments: usize) -> Vec<EngScanBar> {
        let mut row = self.steps.get(step_idx).cloned().unwrap_or_default();
        // Defensive: keep the row exactly `num_instruments` wide (it already is in the
        // batch path; a live source might not be).
        row.truncate(num_instruments);
        row
    }
    fn step_count(&self) -> usize {
        self.steps.len()
    }
}

/// Boundary copy `image_analysis::GlobalFeatures` → `engine::GlobalFeatures` (S9 §3.2).
#[cfg(feature = "opencv")]
fn to_eng_global(g: &image_analysis::GlobalFeatures) -> EngGlobal {
    EngGlobal {
        avg_hue: g.avg_hue,
        avg_saturation: g.avg_saturation,
        avg_brightness: g.avg_brightness,
        edge_density: g.edge_density,
        hue_spread: g.hue_spread,
        texture_laplacian_var: g.texture_laplacian_var,
        shape_complexity: g.shape_complexity,
        aspect_ratio: g.aspect_ratio,
    }
}

/// Boundary copy `image_analysis::ScanBarFeatures` → `engine::ScanBarFeatures`.
#[cfg(feature = "opencv")]
fn to_eng_scanbar(s: &image_analysis::ScanBarFeatures) -> EngScanBar {
    EngScanBar {
        bar_index: s.bar_index,
        avg_hue: s.avg_hue,
        avg_saturation: s.avg_saturation,
        avg_brightness: s.avg_brightness,
        edge_density: s.edge_density,
        texture_laplacian_var: s.texture_laplacian_var,
        hue_hist: s.hue_hist.clone(),
    }
}

/// Resolve the image path argument into an `ImageSource` (the example image when no
/// path is given) — replaces the old overloaded-`"play"` positional logic.
/// WS-4 Phase 2 (S11): OpenCV-path only (returns the OpenCV `ImageSource`); the pure
/// path resolves to `pure_analysis::PureImageSource` inline below.
#[cfg(feature = "opencv")]
fn resolve_source(image: &Option<std::path::PathBuf>) -> ImageSource {
    match image {
        Some(p) => ImageSource::UserPath(p.to_string_lossy().to_string()),
        None => ImageSource::UserPath("assets/images/example.jpg".to_string()),
    }
}

/// Compute the scan-bar rect for step `si` of `total` (overlay geometry — adapter-only).
/// WS-4 Phase 2 (S11): OpenCV-path only (uses `opencv::core::Rect` for highgui overlays).
#[cfg(feature = "opencv")]
fn step_rect(
    si: usize,
    total: usize,
    width: i32,
    height: i32,
    bar_thickness_frac: f32,
) -> (core::Rect, bool, i32, i32) {
    let vertical_default = width > height;
    let bar_w = if vertical_default {
        ((width as f32) * bar_thickness_frac).max(1.0).round() as i32
    } else {
        width
    };
    let bar_h = if !vertical_default {
        ((height as f32) * bar_thickness_frac).max(1.0).round() as i32
    } else {
        height
    };
    let travel_x = (width - bar_w).max(0);
    let travel_y = (height - bar_h).max(0);
    let x0 = if vertical_default {
        if total <= 1 {
            0
        } else {
            ((si as f32) * (travel_x as f32) / ((total - 1) as f32)).round() as i32
        }
    } else {
        0
    };
    let y0 = if !vertical_default {
        if total <= 1 {
            0
        } else {
            ((si as f32) * (travel_y as f32) / ((total - 1) as f32)).round() as i32
        }
    } else {
        0
    };
    let rect = core::Rect::new(
        x0,
        y0,
        if vertical_default { bar_w } else { width },
        if vertical_default { height } else { bar_h },
    );
    (rect, vertical_default, bar_w, bar_h)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ── CLI (shared clap grammar) ───────────────────────────────────────────────
    let cli = Cli::parse();

    // Phase 1 wires the `play` subcommand (the others — render/analyze/modem — are
    // surfaced by the grammar but their adapter bodies are follow-on work; the modem
    // subcommand mirrors the dedicated bins). Map non-play to a friendly message
    // rather than silently doing nothing.
    let play_args: PlayArgs = match cli.command {
        Command::Play(p) => p,
        Command::Render(_) | Command::Analyze(_) => {
            println!("`render`/`analyze` are recognized but not yet wired in Phase 1; use `play`.");
            return Ok(());
        }
        Command::Modem(_) => {
            println!("Use the dedicated modem bins (modem_encode/modem_decode/channel_sim/make_packetized), or `audiohax modem …` once wired.");
            return Ok(());
        }
        Command::Tui(_) => {
            println!("Use the dedicated `audiohax-tui` bin for the terminal UI.");
            return Ok(());
        }
    };

    // ── `--list-midi-ports` query short-circuit (WS-4 S12) ──────────────────────
    // This is a query, not playback: enumerate the available MIDI output ports and
    // exit BEFORE any mapping/image work. The printed index is the selector accepted
    // by `--midi-port <index>`.
    if play_args.list_midi_ports {
        match MidiOut::list_ports() {
            Ok(ports) if !ports.is_empty() => {
                println!("Available MIDI output ports:");
                for (i, name) in ports {
                    println!("  [{i}] {name}");
                }
            }
            Ok(_) => println!(
                "No MIDI output ports found. Use `--midi-virtual` (Linux/macOS) to create \
                 one, or start a synth (loopMIDI/IAC/Qsynth) and re-run."
            ),
            Err(e) => eprintln!("Could not enumerate MIDI ports: {e}"),
        }
        return Ok(());
    }

    // ── Mappings + engine config ────────────────────────────────────────────────
    let mappings = load_mappings("assets/mappings.json")?;
    println!("Mappings loaded.");

    let engine_config = pipeline_to_engine_config(&play_args.pipeline);
    let instrument_count = engine_config.num_instruments;
    let bar_thickness_frac = engine_config.bar_thickness_frac;
    let ms_per_step = engine_config.ms_per_step;
    let num_steps = play_args.pipeline.steps;
    let jitter_percent = play_args.pipeline.jitter_percent;
    println!("Instrument count: {}", instrument_count);
    println!(
        "Scan bar thickness = {:.2}, steps = {}, ms/step = {}, jitter% = {}",
        bar_thickness_frac, num_steps, ms_per_step, jitter_percent
    );

    // ── Image acquisition + feature extraction (analyzer selection by feature) ──
    //
    // WS-4 Phase 2 (S11) Lane C: the DEFAULT (pure-analysis, no opencv) path acquires
    // the image and extracts features with the pure-Rust analyzer; the `opencv` flag
    // selects the legacy OpenCV path (the A/B reference, plus camera + highgui window
    // + overlay PNGs). Both produce something that implements `engine::FeatureSource`,
    // so the engine driver below is identical behind the trait (design §3.C).

    // OpenCV path: acquire via OpenCV, scan, write overlays, build the PrecomputedSource.
    #[cfg(feature = "opencv")]
    let (source, _ocv_img, _ocv_dims) = {
        let src = resolve_source(&play_args.image);
        let img = load_image_from_source(&src)?;
        println!("Image loaded from source (OpenCV).");

        let global_features = analyze_global(&img)?;
        println!("Global features: {:?}", global_features);

        let steps = scan_image(&img, instrument_count, bar_thickness_frac, num_steps, None)?;
        println!("Completed scanning image. Steps: {}", steps.len());

        // Save overlays for inspection: first / mid / last (OpenCV imwrite).
        let width = img.cols();
        let height = img.rows();
        let indices = vec![0usize, steps.len() / 2, steps.len().saturating_sub(1)];
        for &si in &indices {
            let (rect, vertical_default, _bw, _bh) =
                step_rect(si, steps.len(), width, height, bar_thickness_frac);
            if let Ok(overlay) =
                draw_scan_bar_overlay_for_rect(&img, rect, instrument_count, vertical_default)
            {
                let out = format!("assets/overlay_step_{}.png", si);
                if let Err(e) =
                    opencv::imgcodecs::imwrite(&out, &overlay, &opencv::core::Vector::new())
                {
                    println!("Warning: failed to write overlay {}: {}", out, e);
                } else {
                    println!("Wrote overlay for step {} to {}", si, out);
                }
            }
        }
        (
            PrecomputedSource::new(&global_features, &steps),
            img,
            (width, height),
        )
    };

    // DEFAULT (pure) path: acquire + analyze with the pure-Rust `pure_analysis` module.
    #[cfg(not(feature = "opencv"))]
    let source = {
        use audiohax::pure_analysis::{load_pure_image, PureAnalysisSource, PureImageSource};
        let psrc = match &play_args.image {
            Some(p) => PureImageSource::UserPath(p.clone()),
            None => PureImageSource::UserPath(std::path::PathBuf::from("assets/images/example.jpg")),
        };
        let img = load_pure_image(&psrc)?;
        println!(
            "Image loaded from source (pure-Rust): {}x{}",
            img.width(),
            img.height()
        );
        let src = PureAnalysisSource::extract(
            &img,
            instrument_count,
            bar_thickness_frac,
            num_steps,
            None,
        )?;
        println!(
            "Completed scanning image (pure-Rust). Steps: {}",
            src.step_count()
        );
        println!("Global features: {:?}", src.global_features());
        src
    };

    // ── Build the engine + feed global features (the engine derives mode/plan) ──
    let mut engine = PipelineEngine::new(mappings, engine_config);
    engine.set_features_global(&source.global_features());
    println!("Engine mode: {}", engine.current_state().mode);

    // ── Playback ────────────────────────────────────────────────────────────────
    // `play` always plays. The driver loop pulls per-step decisions from the engine
    // and the ADAPTER applies jitter + Instant scheduling (D8 — timing/RNG are the
    // adapter's). The OpenCV path additionally draws the highgui scan-bar overlay.

    #[cfg(feature = "opencv")]
    let _ = opencv::highgui::named_window("ScanBar Live", opencv::highgui::WINDOW_AUTOSIZE);

    if source.step_count() == 0 {
        println!("No steps to play.");
        return Ok(());
    }

    // ── Sink selection at RUNTIME (WS-4 S12) ────────────────────────────────────
    // Both sinks are compiled in; the concrete one is chosen here, not by cfg.
    // `--midi-virtual` forces the MIDI sink; otherwise `--output` decides (default
    // `synth`). The engine driver below speaks only `Box<dyn AudioSink>` — the seam.
    let want_midi =
        matches!(play_args.output, OutputSink::Midi) || play_args.midi_virtual.is_some();

    let mut sink: Box<dyn AudioSink> = if want_midi {
        let midi = if let Some(vname) = play_args.midi_virtual.as_deref() {
            println!(
                "Creating virtual MIDI output port '{vname}' (subscribe to it from your DAW/Qsynth)..."
            );
            MidiOut::open_virtual(vname)?
        } else {
            let env_port = std::env::var("AUDIOHAX_MIDI_PORT").ok();
            let selector = play_args.midi_port.as_deref().or(env_port.as_deref());
            println!("Connecting to external MIDI port (selector = {selector:?})...");
            MidiOut::open_selector(selector)?
        };
        println!("MIDI output ready.");
        Box::new(midi)
    } else {
        println!("Starting in-process synth (rustysynth + cpal, bundled SoundFont)...");
        let synth = audiohax::synth_sink::SynthSink::with_bundled_soundfont()?;
        println!("Synth audio stream started @ {} Hz.", synth.sample_rate());
        Box::new(synth)
    };

    // Initial per-channel programs (same scheme as before: prog = (i*7)%128).
    for i in 0..instrument_count {
        let ch = (i % 16) as u8;
        let prog = ((i * 7) % 128) as u8;
        // via the AudioSink trait so the adapter speaks one vocabulary to the engine's sink.
        let _ = sink.program_change(ch, prog);
    }

    let total_steps = source.step_count();
    let mut rng = rand::thread_rng();

    for step_idx in 0..total_steps {
        // 1) Overlay for this step (OpenCV highgui — adapter; opencv path only).
        #[cfg(feature = "opencv")]
        {
            let (width, height) = _ocv_dims;
            let (rect, vertical_default, _bw, _bh) =
                step_rect(step_idx, total_steps, width, height, bar_thickness_frac);
            if let Ok(overlay) =
                draw_scan_bar_overlay_for_rect(&_ocv_img, rect, instrument_count, vertical_default)
            {
                let _ = opencv::highgui::imshow("ScanBar Live", &overlay);
                let _ = opencv::highgui::wait_key(1);
            }
        }

        // 2) Pure musical decisions from the engine (no jitter, no wall clock).
        let decisions = engine.decide_step(&source, step_idx);

        // 3) Adapter applies jitter + Instant scheduling, then sends via the sink.
        let mut events: Vec<ScheduledEvent> = Vec::new();
        let t0 = Instant::now();
        for dec in &decisions {
            let channel = dec.channel;
            for ev in &dec.events {
                // jitter_percent on hold_ms (±percent), identical to the old worker path.
                let jitter = rng
                    .gen_range(-(jitter_percent * 100.0) as i32..=(jitter_percent * 100.0) as i32)
                    as f32
                    / 100.0;
                let base_hold = ev.hold_ms as f32;
                let hold_ms_f = (base_hold * (1.0 + jitter)).max(8.0).round() as u64;

                let start_instant = t0 + Duration::from_millis(ev.offset_ms);
                events.push(ScheduledEvent {
                    at: start_instant,
                    on: true,
                    channel,
                    note: ev.note,
                    vel: ev.velocity,
                });
                events.push(ScheduledEvent {
                    at: start_instant + Duration::from_millis(hold_ms_f),
                    on: false,
                    channel,
                    note: ev.note,
                    vel: 0,
                });
            }
        }

        // 4) Time-order and execute (single-threaded wall-clock playback — adapter).
        events.sort_by_key(|e| e.at);
        for sev in events {
            let now = Instant::now();
            if sev.at > now {
                std::thread::sleep(sev.at - now);
            }
            if sev.on {
                if let Err(e) = sink.note_on(sev.channel, sev.note, sev.vel) {
                    eprintln!("note_on error: {}", e);
                }
            } else if let Err(e) = sink.note_off(sev.channel, sev.note) {
                eprintln!("note_off error: {}", e);
            }
        }
    }

    println!("Completed playback of {} steps.", total_steps);
    Ok(())
}

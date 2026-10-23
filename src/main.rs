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
// mode/progression/plan derivation. The wall-clock `thread::sleep` streaming STAYS here
// (playback timing is an adapter concern); the EVENT timeline itself is now built once by
// `build_step_event_timeline` and shared with `render --wav` (S40 divergence #2 fix), so
// `play == render` by construction and the old per-step `ScheduledEvent` batch/drain (which
// serialized cross-step note overlap and is the reason this fix exists) is dissolved.
//
// WS-4 Phase 2 (S11) Lane C: the DEFAULT build is now PURE RUST (`pure-analysis` +
// `synth`) and DOES compile on a headless/clean box. The OpenCV acquisition/analysis
// + highgui window + overlay PNGs are `#[cfg(feature="opencv")]`-gated; the external
// MIDI-out sink is `#[cfg(feature="midi-out")]`-gated. With no flags, main.rs loads
// the image via `pure_analysis::load_pure_image`, drives the engine with
// `PureAnalysisSource`, and plays via the in-process `SynthSink`. (design §3.C/§5.2)
//
// S122 Phase 0 (operator ruling D-28): the render orchestration (`run_render_wav`'s body) and
// the live play loop were MOVED into the library module `audiohax::render` (`render()`,
// `prepare_play()` + `play()`, `build_step_event_timeline`), so the CLI, the TUI and a GUI
// call ONE function and cannot drift. This file keeps the I/O: argv, `mappings.json`, file
// paths, every stdout/stderr line (driven by the lib's `RenderNote`/`PlayEvent` at the exact
// old sequence points), sink construction, and the process-global Ctrl-C handler (BUG-01),
// which drives the lib's `CancelToken`. The OpenCV acquisition stays here too (its modules are
// bin-private) and calls the lib's `*_with_source` entry points.

// OpenCV-only adapter modules — pulled in only under the `opencv` feature.
#[cfg(feature = "opencv")]
mod image_analysis;
#[cfg(feature = "opencv")]
mod image_source;
// External MIDI-out adapter module. S59 Lane H: this module pulls `midir` (via
// `midi_output.rs`), so it is compiled ONLY under the `midi-out` feature. `midi-out`
// is in the DEFAULT set, so the shipped binary still compiles it and selects the sink
// at RUNTIME (see the `--output`/`--midi-virtual` branch below); a pure-Rust build with
// `--no-default-features --features pure-analysis,synth` drops it (no `midir`, no MIDI
// sink) and uses the in-process synth exclusively.
#[cfg(feature = "midi-out")]
mod midi_output;

use audiohax::cli::{
    pipeline_to_engine_config, Cli, Command, OutputSink, PlayArgs, PresetArgs, RenderArgs,
};
use audiohax::engine::AudioSink;
#[cfg(feature = "opencv")]
use audiohax::engine::FeatureSource;
use audiohax::mapping_loader::{resolve_cli_mappings_with_origin, MappingTable, MappingsOrigin};
use audiohax::presets::ExplicitArgs;
use audiohax::render::{CancelToken, PlayEvent, RenderError, RenderNote, RenderRequest};
use clap::{CommandFactory, FromArgMatches};

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

// ── External MIDI-out sink import (gated by `midi-out`) ──────────────────────
// S59 Lane H: `MidiOut` and its `AudioSinkError` use are compiled ONLY when the
// `midi-out` feature is on (it pulls `midir`). `midi-out` is in the DEFAULT set, so
// the shipped binary still compiles both and selects the sink at runtime; a
// `--no-default-features --features pure-analysis,synth` build omits them entirely and
// the in-process synth is the sole sink.
#[cfg(feature = "midi-out")]
use audiohax::engine::AudioSinkError;
#[cfg(feature = "midi-out")]
use midi_output::MidiOut;

/// `engine::AudioSink for MidiOut` lives HERE (S9 §3.3 / D3) — the lib cannot name the
/// bin-private `MidiOut`, and even a re-export would be an orphan violation. Each
/// method maps `MidiOut`'s `Box<dyn Error>` (NOT `Send + Sync`) into an
/// `AudioSinkError` by stringifying it — keeping `midi_output.rs` untouched.
/// S59 Lane H: gated on `midi-out` (the feature that compiles `MidiOut`). Under the
/// default set the sink is still selected at runtime; a pure-Rust build omits it.
#[cfg(feature = "midi-out")]
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

/// S31 A/B harness: render `render_args.image` to a WAV at `wav_path`, OFFLINE (no audio
/// device) and DETERMINISTICALLY (no jitter, no RNG, no wall clock), honoring
/// `--soundfont`/`--reverb`/`--gain`. Because the engine's per-step decisions are
/// deterministic and we lay them out on a fixed `ms_per_step` grid WITHOUT the live
/// jitter, the same image+config always yields a byte-identical WAV — which is exactly
/// the apples-to-apples property an A/B comparison needs.
///
/// This reuses the SAME engine + feature-source path as `play` (so the rendered music is
/// the real composition, not a stand-in); only the SINK differs (offline rustysynth →
/// WAV instead of the live cpal stream / external MIDI).
///
/// S122: the orchestration lives in `audiohax::render::render`; this is the CLI adapter
/// (request, mappings file, image path, the dump file, the WAV file, and every printed line).
fn run_render_wav(
    render_args: RenderArgs,
    wav_path: &std::path::Path,
    explicit: &ExplicitArgs,
) -> Result<(), Box<dyn std::error::Error>> {
    use audiohax::synth_sink::{write_stereo_wav, SoundFontSource};

    // S124: the preset (if any) is laid under the typed flags first (L-30); it prints nothing.
    // The `--loop`/`--gain` refusal ran BEFORE `mappings.json` was read; keep it first.
    let req = with_preset(
        RenderRequest::from(&render_args),
        &render_args.presets,
        explicit,
    )?;
    req.validate()?;

    let (mappings, origin) = resolve_mappings(&render_args.presets)?;

    // ── Acquire the feature source (same selection as `play`; opencv vs pure). ──
    // The pure-Rust acquisition happens inside `audiohax::render::render`.
    #[cfg(feature = "opencv")]
    let source = {
        let engine_config = pipeline_to_engine_config(&render_args.pipeline);
        let instrument_count = engine_config.num_instruments;
        let bar_thickness_frac = engine_config.bar_thickness_frac;
        let num_steps = render_args.pipeline.steps;
        let src = resolve_source(&render_args.image);
        let img = load_image_from_source(&src)?;
        let global_features = analyze_global(&img)?;
        let steps = scan_image(&img, instrument_count, bar_thickness_frac, num_steps, None)?;
        PrecomputedSource::new(&global_features, &steps)
    };

    let font_src = match &render_args.audio.soundfont {
        Some(p) => SoundFontSource::Path(p.as_path()),
        None => SoundFontSource::Bundled,
    };

    // Every line `render --wav` prints from inside the render, printed at the point the lib
    // reports it (the lib prints nothing). Format strings are the pre-S122 ones, verbatim.
    let mut on_note = |note: RenderNote<'_>| -> Result<(), RenderError> {
        match note {
            RenderNote::Planned {
                total_steps,
                instrument_count,
            } => println!(
                "render --wav: {} steps, {} instruments → {}",
                total_steps,
                instrument_count,
                wav_path.display()
            ),
            RenderNote::EventDumpArming => {
                if let Some(p) = &render_args.event_dump {
                    println!("render --wav: event dump ARMED → {}", p.display());
                }
            }
            RenderNote::ExpressionVerified { armed, total } => {
                println!("render --wav: expression ARMED and VERIFIED on {armed}/{total} sections");
            }
            RenderNote::V2TermVerified {
                label,
                field,
                armed: field_armed,
                total,
            } => println!(
                "render --wav: v2 term {label} ({field}) ARMED and VERIFIED on {field_armed}/{total} sections"
            ),
            RenderNote::EventDumpJson(json) => {
                if let Some(p) = &render_args.event_dump {
                    std::fs::write(p, json)?;
                }
            }
            RenderNote::EventDumpWritten {
                total_events,
                frames,
                silent_frames,
            } => {
                if let Some(p) = &render_args.event_dump {
                    println!(
                        "render --wav: event dump wrote {} events over {} frames ({} silent) → {}",
                        total_events,
                        frames,
                        silent_frames,
                        p.display()
                    );
                }
            }
            RenderNote::LoopFolded {
                total_steps,
                grid_ms_per_step,
                period_ms,
                period_frames,
                folded_ms,
            } => println!(
                "render --wav: --loop → period {} steps x {} ms = {} ms ({} frames); \
                 folded {} ms of ring-out onto the head",
                total_steps, grid_ms_per_step, period_ms, period_frames, folded_ms,
            ),
            // `render --wav` never printed the acquisition lines or the play-path mode line.
            RenderNote::ImageLoaded { .. }
            | RenderNote::ImageScanned { .. }
            | RenderNote::GlobalFeatures(_)
            | RenderNote::EngineMode(_) => {}
        }
        Ok(())
    };

    #[cfg(feature = "opencv")]
    let out = audiohax::render::render_with_source(
        &req,
        &source,
        None,
        &mappings,
        font_src,
        &mut on_note,
    )
    .map_err(|e| name_mappings_in(e, &origin))?;
    #[cfg(all(not(feature = "opencv"), feature = "pure-analysis"))]
    let out = {
        let default_image = std::path::PathBuf::from("assets/images/example.jpg");
        let image = render_args.image.as_deref().unwrap_or(&default_image);
        audiohax::render::render(
            &req,
            audiohax::render::ImageInput::Path(image),
            &mappings,
            font_src,
            &mut on_note,
        )
        .map_err(|e| name_mappings_in(e, &origin))?
    };

    write_stereo_wav(wav_path, out.sample_rate, &out.interleaved)?;

    let secs = out.interleaved.len() as f32 / 2.0 / out.sample_rate as f32;
    println!(
        "render --wav: wrote {} ({:.1}s, font={}, reverb={}, gain={})",
        wav_path.display(),
        secs,
        render_args
            .audio
            .soundfont
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "bundled".into()),
        if req.reverb.is_on() { "on" } else { "off" },
        req.gain
    );
    Ok(())
}
/// S124 (lead ruling L-30): lay the chosen preset (if any) under `req`; a flag the user typed
/// keeps its value. No preset ⇒ `req` unchanged. Prints nothing (`s124-01` LD-4).
fn with_preset(
    req: RenderRequest,
    presets: &PresetArgs,
    explicit: &ExplicitArgs,
) -> Result<RenderRequest, Box<dyn std::error::Error>> {
    let Some(id) = presets.preset.as_deref() else {
        return Ok(req);
    };
    let preset = audiohax::presets::find(id).map_err(|e| e.to_string())?;
    let args = audiohax::presets::preset_render_args(preset).map_err(|e| e.to_string())?;
    Ok(req.overlay_preset(&RenderRequest::from(&args), explicit))
}

/// S124 (lead rulings L-29, L-32): the mappings for a `render`/`play` run, always through the
/// library's bounded, validated loader. `--preset P` ⇒ P's (embedded) mappings; `--mappings FILE`
/// ⇒ that file; neither ⇒ `./assets/mappings.json` if it exists, else the built-in copy.
fn resolve_mappings(
    presets: &PresetArgs,
) -> Result<(MappingTable, MappingsOrigin), Box<dyn std::error::Error>> {
    match presets.preset.as_deref() {
        Some(id) => {
            let preset = audiohax::presets::find(id).map_err(|e| e.to_string())?;
            let table = audiohax::presets::resolve_mappings(preset).map_err(|e| e.to_string())?;
            Ok((table, MappingsOrigin::Builtin))
        }
        None => Ok(resolve_cli_mappings_with_origin(
            presets.mappings.as_deref(),
        )?),
    }
}

/// S124 (L-41): the MB-5 piece-length refusal comes from the library, which does not know which
/// mappings were used; name them, and the fields that set a piece's tempo and length. Every other
/// error passes through unchanged.
///
/// The named refusal is returned as a [`PlainRefusal`], so `main`'s `Error: …` line prints it as a
/// sentence (like every mappings refusal) rather than as a quoted, re-escaped Debug string. Every
/// other error keeps its existing form (`RenderError`'s Debug is pinned to the pre-S122 output).
fn name_mappings_in(e: RenderError, origin: &MappingsOrigin) -> Box<dyn std::error::Error> {
    match e {
        RenderError::Refused(msg) if msg.starts_with(audiohax::render::PIECE_TOO_LONG_PREFIX) => {
            let head = msg.split(" (The mappings in use").next().unwrap_or(&msg);
            Box::new(PlainRefusal(format!(
                "{head} {origin} sets this piece's tempo (`composition.affect.energy_tempo`, \
                 `composition.affect.character_tempo`, `global.brightness_to_tempo_bpm`) and its \
                 length (`composition.form_catalogue[].sections`)."
            )))
        }
        other => Box::new(other),
    }
}

/// A refusal whose `Debug` is its `Display` text, so returning it from `main` prints the sentence
/// itself, unquoted (`MappingsError` does the same).
struct PlainRefusal(String);

impl std::fmt::Display for PlainRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::fmt::Debug for PlainRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for PlainRefusal {}

/// Build the in-process synth sink (rustysynth + cpal), honoring the `--soundfont` /
/// `--reverb` / `--gain` A/B controls. This is the DEFAULT sink and — on a build
/// WITHOUT the `midi-out` feature — the ONLY sink. Extracted (S59 Lane H) so the two
/// `midi-out`-gated sink-selection arms below share one construction site instead of
/// duplicating it; no flags ⇒ `SynthConfig::default()` + bundled font ⇒ byte-identical
/// to the pre-extraction inline path.
///
/// S124: reverb and gain come from the RESOLVED request (`req`), so a preset's values reach the
/// live synth; with no preset they are exactly `--reverb` / `--gain`.
fn build_synth_sink(
    play_args: &PlayArgs,
    req: &RenderRequest,
) -> Result<Box<dyn AudioSink>, Box<dyn std::error::Error>> {
    use audiohax::synth_sink::{SoundFontSource, SynthConfig, SynthSink};
    let synth_config = SynthConfig {
        enable_reverb_and_chorus: req.reverb.is_on(),
        gain: req.gain,
    };
    let font_src = match &play_args.audio.soundfont {
        Some(p) => SoundFontSource::Path(p.as_path()),
        None => SoundFontSource::Bundled,
    };
    match &play_args.audio.soundfont {
        Some(p) => println!(
            "Starting in-process synth (rustysynth + cpal, SoundFont {})...",
            p.display()
        ),
        None => println!("Starting in-process synth (rustysynth + cpal, bundled SoundFont)..."),
    }
    println!(
        "  reverb/chorus = {}, master gain = {}",
        if synth_config.enable_reverb_and_chorus {
            "on"
        } else {
            "off"
        },
        synth_config.gain
    );
    let synth = SynthSink::new_with_config(font_src, synth_config)?;
    println!("Synth audio stream started @ {} Hz.", synth.sample_rate());
    Ok(Box::new(synth))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ── CLI (shared clap grammar) ───────────────────────────────────────────────
    // S124 (L-30): `Cli::parse()` split into its own two steps, so the typed-flag set can be read
    // from the matches (`value_source`) for the preset precedence. The error path is clap's
    // `parse()` body verbatim (format against a fresh `Cli::command()`, then exit), so every clap
    // message and exit code is unchanged.
    let matches = Cli::command().get_matches();
    let explicit = ExplicitArgs::from_matches(&matches);
    let cli =
        Cli::from_arg_matches(&matches).unwrap_or_else(|e| e.format(&mut Cli::command()).exit());

    // Phase 1 wires the `play` subcommand (the others — render/analyze/modem — are
    // surfaced by the grammar but their adapter bodies are follow-on work; the modem
    // subcommand mirrors the dedicated bins). Map non-play to a friendly message
    // rather than silently doing nothing.
    let play_args: PlayArgs = match cli.command {
        Command::Play(p) => p,
        // S31: `render --wav <PATH>` does an offline, deterministic synth-to-WAV render
        // (the A/B harness output). `render` without `--wav` keeps the Phase-1 message.
        Command::Render(r) => {
            if let Some(wav_path) = r.wav.clone() {
                return run_render_wav(r, &wav_path, &explicit);
            }
            println!(
                "`render` without `--wav` is recognized but not yet wired; pass `--wav <PATH>` \
                 to render the synthesized audio to a WAV (offline A/B), or use `play`."
            );
            return Ok(());
        }
        Command::Analyze(_) => {
            println!("`analyze` is recognized but not yet wired in Phase 1; use `play`.");
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
    // by `--midi-port <index>`. S59 Lane H: MIDI enumeration exists only when `midi-out`
    // is compiled in; a pure-Rust build answers the query with a clear "not compiled in".
    #[cfg(feature = "midi-out")]
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
    #[cfg(not(feature = "midi-out"))]
    if play_args.list_midi_ports {
        println!(
            "This build has no `midi-out` feature, so there are no external MIDI output \
             ports to list. Rebuild with `--features midi-out` to enable external MIDI."
        );
        return Ok(());
    }

    // ── Mappings + engine config ────────────────────────────────────────────────
    // S124: the request is resolved first (a preset under the typed flags, L-30; prints nothing),
    // so the lines below print the values the piece is actually made with.
    let req = with_preset(
        RenderRequest::from(&play_args),
        &play_args.presets,
        &explicit,
    )?;
    let (mappings, origin) = resolve_mappings(&play_args.presets)?;
    println!("Mappings loaded.");

    let engine_config = pipeline_to_engine_config(&req.pipeline);
    let instrument_count = engine_config.num_instruments;
    let bar_thickness_frac = engine_config.bar_thickness_frac;
    let ms_per_step = engine_config.ms_per_step;
    let num_steps = req.pipeline.steps;
    let jitter_percent = req.pipeline.jitter_percent;
    println!("Instrument count: {}", instrument_count);
    println!(
        "Scan bar thickness = {:.2}, scan-viz steps = {}, scan-viz ms/step = {}, jitter% = {}",
        bar_thickness_frac, num_steps, ms_per_step, jitter_percent
    );

    // ── Image acquisition + composition (S122: in `audiohax::render`) ────────────────
    //
    // WS-4 Phase 2 (S11) Lane C: the DEFAULT (pure-analysis, no opencv) path acquires
    // the image and extracts features with the pure-Rust analyzer; the `opencv` flag
    // selects the legacy OpenCV path (the A/B reference, plus camera + highgui window
    // + overlay PNGs). Both produce something that implements `engine::FeatureSource`,
    // so the engine driver is identical behind the trait (design §3.C). S122: the seed /
    // cadence registers, per-voice programs, engine build, composer plan AND the shared event
    // timeline are all built by `audiohax::render::prepare_play*` on THIS thread, in one call.
    let mut on_note = |note: RenderNote<'_>| -> Result<(), RenderError> {
        match note {
            RenderNote::ImageLoaded { width, height } => {
                println!("Image loaded from source (pure-Rust): {}x{}", width, height)
            }
            RenderNote::ImageScanned { steps } => {
                println!("Completed scanning image (pure-Rust). Steps: {}", steps)
            }
            RenderNote::GlobalFeatures(g) => println!("Global features: {:?}", g),
            RenderNote::EngineMode(mode) => println!("Engine mode: {}", mode),
            // Render-only notes, and the expression read-back (CLI `play` never arms it).
            RenderNote::Planned { .. }
            | RenderNote::EventDumpArming
            | RenderNote::ExpressionVerified { .. }
            | RenderNote::V2TermVerified { .. }
            | RenderNote::EventDumpJson(_)
            | RenderNote::EventDumpWritten { .. }
            | RenderNote::LoopFolded { .. } => {}
        }
        Ok(())
    };
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

    #[cfg(feature = "opencv")]
    let plan =
        audiohax::render::prepare_play_with_source(&req, &source, None, &mappings, &mut on_note)
            .map_err(|e| name_mappings_in(e, &origin))?;

    // DEFAULT (pure) path: acquire + analyze with the pure-Rust `pure_analysis` module.
    #[cfg(all(not(feature = "opencv"), feature = "pure-analysis"))]
    let plan = {
        let default_image = std::path::PathBuf::from("assets/images/example.jpg");
        let image = play_args.image.as_deref().unwrap_or(&default_image);
        audiohax::render::prepare_play(
            &req,
            audiohax::render::ImageInput::Path(image),
            &mappings,
            &mut on_note,
        )
        .map_err(|e| name_mappings_in(e, &origin))?
    };

    // ── Playback ────────────────────────────────────────────────────────────────
    // `play` always plays. `audiohax::render::play` streams the ONE shared timeline (S40)
    // on the absolute plan grid; the OpenCV path additionally draws the highgui scan-bar
    // overlay from the `PlayEvent::Step` playhead.

    #[cfg(feature = "opencv")]
    let _ = opencv::highgui::named_window("ScanBar Live", opencv::highgui::WINDOW_AUTOSIZE);

    if plan.source_step_count == 0 {
        println!("No steps to play.");
        return Ok(());
    }

    // ── Sink selection at RUNTIME ───────────────────────────────────────────────
    // The in-process synth is the default and — on a build without the `midi-out`
    // feature — the ONLY sink. When `midi-out` is compiled in (it is, in the DEFAULT
    // set), `--midi-virtual` forces the external MIDI sink and otherwise `--output`
    // decides (default `synth`). The engine driver below speaks only `Box<dyn
    // AudioSink>` — the seam — so the reduced pure-Rust build differs only in which
    // concrete sink can be constructed. S59 Lane H: the MIDI arm is cfg-gated so the
    // pure-Rust build carries no `MidiOut`/`midir` reference.
    #[cfg(feature = "midi-out")]
    let mut sink: Box<dyn AudioSink> = {
        let want_midi =
            matches!(play_args.output, OutputSink::Midi) || play_args.midi_virtual.is_some();
        if want_midi {
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
            Box::new(midi) as Box<dyn AudioSink>
        } else {
            // S31: honor the A/B controls. No flags ⇒ SynthConfig::default() + Bundled
            // font ⇒ byte-identical to the pre-S31 path (see `build_synth_sink`).
            build_synth_sink(&play_args, &req)?
        }
    };

    // Pure-Rust build (no `midi-out`): the synth is the only sink. If the operator
    // still asked for MIDI, say so plainly and fall back rather than silently ignoring.
    #[cfg(not(feature = "midi-out"))]
    let mut sink: Box<dyn AudioSink> = {
        if matches!(play_args.output, OutputSink::Midi) || play_args.midi_virtual.is_some() {
            eprintln!(
                "Note: this build has no `midi-out` feature; `--output midi` / `--midi-virtual` \
                 are ignored — using the in-process synth. Rebuild with `--features midi-out` \
                 for external MIDI."
            );
        }
        build_synth_sink(&play_args, &req)?
    };

    // ── Graceful-shutdown wiring (BUG-01) ───────────────────────────────────────
    // An abrupt exit (Ctrl-C / SIGINT) while a note is still sounding on the external
    // `--output midi` path would otherwise leave that note sustaining forever in the
    // EXTERNAL synth — the synth is a separate process that outlives us and never gets
    // the note-off. We convert SIGINT into a graceful return: the handler only flips an
    // AtomicBool, the playback loop polls it and BREAKS within ~one step, the function
    // returns normally, the `MidiOut` is dropped, and its `Drop` fires the all-sound-off
    // panic. We avoid trying to move the (non-Send) sink into the handler thread; the
    // flag + break + Drop path is the clean, portable route. For the in-process synth
    // path this simply lets the process exit cleanly (the cpal stream is self-healing).
    let shutdown = CancelToken::default();
    {
        let shutdown = shutdown.clone();
        // First Ctrl-C requests a graceful stop. If the handler is somehow installed
        // more than once (it is not, here), `set_handler` would error — surface it.
        if let Err(e) = ctrlc::set_handler(move || {
            shutdown.cancel();
        }) {
            eprintln!("could not install Ctrl-C handler (continuing without it): {e}");
        }
    }

    // `jitter_percent` is still parsed/printed above for CLI compatibility but no longer
    // perturbs the timeline (the builder is deterministic, so the live monitor exactly equals
    // the WAV gate — S40 divergence #2).
    let _ = jitter_percent; // (parsed for CLI compat; timeline is deterministic — see above)

    // Every line the live loop printed, printed at the point `play` reports it.
    let total_steps = plan.total_steps;
    let mut progress = |ev: PlayEvent| match ev {
        // S40: print the ACTUAL playback grid (plan tempo) so the live header no longer reads
        // the scan-viz `ms/step` as if it were the clock — that mismatch was the red herring
        // behind the false "play runs at the wrong tempo" hypothesis (design-s40 §Divergence A).
        PlayEvent::Started {
            total_steps,
            grid_ms_per_step,
        } => println!(
            "Playback grid: {} steps @ {} ms/step (plan tempo) — matches `render --wav`.",
            total_steps, grid_ms_per_step
        ),
        // OpenCV overlay (cosmetic, non-audible) for the step the playhead entered.
        PlayEvent::Step(step_idx) => {
            #[cfg(feature = "opencv")]
            {
                let (width, height) = _ocv_dims;
                let (rect, vertical_default, _bw, _bh) =
                    step_rect(step_idx, total_steps, width, height, bar_thickness_frac);
                if let Ok(overlay) = draw_scan_bar_overlay_for_rect(
                    &_ocv_img,
                    rect,
                    instrument_count,
                    vertical_default,
                ) {
                    let _ = opencv::highgui::imshow("ScanBar Live", &overlay);
                    let _ = opencv::highgui::wait_key(1);
                }
            }
            #[cfg(not(feature = "opencv"))]
            let _ = step_idx;
        }
        PlayEvent::Cancelled => println!("Shutdown requested — stopping playback."),
        PlayEvent::SinkError(e) => eprintln!("sink dispatch error: {}", e),
        PlayEvent::Finished => println!("Completed playback of {} steps.", total_steps),
    };

    // BUG-01: `play` returns normally on Ctrl-C; `sink` is dropped when `main` returns, and
    // `MidiOut`'s Drop fires the all-sound-off panic. The sink never leaves this function.
    audiohax::render::play(&plan, sink.as_mut(), &shutdown, &mut progress)?;
    Ok(())
}

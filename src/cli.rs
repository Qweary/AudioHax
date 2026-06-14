//! src/cli.rs — the WS-4 Phase 1 unified clap CLI grammar (design S9 §3.6 / §3.7).
//!
//! Pure-Rust; builds under `cargo build --lib --no-default-features`. No OpenCV /
//! image / midir type appears here. `main.rs` and each modem bin parse via these
//! structs; the bins keep a thin `main` that calls the matching `parse_*` helper, so
//! their historical CLI shape (and `--no-default-features` build) survives while the
//! whole app shares one coherent, validated, `--help`-bearing grammar.
//!
//! The overloaded legacy `"play"` token (old main.rs selected the example image AND
//! toggled playback off the same word) is resolved into a real [`Command::Play`]
//! subcommand with an optional positional `<IMAGE>`.

use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// Top-level `audiohax` CLI.
#[derive(Debug, Parser, PartialEq)]
#[command(
    name = "audiohax",
    version,
    about = "Image-to-music + MFSK modem toolkit"
)]
pub struct Cli {
    /// Optional config file; defaults to the platform config dir's `audiohax.toml`.
    #[arg(long, global = true, value_name = "FILE")]
    pub config: Option<PathBuf>,
    /// Emit machine-readable JSON instead of human prose where supported.
    #[arg(long, global = true)]
    pub json: bool,
    #[command(subcommand)]
    pub command: Command,
}

/// The top-level subcommands.
#[derive(Debug, Subcommand, PartialEq)]
pub enum Command {
    /// Scan an image and play it to a MIDI/audio sink.
    Play(PlayArgs),
    /// Scan an image and write overlays / analysis without playing.
    Render(RenderArgs),
    /// Analyze an image and print/JSON-dump its features.
    Analyze(AnalyzeArgs),
    /// MFSK data modem operations (unifies the four legacy modem bins).
    #[command(subcommand)]
    Modem(ModemCommand),
    /// Run the headless ratatui TUI over a synthetic (no-OpenCV) feature source —
    /// the WS-4 Phase 3 seam-proof front-end. NOTE: the main `audiohax` binary links
    /// OpenCV and only builds with the default features; the standalone
    /// `audiohax-tui` bin is what actually runs this headlessly under
    /// `--no-default-features`. This arm exists for grammar completeness so the
    /// unified `audiohax` CLI advertises the subcommand.
    Tui(TuiArgs),
}

/// Args for `audiohax tui` (and the standalone `audiohax-tui` bin). Minimal: the
/// synthetic source spans `--steps` scan steps over `--instruments` instruments,
/// paralleling the [`PipelineArgs`] knobs of the same name.
#[derive(Debug, Args, PartialEq, Clone)]
pub struct TuiArgs {
    /// Number of synthetic scan steps to span (default matches the pipeline default).
    #[arg(long, default_value_t = 40)]
    pub steps: usize,
    /// Number of instruments in the ensemble.
    #[arg(long, default_value_t = 4)]
    pub instruments: usize,
}

/// Shared image-pipeline knobs (replaces the five legacy `parse_cli_arg` flags).
/// Defaults match today's values exactly (4 / 0.10 / 40 / 250 / 15.0); the headless
/// test pins that no-regression contract via [`pipeline_to_engine_config`].
#[derive(Debug, Args, PartialEq, Clone)]
pub struct PipelineArgs {
    /// Number of instruments in the ensemble.
    #[arg(long, default_value_t = 4)]
    pub instruments: usize,
    /// Scan-bar thickness as a fraction of the image's scan axis.
    #[arg(long, default_value_t = 0.10)]
    pub thickness: f32,
    /// Number of scan steps across the image.
    #[arg(long, default_value_t = 40)]
    pub steps: usize,
    /// Milliseconds per scan step.
    #[arg(long = "ms-per-step", default_value_t = 250)]
    pub ms_per_step: u64,
    /// Per-event duration jitter, in percent (e.g. 15 → ±15%).
    #[arg(long = "jitter-percent", default_value_t = 15.0)]
    pub jitter_percent: f32,
}

impl Default for PipelineArgs {
    fn default() -> Self {
        // The single source of truth for "today's defaults" used by merge_config as
        // the fallback layer (precedence: flag > file > THESE defaults).
        PipelineArgs {
            instruments: 4,
            thickness: 0.10,
            steps: 40,
            ms_per_step: 250,
            jitter_percent: 15.0,
        }
    }
}

/// Runtime output sink for `audiohax play` (WS-4 S12). Replaces the S11
/// compile-time `#[cfg(feature="midi-out")]` sink choice: both sinks are now
/// compiled into the default binary and selected here at parse time, so a single
/// shipped binary can play in-process OR route to an external MIDI port/DAW with no
/// rebuild. clap renders the variants kebab-cased as `synth` / `midi`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
pub enum OutputSink {
    /// In-process pure-Rust SoundFont synth (rustysynth + cpal). Dry; the default.
    #[default]
    Synth,
    /// Route NoteEvents to an external MIDI port / DAW / FluidSynth / Qsynth.
    Midi,
}

/// Args for `audiohax play`.
#[derive(Debug, Args, PartialEq)]
pub struct PlayArgs {
    /// Image path (resolves the old magic positional). Omit to use the example image.
    pub image: Option<PathBuf>,
    /// Output sink: `synth` (in-process, default) or `midi` (external port/DAW).
    #[arg(long, value_enum, default_value_t = OutputSink::Synth)]
    pub output: OutputSink,
    /// MIDI port selector (only with `--output midi`): a NAME SUBSTRING or a numeric
    /// INDEX (as shown by `--list-midi-ports`). Else `$AUDIOHAX_MIDI_PORT` or the first
    /// available port.
    #[arg(long)]
    pub midi_port: Option<String>,
    /// List the available MIDI output ports (index + name) and exit. Implies no playback.
    #[arg(long)]
    pub list_midi_ports: bool,
    /// Create a virtual MIDI output port that a DAW / Qsynth can subscribe to, instead
    /// of connecting to an existing port. The optional value sets the port name
    /// (default `AudioHaxOut`). Unix only (Linux ALSA / macOS CoreMIDI); on Windows this
    /// errors at construction with guidance to use loopMIDI. Forces `--output midi`.
    #[arg(long, value_name = "NAME", num_args = 0..=1, default_missing_value = "AudioHaxOut")]
    pub midi_virtual: Option<String>,
    #[command(flatten)]
    pub pipeline: PipelineArgs,
}

/// Args for `audiohax render`.
#[derive(Debug, Args, PartialEq)]
pub struct RenderArgs {
    /// Image path; omit to use the example image.
    pub image: Option<PathBuf>,
    #[command(flatten)]
    pub pipeline: PipelineArgs,
}

/// Args for `audiohax analyze`.
#[derive(Debug, Args, PartialEq)]
pub struct AnalyzeArgs {
    /// Image path; omit to use the example image.
    pub image: Option<PathBuf>,
    #[command(flatten)]
    pub pipeline: PipelineArgs,
}

/// Build an [`crate::engine::EngineConfig`] from validated pipeline args. Pure; the
/// unit test that asserts defaults match today's values (4 / 0.10 / 250) pins the
/// no-regression contract.
pub fn pipeline_to_engine_config(args: &PipelineArgs) -> crate::engine::EngineConfig {
    crate::engine::EngineConfig {
        num_instruments: args.instruments,
        ms_per_step: args.ms_per_step,
        bar_thickness_frac: args.thickness,
        // root_midi is fixed at 60 today (main.rs:378); not a CLI knob in Phase 1.
        root_midi: 60,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Modem grammar (S9 §3.7) — one shared grammar, alias-preserving.
// ─────────────────────────────────────────────────────────────────────────────

/// MFSK modem subcommands (unifies the four legacy modem bins).
#[derive(Debug, Subcommand, PartialEq)]
pub enum ModemCommand {
    /// Encode a file to a WAV (legacy `modem_encode`).
    Encode(ModemEncodeArgs),
    /// Decode a WAV back to a file (legacy `modem_decode`).
    Decode(ModemDecodeArgs),
    /// Apply a channel-simulation model to bytes/samples (legacy `channel_sim`).
    #[command(name = "channel-sim")]
    ChannelSim(ChannelSimArgs),
    /// Build a packetized byte stream from a file (legacy `make_packetized`).
    #[command(name = "make-packetized")]
    MakePacketized(MakePacketizedArgs),
}

/// Encoder preset (legacy `--preset fast|balanced|robust`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ModemPreset {
    /// Fast/low-robustness.
    Fast,
    /// Balanced default.
    Balanced,
    /// Maximum robustness.
    Robust,
}

/// Channel-simulation model (legacy `--mode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ChannelMode {
    /// Independent per-bit flips.
    Bitflip,
    /// Byte-burst erasures (zeroing).
    Byteburst,
    /// Packet-aware drop/flip.
    Packet,
    /// S7 seeded acoustic channel (offset / drift / freq offset / echo).
    Acoustic,
}

/// Args for `audiohax modem encode` / legacy `modem_encode`.
///
/// Legacy invocation: `modem_encode <out.wav> <input_file> [options]`.
#[derive(Debug, Args, PartialEq)]
pub struct ModemEncodeArgs {
    /// Output WAV path.
    pub out_wav: PathBuf,
    /// Input file to encode.
    pub input: PathBuf,
    /// Gzip-compress the payload before framing.
    #[arg(long)]
    pub compress: bool,
    /// AES-GCM encrypt with this hex key.
    #[arg(long, value_name = "KEYHEX")]
    pub encrypt: Option<String>,
    /// Number of MFSK channels.
    #[arg(long)]
    pub channels: Option<usize>,
    /// Symbol duration in milliseconds.
    #[arg(long = "symbol-ms")]
    pub symbol_ms: Option<f32>,
    /// Number of tones per channel (M).
    #[arg(long)]
    pub mtones: Option<usize>,
    /// Robustness preset.
    #[arg(long, value_enum)]
    pub preset: Option<ModemPreset>,
    /// Print a duration estimate and exit.
    #[arg(long = "estimate-duration")]
    pub estimate_duration: bool,
    /// Run the simple byte-domain channel simulator on the packet bytes.
    #[arg(long)]
    pub simulate: bool,
    /// Simulator byte-flip probability (legacy `--sim-flip`).
    #[arg(long = "sim-flip")]
    pub sim_flip: Option<f64>,
    /// Simulator burst-erase start probability (legacy `--sim-burst-prob`).
    #[arg(long = "sim-burst-prob")]
    pub sim_burst_prob: Option<f64>,
    /// Simulator average burst length in bytes (legacy `--sim-burst-len`).
    #[arg(long = "sim-burst-len")]
    pub sim_burst_len: Option<usize>,
    /// Write simulated packet bytes to this path (legacy `--sim-out`).
    #[arg(long = "sim-out")]
    pub sim_out: Option<PathBuf>,
    /// Disable RS interleaving (default is interleaved).
    #[arg(long = "no-interleave")]
    pub no_interleave: bool,
    /// Explicitly enable RS interleaving (the default; accepted for legacy
    /// invocation compatibility — `--no-interleave` takes precedence if both given).
    #[arg(long)]
    pub interleave: bool,
    /// Packet size for repetition packetization (legacy `--pkt-size`).
    #[arg(long = "pkt-size")]
    pub pkt_size: Option<usize>,
    /// Repetition count (legacy `--repeats`).
    #[arg(long)]
    pub repeats: Option<usize>,
    /// RS data-shard count (legacy `--rs-data`).
    #[arg(long = "rs-data")]
    pub rs_data: Option<usize>,
    /// RS parity-shard count (legacy `--rs-parity`).
    #[arg(long = "rs-parity")]
    pub rs_parity: Option<usize>,
    /// RS shard size in bytes (legacy `--rs-shard-size`).
    #[arg(long = "rs-shard-size")]
    pub rs_shard_size: Option<usize>,
}

/// Args for `audiohax modem decode` / legacy `modem_decode`.
///
/// Legacy invocation: `modem_decode <in.wav> [out_basename] [options]`.
#[derive(Debug, Args, PartialEq)]
pub struct ModemDecodeArgs {
    /// Input WAV path.
    pub in_wav: PathBuf,
    /// Output basename for the recovered file (default `payload`).
    pub out_basename: Option<String>,
    /// AES-GCM decrypt with this hex key.
    #[arg(long, value_name = "KEYHEX")]
    pub decrypt: Option<String>,
    /// Number of MFSK channels.
    #[arg(long)]
    pub channels: Option<usize>,
    /// Number of tones per channel (M).
    #[arg(long)]
    pub mtones: Option<usize>,
    /// Symbol duration in milliseconds.
    #[arg(long = "symbol-ms")]
    pub symbol_ms: Option<f32>,
    /// Expected repetition count for repetition depacketize.
    #[arg(long)]
    pub repeats: Option<usize>,
    /// RS data-shard count.
    #[arg(long = "rs-data")]
    pub rs_data: Option<usize>,
    /// RS parity-shard count.
    #[arg(long = "rs-parity")]
    pub rs_parity: Option<usize>,
}

/// Args for `audiohax modem channel-sim` / legacy `channel_sim`.
///
/// Legacy invocation: `channel_sim <in_bytes> <out_sim> [--mode ...] [options]`.
/// Legacy used UNDERSCORE flag spellings (`--flip_prob`, `--burst_prob`, etc.); those
/// are preserved as aliases so existing scripts keep working, while a hyphenated
/// canonical spelling is also accepted.
#[derive(Debug, Args, PartialEq)]
pub struct ChannelSimArgs {
    /// Input bytes / raw-i16-samples file.
    pub in_bytes: PathBuf,
    /// Output (simulated) file.
    pub out_sim: PathBuf,
    /// Channel model.
    #[arg(long, value_enum, default_value_t = ChannelMode::Bitflip)]
    pub mode: ChannelMode,
    /// Per-bit flip probability (legacy `--flip_prob`).
    #[arg(long = "flip-prob", alias = "flip_prob", default_value_t = 0.0)]
    pub flip_prob: f64,
    /// Burst-start probability (legacy `--burst_prob`).
    #[arg(long = "burst-prob", alias = "burst_prob", default_value_t = 0.0)]
    pub burst_prob: f64,
    /// Burst length in bytes (legacy `--burst_len`).
    #[arg(long = "burst-len", alias = "burst_len", default_value_t = 16)]
    pub burst_len: usize,
    /// Packet size in bytes (legacy `--packet_size`).
    #[arg(long = "packet-size", alias = "packet_size", default_value_t = 128)]
    pub packet_size: usize,
    /// Expected repetition count for the depacketize attempt.
    #[arg(long)]
    pub repeats: Option<usize>,
    /// RS data-shard count.
    #[arg(long = "rs-data")]
    pub rs_data: Option<usize>,
    /// RS parity-shard count.
    #[arg(long = "rs-parity")]
    pub rs_parity: Option<usize>,
    // ── acoustic-mode knobs (S7) ──
    /// Acoustic-channel RNG seed.
    #[arg(long = "acoustic-seed", default_value_t = 0)]
    pub acoustic_seed: u64,
    /// Acoustic start offset in samples.
    #[arg(long = "start-offset", default_value_t = 0)]
    pub start_offset: usize,
    /// Acoustic clock drift in ppm.
    #[arg(long = "clock-ppm", default_value_t = 0.0)]
    pub clock_ppm: f64,
    /// Acoustic carrier frequency offset in Hz.
    #[arg(long = "freq-offset", default_value_t = 0.0)]
    pub freq_offset: f64,
    /// Acoustic echo delay in samples.
    #[arg(long = "echo-delay", default_value_t = 0)]
    pub echo_delay: usize,
    /// Acoustic echo gain.
    #[arg(long = "echo-gain", default_value_t = 0.0)]
    pub echo_gain: f64,
    /// Acoustic per-sample jitter.
    #[arg(long, default_value_t = 0.0)]
    pub jitter: f64,
}

/// Args for `audiohax modem make-packetized` / legacy `make_packetized`.
///
/// Legacy invocation: `make_packetized <input> <out_packetized> [options]`.
#[derive(Debug, Args, PartialEq)]
pub struct MakePacketizedArgs {
    /// Input file.
    pub input: PathBuf,
    /// Output packetized-bytes path.
    pub out_packetized: PathBuf,
    /// Gzip-compress the payload before framing.
    #[arg(long)]
    pub compress: bool,
    /// RS data-shard count (legacy `--rs-data`).
    #[arg(long = "rs-data")]
    pub rs_data: Option<usize>,
    /// RS parity-shard count (legacy `--rs-parity`).
    #[arg(long = "rs-parity")]
    pub rs_parity: Option<usize>,
    /// RS shard size in bytes (legacy `--rs-shard-size`).
    #[arg(long = "rs-shard-size", default_value_t = 128)]
    pub rs_shard_size: usize,
    /// Packet size for repetition packetization (legacy `--pkt-size`).
    #[arg(long = "pkt-size", default_value_t = 200)]
    pub pkt_size: usize,
    /// Repetition count (legacy `--repeats`).
    #[arg(long, default_value_t = 3)]
    pub repeats: usize,
}

// ── Per-bin standalone parsers ───────────────────────────────────────────────
// Each legacy bin parses ITS OWN subcommand-args struct standalone (so the bin's
// historical CLI shape keeps working under --no-default-features). clap's derive
// `Parser` is only implemented for the top-level struct, so we wrap each Args struct
// in a tiny Parser shim and expose `parse_*()` returning the inner struct.

/// `modem_encode` standalone parser shim.
#[derive(Debug, Parser)]
#[command(
    name = "modem_encode",
    version,
    about = "MFSK modem: encode a file to a WAV"
)]
struct ModemEncodeCli {
    #[command(flatten)]
    args: ModemEncodeArgs,
}

/// `modem_decode` standalone parser shim.
#[derive(Debug, Parser)]
#[command(
    name = "modem_decode",
    version,
    about = "MFSK modem: decode a WAV to a file"
)]
struct ModemDecodeCli {
    #[command(flatten)]
    args: ModemDecodeArgs,
}

/// `channel_sim` standalone parser shim.
#[derive(Debug, Parser)]
#[command(
    name = "channel_sim",
    version,
    about = "MFSK modem: channel simulation"
)]
struct ChannelSimCli {
    #[command(flatten)]
    args: ChannelSimArgs,
}

/// `make_packetized` standalone parser shim.
#[derive(Debug, Parser)]
#[command(
    name = "make_packetized",
    version,
    about = "MFSK modem: build packetized bytes"
)]
struct MakePacketizedCli {
    #[command(flatten)]
    args: MakePacketizedArgs,
}

/// Parse `modem_encode` args from the process argv (legacy bin entry).
pub fn parse_modem_encode() -> ModemEncodeArgs {
    ModemEncodeCli::parse().args
}

/// Parse `modem_decode` args from the process argv (legacy bin entry).
pub fn parse_modem_decode() -> ModemDecodeArgs {
    ModemDecodeCli::parse().args
}

/// Parse `channel_sim` args from the process argv (legacy bin entry).
pub fn parse_channel_sim() -> ChannelSimArgs {
    ChannelSimCli::parse().args
}

/// Parse `make_packetized` args from the process argv (legacy bin entry).
pub fn parse_make_packetized() -> MakePacketizedArgs {
    MakePacketizedCli::parse().args
}

// ─────────────────────────────────────────────────────────────────────────────
// Config file (S9 §3.9 / D7) — audiohax.toml, precedence flag > file > default.
// ─────────────────────────────────────────────────────────────────────────────

/// On-disk config (`audiohax.toml`). Every field optional so a partial file is valid;
/// merge precedence is CLI flag > config-file value > built-in default (D7).
#[derive(Debug, Default, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ConfigFile {
    /// Override the instrument count.
    pub instruments: Option<usize>,
    /// Override the scan-bar thickness fraction.
    pub thickness: Option<f32>,
    /// Override the scan-step count.
    pub steps: Option<usize>,
    /// Override the milliseconds-per-step.
    pub ms_per_step: Option<u64>,
    /// Override the jitter percent.
    pub jitter_percent: Option<f32>,
    /// Override the MIDI port hint.
    pub midi_port: Option<String>,
}

/// Errors from config-file loading.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// I/O error reading the config file.
    #[error("reading config file {path}: {source}")]
    Io {
        /// The path that failed.
        path: String,
        /// The underlying I/O error.
        source: std::io::Error,
    },
    /// TOML parse error.
    #[error("parsing config TOML {path}: {source}")]
    Parse {
        /// The path that failed.
        path: String,
        /// The underlying parse error.
        source: toml::de::Error,
    },
}

/// Resolve the default config-file path (platform config dir's `audiohax.toml`).
/// Returns `None` if the platform dir cannot be determined.
pub fn default_config_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "audiohax").map(|d| d.config_dir().join("audiohax.toml"))
}

/// Load the config file from an explicit path or the platform config dir
/// ([`default_config_path`]). A MISSING file → `Ok(ConfigFile::default())` (config is
/// optional). Pure I/O + TOML parse; no OpenCV.
pub fn load_config(explicit: Option<&std::path::Path>) -> Result<ConfigFile, ConfigError> {
    let path: Option<PathBuf> = match explicit {
        Some(p) => Some(p.to_path_buf()),
        None => default_config_path(),
    };
    let Some(path) = path else {
        // No explicit path and no platform dir → no config; defaults apply.
        return Ok(ConfigFile::default());
    };
    if !path.exists() {
        return Ok(ConfigFile::default());
    }
    let text = std::fs::read_to_string(&path).map_err(|source| ConfigError::Io {
        path: path.display().to_string(),
        source,
    })?;
    toml::from_str(&text).map_err(|source| ConfigError::Parse {
        path: path.display().to_string(),
        source,
    })
}

/// Merge precedence: CLI-provided values win over file values win over defaults (D7).
///
/// `cli` carries each pipeline flag's effective value AS PARSED (clap fills unset
/// flags with their `default_value_t`). To distinguish "flag absent" from "flag ==
/// default" — the precedence subtlety in S9 §6 risk 7 — the caller passes the
/// `defaults` struct, and this fn applies the file value ONLY where the CLI value
/// still equals the default (i.e. the user did not override it).
///
/// NOTE(s9): the spec floated `ArgMatches::value_source` as the alternative mechanism.
/// We implement the equal-to-default comparison instead because it needs no access to
/// the raw `ArgMatches` (which the derive API hides behind the typed struct) and is
/// trivially headless-testable: pass a `cli` differing from `defaults` and the CLI
/// wins; pass `cli == defaults` and the file wins. The precedence TABLE is identical
/// either way. The one edge it cannot see is "user explicitly typed the default value"
/// (then the file wins where the user meant the default) — an acceptable corner for
/// Phase 1, documented here and pinned by the precedence test.
pub fn merge_config(
    file: &ConfigFile,
    cli: &PipelineArgs,
    defaults: &PipelineArgs,
) -> PipelineArgs {
    PipelineArgs {
        instruments: if cli.instruments != defaults.instruments {
            cli.instruments
        } else {
            file.instruments.unwrap_or(defaults.instruments)
        },
        thickness: if (cli.thickness - defaults.thickness).abs() > f32::EPSILON {
            cli.thickness
        } else {
            file.thickness.unwrap_or(defaults.thickness)
        },
        steps: if cli.steps != defaults.steps {
            cli.steps
        } else {
            file.steps.unwrap_or(defaults.steps)
        },
        ms_per_step: if cli.ms_per_step != defaults.ms_per_step {
            cli.ms_per_step
        } else {
            file.ms_per_step.unwrap_or(defaults.ms_per_step)
        },
        jitter_percent: if (cli.jitter_percent - defaults.jitter_percent).abs() > f32::EPSILON {
            cli.jitter_percent
        } else {
            file.jitter_percent.unwrap_or(defaults.jitter_percent)
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn pipeline_defaults_match_today() {
        // The no-regression contract: the new CLI's defaults equal the old
        // parse_cli_arg defaults (4 / 0.10 / 40 / 250 / 15.0).
        let d = PipelineArgs::default();
        assert_eq!(d.instruments, 4);
        assert!((d.thickness - 0.10).abs() < 1e-6);
        assert_eq!(d.steps, 40);
        assert_eq!(d.ms_per_step, 250);
        assert!((d.jitter_percent - 15.0).abs() < 1e-6);
    }

    #[test]
    fn pipeline_to_engine_config_maps_fields() {
        let args = PipelineArgs {
            instruments: 6,
            thickness: 0.08,
            steps: 50,
            ms_per_step: 300,
            jitter_percent: 20.0,
        };
        let cfg = pipeline_to_engine_config(&args);
        assert_eq!(cfg.num_instruments, 6);
        assert_eq!(cfg.ms_per_step, 300);
        assert!((cfg.bar_thickness_frac - 0.08).abs() < 1e-6);
        assert_eq!(cfg.root_midi, 60);
    }

    #[test]
    fn cli_parses_play_with_image_positional() {
        // Resolves the old overloaded "play" token into a real subcommand + positional.
        let cli = Cli::try_parse_from(["audiohax", "play", "pic.png"]).expect("parse play");
        match cli.command {
            Command::Play(p) => {
                assert_eq!(p.image, Some(PathBuf::from("pic.png")));
                assert_eq!(p.pipeline.instruments, 4, "default instruments");
            }
            other => panic!("expected Play, got {:?}", other),
        }
    }

    #[test]
    fn cli_play_accepts_pipeline_flags() {
        let cli = Cli::try_parse_from([
            "audiohax",
            "play",
            "--instruments",
            "8",
            "--ms-per-step",
            "120",
        ])
        .expect("parse play flags");
        match cli.command {
            Command::Play(p) => {
                assert_eq!(p.pipeline.instruments, 8);
                assert_eq!(p.pipeline.ms_per_step, 120);
                assert_eq!(p.image, None, "image is optional");
            }
            other => panic!("expected Play, got {:?}", other),
        }
    }

    #[test]
    fn cli_rejects_non_numeric_instruments() {
        let r = Cli::try_parse_from(["audiohax", "play", "--instruments", "notanumber"]);
        assert!(r.is_err(), "non-numeric instrument count must be rejected");
    }

    // ── WS-4 S12: `play` output-selection flags ─────────────────────────────────

    #[test]
    fn play_output_defaults_to_synth() {
        // No-flag behavior is preserved: the default sink is the in-process synth,
        // and the three new MIDI knobs are all absent/false.
        let cli = Cli::try_parse_from(["audiohax", "play", "pic.png"]).expect("parse play");
        match cli.command {
            Command::Play(p) => {
                assert_eq!(p.output, OutputSink::Synth, "default output is synth");
                assert_eq!(p.midi_port, None);
                assert!(!p.list_midi_ports);
                assert_eq!(p.midi_virtual, None, "no virtual port by default");
            }
            other => panic!("expected Play, got {:?}", other),
        }
    }

    #[test]
    fn play_output_midi_parses() {
        let cli = Cli::try_parse_from(["audiohax", "play", "--output", "midi"])
            .expect("parse --output midi");
        match cli.command {
            Command::Play(p) => assert_eq!(p.output, OutputSink::Midi),
            other => panic!("expected Play, got {:?}", other),
        }
    }

    #[test]
    fn play_output_synth_parses_explicitly() {
        let cli = Cli::try_parse_from(["audiohax", "play", "--output", "synth"])
            .expect("parse --output synth");
        match cli.command {
            Command::Play(p) => assert_eq!(p.output, OutputSink::Synth),
            other => panic!("expected Play, got {:?}", other),
        }
    }

    #[test]
    fn play_rejects_unknown_output_variant() {
        // The ValueEnum rejects anything that is not `synth`/`midi`.
        let r = Cli::try_parse_from(["audiohax", "play", "--output", "loud"]);
        assert!(r.is_err(), "unknown --output variant must be rejected");
    }

    #[test]
    fn play_list_midi_ports_flag_parses() {
        let cli = Cli::try_parse_from(["audiohax", "play", "--list-midi-ports"])
            .expect("parse list flag");
        match cli.command {
            Command::Play(p) => assert!(p.list_midi_ports),
            other => panic!("expected Play, got {:?}", other),
        }
    }

    #[test]
    fn play_midi_port_accepts_substring() {
        let cli = Cli::try_parse_from([
            "audiohax",
            "play",
            "--output",
            "midi",
            "--midi-port",
            "FLUID",
        ])
        .expect("parse --midi-port substring");
        match cli.command {
            Command::Play(p) => assert_eq!(p.midi_port.as_deref(), Some("FLUID")),
            other => panic!("expected Play, got {:?}", other),
        }
    }

    #[test]
    fn play_midi_port_accepts_numeric_index() {
        // clap carries the selector as an opaque String; the index-vs-substring
        // discrimination happens in MidiOut::open_selector, not in the grammar.
        let cli = Cli::try_parse_from(["audiohax", "play", "--output", "midi", "--midi-port", "2"])
            .expect("parse --midi-port index");
        match cli.command {
            Command::Play(p) => assert_eq!(p.midi_port.as_deref(), Some("2")),
            other => panic!("expected Play, got {:?}", other),
        }
    }

    #[test]
    fn play_midi_virtual_bare_uses_default_name() {
        // Bare `--midi-virtual` → Some("AudioHaxOut") via default_missing_value.
        let cli = Cli::try_parse_from(["audiohax", "play", "--midi-virtual"])
            .expect("parse bare --midi-virtual");
        match cli.command {
            Command::Play(p) => assert_eq!(p.midi_virtual.as_deref(), Some("AudioHaxOut")),
            other => panic!("expected Play, got {:?}", other),
        }
    }

    #[test]
    fn play_midi_virtual_valued_uses_given_name() {
        let cli = Cli::try_parse_from(["audiohax", "play", "--midi-virtual", "MyPort"])
            .expect("parse valued --midi-virtual");
        match cli.command {
            Command::Play(p) => assert_eq!(p.midi_virtual.as_deref(), Some("MyPort")),
            other => panic!("expected Play, got {:?}", other),
        }
    }

    #[test]
    fn output_sink_default_is_synth() {
        assert_eq!(OutputSink::default(), OutputSink::Synth);
    }

    #[test]
    fn cli_modem_encode_subcommand_parses() {
        let cli = Cli::try_parse_from([
            "audiohax",
            "modem",
            "encode",
            "out.wav",
            "in.png",
            "--compress",
            "--preset",
            "robust",
        ])
        .expect("parse modem encode");
        match cli.command {
            Command::Modem(ModemCommand::Encode(e)) => {
                assert_eq!(e.out_wav, PathBuf::from("out.wav"));
                assert_eq!(e.input, PathBuf::from("in.png"));
                assert!(e.compress);
                assert_eq!(e.preset, Some(ModemPreset::Robust));
            }
            other => panic!("expected modem encode, got {:?}", other),
        }
    }

    #[test]
    fn cli_modem_channel_sim_hyphenated_name() {
        let cli = Cli::try_parse_from([
            "audiohax",
            "modem",
            "channel-sim",
            "a.bin",
            "b.bin",
            "--mode",
            "acoustic",
            "--clock-ppm",
            "500",
        ])
        .expect("parse channel-sim");
        match cli.command {
            Command::Modem(ModemCommand::ChannelSim(c)) => {
                assert_eq!(c.mode, ChannelMode::Acoustic);
                assert!((c.clock_ppm - 500.0).abs() < 1e-9);
            }
            other => panic!("expected channel-sim, got {:?}", other),
        }
    }

    #[test]
    fn channel_sim_legacy_underscore_flip_prob_alias_works() {
        // The legacy underscore spelling MUST still parse (alias preservation, D5).
        let shim =
            ChannelSimCli::try_parse_from(["channel_sim", "a.bin", "b.bin", "--flip_prob", "0.01"])
                .expect("legacy --flip_prob alias");
        assert!((shim.args.flip_prob - 0.01).abs() < 1e-9);
        // …and the new hyphenated spelling parses too.
        let shim2 =
            ChannelSimCli::try_parse_from(["channel_sim", "a.bin", "b.bin", "--flip-prob", "0.02"])
                .expect("hyphenated --flip-prob");
        assert!((shim2.args.flip_prob - 0.02).abs() < 1e-9);
    }

    #[test]
    fn merge_config_precedence_flag_beats_file_beats_default() {
        let defaults = PipelineArgs::default();

        // (a) file present, CLI at default → FILE wins.
        let file = ConfigFile {
            instruments: Some(7),
            ms_per_step: Some(333),
            ..Default::default()
        };
        let cli_default = PipelineArgs::default();
        let merged = merge_config(&file, &cli_default, &defaults);
        assert_eq!(merged.instruments, 7, "file beats default");
        assert_eq!(merged.ms_per_step, 333, "file beats default");
        assert_eq!(merged.steps, defaults.steps, "no file/flag → default");

        // (b) CLI overrides → FLAG beats file.
        let cli_override = PipelineArgs {
            instruments: 9,
            ..PipelineArgs::default()
        };
        let merged2 = merge_config(&file, &cli_override, &defaults);
        assert_eq!(merged2.instruments, 9, "flag beats file");
        assert_eq!(
            merged2.ms_per_step, 333,
            "untouched flag → file value still wins"
        );
    }

    #[test]
    fn load_config_missing_file_is_default() {
        let cfg = load_config(Some(std::path::Path::new(
            "/nonexistent/audiohax-does-not-exist.toml",
        )))
        .expect("missing file is ok");
        assert_eq!(cfg, ConfigFile::default());
    }

    #[test]
    fn config_file_roundtrips_through_toml() {
        let cfg = ConfigFile {
            instruments: Some(5),
            thickness: Some(0.07),
            steps: Some(60),
            ms_per_step: Some(200),
            jitter_percent: Some(10.0),
            midi_port: Some("MyPort".to_string()),
        };
        let text = toml::to_string(&cfg).expect("serialize");
        let back: ConfigFile = toml::from_str(&text).expect("deserialize");
        assert_eq!(cfg, back);
    }
}

//! tests/cli_parse.rs — headless integration suite for the unified clap CLI
//! (design S9 §3.6 / §3.7 / §3.9). Covers: every subcommand parses to the
//! expected typed values; bad input returns a clap Err (never a panic); the
//! resolved `play` subcommand (the old overloaded "play" token is gone);
//! ALIAS PRESERVATION (D5 — legacy underscore flag spellings still parse); and
//! CONFIG PRECEDENCE (flags > config file > defaults). Pure-Rust, no OpenCV.
//!
//! File-disjoint from src/cli.rs's inline tests — exercises the public surface.

use audiohax::cli::{
    load_config, merge_config, parse_channel_sim, parse_make_packetized, parse_modem_decode,
    parse_modem_encode, pipeline_to_engine_config, ChannelMode, Cli, Command, ConfigFile,
    ModemCommand, ModemPreset, PipelineArgs,
};
use clap::Parser;
use std::path::PathBuf;

// Touch the standalone parse_* fns so their imports stay live + meaningful: each
// returns its Args type, proving the bin entry points are reachable from the lib.
// (They call ::parse() on real argv, so they are not invoked at runtime here; the
// reference below is a compile-time guarantee the symbols exist with the right
// signature.)
#[allow(dead_code)]
fn _bin_parsers_exist() {
    let _a: fn() -> _ = parse_modem_encode;
    let _b: fn() -> _ = parse_modem_decode;
    let _c: fn() -> _ = parse_channel_sim;
    let _d: fn() -> _ = parse_make_packetized;
}

// ─────────────────────────────────────────────────────────────────────────────
// C. Subcommand parsing — typed values
// ─────────────────────────────────────────────────────────────────────────────

/// play <IMAGE> parses to Command::Play with the positional image set and the
/// pipeline flags at their defaults — the overloaded legacy "play" token is now a
/// real subcommand with an optional positional.
#[test]
fn test_play_positional_image_resolves_overload() {
    let cli = Cli::try_parse_from(["audiohax", "play", "art.png"]).expect("parse play");
    match cli.command {
        Command::Play(p) => {
            assert_eq!(p.image, Some(PathBuf::from("art.png")));
            assert_eq!(p.pipeline.instruments, 4, "default instruments");
            assert_eq!(p.pipeline.ms_per_step, 250, "default ms-per-step");
            assert_eq!(p.midi_port, None);
        }
        other => panic!("expected Play, got {other:?}"),
    }
}

/// play with NO positional is valid (image optional → example image).
#[test]
fn test_play_image_is_optional() {
    let cli = Cli::try_parse_from(["audiohax", "play", "--instruments", "8"]).expect("parse");
    match cli.command {
        Command::Play(p) => {
            assert_eq!(p.image, None, "omitted image is None (example image)");
            assert_eq!(p.pipeline.instruments, 8);
        }
        other => panic!("expected Play, got {other:?}"),
    }
}

/// render and analyze parse to their own arms with the shared pipeline flags.
#[test]
fn test_render_and_analyze_subcommands() {
    let r = Cli::try_parse_from(["audiohax", "render", "x.jpg", "--steps", "64"]).expect("render");
    match r.command {
        Command::Render(a) => {
            assert_eq!(a.image, Some(PathBuf::from("x.jpg")));
            assert_eq!(a.pipeline.steps, 64);
        }
        other => panic!("expected Render, got {other:?}"),
    }
    let a = Cli::try_parse_from(["audiohax", "analyze", "--thickness", "0.25"]).expect("analyze");
    match a.command {
        Command::Analyze(an) => {
            assert!((an.pipeline.thickness - 0.25).abs() < 1e-6);
            assert_eq!(an.image, None);
        }
        other => panic!("expected Analyze, got {other:?}"),
    }
}

/// The global --json / --config flags attach at the top level for any subcommand.
#[test]
fn test_global_json_and_config_flags() {
    let cli = Cli::try_parse_from([
        "audiohax",
        "--json",
        "--config",
        "/tmp/cfg.toml",
        "analyze",
        "pic.png",
    ])
    .expect("parse globals");
    assert!(cli.json, "--json must set the global flag");
    assert_eq!(cli.config, Some(PathBuf::from("/tmp/cfg.toml")));
    assert!(matches!(cli.command, Command::Analyze(_)));
}

/// modem encode parses positionals + flags to the typed struct (preset, compress,
/// symbol-ms, sim-*).
#[test]
fn test_modem_encode_parses_typed() {
    let cli = Cli::try_parse_from([
        "audiohax",
        "modem",
        "encode",
        "out.wav",
        "msg.txt",
        "--compress",
        "--preset",
        "robust",
        "--symbol-ms",
        "12.5",
        "--sim-flip",
        "0.01",
    ])
    .expect("parse encode");
    match cli.command {
        Command::Modem(ModemCommand::Encode(e)) => {
            assert_eq!(e.out_wav, PathBuf::from("out.wav"));
            assert_eq!(e.input, PathBuf::from("msg.txt"));
            assert!(e.compress);
            assert_eq!(e.preset, Some(ModemPreset::Robust));
            assert_eq!(e.symbol_ms, Some(12.5));
            assert_eq!(e.sim_flip, Some(0.01));
        }
        other => panic!("expected modem encode, got {other:?}"),
    }
}

/// modem decode parses the WAV positional + optional out_basename + RS flags.
#[test]
fn test_modem_decode_parses_typed() {
    let cli = Cli::try_parse_from([
        "audiohax",
        "modem",
        "decode",
        "rx.wav",
        "recovered",
        "--rs-data",
        "8",
        "--rs-parity",
        "4",
    ])
    .expect("parse decode");
    match cli.command {
        Command::Modem(ModemCommand::Decode(d)) => {
            assert_eq!(d.in_wav, PathBuf::from("rx.wav"));
            assert_eq!(d.out_basename, Some("recovered".to_string()));
            assert_eq!(d.rs_data, Some(8));
            assert_eq!(d.rs_parity, Some(4));
        }
        other => panic!("expected modem decode, got {other:?}"),
    }
}

/// modem channel-sim (hyphenated subcommand name) parses mode + acoustic knobs.
#[test]
fn test_modem_channel_sim_parses_typed() {
    let cli = Cli::try_parse_from([
        "audiohax",
        "modem",
        "channel-sim",
        "in.bin",
        "out.bin",
        "--mode",
        "acoustic",
        "--clock-ppm",
        "500",
        "--freq-offset",
        "12.5",
    ])
    .expect("parse channel-sim");
    match cli.command {
        Command::Modem(ModemCommand::ChannelSim(c)) => {
            assert_eq!(c.in_bytes, PathBuf::from("in.bin"));
            assert_eq!(c.out_sim, PathBuf::from("out.bin"));
            assert_eq!(c.mode, ChannelMode::Acoustic);
            assert!((c.clock_ppm - 500.0).abs() < 1e-9);
            assert!((c.freq_offset - 12.5).abs() < 1e-9);
        }
        other => panic!("expected channel-sim, got {other:?}"),
    }
}

/// modem make-packetized (hyphenated) parses positionals + RS/packet defaults.
#[test]
fn test_modem_make_packetized_parses_typed() {
    let cli = Cli::try_parse_from([
        "audiohax",
        "modem",
        "make-packetized",
        "src.dat",
        "pkt.bin",
        "--compress",
        "--repeats",
        "5",
    ])
    .expect("parse make-packetized");
    match cli.command {
        Command::Modem(ModemCommand::MakePacketized(m)) => {
            assert_eq!(m.input, PathBuf::from("src.dat"));
            assert_eq!(m.out_packetized, PathBuf::from("pkt.bin"));
            assert!(m.compress);
            assert_eq!(m.repeats, 5);
            // Defaults survive.
            assert_eq!(m.rs_shard_size, 128, "default rs-shard-size");
            assert_eq!(m.pkt_size, 200, "default pkt-size");
        }
        other => panic!("expected make-packetized, got {other:?}"),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// C. Bad input → clap Err, not a panic
// ─────────────────────────────────────────────────────────────────────────────

/// Non-numeric --instruments is rejected as a clap error (no panic).
#[test]
fn test_non_numeric_instruments_errs() {
    let r = Cli::try_parse_from(["audiohax", "play", "--instruments", "lots"]);
    assert!(r.is_err(), "non-numeric instrument count must error");
}

/// Out-of-range numeric (negative for an unsigned field) errs, not panics.
#[test]
fn test_negative_unsigned_flag_errs() {
    let r = Cli::try_parse_from(["audiohax", "play", "--steps", "-5"]);
    assert!(r.is_err(), "negative value for usize --steps must error");
}

/// An unknown channel mode value is rejected by the ValueEnum.
#[test]
fn test_unknown_channel_mode_errs() {
    let r = Cli::try_parse_from([
        "audiohax",
        "modem",
        "channel-sim",
        "a",
        "b",
        "--mode",
        "telepathy",
    ]);
    assert!(r.is_err(), "unknown channel mode must error");
}

/// A missing required positional (encode needs out_wav AND input) errs cleanly.
#[test]
fn test_missing_required_positional_errs() {
    let r = Cli::try_parse_from(["audiohax", "modem", "encode", "only_one.wav"]);
    assert!(r.is_err(), "encode requires both out_wav and input");
}

/// An unknown subcommand errs (not a silent fallthrough).
#[test]
fn test_unknown_subcommand_errs() {
    let r = Cli::try_parse_from(["audiohax", "teleport", "x.png"]);
    assert!(r.is_err(), "unknown subcommand must error");
}

// ─────────────────────────────────────────────────────────────────────────────
// C. ALIAS PRESERVATION (D5) — legacy spellings still parse
// ─────────────────────────────────────────────────────────────────────────────

/// D5: channel-sim accepts BOTH the legacy underscore --flip_prob AND the new
/// hyphenated --flip-prob, reaching the SAME field with the same value. Routed
/// through the full top-level Cli so the alias survives on the real surface.
#[test]
fn test_channel_sim_flip_prob_underscore_and_hyphen_aliases() {
    let underscore = Cli::try_parse_from([
        "audiohax",
        "modem",
        "channel-sim",
        "a.bin",
        "b.bin",
        "--flip_prob",
        "0.03",
    ])
    .expect("legacy underscore --flip_prob must parse");
    let hyphen = Cli::try_parse_from([
        "audiohax",
        "modem",
        "channel-sim",
        "a.bin",
        "b.bin",
        "--flip-prob",
        "0.03",
    ])
    .expect("new hyphenated --flip-prob must parse");

    let val = |c: Cli| match c.command {
        Command::Modem(ModemCommand::ChannelSim(cs)) => cs.flip_prob,
        _ => panic!("expected channel-sim"),
    };
    let u = val(underscore);
    let h = val(hyphen);
    assert!((u - 0.03).abs() < 1e-9, "underscore alias value");
    assert!(
        (u - h).abs() < 1e-12,
        "both spellings reach the same field/value"
    );
}

/// D5: the other legacy underscore channel-sim aliases (burst_prob / burst_len /
/// packet_size) also still parse to their canonical fields.
#[test]
fn test_channel_sim_other_underscore_aliases() {
    let cli = Cli::try_parse_from([
        "audiohax",
        "modem",
        "channel-sim",
        "a.bin",
        "b.bin",
        "--burst_prob",
        "0.2",
        "--burst_len",
        "32",
        "--packet_size",
        "256",
    ])
    .expect("legacy underscore aliases must parse");
    match cli.command {
        Command::Modem(ModemCommand::ChannelSim(c)) => {
            assert!((c.burst_prob - 0.2).abs() < 1e-9);
            assert_eq!(c.burst_len, 32);
            assert_eq!(c.packet_size, 256);
        }
        other => panic!("expected channel-sim, got {other:?}"),
    }
}

/// D5: encode still accepts --interleave (legacy compatibility flag), and
/// --no-interleave, distinctly.
#[test]
fn test_encode_interleave_flags_accepted() {
    let cli = Cli::try_parse_from([
        "audiohax",
        "modem",
        "encode",
        "o.wav",
        "i.txt",
        "--interleave",
    ])
    .expect("--interleave must still parse on encode");
    match cli.command {
        Command::Modem(ModemCommand::Encode(e)) => {
            assert!(e.interleave, "--interleave sets the flag");
            assert!(!e.no_interleave, "--no-interleave not given");
        }
        other => panic!("expected encode, got {other:?}"),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// C. pipeline_to_engine_config field mapping
// ─────────────────────────────────────────────────────────────────────────────

/// pipeline_to_engine_config maps PipelineArgs → EngineConfig field-for-field.
#[test]
fn test_pipeline_to_engine_config_field_mapping() {
    let args = PipelineArgs {
        instruments: 7,
        thickness: 0.13,
        steps: 33,
        ms_per_step: 175,
        jitter_percent: 9.0,
    };
    let cfg = pipeline_to_engine_config(&args);
    assert_eq!(cfg.num_instruments, 7, "instruments → num_instruments");
    assert_eq!(cfg.ms_per_step, 175, "ms-per-step → ms_per_step");
    assert!(
        (cfg.bar_thickness_frac - 0.13).abs() < 1e-6,
        "thickness → bar_thickness_frac"
    );
    assert_eq!(cfg.root_midi, 60, "root_midi fixed at 60 in Phase 1");
}

/// The default PipelineArgs maps to the default EngineConfig (no-regression).
#[test]
fn test_default_pipeline_maps_to_default_engine_config() {
    let cfg = pipeline_to_engine_config(&PipelineArgs::default());
    assert_eq!(cfg.num_instruments, 4);
    assert_eq!(cfg.ms_per_step, 250);
    assert!((cfg.bar_thickness_frac - 0.10).abs() < 1e-6);
}

// ─────────────────────────────────────────────────────────────────────────────
// C. CONFIG PRECEDENCE (flag > file > default) + load_config
// ─────────────────────────────────────────────────────────────────────────────

/// Precedence cell (a): a file value with the CLI at default → the FILE wins.
#[test]
fn test_precedence_file_beats_default() {
    let defaults = PipelineArgs::default();
    let file = ConfigFile {
        instruments: Some(7),
        ms_per_step: Some(333),
        ..Default::default()
    };
    let merged = merge_config(&file, &PipelineArgs::default(), &defaults);
    assert_eq!(merged.instruments, 7, "file value wins over default");
    assert_eq!(merged.ms_per_step, 333, "file value wins over default");
    // A field neither file nor flag set falls through to the default.
    assert_eq!(merged.steps, defaults.steps, "unset everywhere → default");
}

/// Precedence cell (b): a file value AND an explicit CLI flag → the FLAG wins,
/// while a flag left untouched still takes the file value.
#[test]
fn test_precedence_flag_beats_file() {
    let defaults = PipelineArgs::default();
    let file = ConfigFile {
        instruments: Some(7),
        ms_per_step: Some(333),
        ..Default::default()
    };
    let cli = PipelineArgs {
        instruments: 9, // user explicitly overrode instruments
        ..PipelineArgs::default()
    };
    let merged = merge_config(&file, &cli, &defaults);
    assert_eq!(merged.instruments, 9, "explicit flag wins over file");
    assert_eq!(
        merged.ms_per_step, 333,
        "untouched flag still yields the file value"
    );
}

/// Precedence cell (c): neither file nor flag set a field → the DEFAULT applies.
#[test]
fn test_precedence_default_when_neither_set() {
    let defaults = PipelineArgs::default();
    let empty = ConfigFile::default();
    let merged = merge_config(&empty, &PipelineArgs::default(), &defaults);
    assert_eq!(
        merged, defaults,
        "empty file + default flags → all defaults"
    );
}

/// load_config on a missing path returns Ok(default) — config is optional.
#[test]
fn test_load_config_missing_path_is_ok_default() {
    let cfg = load_config(Some(std::path::Path::new(
        "/nonexistent/audiohax-cli-parse-test-missing.toml",
    )))
    .expect("a missing config file must be Ok(default), not an error");
    assert_eq!(cfg, ConfigFile::default());
}

/// load_config reads and parses a real TOML file end-to-end, then merge_config
/// applies it with file-beats-default precedence. Uses a unique temp path under
/// the system temp dir and cleans it up (no hardcoded path, no repo writes).
#[test]
fn test_load_config_reads_toml_and_merges() {
    let dir = std::env::temp_dir();
    let path = dir.join(format!(
        "audiohax-cli-parse-{}-{}.toml",
        std::process::id(),
        // a per-test salt so parallel test bins don't collide
        line!()
    ));
    std::fs::write(
        &path,
        "instruments = 6\nms_per_step = 180\njitter_percent = 5.0\n",
    )
    .expect("write temp config");

    let loaded = load_config(Some(&path)).expect("parse temp config");
    assert_eq!(loaded.instruments, Some(6));
    assert_eq!(loaded.ms_per_step, Some(180));
    assert_eq!(loaded.jitter_percent, Some(5.0));

    let defaults = PipelineArgs::default();
    let merged = merge_config(&loaded, &PipelineArgs::default(), &defaults);
    assert_eq!(merged.instruments, 6, "loaded file value applied");
    assert_eq!(merged.ms_per_step, 180);
    assert!((merged.jitter_percent - 5.0).abs() < 1e-6);
    // A field absent from the file falls back to the default.
    assert_eq!(merged.steps, defaults.steps);

    let _ = std::fs::remove_file(&path);
}

/// A malformed TOML file surfaces a ConfigError (parse error), not a panic.
#[test]
fn test_load_config_malformed_toml_errs() {
    let dir = std::env::temp_dir();
    let path = dir.join(format!(
        "audiohax-cli-parse-bad-{}-{}.toml",
        std::process::id(),
        line!()
    ));
    std::fs::write(&path, "instruments = = = not toml").expect("write bad toml");
    let r = load_config(Some(&path));
    assert!(r.is_err(), "malformed TOML must return Err(ConfigError)");
    let _ = std::fs::remove_file(&path);
}

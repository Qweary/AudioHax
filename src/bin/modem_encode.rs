// src/bin/modem_encode.rs
//
// S9 (WS-4 Phase 1): the hand-rolled `print_usage` + `while i < args.len()` parser was
// replaced by the shared clap grammar in `audiohax::cli` (one coherent, validated,
// `--help`-bearing CLI across all four modem bins). Legacy flag spellings are
// preserved (the shared struct uses the same long names). The bin keeps its name and
// entry point and runs the SAME `modem::*` library logic unchanged below.
use std::fs::{self, File};
use std::io::Write;

use audiohax::cli::{parse_modem_encode, ModemPreset};
use audiohax::modem::{self, ModemParams};
use hound;

fn apply_preset(p: &mut ModemParams, preset: &str, chosen: &mut PresetParams) {
    match preset {
        "fast" => {
            p.channels = 2;
            p.m_tones = 16;
            p.symbol_ms = 20.0;
            chosen.pkt_size = Some(400);
            chosen.repeats = Some(2);
            chosen.rs = None;
        }
        "balanced" => {
            p.channels = 2;
            p.m_tones = 8;
            p.symbol_ms = 40.0;
            chosen.pkt_size = Some(200);
            chosen.repeats = Some(4);
            chosen.rs = Some((6usize, 3usize, 128usize));
        }
        "robust" => {
            p.channels = 2;
            p.m_tones = 8;
            p.symbol_ms = 50.0;
            chosen.pkt_size = Some(128);
            chosen.repeats = Some(4);
            chosen.rs = Some((4usize, 5usize, 128usize));
        }
        _ => {}
    }
}

#[derive(Default)]
struct PresetParams {
    pkt_size: Option<usize>,
    repeats: Option<usize>,
    rs: Option<(usize, usize, usize)>, // data, parity, shard_size
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Shared clap grammar (S9 §3.7) — replaces the hand-rolled positional parser.
    let cli = parse_modem_encode();
    let out_wav = cli.out_wav.to_string_lossy().to_string();
    let out_wav = out_wav.as_str();
    let input_path = cli.input.clone();

    // basic options (may be overridden by preset or explicit flags)
    let compress = cli.compress;
    let encrypt_key_hex: Option<String> = cli.encrypt.clone();

    // preset: map the typed enum back to the legacy string apply_preset() expects so
    // the downstream preset/RS-finalization logic stays byte-for-byte unchanged.
    let preset: Option<String> = cli.preset.map(|p| match p {
        ModemPreset::Fast => "fast".to_string(),
        ModemPreset::Balanced => "balanced".to_string(),
        ModemPreset::Robust => "robust".to_string(),
    });

    // parse-first-phase: collect flags into variables (same shapes as before)
    let channels_override: Option<usize> = cli.channels;
    let symbol_ms_override: Option<f32> = cli.symbol_ms;
    let mtones_override: Option<usize> = cli.mtones;
    let pkt_size_arg: Option<usize> = cli.pkt_size;
    let repeats_arg: Option<usize> = cli.repeats;

    // RS params
    let mut rs_data_shards: Option<usize> = cli.rs_data;
    let mut rs_parity_shards: Option<usize> = cli.rs_parity;
    let mut rs_shard_size: Option<usize> = cli.rs_shard_size;

    // interleave control (default true; --no-interleave disables)
    let interleave_enabled = !cli.no_interleave;

    // estimate flag
    let estimate_only = cli.estimate_duration;

    // simulator flags
    let simulate = cli.simulate;
    let sim_flip_prob: f64 = cli.sim_flip.unwrap_or(0.0);
    let sim_burst_prob: f64 = cli.sim_burst_prob.unwrap_or(0.0);
    let sim_burst_len: usize = cli.sim_burst_len.unwrap_or(64);
    let sim_out: Option<String> = cli.sim_out.map(|p| p.to_string_lossy().to_string());

    // read file
    let payload = fs::read(&input_path)?;
    let filename = input_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("payload");

    // build frame (header + payload)
    let frame = modem::build_frame(filename, &payload, compress, encrypt_key_hex.as_deref())?;

    println!(
        "Frame built: {} bytes (payload {})",
        frame.len(),
        payload.len()
    );

    // symbol parameters
    let mut params = ModemParams::default();

    // preset handling: allow preset to set many defaults, then explicit overrides apply
    let mut preset_params = PresetParams::default();
    if let Some(ref pstr) = preset {
        apply_preset(&mut params, pstr.as_str(), &mut preset_params);
        println!("Applied preset '{}'", pstr);
    }

    if let Some(c) = channels_override {
        params.channels = c;
    }
    if let Some(ms) = symbol_ms_override {
        params.symbol_ms = ms;
    }
    if let Some(m) = mtones_override {
        params.m_tones = m;
    }

    // final pkt_size & repeats decision (preset values are treated as defaults)
    let pkt_size = pkt_size_arg.or(preset_params.pkt_size).unwrap_or(200);
    let repeats = repeats_arg.or(preset_params.repeats).unwrap_or(3);

    // finalize RS params: explicit flags override preset
    if let Some(d) = rs_data_shards {
        rs_data_shards = Some(d);
    }
    if let Some(p) = rs_parity_shards {
        rs_parity_shards = Some(p);
    }
    if rs_shard_size.is_none() {
        rs_shard_size = preset_params.rs.map(|t| t.2);
    }
    if rs_data_shards.is_none() && preset_params.rs.is_some() {
        let (d, p, s) = preset_params.rs.unwrap();
        rs_data_shards = Some(d);
        rs_parity_shards = Some(p);
        rs_shard_size = Some(s);
    }

    // If estimate-only requested, compute estimate and exit
    if estimate_only {
        if let (Some(d), Some(p), Some(s)) = (rs_data_shards, rs_parity_shards, rs_shard_size) {
            let est = modem::estimate_duration_seconds(
                frame.len(),
                d,
                p,
                s,
                params.m_tones,
                params.channels,
                params.symbol_ms as usize,
                params.preamble_symbols.len() * params.preamble_repeats,
            );
            println!("Estimated encoded bytes: {}", est.encoded_bytes);
            println!("Estimated symbols total: {}", est.symbols_total);
            let secs = est.seconds;
            let hours = (secs / 3600.0).floor() as u64;
            let mins = ((secs % 3600.0) / 60.0).floor() as u64;
            let srem = (secs % 60.0).round() as u64;
            println!(
                "Estimated duration: {:.2} s ({}:{:02}:{:02})",
                secs, hours, mins, srem
            );
        } else {
            // rough estimate for repetition-based packetization
            let pkt_payload = pkt_size;
            let repeats_used = repeats;
            let mut enc_bytes = 0usize;
            let mut offset = 0usize;
            while offset < frame.len() {
                let end = std::cmp::min(offset + pkt_payload, frame.len());
                let payload_len = end - offset;
                let hdr = 4 + 4 + 2 + 4; // PKT1 header
                enc_bytes += (hdr + payload_len) * repeats_used;
                offset = end;
            }
            let bps = modem::bits_per_symbol(params.m_tones);
            let bits_per_symbol = if bps == 0 { 1 } else { bps };
            let symbols_payload = (enc_bytes * 8 + bits_per_symbol - 1) / bits_per_symbol;
            let symbols_total = symbols_payload
                + params.preamble_symbols.len() * params.preamble_repeats * params.channels;
            let samples_per_symbol = (params.sample_rate * params.symbol_ms as usize) / 1000;
            let total_samples = symbols_total * samples_per_symbol;
            let secs = (total_samples as f64) / (params.sample_rate as f64);
            println!("Estimated encoded bytes: {}", enc_bytes);
            println!("Estimated symbols total: {}", symbols_total);
            let hours = (secs / 3600.0).floor() as u64;
            let mins = ((secs % 3600.0) / 60.0).floor() as u64;
            let srem = (secs % 60.0).round() as u64;
            println!(
                "Estimated duration (rough): {:.2} s ({}:{:02}:{:02})",
                secs, hours, mins, srem
            );
        }
        return Ok(());
    }

    // choose packetization: RS if options provided, otherwise repeats
    let mut packetized_bytes: Vec<u8> = if let (Some(d), Some(p), Some(s)) =
        (rs_data_shards, rs_parity_shards, rs_shard_size)
    {
        if interleave_enabled {
            println!("Using Reed-Solomon FEC (interleaved): data_shards={} parity_shards={} shard_size={}", d, p, s);
            modem::packetize_stream_rs_interleaved(&frame, d, p, s)
        } else {
            println!(
                "Using Reed-Solomon FEC: data_shards={} parity_shards={} shard_size={}",
                d, p, s
            );
            modem::packetize_stream_rs(&frame, s, d, p)?
        }
    } else {
        println!(
            "Packetizing frame: pkt_size={} repeats={}",
            pkt_size, repeats
        );
        modem::packetize_stream(&frame, pkt_size, repeats)
    };

    // optionally simulate a channel (operates on packetized bytes)
    if simulate {
        println!(
            "Simulating channel: flip_prob={} burst_prob={} burst_len={}",
            sim_flip_prob, sim_burst_prob, sim_burst_len
        );
        let sim_bytes = modem::simulate_channel_bytes(
            &packetized_bytes,
            sim_flip_prob,
            sim_burst_prob,
            sim_burst_len,
        );
        if let Some(path) = sim_out {
            let mut f = File::create(path)?;
            f.write_all(&sim_bytes)?;
            println!("Wrote simulated packet-bytes to file");
        }
        packetized_bytes = sim_bytes;
    }

    // symbolization -> symbols
    let symbols = modem::bytes_to_symbols(&packetized_bytes, params.m_tones);
    println!("Symbols total (before preamble): {}", symbols.len());

    // split into channels round-robin
    let mut channels_syms = modem::split_round_robin(&symbols, params.channels);

    // --- PREAMBLE: prepend per-channel preamble symbols to each channel's stream
    if params.preamble_symbols.len() > 0 && params.preamble_repeats > 0 {
        let mut pre_vec: Vec<u8> =
            Vec::with_capacity(params.preamble_symbols.len() * params.preamble_repeats);
        for _ in 0..params.preamble_repeats {
            pre_vec.extend_from_slice(&params.preamble_symbols);
        }
        for ch_syms in channels_syms.iter_mut() {
            let mut newv = pre_vec.clone();
            newv.extend_from_slice(ch_syms);
            *ch_syms = newv;
        }
        println!(
            "Prepended preamble ({} symbols per channel)",
            params.preamble_symbols.len() * params.preamble_repeats
        );
    }

    // render to samples
    let samples_i16 = modem::render_symbols_to_samples(&channels_syms, &params);

    // write wav
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: params.sample_rate as u32,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(out_wav, spec)?;
    for s in samples_i16 {
        writer.write_sample(s)?;
    }
    writer.finalize()?;
    println!("Wrote {}", out_wav);
    Ok(())
}

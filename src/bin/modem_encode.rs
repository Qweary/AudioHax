// src/bin/modem_encode.rs
use std::env;
use std::fs::{self, File};
use std::io::Write;

use audiohax::modem::{self, ModemParams};
use hound;

fn print_usage(name: &str) {
    eprintln!("Usage:");
    eprintln!("  {} <out.wav> <input_file> [options]", name);
    eprintln!("Options:");
    eprintln!("  --compress");
    eprintln!("  --encrypt KEYHEX");
    eprintln!("  --channels N");
    eprintln!("  --symbol-ms MS");
    eprintln!("  --mtones M");
    eprintln!("  --pkt-size N");
    eprintln!("  --repeats R");
    eprintln!("  --rs-data D --rs-parity P --rs-shard-size S");
    eprintln!("  --preset <fast|balanced|robust>");
    eprintln!("  --no-interleave    (disable RS interleaving; default is interleaved)");
    eprintln!("  --estimate-duration   (print estimate and exit)");
    eprintln!("  --simulate           (run simple channel simulator on packet bytes)");
    eprintln!("  --sim-flip P         (byte flip prob, 0.0..1.0 default 0.0)");
    eprintln!("  --sim-burst-prob P   (prob to start a burst erase at a byte, default 0.0)");
    eprintln!("  --sim-burst-len N    (average burst length in bytes, default 64)");
    eprintln!("  --sim-out PATH       (write simulated packet-bytes to file)");
    eprintln!();
    eprintln!("Example:");
    eprintln!("  {} out.wav myimage.png --compress --preset robust --rs-data 4 --rs-parity 5 --rs-shard-size 128", name);
}

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
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        print_usage(&args[0]);
        std::process::exit(1);
    }
    let out_wav = &args[1];
    let input_path = &args[2];

    // basic options (may be overridden by preset or explicit flags)
    let mut compress = false;
    let mut encrypt_key_hex: Option<String> = None;

    let mut preset: Option<String> = None;

    // parse-first-phase: collect flags into variables
    let mut channels_override: Option<usize> = None;
    let mut symbol_ms_override: Option<f32> = None;
    let mut mtones_override: Option<usize> = None;
    let mut pkt_size_arg: Option<usize> = None;
    let mut repeats_arg: Option<usize> = None;

    // RS params
    let mut rs_data_shards: Option<usize> = None;
    let mut rs_parity_shards: Option<usize> = None;
    let mut rs_shard_size: Option<usize> = None;

    // interleave control (default true)
    let mut interleave_enabled = true;

    // estimate flag
    let mut estimate_only = false;

    // simulator flags
    let mut simulate = false;
    let mut sim_flip_prob: f64 = 0.0;
    let mut sim_burst_prob: f64 = 0.0;
    let mut sim_burst_len: usize = 64;
    let mut sim_out: Option<String> = None;

    // low-level parse
    let mut i = 3usize;
    while i < args.len() {
        match args[i].as_str() {
            "--compress" => { compress = true; i += 1; }
            "--encrypt" => {
                if i + 1 >= args.len() { eprintln!("Missing key for --encrypt"); std::process::exit(1); }
                encrypt_key_hex = Some(args[i+1].clone());
                i += 2;
            }
            "--channels" => {
                channels_override = args.get(i+1).and_then(|s| s.parse().ok());
                i += 2;
            }
            "--symbol-ms" => {
                symbol_ms_override = args.get(i+1).and_then(|s| s.parse().ok());
                i += 2;
            }
            "--mtones" => {
                mtones_override = args.get(i+1).and_then(|s| s.parse().ok());
                i += 2;
            }
            "--pkt-size" => {
                pkt_size_arg = args.get(i+1).and_then(|s| s.parse().ok());
                i += 2;
            }
            "--repeats" => {
                repeats_arg = args.get(i+1).and_then(|s| s.parse().ok());
                i += 2;
            }
            "--rs-data" => {
                rs_data_shards = args.get(i+1).and_then(|s| s.parse().ok());
                i += 2;
            }
            "--rs-parity" => {
                rs_parity_shards = args.get(i+1).and_then(|s| s.parse().ok());
                i += 2;
            }
            "--rs-shard-size" => {
                rs_shard_size = args.get(i+1).and_then(|s| s.parse().ok());
                i += 2;
            }
            "--preset" => {
                if let Some(pv) = args.get(i+1) { preset = Some(pv.clone()); }
                i += 2;
            }
            "--no-interleave" => { interleave_enabled = false; i += 1; }
            "--interleave" => { interleave_enabled = true; i += 1; }
            "--estimate-duration" => { estimate_only = true; i += 1; }
            "--simulate" => { simulate = true; i += 1; }
            "--sim-flip" => {
                if let Some(v) = args.get(i+1) { sim_flip_prob = v.parse().unwrap_or(0.0); }
                i += 2;
            }
            "--sim-burst-prob" => {
                if let Some(v) = args.get(i+1) { sim_burst_prob = v.parse().unwrap_or(0.0); }
                i += 2;
            }
            "--sim-burst-len" => {
                if let Some(v) = args.get(i+1) { sim_burst_len = v.parse().unwrap_or(64); }
                i += 2;
            }
            "--sim-out" => {
                if let Some(v) = args.get(i+1) { sim_out = Some(v.clone()); }
                i += 2;
            }
            _ => { eprintln!("Unknown arg {}", args[i]); i += 1; }
        }
    }

    // read file
    let payload = fs::read(input_path)?;
    let filename = std::path::Path::new(input_path).file_name().and_then(|s| s.to_str()).unwrap_or("payload");

    // build frame (header + payload)
    let frame = modem::build_frame(
        filename,
        &payload,
        compress,
        encrypt_key_hex.as_deref(),
    )?;

    println!("Frame built: {} bytes (payload {})", frame.len(), payload.len());

    // symbol parameters
    let mut params = ModemParams::default();

    // preset handling: allow preset to set many defaults, then explicit overrides apply
    let mut preset_params = PresetParams::default();
    if let Some(ref pstr) = preset {
        apply_preset(&mut params, pstr.as_str(), &mut preset_params);
        println!("Applied preset '{}'", pstr);
    }

    if let Some(c) = channels_override { params.channels = c; }
    if let Some(ms) = symbol_ms_override { params.symbol_ms = ms; }
    if let Some(m) = mtones_override { params.m_tones = m; }

    // final pkt_size & repeats decision (preset values are treated as defaults)
    let pkt_size = pkt_size_arg.or(preset_params.pkt_size).unwrap_or(200);
    let repeats = repeats_arg.or(preset_params.repeats).unwrap_or(3);

    // finalize RS params: explicit flags override preset
    if let Some(d) = rs_data_shards { rs_data_shards = Some(d); }
    if let Some(p) = rs_parity_shards { rs_parity_shards = Some(p); }
    if rs_shard_size.is_none() { rs_shard_size = preset_params.rs.map(|t| t.2); }
    if rs_data_shards.is_none() && preset_params.rs.is_some() {
        let (d,p,s) = preset_params.rs.unwrap();
        rs_data_shards = Some(d);
        rs_parity_shards = Some(p);
        rs_shard_size = Some(s);
    }

    // If estimate-only requested, compute estimate and exit
    if estimate_only {
        if let (Some(d), Some(p), Some(s)) = (rs_data_shards, rs_parity_shards, rs_shard_size) {
            let est = modem::estimate_duration_seconds(
                frame.len(),
                d, p, s,
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
            println!("Estimated duration: {:.2} s ({}:{:02}:{:02})", secs, hours, mins, srem);
        } else {
            // rough estimate for repetition-based packetization
            let pkt_payload = pkt_size;
            let repeats_used = repeats;
            let mut enc_bytes = 0usize;
            let mut seq = 0usize;
            let mut offset = 0usize;
            while offset < frame.len() {
                let end = std::cmp::min(offset + pkt_payload, frame.len());
                let payload_len = end - offset;
                let hdr = 4 + 4 + 2 + 4; // PKT1 header
                enc_bytes += (hdr + payload_len) * repeats_used;
                seq += 1;
                offset = end;
            }
            let bps = modem::bits_per_symbol(params.m_tones);
            let bits_per_symbol = if bps == 0 { 1 } else { bps };
            let symbols_payload = (enc_bytes * 8 + bits_per_symbol - 1) / bits_per_symbol;
            let symbols_total = symbols_payload + params.preamble_symbols.len() * params.preamble_repeats * params.channels;
            let samples_per_symbol = (params.sample_rate * params.symbol_ms as usize) / 1000;
            let total_samples = symbols_total * samples_per_symbol;
            let secs = (total_samples as f64) / (params.sample_rate as f64);
            println!("Estimated encoded bytes: {}", enc_bytes);
            println!("Estimated symbols total: {}", symbols_total);
            let hours = (secs / 3600.0).floor() as u64;
            let mins = ((secs % 3600.0) / 60.0).floor() as u64;
            let srem = (secs % 60.0).round() as u64;
            println!("Estimated duration (rough): {:.2} s ({}:{:02}:{:02})", secs, hours, mins, srem);
        }
        return Ok(());
    }

    // choose packetization: RS if options provided, otherwise repeats
    let mut packetized_bytes: Vec<u8> = if let (Some(d), Some(p), Some(s)) = (rs_data_shards, rs_parity_shards, rs_shard_size) {
        if interleave_enabled {
            println!("Using Reed-Solomon FEC (interleaved): data_shards={} parity_shards={} shard_size={}", d, p, s);
            modem::packetize_stream_rs_interleaved(&frame, d, p, s)
        } else {
            println!("Using Reed-Solomon FEC: data_shards={} parity_shards={} shard_size={}", d, p, s);
            modem::packetize_stream_rs(&frame, s, d, p)?
        }
    } else {
        println!("Packetizing frame: pkt_size={} repeats={}", pkt_size, repeats);
        modem::packetize_stream(&frame, pkt_size, repeats)
    };

    // optionally simulate a channel (operates on packetized bytes)
    if simulate {
        println!("Simulating channel: flip_prob={} burst_prob={} burst_len={}", sim_flip_prob, sim_burst_prob, sim_burst_len);
        let sim_bytes = modem::simulate_channel_bytes(&packetized_bytes, sim_flip_prob, sim_burst_prob, sim_burst_len);
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
        let mut pre_vec: Vec<u8> = Vec::with_capacity(params.preamble_symbols.len() * params.preamble_repeats);
        for _ in 0..params.preamble_repeats {
            pre_vec.extend_from_slice(&params.preamble_symbols);
        }
        for ch_syms in channels_syms.iter_mut() {
            let mut newv = pre_vec.clone();
            newv.extend_from_slice(ch_syms);
            *ch_syms = newv;
        }
        println!("Prepended preamble ({} symbols per channel)", params.preamble_symbols.len() * params.preamble_repeats);
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

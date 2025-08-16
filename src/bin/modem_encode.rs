// src/bin/modem_encode.rs
use std::env;
use std::fs;

use audiohax::modem::{self, ModemParams};
use hound;

fn print_usage(name: &str) {
    eprintln!("Usage:");
    eprintln!("  {} <out.wav> <input_file> [--compress] [--encrypt KEYHEX] [--channels N] [--symbol-ms MS] [--mtones M] [--pkt-size N] [--repeats R] [--rs-data D --rs-parity P --rs-shard-size S] [--preset NAME] [--estimate-duration] [--no-interleave]", name);
    eprintln!("Example: {} out.wav myimage.png --compress --channels 4 --pkt-size 200 --repeats 3", name);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        print_usage(&args[0]);
        std::process::exit(1);
    }
    let out_wav = &args[1];
    let input_path = &args[2];

    // defaults
    let mut compress = false;
    let mut encrypt_key_hex: Option<String> = None;
    let mut channels_override: Option<usize> = None;
    let mut symbol_ms_override: Option<f32> = None;
    let mut mtones_override: Option<usize> = None;
    let mut pkt_size: Option<usize> = Some(200);
    let mut repeats: usize = 3;

    // Reed-Solomon options (optional)
    let mut rs_data_shards: Option<usize> = None;
    let mut rs_parity_shards: Option<usize> = None;
    let mut rs_shard_size: usize = 100; // default shard size for RS (overrides pkt_size if provided explicitly)

    // NEW flags
    let mut preset: Option<String> = None;
    let mut estimate_duration: bool = false;
    let mut no_interleave: bool = false;

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
                if let Some(v) = args.get(i+1) {
                    if let Ok(n) = v.parse::<usize>() { pkt_size = Some(n); }
                }
                i += 2;
            }
            "--repeats" => {
                if let Some(v) = args.get(i+1) {
                    if let Ok(n) = v.parse::<usize>() { repeats = n; }
                }
                i += 2;
            }
            "--rs-data" => {
                if let Some(v) = args.get(i+1) {
                    if let Ok(n) = v.parse::<usize>() { rs_data_shards = Some(n); }
                }
                i += 2;
            }
            "--rs-parity" => {
                if let Some(v) = args.get(i+1) {
                    if let Ok(n) = v.parse::<usize>() { rs_parity_shards = Some(n); }
                }
                i += 2;
            }
            "--rs-shard-size" => {
                if let Some(v) = args.get(i+1) {
                    if let Ok(n) = v.parse::<usize>() { rs_shard_size = n; }
                }
                i += 2;
            }
            // NEW flags
            "--preset" => {
                if let Some(v) = args.get(i+1) {
                    preset = Some(v.clone());
                }
                i += 2;
            }
            "--estimate-duration" => {
                estimate_duration = true;
                i += 1;
            }
            "--no-interleave" => {
                no_interleave = true;
                i += 1;
            }
            _ => { eprintln!("Unknown arg {}", args[i]); i += 1; }
        }
    }

    // read file
    let payload = fs::read(input_path)?;
    let filename = std::path::Path::new(input_path).file_name().and_then(|s| s.to_str()).unwrap_or("payload");

    // build frame (header + payload) — this applies compression/encryption choices.
    // We build the frame early so estimate can reflect actual post-compress size.
    let frame = modem::build_frame(
        filename,
        &payload,
        compress,
        encrypt_key_hex.as_deref(),
    )?;

    println!("Frame built: {} bytes (payload {})", frame.len(), payload.len());

    // symbol parameters: start from defaults
    let mut params = ModemParams::default();

    // We'll maintain local chosen values (preset -> overridden by explicit flags)
    let mut chosen_channels = params.channels;
    let mut chosen_mtones = params.m_tones;
    let mut chosen_symbol_ms_f32 = params.symbol_ms;
    let mut chosen_pkt_size = pkt_size; // Option<usize>
    let mut chosen_repeats = repeats;

    // Apply preset defaults if provided (only for values not explicitly passed)
    if let Some(p) = preset.as_ref().map(|s| s.as_str()) {
        match p {
            "robust" => {
                // The parameters that worked well in your earlier runs.
                if channels_override.is_none() { chosen_channels = 2; }
                if mtones_override.is_none() { chosen_mtones = 8; }
                if symbol_ms_override.is_none() { chosen_symbol_ms_f32 = 50.0; }
                if pkt_size.is_none() { chosen_pkt_size = None; } // rely on RS
                chosen_repeats = 1;
                if rs_data_shards.is_none() { rs_data_shards = Some(4); }
                if rs_parity_shards.is_none() { rs_parity_shards = Some(5); }
            }
            "medium" => {
                if channels_override.is_none() { chosen_channels = 2; }
                if mtones_override.is_none() { chosen_mtones = 8; }
                if symbol_ms_override.is_none() { chosen_symbol_ms_f32 = 40.0; }
                if pkt_size.is_none() { chosen_pkt_size = Some(200); }
                chosen_repeats = 4;
                if rs_data_shards.is_none() { rs_data_shards = Some(6); }
                if rs_parity_shards.is_none() { rs_parity_shards = Some(3); }
            }
            "fast" => {
                if channels_override.is_none() { chosen_channels = 2; }
                if mtones_override.is_none() { chosen_mtones = 16; }
                if symbol_ms_override.is_none() { chosen_symbol_ms_f32 = 20.0; }
                if pkt_size.is_none() { chosen_pkt_size = Some(400); }
                chosen_repeats = 2;
                // don't force RS on "fast"
            }
            other => {
                eprintln!("Unknown preset '{}', ignoring", other);
            }
        }
    }

    // Now apply explicit overrides (they take precedence)
    if let Some(c) = channels_override { chosen_channels = c; }
    if let Some(ms) = symbol_ms_override { chosen_symbol_ms_f32 = ms; }
    if let Some(m) = mtones_override { chosen_mtones = m; }
    if let Some(p) = pkt_size { chosen_pkt_size = Some(p); }
    chosen_repeats = repeats; // repeats variable already contains explicit value or default

    // apply to params used for rendering
    params.channels = chosen_channels;
    params.m_tones = chosen_mtones;
    params.symbol_ms = chosen_symbol_ms_f32;

    // If estimate-duration requested, compute and print then exit.
    if estimate_duration {
        // preamble symbols per channel: use definition from params
        let preamble_symbols_per_channel = params.preamble_symbols.len() * params.preamble_repeats;
        let rsd = rs_data_shards.unwrap_or(0);
        let rsp = rs_parity_shards.unwrap_or(0);

        let est = modem::estimate_duration_seconds(
            frame.len(), // use frame bytes (post compression/encrypt header+payload)
            rsd,
            rsp,
            rs_shard_size,
            params.m_tones,
            params.channels,
            params.symbol_ms as usize,
            preamble_symbols_per_channel,
        );

        let secs = est.seconds;
        let hh = (secs / 3600.0).floor() as u64;
        let mm = ((secs % 3600.0) / 60.0).floor() as u64;
        let ss = (secs % 60.0).round() as u64;

        println!("Estimated encoded bytes: {}", est.encoded_bytes);
        println!("Estimated symbols total: {}", est.symbols_total);
        println!("Estimated duration: {:.2} s ({}:{:02}:{:02})", secs, hh, mm, ss);
        return Ok(());
    }

    // choose packetization: RS if options provided, otherwise repeats
    let packetized_bytes = if let (Some(d), Some(p)) = (rs_data_shards, rs_parity_shards) {
        // If user explicitly requested no-interleave -> use original sequential emission
        if no_interleave {
            println!("Using Reed-Solomon FEC (sequential): data_shards={} parity_shards={} shard_size={}", d, p, rs_shard_size);
            // original function signature: packetize_stream_rs(data: &[u8], shard_size: usize, data_shards: usize, parity_shards: usize)
            modem::packetize_stream_rs(&frame, rs_shard_size, d, p)?
        } else {
            println!("Using Reed-Solomon FEC (interleaved): data_shards={} parity_shards={} shard_size={}", d, p, rs_shard_size);
            // interleaved variant signature: (payload, data_shards, parity_shards, shard_size) -> Vec<u8>
            modem::packetize_stream_rs_interleaved(&frame, d, p, rs_shard_size)
        }
    } else {
        // fall back to repeat-based
        let pkt_size_final = chosen_pkt_size.unwrap_or(200);
        println!("Packetizing frame: pkt_size={} repeats={}", pkt_size_final, chosen_repeats);
        modem::packetize_stream(&frame, pkt_size_final, chosen_repeats)
    };

    // symbolization -> symbols
    let symbols = modem::bytes_to_symbols(&packetized_bytes, params.m_tones);
    println!("Symbols total (before preamble): {}", symbols.len());

    // split into channels round-robin
    let mut channels_syms = modem::split_round_robin(&symbols, params.channels);

    // --- PREAMBLE: prepend per-channel preamble symbols to each channel's stream
    // This helps the decoder find symbol/frame boundaries quickly.
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
        println!("Prepended preamble ({} symbols per channel)", pre_vec.len());
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

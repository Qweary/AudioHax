// src/bin/modem_encode.rs
use std::env;
use std::fs;

use audiohax::modem::{self, ModemParams};
use hound;

fn print_usage(name: &str) {
    eprintln!("Usage:");
    eprintln!("  {} <out.wav> <input_file> [--compress] [--encrypt KEYHEX] [--channels N] [--symbol-ms MS] [--mtones M] [--pkt-size N] [--repeats R] [--rs-data D --rs-parity P --rs-shard-size S]", name);
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
    let mut pkt_size: usize = 200;
    let mut repeats: usize = 3;

    // Reed-Solomon options (optional)
    let mut rs_data_shards: Option<usize> = None;
    let mut rs_parity_shards: Option<usize> = None;
    let mut rs_shard_size: usize = 100; // default shard size for RS (overrides pkt_size if provided explicitly)

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
                    if let Ok(n) = v.parse::<usize>() { pkt_size = n; }
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
    if let Some(c) = channels_override { params.channels = c; }
    if let Some(ms) = symbol_ms_override { params.symbol_ms = ms; }
    if let Some(m) = mtones_override { params.m_tones = m; }

    // choose packetization: RS if options provided, otherwise repeats
    let mut packetized_bytes = if let (Some(d), Some(p)) = (rs_data_shards, rs_parity_shards) {
        println!("Using Reed-Solomon FEC: data_shards={} parity_shards={} shard_size={}", d, p, rs_shard_size);
        modem::packetize_stream_rs(&frame, rs_shard_size, d, p)?
    } else {
        println!("Packetizing frame: pkt_size={} repeats={}", pkt_size, repeats);
        modem::packetize_stream(&frame, pkt_size, repeats)
    };

    // symbolization -> symbols
    let mut symbols = modem::bytes_to_symbols(&packetized_bytes, params.m_tones);
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

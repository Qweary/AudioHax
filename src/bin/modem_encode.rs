// src/bin/modem_encode.rs
use std::env;
use std::fs;

use audiohax::modem::{self, ModemParams};
use hound;

fn print_usage(name: &str) {
    eprintln!("Usage:");
    eprintln!("  {} <out.wav> <input_file> [--compress] [--encrypt KEYHEX] [--channels N] [--symbol-ms MS] [--mtones M] [--pkt-size BYTES] [--repeats N]", name);
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
    let mut pkt_size_override: Option<usize> = None;
    let mut repeats_override: Option<usize> = None;

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
                pkt_size_override = args.get(i+1).and_then(|s| s.parse().ok());
                i += 2;
            }
            "--repeats" => {
                repeats_override = args.get(i+1).and_then(|s| s.parse().ok());
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

    println!("Encoder params: channels={}, m_tones={}, symbol_ms={}, sample_rate={}",
             params.channels, params.m_tones, params.symbol_ms, params.sample_rate);

    // packetize + repetition (simple FEC)
    let pkt_size = pkt_size_override.unwrap_or(200);
    let repeats = repeats_override.unwrap_or(3);
    println!("Packetizing frame: pkt_size={} repeats={}", pkt_size, repeats);
    let packetized = modem::packetize_stream(&frame, pkt_size, repeats);
    println!("Packetized bytes: {}", packetized.len());

    // symbolization -> symbols
    let mut symbols = modem::bytes_to_symbols(&packetized, params.m_tones);
    println!("Symbols total (before preamble): {}", symbols.len());

    // insert preamble (repeat pilot symbol pattern)
    let mut with_preamble: Vec<u8> = Vec::new();
    for _ in 0..params.preamble_repeats {
        for &s in &params.preamble_symbols {
            with_preamble.push(s);
        }
    }
    with_preamble.extend_from_slice(&symbols);
    symbols = with_preamble;
    println!("Symbols total (after preamble): {}", symbols.len());

    // split into channels round-robin
    let channels_syms = modem::split_round_robin(&symbols, params.channels);

    // render to samples
    let samples_i16 = modem::render_symbols_to_samples(&channels_syms, &params);

    // SAFETY CHECK: WAV data size cannot exceed u32::MAX bytes (WAV uses 32-bit chunk sizes).
    // For 16-bit mono, bytes_per_sample = 2.
    let bytes_per_sample = 2usize; // 16-bit
    let total_samples = samples_i16.len();
    let total_data_bytes = total_samples.checked_mul(bytes_per_sample).unwrap_or(usize::MAX);
    // hound/wav header uses u32 for data size; ensure it fits
    const U32_MAX_USIZE: usize = std::u32::MAX as usize;
    if total_data_bytes > U32_MAX_USIZE {
        // estimate duration
        let duration_secs = (total_samples as f64) / (params.sample_rate as f64);
        let hours = duration_secs / 3600.0;
        eprintln!("\nERROR: Generated WAV would be larger than 4GiB (WAV data chunk cannot be represented).");
        eprintln!("Generated samples: {}, bytes: {} (> 4GiB). Estimated duration: {:.2} seconds ({:.2} hours).", total_samples, total_data_bytes, duration_secs, hours);
        eprintln!("This usually means your transmission parameters produce an extremely long audio file (too many symbols).");
        eprintln!("Suggestions to reduce size:");
        eprintln!("  * Decrease --repeats (currently {})", repeats);
        eprintln!("  * Increase --pkt-size (currently {}) so fewer packet headers / repeats", pkt_size);
        eprintln!("  * Increase --mtones (more bits per symbol) or increase channels");
        eprintln!("  * Increase --symbol-ms to lower symbol rate or reduce samples-per-symbol");
        eprintln!("  * For large files, consider streaming/resumable approach or stronger FEC (Reed-Solomon) instead of raw repetition.");
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "WAV would exceed 4GiB, aborting to avoid crash")));
    }

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

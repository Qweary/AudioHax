// src/bin/modem_encode.rs
use std::env;
use std::fs;

use audiohax::modem::{self, ModemParams};
use hound;

fn print_usage(name: &str) {
    eprintln!("Usage:");
    eprintln!("  {} <out.wav> <input_file> [--compress] [--encrypt KEYHEX] [--channels N] [--symbol-ms MS] [--mtones M]", name);
    eprintln!("Example: {} out.wav myimage.png --compress --channels 4", name);
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

    // symbolization -> symbols
    let symbols = modem::bytes_to_symbols(&frame, params.m_tones);
    println!("Symbols total: {}", symbols.len());

    // split into channels round-robin
    let channels_syms = modem::split_round_robin(&symbols, params.channels);

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

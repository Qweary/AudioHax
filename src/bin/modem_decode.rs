// src/bin/modem_decode.rs
use std::env;
use std::fs::File;
use std::io::Write;

use audiohax::modem::{self, ModemParams};

fn detect_and_write_file(bytes: &[u8], out_basename: &str) -> std::io::Result<()> {
    // simple magic detection
    let mut ext = "bin";
    if bytes.len() >= 8 && &bytes[0..8] == [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A] {
        ext = "png";
    } else if bytes.len() >= 3 && bytes[0..3] == [0xFF, 0xD8, 0xFF] {
        ext = "jpg";
    } else if bytes.len() >= 6 && (bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a")) {
        ext = "gif";
    } else if bytes.len() >= 2 && &bytes[0..2] == [0x42, 0x4D] {
        ext = "bmp";
    }
    let out_path = format!("{}_recovered.{}", out_basename, ext);
    println!("Writing {} ({} bytes)", out_path, bytes.len());
    let mut f = File::create(&out_path)?;
    f.write_all(bytes)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <in.wav> [out_basename] [--decrypt KEYHEX]", args[0]);
        std::process::exit(1);
    }
    let in_wav = &args[1];
    let out_basename = args.get(2).map(|s| s.as_str()).unwrap_or("payload");
    let maybe_key = args.iter().position(|a| a == "--decrypt").and_then(|i| args.get(i+1)).map(|s| s.as_str());

    // params must match encoder
    let params = ModemParams::default();
    // You can extend to parse optional params like symbol-ms/mtones/channels from CLI

    let reader = hound::WavReader::open(in_wav)?;
    let samples_i16: Vec<i16> = reader.into_samples::<i16>().filter_map(Result::ok).collect();
    println!("Read {} samples", samples_i16.len());

    let samples_per_symbol = ((params.sample_rate as f32) * (params.symbol_ms / 1000.0)).round() as usize;

    // Build tone frequency map per channel
    let tone_freqs = modem::build_tone_frequencies(&params);

    // For each symbol window, compute energy across each channel's tones and select index of max energy per channel
    let mut detected_by_channel: Vec<Vec<u8>> = vec![Vec::new(); params.channels];

    for window_start in (0..samples_i16.len()).step_by(samples_per_symbol) {
        if window_start + samples_per_symbol > samples_i16.len() { break; }
        let slice = &samples_i16[window_start .. window_start + samples_per_symbol];
        for ch in 0..params.channels {
            let freqs = &tone_freqs[ch];
            let mut max_idx = 0usize;
            let mut max_val = 0f32;
            for (i, &f) in freqs.iter().enumerate() {
                let mag = modem::goertzel_mag_squared(slice, f, params.sample_rate);
                if mag > max_val {
                    max_val = mag;
                    max_idx = i;
                }
            }
            detected_by_channel[ch].push(max_idx as u8);
        }
    }

    // Now reinterleave round-robin into a single symbols stream
    let mut symbols: Vec<u8> = Vec::new();
    let max_len = detected_by_channel.iter().map(|v| v.len()).max().unwrap_or(0);
    for i in 0..max_len {
        for ch in 0..params.channels {
            if i < detected_by_channel[ch].len() {
                symbols.push(detected_by_channel[ch][i]);
            }
        }
    }

    println!("Detected {} symbols", symbols.len());

    // Convert symbols -> bytes
    let bytes = modem::symbols_to_bytes(&symbols, params.m_tones);
    println!("Recovered {} bytes from symbols", bytes.len());

    // Try to find frame header (we're tolerant: header may start at index 0)
    match modem::parse_frame_header(&bytes) {
        Ok((_fname, _compr, _enc, start, plen, _crc)) => {
            println!("Frame header parsed. payload start {} len {}", start, plen);
            let frame = bytes;
            match modem::extract_frame(&frame, maybe_key) {
                Ok((fname, recovered_bytes)) => {
                    println!("Recovered filename: {}", fname);
                    detect_and_write_file(&recovered_bytes, out_basename)?;
                }
                Err(e) => {
                    eprintln!("Failed to extract frame: {}", e);
                }
            }
        }
        Err(e) => {
            eprintln!("Could not parse frame header from recovered bytes: {}", e);
            // still try writing raw bytes
            detect_and_write_file(&bytes, out_basename)?;
        }
    }

    Ok(())
}

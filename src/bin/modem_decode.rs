// src/bin/modem_decode.rs
use std::env;
use std::fs::File;
use std::io::Write;

use audiohax::modem::{self, ModemParams};
use hound;

fn detect_and_write_file(bytes: &[u8], out_basename: &str) -> std::io::Result<()> {
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
        eprintln!("Usage: {} <in.wav> [out_basename] [--decrypt KEYHEX] [--channels N] [--mtones M] [--symbol-ms MS] [--repeats N]", args[0]);
        std::process::exit(1);
    }
    let in_wav = &args[1];
    let out_basename = args.get(2).map(|s| s.as_str()).unwrap_or("payload");
    let maybe_key = args.iter().position(|a| a == "--decrypt").and_then(|i| args.get(i+1)).map(|s| s.as_str());

    // Parse optional params
    let mut params = ModemParams::default();
    let mut expected_repeats: usize = 3;
    let mut i = 2usize;
    while i < args.len() {
        match args[i].as_str() {
            "--channels" => {
                if let Some(v) = args.get(i+1) {
                    if let Ok(c) = v.parse::<usize>() {
                        params.channels = c;
                    }
                }
                i += 2;
            }
            "--mtones" => {
                if let Some(v) = args.get(i+1) {
                    if let Ok(m) = v.parse::<usize>() {
                        params.m_tones = m;
                    }
                }
                i += 2;
            }
            "--symbol-ms" => {
                if let Some(v) = args.get(i+1) {
                    if let Ok(ms) = v.parse::<f32>() {
                        params.symbol_ms = ms;
                    }
                }
                i += 2;
            }
            "--repeats" => {
                if let Some(v) = args.get(i+1) {
                    if let Ok(r) = v.parse::<usize>() {
                        expected_repeats = r;
                    }
                }
                i += 2;
            }
            _ => { i += 1; }
        }
    }

    println!("Using params: channels={}, m_tones={}, symbol_ms={}, sample_rate={}, expected_repeats={}",
             params.channels, params.m_tones, params.symbol_ms, params.sample_rate, expected_repeats);

    let reader = hound::WavReader::open(in_wav)?;
    let samples_i16: Vec<i16> = reader.into_samples::<i16>().filter_map(Result::ok).collect();
    println!("Read {} samples", samples_i16.len());

    let samples_per_symbol = ((params.sample_rate as f32) * (params.symbol_ms / 1000.0)).round() as usize;
    println!("Samples per symbol (computed) = {}", samples_per_symbol);

    // Build tone frequency map (and print it)
    let tone_freqs = modem::build_tone_frequencies(&params);
    println!("Tone frequencies per channel:");
    for (ch, freqs) in tone_freqs.iter().enumerate() {
        println!(" Ch {}: {:?}", ch, freqs);
    }

    // detection
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

    // debug print (first few)
    for (ch, vec) in detected_by_channel.iter().enumerate() {
        println!("Channel {} detected (first 20): {:?}", ch, &vec[..std::cmp::min(20, vec.len())]);
    }

    // round-robin reinterleave
    let mut symbols: Vec<u8> = Vec::new();
    let max_len = detected_by_channel.iter().map(|v| v.len()).max().unwrap_or(0);
    for idx in 0..max_len {
        for ch in 0..params.channels {
            if idx < detected_by_channel[ch].len() {
                symbols.push(detected_by_channel[ch][idx]);
            }
        }
    }

    println!("Detected {} symbols", symbols.len());
    let bytes = modem::symbols_to_bytes(&symbols, params.m_tones);
    println!("Recovered {} bytes from symbols", bytes.len());
    // debug: show head
    let head_len = std::cmp::min(64, bytes.len());
    print!("Head of recovered bytes (first {}):", head_len);
    for b in &bytes[..head_len] { print!(" {:02X}", b); }
    println!();

    // NOTE: do not move `bytes` (Vec<u8>) if we want to reuse it later.
    // First try to depacketize (we expect the encoder used packetize_stream)
    match modem::depacketize_stream(&bytes, expected_repeats) {
        Ok(recovered_frame) => {
            println!("Depacketize succeeded: {} bytes", recovered_frame.len());
            // Try to parse frame header from recovered_frame
            match modem::parse_frame_header(&recovered_frame) {
                Ok((_fname, _compr, _enc, start, plen, _crc)) => {
                    println!("Frame header parsed. payload start {} len {}", start, plen);
                    match modem::extract_frame(&recovered_frame, maybe_key) {
                        Ok((fname, recovered_bytes)) => {
                            println!("Recovered filename: {}", fname);
                            detect_and_write_file(&recovered_bytes, out_basename)?;
                        }
                        Err(e) => {
                            eprintln!("Failed to extract frame from depacketized data: {}", e);
                            // fallback: write the raw recovered frame for inspection
                            let mut f = File::create(format!("{}_raw_frame.bin", out_basename))?;
                            f.write_all(&recovered_frame)?;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Could not parse frame header from depacketized bytes: {}", e);
                    let mut f = File::create(format!("{}_raw_frame_fail.bin", out_basename))?;
                    f.write_all(&recovered_frame)?;
                }
            }
        }
        Err(e) => {
            eprintln!("Depacketize failed or no packets found: {}. Falling back to trying raw frame parsing.", e);
            // try parsing header directly from raw `bytes`
            match modem::parse_frame_header(&bytes) {
                Ok((_fname, _compr, _enc, start, plen, _crc)) => {
                    println!("Frame header parsed (raw). payload start {} len {}", start, plen);
                    // pass a reference to bytes (don't move it)
                    match modem::extract_frame(&bytes, maybe_key) {
                        Ok((fname, recovered_bytes)) => {
                            println!("Recovered filename: {}", fname);
                            detect_and_write_file(&recovered_bytes, out_basename)?;
                        }
                        Err(e) => {
                            eprintln!("Failed to extract frame: {}", e);
                            detect_and_write_file(&bytes, out_basename)?;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Could not parse frame header from raw recovered bytes: {}", e);
                    // still write raw bytes for inspection
                    detect_and_write_file(&bytes, out_basename)?;
                }
            }
        }
    }

    Ok(())
}

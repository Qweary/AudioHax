// src/bin/modem_decode.rs
//
// S9 (WS-4 Phase 1): hand-rolled positional parsing replaced by the shared clap
// grammar in `audiohax::cli`. Same library logic runs unchanged below.
use std::fs::File;
use std::io::Write;

use audiohax::cli::parse_modem_decode;
use audiohax::modem::{self, ModemParams};
use hound;

/// helper: write recovered file to disk and pick extension
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

/// find a subslice pattern in `haystack`; returns first index if found, else None
fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    for i in 0..=haystack.len() - needle.len() {
        if &haystack[i..i + needle.len()] == needle {
            return Some(i);
        }
    }
    None
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Shared clap grammar (S9 §3.7).
    let cli = parse_modem_decode();
    let in_wav = cli.in_wav.to_string_lossy().to_string();
    let in_wav = in_wav.as_str();
    let out_basename = cli
        .out_basename
        .clone()
        .unwrap_or_else(|| "payload".to_string());
    let out_basename = out_basename.as_str();
    let maybe_key = cli.decrypt.as_deref();

    // Parse optional params (same shapes as before; clap fills the Options).
    let mut params = ModemParams::default();
    if let Some(c) = cli.channels {
        params.channels = c;
    }
    if let Some(m) = cli.mtones {
        params.m_tones = m;
    }
    if let Some(ms) = cli.symbol_ms {
        params.symbol_ms = ms;
    }
    let expected_repeats: Option<usize> = cli.repeats;
    let rs_data_shards: Option<usize> = cli.rs_data;
    let rs_parity_shards: Option<usize> = cli.rs_parity;

    println!(
        "Using params: channels={}, m_tones={}, symbol_ms={}, sample_rate={}{}",
        params.channels,
        params.m_tones,
        params.symbol_ms,
        params.sample_rate,
        match expected_repeats {
            Some(r) => format!(", expected_repeats={}", r),
            None => String::new(),
        }
    );

    let reader = hound::WavReader::open(in_wav)?;
    let samples_i16: Vec<i16> = reader
        .into_samples::<i16>()
        .filter_map(Result::ok)
        .collect();
    println!("Read {} samples", samples_i16.len());

    let samples_per_symbol =
        ((params.sample_rate as f32) * (params.symbol_ms / 1000.0)).round() as usize;
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
        if window_start + samples_per_symbol > samples_i16.len() {
            break;
        }
        let slice = &samples_i16[window_start..window_start + samples_per_symbol];
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
        println!(
            "Channel {} detected (first 20): {:?}",
            ch,
            &vec[..std::cmp::min(20, vec.len())]
        );
    }

    // --- PREAMBLE DETECTION & ALIGNMENT ---
    // If encoder used params.preamble_symbols repeated params.preamble_repeats per channel,
    // detect per-channel and trim each channel's vector so it starts after the preamble.
    if params.preamble_symbols.len() > 0 && params.preamble_repeats > 0 {
        let mut pattern: Vec<u8> = Vec::new();
        for _ in 0..params.preamble_repeats {
            pattern.extend_from_slice(&params.preamble_symbols);
        }
        let pat_len = pattern.len();
        println!(
            "Looking for per-channel preamble pattern ({} symbols)",
            pat_len
        );
        for ch in 0..params.channels {
            let chvec = &mut detected_by_channel[ch];
            if let Some(idx) = find_subslice(chvec, &pattern) {
                println!("Channel {}: found preamble at index {} (trimming)", ch, idx);
                // trim to start after preamble
                if idx + pat_len <= chvec.len() {
                    *chvec = chvec[idx + pat_len..].to_vec();
                } else {
                    *chvec = Vec::new();
                }
            } else {
                println!("Channel {}: preamble not found; leaving as-is", ch);
            }
        }
    } else {
        println!("No preamble symbols configured in params; skipping preamble alignment.");
    }

    // round-robin reinterleave (after alignment/trimming)
    let mut symbols: Vec<u8> = Vec::new();
    let max_len = detected_by_channel
        .iter()
        .map(|v| v.len())
        .max()
        .unwrap_or(0);
    for i in 0..max_len {
        for ch in 0..params.channels {
            if i < detected_by_channel[ch].len() {
                symbols.push(detected_by_channel[ch][i]);
            }
        }
    }

    println!("Detected {} symbols", symbols.len());

    // First attempt: the straightforward convert to bytes and standard flow
    let bytes = modem::symbols_to_bytes(&symbols, params.m_tones);
    println!("Recovered {} bytes from symbols", bytes.len());
    let head_len = std::cmp::min(64, bytes.len());
    print!("Head of recovered bytes (first {}):", head_len);
    for b in &bytes[..head_len] {
        print!(" {:02X}", b);
    }
    println!();

    // If RS options were provided, attempt RS depacketize, otherwise use repeat-based depacketize, then fallback to raw frame parse.
    let packetized_result = if rs_data_shards.is_some() && rs_parity_shards.is_some() {
        println!("Attempting Reed-Solomon depacketize on straightforward bytes...");
        match modem::depacketize_stream_rs(&bytes) {
            Ok(v) => {
                println!("Depacketize RS succeeded: {} bytes", v.len());
                Some(v)
            }
            Err(e) => {
                eprintln!(
                    "Depacketize RS failed: {}. Falling back to repeat-depacketize.",
                    e
                );
                None
            }
        }
    } else {
        // try repetition-based
        if let Some(r) = expected_repeats {
            println!(
                "Attempting repetition depacketize (expected repeats = {})",
                r
            );
            match modem::depacketize_stream(&bytes, r) {
                Ok(v) => {
                    println!("Depacketize succeeded: {} bytes", v.len());
                    Some(v)
                }
                Err(e) => {
                    eprintln!("Depacketize failed or no packets found: {}. Falling back to trying raw frame parsing.", e);
                    None
                }
            }
        } else {
            None
        }
    };

    // Decide what to parse as frame bytes: prefer depacketized result if available, else raw bytes
    let frame_bytes = if let Some(v) = packetized_result {
        v
    } else {
        bytes
    };

    // Try parse frame header / extract
    match modem::parse_frame_header(&frame_bytes) {
        Ok((_fname, _compr, _enc, start, plen, _crc)) => {
            println!("Frame header parsed. payload start {} len {}", start, plen);
            let frame: &[u8] = &frame_bytes;
            match modem::extract_frame(frame, maybe_key) {
                Ok((fname, recovered_bytes)) => {
                    println!("Recovered filename: {}", fname);
                    detect_and_write_file(&recovered_bytes, out_basename)?;
                }
                Err(e) => {
                    eprintln!("Failed to extract frame: {}", e);
                    // as a last resort write entire frame_bytes
                    detect_and_write_file(&frame_bytes, out_basename)?;
                }
            }
        }
        Err(e) => {
            eprintln!("Could not parse frame header from recovered bytes: {}", e);
            detect_and_write_file(&frame_bytes, out_basename)?;
        }
    }

    Ok(())
}

// src/bin/modem_decode.rs
use std::env;
use std::fs::File;
use std::io::Write;

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

/// convert symbol stream -> bit vector (MSB-first per symbol)
fn symbols_to_bitvec(symbols: &[u8], m_tones: usize) -> Vec<u8> {
    let bps = modem::bits_per_symbol(m_tones);
    let mut bits: Vec<u8> = Vec::with_capacity(symbols.len() * bps);
    for &s in symbols {
        let mut val = s as usize;
        // mask to bps bits
        val &= (1usize << bps) - 1;
        // produce MSB-first bits
        for i in (0..bps).rev() {
            let bit = ((val >> i) & 1) as u8;
            bits.push(bit);
        }
    }
    bits
}

/// take a bit vector and produce bytes starting at given bit offset
fn bitvec_to_bytes_with_offset(bits: &[u8], bit_offset: usize) -> Vec<u8> {
    if bit_offset >= 8 {
        return Vec::new();
    }
    if bits.len() <= bit_offset {
        return Vec::new();
    }
    let mut out: Vec<u8> = Vec::with_capacity((bits.len() - bit_offset + 7) / 8);
    let mut i = bit_offset;
    while i + 8 <= bits.len() {
        let mut b = 0u8;
        for j in 0..8 {
            b = (b << 1) | bits[i + j];
        }
        out.push(b);
        i += 8;
    }
    out
}

fn try_all_bit_alignments_and_depacketize(
    symbols: &[u8],
    m_tones: usize,
    expected_repeats: Option<usize>,
    rs_opts: Option<(usize, usize)>,
    maybe_key: Option<&str>,
    out_basename: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let bits = symbols_to_bitvec(symbols, m_tones);
    let max_offset = 8usize; // try 0..7
    println!("Trying bit-alignment search (0..7) on bitstream length {} bits", bits.len());
    for off in 0..max_offset {
        let candidate = bitvec_to_bytes_with_offset(&bits, off);
        if candidate.is_empty() { continue; }
        // Quick head print for diagnostics
        let head_len = std::cmp::min(32, candidate.len());
        print!("Try offset {} head:", off);
        for b in &candidate[..head_len] { print!(" {:02X}", b); }
        println!();

        // If RS options requested, try RS depacketize first
        let packetized_result = if rs_opts.is_some() {
            match modem::depacketize_stream_rs(&candidate) {
                Ok(v) => {
                    println!("Offset {}: RS depacketize succeeded ({} bytes)", off, v.len());
                    Some(v)
                }
                Err(e) => {
                    // RS failed (no packets or reconstruct failure)
                    // continue falling back to repeats/raw parsing
                    //print!("Offset {}: RS depacketize failed: {}\n", off, e);
                    None
                }
            }
        } else if let Some(r) = expected_repeats {
            match modem::depacketize_stream(&candidate, r) {
                Ok(v) => {
                    println!("Offset {}: repetition depacketize succeeded ({} bytes)", off, v.len());
                    Some(v)
                }
                Err(_e) => None,
            }
        } else {
            None
        };

        let frame_bytes = if let Some(v) = packetized_result { v } else { candidate };

        // Try to parse AHX1 header
        match modem::parse_frame_header(&frame_bytes) {
            Ok((_fname, _compr, _enc, start, plen, _crc)) => {
                println!("Offset {}: Parsed frame header: payload start {} len {}", off, start, plen);
                // attempt extract
                match modem::extract_frame(&frame_bytes, maybe_key) {
                    Ok((fname, recovered_bytes)) => {
                        println!("Recovered filename: {} ({} bytes) at offset {}", fname, recovered_bytes.len(), off);
                        detect_and_write_file(&recovered_bytes, out_basename)?;
                        return Ok(());
                    }
                    Err(e) => {
                        eprintln!("Offset {}: Failed to extract frame: {}", off, e);
                        // fallback to writing frame_bytes raw later
                        detect_and_write_file(&frame_bytes, out_basename)?;
                        return Ok(());
                    }
                }
            }
            Err(_e) => {
                // Not a valid AHX1 at this offset; continue trying other offsets
            }
        }
    }

    Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Bit-alignment search failed to find a valid frame")))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <in.wav> [out_basename] [--decrypt KEYHEX] [--channels N] [--mtones M] [--symbol-ms MS] [--repeats R] [--rs-data D --rs-parity P]", args[0]);
        std::process::exit(1);
    }
    let in_wav = &args[1];
    let out_basename = args.get(2).map(|s| s.as_str()).unwrap_or("payload");
    let maybe_key = args.iter().position(|a| a == "--decrypt").and_then(|i| args.get(i+1)).map(|s| s.as_str());

    // Parse optional params
    let mut params = ModemParams::default();
    let mut expected_repeats: Option<usize> = None;
    let mut rs_data_shards: Option<usize> = None;
    let mut rs_parity_shards: Option<usize> = None;

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
                    if let Ok(r) = v.parse::<usize>() { expected_repeats = Some(r); }
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
            _ => { i += 1; }
        }
    }

    println!("Using params: channels={}, m_tones={}, symbol_ms={}, sample_rate={}{}",
             params.channels, params.m_tones, params.symbol_ms, params.sample_rate,
             match expected_repeats { Some(r) => format!(", expected_repeats={}", r), None => String::new() });

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
    for b in &bytes[..head_len] { print!(" {:02X}", b); }
    println!();

    // If RS options were provided, attempt RS depacketize; otherwise, repetition depacketize if requested.
    let rs_opts = if rs_data_shards.is_some() && rs_parity_shards.is_some() {
        Some((rs_data_shards.unwrap(), rs_parity_shards.unwrap()))
    } else { None };

    // Try direct RS/repetition/fallback parsing on the straightforward bytes first
    let mut packetized_result: Option<Vec<u8>> = None;

    if let Some((d, p)) = rs_opts {
        println!("Attempting Reed-Solomon depacketize on straightforward bytes...");
        match modem::depacketize_stream_rs(&bytes) {
            Ok(v) => {
                println!("Depacketize RS succeeded: {} bytes", v.len());
                packetized_result = Some(v);
            }
            Err(e) => {
                eprintln!("Depacketize RS failed: {}. Will try bit-alignment fallback...", e);
            }
        }
    } else if let Some(r) = expected_repeats {
        println!("Attempting repetition depacketize (expected repeats = {}) on straightforward bytes", r);
        match modem::depacketize_stream(&bytes, r) {
            Ok(v) => {
                println!("Depacketize succeeded: {} bytes", v.len());
                packetized_result = Some(v);
            }
            Err(e) => {
                eprintln!("Depacketize failed or no packets found on straightforward bytes: {}. Will try bit-alignment fallback.", e);
            }
        }
    }

    // Decide what to parse as frame bytes: prefer depacketized result if available, else raw bytes.
    if let Some(v) = packetized_result {
        let frame_bytes = v;
        match modem::parse_frame_header(&frame_bytes) {
            Ok((_fname, _compr, _enc, start, plen, _crc)) => {
                println!("Frame header parsed. payload start {} len {}", start, plen);
                match modem::extract_frame(&frame_bytes, maybe_key) {
                    Ok((fname, recovered_bytes)) => {
                        println!("Recovered filename: {}", fname);
                        detect_and_write_file(&recovered_bytes, out_basename)?;
                    }
                    Err(e) => {
                        eprintln!("Failed to extract frame: {}", e);
                        detect_and_write_file(&frame_bytes, out_basename)?;
                    }
                }
            }
            Err(e) => {
                eprintln!("Could not parse frame header from depacketized bytes: {}. Saving them raw.", e);
                detect_and_write_file(&frame_bytes, out_basename)?;
            }
        }
        return Ok(());
    }

    // If we reach here, straightforward path failed. Try bit-alignment search (0..7)
    match try_all_bit_alignments_and_depacketize(&symbols, params.m_tones, expected_repeats, rs_opts, maybe_key, out_basename) {
        Ok(()) => return Ok(()),
        Err(e) => {
            eprintln!("Bit-alignment fallback failed: {}", e);
            // As a last resort write the straightforward bytes to disk
            eprintln!("Writing raw straightforward bytes to disk as last resort.");
            detect_and_write_file(&bytes, out_basename)?;
        }
    }

    Ok(())
}

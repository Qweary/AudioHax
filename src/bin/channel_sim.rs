// src/bin/channel_sim.rs
use std::env;
use std::fs::File;
use std::io::{Read, Write};

use rand::prelude::*;
use audiohax::modem;
use audiohax::modem::{depacketize_stream, depacketize_stream_rs};

fn print_usage(name: &str) {
    eprintln!("Usage:");
    eprintln!("  {} <in_bytes.bin> <out_sim.bin> [--mode bitflip|byteburst|packet] [--flip_prob P] [--burst_prob P] [--burst_len L] [--packet_size N] [--repeats R] [--rs-data D --rs-parity P]", name);
    eprintln!();
    eprintln!("Examples:");
    eprintln!("  {} packetized.bin sim_bytes.bin --mode packet --packet_size 152 --burst_prob 0.05 --flip_prob 0.001 --rs-data 4 --rs-parity 2", name);
    eprintln!("  {} packetized.bin sim_bytes.bin --mode bitflip --flip_prob 0.0005", name);
}

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
    let out_path = format!("{}_sim_recovered.{}", out_basename, ext);
    println!("Writing {} ({} bytes)", out_path, bytes.len());
    let mut f = File::create(&out_path)?;
    f.write_all(bytes)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        print_usage(&args[0]);
        std::process::exit(1);
    }
    let in_path = &args[1];
    let out_path = &args[2];

    // defaults
    let mut mode = "bitflip".to_string();
    let mut flip_prob: f64 = 0.0;
    let mut burst_prob: f64 = 0.0;
    let mut burst_len: usize = 16;
    let mut packet_size: usize = 128;
    let mut repeats_opt: Option<usize> = None;
    let mut rs_data: Option<usize> = None;
    let mut rs_parity: Option<usize> = None;

    // parse args
    let mut i = 3usize;
    while i < args.len() {
        match args[i].as_str() {
            "--mode" => { if let Some(v) = args.get(i+1) { mode = v.clone(); } i += 2; }
            "--flip_prob" => { if let Some(v) = args.get(i+1) { flip_prob = v.parse::<f64>().unwrap_or(0.0); } i += 2; }
            "--burst_prob" => { if let Some(v) = args.get(i+1) { burst_prob = v.parse::<f64>().unwrap_or(0.0); } i += 2; }
            "--burst_len" => { if let Some(v) = args.get(i+1) { burst_len = v.parse::<usize>().unwrap_or(16); } i += 2; }
            "--packet_size" => { if let Some(v) = args.get(i+1) { packet_size = v.parse::<usize>().unwrap_or(128); } i += 2; }
            "--repeats" => { if let Some(v) = args.get(i+1) { repeats_opt = v.parse::<usize>().ok(); } i += 2; }
            "--rs-data" => { if let Some(v) = args.get(i+1) { rs_data = v.parse::<usize>().ok(); } i += 2; }
            "--rs-parity" => { if let Some(v) = args.get(i+1) { rs_parity = v.parse::<usize>().ok(); } i += 2; }
            _ => { eprintln!("Unknown arg {}", args[i]); i += 1; }
        }
    }

    // read input bytes
    let mut f = File::open(in_path)?;
    let mut bytes: Vec<u8> = Vec::new();
    f.read_to_end(&mut bytes)?;
    println!("Read {} bytes from {}", bytes.len(), in_path);

    // RNG
    let mut rng = StdRng::from_entropy();

    // simulate
    let mut sim = bytes.clone();

    match mode.as_str() {
        "bitflip" => {
            if flip_prob <= 0.0 {
                println!("flip_prob <= 0, nothing to do.");
            } else {
                println!("Applying bitflip channel: flip_prob={}", flip_prob);
                for b in sim.iter_mut() {
                    for bit in 0..8 {
                        if rng.gen_bool(flip_prob) {
                            *b ^= 1u8 << bit;
                        }
                    }
                }
            }
        }
        "byteburst" => {
            println!("Applying byte-burst erasure model: burst_prob={}, burst_len={}", burst_prob, burst_len);
            let mut i = 0usize;
            while i < sim.len() {
                if rng.gen_bool(burst_prob) {
                    let len = burst_len; // fixed length; could be randomized
                    let end = std::cmp::min(i+len, sim.len());
                    for j in i..end { sim[j] = 0u8; } // erase -> zeros
                    i = end;
                } else {
                    i += 1;
                }
            }
        }
        "packet" => {
            println!("Packet-aware mode: packet_size={}, burst_prob={}, flip_prob={}", packet_size, burst_prob, flip_prob);
            let mut idx = 0usize;
            while idx < sim.len() {
                let end = std::cmp::min(idx + packet_size, sim.len());
                if rng.gen_bool(burst_prob) {
                    // drop whole packet -> zero it
                    for j in idx..end { sim[j] = 0u8; }
                } else {
                    // surviving packet: optionally flip bits inside
                    if flip_prob > 0.0 {
                        for b in &mut sim[idx..end] {
                            for bit in 0..8 {
                                if rng.gen_bool(flip_prob) {
                                    *b ^= 1u8 << bit;
                                }
                            }
                        }
                    }
                }
                idx = end;
            }
        }
        other => {
            eprintln!("Unknown mode: {}. Supported: bitflip, byteburst, packet", other);
            std::process::exit(1);
        }
    }

    // write simulated bytes to out_path
    let mut of = File::create(out_path)?;
    of.write_all(&sim)?;
    println!("Wrote simulated bytes to {}", out_path);

    // Attempt depacketize (RS if rs_data/rs_parity set, otherwise repetition with repeats)
    let recovered = if let (Some(d), Some(p)) = (rs_data, rs_parity) {
        println!("Attempting RS depacketize: data={} parity={}", d, p);
        match depacketize_stream_rs(&sim) {
            Ok(v) => {
                println!("RS depacketize succeeded: {} bytes", v.len());
                Some(v)
            }
            Err(e) => {
                eprintln!("RS depacketize failed: {}", e);
                None
            }
        }
    } else if let Some(r) = repeats_opt {
        println!("Attempting repetition depacketize (expected repeats = {})", r);
        match depacketize_stream(&sim, r) {
            Ok(v) => {
                println!("Repetition depacketize succeeded: {} bytes", v.len());
                Some(v)
            }
            Err(e) => {
                eprintln!("Repetition depacketize failed: {}", e);
                None
            }
        }
    } else {
        println!("No depacketize options provided (no RS & no repeats); skipping depacketize attempt.");
        None
    };

    // If we recovered bytes, try parsing/extracting frame
    if let Some(frame_bytes) = recovered {
        println!("Attempting to parse frame header from recovered bytes ({} bytes)...", frame_bytes.len());
        match modem::parse_frame_header(&frame_bytes) {
            Ok((_fname, _compr, _enc, start, plen, _crc)) => {
                println!("Frame header parsed. payload start {} len {}", start, plen);
                match modem::extract_frame(&frame_bytes, None) {
                    Ok((fname, recovered_payload)) => {
                        println!("Successfully extracted file '{}' ({} bytes)", fname, recovered_payload.len());
                        detect_and_write_file(&recovered_payload, "sim_out")?;
                    }
                    Err(e) => {
                        eprintln!("Failed to extract frame: {}", e);
                        // write full frame for inspection
                        let mut dump = File::create("sim_frame_dump.bin")?;
                        dump.write_all(&frame_bytes)?;
                        println!("Wrote sim_frame_dump.bin for inspection.");
                    }
                }
            }
            Err(e) => {
                eprintln!("Could not parse frame header from recovered bytes: {}", e);
                // write recovered bytes to file for inspection
                let mut dump = File::create("sim_recovered_raw.bin")?;
                dump.write_all(&frame_bytes)?;
                println!("Wrote sim_recovered_raw.bin for inspection.");
            }
        }
    } else {
        println!("No recovered bytes (depacketize failed or not attempted).");
    }

    println!("Simulation complete.");
    Ok(())
}

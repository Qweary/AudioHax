// src/bin/channel_sim.rs
//
// S9 (WS-4 Phase 1): hand-rolled positional parsing replaced by the shared clap
// grammar in `audiohax::cli`. Legacy UNDERSCORE flag spellings (`--flip_prob`,
// `--burst_prob`, `--burst_len`, `--packet_size`) are preserved as aliases in the
// shared struct so existing scripts keep working. Same library logic runs unchanged.
use std::fs::File;
use std::io::{Read, Write};

use audiohax::cli::{parse_channel_sim, ChannelMode};
use audiohax::modem;
use audiohax::modem::{depacketize_stream, depacketize_stream_rs, AcousticChannelParams};
use rand::prelude::*;

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
    // Shared clap grammar (S9 §3.7), legacy underscore flag aliases preserved.
    let cli = parse_channel_sim();
    let in_path = cli.in_bytes.to_string_lossy().to_string();
    let in_path = in_path.as_str();
    let out_path = cli.out_sim.to_string_lossy().to_string();
    let out_path = out_path.as_str();

    // map the typed ChannelMode enum back to the legacy mode string the match below
    // already dispatches on (keeps the simulation logic byte-for-byte unchanged).
    let mode = match cli.mode {
        ChannelMode::Bitflip => "bitflip".to_string(),
        ChannelMode::Byteburst => "byteburst".to_string(),
        ChannelMode::Packet => "packet".to_string(),
        ChannelMode::Acoustic => "acoustic".to_string(),
    };
    let flip_prob: f64 = cli.flip_prob;
    let burst_prob: f64 = cli.burst_prob;
    let burst_len: usize = cli.burst_len;
    let packet_size: usize = cli.packet_size;
    let repeats_opt: Option<usize> = cli.repeats;
    let rs_data: Option<usize> = cli.rs_data;
    let rs_parity: Option<usize> = cli.rs_parity;

    // Acoustic-channel (S7) knobs — used only in --mode acoustic.
    // Cast to AcousticChannelParams' field types at the boundary (the bin owns this
    // conversion). The legacy `.parse()` inferred these from the field types directly;
    // the CLI exposes width-neutral usize/f64 and the bin narrows here — same values.
    let mut acoustic = AcousticChannelParams::identity();
    acoustic.seed = cli.acoustic_seed;
    acoustic.start_offset_samples = cli.start_offset as isize;
    acoustic.clock_ppm = cli.clock_ppm as f32;
    acoustic.freq_offset_hz = cli.freq_offset as f32;
    acoustic.echo_delay_samples = cli.echo_delay;
    acoustic.echo_gain = cli.echo_gain as f32;
    acoustic.jitter_samples = cli.jitter as f32;

    // read input bytes
    let mut f = File::open(in_path)?;
    let mut bytes: Vec<u8> = Vec::new();
    f.read_to_end(&mut bytes)?;
    println!("Read {} bytes from {}", bytes.len(), in_path);

    // RNG
    let mut rng = StdRng::from_entropy();

    // ── ACOUSTIC mode: interpret input as raw little-endian i16 audio samples,
    // apply the S7 seeded acoustic-channel model, and write the perturbed samples
    // back. This is the audio-domain channel (start offset / clock drift / freq
    // offset / multipath echo), distinct from the byte-domain models below. It
    // exercises the SAME library function the integration tests use, so behavior
    // is identical across the bin and the test crate.
    if mode == "acoustic" {
        if bytes.len() % 2 != 0 {
            eprintln!("acoustic mode: input length must be even (raw i16 LE samples)");
            std::process::exit(1);
        }
        let samples: Vec<i16> = bytes
            .chunks_exact(2)
            .map(|c| i16::from_le_bytes([c[0], c[1]]))
            .collect();
        println!(
            "Acoustic channel: {} samples, seed={} start_offset={} clock_ppm={} freq_offset={} echo_delay={} echo_gain={} jitter={}",
            samples.len(),
            acoustic.seed,
            acoustic.start_offset_samples,
            acoustic.clock_ppm,
            acoustic.freq_offset_hz,
            acoustic.echo_delay_samples,
            acoustic.echo_gain,
            acoustic.jitter_samples
        );
        let out = modem::simulate_acoustic_channel(&samples, &acoustic);
        let mut out_bytes: Vec<u8> = Vec::with_capacity(out.len() * 2);
        for s in &out {
            out_bytes.extend_from_slice(&s.to_le_bytes());
        }
        let mut of = File::create(out_path)?;
        of.write_all(&out_bytes)?;
        println!(
            "Wrote {} perturbed samples ({} bytes) to {}",
            out.len(),
            out_bytes.len(),
            out_path
        );
        println!("Simulation complete.");
        return Ok(());
    }

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
            println!(
                "Applying byte-burst erasure model: burst_prob={}, burst_len={}",
                burst_prob, burst_len
            );
            let mut i = 0usize;
            while i < sim.len() {
                if rng.gen_bool(burst_prob) {
                    let len = burst_len; // fixed length; could be randomized
                    let end = std::cmp::min(i + len, sim.len());
                    for j in i..end {
                        sim[j] = 0u8;
                    } // erase -> zeros
                    i = end;
                } else {
                    i += 1;
                }
            }
        }
        "packet" => {
            println!(
                "Packet-aware mode: packet_size={}, burst_prob={}, flip_prob={}",
                packet_size, burst_prob, flip_prob
            );
            let mut idx = 0usize;
            while idx < sim.len() {
                let end = std::cmp::min(idx + packet_size, sim.len());
                if rng.gen_bool(burst_prob) {
                    // drop whole packet -> zero it
                    for j in idx..end {
                        sim[j] = 0u8;
                    }
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
            eprintln!(
                "Unknown mode: {}. Supported: bitflip, byteburst, packet",
                other
            );
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
        println!(
            "Attempting repetition depacketize (expected repeats = {})",
            r
        );
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
        println!(
            "No depacketize options provided (no RS & no repeats); skipping depacketize attempt."
        );
        None
    };

    // If we recovered bytes, try parsing/extracting frame
    if let Some(frame_bytes) = recovered {
        println!(
            "Attempting to parse frame header from recovered bytes ({} bytes)...",
            frame_bytes.len()
        );
        match modem::parse_frame_header(&frame_bytes) {
            Ok((_fname, _compr, _enc, start, plen, _crc)) => {
                println!("Frame header parsed. payload start {} len {}", start, plen);
                match modem::extract_frame(&frame_bytes, None) {
                    Ok((fname, recovered_payload)) => {
                        println!(
                            "Successfully extracted file '{}' ({} bytes)",
                            fname,
                            recovered_payload.len()
                        );
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

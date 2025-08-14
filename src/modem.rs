// src/modem.rs
//! Simple multi-channel MFSK modem utilities with:
//! - header (magic, flags, filename length + name, payload len, crc32)
//! - optional gzip compression
//! - optional AES-GCM encryption (key supplied as 32-byte hex)
//! - bitpacking into symbols (base-m_tones) and round-robin channel splitting
//! - simple packetization + repetition-based FEC (for prototyping robustness)
//! - MFSK rendering (sum of sine carriers for simultaneous channels)
//! - simple Goertzel detector + helpers for decoding
//!
//! NOTE: demo-oriented; tune params (symbol_ms, tone spacing, channel spacing,
//! packet size and repetition) to suit acoustic environment.

use std::error::Error;
use std::collections::HashMap;

use crc32fast::Hasher as Crc32;
use flate2::{write::GzEncoder, Compression};
use std::io::Write;

use rand_core::{OsRng, RngCore};

pub use hound;
pub use hex;

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};

use std::f32::consts::PI;

/// Modem params and defaults (sane demo defaults)
#[derive(Debug, Clone)]
pub struct ModemParams {
    pub sample_rate: usize,
    pub symbol_ms: f32,
    pub m_tones: usize,
    pub channels: usize,            // parallel channels
    pub amplitude: f32,             // per-channel amplitude scale (0..1)
    pub base_freq_hz: f32,          // base freq for channel 0
    pub channel_spacing_hz: f32,    // spacing between channel bands
    pub tone_spacing_hz: f32,       // spacing between tones in a band
    pub preamble_repeats: usize,    // repeats of preamble
    pub preamble_symbols: Vec<u8>,  // small pattern, relative to m_tones
}

impl Default for ModemParams {
    fn default() -> Self {
        ModemParams {
            sample_rate: 48_000,
            symbol_ms: 30.0,
            m_tones: 32,
            channels: 4,
            amplitude: 0.55,
            base_freq_hz: 400.0,
            channel_spacing_hz: 400.0,
            tone_spacing_hz: 30.0,
            preamble_repeats: 8,
            preamble_symbols: vec![ (32/2) as u8 ], // use middle tone as pilot
        }
    }
}

/// Build a frame: header + optional compression/encryption payload.
///
/// Header layout (big-endian):
/// - 4 bytes magic: b"AHX1"
/// - 1 byte flags: bit0 = compressed, bit1 = encrypted
/// - 2 bytes filename_len (u16)
/// - filename bytes
/// - 4 bytes payload_len (u32) -- length of payload after compression/encryption
/// - 4 bytes crc32 (crc of compressed payload BEFORE encryption)
/// - payload bytes
pub fn build_frame(
    filename: &str,
    data: &[u8],
    compress: bool,
    encrypt_key_hex: Option<&str>,
) -> Result<Vec<u8>, Box<dyn Error>> {
    // compress if requested
    let compressed_bytes = if compress {
        let mut enc = GzEncoder::new(Vec::new(), Compression::default());
        enc.write_all(data)?;
        enc.finish()?
    } else {
        data.to_vec()
    };

    // compute CRC of compressed_bytes (useful to verify after decrypt/decompress)
    let mut hasher = Crc32::new();
    hasher.update(&compressed_bytes);
    let crc = hasher.finalize();

    // optional encryption: AES-GCM-256. final_payload = nonce || ciphertext
    let (final_payload, encrypted_flag) = if let Some(khex) = encrypt_key_hex {
        let key_bytes = hex::decode(khex).map_err(|e| {
            Box::<dyn Error>::from(std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("Invalid hex key: {}", e)))
        })?;
        if key_bytes.len() != 32 {
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Encryption key must be 32 bytes (64 hex chars)")));
        }
        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(key);

        // fill nonce using OsRng
        let mut rng = OsRng;
        let mut nonce_bytes = [0u8; 12];
        rng.fill_bytes(&mut nonce_bytes);

        let nonce = Nonce::from_slice(&nonce_bytes);

        // encrypt
        let cipher_text = cipher
            .encrypt(nonce, compressed_bytes.as_ref())
            .map_err(|e| Box::<dyn Error>::from(std::io::Error::new(std::io::ErrorKind::Other, format!("encrypt error: {}", e))))?;

        // final payload = nonce || ciphertext
        let mut v = Vec::with_capacity(12 + cipher_text.len());
        v.extend_from_slice(&nonce_bytes);
        v.extend_from_slice(&cipher_text);
        (v, true)
    } else {
        (compressed_bytes, false)
    };

    // assemble header + payload
    let mut out = Vec::new();
    out.extend_from_slice(b"AHX1");
    let mut flags: u8 = 0;
    if compress { flags |= 1; }
    if encrypted_flag { flags |= 2; }
    out.push(flags);

    let fname_bytes = filename.as_bytes();
    let fname_len = fname_bytes.len() as u16;
    out.extend_from_slice(&fname_len.to_be_bytes());
    out.extend_from_slice(fname_bytes);

    let payload_len = final_payload.len() as u32;
    out.extend_from_slice(&payload_len.to_be_bytes());

    out.extend_from_slice(&crc.to_be_bytes());

    out.extend_from_slice(&final_payload);

    Ok(out)
}

/// Parse header and return (filename, compressed_flag, encrypted_flag, payload_start_index, payload_len, crc)
pub fn parse_frame_header(buf: &[u8]) -> Result<(String, bool, bool, usize, usize, u32), Box<dyn Error>> {
    if buf.len() < 4 + 1 + 2 + 4 + 4 {
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Buffer too small")));
    }
    if &buf[0..4] != b"AHX1" {
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid magic")));
    }
    let flags = buf[4];
    let compressed = (flags & 1) != 0;
    let encrypted = (flags & 2) != 0;
    let mut idx = 5usize;
    let fname_len = u16::from_be_bytes([buf[idx], buf[idx+1]]) as usize;
    idx += 2;
    if buf.len() < idx + fname_len + 4 + 4 {
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "Buffer too small for filename and lengths")));
    }
    let fname = String::from_utf8(buf[idx .. idx + fname_len].to_vec())
        .map_err(|e| Box::<dyn Error>::from(std::io::Error::new(std::io::ErrorKind::InvalidData, format!("Invalid filename utf8: {}", e))))?;
    idx += fname_len;
    let payload_len = u32::from_be_bytes([buf[idx], buf[idx+1], buf[idx+2], buf[idx+3]]) as usize;
    idx += 4;
    let crc = u32::from_be_bytes([buf[idx], buf[idx+1], buf[idx+2], buf[idx+3]]);
    idx += 4;
    let payload_start = idx;
    if buf.len() < payload_start + payload_len {
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "Buffer does not contain full payload")));
    }
    Ok((fname, compressed, encrypted, payload_start, payload_len, crc))
}

/// Extract frame: decrypt if needed (requires decrypt key), verify CRC, decompress if needed.
pub fn extract_frame(frame: &[u8], decrypt_key_hex: Option<&str>) -> Result<(String, Vec<u8>), Box<dyn Error>> {
    let (fname, compressed, encrypted, payload_start, payload_len, crc) = parse_frame_header(frame)?;
    let payload = &frame[payload_start .. payload_start + payload_len];

    // decrypt if needed
    let decrypted: Vec<u8> = if encrypted {
        let khex = decrypt_key_hex.ok_or_else(|| Box::<dyn Error>::from(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Frame is encrypted but no decryption key provided")))?;
        let key_bytes = hex::decode(khex).map_err(|e| Box::<dyn Error>::from(std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("Invalid hex key: {}", e))))?;
        if key_bytes.len() != 32 {
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Decryption key must be 32 bytes (64 hex)")));
        }
        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(key);
        if payload.len() < 12 {
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, "Encrypted payload too short")));
        }
        let (nonce_bytes, ciphertext) = payload.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);
        let plain = cipher.decrypt(nonce, ciphertext.as_ref())
            .map_err(|e| Box::<dyn Error>::from(std::io::Error::new(std::io::ErrorKind::Other, format!("decrypt error: {}", e))))?;
        plain
    } else {
        payload.to_vec()
    };

    // CRC check
    let mut h = Crc32::new();
    h.update(&decrypted);
    let computed_crc = h.finalize();
    if computed_crc != crc {
        eprintln!("Warning: CRC mismatch: header {} != computed {}", crc, computed_crc);
        // continue; caller may verify contents
    }

    // decompress if needed
    let recovered = if compressed {
        use flate2::read::GzDecoder;
        use std::io::Read;
        let mut d = GzDecoder::new(&decrypted[..]);
        let mut out = Vec::new();
        d.read_to_end(&mut out)?;
        out
    } else {
        decrypted
    };

    Ok((fname, recovered))
}

/// Compute bits per symbol given m_tones (largest power-of-two bits)
pub fn bits_per_symbol(m_tones: usize) -> usize {
    let mut bits = 0usize;
    let mut val = 1usize;
    while val * 2 <= m_tones {
        val *= 2;
        bits += 1;
    }
    bits
}

/// Pack bytes into symbol stream using bits_per_symbol (MSB-first).
pub fn bytes_to_symbols(payload: &[u8], m_tones: usize) -> Vec<u8> {
    let bps = bits_per_symbol(m_tones);
    if bps == 0 {
        return payload.iter().map(|b| (*b % (m_tones as u8))).collect();
    }
    let mut out: Vec<u8> = Vec::new();
    let mut bitbuf: u64 = 0;
    let mut bits_in_buf: usize = 0;

    for &byte in payload {
        bitbuf = (bitbuf << 8) | (byte as u64);
        bits_in_buf += 8;
        while bits_in_buf >= bps {
            let shift = bits_in_buf - bps;
            let symbol = ((bitbuf >> shift) & ((1u64 << bps) - 1)) as u8;
            out.push(symbol);
            bits_in_buf -= bps;
            bitbuf &= (1u64 << bits_in_buf) - 1;
        }
    }
    if bits_in_buf > 0 {
        let symbol = ((bitbuf << (bps - bits_in_buf)) & ((1u64 << bps) - 1)) as u8;
        out.push(symbol);
    }
    out
}

/// Convert symbol stream back to bytes (inverse).
pub fn symbols_to_bytes(symbols: &[u8], m_tones: usize) -> Vec<u8> {
    let bps = bits_per_symbol(m_tones);
    if bps == 0 {
        return symbols.iter().map(|s| *s).collect();
    }
    let mut bitbuf: u64 = 0;
    let mut bits_in_buf: usize = 0;
    let mut out: Vec<u8> = Vec::new();
    for &sym in symbols {
        let sym_u64 = (sym as u64) & ((1u64 << bps) - 1);
        bitbuf = (bitbuf << bps) | sym_u64;
        bits_in_buf += bps;
        while bits_in_buf >= 8 {
            let shift = bits_in_buf - 8;
            let byte = ((bitbuf >> shift) & 0xFF) as u8;
            out.push(byte);
            bits_in_buf -= 8;
            bitbuf &= (1u64 << bits_in_buf) - 1;
        }
    }
    out
}

/// Round-robin split of symbols into `channels` channels.
pub fn split_round_robin(symbols: &[u8], channels: usize) -> Vec<Vec<u8>> {
    let mut out: Vec<Vec<u8>> = vec![Vec::new(); channels];
    for (i, &s) in symbols.iter().enumerate() {
        out[i % channels].push(s);
    }
    out
}

/// Render channels' symbol streams into mono i16 samples.
/// Each channel contributes a single sine per-symbol. Tones are placed in separated bands.
pub fn render_symbols_to_samples(channels_symbols: &Vec<Vec<u8>>, params: &ModemParams) -> Vec<i16> {
    let sample_rate = params.sample_rate as f32;
    let samples_per_symbol = ((params.sample_rate as f32) * (params.symbol_ms / 1000.0)).round() as usize;
    let mut out_samples: Vec<f32> = Vec::new();

    let max_len = channels_symbols.iter().map(|v| v.len()).max().unwrap_or(0);
    let per_chan_amp = params.amplitude / (params.channels as f32).max(1.0);

    // Hann window
    let mut hann: Vec<f32> = vec![0.0; samples_per_symbol];
    for n in 0..samples_per_symbol {
        let x = (n as f32) / (samples_per_symbol as f32);
        hann[n] = 0.5 * (1.0 - (2.0 * PI * x).cos());
    }

    for symbol_index in 0..max_len {
        for n in 0..samples_per_symbol {
            let t = n as f32 / sample_rate;
            let mut s = 0f32;
            for (ch, ch_symbols) in channels_symbols.iter().enumerate() {
                let sym = if symbol_index < ch_symbols.len() {
                    ch_symbols[symbol_index]
                } else { 0u8 } as usize;
                let tone_freq = params.base_freq_hz
                    + (ch as f32) * params.channel_spacing_hz
                    + (sym as f32) * params.tone_spacing_hz;
                let phase = 2.0 * PI * tone_freq * t;
                s += (phase.sin()) * per_chan_amp * hann[n];
            }
            out_samples.push(s);
        }
    }

    // normalize to i16
    let maxv = out_samples.iter().fold(0.0f32, |m, &v| m.max(v.abs())).max(1e-6);
    let scale = (i16::MAX as f32 * 0.9) / maxv;
    let mut out_i16: Vec<i16> = Vec::with_capacity(out_samples.len());
    for &v in out_samples.iter() {
        out_i16.push((v * scale) as i16);
    }
    out_i16
}

/// Build frequencies table (outer channel, inner tone)
pub fn build_tone_frequencies(params: &ModemParams) -> Vec<Vec<f32>> {
    let mut out: Vec<Vec<f32>> = Vec::with_capacity(params.channels);
    for ch in 0..params.channels {
        let mut v = Vec::with_capacity(params.m_tones);
        for sym in 0..params.m_tones {
            let tone_freq = params.base_freq_hz
                + (ch as f32) * params.channel_spacing_hz
                + (sym as f32) * params.tone_spacing_hz;
            v.push(tone_freq);
        }
        out.push(v);
    }
    out
}

/// Goertzel: magnitude-squared for target frequency on slice of i16 samples.
pub fn goertzel_mag_squared(samples: &[i16], target_freq: f32, sample_rate: usize) -> f32 {
    let n = samples.len();
    if n == 0 { return 0.0; }
    let sr = sample_rate as f32;
    let kf = target_freq * (n as f32) / sr;
    let k = kf.round() as usize;
    let omega = 2.0 * PI * (k as f32) / (n as f32);
    let coeff = 2.0 * omega.cos();
    let mut s_prev = 0.0f32;
    let mut s_prev2 = 0.0f32;
    for &x in samples {
        let s = (x as f32) + coeff * s_prev - s_prev2;
        s_prev2 = s_prev;
        s_prev = s;
    }
    let power = s_prev2 * s_prev2 + s_prev * s_prev - coeff * s_prev * s_prev2;
    power
}

/* -----------------------------
   Packetization + improved depacketization (majority / scanning)
   ----------------------------- */

/// Packetize a frame into repeated packets.
/// Packet header:
///  - 4 bytes magic: b"PKT1"
///  - 2 bytes payload_len (u16 BE)
///  - 4 bytes seq (u32 BE)
///  - 4 bytes total_packets (u32 BE)
///  - 4 bytes payload_crc (u32 BE)
///  - payload bytes
pub fn packetize_stream(data: &[u8], pkt_payload_size: usize, repeats: usize) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    if pkt_payload_size == 0 {
        return out;
    }
    let total = ((data.len() + pkt_payload_size - 1) / pkt_payload_size) as u32;
    let mut seq: u32 = 0;
    for start in (0..data.len()).step_by(pkt_payload_size) {
        let end = std::cmp::min(start + pkt_payload_size, data.len());
        let payload = &data[start..end];
        let payload_len = payload.len() as u16;

        // compute crc of payload
        let mut h = Crc32::new();
        h.update(payload);
        let crc = h.finalize();

        for _r in 0..repeats.max(1) {
            out.extend_from_slice(b"PKT1");
            out.extend_from_slice(&payload_len.to_be_bytes());
            out.extend_from_slice(&seq.to_be_bytes());
            out.extend_from_slice(&total.to_be_bytes());
            out.extend_from_slice(&crc.to_be_bytes());
            out.extend_from_slice(payload);
        }

        seq = seq.wrapping_add(1);
    }
    out
}

/// Depacketize a raw recovered byte stream into the original frame.
/// Scans for PKT1 headers anywhere in the stream, validates CRC, groups payloads by seq,
/// selects majority/first valid payload per seq, and then assembles in sequence order.
/// If no CRC-valid copies exist, falls back to majority-selection among the observed (possibly corrupted)
/// copies for each seq. Missing sequences are filled with zero-bytes of the most-common payload length.
pub fn depacketize_stream(buf: &[u8], _expected_repeats: usize) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // minimum header size = 4 (magic) + 2 (len) + 4 (seq) + 4 (total) + 4 (crc)
    const HDR_LEN: usize = 18;
    if buf.len() < HDR_LEN {
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "Buffer too small to contain packets")));
    }

    let mut i = 0usize;
    // map: seq -> Vec<(payload, crc_ok)>
    let mut map_all: HashMap<u32, Vec<(Vec<u8>, bool)>> = HashMap::new();
    let mut totals_seen: HashMap<u32, u32> = HashMap::new();

    while i + HDR_LEN <= buf.len() {
        if &buf[i..i+4] == b"PKT1" {
            // read header fields
            let payload_len = u16::from_be_bytes([buf[i+4], buf[i+5]]) as usize;
            let seq = u32::from_be_bytes([buf[i+6], buf[i+7], buf[i+8], buf[i+9]]);
            let total = u32::from_be_bytes([buf[i+10], buf[i+11], buf[i+12], buf[i+13]]);
            let crc = u32::from_be_bytes([buf[i+14], buf[i+15], buf[i+16], buf[i+17]]);

            let end = i + HDR_LEN + payload_len;
            if end > buf.len() {
                // partial packet at tail — stop scanning here
                break;
            }

            let payload = buf[i + HDR_LEN .. end].to_vec();

            // compute crc
            let mut h = Crc32::new();
            h.update(&payload);
            let computed = h.finalize();
            let crc_ok = computed == crc;

            // store payload and crc_ok
            map_all.entry(seq).or_default().push((payload.clone(), crc_ok));
            totals_seen.entry(seq).or_insert(total);

            // advance cursor past this packet
            i = end;
        } else {
            // not aligned - advance by 1 and scan again (resilient to noise/preamble)
            i += 1;
        }
    }

    if map_all.is_empty() {
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, "No PKT1 packets found in buffer")));
    }

    // Determine total packets (prefer the maximum total seen)
    let mut total_candidates: Vec<u32> = totals_seen.values().copied().collect();
    total_candidates.sort();
    total_candidates.dedup();
    let total = if !total_candidates.is_empty() {
        *total_candidates.last().unwrap()
    } else {
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, "Could not determine total packets")));
    };

    // determine the global-most-common payload length (used to fill missing sequences)
    let mut len_counts: HashMap<usize, usize> = HashMap::new();
    for v in map_all.values() {
        for (p, _ok) in v.iter() {
            *len_counts.entry(p.len()).or_insert(0) += 1;
        }
    }
    let global_fill_len = if !len_counts.is_empty() {
        *len_counts.iter().max_by_key(|kv| kv.1).unwrap().0
    } else {
        0usize
    };

    // assemble packets in order, choosing best candidate for each seq
    let mut assembled: Vec<u8> = Vec::new();
    let mut missing_seqs: Vec<u32> = Vec::new();

    for seq in 0u32..total {
        if let Some(cands) = map_all.get(&seq) {
            // If any cands have crc_ok==true, consider only those and majority-select among them.
            let mut chosen_payload: Option<Vec<u8>> = None;

            let mut crc_good_count = 0usize;
            for (_p, ok) in cands.iter() { if *ok { crc_good_count += 1 } }

            let candidates_to_consider: Vec<Vec<u8>> = if crc_good_count > 0 {
                cands.iter().filter(|(_,ok)| *ok).map(|(p,_)| p.clone()).collect()
            } else {
                cands.iter().map(|(p,_)| p.clone()).collect()
            };

            // majority-select the most frequent payload among candidates_to_consider
            let mut freq: HashMap<Vec<u8>, usize> = HashMap::new();
            for p in candidates_to_consider {
                *freq.entry(p).or_insert(0) += 1;
            }
            if !freq.is_empty() {
                let (best_payload, _count) = freq.into_iter().max_by_key(|kv| kv.1).unwrap();
                chosen_payload = Some(best_payload);
            }

            if let Some(payload) = chosen_payload {
                assembled.extend_from_slice(&payload);
            } else {
                // unexpected: no candidate payload (shouldn't happen), fill zeros
                assembled.extend_from_slice(&vec![0u8; global_fill_len]);
                missing_seqs.push(seq);
            }
        } else {
            // no copies at all for this sequence
            assembled.extend_from_slice(&vec![0u8; global_fill_len]);
            missing_seqs.push(seq);
        }
    }

    if !missing_seqs.is_empty() {
        eprintln!("Warning: missing {} sequences (examples): {:?}", missing_seqs.len(), &missing_seqs[..std::cmp::min(8, missing_seqs.len())]);
        // continue with best-effort assembled buffer
    }

    Ok(assembled)
}

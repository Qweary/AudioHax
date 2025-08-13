// src/modem.rs
//! Simple multi-channel MFSK modem utilities with:
//! - header (magic, flags, filename length + name, payload len, crc32)
//! - optional gzip compression
//! - optional AES-GCM encryption (key supplied as 32-byte hex)
//! - bitpacking into symbols (base-m_tones) and round-robin channel splitting
//! - MFSK rendering (sum of sine carriers for simultaneous channels)
//! - simple Goertzel-based energy detection for decoding
//!
//! NOTE: this is a demo-oriented modem, not a production SDR stack. Tune params
//! (symbol_ms, tone spacing, channel spacing) to suit your acoustic environment.

use std::error::Error;

use crc32fast::Hasher as Crc32;
use flate2::{write::GzEncoder, Compression};
use rand_core::OsRng;
use std::io::Write;

/// Re-exports for bins
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
    pub channels: usize,            // parallel instrument channels
    pub amplitude: f32,             // per-channel amplitude scale (0..1)
    pub base_freq_hz: f32,          // base freq for channel 0
    pub channel_spacing_hz: f32,    // spacing between channel bands
    pub tone_spacing_hz: f32,       // spacing between tones in a band
    pub preamble_repeats: usize,    // repeats of preamble
    pub preamble_symbols: Vec<u8>,  // pattern
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
            preamble_repeats: 6,
            preamble_symbols: vec![0,  (32/2) as u8], // will be truncated/used relative to m_tones
        }
    }
}

/// Build a full "frame" from a file payload:
/// header + (maybe gzipped, maybe encrypted) payload
///
/// Header format (all big-endian):
/// 4 bytes magic = b"AHX1"
/// 1 byte flags: bit0 = compressed, bit1 = encrypted
/// 2 bytes filename_len (u16)
/// filename bytes (utf-8)
/// 4 bytes payload_len (u32) <-- length of body after gzip/encrypt
/// 4 bytes crc32 (crc of post-compression, pre-encryption payload)  <-- helpful to verify after decrypt/decompress
/// Then the payload bytes follow (if encrypted, the ciphertext includes a 12-byte nonce prefix)
pub fn build_frame(
    filename: &str,
    data: &[u8],
    compress: bool,
    encrypt_key_hex: Option<&str>,
) -> Result<Vec<u8>, Box<dyn Error>> {
    // 1) optionally compress
    let compressed_bytes = if compress {
        let mut e = GzEncoder::new(Vec::new(), Compression::default());
        e.write_all(data)?;
        let out = e.finish()?;
        out
    } else {
        data.to_vec()
    };

    // crc of the compressed (or raw) payload (we store it in header)
    let mut hasher = Crc32::new();
    hasher.update(&compressed_bytes);
    let crc = hasher.finalize();

    // 2) optionally encrypt (AES-GCM-256). ciphertext = nonce|cipher
    let (final_payload, encrypted_flag) = if let Some(khex) = encrypt_key_hex {
        // key must be 64 hex chars (32 bytes)
        let key_bytes = hex::decode(khex)?;
        if key_bytes.len() != 32 {
            return Err("Encryption key must be 32 bytes (64 hex chars)".into());
        }
        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(key);

        // random nonce 12 bytes
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        // encrypt: additional data = empty
        let cipher_text = cipher.encrypt(nonce, compressed_bytes.as_ref())?;
        // final payload = nonce || ciphertext
        let mut v = Vec::with_capacity(12 + cipher_text.len());
        v.extend_from_slice(&nonce_bytes);
        v.extend_from_slice(&cipher_text);
        (v, true)
    } else {
        (compressed_bytes, false)
    };

    // 3) assemble header + payload
    let mut out = Vec::new();
    out.extend_from_slice(b"AHX1"); // magic
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

/// Try to parse frame header and return (filename, flags, payload_bytes_start_index, payload_len, crc)
/// Does NOT decrypt or decompress here — just parses header and returns offset where payload begins.
pub fn parse_frame_header(buf: &[u8]) -> Result<(String, bool, bool, usize, usize, u32), Box<dyn Error>> {
    if buf.len() < 4 + 1 + 2 + 4 + 4 {
        return Err("Buffer too small for header".into());
    }
    if &buf[0..4] != b"AHX1" {
        return Err("Invalid magic".into());
    }
    let flags = buf[4];
    let compressed = (flags & 1) != 0;
    let encrypted = (flags & 2) != 0;
    let mut idx = 5usize;
    let fname_len = u16::from_be_bytes([buf[idx], buf[idx+1]]) as usize;
    idx += 2;
    if buf.len() < idx + fname_len + 4 + 4 {
        return Err("Buffer too small for filename and lengths".into());
    }
    let fname = String::from_utf8(buf[idx .. idx + fname_len].to_vec())?;
    idx += fname_len;
    let payload_len = u32::from_be_bytes([buf[idx], buf[idx+1], buf[idx+2], buf[idx+3]]) as usize;
    idx += 4;
    let crc = u32::from_be_bytes([buf[idx], buf[idx+1], buf[idx+2], buf[idx+3]]);
    idx += 4;
    let payload_start = idx;
    if buf.len() < payload_start + payload_len {
        return Err("Buffer does not contain full payload".into());
    }
    Ok((fname, compressed, encrypted, payload_start, payload_len, crc))
}

/// Reverse frame: decrypt (if key provided) and decompress (if compressed flag)
/// Returns (filename, recovered_bytes)
pub fn extract_frame(
    frame: &[u8],
    decrypt_key_hex: Option<&str>,
) -> Result<(String, Vec<u8>), Box<dyn Error>> {
    let (fname, compressed, encrypted, payload_start, payload_len, crc) = parse_frame_header(frame)?;
    let payload = &frame[payload_start .. payload_start + payload_len];

    // 1) decrypt if needed
    let decrypted: Vec<u8> = if encrypted {
        let khex = decrypt_key_hex.ok_or("Frame is encrypted but no decryption key provided")?;
        let key_bytes = hex::decode(khex)?;
        if key_bytes.len() != 32 { return Err("Decryption key must be 32 bytes (64 hex)".into()); }
        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(key);
        if payload.len() < 12 {
            return Err("Encrypted payload too short".into());
        }
        let (nonce_bytes, ciphertext) = payload.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);
        let plain = cipher.decrypt(nonce, ciphertext.as_ref())?;
        plain
    } else {
        payload.to_vec()
    };

    // verify CRC
    let mut h = Crc32::new();
    h.update(&decrypted);
    let computed_crc = h.finalize();
    if computed_crc != crc {
        // warn but continue; caller can decide
        eprintln!("Warning: CRC mismatch: header {} != computed {}", crc, computed_crc);
    }

    // 2) decompress if needed
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

/// bit packing helpers: compute bits per symbol for given m_tones
pub fn bits_per_symbol(m_tones: usize) -> usize {
    // choose largest bits such that (1<<bits) <= m_tones
    let mut bits = 0usize;
    let mut val = 1usize;
    while val * 2 <= m_tones {
        val *= 2;
        bits += 1;
    }
    bits
}

/// Convert bytes -> symbol stream (symbols in range [0, m_tones-1]) using bit grouping.
/// This packs MSB-first across bytes.
pub fn bytes_to_symbols(payload: &[u8], m_tones: usize) -> Vec<u8> {
    let bps = bits_per_symbol(m_tones);
    if bps == 0 {
        // fallback: treat each byte as a symbol mod m_tones
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
    // pad remaining bits into a final symbol (left aligned)
    if bits_in_buf > 0 {
        let symbol = ((bitbuf << (bps - bits_in_buf)) & ((1u64 << bps) - 1)) as u8;
        out.push(symbol);
    }
    out
}

/// Convert symbol stream -> bytes (inverse of bytes_to_symbols).
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
    // any leftover bits are ignored (they were padding)
    out
}

/// Split a symbol stream into `channels` channels via round-robin distribution.
/// Returns Vec of per-channel symbol streams.
pub fn split_round_robin(symbols: &[u8], channels: usize) -> Vec<Vec<u8>> {
    let mut out: Vec<Vec<u8>> = vec![Vec::new(); channels];
    for (i, &s) in symbols.iter().enumerate() {
        out[i % channels].push(s);
    }
    out
}

/// Render N channel symbol streams into interleaved audio samples (mono mix)
/// Each channel contributes one sine tone per symbol; tones are placed in distinct bands
/// separated by `channel_spacing_hz`, with each tone's index mapped to `tone_spacing_hz`.
///
/// Windowing: simple Hann window per-symbol to reduce clicks.
pub fn render_symbols_to_samples(channels_symbols: &Vec<Vec<u8>>, params: &ModemParams) -> Vec<i16> {
    let sample_rate = params.sample_rate as f32;
    let samples_per_symbol = ((params.sample_rate as f32) * (params.symbol_ms / 1000.0)).round() as usize;
    let mut out_samples: Vec<f32> = Vec::new();

    // maximum symbol stream length
    let max_len = channels_symbols.iter().map(|v| v.len()).max().unwrap_or(0);

    // per-channel amplitude normalization
    let per_chan_amp = params.amplitude / (params.channels as f32).max(1.0);

    // precompute hann window
    let mut hann: Vec<f32> = vec![0.0; samples_per_symbol];
    for n in 0..samples_per_symbol {
        hann[n] = (PI * 2.0 * (n as f32) / (samples_per_symbol as f32)).sin() * 0.5 + 0.5;
        // simpler Hann alternative: 0.5*(1 - cos(2pi*n/(N-1)))
        // but current formula gives a smooth envelope
    }

    for symbol_index in 0..max_len {
        // for each sample in symbol window, sum contributions from each channel
        for n in 0..samples_per_symbol {
            let t = n as f32 / sample_rate; // relative time within symbol
            let mut s = 0f32;
            for (ch, ch_symbols) in channels_symbols.iter().enumerate() {
                let sym = if symbol_index < ch_symbols.len() {
                    ch_symbols[symbol_index]
                } else {
                    0u8
                } as usize;
                // frequency for this channel & symbol
                let tone_freq = params.base_freq_hz
                    + (ch as f32) * params.channel_spacing_hz
                    + (sym as f32) * params.tone_spacing_hz;
                let phase = 2.0 * PI * tone_freq * t;
                s += (phase.sin()) * per_chan_amp * hann[n];
            }
            out_samples.push(s);
        }
    }

    // scale to i16
    // find max amplitude
    let maxv = out_samples.iter().fold(0.0f32, |m, &v| m.max(v.abs())).max(1e-6);
    let scale = (i16::MAX as f32 * 0.9) / maxv;
    let mut out_i16: Vec<i16> = Vec::with_capacity(out_samples.len());
    for &v in out_samples.iter() {
        out_i16.push((v * scale) as i16);
    }
    out_i16
}

/// Build a list of tone frequencies across the whole multi-channel layout
/// (for decoder to scan). This returns Vec of Vec: outer by channel, inner by tone index.
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

/// Goertzel detector: returns magnitude-squared for target frequency on the given slice of i16 samples.
pub fn goertzel_mag_squared(samples: &[i16], target_freq: f32, sample_rate: usize) -> f32 {
    let n = samples.len();
    if n == 0 { return 0.0; }
    let sr = sample_rate as f32;
    let k = (0.5 + (n as f32 * target_freq / sr)).floor() as usize;
    let omega = 2.0 * PI * (k as f32) / (n as f32);
    let coeff = 2.0 * omega.cos();
    let mut s_prev = 0.0f32;
    let mut s_prev2 = 0.0f32;
    for &x in samples {
        let s = x as f32 + coeff * s_prev - s_prev2;
        s_prev2 = s_prev;
        s_prev = s;
    }
    let power = s_prev2 * s_prev2 + s_prev * s_prev - coeff * s_prev * s_prev2;
    power
}

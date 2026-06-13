// src/modem.rs
//! Simple multi-channel MFSK modem utilities with:
//! - header (magic, flags, filename length + name, payload len, crc32)
//! - optional gzip compression
//! - optional AES-GCM encryption (key supplied as 32-byte hex)
//! - bitpacking into symbols (base-m_tones)
//! - simple packetization + repetition-based FEC (for prototyping robustness)
//! - optional Reed-Solomon FEC packetization
//! - MFSK rendering (sum of sine carriers for simultaneous channels)
//! - simple Goertzel detector + helpers for decoding
//!
//! NOTE: demo-oriented; tune params (symbol_ms, tone spacing, channel spacing,
//! packet size, and FEC) to suit the acoustic environment.

use std::collections::HashMap;
use std::error::Error;

use crc32fast::Hasher as Crc32;
use flate2::{write::GzEncoder, Compression};
use std::io::Write;

use rand_core::{OsRng, RngCore};

pub use hex;
pub use hound;

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};

use std::f32::consts::PI;

/// Reed-Solomon crate (keep the same import style you used)
use reed_solomon_erasure::galois_8::ReedSolomon;

/// Structured error type for the modem DECODE path.
///
/// Every decode-side failure mode (header/magic, truncation/bounds, CRC, AES-GCM,
/// gzip, Reed-Solomon, repetition-depacketize, and a catch-all) maps to a distinct
/// variant so callers can diagnose *why* a decode failed instead of getting an
/// opaque `Box<dyn Error>` or — worse — silently-wrong bytes / a panic.
///
/// `ModemError` is `Send + Sync + 'static` (all fields are owned `String`/scalars
/// or `#[from]` source types that are themselves `Send + Sync`), so it composes
/// cleanly with `anyhow` in the modem bins via `?`.
#[derive(Debug, thiserror::Error)]
pub enum ModemError {
    /// Header is shorter than the fixed minimum, or the magic bytes are not "AHX1"
    /// (frame) — a structurally invalid header that cannot be parsed at all.
    #[error("invalid frame header: {0}")]
    BadHeader(String),

    /// A length/bounds failure: the buffer is too short to contain the declared
    /// filename, lengths, or payload — i.e. a truncated frame/stream. This replaces
    /// the previous index-into-too-short-slice / `.unwrap()` panic sites.
    #[error("truncated frame/buffer: {0}")]
    Truncated(String),

    /// CRC32 of the recovered (decrypted, pre-decompression) payload does not match
    /// the CRC carried in the header. Carries both values for diagnosis. A failed
    /// CRC now produces this error instead of silently returning garbage bytes.
    #[error("CRC mismatch: header expected {expected:#010x}, computed {computed:#010x}")]
    CrcMismatch { expected: u32, computed: u32 },

    /// Frame is flagged encrypted but no decryption key was supplied.
    #[error("frame is encrypted but no decryption key was provided")]
    MissingKey,

    /// Supplied key is not valid hex, or is not exactly 32 bytes (64 hex chars).
    #[error("invalid decryption key: {0}")]
    BadKey(String),

    /// AES-256-GCM decryption / authentication failed (wrong key, tampered
    /// ciphertext, or truncated nonce/tag).
    #[error("decryption (AES-GCM) failed: {0}")]
    Decrypt(String),

    /// gzip decompression of the recovered payload failed.
    #[error("decompression (gzip) failed")]
    Decompress(#[from] std::io::Error),

    /// Reed-Solomon reconstruction failed, or a block had fewer than `data_shards`
    /// surviving shards so reconstruction was not even attempted.
    #[error("Reed-Solomon reconstruction failed: {0}")]
    ReedSolomon(String),

    /// Repetition-FEC depacketize found no usable packets (e.g. no "PKT1" magic
    /// survived, so there are no copies to majority-vote over).
    #[error("repetition depacketize failed: {0}")]
    Depacketize(String),

    /// Catch-all for a lower-level error that does not map cleanly to a variant
    /// above; the message is owned so the error stays `Send + Sync + 'static`.
    #[error("modem error: {0}")]
    Other(String),
}

impl From<reed_solomon_erasure::Error> for ModemError {
    fn from(e: reed_solomon_erasure::Error) -> Self {
        ModemError::ReedSolomon(e.to_string())
    }
}

/// Modem params and defaults (sane demo defaults)
#[derive(Debug, Clone)]
pub struct ModemParams {
    pub sample_rate: usize,
    pub symbol_ms: f32,
    pub m_tones: usize,
    pub channels: usize,           // parallel channels
    pub amplitude: f32,            // per-channel amplitude scale (0..1)
    pub base_freq_hz: f32,         // base freq for channel 0
    pub channel_spacing_hz: f32,   // spacing between channel bands
    pub tone_spacing_hz: f32,      // spacing between tones in a band
    pub preamble_repeats: usize,   // repeats of preamble
    pub preamble_symbols: Vec<u8>, // small pattern, relative to m_tones
}

impl Default for ModemParams {
    fn default() -> Self {
        // ── Default frequency plan (WS-2 acoustic-hardening, S5) ──────────────
        //
        // The previous default (base 400, channel_spacing 400, tone_spacing 30,
        // symbol_ms 30) was internally broken on TWO counts: (a) each per-channel
        // tone band was 31*30 = 930 Hz wide but channels were only 400 Hz apart,
        // so the four bands overlapped heavily — Goertzel on one channel picked up
        // adjacent-channel energy (~54% symbol error, pilot never detected even on
        // a clean signal); and (b) the whole plan sat inside the ~65–2000 Hz
        // FluidSynth MIDI music band, so the modem and the music engine collided
        // acoustically.
        //
        // This plan fixes both, while KEEPING channels = 4 and m_tones = 32 (the
        // test net's pilot = m_tones/2 = 16 and the channel/tone counts depend on
        // these). The three signal knobs are chosen so every tone lands exactly on
        // a Goertzel bin center and the four bands are non-overlapping with a guard:
        //
        //   sample_rate      = 48_000 Hz  → Nyquist 24_000 Hz
        //   symbol_ms        = 40.0       → N = 1920 samples/symbol
        //                                  → Goertzel bin resolution = 48000/1920
        //                                    = 25.0 Hz
        //   tone_spacing_hz  = 50.0       = 2 bins  (each tone on its own bin pair,
        //                                   well above the 25 Hz resolution floor)
        //   channel_spacing  = 2000.0     = 80 bins
        //   base_freq_hz     = 3000.0     = 120 bins (≥ 2500 Hz music-clear floor,
        //                                   with margin)
        //
        // Per-channel band width = (m_tones-1)*tone_spacing = 31*50 = 1550 Hz.
        // Bands (base + ch*channel_spacing .. +1550):
        //   ch0: 3000..4550   ch1: 5000..6550   ch2: 7000..8550   ch3: 9000..10550
        // Each pair has a 450 Hz guard band (e.g. 4550→5000), so adjacent-channel
        // tone frequencies never coincide and Goertzel stays selective.
        //
        // Top tone (ch3, tone31) = 3000 + 3*2000 + 31*50 = 10_550 Hz, which is well
        // below Nyquist (24_000 Hz) — ~44% of Nyquist, no aliasing risk.
        //
        // Music separability: the lowest tone (3000 Hz) sits above the music-clear
        // floor of 2500 Hz and the whole band (3000..10_550 Hz) lives entirely
        // above the ~65–2000 Hz FluidSynth MIDI range, so the modem and the
        // image-to-music engine coexist in a shared acoustic channel.
        ModemParams {
            sample_rate: 48_000,
            symbol_ms: 40.0,
            m_tones: 32,
            channels: 4,
            amplitude: 0.55,
            base_freq_hz: 3000.0,
            channel_spacing_hz: 2000.0,
            tone_spacing_hz: 50.0,
            preamble_repeats: 8,
            preamble_symbols: vec![(32 / 2) as u8], // middle tone (index 16) as pilot
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
            Box::<dyn Error>::from(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid hex key: {}", e),
            ))
        })?;
        if key_bytes.len() != 32 {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Encryption key must be 32 bytes (64 hex chars)",
            )));
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
            .map_err(|e| {
                Box::<dyn Error>::from(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("encrypt error: {}", e),
                ))
            })?;

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
    if compress {
        flags |= 1;
    }
    if encrypted_flag {
        flags |= 2;
    }
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
pub fn parse_frame_header(
    buf: &[u8],
) -> Result<(String, bool, bool, usize, usize, u32), ModemError> {
    if buf.len() < 4 + 1 + 2 + 4 + 4 {
        return Err(ModemError::BadHeader(
            "buffer too small for fixed header".to_string(),
        ));
    }
    if &buf[0..4] != b"AHX1" {
        return Err(ModemError::BadHeader(
            "invalid magic (expected AHX1)".to_string(),
        ));
    }
    let flags = buf[4];
    let compressed = (flags & 1) != 0;
    let encrypted = (flags & 2) != 0;
    let mut idx = 5usize;
    let fname_len = u16::from_be_bytes([buf[idx], buf[idx + 1]]) as usize;
    idx += 2;
    if buf.len() < idx + fname_len + 4 + 4 {
        return Err(ModemError::Truncated(
            "buffer too small for filename and lengths".to_string(),
        ));
    }
    let fname = String::from_utf8(buf[idx..idx + fname_len].to_vec())
        .map_err(|e| ModemError::BadHeader(format!("invalid filename utf8: {}", e)))?;
    idx += fname_len;
    let payload_len =
        u32::from_be_bytes([buf[idx], buf[idx + 1], buf[idx + 2], buf[idx + 3]]) as usize;
    idx += 4;
    let crc = u32::from_be_bytes([buf[idx], buf[idx + 1], buf[idx + 2], buf[idx + 3]]);
    idx += 4;
    let payload_start = idx;
    if buf.len() < payload_start + payload_len {
        return Err(ModemError::Truncated(
            "buffer does not contain full payload".to_string(),
        ));
    }
    Ok((
        fname,
        compressed,
        encrypted,
        payload_start,
        payload_len,
        crc,
    ))
}

/// Extract frame: decrypt if needed (requires decrypt key), verify CRC, decompress if needed.
pub fn extract_frame(
    frame: &[u8],
    decrypt_key_hex: Option<&str>,
) -> Result<(String, Vec<u8>), ModemError> {
    let (fname, compressed, encrypted, payload_start, payload_len, crc) =
        parse_frame_header(frame)?;
    let payload = &frame[payload_start..payload_start + payload_len];

    // decrypt if needed
    let decrypted: Vec<u8> = if encrypted {
        let khex = decrypt_key_hex.ok_or(ModemError::MissingKey)?;
        let key_bytes =
            hex::decode(khex).map_err(|e| ModemError::BadKey(format!("invalid hex key: {}", e)))?;
        if key_bytes.len() != 32 {
            return Err(ModemError::BadKey(
                "key must be 32 bytes (64 hex chars)".to_string(),
            ));
        }
        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(key);
        if payload.len() < 12 {
            return Err(ModemError::Truncated(
                "encrypted payload too short for nonce".to_string(),
            ));
        }
        let (nonce_bytes, ciphertext) = payload.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);
        cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|e| ModemError::Decrypt(e.to_string()))?
    } else {
        payload.to_vec()
    };

    // CRC check — ENFORCING. The CRC is computed over the decrypted, pre-decompression
    // bytes (the same `compressed_bytes` build_frame hashed). A mismatch means the
    // payload is corrupt; return an error rather than emit silently-wrong bytes.
    let mut h = Crc32::new();
    h.update(&decrypted);
    let computed_crc = h.finalize();
    if computed_crc != crc {
        return Err(ModemError::CrcMismatch {
            expected: crc,
            computed: computed_crc,
        });
    }

    // decompress if needed
    let recovered = if compressed {
        use flate2::read::GzDecoder;
        use std::io::Read;
        let mut d = GzDecoder::new(&decrypted[..]);
        let mut out = Vec::new();
        d.read_to_end(&mut out)?; // std::io::Error -> ModemError::Decompress via #[from]
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
pub fn render_symbols_to_samples(
    channels_symbols: &Vec<Vec<u8>>,
    params: &ModemParams,
) -> Vec<i16> {
    let sample_rate = params.sample_rate as f32;
    let samples_per_symbol =
        ((params.sample_rate as f32) * (params.symbol_ms / 1000.0)).round() as usize;
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
                // A channel that has no symbol at this index (it is SHORTER than the
                // longest channel) must contribute SILENCE for this window — NOT a
                // real tone. The previous code substituted symbol 0, which rendered
                // that channel's tone-0 carrier (e.g. 5000 Hz for ch1 under the
                // default plan) at full amplitude. That injected spurious in-band
                // energy into every trailing window of a ragged-length transmission
                // and made an "only channel 0 active" render leak a strong adjacent-
                // channel tone. Skipping the carrier entirely keeps each channel's
                // band clean when that channel has nothing to send.
                let sym = match ch_symbols.get(symbol_index) {
                    Some(&v) => v as usize,
                    None => continue, // no symbol here -> emit silence for this channel
                };
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
    let maxv = out_samples
        .iter()
        .fold(0.0f32, |m, &v| m.max(v.abs()))
        .max(1e-6);
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
    if n == 0 {
        return 0.0;
    }
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
Packetization + simple repetition FEC
----------------------------- */

/// Packetize an arbitrary byte stream into small packets with header and repeat each packet `repeats` times.
/// Old (repeat-based) Packet format:
/// [4 bytes magic "PKT1"] [4 bytes seq (BE)] [2 bytes len (BE)] [4 bytes crc32 (BE)] [payload bytes]
pub fn packetize_stream(data: &[u8], pkt_payload_size: usize, repeats: usize) -> Vec<u8> {
    let mut out = Vec::new();
    let mut seq: u32 = 0;
    let mut offset = 0usize;
    while offset < data.len() {
        let end = std::cmp::min(offset + pkt_payload_size, data.len());
        let payload = &data[offset..end];
        // crc of payload
        let mut h = Crc32::new();
        h.update(payload);
        let crc = h.finalize();
        // build a single packet
        let mut pkt = Vec::with_capacity(4 + 4 + 2 + 4 + payload.len());
        pkt.extend_from_slice(b"PKT1");
        pkt.extend_from_slice(&seq.to_be_bytes());
        pkt.extend_from_slice(&(payload.len() as u16).to_be_bytes());
        pkt.extend_from_slice(&crc.to_be_bytes());
        pkt.extend_from_slice(payload);
        // repeat it
        for _ in 0..repeats {
            out.extend_from_slice(&pkt);
        }
        seq = seq.wrapping_add(1);
        offset = end;
    }
    out
}

/// Attempt to depacketize a stream produced by packetize_stream using majority voting across repeated packet instances.
/// Returns Err if no packets found at all.
pub fn depacketize_stream(buf: &[u8], _expected_repeats: usize) -> Result<Vec<u8>, ModemError> {
    let magic = b"PKT1";
    // Per seq, keep ALL observed copies but tag each with whether its payload CRC
    // matched the header CRC. CRC-valid copies are trusted absolutely; CRC-invalid
    // copies are only a fallback for the majority vote when NO clean copy survived.
    let mut map: HashMap<u32, Vec<(Vec<u8>, bool)>> = HashMap::new();
    let mut max_seq: i64 = -1;
    let mut i = 0usize;
    while i + 4 + 4 + 2 + 4 <= buf.len() {
        if &buf[i..i + 4] == magic {
            // try to parse packet header
            if i + 4 + 4 + 2 + 4 > buf.len() {
                break;
            }
            let seq = u32::from_be_bytes([buf[i + 4], buf[i + 5], buf[i + 6], buf[i + 7]]);
            let len = u16::from_be_bytes([buf[i + 8], buf[i + 9]]) as usize;
            let hdr_crc = u32::from_be_bytes([buf[i + 10], buf[i + 11], buf[i + 12], buf[i + 13]]);
            let pkt_total = 4 + 4 + 2 + 4 + len;
            if i + pkt_total > buf.len() {
                break;
            }
            let payload = &buf[i + 14..i + 14 + len];
            // Validate this copy's CRC so the reconstruction step can prefer clean copies.
            let mut h = Crc32::new();
            h.update(payload);
            let crc_ok = h.finalize() == hdr_crc;
            // store the payload (copy) with its CRC-validity tag
            map.entry(seq)
                .or_insert_with(Vec::new)
                .push((payload.to_vec(), crc_ok));
            if (seq as i64) > max_seq {
                max_seq = seq as i64;
            }
            // advance index by 1 to allow overlapping detection
            i += 1;
        } else {
            i += 1;
        }
    }

    if map.is_empty() {
        return Err(ModemError::Depacketize(
            "no PKT1 packets found in stream".to_string(),
        ));
    }

    // reconstruct by sequence order
    let mut seqs: Vec<u32> = map.keys().cloned().collect();
    seqs.sort_unstable();

    let mut out: Vec<u8> = Vec::new();

    // For each sequence, majority-vote bytes across the observed copies.
    for &seq in &seqs {
        let all_copies = &map[&seq];
        // Prefer CRC-VALID copies: a copy whose payload CRC matched its header is
        // known-intact, so if any clean copy survived we vote ONLY among clean
        // copies (a single clean copy is authoritative — it beats any number of
        // corrupt ones). Only when EVERY copy is corrupt do we fall back to voting
        // over all of them (best-effort majority). On a clean channel all copies
        // pass, so this is identical to plain majority voting — it strictly adds
        // robustness without changing clean-path behavior.
        let clean: Vec<&Vec<u8>> = all_copies
            .iter()
            .filter(|(_, ok)| *ok)
            .map(|(p, _)| p)
            .collect();
        let copies: Vec<&Vec<u8>> = if clean.is_empty() {
            all_copies.iter().map(|(p, _)| p).collect()
        } else {
            clean
        };
        // find most common length among copies
        let mut len_counts: HashMap<usize, usize> = HashMap::new();
        for c in copies.iter() {
            *len_counts.entry(c.len()).or_insert(0) += 1;
        }
        let (&chosen_len, _cnt) = len_counts.iter().max_by_key(|kv| kv.1).ok_or_else(|| {
            ModemError::Depacketize(format!("no surviving copies for seq {}", seq))
        })?;
        // prepare vector of byte-majority for each position
        let mut rec = vec![0u8; chosen_len];
        for pos in 0..chosen_len {
            let mut counts: HashMap<u8, usize> = HashMap::new();
            for c in copies.iter() {
                if pos < c.len() {
                    *counts.entry(c[pos]).or_insert(0) += 1;
                }
            }
            // pick majority value
            if counts.is_empty() {
                rec[pos] = 0u8;
            } else {
                let (&val, _c) = counts.iter().max_by_key(|kv| kv.1).ok_or_else(|| {
                    ModemError::Depacketize(format!(
                        "byte-majority vote produced no value at seq {} pos {}",
                        seq, pos
                    ))
                })?;
                rec[pos] = val;
            }
        }
        out.extend_from_slice(&rec);
    }

    Ok(out)
}

/* -----------------------------
Reed-Solomon Packetization (RS)
-----------------------------
RS packet format used here (header):
[4 bytes magic "RS01"]
[8 bytes orig_len (BE u64)]   <- original total stream length (so we can trim padding)
[4 bytes block_seq (BE u32)]
[2 bytes data_shards (BE u16)]
[2 bytes parity_shards (BE u16)]
[2 bytes shard_idx (BE u16)]
[2 bytes shard_size (BE u16)]
[4 bytes crc32 (BE u32) - crc of shard payload]
[shard payload - exactly shard_size bytes]
----------------------------- */

/// Packetize using Reed-Solomon shards. `shard_size` is bytes per shard.
/// For a block we pack `data_shards * shard_size` bytes (pad last block with zeros),
/// then create `data_shards + parity_shards` shards and emit each as a packet.
pub fn packetize_stream_rs(
    data: &[u8],
    shard_size: usize,
    data_shards: usize,
    parity_shards: usize,
) -> Result<Vec<u8>, Box<dyn Error>> {
    if data_shards == 0 || parity_shards == 0 || shard_size == 0 {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "RS params must be > 0",
        )));
    }
    let total_shards = data_shards + parity_shards;
    let block_size = data_shards * shard_size;
    let orig_len = data.len() as u64;

    let mut out: Vec<u8> = Vec::new();
    let mut block_seq: u32 = 0;
    let mut offset = 0usize;

    while offset < data.len() {
        // prepare block bytes (pad with zeros)
        let end = std::cmp::min(offset + block_size, data.len());
        let mut block = vec![0u8; block_size];
        block[..(end - offset)].copy_from_slice(&data[offset..end]);

        // split into data shards
        let mut shards: Vec<Vec<u8>> = Vec::with_capacity(total_shards);
        for d in 0..data_shards {
            let start = d * shard_size;
            shards.push(block[start..start + shard_size].to_vec());
        }
        // parity placeholders
        for _ in 0..parity_shards {
            shards.push(vec![0u8; shard_size]);
        }

        // RS encode
        let rs = ReedSolomon::new(data_shards, parity_shards)?;
        // create mutable slice refs
        let mut shard_refs: Vec<&mut [u8]> = shards.iter_mut().map(|v| v.as_mut_slice()).collect();
        rs.encode(&mut shard_refs)?;

        // emit per-shard packets with header (include orig_len so decoder can trim)
        for shard_idx in 0..total_shards {
            let payload = &shards[shard_idx];
            let mut hdr = Vec::with_capacity(4 + 8 + 4 + 2 + 2 + 2 + 2 + 4);
            hdr.extend_from_slice(b"RS01");
            hdr.extend_from_slice(&orig_len.to_be_bytes());
            hdr.extend_from_slice(&block_seq.to_be_bytes());
            hdr.extend_from_slice(&(data_shards as u16).to_be_bytes());
            hdr.extend_from_slice(&(parity_shards as u16).to_be_bytes());
            hdr.extend_from_slice(&(shard_idx as u16).to_be_bytes());
            hdr.extend_from_slice(&(shard_size as u16).to_be_bytes());
            let mut h = Crc32::new();
            h.update(payload);
            hdr.extend_from_slice(&h.finalize().to_be_bytes());

            out.extend_from_slice(&hdr);
            out.extend_from_slice(payload);
        }

        block_seq = block_seq.wrapping_add(1);
        offset += block_size;
    }

    Ok(out)
}

/// Depacketize RS stream produced by `packetize_stream_rs`.
/// Scans stream for RS packets, groups shards by block, attempts to reconstruct when at least `data_shards` present.
/// Returns reconstructed original stream trimmed to original length (using orig_len).
pub fn depacketize_stream_rs(buf: &[u8]) -> Result<Vec<u8>, ModemError> {
    let magic = b"RS01";
    #[derive(Debug, Clone)]
    struct BlockState {
        orig_len: u64,
        data_shards: usize,
        parity_shards: usize,
        shard_size: usize,
        shards: Vec<Option<Vec<u8>>>,
    }

    let mut map: HashMap<u32, BlockState> = HashMap::new();
    let header_min = 4 + 8 + 4 + 2 + 2 + 2 + 2 + 4; // 28 bytes

    let mut i = 0usize;
    while i + header_min <= buf.len() {
        if &buf[i..i + 4] == magic {
            // ensure full header available
            if i + header_min > buf.len() {
                break;
            }
            let base = i + 4;
            let orig_len = u64::from_be_bytes([
                buf[base],
                buf[base + 1],
                buf[base + 2],
                buf[base + 3],
                buf[base + 4],
                buf[base + 5],
                buf[base + 6],
                buf[base + 7],
            ]);
            let seq =
                u32::from_be_bytes([buf[base + 8], buf[base + 9], buf[base + 10], buf[base + 11]]);
            let data_shards = u16::from_be_bytes([buf[base + 12], buf[base + 13]]) as usize;
            let parity_shards = u16::from_be_bytes([buf[base + 14], buf[base + 15]]) as usize;
            let shard_idx = u16::from_be_bytes([buf[base + 16], buf[base + 17]]) as usize;
            let shard_size = u16::from_be_bytes([buf[base + 18], buf[base + 19]]) as usize;
            let crc = u32::from_be_bytes([
                buf[base + 20],
                buf[base + 21],
                buf[base + 22],
                buf[base + 23],
            ]);
            let pkt_len = header_min + shard_size;
            if i + pkt_len > buf.len() {
                // incomplete packet at end
                break;
            }
            let payload = buf[i + header_min..i + header_min + shard_size].to_vec();

            // check or create blockstate
            let entry = map.entry(seq).or_insert_with(|| BlockState {
                orig_len,
                data_shards,
                parity_shards,
                shard_size,
                shards: vec![None; data_shards + parity_shards],
            });

            // header consistency check
            if entry.data_shards != data_shards
                || entry.parity_shards != parity_shards
                || entry.shard_size != shard_size
            {
                // inconsistent -> skip packet
                i += 1;
                continue;
            }

            // store shard if missing — ENFORCING the per-shard CRC.
            //
            // Reed-Solomon erasure coding reconstructs from a subset of KNOWN-GOOD
            // shards: a shard whose payload CRC does not match its header CRC is
            // CORRUPT, and feeding a corrupt shard into rs.reconstruct() as if it
            // were intact produces a silently-wrong block (RS cannot tell a lie
            // from the truth — it trusts every shard it is given). The previous code
            // stored every shard "anyway", so a corrupt-but-header-intact shard
            // poisoned reconstruction. We now DROP CRC-mismatched shards, leaving the
            // slot `None` (a clean erasure) so RS can correct it from parity instead
            // of trusting bad data. This is what makes the interleaved-RS burst-
            // recovery path actually recover rather than reconstruct garbage.
            if shard_idx < entry.shards.len() && entry.shards[shard_idx].is_none() {
                let mut h = Crc32::new();
                h.update(&payload);
                if h.finalize() == crc {
                    entry.shards[shard_idx] = Some(payload);
                }
                // CRC mismatch -> leave as None (erasure); RS recovers from parity.
            }

            // jump forward by whole packet (we have a valid header+payload)
            i += pkt_len;
        } else {
            i += 1;
        }
    }

    if map.is_empty() {
        return Err(ModemError::ReedSolomon(
            "no RS01 packets found in stream".to_string(),
        ));
    }

    // reconstruct blocks in ascending seq order
    let mut seqs: Vec<u32> = map.keys().cloned().collect();
    seqs.sort_unstable();
    let mut assembled: Vec<u8> = Vec::new();
    let mut overall_orig_len: Option<u64> = None;

    for seq in seqs {
        // `seq` came from map.keys(), so the entry is present; treat a missing entry
        // as a typed error rather than panicking.
        let state = map.remove(&seq).ok_or_else(|| {
            ModemError::ReedSolomon(format!("missing block state for seq {}", seq))
        })?;
        if overall_orig_len.is_none() {
            overall_orig_len = Some(state.orig_len);
        }

        let data_shards = state.data_shards;
        let parity_shards = state.parity_shards;
        let total_shards = data_shards + parity_shards;
        let shard_size = state.shard_size;

        // count present shards
        let present = state.shards.iter().filter(|s| s.is_some()).count();
        if present < data_shards {
            return Err(ModemError::ReedSolomon(format!(
                "not enough shards for block {}: have {}, need {}",
                seq, present, data_shards
            )));
        }

        // Convert to Vec<Option<Vec<u8>>> for crate
        let mut shards_opt = state.shards.clone();

        // reconstruct
        let rs = ReedSolomon::new(data_shards, parity_shards)?;
        rs.reconstruct(&mut shards_opt)?;

        // append first data_shards to assembled bytes
        for d in 0..data_shards {
            if let Some(sh) = &shards_opt[d] {
                assembled.extend_from_slice(sh);
            } else {
                // shouldn't happen
                assembled.extend_from_slice(&vec![0u8; shard_size]);
            }
        }
    }

    // trim to original length (orig_len came from packet headers)
    if let Some(olen) = overall_orig_len {
        let olen_usz = olen as usize;
        if assembled.len() > olen_usz {
            assembled.truncate(olen_usz);
        }
    }

    Ok(assembled)
}

/// Small wrapper describing a single RS shard "packet" as bytes plus addressing metadata.
#[derive(Clone)]
pub struct ShardPacket {
    pub block_idx: u32,
    pub shard_idx: u32,
    pub total_shards: u32,
    pub data_shards: u32,
    pub shard_size: u32,
    pub bytes: Vec<u8>, // header (28 bytes) + shard payload
}

/// Interleave order: 0..total_shards across all blocks, then next shard index, etc.
fn interleave_packets(blocks: &Vec<Vec<ShardPacket>>) -> Vec<u8> {
    if blocks.is_empty() {
        return Vec::new();
    }
    let total_shards = blocks[0].len();
    let mut out = Vec::with_capacity(
        blocks
            .iter()
            .map(|b| b.iter().map(|s| s.bytes.len()).sum::<usize>())
            .sum(),
    );
    for s in 0..total_shards {
        for b in 0..blocks.len() {
            if s < blocks[b].len() {
                out.extend_from_slice(&blocks[b][s].bytes);
            }
        }
    }
    out
}

/// Interleaved RS packetizer that emits the *same per-shard header format* as packetize_stream_rs,
/// but reorders (interleaves) shards across blocks to spread burst losses.
pub fn packetize_stream_rs_interleaved(
    payload: &[u8],
    data_shards: usize,
    parity_shards: usize,
    shard_size: usize,
) -> Vec<u8> {
    // Validate a few things early (silently return empty on bad params)
    if data_shards == 0 || parity_shards == 0 || shard_size == 0 {
        return Vec::new();
    }

    let d = data_shards;
    let p = parity_shards;
    let t = d + p;
    let block_bytes = d * shard_size;
    let orig_len = payload.len() as u64;

    // Split payload into blocks (pad last block)
    let mut blocks_payloads: Vec<Vec<u8>> = Vec::new();
    let mut offset = 0usize;
    while offset < payload.len() {
        let end = std::cmp::min(offset + block_bytes, payload.len());
        let mut block = vec![0u8; block_bytes];
        block[..(end - offset)].copy_from_slice(&payload[offset..end]);
        blocks_payloads.push(block);
        offset += end - offset;
    }
    // If payload was empty, still create one empty block so we encode headers
    if blocks_payloads.is_empty() {
        blocks_payloads.push(vec![0u8; block_bytes]);
    }

    // Prepare ReedSolomon instance once
    let rs = match ReedSolomon::new(d, p) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };

    // For each block, create t shards, RS-encode, then serialize each shard using the SAME header
    // format as packetize_stream_rs (28-byte header: RS01 + orig_len(u64) + block_seq(u32) + data_shards(u16) +
    // parity_shards(u16) + shard_idx(u16) + shard_size(u16) + crc(u32) )
    let mut per_block_packets: Vec<Vec<ShardPacket>> = Vec::with_capacity(blocks_payloads.len());

    for (block_seq, block_payload) in blocks_payloads.iter().enumerate() {
        // Build shard buffers (t shards of shard_size bytes)
        let mut shards: Vec<Vec<u8>> = (0..t).map(|_| vec![0u8; shard_size]).collect();

        // fill data shards from block payload
        for s in 0..d {
            let start = s * shard_size;
            let end = start + shard_size;
            shards[s][..].copy_from_slice(&block_payload[start..end]);
        }

        // compute parity in-place
        {
            let mut refs: Vec<&mut [u8]> = shards.iter_mut().map(|v| v.as_mut_slice()).collect();
            if rs.encode(&mut refs).is_err() {
                // encoding error: skip block
                continue;
            }
        }

        // serialize shards with matching header
        let mut shard_packets: Vec<ShardPacket> = Vec::with_capacity(t);
        for shard_idx in 0..t {
            let payload_bytes = &shards[shard_idx];

            // build header exactly like packetize_stream_rs
            let mut hdr = Vec::with_capacity(28);
            hdr.extend_from_slice(b"RS01");
            hdr.extend_from_slice(&orig_len.to_be_bytes());
            hdr.extend_from_slice(&(block_seq as u32).to_be_bytes());
            hdr.extend_from_slice(&(d as u16).to_be_bytes());
            hdr.extend_from_slice(&(p as u16).to_be_bytes());
            hdr.extend_from_slice(&(shard_idx as u16).to_be_bytes());
            hdr.extend_from_slice(&(shard_size as u16).to_be_bytes());
            let mut hcrc = Crc32::new();
            hcrc.update(&payload_bytes);
            hdr.extend_from_slice(&hcrc.finalize().to_be_bytes());

            let mut pkt_bytes = hdr;
            pkt_bytes.extend_from_slice(payload_bytes);

            shard_packets.push(ShardPacket {
                block_idx: block_seq as u32,
                shard_idx: shard_idx as u32,
                total_shards: t as u32,
                data_shards: d as u32,
                shard_size: shard_size as u32,
                bytes: pkt_bytes,
            });
        }

        per_block_packets.push(shard_packets);
    }

    // Interleave across blocks and return concatenated serialized packets
    interleave_packets(&per_block_packets)
}

/// Estimate duration (seconds) given payload length and modem params.
/// This matches the math you’ve been logging so estimates are very close to real output.
pub struct DurationEstimate {
    pub encoded_bytes: usize,
    pub symbols_total: usize,
    pub seconds: f64,
}

pub fn estimate_duration_seconds(
    payload_len: usize,
    rs_data: usize,
    rs_parity: usize,
    rs_shard_size: usize,
    m_tones: usize,
    channels: usize,
    symbol_ms: usize,
    preamble_symbols_per_channel: usize, // pass your real preamble (e.g., 8)
) -> DurationEstimate {
    let d = rs_data;
    let p = rs_parity;
    let t = d + p;
    let block_bytes = d * rs_shard_size;

    // number of blocks
    let mut blocks = payload_len / block_bytes;
    if payload_len % block_bytes != 0 {
        blocks += 1;
    }

    // shard packet size (your observed was 28 bytes header + shard_size)
    let shard_packet_bytes = 28 + rs_shard_size;

    // encoded bytes (headers + data + parity)
    let encoded_bytes = blocks * t * shard_packet_bytes;

    // bits per symbol
    let bps = bits_per_symbol(m_tones);
    let bits_per_symbol = if bps == 0 { 1 } else { bps };

    // symbols for payload
    let symbols_payload = (encoded_bytes * 8 + bits_per_symbol - 1) / bits_per_symbol;

    // add preamble (per channel; encoder adds the same count on each)
    let symbols_total = symbols_payload + preamble_symbols_per_channel * channels;

    // samples per symbol and total seconds
    let samples_per_symbol = (48_000usize * symbol_ms) / 1000; // your default SR = 48 kHz
    let total_samples = symbols_total * samples_per_symbol;
    let seconds = total_samples as f64 / 48_000f64;

    DurationEstimate {
        encoded_bytes,
        symbols_total,
        seconds,
    }
}

/// Simple channel simulator operating on the serialized packet-bytes stream.
/// - `random_flip_prob`: per-byte probability to flip a single random bit (0.0..1.0)
/// - `burst_erase_prob`: per-byte probability to start a burst erase (remove bytes)
/// - `avg_burst_len`: average length (bytes) of a burst erase
///
/// This is intentionally simple and byte-oriented. Removing bytes simulates lost/erased packets
/// or continuous loss. The RS depacketizer is tolerant because it scans for RS headers.
/// Use this in tests to quantify the benefit of interleaving + RS over burst losses.
pub fn simulate_channel_bytes(
    input: &[u8],
    random_flip_prob: f64,
    burst_erase_prob: f64,
    avg_burst_len: usize,
) -> Vec<u8> {
    // Safety clamp params
    let rf = if random_flip_prob < 0.0 {
        0.0
    } else if random_flip_prob > 1.0 {
        1.0
    } else {
        random_flip_prob
    };
    let bp = if burst_erase_prob < 0.0 {
        0.0
    } else if burst_erase_prob > 1.0 {
        1.0
    } else {
        burst_erase_prob
    };
    let avg_burst = if avg_burst_len == 0 {
        1usize
    } else {
        avg_burst_len
    };

    // copy input
    let mut out: Vec<u8> = input.to_vec();

    // Flip random bits
    if rf > 0.0 {
        let mut rng = OsRng;
        let thresh = ((u64::MAX as f64) * rf) as u64;
        for i in 0..out.len() {
            let r = rng.next_u64();
            if r <= thresh {
                // flip a single random bit in this byte
                let bit = (rng.next_u32() % 8) as u8;
                out[i] ^= 1u8 << bit;
            }
        }
    }

    // Burst erasures: iterate with index; when a burst starts, remove a random length around avg_burst
    if bp > 0.0 && out.len() > 0 {
        let mut rng = OsRng;
        let thresh = ((u64::MAX as f64) * bp) as u64;
        let mut i = 0usize;
        while i < out.len() {
            let r = rng.next_u64();
            if r <= thresh {
                // start a burst erase here
                // length: 1 .. (2 * avg_burst)
                let len = 1usize + ((rng.next_u32() as usize) % (avg_burst * 2));
                let end = std::cmp::min(i + len, out.len());
                // remove bytes i..end
                out.drain(i..end);
                // continue at same index (which now points to next byte after erased region)
            } else {
                i += 1;
            }
        }
    }

    out
}

// ============================================================================
// ERROR-PATH UNIT TESTS (WS-2 Phase B — the negative/Err-path net)
//
// Phase A's tests/modem_roundtrip.rs is a POSITIVE byte-identity net. These tests
// pin the NEW enforcing behavior: corrupt/truncated/malformed input must yield a
// typed `ModemError` (never silently-wrong bytes, never a panic), and a clean
// frame must still round-trip Ok (so enforcement does not over-fire).
//
// Everything runs IN MEMORY: no WAV files, no filesystem, no audio hardware.
// Each test's top comment names the property it validates.
// ============================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use rand::RngCore;
    use rand::SeedableRng;

    // Fixed 32-byte (64 hex char) AES-256 key for deterministic encrypted-frame tests.
    const TEST_KEY_HEX: &str = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";

    /// Deterministic pseudo-random payload, seeded for stable tests.
    fn seeded_payload(seed: u64, len: usize) -> Vec<u8> {
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed);
        let mut v = vec![0u8; len];
        rng.fill_bytes(&mut v);
        v
    }

    /// Property: a plain frame whose payload region is corrupted (one byte flipped
    /// AFTER build_frame) is REJECTED with `CrcMismatch` — extract_frame returns Err,
    /// not Ok-with-garbage. This is the core CRC-enforcement guarantee.
    #[test]
    fn test_corrupted_payload_rejected_with_crc_mismatch() {
        let payload = seeded_payload(7, 256);
        let mut frame = build_frame("corrupt.bin", &payload, false, None).expect("build_frame");

        // Locate the payload start via the header and flip a byte inside the payload.
        let (_f, _c, _e, start, len, _crc) = parse_frame_header(&frame).expect("parse header");
        assert!(len > 0);
        frame[start] ^= 0xFF; // corrupt first payload byte

        match extract_frame(&frame, None) {
            Err(ModemError::CrcMismatch { expected, computed }) => {
                assert_ne!(expected, computed, "CRC values must differ on corruption");
            }
            other => panic!("expected CrcMismatch, got {:?}", other),
        }
    }

    /// Property: a buffer shorter than the fixed header minimum does NOT panic; it
    /// returns a header/truncation Err from both parse_frame_header and extract_frame.
    #[test]
    fn test_short_buffer_returns_err_not_panic() {
        let tiny = [b'A', b'H', b'X']; // 3 bytes — below the fixed-header minimum
        match parse_frame_header(&tiny) {
            Err(ModemError::BadHeader(_)) => {}
            other => panic!("expected BadHeader on short buffer, got {:?}", other),
        }
        // extract_frame must surface the same class of error, not panic.
        assert!(matches!(
            extract_frame(&tiny, None),
            Err(ModemError::BadHeader(_))
        ));
    }

    /// Property: a well-sized buffer whose declared payload_len overruns the actual
    /// buffer is reported as `Truncated`, not an out-of-bounds index panic.
    #[test]
    fn test_truncated_payload_returns_truncated_err() {
        let payload = seeded_payload(8, 64);
        let frame = build_frame("trunc.bin", &payload, false, None).expect("build_frame");
        // Chop off the tail so the header still parses but the payload is incomplete.
        let chopped = &frame[..frame.len() - 10];
        match parse_frame_header(chopped) {
            Err(ModemError::Truncated(_)) => {}
            other => panic!("expected Truncated, got {:?}", other),
        }
    }

    /// Property: a frame with bad magic bytes yields the BadHeader Err (not a panic,
    /// not a CRC walk over garbage).
    #[test]
    fn test_bad_magic_returns_bad_header() {
        let payload = seeded_payload(9, 64);
        let mut frame = build_frame("magic.bin", &payload, false, None).expect("build_frame");
        frame[0] = b'X'; // break "AHX1"
        match extract_frame(&frame, None) {
            Err(ModemError::BadHeader(_)) => {}
            other => panic!("expected BadHeader on bad magic, got {:?}", other),
        }
    }

    /// Property: decrypting an encrypted frame with the WRONG key fails gracefully
    /// with a Decrypt Err (AES-GCM auth failure) — never a panic, never garbage.
    #[test]
    fn test_wrong_decrypt_key_returns_decrypt_err() {
        let payload = seeded_payload(10, 128);
        let frame =
            build_frame("enc.bin", &payload, false, Some(TEST_KEY_HEX)).expect("build_frame");
        // A different valid 32-byte key.
        let wrong_key = "ffeeddccbbaa99887766554433221100ffeeddccbbaa99887766554433221100";
        match extract_frame(&frame, Some(wrong_key)) {
            Err(ModemError::Decrypt(_)) => {}
            other => panic!("expected Decrypt on wrong key, got {:?}", other),
        }
    }

    /// Property: an encrypted frame extracted with NO key returns MissingKey, not a panic.
    #[test]
    fn test_encrypted_frame_without_key_returns_missing_key() {
        let payload = seeded_payload(11, 96);
        let frame =
            build_frame("enc2.bin", &payload, false, Some(TEST_KEY_HEX)).expect("build_frame");
        assert!(matches!(
            extract_frame(&frame, None),
            Err(ModemError::MissingKey)
        ));
    }

    /// Property: depacketize_stream on a buffer containing NO PKT1 magic returns
    /// Depacketize Err (no copies to vote over) instead of hitting a `.unwrap()` panic.
    #[test]
    fn test_depacketize_repetition_no_packets_returns_err() {
        let junk = seeded_payload(12, 200); // random bytes, vanishingly unlikely to contain PKT1
        match depacketize_stream(&junk, 3) {
            Err(ModemError::Depacketize(_)) => {}
            // If by astronomically-low chance random bytes contained a PKT1 frame, an Ok
            // is still non-panicking; assert no panic by accepting Ok too.
            Ok(_) => {}
            other => panic!("expected Depacketize Err or Ok, got {:?}", other),
        }
    }

    /// Property: depacketize_stream_rs on a buffer with no RS01 magic returns a
    /// ReedSolomon Err — exercises the malformed-stream path that previously reached
    /// the `map.remove(..).unwrap()` site only via valid input. No panic.
    #[test]
    fn test_depacketize_rs_no_packets_returns_err() {
        let junk = seeded_payload(13, 200);
        match depacketize_stream_rs(&junk) {
            Err(ModemError::ReedSolomon(_)) => {}
            Ok(_) => {}
            other => panic!("expected ReedSolomon Err or Ok, got {:?}", other),
        }
    }

    /// Property (POSITIVE GUARD): a clean, uncorrupted frame STILL round-trips Ok
    /// across all four flag combinations — CRC enforcement must not over-fire and
    /// reject valid frames. Mirrors Phase A intent at the unit level.
    #[test]
    fn test_clean_frame_still_round_trips_ok() {
        let payload = seeded_payload(14, 300);
        let cases: [(bool, Option<&str>); 4] = [
            (false, None),
            (true, None),
            (false, Some(TEST_KEY_HEX)),
            (true, Some(TEST_KEY_HEX)),
        ];
        for (compress, key) in cases {
            let frame = build_frame("clean.bin", &payload, compress, key).expect("build_frame");
            let (fname, recovered) =
                extract_frame(&frame, key).expect("clean frame must extract Ok");
            assert_eq!(fname, "clean.bin");
            assert_eq!(
                recovered, payload,
                "clean payload must round-trip (compress={compress})"
            );
        }
    }

    // ========================================================================
    // WS-2 ACOUSTIC-HARDENING RED PASS (S5) — frequency-plan property tests.
    //
    // These pin POST-FIX behavior of the default frequency plan and are RED on
    // HEAD. They use only `pub` fns (build_tone_frequencies, render_symbols_to_samples,
    // goertzel_mag_squared) — no internal API exposure is required from the
    // Signal Processing Specialist for these two. Everything is in-memory.
    // ========================================================================

    /// Render a single channel's single tone (one symbol, no preamble) into i16
    /// samples by zeroing every other channel. Returns the i16 buffer for one
    /// symbol window. Helper for the band-isolation test.
    fn render_single_tone(params: &ModemParams, channel: usize, tone: u8) -> Vec<i16> {
        // Build a per-channel symbol stream where only `channel` carries `tone`
        // for a single symbol; all other channels carry nothing (empty), so
        // render_symbols_to_samples emits silence for them.
        let mut channels_symbols: Vec<Vec<u8>> = vec![Vec::new(); params.channels];
        channels_symbols[channel] = vec![tone];
        render_symbols_to_samples(&channels_symbols, params)
    }

    /// Property (RED NOW): the per-channel tone bands must be ISOLATED — energy
    /// rendered into ONE channel's band must NOT register strongly in an ADJACENT
    /// channel's detector. The decode-time failure mode is concrete: the ch1
    /// detector scans ALL of ch1's tone frequencies and picks the max-energy one;
    /// if ch0's emitted tone frequency coincides with (or sits very close to) ANY
    /// ch1 tone frequency, ch1 falsely detects strong energy and mis-decodes.
    ///
    /// We render ONLY channel 0's upper-range tone, then compute the STRONGEST
    /// Goertzel response that the adjacent channel's detector would see across all
    /// of ch1's tone frequencies, and compare it to ch0's own in-band response.
    /// Band isolation requires the adjacent channel's best response be a small
    /// fraction of the in-band response.
    ///
    /// On HEAD the default plan (base 400, channel_spacing 400, tone_spacing 30,
    /// 32 tones) makes ch0's band (400..1330 Hz) overlap ch1's band (800..1730 Hz):
    /// ch0's upper tones land exactly on ch1 tone frequencies, so ch1's best
    /// response RIVALS the in-band one — RED. Once the plan separates the bands,
    /// ch1's best response collapses — GREEN.
    #[test]
    fn test_channel_band_isolation_default_params() {
        let params = ModemParams::default();
        assert!(
            params.channels >= 2,
            "need >= 2 channels to test adjacent-band isolation"
        );

        let freqs = build_tone_frequencies(&params);
        // Use an upper-range tone of channel 0 so on HEAD it falls inside ch1's band.
        let tone: u8 = (params.m_tones as u8) * 3 / 4; // upper quartile tone index
        let tone = tone.min((params.m_tones - 1) as u8);

        let samples = render_single_tone(&params, 0, tone);
        assert!(!samples.is_empty(), "render produced no samples");

        // In-band: Goertzel of ch0 at the emitted tone's own frequency.
        let f_inband = freqs[0][tone as usize];
        let mag_inband = goertzel_mag_squared(&samples, f_inband, params.sample_rate);
        assert!(
            mag_inband > 0.0,
            "in-band Goertzel response must be positive (got {mag_inband})"
        );

        // Adjacent: the STRONGEST response the ch1 detector would see across ALL
        // of ch1's tone frequencies (this is exactly what the decoder does).
        let mut mag_adjacent_best = 0f32;
        let mut f_adjacent_best = 0f32;
        for &f in &freqs[1] {
            let mag = goertzel_mag_squared(&samples, f, params.sample_rate);
            if mag > mag_adjacent_best {
                mag_adjacent_best = mag;
                f_adjacent_best = f;
            }
        }

        // Require strong isolation: adjacent channel's BEST response < 10% of in-band.
        assert!(
            mag_adjacent_best < 0.10 * mag_inband,
            "adjacent-channel band must isolate ch0 energy: ch0 tone {tone} (f={f_inband:.1} Hz) \
             leaks into ch1 — ch1's strongest detector response is at f={f_adjacent_best:.1} Hz \
             with mag={mag_adjacent_best:.3e} vs in-band mag={mag_inband:.3e} \
             ({:.1}% of in-band). RED until the default frequency plan separates the \
             per-channel bands.",
            100.0 * mag_adjacent_best / mag_inband
        );
    }

    /// Property (RED NOW): every tone frequency produced by the DEFAULT plan must
    /// sit CLEAR of the FluidSynth MIDI music band so the modem does not collide
    /// with the image-to-music engine in a shared acoustic environment. We assert
    /// every tone frequency is >= a stated spec floor of 2500 Hz (above the
    /// ~65–2000 Hz MIDI music band, with margin). The Signal Processing Specialist
    /// chooses the actual plan; this only pins the clearance requirement.
    ///
    /// On HEAD the default plan starts at base_freq_hz=400 Hz and tones run up
    /// through ~1730 Hz — squarely INSIDE the music band — so this is RED until
    /// the plan is moved above the floor.
    #[test]
    fn test_default_tones_clear_of_music_band() {
        // Spec floor: modem tones must live at or above this to clear the
        // FluidSynth MIDI music band (~65..2000 Hz) with margin.
        const MUSIC_CLEAR_FLOOR_HZ: f32 = 2500.0;

        let params = ModemParams::default();
        let freqs = build_tone_frequencies(&params);
        let mut min_freq = f32::INFINITY;
        for (ch, ch_freqs) in freqs.iter().enumerate() {
            for (tone, &f) in ch_freqs.iter().enumerate() {
                if f < min_freq {
                    min_freq = f;
                }
                assert!(
                    f >= MUSIC_CLEAR_FLOOR_HZ,
                    "default tone (ch {ch}, tone {tone}) = {f:.1} Hz is BELOW the music-band \
                     clearance floor of {MUSIC_CLEAR_FLOOR_HZ} Hz — it would collide with the \
                     FluidSynth MIDI music band. RED until the default frequency plan is moved \
                     clear of the music band."
                );
            }
        }
        assert!(
            min_freq >= MUSIC_CLEAR_FLOOR_HZ,
            "lowest default tone {min_freq:.1} Hz must be >= {MUSIC_CLEAR_FLOOR_HZ} Hz"
        );
    }
}

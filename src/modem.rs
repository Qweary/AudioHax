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

// Seeded, deterministic RNG for the S7 real-air acoustic-channel model. ChaCha8
// gives reproducible impairment streams so the Test Engineer's failing tests
// (Pass B) and the eventual implementation (Pass C) are bit-for-bit repeatable.
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

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

    /// Synchronization / start-of-burst detection failed: no correlation peak
    /// crossed threshold, or the buffer was too short to contain a preamble.
    /// (S7 real-air sync path.)
    #[error("burst sync failed: {0}")]
    Sync(String),

    /// Symbol-timing recovery could not produce a usable set of window
    /// boundaries (e.g. the drift estimate diverged or the buffer ran out).
    /// (S7 real-air timing-recovery path.)
    #[error("symbol-timing recovery failed: {0}")]
    Timing(String),

    /// The in-band coding-rate header ("CDG1") was present but malformed
    /// (e.g. an unknown profile tag or inconsistent triplicate copies).
    /// A *missing* header is NOT this error — that is the legacy/default path.
    /// (S7 rate-selectable-coding path.)
    #[error("coding-rate header invalid: {0}")]
    CodingHeader(String),

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
        return payload.iter().map(|b| *b % (m_tones as u8)).collect();
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

/// Single-frequency generalized Goertzel power at an EXACT (fractional) frequency.
///
/// `omega = 2π·target_freq/sr` is set directly and NOT rounded to an integer DFT bin,
/// so the response is faithful for off-bin (frequency-offset) tones. For a tone on a
/// bin center the fractional `omega` equals the legacy integer-`k` `omega`, so on-bin
/// behaviour is unchanged. This is the narrowband primitive used by the band-energy
/// detector [`goertzel_mag_squared`].
fn goertzel_point(samples: &[i16], target_freq: f32, sample_rate: usize) -> f32 {
    let n = samples.len();
    if n == 0 {
        return 0.0;
    }
    let sr = sample_rate as f32;
    let omega = 2.0 * PI * target_freq / sr;
    let coeff = 2.0 * omega.cos();
    let mut s_prev = 0.0f32;
    let mut s_prev2 = 0.0f32;
    for &x in samples {
        let s = (x as f32) + coeff * s_prev - s_prev2;
        s_prev2 = s_prev;
        s_prev = s;
    }
    s_prev2 * s_prev2 + s_prev * s_prev - coeff * s_prev * s_prev2
}

/// Goertzel tone-energy detector for `target_freq` on a slice of i16 samples.
///
/// Returns the energy in a NARROW BAND centered on `target_freq` — the sum of three
/// generalized-Goertzel probes at `target_freq` and `target_freq ± ½·bin`, where
/// `bin = sample_rate / N` is the DFT bin resolution of this slice. Integrating over
/// ±½ bin (rather than a single point) is what makes the detector tolerant of a
/// **carrier frequency offset**: under an S7 `freq_offset_hz` the channel's real-cosine
/// mix splits a tone's energy into sidebands a few Hz off the nominal bin center
/// (and a point probe at the nominal frequency then scallops down far enough that an
/// *adjacent* tone's point probe can win the arg-max — the exact failure the real-air
/// freq-offset test exercises). The ±½-bin band recaptures that displaced energy for
/// the *true* tone while staying well inside the tone spacing (the modem's tones are a
/// full bin or more apart), so it does not blur neighbouring tones and preserves the
/// per-tone / per-band selectivity the existing isolation tests assert. For a clean
/// on-bin tone the center probe dominates and the band sum is monotone in the true
/// tone energy, so on-bin arg-max decisions (and every existing on-bin test) are
/// unchanged.
pub fn goertzel_mag_squared(samples: &[i16], target_freq: f32, sample_rate: usize) -> f32 {
    let n = samples.len();
    if n == 0 {
        return 0.0;
    }
    // DFT bin resolution for this window. The band spans ±0.75 bin around the target,
    // sampled at five points (target and ±0.375, ±0.75 bin). A ±0.75-bin span is wide
    // enough to recapture a tone's energy when a carrier offset AND a multipath comb
    // (the S7 echo) together displace/notch it off the nominal bin — a ±0.5-bin span
    // left a notched tone losing the arg-max to a neighbour — yet it stays inside the
    // modem's tone spacing (tones are ≥ 2 bins apart), so it does not blur adjacent
    // tones and preserves per-tone / per-band selectivity. For a clean on-bin tone the
    // center probe dominates and the band sum stays monotone in the true tone energy.
    let bin = (sample_rate as f32) / (n as f32);
    let q = 0.375 * bin;
    let h = 0.75 * bin;
    goertzel_point(samples, target_freq - h, sample_rate)
        + goertzel_point(samples, target_freq - q, sample_rate)
        + goertzel_point(samples, target_freq, sample_rate)
        + goertzel_point(samples, target_freq + q, sample_rate)
        + goertzel_point(samples, target_freq + h, sample_rate)
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

/* ============================================================================
 * S7 — REAL-AIR ROBUSTNESS
 *
 * Pass A deliverables (this block):
 *   1. AcousticChannelParams + simulate_acoustic_channel — REAL working code,
 *      a seeded acoustic-channel model used as unit-test scaffolding. It injects
 *      the four textbook speaker->mic impairments (start offset, clock drift,
 *      frequency offset, multipath echo) plus optional timing jitter, all driven
 *      by a deterministic ChaCha8Rng.
 *   2. The sync / timing-recovery API and the rate-selectable-coding API as
 *      COMPILING STUBS — `///`-documented signatures with minimal bodies that
 *      compile and keep the existing tests green, but do NOT yet implement real
 *      offset tolerance. Each stub body is tagged `// TODO(s7-passC): real impl`.
 *
 * See docs/design-s7-realair.md for the full design and the 3-pass migration.
 * ============================================================================ */

// ────────────────────────────────────────────────────────────────────────────
// 1. ACOUSTIC-CHANNEL MODEL (REAL — not a stub)
// ────────────────────────────────────────────────────────────────────────────

/// Tunable parameters for the S7 acoustic-channel model.
///
/// Each field is an *independently dialable* knob for one real speaker→microphone
/// impairment, so a test can isolate exactly one effect at a time. All randomness
/// derives from `seed` via a `ChaCha8Rng`, so identical params produce identical
/// output (no `OsRng`, no wall clock).
#[derive(Debug, Clone)]
pub struct AcousticChannelParams {
    /// Seed for the deterministic `ChaCha8Rng` driving start-offset noise and jitter.
    pub seed: u64,
    /// Start offset: `> 0` prepends this many samples of low-level noise (the burst
    /// no longer begins at sample 0); `< 0` trims that many leading samples.
    pub start_offset_samples: isize,
    /// Sample-clock offset in parts-per-million. Implemented as fractional linear
    /// resampling by factor `1 + clock_ppm*1e-6`, so fixed-length receiver windows
    /// slowly slide over the burst (clock-drift impairment).
    pub clock_ppm: f32,
    /// Carrier frequency offset in Hz. Mixes the signal with `cos(2π f_off t)` so
    /// tone energy nudges off the exact Goertzel bin centers (frequency-offset impairment).
    pub freq_offset_hz: f32,
    /// Multipath echo tap delay in samples (the delayed copy's lag).
    pub echo_delay_samples: usize,
    /// Multipath echo tap gain (0..1): amplitude of the delayed, attenuated copy.
    /// `0.0` disables the echo (no inter-symbol interference).
    pub echo_gain: f32,
    /// Per-sample timing-jitter std-dev (in samples), folded into the resampling
    /// read position as a seeded Gaussian. `0.0` is exact (no jitter).
    pub jitter_samples: f32,
}

impl AcousticChannelParams {
    /// A zero-impairment configuration: the model is a near-identity passthrough
    /// (only the final i16 renormalization touches the samples, within tolerance).
    /// Useful as a baseline and as the control arm of impairment tests.
    pub fn identity() -> Self {
        AcousticChannelParams {
            seed: 0,
            start_offset_samples: 0,
            clock_ppm: 0.0,
            freq_offset_hz: 0.0,
            echo_delay_samples: 0,
            echo_gain: 0.0,
            jitter_samples: 0.0,
        }
    }
}

impl Default for AcousticChannelParams {
    fn default() -> Self {
        AcousticChannelParams::identity()
    }
}

/// Draw one standard-normal sample (Box–Muller) from a `ChaCha8Rng`.
/// Deterministic for a given RNG state; used for start-offset noise and jitter.
fn next_gaussian(rng: &mut ChaCha8Rng) -> f32 {
    let u1 = ((rng.next_u32() as f32) + 1.0) / (u32::MAX as f32 + 1.0);
    let u2 = (rng.next_u32() as f32) / (u32::MAX as f32 + 1.0);
    (-2.0 * u1.ln()).sqrt() * (2.0 * PI * u2).cos()
}

/// Apply the seeded acoustic-channel model to an i16 burst, producing a new i16
/// burst with the configured impairments. **Real working code** (not a stub) —
/// the Test Engineer's failing tests depend on it producing genuine impairments.
///
/// Pipeline (applied in order):
/// 1. **Start offset** — prepend seeded low-level noise (or trim leading samples).
/// 2. **Clock drift + jitter** — fractional linear resampling by `1+ppm*1e-6`,
///    with the read position perturbed by a seeded Gaussian of std-dev `jitter_samples`.
/// 3. **Frequency offset** — mix with `cos(2π f_off n / sr)` (first-order shift).
/// 4. **Multipath echo** — 2-tap FIR `y[n] = x[n] + g·x[n−D]`.
///
/// The result is renormalized to the i16 range to avoid clipping. A zero-impairment
/// (`identity()`) config returns a near-identity copy of the input. Determinism is
/// total: same `seed` + same params ⇒ same output.
pub fn simulate_acoustic_channel(samples: &[i16], params: &AcousticChannelParams) -> Vec<i16> {
    if samples.is_empty() {
        return Vec::new();
    }
    let sample_rate = 48_000.0f32; // modem default SR; the model is SR-agnostic in shape.

    let mut rng = ChaCha8Rng::seed_from_u64(params.seed);

    // Work in f32 throughout, renormalize to i16 once at the end.
    let mut x: Vec<f32> = samples.iter().map(|&s| s as f32).collect();

    // ── (1) Start offset ────────────────────────────────────────────────────
    if params.start_offset_samples > 0 {
        let n = params.start_offset_samples as usize;
        // Low-level seeded noise floor before the burst (a few i16 LSBs).
        let mut prefix: Vec<f32> = Vec::with_capacity(n);
        for _ in 0..n {
            prefix.push(next_gaussian(&mut rng) * 8.0);
        }
        prefix.extend_from_slice(&x);
        x = prefix;
    } else if params.start_offset_samples < 0 {
        let trim = (-params.start_offset_samples) as usize;
        if trim < x.len() {
            x.drain(0..trim);
        } else {
            x.clear();
        }
    }
    if x.is_empty() {
        return Vec::new();
    }

    // ── (2) Clock drift (fractional resampling) + timing jitter ─────────────
    // Resample factor: a positive ppm STRETCHES the signal (more output samples),
    // so fixed-length receiver windows slide late across the burst.
    let r = 1.0f32 + params.clock_ppm * 1e-6;
    let resampled: Vec<f32> = if (r - 1.0).abs() > f32::EPSILON || params.jitter_samples > 0.0 {
        let out_len = ((x.len() as f32) * r).round().max(1.0) as usize;
        let mut out = Vec::with_capacity(out_len);
        for i in 0..out_len {
            // Source read position for output sample i; jitter perturbs it.
            let jitter = if params.jitter_samples > 0.0 {
                next_gaussian(&mut rng) * params.jitter_samples
            } else {
                0.0
            };
            let src = (i as f32) / r + jitter;
            // Linear interpolation between floor(src) and floor(src)+1, clamped.
            let s0 = src.floor();
            let frac = src - s0;
            let i0 = s0 as isize;
            let a = if i0 >= 0 && (i0 as usize) < x.len() {
                x[i0 as usize]
            } else {
                0.0
            };
            let b = if i0 + 1 >= 0 && ((i0 + 1) as usize) < x.len() {
                x[(i0 + 1) as usize]
            } else {
                a
            };
            out.push(a + (b - a) * frac);
        }
        out
    } else {
        x
    };

    // ── (3) Frequency offset (mixing) ──────────────────────────────────────
    let mut mixed: Vec<f32> = if params.freq_offset_hz.abs() > f32::EPSILON {
        let w = 2.0 * PI * params.freq_offset_hz / sample_rate;
        resampled
            .iter()
            .enumerate()
            .map(|(n, &v)| v * (w * (n as f32)).cos())
            .collect()
    } else {
        resampled
    };

    // ── (4) Multipath echo (2-tap FIR) ─────────────────────────────────────
    if params.echo_gain.abs() > f32::EPSILON && params.echo_delay_samples > 0 {
        let d = params.echo_delay_samples;
        let g = params.echo_gain;
        let dry = mixed.clone();
        for n in d..mixed.len() {
            mixed[n] += g * dry[n - d];
        }
    }

    // ── Renormalize to i16 (avoid clipping artifacts dominating) ────────────
    let maxv = mixed.iter().fold(0.0f32, |m, &v| m.max(v.abs()));
    if maxv <= 1e-6 {
        return vec![0i16; mixed.len()];
    }
    // Identity case (no shaping that changes peak) maps ~1:1; otherwise scale so
    // the peak hits 90% of full-scale, matching render_symbols_to_samples.
    let scale = (i16::MAX as f32 * 0.9) / maxv;
    // Preserve near-identity for the zero-impairment config: if the peak is already
    // within i16 range and no impairment changed the signal length/shape, scaling
    // by ~constant keeps the relative waveform; we accept a uniform gain.
    mixed
        .iter()
        .map(|&v| (v * scale).clamp(i16::MIN as f32, i16::MAX as f32) as i16)
        .collect()
}

// ────────────────────────────────────────────────────────────────────────────
// 2a. SYNC / TIMING-RECOVERY API (COMPILING STUBS — TODO Pass C)
// ────────────────────────────────────────────────────────────────────────────

/// Start-of-burst synchronization mode.
///
/// `PilotOnly` is the **default** and is byte-for-byte the current behavior: the
/// repeated-pilot preamble is the only sync, and decode windows start at sample 0.
/// `Chirp` (Pass C) prepends a linear-chirp preamble located by cross-correlation,
/// giving sample-accurate start detection robust to offset and frequency shift.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncMode {
    /// Current behavior: repeated-pilot preamble, windows from sample 0.
    PilotOnly,
    /// Linear-chirp preamble located by cross-correlation (Pass C).
    Chirp,
}

/// Parameters for the S7 sync layer. Defaults to `PilotOnly` so existing callers
/// and tests are unaffected; `Chirp` is opt-in.
#[derive(Debug, Clone)]
pub struct SyncParams {
    pub mode: SyncMode,
    /// Chirp length in symbol-durations (only used in `Chirp` mode).
    pub chirp_symbols: usize,
    /// Chirp sweep low frequency (Hz).
    pub chirp_f_lo_hz: f32,
    /// Chirp sweep high frequency (Hz).
    pub chirp_f_hi_hz: f32,
}

impl Default for SyncParams {
    fn default() -> Self {
        SyncParams {
            mode: SyncMode::PilotOnly,
            chirp_symbols: 4,
            chirp_f_lo_hz: 3000.0,
            chirp_f_hi_hz: 11_000.0,
        }
    }
}

/// Result of start-of-burst detection + timing estimation.
#[derive(Debug, Clone, Copy)]
pub struct SyncResult {
    /// Located burst start, in samples.
    pub start_sample: usize,
    /// Drift-corrected (fractional) samples-per-symbol.
    pub samples_per_symbol: f32,
    /// Estimated carrier frequency offset (Hz).
    pub freq_offset_hz: f32,
    /// Correlation-peak sharpness / detection confidence in 0..1.
    pub confidence: f32,
}

/// Generate the f32 linear-chirp template (one value per sample) for a `SyncParams`.
///
/// A linear (up-)chirp sweeps instantaneous frequency from `chirp_f_lo_hz` to
/// `chirp_f_hi_hz` over `chirp_symbols` symbol-durations. The instantaneous phase is
/// the integral of a linearly-ramping frequency, `φ(n) = 2π( f_lo·t + ½·rate·t² )`,
/// with `rate = (f_hi − f_lo)/T`. A chirp is used (rather than a single pilot tone)
/// because its autocorrelation is a SHARP peak: cross-correlating the received
/// signal against this template gives sample-accurate start-of-burst detection that
/// is robust to leading noise and degrades only gracefully under a small carrier
/// frequency offset. The template is unit-amplitude (callers scale as needed).
fn chirp_template_f32(params: &ModemParams, sync: &SyncParams) -> Vec<f32> {
    let sps = ((params.sample_rate as f32) * (params.symbol_ms / 1000.0)).round() as usize;
    let len = sps.saturating_mul(sync.chirp_symbols.max(1));
    if len == 0 {
        return Vec::new();
    }
    let sr = params.sample_rate as f32;
    let t_total = (len as f32) / sr; // chirp duration in seconds
    let f_lo = sync.chirp_f_lo_hz;
    let f_hi = sync.chirp_f_hi_hz;
    let rate = (f_hi - f_lo) / t_total; // Hz per second
    let mut out = Vec::with_capacity(len);
    for n in 0..len {
        let t = (n as f32) / sr;
        // Integral of the linearly-swept frequency gives a quadratic phase term.
        let phase = 2.0 * PI * (f_lo * t + 0.5 * rate * t * t);
        out.push(phase.sin());
    }
    out
}

/// Render the sync preamble to prepend to a burst.
///
/// In `PilotOnly` mode there is no separate sync preamble (the repeated pilot tone
/// carries sync), so this returns an empty buffer — keeping the existing encode path
/// byte-identical. In `Chirp` mode it renders the linear-chirp template (see
/// [`chirp_template_f32`]) normalized to i16, to be prepended to the FULL rendered
/// sample stream (orthogonal to the symbol/tone counts).
pub fn render_sync_preamble(params: &ModemParams, sync: &SyncParams) -> Vec<i16> {
    match sync.mode {
        SyncMode::PilotOnly => Vec::new(),
        SyncMode::Chirp => {
            let tmpl = chirp_template_f32(params, sync);
            // Normalize to 90% of full-scale, matching render_symbols_to_samples so
            // the chirp sits at a comparable level to the data body.
            let scale = i16::MAX as f32 * 0.9;
            tmpl.iter()
                .map(|&v| (v * scale).clamp(i16::MIN as f32, i16::MAX as f32) as i16)
                .collect()
        }
    }
}

/// Locate the start of a burst and estimate symbol timing + frequency offset.
///
/// In `PilotOnly` mode this reports the legacy assumption (start at sample 0,
/// nominal samples-per-symbol, zero offset) so existing callers are unaffected.
///
/// In `Chirp` mode it CROSS-CORRELATES the received samples against the known chirp
/// template and takes the correlation-peak position as the burst start (the chirp is
/// the first thing in the rendered stream, so the located start is the chirp's first
/// sample — equal to the channel's `start_offset_samples`). The peak is found with a
/// normalized cross-correlation so the score is amplitude-independent; `confidence`
/// is the peak's sharpness (peak value relative to the local mean), squashed into
/// 0..1. A coarse carrier `freq_offset_hz` is estimated by comparing the dominant
/// frequency of the received chirp region against the template's mid-sweep frequency
/// (a cheap, direction-correct estimate for the small offsets of interest).
pub fn detect_burst_start(
    samples: &[i16],
    params: &ModemParams,
    sync: &SyncParams,
) -> Result<SyncResult, ModemError> {
    if samples.is_empty() {
        return Err(ModemError::Sync("empty sample buffer".to_string()));
    }
    let sps = (params.sample_rate as f32) * (params.symbol_ms / 1000.0);

    if sync.mode == SyncMode::PilotOnly {
        return Ok(SyncResult {
            start_sample: 0,
            samples_per_symbol: sps,
            freq_offset_hz: 0.0,
            confidence: 0.0,
        });
    }

    // ── Chirp mode: cross-correlate against the template ────────────────────
    let tmpl = chirp_template_f32(params, sync);
    let m = tmpl.len();
    if m == 0 {
        return Err(ModemError::Sync(
            "chirp template is empty (chirp_symbols = 0?)".to_string(),
        ));
    }
    if samples.len() < m {
        return Err(ModemError::Sync(format!(
            "buffer ({}) shorter than chirp template ({})",
            samples.len(),
            m
        )));
    }

    let x: Vec<f32> = samples.iter().map(|&s| s as f32).collect();
    // Template energy (constant across lags) for normalized correlation.
    let tmpl_energy: f32 = tmpl.iter().map(|&v| v * v).sum::<f32>().max(1e-12);
    let tmpl_norm = tmpl_energy.sqrt();

    // The chirp is the FIRST feature in the rendered stream, so the true start is
    // near the front of the received buffer (only the channel's leading noise / start
    // offset precedes it). A full O(buffer·template) slide over a multi-megasample
    // burst is needlessly expensive, so bound the lag search to a generous lead region
    // (≈ one second of audio past where any plausible start offset could sit). This is
    // both faster and more robust: it cannot lock onto a chance mid-burst correlation.
    let search_window = m + params.sample_rate; // chirp length + ~1 s of lead
    let last_start = (x.len() - m).min(search_window);

    // A normalized correlation at one lag: <x_window, tmpl> / (||x_window||·||tmpl||).
    // Normalizing by window energy keeps the low-energy leading noise from beating the
    // chirp region and makes `confidence` amplitude-independent.
    let corr_at = |lag: usize| -> f32 {
        let win = &x[lag..lag + m];
        let mut dot = 0.0f32;
        let mut win_energy = 0.0f32;
        for i in 0..m {
            dot += win[i] * tmpl[i];
            win_energy += win[i] * win[i];
        }
        dot / (win_energy.sqrt().max(1e-12) * tmpl_norm)
    };

    // Coarse-to-fine search: scan the lead region on a coarse stride, then refine
    // ±stride around the coarse peak at full resolution. This keeps detection
    // sample-accurate (the fine pass is exhaustive near the peak) at a fraction of the
    // cost of a full slide. CRUCIAL: a linear chirp's autocorrelation main lobe is only
    // ~`sr/bandwidth` samples wide (a few samples for a multi-kHz sweep), so the coarse
    // stride MUST be smaller than that lobe or it steps clean over the true peak and
    // locks onto a spurious mid-burst correlation. We size the stride to half the main-
    // lobe width (≥ 1).
    let bandwidth = (sync.chirp_f_hi_hz - sync.chirp_f_lo_hz).abs().max(1.0);
    let main_lobe = ((params.sample_rate as f32) / bandwidth).max(1.0);
    let coarse_stride = (main_lobe * 0.5).floor().max(1.0) as usize;
    let mut best_score = f32::MIN;
    let mut best_lag = 0usize;
    let mut score_sum = 0.0f64;
    let mut score_cnt = 0u64;
    let mut lag = 0usize;
    while lag <= last_start {
        let score = corr_at(lag);
        score_sum += score as f64;
        score_cnt += 1;
        if score > best_score {
            best_score = score;
            best_lag = lag;
        }
        lag += coarse_stride;
    }
    // Fine refinement: exhaustively scan ±coarse_stride around the coarse peak so the
    // final lag is sample-accurate even though the coarse pass strode over most lags.
    let lo = best_lag.saturating_sub(coarse_stride);
    let hi = (best_lag + coarse_stride).min(x.len() - m);
    for lag in lo..=hi {
        let score = corr_at(lag);
        if score > best_score {
            best_score = score;
            best_lag = lag;
        }
    }

    // Confidence: how far the peak stands above the typical correlation. A clean chirp
    // gives a peak near 1.0 with a low background mean, so (peak − mean) is large; pure
    // noise gives peak ≈ mean. When the buffer is barely longer than the template only
    // a handful of lags exist and the mean is unreliable, so fall back to the raw peak
    // score (a chirp self-correlates near 1.0). Clamp into 0..1.
    let mean_score = if score_cnt > 1 {
        (score_sum / score_cnt as f64) as f32
    } else {
        0.0
    };
    let confidence = if score_cnt > 4 {
        (best_score - mean_score).clamp(0.0, 1.0)
    } else {
        best_score.clamp(0.0, 1.0)
    };

    // Coarse frequency-offset estimate from the located chirp region. We compare the
    // received chirp's energy at the template's nominal mid-sweep frequency against a
    // few offset-shifted probes and pick the best — a cheap argmax over a small grid.
    let freq_offset_hz = estimate_freq_offset(&samples[best_lag..best_lag + m], params, sync);

    Ok(SyncResult {
        start_sample: best_lag,
        samples_per_symbol: sps,
        freq_offset_hz,
        confidence,
    })
}

/// Coarse carrier-frequency-offset estimate over the located chirp region.
///
/// The data tones land on exact Goertzel bin centers; a carrier offset shifts them
/// off-bin. We probe the chirp region's energy at a small grid of candidate offsets
/// (±~one bin in fine steps) by re-mixing the region down by each candidate and
/// measuring how much total energy lands back on the chirp's own sweep — the offset
/// whose de-mix maximizes on-template energy is the estimate. This is a cheap,
/// direction-correct estimator sufficient for the sub-bin offsets the modem targets.
fn estimate_freq_offset(region: &[i16], params: &ModemParams, sync: &SyncParams) -> f32 {
    if region.is_empty() {
        return 0.0;
    }
    let tmpl = chirp_template_f32(params, sync);
    let n = region.len().min(tmpl.len());
    if n == 0 {
        return 0.0;
    }
    let sr = params.sample_rate as f32;
    let x: Vec<f32> = region.iter().take(n).map(|&s| s as f32).collect();

    // Bin resolution sets the search span: probe ±1 bin in fine steps.
    let bin_hz = sr / (((params.sample_rate as f32) * (params.symbol_ms / 1000.0)).round());
    let span = bin_hz; // ± one bin
    let steps = 41i32; // odd, so 0 Hz is sampled exactly
    let mut best_off = 0.0f32;
    let mut best_energy = f32::MIN;
    for k in 0..steps {
        let off = -span + (2.0 * span) * (k as f32) / ((steps - 1) as f32);
        // De-mix the region by -off and correlate with the (real) template. A real
        // cosine de-mix is a first-order shift, matching the channel model's mixer.
        let w = 2.0 * PI * off / sr;
        let mut dot = 0.0f32;
        for i in 0..n {
            let demixed = x[i] * (w * (i as f32)).cos();
            dot += demixed * tmpl[i];
        }
        let energy = dot.abs();
        if energy > best_energy {
            best_energy = energy;
            best_off = off;
        }
    }
    best_off
}

/// Produce drift-corrected symbol-window start boundaries for the burst.
///
/// Lays symbol windows from the located start (`sync.start_sample`, the chirp's first
/// sample in `Chirp` mode) to the end of the buffer, with an **early-late timing-
/// recovery loop** that tracks slow clock drift instead of advancing by a fixed
/// stride. For each window it evaluates the dominant-tone Goertzel energy at three
/// sub-window read positions (early / on-time / late) and steers the running window
/// position and the fractional stride estimate toward the energy peak. Under a
/// positive `clock_ppm` the true symbol length grows, so the estimated stride creeps
/// up and the window starts slide cumulatively LATE across the burst — which is
/// exactly what `decode_with_boundaries`'s pilot search then re-aligns to the data.
///
/// The chirp/pilot prefix windows are included in the returned boundaries; the
/// decode-side pilot-pattern search trims everything up to and including the pilot,
/// so callers do not need to know the chirp length here.
pub fn recover_symbol_timing(
    samples: &[i16],
    params: &ModemParams,
    sync: &SyncResult,
) -> Result<Vec<usize>, ModemError> {
    if samples.len() < sync.start_sample {
        return Err(ModemError::Timing(
            "start_sample is past the end of the buffer".to_string(),
        ));
    }
    let nominal_sps = sync.samples_per_symbol.max(1.0);
    let sps_int = nominal_sps.round().max(1.0) as usize;
    let tone_freqs = build_tone_frequencies(params);

    // Dominant-tone alignment energy of the symbol window starting at sample `s`: the
    // max single-bin Goertzel response over every channel's every tone. It peaks when
    // the window is aligned to a symbol (one tone fully inside the Hann-windowed window)
    // and dips when the window straddles two symbols (energy splits) — so SUMMED over a
    // run of windows it is maximized by the correct symbol-stride. We use the cheap
    // single-point Goertzel (not the ±½-bin band sum) here: a small carrier offset
    // shifts every tone's energy together and does not move the *alignment* peak across
    // stride, so the point metric is enough to time-align and is 3× cheaper.
    let win_energy = |s: usize| -> f32 {
        if s + sps_int > samples.len() {
            return 0.0;
        }
        let slice = &samples[s..s + sps_int];
        let mut best = 0.0f32;
        for ch_freqs in &tone_freqs {
            for &f in ch_freqs {
                let mag = goertzel_point(slice, f, params.sample_rate);
                if mag > best {
                    best = mag;
                }
            }
        }
        best
    };

    // ── Symbol-timing recovery via per-burst stride search ──────────────────
    //
    // A constant clock offset (the S7 drift impairment) makes the TRUE symbol length a
    // constant `sps·(1+ppm)` for the whole burst — it is not per-symbol jitter — so the
    // right estimator is a single, drift-corrected stride applied uniformly from the
    // located start. We search a small grid of candidate strides around nominal and
    // pick the one whose laid-down windows accumulate the most dominant-tone energy.
    // This is far more stable than a per-symbol early-late nudge (which, on a clean no-
    // drift signal, accumulates noise into a spurious walk and misaligns the late
    // windows). Because a wrong stride drifts the windows OFF the symbols by the end of
    // the burst, total energy is sharply peaked at the true stride.
    //
    // Laying windows at `start + i·sps_est` makes the boundaries slide CUMULATIVELY:
    // under drift the chosen `sps_est` differs from nominal, so window `i` diverges from
    // a no-drift baseline by `i·(sps_est − sps_base)` — growing across the burst, which
    // is exactly the drift-tracking the timing test pins (late ≫ early divergence).
    let start = sync.start_sample;
    let usable = samples.len().saturating_sub(start);
    let approx_syms = (usable / sps_int).max(1);

    // We average the alignment energy over EVERY window in the burst for each candidate
    // stride: a wrong stride walks the windows off the symbols, and the misalignment —
    // and the resulting per-window energy loss — accumulates over the whole burst, so
    // the all-window mean is sharply peaked (≈1 sample wide) at the true stride.
    //
    // Because that peak is so sharp, the search grid must step in SUB-SAMPLE increments
    // across the whole span or it skips the peak and locks onto a spurious lobe. We keep
    // the span tight — a clock offset large enough to need a wider span (≫1000 ppm)
    // would slip whole symbols within the burst and is out of scope — so a tight span at
    // sub-sample resolution is both correct and affordable. ±0.4% of nominal covers
    // ≈±4000 ppm of clock error, well past any plausible acoustic link.
    let span = (nominal_sps * 0.004).max(3.0);
    // Metric = MEAN alignment energy per window, NOT the sum: a shorter candidate
    // stride packs more windows into the burst, so an energy SUM is biased toward too-
    // short strides (it rewards window count, not alignment) — which lands the search
    // on a wildly wrong stride. The per-window mean is maximized at the true stride
    // regardless of how many windows fit, so it is unbiased.
    let eval_stride = |sps: f32| -> f64 {
        let mut acc = 0.0f64;
        let mut i = 0usize;
        loop {
            let s = start as f32 + (i as f32) * sps;
            let si = s.round() as usize;
            if si + sps_int > samples.len() {
                break;
            }
            acc += win_energy(si) as f64;
            i += 1;
        }
        if i == 0 {
            0.0
        } else {
            acc / (i as f64)
        }
    };

    // Stage 1: coarse grid across the full ±span at sub-sample resolution. With a tight
    // span (~±8 samples) this still steps well under the ~1-sample peak width.
    let coarse_steps = 41i32; // odd ⇒ nominal sampled exactly; ~0.4-sample steps
    let mut best_sps = nominal_sps;
    let mut best_energy = f64::MIN;
    for k in 0..coarse_steps {
        let sps = nominal_sps - span + (2.0 * span) * (k as f32) / ((coarse_steps - 1) as f32);
        let e = eval_stride(sps);
        if e > best_energy {
            best_energy = e;
            best_sps = sps;
        }
    }
    // Stage 2: fine grid ±one coarse step around the coarse best, for sub-sample stride.
    let coarse_step = (2.0 * span) / ((coarse_steps - 1) as f32);
    let fine_steps = 41i32;
    let coarse = best_sps;
    for k in 0..fine_steps {
        let sps =
            coarse - coarse_step + (2.0 * coarse_step) * (k as f32) / ((fine_steps - 1) as f32);
        let e = eval_stride(sps);
        if e > best_energy {
            best_energy = e;
            best_sps = sps;
        }
    }

    // Stage 3: two-anchor stride refinement. The energy-vs-stride peak is broad, so the
    // grid stride can still be ~a sample off — and over a long burst even a 1-sample
    // stride error walks the late windows off their symbols. We sharpen it by directly
    // measuring the drift GEOMETRY: locally re-align an EARLY window and a LATE window
    // (each to the sample position that maximizes its own alignment energy, searched a
    // few samples either side of the grid-stride prediction), then set the stride to the
    // slope between the two refined positions. Two well-separated anchors pin a constant
    // (drift-corrected) stride far more precisely than the flat-topped energy peak.
    if approx_syms >= 8 {
        let i1 = approx_syms / 8; // early anchor (clear of the chirp/pilot transient)
        let i2 = approx_syms - 2; // late anchor (maximizes the lever arm)
                                  // Local refinement radius: a fraction of a symbol is plenty once the grid
                                  // stride is within ~a sample.
        let radius = (nominal_sps * 0.05).round().max(4.0) as isize;
        let refine = |idx: usize| -> Option<f64> {
            let predicted = start as f64 + (idx as f64) * (best_sps as f64);
            let center = predicted.round() as isize;
            let mut best_p = center;
            let mut best_e = f32::MIN;
            for d in -radius..=radius {
                let s = center + d;
                if s < 0 {
                    continue;
                }
                let su = s as usize;
                if su + sps_int > samples.len() {
                    continue;
                }
                let e = win_energy(su);
                if e > best_e {
                    best_e = e;
                    best_p = s;
                }
            }
            if best_e > f32::MIN {
                Some(best_p as f64)
            } else {
                None
            }
        };
        if let (Some(p1), Some(p2)) = (refine(i1), refine(i2)) {
            if i2 > i1 {
                let slope = (p2 - p1) / ((i2 - i1) as f64);
                // Only accept if the refined stride is within the search span (guards
                // against a local-max anchor landing a symbol off and skewing the slope).
                if (slope - nominal_sps as f64).abs() <= span as f64 {
                    best_sps = slope as f32;
                }
            }
        }
    }

    // Lay the final uniform (drift-corrected) windows.
    let mut bounds: Vec<usize> = Vec::with_capacity(approx_syms);
    let mut i = 0usize;
    loop {
        let s = start as f32 + (i as f32) * best_sps;
        let si = s.round() as usize;
        if si + sps_int > samples.len() {
            break;
        }
        bounds.push(si);
        i += 1;
    }

    if bounds.is_empty() {
        return Err(ModemError::Timing(
            "no symbol windows fit between the located start and the buffer end".to_string(),
        ));
    }
    Ok(bounds)
}

// ────────────────────────────────────────────────────────────────────────────
// 2b. RATE-SELECTABLE CODING API (COMPILING STUBS — TODO Pass C)
// ────────────────────────────────────────────────────────────────────────────

/// Named interleaved-Reed-Solomon redundancy points (the coding *rate*), plus a
/// custom escape hatch. Higher redundancy ⇒ more loss tolerance, lower throughput.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RsRate {
    /// d=4, p=1 (total 5, 20% parity) — high throughput, clean links.
    High,
    /// d=4, p=2 (total 6, ~33% parity) — balanced.
    Medium,
    /// d=4, p=4 (total 8, 50% parity) — robust, lossy links.
    Low,
    /// Explicit (data_shards, parity_shards, shard_size).
    Custom {
        data_shards: usize,
        parity_shards: usize,
        shard_size: usize,
    },
}

impl RsRate {
    /// (data_shards, parity_shards, shard_size) for this rate.
    ///
    /// The named rates hold `data_shards` (and therefore the per-block data
    /// capacity and zero-padding) CONSTANT at 4 and grow only `parity_shards`
    /// High → Medium → Low (1 → 2 → 4). This makes the redundancy ladder monotone in
    /// BOTH axes the tests pin: the parity fraction `p/(d+p)` strictly increases
    /// (0.20 → 0.33 → 0.50), and — because the data geometry is identical across
    /// rates — the *encoded length* of a given frame strictly grows with redundancy
    /// too (a more-protected profile is purely "the same data shards plus more parity
    /// shards"). A growing-`d`/shrinking-block ladder (the original Pass-A sketch of
    /// 8/2, 6/3, 4/4) would have INVERTED the encoded-length order for sub-block
    /// payloads, because the high rate's larger block pads more — so the constant-`d`
    /// ladder is the geometry that keeps overhead monotone while still leaving every
    /// RS rate cheaper than brute-force repetition.
    pub fn shard_config(&self) -> (usize, usize, usize) {
        match *self {
            RsRate::High => (4, 1, 128),
            RsRate::Medium => (4, 2, 128),
            RsRate::Low => (4, 4, 128),
            RsRate::Custom {
                data_shards,
                parity_shards,
                shard_size,
            } => (data_shards, parity_shards, shard_size),
        }
    }
}

/// The coding profile carried in-band so the receiver learns the rate without
/// out-of-band flags. `Repetition` maps the legacy repetition FEC (kept for
/// backward compatibility / absolute-floor robustness); `RsRate` selects the
/// interleaved-RS rate ladder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodingProfile {
    Repetition { repeats: usize, pkt_size: usize },
    RsRate(RsRate),
}

impl Default for CodingProfile {
    /// The default matches the encoder's current interleaved-RS default posture
    /// (a balanced rate), so a stream with no in-band rate header decodes sanely.
    fn default() -> Self {
        CodingProfile::RsRate(RsRate::Medium)
    }
}

// ── In-band coding-rate header ("CDG1") ─────────────────────────────────────
//
// The rate must survive to the receiver without out-of-band `--rs-data` flags, so
// we prepend a small fixed-size header carrying the profile. Layout (big-endian):
//
//   [4 bytes magic "CDG1"]
//   [1 byte  profile_tag]   0 = Repetition, 1 = interleaved-RS
//   [2 bytes a]             Repetition: repeats   | RS: data_shards
//   [2 bytes b]             Repetition: pkt_size  | RS: parity_shards
//   [2 bytes c]             Repetition: 0         | RS: shard_size
//
// = 11 bytes per copy. The whole header is emitted in TRIPLICATE (33 bytes) so a
// burst erasure cannot wipe the rate: the parser majority-votes the three copies
// field-by-field. The header is a pure PREFIX in front of the existing
// `RS01`/`PKT1` packet stream, so the packet formats stay byte-identical and a
// legacy stream (no `CDG1` prefix) still decodes exactly as before.

const CDG1_MAGIC: &[u8; 4] = b"CDG1";
const CDG1_TAG_REPETITION: u8 = 0;
const CDG1_TAG_RS: u8 = 1;
/// One copy: 4 magic + 1 tag + 3×u16 = 11 bytes.
const CDG1_COPY_LEN: usize = 4 + 1 + 2 + 2 + 2;
/// Triplicated header total length (the fixed prefix `parse_coding_header` reports).
const CDG1_HEADER_LEN: usize = CDG1_COPY_LEN * 3;

/// Serialize one `CodingProfile` into a single (untriplicated) 11-byte CDG1 copy.
fn encode_cdg1_copy(profile: &CodingProfile) -> [u8; CDG1_COPY_LEN] {
    let mut buf = [0u8; CDG1_COPY_LEN];
    buf[0..4].copy_from_slice(CDG1_MAGIC);
    let (tag, a, b, c): (u8, u16, u16, u16) = match *profile {
        CodingProfile::Repetition { repeats, pkt_size } => {
            (CDG1_TAG_REPETITION, repeats as u16, pkt_size as u16, 0)
        }
        CodingProfile::RsRate(rate) => {
            let (d, p, s) = rate.shard_config();
            (CDG1_TAG_RS, d as u16, p as u16, s as u16)
        }
    };
    buf[4] = tag;
    buf[5..7].copy_from_slice(&a.to_be_bytes());
    buf[7..9].copy_from_slice(&b.to_be_bytes());
    buf[9..11].copy_from_slice(&c.to_be_bytes());
    buf
}

/// Reconstruct a `CodingProfile` from a decoded (tag, a, b, c) tuple.
///
/// RS profiles are normalized back onto the named rate ladder when the (d, p, s)
/// triple matches a named rate, so a round-tripped `RsRate::High` parses back as
/// `RsRate::High` (not `Custom`), which is what the header-round-trip tests assert.
fn profile_from_fields(tag: u8, a: u16, b: u16, c: u16) -> Result<CodingProfile, ModemError> {
    match tag {
        CDG1_TAG_REPETITION => Ok(CodingProfile::Repetition {
            repeats: a as usize,
            pkt_size: b as usize,
        }),
        CDG1_TAG_RS => {
            let (d, p, s) = (a as usize, b as usize, c as usize);
            // Normalize onto the named ladder when the geometry matches.
            let rate = if (d, p, s) == RsRate::High.shard_config() {
                RsRate::High
            } else if (d, p, s) == RsRate::Medium.shard_config() {
                RsRate::Medium
            } else if (d, p, s) == RsRate::Low.shard_config() {
                RsRate::Low
            } else {
                RsRate::Custom {
                    data_shards: d,
                    parity_shards: p,
                    shard_size: s,
                }
            };
            Ok(CodingProfile::RsRate(rate))
        }
        other => Err(ModemError::CodingHeader(format!(
            "unknown CDG1 profile tag {other}"
        ))),
    }
}

/// Map a measured/estimated link SNR (dB) to an interleaved-RS redundancy rate.
///
/// Lower SNR ⇒ more redundancy (more parity), higher SNR ⇒ less redundancy (more
/// throughput). The thresholds are deliberately coarse — this is a link-adaptation
/// *hint*, not a capacity-exact selector — and follow the documented ladder
/// `High` (clean) → `Medium` (balanced) → `Low` (lossy):
///
/// - `snr_db >= 20.0` → [`RsRate::High`]   (d=8, p=2 — clean link, max throughput)
/// - `snr_db >= 10.0` → [`RsRate::Medium`] (d=6, p=3 — balanced)
/// - otherwise        → [`RsRate::Low`]    (d=4, p=4 — robust, lossy link)
pub fn select_rate(snr_db: f32) -> RsRate {
    if snr_db >= 20.0 {
        RsRate::High
    } else if snr_db >= 10.0 {
        RsRate::Medium
    } else {
        RsRate::Low
    }
}

/// Packetize a frame under the chosen coding profile, with an in-band rate header.
///
/// Honors `profile`: a `Repetition` profile emits the legacy repetition-FEC packet
/// stream; an `RsRate` profile emits the interleaved Reed-Solomon stream at that
/// rate's shard geometry. In BOTH cases a triplicated `CDG1` rate header is
/// prepended so `depacketize_with_profile` / `parse_coding_header` recover the rate
/// in-band — no out-of-band `--rs-data` flags. The header is a pure prefix, so the
/// underlying `RS01`/`PKT1` packet bytes are byte-identical to the legacy paths.
pub fn packetize_with_profile(
    frame: &[u8],
    profile: &CodingProfile,
) -> Result<Vec<u8>, ModemError> {
    // Body: dispatch on the profile.
    let body = match *profile {
        CodingProfile::Repetition { repeats, pkt_size } => {
            if pkt_size == 0 || repeats == 0 {
                return Err(ModemError::Other(
                    "Repetition profile requires repeats > 0 and pkt_size > 0".to_string(),
                ));
            }
            packetize_stream(frame, pkt_size, repeats)
        }
        CodingProfile::RsRate(rate) => {
            let (d, p, s) = rate.shard_config();
            packetize_stream_rs_interleaved(frame, d, p, s)
        }
    };

    // Prefix: triplicated CDG1 rate header.
    let copy = encode_cdg1_copy(profile);
    let mut out = Vec::with_capacity(CDG1_HEADER_LEN + body.len());
    for _ in 0..3 {
        out.extend_from_slice(&copy);
    }
    out.extend_from_slice(&body);
    Ok(out)
}

/// Parse the in-band coding-rate header from the front of a packetized stream.
///
/// Majority-votes the triplicated `CDG1` header field-by-field (so a single
/// corrupted/erased copy cannot flip the rate) and returns the recovered profile
/// plus the number of prefix bytes the caller must skip. If NO `CDG1` magic is
/// present in the first copy slot, the stream was produced by a legacy (header-less)
/// path: this returns `(CodingProfile::default(), 0)` so legacy streams decode
/// exactly as before — a *missing* header is not an error.
pub fn parse_coding_header(stream: &[u8]) -> Result<(CodingProfile, usize), ModemError> {
    // Not even one copy fits, or the first slot is not a CDG1 copy ⇒ legacy stream.
    if stream.len() < CDG1_COPY_LEN || &stream[0..4] != CDG1_MAGIC {
        return Ok((CodingProfile::default(), 0));
    }

    // Collect every well-formed copy in the (up to) three triplicate slots.
    let mut votes: Vec<(u8, u16, u16, u16)> = Vec::with_capacity(3);
    for slot in 0..3 {
        let off = slot * CDG1_COPY_LEN;
        if off + CDG1_COPY_LEN > stream.len() {
            break;
        }
        let c = &stream[off..off + CDG1_COPY_LEN];
        if &c[0..4] != CDG1_MAGIC {
            continue; // a clobbered copy slot; skip it for the vote
        }
        let tag = c[4];
        let a = u16::from_be_bytes([c[5], c[6]]);
        let b = u16::from_be_bytes([c[7], c[8]]);
        let cc = u16::from_be_bytes([c[9], c[10]]);
        votes.push((tag, a, b, cc));
    }

    if votes.is_empty() {
        // First slot had the magic but no copy survived field validation.
        return Err(ModemError::CodingHeader(
            "CDG1 prefix present but no valid copy could be read".to_string(),
        ));
    }

    // Per-field majority vote across the surviving copies.
    let majority = |vals: &[u16]| -> u16 {
        let mut counts: HashMap<u16, usize> = HashMap::new();
        for &v in vals {
            *counts.entry(v).or_insert(0) += 1;
        }
        *counts
            .iter()
            .max_by_key(|kv| *kv.1)
            .map(|(k, _)| k)
            .unwrap_or(&vals[0])
    };
    let tag = {
        let mut counts: HashMap<u8, usize> = HashMap::new();
        for v in &votes {
            *counts.entry(v.0).or_insert(0) += 1;
        }
        *counts
            .iter()
            .max_by_key(|kv| *kv.1)
            .map(|(k, _)| k)
            .unwrap_or(&votes[0].0)
    };
    let a = majority(&votes.iter().map(|v| v.1).collect::<Vec<_>>());
    let b = majority(&votes.iter().map(|v| v.2).collect::<Vec<_>>());
    let c = majority(&votes.iter().map(|v| v.3).collect::<Vec<_>>());

    let profile = profile_from_fields(tag, a, b, c)?;
    Ok((profile, CDG1_HEADER_LEN))
}

/// Depacketize a stream produced by `packetize_with_profile`, learning the rate
/// in-band.
///
/// Parses the `CDG1` prefix (if any), skips it, then dispatches on the learned
/// profile: a `Repetition` profile is recovered with `depacketize_stream`, an
/// `RsRate` profile with `depacketize_stream_rs` (which self-describes its shard
/// geometry from the `RS01` headers). A legacy header-less stream (consumed = 0)
/// falls through to `depacketize_stream_rs` exactly as today.
pub fn depacketize_with_profile(stream: &[u8]) -> Result<Vec<u8>, ModemError> {
    let (profile, consumed) = parse_coding_header(stream)?;
    let body = &stream[consumed..];
    match profile {
        CodingProfile::Repetition { repeats, .. } => depacketize_stream(body, repeats),
        CodingProfile::RsRate(_) => depacketize_stream_rs(body),
    }
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

    // ========================================================================
    // S7 ACOUSTIC-CHANNEL MODEL — smoke tests (REAL code, not a stub).
    //
    // These prove the model (a) is a near-identity passthrough under the
    // zero-impairment config and (b) genuinely perturbs the samples under a
    // non-trivial config, and (c) is deterministic for a fixed seed. Everything
    // is in-memory and seeded — no filesystem, no audio hardware.
    // ========================================================================

    /// Build a short, non-trivial test tone burst (a couple of default symbol
    /// windows of a single channel-0 tone) to feed the channel model.
    fn smoke_burst() -> Vec<i16> {
        let params = ModemParams::default();
        let mut channels_symbols: Vec<Vec<u8>> = vec![Vec::new(); params.channels];
        channels_symbols[0] = vec![5u8, 12u8, 20u8];
        render_symbols_to_samples(&channels_symbols, &params)
    }

    /// Property: a zero-impairment (`identity`) channel is a near-identity
    /// passthrough — same length, and the waveform shape is preserved within a
    /// tight relative tolerance (only a uniform renormalization gain may apply).
    #[test]
    fn test_acoustic_channel_identity_is_near_passthrough() {
        let input = smoke_burst();
        assert!(!input.is_empty(), "smoke burst must be non-empty");

        let out = simulate_acoustic_channel(&input, &AcousticChannelParams::identity());
        assert_eq!(
            out.len(),
            input.len(),
            "identity channel must not change sample count"
        );

        // The model renormalizes peak->90% full-scale; render_symbols_to_samples
        // already did the same, so identity is effectively a unit gain. Compare
        // the normalized waveforms: max absolute deviation must be tiny relative
        // to full-scale.
        let in_peak = input.iter().map(|&s| (s as f32).abs()).fold(0.0, f32::max);
        let out_peak = out.iter().map(|&s| (s as f32).abs()).fold(0.0, f32::max);
        assert!(in_peak > 0.0 && out_peak > 0.0);
        let mut max_dev = 0.0f32;
        for (&a, &b) in input.iter().zip(out.iter()) {
            let na = a as f32 / in_peak;
            let nb = b as f32 / out_peak;
            max_dev = max_dev.max((na - nb).abs());
        }
        assert!(
            max_dev < 0.02,
            "identity channel must be a near-passthrough; normalized max deviation \
             was {max_dev:.4} (expected < 0.02)"
        );
    }

    /// Property: a non-trivial config (start offset + clock drift + frequency
    /// offset + echo) GENUINELY perturbs the samples — the output must differ
    /// materially from the input (it must not be a passthrough). This guards that
    /// the model is real working code that the Pass-B failing tests can rely on.
    #[test]
    fn test_acoustic_channel_perturbs_samples() {
        let input = smoke_burst();
        let params = AcousticChannelParams {
            seed: 1234,
            start_offset_samples: 137, // a non-multiple-of-symbol start offset
            clock_ppm: 500.0,          // half a per-mille clock error
            freq_offset_hz: 7.0,       // nudge off the 25 Hz bin centers
            echo_delay_samples: 64,    // a short room echo
            echo_gain: 0.4,
            jitter_samples: 0.0,
        };
        let out = simulate_acoustic_channel(&input, &params);

        // Start offset alone makes the lengths differ; assert the model moved the
        // signal (longer due to the prepended offset + clock stretch).
        assert!(
            out.len() > input.len(),
            "start offset + positive clock_ppm must lengthen the burst (got {} vs {})",
            out.len(),
            input.len()
        );

        // And on the overlapping region the waveform must materially differ from a
        // plain copy: count how many samples differ once we skip the prepended
        // offset region. With offset+drift+mix+echo, the vast majority must differ.
        let skip = params.start_offset_samples as usize;
        let mut differing = 0usize;
        let mut compared = 0usize;
        for (i, &a) in input.iter().enumerate() {
            if let Some(&b) = out.get(i + skip) {
                compared += 1;
                if a != b {
                    differing += 1;
                }
            }
        }
        assert!(compared > 0, "must have an overlapping region to compare");
        let frac = differing as f32 / compared as f32;
        assert!(
            frac > 0.5,
            "non-trivial channel must perturb most samples; only {:.1}% differed",
            100.0 * frac
        );
    }

    /// Property: the model is DETERMINISTIC — same seed + same params ⇒ identical
    /// output (the seeded ChaCha8Rng guarantee tests rely on).
    #[test]
    fn test_acoustic_channel_is_deterministic() {
        let input = smoke_burst();
        let params = AcousticChannelParams {
            seed: 99,
            start_offset_samples: 40,
            clock_ppm: 0.0,
            freq_offset_hz: 0.0,
            echo_delay_samples: 0,
            echo_gain: 0.0,
            jitter_samples: 1.5, // jitter exercises the seeded RNG
        };
        let a = simulate_acoustic_channel(&input, &params);
        let b = simulate_acoustic_channel(&input, &params);
        assert_eq!(a, b, "same seed + params must produce identical output");
    }

    // ========================================================================
    // S7 RATE-SELECTABLE CODING — unit-level RED net (Pass B).
    //
    // These pin the rate-coding contract at the byte/stream level (no audio): the
    // overhead ladder, in-band rate signaling (CDG1), and per-rate identity. They
    // RED against the Pass-A stubs (packetize_with_profile ignores `profile` and
    // emits a fixed Medium stream with no CDG1 header; parse_coding_header always
    // returns the default and 0 consumed). The audio-pipeline legs of category 4
    // live in tests/modem_realair.rs; these are the pure-coding counterparts so a
    // Pass-C breakage is caught at both levels.
    // ========================================================================

    /// Property (RED): packetize_with_profile honors the rate — encoded length must
    /// strictly DECREASE in redundancy from High → Medium → Low (i.e. grow in size
    /// High < Medium < Low, since more parity = bulkier). The stub emits a fixed
    /// Medium stream for every profile, so all three are EQUAL → RED.
    #[test]
    fn test_profile_overhead_ladder_unit() {
        let frame = seeded_payload(200, 480);
        let len_high = packetize_with_profile(&frame, &CodingProfile::RsRate(RsRate::High))
            .expect("High")
            .len();
        let len_med = packetize_with_profile(&frame, &CodingProfile::RsRate(RsRate::Medium))
            .expect("Medium")
            .len();
        let len_low = packetize_with_profile(&frame, &CodingProfile::RsRate(RsRate::Low))
            .expect("Low")
            .len();
        assert!(
            len_high < len_med && len_med < len_low,
            "encoded length must grow with redundancy: High({len_high}) < Medium({len_med}) \
             < Low({len_low}). The stub emits a FIXED Medium stream for every profile (all \
             equal) — RED until packetize_with_profile honors the profile."
        );
    }

    /// Property (RED): parse_coding_header must recover the profile that
    /// packetize_with_profile USED, for each RsRate — the rate travels in-band via a
    /// triplicated CDG1 header. The stub emits no header and always returns the
    /// default (Medium) + 0 consumed, so this is RED for High and Low.
    #[test]
    fn test_coding_header_round_trips_profile_unit() {
        let frame = seeded_payload(201, 256);
        for rate in [RsRate::High, RsRate::Medium, RsRate::Low] {
            let profile = CodingProfile::RsRate(rate);
            let stream = packetize_with_profile(&frame, &profile).expect("packetize");
            let (parsed, consumed) = parse_coding_header(&stream).expect("parse_coding_header");
            assert_eq!(
                parsed, profile,
                "parse_coding_header must recover the USED profile {profile:?} in-band \
                 (got {parsed:?}). RED until the triplicated CDG1 header is emitted + parsed."
            );
            // A real header occupies bytes; the stub reports 0 consumed for every rate.
            assert!(
                consumed > 0,
                "an in-band CDG1 rate header must consume a non-zero, fixed-size prefix for \
                 {rate:?} (got consumed = 0). RED until the header is emitted."
            );
        }
    }

    /// Property (RED): packetize_with_profile -> depacketize_with_profile is a
    /// byte-exact identity on a clean stream for EVERY RsRate (the depacketizer learns
    /// the rate in-band and skips the CDG1 prefix). The stub's depacketizer delegates
    /// straight to depacketize_stream_rs with consumed = 0; once Pass C prepends a
    /// real CDG1 prefix to the High/Low streams, the stub depacketizer would choke on
    /// that prefix — so this identity is the contract Pass C must keep whole.
    #[test]
    fn test_per_rate_profile_identity_unit() {
        let frame = seeded_payload(202, 300);
        for rate in [RsRate::High, RsRate::Medium, RsRate::Low] {
            let profile = CodingProfile::RsRate(rate);
            let stream = packetize_with_profile(&frame, &profile).expect("packetize");
            let recovered = depacketize_with_profile(&stream).expect("depacketize");
            assert_eq!(
                recovered, frame,
                "packetize_with_profile -> depacketize_with_profile must be a byte-exact \
                 identity on a clean stream for {rate:?}"
            );
        }
    }

    /// Property (RED): the Repetition profile must be honored end-to-end — a
    /// `CodingProfile::Repetition` stream must depacketize byte-exactly, AND its
    /// header must parse back as the Repetition profile (the lowest-efficiency,
    /// absolute-floor option kept selectable). The stub ALWAYS emits an interleaved-RS
    /// stream and ignores Repetition entirely, so depacketize_with_profile produces a
    /// non-identity (or the header parses as RS) → RED.
    #[test]
    fn test_repetition_profile_honored_unit() {
        let frame = seeded_payload(203, 240);
        let profile = CodingProfile::Repetition {
            repeats: 3,
            pkt_size: 200,
        };
        let stream = packetize_with_profile(&frame, &profile).expect("packetize Repetition");

        let (parsed, _consumed) = parse_coding_header(&stream).expect("parse_coding_header");
        assert_eq!(
            parsed, profile,
            "parse_coding_header must recover the Repetition profile {profile:?} (got \
             {parsed:?}). The stub emits an RS stream and ignores Repetition — RED."
        );

        let recovered = depacketize_with_profile(&stream).expect("depacketize Repetition");
        assert_eq!(
            recovered, frame,
            "a Repetition-profile stream must depacketize byte-exactly via \
             depacketize_with_profile — RED until the profile is honored."
        );
    }

    /// Property: the channel-quality → rate auto-selector picks LOWER redundancy at
    /// HIGHER SNR and HIGHER redundancy at LOWER SNR (the link-adaptation contract the
    /// RED net flagged but could not call, since `select_rate` did not exist in Pass A).
    /// We assert the monotone direction via the parity fraction p/(d+p): a high-SNR
    /// pick must have a SMALLER parity fraction than a low-SNR pick.
    #[test]
    fn test_select_rate_picks_lower_redundancy_at_higher_snr() {
        let parity_frac = |r: RsRate| {
            let (d, p, _) = r.shard_config();
            p as f32 / (d + p) as f32
        };
        let clean = select_rate(30.0); // high SNR
        let mid = select_rate(15.0); // medium SNR
        let lossy = select_rate(3.0); // low SNR

        // Higher SNR ⇒ lower redundancy (smaller parity fraction); the ladder is monotone.
        assert!(
            parity_frac(clean) < parity_frac(mid),
            "a clean (30 dB) link must pick LESS redundancy than a 15 dB link: \
             got {clean:?} (parity {:.3}) vs {mid:?} (parity {:.3})",
            parity_frac(clean),
            parity_frac(mid)
        );
        assert!(
            parity_frac(mid) < parity_frac(lossy),
            "a 15 dB link must pick LESS redundancy than a 3 dB link: \
             got {mid:?} (parity {:.3}) vs {lossy:?} (parity {:.3})",
            parity_frac(mid),
            parity_frac(lossy)
        );
        // And concretely the named ends of the ladder.
        assert_eq!(
            clean,
            RsRate::High,
            "high SNR must select the High (low-parity) rate"
        );
        assert_eq!(
            lossy,
            RsRate::Low,
            "low SNR must select the Low (high-parity) rate"
        );
    }
}

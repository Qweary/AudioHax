// tests/modem_roundtrip.rs
//
// AudioHax MFSK modem — POSITIVE round-trip regression net (WS-2 Phase A).
//
// This file establishes a byte-identity safety net over the modem's CURRENT
// behavior: every test proves that data driven forward through one stage (or the
// whole pipeline) comes back byte-for-byte identical when driven back through the
// inverse. It is intentionally a POSITIVE net only — no CRC-failure, corruption,
// or channel-impairment tests live here (those are a later phase). Each test
// carries a top comment naming the exact property it pins.
//
// Everything runs IN MEMORY: no WAV files, no hound file I/O, no audio hardware.
//
// Run headless (avoids OpenCV/ALSA):
//   cargo test --test modem_roundtrip --no-default-features

use audiohax::modem::{self, ModemParams};

use rand::RngCore;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

// A fixed 32-byte (64 hex char) AES-256 key used for all encrypted-frame tests,
// so encryption round-trips are deterministic at the API boundary.
const TEST_KEY_HEX: &str = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";

/// Deterministic pseudo-random payload of `len` bytes, seeded so tests are stable.
fn seeded_payload(seed: u64, len: usize) -> Vec<u8> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut v = vec![0u8; len];
    rng.fill_bytes(&mut v);
    v
}

// ============================================================================
// CATEGORY 1 — FRAME ROUND-TRIP across all 4 flag combinations
// Property: build_frame(...) -> extract_frame(...) recovers the identical
// filename AND payload bytes, for {plain, gzip, AES, gzip+AES}.
// ============================================================================

/// Property: a plain (no compress, no encrypt) frame round-trips filename+payload.
#[test]
fn test_frame_roundtrip_plain() {
    let filename = "plain.bin";
    let payload = seeded_payload(1, 300);

    let frame = modem::build_frame(filename, &payload, false, None).expect("build_frame");
    let (fname_out, payload_out) = modem::extract_frame(&frame, None).expect("extract_frame");

    assert_eq!(fname_out, filename, "filename must round-trip (plain)");
    assert_eq!(
        payload_out, payload,
        "payload bytes must round-trip (plain)"
    );
}

/// Property: a gzip-compressed frame round-trips filename+payload (compress flag set,
/// decompressed on extract).
#[test]
fn test_frame_roundtrip_compressed() {
    let filename = "compressed.bin";
    // Compressible-ish but still varied payload.
    let payload = seeded_payload(2, 400);

    let frame = modem::build_frame(filename, &payload, true, None).expect("build_frame");
    let (fname_out, payload_out) = modem::extract_frame(&frame, None).expect("extract_frame");

    assert_eq!(fname_out, filename, "filename must round-trip (compressed)");
    assert_eq!(
        payload_out, payload,
        "payload bytes must round-trip (compressed)"
    );
}

/// Property: an AES-256-GCM encrypted frame round-trips filename+payload when the
/// same key is supplied on both sides (nonce is carried in the payload).
#[test]
fn test_frame_roundtrip_encrypted() {
    let filename = "encrypted.bin";
    let payload = seeded_payload(3, 256);

    let frame =
        modem::build_frame(filename, &payload, false, Some(TEST_KEY_HEX)).expect("build_frame");
    let (fname_out, payload_out) =
        modem::extract_frame(&frame, Some(TEST_KEY_HEX)).expect("extract_frame");

    assert_eq!(fname_out, filename, "filename must round-trip (encrypted)");
    assert_eq!(
        payload_out, payload,
        "payload bytes must round-trip (encrypted)"
    );
}

/// Property: a compressed-AND-encrypted frame (both flags) round-trips
/// filename+payload — gzip-then-encrypt forward, decrypt-then-gunzip on extract.
#[test]
fn test_frame_roundtrip_encrypted_compressed() {
    let filename = "comp_enc.bin";
    let payload = seeded_payload(4, 512);

    let frame =
        modem::build_frame(filename, &payload, true, Some(TEST_KEY_HEX)).expect("build_frame");
    let (fname_out, payload_out) =
        modem::extract_frame(&frame, Some(TEST_KEY_HEX)).expect("extract_frame");

    assert_eq!(
        fname_out, filename,
        "filename must round-trip (compressed+encrypted)"
    );
    assert_eq!(
        payload_out, payload,
        "payload bytes must round-trip (compressed+encrypted)"
    );
}

/// Property: the parsed header flags agree with the requested compress/encrypt
/// options for each of the 4 combinations (header-level sanity for the net).
#[test]
fn test_frame_header_flags_match_options() {
    let payload = seeded_payload(5, 128);
    let cases = [
        (false, None, false, false),
        (true, None, true, false),
        (false, Some(TEST_KEY_HEX), false, true),
        (true, Some(TEST_KEY_HEX), true, true),
    ];
    for (compress, key, exp_compressed, exp_encrypted) in cases {
        let frame = modem::build_frame("f.bin", &payload, compress, key).expect("build_frame");
        let (fname, compressed, encrypted, _start, _len, _crc) =
            modem::parse_frame_header(&frame).expect("parse_frame_header");
        assert_eq!(fname, "f.bin");
        assert_eq!(compressed, exp_compressed, "compressed flag mismatch");
        assert_eq!(encrypted, exp_encrypted, "encrypted flag mismatch");
    }
}

// ============================================================================
// CATEGORY 2 — SYMBOL ENCODING identity
// Property: bytes_to_symbols -> symbols_to_bytes is the identity on the input
// byte stream, across several m_tones (hence several bits_per_symbol). Note the
// symbol stream may carry trailing zero-fill bits in a final partial symbol, so
// identity is asserted on the original-length prefix of the decoded output.
// ============================================================================

/// Property: byte<->symbol packing is loss-free for several m_tones values that
/// exercise different bits_per_symbol (2,3,4,5,6 bits), over multiple payloads.
#[test]
fn test_symbol_encoding_identity_various_mtones() {
    // m_tones -> expected bits_per_symbol: 4->2, 8->3, 16->4, 32->5, 64->6
    let mtones_values = [4usize, 8, 16, 32, 64];
    let payloads = [
        seeded_payload(10, 200),
        seeded_payload(11, 333),
        b"The quick brown fox jumps over 0123456789".to_vec(),
    ];

    for &m in &mtones_values {
        let bps = modem::bits_per_symbol(m);
        assert!(bps >= 1, "m_tones {} should yield >=1 bits/symbol", m);

        for payload in &payloads {
            let symbols = modem::bytes_to_symbols(payload, m);
            // Every symbol must be representable in the chosen tone alphabet.
            for &s in &symbols {
                assert!(
                    (s as usize) < m,
                    "symbol {} out of range for m_tones {}",
                    s,
                    m
                );
            }
            let recovered = modem::symbols_to_bytes(&symbols, m);
            assert!(
                recovered.len() >= payload.len(),
                "decoded byte stream shorter than input (m_tones {})",
                m
            );
            assert_eq!(
                &recovered[..payload.len()],
                &payload[..],
                "byte<->symbol identity failed for m_tones {} (bps {})",
                m,
                bps
            );
        }
    }
}

// ============================================================================
// CATEGORY 3 — PACKETIZATION round-trip (clean, no impairment this phase)
// Property: each packetizer's depacketizer recovers the original byte stream.
// ============================================================================

/// Property: repetition-FEC packetize_stream -> depacketize_stream recovers the
/// original stream under clean conditions, for several (pkt_size, repeats) pairs.
#[test]
fn test_packetize_repetition_roundtrip() {
    let data = seeded_payload(20, 500);
    let cases = [(64usize, 1usize), (128, 3), (200, 4), (50, 5)];

    for (pkt_size, repeats) in cases {
        let packetized = modem::packetize_stream(&data, pkt_size, repeats);
        let recovered =
            modem::depacketize_stream(&packetized, repeats).expect("depacketize_stream");
        assert_eq!(
            recovered, data,
            "repetition packetize round-trip failed (pkt_size {}, repeats {})",
            pkt_size, repeats
        );
    }
}

/// Property: Reed-Solomon packetize_stream_rs -> depacketize_stream_rs recovers
/// the original stream under clean conditions (all shards present), for several
/// (shard_size, data_shards, parity_shards) configs.
#[test]
fn test_packetize_rs_roundtrip() {
    let data = seeded_payload(21, 500);
    // (shard_size, data_shards, parity_shards)
    let cases = [
        (128usize, 4usize, 2usize),
        (64, 6, 3),
        (32, 8, 4),
        (128, 4, 5),
    ];

    for (shard_size, d, p) in cases {
        let packetized =
            modem::packetize_stream_rs(&data, shard_size, d, p).expect("packetize_stream_rs");
        let recovered = modem::depacketize_stream_rs(&packetized).expect("depacketize_stream_rs");
        assert_eq!(
            recovered, data,
            "RS packetize round-trip failed (shard_size {}, d {}, p {})",
            shard_size, d, p
        );
    }
}

/// Property: the interleaved RS packetizer (packetize_stream_rs_interleaved) shares
/// the RS01 header format with packetize_stream_rs, so depacketize_stream_rs
/// recovers the original stream under clean conditions.
#[test]
fn test_packetize_rs_interleaved_roundtrip() {
    let data = seeded_payload(22, 500);
    // (data_shards, parity_shards, shard_size) — note arg order differs from rs().
    let cases = [
        (4usize, 2usize, 128usize),
        (6, 3, 64),
        (8, 4, 32),
        (4, 5, 128),
    ];

    for (d, p, shard_size) in cases {
        let packetized = modem::packetize_stream_rs_interleaved(&data, d, p, shard_size);
        assert!(
            !packetized.is_empty(),
            "interleaved RS produced empty output (d {}, p {}, shard_size {})",
            d,
            p,
            shard_size
        );
        let recovered = modem::depacketize_stream_rs(&packetized).expect("depacketize_stream_rs");
        assert_eq!(
            recovered, data,
            "interleaved RS round-trip failed (d {}, p {}, shard_size {})",
            d, p, shard_size
        );
    }
}

// ============================================================================
// CATEGORY 4 — FULL AUDIO PIPELINE round-trip, in memory
// Property: payload -> frame -> packetize -> symbols -> round-robin -> preamble
// -> render_symbols_to_samples (i16), then the decode path (Goertzel per symbol
// window per channel -> preamble alignment -> reinterleave -> symbols_to_bytes
// -> depacketize -> extract_frame) recovers the identical filename + payload.
// This mirrors src/bin/modem_encode.rs + src/bin/modem_decode.rs entirely in RAM.
//
// IMPORTANT — PARAMS CHOICE FOR THE GREEN NET (see also the bug report below):
// The green full-pipeline tests use `well_separated_params()` (the "balanced"
// preset baked into src/bin/modem_encode.rs: 2 channels, 8 tones, 40 ms symbols).
// This config keeps the per-channel tone bands non-overlapping, so clean
// (noise-free) Goertzel detection recovers every symbol exactly — it is the
// configuration under which the acoustic round trip is byte-exact on CURRENT code.
//
// The library DEFAULT params (4 channels, 32 tones, 30 Hz tone spacing, 400 Hz
// channel spacing) do NOT round-trip even with zero channel noise: the four
// summed channel bands overlap heavily, so Goertzel on one channel's band picks
// up adjacent channels' energy, the pilot/preamble is mis-detected, and ~54% of
// symbols are wrong. That fragility is pinned (not hidden) by the explicit
// `test_full_pipeline_default_params_is_currently_lossy` test below, which
// asserts the CURRENT (broken) behavior so a future fix flips it visibly.
// ============================================================================

/// A separation-correct fixed params set: the "balanced" preset from
/// src/bin/modem_encode.rs. Under this config the noise-free acoustic round trip
/// is byte-exact on current code (verified: 0 symbol errors).
fn well_separated_params() -> ModemParams {
    let mut p = ModemParams::default();
    p.channels = 2;
    p.m_tones = 8;
    p.symbol_ms = 40.0;
    // Pilot/preamble = middle tone of the 8-tone alphabet (matches the
    // "middle tone as pilot" intent in ModemParams::default()).
    p.preamble_symbols = vec![(8 / 2) as u8];
    p
}

/// Decode-side mirror of src/bin/modem_decode.rs, driven purely in memory from
/// an i16 sample buffer. Returns the recovered (filename, payload).
///
/// `depacketize`: a closure that turns the recovered "frame-ish" bytes into frame
/// bytes (RS or repetition or identity), matching how the encoder packetized.
fn decode_samples_to_frame(
    samples_i16: &[i16],
    params: &ModemParams,
    decrypt_key_hex: Option<&str>,
    depacketize: impl Fn(&[u8]) -> Vec<u8>,
) -> Result<(String, Vec<u8>), Box<dyn std::error::Error>> {
    let samples_per_symbol =
        ((params.sample_rate as f32) * (params.symbol_ms / 1000.0)).round() as usize;

    let tone_freqs = modem::build_tone_frequencies(params);

    // Goertzel detection: per symbol window, per channel, pick the strongest tone.
    let mut detected_by_channel: Vec<Vec<u8>> = vec![Vec::new(); params.channels];
    let mut window_start = 0usize;
    while window_start + samples_per_symbol <= samples_i16.len() {
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
        window_start += samples_per_symbol;
    }

    // Preamble detection & per-channel alignment (mirrors decode bin).
    if !params.preamble_symbols.is_empty() && params.preamble_repeats > 0 {
        let mut pattern: Vec<u8> = Vec::new();
        for _ in 0..params.preamble_repeats {
            pattern.extend_from_slice(&params.preamble_symbols);
        }
        let pat_len = pattern.len();
        for ch in 0..params.channels {
            let chvec = &mut detected_by_channel[ch];
            if let Some(idx) = find_subslice(chvec, &pattern) {
                if idx + pat_len <= chvec.len() {
                    *chvec = chvec[idx + pat_len..].to_vec();
                } else {
                    *chvec = Vec::new();
                }
            }
        }
    }

    // Round-robin reinterleave (inverse of split_round_robin).
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

    let bytes = modem::symbols_to_bytes(&symbols, params.m_tones);

    // Depacketize back to frame bytes.
    let frame_bytes = depacketize(&bytes);

    // Extract frame.
    Ok(modem::extract_frame(&frame_bytes, decrypt_key_hex)?)
}

/// find a subslice pattern in `haystack`; returns first index if found, else None.
/// (Local copy of the helper used in src/bin/modem_decode.rs.)
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

/// Encode-side mirror of src/bin/modem_encode.rs producing an in-memory i16 buffer.
/// `packetize` turns frame bytes into the on-wire packetized byte stream.
fn encode_frame_to_samples(
    frame: &[u8],
    params: &ModemParams,
    packetize: impl Fn(&[u8]) -> Vec<u8>,
) -> Vec<i16> {
    let packetized = packetize(frame);
    let symbols = modem::bytes_to_symbols(&packetized, params.m_tones);
    let mut channels_syms = modem::split_round_robin(&symbols, params.channels);

    if !params.preamble_symbols.is_empty() && params.preamble_repeats > 0 {
        let mut pre_vec: Vec<u8> =
            Vec::with_capacity(params.preamble_symbols.len() * params.preamble_repeats);
        for _ in 0..params.preamble_repeats {
            pre_vec.extend_from_slice(&params.preamble_symbols);
        }
        for ch_syms in channels_syms.iter_mut() {
            let mut newv = pre_vec.clone();
            newv.extend_from_slice(ch_syms);
            *ch_syms = newv;
        }
    }

    modem::render_symbols_to_samples(&channels_syms, params)
}

/// Property: FULL pipeline, repetition-FEC, plain frame — payload survives the
/// round trip all the way through rendered i16 samples and back via Goertzel.
#[test]
fn test_full_pipeline_repetition_plain() {
    let params = well_separated_params();
    let filename = "pipe_rep.bin";
    let payload = seeded_payload(30, 300);
    let pkt_size = 200usize;
    let repeats = 3usize;

    let frame = modem::build_frame(filename, &payload, false, None).expect("build_frame");

    let samples = encode_frame_to_samples(&frame, &params, |f| {
        modem::packetize_stream(f, pkt_size, repeats)
    });

    let (fname_out, payload_out) = decode_samples_to_frame(&samples, &params, None, |bytes| {
        modem::depacketize_stream(bytes, repeats).unwrap_or_else(|_| bytes.to_vec())
    })
    .expect("decode full pipeline (repetition/plain)");

    assert_eq!(
        fname_out, filename,
        "filename must survive full pipeline (rep/plain)"
    );
    assert_eq!(
        payload_out, payload,
        "payload must survive full pipeline (rep/plain)"
    );
}

/// Property: FULL pipeline, repetition-FEC, compressed+encrypted frame — exercises
/// the header flags through the whole acoustic round trip in memory.
#[test]
fn test_full_pipeline_repetition_compressed_encrypted() {
    let params = well_separated_params();
    let filename = "pipe_rep_ce.bin";
    let payload = seeded_payload(31, 256);
    let pkt_size = 200usize;
    let repeats = 3usize;

    let frame =
        modem::build_frame(filename, &payload, true, Some(TEST_KEY_HEX)).expect("build_frame");

    let samples = encode_frame_to_samples(&frame, &params, |f| {
        modem::packetize_stream(f, pkt_size, repeats)
    });

    let (fname_out, payload_out) =
        decode_samples_to_frame(&samples, &params, Some(TEST_KEY_HEX), |bytes| {
            modem::depacketize_stream(bytes, repeats).unwrap_or_else(|_| bytes.to_vec())
        })
        .expect("decode full pipeline (repetition/comp+enc)");

    assert_eq!(
        fname_out, filename,
        "filename must survive full pipeline (rep/comp+enc)"
    );
    assert_eq!(
        payload_out, payload,
        "payload must survive full pipeline (rep/comp+enc)"
    );
}

/// Property: FULL pipeline, Reed-Solomon FEC (non-interleaved), plain frame —
/// payload survives the round trip through rendered samples and RS depacketize.
#[test]
fn test_full_pipeline_rs_plain() {
    let params = well_separated_params();
    let filename = "pipe_rs.bin";
    let payload = seeded_payload(32, 300);
    let (shard_size, d, p) = (128usize, 4usize, 2usize);

    let frame = modem::build_frame(filename, &payload, false, None).expect("build_frame");

    let samples = encode_frame_to_samples(&frame, &params, |f| {
        modem::packetize_stream_rs(f, shard_size, d, p).expect("packetize_stream_rs")
    });

    let (fname_out, payload_out) = decode_samples_to_frame(&samples, &params, None, |bytes| {
        modem::depacketize_stream_rs(bytes).unwrap_or_else(|_| bytes.to_vec())
    })
    .expect("decode full pipeline (RS/plain)");

    assert_eq!(
        fname_out, filename,
        "filename must survive full pipeline (RS/plain)"
    );
    assert_eq!(
        payload_out, payload,
        "payload must survive full pipeline (RS/plain)"
    );
}

/// Property: FULL pipeline, interleaved Reed-Solomon FEC, plain frame — the
/// interleaved packetizer's output decodes back to the identical payload through
/// the full in-memory acoustic round trip.
#[test]
fn test_full_pipeline_rs_interleaved_plain() {
    let params = well_separated_params();
    let filename = "pipe_rs_il.bin";
    let payload = seeded_payload(33, 300);
    let (d, p, shard_size) = (4usize, 2usize, 128usize);

    let frame = modem::build_frame(filename, &payload, false, None).expect("build_frame");

    let samples = encode_frame_to_samples(&frame, &params, |f| {
        modem::packetize_stream_rs_interleaved(f, d, p, shard_size)
    });

    let (fname_out, payload_out) = decode_samples_to_frame(&samples, &params, None, |bytes| {
        modem::depacketize_stream_rs(bytes).unwrap_or_else(|_| bytes.to_vec())
    })
    .expect("decode full pipeline (RS-interleaved/plain)");

    assert_eq!(
        fname_out, filename,
        "filename must survive full pipeline (RS-interleaved/plain)"
    );
    assert_eq!(
        payload_out, payload,
        "payload must survive full pipeline (RS-interleaved/plain)"
    );
}

// ----------------------------------------------------------------------------
// BUG PIN — default-params acoustic round trip must be BYTE-EXACT (zero noise).
//
// This is the WS-2 acoustic-hardening DONE-SIGNAL. The library DEFAULT
// ModemParams must, on a CLEAN (noise-free) in-memory signal, drive a payload
// all the way through the rendered i16 samples and back via Goertzel and recover
// it byte-for-byte — exactly as `well_separated_params()` already does.
//
// HISTORY (the defect this test now pins the FIX of): the HEAD default
// ModemParams (4 channels, 32 tones, 30 Hz tone spacing, 400 Hz channel spacing,
// 400 Hz base) lays out per-channel tone bands that overlap heavily — ch0 spans
// 400..1330 Hz, ch1 spans 800..1730 Hz, etc. Summed into one mono stream the
// bands collide, so Goertzel on one channel's band picks up adjacent channels'
// energy: the preamble pilot is never reliably detected and ~54% of data symbols
// are mis-detected even with ZERO channel noise, and extract_frame fails to find
// the "AHX1" magic. (Distinct from the FluidSynth music-band collision — this is
// an INTERNAL self-collision of the default frequency plan.)
//
// RED NOW / GREEN AT FIX: on HEAD this test FAILS (the default plan is broken).
// The Signal Processing Specialist fixes the default frequency plan / detector so
// the default-params round trip becomes byte-exact; THIS TEST GOING GREEN is the
// session's done-signal. It is deliberately NOT #[ignore]-d so it cannot rot.
//
// Discovered while building the WS-2 Phase A net; flipped to its positive form in
// the WS-2 acoustic-hardening RED pass (S5).
#[test]
fn test_full_pipeline_default_params_roundtrip() {
    let params = ModemParams::default(); // post-fix: a non-overlapping, music-clear plan
    let filename = "pipe_default.bin";
    let payload = seeded_payload(40, 300);
    let pkt_size = 200usize;
    let repeats = 3usize;

    let frame = modem::build_frame(filename, &payload, false, None).expect("build_frame");

    let samples = encode_frame_to_samples(&frame, &params, |f| {
        modem::packetize_stream(f, pkt_size, repeats)
    });

    let (fname_out, payload_out) = decode_samples_to_frame(&samples, &params, None, |bytes| {
        modem::depacketize_stream(bytes, repeats).unwrap_or_else(|_| bytes.to_vec())
    })
    .expect(
        "default-params acoustic round trip must extract Ok on a clean signal \
         (RED until the default frequency plan is fixed — see BUG PIN above)",
    );

    assert_eq!(
        fname_out, filename,
        "filename must survive the default-params full pipeline on a clean signal"
    );
    assert_eq!(
        payload_out, payload,
        "DEFAULT-PARAMS DONE-SIGNAL: payload must round-trip byte-exactly under \
         ModemParams::default() on a clean signal (RED until the overlapping default \
         frequency plan is fixed — see BUG PIN above)"
    );
}

// ----------------------------------------------------------------------------
// PREAMBLE / PILOT DETECTION on the default params (clean signal).
//
// Property (RED NOW): under ModemParams::default(), the per-channel preamble
// pilot tone must be reliably detected at the correct symbol offset so the
// decoder can strip the preamble and align to the true frame start. We drive a
// known payload through the encode mirror (which prepends `preamble_repeats`
// copies of `preamble_symbols` per channel), run the Goertzel detector per
// channel, and assert that the FULL preamble pattern is found at offset 0 of
// every channel's detected-symbol stream (the encoder prepends it first, so a
// correctly-separated plan detects it at the very start).
//
// On HEAD the overlapping default bands cause the pilot to be mis-detected, so
// the pattern is NOT found at offset 0 (often not found at all) — RED. Once the
// frequency plan is fixed, the pilot detects cleanly at offset 0 — GREEN. This
// isolates the PREAMBLE-ALIGNMENT half of the defect from the data-symbol half
// pinned by the round-trip test above.
#[test]
fn test_preamble_pilot_detected_on_default_params() {
    let params = ModemParams::default();
    assert!(
        !params.preamble_symbols.is_empty() && params.preamble_repeats > 0,
        "default params must carry a preamble for this test to be meaningful"
    );

    let filename = "pre_default.bin";
    let payload = seeded_payload(41, 200);
    let frame = modem::build_frame(filename, &payload, false, None).expect("build_frame");

    // Encode with the same preamble-prepend the decoder expects.
    let samples = encode_frame_to_samples(&frame, &params, |f| modem::packetize_stream(f, 200, 3));

    // Re-run the raw Goertzel detection (no preamble stripping) so we can inspect
    // the per-channel detected-symbol streams directly.
    let samples_per_symbol =
        ((params.sample_rate as f32) * (params.symbol_ms / 1000.0)).round() as usize;
    let tone_freqs = modem::build_tone_frequencies(&params);
    let mut detected_by_channel: Vec<Vec<u8>> = vec![Vec::new(); params.channels];
    let mut window_start = 0usize;
    while window_start + samples_per_symbol <= samples.len() {
        let slice = &samples[window_start..window_start + samples_per_symbol];
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
        window_start += samples_per_symbol;
    }

    // Build the expected preamble pattern (repeated pilot symbols), one per channel.
    let mut pattern: Vec<u8> = Vec::new();
    for _ in 0..params.preamble_repeats {
        pattern.extend_from_slice(&params.preamble_symbols);
    }

    for ch in 0..params.channels {
        let detected = &detected_by_channel[ch];
        let idx = find_subslice(detected, &pattern);
        assert_eq!(
            idx,
            Some(0),
            "channel {ch}: preamble pilot pattern {:?} must be detected at symbol offset 0 \
             on a clean default-params signal (got {:?}; detected head = {:?}). RED until the \
             overlapping default frequency plan is fixed.",
            pattern,
            idx,
            &detected[..detected.len().min(pattern.len() + 2)]
        );
    }
}

// ----------------------------------------------------------------------------
// FEC RECOVERY under noise + bounded burst loss (DEFAULT params, deterministic).
//
// Property (RED NOW for default params): with the default frequency plan fixed,
// a payload packetized with INTERLEAVED Reed-Solomon, rendered to audio, then
// passed through a SEEDED channel model that (a) applies moderate AWGN across the
// whole audio stream and (b) ZEROES one bounded contiguous run of audio samples (a
// dropout — the audio-domain analogue of channel_sim.rs's byte-burst erasure),
// must still RECOVER the payload byte-exactly. The dropout spans only a few symbol
// windows; with INTERLEAVED RS those lost symbols spread across many blocks so each
// block stays within its parity capacity and is reconstructible. AWGN is moderate
// so residual symbol errors stay sparse.
//
// On HEAD this is RED because the default plan does not even round-trip on a CLEAN
// signal — so it certainly cannot under impairment. Once the plan is fixed (clean
// round-trip green), the RS+interleave FEC carries the bounded burst and this goes
// GREEN. Determinism comes from a seeded ChaCha8Rng so the noise is reproducible
// and the dropout placement is fixed.
//
// The channel model here replicates channel_sim.rs's primitives — AWGN and a
// fixed-length contiguous erasure — at the audio (i16) layer, which is the layer
// the real decode path consumes; channel_sim's primitives live only in the bin, so
// we keep a minimal SEEDED model inline.
#[test]
fn test_fec_recovers_bounded_burst_default_params() {
    let params = ModemParams::default();
    let filename = "fec_default.bin";
    let payload = seeded_payload(42, 256);
    // Strong RS: 4 data + 4 parity, interleaved, spreads a burst across blocks.
    let (d, p, shard_size) = (4usize, 4usize, 128usize);

    let frame = modem::build_frame(filename, &payload, false, None).expect("build_frame");

    // Encode to audio via interleaved RS.
    let mut samples = encode_frame_to_samples(&frame, &params, |f| {
        modem::packetize_stream_rs_interleaved(f, d, p, shard_size)
    });

    // (a) Moderate AWGN across the whole stream — seeded for determinism.
    let mut rng = ChaCha8Rng::seed_from_u64(2025);
    add_awgn_i16(&mut samples, 200.0, &mut rng);

    // (b) One BOUNDED contiguous dropout (burst erasure): zero a run spanning a
    // few symbol windows, placed deterministically a third of the way in (clear of
    // the preamble). Interleaving spreads the lost symbols across RS blocks so the
    // burst stays within per-block parity capacity.
    let samples_per_symbol =
        ((params.sample_rate as f32) * (params.symbol_ms / 1000.0)).round() as usize;
    let burst_symbols = 3usize; // a few symbol windows — within interleaved-RS capacity
    let burst_len = burst_symbols * samples_per_symbol;
    if samples.len() > burst_len {
        let start = samples.len() / 3;
        let end = (start + burst_len).min(samples.len());
        for s in &mut samples[start..end] {
            *s = 0;
        }
    }

    // Decode through the real audio path under both impairments.
    let (fname_out, payload_out) = decode_samples_to_frame(&samples, &params, None, |bytes| {
        modem::depacketize_stream_rs(bytes).unwrap_or_else(|_| bytes.to_vec())
    })
    .expect(
        "default-params interleaved-RS pipeline must recover under moderate AWGN + a \
         bounded dropout on a clean plan (RED until the default frequency plan is fixed)",
    );

    assert_eq!(
        fname_out, filename,
        "filename must survive default-params RS pipeline under AWGN + bounded burst"
    );
    assert_eq!(
        payload_out, payload,
        "payload must be RECOVERED byte-exact under moderate AWGN + a bounded burst \
         within interleaved-RS capacity (default params; RED until the frequency plan is fixed)"
    );
}

// ----------------------------------------------------------------------------
// FEC GRACEFUL FAILURE beyond capacity (packet-byte layer, deterministic).
//
// Property: when damage EXCEEDS the FEC's correction capacity, depacketize +
// extract must produce a typed Err (or non-identical bytes that fail the CRC and
// thus surface as Err at extract) — NEVER silently-wrong bytes, never a panic.
// We packetize with interleaved RS, then ERASE far more than `parity_shards`
// worth of shards via a long seeded byte-burst, and assert the recovery path
// returns Err at depacketize OR at extract_frame (CRC enforcement).
//
// This is largely PLAN-INDEPENDENT (it operates on the packet byte stream, not
// the audio), so it may already be green — it guards the graceful-failure
// contract regardless of the frequency-plan fix.
#[test]
fn test_fec_graceful_failure_beyond_capacity_default_params() {
    let params = ModemParams::default();
    let filename = "fec_fail.bin";
    let payload = seeded_payload(43, 256);
    let (d, p, shard_size) = (4usize, 2usize, 128usize);

    let frame = modem::build_frame(filename, &payload, false, None).expect("build_frame");
    let mut packetized = modem::packetize_stream_rs_interleaved(frame.as_slice(), d, p, shard_size);
    assert!(!packetized.is_empty(), "interleaved RS produced no packets");

    // Erase a HUGE contiguous burst — far beyond parity capacity — deterministically.
    // Zeroing wrecks the RS01 magic of many shards across blocks so depacketize
    // either errors (too few shards) or yields bytes that fail the frame CRC.
    let burst_len = (packetized.len() * 3) / 4; // 75% of the stream
    for b in packetized.iter_mut().take(burst_len) {
        *b = 0u8;
    }

    let depacked = modem::depacketize_stream_rs(&packetized);
    match depacked {
        Err(_) => { /* graceful typed Err at depacketize — acceptable */ }
        Ok(bytes) => {
            // If depacketize somehow produced bytes, extract_frame MUST reject them
            // (bad magic / truncation / CRC) rather than emit silently-wrong data.
            match modem::extract_frame(&bytes, None) {
                Err(_) => { /* graceful typed Err at extract — acceptable */ }
                Ok((_f, recovered)) => {
                    // The only non-error outcome that is acceptable is exact recovery;
                    // anything else (silent garbage) is a contract violation.
                    assert_eq!(
                        recovered, payload,
                        "beyond-capacity damage produced SILENTLY-WRONG bytes — \
                         FEC/CRC must fail gracefully, not emit garbage"
                    );
                }
            }
        }
    }
    let _ = params;
}

/// Add seeded additive white Gaussian-ish noise to an i16 sample buffer in place.
/// Uses a Box-Muller transform over the seeded ChaCha8Rng for deterministic,
/// reproducible noise. `sigma` is the noise standard deviation in i16 LSBs.
/// (Replicates the AWGN primitive intent of channel_sim.rs, which only lives in
/// the bin, kept minimal + seeded here for the test.)
fn add_awgn_i16(samples: &mut [i16], sigma: f32, rng: &mut ChaCha8Rng) {
    for s in samples.iter_mut() {
        // Box-Muller: two uniforms in (0,1] -> one standard normal.
        let u1 = ((rng.next_u32() as f32) + 1.0) / (u32::MAX as f32 + 1.0);
        let u2 = (rng.next_u32() as f32) / (u32::MAX as f32 + 1.0);
        let mag = (-2.0 * u1.ln()).sqrt();
        let z = mag * (2.0 * std::f32::consts::PI * u2).cos();
        let noisy = (*s as f32) + z * sigma;
        *s = noisy.clamp(i16::MIN as f32, i16::MAX as f32) as i16;
    }
}

// tests/modem_realair.rs
//
// AudioHax MFSK modem — S7 REAL-AIR ROBUSTNESS test net (WS-2 Pass B).
//
// This is the **RED net** for the S7 sync + rate-selectable-coding work. It drives
// the encode→render→channel→sync→decode pipeline through the REAL seeded acoustic-
// channel model (`simulate_acoustic_channel`, landed in Pass A) and asserts the
// INTENDED real-air behavior of the sync / timing-recovery / rate-coding API that
// Pass A shipped only as COMPILING STUBS. Most of these tests therefore FAIL today
// (RED) and become the concrete spec Pass C must satisfy; a handful (clean-channel
// round-trips, channel-model smoke checks) are GREEN immediately, which is fine —
// they pin the control arm.
//
// Property categories pinned here (see the per-test top comment for which):
//   1. START-OFFSET SYNC                 — detect_burst_start under a real offset
//   2. CLOCK/SAMPLE-RATE DRIFT           — recover_symbol_timing tracks drift
//   3. FREQUENCY OFFSET + MULTIPATH      — coded round-trip survives / fails gracefully
//   4. RATE-SELECTABLE CODING            — overhead ladder + per-rate recovery
//   5. NO REGRESSION                     — pinned by the existing S5 nets (not rewritten)
//
// Everything runs IN MEMORY and is deterministically seeded (rand_chacha::ChaCha8Rng):
// no WAV files, no hound I/O, no audio hardware. Payloads are 200–500 bytes.
//
// Run headless (avoids OpenCV/ALSA):
//   cargo test --test modem_realair --no-default-features
//
// ─────────────────────────────────────────────────────────────────────────────
// NOTE TO PASS C — MISSING AUTO-SELECTOR ENTRY POINT
// Property category 4 pins the OBSERVABLE contract of rate selection (the overhead
// ladder + per-rate recovery) using only functions that exist in the Pass-A stubs,
// so this file COMPILES today. It does NOT test a channel-quality → rate auto-
// selector, because no such entry point exists yet.
//   // PASS C: if an auto-selector `select_rate(...)` is added, add a test asserting
//   //         it picks LOWER redundancy at HIGHER SNR. Expected signature:
//   //             pub fn select_rate(snr_db: f32) -> RsRate;
//   //         (or `fn select_rate(channel: &AcousticChannelParams) -> RsRate;`)
// The lead will relay this to Pass C.
// ─────────────────────────────────────────────────────────────────────────────

use audiohax::modem::{
    self, AcousticChannelParams, CodingProfile, ModemParams, RsRate, SyncMode, SyncParams,
};

use rand::RngCore;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

const TEST_KEY_HEX: &str = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";

/// Deterministic pseudo-random payload of `len` bytes, seeded so tests are stable.
fn seeded_payload(seed: u64, len: usize) -> Vec<u8> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut v = vec![0u8; len];
    rng.fill_bytes(&mut v);
    v
}

// ─────────────────────────────────────────────────────────────────────────────
// SHARED PIPELINE HELPERS
//
// These mirror tests/modem_roundtrip.rs's encode/decode mirrors, but the decode
// mirror is parameterized by a CALLER-SUPPLIED set of symbol-window start
// boundaries (so it can consume the boundaries produced by recover_symbol_timing
// rather than the fixed sample-0 / fixed-stride windows the legacy mirror uses).
// ─────────────────────────────────────────────────────────────────────────────

/// A separation-correct fixed params set (matches well_separated_params() in
/// tests/modem_roundtrip.rs): 2 channels, 8 tones, 40 ms symbols. Under this config
/// a CLEAN noise-free acoustic round trip is byte-exact on current code, so it is
/// the right control vehicle for isolating CHANNEL impairments from frequency-plan
/// fragility.
fn well_separated_params() -> ModemParams {
    let mut p = ModemParams::default();
    p.channels = 2;
    p.m_tones = 8;
    p.symbol_ms = 40.0;
    p.preamble_symbols = vec![(8 / 2) as u8];
    p
}

/// find a subslice pattern in `haystack`; first index if found, else None.
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

/// Encode-side mirror of src/bin/modem_encode.rs producing an in-memory i16 buffer,
/// with an OPTIONAL sync preamble prepended in front of the whole rendered burst.
/// `packetize` turns frame bytes into the on-wire packetized byte stream. The sync
/// preamble (from render_sync_preamble) is prepended to the FULL sample stream so it
/// is orthogonal to the symbol/tone counts (matches the design note's "chirp is
/// prepended to the rendered sample stream").
fn encode_frame_to_samples_synced(
    frame: &[u8],
    params: &ModemParams,
    sync: &SyncParams,
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

    let body = modem::render_symbols_to_samples(&channels_syms, params);

    // Prepend the sync preamble (chirp) to the FULL sample stream.
    let preamble = modem::render_sync_preamble(params, sync);
    let mut out = Vec::with_capacity(preamble.len() + body.len());
    out.extend_from_slice(&preamble);
    out.extend_from_slice(&body);
    out
}

/// Decode an i16 sample buffer into (filename, payload) using EXPLICIT symbol-window
/// start boundaries (e.g. those produced by recover_symbol_timing). Each boundary is
/// the first sample of a symbol window; a window is `samples_per_symbol` long. This
/// is the timing-recovery-aware analogue of decode_samples_to_frame in
/// tests/modem_roundtrip.rs (which uses fixed sample-0 / fixed-stride windows).
fn decode_with_boundaries(
    samples_i16: &[i16],
    params: &ModemParams,
    boundaries: &[usize],
    decrypt_key_hex: Option<&str>,
    depacketize: impl Fn(&[u8]) -> Vec<u8>,
) -> Result<(String, Vec<u8>), Box<dyn std::error::Error>> {
    let samples_per_symbol =
        ((params.sample_rate as f32) * (params.symbol_ms / 1000.0)).round() as usize;
    let tone_freqs = modem::build_tone_frequencies(params);

    let mut detected_by_channel: Vec<Vec<u8>> = vec![Vec::new(); params.channels];
    for &w in boundaries {
        if w + samples_per_symbol > samples_i16.len() {
            continue;
        }
        let slice = &samples_i16[w..w + samples_per_symbol];
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
    let frame_bytes = depacketize(&bytes);
    Ok(modem::extract_frame(&frame_bytes, decrypt_key_hex)?)
}

/// Run the full real-air round trip: encode (with a Chirp sync preamble) → push
/// through the seeded acoustic channel → detect_burst_start → recover_symbol_timing
/// → decode-with-boundaries. Returns the recovered (filename, payload) or an error
/// from any stage. Used by the drift / freq-offset / multipath round-trip tests.
fn realair_roundtrip(
    frame: &[u8],
    params: &ModemParams,
    sync: &SyncParams,
    channel: &AcousticChannelParams,
    decrypt_key_hex: Option<&str>,
    packetize: impl Fn(&[u8]) -> Vec<u8>,
    depacketize: impl Fn(&[u8]) -> Vec<u8>,
) -> Result<(String, Vec<u8>), Box<dyn std::error::Error>> {
    let clean = encode_frame_to_samples_synced(frame, params, sync, packetize);
    let rx = modem::simulate_acoustic_channel(&clean, channel);
    let sync_result = modem::detect_burst_start(&rx, params, sync)?;
    let boundaries = modem::recover_symbol_timing(&rx, params, &sync_result)?;
    decode_with_boundaries(&rx, params, &boundaries, decrypt_key_hex, depacketize)
}

// =============================================================================
// CATEGORY 1 — START-OFFSET SYNC
// =============================================================================

/// Property (category 1, RED): with a Chirp sync preamble rendered in front of the
/// burst and the whole thing shifted by a NON-ZERO `start_offset_samples` (plus the
/// seeded leading noise the channel model prepends), `detect_burst_start` must locate
/// the true burst start within a small sample tolerance AND report confidence above a
/// floor. The stub always returns start_sample = 0 and confidence = 0.0, so this is RED.
///
/// The "true start" of the rendered burst (chirp preamble first sample) after the
/// channel prepends `start_offset_samples` of noise is exactly `start_offset_samples`
/// (the only length-changing stage before mixing/echo is the start-offset prepend;
/// clock_ppm is left at 0 here to keep the geometry exact for the assertion).
#[test]
fn test_sync_finds_start_under_offset() {
    let params = well_separated_params();
    let sync = SyncParams {
        mode: SyncMode::Chirp,
        ..SyncParams::default()
    };
    let payload = seeded_payload(100, 256);
    let frame = modem::build_frame("offset.bin", &payload, false, None).expect("build_frame");

    // Render WITH the chirp preamble in front.
    let clean = encode_frame_to_samples_synced(&frame, &params, &sync, |f| {
        modem::packetize_stream(f, 200, 3)
    });
    assert!(
        !clean.is_empty(),
        "rendered burst (with sync preamble) must be non-empty"
    );

    // Push through the channel with a non-zero, non-symbol-multiple start offset.
    let true_start: isize = 1733; // arbitrary non-multiple-of-samples_per_symbol offset
    let channel = AcousticChannelParams {
        seed: 7,
        start_offset_samples: true_start,
        clock_ppm: 0.0,
        freq_offset_hz: 0.0,
        echo_delay_samples: 0,
        echo_gain: 0.0,
        jitter_samples: 0.0,
    };
    let rx = modem::simulate_acoustic_channel(&clean, &channel);

    let result = modem::detect_burst_start(&rx, &params, &sync)
        .expect("detect_burst_start must not error on a well-formed offset burst");

    let tol = 64usize; // a couple ms at 48 kHz — generous sample-accurate tolerance
    let located = result.start_sample as isize;
    assert!(
        (located - true_start).unsigned_abs() <= tol,
        "detect_burst_start must locate the true burst start under a real start offset: \
         true_start = {true_start}, located = {located} (|err| = {}, tol = {tol}). \
         RED until chirp cross-correlation replaces the offset-0-only stub.",
        (located - true_start).unsigned_abs()
    );
    assert!(
        result.confidence > 0.25,
        "detect_burst_start must report confidence above a floor on a clean (no-noise-only) \
         chirp burst: got confidence = {} (expected > 0.25). RED until the correlation-peak \
         sharpness is computed (stub returns 0.0).",
        result.confidence
    );
}

// =============================================================================
// CATEGORY 2 — CLOCK / SAMPLE-RATE DRIFT, SYMBOL RECOVERY
// =============================================================================

/// Property (category 2, RED): a FULL round trip — encode (Chirp preamble) → render →
/// `simulate_acoustic_channel` with a small `clock_ppm` → detect_burst_start →
/// recover_symbol_timing → decode — recovers the original payload EXACTLY. The stub
/// `recover_symbol_timing` advances by a FIXED integer samples-per-symbol from
/// sample 0, so by late in a drifting burst the window straddles two symbols and
/// detection collapses → the frame fails to extract. RED until early-late tracking
/// lands.
#[test]
fn test_drift_roundtrip_recovers_exact_bytes() {
    let params = well_separated_params();
    let sync = SyncParams {
        mode: SyncMode::Chirp,
        ..SyncParams::default()
    };
    let payload = seeded_payload(101, 300);
    let frame = modem::build_frame("drift.bin", &payload, false, None).expect("build_frame");

    // 500 ppm over this burst accumulates ~1.3k samples of drift by the end (well
    // past a half-symbol slip at 1920 samples/symbol), so the fixed-stride stub's
    // late windows straddle two symbols and decode collapses; a real early-late
    // tracker (which this test asserts recovers) follows the slow boundary slide.
    let channel = AcousticChannelParams {
        seed: 11,
        start_offset_samples: 0,
        clock_ppm: 500.0, // a few hundred ppm of clock error
        freq_offset_hz: 0.0,
        echo_delay_samples: 0,
        echo_gain: 0.0,
        jitter_samples: 0.0,
    };

    let (fname, recovered) = realair_roundtrip(
        &frame,
        &params,
        &sync,
        &channel,
        None,
        |f| modem::packetize_stream(f, 200, 3),
        |bytes| modem::depacketize_stream(bytes, 3).unwrap_or_else(|_| bytes.to_vec()),
    )
    .expect(
        "drift round trip must extract Ok: encode→render→channel(clock_ppm=500)→sync→\
         timing-recovery→decode. RED until recover_symbol_timing tracks clock drift \
         (stub uses a fixed stride and the late-burst windows straddle two symbols).",
    );

    assert_eq!(
        fname, "drift.bin",
        "filename must survive a drifting channel"
    );
    assert_eq!(
        recovered, payload,
        "DRIFT DONE-SIGNAL: payload must round-trip byte-exactly under a few-hundred-ppm \
         clock offset once recover_symbol_timing tracks the drift. RED against the fixed-\
         stride stub."
    );
}

/// Property (category 2, RED): recover_symbol_timing's window starts must TRACK the
/// drift — under a positive clock_ppm the later window centers must shift LATE relative
/// to a no-drift baseline, by an amount that GROWS across the burst (it must not be a
/// fixed stride). We compare the boundaries recovered from a drifted burst against the
/// boundaries recovered from the same burst with NO drift, at the same located start.
/// The stub returns a fixed integer stride in BOTH cases, so the late-window divergence
/// is ~0 → RED.
#[test]
fn test_timing_windows_track_drift_not_fixed_stride() {
    let params = well_separated_params();
    let sync = SyncParams {
        mode: SyncMode::Chirp,
        ..SyncParams::default()
    };
    let payload = seeded_payload(102, 400);
    let frame = modem::build_frame("track.bin", &payload, false, None).expect("build_frame");

    let clean = encode_frame_to_samples_synced(&frame, &params, &sync, |f| {
        modem::packetize_stream(f, 200, 3)
    });

    // No-drift baseline.
    let base_channel = AcousticChannelParams {
        seed: 12,
        ..AcousticChannelParams::identity()
    };
    let rx_base = modem::simulate_acoustic_channel(&clean, &base_channel);
    let sr_base = modem::detect_burst_start(&rx_base, &params, &sync).expect("detect base");
    let b_base = modem::recover_symbol_timing(&rx_base, &params, &sr_base).expect("timing base");

    // Drifted burst (positive ppm stretches the signal: later windows must shift LATE).
    let drift_channel = AcousticChannelParams {
        seed: 12,
        clock_ppm: 400.0,
        ..AcousticChannelParams::identity()
    };
    let rx_drift = modem::simulate_acoustic_channel(&clean, &drift_channel);
    let sr_drift = modem::detect_burst_start(&rx_drift, &params, &sync).expect("detect drift");
    let b_drift =
        modem::recover_symbol_timing(&rx_drift, &params, &sr_drift).expect("timing drift");

    let n = b_base.len().min(b_drift.len());
    assert!(
        n >= 8,
        "need enough recovered windows to measure drift across the burst (got {n})"
    );

    // Per-window divergence of the drifted boundaries from the baseline boundaries,
    // both re-based to their own located start so we measure the STRIDE accumulation,
    // not the start offset.
    let s_base = sr_base.start_sample as isize;
    let s_drift = sr_drift.start_sample as isize;
    let early_idx = n / 8; // near the start
    let late_idx = n - 1; // end of the burst
    let div_early =
        ((b_drift[early_idx] as isize - s_drift) - (b_base[early_idx] as isize - s_base)).abs();
    let div_late =
        ((b_drift[late_idx] as isize - s_drift) - (b_base[late_idx] as isize - s_base)).abs();

    // With real tracking the late divergence grows well beyond the early one (the
    // boundaries slide cumulatively); a fixed-stride stub keeps both ~0.
    assert!(
        div_late > div_early + 16,
        "recover_symbol_timing must TRACK drift: late-window divergence from the no-drift \
         baseline ({div_late} samples) must grow well beyond the early-window divergence \
         ({div_early} samples) — the windows must slide cumulatively, not advance by a fixed \
         stride. RED until early-late tracking replaces the fixed-stride stub."
    );
}

// =============================================================================
// CATEGORY 3 — FREQUENCY OFFSET + MULTIPATH, CODED RECOVERY
// =============================================================================

/// Property (category 3, RED): a full coded round trip through the channel with BOTH
/// a carrier `freq_offset_hz` AND a multipath echo (echo_delay_samples / echo_gain),
/// using an interleaved Reed-Solomon coding profile, recovers the original payload
/// exactly. The frequency offset nudges tone energy off the Goertzel bin centers and
/// the echo smears symbol n−1 into n; the stub sync/timing path cannot correct either,
/// so decode fails → RED until fractional-bin Goertzel + sync land.
#[test]
fn test_freq_offset_plus_multipath_coded_recovers() {
    let params = well_separated_params();
    let sync = SyncParams {
        mode: SyncMode::Chirp,
        ..SyncParams::default()
    };
    let payload = seeded_payload(103, 300);
    let frame = modem::build_frame("multipath.bin", &payload, false, None).expect("build_frame");
    let (d, p, shard_size) = (4usize, 4usize, 128usize); // strong interleaved RS

    // 12 Hz is roughly half a Goertzel bin (25 Hz): big enough that the stub's
    // nearest-bin-rounding detector loses energy / flips to an adjacent tone (the
    // fixed-stride decode breaks at >=12 Hz with this echo), yet < one bin so a real
    // fractional-bin Goertzel restores on-bin behavior and recovers. The echo adds ISI.
    let channel = AcousticChannelParams {
        seed: 21,
        start_offset_samples: 0,
        clock_ppm: 0.0,
        freq_offset_hz: 12.0, // ~half a 25 Hz bin — within plausible fractional-bin correction
        echo_delay_samples: 96, // a short room echo
        echo_gain: 0.35,
        jitter_samples: 0.0,
    };

    let (fname, recovered) = realair_roundtrip(
        &frame,
        &params,
        &sync,
        &channel,
        None,
        |f| modem::packetize_stream_rs_interleaved(f, d, p, shard_size),
        |bytes| modem::depacketize_stream_rs(bytes).unwrap_or_else(|_| bytes.to_vec()),
    )
    .expect(
        "freq-offset + multipath coded round trip must extract Ok: encode→render→\
         channel(freq_offset=12Hz, echo)→sync→timing→RS-decode. RED until fractional-bin \
         Goertzel (freq-offset correction) and real sync land.",
    );

    assert_eq!(
        fname, "multipath.bin",
        "filename must survive a freq-offset + multipath channel"
    );
    assert_eq!(
        recovered, payload,
        "FREQ+MULTIPATH DONE-SIGNAL: interleaved-RS payload must round-trip byte-exactly \
         under a modest carrier offset + a short echo. RED against the stubs."
    );
}

/// Property (category 3, GREEN-or-RED graceful failure): with the echo/offset pushed
/// CLEARLY beyond plausible correction, the decode path must fail GRACEFULLY — a typed
/// Err at depacketize OR at extract_frame (CRC enforcement) — NEVER a panic and NEVER
/// silently-wrong bytes. This guards the failure contract regardless of how good the
/// Pass-C correction becomes (it just must not lie). It may be GREEN today (the stub
/// already fails to recover and surfaces an Err), which is acceptable — it pins the
/// contract.
#[test]
fn test_freq_offset_plus_multipath_beyond_capacity_fails_gracefully() {
    let params = well_separated_params();
    let sync = SyncParams {
        mode: SyncMode::Chirp,
        ..SyncParams::default()
    };
    let payload = seeded_payload(104, 256);
    let frame = modem::build_frame("toofar.bin", &payload, false, None).expect("build_frame");
    let (d, p, shard_size) = (4usize, 2usize, 128usize);

    // Devastating channel: a large carrier offset AND a strong, near-symbol-length echo.
    let channel = AcousticChannelParams {
        seed: 22,
        start_offset_samples: 320,
        clock_ppm: 0.0,
        freq_offset_hz: 900.0, // far beyond the 25 Hz bin grid — energy lands on wrong tones
        echo_delay_samples: 1500, // ~0.8 of a 1920-sample symbol — heavy ISI
        echo_gain: 0.95,       // nearly as strong as the direct path
        jitter_samples: 0.0,
    };

    let clean = encode_frame_to_samples_synced(&frame, &params, &sync, |f| {
        modem::packetize_stream_rs_interleaved(f, d, p, shard_size)
    });
    let rx = modem::simulate_acoustic_channel(&clean, &channel);

    // The whole decode path must not panic. detect_burst_start / recover_symbol_timing
    // may return Ok with garbage boundaries; the recovery must still not lie.
    let outcome: Result<(String, Vec<u8>), Box<dyn std::error::Error>> = (|| {
        let sr = modem::detect_burst_start(&rx, &params, &sync)?;
        let bounds = modem::recover_symbol_timing(&rx, &params, &sr)?;
        decode_with_boundaries(&rx, &params, &bounds, None, |bytes| {
            modem::depacketize_stream_rs(bytes).unwrap_or_else(|_| bytes.to_vec())
        })
    })();

    match outcome {
        Err(_) => { /* graceful typed Err somewhere in the path — acceptable */ }
        Ok((_f, recovered)) => {
            // The ONLY acceptable non-error outcome is exact recovery; silently-wrong
            // bytes are a contract violation.
            assert_eq!(
                recovered, payload,
                "beyond-capacity freq-offset + multipath produced SILENTLY-WRONG bytes — \
                 the decode path must fail gracefully (typed Err) or recover exactly, never \
                 emit garbage"
            );
        }
    }
}

// =============================================================================
// CATEGORY 4 — RATE-SELECTABLE CODING: OVERHEAD LADDER + RECOVERY
// =============================================================================

/// Property (category 4a, RED): packetize_with_profile encoded length must STRICTLY
/// DECREASE from High → Medium → Low redundancy (overhead scales with the rate), and
/// each RS rate must be meaningfully smaller than a brute-force Repetition profile at a
/// comparable protection level. The stub ignores `profile` and always emits a fixed
/// RsRate::Medium stream, so all three RS lengths are EQUAL → the strict-decrease
/// assertion is RED.
#[test]
fn test_rate_overhead_ladder_decreases_high_to_low() {
    let frame = seeded_payload(110, 480);

    let len_high = modem::packetize_with_profile(&frame, &CodingProfile::RsRate(RsRate::High))
        .expect("packetize High")
        .len();
    let len_med = modem::packetize_with_profile(&frame, &CodingProfile::RsRate(RsRate::Medium))
        .expect("packetize Medium")
        .len();
    let len_low = modem::packetize_with_profile(&frame, &CodingProfile::RsRate(RsRate::Low))
        .expect("packetize Low")
        .len();

    // High redundancy (d=8,p=2: 20% parity) is the SMALLEST encoded size; Low
    // (d=4,p=4: 100% parity) is the LARGEST. So encoded length grows High < Medium < Low.
    // "Overhead scales with the rate" = the more-protected profile is bulkier.
    assert!(
        len_high < len_med && len_med < len_low,
        "RS overhead ladder must be strictly ordered by redundancy: encoded lengths must \
         satisfy High({len_high}) < Medium({len_med}) < Low({len_low}). The stub emits a \
         FIXED Medium stream for every profile, so they are equal — RED until \
         packetize_with_profile honors the profile."
    );

    // Each RS rate must be meaningfully smaller than brute-force repetition at a
    // comparable protection level. Low RS gives 2x redundancy (d=4,p=4) with far
    // stronger erasure correction than 2x blind repetition; the bulky 3x repetition
    // profile must be clearly larger than even the most-protected RS rate.
    let len_rep3 = modem::packetize_with_profile(
        &frame,
        &CodingProfile::Repetition {
            repeats: 3,
            pkt_size: 200,
        },
    )
    .expect("packetize Repetition x3")
    .len();
    assert!(
        len_low < len_rep3,
        "the most-protected RS rate (Low, {len_low} bytes) must be meaningfully smaller than \
         a brute-force 3x Repetition profile ({len_rep3} bytes) — RS buys stronger correction \
         per redundancy byte. RED until packetize_with_profile honors the Repetition profile \
         (the stub emits a fixed RS stream and ignores Repetition entirely)."
    );
}

/// Property (category 4b, RED): for EACH RsRate, packetize_with_profile →
/// depacketize_with_profile is a byte-exact identity on a clean stream, AND
/// parse_coding_header recovers the profile that was actually used (so the receiver
/// learns the rate in-band, no out-of-band flags). The stub emits no CDG1 header, so
/// parse_coding_header always returns the DEFAULT (Medium) and 0 consumed — RED for
/// High and Low (the recovered profile won't match what was sent).
#[test]
fn test_per_rate_packetize_identity_and_header_recovers_profile() {
    let frame = seeded_payload(111, 300);

    for rate in [RsRate::High, RsRate::Medium, RsRate::Low] {
        let profile = CodingProfile::RsRate(rate);
        let stream = modem::packetize_with_profile(&frame, &profile)
            .unwrap_or_else(|e| panic!("packetize {rate:?}: {e}"));

        // Byte-exact identity through the profile-aware depacketizer.
        let recovered = modem::depacketize_with_profile(&stream)
            .unwrap_or_else(|e| panic!("depacketize {rate:?}: {e}"));
        assert_eq!(
            recovered, frame,
            "packetize_with_profile -> depacketize_with_profile must be a byte-exact identity \
             on a clean stream for {rate:?}"
        );

        // The receiver must LEARN the rate in-band.
        let (parsed_profile, _consumed) = modem::parse_coding_header(&stream)
            .unwrap_or_else(|e| panic!("parse_coding_header {rate:?}: {e}"));
        assert_eq!(
            parsed_profile, profile,
            "parse_coding_header must recover the profile that packetize_with_profile USED \
             ({profile:?}), so the receiver learns the rate in-band. The stub emits no CDG1 \
             header and always returns the default (Medium), so this is RED for High and Low \
             until the triplicated-CDG1 header is emitted + parsed."
        );
    }
}

/// Property (category 4c, RED): redundancy must scale WITH channel quality and more
/// redundancy must SURVIVE a worse channel. We pin BOTH legs:
///   (i)  a LOW-redundancy rate (High = d8/p2) round-trips through a BENIGN channel
///        (light noise / tiny echo), and
///   (ii) a HIGH-redundancy rate (Low = d4/p4) round-trips through a HARSHER channel
///        (heavier dropout / echo) that would overwhelm the low-redundancy rate.
/// Both legs require the profile to actually take effect end-to-end, so both are RED
/// against the fixed-Medium stub (which neither honors High/Low nor signals the rate).
#[test]
fn test_redundancy_scales_with_channel_quality() {
    let params = well_separated_params();
    let sync = SyncParams {
        mode: SyncMode::Chirp,
        ..SyncParams::default()
    };

    // ── (i) low redundancy survives a benign channel ────────────────────────
    let payload_a = seeded_payload(112, 256);
    let frame_a = modem::build_frame("benign.bin", &payload_a, false, None).expect("build_frame");
    let benign = AcousticChannelParams {
        seed: 31,
        start_offset_samples: 0,
        clock_ppm: 0.0,
        freq_offset_hz: 2.0,    // tiny offset
        echo_delay_samples: 48, // tiny echo
        echo_gain: 0.1,
        jitter_samples: 0.0,
    };
    let (_fa, rec_a) = realair_roundtrip(
        &frame_a,
        &params,
        &sync,
        &benign,
        None,
        |f| {
            modem::packetize_with_profile(f, &CodingProfile::RsRate(RsRate::High))
                .expect("pktz High")
        },
        |bytes| modem::depacketize_with_profile(bytes).unwrap_or_else(|_| bytes.to_vec()),
    )
    .expect(
        "low-redundancy (High rate) must round-trip through a BENIGN channel. RED until \
         packetize_with_profile honors the profile and the sync/timing path corrects the \
         (small) impairments.",
    );
    assert_eq!(
        rec_a, payload_a,
        "low-redundancy High rate must recover exactly through a benign channel"
    );

    // ── (ii) high redundancy survives a harsher channel ─────────────────────
    let payload_b = seeded_payload(113, 256);
    let frame_b = modem::build_frame("harsh.bin", &payload_b, false, None).expect("build_frame");
    // Harsher: a half-bin (12 Hz) carrier offset that the stub's nearest-bin detector
    // cannot follow, plus a heavier echo and a non-multiple start offset that the
    // stub's offset-0 sync misses. A Pass-C with fractional-bin Goertzel + chirp sync
    // + the Low rate's d4/p4 parity recovers; the stub does not -> RED.
    let harsh = AcousticChannelParams {
        seed: 32,
        start_offset_samples: 640,
        clock_ppm: 0.0,
        freq_offset_hz: 12.0,
        echo_delay_samples: 160, // heavier echo
        echo_gain: 0.5,
        jitter_samples: 0.0,
    };
    let (_fb, rec_b) = realair_roundtrip(
        &frame_b,
        &params,
        &sync,
        &harsh,
        None,
        |f| {
            modem::packetize_with_profile(f, &CodingProfile::RsRate(RsRate::Low)).expect("pktz Low")
        },
        |bytes| modem::depacketize_with_profile(bytes).unwrap_or_else(|_| bytes.to_vec()),
    )
    .expect(
        "high-redundancy (Low rate) must round-trip through a HARSHER channel that the \
         low-redundancy rate could not survive. RED until packetize_with_profile honors the \
         profile and the harsher impairments are corrected.",
    );
    assert_eq!(
        rec_b, payload_b,
        "high-redundancy Low rate must recover exactly through the harsher channel — pinning \
         'more redundancy survives a worse channel'"
    );
}

/// Property (category 4, supplementary, likely GREEN): the RsRate shard configs follow
/// the documented redundancy ladder — parity fraction GROWS High → Medium → Low — and
/// shard_config() is internally consistent. This is a pure-data check on the enum (no
/// channel), so it is GREEN today; it pins the ladder semantics the overhead test
/// above relies on, and would catch an accidental Pass-C edit that scrambles the rates.
#[test]
fn test_rsrate_shard_config_redundancy_ladder() {
    let (dh, ph, sh) = RsRate::High.shard_config();
    let (dm, pm, sm) = RsRate::Medium.shard_config();
    let (dl, pl, sl) = RsRate::Low.shard_config();

    assert!(dh > 0 && ph > 0 && sh > 0, "High config must be non-zero");
    assert!(dm > 0 && pm > 0 && sm > 0, "Medium config must be non-zero");
    assert!(dl > 0 && pl > 0 && sl > 0, "Low config must be non-zero");

    // Parity fraction p/(d+p) must strictly increase High < Medium < Low.
    let frac = |d: usize, p: usize| p as f32 / (d + p) as f32;
    let fh = frac(dh, ph);
    let fm = frac(dm, pm);
    let fl = frac(dl, pl);
    assert!(
        fh < fm && fm < fl,
        "RsRate parity fraction must grow with redundancy: High({fh:.3}) < Medium({fm:.3}) < \
         Low({fl:.3})"
    );

    // Custom must pass its explicit values through unchanged.
    let (d, p, s) = RsRate::Custom {
        data_shards: 7,
        parity_shards: 5,
        shard_size: 96,
    }
    .shard_config();
    assert_eq!((d, p, s), (7, 5, 96), "Custom must pass values through");
}

// =============================================================================
// CATEGORY 5 — NO REGRESSION (CONTROL ARM)
// =============================================================================

/// Property (category 5, GREEN): a CLEAN-channel real-air round trip — encode with a
/// Chirp preamble, push through `AcousticChannelParams::identity()` (zero-impairment),
/// then sync + timing-recovery + decode — recovers the payload exactly. This is the
/// CONTROL: it proves the new sync-preamble + boundary-driven decode wiring does not
/// itself break a clean round trip even against the STUBS (start=0, fixed stride,
/// empty chirp preamble), so any RED in categories 1–4 is attributable to the
/// IMPAIRMENT, not to the harness. Expected GREEN today.
#[test]
fn test_clean_channel_realair_roundtrip_is_green_control() {
    let params = well_separated_params();
    let sync = SyncParams {
        mode: SyncMode::Chirp,
        ..SyncParams::default()
    };
    let payload = seeded_payload(120, 300);
    let frame =
        modem::build_frame("control.bin", &payload, true, Some(TEST_KEY_HEX)).expect("build_frame");

    let (fname, recovered) = realair_roundtrip(
        &frame,
        &params,
        &sync,
        &AcousticChannelParams::identity(),
        Some(TEST_KEY_HEX),
        |f| modem::packetize_stream(f, 200, 3),
        |bytes| modem::depacketize_stream(bytes, 3).unwrap_or_else(|_| bytes.to_vec()),
    )
    .expect("clean-channel real-air round trip (control) must extract Ok even against the stubs");

    assert_eq!(
        fname, "control.bin",
        "filename must survive the clean control"
    );
    assert_eq!(
        recovered, payload,
        "CONTROL: a clean (identity) channel must round-trip exactly through the new sync + \
         boundary-driven decode wiring — pinning that the harness itself is sound"
    );
}

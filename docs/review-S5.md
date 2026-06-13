# Quality Gate Review — S5 (WS-2 msfk Acoustic Hardening)

**Reviewer:** Quality Gate (Specialist 6), independent verification
**Date:** 2026-06-12
**Lane under review:** WS-2 modem acoustic hardening — `Cargo.toml`, `src/modem.rs`, `tests/modem_roundtrip.rs`
**Scope exclusion:** The concurrent WS-1 music session (S6) carries +1081 uncommitted lines in `src/chord_engine.rs` with its own in-flight RED net (~7 voice-spacing/dynamics/articulation/rhythm/orchestration tests). **Those are NOT in this review's scope and are NOT a modem defect.** This review filtered to `modem::` for unit tests and ran the modem-only integration suite in isolation. They are excluded from every count and verdict below.

---

## Overall Verdict: **PASS**

The session done-signal is genuine and green, all load-bearing numeric claims were independently re-derived and match exactly, the three substantive fixes (decode isolation, RS-shard CRC, PKT1 CRC vote) are correct at source, and the lane is clean. Two pre-existing non-blocking clippy/fmt items noted below; none touch the S5 changes' correctness.

---

## Compilation Status

- `cargo build --no-default-features` → **Finished** (EXIT 0). Only non-blocking unused-variable/import warnings in pre-existing modem bins.
- Full default `cargo build` not run — opencv/alsa system libs absent in this environment (pre-existing/environmental, expected, not a defect). The Cargo.toml `required-features` fix is precisely what lets the modem lane build/test without those libs.

## Lint Status (NON-BLOCKING)

Three clippy warnings, all pre-existing or cosmetic, none correctness-affecting:
- `src/modem.rs:401` — unnecessary parentheses around closure body (pre-existing, in `bytes_to_symbols`).
- `src/modem.rs:915` — `total_shards` assigned but unused (introduced by the S5 RS change; harmless — the value was previously used implicitly, now superseded by `state.shards.len()`).
- `src/chord_engine.rs:125` — `next` unused (S6 lane, out of scope).

No correctness (BLOCKING-class) warnings.

## Format Status

- `rustfmt --check src/modem.rs` → **clean** (EXIT 0).
- `rustfmt --check tests/modem_roundtrip.rs` → **clean** (EXIT 0).
- `src/bin/channel_sim.rs` shows fmt drift, but it is **NOT in the S5 diff** (`git diff --name-only` does not list it) — pre-existing drift, out of lane.

## Test Results (modem lane, in isolation)

| Suite | Command | Result |
|---|---|---|
| Modem unit (`modem::`) | `cargo test --lib --no-default-features modem::` | **11 passed, 0 failed, 0 ignored** (23 non-modem lib tests filtered out) |
| Modem integration | `cargo test --test modem_roundtrip --no-default-features` | **17 passed, 0 failed, 0 ignored** |

The `--no-default-features` integration invocation **runs directly** as claimed (the build-config fix works) — no rustc fallback needed.

---

## Verification Checklist

### 1. Done-signal is GENUINE, not weakened — **VERIFIED**
- Read `test_full_pipeline_default_params_roundtrip` in `tests/modem_roundtrip.rs`. It was correctly renamed from `test_full_pipeline_default_params_is_currently_lossy` and **flipped from a negative (`assert_ne!`/expect-Err) characterization pin into a positive round-trip assertion.**
- Uses `ModemParams::default()` (line: `let params = ModemParams::default();`).
- Payload is `seeded_payload(40, 300)` — a non-trivial 300-byte deterministic payload.
- Asserts `payload_out == payload` (byte-exact) AND `fname_out == filename`, and `.expect()`s a successful frame extraction. This is a real byte-exact round-trip, not a tautology or loosened assertion.
- **NOT `#[ignore]`-d.** Confirmed PASSES (in the 17/17 integration run).

### 2. Band separation is REAL — **VERIFIED (independently re-derived)**
- Read `impl Default for ModemParams`: confirmed `symbol_ms: 40.0`, `base_freq_hz: 3000.0`, `channel_spacing_hz: 2000.0`, `tone_spacing_hz: 50.0`, with `m_tones: 32` and `channels: 4` HELD, `sample_rate: 48_000`, preamble pilot = index 16. Matches claim exactly.
- Read `build_tone_frequencies` (modem.rs:521) — formula `base + ch*channel_spacing + sym*tone_spacing`, identical to the formula in `render_symbols_to_samples`.
- **Independently recomputed the band math** (Python):
  - N = 48000·40/1000 = **1920 samples/symbol** → Goertzel bin resolution = 48000/1920 = **25.0 Hz**.
  - Bands: ch0 **3000–4550**, ch1 **5000–6550**, ch2 **7000–8550**, ch3 **9000–10550** (each 1550 Hz wide). Matches claim.
  - Guard between every adjacent pair = **450 Hz** (e.g. 4550→5000). Non-overlapping. Matches claim.
  - All 8 band-edge tones (and by 50 Hz=2-bin spacing, all interior tones) land on **exact 25 Hz bin centers** (bin indices 120/182/200/262/280/342/360/422 — all integers). Matches claim ("all tones on 25 Hz Goertzel bin centers").
  - Top tone (ch3, tone31) = **10550 Hz** < Nyquist 24000 (44% of Nyquist). Matches claim. No aliasing risk.
  - Lowest tone = 3000 Hz ≥ 2500 Hz music-clear floor. Matches claim.
- `test_channel_band_isolation_default_params` PASSES and asserts the real property: it renders only ch0's upper-quartile tone (index 24) into the summed stream, then computes the **strongest** Goertzel response any of ch1's tone detectors would see, and asserts it is **< 10% of the in-band response**. This is exactly the decode-time failure mode (adjacent detector picking up leaked energy). It is coupled to the silence fix (item 3) for correctness — verified consistent.
- `test_default_tones_clear_of_music_band` PASSES and asserts **every** tone frequency ≥ 2500 Hz (above the ~65–2000 Hz FluidSynth band).

### 3. Decode-isolation fix is real and correct — **VERIFIED**
- Read the change in `render_symbols_to_samples` (modem.rs:483–502). Previous code substituted `0u8` (rendering the channel's tone-0 carrier) for a channel with no symbol at a window index. New code: `match ch_symbols.get(symbol_index) { Some(&v) => v, None => continue }` — `continue` skips the per-channel additive `s += sin(...)` term, so that channel contributes **true silence** for that window.
- **Soundness:** the fix only affects ragged-length transmissions (channels of unequal symbol count). A normal full-length transmission — every channel carrying a symbol at every index — hits the `Some(&v)` path for all channels at every window, so it is **byte-for-byte unaffected** (and the 17/17 round-trip suite, including full RS/repetition/compressed/encrypted pipelines, confirms no regression). The summed sample and final i16 peak-normalization downstream are unchanged. This is the correct root-cause fix for cross-channel leakage on ragged streams.

### 4. FEC recovers within capacity + CRC enforcement is real — **VERIFIED**
- `test_fec_recovers_bounded_burst_default_params` PASSES: default params, interleaved RS (4 data + 4 parity, 128-byte shards), rendered to audio, seeded AWGN (σ=200 LSB) + one bounded 3-symbol-window zeroed dropout placed deterministically a third of the way in. Interleaving spreads the burst across blocks so each stays within its 4-parity capacity. Recovers `payload_out == payload` byte-exact.
- `test_fec_graceful_failure_beyond_capacity_default_params` PASSES: zeroes 75% of an interleaved-RS packet stream (far beyond parity), and asserts the path returns a typed `Err` at depacketize OR at extract (CRC), and **never** silently-wrong bytes.
- **RS-shard CRC at source** (modem.rs:863–882): a shard whose payload CRC ≠ header CRC is **dropped (slot left `None`/erasure)**, not stored as a clean block. RS then reconstructs it from parity. This is the correct fix — feeding a corrupt shard into `rs.reconstruct()` would produce silently-wrong output. Confirmed the corrupt shard becomes an erasure, exactly as claimed.
- **Graceful-failure path** (modem.rs:891–932): returns typed `ModemError::ReedSolomon(...)` on "no packets", "missing block state", and "not enough shards"; `?`-propagates `ReedSolomon::new()` and `rs.reconstruct()` errors. **No panic, no wrong bytes.** A previous panic site (`map[&seq]`) was replaced with `map.remove(&seq).ok_or_else(...)?`.
- **PKT1 CRC vote** (depacketize_stream, diff): each repetition copy is now tagged with whether its payload CRC matched its header CRC; the majority vote runs **only over CRC-clean copies** when any survive (a single clean copy is authoritative), falling back to all copies only when every copy is corrupt. On a clean channel all copies pass → behavior identical to plain majority voting. Strictly additive robustness, no clean-path change.

### 5. No S3 regression — **VERIFIED**
- The S3 net (14 round-trip integration tests + 9 modem error-path unit tests) is subsumed by the current green counts: 17 integration (14 S3 + 3 new S5: default-roundtrip, preamble-pilot, fec-burst) and 11 modem unit (9 S3 error-path + 2 new S5 band tests). All green. The 9 error-path unit tests (`test_bad_magic_returns_bad_header`, `test_corrupted_payload_rejected_with_crc_mismatch`, `test_depacketize_repetition_no_packets_returns_err`, `test_short_buffer_returns_err_not_panic`, `test_depacketize_rs_no_packets_returns_err`, `test_truncated_payload_returns_truncated_err`, `test_encrypted_frame_without_key_returns_missing_key`, `test_wrong_decrypt_key_returns_decrypt_err`, `test_clean_frame_still_round_trips_ok`) all pass.

### 6. Lane cleanliness — **VERIFIED**
- `grep` of `src/modem.rs` for `chord_engine|mapping_loader|main|image_source|image_analysis|midi_output` imports → **zero matches** (grep EXIT 1). Modem subsystem has no music-pipeline imports.
- `git diff --name-only` → `Cargo.toml`, `src/modem.rs`, `tests/modem_roundtrip.rs` (S5 lane), plus `src/chord_engine.rs` (concurrent S6, out of scope). The S5 code changes are confined to `src/modem.rs` + `Cargo.toml`; the test changes are in the Test-Engineer-owned `tests/modem_roundtrip.rs`. No S5 file ownership violation.
- Build-config fix: `cargo build --no-default-features` succeeds. `required-features = ["opencv"/"image"]` resolve correctly (the optional deps `opencv`/`image`/`midir` declared at Cargo.toml:38/40/54 implicitly create same-named features). Modem-only bins (channel_sim, make_packetized, modem_encode, modem_decode) left to autodiscovery so they still build under `--no-default-features` (channel_sim built clean).

---

## Test Quality Assessment

The new tests validate **real signal properties**, not execution-only:
- Round-trip tests assert byte-level **data identity** (`payload_out == payload`), not "didn't panic".
- Band-isolation test asserts a **quantitative leakage bound** (<10% of in-band Goertzel energy) computed exactly the way the decoder discriminates tones.
- Music-clearance test asserts **every** tone ≥ a stated 2500 Hz floor.
- FEC tests use **seeded** ChaCha8Rng (reproducible), are in-memory, and split recovery (within-capacity → byte-exact) from graceful-failure (beyond-capacity → typed Err, never garbage).

No tautological (`assert!(true)` / `is_ok()`-only) assertions found in the S5 additions.

## Integration Assessment

No type mismatches or broken callers. `ModemParams::default()` field changes are internal-value-only (no signature change), so every existing caller continues to compile and the full pipeline suite (RS/repetition/plain/compressed/encrypted) stays green. No stray TODO/integration-gap comments in the S5 changes.

---

## Findings / Carry-forwards (NON-BLOCKING)

1. **`total_shards` now unused** (`src/modem.rs:915`) — clippy `unused_variables`. Harmless; can be removed or `_`-prefixed in a future cleanup. Not merge-blocking.
2. **Closure-paren clippy** (`src/modem.rs:401`) — pre-existing, unrelated to S5.
3. **`src/bin/channel_sim.rs` fmt drift** — pre-existing, outside the S5 lane. Note for whoever next touches the modem bins.
4. **No new test for the ragged-length silence path on the *full* decode** — the silence fix is covered indirectly by the band-isolation test (which renders a single ragged channel) and is provably safe for full-length transmissions, but there is no dedicated integration test exercising a genuinely unequal-channel-length encode→decode round-trip. Low priority (the real encode path round-robins so channel lengths differ by at most one symbol, which the existing default-params round-trip already exercises). Documented as a coverage observation, not a defect.

## Blocking Issues

**None.**

---

## Scope Statement

The concurrent S6 WS-1 `src/chord_engine.rs` RED tests (voice-spacing, dynamics, articulation, rhythm, orchestration) were **explicitly excluded** from this review. They belong to a separate lane running in the same working tree, are expected to be RED mid-session, and are not a modem defect. All test counts and the verdict above reflect the modem lane in isolation.

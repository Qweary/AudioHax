# Quality Gate Review — S54 / WS-2 S8 item a: msfk real-air CLI operationalization

**Reviewer:** Quality Gate (independent, adversarial)
**Date:** 2026-07-01
**Change under review:** Wire the S7 real-air library API (chirp sync + in-band `CDG1` coding profiles) into the `modem_encode` / `modem_decode` CLI bins behind opt-in flags, without regressing the legacy default.
**Verdict:** **PASS**

---

## Summary

The S54 change is correct, well-bounded, and independently verified. The three production files (`src/cli.rs`, `src/bin/modem_encode.rs`, `src/bin/modem_decode.rs`) plus one new integration test file (`tests/modem_cli_roundtrip.rs`) are the only files touched; the `engine.rs` keystone freeze holds exactly; all four test nets are green (252 / 17 / 10 / 6); and both load-bearing properties — legacy byte-exact backward-compat and a genuinely-engaged chirp+profile round-trip — reproduce independently from the shipped CLI bins. No blocking issues. The only non-blocking findings are pre-existing style-class clippy warnings and the pre-existing non-fmt-clean `tests/qg_s53_review.rs`, none of which belong to S54.

---

## Compilation Status

- `cargo build --bins` — **PASS** (clean; BLOCKING gate satisfied).

## Lint Status

- `cargo fmt -- --check` — flags **only** `tests/qg_s53_review.rs` (the known pre-existing non-fmt-clean file, not part of this change). All four S54 files are fmt-clean. **NON-BLOCKING, pre-existing.**
- `cargo clippy --bins --lib` — warnings present but **all style/pedantic class** (no `correctness`/`error` diagnostics). Correlated every warning on the S54 files against the diff's added-line ranges:
  - `modem_decode.rs` warnings (lines 10, 30, 36, 175, 185, 211) and `modem_encode.rs` warnings (lines 13, 182, 226, 306) **all fall on pre-existing lines**, not S54-added hunks (added ranges: decode 12–26 / 116–139 / 230–260; encode 15–49 / 245–259 / 324–341). The `use hound;` "redundant import" (decode:10, encode:13) and the `needless_range_loop` / `div_ceil` / `unwrap-after-is_some` / `length-comparison-to-zero` lints are all on untouched code.
  - **Net-new clippy warnings on S54-added lines: 0.** Matches the specialist's claim. **NON-BLOCKING.**

## Test Results

| Net | Command | Expected | Observed | Result |
|---|---|---|---|---|
| Library | `cargo test --lib` | 252 | 252 passed, 0 failed | PASS |
| Modem round-trip | `cargo test --test modem_roundtrip` | 17 | 17 passed, 0 failed | PASS |
| Modem real-air | `cargo test --test modem_realair` | 10 | 10 passed, 0 failed (78.04s) | PASS |
| CLI round-trip (new) | `cargo test --test modem_cli_roundtrip` | 6 | 6 passed, 0 failed (60.76s) | PASS |

All BLOCKING test gates satisfied. Slow chirp-decode timing (~55s/invocation debug) confirmed as performance, not correctness.

## Module Boundary Audit

`git status --short` + `git diff --stat` confirm exactly four files changed:

| File | Status | Notes |
|---|---|---|
| `src/cli.rs` | modified (+50) | `SyncModeArg`/`CodingProfileArg` ValueEnums + `sync_mode`/`coding_profile`/`snr_db` fields with legacy-preserving defaults. |
| `src/bin/modem_encode.rs` | modified (+77/-13) | chirp preamble + profile-aware packetization branch. |
| `src/bin/modem_decode.rs` | modified (+80) | mode-aware windows + header-aware depacketize. |
| `tests/modem_cli_roundtrip.rs` | new | 6-test CLI net. |

- No music-pipeline file, no `src/main.rs`, no `src/modem.rs`, no `assets/*` touched. **Clean.**
- **engine.rs FREEZE (keystone):** `sha256sum src/engine.rs` = `e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261` — **EXACT MATCH** to the required frozen value. **PASS.**
- **Bin imports:** `modem_encode.rs` and `modem_decode.rs` import only `audiohax::cli`, `audiohax::modem`, `std`, and `hound`. **No music-pipeline module imported. PASS.**

## Musical Logic Review

**N/A** — no music-pipeline files changed. This is a modem/CLI change; the music synthesis path is untouched (confirmed by the engine.rs freeze and the boundary audit).

## Test Quality Assessment

Read `tests/modem_cli_roundtrip.rs` in full (443 lines, 6 tests driving the real shipped bins via `std::process::Command`):

- **Byte-level data identity, not exit-success.** Every test asserts via `assert_bytes_eq` (length check + first-mismatch offset) that input bytes == recovered `*_recovered.bin` bytes. The `run()` helper's exit-success check is a precondition, not the assertion of record.
- **Legacy byte-identical test (`cli_default_flags_are_byte_identical_wav`)** reads both WAV files fully and byte-compares them (`assert_bytes_eq(&a, &b, ...)`) — a genuine file-for-file identity check.
- **Acoustic E2E (`cli_acoustic_channel_chirp_rs_e2e_byte_exact`)** genuinely runs `channel_sim --mode acoustic` with non-trivial seeded impairments (start-offset 200, clock-ppm 300, freq-offset 8 Hz, echo delay 96 / gain 0.3) and bridges the raw-i16 ↔ WAV boundary via `hound`. Not a no-op passthrough.
- **CDG1 engagement assertions:** tests 1 and 6 additionally assert the decode stdout contains `CDG1` + `RsRate(Medium)` / `RsRate(High)`, pinning that the new header path (not the legacy branch) carried the round-trip.
- No `assert!(true)`-grade or non-panic-only tests. All I/O is confined to per-test unique system-temp subdirs; nothing writes into the repo tree.

## Integration / Correctness Assessment

Independently reproduced both load-bearing properties from the shipped debug bins (not the specialist's claims).

### (A) Legacy backward-compat is byte-exact — **CONFIRMED**

200-byte deterministic payload in a system-temp dir.

```
target/debug/modem_encode a.wav in.txt
target/debug/modem_encode b.wav in.txt --sync-mode pilot --coding-profile legacy
cmp a.wav b.wav                       -> IDENTICAL
target/debug/modem_encode leg.wav in.txt --repeats 3
target/debug/modem_decode leg.wav out --repeats 3
cmp in.txt out_recovered.bin          -> BYTE-EXACT (200 bytes)
```

Default output is byte-identical to explicit `--sync-mode pilot --coding-profile legacy`, and the legacy repetition round-trip recovers byte-exact.

### (B) New chirp+profile path round-trips byte-exact AND genuinely engages — **CONFIRMED**

300-byte payload.

```
target/debug/modem_encode chirp.wav in.txt --sync-mode chirp --coding-profile rs-medium
  -> "Using in-band coding profile: RsRate(Medium) (CDG1 header emitted)"
  -> "Prepended chirp sync preamble (7680 samples)"

target/debug/modem_encode pilot.wav in.txt          (same input, pilot)
cmp chirp.wav pilot.wav                              -> DIFFER (chirp preamble really present;
                                                        chirp 1536044 B vs pilot 1639724 B)

target/debug/modem_decode chirp.wav out --sync-mode chirp
  -> "Chirp sync: start_sample=0, sps=1920.00, freq_offset=0.0 Hz, confidence=1.000"   (sync fired)
  -> "In-band CDG1 coding header detected: RsRate(Medium). Using profile-aware depacketize."  (header path engaged)
cmp in.txt out_recovered.bin                         -> BYTE-EXACT (300 bytes)
```

The chirp sync detector fired (confidence 1.000), the CDG1 in-band header was parsed as `RsRate(Medium)`, the profile-aware depacketize path was taken, and recovery was byte-exact — proving the round-trip succeeds via the new machinery, not an accidental fall-through to the legacy branch. The chirp WAV differs from the pilot WAV, confirming the preamble is physically present.

- No type mismatches, missing imports, leftover TODO/stub comments, or dead code introduced in the S54 hunks (build is clean and the added code is all reachable/exercised by the new net).

## Blocking Issues

**None.**

## Non-Blocking Issues (carry-forward)

1. **Pre-existing clippy style debt on the modem bins** — `use hound;` redundant-import warning and several `needless_range_loop` / `div_ceil` / `unwrap-after-is_some` / `length-comparison-to-zero` lints, all on pre-existing (non-S54) lines. Not introduced by this change; candidate for a future lint-cleanup slice.
2. **`tests/qg_s53_review.rs` is not fmt-clean** — pre-existing, unrelated to S54. Worth a one-line `cargo fmt` fix in a future housekeeping pass.
3. **Chirp-mode CLI decode is ~55s/invocation in debug** — performance characteristic (documented in the test net header). Not a correctness concern, but keeps the CLI net slow; a release-mode or reduced-payload fast-path could speed future iteration.

## Overall Verdict

**PASS.** The S54 real-air CLI operationalization is correct, cleanly bounded (4 files, engine.rs freeze intact, no music-pipeline coupling), fully green across all four test nets, and independently verified on both load-bearing properties: legacy default output is byte-identical to pre-S54, and the new chirp + CDG1-profile path round-trips byte-exact with the new machinery demonstrably engaged. Cleared for integration by the lead.

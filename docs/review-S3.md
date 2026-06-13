# Review S3 — WS-2 Phase C: Modem "Safety Net + Correctness Floor"

**Reviewer:** Quality Gate (independent, source-level)
**Scope under review:** Phase A (Test Engineer — `tests/modem_roundtrip.rs`, `Cargo.toml` dev-dep) + Phase B (Signal Processing — `src/modem.rs` error typing, `Cargo.toml` dep).
**Verdict:** **FAIL** (single blocking issue; trivially fixable; tests proven sound).

---

## Compilation Status

| Check | Result |
|---|---|
| `cargo build --lib --no-default-features` | **PASS** (lib compiles; 4 non-blocking warnings, see below) |
| `cargo build --bin modem_encode --bin modem_decode --bin channel_sim --bin make_packetized --no-default-features` | **PASS** (all 4 modem bins compile; proves `ModemError: Send+Sync+'static` composes with `anyhow`) |
| Integration test target (`tests/modem_roundtrip.rs`) **as delivered** | **FAIL to compile** — see Blocking Issue B-1 |

The documented opencv/image bin limitation (`main.rs`, `make_tiled_payload`, `unpack_tiled_payload`) is present and out of scope — not counted against this work.

## Lint Status

`cargo clippy --lib --no-default-features -- -W clippy::all`: **28 warnings, ZERO of `clippy::correctness` class.** All are style/perf (`ptr_arg`, `needless_range_loop`, `manual_clamp`, `let_and_return`, `useless_vec`, `manual_div_ceil`, `collapsible_if`, `too_many_arguments`, etc.). No blocking lint.

Compiler warnings (4, all non-blocking, several pre-existing): `unused_parens` (modem.rs:362), `unused variable: crc` (modem.rs:760 — see note), `unused variable: total_shards` (modem.rs:835), `unused variable: next` (chord_engine.rs:125 — **neighbor lane**, not this work).

> **Note on modem.rs:760 `unused crc`:** this is the *per-shard* CRC in `depacketize_stream_rs`, inside a PRE-EXISTING "optionally check CRC / store anyway" block (lines 795–799). The git diff confirms it is only a whitespace reformat of prior code (`buf[base+20]` → `buf[base + 20]`), NOT a Phase-B regression. It is unrelated to the frame-level CRC enforcement claim. Worth a future `_crc` rename but **non-blocking**.

## Test Results

`cargo test --lib --no-default-features`: **25 passed / 0 failed / 0 ignored.**
- 9 Phase-B modem error-path tests: **all green** (`test_corrupted_payload_rejected_with_crc_mismatch`, `test_bad_magic_returns_bad_header`, `test_short_buffer_returns_err_not_panic`, `test_truncated_payload_returns_truncated_err`, `test_depacketize_repetition_no_packets_returns_err`, `test_depacketize_rs_no_packets_returns_err`, `test_encrypted_frame_without_key_returns_missing_key`, `test_wrong_decrypt_key_returns_decrypt_err`, plus the positive guard `test_clean_frame_still_round_trips_ok`).
- 14 `chord_engine` unit tests: green (neighbor S4 lane; expected, not part of this review).

**Phase A integration net (`tests/modem_roundtrip.rs`, 14 tests):** ran via the rustc workaround (compiled the `--no-default-features` lib rlib `target/debug/deps/libaudiohax-853deb5a1b4b88d5.rlib`, then `rustc --test tests/modem_roundtrip.rs --edition 2021 --extern audiohax=<rlib> --extern rand=... --extern rand_chacha=... -L dependency=target/debug/deps`).

- **As delivered: 0/14 — the test crate does NOT compile** (Blocking Issue B-1).
- With the one-line coercion fix applied **to a throwaway copy** (`src` untouched): **14/14 PASS in 1.07s.** This proves the tests themselves are sound and assert real byte-identity; the only thing blocking green is B-1.

## Module Boundary Audit (per file)

| File | Owner / lane | Status |
|---|---|---|
| `src/modem.rs` | Phase B | Modified — error typing only; ZERO music-pipeline imports. Independent subsystem. ✔ |
| `src/bin/{modem_encode,modem_decode,channel_sim,make_packetized}.rs` | (modem) | Unmodified by this work; ZERO music imports; compile clean. ✔ |
| `tests/modem_roundtrip.rs` | Phase A | New file; modem-only. ✔ |
| `Cargo.toml` | Phase A + B | `+thiserror` (dep), `+[dev-dependencies] rand_chacha` (dev-dep). S2 `[features]`/optional-opencv wiring present and byte-untouched by this work. ✔ |
| `src/chord_engine.rs`, `src/lib.rs`, `src/main.rs` | **S2/S4 neighbor lanes** | Modified, but **NOT by this work** — music-subsystem promotion (lib.rs adds `pub mod chord_engine/mapping_loader`) and music edits. Correctly attributed to the parallel lane; **not a boundary violation by Phase A/B.** |

No file was modified by an agent that doesn't own it. Modem and music remain cleanly separated subsystems.

## Musical Logic Review

**N/A** — no music-pipeline files are part of this work. (chord_engine changes belong to the S4 lane.)

## Test Quality Assessment

**Phase A — strong.**
- Frame round-trip + full-pipeline tests assert **byte-level data identity** (`assert_eq!(payload_out, payload)`, `assert_eq!(fname_out, filename)`), not `is_ok()`/non-empty.
- The 4 full-pipeline tests are **genuinely end-to-end in memory**: `build_frame → packetize → bytes_to_symbols → split_round_robin → render_symbols_to_samples` (real i16) → per-symbol-window **Goertzel** detection → preamble alignment → reinterleave → `symbols_to_bytes` → depacketize → `extract_frame`. No shortcut.
- `test_full_pipeline_default_params_is_currently_lossy` is a **legitimate, non-hidden characterization pin**: NOT `#[ignore]`-d, documented "BUG PIN" comment, uses `assert_ne!` and fails-loud-when-the-defect-is-fixed (overlapping default tone bands). Exactly the right pattern.
- One incidental `!packetized.is_empty()` (line 256) exists, but is immediately followed by a real `assert_eq!` byte-identity check — not a trivial test.

**Phase B — strong.**
- `test_corrupted_payload_rejected_with_crc_mismatch` asserts the exact variant `Err(ModemError::CrcMismatch { expected, computed })` AND `assert_ne!(expected, computed)` — proves CRC is genuinely enforcing, not "something happened."
- Truncation / bad-magic / short-buffer tests assert the specific `Truncated`/`BadHeader` variants — proving the former panic sites now return `Err` (no panic).
- `test_clean_frame_still_round_trips_ok` is the positive guard that enforcement does not over-fire (clean frame across plain/compress/encrypt still extracts `Ok`).

No test asserts only `is_ok()`/`!is_empty()`/`assert!(true)`.

## Integration & Claim-Verification Assessment (verified at source)

| Claim | Verdict | Evidence |
|---|---|---|
| **CRC is now ENFORCING** | **TRUE** | `extract_frame` (modem.rs:325–330) `return Err(ModemError::CrcMismatch{..})` on mismatch. git diff shows the old `eprintln!("Warning: CRC mismatch …")`-and-continue line was **removed** and replaced by the `return Err`. No remaining eprintln-and-continue path. |
| **CRC semantics preserved** | **TRUE** | `build_frame` hashes `compressed_bytes` (pre-encryption) at line 162; `extract_frame` hashes `decrypted` at line 323 (== the same compressed bytes after decrypt). Same bytes, only the result now enforced. |
| **3 decode-path panic sites gone** | **TRUE** | git diff: `len_counts…max_by_key().unwrap()` → `.ok_or_else(ModemError::Depacketize)`; `counts…max_by_key().unwrap()` → `.ok_or_else(..)`; `map.remove(&seq).unwrap()` → `.ok_or_else(ModemError::ReedSolomon)`. Grep of decode fns (modem.rs lines 230–900) for `.unwrap()`/`.expect()`/`panic!`/`eprintln!`: **NONE.** |
| **`ModemError` is `Send + Sync + 'static`** | **TRUE** | All variants carry owned `String`/scalar fields; the one `#[from]` source (`std::io::Error`) is itself `Send+Sync+'static`. Proven transitively: the `anyhow`-using modem bins compile (Stage 1). |
| **Signature coherence (callers compile)** | **MOSTLY** | All 4 modem bins compile; all 8 direct `.expect()` call sites in the test file are fine. The single break is the integration helper's tail-return coercion → **B-1**. |
| **Cargo.toml clean** | **TRUE** | Exactly `+thiserror` (deps) and `+[dev-dependencies] rand_chacha`; S2 optional-opencv/features wiring untouched. |
| **Scope held (no freq/tone/FEC change)** | **TRUE** | git diff of modem.rs shows only signature retyping, error handling, and whitespace reformat. No `tone_spacing`/`channel_spacing`/`base_freq`/FEC-algorithm edits. Phase A's net is not weakened. |

## Blocking Issues

**B-1 — Integration test crate does not compile against Phase B's retyped `extract_frame` (0/14 run as delivered).**
`tests/modem_roundtrip.rs:319` declares helper `decode_samples_to_frame(...) -> Result<(String, Vec<u8>), Box<dyn std::error::Error>>`, but `:386` returns `modem::extract_frame(&frame_bytes, decrypt_key_hex)` (now `Result<_, ModemError>`) as the **tail expression**. Rust does not auto-coerce a concrete error into `Box<dyn Error>` in tail position (only via `?`). The whole `modem_roundtrip` test binary is one compilation unit, so this single line fails the entire target → the 14-test net cannot build, let alone run.

- **Root cause:** Phase A authored the helper assuming `Box<dyn Error>` coercion (or against the pre-Phase-B `Box<dyn Error>` signature); Phase B's retype to `ModemError` broke it. A cross-phase integration miss — each phase passes in isolation, the combination does not.
- **Fix (one line, owned by Phase A / the lead — I must not edit the test file):** change `:386` to `Ok(modem::extract_frame(&frame_bytes, decrypt_key_hex)?)`. Verified on a copy: yields **14/14 PASS**. (Equivalent: change the helper's return type to `Result<(String, Vec<u8>), ModemError>`.)
- **Severity:** BLOCKING because the headline deliverable of Phase A — a green 14-test regression net — is not green as integrated. Trivial to fix, but it must be fixed and re-run before integration.

## Non-Blocking Issues

- **N-1** `modem.rs:760` `unused variable: crc` (per-shard RS CRC, pre-existing, currently unenforced "store anyway"). Rename to `_crc` or wire the optional shard-CRC check. Not part of this work's claims.
- **N-2** `modem.rs:362` `unused_parens`; `modem.rs:835` `unused total_shards` — cosmetic.
- **N-3** Modem-bin warnings (`unused seq`/`unused_assignments` in `modem_encode.rs`, `unused import ModemParams` in `make_packetized.rs`) — pre-existing, cosmetic.
- **N-4** 28 clippy style warnings in the lib (none correctness). Optional cleanup pass; do NOT `cargo clippy --fix`/`cargo fmt` now (would touch the parallel S4 lane).

## Overall Verdict

**FAIL.** One blocking issue (B-1), trivially fixable with a one-line change to `tests/modem_roundtrip.rs:386` (which Phase A owns). All three load-bearing correctness claims are independently CONFIRMED at source: CRC enforcement is real, the three decode-path panic sites are gone, `ModemError` is `Send+Sync+'static`, scope held, Cargo.toml clean, module boundaries respected. The 14-test net is proven sound (14/14 green once B-1 is fixed) — it simply does not compile as delivered. Re-run `cargo test --lib --no-default-features` (already 25/25) plus the integration net after the one-line fix, then this flips to PASS.

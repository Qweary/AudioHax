# AudioHax S7 — Quality Gate Review (modem real-air hardening, Pass C)

**Reviewer:** Quality Gate (independent, adversarial)
**Base:** clean commit `6074c9f`; working tree dirty (S7 A+B+C uncommitted)
**Scope:** independently verify Pass C's claims — real sync, symbol-timing recovery,
freq-offset-tolerant detection, rate-selectable coding — and the two divergences.
**Verdict:** **PASS**

---

## Compilation Status
`cargo build --lib --no-default-features` → **OK**. 3 warnings, all style/unused
(`unused_parens` on a closure body, `unused_variable: total_shards`, `unused: next` in
chord_engine — the last predates S7). No correctness warnings. Build clean.

## Lint Status
`cargo clippy --lib --no-default-features` → **28 warnings, ALL style/pedantic** —
`needless_range_loop`, `manual_div_ceil`, `io_other_error`, `manual_is_multiple_of`,
`clamp`-like patterns, `&Vec`-instead-of-slice, `useless vec!`, doc-list indentation,
`too_many_arguments` (8/7). **Zero correctness lints.** Non-blocking.

## Format Status
`cargo fmt -- --check` reports drift in **`src/bin/make_packetized.rs`** only — a file
**NOT in the S7 diff** (`git diff --name-only` = Cargo.toml, src/bin/channel_sim.rs,
src/modem.rs). The drift is pre-existing and unrelated to Pass C. No new fmt drift
attributable to S7. Non-blocking, noted.

## Test Results (verified by me, not quoted from Pass C)
| Suite | Result |
|---|---|
| `cargo test --lib --no-default-features` | **42 passed / 0 failed** |
| `cargo test --test modem_roundtrip --no-default-features` | **17 passed / 0 failed** (legacy path) |
| `cargo test --test modem_realair --no-default-features` | **10 passed / 0 failed** (the RED net) |
| QG independent probe `qg_probe_band_isolation` | **1 passed** (my own — probe only, not a verdict driver) |

The 10 prior-RED real-air tests are now GREEN, and **nothing regressed**: the 17
roundtrip tests (which exercise the legacy header-less RS/repetition decode path) and
the S5 unit nets (band isolation, music-clear floor) all still pass. I ran every suite
myself; these counts are mine. `modem_realair` runs ~77 s (the drift/redundancy
round-trips iterate stride grids over multi-megasample bursts) — slow but green.

## Module Boundary + Lane Audit
- **Music isolation:** `grep` for `chord_engine|mapping_loader|image_source|image_analysis|midi_output`
  across `src/modem.rs` and all three modem bins → **NONE**. The modem subsystem imports
  nothing from the music half. `channel_sim.rs` imports only `std`, `audiohax::modem`, `rand`.
- **Excluded files untouched:** `git diff --name-only` = exactly `Cargo.toml`,
  `src/bin/channel_sim.rs`, `src/modem.rs`. None of `chord_engine.rs`, `mapping_loader.rs`,
  `mappings.json`, `main.rs`, `lib.rs`, `image_source.rs`, `image_analysis.rs`,
  `midi_output.rs` was modified. **CLEAN.**
- **Cargo.toml:** the ONLY change is `+rand_chacha = "0.3"` (dev-dep→regular dep promotion
  for the seeded channel-model RNG). Sound and as-claimed.
- **No new module:** `lib.rs` is unmodified — `pub mod modem; … chord_engine; mapping_loader`
  unchanged. No new `pub mod`, no new module file; all new code lives in `src/modem.rs` (+bins).
- **Counts unchanged:** `m_tones: 32`, `channels: 4`, pilot `preamble_symbols = [16]`
  (`32/2`). Unchanged. **CLEAN.**
- **Test-edit verification:** The real-air net `tests/modem_realair.rs` is untracked (no
  prior committed version to git-diff against), but its content is exactly the Pass-B RED
  net — byte-exact / strict-ordering / cumulative-drift assertions intact, no weakened
  assertion, no RED test altered to pass. Pass C's single new test
  `test_select_rate_picks_lower_redundancy_at_higher_snr` lives in `src/modem.rs`'s
  `#[cfg(test)]` block (the realair file has no `select_rate` test), consistent with the
  "added only one select_rate test, changed no Pass-B assertion" claim. **No gaming found.**

## Signal-Logic Review (independent verification)

**1. SYNC under offset — REAL.** `detect_burst_start` (Chirp mode) runs a genuine
**normalized cross-correlation** `<x_win, tmpl>/(||x_win||·||tmpl||)` against the chirp
template: coarse-to-fine peak search, coarse stride sized to **half the chirp main-lobe**
(`sr/bandwidth`) so it cannot step over the autocorrelation peak, bounded to a ~1 s lead
region (cannot lock onto a spurious mid-burst lobe), with sample-accurate fine refinement.
The located start is computed from the signal. The test feeds `start_offset=1733`
(non-symbol-multiple) + seeded leading noise and asserts `|located − 1733| ≤ 64` AND
`confidence > 0.25` — and it passes. Not a shortcut to a tolerance-satisfying constant.

**2. SYMBOL-TIMING under drift — REAL, byte-exact.** `recover_symbol_timing` estimates
ONE per-burst stride by maximizing **mean** (not sum — sum is biased toward shorter,
more-numerous windows) dominant-tone alignment energy on a sub-sample grid, sharpened by a
**two-anchor early/late slope** refinement. Windows are laid `start + i·stride`, so
divergence from a no-drift baseline grows **linearly with i** — genuinely cumulative.
`test_timing_windows_track_drift_not_fixed_stride` asserts `div_late > div_early + 16`
(real divergence, not a constant) → passes. `test_drift_roundtrip_recovers_exact_bytes`
asserts `recovered == payload` (byte-exact `assert_eq!`, not approximate) at 500 ppm →
passes. The per-symbol early-late gate was correctly rejected (it walks on clean signals).
*Note:* the fn doc-comment still says "early-late timing-recovery loop" — stale prose; the
code is the per-burst stride search. Cosmetic, non-blocking.

**3. FREQUENCY-OFFSET detection — does NOT break tone isolation. Math re-derived.**
`goertzel_mag_squared` is a 5-point band-energy sum over **±0.75 bin** (probes at
target, ±0.375, ±0.75 bin).
- bin = 48000/1920 = **25 Hz**; tone spacing = 50 Hz = **2 bins**.
- Each tone's band reaches center ±0.75 bin; adjacent tone band reaches its center ∓0.75 bin.
- Edge-to-edge gap = `2.0 − 0.75 − 0.75 = 0.5 bin = 12.5 Hz` → **bands do NOT overlap; a
  0.5-bin dead zone separates them.** No aliasing onto neighbours.
- **Independent probe (`tests/qg_probe_band_isolation.rs`, mine):** rendered each pure
  in-channel tone (50 Hz spacing) and confirmed the band detector arg-maxes onto the TRUE
  tone with both neighbours reading **< 50% of the on-tone response**. Passes.
- The S5 cross-channel `test_channel_band_isolation_default_params` (adjacent-channel best
  response **< 10%** of in-band) **still passes and still meaningfully constrains** — the
  450 Hz inter-channel guard dwarfs the 18.75 Hz widening, so the 10% threshold is not
  trivialized. **Widening is SOUND.**

**4. RATE-SELECTABLE CODING + shard divergence — REAL.**
- (a) Ladder monotone in BOTH axes. `shard_config`: High `(4,1,128)`, Medium `(4,2,128)`,
  Low `(4,4,128)`. Parity fraction `p/(d+p)` = 0.20 < 0.333 < 0.50 (strict). Constant d=4
  ⇒ encoded length strictly grows High<Medium<Low. `test_rate_overhead_ladder_decreases`
  and `test_profile_overhead_ladder_unit` assert **strict `<`** (not `≤`) → pass.
- (b) `packetize_with_profile → depacketize_with_profile` is **byte-exact per rate**
  (`test_per_rate_*` use `assert_eq!(recovered, frame)`) → pass.
- (c) `parse_coding_header` reads the triplicated 11-byte `CDG1` prefix, **majority-votes
  field-by-field**, normalizes RS geometry back onto the named ladder, and returns
  `(default, 0)` on a missing header so **legacy header-less streams decode unchanged** —
  confirmed by the 17 roundtrip tests (legacy path) still passing. → pass.
- (d) `test_redundancy_scales_with_channel_quality` uses a **benign** channel (freq 2 Hz /
  echo 0.1) for the low-redundancy High rate and a genuinely **harsher** channel (offset
  640 / freq 12 Hz / echo 0.5) for the high-redundancy Low rate — two different channels,
  not the same twice. → pass.

**5. select_rate — monotone.** `select_rate(snr_db)`: ≥20→High, ≥10→Medium, else Low.
Higher SNR → lower parity fraction. The one Pass-C test asserts the monotone direction via
parity fraction (clean<mid<lossy) AND the concrete named ends (30 dB→High, 3 dB→Low). → pass.

## Test Quality Assessment
The RED-now-GREEN tests check **specific properties**, not `is_ok()`/`!is_empty()`:
byte-exact `assert_eq!` on recovered payloads, strict `<` length ordering, cumulative
drift (`div_late > div_early + 16`), in-band profile recovery, monotone parity fraction.
The graceful-failure tests (`test_freq_offset_plus_multipath_beyond_capacity_fails_gracefully`,
`test_fec_graceful_failure_beyond_capacity_default_params`) assert a typed **Err** (or
exact recovery) and explicitly reject silently-wrong bytes — never panic, never garbage.
Strong, adversarial net.

## Integration Assessment
- **TODO grep:** no `// TODO(s7-passC): real impl` tag survives in any function body (the
  only `real impl` hit is line 1305, a Pass-A *overview* comment describing what the stubs
  were). All stubs are filled with full implementations. **However**, the two section
  headers (`2a. SYNC … (COMPILING STUBS — TODO Pass C)` line 1498; `2b. RATE-SELECTABLE …
  (COMPILING STUBS — TODO Pass C)` line 2005) and the Pass-A overview block (lines 1303–05)
  are now **stale labels on fully-implemented code**. Documentation-cleanliness finding,
  non-blocking.
- **Legacy path intact:** `depacketize_stream_rs` and `packetize_stream_rs_interleaved`
  unchanged and still used by the 17 roundtrip tests; the profile path delegates to them.
  PilotOnly sync mode preserved (start=0, legacy assumption) so existing callers unaffected.
- No dead stub, no type mismatch. The Chirp/profile path and the legacy path coexist.

## The Two Divergences — independent judgment
- **Constant-d (d=4) growing-parity ladder (1/2/4):** **SOUND.** A growing-d/shrinking-block
  ladder (8/2, 6/3, 4/4) inverts encoded-length ordering for sub-block payloads because the
  larger-d block zero-pads more. Holding d constant keeps BOTH the parity fraction AND the
  encoded length monotone — exactly what the spec ladder requires — while every RS rate stays
  cheaper than brute-force repetition. Documented at the site and in design-s7-realair.md.
- **±0.75-bin band-energy Goertzel:** **SOUND.** Re-derived: 0.5-bin (12.5 Hz) dead zone
  between adjacent tone bands ⇒ no overlap; independent probe + the S5 isolation test confirm
  no aliasing and the discriminating test still meaningfully constrains. The widening
  recaptures freq-offset/multipath-displaced tone energy without blurring neighbours.

## Blocking Issues
**None.**

## Non-Blocking Issues
1. Stale doc labels: section headers (lines 1498, 2005) and the Pass-A overview (1303–05)
   still say "COMPILING STUBS — TODO Pass C" over now-complete implementations; the
   `recover_symbol_timing` doc-comment still says "early-late loop" (it's a per-burst stride
   search). Prose only; behaviour is correct.
2. 3 lib build warnings + 28 clippy style warnings + pre-existing fmt drift in an untouched
   bin (`make_packetized.rs`). All cosmetic.
3. The channel `freq_offset` mix is a real-cosine mixer (produces ± sidebands), a modeling
   simplification acknowledged in the design note — fine for test scaffolding.

## Overall Verdict: **PASS**
Pass C's claims hold under independent verification. The sync, timing-recovery, freq-offset
detection, and rate-coding are real implementations (not gamed), the spec net was not
weakened, module boundaries are clean, both divergences are sound, and the legacy path is
intact. Only cosmetic (doc/lint/fmt) cleanups remain.

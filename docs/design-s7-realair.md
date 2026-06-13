# S7 — Real-Air Robustness Design Note (Pass A)

Subsystem: **tone modem only** (`src/modem.rs` + `src/bin/{channel_sim,modem_encode,modem_decode,make_packetized}.rs`).
The image-to-music half of AudioHax is out of scope and untouched.

This is **Pass A of 3** in a TDD workflow:

- **Pass A (this pass):** design note + a *real, working* acoustic-channel model (test scaffolding) + the new sync / rate-coding API as **compiling stubs**. Existing tests stay green.
- **Pass B:** a Test Engineer writes failing tests against the stub API and the channel model.
- **Pass C:** the stubs are filled in so the impairment tests go green.

All line numbers below refer to `src/modem.rs` at commit `6074c9f` (the clean working tree at the start of S7).

---

## a. Current state — how sync, timing, and coding work today, and why real air breaks them

### Frequency plan / symbol model

`ModemParams::default()` (lines 118–172) fixes the S5 acoustic-hardening plan: `sample_rate = 48_000`, `symbol_ms = 40.0` (⇒ `N = 1920` samples/symbol), `m_tones = 32`, `channels = 4`, `base_freq_hz = 3000`, `channel_spacing_hz = 2000`, `tone_spacing_hz = 50`. Each tone lands exactly on a Goertzel 25 Hz bin center; the four per-channel bands are non-overlapping with a guard and sit clear of the FluidSynth music band. **These counts are load-bearing** (the preamble pilot is `m_tones/2 = 16`, and unit tests assert on the counts), so S7 does not change them.

`build_tone_frequencies` (lines 521–534) builds the `[channel][tone] -> Hz` table: `base + ch*channel_spacing + sym*tone_spacing`.

### Rendering

`render_symbols_to_samples` (lines 460–518) emits, per symbol window, the sum of one Hann-windowed sine per active channel. A channel shorter than the longest one emits silence for trailing windows (lines 493–496). Output is normalized to i16.

### Preamble / pilot sync (current)

There is **no correlation-based sync**. The preamble is a *pilot tone repeated as data symbols*: `preamble_symbols = [16]` repeated `preamble_repeats = 8` times, **prepended per channel** by the encoder (`modem_encode.rs` lines 308–319; the integration mirror is `encode_frame_to_samples` in `tests/modem_roundtrip.rs` lines 414–425).

Decode "sync" is purely symbol-domain (`modem_decode.rs` lines 121–172, mirrored by `decode_samples_to_frame` in the test file lines 314–387):

1. Slice the buffer into fixed `samples_per_symbol` windows **starting at sample 0** (`while window_start + samples_per_symbol <= len { window_start += samples_per_symbol }`).
2. For each window, run `goertzel_mag_squared` (lines 537–556) at every tone frequency of each channel and take the arg-max as the detected symbol.
3. Build the repeated-pilot pattern and `find_subslice` it in each channel's detected-symbol stream; trim everything up to and including the pattern (decode bin lines 149–169).

`goertzel_mag_squared` rounds the target frequency to the nearest bin `k = round(f*N/sr)` (line 544) — it assumes the tone sits *on* a bin.

### Two coding paths

- **Repetition FEC:** `packetize_stream` / `depacketize_stream` (lines 565–701). Each `PKT1` packet is emitted `repeats` times; decode majority-votes per byte, preferring CRC-valid copies. Bulky (N× the data) but simple.
- **Reed-Solomon erasure FEC:** `packetize_stream_rs` (721–788), `packetize_stream_rs_interleaved` (991–1090), `depacketize_stream_rs` (793–954). Fixed `data_shards` / `parity_shards` / `shard_size` per call, carried in each 28-byte `RS01` shard header. Decode drops CRC-mismatched shards as erasures and reconstructs from parity. Interleaving (`interleave_packets`, 968–987) spreads shard *i* of every block adjacently so a burst erases ≤1 shard per block. **The rate is fixed by the caller** — there is no link-adaptive selection and no in-band rate signaling; the receiver must be told `d`/`p` out of band (e.g. `--rs-data`/`--rs-parity` CLI flags).

### Why the four real-air impairments break the current decode

1. **Start offset.** Windowing begins at sample 0 (decode bin line 123). If the burst starts at sample `δ`, every window straddles two symbols, Goertzel energy splits, arg-max is garbage, and the pilot pattern is never found at offset 0 (`test_preamble_pilot_detected_on_default_params` asserts exactly `Some(0)`). The pilot *repeats*, so a lucky alignment can recover mid-burst, but with no fractional-sample correction a non-multiple offset is never clean.
2. **Clock / sample-rate mismatch.** Windows advance by a *fixed* `samples_per_symbol` (decode bin line 123). If the transmitter's clock differs by a few hundred ppm, the receiver's window boundaries drift relative to the true symbol boundaries; by late in the burst a window spans two symbols and detection collapses. Nothing tracks this drift.
3. **Frequency offset.** Goertzel uses `k = round(f*N/sr)` (line 544). A small Hz shift moves tone energy off the exact 25 Hz bin center; energy leaks into neighbouring bins (scalloping loss) and the arg-max can flip to an adjacent tone. The plan's whole virtue — tones *on* bin centers — is what a frequency offset destroys.
4. **Multipath / room reverb.** A delayed, attenuated echo adds a copy of symbol *n−1* into symbol *n*'s window: inter-symbol interference. The Hann window helps a little, but a strong tap raises the Goertzel response of the *previous* symbol's tone inside the current window and can flip the arg-max.

---

## b. Sync design — start-of-burst detection + symbol-timing recovery

### Approach

Add a **linear-chirp preamble** located by **cross-correlation**, then do **per-symbol timing recovery** that tracks slow clock drift across the burst.

1. **Chirp preamble (start-of-burst).** Prepend a short linear up-chirp sweeping across a band inside the modem's clear region (e.g. ~3 kHz→11 kHz over a few symbol durations). The receiver cross-correlates the incoming samples against the known chirp template; the correlation peak gives the burst start to *sample* resolution (robust to start offset, and — because a chirp's autocorrelation is a sharp peak — robust under noise and tolerant of a modest frequency offset, which a matched filter degrades only gracefully). This replaces "assume start at sample 0."

2. **Symbol-timing recovery (clock drift).** After the chirp locates `t0`, instead of advancing by a *fixed* `samples_per_symbol`, run a per-symbol timing estimator. For each symbol window, compute the Goertzel/energy profile at three sub-window positions (early / on-time / late — a classic early-late gate) and nudge the next window boundary by a fractional sample toward the peak. This tracks a slowly-sliding boundary across the burst. The accumulated correction also yields an estimated clock-offset ppm for diagnostics.

3. **Frequency-offset estimate (optional refinement).** The chirp correlation, or a short pilot tone after it, gives a coarse frequency-offset estimate that Pass C can feed into a Goertzel that evaluates a *fractional* bin `k` rather than rounding — restoring on-bin behavior in the offset case.

### Coexistence with the existing pilot preamble — **gate behind a param, default off**

The repeated-pilot preamble stays the default so **every existing test is unaffected**. A new `SyncParams { mode: SyncMode, ... }` selects:

- `SyncMode::PilotOnly` (default) — current behavior, byte-for-byte.
- `SyncMode::Chirp` — prepend/locate the chirp; the pilot may still follow for fine symbol-phase, or be dropped, decided in Pass C.

The chirp is *prepended to the rendered sample stream*, so it is orthogonal to the symbol/tone counts — **counts are not touched**. Pass A ships these as stubs (`SyncMode::Chirp` falls back to the offset-0 behavior); Pass C implements real correlation + early-late tracking.

### Function signatures added (Pass A: compiling stubs)

```rust
pub enum SyncMode { PilotOnly, Chirp }

pub struct SyncParams {
    pub mode: SyncMode,
    pub chirp_symbols: usize,    // chirp length in symbol-durations
    pub chirp_f_lo_hz: f32,
    pub chirp_f_hi_hz: f32,
}
impl Default for SyncParams { /* PilotOnly, sane chirp band */ }

pub struct SyncResult {
    pub start_sample: usize,         // located burst start
    pub samples_per_symbol: f32,     // fractional, drift-corrected
    pub freq_offset_hz: f32,         // estimated carrier offset
    pub confidence: f32,             // 0..1 correlation-peak sharpness
}

/// Render the sync preamble (chirp) as i16 samples to prepend to a burst.
pub fn render_sync_preamble(params: &ModemParams, sync: &SyncParams) -> Vec<i16>;

/// Locate the start of a burst (cross-correlation in Chirp mode; 0 in PilotOnly).
pub fn detect_burst_start(samples: &[i16], params: &ModemParams, sync: &SyncParams)
    -> Result<SyncResult, ModemError>;

/// Given a located start, produce drift-corrected symbol-window boundaries.
pub fn recover_symbol_timing(samples: &[i16], params: &ModemParams, sync: &SyncResult)
    -> Result<Vec<usize>, ModemError>;
```

---

## c. Rate-selectable coding design

### Goal

A coding-*rate* layer over the existing interleaved RS so redundancy scales with link quality, instead of brute-force repetition. The bulky repetition path becomes the *lowest, last-resort* profile rather than the default robustness lever.

### Rate / profile enum

```rust
pub enum CodingProfile {
    /// Map of legacy repetition FEC (no RS). Lowest efficiency, kept for back-compat.
    Repetition { repeats: usize, pkt_size: usize },
    /// Interleaved Reed-Solomon at a named redundancy point.
    RsRate(RsRate),
}

pub enum RsRate {
    /// d=4, p=1  — high throughput, clean links (20% parity)
    High,
    /// d=4, p=2  — balanced (~33% parity)
    Medium,
    /// d=4, p=4  — robust, lossy links (50% parity)
    Low,
    /// explicit (d, p, shard_size) escape hatch
    Custom { data_shards: usize, parity_shards: usize, shard_size: usize },
}
```

> **Pass C note (shard geometry).** The Pass-A sketch above listed `8/2, 6/3, 4/4`.
> Pass C changed the named rates to a **constant-`data_shards` ladder** — `d=4` for
> all three, growing only parity `p = 1 → 2 → 4` — because the redundancy ladder must
> be monotone in *two* axes the tests pin: the parity fraction `p/(d+p)`
> (0.20 → 0.33 → 0.50) *and* the encoded length of a given frame. A growing-`d`/
> shrinking-block ladder inverts the encoded-length order for sub-block payloads (the
> high rate's larger block zero-pads more), so the encoded size of `High` came out
> *larger* than `Low` — the opposite of "more redundancy ⇒ bulkier". Holding `d`
> constant and adding parity keeps both axes monotone while leaving every RS rate
> cheaper than brute-force repetition. The `Low` robust point (`d=4, p=4`) is
> unchanged from the sketch.

### How the sender picks + signals the rate

- **Pick:** caller chooses a `CodingProfile`, or (Pass C) a helper maps a measured/estimated link SNR → `RsRate`.
- **Signal (in-band):** the rate must survive to the receiver without out-of-band `--rs-data` flags. Add a small fixed-size **coding header** ("`CDG1`") emitted **once at the very front of the packetized stream**, before the `RS01`/`PKT1` packets, carrying `{profile_tag:u8, data_shards:u16, parity_shards:u16, shard_size:u16}`. It is itself protected by being **emitted in triplicate** (cheap, fixed cost) so a burst cannot erase the rate. This keeps the existing `RS01`/`PKT1` packet formats **byte-identical** — the rate header is a pure prefix.

### How the receiver learns it

`parse_coding_header(stream) -> Result<(CodingProfile, usize /*consumed bytes*/), ModemError>`: majority-votes the triplicated `CDG1` header, returns the profile and how many bytes to skip. If no `CDG1` is found, it returns `CodingProfile::default()` and `consumed = 0` — i.e. a stream produced by the *legacy* path (no rate header) is still decoded exactly as today. The receiver then dispatches to `depacketize_stream_rs` / `depacketize_stream` with the learned parameters — **no CLI rate flags needed**.

### Trimming the repetition path

`Repetition` stays a selectable profile for back-compat and as the absolute-floor option, but the *recommended* robustness ladder is `RsRate::High → Medium → Low`. RS at `d=4,p=4` gives 2× redundancy with erasure-correction far stronger than 2× blind repetition, so the bulky high-`repeats` repetition configs become unnecessary in normal operation.

### Backward compatibility (explicit)

- `packetize_stream`, `packetize_stream_rs`, `packetize_stream_rs_interleaved`, and both depacketizers are **unchanged** and remain `pub`.
- A stream with no `CDG1` prefix decodes identically to today (`parse_coding_header` returns the default and consumes 0 bytes).
- The new rate-selectable entry points are *additive wrappers*:

```rust
pub fn packetize_with_profile(frame: &[u8], profile: &CodingProfile) -> Result<Vec<u8>, ModemError>;
pub fn depacketize_with_profile(stream: &[u8]) -> Result<Vec<u8>, ModemError>;
pub fn parse_coding_header(stream: &[u8]) -> Result<(CodingProfile, usize), ModemError>;
```

Pass A ships these as stubs: `packetize_with_profile` ignores the profile and emits a fixed legacy interleaved-RS stream (no `CDG1` yet); `depacketize_with_profile` delegates to `depacketize_stream_rs`; `parse_coding_header` returns the default profile, 0 consumed.

---

## d. Channel model (Pass A — REAL working code)

`AcousticChannelParams` + `simulate_acoustic_channel` live **in `src/modem.rs`** (library) so both `tests/modem_roundtrip.rs` and `src/bin/channel_sim.rs` can call them. All randomness is a **seeded `rand_chacha::ChaCha8Rng`** so impairments are reproducible. Four independently-dialable knobs, applied in this order:

1. **Start offset.** Prepend `start_offset_samples` of low-amplitude seeded noise (or trim, if negative) so the burst no longer begins at sample 0. Models the receiver capturing before the burst.

2. **Clock / sample-rate drift (fractional resampling).** Resample the signal by factor `r = 1 + clock_ppm * 1e-6` using **linear interpolation** at fractional source positions `i/r`. A positive ppm stretches the signal (receiver clock slower), so symbol windows of fixed length slowly slide — exactly impairment (2). This is the canonical way to inject a clock offset into a sampled signal.

3. **Frequency offset (mixing).** Multiply by a complex exponential's real part — i.e. mix the real signal with `cos(2π f_off t)` (single-sideband-style for a real passband signal we apply `s[n]·cos(2π f_off n/sr)` as a first-order shift; documented as an approximation suitable for the small offsets of interest). This nudges tone energy off the 25 Hz bin centers — impairment (3). `freq_offset_hz` is the dial.

4. **Multipath / echo (FIR tap).** Convolve with a 2-tap FIR `y[n] = x[n] + g·x[n − D]`, where `D = echo_delay_samples` and `g = echo_gain` (0..1). One delayed, attenuated copy smears symbol *n−1* into *n* — inter-symbol interference, impairment (4).

Plus a small **jitter** knob: per-sample timing jitter is folded into the resampling step as a seeded Gaussian (Box–Muller over the ChaCha8 stream) perturbation of the fractional read position, std-dev `jitter_samples`. A zero `jitter_samples` is exact.

After all stages the signal is renormalized to i16 range to avoid clipping artifacts dominating the result.

**Determinism:** every random draw (start-offset noise, jitter) comes from `ChaCha8Rng::seed_from_u64(seed)`; identical `seed` + identical params ⇒ identical output. A **zero-impairment config** (`AcousticChannelParams::identity()`: 0 offset, 0 ppm, 0 Hz, `echo_gain = 0`, 0 jitter) is a **near-identity passthrough** (only the final renormalization touches samples, and within a tight tolerance the output equals the input). The smoke unit test pins both: identity ≈ input, and a non-trivial config genuinely perturbs the samples.

### Channel-model surface (Pass A: real impl)

```rust
pub struct AcousticChannelParams {
    pub seed: u64,
    pub start_offset_samples: isize,  // + prepend noise, - trim
    pub clock_ppm: f32,               // sample-clock offset, parts-per-million
    pub freq_offset_hz: f32,          // carrier frequency offset
    pub echo_delay_samples: usize,    // multipath tap delay
    pub echo_gain: f32,               // multipath tap gain (0..1)
    pub jitter_samples: f32,          // per-sample timing jitter std-dev
}
impl AcousticChannelParams {
    pub fn identity() -> Self;        // zero-impairment passthrough
}

/// Apply the seeded acoustic-channel model to an i16 burst.
pub fn simulate_acoustic_channel(samples: &[i16], params: &AcousticChannelParams) -> Vec<i16>;
```

---

## e. Risks / trade-offs + migration path

### Risks / trade-offs

- **Chirp vs. pilot.** A chirp preamble adds fixed airtime overhead and a correlation cost at decode, but buys robust start detection and frequency tolerance the pilot cannot. Gating it behind `SyncMode` (default `PilotOnly`) means zero regression risk in Pass A and a clean opt-in.
- **In-band rate header (`CDG1`).** Triplicating the header trades a few bytes for surviving a burst; the alternative (out-of-band flags) is what we are explicitly removing. The header is a pure prefix, so legacy streams keep working.
- **Channel-model fidelity.** Linear interpolation for resampling and a real-cosine "mix" for frequency offset are first-order approximations (not a full Hilbert/analytic-signal SSB shift). They are *correct in direction and magnitude* for the small offsets of interest and are the standard cheap models for unit-test scaffolding; Pass C can upgrade to polyphase resampling if a test demands it. Documented as such.
- **Counts are frozen.** All new work is additive (chirp prepend, header prefix, channel pre-processing) precisely so the load-bearing `channels=4`/`m_tones=32` invariants and the pilot=16 assumption never move.

### Migration path

- **Pass A (this pass):** real channel model + smoke test; sync + rate API as compiling stubs (`// TODO(s7-passC): real impl`); all existing unit + integration tests stay GREEN; `default-features` excluded build/test commands pass.
- **Pass B:** Test Engineer writes failing tests that drive `simulate_acoustic_channel` (start-offset / drift / freq-offset / multipath cases) through the encode→sync→decode path and assert recovery; they go RED against the stubs.
- **Pass C (done):** implemented `detect_burst_start` (normalized chirp cross-correlation, coarse-to-fine with a coarse stride < the chirp autocorrelation main lobe, bounded to a lead-region search), `recover_symbol_timing`, the frequency-offset-tolerant Goertzel, the `CodingProfile` rate selection + `CDG1` signaling, and the `select_rate(snr_db)` auto-selector. Pass B's 10 RED tests went GREEN with no test edits. Two implementation choices diverged from the Pass-A sketch and are documented at their sites:
  - **Symbol timing — per-burst stride search, not a per-symbol early-late nudge.** A constant clock offset makes the true symbol length a *constant* `sps·(1+ppm)` for the whole burst (not per-symbol jitter), so `recover_symbol_timing` estimates ONE drift-corrected stride by maximizing the **mean** (not sum — a sum is biased toward shorter, more-numerous windows) dominant-tone alignment energy over all windows on a tight sub-sample grid, then sharpens it with a two-anchor (early/late) slope refinement. Uniform windows laid at `start + i·stride` still satisfy the cumulative-slide timing test (divergence grows linearly with `i`). A per-symbol early-late loop proved unstable on clean (no-drift) signals — it accumulated per-symbol energy noise into a spurious walk.
  - **Frequency offset — a narrow band-energy Goertzel, not a de-mix.** The Pass-B decode harness calls `goertzel_mag_squared` at the *nominal* tone frequencies and does not thread `SyncResult.freq_offset_hz` into the slice, so the offset cannot be corrected by a pre-decode de-mix through that harness. Instead the tone detector itself was made offset-tolerant: `goertzel_mag_squared` now sums five generalized-Goertzel probes across a ±0.75-bin band around the target, recapturing the energy a carrier offset (and the multipath comb) displaces off the nominal bin, while staying inside the ≥2-bin tone spacing so per-tone/per-band selectivity (and every on-bin test) is preserved. `detect_burst_start` still estimates and reports `freq_offset_hz` for diagnostics.
</content>
</invoke>

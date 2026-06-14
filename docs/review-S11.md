# Quality Gate Review — S11 (WS-4 Phase 2: OpenCV→pure-Rust + FluidSynth→in-process-synth collapse)

Reviewer: Quality Gate (independent). Base: `fa6b120` (S10 WS-4 Phase 3). Toolchain: cargo 1.96.0 (userspace).
Default features: `["pure-analysis","synth"]`. ALSA dev libs present; cpal builds; `/dev/snd` live.

**Overall verdict: PASS.**

All five verdict criteria PASS. Every previously-green test net stays green; the two new lanes are file-disjoint,
module-clean, and reach the engine through the existing seam with `engine.rs` byte-unchanged. The audible-quality
claim is honestly scoped to "non-silent through the bundled SoundFont; the ear test is the operator's pending gate" —
no overclaim found. The known `imageproc`≠OpenCV fidelity deltas are disclosed in both the module comments and the
design doc.

---

## Compilation Status

| Build | Command | Result |
|---|---|---|
| DEFAULT (pure-Rust) | `cargo build` | **OK** (exit 0). Embeds the 31 MB SF2; warnings only in pre-existing modem bins. |
| Pure lanes explicit | `cargo build --no-default-features --features pure-analysis,synth` | OK |
| Opt-in midi-out gating (no synth) | `cargo check --no-default-features --features pure-analysis,midi-out` | **OK** (exit 0) |
| Opt-in midi-out + synth | `cargo check --no-default-features --features pure-analysis,synth,midi-out` | **OK** (exit 0) |

The default binary is confirmed the PURE path: the smoke run (below) prints `Image loaded from source (pure-Rust)` and
`Starting in-process synth (rustysynth + cpal, bundled SoundFont)` — no OpenCV/FluidSynth/MIDI-port code on the path.
The `opencv` feature is NOT built here (it fails at `clang-sys`/libclang — a pre-existing system-dep blocker, not a
regression, not introduced this session). Its GATING is verified correct without a libclang build via the two
`midi-out` checks above, which exercise the `#[cfg(feature="midi-out")]` `MidiOut` path + the `Box<dyn AudioSink>`
driver. **A libclang-capable box is required to fully compile the `opencv` path — recommend the operator verify there.**

## Format / Lint Status

- **Format (`rustfmt --edition 2021 --check`)** on `src/pure_analysis.rs`, `src/synth_sink.rs`,
  `tests/phase2_pure_pipeline.rs`: **CLEAN** (no diff on any of the three). NON-BLOCKING; none to note.
- **Clippy** (`cargo clippy --no-default-features --features pure-analysis,synth -- -W clippy::all`):
  **ZERO warnings in the new files** (`pure_analysis.rs` / `synth_sink.rs`). Pre-existing style/dead-code warnings in
  `modem_encode.rs` / `unpack_tiled_payload.rs` / `chord_engine.rs` are unchanged and NON-BLOCKING (those files are
  byte-identical to base).

## Test Results (per-net pass/fail counts)

| Net | Command | Result |
|---|---|---|
| lib (default features) | `cargo test --lib` | **98 passed**, 0 failed (expected 98 ✓) |
| lib (features off) | `cargo test --lib --no-default-features` | **70 passed**, 0 failed (expected 70 ✓) |
| phase2 integration (NEW) | `cargo test --test phase2_pure_pipeline` | **7 passed**, 0 failed (expected 7 ✓) |
| cli_parse | `cargo test --test cli_parse` (default) | 24 passed, 0 failed |
| engine_equivalence | default | 9 passed, 0 failed |
| engine_seam | default | 10 passed, 0 failed |
| tui_render | default | 13 passed, 0 failed |
| modem_roundtrip | default | 17 passed, 0 failed |
| modem_realair | default | 10 passed, 0 failed (93.1 s — expected slow, not a failure) |
| qg_probe_band_isolation | default | 1 passed, 0 failed |

**No regression in any previously-green net.** A full `cargo test` (all targets) was also run and is green throughout.

NOTE on a harness artifact: running a prior net with the `--test <name> --no-default-features` SELECTOR errors, because
cargo still compiles ALL integration-test targets (including `phase2_pure_pipeline`, which legitimately needs the
features) before filtering to the named one. This is a cargo target-selection quirk, NOT a net failure: each prior net
passes under its real feature config (verified individually under default features, counts above). The new pure nets'
intended headless config is `--no-default-features` for the *lib* net (70/70 green).

---

## The 5 Verdict Criteria (explicit PASS/FAIL)

### 1. DEFAULT build is pure-Rust + works on this box — **PASS**
`cargo build` (default) succeeds with no OpenCV/FluidSynth system libs, embedding the 31 MB SF2. The produced binary is
the pure path (smoke run confirms `pure-Rust` acquisition + `rustysynth + cpal` synth). Not blocking.

### 2. engine.rs is BYTE-UNCHANGED from base — **PASS**
Independently recomputed:
```
git show fa6b120:src/engine.rs | sha256sum →  66becdaa8400ec649b7755463ebed1502cc5138dd83655fff5ef4569fd8e9fd9
sha256sum src/engine.rs              →  66becdaa8400ec649b7755463ebed1502cc5138dd83655fff5ef4569fd8e9fd9
```
**Identical.** `git diff fa6b120 -- src/engine.rs` is empty. The Phase-2 premise holds: the new analyzer/sink implement
the EXISTING seam traits with zero engine change.

### 3. Opt-in flags still reach OpenCV/external-MIDI (nothing deleted) — **PASS**
- `Cargo.toml`: `opencv = { version = "0.95.1", optional = true }` and `midir = { version = "0.8", optional = true }`
  are both still DECLARED; features `opencv = ["dep:opencv", "image"]` and `midi-out = ["dep:midir"]` still reach them.
  Only the `default` line changed (now `["pure-analysis","synth"]`); the main bin's `required-features` was dropped so
  the default set builds it.
- `main.rs`: every OpenCV reference is `#[cfg(feature="opencv")]`-gated (acquisition/highgui/overlays/`PrecomputedSource`),
  and `MidiOut`/`impl AudioSink for MidiOut` is `#[cfg(feature="midi-out")]`-gated; sink is a `Box<dyn AudioSink>`.
- Both gating checks (criterion-3 commands) compile clean, exercising the `MidiOut` path + the trait-object driver
  WITHOUT a libclang build.
- A libclang-capable box is required to fully compile the `opencv` path; recommend operator verification there.

### 4. All prior + new headless nets green — **PASS**
98 / 70 / 7 hit the expected counts exactly; all eight discovered prior nets pass (counts in the table above). No
previously-green net regressed.

### 5. Parity HONESTLY characterized, NOT overclaimed — **PASS**
- **Audible sound:** the strongest claim made anywhere is "non-silent through the bundled SoundFont." The synth tests
  assert `peak > 1e-4` and `>100 audible samples` (non-silence + sustain), never sound *quality*. The design doc §2/§4.2
  states plainly "neither pure-Rust synth is bit-identical to FluidSynth" and §6.5 marks the audible ear test as
  "operator-owned, post-build gate." Overclaim grep for `matches FluidSynth` / `verified good` / `exact parity` /
  `bit-identical` returned only two hits — both are the HONEST negations ("NOT bit-identical"). No code comment or doc
  claims the sound quality is verified-good.
- **`imageproc`≠OpenCV deltas DISCLOSED:** `pure_analysis.rs` documents each delta inline — `shape_complexity`
  (connected-components vs OpenCV `find_contours`, called out as "the LARGEST honest fidelity delta"), Canny L1-vs-L2 /
  Gaussian-kernel differences, the hand-rolled f64 Laplacian (imageproc 0.23 lacks `laplacian_filter`), and the
  circular-vs-arithmetic hue mean at the red wrap. The design doc §2 table + §9 risks repeat the same disclosures. No
  place presents the port as exact parity where it isn't.

---

## Module Boundary Audit (per file)

| File | Finding |
|---|---|
| `src/pure_analysis.rs` (Lane A, NEW) | **CLEAN.** image→features only (HSV means, Canny edge density, hand-rolled Laplacian variance, Otsu+connected-components shape proxy, 8-bin hue histogram, scan geometry). No music-theory logic, no MIDI, no chord/note selection, no modem refs. Names NO OpenCV type and no `image_analysis` type (verified by read). |
| `src/synth_sink.rs` (Lane B, NEW) | **CLEAN.** Audio transport + synthesis only: `note_on/note_off/program_change` → `MidiCmd` → lock-free SPSC (`rtrb`) → cpal audio-thread callback → `rustysynth::process_midi_message` + `render`. No note-SELECTION, no image logic, no modem refs. |
| `src/engine.rs` | **UNCHANGED** (sha256 match, criterion 2). |
| `src/main.rs` | **Orchestration only** — `#[cfg]` analyzer/sink selection, the driver loop, and the new `Command::Tui` match arm (a friendly redirect to the dedicated `audiohax-tui` bin). The jitter/`Instant` scheduling is the same adapter-owned logic as base. No new music-theory logic added here. |
| Protected set (diff vs `fa6b120`) | **ALL UNCHANGED:** `chord_engine.rs`, `mapping_loader.rs`, `assets/mappings.json`, `modem.rs`, `midi_output.rs`, `image_analysis.rs`, `image_source.rs`, `cli.rs`, `tui.rs`, `engine.rs`, and all of `src/bin/*`. `git diff fa6b120 --name-only` = `Cargo.lock`, `Cargo.toml`, `src/lib.rs`, `src/main.rs` only (plus the untracked new files/assets). No unexpected modification. |
| Lane disjointness | Lane A (`pure_analysis.rs`) and Lane B (`synth_sink.rs`) are FILE-DISJOINT. The only shared touches — `Cargo.toml` (one feature block + opt-in deps), `lib.rs` (two `#[cfg]` module decls), `main.rs` (cfg selection) — are the serialized Lane-C surface, not conflicting. |
| `Cargo.lock` | Only ADDS packages (cpal, imageproc, rustysynth, rtrb + transitive: alsa, nalgebra, coreaudio/objc2, ndk, etc.). **Zero removals or downgrades** of existing pins — no existing dependency perturbed. |

## Musical Logic Review

Largely N/A — `chord_engine.rs` is byte-unchanged (verified). The only musical surface is that the pure analyzer's
features must reach musical decisions. Spot-check of the Test Engineer's two feature→music tests:

- `different_images_drive_different_modes`: a HARD equality assertion (`green_mode == "Ionian"`, `blue_mode ==
  "Aeolian"`, `green != blue`). Mode is a deterministic, RNG-free hue→mode lookup, so this genuinely proves the pure
  analyzer's `avg_hue` reaches the engine — not a tautology. Confirmed PASS.
- `edge_density_difference_changes_realized_note_content`: **confirmed a real strict assertion, not a tautology.** The
  fixture was deliberately corrected to a **4-px checkerboard** (per-scan-bar Canny edge_density ≈ 0.27) so it crosses
  the engine's 0.25 melody-rhythm band boundary (SUSTAINED 1 onset → DOTTED 2 onsets), vs a flat field (edge 0.0 → 1
  onset). The test holds MODE constant (both grayscale → Phrygian, asserted) so only edge_density varies, then asserts
  `edge_melody > flat_melody` (strict increase of melody ONSETS on channel 3), not mere inequality. The 1-px-stripe
  trap (edge ≈ 0.08, below the band) is documented in the `checkerboard` doc-comment as the reason the fixture must
  cross a band boundary. This is the correct, non-tautological feature→music proof.

## Test Quality Assessment

The new tests assert real properties, not `is_ok()`/`!is_empty()` alone:
- `end_to_end_pure_image_emits_valid_midi_stream`: every realized note in `24..=108`, every velocity `1..=127`, every
  channel `< 16`, and `note_on count == note_off count` (no hung/leaked notes).
- `end_to_end_bass_instrument_plays_low_register`: strict `bass_max < melody_min` (role→register mapping reaches the
  stream; not a flat unison).
- `same_engine_pure_path_is_deterministic_across_passes`: byte-identical decision stream across two passes (and asserts
  non-empty first, so the comparison is meaningful).
- `captured_pipeline_events_render_nonsilent_audio_offline`: `peak > 1e-4` AND `>100 audible samples` (sustain, not a
  lone click), rendered **OFFLINE** via `rustysynth` into in-memory L/R buffers.
- `pure_analysis.rs` inline tests: known-color HSV conversions, circular-vs-arithmetic hue at the red wrap, flat-vs-edge
  Canny, flat-vs-checkerboard Laplacian variance (`> 1000`), histogram normalization to 1, FeatureSource contract
  (row width, dense bar_index, pad/truncate, zero-instruments error).

**Headless discipline confirmed:** no test opens a cpal stream (no `SynthSink::new` is CALLED — the `AudioSink` bound is
proven at compile time via `assert_is_audiosink::<SynthSink>()`); audio is always rendered offline. No test writes to the
filesystem; the bundled SF2 is reached via `include_bytes!`.

## Integration Assessment

The lanes fit through the engine seam cleanly: Lane A `impl FeatureSource for PureAnalysisSource` and Lane B
`impl AudioSink for SynthSink` satisfy the exact existing trait signatures (the integration test drives both together
through `PipelineEngine::tick`/`decide_step` and passes). No `TODO(s11…)` markers anywhere in `src/`, `tests/`, `docs/`
(grep clean). Smoke run `cargo run --bin audiohax -- play --steps 3` on this box:
```
Image loaded from source (pure-Rust): 900x440
Completed scanning image (pure-Rust). Steps: 3
Global features: GlobalFeatures { avg_hue: 198.6, avg_saturation: 64.5, ... edge_density: 0.036, texture_laplacian_var: 1957.8, shape_complexity: 1.81, aspect_ratio: 2.045 }
Engine mode: Dorian
Starting in-process synth (rustysynth + cpal, bundled SoundFont)...
Synth audio stream started @ 44100 Hz.
Completed playback of 3 steps.
```
The pure path starts, opens a real cpal/ALSA stream against `/dev/snd`, and completes 3 steps with **no panic**. (The
audible quality itself is the operator's pending ear-test — not asserted here.)

## Overclaim Check (criterion 5 detail)

No overclaim found. The audible quality is consistently scoped to "non-silent / produces audible output through the
bundled SoundFont" in both code (`synth_sink.rs` test message, the `inaudible against a 250 ms step` quantization note)
and docs. The design doc explicitly states the pure synth is NOT bit-identical to FluidSynth and routes the timbre
question to the operator's post-build ear test plus the `midi-out`→external-FluidSynth and oxisynth escape hatches. The
`imageproc` fidelity deltas (esp. `shape_complexity` connected-components-vs-contours, Canny L1/L2, Laplacian border
handling, hue red-wrap) are disclosed at the point of implementation and in the design §2 table + §9 risks.

## Blocking Issues

**None.**

## Non-Blocking Issues

1. **`opencv` path not fully compilable on this box** (pre-existing libclang blocker — NOT introduced this session, NOT
   a regression). Its gating is verified correct without a libclang build, but a full `--features opencv` compile +
   the A/B fidelity comparison must be done on a libclang-capable box. Recommend operator verification there.
2. **Audible quality unverified (by design).** No one has heard the output; the ear test is operator-owned and pending.
   The strongest in-session claim is "non-silent through the bundled SoundFont," which is what the tests prove.
3. **Linux build prereq survives:** cpal's ALSA backend needs `libasound2-dev` at build time (present here). This is the
   single, ubiquitous, documented Linux build dependency — categorically smaller than the removed OpenCV/libclang
   blocker; Windows/macOS default paths are dep-free. Disclosed in design §6.4.
4. **Pre-existing warnings** in `chord_engine.rs` / `modem_encode.rs` / `unpack_tiled_payload.rs` (unused vars, dead
   field) are unchanged from base and out of scope for this session.
5. **`include_bytes!` of the 31 MB SF2** adds ~31 MB to the `audiohax` binary. Acceptable for a zero-config audible
   default; the operator can swap a smaller font via `SoundFontSource::Path` (the override is supported, though the
   `--soundfont` CLI flag wiring is noted as a small follow-on, out of Phase-2 scope).

---

## Overall Verdict: **PASS**

Lane A and Lane B are correct, well-tested, module-clean, and integrate through the existing engine seam with
`engine.rs` byte-unchanged. The default build is pure-Rust and runs end-to-end on this box; all prior nets stay green;
the new nets assert real properties headlessly; opt-in OpenCV/external-MIDI capabilities are retained and correctly
gated; and the parity characterization is honest with the audible-quality gate properly deferred to the operator.
Cleared for the lead to integrate WS-4 Phase 2, with the two operator-owned follow-ups noted (libclang-box `opencv`
verification + the audible ear test).

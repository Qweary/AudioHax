# Design — S55 WS-4: Cross-Platform CI + Self-Serve Render (DESIGN ONLY)

**Workstream:** WS-4 (Usability / Cross-Platform / Packaging), objective/out-of-class slice.
**Author role:** Rust Architect — DESIGN ONLY. This document specifies work for the Implementer
(CI YAML) and the Test Engineer (regression test). It changes no source or config file and does
not touch the FROZEN `src/engine.rs` (sha256 `e50c7db1…a2348261`).

**Scope delivered here:**
1. Cross-platform CI design (ubuntu / windows / macos) proving BUILD + headless tests on the
   default (pure-Rust) target — no operator hardware, no audio device.
2. Self-serve `render --wav` assessment (determinism, device-free, seedability, portability).
3. Quickstart command inventory for the docs-verification specialist.
4. Risks / trade-offs / what CI cannot prove.

All findings below were observed empirically on this Linux box with `cargo 1.96.0`
(`export PATH="$HOME/.cargo/bin:$PATH"`).

---

## 1. Current-State Analysis (measured)

### 1.1 Build / test / clippy — real results

| Command (default features) | Result |
|---|---|
| `cargo build --bins` | **PASS** — `Finished dev profile` (8 bins compile clean). |
| `cargo test` | **PASS** — all default-feature test binaries green; e.g. `valence_mode_s36` 7/7, `variety_s45` 5/5, `variety_scorecard_s45` 3/3. No failures observed. Slowest single test binary: `variety_scorecard_s45` ≈ 28 s; whole suite comfortably < 2 min. |
| `cargo test --no-default-features` | **BROKEN — still true (S11 drift persists).** Fails to compile: `tests/saliency_s18.rs:45 use image::{Rgb, RgbImage};` → `error[E0432]: unresolved import 'image'` (and `pure_analysis` gated out). Several integration tests import `image`/`pure_analysis` unconditionally, so the no-default-features test path does not build. **CI must NOT use `--no-default-features` for tests.** |

**Decision: CI runs the DEFAULT feature set** (`["pure-analysis", "synth", "midi-out"]`, `Cargo.toml:44`).
That set is pure-Rust (no system OpenCV, no libclang, no external FluidSynth/MIDI) and is the
path that is green headless.

### 1.2 Clippy baseline (honest)

The tree is **NOT clippy-clean.** Measured on the default target:

- `cargo clippy` (lib only): **41 warnings.**
- `cargo clippy --all-targets`: lib 41 + lib-test 10-unique (51 w/ 41 dupes) + spread across bins
  and integration tests (e.g. `keyplan_k2b` 14, `keyplan_k2a` 8, `keyplan_s25` 7, `modem_decode` 6,
  `channel_sim` 5, …). Total warning-line population ≈ **178** (includes per-crate summary + dup lines).
- Dominant categories are **style / pedantic / complexity**, not correctness:
  `doc_list_item_without_indentation` (48), `doc_overindented_list_items` (28),
  `manual_range_contains`, `identity_op`, `type_complexity`, `needless_range_loop`,
  `manual_div_ceil`, `manual_is_multiple_of`, `unnecessary_cast`, `useless_vec`, etc.

**Consequences for the CI clippy gate:**
- `cargo clippy --all-targets -- -D warnings` **FAILS immediately** (41+ pre-existing lib warnings).
  Do **not** use it as a required gate now — it would red-CI a clean build for pre-existing debt.
- `cargo clippy --all-targets -- -D clippy::correctness` **PASSES today (EXIT 0, verified).** The
  correctness lint class is currently empty; only style/pedantic warnings remain. This is the
  recommended **gate-with-teeth**: it fails a PR that introduces a genuine correctness regression
  while staying honest about the existing style backlog. See §2.5 for the exact gate.

### 1.3 Feature layout (`Cargo.toml`)

- `default = ["pure-analysis", "synth", "midi-out"]` (line 44) — pure-Rust, audible, headless-buildable.
- `pure-analysis = ["image", "dep:imageproc"]` — `image` 0.24 + `imageproc` 0.23 (pinned; 0.25 would
  drag a second incompatible `image` major — see `Cargo.toml:113–122`).
- `synth = ["dep:rustysynth", "dep:cpal", "dep:rtrb"]` — in-process SF2 synth + cross-platform audio out.
- `midi-out = ["dep:midir"]` — external MIDI (promoted into default at S12; runtime sink selection).
- `opencv = [...]` — **opt-in only**, reintroduces libclang/system-OpenCV. **CI never enables it.**
- No `rust-version` (MSRV) key in `Cargo.toml`; `edition = "2021"`.
- 8 bins: `audiohax` (main app), `audiohax-tui`, `channel_sim`, `make_packetized`, `modem_encode`,
  `modem_decode`, plus `make_tiled_payload` / `unpack_tiled_payload` (the last two carry
  `required-features = ["image"]`, `Cargo.toml:24–32`; they build under the default set because
  `pure-analysis` pulls `image`).

### 1.4 Per-OS system dependencies (from non-optional deps)

Non-optional deps that touch system libraries: `cpal` (via `synth`, default) and `midir` (via
`midi-out`, default). Both need ALSA **on Linux only**.

| OS | System dep to install in CI | Why |
|---|---|---|
| ubuntu-latest | `libasound2-dev` (apt) | cpal ALSA backend + midir ALSA-seq backend both link ALSA at build (`Cargo.toml:39, 42–43`; README prereq table line 67). |
| windows-latest | **none** | cpal → WASAPI, midir → WinMM; built into the OS. |
| macos-latest | **none** | cpal → CoreAudio, midir → CoreMIDI; built into the OS. |

No other system package is required by any non-optional dep (`serde`, `clap`, `toml`, `rand`,
`hound`, `flate2`, `crc32fast`, `aes-gcm`, `reed-solomon-erasure`, `ratatui`, `crossterm`, … are
all pure-Rust).

### 1.5 CRITICAL build prerequisite — the embedded SoundFont (blocks ALL three OSes)

`src/synth_sink.rs:47` embeds the default SoundFont at **compile time**:

```rust
const BUNDLED_SF2: &[u8] = include_bytes!("../assets/soundfonts/default.sf2");
```

But `assets/soundfonts/default.sf2` is **git-ignored** (`git check-ignore` confirms; it is a ~31 MB
file, `assets/soundfonts/.gitignore` + `assets/soundfonts/README.md`). A fresh CI checkout will
NOT contain it, so **`cargo build`/`cargo test` will fail to compile** with a missing-file
`include_bytes!` error on every OS.

**Therefore CI MUST fetch `default.sf2` into `assets/soundfonts/` before any build/test/clippy
step, on all three OSes.** The canonical fetch (README Quick Start step 2 / `assets/soundfonts/README.md`):

```
curl -L -o assets/soundfonts/default.sf2 \
  https://raw.githubusercontent.com/mrbumpy409/GeneralUser-GS/main/GeneralUser-GS.sf2
```

This is the single highest-risk CI detail — omitting it red-lines every job at compile. `curl` is
present on ubuntu, macos, and windows GitHub runners; run the step under `shell: bash` so one
command works cross-OS (windows runners provide git-bash). Optionally wrap in an
`actions/cache` keyed on the SoundFont URL to avoid re-downloading 31 MB per run per OS.

---

## 2. CI Workflow Spec — `.github/workflows/ci.yml`

The Implementer commits this file. The YAML below is a **reference draft** (documentation); the
Implementer owns the real committed file and may adjust action pin versions to current releases.

### 2.1 Intent / invariants

- Prove **BUILD (`cargo build --bins`) + headless TEST (`cargo test`, default features)** on
  `ubuntu-latest`, `windows-latest`, `macos-latest`. This is the objective "it builds on
  Windows/macOS" proof, obtained without operator hardware.
- Default features only. Never `--no-default-features` (broken, §1.1). Never `--features opencv`.
- Fetch `default.sf2` before building on every OS (§1.5).
- Clippy gate = correctness-class only (green today, §1.2), not `-D warnings`.
- CI has **no audio device** → it proves build + headless logic tests, never audible output. Expected
  and acceptable (§5).

### 2.2 Exact cargo invocations per job

1. `cargo build --bins --locked` — compiles all 8 bins on the default feature set.
2. `cargo test --locked` — runs the full default-feature test suite (the GREEN path from §1.1).
3. `cargo clippy --all-targets --locked -- -D clippy::correctness` — the honest gate (§2.5).

Rationale for `--locked`: CI should build against the committed `Cargo.lock` for reproducibility
and to make the cache key meaningful. (Drop `--locked` only if `Cargo.lock` is intentionally not
committed — verify; it is referenced by the cache key either way.)

### 2.3 Triggers, matrix, fail-fast, timeout

- **Triggers:** `push` and `pull_request` on the default branch. Per project memory the AudioHax
  default branch is **`master`** (`origin/master`); the Implementer must confirm and set it. Draft
  uses `master`.
- **Matrix:** `os: [ubuntu-latest, windows-latest, macos-latest]`.
- **`fail-fast: false`** — one OS failing should not cancel the others; we want the full portability
  picture every run.
- **`timeout-minutes: 30`** per job — generous headroom over the observed < 2 min test suite +
  first-build compile + 31 MB SoundFont fetch.

### 2.4 Toolchain pinning

No MSRV in `Cargo.toml`; edition 2021. Recommend **`dtolnay/rust-toolchain@stable`** with clippy
component. One caveat worth stating: clippy's correctness set can shift across toolchain releases,
so a future stable could in principle surface a new correctness lint and (correctly) fail the gate.
That is acceptable — it is a real signal — but if the team wants the gate perfectly reproducible,
**pin to `dtolnay/rust-toolchain@1.96.0`** (matches this box's dev toolchain). Draft uses `stable`;
flipping to a pin is a one-line change.

### 2.5 Clippy gate

Use the correctness class as the required gate (verified EXIT 0 today):

```
cargo clippy --all-targets --locked -- -D clippy::correctness
```

Optionally add a **non-blocking** informational full-lint pass so the style backlog stays visible
without red-lining CI:

```
cargo clippy --all-targets --locked    # report-only; no -D; step-level continue-on-error: true
```

Documented follow-up (out of scope here, requires source edits so not designed now): run
`cargo clippy --fix` + hand-clean the ~41 lib warnings, then tighten the gate to `-D warnings`.
Until then, `-D clippy::correctness` is the honest teeth.

### 2.6 Caching

Use **`Swatinem/rust-cache@v2`** — it keys the registry + git + `target` cache on `Cargo.lock` and
the runner OS automatically (per-OS isolation is built in), which is exactly the requirement. Place
it after toolchain install, before build. (Manual `actions/cache` on `~/.cargo/registry`,
`~/.cargo/git`, `target/` keyed on `${{ runner.os }}-${{ hashFiles('**/Cargo.lock') }}` is the
equivalent if a third-party action is undesirable.)

Optionally cache the 31 MB SoundFont with `actions/cache` keyed on its URL to save bandwidth.

### 2.7 Reference YAML draft (Implementer commits the real file)

```yaml
name: CI

on:
  push:
    branches: [master]        # CONFIRM default branch (memory: origin/master)
  pull_request:
    branches: [master]

jobs:
  build-and-test:
    name: build+test (${{ matrix.os }})
    runs-on: ${{ matrix.os }}
    timeout-minutes: 30
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]

    steps:
      - uses: actions/checkout@v4

      - name: Install Linux audio build deps (ALSA)
        if: runner.os == 'Linux'
        run: sudo apt-get update && sudo apt-get install -y libasound2-dev

      - name: Fetch default SoundFont (embedded via include_bytes! at build time)
        shell: bash            # one curl works on all 3 runners (windows uses git-bash)
        run: |
          curl -L -o assets/soundfonts/default.sf2 \
            https://raw.githubusercontent.com/mrbumpy409/GeneralUser-GS/main/GeneralUser-GS.sf2
          # sanity: non-empty, RIFF/sfbk container
          test -s assets/soundfonts/default.sf2

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable   # or @1.96.0 to pin (see §2.4)
        with:
          components: clippy

      - name: Cache cargo + target
        uses: Swatinem/rust-cache@v2

      - name: Build all bins (default features)
        run: cargo build --bins --locked

      - name: Test (default features — the green headless path)
        run: cargo test --locked

      - name: Clippy correctness gate (green today; not -D warnings, see §1.2)
        run: cargo clippy --all-targets --locked -- -D clippy::correctness

      # Optional, non-blocking style report:
      # - name: Clippy full report (informational)
      #   run: cargo clippy --all-targets --locked
      #   continue-on-error: true
```

---

## 3. Self-Serve `render --wav` Assessment

**Verdict: ALREADY COMPLETE AND CORRECT.** `render --wav` is a first-class, deterministic,
device-free, pure-Rust, cross-platform path today. No source fix is required. The only gap is a
missing **end-to-end determinism regression test** (seed → composition → WAV bytes), specified in §3.5
for the Test Engineer.

### 3.1 Device-free (writes a file, opens no audio device) — CONFIRMED

`run_render_wav` (`src/main.rs:345–477`) synthesizes **offline**: it calls
`audiohax::synth_sink::render_events_to_stereo(...)` then `write_stereo_wav(wav_path, ...)`
(`main.rs:349–456`). It never constructs a `SynthSink`/cpal stream and never opens ALSA/WASAPI/
CoreAudio. The doc comment states it explicitly (`main.rs:335–344`: "OFFLINE (no audio device)").
The live `play` path is the only cpal consumer (`main.rs` ~679–712), and `render --wav` does not
reach it.

### 3.2 Deterministic given `--seed` — CONFIRMED

- `--seed <u64>` exists on `RenderArgs` (`src/cli.rs:216–221`).
- `run_render_wav` calls `audiohax::seed::set_composition_seed(render_args.seed)` at the freeze-safe
  entry, BEFORE composing (`main.rs:399`).
- The composition path's **only** non-deterministic RNG draw is `chord_engine.rs:150` (`thread_rng()`
  inside `pick_progression`). The S41 seam (`src/seed.rs`) routes that draw through a thread-local
  register: `Some(seed)` → a per-call `ChaCha8Rng` keyed by `mix_seed(seed, counter)`
  (`seed.rs:57–70`); `None` → today's exact `thread_rng()` legacy path.
- Grep of the whole `src/` tree confirms no other RNG/clock on the render path: the only other RNG is
  `OsRng` in `src/modem.rs` (modem, not the image→music render path), and the WAV grid is jitter-free
  by construction (`main.rs:336, 431` — "NO jitter ⇒ deterministic"; the live `hold_ms` jitter is
  dropped so `play == render`, `main.rs:249–251`). No `SystemTime`/`Instant::now` on the render path.

So: **same image + same config + same `--seed` ⇒ byte-identical WAV.** Absent `--seed`, the render is
intentionally non-deterministic (legacy `thread_rng`) — documented opt-in (`cli.rs:216–219`,
README:51–58, USAGE:76–81).

### 3.3 Pure-Rust reachable on all three OSes — CONFIRMED

`run_render_wav`'s feature-source acquisition has an `#[cfg(feature = "opencv")]` arm and a
`#[cfg(not(feature = "opencv"))]` arm (`main.rs:361–393`). The **default** build (opencv OFF) takes the
`not(opencv)` pure-Rust arm (`pure_analysis::load_pure_image` / `understand_image_pure` /
`PureAnalysisSource::extract`). No OpenCV-gated code is on the default render path, so it builds and
runs identically on ubuntu/windows/macos with the default feature set.

### 3.4 Existing test coverage (and the precise gap)

- `tests/seed_s41.rs` — proves **plan-level** determinism (`CompositionPlan` `PartialEq`: same
  seed+image ⇒ identical plan; distinct seeds diverge; multi-section decorrelation; `None`
  preserves legacy). Explicitly **NOT** the WAV (`seed_s41.rs:16`).
- `tests/ab_harness_s31.rs` — proves **render-level** byte-identity: `render_wav_same_composition_same_config_is_byte_identical` (`ab_harness_s31.rs:128`) renders the SAME event
  list twice and asserts identical bytes. But it holds the **composition CONSTANT** — it does not
  exercise `set_composition_seed` → the composed events.

**Gap:** no test chains **seed → composition → WAV bytes** end to end. Nothing today would catch a
regression where a newly-introduced unseeded RNG (or a wall-clock read) leaked onto the composition
path — plan determinism and render determinism are each covered in isolation, but not their
composition through the seeded render entry.

### 3.5 Determinism regression test to ADD (for the Test Engineer)

No source change; new integration test only. Two options; **Option A preferred** (no subprocess, no
binary needed, still exercises the full seed→compose→timeline→WAV chain via public lib APIs):

**Option A — library-level (recommended).** New `tests/render_determinism_s55.rs`. For a fixed
committed image (`assets/images/example.jpg`) run the SAME chain the bin runs, twice, and assert
byte-identical interleaved samples:

1. `audiohax::seed::set_composition_seed(Some(7));`
2. pure-analysis: `load_pure_image` + `understand_image_pure` + `PureAnalysisSource::extract`
   (mirrors `main.rs:369–393`);
3. build `PipelineEngine`, `compose_from_image(&understanding)` (mirrors `main.rs:401–423`);
4. build the event timeline via the same shared builder the bin uses;
5. `synth_sink::render_events_to_stereo(SoundFontSource::Bundled, cfg, 44_100, events, 1_500)` →
   `Vec<f32>` A;
6. repeat 1–5 → `Vec<f32>` B; `assert_eq!(A, B)` (or compare `write_stereo_wav` byte outputs).

   Notes: uses default features; needs `assets/soundfonts/default.sf2` present at compile
   (`SoundFontSource::Bundled` → `include_bytes!`), i.e. the SAME CI SoundFont prerequisite from §1.5
   — so this test is automatically covered by the CI fetch step. It must call `set_composition_seed`
   itself and run single-threaded within the test fn (the register is thread-local, per `seed.rs`
   docs). Some builder helpers the bin uses live in `main.rs` (bin-private); if a needed helper is not
   `pub` in the lib, prefer Option B rather than moving code out of the bin (respect module
   boundaries; do not touch `engine.rs`).

**Option B — binary-level (fallback, strongest end-to-end).** New `tests/render_cli_determinism_s55.rs`
invoking the compiled app via `env!("CARGO_BIN_EXE_audiohax")` (Cargo sets this for integration
tests) with `std::process::Command`:

```
render --wav <tmpA> --seed 7 assets/images/example.jpg      # run twice → tmpB
```

then assert the two output files are byte-identical (SHA-256 or raw compare). Requires the bundled
SoundFont present (CI fetch, §1.5) and a temp dir. This exercises the true shipped binary end to end,
including `run_render_wav` itself.

Either test also belongs in CI (it runs headless — no audio device — because the render path never
opens cpal, §3.1).

---

## 4. Quickstart Command Inventory (for the docs-verification specialist)

Exact commands `README.md` and `docs/USAGE.md` instruct a fresh-clone user to run. The verifier
should execute each (with `export PATH="$HOME/.cargo/bin:$PATH"` and after the SoundFont fetch) and
report drift. Not fixing docs here — enumeration only.

**From `README.md`:**
- `git clone https://github.com/Qweary/AudioHax.git` (README:13)
- `cd AudioHax` (README:14)
- `curl -L -o assets/soundfonts/default.sf2 https://raw.githubusercontent.com/mrbumpy409/GeneralUser-GS/main/GeneralUser-GS.sf2` (README:18–19) — the REQUIRED SoundFont fetch
- `./play` (README:22) — build + play bundled sample
- `./play path/to/your-image.jpg` (README:29)
- `export PATH="$HOME/.cargo/bin:$PATH"` (README:33)
- `cargo run --release -- play assets/images/example.jpg` (README:40)
- `cargo run --release -- render assets/images/example.jpg --wav out.wav` (README:48)
- `cargo run --release -- render assets/images/example.jpg --wav out.wav --seed 42` (README:56)
- `./play assets/images/example.jpg --seed 42` (README:57)
- `sudo apt install libasound2-dev` (README:67 — Linux build dep)
- `cargo run --release --bin modem_encode -- out.wav secret.txt --compress` (README:93)
- `cargo run --release --bin modem_decode -- out.wav recovered` (README:94)
- `./play --help` (README:121)
- `cargo run --release -- play --help` (README:122)
- `cargo run --release -- render --help` (README:123)

**From `docs/USAGE.md`:**
- `export PATH="$HOME/.cargo/bin:$PATH"` (USAGE:6)
- `cargo run --release -- --help` (USAGE:11)
- `cargo run --release -- play [IMAGE] [OPTIONS]` (USAGE:24)
- `./play [IMAGE] [OPTIONS]` (USAGE:25)
- `./play assets/images/Lena.png --instruments 6 --ms-per-step 180` (USAGE:62)
- `cargo run --release -- play assets/images/example.jpg --reverb off --gain 1.5` (USAGE:63)
- `cargo run --release -- play assets/images/example.jpg --soundfont assets/soundfonts/FluidR3_GM.sf2` (USAGE:64–65)
- `cargo run --release -- render [IMAGE] --wav <PATH> [OPTIONS]` (USAGE:73)
- `cargo run --release -- render assets/images/example.jpg --wav out.wav --seed 42` (USAGE:84)
- `cargo run --release -- render assets/images/example.jpg --wav fluid.wav --soundfont assets/soundfonts/FluidR3_GM.sf2 --reverb off` (USAGE:85–86)
- `tools/ab-render.sh <IMAGE> [SOUNDFONT.sf2]` (USAGE:89) — **verify this helper exists** (referenced in USAGE:89 and `play` script; confirm `tools/ab-render.sh` is present in-repo, it was not enumerated in `docs/`).
- `cargo run --release --bin modem_encode -- out.wav input.txt --compress --preset robust` (USAGE:113)
- `cargo run --release --bin modem_decode -- out.wav recovered` (USAGE:116)
- `cargo run --release --bin channel_sim -- in.bin out.bin --mode acoustic` (USAGE:119)
- `cargo run --release --bin make_packetized -- input.bin out.bin` (USAGE:120)
- `--config <FILE>` config-file usage (USAGE:127–132)
- `--output midi` / `--midi-virtual` external-routing usage (USAGE:136–140)

**Drift flags to check specifically:**
- Every command depends on the SoundFont fetch succeeding first (else compile fails, §1.5) — verify
  the fetch URL still resolves.
- `soundfonts/FluidR3_GM.sf2`, `TimGM6mb.sf2`, `MuseScore_General.sf2` are git-ignored A/B fonts
  (USAGE:99–101) — the `--soundfont …` examples will fail on a fresh clone that only fetched
  `default.sf2`. Flag as expected drift, not a doc bug, unless docs claim otherwise.
- `tools/ab-render.sh` existence (referenced twice; not confirmed present).

---

## 5. Risks / Trade-offs / What CI Cannot Prove

- **What CI CAN prove:** the default (pure-Rust) target COMPILES and the headless test suite PASSES on
  ubuntu-latest / windows-latest / macos-latest — the objective "it builds on Windows and macOS"
  claim, with zero operator hardware.
- **What CI CANNOT prove (by design, acceptable):** any **audible** property. GitHub runners have no
  audio device; `play`/cpal output is never exercised. This is fine — the taste/ear-gate is a
  separate, operator-in-the-loop concern (WS-4 in-class), and `render --wav` is deliberately
  device-free so its determinism IS provable in CI without sound.
- **Risk — SoundFont fetch is a single point of CI failure (highest).** The `include_bytes!` embed
  (§1.5) means a broken/renamed upstream URL red-lines all three jobs at compile. Mitigations: pin/
  mirror the font, add the `test -s` sanity check (in the draft), and optionally cache it. This is the
  one detail the Implementer must not miss.
- **Risk — clippy gate scope.** `-D warnings` is intentionally NOT the gate (41+ pre-existing lib
  warnings, §1.2). `-D clippy::correctness` is green today but a future stable toolchain could add a
  correctness lint and fail CI; pinning the toolchain (§2.4) removes that variance if desired. The
  large style backlog (~178 warnings) is deferred debt, not a CI failure.
- **Risk — `--no-default-features` remains broken (§1.1).** CI deliberately does not test it. If the
  team wants that path green, that is a separate fix (add `required-features`/`#[cfg]` guards to the
  offending integration tests, e.g. `saliency_s18.rs`) — out of scope for this slice and NOT designed
  here. Do not let anyone "fix" CI by adding `--no-default-features`; it will fail.
- **Trade-off — default vs. release profile in CI.** The draft builds/tests in dev profile (faster
  compile, adequate for a build+logic proof). A `--release` matrix (or a separate release build job
  for artifact production, WS-4 Phase 5) is a later addition; not needed for the build-proof this
  slice targets.
- **Default-branch assumption.** Triggers assume `master` (project memory: `origin/master`).
  Implementer must confirm before committing.

---

## Appendix — Frozen-file compliance

This design requires **no** edit to `src/engine.rs`. The `--seed` determinism it relies on is the S41
thread-local register seam (`src/seed.rs` + `src/main.rs:399`), which was purpose-built to leave
`engine.rs` byte-untouched (verified by `tests/seed_s41.rs::test_pt_seed_5_engine_frozen`). The CI and
the proposed regression test add files only (`.github/workflows/ci.yml`, `tests/render_determinism_s55.rs`)
and modify no source or config.

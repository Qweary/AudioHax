# WS-4 Assessment — Usability, Cross-Platform, Packaging & Interface Architecture

Status: DESIGN / ASSESSMENT ONLY. No source modified. This document is the plan WS-4 builds against.
Author role: Rust Architect. Grounded against the working tree at the time of writing
(`Cargo.toml`, `src/main.rs`, `src/lib.rs`, `src/image_analysis.rs`, `src/image_source.rs`,
`src/chord_engine.rs`, `src/midi_output.rs`, `src/bin/*`, `README.md`, `Cargo.lock`) and against
the two prior design documents in the sibling design repo (`docs/interactive-architecture.md`,
`docs/ROADMAP.md`) so it does not contradict prior design intent.

> Convention in this doc: Rust signatures and ASCII diagrams give the SHAPE of proposed seams.
> No implementation bodies are written. Where a framework/version claim is from general knowledge
> rather than verified against this tree, it is marked **[VERIFY]** — treat those as to-be-confirmed
> before they drive a build decision.

---

## 0. Executive Orientation

AudioHax is two cleanly-separated halves sharing one crate. The **pure-Rust half** (the music engine
`chord_engine`, the modem `modem`, `mapping_loader`) compiles and unit-tests with zero system libraries
via `cargo build/test --no-default-features` — the `[features]` table and the `required-features` on the
image bins are explicitly engineered for this. The **native half** (the `audiohax` binary) depends on
OpenCV (image analysis + the live highgui window), `midir` (MIDI port I/O), and — critically — an
**externally-launched FluidSynth process** for actual sound. The dev box cannot build the native half
(no OpenCV / no libclang / no ALSA / no cmake), and there is no currently-confirmed build-capable machine.

The cross-platform story is dominated by exactly one dependency: **OpenCV via the `opencv` crate, which
needs libclang at build time** (confirmed in `Cargo.lock`: `opencv` → `opencv-binding-generator` →
`clang-sys` → `clang`). Everything else is comparatively tractable. FluidSynth is *not linked* (it is not
in `Cargo.lock`) — it is a runtime process the user starts separately and we route MIDI to over a virtual
port — so its "packaging pain" is a *runtime install + virtual-MIDI-port* problem, not a link/build problem.
That distinction reshapes the whole packaging plan.

The interface today is a single positional/`--flag` CLI in `main.rs` driving a one-shot batch run that pops
an OpenCV `highgui` window. WS-4's mandate — a first-class TUI/CLI *and* a real GUI over a shared core — is
well-supported by the prior `interactive-architecture.md` plan, which already calls for extracting the
pipeline into an `engine.rs` with a `tick()`/`update_image()`/`inject_event()`/`current_state()` surface.
**That engine seam is the linchpin of WS-4** and should be honored, not reinvented.

---

## 1. Current State — How It's Built, Configured, and Run Today

### 1.1 Build configuration (`Cargo.toml`)

- `default = ["opencv", "midir", "image"]`. All three are `optional = true` deps gated behind that feature
  set. `--no-default-features` yields a **pure-Rust library** (music + modem) with no system libs — this is
  the design's single most important cross-platform asset and the reason the headless test suite works.
- Four `[[bin]]` targets are declared with `required-features`:
  - `audiohax` (`src/main.rs`) → `required-features = ["opencv", "image"]` (the image-to-music app).
  - `make_tiled_payload`, `unpack_tiled_payload` → `required-features = ["image"]`.
  - The pure modem bins (`modem_encode`, `modem_decode`, `channel_sim`, `make_packetized`) are left to
    autodiscovery, so they build under `--no-default-features`. The `Cargo.toml` comment documents this
    intent explicitly (it was added to stop the modem integration test from needing a manual `rustc`).
- No `[profile.*]` overrides, no `[package.metadata]` packaging hints, no `build.rs`. There is no
  workspace — single crate.
- Modem/codec deps are all **pure Rust** (`hound`, `hex`, `flate2`, `crc32fast`, `thiserror`, `aes-gcm`,
  `rand*`, `twoway`, `reed-solomon-erasure`). None of these are a cross-platform concern.

### 1.2 What the "UX" actually is today (`src/main.rs`)

The entire user interface is `fn main()` plus a hand-rolled `parse_cli_arg<T>()` helper (lines 39–45) that
does `args.position(==key).and_then(get(i+1)).parse().unwrap_or(default)`. Observed surface:

- Flags: `--instruments` (default 4), `--thickness` (0.10), `--steps` (40), `--ms-per-step` (250),
  `--jitter-percent` (15).
- A magic positional: `args.get(1)` if it does not start with `--` is treated as an image path; the literal
  string `"play"` is special-cased (it both selects the example image *and* — separately, via
  `args.iter().any(|a| a == "play")` at line 447 — switches on playback). So `play` is overloaded as both a
  pseudo-path and a mode toggle.
- Output is a sequence of `println!`/`eprintln!` lines (mappings loaded, global features `{:?}`-dumped,
  chosen mode, progression, generated chords, overlay-write confirmations). The "help" is a single trailing
  `println!` only printed when you *don't* pass `play`.
- A run is strictly **batch**: load image once → `analyze_global` once → pick mode/progression/chords once →
  `scan_image` precomputes ALL steps as `Vec<Vec<ScanBarFeatures>>` → `play_scanned_steps_concurrent`
  iterates a fixed step count with a `Barrier`-synchronized worker-per-instrument pool, the coordinator
  schedules `note_on`/`note_off` by wall-clock `Instant`, and an OpenCV `highgui` window
  (`imshow("ScanBar Live")` + `wait_key(1)`) shows the moving scan bar. Overlays for first/mid/last step are
  also written to `assets/overlay_step_*.png`.

### 1.3 What makes it hard to build / run / use right now (specific)

1. **You cannot build the app on a box without OpenCV + libclang + a C/C++ toolchain.** The `opencv` crate
   generates its FFI bindings at build time through `clang-sys`/`opencv-binding-generator` (both in
   `Cargo.lock`). This is the dev box's exact blocker and is the single biggest barrier to "anyone can build
   it." It is also why no audible validation has ever been possible here.
2. **Sound requires three separate moving parts the user must wire up by hand.** Per `README.md` and
   `midi_output.rs`: (a) a *virtual MIDI port* (loopMIDI on Windows / IAC on macOS / `snd_virmidi` on Linux),
   (b) a *separately launched FluidSynth* bound to that port and an audio backend, (c) a *General-MIDI `.sf2`
   SoundFont* the user downloads themselves. `MidiOut::open_first` (midi_output.rs:11) errors out with
   exactly this instruction if no port exists. Nothing in the program starts or bundles the synth — there is
   no sound at all out of the box.
3. **The CLI is undiscoverable and partly self-contradictory.** No `--help`, no version, no usage on bad
   input; the `"play"` token is overloaded; unknown flags are silently ignored (`unwrap_or(default)`);
   feature ranges are dumped as raw `{:?}` debug. There is no config file — every run is flags-only.
4. **Hard-coded paths.** The example image (`assets/images/example.jpg`), the mappings file
   (`assets/mappings.json`), and the overlay output dir (`assets/`) are all relative literals, so the binary
   only behaves correctly when run from the repo root. There is no notion of an installed/relocatable asset
   location.
5. **The only "GUI" is an OpenCV debug window.** `highgui::imshow` is a developer-grade preview, not a
   designed interface, and it ties the entire visual front-end to OpenCV being present.

---

## 2. Native-Dependency Portability Surface (the technical core)

Confirmed from `Cargo.lock`, the native graph is: `opencv` → `opencv-binding-generator` + `clang-sys`(→`clang`)
+ `pkg-config`; `midir` → `alsa`+`alsa-sys`(Linux) / `coremidi`(macOS) / `winapi`(Windows); `image` →
`jpeg-decoder` (pure Rust). FluidSynth does **not** appear in the lockfile — it is a runtime executable.

### 2.1 OpenCV (`opencv` 0.95.1) — THE blocker

- **What it provides here:** literally all image work — HSV conversion, Canny edges, Laplacian texture
  variance, Sobel orientation, contour/circularity, hue histograms (`image_analysis.rs`), `imread`/camera
  capture (`image_source.rs`), and the live `highgui` scan-bar window + `imwrite` overlays (`main.rs`).
- **Linux:** needs `libopencv-dev` (system OpenCV ≥ 4) **plus** `llvm-dev libclang-dev clang` (the README
  already documents this) **plus** `pkg-config`. The `clang-sys` build-time bindgen step is mandatory.
- **Windows:** this is the historically painful path. The `opencv` crate must find an OpenCV install
  (prebuilt binaries → `OPENCV_DIR` + DLL dir on `PATH`, exactly as the README's `setx` dance), **and** a
  libclang DLL must be discoverable for `clang-sys` (LLVM/Clang for Windows, `LIBCLANG_PATH`). Two heavyweight
  native SDKs must line up at once; version mismatches between the prebuilt OpenCV and the crate's expected
  ABI are the classic failure. There is no static-by-default story; you ship OpenCV `.dll`s alongside the exe.
- **`vcpkg` alternative on Windows:** `vcpkg install opencv4` + the crate's vcpkg discovery is generally the
  least-bad reproducible Windows path **[VERIFY against current `opencv` crate docs]**, but vcpkg building
  OpenCV from source is slow and still needs the libclang piece.
- **Pure-Rust / better-cross-platform alternatives (worth serious consideration):**
  - The `image` crate is *already a dependency* and already pure-Rust. Decode/resize/pixel access need no
    OpenCV at all.
  - HSV stats, a Canny-equivalent edge density, Sobel orientation, and a Laplacian-variance texture metric
    are all reimplementable in pure Rust over `image` buffers (optionally with `imageproc` **[VERIFY]**, a
    pure-Rust CV-ops crate covering Canny/Sobel/contours). Contour *counting* and circularity are the only
    genuinely non-trivial ports; for the music mapping they feed `shape_complexity`/`hue_spread`, which are
    coarse heuristics — a simpler connected-components or gradient-energy proxy would likely preserve the
    musical mapping's intent.
  - **Trade-off, stated honestly:** dropping OpenCV removes the single hardest cross-platform dependency
    (and the libclang requirement entirely), making a one-command `cargo build` on a clean Windows/Linux box
    realistic. The cost is (a) re-deriving and re-validating each feature so the *musical output does not
    drift* (the owner's professional ear is the gate here — feature values feed mode/dynamics/register), and
    (b) losing OpenCV's camera capture and `highgui` window, both of which the GUI front-end will replace
    anyway. This is the highest-leverage architectural decision in WS-4 and is treated as a first-class
    fork in §3 and §6, not a footnote.

### 2.2 `midir` 0.8 (MIDI port I/O) — moderate, well-behaved

- **What it provides:** opening a MIDI-out port and sending raw 3-byte messages (`midi_output.rs`).
- **Per-OS:** `midir` is cross-platform by design (ALSA-seq on Linux, CoreMIDI on macOS, WinMM on Windows;
  the lockfile shows `alsa`/`alsa-sys`/`coremidi`/`winapi`). Linux requires `libasound2-dev` at build time
  (already in the README). It builds on all three without exotic toolchains.
- **The real friction is not the crate, it's the virtual port:** `midir` connects to a port that must already
  exist, and a software synth must be on the other end. That is a *runtime UX* problem (see §3.4), not a
  build problem.
- **Alternative:** none needed for the *port* layer. The bigger question (synthesis, below) can make `midir`
  optional rather than required.

### 2.3 FluidSynth — runtime process, NOT a link dependency

- **Reality check:** FluidSynth is absent from `Cargo.lock`. `midi_output.rs` only sends MIDI bytes to a
  port; `README.md` shows the user launching `fluidsynth ...` as a separate process bound to the virtual
  port. So AudioHax does **not** link or bundle a synth — it *expects one to be running*.
- **Per-OS pain is install + routing, not compilation:** the user must install FluidSynth/Qsynth, create a
  virtual MIDI port, route the synth to it, and supply a `.sf2`. This is the most fragile part of first-run
  UX on every OS and the most common reason "I built it but hear nothing."
- **Alternatives worth considering (these change the dependency posture meaningfully):**
  - **Bundle synthesis in-process** with a pure-Rust SoundFont synth — `rustysynth` **[VERIFY]** is a
    pure-Rust SF2 synthesizer — feeding a cross-platform audio output crate (`cpal` **[VERIFY]**, pure-Rust,
    Linux ALSA/PulseAudio/JACK + Windows WASAPI + macOS CoreAudio). This *eliminates* the external
    FluidSynth process, the virtual-MIDI-port setup, and arguably `midir` for the default "just play sound"
    path. The program would render audio itself. **Trade-off:** the engine still emits MIDI events
    internally (good — the `NoteEvent` seam already exists in `chord_engine`), so a pure-Rust synth is a
    drop-in *sink* behind that seam. The owner loses the ability to point AudioHax at *their* preferred
    DAW/synth — so the right design keeps `midir` as an *optional alternate sink* ("send to external MIDI
    port") while making the bundled synth the zero-config default.
  - This is the second-highest-leverage cross-platform decision after OpenCV: it converts "hear nothing
    until you wire up three things" into "run it and hear sound."

### 2.4 Build-toolchain deps (transitive)

- `clang-sys`/`clang` — required by `opencv` (the Windows pain multiplier). **Eliminated entirely if OpenCV
  is dropped (§2.1).**
- `cc` — C compiler shim pulled by native crates; needs a working C toolchain. Eliminated alongside OpenCV.
- `pkg-config` — Linux dependency discovery for OpenCV/ALSA. Harmless; only relevant while system libs exist.
- `cmake` — the task brief lists it as a missing dev-box dependency; it is the classic OpenCV-from-source /
  FluidSynth-from-source requirement. Not needed once OpenCV is dropped and FluidSynth is replaced by an
  in-process pure-Rust synth.

### 2.5 Blocker ranking (honest)

| Dependency | Cross-platform difficulty | Real nature | Removable? |
|---|---|---|---|
| **OpenCV (`opencv` + libclang)** | **HIGH** — the blocker | Build-time native + bindgen | Yes → pure-Rust `image`(+`imageproc`) |
| **FluidSynth + virtual MIDI** | **MEDIUM-HIGH** | Runtime install + routing | Yes → in-process `rustysynth`+`cpal` |
| ALSA dev libs (via `midir`) | LOW | Build-time, Linux only | Optional if `midir` becomes alternate sink |
| `image`, modem deps | NONE | Pure Rust | — |

Headline: **two dependencies (OpenCV, FluidSynth) carry essentially the entire cross-platform cost.** Both
have credible pure-Rust replacements that the existing module seams already accommodate.

---

## 3. Build & Packaging Story (Linux + Windows)

### 3.1 The floor: a documented, reproducible dev build (achievable now, on the current deps)

- Keep the feature flags exactly as they are. Document two build profiles clearly:
  - `cargo build --no-default-features` → pure-Rust lib + modem bins. **Works on the dev box today.**
  - `cargo build` (default) → full app; requires the per-OS native prerequisites in `README.md`.
- The README install steps are basically correct but scattered; consolidate per-OS into one copy-paste block
  each, and add the missing `cmake`/`pkg-config`/`LIBCLANG_PATH` notes that the current dev-box failure proves
  are needed. This is a docs task, doable now (see §6 Phase 0).

### 3.2 The strategic path: collapse the native surface (recommended)

The cleanest "build and run on both OSes" outcome is to **reduce the default dependency set to pure Rust**:

```
default = []                       # pure Rust: image analysis (image/imageproc), in-process synth (rustysynth+cpal)
[features]
opencv      = ["dep:opencv"]       # opt-in: richer/legacy CV path + camera capture
midi-out    = ["dep:midir"]        # opt-in: route to an external MIDI port / DAW instead of bundled synth
```

Under this posture, a clean Windows or Linux box runs `cargo build --release` and gets a working,
audible program with **no system libraries, no libclang, no external synth, no virtual MIDI port**. OpenCV
and external-MIDI become *capabilities you turn on*, not *prerequisites you must satisfy*. This is the single
biggest usability win available and it is enabled, not blocked, by the existing module boundaries (see §2.1,
§2.3). It must be validated by the owner's ear (feature-parity of the pure-Rust analysis) and on a
build-capable machine (audio actually emits) — see §6 gating.

### 3.3 Static vs dynamic, vendored vs system

- **Pure-Rust default path:** everything links statically into one Rust binary; no DLLs to ship; `cpal`
  talks to the OS audio API present on the machine (no bundled native lib). This is the ideal distributable.
- **OpenCV opt-in path (if retained):** dynamic-link system/prebuilt OpenCV; on Windows you must ship the
  matching `opencv_world*.dll` next to the exe. Vendoring OpenCV statically is possible but slow and
  brittle; not recommended.
- **Assets** (`mappings.json`, example images, and — if bundled — a default `.sf2`) must stop being
  repo-relative literals. Either embed them in the binary (`include_bytes!`/`include_str!` — trivial for
  `mappings.json`) or resolve them via a platform config/data dir (`directories` crate **[VERIFY]**). This is
  required for any installed/distributable artifact (§1.3 item 4).

### 3.4 What an END USER does, per OS

- **Today (with FluidSynth):** install Rust+OpenCV+libclang, build, install a virtual MIDI port, install
  FluidSynth, download an `.sf2`, launch FluidSynth bound to the port, then run the app. ~6 manual steps,
  several OS-specific. This is the current first-run reality and it is the core usability problem.
- **Target (pure-Rust default):** download a single prebuilt binary (or `cargo install`), run it, hear
  sound. Zero system setup. The `.sf2` ships embedded or is auto-located; if none is found the binary falls
  back to a tiny built-in default **[VERIFY rustysynth can take an embedded SF2 buffer]**.
- **Distributable artifacts (prioritized):**
  1. Documented one-command dev build per OS (floor).
  2. `cargo install --path .` producing a single binary (pure-Rust path).
  3. CI-built release binaries: a Linux `x86_64-unknown-linux-gnu` build and a Windows
     `x86_64-pc-windows-msvc` build via GitHub Actions (trivial *once the native deps are gone* — that is the
     whole point of §3.2). Optional `cargo-dist` **[VERIFY]** to generate installers/zips.

### 3.5 Recommendation (prioritized)

1. **Now / headless:** consolidate per-OS build docs + the two-profile model + embed `mappings.json`
   (no build needed to validate via compilation reasoning).
2. **Decide the OpenCV fork (§2.1) and the synth fork (§2.3).** These two decisions determine everything
   downstream. Recommended: pursue pure-Rust replacements for both, behind feature flags that *retain*
   OpenCV and external-MIDI as opt-in power-user paths.
3. **On a build-capable machine:** prove the pure-Rust analysis matches the OpenCV feature values closely
   enough that the music is unchanged (owner's ear gates this), and that `cpal`+`rustysynth` actually emits
   audio on Linux and Windows.
4. **Then:** stand up CI release binaries for both OSes.

---

## 4. Interface Architecture — Terminal + GUI Baked In From the Start

### 4.1 The seam: one engine, many front-ends

The non-negotiable principle: **CLI, TUI, and GUI must all be thin drivers over a single shared engine**,
never divergent copies of the pipeline. The prior `interactive-architecture.md` already specifies exactly
this with a `PipelineEngine` exposing `tick()` / `update_image()` / `inject_event()` / `current_state()`,
and explicitly says "main.rs becomes a thin CLI layer." WS-4 should adopt that engine as the seam and make
the front-ends siblings on top of it.

Refactor target (extract from `main.rs` — the modules underneath it are already stateless enough):

```
                     ┌───────────────────────────────────────────┐
   front-ends        │ CLI (clap)   TUI (ratatui)   GUI (egui)    │   ← thin drivers, no pipeline logic
                     └──────┬───────────┬───────────────┬─────────┘
                            │           │               │
                 control commands  +   subscribe to event/feature stream
                            │           │               │
                     ┌──────▼───────────▼───────────────▼─────────┐
   shared core       │  src/engine.rs : PipelineEngine             │
                     │   tick() / update_image() / inject_event()  │
                     │   current_state()  + event broadcast        │
                     └──────┬───────────────────────┬──────────────┘
                            │                        │
              ┌─────────────▼────────┐     ┌─────────▼─────────────┐
              │ analysis (image/     │     │ chord_engine (pure)   │
              │ image_analysis)      │     │ realize_step→NoteEvent│
              └──────────────────────┘     └─────────┬─────────────┘
                                                     │
                                          ┌──────────▼──────────────┐
              audio sink (behind trait):  │ rustysynth+cpal (default)│
                                          │  | midir external port   │
                                          └──────────────────────────┘
```

Proposed seam shapes (signatures only — no bodies, respecting module boundaries):

```rust
/// What every front-end reads. A snapshot the GUI/TUI render and the CLI can dump.
/// Carries music-domain + image-domain features already present in the code,
/// NOT raw OpenCV/image types (boundary preserved).
pub struct EngineSnapshot {
    pub scan_position: f32,                 // 0.0..=1.0 along the scan axis
    pub step_index: usize,
    pub global: GlobalFeaturesView,         // hue/sat/bright/edge_density/texture (plain scalars)
    pub current_step: PerfFeaturesView,     // = chord_engine::PerfFeatures projection
    pub last_notes: Vec<chord_engine::NoteEvent>,
    pub mode: String,
    pub phrase: chord_engine::PhrasePosition,
}

/// Commands any front-end can send the engine (CLI parses these from flags,
/// TUI/GUI from widgets/keys). One control vocabulary, three input surfaces.
pub enum EngineCommand {
    SetInstruments(usize),
    SetMsPerStep(u64),
    SetJitterPercent(f32),
    SetThickness(f32),
    LoadImage(ImageRef),         // path | preselected | camera | (future) generated
    Play, Pause, Stop, Seek(f32),
    SetAudioSink(SinkKind),      // BundledSynth | ExternalMidi(port_hint)
}

/// The subscription the GUI/TUI consume for reactive rendering AND reactive theming (§5).
/// A broadcast of per-tick snapshots; front-ends are pure observers.
pub trait EngineObserver {
    fn on_tick(&mut self, snapshot: &EngineSnapshot);
}

/// Transport-agnostic audio sink so the synth choice is a runtime decision, not a build fork.
pub trait AudioSink {
    fn send(&mut self, ev: &chord_engine::NoteEvent, channel: u8) -> anyhow::Result<()>;
    fn program_change(&mut self, channel: u8, program: u8) -> anyhow::Result<()>;
}
// midi_output::MidiOut already matches this shape; a rustysynth+cpal sink implements the same trait.
```

Note this does not violate boundaries: the engine subscribes front-ends to *feature/event streams* and
*NoteEvents*; it never hands a front-end an OpenCV `Mat` or lets the GUI reach into `chord_engine` internals.
The `PerfFeatures`/`NoteEvent` types already exist and are explicitly the image-free music-domain projection
(`chord_engine.rs:555–593`), so they are the correct currency for cross-boundary subscription.

### 4.2 CLI ergonomics (in scope NOW — the terminal interface is first-class)

Replace the hand-rolled `parse_cli_arg` with **`clap`** (derive API):

- Proper `--help`/`-h`, `--version`, subcommands (`play`, `render`, `analyze`, `modem encode/decode`),
  validated flags with ranges, helpful errors on bad input (vs today's silent `unwrap_or(default)`).
- Resolve the overloaded `"play"` token (§1.2) into a real `play` subcommand with a positional `<IMAGE>`.
- Add a **config file** (`audiohax.toml` via `serde` + `directories`-located path **[VERIFY]**) so the
  owner's preferred instrument count / tempo / theme / sink live somewhere persistent; CLI flags override the
  file. This directly serves a music-performer who wants repeatable setups.
- Replace `{:?}` feature dumps with formatted, labeled output (and a `--json` mode for tooling/tests).
- **Unify the modem bins' CLIs** under the same `clap` app as subcommands. Today `modem_encode`/`modem_decode`/
  `channel_sim` each hand-roll `print_usage` + positional parsing with subtly different conventions (e.g.
  `channel_sim` requires two positionals, `modem_decode` uses an optional `out_basename`). Folding them into
  `audiohax modem ...` gives one consistent help surface and one argument grammar. This is pure-Rust work,
  doable and testable headless.

### 4.3 TUI

A **`ratatui`** **[VERIFY]** terminal dashboard is a low-cost, high-value front-end and a natural stepping
stone to the GUI: it consumes the exact same `EngineSnapshot`/`EngineObserver` stream and renders feature
meters, the scan position, current chord/mode/phrase, and last-notes — proving the engine seam *before* any
GUI framework is committed. It needs no native deps and runs anywhere the CLI does, so it is buildable and
demoable on the dev box.

### 4.4 GUI framework survey (Linux + Windows; weigh reach, packaging, native-dep coexistence, aesthetics)

All version/maturity claims here are general-knowledge and marked **[VERIFY]** — confirm against current crate
docs before committing, per the brief.

- **egui / eframe** **[VERIFY]** — immediate-mode, pure-Rust, renders via wgpu/glow. *Reach:* excellent on
  Linux + Windows, single static binary, no system GUI toolkit to install. *Packaging:* trivial — pairs
  perfectly with the pure-Rust §3.2 posture. *Coexistence:* immediate-mode redraw maps naturally onto the
  per-tick `EngineSnapshot` push (you just draw the latest snapshot each frame) and onto reactive theming
  (recolor every frame from features — §5). *Aesthetic ceiling:* good and fully themeable (custom colors,
  fonts, visuals), though immediate-mode's look is more "clean instrument panel" than "designed app chrome."
  For a feature-meter / generative-visual interface this is a strength, not a limit.
- **iced** **[VERIFY]** — Elm-style retained/reactive, pure-Rust. *Reach:* good Linux + Windows. *Packaging:*
  good, mostly pure-Rust. *Coexistence:* its message/subscription model fits the engine's event stream well.
  *Aesthetic ceiling:* arguably cleaner default polish than egui; more ceremony for live, every-frame
  feature-driven recoloring.
- **slint** **[VERIFY]** — declarative `.slint` markup + Rust, strong designer-facing styling and animation.
  *Reach:* good Linux + Windows. *Packaging:* reasonable. *Aesthetic ceiling:* high (animation, theming are
  first-class) — the most "designed/polished" option, which speaks to the owner's aesthetic bar. *Cost:*
  separate markup language + a heavier learning/tooling surface; licensing terms should be checked **[VERIFY]**.
- **gtk-rs** **[VERIFY]** — bindings to native GTK. *Reach:* native on Linux; on Windows it requires shipping
  the GTK runtime — reintroduces exactly the kind of heavy native-dep packaging pain we are trying to delete.
  Not recommended given WS-4's cross-platform-simplicity goal.
- **tauri** **[VERIFY]** — web-frontend (HTML/CSS/JS) in a system webview + Rust backend. *Reach:* good.
  *Aesthetic ceiling:* highest (full web design language), and the natural bridge to the WS-3 game/web work.
  *Cost:* a JS/web toolchain enters the build; relies on the platform webview (WebView2 on Windows). Heavier
  than the project needs for a v1 instrument panel, but the strongest answer if the owner ultimately wants a
  showpiece visual or web/game convergence.

**Recommendation: egui/eframe for the WS-4 v1 GUI**, with the engine seam kept framework-agnostic so a later
re-skin to slint or tauri (for a high-polish showpiece) costs only a new front-end, not an engine rewrite.
Rationale: (1) it is the *only* option that adds essentially *zero* packaging burden on top of the pure-Rust
§3.2 posture — the headline cross-platform goal stays intact; (2) immediate-mode redraw is the most natural
fit for a per-tick, feature-reactive interface and for reactive theming (§5), which is the owner's headline
GUI idea; (3) it coexists cleanly with whatever remains of the native deps; (4) it gets a working,
themeable, real GUI in front of the owner fastest, which de-risks the aesthetic direction early. The
aesthetic ceiling is the honest trade — if the owner, after seeing egui, wants a more "designed" chrome,
slint/tauri are the upgrade path and the engine seam makes that upgrade cheap.

---

## 5. Reactive-Theming Design (the owner's idea)

Goal: GUI aesthetics driven **reactively** by either the **image features** (hue/sat/bright/edge density —
from `image_analysis.rs`) or the **music** (the `PerfFeatures`/`NoteEvent` stream from `chord_engine.rs`),
with a user-preference fallback to **static** themes. Design-level only: name the data flow and control
surface, not pixel values.

### 5.1 What the GUI subscribes to (already exists vs. new exposure)

The crucial finding: **the data the theming needs already flows through the engine** — it is the *same*
feature stream the music pipeline consumes. No new analysis is required; only *exposure*.

- **Image-driven source:** `GlobalFeatures`/`ScanBarFeatures` (`image_analysis.rs:11,22`) — `avg_hue` (0..360),
  `avg_saturation`/`avg_brightness` (0..100), `edge_density` (0..1), `texture_laplacian_var`. These are plain
  `f32`s — *not* OpenCV types — so they cross the GUI boundary cleanly. **Already exists**; the GUI just needs
  them surfaced in `EngineSnapshot` (`global` + `current_step`).
- **Music-driven source:** `chord_engine::PerfFeatures` (saturation→dynamic level, brightness→register,
  edge_density→rhythmic activity; `chord_engine.rs:555`) and the per-tick `Vec<NoteEvent>` (note/velocity/
  hold/offset) plus `mode` and `PhrasePosition`. **Already exists**; surface via `EngineSnapshot.last_notes`,
  `.mode`, `.phrase`, `.current_step`.
- **New exposure needed (small):** only the `EngineSnapshot` broadcast itself (§4.1) and a `ThemeSource`
  selector. No new feature extraction, no boundary violation — the GUI is a pure observer of streams the
  engine already produces.

### 5.2 The theming seam

```rust
/// Where the live theme comes from. Default is Static (a named palette).
pub enum ThemeSource {
    Static(ThemeId),     // user-chosen fixed palette — the fallback
    Image,               // drive from GlobalFeatures/ScanBarFeatures
    Music,               // drive from PerfFeatures / NoteEvents / phrase
}

/// The resolved theme the GUI applies each frame. Plain values; no engine internals.
pub struct ThemeParams {
    pub bg: (u8, u8, u8),
    pub accent: (u8, u8, u8),
    pub motion_intensity: f32,   // animation/pulse amount, 0..=1
    pub contrast: f32,           // 0..=1
}

/// Pure mapping from a snapshot to theme parameters. Lives in a NEW gui/theming
/// module (front-end side), reads only EngineSnapshot — never image_analysis or
/// chord_engine internals. Keeps the boundary intact.
pub trait Theming {
    fn resolve(&self, source: &ThemeSource, snap: &EngineSnapshot) -> ThemeParams;
}
```

### 5.3 Mapping concept (design intent, not pixel values)

- **Image source:** `avg_hue` → base palette hue; `avg_saturation` → palette saturation; `avg_brightness` →
  background lightness; `edge_density`/`texture` → `motion_intensity`/`contrast` (busy image → livelier,
  higher-contrast UI). This is deliberately parallel to the *musical* mapping (sat→dynamics, bright→register,
  edges→rhythm) so the screen and the sound move together — which is the artistic point.
- **Music source:** map the *same* `PerfFeatures` the music uses (so theme and music are provably coherent),
  plus accent the UI on note onsets / `PhrasePosition` (e.g. a gentle pulse on each `NoteEvent`, a settle at
  cadence). This makes the interface visibly "breathe" with the phrase structure already in `StepPlan`.
- **Static fallback:** `ThemeSource::Static(ThemeId)` selects a fixed palette from a small built-in set (and
  user-defined palettes in the config file, §4.2). This is the default so the GUI is calm/legible until the
  user opts into reactivity.

### 5.4 Smoothing (one honest caveat)

Per-tick features can jitter (the same reason the music pipeline uses hysteresis in
`interactive-architecture.md`). The theming layer should low-pass / ease theme parameters between frames so
colors glide rather than strobe. This belongs in the front-end `Theming` impl, not the engine, and keeps the
engine boundary clean. (egui's per-frame redraw makes this easing trivial — another point for egui in §4.4.)

---

## 6. Prioritized Roadmap (headless-now vs. build-gated)

The organizing insight from the brief: **the build/portability work is the unblock for the whole
engagement's hardware-gated threads.** WS-1's "hear the music" validation and WS-2's modem hardware
round-trip are both blocked today on the absence of a build-capable machine *and* on the OpenCV/FluidSynth
setup burden. Collapsing the native surface (§3.2) is therefore not just WS-4 hygiene — it is the lever that
makes WS-1 and WS-2 testable on an ordinary machine. Sequence accordingly.

### Phase 0 — Documentation & build hygiene (HEADLESS / NOW)
- Consolidate per-OS build docs; document the two-profile model (§3.1); add the missing
  `cmake`/`pkg-config`/`LIBCLANG_PATH` notes the dev-box failure proves are needed.
- Embed `mappings.json`; stop assuming repo-root CWD (§3.3).
- Deliverable validated by review + compilation reasoning. No build machine required.

### Phase 1 — CLI ergonomics + engine-seam DESIGN (HEADLESS / NOW)
- `clap` migration; real `--help`/`--version`/subcommands; resolve the `"play"` overload; config file;
  `--json`; unify the modem bins under one CLI grammar (§4.2). Pure-Rust → buildable/testable on the dev box.
- Finalize the `engine.rs` seam (`EngineSnapshot`/`EngineCommand`/`EngineObserver`/`AudioSink`/`Theming`),
  reconciled with the prior `interactive-architecture.md` `PipelineEngine`. Design + signatures only.

### Phase 2 — Dependency-portability work (MOSTLY HEADLESS; FINAL PROOF build-gated)
- Implement the pure-Rust analysis path behind a feature flag (port HSV/edge/texture/orientation off OpenCV
  onto `image`(+`imageproc`)) — *writing* and *compiling* this is largely doable on the dev box (pure Rust);
  what is **build-gated** is proving feature parity so the music doesn't drift (owner's ear).
- Implement the `AudioSink` trait + a `rustysynth`+`cpal` bundled sink; keep `midir` behind `midi-out`.
  Compiles headless; **emitting actual audio is build-gated.**
- Flip `default = []` (§3.2) once the above hold.

### Phase 3 — TUI front-end (HEADLESS / NOW once Phase 1 seam lands)
- `ratatui` dashboard over `EngineSnapshot` (§4.3). Proves the seam with zero native deps; runnable on the
  dev box. This is the cheapest way to *demonstrate* the shared-core architecture before any GUI commitment.

### Phase 4 — GUI implementation (BUILD-GATED; needs the UX/GUI specialist, §7)
- egui/eframe front-end over the same seam + the `Theming` layer (§5). Implementation and especially any
  visual/aesthetic validation are **build-gated** (needs a windowing-capable machine and the owner's eye).

### Phase 5 — Distributable artifacts (BUILD-GATED, CI)
- CI release binaries for Linux + Windows; `cargo install`; optional `cargo-dist`. Trivial *because* Phase 2
  removed the native deps. This is also when WS-1/WS-2 finally get a clean machine to run on (the unblock).

**Single highest-leverage first phase:** **Phase 1** (CLI ergonomics + the engine-seam design). It is fully
headless, it converts the worst day-one usability problem (the undiscoverable, self-contradictory CLI) into a
clean front-end, and — most importantly — it *establishes the shared core that every other WS-4 deliverable
(TUI, GUI, reactive theming) and the prior interactive-architecture plan all depend on.* Phase 2 is the
higher *cross-platform* lever, but it depends on the seam Phase 1 defines, so Phase 1 comes first.

---

## 7. The Future UX/GUI Specialist (define the target; do NOT write the prompt)

The specialist roster (`docs/agent-specialist-library.md`) has Rust Architect, Rust Implementer, Music Theory,
Signal Processing, Test Engineer, Quality Gate, Game-Integration — **no UX/GUI specialist.** One should be
fabricated when Phase 4 (GUI implementation) begins. Target definition:

- **Expertise profile:** cross-platform Rust GUI engineering on the **chosen framework (egui/eframe)** with
  the engine seam from §4.1; reactive/generative UI theming (mapping live feature streams → visual
  parameters, with easing/smoothing — §5.4); color theory and palette design adequate to a
  music-performer owner's aesthetic bar; basic accessibility (contrast minimums, colorblind-safe palettes,
  keyboard navigation, not relying on color alone to convey state); and enough audio-visual-sync sensibility
  to make the UI "breathe" with the phrase structure.
- **First deliverable:** an egui front-end that subscribes to `EngineSnapshot` and renders the live
  feature/notes/scan-position dashboard with a *working* `ThemeSource` switch (Static / Image / Music) —
  i.e. the §5 theming seam made real on the smallest meaningful surface. Not the full instrument UI; the
  proof that reactive theming over the shared core works end-to-end.
- **File-ownership boundary (relative to Architect/Implementer):**
  - OWNS: `src/gui/**` (or `src/bin/audiohax-gui.rs` + a `gui` module) and a `gui/theming` module; GUI-only
    assets.
  - READS: `engine.rs` public surface, `chord_engine` public types (`PerfFeatures`/`NoteEvent`/`PhrasePosition`),
    the `EngineSnapshot` view types.
  - EXCLUDES: never modifies `image_analysis.rs`, `chord_engine.rs`, `modem.rs`, or `engine.rs` internals —
    if the GUI needs new data, it requests a new field on `EngineSnapshot` through the **Architect**, who
    designs the exposure, and the **Implementer**, who lands it in the engine. This preserves the
    "front-ends are pure observers" rule (§4.1) and the existing module-boundary discipline.
- Per the swarm's routing rules, the Architect specifies the seam; the GUI specialist designs/implements
  *within* the front-end boundary against that seam.

---

## 8. Risks & Trade-Offs (the honest list)

1. **OpenCV-on-Windows is the marquee risk if OpenCV is retained.** Two heavyweight native SDKs (OpenCV +
   libclang) must align by version and be discoverable via env vars; this is the most common "it won't
   build" failure and there is no clean static story. *Mitigation:* the pure-Rust analysis fork (§2.1, §3.2)
   removes it entirely — at the cost of re-validating feature parity so the music doesn't change.
2. **Feature-parity drift when porting off OpenCV.** A reimplemented Canny/Laplacian/contour metric will not
   produce byte-identical values; because those values feed mode/dynamics/register, the *music can shift*.
   The owner's professional ear is the only adequate gate. *Mitigation:* keep OpenCV as an opt-in reference
   path and A/B the two analyses on the same images before flipping the default.
3. **FluidSynth/virtual-MIDI is the marquee *runtime* risk.** The current "hear nothing until you wire up
   three things" is the worst first-run experience. *Mitigation:* in-process `rustysynth`+`cpal` default
   (§2.3) — but this trades away "use my own DAW/synth" unless `midir` is retained as an optional sink (it
   should be).
4. **The shared-core refactor has real cost.** `main.rs` currently fuses CLI parsing, OpenCV windowing, the
   batch loop, the barrier/worker concurrency, and MIDI scheduling. Extracting a clean `engine.rs` without
   regressing the working playback path is non-trivial; the prior `interactive-architecture.md` flags the
   same batch→reactive structural change as "not a feature addition." *Mitigation:* land the seam first as a
   pure refactor that preserves today's batch behavior (CLI front-end only), *then* add TUI/GUI/reactive
   modes on top — never two pipelines at once.
5. **GUI framework lock-in & aesthetic ceiling.** egui is the pragmatic, lowest-packaging-burden choice but
   its immediate-mode look may not satisfy the owner's polish bar long-term. *Mitigation:* keep the engine
   seam framework-agnostic so a slint/tauri re-skin is a new front-end, not an engine rewrite; get egui in
   front of the owner early (Phase 4) to surface this before it's expensive.
6. **`cpal`/audio backend variance.** Cross-platform audio is real but the device/sample-rate negotiation
   differs per backend (ALSA/PulseAudio/JACK vs WASAPI vs CoreAudio) and can surprise on Linux audio servers.
   *Mitigation:* validate `cpal` output on an actual Linux *and* Windows box early in Phase 2; this is one of
   the genuinely build-gated unknowns.
7. **No confirmed build-capable machine is the meta-risk.** Every "run it and listen/look" validation in
   WS-1, WS-2, and WS-4 Phases 4–5 is blocked until one exists. This assessment's sequencing front-loads all
   headless-validatable work (Phases 0–1, much of 2, and 3) precisely so progress is real before the machine
   appears — and so that *when* it appears, the pure-Rust posture means it needs almost no setup to become
   the build/validation host for the whole engagement.

---

## Appendix — Grounding references (file:line)

- `Cargo.toml`: `default = ["opencv","midir","image"]`; optional deps; `[[bin]]` `required-features`.
- `Cargo.lock`: `opencv` → `opencv-binding-generator` + `clang-sys`(→`clang`) + `pkg-config`; `midir` →
  `alsa`/`alsa-sys`/`coremidi`/`winapi`; `image` → `jpeg-decoder`; **no FluidSynth entry**.
- `src/main.rs`: `parse_cli_arg` 39–45; overloaded `"play"` 345–354 + 447; batch flow 323–468;
  `play_scanned_steps_concurrent` 99–321 (barrier/worker pool, MIDI scheduling, `highgui` window).
- `src/midi_output.rs`: `MidiOut::open_first` 11–31 (errors with virtual-port instruction); raw MIDI sends.
- `src/image_source.rs`: OpenCV `imread`/camera; `AIGenerated` is an unimplemented placeholder.
- `src/image_analysis.rs`: `GlobalFeatures`/`ScanBarFeatures`/`LocalFeatures` 11–46 (plain `f32` features —
  the GUI/theming data source); HSV/Canny/Laplacian/Sobel/contours/hue-histogram.
- `src/chord_engine.rs`: `PerfFeatures` 565–573, `NoteEvent` 583–593, `StepPlan` 240–256,
  `PhrasePosition` 219–230, `realize_step` 642 (the image-free music-domain seam).
- `README.md`: per-OS install (OpenCV/libclang/ALSA/FluidSynth/virtual-MIDI) — confirms FluidSynth + virtual
  port are external, hand-wired runtime steps.
- Sibling design-repo `docs/interactive-architecture.md`: prior `PipelineEngine` (`tick`/`update_image`/
  `inject_event`/`current_state`) + batch→reactive refactor (consistent with §4 here).
- Sibling design-repo `docs/ROADMAP.md`: music-quality Phase 1 (done through S6), modem hardening Phase 3,
  interactive track as concept — WS-4 is the cross-platform/UX layer those phases never covered.
- Sibling design-repo `docs/agent-specialist-library.md`: roster has no UX/GUI specialist (basis for §7).

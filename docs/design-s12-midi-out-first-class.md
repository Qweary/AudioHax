# Design S12 — First-Class MIDI Output

**Lane:** WS-4 output plumbing. **Status:** DESIGN ONLY (no source modified).
**Scope:** make the external-MIDI path a first-class, runtime-selectable output so the
owner can route AudioHax into a real engine (DAW / FluidSynth / Qsynth / hardware) and
judge the music through proper synths + effects (reverb/chorus).

**Out of scope (explicitly):** any musical decision, the image→music mapping, the
"every image sounds the same" diversity problem (a separate future lane). This lane is
*output plumbing only*. It is noted only in passing that routing to a real engine is the
prerequisite that *lets* the owner judge the musical output — that judgement, and the
diversity fix it motivates, are not designed here.

**Seam invariant:** `src/engine.rs` stays **BYTE-UNCHANGED**. All runtime sink selection
lives in the `main.rs` adapter. `SynthSink` (`src/synth_sink.rs`) is **constructed**
alongside `MidiOut`, never modified.

---

## 1. CURRENT STATE ANALYSIS

### 1.1 The output seam (`src/engine.rs`) — frozen, quoted for reference only

```rust
// src/engine.rs:102
pub struct AudioSinkError(pub Box<dyn std::error::Error + Send + Sync + 'static>);
// src/engine.rs:123
impl AudioSinkError { pub fn msg(m: impl Into<String>) -> Self { /* … */ } }
// src/engine.rs:132
pub trait AudioSink {
    fn note_on(&mut self, channel: u8, note: u8, velocity: u8) -> Result<(), AudioSinkError>;     // :134
    fn note_off(&mut self, channel: u8, note: u8) -> Result<(), AudioSinkError>;                  // :136
    fn program_change(&mut self, channel: u8, program: u8) -> Result<(), AudioSinkError>;         // :138
}
```

Both sinks already implement this trait. The engine is driven entirely through
`Box<dyn AudioSink>` (`main.rs:374`/`:388`/`:464`/`:467`). **Nothing in this design
touches the trait, its methods, or any engine type.** The selection logic that picks the
concrete sink is an *adapter* concern and already lives in `main.rs` — today behind
`#[cfg]`, tomorrow behind a runtime branch.

### 1.2 Compile-time sink selection (`src/main.rs`) — the thing we are replacing

Today the sink is chosen at **compile time** by the `midi-out` feature:

```rust
// src/main.rs:373  (CURRENT — external MIDI path, only present when midi-out is ON)
#[cfg(feature = "midi-out")]
let mut sink: Box<dyn AudioSink> = {
    let preferred = std::env::var("AUDIOHAX_MIDI_PORT").ok();
    let preferred_ref = play_args.midi_port.as_deref()
        .or(preferred.as_deref())
        .or(Some("AudioHaxOut"));
    println!("Opening external MIDI port (preferred = {:?})...", preferred_ref);
    let midi = MidiOut::open_first(preferred_ref)?;
    Box::new(midi)
};

// src/main.rs:387  (CURRENT — in-process synth, only present when midi-out is OFF)
#[cfg(not(feature = "midi-out"))]
let mut sink: Box<dyn AudioSink> = {
    let synth = audiohax::synth_sink::SynthSink::with_bundled_soundfont()?;
    Box::new(synth)
};
```

Consequences of the compile-time scheme:
- The two sinks are **mutually exclusive in a build**. `cargo run` (default features) can
  *never* reach `MidiOut` — the owner must rebuild with `--features midi-out` to route
  externally, and that build then **loses** the in-process synth. There is no single
  binary that can do both.
- The MIDI sink module and its `impl AudioSink for MidiOut` are themselves `#[cfg]`-gated
  (`main.rs:39-40`, `:60-64`, `:84-97`).
- The `--midi-port` hint is wired (`main.rs:376`) but only consumed in the gated build.

### 1.3 `MidiOut::open_first` behavior (`src/midi_output.rs`)

```rust
// src/midi_output.rs:6
pub struct MidiOut { conn: MidiOutputConnection }

// src/midi_output.rs:11
impl MidiOut {
    pub fn open_first(port_name_hint: Option<&str>) -> Result<Self, Box<dyn Error>> {
        let midi_out = MidiOutput::new("acoustic_art_out")?;
        let ports = midi_out.ports();
        if ports.is_empty() {
            return Err("No MIDI output ports available. Create a virtual MIDI port \
                        (loopMIDI/IAC/virtual) and run FluidSynth/Qsynth.".into());
        }
        // first port whose name .contains(hint), else ports[0]
        // … midi_out.connect(&port, "acoustic_art_conn")
    }
    // program_change / note_on / note_off / play_chord_arpeggio …
}
```

Current limitations this lane fixes:
- **Connects to the FIRST existing port** (or first name-substring match). It does **not**
  list ports for the user, so the user cannot see what is available to pick.
- The hint is **substring only** — no index selection.
- On *no ports*, it errors with advice but offers **no smooth path**: it cannot itself
  **create a virtual port** that a DAW/Qsynth subscribes to, even though midir supports
  exactly that on Linux/macOS (`MidiOutput::create_virtual` — verified §6.3).
- `Result<…, Box<dyn Error>>` (not `Send + Sync`); the adapter already stringifies this
  into `AudioSinkError` at `main.rs:84-97` and that pattern is preserved.

### 1.4 CLI grammar (`src/cli.rs`) — lib, headless unit-testable

```rust
// src/cli.rs:13
use clap::{Args, Parser, Subcommand, ValueEnum};

// src/cli.rs:104
#[derive(Debug, Args, PartialEq)]
pub struct PlayArgs {
    pub image: Option<PathBuf>,                 // :108  positional
    #[arg(long)]
    pub midi_port: Option<String>,              // :110-111  hint (substring only today)
    #[command(flatten)]
    pub pipeline: PipelineArgs,                 // :112-113
}
```

The grammar is pure-Rust and lib-testable — no `opencv` / `image` / `midir` type appears
in `cli.rs`. The existing test mod (`cli.rs:578`) drives `Cli::try_parse_from([...])`
(e.g. `:614`, `:626`, `:647`), which is the pattern §6.6 reuses to keep the new args
unit-testable even though the live MIDI calls are not.

### 1.5 Feature graph (`Cargo.toml`)

```toml
default      = ["pure-analysis", "synth"]              # :37
pure-analysis = ["image", "dep:imageproc"]            # :42
synth        = ["dep:rustysynth", "dep:cpal", "dep:rtrb"]  # :47
opencv       = ["dep:opencv", "image"]                # :52  (opt-in)
midi-out     = ["dep:midir"]                          # :56  (opt-in)
# …
midir = { version = "0.8", optional = true }          # :66
```

There is **no `default-run` key** in `[package]` (`Cargo.toml:1-4`), so `cargo run` is
ambiguous across the 8 bins — the known papercut §4 fixes.

---

## 2. PROPOSED CHANGES (per file)

### 2.1 `Cargo.toml`

**(a) Promote `midi-out` into the default feature set.**

```toml
# BEFORE
default = ["pure-analysis", "synth"]                  # :37
# AFTER
default = ["pure-analysis", "synth", "midi-out"]
```

`midi-out = ["dep:midir"]` (`:56`) is **unchanged** — only `default` gains it. midir
becomes an always-compiled dependency of the default binary.

**Rationale / trade (the key decision in §RISKS):** runtime sink selection requires
*both* sinks compiled into one binary, which requires midir in the default build. Weight
is small and the platform prerequisites are already met:
- **Linux:** midir's ALSA backend needs `libasound2-dev` — *the exact same* prereq cpal
  already imposes (`Cargo.toml:35-36`). No new system dependency.
- **Windows / macOS:** midir builds clean with no extra system libs (WinMM / CoreMIDI are
  OS-provided).
- midir is a small, pure-ish crate (one `memalloc` dep); negligible compile-time cost vs.
  rustysynth+cpal already in `default`.

A lighter alternative — keep `midi-out` opt-in and instead make selection runtime *only
when the feature is on* — is **rejected**: it does not deliver "first-class" (the owner
would still rebuild to get MIDI, and would still lose the synth in that build). First-class
means the *shipped default binary* offers `--output midi`. The cost (midir always
compiled) is the price of that, and it is cheap. **DECISION POINT for the lead** — see §6.1.

**(b) `default-run` papercut.**

```toml
# [package] (Cargo.toml:1-4) — ADD:
default-run = "audiohax"
```

So `cargo run` (no `--bin`) unambiguously runs the image-to-music app. The 7 other bins
remain reachable via `cargo run --bin <name>`. Pure metadata; affects nothing at runtime.

**(c) No change to `midir`'s version line.** Verified (§6.3): midir 0.8's virtual-port
API is exposed via the `midir::os::unix::VirtualOutput` trait and is gated by
`#[cfg(unix)]` **inside midir** — there is **no `virtual` cargo feature** in midir 0.8.
So `midir = { version = "0.8", optional = true }` is sufficient as-is; nothing to add to
the feature list. (This directly answers the "midir may need a `virtual` feature — verify"
note: in 0.8 it does not.)

### 2.2 `src/cli.rs` (lib — the headless-testable parser)

**(a) New `ValueEnum` for the output sink.**

```rust
/// Runtime output sink for `audiohax play`. Replaces the S11 compile-time
/// `#[cfg(feature="midi-out")]` sink choice; both sinks are now compiled into the
/// default binary and selected here at parse time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
pub enum OutputSink {
    /// In-process pure-Rust SoundFont synth (rustysynth + cpal). Dry; the default.
    #[default]
    Synth,
    /// Route NoteEvents to an external MIDI port / DAW / FluidSynth / Qsynth.
    Midi,
}
```

`ValueEnum` is already imported (`cli.rs:13`). clap renders the variants as `synth` /
`midi` (kebab-cased automatically), matching the prompt's `--output synth|midi`.

**(b) Extend `PlayArgs`.** Before → after:

```rust
// BEFORE  (cli.rs:104-114)
#[derive(Debug, Args, PartialEq)]
pub struct PlayArgs {
    pub image: Option<PathBuf>,
    #[arg(long)]
    pub midi_port: Option<String>,
    #[command(flatten)]
    pub pipeline: PipelineArgs,
}

// AFTER
#[derive(Debug, Args, PartialEq)]
pub struct PlayArgs {
    /// Image path. Omit to use the example image.
    pub image: Option<PathBuf>,

    /// Output sink: `synth` (in-process, default) or `midi` (external port/DAW).
    #[arg(long, value_enum, default_value_t = OutputSink::Synth)]
    pub output: OutputSink,

    /// MIDI port selector (only with `--output midi`): a NAME SUBSTRING or a numeric
    /// INDEX (as shown by `--list-midi-ports`). Else `$AUDIOHAX_MIDI_PORT` or `"AudioHaxOut"`.
    #[arg(long)]
    pub midi_port: Option<String>,

    /// List available MIDI output ports (index + name) and exit. Implies no playback.
    #[arg(long)]
    pub list_midi_ports: bool,

    /// Create a virtual MIDI output port that a DAW/Qsynth can subscribe to, instead of
    /// connecting to an existing port. Optional value sets the port name
    /// (default "AudioHaxOut"). Unix only (Linux ALSA / macOS CoreMIDI); on Windows this
    /// errors with guidance to use loopMIDI. Forces `--output midi`.
    #[arg(long, value_name = "NAME", num_args = 0..=1, default_missing_value = "AudioHaxOut")]
    pub midi_virtual: Option<String>,

    #[command(flatten)]
    pub pipeline: PipelineArgs,
}
```

Notes on the clap derive choices:
- `--output` defaults to `synth`, preserving today's no-flag behavior (in-process synth).
- `--midi-port` is **kept** (back-compat) but its *meaning* widens to substring-OR-index;
  the parsing of "is this an index or a substring" is done in `MidiOut` (a runtime MIDI
  concern), not in clap — clap just carries the `Option<String>`.
- `--midi-virtual` uses `num_args = 0..=1` + `default_missing_value` so bare `--midi-virtual`
  yields `Some("AudioHaxOut")` and `--midi-virtual MyPort` yields `Some("MyPort")`; omission
  yields `None`. This is the standard clap idiom for "optional flag with optional value".
- All four additions are **pure data on `PlayArgs`** — they parse and `PartialEq`-compare in
  the existing lib test harness with zero MIDI subsystem present (§6.6).

**(c) No change to `pipeline_to_engine_config`** (`cli.rs:137`) — output selection is not an
engine-config concern; the engine never learns which sink it drives (correct — the seam).

### 2.3 `src/midi_output.rs` — extend the `MidiOut` API

Three additions: a port lister, an index-or-substring open, and a virtual-port constructor.
`open_first` is **kept** (back-compat / fallback). New public surface:

```rust
use midir::{MidiOutput, MidiOutputConnection};
#[cfg(unix)]
use midir::os::unix::VirtualOutput;   // brings `create_virtual` into scope on unix
use std::error::Error;

pub struct MidiOut { conn: MidiOutputConnection }   // UNCHANGED field

impl MidiOut {
    // ── existing (UNCHANGED) ──────────────────────────────────────────────
    pub fn open_first(port_name_hint: Option<&str>) -> Result<Self, Box<dyn Error>>;

    // ── NEW: enumerate available output ports as (index, name) ────────────
    /// List the available MIDI output ports as `(index, name)` pairs, in the order
    /// midir reports them (the index is the selector accepted by `open_selector`).
    /// Opens a transient `MidiOutput` to query; does not connect. Static (no `&self`).
    pub fn list_ports() -> Result<Vec<(usize, String)>, Box<dyn Error>>;

    // ── NEW: open an EXISTING port by NAME SUBSTRING or numeric INDEX ─────
    /// Connect to an existing output port chosen by `selector`:
    ///   * if `selector` parses as a `usize`, treat it as a 0-based port index;
    ///   * otherwise treat it as a case-sensitive name substring (as `open_first`);
    ///   * if `selector` is `None`, fall back to the first port.
    /// Errors (with the same actionable message family as `open_first`) when no ports
    /// exist or the index/substring matches nothing.
    pub fn open_selector(selector: Option<&str>) -> Result<Self, Box<dyn Error>>;

    // ── NEW: CREATE a virtual output port (the smooth path) ───────────────
    /// Create a virtual MIDI output port named `name` that a DAW / Qsynth / FluidSynth
    /// can subscribe to — no pre-existing port required. Supported on Linux (ALSA) and
    /// macOS (CoreMIDI). On Windows this is a compile-time-absent path; callers reach
    /// `open_virtual_unsupported` instead (see cfg split below).
    #[cfg(unix)]
    pub fn open_virtual(name: &str) -> Result<Self, Box<dyn Error>>;

    /// Windows stub: virtual ports are not supported by midir on Windows. Returns an
    /// actionable error pointing the user at loopMIDI (+ `--midi-port`). Exists so the
    /// main.rs branch is identical across platforms and only the body differs by cfg.
    #[cfg(not(unix))]
    pub fn open_virtual(name: &str) -> Result<Self, Box<dyn Error>>;

    // program_change / note_on / note_off / play_chord_arpeggio — UNCHANGED
}
```

Body-shape notes (signatures only — no bodies authored here, per constraint):
- `list_ports`: `MidiOutput::new("acoustic_art_out")?.ports()` then `.iter().enumerate()`
  mapping each `port` through `port_name(port)?` → `(i, name)`.
- `open_selector`: `selector.and_then(|s| s.parse::<usize>().ok())` picks the index branch;
  else substring (today's loop); else `ports[0]`. Same empty-ports error as `open_first`.
- `open_virtual` (`#[cfg(unix)]`): `MidiOutput::new(...)?.create_virtual(name)?` → that
  returns a `MidiOutputConnection` **directly** (the `MidiOutput` is consumed). Wrap into
  `MidiOut { conn }`. Requires the `use midir::os::unix::VirtualOutput;` trait import above.
- `open_virtual` (`#[cfg(not(unix))]`): returns
  `Err("Virtual MIDI ports are not supported on Windows. Install loopMIDI, create a port, \
   then run with `--output midi --midi-port <loopMIDI port>`.".into())`.

`open_first` may stay as-is or be re-expressed as `open_selector(hint)`; the migration
keeps it for safety and lets `open_selector` supersede it. **No other method changes.**

### 2.4 `src/main.rs` — runtime selection branch (adapter)

**(a) Make the MIDI module + `impl AudioSink for MidiOut` unconditional.** Because
`midi-out` is now in `default`, the gates at `main.rs:39-40`, `:60-64`, `:84-97` change from
`#[cfg(feature = "midi-out")]` to unconditional (the module always compiles). The
`impl AudioSink for MidiOut` (`:84-97`) and its `AudioSinkError` import lose their `#[cfg]`.

> Edge case to preserve: today `AudioSinkError` is imported once for the opencv path
> (`main.rs:48-49`) and again, conditionally, for the midi path when opencv is off
> (`main.rs:62-64`). After this change, make that import unconditional (the `MidiOut` impl
> always needs it) and ensure it is imported exactly once regardless of the `opencv` flag.

**(b) Replace the two `#[cfg]` sink blocks (`main.rs:373-393`) with one runtime branch.**

Before (compile-time, abbreviated — see §1.2). After (runtime):

```rust
use audiohax::cli::OutputSink;

// `--list-midi-ports` short-circuits BEFORE any image work would matter; placed right
// after argv parse in practice (see ordering note). Print and exit.
if play_args.list_midi_ports {
    match MidiOut::list_ports() {
        Ok(ports) if !ports.is_empty() => {
            println!("Available MIDI output ports:");
            for (i, name) in ports { println!("  [{i}] {name}"); }
        }
        Ok(_)  => println!("No MIDI output ports found. Use `--midi-virtual` (Linux/macOS) \
                            or create one (loopMIDI/IAC/Qsynth)."),
        Err(e) => eprintln!("Could not enumerate MIDI ports: {e}"),
    }
    return Ok(());
}

// Effective output: `--midi-virtual` forces midi; otherwise honor `--output`.
let want_midi = matches!(play_args.output, OutputSink::Midi) || play_args.midi_virtual.is_some();

let mut sink: Box<dyn AudioSink> = if want_midi {
    let midi = if let Some(vname) = play_args.midi_virtual.as_deref() {
        println!("Creating virtual MIDI output port '{vname}' (subscribe from your DAW/Qsynth)...");
        MidiOut::open_virtual(vname)?
    } else {
        let env_port = std::env::var("AUDIOHAX_MIDI_PORT").ok();
        let selector = play_args.midi_port.as_deref().or(env_port.as_deref());
        println!("Connecting to external MIDI port (selector = {selector:?})...");
        MidiOut::open_selector(selector)?     // index OR substring OR first
    };
    Box::new(midi)
} else {
    println!("Starting in-process synth (rustysynth + cpal, bundled SoundFont)...");
    let synth = audiohax::synth_sink::SynthSink::with_bundled_soundfont()?;
    println!("Synth audio stream started @ {} Hz.", synth.sample_rate());
    Box::new(synth)
};
```

**Ordering note (migration detail):** `--list-midi-ports` should short-circuit **before**
mappings/image load (it is a query, not a playback). The cleanest placement is immediately
after `let play_args = …` (around `main.rs:257`), so it exits before `load_mappings`
(`:260`). The sink-construction branch stays where the old `#[cfg]` blocks were
(`:373`), after `step_count()==0` guard. Both are pure adapter edits.

**(c) The driver loop (`main.rs:396-470`) is UNCHANGED** — it speaks only `Box<dyn AudioSink>`.
`SynthSink` is *constructed* (`:391`), never modified. The `--midi-port` selector default
`"AudioHaxOut"` is now only meaningful for the synth-less `open_selector` fallback; the
virtual default name lives on `--midi-virtual` instead.

### 2.5 New doc — `docs/midi-routing.md`

Short operator-facing routing recipe (content outline; the Implementer authors prose):

1. **Why** — the in-process synth is dry; route MIDI into a real engine for reverb/chorus
   and the owner's real synths, to *judge the music* (one sentence; not a music-design doc).
2. **List ports:** `cargo run -- play --list-midi-ports`.
3. **Linux — virtual port (smooth path):**
   `cargo run -- play <img> --midi-virtual` → start Qsynth/FluidSynth with a SoundFont +
   reverb/chorus enabled → in Qsynth's connections (or `aconnect -l` / `aconnect <AudioHaxOut> <FLUID>`)
   subscribe FluidSynth to `AudioHaxOut`. Include the bare `fluidsynth -a alsa -o synth.reverb.active=1
   -o synth.chorus.active=1 soundfont.sf2` form and the `aconnect` wiring.
4. **Linux — existing port:** start Qsynth first, then
   `cargo run -- play <img> --output midi --midi-port FLUID` (substring) or `--midi-port 0` (index).
5. **Windows — loopMIDI / DAW:** install loopMIDI, create a port, point the DAW's MIDI-in
   at it, then `cargo run -- play <img> --output midi --midi-port loopMIDI` (no `--midi-virtual`
   on Windows — it errors with this same guidance).
6. **macOS — IAC / virtual:** `--midi-virtual` works (CoreMIDI); or enable the IAC Driver
   and use `--midi-port IAC`.

A README "External MIDI / routing into a DAW" section may link to this doc instead of
duplicating it.

---

## 3. INTERFACE DEFINITIONS (precise code blocks)

### 3.1 New `MidiOut` public API (`src/midi_output.rs`)

```rust
impl MidiOut {
    pub fn open_first(port_name_hint: Option<&str>) -> Result<Self, Box<dyn Error>>;     // existing, kept
    pub fn list_ports() -> Result<Vec<(usize, String)>, Box<dyn Error>>;                 // NEW
    pub fn open_selector(selector: Option<&str>) -> Result<Self, Box<dyn Error>>;        // NEW (index|substring|first)
    #[cfg(unix)]      pub fn open_virtual(name: &str) -> Result<Self, Box<dyn Error>>;   // NEW (ALSA/CoreMIDI)
    #[cfg(not(unix))] pub fn open_virtual(name: &str) -> Result<Self, Box<dyn Error>>;   // NEW (Windows: actionable error)
}
```

### 3.2 New CLI args (clap-derive, `src/cli.rs`)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
pub enum OutputSink { #[default] Synth, Midi }

// within PlayArgs:
#[arg(long, value_enum, default_value_t = OutputSink::Synth)]
pub output: OutputSink,

#[arg(long)]
pub midi_port: Option<String>,                  // substring OR index

#[arg(long)]
pub list_midi_ports: bool,                      // print ports + exit

#[arg(long, value_name = "NAME", num_args = 0..=1, default_missing_value = "AudioHaxOut")]
pub midi_virtual: Option<String>,               // create virtual port (unix); forces midi
```

### 3.3 `main.rs` adapter selection (signature-level)

```rust
// query short-circuit
if play_args.list_midi_ports { /* MidiOut::list_ports() → print → return Ok(()) */ }

// runtime selection
let want_midi = matches!(play_args.output, OutputSink::Midi) || play_args.midi_virtual.is_some();
let mut sink: Box<dyn AudioSink> =
    if want_midi { Box::new(/* MidiOut::open_virtual | open_selector */) }
    else         { Box::new(/* SynthSink::with_bundled_soundfont */) };
```

---

## 4. DATA FLOW DIAGRAM

```
                         argv ──▶ audiohax::cli (Cli::parse)
                                   PlayArgs{ output, midi_port, midi_virtual, list_midi_ports }
                                          │
                          ┌───────────────┴────────────────┐
              list_midi_ports?                       want_midi = output==Midi
              │ yes → MidiOut::list_ports()              || midi_virtual.is_some()
              │       print "[i] name" → exit                     │
              ▼                                  ┌────────────────┴───────────────┐
            (exit)                          want_midi = false              want_midi = true
                                                  │                               │
                                                  ▼                               ▼
   ┌─────────────────────────── ADAPTER (src/main.rs) ──────────────────────────────────┐
   │   PipelineEngine::decide_step()  ──▶  Box<dyn AudioSink>  (engine.rs seam: FROZEN)  │
   └───────────────┬──────────────────────────────────────────────┬────────────────────┘
                   │                                               │
        OutputSink::Synth                                  OutputSink::Midi
                   │                                               │
                   ▼                                ┌──────────────┴───────────────┐
            SynthSink (S11)                  midi_virtual = Some(name)      midi_virtual = None
        rustysynth + cpal + rtrb                    │                              │
                   │                          MidiOut::open_virtual(name)   MidiOut::open_selector(sel)
                   ▼                          #[cfg(unix)] create_virtual   index | substring | first
            cpal → ALSA/WASAPI/CoreAudio              │                              │
                   │                                  └──────────────┬───────────────┘
                   ▼                                                 ▼
            DRY local audio                                  midir → MIDI port
                                                                     │
                                    ┌────────────────────────────────┼─────────────────────┐
                              virtual port (unix)            existing port            (Windows: virtual
                              DAW/Qsynth subscribes       Qsynth/loopMIDI/DAW          → actionable error)
                                    └──────────┬─────────────────────┘
                                               ▼
                                External engine + EFFECTS (reverb / chorus / owner's synths)
                                               ▼
                                       music the owner can JUDGE
```

---

## 5. MIGRATION PATH

**Single coherent Implementer lane — NOT a fan-out.** `cli.rs`, `main.rs`,
`midi_output.rs`, `Cargo.toml`, and `docs/midi-routing.md` change **together** and depend
on each other: the `OutputSink` enum (`cli.rs`) is named by `main.rs`; `main.rs` calls the
new `MidiOut` constructors (`midi_output.rs`); both require `midi-out` in `default`
(`Cargo.toml`). These files are **not disjoint**, so this is **ONE Implementer**, not a
parallel split. The doc is the only file with no code coupling and could trail, but it is
small and belongs in the same commit.

Suggested implementation order (all within the one lane):
1. `Cargo.toml`: `default += midi-out`, add `default-run = "audiohax"`. Build the default
   binary — confirms midir compiles in default (ALSA present).
2. `cli.rs`: add `OutputSink` + the four `PlayArgs` fields; add/extend unit tests
   (`Cli::try_parse_from`) for `--output midi`, `--list-midi-ports`, `--midi-virtual`
   (bare + valued), and `--midi-port 2` vs `--midi-port FLUID`. Lib builds/tests headless.
3. `midi_output.rs`: add `list_ports`, `open_selector`, `open_virtual` (unix + windows
   cfg) + the `#[cfg(unix)] use midir::os::unix::VirtualOutput;` import.
4. `main.rs`: drop the `#[cfg(feature="midi-out")]` gates on the module/import/impl; add the
   `--list-midi-ports` short-circuit; replace the two `#[cfg]` sink blocks with the runtime
   branch.
5. `docs/midi-routing.md` + README link.

**No break to the default pure-Rust play path:** `--output` defaults to `synth`, so
`cargo run -- play <img>` behaves byte-for-byte as today (in-process `SynthSink`). The
`opencv` feature is orthogonal and unaffected.

**Independent vs coordinated:**
- *Coordinated* (must land together): the 4 code/TOML files above.
- *Independent* (can trail in the same lane): the routing doc; the `default-run` key (pure
  papercut, could even precede everything).

**OUT-OF-SCOPE — the Implementer must NOT touch these files:**
`src/engine.rs`, `src/chord_engine.rs`, `src/mapping_loader.rs`, `assets/mappings.json`,
`src/pure_analysis.rs`, `src/synth_sink.rs`, `src/modem.rs`, `src/bin/modem_*`, `src/tui.rs`.
`SynthSink` is *constructed* only; `engine.rs` stays **byte-unchanged**.

---

## 6. RISKS & TRADE-OFFS / DECISION POINTS

### 6.1 DECISION POINT — `midi-out` in `default` (recommended: YES)
Making selection runtime *requires* both sinks in one binary, hence midir in `default`.
- **Cost:** midir always compiled into the app binary. On Linux it shares cpal's
  `libasound2-dev` prereq (no *new* system dep); Windows/macOS need nothing extra; compile
  weight is negligible next to rustysynth+cpal.
- **Benefit:** the *shipped default binary* offers `--output midi` — the definition of
  "first-class". The opt-in alternative cannot deliver this without a rebuild.
- **Recommendation:** add `midi-out` to `default`. *Lead's call before Implementer spawn.*

### 6.2 DECISION POINT — Windows virtual-port handling (recommended: cfg stub + loopMIDI)
midir has **no** `create_virtual` on Windows. Design: `open_virtual` is split by
`#[cfg(unix)]` / `#[cfg(not(unix))]`; the Windows arm returns an actionable error pointing
to loopMIDI + `--midi-port`. `--midi-virtual` therefore *parses* on Windows (clap is
platform-agnostic) but *fails fast with guidance at construction*. Alternative (reject the
flag at parse time on Windows) would make `cli.rs` platform-specific and break the
lib-testability symmetry — **not recommended**. *Lead's call: cfg-stub error vs. parse-time
rejection.*

### 6.3 RESOLVED — midir `virtual` cargo feature (verified: NONE needed in 0.8)
Verified against `~/.cargo/registry/.../midir-0.8.0`: there is **no `virtual` feature** in
midir 0.8's `[features]`. `create_virtual` is provided by the `midir::os::unix::VirtualOutput`
trait (`src/os/unix.rs`), and the whole `os::unix` module is `#[cfg(unix)] pub mod unix;`
(`src/os/mod.rs`). So virtual support is gated by **target OS inside midir**, not by a cargo
feature — `midir = { version = "0.8", optional = true }` is already sufficient. The
Implementer only needs the trait import under `#[cfg(unix)]`. Signature confirmed:
`fn create_virtual(self, port_name: &str) -> Result<MidiOutputConnection, ConnectError<MidiOutput>>`
— it **consumes** the `MidiOutput` and yields the connection directly (no separate `connect`).

### 6.4 midir-as-default-dep weight
Minor: one small crate (+`memalloc`). Linux ALSA backend reuses cpal's existing
`libasound2-dev`. No libclang / system-OpenCV implications (those stay on the opt-in
`opencv` feature). Net: the "clean box runs `cargo run` and gets audible output" property
(`Cargo.toml:33-37`) is preserved; the box now *also* can `--output midi`.

### 6.5 `default-run` papercut
Adding `default-run = "audiohax"` is pure metadata; the only behavioral change is `cargo run`
(no `--bin`) resolving to the app bin instead of erroring on ambiguity. Zero risk.

### 6.6 Headless testability (mirror the S9 pattern)
The live MIDI calls (`list_ports`, `open_selector`, `open_virtual`) need a MIDI subsystem
and are **not** unit-testable in CI / on a headless box without ALSA sequencer / a virtual
backend — by design they are **not** unit-tested. What **is** unit-tested, exactly as S9
did for the clap grammar (`cli.rs:578` test mod, `Cli::try_parse_from`): the **parsing** of
`--output synth|midi`, `--list-midi-ports`, `--midi-port <substr|index>`, and `--midi-virtual`
(bare → `Some("AudioHaxOut")`, valued → `Some("X")`, omitted → `None`). The `OutputSink`
enum's `ValueEnum` round-trip and `default_value_t` are asserted in the lib test. This keeps
the *decision surface* (which sink, which selector) fully testable while the irreducibly-live
MIDI I/O is exercised manually via the routing doc. The index-vs-substring discrimination in
`open_selector` is a `str::parse::<usize>()` decision that *could* be factored into a tiny
pure helper for unit-testing if the Implementer wants belt-and-suspenders — optional.

### 6.7 Engine seam — explicit non-change
There is **no** point in this design where `engine.rs` needs to change. Sink construction,
sink selection, port enumeration, and virtual-port creation are *all* adapter concerns
living in `main.rs` + `midi_output.rs`. The engine continues to drive `Box<dyn AudioSink>`
and never learns which concrete sink it holds. **The seam stays BYTE-UNCHANGED. This is the
correct boundary and the design does not relax it.**
```

# Quality Gate Review — S12: First-Class MIDI Output Lane

**Reviewer:** Quality Gate (independent)
**Date:** 2026-06-14
**Lane:** WS-4 S12 — external-MIDI output path made first-class (runtime sink selection; output plumbing only)
**Baseline:** `afc9478` (S11 — pure-Rust dependency collapse)
**Design:** `docs/design-s12-midi-out-first-class.md`
**Toolchain:** cargo 1.96.0 (userspace, `$HOME/.cargo/bin`)

**OVERALL VERDICT: PASS**

All five verdict criteria independently re-verified and PASS. engine.rs is byte-unchanged
(sha256 re-computed, matches). The default synth play path was re-run on a real image and
still plays unchanged with no `--output`. No blocking issues. Pre-existing modem/chord_engine
warnings are the only lint noise and are explicitly out of scope (and one is a preserved
investigation lead).

---

## Compilation Status

- `cargo build` (DEFAULT features `["pure-analysis","synth","midi-out"]`): **SUCCEEDS.** midir
  now compiles into the default binary (ALSA backend reuses cpal's `libasound2-dev`; no new
  system dep). Finished clean.
- New-file warnings: **NONE.** After touching `src/cli.rs`, `src/main.rs`, `src/midi_output.rs`
  and rebuilding, no warning's `-->` location points at any of the three lane files. The only
  lib warnings are in `src/modem.rs` (2) and `src/chord_engine.rs:125` (1) — all pre-existing.

## Format / Lint

- `rustfmt --edition 2021 --check src/cli.rs` → **clean.**
- `rustfmt --edition 2021 --check src/midi_output.rs` → **clean.**
  (Tree-wide `cargo fmt --check` deliberately NOT run, per lane instruction — pre-existing files
  would create noise.)
- `cargo clippy --bin audiohax -- -W clippy::all`: **no errors; zero clippy findings whose
  `-->` path is `src/cli.rs`, `src/main.rs`, or `src/midi_output.rs`.** The 29 lib-level clippy
  warnings are all pre-existing modem/shard/chord_engine items (div_ceil, index-loops, unused
  parens, etc.) — NON-BLOCKING per the lane's stated criteria (midi_output.rs is bin-private,
  and the new code is correctness-clean).

## Test Results (per-net counts, all green)

| Net | Command | Result |
|---|---|---|
| lib (default feats) | `cargo test --lib` | **108 passed**, 0 failed |
| lib (no-default-features) | `cargo test --lib --no-default-features` | **80 passed**, 0 failed |
| phase2_pure_pipeline | `cargo test --test phase2_pure_pipeline` | 7 passed |
| engine_seam | `cargo test --test engine_seam` | 10 passed |
| engine_equivalence | `cargo test --test engine_equivalence` | 9 passed |
| cli_parse | `cargo test --test cli_parse` | 24 passed |
| tui_render | `cargo test --test tui_render` | 13 passed |
| modem_roundtrip | `cargo test --test modem_roundtrip` | 17 passed |
| modem_realair | `cargo test --test modem_realair` | **10 passed** (74.0s; ~80-90s expected ✓) |
| qg_probe_band_isolation | `cargo test --test qg_probe_band_isolation` | 1 passed |

The 10 new S12 parser tests live in `cli::tests` (run inside `cargo test --lib`, contributing to
the 108): `play_output_defaults_to_synth`, `play_output_midi_parses`,
`play_output_synth_parses_explicitly`, `play_rejects_unknown_output_variant`,
`play_list_midi_ports_flag_parses`, `play_midi_port_accepts_substring`,
`play_midi_port_accepts_numeric_index`, `play_midi_virtual_bare_uses_default_name`,
`play_midi_virtual_valued_uses_given_name`, `output_sink_default_is_synth`. No regressions in any
net.

---

## Verdict Criteria — Findings (explicit PASS/FAIL)

### Criterion 1 — engine.rs BYTE-UNCHANGED + protected set untouched — **PASS**

- `git show afc9478:src/engine.rs | sha256sum` →
  `66becdaa8400ec649b7755463ebed1502cc5138dd83655fff5ef4569fd8e9fd9`
- `sha256sum src/engine.rs` →
  `66becdaa8400ec649b7755463ebed1502cc5138dd83655fff5ef4569fd8e9fd9`
- **Identical — independently re-computed.**
- `git diff --name-only afc9478` (tracked) lists ONLY: `Cargo.toml`, `src/cli.rs`, `src/main.rs`,
  `src/midi_output.rs`. The new docs (`docs/midi-routing.md`, `docs/design-s12-…md`) and this
  review are untracked additions; `assets/images/magicstudio-art.jpg` is an untracked test asset
  (not a modification of any protected asset).
- Protected files all show 0 diff lines vs `afc9478`: `src/synth_sink.rs`, `src/chord_engine.rs`,
  `src/pure_analysis.rs`, `src/mapping_loader.rs`, `src/modem.rs`, `src/tui.rs`, `assets/**`,
  `src/bin/**`.
- **Preserved warning confirmed:** `src/chord_engine.rs:125` `unused variable: next` STILL fires
  in the default build (chord_engine.rs untouched). The Lane-2 investigation lead is intact, not
  silenced.
- Cargo.lock: not changed (no tracked diff vs `afc9478`).

### Criterion 2 — DEFAULT build compiles + DEFAULT play path still works — **PASS**

- `cargo build` (now includes midir) succeeds.
- `cargo run --bin audiohax -- play assets/images/example.jpg` (NO `--output`) independently
  re-run: loads pure-Rust (`Image loaded from source (pure-Rust): 900x440`), computes global
  features, selects engine mode (Dorian), then `Starting in-process synth (rustysynth + cpal,
  bundled SoundFont)...` → `Synth audio stream started @ 44100 Hz.` → completes, **exit 0.**
- The default play path is the in-process synth, unchanged, with no `--output` required. **Not
  a restatement — re-executed.**

### Criterion 3 — Runtime MIDI selection is correct — **PASS**

- `--list-midi-ports` re-run: prints
  `Available MIDI output ports:` then `[0] Midi Through…`, `[1] PipeWire-System…`,
  `[2] PipeWire-RT-Event…` and exits **with no playback** (no synth stream, no MIDI connect). In
  `main.rs` the short-circuit sits AFTER subcommand parse but BEFORE `load_mappings(...)` and all
  image/feature-source work — confirmed by reading lines 257–280.
- Sink routing (`main.rs`): `want_midi = matches!(output, Midi) || midi_virtual.is_some()`. When
  true and `midi_virtual` set → `MidiOut::open_virtual(name)`; else → `MidiOut::open_selector(...)`.
  When false → `SynthSink::with_bundled_soundfont()`. The live MIDI path therefore uses
  `open_selector`/`open_virtual`, **never `open_first`.**
- `open_selector` resolution logic (read in `midi_output.rs`): empty ports → actionable
  `NO_PORTS_MSG`; `Some(s)` that `parse::<usize>()` succeeds → index branch with explicit
  out-of-range error citing port count + `--list-midi-ports`; `Some(s)` non-numeric → substring
  match, else "No MIDI output port matched '…'" error; `None` → first port. Correct
  index→substring→error/first ordering with actionable errors.
- `open_virtual` is real `#[cfg(unix)]` (`MidiOutput::create_virtual`) and `#[cfg(not(unix))]`
  returns an actionable loopMIDI error — selection branch identical across platforms, body differs
  by cfg.
- `--midi-port` carried by clap as an opaque `String`; index-vs-substring discrimination correctly
  deferred to `open_selector` (asserted by `play_midi_port_accepts_numeric_index` /
  `play_midi_port_accepts_substring`). Live MIDI *send* not exercised against a real synth (per
  scope); enumeration + branch logic confirmed.

### Criterion 4 — All nets green — **PASS**

See the Test Results table. Lib 108 (default) / 80 (no-default-features) match expectations
exactly; all 8 integration nets pass; modem_realair 10/10 in 74s. No regressions.

### Criterion 5 — No overclaim + clean build — **PASS**

- Default build is WARNING-CLEAN for the new files. The `#[allow(dead_code)]` on `open_first`
  and `play_chord_arpeggio` is genuine dead-by-design / kept-per-design, NOT masking a wiring bug:
  the live sink path provably uses `open_selector`/`open_virtual` (Criterion 3), so `open_first`
  is legitimately unreferenced once midi-out is default. Acceptable.
- `docs/midi-routing.md` is honest: explicitly labels `--output synth` as "**dry**" and states the
  feature is "**output plumbing only** … the external engine just renders them through better
  synths + effects." Reverb/chorus are attributed to FluidSynth/Qsynth/DAW, never to AudioHax.
  Commands match the real CLI grammar (`--list-midi-ports`, `--midi-virtual [NAME]`, `--output
  midi --midi-port <substring|index>`, `$AUDIOHAX_MIDI_PORT`), and the `cargo run` (no `--bin`)
  examples are valid because `default-run = "audiohax"` was added to Cargo.toml. No overclaim.

---

## Boundary Audit

- The protected set (engine.rs, synth_sink.rs, chord_engine.rs, pure_analysis.rs, mapping_loader.rs,
  modem.rs, tui.rs, assets/**, src/bin/**) is byte-identical to `afc9478`. Engine seam unchanged.
- `main.rs` is orchestration only. The diff adds: three import lines (now-unconditional `MidiOut`
  + `AudioSinkError`, plus `OutputSink`), the `--list-midi-ports` query short-circuit, and the
  runtime sink-selection `if want_midi {…} else {…}` block. A targeted grep of added `+` lines for
  any musical-logic tokens (hue/scan/chord/note/scale/mode/instrument/tempo/brightness/pipeline
  computation) returns nothing beyond sink/list/import plumbing. The S11 compile-time
  `#[cfg(feature="midi-out")]` fork was replaced by a runtime branch; no musical behavior moved.
- `cli.rs` adds the `OutputSink` ValueEnum and three `PlayArgs` fields (`output`, `list_midi_ports`,
  `midi_virtual`) plus tests — grammar surface only.
- `Cargo.toml`: `default` gains `midi-out`; `default-run = "audiohax"` added. No protected-target
  table changed.

## Integration Assessment

The runtime branch fits the existing `Box<dyn AudioSink>` seam cleanly — the engine driver below
the selection point speaks only the trait, so promoting midi-out to a runtime choice required no
change to the engine or the driver loop. `impl AudioSink for MidiOut` is now unconditional (matches
midi-out being default) and stringifies `MidiOut`'s non-`Send` `Box<dyn Error>` into `AudioSinkError`
without touching `midi_output.rs`. engine_seam (10) and engine_equivalence (9) still pass, confirming
the seam is intact. No stray `TODO(s12` markers anywhere (grep clean).

## Test Quality

The 10 new parser tests assert concrete parse outcomes, not `is_ok()` alone:
default→`OutputSink::Synth` with the three knobs absent/false; `--output midi`→`Midi`;
`--output loud`→`is_err()` (ValueEnum rejection); `--list-midi-ports`→flag true;
`--midi-port FLUID`→`Some("FLUID")` and `--midi-port 2`→`Some("2")` (proving the grammar carries
the selector opaquely and defers index/substring to `open_selector`); bare `--midi-virtual`→
`Some("AudioHaxOut")` via `default_missing_value`; `--midi-virtual MyPort`→`Some("MyPort")`;
`OutputSink::default()`==`Synth`. Good coverage of the new surface.

---

## Blocking Issues

**NONE.**

## Non-Blocking Issues

1. Pre-existing lint noise outside scope, unchanged by this lane: `chord_engine.rs:125` (preserved
   investigation lead — must stay), `modem.rs:426` (unused parens), `modem.rs:983` (`total_shards`
   unused), and ~29 lib-level clippy warnings in modem/shard code. All out of scope for S12.
2. The 10 new parser tests live in `cli::tests` (run via `cargo test --lib`), not in the
   `cli_parse` integration net. Functionally fine and intentional; noted only so the count of "108
   lib tests" is understood to include them.
3. The live MIDI *send* path (actual note bytes to a real synth) is not automatically exercised —
   correctly out of scope; enumeration and branch correctness are covered.

---

## Overall Verdict: **PASS**

The S12 lane delivers first-class runtime MIDI sink selection as output plumbing only, with the
engine and the entire protected set byte-unchanged, the default synth play path intact and
re-verified live, correct and actionable MIDI selection logic, all 10 test nets green
(108/80/7/10/9/24/13/17/10/1), warning-clean new files, and honest documentation. Safe for the
lead to integrate.

# Quality Gate Review — S59 Lane H (main.rs feature-gating hygiene)

Independent re-derivation. Every claim below was verified from scratch against the
working tree; the implementer's self-description was not trusted.

## Scope under review
Make the `audiohax` app bin genuinely build pure-Rust headless (no opencv, no midir),
fixing the root feature-gating defect that the prior session (S58) had only
Cargo-skipped by declaring the full default set as `required-features`.

## Verdict: PASS

All ten checks pass. The core win — a real pure-Rust headless build of the app bin —
is verified by forced recompilation, not a cached artifact. No scope creep, no test
regression, no net-new lint, engine.rs frozen, no double-definition risk, real
compiling `not(midi-out)` fallback, default runtime behaviour preserved, scrub clean.

## Per-check table

| # | Check | Result | Evidence |
|---|-------|--------|----------|
| 1 | engine.rs FROZEN | PASS | `sha256sum src/engine.rs` = `e50c7db1…2348261` (exact match); engine.rs absent from `git diff --stat` |
| 2 | SCOPE | PASS | `git status --short` + `git diff --stat` show ONLY `Cargo.toml`, `src/main.rs` (M) and `docs/design-s59-mainrs-hygiene.md` (??). No stray file. |
| 3 | CORE WIN build | PASS | `cargo build --no-default-features --bin audiohax --features pure-analysis,synth` → Compiling + Finished, exit 0, after `touch src/main.rs` forced a genuine recompile |
| 4 | DEFAULT build | PASS | `cargo build` → Finished, exit 0 |
| 5 | DEFAULT suite | PASS | `cargo test` → **580 passed / 0 failed**; 39 integration test binaries executed (39 on disk = 39 run — none dropped) |
| 6 | HEADLESS suite | PASS | `cargo test --no-default-features` → compiles + all green (lib unittests 210 passed, all integration bins ok, 0 failed) |
| 7 | CLIPPY net-new | PASS | `cargo clippy --all-targets`: every warning anchors in `src/modem.rs`, `src/chord_engine.rs`, `bin make_tiled_payload`, `bin modem_decode`, and test targets — ZERO anchor in the changed files (`src/main.rs`, `Cargo.toml`). Net-new = 0. |
| 8a | No `not(opencv)` without `pure-analysis` | PASS | grep of all `cfg(` in main.rs: every `not(feature="opencv")` predicate (lines 378/416/418/659/707/709) is now `all(not(opencv), feature="pure-analysis")`. No bare `not(opencv)` remains. |
| 8b | opencv vs pure mutually exclusive | PASS | opencv arms `#[cfg(feature="opencv")]`; pure arms `all(not(opencv), pure-analysis)`. The `not(opencv)` guard makes them disjoint — under `--features opencv` the pure arm drops out; under default (no opencv) the opencv arm drops out. No duplicate `source`/`understanding` definition. `required-features` guarantees pure-analysis present whenever the bin builds, so neither-arm-defined is impossible. |
| 8c | MidiOut/midir behind `midi-out`, real fallback | PASS | Every executable `MidiOut`/`midir`/`midi_output` token is behind `#[cfg(feature="midi-out")]`: `mod midi_output` (48), imports (76), `impl AudioSink for MidiOut` (89), `list_ports` (570), sink arm (738–752). `not(midi-out)` siblings at 585 (list-ports message → `return Ok`) and 765 (eprintln + `build_synth_sink`) are real compiling fallbacks that construct the synth — not panic stubs. Confirmed to compile by check 3. |
| 8d | build_synth_sink behaviour-preserving | PASS | Extracted fn takes same `&PlayArgs`, builds same `SynthConfig{ enable_reverb_and_chorus, gain }`, same `SoundFontSource` match, same `SynthSink::new_with_config(font_src, synth_config)`, same prints. Byte-identical to the former inline `else` block; no synth-config drift. |
| 9 | Default runtime behaviour preserved | PASS | `default = ["pure-analysis","synth","midi-out"]`. Default has no opencv → pure image provider selected (same as before). midi-out ON → the `#[cfg(feature="midi-out")]` sink-selection arm is the compiled one, with identical `want_midi` order (`--midi-virtual` forces MIDI, else `--output`, default synth). Selection order unchanged. |
| 10 | SCRUB | PASS | Word-boundary scan for internal build-process codenames over main.rs, Cargo.toml, design doc → exit 1 (no matches). No codename-enumerating pattern line in the design doc. |

## Module-boundary note
Only `src/main.rs` (orchestration/sink-selection — in bounds) and `Cargo.toml`
changed in source. No music-pipeline, modem, or engine file touched. engine.rs frozen.

## Non-blocking observation (not a defect)
The implementer's summary said "the only warnings are pre-existing `doc_lazy_continuation`
notes." The default codebase actually carries a broader set of pre-existing clippy
warnings (modem.rs loop-index/`div_ceil`/`is_multiple_of`, chord_engine.rs doc-list
items, etc.). This does not affect the verdict: none of those anchor in the changed
lines, so **net-new = 0** as required. The characterization was imprecise; the
substantive claim held.

## Blocking issues
None.

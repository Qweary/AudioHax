# Quality Gate Review — S9 (WS-4 Phase 1: engine seam + clap CLI / modem-bin unification)

Reviewer: Quality Gate. Date: 2026-06-13. Base: `21fd304`. Spec: `docs/design-s9-engine-seam-cli.md`.
Files reviewed: `src/engine.rs`, `src/cli.rs`, `src/lib.rs`, `src/main.rs`, the four modem bins,
`Cargo.toml`, and the new suite `tests/{engine_seam,engine_equivalence,cli_parse}.rs`.

**Overall verdict: PASS.**

---

## 1. Compilation Status

| Target | Command | Result |
|---|---|---|
| Library (headless) | `cargo build --lib --no-default-features` | **PASS** (3 warnings, all pre-existing in `modem.rs`/`chord_engine.rs`) |
| `modem_encode` bin | `cargo build --bin modem_encode --no-default-features` | **PASS** |
| `modem_decode` bin | `cargo build --bin modem_decode --no-default-features` | **PASS** |
| `channel_sim` bin | `cargo build --bin channel_sim --no-default-features` | **PASS** |
| `make_packetized` bin | `cargo build --bin make_packetized --no-default-features` | **PASS** |
| `audiohax` bin (`main.rs`) | n/a — OpenCV/ALSA adapter, cannot build headless | validated by INSPECTION (accepted constraint) |

The lib builds clean under `--no-default-features` — strong evidence the engine carries no system-lib
linkage. `Cargo.toml` gained `clap 4 (derive)`, `toml 0.8`, `directories 5`; `thiserror 1.0` was
already present (used by `cli::ConfigError`). `src/lib.rs` adds `pub mod engine; pub mod cli;`.

## 2. Lint Status

`cargo clippy --lib --no-default-features`: **ZERO findings in `engine.rs` or `cli.rs`.** The 28
clippy warnings emitted are all in pre-existing files (`chord_engine.rs`, `modem.rs`) — not this
session's concern, noted as pre-existing (`unused_parens`, `unused_variables`, `io_other_error`).
Formatting of the new files is clean and consistent on inspection (`cargo fmt --check` is unusable
here — it pulls the binary — so this is by-read, NON-BLOCKING).

## 3. Test Results

| Suite | Command | Pass / Fail / Ignored |
|---|---|---|
| Library unit | `cargo test --lib --no-default-features` | **61 / 0 / 0** |
| `engine_seam` | `cargo test --test engine_seam --no-default-features` | **10 / 0 / 0** |
| `engine_equivalence` | `cargo test --test engine_equivalence --no-default-features` | **9 / 0 / 0** |
| `cli_parse` | `cargo test --test cli_parse --no-default-features` | **24 / 0 / 0** |
| `modem_roundtrip` | `cargo test --test modem_roundtrip --no-default-features` | **17 / 0 / 0** |
| `modem_realair` | `cargo test --test modem_realair --no-default-features` | **10 / 0 / 0** (85s, expected) |

**ALL NETS GREEN.** The library unit count rose from the kickoff's 42 to 61 — the +19 are the new
`engine.rs` (8) and `cli.rs` (11) inline tests, purely additive. The prior music/modem nets are
untouched and still pass.

## 4. Module Boundary Audit

**Engine purity — VERDICT: PURE (load-bearing pass).**
`grep` over `src/engine.rs` for `opencv|Mat|midir|image::|imshow|imwrite|highgui` returns ONLY
doc-comment mentions ("the engine never sees a `Mat`") — no import, no type, no field. The mirror
`GlobalFeatures`/`ScanBarFeatures` are plain `f32`/`usize`/`Vec<f32>`; the two traits
(`FeatureSource`, `AudioSink`) and all value structs are image-free. The clean
`--no-default-features` lib build corroborates: with no system libs available, any OpenCV/midir
linkage would have failed the build. The engine is pure orchestration over `chord_engine`'s public
API plus the two traits.

**main.rs is a THIN ADAPTER — VERDICT: PASS (extraction complete).**
Read in full (375 lines). It retains ONLY: argv parse via `audiohax::cli`, OpenCV image acquisition
(`load_image_from_source`/`analyze_global`/`scan_image`), overlay `imwrite` + highgui
`imshow`/`wait_key`, the `PrecomputedSource: FeatureSource` field-copy boundary
(`to_eng_global`/`to_eng_scanbar`), `impl AudioSink for MidiOut`, the jitter RNG, and
`Instant`/`thread::sleep` scheduling. The driver loop calls `engine.decide_step(&source, step_idx)`
and only schedules/sends/draws. **No chord/voice/realize/mode/progression/plan decision logic
remains** — `worker_decide_action`, `play_scanned_steps_concurrent`, the `Barrier` pool, and
`InstrumentAction` are all gone (confirmed absent by reading). The per-channel program scheme
`(i*7)%128` and channel `i%16` are the only "music-adjacent" values left, and both are wiring the
adapter feeds to the sink, not musical decisions.

**Forbidden files untouched — VERDICT: PASS.**
`git diff 21fd304 --stat` over `chord_engine.rs`, `mapping_loader.rs`, `mappings.json`,
`image_analysis.rs`, `image_source.rs`, `midi_output.rs`, `modem.rs` returns **EMPTY** — none
changed. The full diff stat shows only `Cargo.{toml,lock}`, the four modem bins, `lib.rs`, and
`main.rs` modified; `engine.rs`/`cli.rs`/the three new test files are new (untracked — this is a
pre-commit review).

**engine.rs only CALLS chord_engine — VERDICT: PASS.** It uses `ChordEngine::new`,
`pick_progression`, `generate_chords`, `plan_phrases`, `realize_step`, and reads
`PerfFeatures`/`NoteEvent`/`StepPlan`/`PhrasePosition` through the public API. `decide_instrument_action`
projects `ScanBarFeatures → PerfFeatures` (plain field copy, no cast — units match) and calls
`realize_step`; it re-derives no music logic.

**modem.rs untouched; only the bins changed — VERDICT: PASS.** The four targeted bins
(`modem_encode`/`modem_decode`/`channel_sim`/`make_packetized`) now `use audiohax::cli::parse_*`
and run the same `modem::*` logic below; no hand-rolled `while i < args` / `print_usage` remains in
them. (The two `*_tiled_payload.rs` bins still hand-roll args — they are OUT of this session's
named scope of four; correctly left alone, not a finding.)

## 5. Interface / Spec Conformance

Cross-checked `engine.rs` + `cli.rs` public surface against the spec §3.1/§3.6/§3.7/§3.9. The four
Implementer-flagged deviations are each a sound, documented adaptation — not a silent contract break:

1. **`InstrumentDecision.events: Vec<NoteEvent>`** (vs the spec's prose "same `(note,vel,hold,offset)`
   payload"). SOUND — `NoteEvent` carries exactly those four fields; using the typed struct rather
   than a tuple is strictly better and stays image-free. Documented in the struct doc-comment.
2. **`merge_config` equal-to-default instead of `ArgMatches::value_source`.** SOUND — documented in a
   NOTE(s9) at `cli.rs:516`. It needs no raw `ArgMatches` (hidden by the derive API), is trivially
   headless-testable, and the precedence TABLE is identical. The one acknowledged corner ("user types
   the exact default → file wins") is documented and acceptable for Phase 1; the precedence test pins
   the three cells.
3. **No-op `interleave` flag on `ModemEncodeArgs`** alongside `no_interleave`. SOUND — preserves
   legacy invocation compatibility (`--interleave` accepted; `--no-interleave` takes precedence),
   documented in the field doc.
4. **`ChannelSimArgs` hyphen + underscore aliases** (`--flip-prob`/`--flip_prob` etc.). SOUND and
   REQUIRED by the unification mandate — legacy underscore spellings preserved via `alias = "..."`,
   canonical hyphen spelling added. Pinned by `cli_parse` + `cli.rs` inline tests asserting BOTH
   spellings reach the same value.

**AudioSink decision — PASS.** `AudioSinkError(Box<dyn Error + Send + Sync>)` (NOT `anyhow`), with
`note_on`/`note_off`/`program_change` split (not a single `send`), exactly as the lead approved.
`impl AudioSink for MidiOut` lives in `main.rs` (adapter), not the lib — orphan-rule correct (the lib
cannot name bin-private `MidiOut`); the impl maps `MidiOut`'s non-`Send+Sync` `Box<dyn Error>` into
`AudioSinkError` by stringifying, so `midi_output.rs` is never touched.

**Lead-approved invariants hold — PASS.** Single-threaded engine (no `Barrier` in `engine.rs` —
the core loops instruments sequentially); config precedence flags > file > defaults (in
`merge_config`); the regression anchor `decide_instrument_action` is a pure, deterministic free
function; modem flag aliases preserved.

## 6. Test Quality Assessment

**Batch-equivalence golden — VERDICT: PASS (genuine golden, NOT tautology).**
`tests/engine_equivalence.rs` pins `decide_instrument_action` against a FIXED hand-built
`&[StepPlan]` (no `pick_progression`/`thread_rng`) and asserts CONCRETE, HAND-DERIVED golden
constants as literal expected values:
- `G_BASS_NOTE = 36` and `G_MELODY_NOTE = 79` — each with a full register-math derivation comment
  (`seat_pc_in_register`, register floors, brightness lift), asserted via `assert_eq!(... .note,
  G_BASS_NOTE)`.
- Cadence velocities **114** (sat 100 → floor 96 + gain +18) and **84** (sat 0 → floor 96 − 12),
  asserted as literal `114`/`84`, with the `realize_velocity` derivation shown.
- Cadence hold **240 ms** (ritardando `1.20 × 200`), asserted as literal `240`.

These are independent of the production function (hand-derived from the documented realizer algorithm),
so a future change to musical output WOULD fail them — the regression alarm is real. The one
self-equality test (`test_full_golden_sweep_is_byte_identical`) is explicitly labelled a determinism
check that SUPPLEMENTS the golden pins, not a substitute for them; both properties (determinism AND
concrete golden) are present. The net also pins channel `inst%16`, empty-plan silence, modulo-wrap
step selection, saturation→velocity monotonicity, role stratification (bass < melody), and band
membership (24..=108). No tautological tests found.

**CLI alias preservation — PASS.** `channel_sim_legacy_underscore_flip_prob_alias_works` (and the
`cli_parse` analogues) assert BOTH `--flip_prob` and `--flip-prob` reach the same `flip_prob` value.
Additional underscore aliases (`burst_prob`/`burst_len`/`packet_size`) covered.

**Config precedence — PASS.** All three cells covered: file-beats-default, flag-beats-file, and
default-when-neither (`cli_parse` tests `test_precedence_*`, plus the `cli.rs` inline
`merge_config_precedence_flag_beats_file_beats_default`).

**engine_seam — PASS.** Asserts the sink saw EXACTLY the decided `(channel, note, velocity)` note_ons
in order (`assert_eq!(sink.note_ons(), expected_ons)`) — a meaningful property, not `is_ok()`. Also
covers note_on/note_off pairing, monotonic position advance to 1.0, tick==decide_step equality,
graceful sink-error propagation (no panic), `AudioSinkError` Display, and the pause/stop/seek
transport contract.

## 7. Integration Assessment

**TODO(s9)/NOTE(s9) judgment — ACCEPTABLE documented boundaries, not incomplete integration.**
Two `NOTE(s9)` sites, both sound:
- `engine.rs:571` `rebuild_mapping_table` — `MappingTable` derives only `Deserialize`, not `Clone`,
  and `ChordEngine::new` consumes it by value, so the engine rebuilds a fresh table from
  `MappingTable`'s public fields each re-derivation. I cross-checked the helper against
  `mapping_loader.rs`: it reproduces EVERY field of `MappingTable`/`GlobalMapping` (all 7) / the three
  nested triggers / `InstrumentSectionMapping` (all 5) / `FineDetailMapping` (all 4) — **lossless**,
  reads public fields only, **does NOT edit `mapping_loader.rs`**. The note records that adding
  `#[derive(Clone)]` later collapses it to `table.clone()`. Acceptable Phase-1 boundary.
- `cli.rs:516` — documents the `merge_config` precedence approach (see §5.2). Not a defect.

**main.rs inspection (type-correctness by reading — the only check it gets).** The engine calls use
real signatures: `PipelineEngine::new(mappings, engine_config)`, `set_features_global(&EngGlobal)`,
`decide_step(&source, step_idx) -> Vec<InstrumentDecision>`, `current_state().mode`. The
`FeatureSource` impl returns `Vec<EngScanBar>` and `EngGlobal` matching the trait; the
`impl AudioSink for MidiOut` matches the trait's three method signatures and maps errors correctly.
The boundary copies (`to_eng_global`/`to_eng_scanbar`) map field-for-field between the
`image_analysis::*` and `engine::*` mirrors. **No obvious type mismatch visible by inspection.**

## 8. Blocking Issues

**NONE.**

## 9. Non-Blocking Issues

1. **Mirror-type drift risk (already mitigated, watch-item).** `engine::{GlobalFeatures,ScanBarFeatures}`
   duplicate the `image_analysis` structs; a field added to one and not the other silently de-syncs
   the boundary copy. Mitigated by cross-referencing doc-comments on both struct pairs and the
   boundary being one function — but there is no compile-time guarantee. Acceptable for Phase 1
   (the spec §6 risk 2 accepts it); flagged for awareness.
2. **`render`/`analyze`/`modem` subcommands print a "not yet wired" message in `main.rs`.** This is
   intentional Phase-1 scoping (the grammar surfaces them; the adapter bodies are follow-on), clearly
   messaged rather than silently no-op. Not a defect — noted so the lead is aware these arms are
   stubs.
3. **Pre-existing clippy warnings in `chord_engine.rs`/`modem.rs`** (28 total) are unrelated to this
   session but remain; a future cleanup pass could clear them.

---

**Verdict: PASS.** The engine core is genuinely pure and headless, `main.rs` is a thin adapter with
no musical decision logic, the batch-equivalence net pins real hand-derived golden constants (not
self-equality), all forbidden files are untouched, and all six test suites are green.

# Quality Gate Review — WS-4 Phase 3 (S10): ratatui TUI seam-proof

**Reviewer:** Quality Gate (Swaram specialist, TMP-orchestrated)
**Date:** 2026-06-13
**Base:** `29d6848` (clean S9 head)
**Verdict:** **PASS**

The change adds a `ratatui` TUI front-end as a PURE OBSERVER over the S9
`engine.rs` shared-core seam. It proves the pure-Rust core drives a real
front-end with ZERO native dependencies, adds no music/modem changes, no
dependency collapse, and does not mutate engine decision logic. All mechanical
checks pass; the module-boundary / pure-observer audit is clean.

---

## Change Set (verified against `29d6848`)

Exactly the seven expected files changed (plus `Cargo.lock`, an inevitable
consequence of adding two crates):

| File | Disposition | Status |
|---|---|---|
| `src/tui.rs` | NEW — render + SyntheticSource + NullSink + SnapshotCollector + drive helpers + inline tests | OK |
| `src/bin/audiohax-tui.rs` | NEW — thin crossterm runner (autodiscovered, no `required-features`) | OK |
| `src/lib.rs` | MOD — added `pub mod tui;` (+ reordered cli/engine) | OK |
| `src/cli.rs` | MOD — added `Command::Tui(TuiArgs)` arm + `TuiArgs` struct | OK |
| `Cargo.toml` | MOD — added `ratatui = "0.29"` + `crossterm = "0.28"` (regular, pure-Rust) | OK |
| `tests/tui_render.rs` | NEW — 13 TestBackend/synthetic/drive tests | OK |
| `Cargo.lock` | MOD — dep resolution | expected |

`git diff 29d6848 --name-only` plus untracked listing returns precisely this
set. NONE of `chord_engine.rs`, `mapping_loader.rs`, `assets/mappings.json`,
`image_*.rs`, `midi_output.rs`, `main.rs`, `modem.rs`, or `src/bin/modem_*`
changed.

---

## Compilation Status

All headless builds succeed (3 build warnings observed are PRE-EXISTING in
`chord_engine.rs`, e.g. unused `next` at line 125 — not introduced by this
change).

| Build | Command | Result |
|---|---|---|
| Headless lib | `cargo build --lib --no-default-features` | **OK** |
| Headless TUI bin (seam-proof) | `cargo build --bin audiohax-tui --no-default-features` | **OK** |

**Zero-native-dep verification (the seam-proof):**
`cargo tree --bin` is unsupported on this cargo (1.96.0), so the `--bin` form of
the native-dep grep was a false signal and was discarded. Verified rigorously
instead via the package-level no-default-features tree:

- `cargo tree --no-default-features -p audiohax` → contains `ratatui v0.29.0` +
  `crossterm v0.28.1` and **ZERO** of `opencv / midir / image / alsa / cpal /
  rustysynth / egui / imageproc / fluidsynth`.
- Proof the native deps ARE feature-gated (not merely absent): the DEFAULT tree
  (`cargo tree -p audiohax`) DOES list `midir v0.8.0` and `opencv v0.95.1`.
  They vanish under `--no-default-features`, and the `audiohax-tui` bin compiles
  in that mode — so the headless bin links no native deps. **Seam-proof holds.**

---

## Lint Status

- `rustfmt --edition 2021 --check` on every changed file
  (`src/tui.rs`, `src/cli.rs`, `src/lib.rs`, `tests/tui_render.rs`,
  `src/bin/audiohax-tui.rs`) → **all clean (no diff).**
- `cargo clippy` was **NOT RUN** — it is unavailable in this environment because
  it would trigger the OpenCV/default-feature build, which has no OpenCV/ALSA on
  this box. This is an environment limitation, not a pass.

---

## Test Results

All nets pass at the expected counts (headless, `--no-default-features`):

| Net | Expected | Result |
|---|---|---|
| `--lib` | 70 | **70 passed** |
| `--test tui_render` | 13 | **13 passed** |
| `--test engine_seam` | 10 | **10 passed** |
| `--test engine_equivalence` | 9 | **9 passed** |
| `--test cli_parse` | 24 | **24 passed** |
| `--test modem_roundtrip` | 17 | **17 passed** (5.8s) |
| `--test modem_realair` | 10 | **10 passed** (82.8s) |

Total: 153 across the listed nets; 0 failed, 0 ignored.

---

## Module Boundary & Pure-Observer Audit (heart of the review)

**engine.rs is byte-unchanged — verified independently, not on the
Implementer's word:**
- `git diff 29d6848 -- src/engine.rs` → **EMPTY.**
- `sha256(git show 29d6848:src/engine.rs)` == `sha256(src/engine.rs)` ==
  `66becdaa8400ec649b7755463ebed1502cc5138dd83655fff5ef4569fd8e9fd9`.

No decision/`realize`/plan logic in the engine was altered.

**`render()` is a PURE function of the snapshot.** It reads
`snapshot.global / scan_position / step_index / current_step / mode / phrase /
last_notes` and constructs ratatui widgets. It performs no engine calls, no RNG,
no mutation of the snapshot (the `&EngineSnapshot` is borrowed immutably; helper
fns take `&` too). Layout/normalize/`meter_gauge` are pure widget construction.

**No reach into chord_engine internals.** `tui.rs` NAMES chord_engine types
(`NoteEvent` / `PerfFeatures` / `PhrasePosition`) only inside the inline test
fixture to *construct a snapshot* — that is the explicitly-allowed rendering
use. It does NOT call `chord_engine::realize_step`, `pick_progression`, or any
decision/harmony function. All engine interaction goes through the PUBLIC seam
only: `PipelineEngine::new` / `set_features_global` / `tick` / `current_state` /
`config`, `EngineCommand::{Stop,Play}` (bin loop reset), and the
`FeatureSource` / `AudioSink` / `EngineObserver` traits.

- `SyntheticSource` is a deterministic `FeatureSource` (pure functions of the
  step index; triangle waves; explicitly no RNG).
- `NullSink` is a no-op `AudioSink` (observe, do not sound).
- `SnapshotCollector` is an `EngineObserver` that clones one snapshot per tick.

**The bin `src/bin/audiohax-tui.rs` is thin by contract.** It owns only
crossterm terminal setup/teardown (`enable_raw_mode` / alternate screen /
restore on every exit path) and the event/poll loop. All
render/feature/drive/wiring logic is delegated to `audiohax::tui`. It is left to
bin autodiscovery with **no `required-features`** table, so it builds under
`--no-default-features` (confirmed above). Teardown runs on success and error
paths so a crash never leaves the terminal in raw mode — a correctness nicety.

**Cargo.toml added ONLY `ratatui = "0.29"` + `crossterm = "0.28"`**, both
regular (non-optional) pure-Rust deps. Nothing from Phase 2/4: no `egui`,
`cpal`, `rustysynth`, `imageproc`, or `image` additions.

**cli.rs** adds only the `Tui(TuiArgs)` enum arm + a 2-field `TuiArgs` struct
(`--steps` default 40, `--instruments` default 4). Its doc comment correctly
notes that the unified `audiohax` binary links OpenCV and only builds with
default features, so the standalone `audiohax-tui` bin is what runs the TUI
headlessly — the arm exists for grammar completeness. This is accurate and does
not undermine the seam-proof (which rests on the standalone bin, verified
headless above).

---

## Musical Logic Review

**N/A** — no music-pipeline file changed (`chord_engine.rs`,
`mapping_loader.rs`, `assets/mappings.json`, `midi_output.rs`, `main.rs` all
byte-unchanged; engine.rs byte-unchanged). There is no musical decision logic in
this diff to review. `SyntheticSource` produces *image features*, not musical
decisions; the engine's existing (unchanged) logic turns those into notes.

---

## Test Quality Assessment

**Strong, and correctly scoped to the non-determinism boundary.**

- **Render tests assert real buffer content**, not `assert!(true)`. Group A
  flattens the `TestBackend` buffer to a string and asserts on it: mode
  (`"Dorian"`), labels (`"hue"/"sat"/"bright"/"edge"`, `"feature meters"`,
  `"mode:"`), note numbers (`60`, `67`) and the `@` separator, `"step 7"`, and
  the transport reading (`50.0%`, `0.0%`, `100.0%`). Negative assertions too
  (empty notes → `"silent"` and NO `@`; the 0.0 buffer must NOT contain
  `100.0%`). All render fixtures are HAND-CONSTRUCTED, not live-derived.
- **No-panic-on-cramped-layout tests** (20x10 and 1x1) are legitimate robustness
  checks, not filler — ratatui clips and the draw must still return Ok.
- **Drive/observer tests assert SHAPE/INVARIANTS only**, never live note/
  velocity/mode VALUES: step index increments one per tick, scan position
  monotone non-decreasing reaching ~1.0, observer records exactly one frame per
  tick in order, one decision per instrument, channel = `inst%16`, and
  note_on/note_off counts pair. **No flaky live-value assertion exists** — the
  closest (`drive_advances_scan_to_completion`) asserts only that `mode` is
  non-empty, never a specific mode. This correctly respects that
  `set_features_global → pick_progression` uses `thread_rng`.
- **Label strings the tests assert match what `tui.rs` emits** — spot-checked:
  `meter_gauge("hue"/"sat"/"bright"/"edge", …)` (tui.rs L111-123); transport
  title `" transport — step {} "` and `format!("scan {:5.1}%", …)` (L151,L155);
  `"mode: "` (L165); `note@velocity` via `format!("{}@{}", n.note, n.velocity)`
  (L199); silent placeholder `"(silent — no notes this step)"` (L194); block
  title `" feature meters (whole image) "` (L89). All consistent.
- The inline `src/tui.rs` tests duplicate-but-do-not-conflict with the
  integration tests (both legitimately exercise the same surface; the prompt
  expects 70 lib + 13 integration, which matched exactly).

---

## Integration Assessment

- The `Tui` arm integrates cleanly: the 24 `cli_parse` tests still pass, so the
  enum/struct addition broke no existing grammar.
- No type mismatch at the seam: `build_engine` / `drive_one_tick` use the
  engine's public types throughout; the bin and the tests share the same drive
  helpers (no duplicated wiring).
- No `TODO`/`unimplemented!`/`todo!` left in the new code.
- `Cargo.lock` updates are the expected resolution of the two new crates.

---

## Blocking Issues

**None.**

---

## Non-Blocking Issues

1. **clippy not run** (environment limitation — would require the OpenCV default
   build). Recommend running `cargo clippy --no-default-features` on an
   OpenCV-equipped box, or scoping a clippy invocation that excludes the OpenCV
   target, before any release tag — purely as belt-and-suspenders; rustfmt is
   clean and the code reads idiomatically.
2. **3 pre-existing build warnings** in `chord_engine.rs` (e.g. unused `next`,
   L125) surface during the lib build. NOT introduced by this change and out of
   scope for S10, but worth a future cleanup pass.
3. **`audiohax tui` (unified-binary arm) is non-functional by design** — only
   `audiohax-tui` runs headlessly; the unified arm links OpenCV. This is
   documented in the cli.rs doc comment and is intentional ("grammar
   completeness"). No action needed; flagged only so it is not mistaken for a
   bug later.

---

## Overall Verdict: **PASS**

The TUI is a genuine pure observer over the S9 engine seam. engine.rs is
byte-identical to base, the headless `audiohax-tui` bin compiles with zero
native dependencies (native deps confirmed present-then-gated), all 153 tests
across the seven nets pass, formatting is clean, and the test suite asserts real
render content and engine SHAPE invariants while correctly avoiding the
`thread_rng` non-determinism boundary. No music/modem/dependency-collapse
changes leaked in. Cleared for the lead to commit.

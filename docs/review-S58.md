# Quality Gate Review — S58 Lane B (`cargo test --no-default-features` compile fix)

**Reviewer:** Quality Gate (Specialist 6) — independent adversarial re-derivation.
**Change under review:** `Cargo.toml`-only — add `[[test]]` `required-features` tables for
feature-gated integration tests + restore `required-features = ["pure-analysis","synth","midi-out"]`
on the `[[bin]] audiohax` app target. Fixes the repo-wide-broken `--no-default-features` compile
(broken since S11).
**Toolchain:** cargo 1.96.0 (userspace).

---

## Compilation Status
- `cargo test --no-default-features` — **COMPILES + RUNS GREEN.** Exit 0.
- `cargo test` (default features) — **COMPILES + RUNS GREEN.** Exit 0.
- `cargo clippy --all-targets` — **Finished, exit 0, no errors.**

## Test Results
- **`--no-default-features`:** all running binaries pass, 0 failed, 0 ignored. The suite that runs is
  the pure-Rust **music + modem** core: lib unittests, TUI, modem bins (`modem_encode`,
  `modem_decode`, `channel_sim`, `make_packetized`), and pure-Rust integration tests
  (`modem_roundtrip`, `modem_cli_roundtrip`, `modem_realair`, `composition_s15`, `counterpoint_s30`,
  `keyplan_*`, `variety_s45`, `valence_mode_s36`, `tui_render`, `cell_distinctness_s53`,
  `keyplan_k2a`, etc.). Every observed `test result:` line was `ok`.
- **default features:** **580 passed / 0 failed** across **39 test binaries** + lib/bin unittests +
  doc-tests. Exit 0.

## Module Boundary Audit
N/A for source — change is `Cargo.toml`-only (`git diff --stat` = `Cargo.toml | 78 ... 1 file changed`;
`git status --porcelain` = ` M Cargo.toml` only). No `.rs`, no `assets`, no docs, no SWARM-STATE touched.

## Detailed Findings (per the 9 verification items)

### 1. `--no-default-features` compiles + runs green; gated tests cleanly skipped
PASS. The 8 `required-features` test tables are **not built** under `--no-default-features` (absent
from the `Running tests/` list) — a clean skip, not a silent failure. The 2 self-guarded tests
(`ab_harness_s31`, `render_cli_determinism_s55`) build to **empty binaries** (file-level `#![cfg]`
compiles the whole file out) and run 0 tests — also a clean skip. The app bin (`src/main.rs`) and the
image-gated bins (`make_tiled_payload`, `unpack_tiled_payload`) are correctly skipped. Bins that DO
build bare: `audiohax-tui`, `channel_sim`, `make_packetized`, `modem_decode`, `modem_encode`.

### 2. Default run green + no test binary dropped
PASS. Independently: `ls tests/*.rs` = **39 files**; default `cargo test` runs **exactly 39**
`Running tests/` binaries — the identical set. All 8 tabled tests
(`phase2_pure_pipeline`, `prominence_s43`, `qg_s53_review`, `rhythm_variety_s50`,
`runtime_reachability_s37`, `s52_probe_identity`, `saliency_s18`, `variety_scorecard_s45`) still appear
in the default run. Adding a `[[test]]` table with matching `name` + `path` does **not** remove the
target from default-feature auto-discovery. **Nothing silently disappeared.**

### 3. Over-gating check (`cell_distinctness_s53`, `keyplan_k2a`) — must NOT be gated
PASS. Both reference `pure_analysis` **only in doc-comments** (`//!` / `//`). Real `use` statements:
`cell_distinctness_s53` → `chord_engine`, `composition`, `mapping_loader`, `seed`;
`keyplan_k2a` → `chord_engine`, `composition`, `mapping_loader`. All pure-Rust default-buildable
modules. Correctly left **ungated**, and both **compile + run under `--no-default-features`** (present in
that run's `Running tests/` list). Gating them would have wrongly dropped them from the headless run.

### 4. Self-guard check (`ab_harness_s31`, `render_cli_determinism_s55`)
PASS. Both carry file-level `#![cfg(all(feature = "synth", feature = "pure-analysis"))]`. No Cargo table
needed — they compile to empty under `--no-default-features` on their own. Correctly omitted from the
`[[test]]` tables.

### 5. Feature-minimality
PASS — every set correct and minimal, verified against real `use` statements AND the `[features]` table:
- `pure-analysis = ["image", "dep:imageproc"]` — so `["pure-analysis"]` **transitively pulls the
  `image` crate**. The 7 tests tabled `["pure-analysis"]` (incl. `runtime_reachability_s37` and
  `saliency_s18` which `use image::{Rgb, RgbImage}`) are fully satisfied — **not under-broad.**
- `phase2_pure_pipeline` imports `synth_sink` + `rustysynth` (behind `synth`) AND `pure_analysis`/`image`
  → `["pure-analysis","synth"]`. Correct, minimal — no `midi-out` needed (no `midir` ref). **Not
  over-broad.**
- App bin `["pure-analysis","synth","midi-out"]`: `use midi_output::MidiOut` (main.rs:68) +
  `impl AudioSink for MidiOut` are **unconditional** → `midi-out`(midir) genuinely required;
  `synth_sink::TimedMidiEvent` return type (main.rs:259) is **unconditional** → `synth` required;
  the `pure_analysis` blocks (main.rs:371/603) are guarded by `#[cfg(not(feature="opencv"))]`
  (a predicate that does NOT match `pure-analysis`), so bare they reference a non-existent module →
  E0432/E0433 → `pure-analysis` required. All three genuine; the set equals the full default set, as the
  diff comment states. Minimal.

### 6. Lint — net-new warnings = 0
PASS. `cargo clippy --all-targets` finished exit 0, no errors. ~178 warning-lines total — all
**pre-existing `.rs`** style/lint warnings spread across lib + bins + tests. A `Cargo.toml`-only
metadata change touches no source and, under `--all-targets` (all features on), compiles the same target
set as before, so the warning set is unchanged. **Net-new = 0.**

### 7. `src/engine.rs` FROZEN
PASS. `sha256sum src/engine.rs` =
`e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261` — matches the expected hash exactly.

### 8. Scope
PASS. `git diff --stat` = `Cargo.toml` only (1 file, +75/-3). No `.rs`, docs, or SWARM-STATE.

### 9. Scrub
PASS. The Cargo.toml diff contains **no internal tooling codenames** (checked against the
full internal-agent name set → NONE FOUND). Comments reference only the project's own
WS-4/S-lane vocabulary.

## Judgment call — app-bin gating consequence
Because the app bin now carries `required-features`, **no application binary builds under
`--no-default-features`** (the `audiohax` image-to-music app and the image bins are skipped). This is
**not a defect** for the charter's "headless music+modem core" goal: the pure-Rust modem CLI tooling
(`modem_encode`/`modem_decode`/`channel_sim`/`make_packetized`), the TUI, and the whole lib + music/modem
test suite still build and run bare — verified above. The image-to-music app inherently needs image
analysis + synth + MIDI, so requiring those features to build it is correct, not a regression. The
cleaner long-term fix (retarget the `#[cfg(not(feature="opencv"))]` predicates onto `pure-analysis`) is
**source surgery explicitly out of charter scope** for this objective build; the Cargo-level whole-bin
gate is the correct minimal fix. **Accepted trade, not a blocker.**

## Integration Assessment
No API/type changes; no cross-module coupling introduced. `[[test]]` tables mirror the existing S5
`[[bin]] required-features` pattern. Default `cargo run`/`cargo build` unaffected (default enables all
three features). No TODO/incomplete-integration markers.

## Blocking Issues
**None.**

## Non-Blocking Issues
- Pre-existing (not introduced here): ~178 clippy style warnings across the workspace, and the latent
  design smell that main.rs gates its pure path on `not(feature="opencv")` rather than
  `feature="pure-analysis")` — the root reason the app bin can't build bare. Out of scope for this
  Cargo-only fix; note for a future source-hygiene lane.

## Overall Verdict: **PASS**

The `--no-default-features` workspace compile is fixed; the default suite is fully green (580/0) with
zero test binaries dropped; gating is correct, minimal, and over-gating is avoided; engine.rs is frozen;
scope and scrub are clean; net-new lint = 0.

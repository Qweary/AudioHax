# Quality Gate Review — S37: Wire the plan-first composer into the live `play`/`render` runtime

**Reviewer:** Quality Gate (independent verification — re-derived, not trusting agent self-reports)
**Date:** 2026-06-18
**Scope under review:** `src/main.rs` (Implementer), `tests/runtime_reachability_s37.rs` (Test Engineer, NEW), `docs/spec-s37-wire-composer.md` (Architect, NEW).

---

## Overall Verdict: **PASS**

The plan-first composer is now installed on the live pure-Rust `play` and `render` paths. The
engine.rs freeze holds byte-for-byte, the render ms-grid asymmetry is correctly handled, S13
diversity and C6.6 valence-family-mode are both provably on the wired path, and the new
runtime-reachability net asserts meaningful plan-derived properties (not theater). No codename
leaks in the public-bound files. Scope is held to the three S37 files plus the known
pre-existing uncommitted set.

---

## Compilation Status

- `cargo build` (default = pure-Rust profile): **PASS**. Binary compiles clean.
- The only warnings emitted are PRE-EXISTING and out of S37 scope (`src/bin/modem_encode.rs`
  `unused_assignments` on `seq`, and an `unpack_tiled_payload` warning) — neither is in
  `main.rs` or any S37-touched file.

## Lint Status

- `cargo clippy --tests -- -W clippy::all`: 139 warnings total across the workspace; **ZERO**
  in `src/main.rs` or `tests/runtime_reachability_s37.rs`. All hits are pre-existing in
  modem/bin/legacy modules. No correctness-class clippy warning introduced by S37.

## Format Status

- `cargo fmt -- --check`: **PASS** (exit 0, no diff). The new code is rustfmt-clean.

## Test Results (full default-feature suite)

BLOCKING gate (any fail = FAIL): **0 failures across the entire suite.**

| Suite | Result |
|---|---|
| lib unit tests (`src/lib.rs`) | **218 passed**, 0 failed |
| `src/main.rs` unit tests | 5 passed, 0 failed |
| `engine_equivalence` | **9 passed, 0 failed** (FREEZE GOLDENS — 9/9 GREEN) |
| `diversity_s13` | **10 passed, 0 failed** (S13 diversity net) |
| `valence_mode_s36` | **7 passed, 0 failed** (C6.6 net) |
| `runtime_reachability_s37` (NEW) | **3 passed, 0 failed** |
| `keyplan_k2a` | 9 passed, 0 failed |
| `keyplan_k2b` | 14 passed, 0 failed |
| `composition_s15` | 5 passed, 0 failed |
| `affect_s22`, `figuration_s20`, `prominence_s23`, `texture_s17`, `saliency_s18` | all passed |
| `counterpoint_s30` (13), `pattern_library_s34` (16), `ab_harness_s31` (3) | all passed |
| `modem_roundtrip` (17), `modem_realair` (10) | all passed |
| `tui_render` (13), `engine_seam` (10), `phase2_pure_pipeline` (7), `cli_parse` (24) | all passed |

Every suite GREEN, 0 failed / 0 ignored across the board.

---

## FREEZE WITNESS (load-bearing) — CONFIRMED

- `sha256sum src/engine.rs` = `e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261`
  — **MATCHES the freeze witness exactly. engine.rs is UNCHANGED.**
- `git diff src/engine.rs` is **empty** (no staged or unstaged change).
- `engine_equivalence` golden sweep: **9/9 GREEN.** The 240ms/114/84/36/79 goldens did not
  move because no code they depend on was edited.
- mtime corroboration: `src/engine.rs` last modified 2026-06-16 (before this session); the S37
  work all lands 2026-06-18 16:36–16:53. Engine was not touched by S37.

The Architect's "NO engine.rs change required" verdict and the Implementer's main.rs-only
discipline are **confirmed independently.**

---

## Module Boundary Audit (per-file)

- **`src/main.rs` (Implementer, sole writer):** Orchestration-only respected. The additions
  call EXISTING public lib functions (`understand_image_pure`, `compose_from_image`,
  `engine.composition()`, `set_features_global`) and read scalars (`plan.total_steps`,
  `plan.key_tempo.base_ms_per_step`) back off the read-only accessor. Grep of the `+` lines
  for inline music theory (mode tables `[0,2,...]`, `home_mode =`, `dorian/phrygian/lydian`,
  `scale_degree`, interval math) returns **nothing** — no music-theory logic moved into
  main.rs. Boundary held.
- **Both entry points compose:** `play` (main.rs:570) AND `render` (main.rs:306) both call
  `engine.compose_from_image(&understanding)`. Confirmed by grep, not by self-report.
- **`set_features_global` is now the FALLBACK only on the live path:** on the pure-Rust default
  arm it is reached solely inside `if !composed { ... }` (main.rs:311 render, 573 play). The
  live default path installs the plan; the flat S13 path is byte-identical pre-S37 behavior
  only when `mappings.json` has no `composition` block.
- **OpenCV arm = untouched-legacy (Option A):** the compose block is gated
  `#[cfg(not(feature="opencv"))]`; a sibling `#[cfg(feature="opencv")]
  engine.set_features_global(...)` (main.rs:313-314, 575-576) retains the literal pre-S37 line.
  OpenCV build stays byte-stable. Correctly applied.
- **engine.rs:** frozen, nobody wrote it. Confirmed.
- **No out-of-scope frozen file touched by S37** (mtime analysis below).

## Musical Logic Review

No music-pipeline source file was modified by S37 (composition.rs / engine.rs / mappings.json
all carry mtimes from earlier in the day — the pre-existing C6.6 work, not S37). Per Stage 3
this review focuses on whether the wiring correctly *routes* the existing music logic:

- **render ms-grid asymmetry (THE key correctness check) — CORRECT.** `render`'s per-step grid
  multiplier was swapped from the config `ms_per_step` to the plan's `base_ms_per_step`:
  `let step_base_ms = step_idx as u64 * grid_ms_per_step;` (main.rs:348) where
  `grid_ms_per_step = plan.key_tempo.base_ms_per_step` when composing, falling back to config
  `ms_per_step` when not (main.rs:318-321). Had this been left on the flat config value while
  composing, steps would be spaced at config tempo while notes hold at plan tempo — a cadence +
  determinism break. It is NOT left flat. **No cadence bug.**
- **`play` correctly does NOT swap a grid:** `play` schedules each note from `ev.offset_ms`
  relative to a per-step `t0`, so the plan tempo is already honored inside each decision; only
  `total_steps` is swapped (main.rs:682-685). The asymmetry the spec §3.2 calls out is
  respected exactly — play gets the total_steps swap, render gets total_steps AND the
  multiplier swap.
- **`total_steps` is plan-derived in BOTH entry points:** `match engine.composition() {
  Some(plan) => plan.total_steps, None => source.step_count() }` at both render (main.rs:318)
  and play (main.rs:682). Not `source.step_count()` on the composed path.
- **C6.6 valence-family-mode is on the live path:** `assets/mappings.json` contains BOTH the
  `composition` block (so `compose_from_image` returns `true` and a plan installs — not the
  fallback) AND a populated `mode_valence_cuts` (so `valence_family_mode` is the audible
  non-NO-OP). `valence_family_mode` is invoked at `composition.rs:1297` inside `plan()`, which
  `compose_from_image` drives. Chain confirmed: `main.rs → compose_from_image → planner.plan()
  → valence_family_mode → home_mode → engine.mode`. `valence_mode_s36` 7/7 green.

## Test Quality Assessment (Stage 4)

`tests/runtime_reachability_s37.rs` asserts MEANINGFUL properties, not `is_ok()`/`is_some()`
theater:

- **Assertion 1** (`composition_block_present_means_engine_installs_a_plan`): builds the engine
  from the REAL on-disk `assets/mappings.json` (same path main.rs uses), runs the exact
  `compose_from_image(&understand_image_pure(img))` call main.rs makes, asserts it returns
  `true` AND `engine.composition().is_some()`. This is a genuine data-layer guard against a
  future drop of the `composition` block. Adequate.
- **Assertion 2** (`drive_count_is_plan_derived_not_scan_derived`): asserts the plan's cursor
  invariant `total_steps == sum(section.step_len)`, pins the concrete value `total_steps == 24`
  for the dark-flat fixture (so a silent change in `BASE_STEPS_PER_SECTION`/section count is
  visible), AND `assert_ne!(plan.total_steps, 40)` where 40 is the CLI default scan count
  (`src/cli.rs:97`) the reverted `None` arm would loop to. Concrete numbers on both sides
  (24 plan vs 40 scan). This is a REAL guard against the loop bound reverting to
  `source.step_count()` — exactly the regression S37 fixes. Strong.
- **Assertion 3** (`distinct_images_install_distinct_plans`): composes a calm/dark flat field
  vs a vivid high-edge checker THROUGH the composer install path, asserts the two installed
  `CompositionPlan`s differ in ≥1 audible spine field (`base_ms_per_step` OR `home_mode` OR
  `total_steps`). This is a REAL spine-field divergence check (not a weak `is_some()`), proved
  end-to-end through the path the binary now uses — the precise property that locks S13
  diversity to the wired path. **Meets spec §8 assertion 3; not weaker.** The 3/3 green run
  proves the fixtures actually diverge in at least one spine field at runtime.
- **Determinism / RNG robustness:** every assertion is a pure function of the in-memory fixture
  + on-disk mappings. The composer spine (`home_mode`/`base_ms_per_step`/`total_steps`) is
  derived deterministically and does NOT take the `set_features_global → pick_progression`
  (`thread_rng`) path; the test never asserts on per-step harmony realization, so no seed is
  needed. Deterministic and RNG-robust. Documented in the file header (lines 24-29) and the
  documentation is accurate.

One observation (non-blocking): per spec §10 risk #1 and §8 assertion-4, the reachability net
does NOT directly assert the render WAV cadence multiplier equals `base_ms_per_step` (the
optional belt-and-suspenders seam smoke). The render multiplier swap is verified here by code
read (main.rs:348 uses `grid_ms_per_step`) and is covered transitively by `diversity_s13` /
spine-field divergence, but there is no standalone test pinning the render grid value. The
spec itself names assertions 1–3 as the minimum bar and assertion 4 as optional, so this is a
test-coverage gap relative to the *strongest* possible net, not a spec-compliance miss. Noted
as non-blocking.

## Integration Assessment

- No type mismatch at the module boundary: `understand_image_pure(img.as_rgb()) ->
  ImageUnderstanding` feeds `compose_from_image(&ImageUnderstanding) -> bool`; `composition()
  -> Option<&CompositionPlan>` yields `plan.total_steps: usize` and
  `plan.key_tempo.base_ms_per_step: u64` — both consumed with matching types.
- `img` correctly hoisted: `understand_image_pure` is computed inside the source block where
  `img` is live (main.rs:545 play, main.rs:285 render) and the understanding is carried forward
  via the `(source, understanding)` tuple. No lifetime/scope break; source-extraction path
  unchanged.
- No TODO/incomplete-integration markers introduced.
- Full suite links and runs clean, confirming no API-signature breakage.

---

## Codename-Leak Scrub (AudioHax is a PUBLIC repo) — CLEAN

Grepped the two NEW public files and the `src/main.rs` diff for the internal TMP codename set
(Neo, Oppenheimer, Groves, Geiger, Fermi, Curie, Hahn, Bohr, Criticality, Crosscheck, Chain,
Compton, Lattice, Swaram, "Manhattan Project", "Specialist N", Trinity, reactor):

- `src/main.rs` diff: **CLEAN.**
- `docs/spec-s37-wire-composer.md`: **CLEAN.**
- `tests/runtime_reachability_s37.rs`: **CLEAN.**

No internal framework identifiers in any public-bound file. No push-blocking hygiene issue.

---

## Scope Hold + Hygiene

`git status --short` matches the expected S37 set exactly. The three S37-authored files
(`src/main.rs` M, `tests/runtime_reachability_s37.rs` new, `docs/spec-s37-wire-composer.md`
new) all carry today's 16:36–16:53 mtimes. The rest of the dirty/untracked set
(`src/composition.rs`, `assets/mappings.json`, `tests/keyplan_k2a.rs`,
`tests/valence_mode_s36.rs`, `src/bin/make_tiled_payload.rs`, `src/bin/unpack_tiled_payload.rs`,
`src/image_analysis.rs`, `src/image_source.rs`, `assets/images/magicstudio-art.jpg`) carries
mtimes from earlier (13:36–13:48 or earlier) — the KNOWN pre-existing C6.6 + image-asset set,
NOT touched by S37. The C6.6 files were therefore left UNCHANGED by S37, as required. No
frozen / out-of-scope file modified.

(Note: the magicstudio-art image is at `assets/images/magicstudio-art.jpg` — under
`assets/images/`. Consistent with the known pre-existing set; flagged for path-awareness only.)

---

## Blocking Issues

**NONE.**

## Non-Blocking Issues

1. **No standalone render-cadence assertion (spec §8 assertion-4 / §10 risk #1).** The render
   grid multiplier swap to `base_ms_per_step` is verified by code read and covered transitively
   by diversity, but there is no test pinning the render WAV step spacing to the plan tempo.
   Optional per spec. A future small WAV-length / step-spacing sanity test would close the last
   un-wire vector (render cadence reverting to config tempo while still composing).
2. **OpenCV arm divergence (documented, Option A).** Per spec §4/§10 risk #3, the
   `#[cfg(feature="opencv")]` build stays on the legacy `set_features_global` path until a
   `Mat→RgbImage` bridge lands. Acceptable for S37 (default is pure-Rust) and documented in the
   code comments; not a defect. Tracked as the documented follow-up.
3. **Operator decision pending (spec §11) — tempo de-cap.** The composer can produce tempos
   outside the legacy 56–96 BPM Ballad clamp for non-Ballad characters (intended de-cap, left
   audible). This is a `mappings.json` data choice owned by Music Theory, surfaced as a decision
   point, not a code defect. Noted for operator awareness.

---

## Summary

S37 wires the plan-first composer into both live entry points correctly and with the engine
freeze fully intact. The single most important correctness detail — render's plan-tempo
ms-grid asymmetry — is handled right. S13 diversity and C6.6 are provably reachable through the
wired path (composition block + mode_valence_cuts both present in the real mappings; their nets
green). The new reachability net is genuine, deterministic, and would catch a future silent
un-wire. No codename leaks. **PASS.**

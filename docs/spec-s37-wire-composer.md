# S37 — Wire the plan-first composer into the live `play` / `render` runtime

**Status:** DESIGN ONLY (Rust Architect). No `src/*`, `assets/*`, or `tests/*` file is
modified by this document. Output is this spec; an Implementer executes against it.

**One-line verdict:** The wiring requires **NO `engine.rs` change** — the byte-freeze
(`sha256 e50c7db1…48261`, `engine_equivalence` 9/9) **HOLDS**. The composer path already
carries **every** S13 diversity feature with byte-identical inputs, so S13 diversity is
**provably preserved** with **zero port work**. The whole change lives in `main.rs` at two
call sites. One decision point for the operator is the deliberate, documented tempo
**de-cap** (`character_tempo_bpm`), described in §11.

---

## 1. Current-state analysis (both code paths, with line refs)

### 1.1 The S13 legacy path that `play` / `render` are wired to TODAY

Both entry points build the engine and then call `set_features_global`:

- `render --wav`: `src/main.rs:286-287`
  ```rust
  let mut engine = PipelineEngine::new(mappings, engine_config);
  engine.set_features_global(&source.global_features());
  ```
  then `total_steps = source.step_count()` (`main.rs:289`), and lays decisions on a fixed
  grid `step_base_ms = step_idx * ms_per_step` (`main.rs:312`), where `ms_per_step` is the
  **config** value `engine_config.ms_per_step` (`main.rs:261`).

- `play`: `src/main.rs:523-524`
  ```rust
  let mut engine = PipelineEngine::new(mappings, engine_config);
  engine.set_features_global(&source.global_features());
  ```
  then `total_steps = source.step_count()` (`main.rs:627`), driving the live tick loop
  `for step_idx in 0..total_steps` (`main.rs:630`) over `engine.decide_step(&source, step_idx)`
  (`main.rs:653`). The adapter applies jitter + `Instant` scheduling. The per-step note
  offsets come from the decision; the **inter-step** cadence in `play` is governed by the
  `ev.offset_ms` carried in each decision (the engine's `ms_per_step`), not a main.rs grid
  multiply — see §1.3.

`set_features_global` (`src/engine.rs:382-441`) is the **S13-era derivation**:
- mode from HUE: `lookup_range_map(hue_to_mode, global.avg_hue)` (`engine.rs:383-385`);
- S13 tempo: `interp_tempo_bpm(brightness_to_tempo_bpm, avg_brightness)` →
  `config.ms_per_step` (`engine.rs:399-403`);
- S13 modal interchange: `brightness_drop = (0.5 - avg_brightness/100).clamp(0,1)*2`
  (`engine.rs:413`);
- S13 harmonic complexity feeds: raw `avg_saturation` + raw `hue_spread` passed into
  `generate_chords` (`engine.rs:424-427`);
- a **FLAT** phrase plan: `chord_engine.plan_phrases(&chords)` → `self.plan`
  (`engine.rs:431-434`). No sections, no form, no themes.

Net effect: NONE of the S14→S36 composition arc (`CompositionPlan`, sectioned non-looping
walk, returning theme, figuration/bass/prominence, affect bridge, C6.6 valence-family mode)
is reachable through the shipped binary. The last thing wired to `play` was S13.

### 1.2 The composer path (reachable today ONLY via `compose_from_image`)

`engine.compose_from_image(&understanding)` (`src/engine.rs:363-372`):
```rust
pub fn compose_from_image(&mut self, understanding: &ImageUnderstanding) -> bool {
    let Some(comp) = self.mappings.composition.clone() else { return false };   // :364-366
    let planner = CompositionPlanner::new(comp.into());                          // :367
    let plan = planner.plan(understanding, &self.mappings);                      // :368
    self.mode = plan.key_tempo.home_mode.clone();                               // :369
    self.set_plan(plan);                                                        // :370
    true                                                                        // :371
}
```
- The `ImageUnderstanding` input is produced by `pure_analysis::understand_image_pure(&img)`
  (`src/pure_analysis.rs:639`), which calls the **same** `analyze_global_pure(img)`
  (`pure_analysis.rs:642`) that `PureAnalysisSource::global_features()` returns
  (`pure_analysis.rs:1064-1066`).
- `CompositionPlanner::plan` (`src/composition.rs:1256-1494`) runs the form/character/meter/
  key-scheme ladders, expands the form, and fills per-section harmony via the **existing**
  `chord_engine` craft (`composition.rs:1424-1444`).
- `set_plan` (`engine.rs:349-357`) installs the plan into `self.composition: Option<CompositionPlan>`.
- Once installed, `decide_step` takes the **COMPOSE branch** (`engine.rs:543-579`):
  `comp.locate(step_idx)` resolves `(section, step_in_section)` with **NO modulo** — the
  engine cursor never wraps (`engine.rs:537-544`).

### 1.3 Cursor + tempo authority differs between the two installed states

This is the load-bearing integration detail:

| Quantity            | Legacy (`set_features_global`)                    | Composer (`set_plan`)                                              |
|---------------------|---------------------------------------------------|-------------------------------------------------------------------|
| Step count `N`      | `source.step_count()` == `num_steps`              | `comp.total_steps` = `steps_per_section * n_sections` (`composition.rs:1326`) — **independent of `num_steps`** |
| ms / step           | `config.ms_per_step` (one flat value)             | `section.ms_per_step` == `key_tempo.base_ms_per_step` (per-plan, brightness+character derived, `composition.rs:1319,1458`) |
| Cursor              | `step_idx % plan.len()` (loops)                   | non-looping `comp.locate(step_idx)` (`engine.rs:544`)             |
| Engine's own helper | `total_steps_or` returns `source.step_count()`    | `total_steps_or` returns `comp.total_steps` (`engine.rs:621-628`) |

`main.rs` TODAY hard-codes `total_steps = source.step_count()` and `ms_per_step = config`
(`main.rs:289/312/627`). After wiring, **both must come from the installed plan**, not from
the source or config. The engine already exposes both via the read-only accessor
`engine.composition() -> Option<&CompositionPlan>` (`engine.rs:341-343`):
`plan.total_steps` and `plan.key_tempo.base_ms_per_step`. **No new engine API is required.**

**Out-of-range scan rows are safe.** When `comp.total_steps > source.step_count()`,
`decide_step` still calls `source.scan_bar_features(step_idx, …)`; the pure source returns a
neutral zero-bar row via `unwrap_or_default()` for any `step_idx` past the precomputed steps
(`pure_analysis.rs:1073`, truncate/pad to `num_instruments`). So the per-bar texture simply
runs out and goes neutral past the scanned region — deterministic, never a panic. The plan's
structure (sections/harmony/theme) still drives those tail steps. (The OpenCV `PrecomputedSource`
and the `engine.rs` `CannedSource` follow the same `unwrap_or_default` discipline —
`engine.rs:844`.) This is the property that makes a plan longer than the scan legal.

---

## 2. S13-preservation reconciliation (the make-or-break investigation)

**Method:** read both code paths feature-by-feature and confirm the composer path derives an
equivalent of each S13 diversity feature from byte-identical inputs.

### 2.1 Input equivalence (the foundation)

`understand_image_pure` (`pure_analysis.rs:759-…`) field-copies from the **same**
`analyze_global_pure` result `g` that `global_features()` exposes:

| `GlobalFeatures` field (set_features_global input) | `ImageUnderstanding` field (plan input) | Equivalence |
|---|---|---|
| `avg_brightness` | `avg_brightness` (`pure_analysis.rs:769`) | identical copy |
| `avg_saturation` | `avg_saturation` (`pure_analysis.rs:770`) | identical copy |
| `avg_hue`        | `dominant_hue = g.avg_hue` (`pure_analysis.rs:737,763`) | identical copy |
| `hue_spread`     | `colorfulness = g.hue_spread` (`pure_analysis.rs:767`) | identical copy |
| `edge_density`   | `edge_activity = clamp(edge_density/0.05,0,1)` (`pure_analysis.rs:760`) | re-scaled, but `plan` re-multiplies by `EDGE_ACTIVITY_RANGE_MAX (0.05)` at `composition.rs:1429` to recover the raw `edge_density` it passes to `generate_chords` |

The four scalars that drive S13 diversity (`avg_brightness`, `avg_saturation`, `avg_hue`,
`hue_spread`) reach the planner **byte-for-byte** identical to what `set_features_global`
sees. This is the bedrock of the reconciliation.

### 2.2 Per-feature reconciliation TABLE

| S13 diversity feature | Where it lives today (`set_features_global`, `engine.rs`) | Composer-path equivalent (`plan()`, `composition.rs`) | Port action required |
|---|---|---|---|
| **Brightness→tempo continuous interpolation** | `interp_tempo_bpm(brightness_to_tempo_bpm, avg_brightness)` → `ms_per_step` (`engine.rs:399-403`) | `interp_tempo_bpm(mappings.global.brightness_to_tempo_bpm, u.avg_brightness)` (`composition.rs:1315`) — the two `interp_tempo_bpm` fns (`engine.rs:796`, `composition.rs:1863`) are **byte-identical source**; then `character_tempo_bpm` window (`composition.rs:1318`) → `base_ms_per_step` (`composition.rs:1319`) | **YES — carried** (with documented tempo de-cap, §11; default `character_tempo` window for `Ballad` = `{56,96}` reproduces the legacy clamp byte-for-byte, `composition.rs:370`) |
| **Modal-interchange `brightness_drop` trigger** | `brightness_drop = (0.5 - avg_brightness/100).clamp(0,1)*2` → `generate_chords` (`engine.rs:413,419`) | identical formula `(0.5 - u.avg_brightness/100).clamp(0,1)*2` → `generate_chords` (`composition.rs:1355,1430`) | **YES — carried**, character-for-character identical |
| **Saturation→harmonic complexity** | raw `avg_saturation` (0..100) → `generate_chords` arg 5 (`engine.rs:424`) | raw `u.avg_saturation` → `generate_chords` arg 5 (`composition.rs:1431`) | **YES — carried**, same raw scalar, same arg position |
| **hue_spread→colorfulness / mode-mixture widening** | raw `hue_spread` → `generate_chords` arg 6 (`engine.rs:427`) | raw `u.colorfulness` (== `g.hue_spread`) → `generate_chords` arg 6 (`composition.rs:1432`) | **YES — carried**, same raw scalar, same arg position |
| **Edge-density harmonic feed** | raw `edge_density` → `generate_chords` arg 4 (`engine.rs:418`) | `u.edge_activity * EDGE_ACTIVITY_RANGE_MAX` (recovers raw `edge_density`) → `generate_chords` arg 4 (`composition.rs:1429`) | **YES — carried**, value recovered to the same raw scale |
| **Mode-from-hue derivation** | `lookup_range_map(hue_to_mode, avg_hue)` (`engine.rs:383-385`) | `lookup_range_map(hue_to_mode, u.dominant_hue)` → `hue_mode`, then C6.6 `valence_family_mode` projection (`composition.rs:1294-1301`) | **YES — carried**; with no `mode_valence_cuts` block the C6.6 projection is a NO-OP and `home_mode` is the **byte-for-byte** legacy pure-hue mode (`composition.rs:1289-1292`). With cuts present, C6.6 is the **intended NEW** behavior S37 makes audible. |
| **Voice-leading inside the plan** | `plan_phrases` runs `voice_lead_sequence` internally (`engine.rs:429-431`) | `plan_phrases` per section (`composition.rs:1444`) — same fn, same internal voice-leading | **YES — carried** |

**Finding: ALL-CARRIED. Zero port work.** Every S13 diversity feature has an exact equivalent
on the composer path, fed from byte-identical image scalars. No feature is missing; therefore
no "port to X" action is needed. A regression of the heard S13 diversity is **not possible**
from this wiring, because the composer derives the same tempo/`brightness_drop`/saturation/
hue_spread/edge/mode quantities the S13 path did, and additionally layers the sectioned form
on top.

### 2.3 Why the `diversity_s13` test stays green regardless

`tests/diversity_s13.rs` exercises `set_features_global` + `chord_engine` **directly**
(`tests/diversity_s13.rs:129` calls `engine.set_features_global`, the harmony asserts drive a
fixed progression through `realize_step`, never through `main.rs`). Wiring `main.rs` to the
composer touches **neither** `set_features_global` (it remains a public fn, still called by
`tests/tui_render.rs:426` and the test harness) **nor** the chord engine. The S13 regression
net is therefore unmoved. Diversity is preserved both by code (§2.2) AND by an untouched test.

---

## 3. The exact `main.rs` call-sequence change (before / after)

The same shape applies to both sites. The pattern is: build the engine, run the composer if a
`composition` block is present, and read `total_steps` + `base_ms_per_step` back from the
installed plan; fall back to the legacy `set_features_global` path when `compose_from_image`
returns `false` (no `composition` block in `mappings.json`).

### 3.1 `play` — `src/main.rs:522-524`

**BEFORE**
```rust
// ── Build the engine + feed global features (the engine derives mode/plan) ──
let mut engine = PipelineEngine::new(mappings, engine_config);
engine.set_features_global(&source.global_features());
println!("Engine mode: {}", engine.current_state().mode);
```
…and **BEFORE** at `main.rs:627`:
```rust
let total_steps = source.step_count();
```

**AFTER** (signatures/types only — Implementer writes the bodies)
```rust
// ── Build the engine + install the COMPOSER plan (S37). The plan-first composer is
//    the audible path; the S13 flat path is the fallback when mappings.json has no
//    `composition` block (compose_from_image -> false). ──
let mut engine = PipelineEngine::new(mappings, engine_config);

// understand_image_pure reads the SAME analyze_global_pure stats global_features() exposes;
// `img` is the already-loaded RgbImage from the #[cfg(not(feature="opencv"))] block above.
// (OpenCV path: build the ImageUnderstanding from the same image, see §4.)
let understanding = audiohax::pure_analysis::understand_image_pure(&img)?;
let composed: bool = engine.compose_from_image(&understanding);
if !composed {
    // No `composition` block in mappings.json -> keep the S13 flat path, byte-identical
    // to the pre-S37 binary.
    engine.set_features_global(&source.global_features());
}
println!("Engine mode: {}", engine.current_state().mode);

// `total_steps` and the playback ms-grid now come FROM THE PLAN when composing, NOT from
// source.step_count()/config. Read them back via the read-only accessor (engine.rs:341).
let (total_steps, _plan_ms_per_step): (usize, u64) = match engine.composition() {
    Some(plan) => (plan.total_steps, plan.key_tempo.base_ms_per_step),
    None => (source.step_count(), ms_per_step), // legacy fallback
};
```

**Notes for `play`:**
- The live loop body at `main.rs:630-708` is **unchanged** except that its `total_steps`
  binding (`main.rs:627`) is replaced by the `total_steps` above. The loop already pulls
  `engine.decide_step(&source, step_idx)` (`main.rs:653`); in compose mode that takes the
  non-looping COMPOSE branch automatically.
- `play` schedules each note from `ev.offset_ms`/`ev.hold_ms` carried in the decision
  (`main.rs:660-684`) — these come from the engine's per-section `ms_per_step`
  (`engine.rs:565`), so the **plan's** tempo is already honored inside the decision; `play`
  does NOT need `_plan_ms_per_step` for note timing. It is bound here only for parity with
  `render` and possible diagnostics; mark it `_`-prefixed if unused to avoid a warning.
- The `understand_image_pure` call must be placed **after** `img` is in scope. In the pure
  build `img` is created at `main.rs:501` inside the `#[cfg(not(feature="opencv"))]` source
  block; the Implementer must hoist `img` out of that block (or compute `understanding`
  inside it and carry it forward) so it is live at the engine-build site. See §4 for the
  OpenCV arm.

### 3.2 `render --wav` — `src/main.rs:286-289`

**BEFORE**
```rust
let mut engine = PipelineEngine::new(mappings, engine_config);
engine.set_features_global(&source.global_features());

let total_steps = source.step_count();
```
…and the grid multiply **BEFORE** at `main.rs:312`:
```rust
let step_base_ms = step_idx as u64 * ms_per_step;
```

**AFTER**
```rust
let mut engine = PipelineEngine::new(mappings, engine_config);

let understanding = audiohax::pure_analysis::understand_image_pure(&img)?;
let composed: bool = engine.compose_from_image(&understanding);
if !composed {
    engine.set_features_global(&source.global_features());
}

// total_steps AND the deterministic ms-grid come from the plan when composing.
let (total_steps, grid_ms_per_step): (usize, u64) = match engine.composition() {
    Some(plan) => (plan.total_steps, plan.key_tempo.base_ms_per_step),
    None => (source.step_count(), ms_per_step),
};
```
…and the grid multiply **AFTER** at `main.rs:312` uses `grid_ms_per_step`:
```rust
let step_base_ms = step_idx as u64 * grid_ms_per_step;
```

**`render` vs `play` — the one material difference:** `render` lays notes on an **absolute
ms grid** computed in `main.rs` itself (`step_base_ms = step_idx * ms_per_step`,
`main.rs:312`). In compose mode the per-step cadence is the **plan's** `base_ms_per_step`, so
`render` MUST swap its grid multiplier from the config `ms_per_step` to
`grid_ms_per_step` (above), or every step will be spaced at the wrong (config) tempo while the
notes themselves carry plan-tempo holds — a tempo/spacing mismatch and a determinism break.
`play` has no such grid (it schedules from `ev.offset_ms` relative to a per-step `t0`,
`main.rs:657,669`), so `play` needs only the `total_steps` swap. **This asymmetry is the single
most important wiring detail and must be in the Implementer's checklist.**

---

## 4. OpenCV-arm parity (do not strand the `#[cfg(feature="opencv")]` build)

`understand_image_pure` takes an `image::RgbImage` (`pure_analysis.rs:639`). The OpenCV source
arm builds an OpenCV `Mat`, not an `RgbImage` (`main.rs:265-271`, `main.rs:455-461`). Two
acceptable options for the Implementer (Architect recommends Option A):

- **Option A (preferred, smallest blast radius):** gate the composer install behind
  `#[cfg(not(feature="opencv"))]` and keep the OpenCV arm on the existing
  `set_features_global` legacy path for now. The default build on this box is pure-Rust
  (no OpenCV), so the composer becomes audible on the shipped/default binary immediately;
  OpenCV stays byte-stable. Document the OpenCV composer arm as a follow-up.
- **Option B:** add a `Mat → RgbImage` adapter (pure conversion, no music logic) so the
  OpenCV arm can also call `understand_image_pure`. This is an Implementer task in a NEW
  helper (e.g. `src/opencv_bridge.rs` or an existing OpenCV-only module), **not** in
  `engine.rs`, and must respect the module boundary (image analysis only). Larger surface;
  defer unless OpenCV parity is required this session.

The before/after in §3 is written for the pure arm. If Option A is chosen, the
`compose_from_image` install block is itself `#[cfg(not(feature="opencv"))]` and the OpenCV
arm retains the literal pre-S37 `set_features_global` lines.

---

## 5. `single_section_default` is NOT on the live composed path (freeze witness intact)

`StepContext::single_section_default` (`composition.rs:1174`, `engine.rs:586`) is built ONLY
inside `decide_step`'s **LEGACY FLAT branch** (`engine.rs:581-597`) — the branch reached when
`self.composition.is_none()`. The instant `set_plan` installs a plan
(`engine.rs:356`, via `compose_from_image`), `decide_step` takes the COMPOSE branch
(`engine.rs:543-569`), which builds its context via `StepContext::with_prev`
(`engine.rs:551`), NOT `single_section_default`. So:

- The live composed `play`/`render` path **never** constructs `single_section_default`.
- The `engine_equivalence` golden sweep (`tests/engine_equivalence.rs:150,167,…,388`) builds
  `single_section_default` **directly in the test** over a FIXED `&[StepPlan]` — it does not
  go through `main.rs` at all and is independent of which path `play` is wired to. The byte
  witness is untouched.
- Wiring `play` to the real plan therefore **cannot** disturb `single_section_default` or its
  9/9 goldens. Confirmed.

---

## 6. The engine.rs FREEZE verdict

**VERDICT: NO engine.rs touch required. The freeze holds.**

The wiring lives entirely in `main.rs`, calling only **existing public** engine functions:
- `PipelineEngine::new` (existing),
- `compose_from_image(&ImageUnderstanding) -> bool` (existing, `engine.rs:363`),
- `composition() -> Option<&CompositionPlan>` (existing read-only accessor, `engine.rs:341`),
- `decide_step` (existing, `engine.rs:527`),
- `set_features_global` (existing, retained as the fallback, `engine.rs:382`),
- `current_state` (existing, `engine.rs:662`).

…plus one existing public function in `pure_analysis.rs`
(`understand_image_pure`, `pure_analysis.rs:639`). No new engine type, no new engine method,
no change to any decision kernel or golden derivation. The goldens
(240ms/114/84/36/79) **do not move** because no code they depend on is edited.

Because the answer is NO, the YES branch (minimal engine.rs change, golden re-derivation,
operator freeze-break gate) does **not apply** and is not invoked. The Implementer must be
instructed: **if you find yourself editing `engine.rs`, STOP — the design says you should not
need to; re-read §3/§6 and escalate, because that is an unplanned freeze break.**

---

## 7. File-ownership split for the build

| File | Owner | What changes | Disjoint? |
|---|---|---|---|
| `src/main.rs` | **Implementer** | The §3 call-sequence swap at both sites (`play` ~`:522-524,:627`; `render` ~`:286-289,:312`), the `img` hoist (§3.1 note / §4), the OpenCV gating choice (§4). | sole writer |
| `src/engine.rs` | **frozen — NOBODY** | none (§6). | n/a |
| `src/composition.rs` | **Music Theory — NOT required this build** | none. All chord/mode logic the composer needs already exists and is already called by `plan()` (`composition.rs:1424-1444`). No chord/mode logic must move. | no write |
| `assets/mappings.json` | **Music Theory — OPTIONAL** | Only if the operator wants the C6.6 valence-family mode and/or character tempo windows AUDIBLE (add `composition.affect.mode_valence_cuts` / tune `character_tempo`). With these ABSENT, the composer reproduces legacy mode + the Ballad `{56,96}` tempo clamp byte-for-byte (§2.2, §11). | Music Theory only |
| `tests/*` | **Implementer** | ADD the runtime-reachability test (§8) in a NEW test file; touch no existing golden. | new file only |

**Music Theory does NOT need to move any chord or mode logic** for S37. The build is
file-disjoint: Implementer owns `main.rs` + the new test; Music Theory's involvement is the
optional `mappings.json` data tuning that decides whether C6.6/character-tempo are heard.

---

## 8. Runtime-reachability test (so play/render can never silently un-wire again)

The regression S37 fixes is "the binary's entry points stopped calling the composer." A test
must assert the entry points DRIVE THE PLAN, not just that the composer exists. Put it in a
NEW file, e.g. `tests/runtime_reachability_s37.rs`. It must assert, **without** going through
`main()` (which is hard to call), the contract the §3 wiring establishes:

1. **A `composition` block in `mappings.json` ⇒ the engine installs a plan.** Build a
   `PipelineEngine` from the real `assets/mappings.json`, call
   `compose_from_image(&understand_image_pure(test_img))`, assert it returns `true` AND
   `engine.composition().is_some()`. This is the exact call `main.rs` now makes; if a future
   refactor drops the `composition` block or breaks `compose_from_image`, this fails.

2. **The drive count comes from the plan, not the scan.** Assert
   `engine.composition().unwrap().total_steps == engine.composition().unwrap()` total (i.e.
   `steps_per_section * n_sections`) and that it is the value `main.rs` must loop to —
   pin it `!= source.step_count()` for a fixture whose plan length differs from `num_steps`,
   proving the loop bound is plan-derived. (Guards against a regression that reverts
   `total_steps` back to `source.step_count()`.)

3. **Two visibly different images ⇒ different installed plans.** Compose from a
   calm/dark image and a vivid/busy image; assert the two `CompositionPlan`s differ in at
   least one audible spine field (`key_tempo.base_ms_per_step` OR `key_tempo.home_mode` OR
   `total_steps`). This is the END-TO-END echo of `diversity_s13` but proved THROUGH the
   composer install path, locking S13 diversity to the path the binary now uses.

4. **(Strongest, optional) a thin `play`/`render` seam smoke.** If the Implementer extracts
   the engine-build-and-install block into a small testable helper fn in `main.rs`'s lib
   surface (e.g. `audiohax::run_support::build_composed_engine(mappings, cfg, &img) ->
   (PipelineEngine, usize)`), assert that helper returns an engine with
   `composition().is_some()` and the plan-derived `total_steps`. This makes the wiring itself
   unit-addressable so an un-wire is caught at the seam, not only at the data layer. Extracting
   the helper is an Implementer call; if done, it lives in a NEW module, never `engine.rs`.

The minimum bar is assertions 1–3; assertion 4 is the belt-and-suspenders that most directly
prevents a silent un-wire.

---

## 9. Data flow (after wiring)

```
                 assets/mappings.json (has `composition` block)
                          │
   img:RgbImage ──────────┼─────────────────────────────────────────────┐
        │                 │                                              │
        ▼                 ▼                                              ▼
 understand_image_pure   PureAnalysisSource::extract            PipelineEngine::new
 (pure_analysis.rs:639)  (per-bar scan rows + global)           (engine_config)
        │ ImageUnderstanding        │ FeatureSource                      │
        └──────────────┐           │                                    │
                       ▼           │                                    │
        engine.compose_from_image(&understanding)  ◄────────────────────┘
                       │  (composition.rs::plan() — form/sections/themes,
                       │   S13 tempo+brightness_drop+sat+hue_spread carried,
                       │   C6.6 valence-family mode)
                       ▼
            engine.composition() = Some(CompositionPlan)
                       │  total_steps, key_tempo.base_ms_per_step
        ┌──────────────┴───────────────┐
        ▼ play                          ▼ render --wav
  for s in 0..plan.total_steps    for s in 0..plan.total_steps
    engine.decide_step(src, s)      engine.decide_step(src, s)
      → COMPOSE branch (no modulo)    → COMPOSE branch
    adapter: jitter + Instant       grid: s * base_ms_per_step (NOT config)
      → AudioSink (synth/MIDI)        → TimedMidiEvent → WAV
```
If the `composition` block is ABSENT, `compose_from_image` returns `false`, the `if !composed`
arm calls `set_features_global`, `engine.composition()` is `None`, and both entry points fall
back to `source.step_count()` + config `ms_per_step` — the literal pre-S37 behavior.

---

## 10. Risks & trade-offs

1. **`render` grid-tempo mismatch (HIGH if missed).** If the Implementer swaps `total_steps`
   but forgets to swap the `step_base_ms` multiplier (`main.rs:312`) from `ms_per_step` to
   `grid_ms_per_step`, render spaces steps at config tempo while notes hold at plan tempo →
   wrong cadence + a determinism shift. §3.2 calls this out; the runtime-reachability test
   (§8) does not catch it (it asserts plan install, not WAV cadence). Mitigation: add an
   assertion that the render grid multiplier equals `plan.key_tempo.base_ms_per_step` when
   composing, or a small WAV-length sanity check.
2. **`img` lifetime/scope in `main.rs` (MEDIUM).** `understand_image_pure(&img)` needs `img`
   live at the engine-build site; today `img` is local to the source block (`main.rs:501`).
   Hoisting is mechanical but must not change the source-extraction path. Pure-arm only.
3. **OpenCV arm strand (MEDIUM).** Option A (§4) leaves OpenCV on the legacy path — acceptable
   because default is pure-Rust, but it means the two builds diverge in behavior until a
   `Mat→RgbImage` bridge lands. Must be documented, not silent.
4. **Plan longer than scan (LOW — handled).** `comp.total_steps` can exceed
   `source.step_count()`; tail steps get neutral per-bar rows (`pure_analysis.rs:1073`).
   Deterministic, no panic. Only a perceptual note: very long plans over a short scan will run
   structure past the image's texture data. Acceptable for S37; tunable via
   `BASE_STEPS_PER_SECTION` later (Music Theory, mappings).
5. **Tempo de-cap surprise (LOW — operator gate, §11).** The composer can produce tempos
   OUTSIDE the legacy 56–96 BPM Ballad clamp when a non-Ballad character + window is selected.
   This is intended de-capping, but it is an audible change vs S13; surfaced as a decision
   point, not a defect.

---

## 11. DECISION POINT for the operator — the tempo de-cap

The one place the composer can legitimately produce a DIFFERENT tempo than `set_features_global`
is `character_tempo_bpm` (`composition.rs:372`, called at `:1318`):

- `set_features_global` always clamps brightness→BPM to the single legacy Ballad window
  (effectively 56–96, the S13 operating point).
- `plan()` selects a CHARACTER from the image, then clamps within **that character's** tempo
  window (`composition.rs:1316-1319`). The default `affect.character_tempo` map seeds `ballad
  = {56,96}` (`composition.rs:370,225`), so a Ballad image is byte-for-byte identical to S13;
  but a March/Scherzo/Gigue/etc. image can resolve to a faster window and a genuinely faster
  tempo.

**This is not a regression of S13 diversity** (every S13 axis is preserved, §2.2) — it is an
ADDITIONAL, intended degree of freedom S37 makes audible. The operator decision is simply:

> Ship S37 with the character-tempo de-cap AUDIBLE (richer, can exceed 96 BPM), or pin all
> characters to the Ballad window for a conservative first wiring (set every
> `character_tempo` window to `{56,96}` in `mappings.json`, byte-stable vs S13 tempo)?

No `engine.rs` or code change either way — it is a `mappings.json` data choice owned by Music
Theory. Default (do nothing) = de-cap audible. Recommend shipping the de-cap audible (it is the
whole point of making the composer heard), but surface it so it is a choice, not an accident.

(Separately, the C6.6 `valence_family_mode` projection is audible iff
`composition.affect.mode_valence_cuts` is populated; absent ⇒ legacy pure-hue mode byte-for-byte,
§2.2. That is also a Music Theory `mappings.json` choice, not a code gate, and is part of "C6.6
audible" in the acceptance criteria.)

---

## 12. Acceptance criteria

S37 is DONE when ALL hold:

1. **Composer is what `play` renders.** With a `composition` block present, `play`
   (`main.rs`) installs a `CompositionPlan` (`engine.composition().is_some()`) and drives
   `0..plan.total_steps`; the audible output is the sectioned, themed composition, not the S13
   flat plan. Same for `render --wav` (with `step_base_ms` using `base_ms_per_step`).
2. **S13 diversity provably preserved.** Two visibly different images differ through the
   composer path in ≥3 musical dimensions (the §8 assertion 3 + the unchanged
   `tests/diversity_s13.rs` green), because tempo / `brightness_drop` / saturation / hue_spread
   / edge / mode are all carried (§2.2 ALL-CARRIED).
3. **C6.6 audible.** When `mode_valence_cuts` is populated, valence projects the hue-mode into
   the major/minor family it demands through the binary (the composer's `home_mode` flows to
   `play`/`render`); when absent, mode is byte-for-byte legacy.
4. **Freeze intact.** `engine.rs` is unmodified; `engine_equivalence` 9/9 green; the
   240ms/114/84/36/79 goldens unmoved; `single_section_default` off the live composed path.
5. **Fallback intact.** With NO `composition` block, both entry points reproduce the pre-S37
   binary (legacy `set_features_global` + `source.step_count()` + config `ms_per_step`),
   byte-for-byte.
6. **Un-wire guarded.** The §8 runtime-reachability test exists and fails if a future change
   reverts `play`/`render` off the composer or back to a `source.step_count()` drive bound.

---

## 13. Implementer checklist (one screen)

- [ ] `play` (`main.rs:~522-524`): insert `understand_image_pure` + `compose_from_image` +
      `if !composed { set_features_global }`; hoist `img` into scope.
- [ ] `play` (`main.rs:627`): replace `total_steps = source.step_count()` with the
      `match engine.composition()` plan-derived bind.
- [ ] `render` (`main.rs:~286-289`): same install block.
- [ ] `render` (`main.rs:289` + `:312`): plan-derived `total_steps` AND swap the
      `step_base_ms` multiplier to `grid_ms_per_step` (**do not forget the multiplier**).
- [ ] OpenCV arm: choose Option A (gate composer `#[cfg(not(opencv))]`) or Option B (Mat→RgbImage bridge in a NEW non-engine module).
- [ ] NEW `tests/runtime_reachability_s37.rs`: assertions 1–3 (+ optional 4).
- [ ] DO NOT touch `engine.rs`. If you think you must — STOP and escalate (§6).
- [ ] `export PATH="$HOME/.cargo/bin:$PATH"; cargo build --release && cargo test` green,
      including `engine_equivalence`, `diversity_s13`, the new reachability test.
- [ ] Operator decision (§11): character-tempo de-cap audible vs pinned (mappings.json, Music Theory).

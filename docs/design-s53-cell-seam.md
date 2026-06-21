# Design — S53 / fix-direction-2 SLICE 1: Un-gate the rhythm-cell axis to run PER-PIECE

**Status:** DESIGN ONLY (seam spec). No source modified. engine.rs FROZEN and untouched by this design.
**Arc:** fix-direction-2 (within-piece rhythmic identity). This is **Slice 1 = D-CELL**. Slice 2 = D-METER is DEFERRED (thin forward note §7 only).
**HEAD verified against:** `64b6883b058a55987c11a7e4eb0da8234babc51c`
**engine.rs freeze witnessed:** `sha256 = e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261` (matches the freeze anchor).
**Author role:** Rust Architect (design only — signatures, contracts, no implementation bodies).

---

## 0. Anchor re-verification (the kickoff prose had mismatches — these are the TRUE anchors)

Every anchor below was re-read against current HEAD. The upstream kickoff prose mis-cited several; the corrected anchors are authoritative for this spec.

| Item | Kickoff cited | **Verified at HEAD** | Note |
|---|---|---|---|
| `pick_rhythm_cell` def | chord_engine.rs:1806 | **`src/composition.rs:1806`** (signature 1806–1810; body to :1826) | Kickoff file was WRONG; line was right. |
| Theme gate | composition.rs:1478 | **`src/composition.rs:1478`** (`let themes = if theme_behaviour == "absent" || !needs_theme`) | Confirmed. |
| `pick_rhythm_cell` call site | composition.rs:1490 | **`src/composition.rs:1490`** (inside the `else` branch, :1480–1498) | Confirmed — call is inside the theme `else`. |
| `theme_behaviour` select | composition.rs:1409 | **`src/composition.rs:1411`** (`self.plan_mappings.theme_behaviour.select(u)`) | Off by 2 (1409 is the `character` select). |
| `theme_behaviour` mapping rule | mappings.json:161–164 | **`assets/mappings.json:120–123`** | Lines 161–164 are the **character** rules. The TRUE rule: `{"knob":"complexity","op":"ge","lo":0.4}` ⇒ `"fragment"`, default `"absent"`. |
| `rhythm_cells` vocabulary | chord_engine.rs:3241 | **`src/chord_engine.rs:3241`** (def 3241; cells 3242–3314; `rhythm_cell_count` :3331) | Confirmed. K = 4 for all 8 archetypes. |
| S50 reverted-const region | composition.rs:1770–1792 | **`src/composition.rs:1770–1792`** (`CELL_EDGE_BROAD/BUSY`, `CELL_COMPLEXITY_PROFILED`) | Confirmed. |
| `band_activity_spread` | chord_engine.rs:1097 | **`src/chord_engine.rs:1097`** | Confirmed. Reuse candidate — see §2.4. |
| SelectTable / Knob infra | composition.rs:817–839 | **`Knob::read` :815–841; `SelectTable` :899–918; `Predicate`/`CmpOp` :844–891`** | Confirmed. |

---

## 1. CURRENT STATE ANALYSIS

### 1.1 The exact theme-gate coupling (the dormancy)

`CompositionPlanner::plan` (`composition.rs:1395`) selects `theme_behaviour` at **:1411** from the `theme_behaviour` SelectTable, which is loaded from `assets/mappings.json:120–123`:

```jsonc
"theme_behaviour": {
  "default": "absent",
  "rules": [ { "when": [ {"knob":"complexity","op":"ge","lo":0.4,"hi":0.0} ], "pick": "fragment" } ]
}
```

The theme block at **:1477–1498**:

```rust
let needs_theme = form_spec.sections.iter().any(|s| s.theme.is_some());
let themes = if theme_behaviour == "absent" || !needs_theme {
    Vec::new()                                   // ← REAL PHOTOS LAND HERE
} else {
    let archetype = pick_archetype(u);
    let range_degrees = (2.0 + u.edge_activity * 5.0).round() as u8;
    let length_steps  = (3.0 + u.complexity * 5.0).round() as usize;
    let cell_count = archetype.rhythm_cell_count();
    let cell_index = pick_rhythm_cell(u, archetype, cell_count);   // ← :1490 ONLY call site
    let motif = chord_engine::resolve_motif_celled(archetype, range_degrees, length_steps, cell_index);
    vec![ThemeSeed { id: 0, motif }]
};
```

**The dormancy chain (verified):**

1. `pick_rhythm_cell` is called at **exactly one site**: composition.rs:1490 (`grep` over the whole tree confirms — the only other mentions are the def at :1806 and comment lines :1771/:1787/:1800).
2. That site is inside the `else` branch of the theme gate at :1478.
3. The `else` is reached only when `theme_behaviour != "absent"` **and** `needs_theme`.
4. `theme_behaviour == "fragment"` requires `complexity >= 0.4` (mappings.json:122).
5. Real photos cluster `complexity` ≈ 0.005–0.23 and **never reach 0.4**, so the `if` branch (`Vec::new()`) is always taken on real images.
6. ⇒ `pick_rhythm_cell` is **unreachable** for any real photo. The cell axis is not force-pinned; it is dead code on the real path.

### 1.2 The data flow today: feature → theme gate → pick_rhythm_cell → realization

```
ImageUnderstanding.complexity ──► theme_behaviour SelectTable (:1411)
                                         │
                          "absent" (real photos, complexity<0.4) ──► themes = Vec::new()
                                         │                                    │
                          "fragment" (synthetic, complexity>=0.4)             │
                                         ▼                                    │
                    pick_rhythm_cell(u, archetype, K) (:1490)                 │
                                         ▼                                    │
                    resolve_motif_celled(...) ──► ThemeSeed { motif }         │
                                         ▼                                    ▼
                    plan.themes = vec![seed]                       plan.themes = []
                                         │                                    │
                                         ▼                                    ▼
               engine.rs:545 section.theme.and_then(...) ──► ctx.theme = Some / None
                                         │
                                         ▼
               chord_engine realizer reads ctx.theme for the melodic-subject replay;
               the PER-STEP RHYTHM of a no-theme image comes from the BAND LADDER
               (realize_rhythm :2052, band_activity_spread :1097), NOT from any cell.
```

**The architecturally load-bearing fact:** the rhythm cell has **no carrier into the engine other than `ThemeSeed.motif`** (a fully-resolved `Vec<MotifNote>`). On the no-theme path `plan.themes == []`, `ctx.theme == None`, and the realizer's rhythm is entirely band-ladder-driven. So "run the cell per-piece" is **not** simply "call `pick_rhythm_cell` outside the gate" — the call has nowhere to deposit its result that the realizer will read on a no-theme image. **A new per-piece carrier is required.** This is the central seam decision of this slice (§2).

### 1.3 The S50 revert trap (why the cuts are NOT the lever)

`composition.rs:1770–1792` documents the reverted consts:

```rust
const CELL_EDGE_BROAD: f32 = 0.33;          // pre-S50 value, restored
const CELL_EDGE_BUSY:  f32 = 0.66;          // pre-S50 value, restored
const CELL_COMPLEXITY_PROFILED: f32 = 0.66; // pre-S50 value, restored
```

`pick_rhythm_cell` (:1806–1826) uses `CELL_COMPLEXITY_PROFILED` as the SECONDARY divert: `if u.complexity >= CELL_COMPLEXITY_PROFILED && cell_count > 3 { 3 }`.

The S50 attempt lowered `CELL_COMPLEXITY_PROFILED` to ~0.20 to bring the cell axis into the real-photo `complexity` band. But the divert lives **behind** the same theme gate, which requires `complexity >= 0.4`. So **every** image that even reaches `pick_rhythm_cell` already has `complexity >= 0.4`; with the PROFILED cut at 0.20, the condition `complexity >= 0.20` is satisfied for **all** of them, force-pinning every themed image onto cell 3 and killing the cells 0/1/2 edge ramp. Net benefit on the six bundled images: zero. It was **reverted**.

**Lesson encoded in the spec:** the lever is the **theme-gate coupling** (the cell only ever sees `complexity >= 0.4` images), NOT the cut-points. Lowering cuts inside a gate that pre-filters to `complexity >= 0.4` cannot reach the real-photo cluster. **Slice 1 must break the coupling — decouple `pick_rhythm_cell` from the theme path — not re-tune cuts.**

### 1.4 The realizer seam precedent (how a per-piece value reaches the realizer without touching engine.rs)

`realize_rhythm` (`chord_engine.rs:2052`) already reads a **per-section planner-set scalar zero-copy off the borrowed context**:

```rust
// chord_engine.rs:2092
let density_nudge = (ctx.section.density - 0.5) * DENSITY_ACTIVITY_GAIN;
```

`ctx.section` is `StepContext.section: &'a Section`. The engine builds `ctx` once per step (`engine.rs:551 StepContext::with_prev(...)`) and passes it through to the realizer; **the engine never reads `section.density` itself** — the planner writes it and the realizer reads it. This is the proven pattern (the S29 comment at :2088–2091 names it explicitly: *"Read zero-copy off the already-borrowed `ctx` — no new field, no seam change"*). The per-piece rhythmic motto rides the **identical** seam: a new field on `Section`, written by the planner, read by `realize_rhythm` through `ctx.section`. **engine.rs is a pure pass-through and is not edited.**

---

## 2. PROPOSED CHANGE — the per-piece decouple (path b, confirmed sound)

### 2.1 The seam, in one sentence

Add a per-piece **rhythmic motto** — a `(MotifArchetype, cell_index)` selection — computed once per plan from robust per-image features **independent of the theme gate**, stamped onto every `Section`, and read by `realize_rhythm` through `ctx.section` to bias the band-ladder onset placement. `pick_rhythm_cell` stops being theme-path-only; it (or its successor `pick_piece_cell`) runs on **every** image.

### 2.2 What MOVES / CHANGES (the call site)

- **The call at composition.rs:1490 does NOT move out of the `else` and become the per-piece selector directly** — it stays where it is for the *themed* (synthetic) path so the themed motif keeps its cell. Instead, a **new per-piece selection** is computed ONCE in `plan()` **before** the section-expansion loop (alongside the other once-per-plan resolves at :1517/:1533/:1538), unconditionally, and stamped onto each `Section`.
- Rationale for not relocating the single call: the themed path resolves the cell **into a concrete `Vec<MotifNote>`** (`resolve_motif_celled`), which is the right representation for a melodic subject. The per-piece path does **not** want a resolved subject motif — real photos have no theme to replay; they want a **rhythmic motto** (an archetype + cell *identity*) that biases the band ladder. These are two consumers of the same `pick_*_cell` logic with different downstream needs. The selection **logic** is shared; the **carrier** differs.

### 2.3 The per-piece selection DRIVER (design decision — proposed, with music rationale)

**Constraint recall (from §1.4 / ImageUnderstanding audit):** many `ImageUnderstanding` fields are **slice-1-pinned defaults** and carry NO per-image signal — `palette_bimodality (=0)`, `quadrant_contrast (=0)`, `fg_bg_contrast (=0)`, `vertical_emphasis (=0.5)`, `subject_size (=1)`, and all `subject_*` (= whole-image). Driving the motto off any of these would re-pin every photo to one cell. They are **excluded**.

The features that **real photos actually vary across** (live, pixel-derived) are: `edge_activity`, `texture`, `complexity`, `colorfulness`, `avg_brightness`, `avg_saturation`, and the planner-derived composites `affect_arousal` / `affect_valence`.

**Proposed driver (primary + secondary, mirroring the existing two-axis shape but with corrected reachability):**

- **PRIMARY axis = `edge_activity`** — rhythmic energy / onset density. This is the music-correct primary for a rhythmic motto (visual activity → onset density is the project's standing affect bridge, see `band_activity_spread` theory at chord_engine.rs:1082). It selects along the BROAD→BUSY density ramp (cells 1/0/2) — **the cell vocabulary's cells 0/1/2 are authored as exactly this ramp** (chord_engine.rs:3245–3247 doc), so the existing cuts `CELL_EDGE_BROAD=0.33` / `CELL_EDGE_BUSY=0.66` are reused **unchanged** but now applied to the SPREAD edge_activity (see §2.4) so the compressed real-photo band actually spans them.
- **SECONDARY axis = `affect_arousal`** (NOT raw `complexity`) for the cell-3 PROFILED/SYNCOPATED divert. Rationale: (a) `complexity` is the very feature that clusters 0.005–0.23 and never discriminates — using it as the divert key re-creates the S50 dead-axis; (b) `affect_arousal` is a *composite* (`0.45·avg_saturation + 0.25·colorfulness + 0.20·edge_activity + 0.10·complexity`, mappings.json:139–143) that spreads real photos across a usable 0..1 range and is **already computed** in `plan()` at :1399–1402 before the section loop, so it is free to read. A high-arousal image (saturated, colorful, busy) takes the characteristic syncopated gait (cell 3) — the decorrelating tiebreak that lets two equal-`edge_activity` images split gaits on their *affective* character, which is musically the right axis for "which rhythmic personality."

> **DECISION POINT for the lead (taste/affect call):** the secondary-divert key — `affect_arousal` (proposed) vs `texture` vs a `colorfulness`/`avg_saturation` pair. The Music Theory Specialist / taste-affect reviewer should confirm `affect_arousal` is the right "rhythmic personality" axis, and confirm the cut value (a new `PIECE_AROUSAL_PROFILED` const; **not** `CELL_COMPLEXITY_PROFILED`, which is the themed-path const and stays as-is for the synthetic path). This is a TASTE call (DP-A class, same as the original cell cuts at :1760–1763).

### 2.4 `band_activity_spread` reuse (the cluster re-expansion)

`band_activity_spread` (chord_engine.rs:1097) linearly re-expands the compressed real-photo `edge_activity` sub-band about its centroid (identity at center; asymmetric slope). Today it lives in `chord_engine` and is applied to the band-ladder comparison input. The PRIMARY axis of the per-piece selector should compare **`band_activity_spread(u.edge_activity)`** against `CELL_EDGE_BROAD`/`CELL_EDGE_BUSY`, not raw `edge_activity` — otherwise the same 0.30–0.51 compression that motivated the spread for the band ladder will compress the cell selection into one or two cells. Reusing the existing helper (rather than a new stretch) keeps the cell motto and the band ladder reading **the same re-expanded activity**, so the rhythmic motto and the realized onset density agree.

> **Module-boundary note:** `band_activity_spread` is a private free fn in `chord_engine`. The per-piece selector lives in `composition.rs`. Two clean options for the Implementer, both inside module boundaries (image analysis stays out of music; the selector IS music-craft and may live in `chord_engine`):
> - **(b-i, preferred)** Move the per-piece selection logic itself into `chord_engine` as a pure free fn `chord_engine::pick_piece_cell(...)` that internally calls `band_activity_spread`, and `composition.rs` calls it (exactly as it already calls `chord_engine::resolve_motif_celled`). The cuts then live beside the vocabulary they index. **This is the recommended home — it co-locates the cell cuts with the cell vocabulary and reuses the spread without a cross-module export.**
> - **(b-ii)** Make `band_activity_spread` `pub(crate)` and keep the selector in `composition.rs`. Smaller diff, but splits the cuts from the vocabulary.
> Recommend **b-i**. Flag for the lead.

### 2.5 Complete Rust signatures (NO bodies)

**NEW — the per-piece cell selector (recommended home: `chord_engine.rs`, beside the vocabulary).** Mirrors `pick_rhythm_cell` but keys on the per-piece driver and applies the spread internally:

```rust
/// Select the PER-PIECE rhythmic-motto cell (`0..cell_count-1`) for `archetype` from robust
/// per-image features, INDEPENDENT of the theme path. PRIMARY axis: `band_activity_spread`-
/// expanded `edge_activity` along the BROAD→BUSY density ramp (cells 1/0/2). SECONDARY axis:
/// `affect_arousal` diverts high-arousal images onto the PROFILED/SYNCOPATED character gait
/// (cell 3). Pure; no RNG, no clock. The returned index is clamped to `cell_count`.
///
/// Distinct from `composition::pick_rhythm_cell` (the THEMED-path selector): this one keys on
/// the spread edge + arousal so it reaches the real-photo cluster; the themed selector keeps the
/// pre-S41 (edge, complexity) keying for the synthetic theme path. fix-direction-2 / S53 D-CELL.
pub fn pick_piece_cell(
    edge_activity: f32,
    affect_arousal: f32,
    archetype: MotifArchetype,
    cell_count: usize,
) -> usize;
```

> Signature note: it takes the two scalar features by value (not `&ImageUnderstanding`) so `chord_engine` does NOT import the image type — preserving the boundary "chord engine has no image logic." `composition.rs` reads the fields off `u` and passes scalars, exactly as it passes `range_degrees`/`length_steps` to `resolve_motif_celled`.

**NEW — the per-piece motto value object (lives in `chord_engine.rs` next to `MotifArchetype`).** A small `Copy` identity, NOT a resolved motif:

```rust
/// A per-piece RHYTHMIC MOTTO: the chosen melodic archetype and the index of its rhythm cell,
/// selected once per plan and stamped on every `Section`. Carries IDENTITY (archetype + cell),
/// not a resolved `Vec<MotifNote>` — the no-theme path has no subject to replay; the motto only
/// BIASES the band-ladder onset placement in `realize_rhythm`. `Copy` so it rides `Section`
/// (which is `Clone`) and the per-section `orchestration.clone()` with zero allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RhythmMotto {
    pub archetype: MotifArchetype,
    pub cell_index: usize,
}
```

**CHANGED — `Section` gains one field (composition.rs:1180).** Additive, mirrors the `density` precedent:

```rust
// before (composition.rs:1180): unchanged fields …
//   pub density: f32,
//   pub orchestration: OrchestrationProfile,
//   pub steps: Vec<StepPlan>,

// after: one additive field, defaulting to a byte-stable neutral on every legacy/identity path.
pub struct Section {
    // … all existing fields UNCHANGED …
    /// NEW S53 (D-CELL) — the per-piece rhythmic motto selected once by the planner from
    /// `edge_activity` + `affect_arousal`, stamped on EVERY section so `realize_rhythm` can
    /// bias onset placement off `ctx.section.motto` zero-copy (the `density` precedent). The
    /// legacy/identity/`single_section_default` paths carry `RhythmMotto::neutral()` (the
    /// byte-stable no-op — see §5 migration), so the engine_equivalence goldens cannot move.
    pub motto: RhythmMotto,
    // pub steps: Vec<StepPlan>,   // stays last
}
```

**NEW — the byte-stable neutral constructor (so legacy/identity sections are a no-op):**

```rust
impl RhythmMotto {
    /// The behaviour-neutral motto: the value `realize_rhythm` treats as "apply NO onset bias",
    /// so a section carrying it produces byte-identical output to pre-S53. Used by
    /// `legacy_default_section`, `single_section_default`, and the planner's fallback section.
    /// MUST round-trip to the identity branch of `realize_rhythm`'s motto read (§3, §5).
    pub fn neutral() -> RhythmMotto;
}
```

**UNCHANGED but newly-reached — `composition::pick_rhythm_cell` (:1806).** Stays exactly as-is, still the themed-path selector at :1490. NOT deleted, NOT re-keyed. The reverted consts at :1770–1792 stay reverted (they govern the synthetic themed path, which still requires `complexity >= 0.4`).

**`realize_rhythm` (chord_engine.rs:2052) — signature UNCHANGED.** It already borrows `ctx: &StepContext`. It gains an internal read of `ctx.section.motto` (a body change, NOT a signature change — exactly like the `ctx.section.density` read at :2092). The motto's onset-placement effect is applied to the band-ladder onset positions; the identity motto short-circuits to the existing path. **No signature on the realize_step / decide_instrument_action public seam changes — engine.rs is untouched.**

### 2.6 What stays untouched

- **engine.rs** — entirely. It constructs `ctx` (`with_prev` / `single_section_default`) and passes it through; it never reads `motto`. Confirmed reachable: `realize_rhythm` reads `ctx.section.motto` exactly as it reads `ctx.section.density` today (chord_engine.rs:2092), and that read crosses no engine.rs line.
- The `MotifNote` type, `resolve_motif_celled`, the rhythm-cell **vocabulary** (chord_engine.rs:3241–3314), and `rhythm_cell` / `rhythm_cell_count` — all unchanged. The motto **indexes** the existing vocabulary; it does not author new cells.
- The S50-reverted consts and the themed-path `pick_rhythm_cell` — unchanged.
- All `ImageUnderstanding` fields and `pure_analysis` — no new extraction (no OpenCV change). The driver reuses already-computed features.

---

## 3. INTERFACE DEFINITIONS (the contracts the Implementer codes against)

**I-1 — `chord_engine::pick_piece_cell` (pure selector).**
- **Inputs:** `edge_activity: f32` (0..1), `affect_arousal: f32` (0..1; the planner-computed composite, NOT the −1.0 sentinel — the planner seats it at composition.rs:1399–1402 before this is called), `archetype: MotifArchetype`, `cell_count: usize` (`>= 1`; from `archetype.rhythm_cell_count()`).
- **Output:** `usize` in `0..cell_count` (clamped; never panics; never out-of-range — the realizer also clamps defensively in `rhythm_cell`).
- **Determinism:** pure function of inputs. No RNG, no clock.
- **Spread contract:** the PRIMARY comparison is against `band_activity_spread(edge_activity)`, not raw `edge_activity`.
- **Reachability:** must be reachable for EVERY image (no `complexity >= 0.4` precondition anywhere on its path).

**I-2 — `RhythmMotto` (value object).** `Copy + Eq`. `archetype: MotifArchetype`, `cell_index: usize`. `RhythmMotto::neutral()` returns the value that `realize_rhythm` maps to "no onset bias."

**I-3 — `Section.motto: RhythmMotto`.** Set by the planner ONCE per plan (same value on every section in slice 1 — per-section variation is slice-2 D-METER territory). Legacy/identity/fallback sections carry `RhythmMotto::neutral()`.

**I-4 — `realize_rhythm` motto contract.** Reads `ctx.section.motto`. When `motto == RhythmMotto::neutral()`, output is BYTE-IDENTICAL to pre-S53 (the freeze hinge). When non-neutral, the motto's cell biases onset placement **within** the band the band-ladder already chose — it may re-distribute onsets to match the cell's gait but must respect the existing **figure-ground governor** (I-5). The exact onset-bias mapping (cell weights → onset offsets) is the **Music Theory Specialist's** call; this spec fixes the SEAM and the neutral contract, not the bias curve.

**I-5 — figure-ground preservation (HARD).** The motto bias is applied per-voice, but the per-role rhythm distinctness must keep `melody_activity_class` (chord_engine.rs:1136) the governing rank: a background/counter voice's motto-biased onsets must never exceed the melody's `ActivityClass`. The motto must route through (or below) the existing S46 governor, not around it. **Contract: the motto can only re-place onsets the band already permits for that role; it cannot promote a voice's `ActivityClass`.**

---

## 4. DATA FLOW DIAGRAM — before vs after

```
BEFORE (cell axis dormant on real photos):

  ImageUnderstanding
        │ complexity
        ▼
  theme_behaviour.select  ──"absent" (real, complexity<0.4)──►  themes = []
        │                                                            │ (cell NEVER selected)
        │"fragment" (synthetic, complexity>=0.4)                     │
        ▼                                                            │
  ┌─ else branch (composition.rs:1480) ───────────────┐             │
  │  pick_rhythm_cell(u, archetype, K)   (:1490)      │             │
  │  resolve_motif_celled(...) ► ThemeSeed{motif}     │             │
  └───────────────────────────────────────────────────┘             │
        │                                                            │
        ▼                                                            ▼
  plan.themes = [seed]                                        plan.themes = []
        │                                                            │
        ▼                                                            ▼
  engine ctx.theme = Some(&seed)                            engine ctx.theme = None
        │                                                            │
        ▼                                                            ▼
  realizer replays subject + band ladder                   realizer = BAND LADDER ONLY
                                                           (NO cell signal — dormant axis)


AFTER (cell axis runs PER-PIECE, decoupled from the theme gate):

  ImageUnderstanding
        │ edge_activity, affect_arousal          (affect seated at plan() :1399–1402)
        ▼
  motto = RhythmMotto {                                ◄── computed ONCE in plan(),
            archetype: pick_archetype(u),                  BEFORE the section loop,
            cell_index: chord_engine::pick_piece_cell(     UNCONDITIONALLY (no theme gate)
                          band_activity_spread(edge_activity-internal),
                          affect_arousal, archetype, K),
          }
        │
        │  (the theme gate at :1478 is UNCHANGED and still produces themes only for
        │   synthetic complexity>=0.4 images — cell stamped on sections REGARDLESS)
        ▼
  every Section.motto = motto         ◄── stamped on ALL sections (slice 1: same value)
        │
        ▼
  engine builds ctx (with_prev / single_section_default) — UNCHANGED, passes ctx through
        │
        ▼
  realize_rhythm reads ctx.section.motto (chord_engine.rs, beside the :2092 density read)
        │
        ├─ motto == neutral()  ──► byte-identical to BEFORE (legacy/identity freeze hinge)
        └─ motto != neutral()  ──► onset placement biased to the cell's gait,
                                    CLAMPED under the S46 figure-ground governor
```

---

## 5. MIGRATION PATH

### 5.1 Step order (Implementer = Music Theory Specialist)

1. **Add `RhythmMotto` + `RhythmMotto::neutral()`** in `chord_engine.rs` (next to `MotifArchetype`). Pure type; no behaviour yet.
2. **Add `chord_engine::pick_piece_cell`** (I-1). Pure selector reusing `band_activity_spread`. Unit-tested in isolation (the `motif_s41`/`rhythm_variety_s50` test style — no image type, scalar inputs).
3. **Add `Section.motto: RhythmMotto`** field (composition.rs:1180). Wire `RhythmMotto::neutral()` into the THREE neutral constructors: `legacy_default_section`, `single_section_default` consumers, and the planner's identity/fallback section. **At this step, with the planner still stamping `neutral()` everywhere, the whole tree is byte-stable — run engine_equivalence 9/9 here as a checkpoint; it MUST stay green (this is the "field added, behaviour inert" freeze witness, identical to how `density` / `prev_key_offset` were landed).**
4. **Stamp the real motto in `plan()`**: compute `motto` once (after the affect seat at :1399–1402, near the once-per-plan resolves at :1517–1538) and assign it to each `Section` in the expansion loop (replacing `neutral()` on the real path). **This is the step that changes no-theme output** — see §5.2.
5. **Add the motto read in `realize_rhythm`** (chord_engine.rs, beside :2092): `neutral()` → existing path; non-neutral → the onset-bias the Music Theory Specialist designs, clamped under the figure-ground governor (I-4/I-5).
6. **Re-baseline the changed goldens** (§5.2), each with a written justification hunk.
7. **`cargo test --no-default-features`** (headless lib/test path) + **`cargo clippy -- -W clippy::all`** green. The OpenCV binary path is hardware-gated and not built here.

### 5.2 Equivalence-golden handling (the highest-risk part — be explicit)

There are TWO classes of golden, and they are handled OPPOSITELY:

**(A) MUST stay byte-identical — the engine-kernel freeze goldens.**
- `tests/engine_equivalence.rs` (the 9/9 byte goldens), the **no-counter** and **no-theme equivalence** goldens, the `seed_s41` engine-sha witness (`ENGINE_SHA256`), and the `keyplan_s25 / affect_s22` engine-kernel sha anchors.
- These run through `StepContext::single_section_default` / `legacy_default_section`, which carry `RhythmMotto::neutral()`. Under I-4, a neutral motto is a guaranteed no-op in `realize_rhythm`. **They MUST NOT move.** If step 3's checkpoint (field added, all-neutral) ever shows a diff, the neutral contract is broken — STOP and fix `neutral()`/the read, do not re-baseline.
- **The "no-theme equivalence golden" nuance:** the existing no-theme equivalence golden in `engine_equivalence.rs` is built from a **synthetic neutral plan** (no real image, `single_section_default`), so its section carries `neutral()` and it stays byte-frozen. This is the golden the kickoff flagged — and the resolution is precisely that the *engine-equivalence* no-theme golden is the synthetic-neutral one, which neutral-motto keeps frozen.

**(B) LEGITIMATELY CHANGE and MUST be re-baselined with justification — the real-image rhythm-variety goldens.**
- `tests/rhythm_variety_s50.rs` — specifically `realized_cell` (:273) and the `NO_THEME_CELL` sentinel (:250). **Today** `realized_cell` returns `NO_THEME_CELL` for every real photo (because `plan.themes.is_empty()`, :274). **After** this slice, the cell is selected per-piece and stamped on the section, so the no-theme branch's rhythmic observable changes from "no cell" to "the per-piece motto cell." This is the **intended** effect of the slice — the dormant axis becoming live IS the deliverable. The test's `realized_cell`/sentinel machinery must be re-pointed at `Section.motto.cell_index` (the new honest observable) instead of `themes[0].motif`, and any frozen per-image cell expectations re-baselined to the per-piece values.
  - **Justification hunk required:** "fix-direction-2 D-CELL un-gates the cell axis to run per-piece; the no-theme rhythmic observable is now `Section.motto.cell_index`, not the (always-empty) `themes[0]`. The S52 honesty invariant is PRESERVED because the tree now honestly reflects a cell the engine actually reads."
- Any `variety_s45` / cross-piece spread assertion that compares the six probe images' rhythmic signatures: these should **improve** (more separation), not regress. Re-baseline only if the new values still satisfy the S50 cross-piece spread floor (§6) and the S46 figure-ground rank (§6). If a probe pair COLLAPSES, that is a driver-tuning failure, not a re-baseline — fix the driver/cuts, don't accept the diff.

**(C) S52 honesty invariant.** The "tree honestly reflects what the engine reads" goldens stay valid: after the slice the engine DOES read `ctx.section.motto`, so a test asserting the cell is honest. The honesty machinery moves from `themes[0].motif` to `Section.motto`; it does not weaken.

### 5.3 engine.rs untouched — confirmation

- The only engine.rs lines in the rhythm path are the `ctx` build (`with_prev` :551, `single_section_default` :586) and the pass-through to `decide_instrument_action`. None reads `motto`. The motto read is entirely inside `chord_engine::realize_rhythm`, reached via `ctx.section`, which engine.rs already borrows and forwards. **No engine.rs edit is proposed or required.** The freeze sha `e50c7db1…` is preserved; the `seed_s41` ENGINE_SHA256 witness stays green.

---

## 6. RISKS AND TRADE-OFFS

**R-1 — the S50 collision trap (the dominant risk; why path (b) over path (a)).**
Path (a) = "lower the theme-gate complexity cut so themes (and thus the cell call) fire on real photos." **REJECTED.** Lowering the theme gate to `complexity < 0.4` would (i) manufacture spurious melodic THEMES on every real photo (a theme is a replayed subject — real photos have no subject to replay; this would add unwanted thematic recapitulation), and (ii) re-create the exact S50 force-pin: with the gate low, every image clears `CELL_COMPLEXITY_PROFILED` and collapses to cell 3. Path (a) couples the cell axis to a theme-creation decision that has nothing to do with rhythm. **Path (b) decouples** — the cell becomes a per-piece *rhythmic motto* (an onset bias) with NO theme manufactured and NO `complexity >= 0.4` precondition, so the S50 collision cannot recur (the new secondary key is `affect_arousal`, not `complexity`, and there is no gate pre-filtering to `complexity >= 0.4`). This is why (b) is sound and (a) is trapped.

**R-2 — figure-ground inversion (HARD preservation, S46 6-VARIED).**
A per-voice motto bias could let a background/counter voice's onsets exceed the melody's. **Mitigation (I-5):** the motto re-places only onsets the band ALREADY permits for that role and routes through the existing `melody_activity_class` governor (chord_engine.rs:1136); it cannot promote a voice's `ActivityClass`. The melody must remain melody-most-active + melody-on-top. **Verification:** the S46 figure-ground metric (`spec-s46-figure-ground-metrics.md`) must stay green post-slice; treat any inversion as a build defect, not a re-baseline.

**R-3 — cross-piece spread regression (HARD preservation, S50).**
The motto must not COLLAPSE the six probe images' band/character/tempo variety. The driver deliberately reuses `band_activity_spread` (so the motto reads the same re-expanded activity the S50 spread delivers) and keys the secondary on `affect_arousal` (which spreads real photos). **Verification:** the S50 cross-piece spread assertion must hold or improve; a probe-pair collapse is a driver-tuning failure to fix, not a diff to accept (§5.2-B).

**R-4 — slice-1-pinned features re-pinning the axis.**
If the driver accidentally keys on a pinned default (`complexity`-only, `quadrant_contrast`, `fg_bg_contrast`, any `subject_*`), every photo re-collapses to one cell — the dormancy returns in a new disguise. **Mitigation:** the driver is restricted to live features (`edge_activity` + `affect_arousal`); §2.3 documents the exclusion list. **The taste/affect reviewer must confirm the chosen features actually separate the six probes** before the cuts are frozen.

**R-5 — equivalence-golden mishandling (highest-consequence).**
Re-baselining a Class-A (engine-kernel) golden would silently break the freeze; failing to re-baseline a Class-B (real-image) golden would block the slice's own deliverable. **Mitigation:** the step-3 all-neutral checkpoint (§5.1) proves Class-A stays frozen BEFORE any real motto is stamped; only Class-B real-image goldens are re-baselined, each with the §5.2-B justification hunk. The no-theme **engine-equivalence** golden is Class-A (synthetic neutral) and stays frozen; the no-theme **rhythm-variety** observable is Class-B and changes.

**Trade-off accepted:** one additive `Copy` field on `Section` (8–16 bytes, rides the existing per-section clone). This is the same cost the `density` field already pays and is the price of keeping engine.rs frozen (the alternative — a `StepContext` field — would touch the engine's `with_prev`/`single_section_default` surface and is rejected).

---

## 7. THIN FORWARD NOTE — D-METER (Slice 2, DEFERRED, NOT designed here)

Slice 2 (D-METER) un-gates the **meter** axis (today pinned `"four4"`, mappings.json:170, `meter` SelectTable with empty rules) so a per-image meter selects a **bar grid → beat-strength profile → onset-placement** consumer chain. It docks onto **this slice's seam directly**: `Section.motto` (the per-piece `RhythmMotto`) becomes the natural carrier for a companion `Section.meter`/beat-grid field set the same way (planner-stamped, read in `realize_rhythm` via `ctx.section`, byte-stable neutral on legacy paths). The onset-bias contract this slice fixes (I-4: "re-place onsets the band permits, clamped under the figure-ground governor") is exactly the hook D-METER's beat-strength weighting refines — the motto says *which gait*, the meter will say *where the strong beats fall*, and the two compose at the same onset-placement site in `realize_rhythm`. Slice 2 will need its own affect/taste pass on meter→character coupling; it is out of scope here and not designed.

---

## Appendix — files read for this spec (absolute paths)

- `/home/qweary/working/audiohax-engagement/AudioHax/src/composition.rs` (theme gate ~1477–1498; `pick_rhythm_cell` 1806–1826; reverted consts 1770–1792; `Section` 1180–1221; `StepContext` 1293–1349; `Knob`/`SelectTable`/`Predicate` 812–918; `ImageUnderstanding` 40–117; `plan()` 1395+)
- `/home/qweary/working/audiohax-engagement/AudioHax/src/chord_engine.rs` (`band_activity_spread` 1097; `melody_activity_class` 1136; `realize_rhythm` 2052+ incl. the `ctx.section.density` read 2092; `rhythm_cells` 3241–3314; `rhythm_cell`/`rhythm_cell_count` 3321–3333)
- `/home/qweary/working/audiohax-engagement/AudioHax/src/engine.rs` (FROZEN — read only; ctx build/pass-through 543–597)
- `/home/qweary/working/audiohax-engagement/AudioHax/assets/mappings.json` (`theme_behaviour` 120–123; `character` 160–169; `motif_rhythm` mirror 124–137; `affect` weights 138–158)
- `/home/qweary/working/audiohax-engagement/AudioHax/tests/rhythm_variety_s50.rs` (`realized_cell` 273; `NO_THEME_CELL` 250; `identify_cell` 238)

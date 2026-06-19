# S41 — Finding B: Rhythmic / Pattern Depth (the clap-test) — buildable work order

**Author:** Rust Architect (DESIGN ONLY — no `src/*`/`tests/*`/`assets/*` edited this session)
**Freeze anchor honored throughout:** `src/engine.rs` byte-frozen at sha256 `e50c7db1…48261` (verified this session). `engine_equivalence` 9/9 must stay green. Every edit below lands in `composition.rs`, `chord_engine.rs`, or `assets/mappings.json`. `engine.rs` is never opened.
**Builds on:** S39 (8-archetype contour vocabulary + per-archetype `rhythm_profile` + `theme_melody_pitch` duration thread, `tests/motif_s39.rs`) and S40 (per-image HOME tonal center). This work order is the *third* slice of the §S38 arc, targeting **Finding B** (rhythmic/pattern sameness across images — the "clap test").

---

## 0. The defect, stated precisely against the as-built code

After S39 the motif's rhythm is **a pure function of its contour archetype**. The chain is:

```
image → pick_archetype(u)                    composition.rs:1653  → one of 8 MotifArchetype
      → resolve_motif(archetype, range, len) chord_engine.rs:2435
            → archetype.rhythm_profile()      chord_engine.rs:2396 → ONE &'static [u8] per archetype
```

`rhythm_profile()` (`chord_engine.rs:2396–2414`) is a closed `match self { … }` keyed on **nothing but the archetype**. Therefore:

> **Two different images that land on the same `MotifArchetype` get a byte-identical `dur_steps` sequence.**

`pick_archetype` partitions the affect plane into 4 quadrants × an up/down tiebreak → exactly the 8 archetypes (`composition.rs:1667–1705`). With only 8 finite contours *and* rhythm welded 1:1 to contour, the gait space the ear hears is **8 fixed (contour, rhythm) pairs**. By the ~4th image the listener has heard most of them — that is exactly the operator's "familiar by the 4th image" report, and the clap-test names its sharpest edge: clap two images' themes and they share a gait *because the gait is the archetype's and the archetype recurs*.

S39's `InvertedArch` even shares `[2,1,1,2]` with `Arch` and `RisingSequence` shares `[1,1,2]` with `Ascent` — so the *effective* rhythm vocabulary is only **6 distinct dur-sequences across 8 contours**. The rhythm axis is the weakest-differentiated axis in the system.

---

## 1. Lever ranking (leverage on clap-test cross-image sameness, subject to freeze-safety)

| Rank | Lever | Clap-test leverage | Freeze | One-line justification |
|---|---|---|---|---|
| **1** | **(A) image-selected rhythm cells** | **Highest** | **SAFE** | Directly breaks the contour→rhythm weld: the same archetype now emits *different* gaits for different images, so the clap test stops collapsing onto 6–8 fixed gaits. This is the only lever that adds variance *on the exact axis the clap test measures* without depending on (B) or (C). |
| 2 | (C) deferred prominence→melody + saliency vocabulary-depth | Medium | SAFE-ish | Adds *contour/pitch* differentiation and attacks the "familiar by 4th image" tail, but it widens the **pitch** axis, not the **rhythm/gait** axis the clap test isolates; also entangled with the saliency-reader (region pass) which is a larger, less-revertible change. Second-order for *this* finding. |
| 3 | (B) pattern-library Slice 3/4 (lament/Andalusian bass, 12-bar form; chaos↔order clash) | Low (for clap-test) | SAFE | Operates on bass/form/orchestration layers, not the *theme's gait*. Two images can ride a 12-bar form with identical opening-theme rhythm — it doesn't move the clap test, which is run on the **opening themes** (§S38 GR-7). Valuable, but for a different finding. |

**Why (A) beats (C) on the clap test specifically.** The clap test is run on rhythm-only ("clap two themes back-to-back — same gait?"). (C) raises *pitch/contour* variety; you can hand it two distinctly-pitched melodies that still clap identically. (A) is the only candidate that puts variance on the clapped dimension itself. (C) is the right *next* slice for the "familiar by 4th image" tail (a separate, pitch-axis complaint), but it is not the rhythm-depth lever.

**Why (A) beats (B) on the clap test specifically.** (B) lives below the theme (bass tetrachord, form, orchestration clash). The clap test claps the **opening theme melody**; (B) leaves that melody's gait untouched. (B) also carries the heaviest build (form machinery + ear-gated clash control) for the least clap-test movement.

---

## 2. Chosen lever: (A) image-selected rhythm cells — FREEZE-SAFE

> **Slice = decouple rhythm from contour. Replace the archetype-FIXED `rhythm_profile()` with an image-driven SELECTION among a small vocabulary of rhythm cells, so two images sharing a contour archetype still emit different `dur_steps` sequences.**

Independent and individually-revertable: it touches only the rhythm half of the motif. Revert = restore the single `rhythm_profile()` call. It does not depend on (B) or (C), and (B)/(C) do not depend on it.

**Freeze verdict: SAFE.** Full hinge analysis in §6. The short form: all new work is plan-time selection (`composition.rs`) plus a *widened* — not new — realize-side read; the `engine_equivalence` goldens carry `theme: None` on every pinned section, so the theme path they pin is never exercised; and the §6 byte-identity invariant ("an all-`dur==1` motif realizes 1:1") is preserved by construction because every cell is still a `[u8]` of weights `>= 1` consumed by the *unchanged* `resolve_motif` accumulation loop.

---

## 3. The selection model (concrete music + data design)

### 3.1 Concept: a contour gets a FAMILY of compatible cells, the image picks one

Today each archetype owns exactly one profile. We give each archetype a **small ordered set of rhythm cells** that are all *musically idiomatic for that contour* (theory-owned), and let an image feature **index into that set**. The contour identity is preserved (still that archetype's pitch shape); the gait now varies per image.

Two decorrelated image axes drive the pick so that images differing on *either* axis diverge rhythmically:

- **Primary index — rhythmic density / activity:** `edge_activity` (already on `u`, already in scope at `composition.rs:1472`). Busy images → busier (more `1`s, syncopated) cells; calm images → broader (more `2`s/augmentation) cells. This is the affect-faithful rhythm axis (arousal→rhythmic density is textbook).
- **Secondary index — within-density tiebreak:** `complexity` (also on `u`, already in scope at `:1473`). Decorrelates two images of equal `edge_activity` so they don't re-collapse.

A two-axis index over a per-archetype cell set of size `K` (recommend **K = 4**) yields up to `8 × 4 = 32` effective (contour, gait) pairs — versus today's 6. That is the clap-test win.

### 3.2 Why a cell *vocabulary per archetype*, not a global cell table

A global rhythm-cell table indexed independently of contour can pair a gait against a contour it doesn't fit (e.g. a 5-note `Descent` against a 2-cell `[2,2]` gait truncates the line). Theory owns the constraint that **each cell must be legal for its archetype's contour** — same constraint S39's table already satisfied. So the vocabulary is **per-archetype** and Music Theory authors it; Affect/Aesthetics own only the *index cuts* (which feature value selects cell 0..K-1).

### 3.3 New / changed data structures (concrete Rust)

**`chord_engine.rs` — replace the single-profile method with a cell-set method.** `rhythm_profile(self) -> &'static [u8]` (`:2396`) becomes a *family* accessor plus a selector that takes the image-derived index:

```rust
impl MotifArchetype {
    /// The archetype's RHYTHM-CELL VOCABULARY: K musically-idiomatic durational cells,
    /// ordered from BROADEST (index 0, most augmentation — calm) to BUSIEST (index K-1,
    /// most subdivision — energetic). Each cell is a `dur_steps`-weight list, every weight
    /// >= 1, cycled across the sampled contour by `resolve_motif` exactly as the single
    /// S39 profile was. theory: all cells of one archetype share that contour's gesture but
    /// differ in augmentation/subdivision so the SAME melodic shape can be clapped two
    /// different ways. cell 0 of every archetype == the S39 `rhythm_profile()` value (the
    /// back-compat anchor — see §6 freeze hinge).
    fn rhythm_cells(self) -> &'static [&'static [u8]] {
        match self {
            // cell 0 == S39 [2,1,1,2]; then broader, then busier, then syncopated.
            MotifArchetype::Arch => &[&[2, 1, 1, 2], &[2, 2, 2], &[1, 1, 1, 1], &[1, 2, 1, 1, 1]],
            MotifArchetype::InvertedArch => &[&[2, 1, 1, 2], &[2, 2, 2], &[1, 1, 1, 1], &[1, 1, 2, 1, 1]],
            MotifArchetype::Descent => &[&[1, 1, 1, 1, 2], &[2, 2, 2], &[1, 1, 1, 1, 1], &[2, 1, 1, 1, 1]],
            MotifArchetype::Ascent => &[&[1, 1, 2], &[2, 2], &[1, 1, 1, 1], &[1, 1, 1, 2]],
            MotifArchetype::NeighborTurn => &[&[1, 1, 1, 1, 2], &[2, 2], &[1, 1, 1, 1], &[1, 2, 1, 1]],
            MotifArchetype::LeapStep => &[&[2, 1, 1, 1, 1], &[2, 2, 1], &[1, 1, 1, 1, 1], &[3, 1, 1, 1]],
            MotifArchetype::Pendulum => &[&[2, 2], &[2, 2, 2], &[1, 1, 1, 1], &[3, 1]],
            MotifArchetype::RisingSequence => &[&[1, 1, 2], &[2, 2], &[1, 1, 1, 1, 1, 1], &[1, 2, 1, 2]],
        }
    }

    /// Select ONE rhythm cell from this archetype's vocabulary given a 0..K-1 index
    /// (image-derived upstream by `composition::pick_rhythm_cell`). Defensive clamp so an
    /// out-of-range index can never panic — it saturates to the busiest cell. Index 0 is the
    /// S39 profile, so `rhythm_cell(0)` reproduces S39 exactly (freeze anchor).
    fn rhythm_cell(self, index: usize) -> &'static [u8] {
        let cells = self.rhythm_cells();
        cells[index.min(cells.len() - 1)]
    }
}
```

> The Music Theory specialist OWNS the contents of the `rhythm_cells` table above — the cell lists shown are an architect SEED that satisfies the structural contract (every weight `>= 1`; cell 0 == S39 profile; broad→busy ordering); Music Theory refines the actual durational content to professional standard and confirms each cell is idiomatic for its contour. **Index 0 of every archetype MUST remain the S39 value** — that is the freeze anchor (§6) and a hard constraint, not a taste call.

**`resolve_motif` signature change (`chord_engine.rs:2435`).** Add the cell index. To avoid breaking the 11 existing test callers (`composition_s15.rs`, `chord_engine.rs` unit tests, `motif_s39.rs` — see §5.1), add it as a **new third-position parameter via a sibling function**, NOT by mutating the existing 3-arg signature:

```rust
/// S41: resolve a motif with an EXPLICIT rhythm-cell index (image-selected). The S39
/// `resolve_motif(archetype, range, len)` is retained as a thin wrapper that calls this
/// with `cell_index = 0` (the S39 profile) — so every existing caller and golden is
/// byte-unchanged.
pub fn resolve_motif_celled(
    archetype: MotifArchetype,
    range_degrees: u8,
    length_steps: usize,
    cell_index: usize,
) -> Vec<MotifNote> { /* body == today's resolve_motif but `let profile = archetype.rhythm_cell(cell_index);` */ }

/// BACK-COMPAT: the S39 entry point. Equivalent to `resolve_motif_celled(.., 0)`.
pub fn resolve_motif(
    archetype: MotifArchetype,
    range_degrees: u8,
    length_steps: usize,
) -> Vec<MotifNote> {
    resolve_motif_celled(archetype, range_degrees, length_steps, 0)
}
```

This keeps `resolve_motif`'s contract and all of its callers/goldens byte-identical (cell 0 == S39 profile), and isolates the new behavior behind a new symbol the planner opts into.

### 3.4 The plan-time selector (`composition.rs`)

Add a pure selector beside `pick_archetype` (`composition.rs:1653`):

```rust
/// Pick a rhythm-cell index (0..K-1) for the chosen archetype from the image's rhythmic
/// energy. PRIMARY axis edge_activity (busy → busier cell), SECONDARY axis complexity
/// (within-energy tiebreak so two equal-activity images still diverge). Pure; no RNG.
/// Returns an index the realizer clamps defensively; K is the archetype's cell count.
fn pick_rhythm_cell(u: &ImageUnderstanding, archetype: MotifArchetype, cell_count: usize) -> usize {
    // body: map (edge_activity, complexity) → 0..cell_count-1.
    // architect seed (Affect/Aesthetics tune the exact cuts — DP-A):
    //   coarse = edge_activity bucketed into cell_count bands;
    //   then nudge ±0 within band by complexity parity so equal-activity images split.
}
```

And change the theme build at `composition.rs:1469–1474` from:

```rust
let archetype = pick_archetype(u);
let range_degrees = (2.0 + u.edge_activity * 5.0).round() as u8;
let length_steps  = (3.0 + u.complexity * 5.0).round() as usize;
let motif = chord_engine::resolve_motif(archetype, range_degrees, length_steps);
```

to:

```rust
let archetype = pick_archetype(u);
let range_degrees = (2.0 + u.edge_activity * 5.0).round() as u8;
let length_steps  = (3.0 + u.complexity * 5.0).round() as usize;
let cell_count    = archetype.rhythm_cell_count(); // tiny pub accessor → rhythm_cells().len()
let cell_index    = pick_rhythm_cell(u, archetype, cell_count);
let motif = chord_engine::resolve_motif_celled(archetype, range_degrees, length_steps, cell_index);
```

`rhythm_cell_count(self) -> usize` is a one-line `pub fn` on `MotifArchetype` (`= self.rhythm_cells().len()`) so the planner can size the index without exposing the static table.

### 3.5 `assets/mappings.json` rows (data mirror — keep the S39 discipline)

The S39 `composition.motif_rhythm` block (`assets/mappings.json:165–177`) is a **non-authoritative data mirror** — `resolve_motif` does NOT read it. Keep that exact discipline for S41 to avoid a loader-wiring scope blowout: extend the mirror to the cell *families* and document the index cuts, but the inline Rust `rhythm_cells()` stays AUTHORITATIVE this slice. Proposed replacement for the `motif_rhythm` block (Music Theory holds the pen; Affect/Aesthetics fill `_index_cuts`):

```jsonc
"motif_rhythm": {
  "_note": "DATA MIRROR (S41 DP-6 discipline retained): a tunable copy of the per-archetype rhythm-cell VOCABULARIES authored inline in chord_engine.rs::MotifArchetype::rhythm_cells(). The inline Rust cells are AUTHORITATIVE — resolve_motif_celled does NOT read this block (no loader wiring this slice). cell 0 of every archetype MUST equal the S39 single profile (freeze anchor). Keep in sync if either is edited.",
  "cells": {
    "Arch":           [[2,1,1,2],[2,2,2],[1,1,1,1],[1,2,1,1,1]],
    "InvertedArch":   [[2,1,1,2],[2,2,2],[1,1,1,1],[1,1,2,1,1]],
    "Descent":        [[1,1,1,1,2],[2,2,2],[1,1,1,1,1],[2,1,1,1,1]],
    "Ascent":         [[1,1,2],[2,2],[1,1,1,1],[1,1,1,2]],
    "NeighborTurn":   [[1,1,1,1,2],[2,2],[1,1,1,1],[1,2,1,1]],
    "LeapStep":       [[2,1,1,1,1],[2,2,1],[1,1,1,1,1],[3,1,1,1]],
    "Pendulum":       [[2,2],[2,2,2],[1,1,1,1],[3,1]],
    "RisingSequence": [[1,1,2],[2,2],[1,1,1,1,1,1],[1,2,1,2]]
  },
  "_index_cuts": { "_axes": ["edge_activity (primary, busier→higher cell)", "complexity (secondary tiebreak)"], "_note": "Affect/Aesthetics fill the exact band edges; mirror only, pick_rhythm_cell is authoritative this slice." }
}
```

---

## 4. Data flow

```
ImageUnderstanding u (edge_activity, complexity already populated by understand_image_pure)
        │
        ├─ pick_archetype(u)                       composition.rs:1653  → MotifArchetype (contour)   [UNCHANGED]
        │
        └─ pick_rhythm_cell(u, archetype, K)       composition.rs:NEW   → cell_index 0..K-1          [NEW, plan-time]
                    │
                    ▼
        resolve_motif_celled(archetype, range, len, cell_index)   chord_engine.rs:NEW (wraps S39 body)
                    │  profile = archetype.rhythm_cell(cell_index)   ← the ONLY behavioral change
                    ▼
        Vec<MotifNote{degree, dur_steps}>  (same accumulation loop, same Σ<=len cap, same static-tail fix)
                    │
                    ▼  stored on ThemeSeed.motif (plan data)
        theme_melody_pitch / motif_step_at  chord_engine.rs:2567/2610  [UNCHANGED — reads dur_steps as before]
                    │
                    ▼
        realized melody (onset plays, continuation rests)  — gait now varies per image
```

Module boundary preserved: image analysis still emits scalars; the chord engine still owns contour→pitch and the rhythm-cell table; the planner owns selection. No image logic enters `chord_engine`; no music logic enters `pure_analysis`.

---

## 5. File ownership / single-writer coordination

The three edited files are split so the two implementing specialists are **FILE-DISJOINT** — no sequencing needed.

| File | Owner | Edits |
|---|---|---|
| `src/chord_engine.rs` | **Music Theory specialist** | `rhythm_profile()` → `rhythm_cells()` + `rhythm_cell(index)` + `rhythm_cell_count()` accessors (`:2396` region); split `resolve_motif` into `resolve_motif_celled` (new body) + `resolve_motif` (thin wrapper) (`:2435`). All of this is *theory data + the contour/rhythm engine* — Music Theory's domain. |
| `src/composition.rs` | **Rust Implementer** | new `pick_rhythm_cell(u, archetype, K)` selector beside `pick_archetype` (`:1653`); rewire the theme build at `:1469–1474` to call `pick_rhythm_cell` + `resolve_motif_celled`. Plan-time wiring — Implementer's domain. |
| `assets/mappings.json` | **Music Theory specialist** | replace the `motif_rhythm` block (`:165–177`) with the cell-vocabulary mirror (§3.5). Theory data; same owner as the inline table it mirrors — keeps the "mirror in sync with authoritative inline" invariant single-writer. |
| `tests/motif_s41.rs` (NEW) | **Test Engineer** | §7 property net. New file, no contention. |

**Disjoint check:** Music Theory owns `chord_engine.rs` + `mappings.json`; Implementer owns `composition.rs`; Test Engineer owns the new test file. No file is co-owned. The only cross-file contract is the **function signatures in §3.3/§3.4** (`resolve_motif_celled`, `rhythm_cell_count`, `pick_rhythm_cell`) — frozen here so neither side waits on the other. Implementer codes against the Music-Theory signatures as published in this doc; Music Theory codes the bodies; they integrate at compile.

**One ordering note (not a shared file):** the Implementer's `composition.rs` call to `resolve_motif_celled` / `rhythm_cell_count` won't compile until Music Theory's `chord_engine.rs` symbols exist. This is a *compile* dependency, not a *file* contention — resolved by both landing in the same build slice (they're spawned together; the slice is green only when both are in). If they must land sequentially, **Music Theory's `chord_engine.rs` lands first** (it has no dependency on the planner side).

---

## 6. FREEZE VERDICT — **SAFE** (no engine.rs edit, no golden move)

The freeze hinge and why each protected thing is insensitive:

1. **`engine.rs` is not opened.** All edits are in `composition.rs`, `chord_engine.rs`, `mappings.json`. The sha `e50c7db1…48261` is untouched.

2. **`engine_equivalence` (9/9) cannot move.** Those goldens are hand-built fixed plans whose pinned sections carry `theme: None` (established in S38 §3e / S39 `motif_s39.rs` freeze comment). The motif/theme path is never exercised by them — they pin the kernel's free-select path, which this slice does not touch.

3. **The S39 byte-identity invariant is preserved by construction.** `resolve_motif(archetype, range, len)` is retained as a wrapper calling `resolve_motif_celled(.., 0)`, and **cell 0 of every archetype == the S39 `rhythm_profile()` value** (hard constraint in §3.3). So every one of the **11 existing `resolve_motif` call sites** (`composition_s15.rs:258/378/379/388`, `chord_engine.rs` units `:6771…:7083`, `motif_s39.rs:89/90/195/244`) gets a byte-identical `Vec<MotifNote>` — the rhythm-cell machinery is inert at index 0. `composition_s15.rs:258` reconstructs the planner's motif by calling `resolve_motif`; that reconstruction stays valid **only if the planner still selected cell 0 for that fixture** — see DP-B (the S15 reconstruction may need to call `resolve_motif_celled` with the planner's actual `pick_rhythm_cell` index, or its fixture must be one that lands on cell 0). This is the single golden-adjacent watch-item; it is in `composition_s15.rs` (a structural test, freely re-blessable), NOT in `engine_equivalence`.

4. **The realize path widens, it does not change shape.** `motif_step_at` / `theme_melody_pitch` (`chord_engine.rs:2567/2610`) read `dur_steps` exactly as in S39 — a cell index never reaches them; they only ever see a resolved `Vec<MotifNote>`. The S39 freeze hinge ("an all-`dur==1` motif realizes 1:1, no continuation reachable") still holds because every cell is still a list of `>= 1` weights consumed by the unchanged accumulation loop, and a busier cell only produces *more* `dur==1` notes (which the realizer already plays 1:1) — it never produces a held note that crosses a kernel step. `motif_s39.rs::test_all_dur1_motif_realizes_identically_to_pre_s39_index` continues to pass unchanged.

**The byte-identity hinge in one sentence:** *cell 0 == the S39 profile + a thin wrapper that defaults to cell 0 ⇒ every existing caller, golden, and freeze test sees the identical pre-S41 bytes; only the planner, by opting into a non-zero cell, changes anything — and only on the theme path the goldens never touch.*

No path here requires touching `engine.rs`. **SAFE.**

---

## 7. OBJECTIVE property tests (Test Engineer; mirror `tests/motif_s39.rs` idiom)

New file `tests/motif_s41.rs`. Deterministic, RNG-free (planner `plan()` + `load_mappings` + direct `resolve_motif_celled`), run under DEFAULT features (`cargo test --test motif_s41`). The SUBJECTIVE "less same-y?" verdict is the operator's ear + taste subagents — NOT designed for here.

1. **P1 — same contour, different gait (the core defect closed).** For a fixed archetype, two cell indices `i != j` (both in range) produce `dur_steps` sequences that differ in `>= 1` position: `dur_seq(resolve_motif_celled(A, r, l, i)) != dur_seq(resolve_motif_celled(A, r, l, j))` for at least one `(i,j)` pair per archetype. This is the property that *could not hold* pre-S41 (rhythm was archetype-fixed).

2. **P2 — cross-image rhythm spread over a fixture set.** Across `>= 16` varied image fixtures sweeping `edge_activity × complexity` (the two index axes) at multiple archetype-selecting affect points, collect the realized `(archetype, dur_seq)` pairs via `planner.plan(u)`. Assert **`>= 10` distinct `dur_seq` values observed** (vs. the pre-S41 ceiling of 6 distinct dur-sequences across all 8 contours). Log each `(archetype, cell_index, dur_seq)` with `eprintln!` for `--nocapture`.

3. **P3 — ≤cap share for any single gait.** Of the realized `dur_seq` values in P2's fixture set, **no single `dur_seq` exceeds ~30% share** (`cap = ceil(n * 0.30)`, the same anti-collapse bound `motif_s39.rs:219` uses for archetype spread) — proves the selector spreads across cells rather than re-collapsing onto cell 0.

4. **P4 — per-image rhythm variance floor across same-contour images.** Take `>= 4` fixtures that all select the SAME archetype (hold the affect quadrant fixed, vary only `edge_activity`/`complexity`); assert they yield `>= 2` distinct `dur_seq` values — i.e. holding contour constant, the image still moves the gait. This is the direct clap-test proxy.

5. **P5 — cell-0 back-compat anchor (freeze guard).** `resolve_motif_celled(A, r, l, 0) == resolve_motif(A, r, l)` for every archetype A over a grid of `(r, l)` — pins the freeze anchor (cell 0 reproduces S39) so a future cell-table edit that disturbs index 0 fails HERE before it can move any S39 golden.

6. **P6 — contract preserved at every cell.** For every archetype × every cell index × a grid of `length_steps`: every `dur_steps >= 1` and `Σ dur_steps <= length_steps` (the `resolve_motif` invariants, now over the whole cell vocabulary, not just the single profile).

---

## 8. Decision points for the lead / operator

- **DP-A (index cuts — taste call, Affect/Aesthetics).** `pick_rhythm_cell`'s exact band edges (which `edge_activity` value crosses from cell 0→1→2→3, and how `complexity` breaks the within-band tie). Architect seed: equal-width `edge_activity` bands into `cell_count`, `complexity` parity as the within-band ±. Surface for Affect/Aesthetics to tune; does not block the build (the seed is buildable).
- **DP-B (S15 reconstruction watch-item).** `composition_s15.rs:258` rebuilds the planner's motif via `resolve_motif(archetype, …)`. If that fixture's image now selects a non-zero cell, the reconstruction must switch to `resolve_motif_celled(archetype, …, pick_rhythm_cell(u, archetype, K))` (or the fixture must be one that lands on cell 0). Test Engineer resolves at integration; this is a **structural-test re-bless, NOT a freeze break** (`composition_s15` is freely re-blessable; `engine_equivalence` is untouched).
- **DP-C (cell vocabulary size K).** Recommend **K = 4** (yields 32 effective gait pairs, comfortably clears the P2 `>=10`-distinct bar with margin). Music Theory may vary K per archetype (`rhythm_cells()` is a slice, so K is per-archetype-flexible) — the only structural rule is cell 0 == S39 and `K >= 1`.
- **DP-D (data vs. inline, DP-6 carry-over).** Recommend **inline-Rust authoritative + JSON mirror** (the exact S39 discipline) for this slice — no loader wiring, smallest blast radius. Promoting `rhythm_cells` to a loaded `SelectTable` (so cells are tunable without recompile) is a clean follow-on slice, not this one.

---

*End of S41 Finding-B work order. The build is one independent, individually-revertable slice: decouple rhythm from contour via image-selected cells. Freeze-SAFE. Implementer (composition.rs) and Music Theory (chord_engine.rs + mappings.json) are file-disjoint; Test Engineer owns tests/motif_s41.rs.*

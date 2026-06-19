# Design S41 — Hue Inter-Bucket Gap Fix (Rust Architect work order)

**Status:** DESIGN ONLY (no `src/` code written by this work order).
**Scope:** plan-time `src/composition.rs` only. Freeze-safe by construction.
**Freeze keystone:** `src/engine.rs` byte-frozen (`e50c7db…61`); `engine_equivalence` 9/9 must stay green.
**Classification:** QG-only objective slice (no taste/affect review needed).

---

## 1. The bug — exact mechanism

### 1.1 The lookup primitive (shared, in a FROZEN-ADJACENT file — do NOT touch)

`src/mapping_loader.rs:281`:

```rust
pub fn lookup_range_map(map: &HashMap<String, String>, value: f32) -> Option<String> {
    for (key, val) in map.iter() {
        if let Some((a, b)) = parse_range(key) {     // parse_range -> (i32, i32)
            if value >= a as f32 && value <= b as f32 {
                return Some(val.clone());
            }
        }
    }
    None
}
```

`parse_range` (`mapping_loader.rs:292`) parses each key `"lo-hi"` to **integer** endpoints `(a, b): (i32, i32)`. The membership test is the closed interval `[a, b]` over `f32`.

### 1.2 The bucket tables (integer endpoints, with ~1° gaps between them)

`assets/mappings.json composition.home_root.hue_to_pc` (S40):

```
0-29, 30-59, 60-89, 90-119, 120-149, 150-179, 180-209, 210-239, 240-269, 270-299, 300-329, 330-359
```

`assets/mappings.json global.hue_to_mode` (pre-existing convention):

```
0-30, 31-90, 91-150, 151-210, 211-270, 271-330
```

### 1.3 Why fractional hues miss

The buckets tile the circle with **integer** endpoints and a 1-unit step between adjacent buckets (`…-29` then `30-…`; `…-30` then `31-…`). A *fractional* dominant hue that lands in the open gap `(29.0, 30.0)` (e.g. `29.5°`) satisfies:

- `hue_to_pc`: `29.5 > 29` → fails `0-29`; `29.5 < 30` → fails `30-59`. **No bucket matches.**
- `hue_to_mode`: `30.5 > 30` → fails `0-30`; `30.5 < 31` → fails `31-90`. **No bucket matches.**

`lookup_range_map` therefore returns `None`, and BOTH call sites apply their floor:

- `composition.rs:1424` (`hue_to_mode`): `.unwrap_or_else(|| "Ionian".to_string())` → **Ionian floor**.
- `composition.rs:1058` (`hue_to_pc`, inside `resolve_home_root_midi`): `None =>` arm → **`LEGACY_HOME_ROOT_MIDI` (= 60) floor**.

`dominant_hue: f32` is genuinely fractional in production — it is a perceptual average over image pixels (`understand_image_pure`), so values like `29.5`, `90.4`, `149.7` are normal, not edge cases. The defect is that ~1/30 of the hue circle (every inter-bucket gap) silently collapses to the floor.

**Mechanism class:** NOT a `==` integer match and NOT a floor/truncation — it is a **range-table with non-covering gaps** (integer-endpoint buckets that do not abut on the real line) consumed by a closed-interval `f32` test.

---

## 2. The fix — minimal, freeze-safe, applied at the two composition.rs sites

### 2.1 Why NOT fix the primitive

`lookup_range_map` is called from THREE sites:

| Site | File | Map | Frozen? |
|---|---|---|---|
| `engine.rs:384` | **`src/engine.rs`** | `global.hue_to_mode` (`global.avg_hue`) | **YES — byte-frozen, do not touch, golden-pinned** |
| `chord_engine.rs:70` | `src/chord_engine.rs` | `saturation_to_harmonic_complexity` (`sat*100`) | out of scope |
| `composition.rs:1058` | `src/composition.rs` | `home_root.hue_to_pc` | in scope |
| `composition.rs:1424` | `src/composition.rs` | `global.hue_to_mode` | in scope |

Adding a `.round()` *inside* `lookup_range_map` (or `parse_range`) would change behavior on the FROZEN `engine.rs:384` path and the `chord_engine.rs:70` saturation path — both off-scope and freeze-hostile. So the rounding is applied to the **hue value at each in-scope composition.rs call site**, leaving the primitive byte-identical.

### 2.2 The normalization helper (new, module-private, in composition.rs)

Add one small private fn near `resolve_home_root_midi` (~line 1050). `.round()` to the nearest integer degree closes every 1° gap (a hue in `(29.0,30.0)` rounds to `29` or `30`, both of which are real bucket endpoints); `rem_euclid(360.0)` normalizes wrap so `360.x`/negative-drift inputs land on the circle before rounding. Order matters: normalize-then-round.

```rust
/// Snap a fractional/denormalized hue (degrees) to the nearest integer degree on the
/// 0..360 circle, so the integer-endpoint range tables (`hue_to_pc`, `hue_to_mode`) stop
/// dropping inter-bucket fractional hues to their floor (design-s41-hue-gap-fix.md §1.3).
/// `rem_euclid` normalizes wrap/negative drift FIRST, then `.round()` lands on a real
/// bucket endpoint. A second `rem_euclid` after round keeps 359.6 -> 360 -> 0 on-circle.
fn snap_hue_to_bucket_grid(hue: f32) -> f32 {
    (hue.rem_euclid(360.0).round()).rem_euclid(360.0)
}
```

(The trailing `rem_euclid` guards the single wrap case `359.5..360.0 → round 360 → 0`, keeping the result in `[0,360)` and matching the `330-359` top bucket cleanly. Without it, `359.6` would round to `360`, which matches no `hue_to_pc` bucket — re-introducing exactly one gap at the seam.)

### 2.3 Edit site A — `hue_to_mode` (composition.rs:1423-1425)

**Current:**
```rust
        let hue_mode =
            crate::mapping_loader::lookup_range_map(&mappings.global.hue_to_mode, u.dominant_hue)
                .unwrap_or_else(|| "Ionian".to_string());
```

**Change to:**
```rust
        let hue_mode = crate::mapping_loader::lookup_range_map(
            &mappings.global.hue_to_mode,
            snap_hue_to_bucket_grid(u.dominant_hue),
        )
        .unwrap_or_else(|| "Ionian".to_string());
```

### 2.4 Edit site B — `hue_to_pc` (composition.rs:1051-1058, inside `resolve_home_root_midi`)

Normalize at the single entry of `resolve_home_root_midi` so the snap covers the lookup but NOT the `None`-home fallback (which short-circuits above it — keystone untouched).

**Current (lines 1051-1058):**
```rust
fn resolve_home_root_midi(home: Option<&HomeRootMap>, dominant_hue: f32) -> u8 {
    let Some(home) = home else {
        return LEGACY_HOME_ROOT_MIDI;
    };
    …
    match crate::mapping_loader::lookup_range_map(&home.hue_to_pc, dominant_hue) {
```

**Change to:**
```rust
fn resolve_home_root_midi(home: Option<&HomeRootMap>, dominant_hue: f32) -> u8 {
    let Some(home) = home else {
        return LEGACY_HOME_ROOT_MIDI;
    };
    let dominant_hue = snap_hue_to_bucket_grid(dominant_hue);
    …
    match crate::mapping_loader::lookup_range_map(&home.hue_to_pc, dominant_hue) {
```

The `let Some(home) = home else { return 60 }` guard executes BEFORE the snap, so the absent-block fallback is byte-identical (snap never runs on the `None` path).

### 2.5 Shared helper or two sites?

`hue_to_pc` and `hue_to_mode` do **NOT** share a lookup helper at the composition layer — `hue_to_mode` is inlined at `1424`, `hue_to_pc` is wrapped in `resolve_home_root_midi`. So this is **two edit sites**, but both consume **one new shared normalization helper** (`snap_hue_to_bucket_grid`). The fix is "shared helper, two call sites."

---

## 3. Freeze verdict

**FREEZE-SAFE. Confirmed.**

1. `src/engine.rs` is not touched. The fix is `src/composition.rs` only (one new private fn + two call-site edits).
2. `engine_equivalence.rs` constructs a fixed `&[StepPlan]` by hand and pins `decide_instrument_action`; it never routes through `CompositionPlanner::plan`, `resolve_home_root_midi`, or the `hue_to_mode` site. No hue-snap can reach it. (9/9 stays green.)
3. The freeze keystone INV-4 (`home_root: None` → 60 for all hues) is preserved: the `None` guard returns before the snap; `snap_hue_to_bucket_grid` only changes the *value handed to a present-block lookup*, never the fallback branch.
4. `engine.rs:384` (`hue_to_mode` over `global.avg_hue`) and `chord_engine.rs:70` (saturation) keep calling the byte-identical `lookup_range_map`/`parse_range` — the primitive is unmodified, so every existing golden over those paths is undisturbed.
5. On-bucket integer hues already in the tables are fixed points of `.round()` (e.g. `10.0.round() == 10.0`, `45.0.round() == 45.0`), so every currently-passing on-bucket hue maps identically — **no regression** on existing `home_s40.rs`/`valence_mode_s36.rs`/`composition_s15.rs` assertions.

---

## 4. Objective property tests

New integration test file: **`tests/hue_gap_s41.rs`** (DEFAULT features — public-API only, RNG-boundary disciplined like `home_s40.rs`; `home_root_midi` is set before any `thread_rng` in `plan()`). Plus a focused unit assertion in `src/composition.rs::home_root_tests`.

### 4.1 Integration props (via `CompositionPlanner::plan(...)`, reading `plan.key_tempo.home_root_midi` / `home_mode`)

- **P1 — inter-bucket gap now snaps (hue_to_pc).** For each gap hue in `{29.5, 30.0 (on-edge), 59.5, 89.5, 119.5, 149.5}` with the shipped `home_root` block: assert `home_root_midi != 60` (it no longer falls to the floor) AND `home_root_midi ∈ [57,68]`. Specifically assert `29.5` snaps to the SAME pc as the nearer integer bucket: `29.5 → round 30 → pc 1` (D# seated), distinct from `pc 0`.
- **P2 — inter-bucket gap now snaps (hue_to_mode).** For gap hues `{30.5, 90.5, 150.5, 210.5, 270.5, 330.5}`: assert the resolved `home_mode` is the nearest real bucket's mode (e.g. `30.5 → round 31 → Lydian`, NOT the `Ionian` floor) — read via a plan whose valence is neutral so `valence_family_mode` is a no-op projection, exposing the raw hue-mode pick. (If a neutral-valence no-op isn't directly observable through `plan`, assert instead that two gap hues straddling a boundary — `30.4` vs `30.6` — yield DIFFERENT homes/modes, proving neither collapses to a shared floor.)
- **P3 — on-bucket no-regression.** For on-bucket hues `{0.0, 10.0, 45.0, 100.0, 200.0, 300.0}`: assert `home_root_midi` and `home_mode` are IDENTICAL to the pre-fix values (snapshot the integer-hue results; `.round()` is identity on integers, so these must not move).
- **P4 — wrap seam.** Assert `359.6` (rounds to `360 → rem_euclid → 0`) maps to the SAME home as `0.0` (pc 0, top of circle wraps to red), and does NOT fall to 60.
- **P5 — full-circle no-floor sweep.** Sweep `hue` from `0.0` to `360.0` in `0.5°` steps with the shipped block; assert `home_root_midi ∈ [57,68]` (never 60-by-miss) on EVERY step AND assert the distinct-home count is unchanged-or-greater vs the integer-only sweep (differentiation preserved).

### 4.2 Freeze-keystone re-assertion (in the new file, cheap and load-bearing)

- **P6 — absent-block fallback intact.** With `PlanMappings { home_root: None, .. }`, sweep the SAME `0.5°` grid (including gap hues `29.5`, `30.5`): assert `home_root_midi == 60` on every hue. This proves the snap did not perturb the freeze keystone (mirrors `home_s40.rs` INV-4 but at sub-integer resolution).

### 4.3 Unit assertion (add to `src/composition.rs::home_root_tests`, `--lib --no-default-features`)

- **U1 — snap helper correctness.** Direct on `snap_hue_to_bucket_grid`: `29.5 → 30.0`, `29.4 → 29.0`, `359.6 → 0.0`, `-0.4 → 0.0` (negative drift), `360.0 → 0.0`, `45.0 → 45.0` (identity). Plus `resolve_home_root_midi(Some(&home), 29.5)` with the `{0-29→0, 30-59→7}` fixture now returns the `pc 7` (G) seat (rounds up to bucket `30-59`), NOT 60 — the direct module-private witness of the fix.

---

## 5. Summary

- **Mechanism:** integer-endpoint range tables with 1° non-covering gaps + closed-interval `f32` membership test → fractional production hues in the gaps match no bucket → `None` → floor (60 / Ionian). Same root cause in both `hue_to_pc` (S40) and `hue_to_mode` (pre-existing).
- **Fix:** new private `snap_hue_to_bucket_grid(hue) = hue.rem_euclid(360.0).round().rem_euclid(360.0)`, applied to the hue value at the two composition.rs lookup sites (`1424` hue_to_mode; entry of `resolve_home_root_midi` for hue_to_pc at `1051`). One shared helper, two call sites.
- **Freeze:** SAFE — engine.rs untouched, primitive untouched, `None` fallback short-circuits before the snap, on-bucket hues are round-fixed-points, engine_equivalence does not route through these paths.
- **Tests:** `tests/hue_gap_s41.rs` (P1–P6 via public `plan()`) + a `home_root_tests` unit (U1) on the helper and `resolve_home_root_midi`.

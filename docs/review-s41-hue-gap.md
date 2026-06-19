# Quality Gate Review — S41 Hue Inter-Bucket Gap Fix

**Reviewer:** Quality Gate (Specialist 6)
**Date:** 2026-06-19
**Work order:** `docs/design-s41-hue-gap-fix.md`
**Change under review:** `src/composition.rs` only — new module-private `snap_hue_to_bucket_grid` + two call-site edits; new test file `tests/hue_gap_s41.rs`.
**Verdict:** **PASS WITH ISSUES** (only non-blocking issues; safe to commit)

---

## 1. Freeze verification (independent)

- `sha256sum src/engine.rs` = `e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261` — **MATCHES** the pinned keystone hash. engine.rs byte-identical.
- `git diff --stat` shows **`src/composition.rs` ONLY** (15 insertions, 3 deletions). No other src/ file touched. engine.rs NOT in the diff.
- The fix is a new private fn + two call-site edits, exactly as the work order specifies.

## 2. Full net (independently re-run)

| Suite | Command | Result |
|---|---|---|
| Build (default) | `cargo build` | Clean (pre-existing modem_encode / unpack_tiled_payload warnings only, unrelated) |
| Lib, no-default | `cargo test --lib --no-default-features` | **180 passed, 0 failed** |
| Engine equivalence | `cargo test --test engine_equivalence` | **9 / 9 passed** (keystone golden intact) |
| Hue gap S41 | `cargo test --test hue_gap_s41` | **11 / 11 passed** |
| Broad (default, all targets) | `cargo test` | **ALL GREEN** — lib 223, main 14, plus every integration target incl. home_s40 9/9, valence_mode_s36 7/7, composition_s15 5/5, modem_roundtrip 17/17, modem_realair 10/10. Zero failures across the entire workspace. |
| Clippy | `cargo clippy --tests` | No warnings/errors on changed code (composition.rs, hue_gap_s41.rs) |

## 3. Freeze keystone preserved (read the actual diff)

Confirmed by reading `resolve_home_root_midi` (composition.rs:1060-1075):

```rust
fn resolve_home_root_midi(home: Option<&HomeRootMap>, dominant_hue: f32) -> u8 {
    let Some(home) = home else {
        return LEGACY_HOME_ROOT_MIDI;   // <-- None path returns 60 BEFORE the snap
    };
    let dominant_hue = snap_hue_to_bucket_grid(dominant_hue);   // <-- snap only on present-block path
    ...
}
```

The `None`-home guard short-circuits to `LEGACY_HOME_ROOT_MIDI` (60) **before** the snap line. The snap can never run on the absent-block path → INV-4 (`home_root: None` → 60 for all hues) is byte-for-byte preserved. P6 in the test net re-proves this at 0.5° resolution including the gap hues (29.5/30.5) — 720 hues all resolve to 60 with `home_root = None`. The `hue_to_mode` site (composition.rs:1430-1437) likewise snaps only the value handed to the lookup; the `.unwrap_or_else("Ionian")` floor is untouched.

## 4. Test quality — real, not gamed

`tests/hue_gap_s41.rs` (11 tests) asserts meaningful, non-tautological properties:

- **P1** (`hue_to_pc` gap snap): gap hues 29.5/59.5/89.5/119.5/149.5 resolve in-band [57,68] AND `!= 60`; **pins the exact target** — 29.5 → pc 1 (C#), distinct from the pc-0 home of the 0-29 bucket, and equal to the on-edge 30.0 home. Not "somewhere in band."
- **P2** (`hue_to_mode` gap snap): 30.5 → Lydian (not Ionian floor); a straddling pair 30.4→Phrygian vs 30.6→Lydian proves the round picks the *nearer* bucket; a 5-gap sweep checks each interior modal gap maps to its up-rounded bucket mode.
- **P3** (no-regression): on-bucket integers {0,10,45,100,200,300} pinned to **exact shipped-table pc and mode values** (not "unchanged vs itself"). This is the claim that `.round()` is identity on integers — directly tested. Confirmed correct against `assets/mappings.json` (hue_to_pc and hue_to_mode tables read independently; all 6 cases match).
- **P4** (wrap seam): 359.6 ≡ 0.0 for both home and mode, pc 0; guards the trailing `rem_euclid`.
- **P5**: full 0.5° sweep all in-band, plus distinct-home count == 12 for both fractional and integer sweeps (differentiation preserved, gap-collapse would shrink it).
- **P6**: freeze-keystone re-assertion (above) + a corollary that the shipped block never floors-by-parse on gap hues.
- A **premise guard** test verifies the neutral-valence dead-band assumption that makes `home_mode` observable as the raw hue→mode pick — so P2/P3-mode can't silently test the wrong quantity.

No `assert!(true)`, no tautologies. The on-bucket round-fixed-point claim (item 4 of the charter) is explicitly tested in P3.

## 5. Helper correctness (independently re-derived)

Compiled `snap_hue_to_bucket_grid` standalone and swept `[0,720)` at 0.1° resolution:

- **0 cases** land outside integer `[0,359]` after snap → no new gap is introduced anywhere, including the `359.5→360→0` seam (the trailing `rem_euclid` does its job).
- Half-integer midpoints round **up / away from zero** consistently (29.5→30, 89.5→90, … 329.5→330) — Rust `f32::round` is round-half-away-from-zero, so straddle pairs are well-defined and there is no banker's-rounding surprise at exact `.5`.
- Negative drift (-0.4→0), exact 360.0→0, and identity on integers (45→45, 10→10) all correct.

**One behavior worth the lead's awareness (correct, not a defect):** `359.5` snaps to `0`, so the top half of the `330-359` bucket's trailing gap `(359.0, 360.0)` maps to pc 0 (red/C), not pc 11. This is intended color-wheel wrap (360° = 0° = red) and is exactly what P4 pins. No off-by-one at bucket edges: each integer endpoint sits inside its own closed `[a,b]` bucket, and after snap every value is an integer that some bucket contains.

## 6. The flagged open item — U1 direct unit witness (NON-BLOCKING)

The Test Engineer flagged that the direct unit assertion on the private helper (`snap_hue_to_bucket_grid` and `resolve_home_root_midi(Some(&home), 29.5)`) was not written, since it would require a production-visibility change or live in `src/composition.rs::home_root_tests`. Confirmed: that module exists (S40 tests at composition.rs:1077+) but contains **no** new gap-fix unit witness.

**My call: NOT BLOCKING.** Rationale:
- The integration net already exercises the helper through the public `plan()` path with the helper's full behavioral contract pinned: P1 pins 29.5→pc1 (the up-round), P4 pins the wrap seam, P3 pins on-integer identity, P5 pins full-circle no-floor. These are the same properties U1 would assert, reached through the real call site rather than a unit harness.
- I independently re-derived the helper math in isolation (§5) — the property U1 would check (round + double rem_euclid closes gaps and the seam without a new gap) holds across a dense sweep.
- The helper is genuinely module-private; adding U1 inside `home_root_tests` is a *nice-to-have* defense-in-depth, not a coverage gap that lets a real defect through. The behavior is not under-tested.

If the lead wants belt-and-suspenders, the cheap follow-up is to add U1 to `home_root_tests` (same file the helper lives in, no visibility change needed). I recommend it as a follow-up, not a merge gate.

## 7. Module boundary / integration

- All changes confined to `src/composition.rs` (the in-scope file). Test file is QG-owned `tests/`. No agent touched a file it doesn't own.
- No API signature changes — `snap_hue_to_bucket_grid` is module-private; `resolve_home_root_midi` signature unchanged. No new types crossing module boundaries. No TODO/incomplete-integration markers.
- `mapping_loader`/`parse_range` primitive untouched → engine.rs:384 and chord_engine.rs:70 paths undisturbed (confirmed: engine.rs hash unchanged, full suite green).

## 8. Issues

**Blocking:** none.

**Non-blocking:**
1. `cargo fmt -- --check` reports two cosmetic diffs in `tests/hue_gap_s41.rs` (comment-column alignment at :226 and a long-array line-wrap at :442). Test file only, no semantic effect. Run `cargo fmt` before commit to clear.
2. (Recommendation, not a defect) Add the U1 unit witness to `src/composition.rs::home_root_tests` as a follow-up — see §6.

---

## Verdict

**PASS WITH ISSUES.** The fix is correct, minimal, and freeze-safe: engine.rs byte-identical, the `None` keystone preserved (guard precedes the snap, re-proven at sub-integer resolution), the helper math independently re-derived to close every gap and the wrap seam with no new gap or edge off-by-one, and the 11-test net validates real properties (not tautologies) including the on-integer no-regression claim. Full workspace test suite is green (engine_equivalence 9/9). The only issues are a cosmetic `cargo fmt` pass on the test file and an optional U1 follow-up — neither blocks commit. The flagged missing unit witness is **not** a real coverage gap given the integration net plus the standalone re-derivation. **Safe to commit after `cargo fmt`.**

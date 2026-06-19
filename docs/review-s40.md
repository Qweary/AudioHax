# Quality Gate Review — S40 Slice-2 "per-image HOME" (Finding #1)

**Reviewer:** Quality Gate (AudioHax)
**Date:** 2026-06-18
**Governing spec:** `docs/design-s40-slice2-workorder.md` (§4 invariants, §5 file-disjoint split, §6 keyplan freeze-watch, §7 freeze verdict); `docs/design-s38-synthesis.md` §4 Slice-2.
**Branch:** `master` (working tree, uncommitted)

## Overall Verdict: **PASS**

The change makes the planner's home root image-derived (dominant hue → chromatic pitch class → seated into MIDI band [57,68]) with a defensive byte-for-byte fallback to 60 when the optional block is absent or a hue/pc fails to resolve. The implementation conforms to the work order on every axis: build, lint, tests, module boundaries, musical logic, test quality, integration, and the frozen-kernel guarantee. The freeze keystone (`engine.rs` sha unchanged, `engine_equivalence` 9/9) is intact.

---

## 1. Compilation

`cargo build --release` — **PASS** (Finished in 15.65s). Main binary builds pure-Rust by default. The only warnings emitted are pre-existing dead-code/unused-assignment warnings in unrelated `modem_encode` / `unpack_tiled_payload` binaries; **zero** warnings from `composition.rs` or `mapping_loader.rs`.

## 2. Lint

- `cargo fmt -- --check` — **clean** (exit 0, no diff). NON-BLOCKING gate satisfied with nothing to report.
- `cargo clippy -- -W clippy::all` — **0 errors; 0 correctness-class warnings.** 71 total warning lines, all style-class (doc-list indentation, `div_ceil`/`is_multiple_of` suggestions, loop-index style, same-type casts) and all located in unrelated modem/payload/channel-sim bins and pre-existing lib code. **Zero clippy findings in the two changed Rust files.** Per the post-S11 convention (correctness BLOCKING, style NON-BLOCKING), nothing here blocks.

## 3. Test Results

| Target | Result |
|---|---|
| `cargo test --lib --no-default-features` | **180 passed, 0 failed** (incl. the new inline `home_root_tests` unit net) |
| `tests/home_s40` (NEW) | **9 passed, 0 failed** |
| `tests/engine_equivalence` | **9 passed, 0 failed** (freeze keystone) |
| `tests/keyplan_k2a` | **9 passed, 0 failed** |
| `tests/keyplan_k2b` | **14 passed, 0 failed** |
| `tests/keyplan_k3` | **4 passed, 0 failed** |
| `tests/keyplan_s25` | **11 passed, 0 failed** |
| `tests/keyplan_s29` | **4 passed, 0 failed** |
| `tests/diversity_s13` | **10 passed, 0 failed** |
| `tests/composition_s15` | **5 passed, 0 failed** |
| `tests/motif_s39` | **5 passed, 0 failed** |
| `cargo test` (full default-features catch-all) | **472 passed, 0 failed** (aggregate across all targets) |

No failures anywhere in the tree.

## 4. Module Boundary Audit (per-file, work-order §5)

`git status` / `git diff --stat` confirm the exact changed set; every file maps to its declared owner with **no cross-ownership**:

| File | Owner (§5) | Change | Verdict |
|---|---|---|---|
| `src/composition.rs` | Implementer | `resolve_home_root_midi` + `seat_pc_in_band` helpers; `home_root: Option<HomeRootMap>` on `PlanMappings` + `From` row; call-site replacement at the home-derivation line; inline `#[cfg(test)]` `home_root_tests`; `HomeRootMap` added to the `mapping_loader` import | **In-lane** |
| `src/mapping_loader.rs` | Implementer | `HomeRootMap` / `HomeBand` deserialize types (with `Debug, Clone, PartialEq, Deserialize`); `#[serde(default)] pub home_root: Option<HomeRootMap>` on `CompositionMappings` | **In-lane** |
| `tests/keyplan_k2a.rs` | Implementer | TWO comment rewords only (`:289` docstring addendum, `:304` const comment); **value `60` and all assertions UNCHANGED** (diff confirms) | **In-lane** |
| `assets/mappings.json` | Music Theory | new `composition.home_root` block ONLY (band `{lo:57,hi:68}`, uniform 30°/pc `hue_to_pc`, 12 buckets, rationale `_note`); no other JSON touched | **In-lane** |
| `tests/home_s40.rs` (NEW) | Test Engineer | 9 invariant tests (INV-1..INV-5) through the public planner; references only public API + the module-private helpers' *behavior* (never the private fns directly) | **In-lane** |
| `docs/design-s40-slice2-workorder.md` (NEW, untracked) | Rust Architect | the work order itself (design doc) | **In-lane** |

**`src/engine.rs` NOT in the changed set** (`git status --short src/engine.rs` empty). **No agent modified a file it did not own.** Disjointness holds.

## 5. Musical Logic Review (Stage 3)

- **hue → pc → [57,68] seating is musically sound and register-safe (GR-2).** `seat_pc_in_band(pc, lo, hi)` places the pc in `lo`'s octave then lifts one octave if below `lo`. Because the shipped band spans exactly 12 semitones (`hi − lo == 11`, [57=A3 .. 68=G#4]), every one of the 12 pitch classes has exactly one in-band representative, so the single lift always lands `note ∈ [lo,hi]` — proven, not clamped. The code correctly `debug_assert!`s the upper bound rather than adding a top clamp that would silently mask a band-width regression (matches §2.3 GR-2 directive). The inline unit test `seat_pc_in_band_all_pcs_in_band` verifies all 12 pcs land in-band AND reduce back to the requested pc.
- **The color-wheel → chromatic-wheel mapping is musically meaningful, not arbitrary.** Music Theory chose a uniform 30°/pc synesthetic identity (red=hue0→C, ascending clockwise around both 12-fold circles). The JSON `_note` justifies uniform-over-perceptual on the differentiation goal (a 30° hue rotation always lands a new chromatic home; no large hue range collapses to one pc), and reconciles it with `global.hue_to_mode` so color heat tracks rising chromatic center without contradicting the mode the same hue picks. This is a defensible aesthetic judgment, single-written by the owning lane.
- **Moving the home rotates pitch classes WITHOUT moving absolute register.** Confirmed against the downstream-flow trace in §1.2 and re-read in code: realizer consumers reduce `home_root_midi` to a pitch class (`% 12` / `rem_euclid(12)`) and re-seat by role floor via `seat_pc_in_register(pc, floor)`, which is independent of the home's absolute octave. So the bass does not boom and the melody does not shriek when the home moves — the design's central register-safety claim. The section-re-root path is a uniform transposition (`home + key_offset_semitones`), keeping excursions relative.
- **GR-1 (one home per piece) is structural.** `home_root_midi` is a single field on the plan/`KeyTempoPlan`; no section re-derives its own home. INV-1 asserts this structurally (re-read of the single field) plus offset-0 on all home-role sections.
- **Band [57,68] is held; `hi` not raised above 68.** The shipped JSON band is `{lo:57,hi:68}`; the load-bearing register-safety guard is respected.

## 6. Test Quality Review (Stage 4)

The 9 `home_s40` tests assert REAL properties, not `is_ok()`/non-empty:

- **INV-2** — full-circle 5° sweep AND a fractional 1° sweep both assert in-band membership `[57,68]` on every hue (the fractional sweep also guards that the fallback stays in-band).
- **INV-3** — cross-bucket hues (10°→C vs 200°→F#) assert **distinct** homes; a same-bucket counterpart (5°,25°) asserts a **tie** (proving variation is bucket-driven, not noise); a full-sweep distinct-count asserts `>= 6` AND tightly `== 12` (full chromatic coverage of the shipped map). This is a meaningful variance check, not a trivial non-empty.
- **INV-1** — premise asserts the hue-200 home is in-band AND `!= 60` (so the test genuinely exercises a per-image home), then every Statement/Return section offset-0, then the single-center structural re-read.
- **INV-4 (freeze keystone)** — genuinely exercised: clones the shipped mappings, NULLs `home_root`, and asserts `== 60` byte-for-byte across a full hue sweep. A corollary (`present vs absent diverge at a non-C hue`) proves the absence path is a real branch, not an accidental always-60 no-op. This is exactly the invariant the freeze argument rests on.
- **INV-5** — asserts the per-section offset structure is byte-identical across two different-home pieces (uniform transposition), re-roots each section explicitly and proves `root_a − root_b == home_delta` for all sections, and confirms home-role offset-0 / excursion-in-menu. Proves no consumer hardcodes 60.

The inline `home_root_tests` adds 4 unit tests (all-pcs-in-band, none→60 sweep, cross-bucket differ + pc-correct, bad-data→60 for out-of-range/unparseable/unmatched). Together the net covers the populated path, the absent path, and the bad-data defense at both unit and integration altitude.

## 7. Integration Assessment (Stage 5)

- **JSON deserializes against the landed type field-for-field.** `tests/home_s40.rs::base_plan_mappings` calls `load_mappings("assets/mappings.json")` then `.expect("composition block present")` and `.into()` — all 9 tests pass, proving the shipped `composition.home_root` block (`band{lo,hi}` + `hue_to_pc` HashMap) deserializes cleanly against `HomeRootMap`/`HomeBand`. No type mismatch.
- **Imports clean** — `HomeRootMap` added to the `composition.rs` use-list; `lookup_range_map` is `pub` in `mapping_loader.rs` and reused verbatim. No missing import; no leftover TODO in the changed code.
- **Both paths proven.** Populated path: INV-1/2/3/5 + `home_s40` corollary. Defensive fallback path: INV-4 + the inline none/bad-data tests. `engine_equivalence` (which carries no block) staying 9/9 is the live proof the fallback keeps non-block fixtures on legacy behavior.

## 8. Freeze Verification

- **`src/engine.rs` sha256:** `e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261` — **MATCHES** the frozen anchor exactly (verified before and after the full test run; `git status` shows engine.rs unmodified).
- **`engine_equivalence`:** **9/9 PASS.**
- **No golden fixture moved; no realizer seam added.** `home_root_midi` enters the kernel only as an existing `ctx.key_tempo.home_root_midi` field consumed as `% 12` or uniform transposition. Freeze verdict (§7) confirmed: **SAFE, no FREEZE-BREAK.**

## 9. Blocking Issues

**None.**

## 10. Non-Blocking Issues / Notes

- **NB-1 (fractional-hue gap → 60 fallback) — CONFIRMED CONSISTENT WITH EXISTING CONVENTION.** `lookup_range_map` uses inclusive integer-bound compares (`value >= a && value <= b` where `a,b` are parsed integers). The shipped `hue_to_pc` buckets are `"0-29","30-59",…` so a hue in a 1° integer gap (e.g. 29.5°) matches no bucket and falls to the legacy-60 fallback. **I agree with the lead this is NOT a defect:** (a) it exactly matches the pre-existing shipped `global.hue_to_mode` convention, which uses the identical `lookup_range_map` primitive and therefore has the same fractional-gap behavior (e.g. 30.5° between `"0-30"`/`"31-90"` also falls through); (b) the fallback is a valid, in-band musical result (60 ∈ [57,68]), so GR-2 is never violated by it — the `test_inv2_home_in_band_fractional_hue_sweep` test explicitly proves the fractional path stays in-band. Recording as a non-blocking note, not a flag. If a future slice wants gapless coverage it can widen the cuts to `"0-30","30-60",…` (overlapping integer edges resolve to whichever bucket `iter()` reaches first, which is already the existing convention's behavior at shared edges), but this is out of scope and not required.
- **NB-2 (clippy style warnings) — pre-existing, out of scope.** 71 style-class clippy warnings exist tree-wide (modem/payload/channel-sim bins + pre-existing lib code). None are correctness-class and none touch the changed files. Not introduced by this slice.
- **NB-3 (keyplan_k2a `:304` const) — correctly preserved.** The `const HOME: u8 = 60` value and the `harmony_reroots` assertions are unchanged (diff-verified); only the trailing comment was reworded for post-S40 accuracy. `harmony_reroots` bypasses the planner entirely (calls `generate_chords` directly), so it is structurally immune to the per-image home derivation, as §6 predicted.

---

*End of review-s40.md. Verdict: PASS. Freeze intact (engine.rs sha matched, engine_equivalence 9/9). 472 tests pass, 0 fail.*

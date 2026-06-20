# Quality Gate Review — Session 45, Slice 1 (variety arc)

**Reviewer role:** Quality Gate (correctness checkpoint before lead integration). VERIFY only — no production code, asset, or commit touched by this review.
**Date:** 2026-06-19
**Design ground truth:** `docs/design-s44-variety-nvoice.md` (§2 reconciled code truth, §5 freeze ledger, §6 slice plan, §8 gates).
**Working tree state:** DIRTY (uncommitted). Reviewed against the dirty tree.

**Slice under review — three freeze-safe moves:**
- **Move 1 (`assets/mappings.json`):** relaxed the `texture` SelectTable `pad_bed_counter` rule — `foreground_energy` 0.35→0.15, `fg_bg_contrast` 0.20→0.10 (:343-344). Routes the existing `CounterMelody` species voice into the default for ordinary images; calmest images still fall to `pad_bed`.
- **Move 2 (`src/composition.rs`):** per-section figuration variation — new `section_figuration_id(base_id, role)` helper + a per-section override in the section loop. Anchor roles keep base figuration; departure roles take a contrasting cell. Fires only when `figuration.is_some()`.
- **Move 3 (`src/chord_engine.rs`):** COMMENT-ONLY corrections — stale "stubbed / delegates to HarmonicFill" prose replaced with a true description of the live species realize path. No behavior change.
- **Tests (`tests/variety_s45.rs`):** 5 new integration tests pinning routing / calm-fallback / rule-ordering / per-section figuration / identity byte-neutrality.

---

## Compilation Status

**PASS.** `cargo build --release` finished clean (`Finished release profile` in ~16s). The only warnings emitted are pre-existing, in unrelated binaries (`src/bin/modem_encode`: `unused_assignments`; `src/bin/unpack_tiled_payload`: dead `encoding` field) — none in any file this slice owns.

## Lint Status

- **`cargo fmt -- --check` (NON-BLOCKING):** No fmt drift in any owned file (`composition.rs`, `chord_engine.rs`, `variety_s45.rs`) — owned-file fmt is clean. (Pre-existing repo-wide drift exists in unrelated files; not this slice's defect and not actioned, per scope.)
- **`cargo clippy --all-targets -- -W clippy::all` (correctness BLOCKING / style NON-BLOCKING):** No correctness warnings introduced by the slice. All emitted warnings are pre-existing style lints (`doc list item indentation`, `needless_range_loop`, `manual div_ceil`, etc.), overwhelmingly in `modem.rs` / DFIR bins. The 2 warnings nominally attributed to the `variety_s45` test target are `modem.rs` lints surfaced through `--all-targets` cross-compilation, NOT defects in the test or owned files. **No blocking lint.**

## Test Results

**`cargo test`: PASS — 517 passed / 0 failed / 0 ignored.**

Specifically:
- **`engine_equivalence`: 9 passed / 0 failed (9/9 byte-green).** ✅
- **`variety_s45`: 5 passed / 0 failed (5/5).** ✅ (Properties A–E: routing, calm-fallback, rule-ordering, per-section figuration, identity byte-neutrality.)

No skips, no failures anywhere in the suite.

## Freeze Verification

- **`sha256sum src/engine.rs`** = `e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261` — **EXACTLY equals the frozen hash.** Re-verified twice (start and after the test run); unchanged. `engine.rs` is byte-frozen and untouched. ✅
- **`engine_equivalence`: 9/9 byte-green.** ✅
- `git status --short` shows `engine.rs` is NOT in the modified set. ✅

**Freeze verdict: HELD.**

## Module Boundary Audit

`git status --short` shows EXACTLY the four expected paths and nothing else:
```
 M assets/mappings.json
 M src/chord_engine.rs
 M src/composition.rs
?? tests/variety_s45.rs   (untracked — new test, expected)
```
- No locked file modified: `engine.rs`, all `image_*`, `midi_output`, `modem`, `lib`, `synth*`, `cli`, `main`, `tui`, `bin/*` — all untouched. ✅
- **`composition.rs`:** planner logic only. New code is `section_figuration_id` (pure fn over `base_id` + `ThematicRole`) + a per-section `figuration_resolved` re-resolve via the existing `lookup_figuration`. No image processing, no MIDI calls, no synth. ✅
- **`chord_engine.rs`:** verified TRULY comment-only (see Move 3 below). Zero non-comment lines changed. ✅

**Module boundary verdict: CLEAN.**

## Musical / Logic Review (per move)

### Move 1 — `assets/mappings.json` routing (FREEZE-SAFE)

- **Backward-compat / parse:** the diff edits only the two numeric `lo` thresholds inside the existing `pad_bed_counter` `when` rule; shape is unchanged. The mappings load successfully (the entire `variety_s45` net loads `assets/mappings.json` via the real `load_mappings`, and all 517 tests — many loader-backed — pass). Thresholds 0.15 / 0.10 are in sensible `0..1` range. ✅
- **Rule ordering (the load-bearing claim):** read the full `texture` SelectTable (:339-348). Order is: **(1) `pad_figured`** (`subject_energy ≥ 0.45 AND fg_bg_contrast ≥ 0.25`) → **(2) `pad_bed_counter`** (now `foreground_energy ≥ 0.15 AND fg_bg_contrast ≥ 0.10`) → (3) arousal/valence rules → default `pad_bed`. First-match-wins. A high-subject image (≥0.45 / ≥0.25) still hits `pad_figured` FIRST, so the relaxed counter rule cannot steal it — pinned by Property C with a non-vacuous guard (the fixture is asserted to also satisfy the relaxed counter gate). A calm image below the new floor (fg_energy < 0.15 OR contrast < 0.10) falls through to the `pad_bed` default — pinned by Property B. ✅
- **`pad_bed_counter` layer set genuinely swaps Fill→Counter:** `pad_bed_counter` (mappings :265) carries `["Bass","Pad","CounterMelody","Melody"]` — it contains `CounterMelody` and does NOT contain `HarmonicFill`. The default `pad_bed` (:264) carries `HarmonicFill`, no `CounterMelody`. So routing ordinary images to `pad_bed_counter` genuinely replaces the static fill with the moving species line. ✅

### Move 2 — `src/composition.rs` per-section figuration (FREEZE-REACHABLE)

- **(a) Fires only when `figuration.is_some()`:** the override is wrapped in `if let Some(base_fig_id) = orchestration.figuration.as_deref()` (:1623). A profile with `figuration == None` (identity, `pad_bed`, `pad_bed_counter`) never enters the block → `section_orch.figuration_resolved` stays exactly the cloned base (None on identity). Byte-neutrality argument HOLDS and is pinned by Property E (real planner on a neutral image, checking the departure Contrast section specifically). ✅
- **(b) Anchor vs departure mapping + only-existing-catalogue-ids:** `section_figuration_id` maps `Statement | Return | Coda → base_id` (anchors hold) and `Contrast | Development → broken_chord_wave` (with `broken_chord_up`↔`broken_chord_wave` inversions). Both partner ids (`broken_chord_wave` :293, `broken_chord_up` :286) exist in the `figuration_catalogue`. Unrecognized base returns its own id (no-op departure). Re-resolution uses the same `lookup_figuration` against the same catalogue; an unresolved id leaves the already-cloned base spec (never None) — graceful degrade. ✅
- **(c) Identity/non-figured no-op (freeze-safety claim):** confirmed — see (a). The override re-resolves only the EXISTING per-section `figuration_resolved` the realizer already consumes; no new realization path. ✅
- **Once-per-plan resolve not broken:** the base `orchestration.figuration_resolved` / `bass_pattern_resolved` resolves (:1524-1537) are untouched. The override operates on `section_orch = orchestration.clone()` and only re-resolves `figuration_resolved`; `bass_pattern_resolved` is carried through the clone unchanged. The single `orchestration: section_orch` substitution at the `Section` push (was `orchestration.clone()`) is the only wiring change. ✅

### Move 3 — `src/chord_engine.rs` comment corrections (the critical check)

- **Comment-only — verified by diff:** `git diff src/chord_engine.rs` touches ONLY comment lines.
  - Hunk 1 (:866-885): edits the `///` doc comment on the `CounterMelody` enum variant. The variant declaration line `CounterMelody,` is unchanged.
  - Hunk 2 (:1275-1286): edits `//` inline comments inside `role_pitch`. The actual match-arm line `OrchestralRole::HarmonicFill | OrchestralRole::Pad | OrchestralRole::CounterMelody =>` and all code below are unchanged.
  - **Zero non-comment changes.** No behavior change. ✅ (This is the load-bearing Move-3 verification.)
- **New comments are TRUE against the live code** (read the actual realize path, not just the prose):
  - `realize_rhythm` `OrchestralRole::CounterMelody` arm (:1831-1894) is a genuine moving species line: it recomputes melody pitch this/prev step for contrary motion (`melody_pitch_for` / `melody_pitch_for_step`, :1846-1847), seeds the realized previous counter pitch by deterministic replay (`realized_prev_counter`, :1872), selects its own pitch via the shared `realized_counter_pitch_with_prev` (:1876), and **rebinds every emitted event's pitch** via `let with_note = |ev| NoteEvent { note: cnt, ..ev }` (:1878), applied at every emission (:1888, :1894). Held-period activation (the guaranteed off-beat onset, :1881-1889) and oblique/rest modes are present. So CounterMelody really IS a wired species voice, not a HarmonicFill delegate. ✅
  - The `role_pitch` seat (:1284) really is a DEAD anchor for CounterMelody — overwritten on every event by `with_note`. The new comment describing it as a "harmless default (the value never sounds)" is accurate. ✅

## Test Quality Review

Read `tests/variety_s45.rs` in full. The net is high-quality and drives REAL code, not reimplementations:

- **Property A (routing):** drives the REAL `texture.select` on the SHIPPED mappings; asserts the resolved profile id is `pad_bed_counter` AND that its layer set contains `CounterMelody` and NOT `HarmonicFill`. Includes the load-bearing old-dead-band case `(0.20, 0.12)` that proves the relaxation actually fired. Specific property assertions, not `is_ok()`. ✅
- **Property B (calm fallback):** drives real `select`; asserts `pad_bed` + layer membership; each fixture deliberately fails at least one relaxed predicate. ✅
- **Property C (rule ordering):** NON-VACUOUS — explicitly asserts the salient fixture ALSO satisfies the relaxed counter gate before asserting `pad_figured` still wins, so it genuinely proves first-match ordering. ✅
- **Property D (per-section figuration):** drives the end-to-end `CompositionPlanner::plan`, GUARDS the precondition (`pm.texture.select(&img) == "pad_figured"`, so the test isn't vacuous if the figured profile fails to select), asserts anchors keep base / a departure differs / ≥2 distinct cells across the piece / every resolved id is a real catalogue entry. Strong sequence-property assertions. ✅
- **Property E (identity byte-neutrality):** drives the real planner on a neutral image, explicitly locates the departure Contrast section (the one place a leak would show), and asserts `figuration_resolved == None` everywhere; plus a direct identity-profile check. ✅

No weak/vacuous tests. Each asserts a specific id / layer-membership / per-section sequence property. RNG-boundary discipline is respected (no chord/note-value assertions). **Test quality verdict: STRONG.**

## Integration Assessment

- No type mismatches or broken callers: full suite (517) compiles and passes. The realizer consumes per-section `figuration_resolved` as before; the only change is WHICH spec each departure section resolves — a value change within the existing mechanism, not an interface change.
- `composition.rs` still compiles against the realizer (engine_equivalence 9/9 confirms the identity path is byte-stable).
- No leftover TODOs, no dead introductions, no `dbg!`/`unwrap` regressions in owned files.
- The override's graceful-degrade (unresolved id → keep base spec, never None) means a future catalogue edit cannot silently strip figuration off a figured section.

**Integration verdict: CLEAN.**

---

## VERDICT

# ✅ PASS

The slice is correct, freeze-safe, and well-tested. All five gate stages clear with no blocking findings.

**Blocking issues:** NONE.

**Non-blocking notes (informational only — not this slice's defects):**
- Pre-existing repo-wide clippy style lints (doc-comment indentation, `needless_range_loop`, `manual div_ceil`, etc.) and pre-existing fmt drift exist in unrelated files (`modem.rs`, DFIR bins). Out of scope for this slice; flagged only for the backlog.
- Pre-existing `unused_assignments` / dead-`encoding`-field warnings in `src/bin/modem_encode` and `src/bin/unpack_tiled_payload`. Unrelated to this slice.

**Key confirmations for the lead:**
- `engine.rs` sha = `e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261` (equals frozen hash); `engine_equivalence` 9/9 byte-green.
- `chord_engine.rs` change is verified COMMENT-ONLY (zero non-comment lines changed) — no behavior change; the new comments are accurate against the live species realize path.
- Move 1 rule ordering confirmed: `pad_figured` precedes `pad_bed_counter`; high-subject images keep `pad_figured`, calm-band images fall through to `pad_bed`; `pad_bed_counter` carries `CounterMelody` and not `HarmonicFill`.
- Move 2 freeze no-op confirmed: per-section override gated on `figuration.is_some()`; identity / `pad_bed` / `pad_bed_counter` keep `figuration_resolved == None`; once-per-plan resolve intact.

---

# Round 2 Re-Review — Second-round changes on top of the first PASS

**Reviewer role:** Quality Gate. VERIFY only — no production code/asset/commit touched.
**Date:** 2026-06-19
**What changed since the first PASS:** three moves grew from the comment-only / mappings-only first round into real behavior. (1) `mappings.json` `pad_bed_counter` gate **RE-TUNED** to `foreground_energy ≥ 0.015 AND fg_bg_contrast ≥ 0.15` (token fe floor; ct is the real discriminator). (2) `chord_engine.rs` gained a **CRASH FIX + tiling-consistency rework** of the CounterMelody replay path (now reachable because real images route to the counter). (3) `composition.rs` `section_figuration_id` replaced the flat `_ => broken_chord_wave` catch-all with a **per-base opposite-density-class partner map**.

## Compilation / Lint / Test

- **`cargo build --release`: PASS.** Clean; only pre-existing warnings in unrelated bins (`modem_encode` unused_assignments; `unpack_tiled_payload` dead field). None in owned files.
- **`cargo clippy --release -- -W clippy::all`: PASS (no correctness lint).** All emitted warnings are pre-existing style/pedantic (`doc list item indentation`, loop-index, `manual div_ceil`, `clamp-like`, `too many arguments 8/7`, `io_other_error`), in `modem.rs`/DFIR bins or as doc-comment style on the new functions. No correctness warning introduced. **Non-blocking.**
- **`cargo test --release`: PASS — all suites green, 0 failed.** Gate-required suites:
  - `variety_s45` **5/5** (calm-keeps-pad_bed, ordinary-routes-counter, identity-figuration-None, pad_figured-ordering, figuration-varies-per-section).
  - `variety_scorecard_s45` **2/2** green (incl. `scorecard_engine_frozen`).
  - `counterpoint_s30` **13/13** green.
  - `engine_equivalence` **9/9** byte-green.
  - lib `226`, main `14`, plus every integration suite (keyplan_*, figuration_s20, texture_s17, motif_*, modem_*, etc.) green.

## Freeze verification

- `git status --short`: only `M assets/mappings.json`, `M src/chord_engine.rs`, `M src/composition.rs` + untracked docs/tests. **`src/engine.rs` is NOT in the diff.**
- `sha256sum src/engine.rs` = `e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261` — **equals the frozen target.**
- `engine_equivalence` **9/9**; `scorecard_engine_frozen` PASS. **Freeze intact.**

## Move 1 — gate re-tune (mappings.json) — VERIFIED

JSON parses (build + runtime test load succeed). New `pad_bed_counter` rule at :344-345 gates `foreground_energy ≥ 0.015 AND fg_bg_contrast ≥ 0.15`. Values sane. **Rule ordering intact:** `pad_figured` (:340) → `pad_bed_counter` (:343) → … → `pad_broken_wave` (:355), with `pad_bed` as the table `default` (:338). Scorecard sweep confirms the intended routing on the 6-image set: AudioHaxImg1/2/3 → `pad_bed_counter`; Lena + magicstudio → `pad_bed`; example.jpg → `pad_broken_wave`. No panic on any of the six.

## Move 2 — CRASH FIX + counter-replay correctness (chord_engine.rs) — RISKIEST, VERIFIED CORRECT

**The crash:** a section TILES when `step_len > steps.len()` (the planner distributes `total_steps` by `rel_len`, unrelated to `plan_phrases` length). The global cursor walks `step_in_section` past `steps.len()`; the live realizer voices the WRAPPED step `steps[step_idx % len]` (engine.rs:723, frozen). The pre-fix replay path raw-indexed `steps[si+1]` / `steps[prev_idx]` and panicked / diverged across tile boundaries.

**Wrapped-index reconstruction — CORRECT.** The fix makes every step-plan fetch on the counter realize path go through the realizer's own `% len` wrap, matching engine.rs:723 byte-for-byte:
- `realized_prev_counter` recurses on the RAW tiled position `si-1` (walking actual time) and fetches `steps[prev_idx % len]` — reconstructing the same realized pitch step `si-1` actually emitted, including across tile boundaries (a phrase-wrapped recursion would wrongly re-open a tiled step as phrase-position-0; the raw-recurse + `%len`-fetch avoids that). Recursion depth = `si` (≤ step_len), RNG-free, deterministic.
- `realized_counter_pitch_with_prev`: prev step `steps[p % l]`, prev-melody read at raw `si-1` (matching the Melody role's own raw motif index, resolved past-end by the walk-based `motif_step_at`, never a raw `motif[si]`), and `next_chord = steps[(si+1) % len]` **only for interior tiled steps** (`si+1 < step_len`), preserving `None` at the GENUINE terminal so the §GAP-2 terminal-diminished bite + its witnesses are untouched.
- the live counter arm (`realize_rhythm`, :1849-1864) uses the same `%len` prev + the borrowed-ctx re-pointed melody read.
- `pivot_counter_pitch` / `pivot_melody_pitch` reconstruct the V7-pivot dom-7th / dom-5th seats so the replay tracks what a modulating step-0 boundary actually sounds; `melody_pitch_for` now mirrors the Melody role's land_home PAC + opening V→I pivot re-voicings (Stages 0/2/3) so `m_now`/`m_prev` equal the melody the ear hears. Both correct against the frozen Melody/pivot formulas.

**No remaining unguarded indexes on the runtime realize path.** Grep for `steps[...]` not wrapped by `% len`/`% l`: the only runtime hits are `steps[0]` (guarded by the `len == 0` early-return immediately above it — new empty-plan floor that returns a neutral counter anchor, never panics) and `steps[prev_idx % len]` (wrapped). Every `(si+1)`, `prev`, `prev_idx` access is `% len`. All other raw `steps[...]` / `melody_pitch_for_step` references are inside `#[cfg(test)] mod tests` (line 5526+) — `melody_pitch_for_step` itself is now `#[cfg(test)]`-gated (test-only). The theme/motif seam never raw-indexes by `step_in_section` (it walks via `motif_step_at` → `PastEnd`). **No panic surface remains.**

**Freeze-path byte-neutrality — CONFIRMED.** Every changed/new function (`realized_prev_counter`, `realized_counter_pitch_with_prev`, `melody_pitch_for`, `pivot_counter_pitch`, `pivot_melody_pitch`, `pivot_role_pitch`, `nearest_consonant_independent_counter`) is reachable ONLY from inside the `OrchestralRole::CounterMelody` arm of `realize_rhythm` (and from each other). That arm fires only when a CounterMelody instrument is routed — never on identity/home_only/non-counter profiles. `engine_equivalence` 9/9 and `scorecard_engine_frozen` independently confirm zero byte change on the frozen path.

**Species invariants — PRESERVED.** `counterpoint_s30` 13/13 + the keyplan pivot suites green. The new M1.4 strict-parallel re-point is tightly scoped (only the plain `Sustain` figure, only `Interior`/`HalfCadence` positions, only a CONSONANT contrary/oblique chord tone, `unwrap_or(cnt)` when none exists), so the dissonant Passing/Neighbor/Suspension figures, the PhraseStart opening, the PAC clausula, and the terminal-diminished bite all keep their own formulas and witnesses. `nearest_consonant_independent_counter` is built only from existing pure helpers (`counter_candidate_pitches`, `motion_dir`, `is_consonant`, `has_parallel_perfects`).

**Scorecard renders AudioHaxImg1/2/3 without panic.** L1 rows captured:
- example.jpg → `pad_broken_wave`, L1 CounterMelody N/A (not routed), M5.1 = 7 PASS.
- Lena.png → `pad_bed`, L1 N/A, M5.1 = 6 PASS.
- **AudioHaxImg1.jpg → `pad_bed_counter`, L1 CounterMelody PARTIAL, M5.1 = 11 PASS.**
- **AudioHaxImg2.jpg → `pad_bed_counter`, L1 CounterMelody PARTIAL, M5.1 = 10 PASS.**
- **AudioHaxImg3.jpg → `pad_bed_counter`, L1 CounterMelody PARTIAL, M5.1 = 7 PASS.**
- magicstudio-art.jpg → `pad_bed`, L1 N/A, M5.1 = 6 PASS.

The three structured images now render the counter (was the panic surface); L1 reads PARTIAL (was N/A). No panic.

## Move 3 — figuration density-partner map (composition.rs) — VERIFIED

`section_figuration_id(base_id, role)` audited against the `figuration_catalogue`:

| base (onsets) | partner (onsets) | class crossing | partner in catalogue |
|---|---|---|---|
| alberti (4) | block_comp_24 (2) | DENSE→SPARSE | yes |
| broken_chord_up (4) | block (0) | DENSE→SPARSE | yes |
| broken_chord_wave (4) | block (0) | DENSE→SPARSE | yes |
| stride (4) | block_comp_24 (2) | DENSE→SPARSE | yes |
| arp_waltz (3) | broken_chord_wave (4) | MEDIUM→DENSE | yes |
| oom_pah_pah (3) | broken_chord_up (4) | MEDIUM→DENSE | yes |
| block (0) | broken_chord_wave (4) | SPARSE→DENSE | yes |
| block_comp_24 (2) | broken_chord_up (4) | SPARSE→DENSE | yes |
| oom_pah (2) | alberti (4) | SPARSE→DENSE | yes |
| `_` fallback | broken_chord_wave (4) | (safe DENSE) | yes |

- **No dangling ids** — every partner is an existing catalogue cell.
- **Density-class contrast holds** — every pair crosses to a genuinely different onset count (the flat-catch-all defect, where an already-broken base swapped to another broken cell with no felt density change, is gone).
- **`ThematicRole` is exhaustively matched** (5 variants: anchor Statement/Return/Coda hold base; departure Contrast/Development take the partner) — no role wildcard.
- **Gated on `figuration.is_some()`** — `test_s45_identity_figuration_stays_none` asserts every section (departure roles included) keeps `figuration_resolved == None` on non-figured / `pad_bed` / `pad_bed_counter` / identity profiles → byte-stable.
- **Recap returns to base** — `Return`/`Coda` map to `base_id`; `test_s45_figuration_varies_per_section` confirms anchors keep base while departures diverge.

## VERDICT — **PASS**

All three second-round moves are correct and freeze-safe.
- **Blocking issues:** NONE. Build/clippy/test all green; engine.rs frozen (sha + 9/9); the crash fix's wrapped-index reconstruction is the correct match to the frozen engine.rs:723 wrapping with no remaining unguarded index on any runtime path; species invariants preserved; the figuration partner map is dangling-free, density-contrasting, role-exhaustive, and identity-byte-stable.
- **Non-blocking notes:** (a) the new functions carry doc-comment style clippy nits (`doc list item indentation`/overindent) — cosmetic, pre-existing repo style. (b) L1 CounterMelody reads PARTIAL (not full) on the three routed images and M7.1 orchestration stays flat (=1 id) — these are scorecard *quality* observations, not correctness defects, and are taste/affect calls left for the operator's ear, not a QG block. (c) the `pad_bed_counter` routing of AudioHaxImg3 ahead of `pad_broken_wave` is an intentional, operator-tunable musical call documented inline.

# Review S29 — K3 RE-TUNE BUILD (Quality Gate)

**Slice:** S29 — the K3 re-tune that makes the modulation PERCEPTIBLE in three levers, all designed
to keep `src/engine.rs` BYTE-FROZEN. Lever 1: confirm the destination key (new
`ChordEngine::tonic_triad` forces a modulating section's `chords[0]` to the destination root-position
I in `composition.rs`; `chord_engine.rs` re-voices step 1 as a V→I authentic cadence via
`pivot_resolution_is_armed` + `pivot_resolution_pitch`, Option-A voice-leading, NO `plan_phrases`
change). Lever 2 (MX-4): `Section.density` (previously dead/write-only) is SET from region energy in
`composition.rs` (`resolve_key_scheme -> Vec<(i8,f32)>`; `f(e)=clamp(0.5+0.30*(e-0.5),0.35,0.65)`)
and READ in `chord_engine.rs` `realize_rhythm` as an `edge_activity` nudge. Lever 3: the pivot gains
a dominant 7th `(dom_root+10)%12` in its inner/fill voice.

**HEAD:** `dfcfb4c` (S28/K3 BUILT & CLOSED). **Built in the working tree, UNCOMMITTED.**
**Surface verified:** `src/chord_engine.rs`, `src/composition.rs` (modified); `tests/keyplan_k3.rs`
(modified); `tests/keyplan_s29.rs`, `docs/spec-s29-k3-retune-build.md`,
`docs/input-s29-k3-retune-harmony.md` (new). `assets/mappings.json` UNTOUCHED. `src/engine.rs`
UNTOUCHED.

## OVERALL VERDICT: PASS

All eight checks pass on independently re-derived evidence. `engine.rs` is byte-frozen at the exact
anchor with a zero diff vs HEAD — no re-baseline this slice, exactly as the design promised by riding
the existing dead `Section.density` field instead of adding a ctx field. `engine_equivalence` is 9/9
with goldens 240/114/84/36/79 unmoved. The three levers are real, not gamed: every new test pins
actual pitch classes / densities / onset counts (re-derived by hand), and the dead-field trap is
escaped — `density_varies_between_home_and_excursion` proves the density READ changes realized output
(home 2 onsets vs excursion 3). The flagged `no_inversion_under_pivot_path` sweep change is a genuine
STRENGTHENING: the old sweep fired the pivot only for the bass and let fill/melody fall through to
free-select; the new sweep realizes all three roles at step 0 and adds an explicit
`fill == dom_seventh_pc` assertion, so it now actually guards the V7 voicing, and `bass<fill<melody`
holds across all `{+7,+5,+3,−3}×{12,50,100}` combos with the combo count asserted so the sweep cannot
silently shrink. Module boundaries are clean (chord_engine names no image/pixel type; density is set
in exactly one runtime place and read in exactly one place). Operator lock holds (`mappings.json`
untouched). Codename scrub clean. Full net all-green (29 binaries, 0 failures);
`--lib --no-default-features` 128/0. No blockers. Two non-blocking nits, both carry-forward.

---

## CHECK 1 — BYTE-FREEZE (central): PASS

- **`sha256sum src/engine.rs` = `e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261`**
  — matches the anchor EXACTLY. No re-baseline this slice (unlike S28).
- **`git diff HEAD -- src/engine.rs` is EMPTY** — engine.rs has ZERO changes worktree vs HEAD. The
  decisive design choice held: S29 rides the EXISTING `Section.density` field (dead until now) and the
  existing `ctx`, so no new field threads through the engine ctx-build and engine.rs is not edited at
  all.
- **`realize_step` PUBLIC 7-param signature byte-identical worktree vs HEAD** — diffed
  `git show HEAD:src/chord_engine.rs` against the worktree at the fn: same
  `(step, inst_idx, num_instruments, features, ms_per_step, ctx)` signature. The new data rides `ctx`
  (the density read) and the role/ctx params already present (the pivot/resolution).
- **`sha256sum src/chord_engine.rs` CHANGED** (expected): `ef635be…`→`df772d85…`. The change is
  behaviorally gated (CHECK 2).
- **`git diff --stat HEAD`** = `src/chord_engine.rs`, `src/composition.rs`, `tests/keyplan_k3.rs`
  only; untracked new: `tests/keyplan_s29.rs`, `docs/spec-s29-*`, `docs/input-s29-*`, plus the stray
  `assets/images/magicstudio-art.jpg` (carry-forward nit N-S29-2).

## CHECK 2 — IDENTITY INSERTS NOTHING: PASS

- **`engine_equivalence` → 9/9 byte-green.** Read the test source: goldens **240** (cadence hold),
  **114/84** (bass vel sat100/sat0), **36/79** (`G_BASS_NOTE`/`G_MELODY_NOTE`) are still those exact
  values — unchanged (the test file is not even in the S29 diff). Not silently re-baselined.
- **`keyplan_k3::pivot_inserts_nothing_on_identity` green and REAL** — read in full: builds concrete
  Section/StepContext fixtures and `assert_eq!`s the full realized stream against `realize_baseline`
  (`single_section_default`) across the role sweep, covering home/identity, same-key boundary, and
  `pivot:false` modulating boundary. Not `assert!(true)`.
- **`keyplan_s29::density_nudge_zero_on_identity` green and REAL** — read in full: for 5 edge_density
  values × 4 role cases it `assert_eq!`s `realize_with_prev(density=0.5)` BYTE-IDENTICAL to the
  `single_section_default` baseline, AND asserts the identity fill voice sounds only home C-major
  tones (pc 0/4/7), proving the dom7 add is never reached on `pivot:false`. Non-vacuous.
- **`density==0.5 ⇒ nudge==0.0` re-derived from the code** (`chord_engine.rs:1485`):
  `density_nudge = (ctx.section.density - 0.5) * DENSITY_ACTIVITY_GAIN`; at `density==0.5` this is
  `0.0 * GAIN == 0.0` exactly, so `edge_activity = (base + 0.0).clamp(…)` is byte-identical to pre-S29.

## CHECK 3 — LEVERS ARE REAL (not gamed): PASS

All re-derived by hand for the test fixture (home C pc 0; prev 0 → dest +7):

- **dom7 pc**: `dom_root_pc=(7+7)%12=2` (D); `dom_seventh_pc=(2+10)%12=0` (C) = `(dest_root_pc+5)%12=
  (7+5)%12=0`. ✓ `pivot_voicing_carries_dom7` pins `fill % 12 == 0`, with sanity asserts that the
  dom7 ≠ bass (dom root 2) and ≠ melody (dom fifth 9) — i.e. a real ADDED tone, not a re-labelled
  triad member. PASS.
- **forced tonic root-position I, name=="I"**: `opening_pac_confirms_destination_key` calls the real
  `tonic_triad(67,"Ionian")` and asserts `name=="I"` and `notes[0]%12 == dest_root_pc (7)`. Re-read
  `tonic_triad` body: reuses `roman_to_chord_complex("I", root_midi, &scale, Triad)`, no RNG, no
  secondary-dominant/borrow — the chord tones are identical to a free-selected I; only the SELECTION
  is forced. PASS.
- **V→I step-1 bass == dest_root_pc**: same test asserts the step-1 (resolution) bass `% 12 ==
  dest_root_pc (7)`. Re-read `pivot_resolution_pitch`: Bass→`seat_pc_in_register(dest_root_pc,
  BASS_REGISTER_FLOOR)`. The test ALSO asserts a same-key "boundary" (prev==dest) falls through to the
  baseline (cadence is a modulation marker, not a default). PASS.
- **f(e) endpoints**: `f(0)=clamp(0.5+0.30*(−0.5),…)=clamp(0.35)=0.35`; `f(0.5)=0.5`;
  `f(1.0)=clamp(0.5+0.30*0.5)=0.65`. ✓ Matches the FLOOR/NEUTRAL/CEIL consts in `composition.rs`.
- **density is AUDIBLE, not just SET (the dead-field trap)**:
  `density_varies_between_home_and_excursion` green — re-derived its band math against the real
  realizer: `EDGE_ACTIVITY_RANGE_MAX=0.05`, edge_density 0.04 → base 0.80; home (density 0.5, nudge 0)
  → activity 0.80 (NOT > the 0.80 arpeggio cutoff → syncopated → 2 onsets); excursion (density 0.65,
  nudge `+0.15*0.5=+0.075`) → activity 0.875 (> 0.80 → arpeggio → 3 onsets). The test pins the EXACT
  counts (2 vs 3), so a future re-band cannot make it pass on a fluke. The 0.80 cutoff is the real
  band edge (`chord_engine.rs:977-978`). This is the decisive proof Lever 2 escapes the dead-field
  trap the spec warned about.

## CHECK 4 — NO REGISTER INVERSION (the flagged sweep change scrutinized): PASS

- **`no_inversion_invariant` green** in BOTH `keyplan_s25.rs` and `prominence_s23.rs`.
- **`no_inversion_under_pivot_path` (extended, keyplan_k3) green** WITH the dom7.
- **The flagged change STRENGTHENS, does not hide.** I diffed both versions of the sweep. The OLD K3
  sweep passed `step_in_section == inst_idx` (bass=0, fill=1, melody=2). Re-read the pivot gate
  (`chord_engine.rs:2265`: `step_in_section != 0 → None`): the pivot fires ONLY at step 0, so the OLD
  sweep fired the pivot for the BASS ONLY — fill (step 1) and melody (step 2) fell through to
  free-select, and the `bass<fill<melody` frame held trivially against non-pivot voicing. The dom7
  was NEVER exercised by the old sweep. (Worse, under S29 the old fill at step 1 of a modulating
  section would hit the resolution re-voicing, not the pivot.) The NEW sweep realizes ALL THREE roles
  at `step_in_section == 0` so the pivot fires for each, AND adds an explicit
  `fill.iter().all(|e| e.note % 12 == dom_seventh_pc)` assertion — it now guards the actual V7
  voicing. This is a strict strengthening.
- **Frame re-derived by hand for the dom7 pivot** (dest +7): bass pc 2 @ floor 36 → 38 (D2); fill pc
  0 @ floor 55 → 60 (C4); melody pc 9 @ floor 67 → 69 (A4). 38 < 60 < 69 ✓. The brightness lifts
  (`bright_octaves * 6.0` for fill vs `* 12.0` for melody) preserve the ordering, and the test sweeps
  `{+7,+5,+3,−3} × {12,50,100}` = 12 pivot combos, all passing. The combo count is asserted (`combos`
  at `keyplan_k3.rs:579`), so the sweep cannot silently shrink. The dom7 is seated via the SAME
  `seat_pc_in_register(pc, FILL_REGISTER_FLOOR)` between bass and melody floors → no inversion by
  construction.

## CHECK 5 — MODULE BOUNDARIES: PASS

- **`chord_engine.rs` added lines name NO image/pixel type** — grepped the `+` lines for
  `ImageUnderstanding|pixel|RegionAffect|image_analysis|hue|GlobalFeatures|avg_saturation|
  colorfulness|foreground|background` → NONE. The new code reads `ctx`/`features` only
  (`ctx.section.density`, `ctx.section.key_offset_semitones`, `ctx.prev_key_offset_semitones`,
  `ctx.key_tempo.*`, `ctx.step_in_section`, `features.brightness`).
- **`composition.rs` added lines name no pixel type beyond `ImageUnderstanding`/`RegionAffect`** — the
  only `match`/`Mat` substring hit was the keyword `match`, a false positive. The energy comes from
  the already-existing `RegionAffect.energy` ranking; no new pixel surface.
- **Density single-writer / single-reader confirmed by grep:**
  - SET: exactly one RUNTIME write — `composition.rs:1266` (`density: section_density`). The other
    `Section.density` mentions are the field decl (`:888`) and two fixtures that stay `0.5`
    (`legacy_default_section` `:1908`; the engine legacy fixture). `OrchestrationProfile.density`
    (`:390`, fixture `:428`) is a SEPARATE dead field, untouched.
  - READ: exactly one — `chord_engine.rs:1485` (the `realize_rhythm` nudge). No other reader/writer.
  - The S23 prominence path writes disjoint fields (orchestration / the prominence Vec), so there is
    provably no double-write.

## CHECK 6 — OPERATOR LOCK: PASS

- **`git diff --stat HEAD -- assets/mappings.json` is EMPTY** — `mappings.json` UNCHANGED. The density
  map is code consts; the pivot/cadence are code. No `feature_normalization`/coupling row added (the
  optional texture fast-follow was correctly NOT built).
- **`keyplan_k2b::no_routed_image_ends_off_home` green** (1/1; binary 14/14). The Open
  `theme_and_variations_excursion` stays `pivot:false` and unrouted — operator lock holds.

## CHECK 7 — SCOPE / HELD LEVERS: PASS

- **NO dwell change** — `BASE_STEPS_PER_SECTION` (8) does NOT appear in the +/- diff lines; step-count
  untouched.
- **NO `plan_phrases` change** — `fn plan_phrases` is NOT in the `chord_engine.rs` diff. Lever 1 is
  Option A (voice-leading only via `pivot_resolution_pitch`); Option B (the explicit opening-PAC
  stamp) was correctly HELD.
- **NO mode-change-at-B, NO wider key menu** — the menu stays `{+7,+5,+3,−3}`; `home_mode` is one mode
  across the piece; nothing in the diff widens either.
- **NO texture/figuration row** — `mappings.json` untouched; the optional second-dimension texture
  fast-follow was deferred, as the spec recommended (density-only for cleaner re-listen attribution).
- The slice is EXACTLY the three approved levers (tonic_triad + V→I re-voicing; density set+read;
  dom7 in the pivot fill).

## CHECK 8 — FULL NET: PASS

- **`cargo test` (full default net) → ALL-GREEN, 0 failures.** 29 test binaries. Notable counts: lib
  163/0, main 5/0, engine_equivalence 9/9, keyplan_s29 4/0, keyplan_k3 4/0, keyplan_s25 11/0,
  keyplan_k2b 14/0, keyplan_k2a 9/0, prominence_s23 5/0, affect_s22 8/0, texture_s17 10/0,
  figuration_s20 9/0, saliency_s18 12/0, diversity_s13 8/0, modem_roundtrip 17/0, modem_realair 10/0,
  qg_probe_band_isolation 1/0, all others green.
- **`cargo test --lib --no-default-features` → 128 passed; 0 failed.**

---

## BLOCKING ISSUES

None.

## NON-BLOCKING NITS

1. **N-S29-1 — `_common_tone_pc` and `prev_dom_pc` are now computed-but-effectively-unused in
   `pivot_chord_events`.** Since the inner voice now sounds the dom7 instead of the hinge, the
   common-tone picker's result is bound with a leading underscore and kept only as the documented
   proof a shared tone exists. This is INTENTIONAL and documented in-comment (the hinge is preserved
   as the 7th's resolution-target line), and it is harmless dead computation (no behavior, no
   warning beyond what the underscore suppresses). Optionally collapse the picker to a comment-only
   note in a later slice; leaving it is fine and arguably clearer as a witness. Style-only.
2. **N-S29-2 — stray untracked image artifact** `assets/images/magicstudio-art.jpg` (carry-forward
   from S28's N-K3-3) — untracked, referenced by no S29 source/test/doc, an operator ear-test input.
   No effect on correctness or the byte-freeze. Housekeeping only (add or remove before commit).

> Carry-forward from S28 still standing and non-chargeable: pre-existing `cargo fmt --check` diffs in
> LOCKOFF files (none are S29-touched) and the N-K3-1 clippy cast nit in `land_home_pitch` (untouched
> by S29).

---

*Quality Gate, S29 / K3 RE-TUNE. Independently re-derived: engine.rs sha computed and diffed to
HEAD (empty), engine_equivalence + every new witness test run, the dom7 / forced-tonic / V→I /
density-band harmony re-derived by hand, the flagged no-inversion sweep change git-diffed both ways
and confirmed to STRENGTHEN, the density single-writer/single-reader grepped, module boundaries and
codenames grepped, the full net and the no-default-features lib run. Working tree left exactly as
found — no edit to any src/test/asset, no stage, no commit; only this review doc written. Build-role
titles (Architect, Implementer, Music Theory Specialist, Test Engineer, Quality Gate) are the
S21/S24/S26/S28 domain titles.*

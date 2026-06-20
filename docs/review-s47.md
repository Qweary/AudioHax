# Quality Gate Review — S47 Slice 1: The Figure-Ground Hierarchy

**Reviewer role:** Quality Gate (validation only — no `src/*.rs` / `assets/*` modified).
**Date:** 2026-06-19.
**Scope under review:** S47 Slice 1 (figure-ground activity hierarchy) as built in
`src/chord_engine.rs` + `assets/mappings.json`, with the F1–F5 scorecard + 5 re-derived
witnesses in `tests/`. The working tree also lands **S47 Slice 4** (the Pad bed activity
recession, part (e) of the engagement brief) coupled into the same commit — reviewed here as
the load-bearing F5b/F1 lever.

**Contract docs read to ground this review:** `docs/spec-s47-slice1-build.md` (build contract),
`docs/design-s46-figure-ground.md` (work order), `docs/spec-s46-figure-ground-metrics.md` (F1–F5
semantics).

---

## OVERALL VERDICT: **PASS**

No blocking issues. The build achieves every measured result it claims, the byte-freeze holds
(independently confirmed three ways), module boundaries are clean, all five musical-logic parts
are sound (none gamed), and all five re-derived witnesses are genuinely re-derived from correct
behavior (none gutted-to-green). Non-blocking observations are documentation/coupling notes only.

---

## Compilation

- `cargo build` (default features): **PASS** (`Finished dev profile`). The only build warnings
  are pre-existing `unused_variables`/`unused_assignments` in `src/bin/modem_encode.rs` +
  `unpack_tiled_payload` — unrelated to S47.
- `cargo build`/test for the pure-lib subset: builds clean.

## Lint (net-new vs baseline)

- `cargo fmt -- --check`: **PASS** (exit 0, no diff).
- `cargo clippy -- -W clippy::all`: **74 total warnings**, against the ~67 pre-existing
  modem/lib baseline. Every warning located inside `chord_engine.rs` is pre-existing in
  CATEGORY and predates S47: `doc list item without indentation` (:893–895, :3609–3617),
  `casting to the same type is unnecessary` (:3543, a `u8`→`u8` cast in legacy code),
  `useless use of vec!` (:123–124). **Zero NET-NEW correctness warnings attributable to the
  S47 additions.** The S47 code itself draws no new clippy. Correctness warnings: none →
  non-blocking. Style: within baseline.

## Test Results (counts)

Full `cargo test` (default features) — **all green, 0 failed**:

| Binary | Passed |
|---|---|
| unittests `src/lib.rs` | 243 |
| unittests `src/main.rs` | 14 |
| `engine_equivalence` | **9** |
| `counterpoint_s30` | 13 |
| `prominence_s43` | 6 |
| `saliency_s18` | 12 |
| `variety_scorecard_s45` | 3 |
| (all other integration binaries) | green (incl. modem_realair 10, modem_roundtrip 17) |

`cargo test --lib --no-default-features` (pure-lib subset): **200 passed, 0 failed.**

The new S47 helper unit tests live in `chord_engine.rs mod tests` (`s47_*`, :9359–9660+) and
are counted in the 243 lib total.

## Freeze Audit (LOAD-BEARING — independently confirmed)

- **sha256 `src/engine.rs`** = `e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261`
  — **MATCHES the frozen value** (verified at review start AND after the full test run).
- **`cargo test engine_equivalence` = 9/9 PASS**, including
  `test_full_golden_sweep_is_byte_identical`. **No golden moved.**
- **Independent confirmation the seat guard is genuinely gated (not just claimed):** read the
  call site at `chord_engine.rs:1543–1561`. The guard computes
  `seat_floor = if counter_present { COUNTER_CEILING + MIN_FIGURE_GAP } else { i16::MIN }` —
  it is **`i16::MIN` (a true no-op) when no counter is present**, folded under the existing
  single `.clamp(24,96)`. `counter_present` is derived (`:1417–1421`) by scanning `assign_role`
  over the ensemble; on the identity/golden path the profile is identity → `assign_role`
  delegates to `instrument_role`, which never yields `CounterMelody` → `counter_present == false`
  → guard inert. The dedicated unit test `s47_seat_guard_lifts_dark_melody_only_when_counter_present`
  proves the no-counter path seats below the ceiling (no-op) and the counter path lifts.
- **Pad recession + melody floor no-op at neutral 0.5 (the identity path):** confirmed by code
  + unit tests. `pad_onset_cap` returns `None` (no cap) for `pad_w >= PROMINENCE_NEUTRAL`
  (`s47_pad_onset_cap_noop_at_neutral_weight`); the melody floor's `floor_to_dotted` predicate
  is `melody_w > ACTIVITY_FLOOR_THRESHOLD(0.50)` so strict `>` is false at 0.5
  (`s47_activity_floor_lifts_foreground_sustained_to_dotted` asserts the 0.5 no-op). The
  `prom_shift`/`prom_lift` nudges are all `(0.5 − 0.5)*SPAN == 0` at neutral. **All three S47
  levers are byte-neutral on the freeze path.**

**Freeze verdict: HOLDS, independently confirmed.**

## Module Boundary Audit

- `chord_engine.rs` S47 additions contain **no image processing, no MIDI output, no raw image
  data** — grep for `image::`/`ImageBuffer`/`midi_output`/`MidiMessage`/`DynamicImage`/`opencv`/
  file I/O over the diff returns **nothing**. The new code receives resolved features
  (`PerfFeatures`, `edge_activity`) + resolved prominence weights (`prominence_weight(ctx, role)`)
  — exactly the realizer's existing inputs.
- `mappings.json` carries **no hardcoded Rust musical logic** — the new tiers (`melody_lead_strong`,
  `melody_lead_gentle`) are pure data rows; routing uses the EXISTING `Knob::FgBgContrast`/
  `Knob::SubjectSize` + closed `CmpOp` set (`ge`/`lt`/`in_range`). No Rust schema change.
- `engine.rs` **untouched** (sha confirmed).
- **Ownership:** `git status` shows only `chord_engine.rs` + `mappings.json` (producer) and
  `tests/{counterpoint_s30,prominence_s43,saliency_s18,variety_scorecard_s45}.rs` (Test Engineer).
  No file modified by an agent that doesn't own it.

## Musical Logic Review (per-part)

- **GOVERNOR — SOUND, not a rename.** Read at `:2206–2260`. It computes the MELODY's
  `ActivityClass` via `melody_activity_class(...)` (off the MELODY role's prom_shift — correct
  per spec §8.4) and routes the counter by a `match` on that class. The old
  `held_chord || melody_static` predicate is explicitly retired (`let _ = (held_chord,
  melody_static);`) — it is genuinely class-driven, not a renamed predicate. **Subdividing →
  MOVING (S45 preserved); Oblique/Sustained → oblique-or-rest, NEVER the guaranteed onset.**
  The counter is never silenced (rest-as-gesture stays gated on the existing `FILL_REST_ACTIVITY
  && weak_interior` — no-hollow). Unit-proven by `s47_governor_counter_moves_iff_melody_subdividing`
  + the re-derived `test_counter_moves_when_melody_subdividing` (PRESERVE-S45) and
  `test_counter_recedes_under_holding_melody` (the fix).
- **MELODY FLOOR — SOUND, genuine no-op at 0.5.** At `:2294–2330` `floor_to_dotted` lifts a
  foreground (`> 0.50`) melody that would otherwise SUSTAIN up to the DOTTED (Oblique-rank, ≥2
  onsets) band — never the reverse (it can only ADD activity:
  `s47_activity_floor_noop_when_melody_already_moving`). No-op at neutral 0.5 (strict `>`).
- **SEAT GUARD — SOUND + musically correct re-seat.** `:1543–1561`. Enforces
  `melody_seat ≥ COUNTER_CEILING + MIN_FIGURE_GAP(2)` via `.max(...)` under the existing clamp,
  gated on counter-present. The re-seat uses `seat_pc_in_register(pc, floor)` (`:1604`), which
  **preserves the pitch CLASS and lifts it to the lowest octave at/above the floor** — NOT an
  arbitrary +N. (Witness: 67→79 = G4→G5, same chord-appropriate pitch class.) Bright-melody
  no-op proven by `s47_seat_guard_noop_on_bright_melody`.
- **PAD RECESSION — SOUND, no-hollow independently re-derived.** `:2113–2138` +
  `pad_onset_cap`/`recede_pad_onsets`/`melody_min_onsets` (:1180–1270). Image-conditioned by the
  resolved Pad weight: deep tier (≤0.40) caps ONE below the melody's per-class minimum (strict
  positive F1 lead), shallow tier caps AT it (near-even). Weak-beat displacement of a surviving
  block-bed stab off offset 0 (F5a anti-fusion, count-preserving). **No-hollow independently
  re-derived:** every (tier × class) combo floors at `PAD_ONSET_FLOOR = 1` via
  `.max(PAD_ONSET_FLOOR)` + `saturating_sub` — e.g. deep × Sustained = `max(1-1, 1) = 1`, never
  0. Confirmed by `s47_pad_onset_cap_never_hollows`.
- **PROMINENCE FAMILY — SANE + byte-stable mid.** `mappings.json` tiers: deep
  (0.90/0.45/0.30/0.30/0.50), mid `melody_forward` **UNCHANGED (0.78/0.58/0.40/0.40/0.50 —
  DECISION-2 compliant)**, shallow (0.72/0.65/0.45/0.45/0.50). All bed roles recede but stay
  **> 0.25** (S43 level floor — lowest is 0.30). SelectTable order DEEP → subject_melody
  (preserved verbatim) → SHALLOW → default. All weights in range.

**Musical-logic verdict: governor / floor / seat-guard / pad-recession / prominence ALL SOUND.
None gamed.**

## Test Quality / Re-Derivation Audit (per-witness)

All five witnesses are **genuinely re-derived from correct behavior** — each preserves the
property it actually validates; only incidental pins moved.

1. **`counterpoint_s30::test_no_audible_parallel_perfect_counter_vs_melody`** — the no-parallel-
   perfect property is **PRESERVED and still genuinely detected**: `!forms_parallel_perfect(...)`
   still asserts, and ic classes genuinely DIFFER (P8 ic 0 → P5 ic 7). Only the incidental
   motion-direction pin moved Contrary→Similar (a correct consequence of the seat guard lifting
   the melody 67→79). **Not gutted.**
2. **`counterpoint_s30::test_terminal_diminished_keeps_prepared_bite`** — the consonant-vs-bite
   property holds for all four consonant openers; I/ii shifted 62→65 because the counter now
   tracks the seat-guarded melody (still consonant ic 0, no bite). Property preserved.
3. **`counterpoint_s30::test_cadence_resolves_perfect_no_leap`** — still asserts `is_perfect` +
   `ic == 7` (P5/P12) + no-leap; melody pin 67→79 (seat guard), counter unchanged 55/60.
   Property preserved.
4. **`prominence_s43::guard2_per_image_resolution_divergence`** — still asserts the two images
   DIVERGE (example 1.0 vs Lena 0.72) **with headroom**; Lena re-pinned 0.78→0.72 (the new
   SHALLOW tier — the intended image-conditioning, not a green-fudge). Property preserved.
5. **`saliency_s18::test_complementary_rhythm_onset_contrast`** — still asserts a REAL onset
   contrast (`assert_ne!` on offsets) via the NEW discriminator (melody activity: `perf(0.30)`
   subdividing → step_ms/4 vs `perf(0.005)` holding → offset 0). The contrast is genuinely
   exercised on two distinct governor branches. **Not re-baselined-to-green.**

**F5b** asserted at **0** (`s46_recession_bound` returns 0 for all six; the assertion is a
genuine regression gate — `<= bound`). **F1** promoted: hard floor `>= 0` asserted on all six +
subject-margin `>= f(fg_bg_contrast)` asserted on both SUBJECT images (with documented headroom).
**F4** correctly **LEFT REPORTED, not asserted** (slice-3 inverse-comp instrument — `f4_tag`
computes but no `assert` fires; confirmed by grep). No witness looks gamed.

## Integration

- Types line up at boundaries: the two new private params (`counter_present: bool` on
  `role_pitch`, alongside the existing `prominence_w`) follow the established `pad_voices`
  precedent — `realize_step`'s PUBLIC signature is unchanged. `melody_pitch_for` passes
  `counter_present = true` (correct: it is only reached from the CounterMelody arm, so a counter
  is present by construction — this keeps the counter tracking the seat-guarded melody, avoiding
  a phantom-melody divergence).
- No stray TODO / incomplete integration in the S47 additions.
- The scorecard reads the build through the EXISTING render streams (`render`/`per_step_pitch`/
  `step_shape_key`) — **no new render path, no new image type, no audio/OpenCV**. F1–F5 are pure
  functions of streams already in hand.

## MEASURED RESULT — independently verified (the build achieves what it claims)

Ran `variety_scorecard_s45 --nocapture`; the printed figure-ground rows confirm the claim:

| Image | strength | F1 margin | F5b viol | rollup |
|---|---|---|---|---|
| example.jpg | MID | +1.154 | 0 | VARIED |
| Lena.png | FIELD | +0.500 | 0 | VARIED |
| AudioHaxImg1 | SUBJECT | +1.000 (thr +0.30) | 0 | VARIED |
| AudioHaxImg2 | SUBJECT | +1.194 (thr +0.30) | 0 | PARTIAL (F4 only) |
| AudioHaxImg3 | (subject/mid) | +1.250 | 0 | (per print) |
| magicstudio-art | (field) | +0.832 | 0 | (per print) |

**F5b = 0 on ALL 6** (down from baseline 21/18/4/0/18/12). **F1 margin POSITIVE on all 6**
(was negative on 4/6). **Rollup: 4 VARIED / 2 PARTIAL, the 2 PARTIAL blocked ONLY by F4** (the
deferred slice-3 inverse-compensation metric, `F4 FAIL` in the rollup line). Matches the claim
exactly.

## Blocking Issues

**None.**

## Non-Blocking Issues / Observations

1. **Scope is larger than the slice-1 spec named.** The working tree lands Slice 4 (Pad bed
   activity recession) coupled with Slice 1. This is the correct and load-bearing lever (the F5b
   defect proved Pad-driven, not counter-driven — the producer's in-code rationale at :2056–2092
   documents this), and it is fully freeze-neutral + unit-covered. Flagged only so the commit
   message / handoff records that S47 shipped slices 1 + 4 together, not slice 1 alone.
2. **Cutoff-duplication coupling (spec §8.2) handled well.** The producer extracted
   `MELODY_ARP/SYNC/DOTTED_CUTOFF` consts so `melody_activity_class` and the Melody arm read ONE
   source — the recommended freeze-neutral refactor. The 1:1 coupling is unit-asserted
   (`s47_melody_activity_class_band_mapping`). No drift risk.
3. **Pre-existing clippy baseline (~67) untouched.** Cleaning the legacy `doc list item` /
   `useless vec!` warnings in `chord_engine.rs` is out of S47 scope and correctly left alone.

---

*Review complete. `src/engine.rs` sha256 re-verified UNCHANGED:
`e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261`. Verdict: PASS.*

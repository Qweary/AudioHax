# Quality-Gate Review — S48 Slice 3: Inverse-Register Compensation + the Level Finish

**Reviewer role:** Quality Gate (correctness only — the F4 perceptual/design decision is the taste gate's + operator's call, NOT this review's).
**Date:** 2026-06-20.
**Under review:** S48 slice-3 build against `docs/spec-s48-slice3-build.md` (LOCKED contract).
**Built by:** Music Theory Specialist (`src/chord_engine.rs` + `assets/mappings.json`) and the Test Engineer (`tests/variety_scorecard_s45.rs` + `tests/prominence_s43.rs`).
**Tree:** working tree at HEAD `fda3c06`, 4 files modified, engine.rs untouched.

---

## Overall Verdict: **PASS WITH ISSUES**

The build is **correctness-clean and freeze-safe**. engine.rs is byte-identical to the frozen digest, all new code paths are genuine no-ops on the identity/no-counter/neutral-prominence golden path, module boundaries are respected, the musical logic is count-preserving and correctly gated, and the test assertions are real. The F4 held promotion is an **honest hold, NOT a masked bug**. The single non-blocking issue is one new style-lint (`clippy::unnecessary_map_or`) introduced in the new test code — no correctness impact.

---

## 1. Compilation / Lint / Format Status

| Check | Result |
|---|---|
| `cargo build --release` | **PASS** — clean build; only pre-existing `unused_variables`/`unused_assignments` warnings in unrelated `modem_encode.rs`/`unpack_tiled_payload.rs` binaries (not in any reviewed file). |
| `cargo fmt -- --check` | **PASS** — no diff. |
| `cargo clippy -- -W clippy::all` | **PASS (no correctness lints)** — see below. |

**Clippy detail (new-vs-pre-existing, established by diff-hunk overlap):**
- `src/chord_engine.rs`: 12 warnings total, ALL at lines (123-124, 893-895, 3717, 3783-3791) OUTSIDE every S48 diff hunk — pre-existing `useless_vec`/`doc_lazy_continuation`/`unnecessary_cast` in unchanged regions. **The S48 production changes introduce ZERO new clippy warnings.**
- `tests/variety_scorecard_s45.rs:1339` — `clippy::unnecessary_map_or` (`map_or(false, …)` → `is_some_and`). This IS inside the new S48 melody-level-floor code (hunk 1299-1363). **One NEW style lint.** Non-blocking (style, not correctness).
- `tests/variety_scorecard_s45.rs:1846` (empty-format-string) and `tests/prominence_s43.rs:391-392` (doc-list-item) are OUTSIDE the S48 hunks — pre-existing.

No compilation failures, no correctness-clippy warnings → nothing BLOCKING here.

---

## 2. Test Results (counts)

`cargo test` full suite: **541 passed / 0 failed / 0 ignored** across all binaries and integration tests.

Files under review (explicit run):
- `tests/variety_scorecard_s45.rs`: **3 passed / 0 failed**
- `tests/prominence_s43.rs`: **6 passed / 0 failed**
- `tests/engine_equivalence.rs`: **9 passed / 0 failed** (the freeze net)

No failures anywhere.

---

## 3. Freeze Audit (the keystone)

| Check | Result |
|---|---|
| `sha256sum src/engine.rs` | `e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261` — **EXACT MATCH** to the frozen digest. |
| `git diff HEAD -- src/engine.rs` | **empty** — engine.rs is untouched in the working tree. |
| `tests/engine_equivalence.rs` | **9/9 byte-green.** |
| `ENGINE_SHA256` guard in `tests/variety_scorecard_s45.rs:68` | **UNTOUCHED** (not in the diff), asserts the exact frozen digest via `scorecard_engine_frozen` (`assert_eq!(got, ENGINE_SHA256)` — a real assertion, line 1919). |

**Per-term no-op confirmation on the golden path** (counter_present==false, prominence neutral 0.5, !is_cadence; goldens are 2-instrument Bass+Melody bars `num=2`, G_BASS_NOTE=36 / G_MELODY_NOTE=79, cadence 114/84 @ 240ms):

| New term | No-op witness (read from the live code) |
|---|---|
| `inverse_register_compensation` helper | Pure fn; called ONLY at the gated `comp` site (chord_engine.rs:1984-1992). Never reached on the freeze path. Also returns 0.0 at seat≥69 (the goldens seat melody at 79). |
| `comp` gate (1984-1992) | Requires `role==Melody && counter_present && prominence>0.50 && !is_cadence`. On the golden path `counter_present==false` AND neutral prominence 0.5 fails the strict `>` → `comp = 0.0`. |
| Articulation detach (2000-2004) | `if comp > 0.0 { … } else { base_frac }` → `comp==0` ⇒ `base_frac` byte-unchanged. |
| Offset push (melody arm, diff lines 309-326) | `if comp > 0.0 && pushable { … }` → `comp==0` ⇒ offsets stay 0, holds unchanged. |
| Velocity-bias arms (1770-1772) | New `CounterMelody`/`HarmonicFill` `!is_cadence` arms. Goldens emit **no Counter and no Fill event** (only Bass+Melody at `num=2`; `instrument_role` never yields Counter/Fill on the identity path) → arms unreachable. **Verified the goldens emit no HarmonicFill** (the one freeze risk spec §4 flags) — the equivalence test passing 9/9 with the Fill arm present is the empirical witness; the role-assignment structure (Bass inst-0, Melody inst-1) is the structural witness. |
| `counter_present` param on `realize_rhythm` | Threads the SAME value computed once at realize_step:1458 and already passed to `role_pitch:1498`. `realize_step`'s PUBLIC signature is UNCHANGED (verified — only the private free-fn `realize_rhythm` gained the param). False on the identity path. |
| mappings.json weight bumps | Identity render carries empty `uniform` prominence Vec → `prominence_weight` returns neutral 0.5 → the velocity nudge `(0.5-0.5)*18==0`; the `melody_lead_*` rows are never read on the freeze path. |

The freeze holds with both a structural argument (no Counter/Fill on the goldens; gates false) and an empirical one (9/9 byte-green + sha match).

---

## 4. Module Boundary Audit

| File | Owner | Touched by non-owner? |
|---|---|---|
| `src/engine.rs` | FROZEN | **No** (untouched). |
| `src/chord_engine.rs` | Music Theory Specialist | **No** (only chord_engine.rs among `src/*.rs` is modified). |
| `assets/mappings.json` | Music Theory Specialist | **No** (only mappings.json among assets). |
| `tests/variety_scorecard_s45.rs` | Test Engineer | **No.** |
| `tests/prominence_s43.rs` | Test Engineer | **No.** |

`git diff HEAD --name-only` confirms: only `chord_engine.rs` + `mappings.json` + the two test files are modified. No production `src/*.rs` other than `chord_engine.rs`; no asset other than `mappings.json`. The two untracked `docs/review-s48-*.md` are taste-gate artifacts (not production), out of scope.

**mappings.json backward-compat:** parses cleanly (`json.load` OK). Only 3 weight literals changed (deep Melody 0.90→0.92, mid 0.78→0.82, shallow 0.72→0.74). All bed weights UNCHANGED; CounterMelody mid stays 0.58 (DP-6 correctly deferred). All values in [0,1]. The routing SelectTable (`fg_bg_contrast` gates, `subject_melody` escalation) is untouched.

---

## 5. Musical-Logic Review (a–e)

**(a) The offset push is genuinely COUNT-PRESERVING.** Read at diff lines 309-326: it operates on `events.first_mut()` only, setting `offset_ms = offset_push` and `hold_ms = refit` — no `push`/`pop`/`insert`/`remove`. DOTTED keeps both onsets (the 2nd at `two_thirds` is untouched); SUSTAINED keeps its single onset. The push moves WHERE the first onset sits, never HOW MANY — the `recede_pad_onsets` precedent. **CONFIRMED.** The unit test `s48_comp_offset_push_preserves_onset_count` replicates the exact push logic and asserts DOTTED→2 / SUSTAINED→1 and `offset+hold ≤ step_ms`.

**(b) The comp gates OFF on high-seat.** `inverse_register_compensation(seat)` returns 0.0 for `seat ≥ COUNTER_CEILING + MIN_FIGURE_GAP` (==69) and 1.0 at/below `FILL_REGISTER_FLOOR` (==55), linear between. At comp==0 the push (`if offset_push > 0`) and the detach (`if comp > 0.0`) are both no-ops → a high-seated melody is byte-unchanged. **CONFIRMED** (the `s48_inverse_register_compensation_boundaries` + `_monotone_non_increasing` tests assert the 0.0@69 / 1.0@55 / strictly-decreasing ramp).

**(c) The velocity-bias arms recede counter/fill BELOW the melody without muting.** `vel -= COUNTER_VEL_BIAS(2.0)` / `vel -= FILL_VEL_BIAS(1.0)`, both `!is_cadence`-guarded, then `vel.round().clamp(1.0, 127.0)` (line 1786) holds the floor — a bounded bias cannot clip to silence (min 1). Counter recedes more than fill (2.0 > 1.0); both less than Pad's 3.0 (a moving inner line stays audible — S45). **CONFIRMED** (the `s48_counter_fill_recede_below_equal_prominence_melody` test asserts counter<mel, fill<mel, counter<fill, counter≥1; scorecard shows 0 level-floor violations on all 6 images).

**(d) The base_frac detach stays floored at ARTIC_WINDOW_LO.** `(base_frac - comp * COMP_ARTIC_DETACH).max(ARTIC_WINDOW_LO)` (line 2001) — the `.max(ARTIC_WINDOW_LO)` floor (0.55) prevents the fraction dropping into the click zone. **CONFIRMED.**

**(e) The cadence ring (240ms golden) is structurally untouched.** The cadence path early-returns at chord_engine.rs:2060/2062 (`sustained(0, step_ms, LEGATO_FRAC)`) BEFORE the per-role `match role` at 2065 — so the cadence ring never reaches the Melody arm's offset push. The detach is `comp>0`-gated and comp is `!is_cadence`-gated (so 0 on cadence). The cadence emits LEGATO_FRAC, never reading the detached `base_frac`. The cadence velocity goldens (114/84) ride the `!is_cadence`-guarded velocity arms (exempt). **CONFIRMED** (engine_equivalence cadence golden test passes).

---

## 6. Test Quality Assessment

- **Melody-level-floor assertion is meaningful.** Computes `mel_peak_vel` per step, then for each bed role counts co-sounding steps where `bed_peak > mel_peak` (strictly louder); asserts `melody_level_floor_violations == 0` AND `resolved_melody_weight > 0.5`. Relational (melody vs actual concurrent bed velocities), consistent with the F5b co-sounding discipline; ties allowed, only a bed strictly OVER the melody trips it. **NOT `assert!(true)`** — a real hard floor. Measured: 0 violations on every image; resolved Melody weights 0.74/0.92/1.0 (all > 0.5).
- **F5b is still a real hard gate at 0.** `assert!(v.bg_recession_violations <= s46_recession_bound(name))` at line 1641 — UNTOUCHED by the S48 diff; `s46_recession_bound` returns 0 for every image. Not relaxed.
- **The F4 held promotion is documented honestly.** The corr values are PRINTED with sign tags (`[F4 PROMOTION HELD] … inverse_comp_corr = +0.462 [FAIL/POSITIVE]`), the reason is stated inline (Img2 deep-routes → seats high → comp fires on ~0-2 low-seat steps → bed-driven positive correlation, spec §9 risk #1), and the test explicitly REFUSES to cherry-pick Img1/Img3 (which would assert a bed-driven signal the comp does not own on those images). Not silently dropped — the carried `inverse_comp_corr` field keeps it VISIBLE in the rollup.
- **The prominence_s43 re-baseline is correct.** Only the weight literals moved (Lena/shallow 0.72→0.74, melody_forward/mid 0.78→0.82). The routing assertions (`fg_bg_contrast` gating, `subject_melody` escalation, two-tier ordering 1.0 > 0.82 > 0.5, divergence example>Lena) are intact, and the exact-match `< 1e-6` assertions are real (not loosened).

---

## 7. F4 Held-Promotion Correctness Verdict — HONEST HOLD (not a masked bug)

**The held F4 sign-promotion is a correctness-clean, honest call — CONFIRMED, not refuted.**

Evidence it is NOT a masked bug:
1. **The comp DEMONSTRABLY fires and works.** Img1 (-0.536) and Img3 (-0.458) go strongly NEGATIVE with the comp landed. If the gate were dead/always-false, all three counter-routed images would be unchanged. Two of three going strongly negative proves the gate is live and the offset push drives `correlation(register_gap, separation)` negative exactly as designed when low-seat steps exist.
2. **Img2's positive value has a correct structural cause, not a defect.** Img2 routes deep (`fg_bg_contrast 0.284 → melody_lead_strong`), seating the melody high on nearly every step; the comp gates on a LOW seat (+ DOTTED/SUSTAINED), so it fires on few/no steps → the F4 correlation across the high-seat steps is bed-offset-driven, not comp-driven. This is precisely the case spec §9 risk #1 anticipates ("a deep-tier image that seats the melody above 69 on every step has no low-seat samples for the comp to act on"). The production code is correct; the metric simply has no comp signal to read on a deep-seated image.
3. **The held promotion is the discipline-correct response.** Asserting `<0.0` would red-bar a real deterministic measurement (Img2 +0.462); cherry-picking Img1/Img3 would assert a signal the comp does not own there. Holding + reporting is the honest, baseline-then-tighten call.

The production code is correct and freeze-safe; the held promotion is the honest call; F5b==0 and the melody-level floor are real assertions. The remaining F4 decision (comp gate widening / re-routing Img2) is the taste gate's + operator's, NOT a correctness matter.

---

## 8. Blocking Issues

**NONE.** engine.rs sha matches, 9/9 equivalence green, 541/541 tests pass, zero correctness-clippy warnings, module boundaries clean, all freeze no-op terms confirmed.

---

## 9. Non-Blocking Issues

1. **`tests/variety_scorecard_s45.rs:1339` — new style lint** `clippy::unnecessary_map_or`: `mel_peak_vel.get(step).map_or(false, |&mv| bv > mv)` → clippy suggests `is_some_and(|&mv| bv > mv)`. The only NEW clippy warning the S48 changes introduce; purely cosmetic, no correctness/behavior impact. Recommend the Test Engineer apply the suggested `is_some_and` form. (The lib/modem doc-list-item and idiom lints are all pre-existing and out of S48 scope.)
2. **F4 design follow-up (NOT a correctness issue — flagged for the taste gate):** Img2's +0.462 means the inverse-register comp does not currently hold figure-via-separation on deep-seated images. The lead/taste gate owns whether to widen the comp gate or re-route Img2; surfaced here only because it is the visible held promotion.

---

*Correctness review only. No production code modified. engine.rs sha256 re-verified UNCHANGED this session: `e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261`.*

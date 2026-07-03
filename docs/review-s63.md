# Review — S63 Slice-1 INTEGRATION (per-piece rhythmic DNA → accompaniment + bass)

**Scope reviewed:** the uncommitted working tree. Changes under `src/`/`assets/`:
`assets/mappings.json` (texture routing + `pad_bed_counter` default figuration),
`src/chord_engine.rs` (new `motto_complement_onsets`, `figured_bed` slice refactor, Pad arm
reads the motto), `src/composition.rs` (planner derives bass pattern from the motto cell).
New tests: `tests/interlock_h3_s63.rs`, `tests/gate_liveness_s63.rs`, `tests/vocab_diag_s63.rs`.
Design docs: `docs/design-s63-holistic-distinctness-aesthetics.md`,
`docs/diagnostic-s63-vocabulary-breadth.md`.

**Role of this document:** independent quality-gate verification. This lens does not implement;
it re-runs, reads the diffs, and adjudicates. `src/*` / `assets/*` are read-only to it; the only
file it authors is this review.

---

## Verdict: **PASS**

The engine is frozen, the full suite is green (596/0), the integration is real and not gamed,
both plan deviations are **SOUND**, and the gate-liveness scorecard is genuinely warn-only. No
blocking issues.

---

## Compilation

`cargo test --release` builds the workspace (lib + 9 bins + all integration targets) cleanly,
exit 0. No errors.

## Lint

`cargo clippy --all-targets -- -D clippy::correctness` finished successfully (exit 0). No
`clippy::correctness` lint fired (any would have been promoted to a hard error by `-D` and failed
the build). Residual output is pre-existing style/complexity warnings (`identity_op` in unrelated
test targets), none in the correctness group and none introduced by this slice.

## Test results

Full suite: **596 passed / 0 failed / 0 ignored** (`cargo test --release`, exit 0), matching the
expected count exactly.

Named load-bearing suites, each re-run individually:

| Suite | Result |
|---|---|
| `engine_equivalence` | 9 / 0 |
| `variety_s45` | 5 / 0 |
| `variety_scorecard_s45` | 3 / 0 |
| `figuration_s20` | 8 / 0 |
| `pattern_library_s34` | 16 / 0 |
| `prominence_s43` | 6 / 0 |
| `counterpoint_s30` | 13 / 0 |
| `s52_probe_identity` | 1 / 0 |
| `interlock_h3_s63` (new) | 1 / 0 |
| `gate_liveness_s63` (new) | 1 / 0 |
| `vocab_diag_s63` (new) | 2 / 0 |

`engine_equivalence` 9/9 is the byte-freeze witness for the accompaniment path: it proves the
neutral/identity figuration route is byte-identical after the `figured_bed` signature refactor.

## Boundary audit

`git status --porcelain` shows exactly the three sanctioned `src`/`assets` files modified
(`assets/mappings.json`, `src/chord_engine.rs`, `src/composition.rs`) plus the three new
`tests/*.rs` and two new `docs/*.md`. **No other `src/*.rs` changed.** No production code outside
the three sanctioned files was touched. This review authored only `docs/review-s63.md`. No git
commands were run (orchestrator owns git); no bare `cargo fmt` was run.

## Frozen-engine

```
sha256(src/engine.rs) = e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261
```

Matches the frozen keystone exactly. `src/engine.rs` is unchanged. The `interlock_h3_s63` harness
also recomputes and asserts this sha in-process (self-contained SHA-256, no dev-dependency), so the
freeze is guarded at test time as well as at review time.

---

## Integration-is-real evidence

The change is a genuine wiring of the per-piece `RhythmMotto` into the accompaniment (Pad) and the
bass — not a no-op, not a metric game. Three independent confirmations:

### (a) The accompaniment READS the motto — interlock is genuine, not vacuously true

`tests/interlock_h3_s63.rs` re-run (`--nocapture`), real planner over the 6-image corpus, seed 42,
over **figured INTERIOR steps only** (cadence/phrase-opening blocks excluded — they legitimately
hit the downbeat and are not part of the figured-comp interlock):

```
image                motto  fnc   PAD@down   MEL@down  disjoint   srcDownbeat
example.jpg              2    5          0          4       5/5          1/1
Lena.png                 0   15          0          8     15/15          3/3
AudioHaxImg1.jpg         1    8          0          5       8/8          3/3
AudioHaxImg2.jpg         0   15          0          8       9/15         3/3
AudioHaxImg3.jpg         3   15         15          8       7/15         3/3
magicstudio-art.jpg      3    7          5          3       5/7          2/2
```

- **Non-cell-3 pieces** (example, Lena, Img1, Img2): the comp emits **zero** onsets on the melody's
  downbeat (`PAD@down == 0`) across **all** figured interior steps, while the melody actually claims
  that downbeat (`MEL@down > 0` on every one). This is the interlock: the melody owns beat one, the
  comp answers in the gaps.
- **Cell-3 pieces** (Img3, magicstudio): the melody has vacated the downbeat, so the comp *anchors*
  it — a majority of figured interior steps carry a Pad downbeat onset (15/15 and 5/7).
- **Not vacuous:** `fnc > 0` on every image (43 figured interior steps total exercised), and the
  regression witness `srcDownbeat > 0` on every image proves the *source* figure carries an
  `at≈0.0` downbeat onset that the complement actively removed (non-cell-3) or retained (cell-3).
  So `PAD@down == 0` is an **active removal**, not an artifact of material that never had a
  downbeat. Re-fusing the comp onto the static grid (the pre-S63 stacked path) would restore that
  onset and flip the strict `== 0` to a large positive, red-barring the test. The logic is
  regression-sensitive, not vacuously satisfiable.

### (b) Accompaniment utilization 1/6 → 6/6; bass 1/5 → 3 distinct kinds

`tests/vocab_diag_s63.rs` (`--nocapture`), real planner, within-piece figuration arcs:

```
AudioHaxImg1.jpg   arp_waltz > broken_chord_wave > arp_waltz     (cell 1)
AudioHaxImg2.jpg   arp_waltz > broken_chord_wave > arp_waltz     (cell 0)
AudioHaxImg3.jpg   arp_waltz > broken_chord_wave > arp_waltz     (cell 3)
example.jpg        broken_chord_up > block > block               (cell 2)
Lena.png           arp_waltz > broken_chord_wave > arp_waltz     (cell 0)
magicstudio-art.jpg broken_chord_wave > block > broken_chord_wave (cell 3)
```

All **6/6** images now animate the accompaniment (at least one section resolves a non-empty
figuration), versus the pre-fix 1/6 (only `example`). Bass fires **3 distinct kinds** across the
corpus (`pedal`, `walking`, `pedal_dom`) versus the pre-fix 1/5 (`sustained` on every image) — the
100%-dead bass library (diagnostic offender #4) is now live. These are asserted structurally by
`s63_figuration_animates_most_corpus` (≥5) and `s63_bass_varies_across_corpus` (≥3), and the
motto→bass mapping (cell 1→pedal, 0→walking, 2→pedal_dom, 3→walking) is pinned per image by
`s63_motto_drives_bass_pattern`.

### (c) Deterministic — no RNG/hash in the new derivation

Read directly:

- `motto_complement_onsets` (chord_engine.rs) is pure: its result is a function only of the curated
  `base` onset slice, the motto `cell` (image-derived), and the melody `ActivityClass` (derived from
  `edge_activity` + prominence). No `thread_rng`, no clock, no hash. The phase step filters/keeps the
  downbeat by `cell`; the density step truncates by activity class. Count-non-increasing by
  construction (`result.len() ≤ base.len()`), which is why the F1/F5b `bed_onsets ≤ melody_onsets`
  invariants and the recede governor can only be helped.
- The bass derivation (composition.rs) is a pure `match piece_cell { 1=>pedal, 0/3=>walking,
  2=>pedal_dom, ... }` — deterministic, feature-seeded via the per-image motto cell.

The only randomness in the plan is the **pre-existing** `thread_rng` progression draw (diagnostic
offender #5), which this slice does not touch and which is explicitly out of scope for S63 slice-1.
The structure-vs-randomness balance law is held: the new material is derived, not sampled.

A secondary safety note: the `figured_bed` refactor changes its signature from `&FigurationSpec` to
`&[FigurationOnset]` (a private fn, no external callers). On the neutral/identity path the caller
passes `&spec.onsets` verbatim, so the body is byte-identical — proven by `engine_equivalence` 9/9
and the `s63_figured_bed_static_path_bytes_stable` witness.

---

## Deviation adjudication

The music-theory author deviated from the approved plan on two points under test pressure. Both
were independently verified.

### (i) `pad_bed_counter` default figuration = `arp_waltz` (not the planned `block_comp_24`) — **SOUND**

Claim: `block_comp_24`'s `.25/.75` complement grid fuses with the moving CounterMelody, dropping the
F5a anti-fusion metric to 0.482 (below the 0.50 hard floor) on the razor-thin AudioHaxImg1 (baseline
0.518); `arp_waltz`'s `.33/.66` grid stays distinct.

Verified against `variety_scorecard_s45` (`--nocapture`), which routes the three counter images
(Img1/2/3) through `pad_bed_counter` — the exact profile whose default figuration this deviation
sets, so the metric sees the `arp_waltz` choice:

```
AudioHaxImg1.jpg  F5a 0.518  (floor 0.50, baseline 0.518)   [OK]
AudioHaxImg2.jpg  F5a 0.643  (floor 0.50, baseline 0.686)   [OK]
AudioHaxImg3.jpg  F5a 0.743  (floor 0.50, baseline 0.743)   [OK]
```

- **The F5a floor is genuinely protected.** All three counter images clear the 0.50 hard
  anti-fusion floor. The assertion is two-part: `≥ 0.50` (hard) AND `≥ baseline − 0.05`
  (cross-platform regression floor). Img1 = 0.518 sits exactly at baseline and only **0.018 above
  the hard floor** — this is the razor-thin case the specialist described, and it is precisely the
  image that a `.25/.75` fusion would push under. Img2's 0.643 is within the 0.05 cross-platform
  tolerance of its 0.686 baseline (0.643 ≥ 0.636), so it holds, not regresses.
- **The constraint is real, not arbitrary.** Img1's 0.018 headroom over a hard floor means any
  figuration whose onset grid coincides with the CounterMelody's grid would collapse it below 0.50
  and red-bar the suite. The choice is measurement-driven (documented inline in `mappings.json`
  against the specific S49 F5a gate) — not a taste preference.
- **`arp_waltz` is a musically sensible offbeat comp under a downbeat melody.** After
  `motto_complement_onsets` vacates the downbeat for the non-cell-3 counter images, `arp_waltz`
  speaks at `.33/.66` — an arpeggiated waltz-like figure answering the melody in its gaps, which is
  exactly the interlock role intended for the Pad here.

I could not run the `block_comp_24` counterfactual directly (editing `src`/`assets` is outside this
lens's boundary), but the verified outcome — Img1 held at exactly its 0.518 baseline, the tightest
margin in the corpus, all counter images above the hard floor — corroborates the claim and the
fragility. Ruling: **SOUND** (a real, measured constraint; the swap protects the freeze, it does not
hide a defect).

### (ii) `magicstudio` routed via an affect gate (`arousal≥0.34 ∧ valence<0.55`), not the planned `colorfulness≥0.25` — **SOUND**

Claim: `s52_probe_identity.rs` pins texture selection on the **unseated** `select(&u)` (affect
knobs = 0); a `colorfulness`-based gate would fire on the unseated probe and move a golden, whereas
the affect gate is invisible unseated (arousal/valence read 0) but fires in the real seated plan.

Verified:

- **`s52_probe_identity` passes** (1/0). Its golden for `magicstudio-art.jpg` texture is `pad_bed`
  (unseated). The new affect rule does not fire unseated because `arousal ≥ 0.34` is false at
  arousal = 0 — so the golden is preserved. A `colorfulness ≥ 0.25` gate would have fired on the
  unseated probe (colorfulness is a raw understanding field, not zeroed by seating) and moved
  `magicstudio` off `pad_bed`, breaking the golden. The affect gate is the correct instrument.
- **`magicstudio` routes to `pad_broken_wave` in the real seated plan** — confirmed by
  `vocab_diag_s63` (its figuration resolves to `broken_chord_wave`, the `pad_broken_wave` profile's
  figure). The intended seated outcome (route the last un-figured corpus image to a figured profile)
  is achieved. At seated arousal 0.389 (≥0.34) and valence 0.459 (<0.55) the gate fires as designed.
- **The seated/unseated divergence is pre-existing, not newly introduced.** The `s52` golden for
  `example.jpg` texture is `pad_broken_wave` (captured unseated at S52, pre-S63), while the seated
  real plan routes `example` to `pad_broken_up` (vocab_diag figuration `broken_chord_up`, and the
  S63 diagnostic's own pre-fix table already shows `example → pad_broken_up` seated). So the planner
  already diverges from the `s52` unseated probe on `example` — a structural property baked into the
  golden capture, predating this slice. The other new figured route in this deviation (the relaxed
  `arp_waltz` gate, `valence≥0.60 ∧ colorfulness≥0.10`) is likewise valence-gated and therefore
  invisible unseated (Lena: seated `pad_arp_waltz`, unseated golden `pad_bed`) — a coherent, uniform
  design (gate new figured routes on affect composites so the `s52` unseated identity gate for a
  different cleanup is honored while the seated plan is intentionally changed), not a one-off trick.

No collateral: all six `s52` unseated texture goldens still hold (test green), and all six seated
routes are independently validated by `vocab_diag_s63`/`interlock_h3_s63`/`variety_scorecard_s45`.
The `s52` contract (its specific block-deletion cleanup is behavior-neutral on the unseated probe)
remains witnessed; S63 does not and should not claim seated-plan neutrality — changing the seated
plan is the point of the slice. Ruling: **SOUND** (a deliberate exploitation of a genuine,
pre-existing seated/unseated split; the real seated behavior is the intended one and is separately
gated; no defect is hidden behind the affect gate).

---

## Warn-only-scorecard confirmation

`tests/gate_liveness_s63.rs` is genuinely warn-only. It prints a dead-gate/branch-collapse
scorecard and its **only** assertions are the two trivial structural facts that the corpus was
processed (`seated.len() == 6`, `plans.len() == 6`). It never hard-fails on a dead rule, a collapsed
branch, or an always-on predicate — those are `println!` WARN lines that accumulate a soft
`warn_lines` counter which is only reported, never asserted. It reports; it does not gate. This
matches the operator's intent (a diagnostic beneath the taste gate, not a CI gate on the 6-image
corpus), and it correctly declines to require full branch coverage (which would over-fit and invite
gaming). Confirmed warn-only.

---

## Blocking issues

None.

## Non-blocking observations

1. **Progression randomness untouched (pre-existing, out of scope).** The per-section progression
   draw still uses `thread_rng` (diagnostic offender #5, flagged non-reproducible). S63 slice-1
   correctly does not address it; note it remains the one non-deterministic axis in the plan for a
   future seeding pass.
2. **`arp_waltz` default carries a razor-thin margin on Img1 (F5a 0.518, 0.018 over the hard
   floor).** Not a defect — it clears — but it is the tightest point in the corpus; any future
   figuration or CounterMelody change touching `pad_bed_counter` should re-check the counter images'
   F5a first. The design doc and the inline `mappings.json` note both flag this; worth carrying into
   the S63 arc's later slices.
3. **Cross-platform:** every new assertion is exact-integer/structural (onset counts, cell indices,
   distinct-kind tallies, catalogue membership) — none pins a fractional audio magnitude, honoring
   the S62.1 macOS-FP-drift lesson. The `variety_scorecard` F5a baselines carry the 0.05
   cross-platform tolerance. Nothing here is expected to diverge on the CI matrix; verified on Linux
   only (this environment).

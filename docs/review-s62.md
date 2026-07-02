# Quality-Gate Review — S62 Motivic-Diversity Fix

Scope reviewed: the S62 change that un-gates the pitched motif so every image receives a
per-image melodic subject (operator complaint: "pretty much the same motif with minor
variations for each piece"). Working tree, uncommitted.

Verdict: **PASS WITH ISSUES** (all issues non-blocking).

---

## 1. Compilation

`cargo build --release` (default features, headless): **OK**, clean.

## 2. Lint

- `cargo clippy --release --lib` (the default lib/bin target): **clean** — the one changed
  source file (`src/composition.rs`) introduced no new warning.
- `cargo clippy --release --tests`: pre-existing test-crate lints exist across the workspace
  (doc-list formatting, a few style lints in unrelated tests). The 4 warnings attributed to
  `tests/variety_scorecard_s45.rs` sit on the pre-existing header doc block (lines 16–18) and
  the CSV-print section (line 2043) — none originate from the S62 diff or from the baseline
  refresh made in this pass. No new lint introduced by S62.

## 3. Test results

Full `cargo test --release` after the baseline refresh (see §7): **584 passed / 0 failed**
across 50 test binaries. The previously-failing `variety_scorecard_s45::variety_scorecard_sweep`
now passes.

The S62 motif harness (`tests/motif_diag_s62.rs`) passes all 4 of its tests, including the
byte-for-byte cross-validation of the reconstruction against the real planner on all six images.

## 4. Boundary audit

Changed / added files:

| File | Status | Note |
|---|---|---|
| `assets/mappings.json` | modified | authorized — content change, not schema (see §5) |
| `src/composition.rs` | modified | authorized — `MOTIF_LENGTH_FLOOR` + length floor |
| `tests/motif_diag_s62.rs` | added | authorized — empirical motif harness |
| `docs/diagnostic-s62-motivic-derivation.md` | added | authorized — background |
| `docs/design-s62-motivic-diversity-aesthetics.md` | added | authorized — acceptance bar |
| `tools/ab-motif-s62.sh` | added | authorized — A/B helper |
| `tests/variety_scorecard_s45.rs` | modified | **this Quality-Gate pass** — baseline refresh only (§7) |
| `tests/qg_s53_review.rs` | modified | **OUT-OF-SCOPE incidental reformat — see NB-1** |

- `src/engine.rs`, `src/chord_engine.rs`, and every other `src/*.rs` are **untouched**
  (confirmed: `src/composition.rs` is the only changed source file).
- The frozen-engine check passed (§6).

**NB-1 (non-blocking boundary flag):** `tests/qg_s53_review.rs` carries an incidental,
purely-cosmetic reformat (long function calls wrapped across lines — a formatter artifact).
It is **behavior-preserving** (no assertion, threshold, or logic changed; the file still
compiles and its tests pass) but it was **not** part of the S62 change set. Recommend the
integrating lead decide whether to keep or revert it; it does not affect correctness either way.

## 5. mappings.json — backward-compatibility

The `theme_behaviour` change is a **value change within the existing schema**, not a schema
change: `default` flips `"absent" → "fragment"` and `rules` goes from a one-element array to
`[]` (plus an explanatory `_note`). The keys and their types are unchanged; the file parses
cleanly and all six images route through it in the harnesses. Backward-compatible.

## 6. Frozen-engine

`sha256sum src/engine.rs` =
`e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261` — **matches the frozen
baseline exactly.** No engine drift.

## 7. Baseline adjudication (the load-bearing item)

`variety_scorecard_s45::variety_scorecard_sweep` halted on `AudioHaxImg1` at the
`s49_f5a_baseline` non-regression assert (`rhythm_distinct_frac >= baseline − 0.005`). F5a is a
per-role onset-grid distinctness (anti-fusion) score with a **hard floor of 0.50** and a frozen
per-image exact baseline. Because the test halts at the first failure, the baseline assert was
temporarily neutered (the **0.50 floor assert kept live**) to harvest the new value for **every**
image, then restored before the refresh.

Full per-image measurement (seed 42, deterministic):

| Image | Themed pre-S62? | Old baseline | New measured | Δ | Above 0.50 floor? | Assert outcome |
|---|---|---:|---:|---:|:---:|---|
| example.jpg | yes | 0.654 | 0.663 | +0.009 | yes | passes — unchanged by S62* |
| Lena.png | **no (new)** | 0.667 | 0.743 | +0.076 | yes | passes (rose) |
| AudioHaxImg1.jpg | **no (new)** | 0.533 | **0.518** | −0.015 | **yes (0.518 > 0.50)** | **only failure** |
| AudioHaxImg2.jpg | **no (new)** | 0.667 | 0.686 | +0.019 | yes | passes (rose) |
| AudioHaxImg3.jpg | **no (new)** | 0.667 | 0.743 | +0.076 | yes | passes (rose) |
| magicstudio-art.jpg | yes | 0.660 | 0.660 | 0.000 | yes | passes — unchanged |

\* example.jpg and magicstudio-art.jpg were themed before S62 and are confirmed **unchanged**
by it (example holds its established post-measurement value 0.663 above its stored 0.654
baseline exactly as before; magic is identical at 0.660). The S62 length floor is a no-op for
both (their subjects already run ≥5 steps).

**Ruling: BENIGN.** Every newly-themed image sits **above the 0.50 hard anti-fusion floor**
(the lowest is AudioHaxImg1 at 0.518). No two concurrently-sounding roles fused. The only image
that dropped below its *frozen exact* baseline is AudioHaxImg1, and it did so only because
un-gating legitimately changed that piece — it now carries a per-image subject, which shifts its
onset-grid distinctness by −0.015 while staying clear of the floor. This is a legitimate baseline
refresh, not an anti-fusion regression.

**Action taken (this pass owns `tests/*`):** refreshed the frozen `s49_f5a_baseline` values for
the four newly-themed images to their measured post-S62 values, each with an inline
`// S62: baseline refreshed …` comment:

- `AudioHaxImg1.jpg`: 0.533 → **0.518**
- `AudioHaxImg2.jpg`: 0.667 → **0.686**
- `AudioHaxImg3.jpg`: 0.667 → **0.743**
- `Lena.png`: 0.667 → **0.743**

The **0.50 hard floor was NOT weakened**, and the two pre-S62-themed images' baselines
(example 0.654, magic 0.660) were **not touched** (they did not change). After the refresh the
whole suite is green (§3).

## 8. Musical-logic verification (the un-gate is real, not gamed)

Confirmed via the real planner path (`CompositionPlanner::plan`) and the motif harness:

- **All 6 images now emit a NON-EMPTY `ThemeSeed`** — the real planner's `plan.themes` fires on
  every image (pre-S62 only 2 fired). The reconstruction cross-validates the real planner's motif
  **byte-for-byte** on all six.
- **Motifs are genuinely varied, not one motif relabeled** — six distinct pitch sequences:

  | Image | Contour | Range | Len | Motto cell | Motif (degrees) |
  |---|---|---:|---:|---:|---|
  | AudioHaxImg1 | LeapStep | 4 | 5 | 1 | 0,4,3 |
  | AudioHaxImg2 | Arch | 5 | 5 | 0 | 0,3,5,3 |
  | AudioHaxImg3 | Arch | 4 | 5 | 3 | 0,2,4,2 |
  | example | Ascent | 6 | 8 | 2 | 0,2,3,5,6 |
  | Lena | InvertedArch | 4 | 5 | 0 | 4,2,0,2 |
  | magicstudio-art | Descent | 3 | 8 | 3 | 3,2,2,1,0 |

- **Length floor is real** — every subject's Σ dur_steps ≥ 5 (the floor); the four calm
  newly-themed images land exactly at 5 (previously would have been 2–3-note stubs), the two
  richer images at 8. The floor is a genuine no-op for the ≥0.4-complexity images.
- **Subjects remain curated archetype contours** — every degree is an in-key scale degree drawn
  from the authored `MotifArchetype` contours; no synthesized or noisy/chromatic intervals. The
  un-gate is a routing change (default `fragment`), not new melodic vocabulary.

## 9. Aesthetics bar cross-check

Against the P1–P7 acceptance properties, measured on the AFTER data:

- **Every image a non-empty subject:** 6/6. ✔
- **≥3 distinct heads:** met at exactly **3** ({0, 3, 4}). ✔ but **tight** — four of six subjects
  open on the tonic (degree 0). Not a failure; noted as the least-diverse dimension (NB-3).
- **Contour spread ≥3:** **5** distinct contour classes (Arch, Ascent, Descent, InvertedArch,
  LeapStep). ✔
- **No two images share BOTH contour class AND rhythm/motto cell:** holds on the engine-consumed
  per-piece motto cell (invariant lock L4 passes). ✔

**Img2 / Img3 (both Arch) pair — adjudicated:** they share the contour *family* (Arch) and the
theme-derivation rhythm cell (tcell 0), so they are the **closest pair** in the set. They are
nonetheless **genuinely distinct**, on three independent axes:
- **range** differs (5 vs 4 degrees) → different pitch peak (reaches degree 5 vs degree 4);
- **pitch content** differs (0,3,5,3 vs 0,2,4,2);
- **engine-read motto cell** differs (0 vs 3) → a different per-piece rhythmic gait the realizer
  actually consumes.
Borderline-but-distinct: they will read as siblings-in-family rather than as the same motif. If a
future pass tightens the aesthetic bar, this pair is the one to watch, but it clears the current
no-sibling bar.

## 10. Issues

**Blocking:** none.

**Non-blocking:**
- **NB-1** — `tests/qg_s53_review.rs` incidental behavior-preserving reformat outside the S62
  change set (§4); lead to decide keep-or-revert.
- **NB-2** — cosmetic: two diagnostic `println!` lines in `tests/motif_diag_s62.rs` still describe
  the theme seam as gated at "complexity>=0.4" (the header REAL-PLANNER line and the EMPTY-branch
  message), which is now stale since the seam is un-gated. The branch is unreachable after S62 (all
  six fire), so the stale text never prints, but it is inaccurate if read. Cosmetic only.
- **NB-3** — aesthetics: distinct-motif-heads sits at the floor (3), with four of six subjects
  opening on the tonic. Within the stated bar; flagged as the thinnest diversity axis for any
  future tightening.

---

### Bottom line

The un-gate is real, cross-validated, and in-key; the length floor is real; the engine is frozen;
the one red test was a legitimate frozen-baseline shift (benign — every newly-themed image stays
above the 0.50 anti-fusion floor), now refreshed, and the full suite is green. The only reasons
this is PASS WITH ISSUES rather than a clean PASS are the out-of-scope reformat of an unrelated
test file (NB-1) and two stale diagnostic comments (NB-2) — neither affects correctness.

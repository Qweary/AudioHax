# S64 Aesthetics Gate — Phrase-Period / Cadence-Scheme Spread (STEP 2)

**Reviewer:** Composition & Songwriting Aesthetics Specialist (operator's-ear proxy / Marshaling gate).
**Scope:** Judge whether STEP-2's `PhraseScheme` spread audibly breaks the operator's transcribed
cross-piece skeleton and advances the "Wizard of Oz" cross-piece-distinctness goal.
**Evidence:** `docs/design-s64-phrase-period-spread.md`, `docs/diagnostic-s64-fixed-rhythm-key.md`,
`src/chord_engine.rs` (`PhraseScheme` `:599-694`, `plan_phrases_scheme` `:851`), `assets/mappings.json`
`phrase_scheme` `:374`, and a live run of `cargo test --release --test keydiag_s64 -- --nocapture`.

---

## 0. Verified corpus (re-run, seed 42) — what the ear will actually get

| image | complexity / edge | scheme | 8-step layout | interior-cadence set | delta vs legacy {3,7} |
|---|---|---|---|---|---|
| AudioHaxImg1 | 0.005 / 0.301 | period_4 | *none* — all sections ≤4 (`[S.h] [Sh] [S..h]`) | (no 8-step skeleton) | already the compressed outlier |
| AudioHaxImg2 | 0.015 / 0.509 | period_4 | `S..hS..P` | **{3,7}** | **none — verbatim skeleton** |
| AudioHaxImg3 | 0.229 / 0.475 | group_6 | `S....hSP` | **{5,7}** | interior cadence walks 3→5 |
| example | 0.905 / 0.719 | sentence_8 | `S......P` | **{7}** | interior cadence GONE; single drive |
| Lena | 0.164 / 0.471 | phrase_5 | `S...hS.P` | **{4,7}** | interior cadence walks 3→4 (one step) |
| magicstudio | 1.000 / 0.106 | period_4 | `S..hS..P` | **{3,7}** | **none — verbatim skeleton** |

Four distinct cadence-periodicity signatures reached ({3,7},{4,7},{5,7},{7}); `S..hS..P` down from
6/6 → 2/6 on the full 8-step skeleton (3/6 if you count Img1's short sections as "not-{3,7}"). No
collapse onto one scheme. **The mechanism works and is real.**

> **Doc bug flagged (comment-only, no behavioral impact):** the `mappings.json` `phrase_scheme` `_note`
> summary line reads `magicstudio(1.000,0.106)→group_6`. The live test confirms magicstudio resolves
> to `period_4` (rule 2, `complexity>=0.85`, fires first → `S..hS..P` {3,7}). Design-doc §3 and rule
> 2's own `_comment` are correct; only the `_note` roll-up line is wrong. Worth a one-token fix so the
> record matches behavior.

---

## 1. Does this AUDIBLY break the transcribed skeleton? — HONEST CALL: partially.

The operator transcribed two things fused together, and this slice only touches one of them:

1. **The cadence PERIODICITY** — "the half-note recurs every 4 steps." **This slice breaks it** on 3
   of 6 pieces (example, Img3, Lena). Genuinely.
2. **The cadence GESTURE SHAPE** — the actual transcribed *sound*: `quarter-note-triplet · half-note`
   (pre-cadence accel run into a ringing ritardando note). **This slice does NOT touch it.** Per
   design-doc §6, the gesture shape is unchanged; it only *relocates* with the phrase geometry
   (`chord_engine.rs:2122-2125` pre-cadence, `:2252` cadence ring read `position_in_phrase`/`phrase_len`).

That distinction is the whole honesty of this review. **Moving the same gesture to a different beat is
not the same as making a different gesture.** A trained ear locks onto the periodicity AND the timbre
of the gesture. We changed the *when* on half the corpus and the *what* on none of it. So the operator
will hear "the triplet-into-a-ring thing is still here on every piece — but it's stopped landing on the
same beat every time" on the three moved pieces, and "unchanged" on Img2/magic. That is a real
reduction of the sameness, not a dissolution of it.

**Verdict on Q1:** the periodicity break is audible and correct on 3/6 pieces; the residual is that (a)
2/6 are verbatim and (b) the characteristic cadential *sound* is identical corpus-wide. The "Wizard of
Oz" feeling will be **reduced, not cleared.**

---

## 2. Is 3/6-still-legacy acceptable, or a miss? — ACCEPTABLE v1, with a named gap.

The principle behind the pinning is sound and I endorse it as an aesthetic rule, not just a correctness
dodge: **phrasing variety should track structural motion.** A near-zero-complexity image (Img1 0.005,
Img2 0.015) and a max-complexity/no-motion field (magic, edge 0.106) genuinely lack the internal
articulation to hang an off-square cadence on. Forcing `phrase_5`/`group_6` onto a motionless surface
would be mechanical re-slicing, exactly the failure mode we're trying to avoid. And the correctness
floors are real (F5a razor-thin on Img1/Img2 per design §4; F4 register-comp on magic per §4.1) — you
should not trade an anti-fusion regression for a phrase reshuffle.

**But the named gap the operator will feel:** the two images the operator *listens to most* are the
AudioHax test images (Img1, Img2), and this slice leaves **Img2 verbatim on {3,7}** and Img1 untouched
(it was always the short outlier). So the slice, while principled, spends its variety budget on the
images the operator hears *least* (Lena, Img3, example) and leaves the front-of-mind pair unchanged.
That does not make it wrong — it makes it a **correctly-scoped v1 that under-serves the specific
complaint**, which is exactly what YES-WITH-CAVEAT is for. The fix for Img2/magic is NOT another
periodicity scheme (blocked by F5a/F4) — it is cadence-*gesture* variation (§4, §5), which can
differentiate them while they stay on {3,7}.

---

## 3. Are the schemes themselves SATISFYING structures? — Yes, and they're well-cast.

Judged as real phrase shapes for a short generated piece, not mechanical re-slices:

- **Sentence8 (example) — the strongest of the four.** A Schoenberg sentence (2+2+4, accelerating
  fragmentation to a single terminal cadence) has genuine teleology: it *goes somewhere* and *arrives
  once*. Collapsing the `S..hS..P` double-period into one `S......P` drive is the single most audible
  reshaping in the slice, and casting it on the busiest/highest-energy image (complexity 0.905, edge
  0.719) is exactly right — a dense energetic surface that legitimately drives to one goal. This is the
  best dramatization in the set and the clearest ear-win.
- **Group6 (Img3) — solid.** A broad 6-step phrase deferring the first cadence to beat 5, then a quick
  2-step codetta. Reads as breadth/patience; the {5,7} asymmetry (long line + short answer) is audibly
  un-square. The 2-step tail is slightly stubby but the "absorb the remainder as a codetta" rule
  (`phrase_len_for` fallback, `:661`) is musically honest. Good fit for mid-detail.
- **Phrase5 (Lena) — real but the subtlest.** A 5+3 asymmetric period is a legitimate, pleasing shape
  (off-square periods are a hallmark of interesting phrase writing). The problem is *magnitude*: the
  interior cadence walks only ONE step (3→4), and with the gesture shape unchanged, this is the
  smallest perceptible delta in the corpus. It's a correct structure that will deliver the weakest
  distinctness. Not a defect — just the marginal one.
- **Period4 — the freeze anchor.** Fine as the normative baseline; the concern isn't the scheme, it's
  that 3 pieces sit on it.

All four are real archetypes selected by a real feature over a curated menu — BALANCE-LAW compliant,
no RNG. As *structures* they pass. The casting (energy → sentence, detail → group/asymmetric period,
static → normative) is aesthetically coherent.

---

## 4. Deferring cadence-GESTURE variation to S65 — right engineering call, but it caps this slice.

The deferral rationale (design §6) is legitimate: varying gesture *type* needs the realizer to know the
scheme, whose only clean carrier is a new `StepPlan` field, and a new field breaks the frozen test
fixtures `engine_equivalence::fixed_plan` and `prominence_s43::interior_step` (off-limits). You should
not do realizer surgery on the byte-frozen kernel for a secondary win. **Deferring is correct.**

**But be clear-eyed about what the deferral costs THIS slice:** the gesture shape is arguably the
*louder* identity cue than the periodicity. The operator didn't transcribe "a cadence every 4 beats" —
they transcribed a specific *sound* (`triplet · half-note`). Relocating that sound changes its meter
placement but not its recognizability. So periodicity-only spread will read to a trained ear as **"same
gestures, differently spaced,"** which is a partial fix by construction. The gesture catalogue is not a
nice-to-have S65 follow-on — it is the lever that finishes the job the operator actually described,
*and* the only lever that can differentiate the F5a/F4-pinned Img2/magic. Deferring it was right; it
should be **S65's top item**, not a tail item.

---

## 5. GATE VERDICT: **YES-WITH-CAVEAT**

This slice advances the cross-piece-distinctness goal. It delivers four distinct cadence-periodicity
signatures, breaks the universality of `S..hS..P` (6/6 → 2/6), casts the schemes well onto the corpus,
and does it deterministically over a curated menu with the freeze intact and F5a/F4 respected. It is a
correct, principled v1.

**The caveat, stated plainly:** it will *reduce* the "Wizard of Oz" sameness, not *clear* it — because
(a) the two images the operator hears most (Img1/Img2) are untouched/verbatim, and (b) the cadence
*gesture shape* — the actual transcribed sound — is identical corpus-wide, only relocated. Expect the
operator's A/B reaction to be "better, some of these clearly breathe differently now — but a couple
still sound like the same piece, and that triplet-into-a-ring cadence is still on everything."

### Per-image ear prediction (what the operator will hear in the A/B)

| image | prediction |
|---|---|
| **example** | **NEWLY DISTINCT — strongest.** The interior half-cadence is gone; V2 is one long 8-step push to a single arrival. Will clearly read as "this one doesn't keep stopping — it drives." Best win. |
| **AudioHaxImg3** | **NEWLY DISTINCT — clear.** Interior cadence arrives ~2 beats later (beat 5 vs 3); broad phrase + quick codetta is an audibly different periodicity. Reads as "a longer line." |
| **Lena** | **MARGINALLY distinct — at risk.** Interior cadence walks one step (3→4), same gesture. A trained ear *may* catch the off-square period, but Lena will still sound kindred to the {3,7} pieces. Weakest win. |
| **AudioHaxImg2** | **STILL SAME-Y — verbatim {3,7}.** Unchanged skeleton. One of the two the operator hears most. |
| **magicstudio** | **STILL SAME-Y — verbatim {3,7}.** Unchanged skeleton. |
| **AudioHaxImg1** | **Unchanged, but was always the compressed short-section outlier** (no 8-step skeleton to break). Neither newly-distinct nor newly-same. |

Net: ~2 strong + 1 marginal newly-distinct; 2 verbatim-same; 1 pre-existing outlier. Plus the
corpus-wide constant: the same cadential *gesture sound* on every piece.

### Ranked S65 next-lever recommendation

1. **Cadence-GESTURE-TYPE variation (feminine ending / suspension / elided / accel-run), selected per
   scheme or per character.** Highest audibility against the *specific* transcription — it changes the
   *what*, not just the *when*. It subsumes the "get Img2/magic off legacy" goal for free: those two can
   receive a *different gesture* while staying on the F5a/F4-safe {3,7} period. Land it together with the
   `StepPlan` schema field the Test Engineer re-baselines (the exact blocker design §6 named). This is
   the lever that finishes the operator's complaint.
2. **Key register-spread + turn on modulation.** The diagnostic's #2 lever (5/6 roots clustered A3–C#4,
   5/6 pieces never modulate — `docs/diagnostic-s64-fixed-rhythm-key.md` §3). Independent axis, real
   secondary contributor to the "shared key" hearing. Worth doing, but the operator led with rhythm, so
   it ranks below the gesture.
3. **Form spread (open the dead `ternary_aba`/`abac` gates).** LEAST effective against the transcribed
   skeleton — the diagnostic proved `example` escapes to `theme_and_variations` yet *still* realizes the
   period (every section is re-partitioned by `plan_phrases`). Good for macro variety, but it does not
   touch the phrase-level gesture the operator hears. Lowest priority for *this* complaint.

("Get Img1/Img2/magic off legacy" is not a standalone lever — it's blocked as a periodicity move by
F5a/F4, and is best achieved as a *consequence* of lever 1: give the pinned pieces a different cadence
gesture on the same period.)

---

## Appendix — reproduction

```
export PATH="$HOME/.cargo/bin:$PATH"
cargo test --release --test keydiag_s64 -- --nocapture   # per-image scheme / layout / cadence set
```

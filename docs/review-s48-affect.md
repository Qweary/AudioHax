# S48 Slice 3 — Affect / Cross-Modal Review: THE LEVEL FINISH + INVERSE-REGISTER COMP

REVIEW / RECOMMENDATION DOCUMENT (Perceptual / Cross-Modal Affect lens). **No
source, test, or asset file was changed to produce it** — this is the standing
TASTE/AFFECT review voice wired into the S48 build cadence (per the Specialist
Marshaling Gate), sitting BESIDE correctness review, not as an end-of-slice
ear-test. Its job: size the ear-tuned magnitudes of the just-landed level finish,
render the load-bearing perceptual verdict on the inverse-comp design + the held
F4 promotion, and hand the operator a concrete A/B listen-for checklist.
**I reason from cross-modal / auditory-scene-analysis research + the scorecard
data; I cannot hear the WAVs — the operator is the final ear.** Recommendations
are starting A/B anchors, not verdicts.

**Grounded against the live tree (read this session, cited file:line):**
`src/engine.rs` BYTE-FROZEN at sha256
`e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261` (re-verified
UNCHANGED via the seat-guard witness + `scorecard_engine_frozen`). The build
landed exactly to `docs/spec-s48-slice3-build.md`:
- LEVEL/JSON: deep `melody_lead_strong` Melody **0.92** (`mappings.json:375`), mid
  `melody_forward` Melody **0.82** (`:381`), shallow `melody_lead_gentle` Melody
  **0.74** (`:387`) — all at the recommended STARTS.
- LEVEL/CE: the Counter/Fill negative velocity-bias arm landed
  (`chord_engine.rs:1770-1771`), `COUNTER_VEL_BIAS = 2.0` (`:1028`),
  `FILL_VEL_BIAS = 1.0` (`:1037`), both `!is_cadence`-guarded.
- INVERSE-COMP: `inverse_register_compensation(seat)` (`:1174-1187`, LINEAR
  1.0@55 → 0.0@69), the gated `comp` factor (`:1984-1992`,
  `counter_present && prom>0.50 && !is_cadence`), the PRIMARY onset-offset push
  (`:2510-2524`, `COMP_OFFSET_FRAC = 0.25` `:1151`), the SECONDARY articulation
  detach (`:2000-2004`, `COMP_ARTIC_DETACH = 0.10` `:1159`).

Prior lens: `docs/design-s46-figure-ground.md` (my cue-strength ranking
rhythm-grid > onset > register > articulation > LEVEL), `docs/review-s47-affect.md`
(the slice-1/4 affect sizing this continues). Metrics:
`docs/spec-s46-figure-ground-metrics.md` (F1–F5 §1, F4 inverse-comp §1/§2).

**The pinned per-image figure-strength axis** (live routing, re-measured this
session from the scorecard sweep — seed 42):

| Image | `fg_bg_contrast` | counter routed? | prominence tier | figure-strength | F4 corr (measured) |
|---|---|---|---|---|---|
| AudioHaxImg1 | 0.341 | yes | DEEP `melody_lead_strong` | SUBJECT | **−0.536 OK** |
| AudioHaxImg2 | 0.284 | yes | DEEP `melody_lead_strong` | SUBJECT | **+0.462 FAIL** |
| AudioHaxImg3 | 0.203 | yes | MID `melody_forward` | MID | **−0.458 OK** (0 low-seat steps) |
| example.jpg | 0.136 | no | MID `melody_forward` | MID | −0.265 (reported) |
| Lena | 0.052 | no | SHALLOW `melody_lead_gentle` | FIELD | −0.342 (reported) |
| magicstudio | 0.084 | no | SHALLOW `melody_lead_gentle` | FIELD | +0.117 (reported) |

This is the load-bearing table for the whole review. F4 is REPORTED on all six and
the PROMOTION to a sign-asserted gate is HELD (correctly — see §2).

---

## 0. The one finding that frames the whole review (read before the verdicts)

**The comp and the F4 metric disagree about WHERE figure-ground separation is
measured, and the disagreement is REAL, not a bug.** The comp is a *targeted*
tool: it fires ONLY on a melody that is simultaneously (a) low-seated (< 69), (b)
on a DOTTED/SUSTAINED step, (c) over a present counter, (d) foreground. The F4
metric correlates `register_gap` against `separation` across *every* co-sounding
step, high-seat steps included. On the three counter-routed images the melody is
seated HIGH on nearly every step (DEEP routing lifts it to 73–87; the diagnostic
at `tests/variety_scorecard_s45.rs:1702-1711` confirms Img2 has only ONE step
< 69), so the comp acts on ~0–2 steps per counter-routed image — and the F4
correlation those images report is therefore *bed-driven*, a statement about the
seed-driven melody contour vs the Pad/Counter offset pattern on HIGH-seat steps,
not a statement about what the comp did. **The comp barely acts where F4 is
asserted, and acts cleanly where F4 is only reported** (the mid/shallow images that
seat lower). Every verdict below turns on this.

---

## 1. SIZE THE LEVEL FINISH — VERDICT: magnitudes are RIGHT; mid is the one to A/B

**Recommendation: HOLD all three tiers at the landed starts (deep 0.92 / mid 0.82
/ shallow 0.74) and the Counter/Fill biases (−2.0 / −1.0) as the A/B anchor. The
mid bump is the single most-audible knob — A/B it, lower to 0.78 only if over-loud
on `example.jpg`/`AudioHaxImg3`.** Confidence **HIGH** that the SIGNS and
 proportions are correct; **MEDIUM** on the exact mid magnitude (it is the operator's
ear that settles 0.82 vs 0.78).

**The velocity arithmetic, sized against my 90/10.** The melody velocity nudge is
`(w − 0.5) * PROMINENCE_VEL_SPAN(18)` (`chord_engine.rs:1691-1693`). Over the
neutral 0.5 baseline the bumps yield:

| tier | Melody w | melody nudge over neutral | vs the OLD weight | bed bias on top |
|---|---|---|---|---|
| deep | 0.92 | (0.42)·18 = **+7.6 vel** | was 0.90 → +7.2 (Δ +0.4) | Counter −2.0, Fill −1.0 |
| mid | 0.82 | (0.32)·18 = **+5.8 vel** | was 0.78 → +5.0 (Δ +0.7) | Counter −2.0, Fill −1.0 |
| shallow | 0.74 | (0.24)·18 = **+4.3 vel** | was 0.72 → +4.0 (Δ +0.4) | Counter −2.0, Fill −1.0 |

**This respects the 90/10 cleanly, and I confirm it.** A +7.6 / +5.8 / +4.3 vel
lift on a 1–127 MIDI scale is a *few-percent* gain trim — a finishing nudge, not a
figure-maker. The figure-ground work was already done by the slice-1 activity
governor (cue rank 1) + the seat guard (cue rank 3) + the slice-4 Pad recession;
the scorecard proves it (F1 margins +1.0 to +1.25 onsets/step, F3 = 1.000 on every
image, F5b = 0 everywhere). Level here only WIDENS an already-won gap. The
proportion is correct: the deep tier (clear subject) gets the loudest lead, the
shallow tier (field) the gentlest — monotone, image-conditioned, exactly the
arousal→sharper-separation gradient. **Crucially the level finish does NOT carry
the hierarchy** — if you removed it, F1/F3/F5b are untouched (level ≠ onset count ≠
pitch). That is the proof it is the last 10%.

**The Counter/Fill negative bias is the better half of this slice, perceptually.**
The −2.0 counter / −1.0 fill structural bias is more valuable than the melody bump
because it widens the gap from the BED side without pushing the melody toward the
ceiling, AND it gives the counter a level floor BELOW the melody that is
independent of (additive to) the prominence nudge — mirroring the Pad's −3. Sizing
is right: the counter's −2.0 is kept SMALLER than the Pad's −3.0 (the counter is a
MOVING line, must stay audible as a second voice — S45), and the fill's −1.0 is
smaller still (more recessive by role). I confirm both magnitudes. **Watch the
clamp:** on a quiet (low-saturation) image a deeply-recessed counter rides
`round().clamp(1,127)` (`:1695`) — confirm at the A/B that the counter does not
clip toward silence on the quietest render (S45 forbids a silent counter); the
[1,4]/[0.5,2] ranges are sized to stay well above 1 on a normal render, so this is
a confirm-don't-expect-it item.

**On the mid bump specifically (the most-routed knob).** `example.jpg` (ct 0.136)
and `AudioHaxImg3` (ct 0.203) both route MID, so the 0.78→0.82 bump touches the two
most-common renders. The +0.7 vel delta is the largest of the three tiers — it is
the operator's explicit "bump the melody volume," delivered. **HOLD at 0.82 as the
anchor, do NOT silently retreat to 0.78** (that under-delivers the ask); but it is
the one knob to A/B for over-loudness, because it is the most-audible and touches
the most images. If `example.jpg` reads "shouty" or `AudioHaxImg3`'s melody starts
to sit *on top of* rather than *over* its texture, drop mid to 0.78–0.80 and keep
the finish on deep/shallow. Confidence **MEDIUM** here — purely an ear call.

**Per-tier sizing verdict:** deep **HOLD 0.92** (HIGH) · mid **HOLD 0.82, A/B for
over-loud → floor 0.78** (MEDIUM) · shallow **HOLD 0.74** (HIGH) · Counter bias
**HOLD −2.0** (HIGH) · Fill bias **HOLD −1.0** (HIGH).

---

## 2. THE INVERSE-COMP DESIGN + THE F4 FINDING — the load-bearing verdict

**My perceptual answer to the framed question is YES: a high-seated, deep-tier,
clear-subject melody SHOULD get little or no inverse-comp, because a melody already
sitting clearly on top self-projects and does not need non-level separation help.
The comp firing only on LOW-seat steps is therefore the CORRECT behavior, and the
held promotion is the RIGHT outcome. The problem is the F4 METRIC's scope, not the
comp.** Confidence **HIGH** on the perceptual principle; **HIGH** on "do not assert
F4 as-is"; **MEDIUM** on the exact resolution mechanics (a Test-Engineer scoping
call I recommend, below).

### 2.1 Why the high-seat melody correctly gets no help (the principle)

This is straight from the cue-strength ranking I authored (`design-s46-affect.md`
§1) and Bregman frequency-streaming: **register-order is itself a segregation
cue.** A melody seated at 79–87 over a bed ceilinged at 67 is separated by
12–20 semitones — a full octave-plus. The ear streams that melody as a distinct
high voice on register alone; it needs no *additional* non-level help to hold
figure. Spending onset-offset separation + articulation detachment on a melody that
is already a clear octave above the bed would be REDUNDANT at best and, at worst,
would make a melody that is *already* the clear figure sound nervous/over-worked
(more on the jitter risk in §3). The comp's whole design premise — DP-3, "help is
INVERSE to register height" — is the perceptually correct one: **the help goes to
the melody that needs it (low-seated, small register gap, weak self-projection),
and is withheld from the one that doesn't (high-seated, large gap, strong
self-projection).** The helper realizes this exactly: `inverse_register_compensation`
ramps to 0.0 at seat 69 and to 1.0 at seat 55 (`:1174-1187`). That is correct.

So a deep-tier subject image (Img1/Img2), which seats the melody HIGH precisely
because it is a clear subject, *should* get near-zero comp. The comp firing on ~0–2
steps there is not a failure of the comp — it is the comp correctly declining to
"fix" a melody that is already separated by register. **The comp is doing the right
thing on exactly the images where F4 reports it failing.**

### 2.2 Therefore the F4 metric is mis-scoped, not the comp

F4 correlates gap-vs-separation across ALL co-sounding steps. On a deep image with
~1 low-seat step, the correlation is computed over ~35 high-seat steps where the
comp never fired — so F4 is measuring the BED's offset pattern against the
seed-driven melody contour, NOT the comp's effect. The diagnostic confirms this
verbatim (`tests/variety_scorecard_s45.rs:1702-1711`): Img2's positive +0.462 is
because its high-gap steps happen to coincide with split-bed-offset steps. **That
is a seed/bed artifact dressed up as a comp measurement.** Asserting `F4 < 0` on
Img2 would red-bar a real deterministic number that the comp does not own — it
would be asserting a bed-driven signal. Cherry-picking only Img1/Img3 (which happen
to land negative) would assert the *same* bed-driven signal where it happens to
agree. Both are dishonest. **Holding the promotion is the only honest call**, and
the Test Engineer made the right one.

### 2.3 The cleanest honest resolution — SCOPE THE METRIC (my single most important output)

**Recommendation: SCOPE F4 to the comp-eligible (low-seat DOTTED/SUSTAINED) steps,
then re-evaluate the sign promotion on that scoped correlation. Do NOT widen where
the comp fires. Do NOT accept the all-steps F4 and assert it.** Confidence **HIGH**
that scope-the-metric is the right family of fix; **MEDIUM** on whether the scoped
sample is large enough to assert at all on the DEEP images.

The reasoning, against the three options the build handed me:

- **SCOPE-THE-METRIC (RECOMMENDED).** F4's job is to measure *the comp's effect*.
  The comp only acts on low-seat DOTTED/SUSTAINED steps. So F4 should correlate
  gap-vs-separation *over those steps* — where a low seat genuinely does earn more
  separation (the comp pushes the onset off the downbeat → higher sep). Scoped that
  way, the correlation measures what the comp does, and the bed-driven high-seat
  noise drops out. This is the honest instrument. The one caveat — and why my
  confidence on *asserting* is only MEDIUM — is sample size: a DEEP image with ~1
  low-seat step has too few samples to correlate (the spec §9 risk #1 anticipated
  exactly this: "a deep-tier image that seats the melody above 69 on every step has
  no low-seat samples for the comp to act on"). So the scoped metric should ASSERT
  only where the comp-eligible sample is large enough (the lower-seating MID/SHALLOW
  images — `example`, `AudioHaxImg3`, `Lena` — which is exactly where the comp
  cleanly fires and F4 is already negative), and REPORT-N/A where there are too few
  low-seat steps (the DEEP images). That is the same `N/A — counter not routed`
  honesty F1/F2 already use for unmeasurable cells (`spec-s46...metrics.md` §0.3).

- **WIDEN-THE-COMP (REJECTED, perceptually).** Widening the comp to fire on
  high-seat steps would force separation onto a melody that is already an octave
  clear of the bed — over-helping the figure that least needs it, risking the
  nervous/over-worked sound. It would also re-fuse risk: pushing a high-seat
  DOTTED/SUSTAINED onset to `step_ms/4` on a SUBJECT image gains nothing
  perceptually (register already separates) while adding offset churn. This trades a
  metric-pass for a perceptual regression. **No.** The whole DP-3 premise is that
  help is INVERSE to seat — widening the comp to high seats violates the principle
  the comp exists to encode.

- **ACCEPT-COMP-WHERE-IT-FIRES + KEEP-F4-REPORTED (acceptable fallback).** If
  scoping the metric is more than the Test Engineer wants to take on this slice,
  keeping F4 REPORTED (as landed) is *honest and shippable* — the values are
  visible, the regression stays bracketed by F5b=0, and the comp demonstrably acts
  where it should (the negative corrs on the lower-seating images). This is strictly
  better than asserting a bed-driven gate. I rank it second only because the scoped
  metric would actually *measure the comp* and give the operator a real before/after
  instrument for the magnitude tuning; the all-steps reported value does not.

**Net F4 verdict:** the comp design is perceptually CORRECT (help is inverse to
seat; high-seat melodies self-project and need no help). The held promotion is the
RIGHT call. The cleanest honest resolution is to SCOPE F4 to comp-eligible low-seat
steps and assert only where the sample supports it (the MID/SHALLOW images, where
the comp cleanly fires) — REPORT-N/A on the DEEP images that have no low-seat
samples. The fallback (keep F4 reported as-is) is acceptable. Widening the comp is
the one option to reject.

---

## 3. COMP CURVE MAGNITUDES — VERDICT: both well-sized; keep as anchors

**Recommendation: HOLD `COMP_OFFSET_FRAC = 0.25` and `COMP_ARTIC_DETACH = 0.10`.**
Confidence **MEDIUM-HIGH** (these are exactly the knobs the operator's ear settles,
but the perceptual sizing argument is strong).

**`COMP_OFFSET_FRAC = 0.25` (push the first onset to ≤ step_ms/4 at full comp) — POP
without jitter, HOLD.** At full comp a low-seated melody attacks on the "and" of
the beat — the same metrically-stable off-beat the counter's MOVING mode uses
(`:2256`) and the same family as the Pad's `step_ms/2` displacement, all distinct
from each other. step_ms/4 is the cleanest single off-beat: it reads as a
deliberate anticipation, not a random syncopation that would itself recapture
attention (`design-s46-affect.md` §2.3 — irregular onsets capture attention; we
want the *figure* to carry deliberate syncopation, never to sound *nervous*).
Critically the comp is COUNT-PRESERVING (`:2506-2509` moves WHERE the onset sits,
never HOW MANY) and the push SCALES with comp — so it ramps in smoothly from 0 as
the seat drops, never a step-function lurch. A low-seated melody POPS via the
distinct attack phase; it does not jitter, because (a) only the FIRST onset moves,
(b) it is bounded ≤ step_ms/4 so it never crosses the beat, (c) it is confined to
DOTTED/SUSTAINED bands where the melody was *on* the downbeat (the ARPEGGIO/
SYNCOPATED bands — already off-beat, already `Subdividing` — are correctly excluded,
`:2510 pushable` gate). The "nervous/jittery" failure mode would come from pushing
*every* onset or from an unbounded push; neither is present. **HOLD 0.25.** If the
ear ever finds the "and" too anticipatory on the lowest-seat passage, the range
[0.125, 0.375] lets it ease toward 0.125 — but 0.25 is the recession-correct anchor.

**`COMP_ARTIC_DETACH = 0.10` (shorten base_frac by ≤ 0.10 at full comp) — gentle and
correct, HOLD.** A 0.10 reduction off `base_frac`, floored at `ARTIC_WINDOW_LO(0.55)`
(`:2001`), is a *subtle* crispening — DP-3 rank 4, correctly the SECONDARY tool, so
it should be gentle. It gives a low-seated melody slightly more detached, separated
notes (articulation segregation the ear hears even though F4's onset metric does not
read it). The floor at 0.55 guarantees it never clicks/staccatos into harshness. The
sign is right (more detached when low), the magnitude is appropriately small (it is
the support act, not the lead). **HOLD 0.10.** It is the right "ear hears it, metric
doesn't" complement to the offset push.

**One interaction to note (not a change):** on a full-comp low-seat step BOTH tools
fire — the onset pushes to step_ms/4 AND the note shortens by 0.10. That is the
intended stacked separation (rhythmic FIRST + articulation SECOND, DP-3 order). The
ear should read it as "the low melody got crisper and more syncopated to pop out" —
confirm at the A/B that the *combination* on the lowest-seat passage reads as
deliberate POP, not as the melody suddenly sounding clipped/anxious. Given both are
bounded and only the first onset moves, the math says it is safe; the ear confirms.

---

## 4. THE A/B WATCH-ITEMS (carried from S47) — what to listen FOR

Render at `--seed 42`, BEFORE vs AFTER, on the named images. The S47 A/B WAVs are
already staged at `../ab-s47-wavs/` (BEFORE_/AFTER_ pairs) — re-render for S48 and
A/B against those. Priority order:

1. **Deep-bed thinness on busy HELD passages (S47 knob-2 carry).** The level finish
   does NOT touch onset counts (it is velocity-only), so it neither causes nor cures
   the deep-bed thinness — but the louder melody can make a thin bed *read* thinner
   by contrast. **LISTEN on AudioHaxImg1/2 (DEEP), calmest section:** is there still
   an audible harmonic cushion under the now-louder melody, or does the bed read as
   isolated stabs? If hollow → the fix is bed-side (lower `PAD_DEEP_RECESSION_CEILING`,
   an S47 carry), NOT the level finish. (Confidence MEDIUM — the §0/S47 finding that a
   capped block bed is still a chord mitigates this, but the louder melody is a new
   contrast the ear must re-judge.)

2. **The Img3 mid-tier counter blur — does the level finish RESOLVE it?** This is
   the key S48 question. `AudioHaxImg3` (ct 0.203) routes MID with a MOVING counter
   and only the 0.20 melody-counter prominence gap — the thinnest-margin competition
   on the set (S47 knob 4). The S48 build adds TWO gap-wideners on top: the −2.0
   counter velocity bias AND the +0.7 mid melody bump. Together they widen the
   *effective* mel-counter gap meaningfully (the melody is now +5.8 vel and the
   counter −2.0, a ~7.8 vel structural separation on top of the prominence nudge).
   **LISTEN on AudioHaxImg3:** can you now clearly track the melody as THE figure
   over its moving counter, or do the two still blur into one wandering duet? If the
   level finish RESOLVES the blur → no re-route needed, hold the deep gate at 0.25
   (the cheaper fix won). If it still blurs → lower the DEEP gate (`mappings.json`
   `fg_bg_contrast ge 0.25` → 0.20) so Img3 routes DEEP and gets the 0.45 gap +
   `melody_lead_strong`. **Try the level fix FIRST — it already landed; the re-route
   is the fallback.** (Confidence MEDIUM-HIGH that the combined gap-wideners resolve
   it; the math says the effective gap roughly doubled, but it is an ear call.)

3. **Did the comp let the LOW melody hold figure WITHOUT loudness?** The reason the
   comp exists. **LISTEN on the lower-seating images where the comp actually fires
   (`example`, `AudioHaxImg3`, `Lena` — the negative-F4 images):** on a calm/dark
   passage where the melody sits low, does it POP out via its off-beat attack +
   crisper articulation, *without* sounding louder? (The comp NEVER touches level —
   DP-3.) If the low melody holds figure → the comp works. If it still buries → the
   comp magnitudes want raising (offset toward 0.375, detach toward 0.20).

4. **S45 intact + counter not clipped (the level recession floor).** On an active
   subject passage the counter must still MOVE (S45), and on the quietest render the
   −2.0 bias must not clip the counter toward silence (`:1695` clamp). **LISTEN:**
   the inner counter line is audibly present and moving, just *under* the melody —
   receded, never silenced.

**Fusion alarms (any = stop, fix activity/register/recession, NOT level):** melody
+ counter blur into one line (Img3, item 2); the now-louder melody makes the bed
read hollow (item 1); the low-seat comp makes the melody sound clipped/nervous
rather than crisp-and-popping (item 3 / §3).

---

## 5. AFFECT VERDICT

**Perceptually SOUND — ship to the A/B as built; the held F4 promotion is correct
and the cleanest honest fix is to SCOPE the metric, not widen the comp.** The slice
is exemplary 90/10 discipline: the level finish is a few-percent velocity trim that
WIDENS an already-won gap (F1/F3/F5b prove the hierarchy was already there), and the
better half — the −2.0 counter / −1.0 fill structural bias — recedes the bed from
its own side without crowding the melody toward the ceiling. The inverse-comp is the
perceptually RIGHT design: help is inverse to register height, so a high-seated
clear-subject melody correctly gets near-zero help (it self-projects on register
alone) and a low-seated melody gets the offset-push + articulation-detach it needs
to hold figure WITHOUT loudness (DP-3 honored — level is never a comp tool). The F4
"failure" on Img2 is a metric-scope artifact, not a comp defect: the comp acts on
~0–2 low-seat steps there, so the all-steps correlation is bed-driven; holding the
promotion is honest and asserting it would red-bar a bed signal the comp does not
own. The comp curve magnitudes (offset 0.25, detach 0.10) are well-sized for POP
without jitter — bounded, count-preserving, first-onset-only, ramped-not-stepped.
`engine.rs` byte-frozen throughout. The single biggest open ear-call is the mid-tier
0.82 bump (most-routed, most-audible — A/B for over-loud); the single most important
*process* recommendation is to scope F4 to comp-eligible steps so it actually
measures the comp.

---

## RETURN TO THE LEAD (raw substance)

**F4 RECOMMENDATION (the single most important output):** the held promotion is
CORRECT — do NOT assert F4 as-is. The comp is perceptually right (high-seated
clear-subject melodies self-project on register and correctly get near-zero help;
the comp acts only where the melody is low and genuinely needs non-level
separation), so the +0.462 on Img2 is a metric-SCOPE artifact (F4 correlates across
all steps but the comp fired on ~0–2 of them; the rest is bed-driven), NOT a comp
defect. Cleanest honest fix: **SCOPE F4 to the comp-eligible low-seat DOTTED/
SUSTAINED steps and assert the negative sign only where that sample is large enough
— i.e. on the lower-seating MID/SHALLOW images (`example`/`AudioHaxImg3`/`Lena`,
already negative), and REPORT-N/A on the DEEP images that have no low-seat samples**
(the same N/A honesty F1/F2 use). REJECT widening the comp to high seats (it would
over-help the figure that least needs it and risks a nervous sound). Keeping F4
reported as-is (as landed) is an acceptable fallback — strictly better than
asserting a bed-driven gate.

**LEVEL-SIZING VERDICT:** HOLD all three tiers at the landed starts — deep 0.92
(+7.6 vel over neutral) and shallow 0.74 (+4.3) are HIGH-confidence keeps; mid 0.82
(+5.8) is the most-routed/most-audible knob — HOLD it as the anchor (it delivers
the operator's "bump the volume" and must NOT silently retreat to 0.78) but A/B it
for over-loudness on `example.jpg`/`AudioHaxImg3` and floor to 0.78 only if the ear
calls it shouty. The Counter −2.0 / Fill −1.0 structural biases are correctly sized
(counter < Pad's −3 because it is a moving line that must stay audible — S45; fill <
counter by role) — HOLD both, confirm at the A/B that the counter does not clip
toward silence on the quietest render. The comp curve magnitudes (offset 0.25,
detach 0.10) are well-sized for POP-without-jitter — HOLD. The whole slice respects
the 90/10: it widens an already-won gap, it does not carry the figure-ground.

---

*End of S48 affect review. Review/design-only: no source, test, or asset modified.
`src/engine.rs` sha256 re-verified UNCHANGED:
`e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261`.*

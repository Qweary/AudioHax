# S48 Slice 3 Aesthetics Review — The Level Finish + Inverse-Register Comp: Does the Figure-Ground Arc CLOSE Without Breaking?

**Author role:** Composition & Songwriting Aesthetics Specialist (the standing TASTE/AFFECT review
voice wired into the S48 build cadence per the Specialist Marshaling Gate — the same gate that ran
beside correctness in S47).
**REVIEW / DESIGN ONLY — no `src/*`, `tests/*`, or `assets/*` modified by this document; `docs/`
only.** I reason from arrangement craft + the build contract + the measured scorecard + the live
code I re-read this session. I CANNOT hear the WAVs — every verdict below names a concrete
LISTEN-FOR so the operator's trained ear (trombone — a single-line lead instrument, the most
figure-ground-literate ear there is) is the final gate.
**Date:** 2026-06-19.
**Grounds:** the binding 90/10 frame (`docs/design-s46-figure-ground.md` §5, §6 — *"differentiation
did the first 90%, level is the last 10% that widens the already-won gap"*), the S48 build contract
(`docs/spec-s48-slice3-build.md` §2 / §3 / §6 / §8), the S47 hierarchy that's already landed
(`docs/spec-s47-slice1-build.md`), my own S47 review continuity (`docs/review-s47-aesthetics.md` —
the deep-bed-thinness watch-item, the image-conditioned spread as the soul of the build), and the
LIVE code re-read this session: the F4 metric computation (`tests/variety_scorecard_s45.rs:1188-1238`,
`f4_corr = correlation(register_gap, separation)` over ALL co-sounding steps, `f4_tag = corr<0 ? OK :
FAIL`, SEEDED), the live prominence routing (`assets/mappings.json:393-407` — deep gate
`fg_bg_contrast ge 0.25 → melody_lead_strong`), and the deep-tier Melody weight 0.90 (now 0.92).

**The finding I am here to weigh (NOT rubber-stamp).** The Test Engineer HELD the F4 sign-promotion.
All three counter-routed images route DEEP (`melody_lead_strong` → melody seats HIGH ≥69), so the
inverse-comp — which gates on a LOW seat — fires on only 0–2 steps there. F4-across-all-steps then
measures the *bed's* offset pattern (the Pad weak-beat displacement, the counter MOVING onsets), not
the comp. Per-image F4 corr: Img1 −0.536 / Img2 +0.462 (fails the sign) / Img3 −0.458 (but comp
fired on ZERO steps) / example −0.265 / Lena −0.342 / magicstudio +0.117. The comp acts cleanly on
the lower-seated mid/shallow images and barely-or-never on the deep ones. **The question is whether
that is a metric-scoping problem, a comp-coverage problem, or correct behavior mislabeled as a
problem.** My answer, argued below, is the third — with the cleanest honest resolution being to
SCOPE the F4 metric to the comp-eligible steps and KEEP the deep images out of the high-seat numerator.

---

## The headline (read this first)

**The S48 finish is PROPORTIONATE and the deep-routing behavior is AESTHETICALLY CORRECT, not a
defect.** Two verdicts, one sentence each:

1. **Level finish: KEEP, it is a true 10% finish, not a "turn it up."** A +5.8-velocity mid melody
   bump (0.82) and a −2 counter / −1 fill recession is small, count-neutral, and pitch-neutral — it
   widens a gap that activity + register already won. Remove the bump and the melody is *still* the
   figure (it still out-moves and out-sits everything). That is the litmus the arc demanded, and the
   build passes it.

2. **The deep/F4 question: the comp idling on deep images is the RIGHT behavior, and F4 should be
   scoped to where the comp can act.** A clear-subject image *should* seat an unambiguous lead that
   needs no propping; a field/ambiguous image is exactly where a lower-seated lead must WORK to stay
   in front — which is where separation help earns its keep. The comp firing on low-seat images and
   idling on high-seat ones is the aesthetic working as designed. **F4 measured across all steps on a
   deep image is measuring a question the comp was never asked** (it reads the bed's offsets, not the
   comp). My single clearest recommendation: **SCOPE-THE-F4-METRIC-TO-COMP-ELIGIBLE-STEPS** — assert
   the negative-correlation sign only over the steps where the melody is actually low-seated (the
   DOTTED/SUSTAINED, comp-firing steps), and REPORT (don't sign-assert) the all-steps corr beside it.
   Do NOT widen the comp onto deep images to satisfy a metric — that would prop a lead that does not
   need propping, which is the same flat-maximum mechanical defect S46 spent the whole arc killing.

Everything else (the Img3 blur, the watch-items) is minor and falls out of these two calls.

---

## Point 1 — DOES THE LEVEL FINISH COMPLETE THE 90/10 WITHOUT UNDOING IT?

**Verdict: YES — KEEP the level finish at the spec'd starts (deep 0.92 / mid 0.82 / shallow 0.74,
`COUNTER_VEL_BIAS = 2.0`, `FILL_VEL_BIAS = 1.0`). It is a proportionate FINISH, not an over-reliance
on level. No re-sizing required; one flag on the mid bump as the single most-audible knob.**

### Why this is a finish and not a "turn it up"

The arc's whole discipline — the lead's binding counterpoint and my S46 lens both — is that level is
the *one cue the engine already balanced* (S42/S43) and the *weakest segregation cue* (Affect ranks
it 5th of 5 on this timbre-flat synth). A fix that LEADS with level is the operator's confirmed trap:
a loud melody over an equally-busy bed is still mush. The test of whether this build respects that is
simple and I can apply it from the contract without hearing a note:

**The removal test (the litmus).** *If you deleted the entire 2(a) level lever — held every Melody
weight at its S47 value and dropped both negative velocity biases — would the melody still be the
figure?* Yes, unambiguously. The figure-ground is carried by THREE structural cues that all landed in
S47/Slice-4 and that 2(a) does not touch:
- **Activity** — the governor (`melody_activity_class` + the counter recession) and the melody
  activity floor make the melody the most-active line; F1 margin is positive everywhere, F5b is 0.
- **Register** — the seat guard makes `melody_seat ≥ COUNTER_CEILING + MIN_FIGURE_GAP` structural;
  F3 → 1.0.
- **Bed activity recession** — the Pad onset cap + weak-beat displacement (S47/Slice-4).

The level lever rides ON TOP of all three. The spec confirms it is **count-neutral** (no F1/F5b
impact — level ≠ onset count) and **pitch-neutral** (no F3 impact — it never moves a seat). So the
level bump cannot, even in principle, be carrying the figure-ground — it is mathematically incapable
of touching the cues that do. That is the structural proof that this is the 10%, not the 90%. The
build did NOT quietly let level become load-bearing.

### Is the MAGNITUDE proportionate as a finish?

Yes, and here is the arithmetic the ear should confirm. The melody velocity nudge is
`(w − 0.5) * PROMINENCE_VEL_SPAN(18)`:
- **Mid tier 0.78 → 0.82:** the nudge goes from `(0.28)*18 = +5.04` to `(0.32)*18 = +5.76` — a **+0.72
  velocity** delta on the melody. The counter additionally drops −2.0 (structural), the fill −1.0. So
  the melody-vs-counter LEVEL gap widens by ≈ **+2.7 velocity** (0.72 melody-up + 2.0 counter-down).
- **Deep tier 0.90 → 0.92:** melody nudge `+7.2 → +7.56`, a **+0.36** melody delta, plus the −2
  counter. Deep already had a 0.45 weight gap to the counter; the bump barely moves it.

A ≈3-velocity widening on a 1–127 scale is a *whisper* of a finish — exactly the size a "last 10%"
should be. This is NOT a 10-to-20-velocity shout. It is the kind of gap a mixing engineer rides in by
a hair to "seat" a lead that's already arranged on top. **Proportionate. Do not grow it.** The risk,
if any, is the opposite — that it is too SUBTLE to hear as a finish at all, which is fine: a finish
you can barely hear over an already-won gap is the correct outcome (it means the differentiation did
the work). The −2 counter recession is the more audible half and the more valuable half — it RECEDES
the bed rather than just lifting the lead, which is the better figure-ground move (lowering the ground
is as good as raising the figure and does not risk a shouting lead).

### The one re-sizing flag (not a change — a watch)

The **mid bump (0.78 → 0.82) touches the most-routed image class** (every image not gated
deep/shallow — `example.jpg`, `AudioHaxImg3`). It is the single most-audible knob in the slice and
the spec flags it correctly: ship it (the operator explicitly asked to bump the melody — holding mid
at 0.78 "to be safe" under-delivers), but A/B it. My prior: +5.76 vs +5.04 melody velocity is small
enough that it will read as "the tune sits a touch clearer," not "the tune got loud." Keep it. If the
ear hears the mid melody as *shouting* on `example.jpg`/`Img3`, the fix is to hold mid at 0.78 and let
the −2 counter recession carry the mid finish alone (recede the ground, don't lift the figure) — but
I expect 0.82 is fine.

**Counter never goes silent (S45 preserved):** `vel −= 2.0` floored by `round().clamp(1,127)` cannot
mute a normally-rendered counter; the counter still MOVES (the governor's MOVING/oblique modes are
untouched). This is a level recession, not a re-suppression — the forbidden `pad_bed_counter`
de-route is nowhere near this slice. Confirmed clean.

**LISTEN-FOR (point 1):** A/B with vs without the 2(a) lever on a deep image and on a mid image. (a)
Does the melody sit a hair *more clearly on top* — or does it sound *louder/pushed*? It should be the
former (a seat, not a shout). (b) With the lever ON, does removing it (mentally) leave the melody
*still obviously the figure*? It must — if the melody dissolves into the bed without the level bump,
the differentiation did NOT do its job and the bump is masking it (it didn't, per the measured F1/F3/F5b,
but the ear confirms). (c) Did the counter stay a present, moving second voice after −2, or did it
duck under to inaudible? It must stay a line (S45).

---

## Point 2 — THE DEEP-ROUTING / F4 QUESTION (the load-bearing aesthetic call)

**Verdict: it is AESTHETICALLY CORRECT that a clear-subject image needs no inverse-register
separation help while a field/ambiguous image is exactly where it earns its keep. The comp firing on
low-seat images and idling on deep ones is CORRECT behavior. F4 measured across all steps on a deep
image is measuring the wrong thing. My single clearest recommendation:
SCOPE-THE-F4-METRIC-TO-COMP-ELIGIBLE-STEPS — sign-assert the negative correlation only over the
low-seat (comp-firing) steps; REPORT the all-steps corr beside it; do NOT widen the comp onto deep
images.**

### The aesthetic argument, run through

This is the most important call in the review, so I will argue it from arrangement first principles,
not from the metric.

**What is a "clear subject" image, musically?** The deep tier routes on `fg_bg_contrast ≥ 0.25` — a
sharply separated subject against its ground. The image is telling the engine *"there is one thing to
look at, and it is unmistakably in front of everything else."* The correct musical translation of
that is **an unambiguous lead** — a melody that sits clearly on top (high seat, strong level gap, the
counter well under it). On such an image the lead does NOT need to *work* to be heard as the figure;
the image's own clarity has already done the figure-ground separation, and the arrangement just
honors it. A lead that's already clearly on top getting *additional* separation help (pushed off the
downbeat, crisped, detached) is **propping a lead that needs no propping** — and that is precisely the
flat-maximum mechanical defect my S46 lens named as the inverse-error of the no-figure defect. A
subject so clear it leads itself, then *also* getting fussed-over separation, would sound like the
engine doesn't trust its own subject. The comp correctly stays its hand.

**What is a "field/ambiguous" image, musically?** Low `fg_bg_contrast` — even energy, no dominant
subject, the texture genuinely shares focus (the shallow tier; the mid tier sits between). The
melody here seats LOWER, in a more even texture, closer to the inner voices. *This* is where a lead
is in danger of being lost in the wash — where there is no image-given clarity to lean on, so the
melody must EARN its figure through its own behavior: by attacking off the bed's downbeat (the
primary comp tool — onset-offset push) and by crisper, more detached articulation (the secondary).
**This is the exact case the inverse-register comp was built for.** A lower-seated melody, in a more
even texture, propelled to the front by separation rather than by loudness — that is operator signal
4, the subtle one, working as designed.

So the design law is: **a strong subject = an unambiguous lead that doesn't need propping; a field
image = a lead that must work to stay in front.** And the comp's gate (`inverse_register_compensation`
ramping `1.0 → 0.0` as the seat rises from `FILL_REGISTER_FLOOR(55)` to `COUNTER_CEILING +
MIN_FIGURE_GAP`) encodes *exactly that law* — help inversely proportional to how clearly the melody
already sits on top. **The comp is not under-firing on deep images. It is correctly declining to prop
a lead that the image already made unmistakable.** This is the same image-conditioned, non-rigid
discipline that made the S47 level spread "the soul of the build" — the comp, too, reads the image and
acts only where the image leaves the lead something to prove.

### Why the F4 metric, as currently scoped, MIS-MEASURES this

I re-read the live F4 computation (`tests/variety_scorecard_s45.rs:1188-1238`). `f4_corr =
correlation(f4_gaps, f4_seps)` over EVERY co-sounding step, where `gap = mel_pitch − max(bed_pitch)`
and `sep = fraction of bed roles whose onset offset ≠ the melody's first onset offset`. The comp only
modulates the melody's onset offset on the DOTTED/SUSTAINED (low-seat) steps. On a deep-routed image
where the melody seats ≥69 on (nearly) every step:
- there are 0–2 low-seat steps, so the comp barely or never moves a melody offset;
- the `sep` variation that DOES exist across the deep image's steps is driven by the **bed's** offset
  pattern — the Pad weak-beat displacement (`recede_pad_onsets` → `step_ms/2`), the counter's MOVING
  onset (`step_ms/4`) — none of which the comp authored;
- so `correlation(gap, sep)` on a deep image is a correlation between the melody's register gap and
  the *bed's* offset behavior. **It is not measuring the comp at all.** Img2's +0.462 and
  magicstudio's +0.117 "failures" are the bed's offset pattern happening to correlate positively with
  the (uniformly high) register gap — an artifact of measuring an instrument on steps where it does
  not play. Img3's −0.458 "pass" with the comp firing on ZERO steps is the same artifact with the
  opposite sign: a coincidence, not a comp success. Both the pass and the fail are noise on deep
  images.

A metric that reads an instrument across steps where the instrument is silent, then sign-asserts on
the result, will produce exactly this scatter (−0.536 / +0.462 / −0.458 / −0.265 / −0.342 / +0.117).
**The Test Engineer was right to HOLD the promotion.** Promoting F4-across-all-steps to a hard sign
gate would red-bar a *correct* engine for not doing something it correctly declines to do — and worse,
it would create pressure to "fix" the metric failure by widening the comp onto deep images, which is
the aesthetic regression.

### The three resolutions, weighed

1. **Scope-the-F4-metric-to-comp-eligible-steps (RECOMMENDED).** Compute the sign-asserted correlation
   only over the steps where the comp can act — the low-seat steps (operationally: the steps where
   `register_gap` is small, i.e. `mel_pitch − max(bed_pitch) ≤` the comp's zero-crossing band, which
   is the same `[FILL_REGISTER_FLOOR, COUNTER_CEILING + MIN_FIGURE_GAP]` band the helper ramps over).
   On those steps the comp genuinely modulates the melody offset, so `correlation(gap, sep)` measures
   the comp and SHOULD go negative — that is a clean, honest sign assertion. Report the all-steps corr
   beside it as a diagnostic (un-asserted), so the operator still sees the global number. **This is
   the cleanest honest resolution: it asserts the metric exactly where the property it names exists,
   and it does not pressure the comp to act where it shouldn't.** It is also faithful to the comp's
   own design — the helper's domain IS the low-seat band, so the metric's assertion domain should match
   the tool's action domain. *(Mechanically light: the Test Engineer already has per-step `gap` in hand
   at `:1226`; gate the push into `f4_gaps`/`f4_seps` on the low-seat condition. Magnitude stays
   ear-tuned; sign-only asserted, per the spec's promotion discipline.)*

2. **Widen-the-comp-onto-deep-images (REJECT).** Make the comp fire on deep images too, so F4 has
   low-seat samples there. This is the aesthetic regression: it props a lead the image already made
   unmistakable, re-introduces the flat-maximum (every image gets max separation) my whole S46 lens
   exists to prevent, and risks pushing a *clearly-on-top* melody off its downbeat for no reason — a
   lead that sounds nervously over-separated. **Do not do this.** Satisfying a mis-scoped metric by
   degrading the arrangement is the tail wagging the dog.

3. **Accept-and-keep-F4-reported (acceptable fallback).** Leave F4 reported-not-asserted as it is
   today. This is HONEST (it doesn't false-fail a correct engine) but it leaves the inverse-comp
   property *unverified by any gate* — the arc closes without a correctness witness on its last
   structural tool. Worse than (1) because (1) gives a real, scoped sign gate at no aesthetic cost.
   Acceptable only if scoping proves to have too few low-seat samples across the 6-image set to
   correlate stably — in which case report-only is the honest floor, and the operator's ear is the
   sole gate on the comp.

**My recommendation: (1), with (3) as the documented fallback if the scoped sample is too thin.**
Resolution (1) puts the assertion exactly where the aesthetic says the comp should act, and the
deep-image "failures" simply leave the comp-eligible numerator (correctly — the comp didn't fire
there, so there is nothing to assert). The mid/shallow images (`example.jpg`, `Lena`, and the
shallow-routed ones) are where the scoped corr lives, and those are exactly the images where the comp
acts cleanly (the finding confirms Img1 −0.536, example −0.265, Lena −0.342 — all correctly negative
where the melody is lower-seated).

**LISTEN-FOR (point 2):** On a mid/shallow (field/even) image with a lower-seated melody: does the
melody pop to the FRONT *without getting louder* — i.e. can you hear it leading by virtue of attacking
off the beat and being crisper, rather than by volume? That is the comp working. Then on a deep
(clear-subject) image: does the lead sit cleanly on top *with no nervous off-beat fidget* — a melody
that's simply, unmistakably in front? That is the comp correctly NOT firing. If the deep-image lead
sounds fussy/over-separated, the comp is leaking onto high-seat steps (check the helper's zero-crossing);
if the field-image lead still gets lost in the wash, the comp is under-sized (raise `COMP_OFFSET_FRAC`
toward 0.375 / `COMP_ARTIC_DETACH` toward 0.20 — magnitude only, never level).

---

## Point 3 — THE S47 WATCH-ITEMS (carried)

### Img3 mid-tier counter blur — **the level finish should be tried FIRST; Img3 does NOT obviously want DEEP**

**Verdict: try the 2(a).ii −2 counter velocity bias + the 0.82 mid melody bump (and optionally DP-6
0.58→0.55) as the FIRST resolution of the Img3 blur — they widen the mid mel−counter gap from the
level side without re-routing. Hold the deep-gate move (0.25 → 0.20) unless the ear still hears the
melody blur into the counter after the level finish. Img3 does not have a strong aesthetic claim to
DEEP, and re-routing it would silence the comp there as a side effect.**

The S47 review and the affect review both flagged it: `AudioHaxImg3` (`fg_bg_contrast 0.203`) routes
MID and gets only the 0.20 mel−counter weight gap with a MOVING counter — the perceptual mis-bin where
the melody and the moving inner line risk blurring. S48 gives three level-side widening tools for the
mid tier, all of which attack the blur without re-routing:
- the **mid melody bump 0.78 → 0.82** (melody up ≈ +0.72 vel);
- the **−2 counter velocity bias** (counter down −2 vel) — together a ≈ +2.7 vel widening of the
  mid mel−counter gap;
- optionally **DP-6 0.58 → 0.55** (counter weight down, ≈ another −0.5 vel).

That is a meaningful level separation added to a pair that previously had only the 0.20 prominence
gap and the (now activity-recessed, S47) counter. **The cheaper, more reversible fix is the level
finish, and it should be tried first** — exactly as the spec's watch-item §8.3 says.

**Why I do NOT recommend routing Img3 DEEP by default.** Three reasons, in aesthetic order:
1. `fg_bg_contrast 0.203` is genuinely MID — it is not a clear separated subject (deep is ≥0.25 for a
   reason). Forcing it deep would give it the strong-lead treatment (Melody 0.92, counter 0.45, deep
   bed recession) an image without a dominant subject does not earn — the over-led inverse-error of my
   S46 lens. The blur is better cured by *separating the existing mid pair* than by *promoting the
   image to a tier it doesn't fit*.
2. **Routing Img3 deep would silence the comp there** (the prompt flags this precisely). At a deep
   seat the melody sits ≥69, `inverse_register_compensation → 0`, and the comp stops firing — so the
   one image where a lower-seated melody could benefit from separation help loses it. The mid routing
   *keeps Img3 in the comp's domain*, which is the right place for a melody that has to work to stay
   in front. Re-routing trades a curable blur for a lost tool.
3. The deep-gate move is a global threshold change (0.25 → 0.20) that re-bins *every* image near the
   boundary, not just Img3 — a blunt instrument for a single-image blur. The level finish is targeted.

**The MIN_FIGURE_GAP 2→1 / comp zero-crossing coupling — frame what to listen for.** The spec
correctly notes (§8.1) that `inverse_register_compensation` ramps to 0.0 at `COUNTER_CEILING +
MIN_FIGURE_GAP`, so if the taste gate drops `MIN_FIGURE_GAP` 2→1 (to soften a jarring seat-lift on a
dark counter image), **the comp's high-seat zero-crossing moves DOWN with it, 1:1, for free** — the
comp would then start firing one semitone lower, i.e. on slightly-higher-seated steps. This is a
*coupling to be aware of, not a problem*: lowering the gap makes the lead sit one semitone closer to
the counter ceiling AND makes the comp slightly more eager. If the operator moves `MIN_FIGURE_GAP`,
they are simultaneously (a) softening the seat-lift leap and (b) widening the comp's firing band — both
in the direction of "a more even texture," which is coherent. Listen for: after a 2→1 drop, does the
dark-image melody still read clearly on top (the seat is now only 1 semitone above the counter
ceiling — a tighter but still-present margin), AND did the comp start fidgeting the melody on steps
where it previously sat clearly (the zero-crossing moved)? If the lead loses its clear-top read at gap
1, keep it at 2. My prior (carried from S47 knob 4): **keep `MIN_FIGURE_GAP = 2`** — the seat-lift is
the lead arriving on top, not a contour break, and 2 semitones is the textbook clear-seat margin. Only
move it if a *specific* dark image lurches audibly.

**Deep-bed thinness (S47 carry).** Carried from my S47 review as a listen-item: a ~1-onset deep bed
under a ~2-onset held melody is a *correct gap over a sparse texture*. The S48 level bump does NOT
touch onset counts (count-neutral), so it neither causes nor cures this — it is orthogonal to slice 3.
But note one interaction: the deep tier's melody just got *slightly louder* (0.90 → 0.92) over that
same sparse bed, which very marginally increases the exposed-lead feel on a busy-subject held passage.
The delta is +0.36 velocity — negligible — so I do not expect it to tip the watch-item, but it is the
one place slice 3 touches the S47 reservation. If the held-passage deep bed already sounded hollow in
S47's A/B, the +0.36 melody bump makes it a *hair* more so; the fix remains the same (deep-tier
`PAD_ONSET_FLOOR = 2`, fill the bed, do not shrink the lead), and remains a bed-floor item, not a
slice-3 item.

**LISTEN-FOR (point 3):** (a) Img3 after the level finish — does the melody now separate cleanly from
the moving counter, or do they still blur? If clean, the level finish resolved it (do NOT re-route).
If still blurred, A/B routing it deep — but listen for whether deep over-leads it (sounds spotlit on a
non-subject image) before committing. (b) Any dark counter image — does the melody seat read clearly
on top at gap 2? Keep 2 unless a specific image lurches.

---

## Point 4 — WHAT THE OPERATOR LISTENS FOR IN THE A/B (seed 42, before/after × 6)

The aesthetic acceptance criteria for the figure-ground arc's CLOSE. This is the trained-ear gate —
each is a yes/no the trombone ear can render that no scorecard can:

1. **Single hummable figure.** Can you track the melody as the ONE figure through the whole render and
   hum it back? The arc's entire purpose is a melody that *arrives as the figure*, not a melody-role
   buried in an even texture. If you can't pick the tune to hum, the figure-ground failed regardless of
   F1/F3/F5b.

2. **Sits on top without shouting.** Does the melody sit *pleasingly* on top — clear, but not pushed,
   honking, or louder-than-the-arrangement-wants? This is the level-finish proportionality test by ear:
   a finish (a seat) reads as clarity; an over-bump reads as volume. The ≈+0.7-vel mid melody and the
   −2 counter should give "clearer," not "louder."

3. **A low-seated (dark/field-image) melody holds figure through SEPARATION, not loudness.** On a
   dark or field/even image where the melody seats lower, can you still hear it leading — and is it
   leading because it *attacks off the beat and is crisper* (the comp), rather than because it's loud?
   This is operator signal 4 — the subtle one — and the whole point of the comp. The proof is a
   low-seated melody that's clearly the figure *with no extra level*. (And its complement: on a
   clear-subject image, the lead sits cleanly on top with NO nervous off-beat fidget — the comp
   correctly NOT firing.)

4. **Backgrounds receded WITHOUT hollowing.** After the −2 counter / −1 fill recession (and the bed's
   S47 activity recession), did the bed get OUT OF THE WAY — or did it drop out, leaving the lead
   exposed over air? The bed must recede into *support*, not into *absence*. (Carries the S47 deep-bed
   thinness watch — listen hardest on held passages of deep/subject images.)

5. **The counter stayed a MOVING line (S45 preserved).** After the activity recession (S47) + the −2
   level bias (S48) + any DP-6 trim, is the counter still an audible, MOVING second voice — or did the
   stacked recessions duck it to inaudible/static? S45's gain (the inner texture finally moves) must
   survive the finish. A receded-but-moving counter is the goal; a silenced or frozen counter forfeits
   S45 and re-opens the static-bed defect. This is the single most important *preserve* check of the
   slice — the recessions stack, and the ear must confirm they receded the counter without killing it.

---

## Overall aesthetics verdict

**The figure-ground arc CLOSES cleanly. The level finish is a true, proportionate 10% — count- and
pitch-neutral, riding on top of a gap that activity and register already won; remove it and the
melody is still the figure. The inverse-register comp's image-conditioned behavior (act on low-seat
field/ambiguous images, idle on clear-subject deep images) is AESTHETICALLY CORRECT — it is the same
non-rigid, image-reading discipline that made the S47 spread "the soul of the build," now applied to
separation: a strong subject leads itself and is not propped; a field image's lead must work and gets
the help. The F4 metric's deep-image scatter is a metric-scoping artifact (it reads the comp across
steps where the comp is silent), NOT an engine defect — the Test Engineer was right to HOLD the
all-steps promotion. The cleanest honest resolution is to SCOPE F4 to the comp-eligible (low-seat)
steps and sign-assert there, reporting the all-steps corr beside it. Do NOT widen the comp onto deep
images to satisfy the metric — that re-introduces the flat-maximum mechanical defect the whole arc
killed.** Ship the level finish at the spec'd starts; resolve Img3 from the level side first; keep
`MIN_FIGURE_GAP = 2`; let the operator's seed-42 A/B — especially criterion 5, that the stacked
recessions left the counter moving — be the final gate.

---

## RETURN TO LEAD — the F4/deep-routing recommendation and the level-finish verdict (one paragraph)

The level finish COMPLETES the 90/10 without undoing it — KEEP it at the spec'd starts (deep 0.92 /
mid 0.82 / shallow 0.74, `COUNTER_VEL_BIAS = 2.0`, `FILL_VEL_BIAS = 1.0`): it is a ≈3-velocity
widening that is count-neutral and pitch-neutral, so it is mathematically incapable of carrying the
figure-ground (the activity governor, seat guard, and Pad recession already do), and the removal test
passes — strip the level lever and the melody is still the figure; flag only the mid bump as the
single most-audible knob for the A/B, ship it (holding mid at 0.78 under-delivers the operator's
explicit ask), and let the −2 counter recession carry the mid finish if 0.82 ever reads as a shout.
On the F4/deep-routing call: the comp idling on deep images is CORRECT behavior, not a coverage gap —
a clear-subject image seats an unambiguous lead that should NOT be propped (propping it is the
flat-maximum mechanical defect S46 spent the whole arc killing), while a field/ambiguous image is
exactly where a lower-seated lead must work to stay in front and the separation earns its keep — so my
single clearest recommendation is **SCOPE THE F4 METRIC TO THE COMP-ELIGIBLE (low-seat) STEPS and
sign-assert the negative correlation there, reporting the all-steps corr beside it as a diagnostic;
the deep-image "failures" (Img2 +0.462, magicstudio +0.117) are the metric reading the bed's offset
pattern on steps where the comp never fired, and Img3's −0.458 with the comp on ZERO steps is the same
artifact with a lucky sign**; do NOT widen the comp onto deep images to make F4 pass (that props a lead
the image already made unmistakable), and keep report-only as the documented fallback only if the
scoped low-seat sample proves too thin across the 6-image set to correlate stably. Img3's blur should
be cured from the level side first (the −2 counter bias + 0.82 mid bump widen the mid mel−counter gap
without re-routing) and held OUT of DEEP, because routing it deep would both over-lead a genuinely-MID
image and silence the comp on the one image where a lower-seated melody most needs it.

---

*End of S48 Slice 3 aesthetics review. Review-only: no source, test, or asset modified.*

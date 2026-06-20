# S47 Slice 1 Aesthetics Review — Figure-Ground Hierarchy: Sizing the Ear-Tunable Magnitudes

**Author role:** Composition & Songwriting Aesthetics Specialist (the standing TASTE/AFFECT
review voice wired into the S47 build cadence per the Specialist Marshaling Gate).
**REVIEW / DESIGN ONLY — no `src/*`, `tests/*`, or `assets/*` modified by this document; `docs/`
only.** I reason from arrangement craft + the build contract + the measured scorecard; I CANNOT
hear the WAVs — every recommendation below names a concrete LISTEN-FOR so the operator's ear is the
final gate.
**Date:** 2026-06-19.
**Grounds:** my S46 lens (`docs/design-s46-aesthetics.md` — the figure must have the BEHAVIOR of a
lead not just its register/level; a flat-maximum figure-ground is itself a defect; a field image
deserves an even texture), the work-order (`docs/design-s46-figure-ground.md`), the build contract
(`docs/spec-s47-slice1-build.md`), the metric spec (`docs/spec-s46-figure-ground-metrics.md`), and
the LIVE landed constants re-read this session in `src/chord_engine.rs`
(`MIN_FIGURE_GAP=2` `:1106`, `ACTIVITY_FLOOR_THRESHOLD=0.50` `:1097`, `COUNTER_CEILING=67` `:3841`,
`MELODY_REGISTER_FLOOR=67` `:1486`, `PAD_DEEP_RECESSION_CEILING=0.40` `:1147`,
`PAD_WEAK_BEAT_FRAC=0.5` `:1165`, the relative `pad_onset_cap` `:1200-1213`, the weak-beat
displacement `recede_pad_onsets` `:1233-1270`).

**The measured result I am sizing.** F5b → 0 everywhere (the activity inversion is dead — the
background no longer out-moves the foreground on any image). F1 melody-most-active margin POSITIVE
everywhere: deep/subject images +1.0 to +1.25 (a strong lead, Pad ~1 onset under a ~2-onset
melody), field images smaller positive margins. Rollup: 4 VARIED / 2 PARTIAL. **This is a clean
correctness pass — the build did exactly what the contract said.** My job is the orthogonal
question the scorecard cannot answer: *does it SOUND PLEASING, or did a correct figure-ground
margin become a mechanical or hollow one?*

---

## The headline (read this first)

**The single biggest pleasing-vs-mechanical risk is knob 1: the DEEP-tier bed at ~1 onset under a
~2-onset melody on a SUBJECT image.** A +1.0 to +1.25 onset margin is a *strong, satisfying* lead
gap on paper, and the architecture protects it well — the cap is RELATIVE (the bed tracks the
melody's class, not an absolute floor), it never goes silent (`PAD_ONSET_FLOOR`), and it
weak-beat-displaces rather than fusing. **But "1 onset per step" is the exact magnitude where a
comp stops reading as accompaniment and starts reading as a sparse, exposed drone** — a lead with
almost nothing under it. The margin is correct; the ABSOLUTE bed density is what I want the
operator's ear on. My recommendation is to **keep the relative shape but verify the deep bed does
not read as hollow on the calm-melody / held passages**, where the melody itself floors to only ~2
onsets and the bed to ~1 — two thin lines and air. That is the one place this otherwise-clean build
could sound under-done rather than clear. Everything else I size as keep-or-minor.

---

## Per-knob sizing

### Knob 1 — DEEP-tier recession depth (the load-bearing call) — **KEEP the shape, WATCH the absolute floor**

**Recommendation: KEEP `PAD_DEEP_RECESSION_CEILING = 0.40` and the relative one-onset-below-melody
cap.** Do NOT soften the *margin*. But flag the *absolute* deep-bed density as the build's one
real listen-for.

**Arrangement justification.** The instinct "a +1 onset gap is a thin bed" is half-right, and the
architecture already answered the dangerous half correctly. Three things make this gap safe that a
naive "Pad = 1 onset, always" would not:

1. **It is RELATIVE, not absolute** (`pad_onset_cap` `:1204-1212`: `mel = melody_min_onsets(class)`,
   deep tier = `mel.saturating_sub(1)`). When the melody is at 2 onsets the bed is at 1; when a
   later texture-arc slice blooms the melody to 3-4 at a climax, the bed blooms to 2-3 UNDER it.
   The gap is one onset, not a starved bed — and crucially it *grows in absolute density with the
   melody*. This is the difference between "a clear lead over a comp" (pleasing) and "a lead over a
   single sustained drone for the whole piece" (hollow). The shape is right.
2. **It never hollows to silence** (`Some(capped.max(PAD_ONSET_FLOOR))` `:1212`). A bed floored at
   ≥1 onset is still *present* — the harmonic floor the lead sits on top of is always there. A bed
   that thins to zero is the empty/exposed failure; this build structurally cannot do that.
3. **The surviving onset is weak-beat-displaced, not downbeat-fused** (`recede_pad_onsets`
   `:1258-1268`). A single bed onset left on the downbeat would fuse with Bass+Fill+Melody into one
   stab (the mush signal). Displacing it to the weak beat turns the thinned bed into an *off-beat
   comp figure* — a deliberate arrangement gesture (see knob 5). This is the move that keeps the
   thin deep bed reading as "comp" rather than "remnant."

**Why I still flag it.** All three protections operate on the *gap* and the *minimum*. None of them
guarantee the deep bed sounds *full enough* on the passages where the melody itself is at its
floor. On a calm/held melody passage with the deep tier active, the melody floors to ~2 onsets
(the `ACTIVITY_FLOOR_THRESHOLD` lift) and the bed to ~1 — so the whole texture is, on those steps,
two sparse lines over the Bass. On a portrait/subject image that may be exactly right (a clear,
intimate lead with an uncluttered bed — think solo voice + light comp). But on a *busy* subject
image (a sharp subject against a detailed scene) the same sparse deep bed could read as
under-supported — the image promised richness the arrangement withholds. The deep tier is routed by
`fg_bg_contrast ≥ 0.25`, which is "separated subject," NOT "simple image" — a high-contrast subject
can sit in a busy field. **That is the one image-class where I suspect the deep bed could be too
thin.**

**LISTEN-FOR.** On each subject-routed image (`fg_bg_contrast ≥ 0.25`), at a CALM/held passage:
does the lead sound *cleanly supported* (a clear tune over a light, present comp — good) or
*exposed/hollow* (a tune over almost-nothing, the bottom dropped out — too thin)? Specifically A/B
the held-melody steps. If hollow on a busy-subject image, the soften is NOT to raise
`PAD_DEEP_RECESSION_CEILING` (that would shrink the margin globally) — it is to lift `PAD_ONSET_FLOOR`
to 2 for the deep tier so a held bed keeps two onsets while the melody's floor keeps the gap. That
preserves the lead while filling the bed. Hold that change unless the ear asks; the default is right
for the intimate-subject case, which is the more common subject image.

### Knob 2 — image-conditioning SPREAD (deep vs shallow) — **KEEP; this is the soul of the build**

**Recommendation: KEEP the spread** (deep Mel 0.90 / Pad 0.30 / Counter 0.45 vs shallow Mel 0.72 /
Pad 0.45 / Counter 0.65). This is my S46 metric-rigidity caution made concrete and the build got it
right.

**Arrangement justification.** The whole point of S46 was that a flat-maximum figure-ground — a
melody that leads equally hard on EVERY image — is *itself* the mechanical defect, identical in
kind to the no-figure defect it replaced. A piece that shouts its lead on an abstract field is as
amateur as one that never leads on a portrait. The spread is what makes the engine sound like it
*understood the image*: a subject image gets a clear "I am looking AT something" lead (deep bed,
strong melody), and a field image gets a "this is an even whole" texture (near-even counter, a bed
that legitimately shares focus). The measured result confirms this lands AS AN AUDIBLE DIFFERENCE —
deep images +1.0/+1.25, field images smaller positive margins. **The margins *differ by image*,
which is exactly the breathing, non-rigid result I asked for.** A flat +1.0 everywhere would have
been the regression.

The two extremes are both safe from reading as mechanical:
- **Deep** (Mel 0.90, Counter 0.45): a clear lead with the counter well under it — not 1.0/0.30
  (the `subject_melody` top-escalation tier, reserved), so it stops short of "spotlight everything
  else off." Good — it leads without sterilizing the texture.
- **Shallow** (Mel 0.72, Counter 0.65): near-even, counter only barely under the melody — the
  texture genuinely shares focus, but the counter is STILL under the melody (0.65 < 0.72) so there
  is still *a* figure. A field image with no figure at all would be a wash; this keeps a gentle lead
  while honoring the evenness. That 0.07 gap is the right "present but not insisted" margin.

**LISTEN-FOR.** A/B a clearly-subject image against a clearly-field image: does the subject one
make you *follow the tune* and the field one make you *take in the whole*? They should feel like two
different arrangement intents, not the same arrangement at two volumes. If the shallow image still
feels like it's "trying to lead and failing" (the original defect), nudge shallow Mel down toward
0.70 and Counter up toward 0.67 — but only if the field images still sound figure-led. I expect they
do NOT; this spread reads correct to me.

### Knob 3 — routing thresholds (deep ≥0.25, shallow <0.10 `fg_bg_contrast`) — **KEEP; one binning caveat to confirm by ear**

**Recommendation: KEEP deep `≥0.25`, shallow `<0.10`, mid in between.** With the caveat below about
the 2 PARTIAL images.

**Arrangement justification.** The bins assign the right *aesthetic treatment* per image in
principle: high subject-separation → subject-led; low separation → even. The live
`fg_bg_contrast` range is 0.052–0.341, so the thresholds carve the set into a deep cohort (the high
end), a field cohort (the low end), and a mid middle. That is the correct *direction*. The risk is
never the direction — it is whether a SPECIFIC image's measured `fg_bg_contrast` matches its
*perceived* figure-strength, because `fg_bg_contrast` is a saliency proxy, not a semantic judgment.

**The image I would scrutinize.** The rollup is 4 VARIED / 2 PARTIAL. A PARTIAL is, by the metric
spec's verdict logic, an image where some F-metrics hold and some don't — and the most likely
aesthetic cause of a PARTIAL (given F5b is 0 everywhere) is an image binned into a tier its content
doesn't justify: a portrait whose background happens to be low-contrast getting binned MID or
SHALLOW (so it reads under-led — a subject that doesn't get its lead), or a busy/even field whose
one bright region spikes `fg_bg_contrast` into DEEP (so it reads over-led — a forced spotlight on a
wash). **Those two failure modes are exactly the §1 inverse-errors of my S46 lens, and a PARTIAL is
where they would show.** I cannot tell from the margins alone which two images are PARTIAL or why —
that is the operator's ear + the per-image scorecard row.

**LISTEN-FOR.** For each of the 2 PARTIAL images: does the treatment MATCH the picture? A portrait
that doesn't lead clearly = mis-binned shallow/mid (lower the deep gate toward 0.20, or this image's
contrast is genuinely low and the fix is content-semantic, out of slice scope). An abstract field
that has an insistent forced lead = mis-binned deep (raise the deep gate toward 0.30). Flag the
specific image to me/Affect; the threshold nudge is cheap and ear-cheap. **This is the one knob
where a per-image listen could reveal a real mis-assignment** — the magnitudes are otherwise sound.

### Knob 4 — `MIN_FIGURE_GAP = 2` semitones / the dark-image seat-lift — **KEEP 2; the 67→79 "octave leap" is a non-issue as I first read it**

**Recommendation: KEEP `MIN_FIGURE_GAP = 2`.** And a correction to the framing in the task: the
octave leap concern is smaller than it sounds.

**Arrangement justification, with the live arithmetic.** I re-read the seat logic
(`chord_engine.rs:1543-1556`). The seat-order guard floors the melody to `COUNTER_CEILING +
MIN_FIGURE_GAP = 67 + 2 = 69` *only when a counter is present and a dark-image lift would otherwise
drop the melody below 69*. So on a dark image the melody seats at **69 (A4)**, not 79. The `79`
(`G_MELODY_NOTE`, = 67 + a +12 bright lift) is the BRIGHT-image seat, not where the guard lands a
dark image. There is no 67→79 octave jump caused by the guard. The guard's actual effect is: a dark
image that would have seated the melody at, say, 55-57 (inside the counter band — the inversion) now
seats it at 69. That is a *register correction of ~12-14 semitones relative to the broken behavior*,
but the broken behavior was the melody sinking INTO the counter — i.e., the "jump" is the melody
being lifted OUT of the bed to where a lead belongs. **That is not a contour break; it is the figure
arriving on top.** A lead that sits a clean 2 semitones above the highest inner voice is the
textbook high-voice-superiority placement.

Where a contour concern *could* be real: if within a single phrase the melody crosses from a step
where the guard does NOT bite (bright, seated high) to a step where it DOES (a momentary
dark-region step, floored to 69), the seat could shift abruptly mid-line. But the seat is a per-step
FLOOR on the register *placement*, and the melody's actual pitches are chord tones drawn within that
register — so the line moves by its own voice-leading, not by the floor snapping. The floor only
prevents the line from sinking too low; it does not yank it. **A 2-semitone clear margin is the
right amount** — 0 (a tie at the counter ceiling) would let the lead and the top inner voice collide
and lose the figure cue; more than ~3-4 would start to over-separate the registers into two
disconnected bands.

**LISTEN-FOR.** On a DARK / low-key image with the counter routed: does the melody stay clearly the
top line *and* does it still sound like one continuous melodic line (not a line that lurches up
whenever a dark step hits)? If you hear a lurch, that's the floor biting mid-phrase — drop
`MIN_FIGURE_GAP` to 1 (still a clear seat, less lift). I expect 2 is fine; the line's pitches are
voice-led, only the floor moved.

### Knob 5 — Pad weak-beat displacement (`PAD_WEAK_BEAT_FRAC = 0.5`) — **KEEP; this is the move that rescues the thin bed**

**Recommendation: KEEP `PAD_WEAK_BEAT_FRAC = 0.5`.**

**Arrangement justification.** This is the knob that decides whether the recessed bed reads as a
*deliberate accompaniment figure* or a *thinned remnant* — and 0.5 (the exact mid-point of the step,
the "and" of the beat) is the most idiomatic comp placement there is. The displacement only fires
when every surviving Pad onset would otherwise be stuck on the downbeat (`recede_pad_onsets`
`:1258`), i.e. precisely the block-bed case where a single thinned stab would FUSE with the
Bass+Fill+Melody downbeat into one undifferentiated hit. Pushing that stab to the half-beat turns
"one lonely chord stab on the downbeat, doubling everyone else's attack" into "an off-beat comp that
answers the melody's downbeat" — the difference between a remnant and a groove. **An off-beat bed
under an on-beat lead is one of the oldest pleasing figure-ground arrangements (the stride/comping
left hand, the reggae skank, the ii-V piano comp).** The build chose the right gesture.

0.5 specifically (vs, say, 0.33 or 0.66) is the safest default because it is metrically neutral —
it lands the comp squarely between beats, legible in any meter, and it maximizes the onset-grid
distance from the downbeat (the F5a anti-fusion margin). A swung or genre-specific feel might
eventually want 0.66, but that is a texture/genre decision for a later arc, not a fix.

**LISTEN-FOR.** On a deep/subject image where the bed thins to one displaced stab: does that
off-beat bed sound *intentional* (a comp answering the tune — good) or *awkward/orphaned* (a stray
hit that doesn't belong)? If orphaned, the cause is more likely the *hold* re-fit (`:1260-1263`)
than the offset — check whether the displaced stab sustains into the back of the step (it should
ring as a comp, not click and die). The 0.5 offset itself I'm confident in.

---

## Climax-bloom cross-arc invariant — **READY. The relative cap is exactly the right shape.**

**Verdict: the slice leaves the climax-bloom invariant correctly armed.** This is the part I'm most
pleased with architecturally.

The cross-arc invariant (work-order §4, spec §7) is: *at a climax the bed may bloom in density ONLY
IF the figure-ground GAP blooms WITH it* — the climax is "the fullest the bed ever gets while the
lead is still unmistakably in front," never an equal-voices tutti. The danger was a bed cap pinned
to an ABSOLUTE low onset count: that would either (a) starve the climax (the bed can't bloom because
it's hard-capped at 1) or (b) require the climax slice to special-case the cap (fragile coupling).

This build pinned the cap RELATIVE to the melody's activity class (`pad_onset_cap` `:1204`:
`mel = melody_min_onsets(class)`; deep = `mel - 1`, shallow = `mel`). So when a later texture-arc
slice blooms the melody to a higher activity class at the climax, `melody_min_onsets(class)` rises,
and the bed's cap rises WITH it — the bed thickens UNDER the melody while the one-onset deep gap (or
the at-melody shallow cap) is preserved. **"The gap stays constant in onsets while both lines grow"
is precisely the "fullest bed under an unmistakable lead" shape the invariant wants.** The bed gets
denser, the lead stays one rank above, the figure never dissolves into the tutti.

One refinement note for the texture-arc author (not a slice-1 defect): a *constant one-onset gap*
at the climax is the floor of correctness, but the most *pleasing* climax often WIDENS the gap
slightly as it blooms (the lead asserting MORE as the texture swells, so the homecoming feels
owned). The spec already encodes this as the climax guard "F1 margin at climax ≥ F1 margin at
Statement" — i.e. the gap may *grow* but must not *shrink*. Slice 1's relative cap leaves exactly
that room: the texture-arc slice can choose to let the melody bloom TWO ranks while the bed blooms
one, widening the gap, and the relative cap accommodates it without a slice-1 change. **The cap
correctly leaves room for the gap to widen at climax — it does not pin a constant margin.** Ready.

**LISTEN-FOR (when the texture-arc slice lands):** at the climax, is the bed audibly FULLER than at
the statement AND is the lead audibly MORE in front, not less? Both must rise together.

---

## Overall aesthetics verdict

**The figure-ground arrangement is PLEASING and correctly sized — not over-done, with ONE
watch-item on the under-done side (the deep bed on busy-subject images).** This is a clean,
musically-literate build: it fixed the inversion on the strongest cues (activity, register), it
made the lead *behave* like a lead rather than just sit where one would, and — most importantly for
my lens — it did NOT replace the no-figure defect with a flat-maximum figure defect. The
image-conditioned spread is the soul of it: subject images get a clear lead, field images keep an
even texture, and the measured margins *differ by image*, which is the breathing, non-rigid result
I asked for in S46. The relative caps protect the climax-bloom future arc for free. Every dangerous
magnitude (silence, downbeat-fusion, a tie at the counter ceiling, a flat margin) is structurally
prevented.

The single honest reservation is the absolute density of the DEEP bed at its thinnest: a ~1-onset
bed under a ~2-onset held melody is a *correct* gap but a *sparse* texture, and on a high-contrast
subject that happens to sit in a BUSY scene, that sparseness could read as exposed/hollow rather
than as clean/intimate. That is the one place this build could be under-done. It is NOT a margin
problem (do not shrink the lead) — if the ear hears it, the fix is a deep-tier `PAD_ONSET_FLOOR` of
2, filling the bed without touching the gap. I'd ship as-is and let the operator's A/B on the
held-passages of subject images decide; my prior is that the intimate-subject case (the common one)
sounds right and only a busy-subject outlier might want the floor lift.

---

## RETURN TO LEAD — raw substance

**Per-knob sizing:**
1. **Deep recession depth:** KEEP `PAD_DEEP_RECESSION_CEILING = 0.40` and the relative
   one-onset-below cap. The margin is right and well-protected (relative cap, never-silent floor,
   weak-beat displacement). WATCH the *absolute* deep-bed density on held passages of
   busy-subject images — if hollow, fix is deep-tier `PAD_ONSET_FLOOR = 2` (fills bed, keeps gap),
   NOT a ceiling change.
2. **Image-conditioning spread (deep 0.90/0.30/0.45 vs shallow 0.72/0.45/0.65):** KEEP. This is the
   soul of the build; the measured per-image margin differences ARE the non-rigid, breathing result
   S46 demanded. Both extremes stop short of mechanical.
3. **Routing thresholds (deep ≥0.25 / shallow <0.10):** KEEP the direction. Scrutinize the 2 PARTIAL
   images by ear for mis-binning (portrait binned shallow → under-led; field spiked into deep →
   over-led). Flag the specific image; threshold nudge is cheap.
4. **`MIN_FIGURE_GAP = 2`:** KEEP. Correction: the guard floors a dark-image melody to 69 (67+2),
   NOT 79 — there is no octave leap; the "jump" is the lead being lifted OUT of the counter band to
   where a lead belongs, which is the line arriving on top, not a contour break. 2 semitones is the
   right clear-seat margin.
5. **`PAD_WEAK_BEAT_FRAC = 0.5`:** KEEP. The half-beat displacement is what converts the thinned bed
   from a downbeat-fused remnant into a deliberate off-beat comp figure — the oldest pleasing
   figure-ground arrangement. 0.5 is the safest, most metrically-neutral default.

**Biggest pleasing-vs-mechanical risk (the headline):** the DEEP bed is too THIN, not too loud —
at its thinnest (~1 onset under a ~2-onset held melody) it could read as an exposed lead with no
comp underneath on a high-contrast-subject-in-a-busy-scene image. It is a correct gap over a sparse
texture. Listen to the held passages of subject images; if hollow, lift the deep-tier onset FLOOR,
never shrink the lead.

**Climax-bloom readiness:** READY. The cap is RELATIVE to the melody's activity class, so the bed
blooms UNDER the melody at climax while the one-onset gap is preserved, and the spec's "gap may
widen, never shrink" guard has room to let the lead assert more as the texture swells. The slice
does NOT pin a constant margin — exactly right.

**Overall aesthetics verdict:** PLEASING and correctly sized — not over-done. A musically-literate
build that fixed the inversion without replacing it with a flat-maximum (mechanical) figure; the
image-conditioned spread makes it sound like the engine read the image. One honest reservation: the
deep bed at its thinnest is sparse and *could* sound exposed on a busy-subject image — an under-done
risk, not an over-done one, and fixable from the bed-floor side without touching the lead. Ship and
let the operator's A/B on held-passage subject images settle the one watch-item.

---

*End of S47 Slice 1 aesthetics review. Review-only: no source, test, or asset modified.*

# S43 Taste Gate — Aesthetic Verdict (Composition & Songwriting Aesthetics)

STANDING TASTE GATE, beside correctness. The S43 two-tier melody-foregrounding
change in `assets/mappings.json` passed its correctness guards; this is the
independent aesthetic read on whether it will SOUND like a fix to a trained ear.
Renders inspected: `/tmp/s43_example.wav`, `/tmp/s43_lena.wav` (`--seed 42`,
pure-Rust default). I cannot literally listen; every judgment below is reasoned
from realized velocities, resolved prominence, per-role onset grids, register,
and the rhythm-band math, then translated into an aesthetic prediction. Where the
structure cannot stand in for the ear, I say so.

## VERDICT: PASS-WITH-WATCH-ITEMS

The fix is real and correctly aimed. On `example` it should land clearly — the
melody now leads, the line is de-fused from the bed, and it earns a genuine
figure/ground gap. On `Lena` the fix is directionally correct and audibly
better, but it is **half a fix**: it lifts the LEVEL but not the GRID, and that
is the single watch-item that could still leave Lena as "the accompaniment with
a loud top note" rather than "a tune over a bed." Honest bottom line: example is
likely solved; Lena is improved-but-on-probation, and the operator's ear is the
gate that decides whether the level-only lift is enough or whether the de-fusion
needs to extend to the default profile.

---

## Per-criterion judgment

**1. MELODY-AS-SUBJECT / MEMORABILITY — PASS (example) / WATCH (Lena).**
`example` resolves to `subject_melody` (Melody w=1.0): realized Melody 102 vs
loudest bed 89 = **+13 over the bed** (the prompt's +9 figure is vs CounterMelody
93; vs the actual loudest *bed* role, HarmonicFill 89, the gap is wider), PLUS
the +2 register lift PLUS prom_shift = +0.05 dropping the melody's rhythm cutoffs
to 0.75/0.50/0.20 so it subdivides on a different grid than the Pad's even alberti
burst. Level + register + independent grid is all three primary stream-segregation
cues firing together — that predicts a line a listener can FOLLOW as a figure, not
read as chord-tops. PASS. `Lena` resolves to `melody_forward` (w=0.78): Melody 98
vs bed 89 = **+9 over the bed** — a real, audible level gap, so it will be heard
as louder. But the de-fusion is the concern (see below). The level alone makes the
melody the loudest thing; whether it reads as a *subject* rather than a loud bed
voice is the watch-item.

**THE SHARPEST RISK — Lena's louder-but-still-not-segregated line — addressed
directly:** `melody_forward` applies prom_shift = (0.78−0.5)×0.10 = **+0.028**,
versus subject_melody's +0.05. Lena's dotted-band cutoff therefore moves from
0.250 to only **0.222** — a 0.028 nudge. That is too small to reliably reclassify
steps: in the S42 BEFORE trace Lena's melody already ran sustained ×17 / arpeggio
×9 / dotted ×9 at prom_shift=0, and a 0.028 cutoff drop will flip only the handful
of steps whose edge_activity happens to sit in the [0.222, 0.250) sliver. So on
Lena the melody is **louder but rhythmically nearly co-located with the bed.** The
one thing that SAVES Lena here is not in the prominence table at all: her Pad fell
to the **plain block triad `(0,0,0)`** (S42 trace A.2), NOT the animated alberti
burst that `example` got. A block-held bed has no competing onset grid to fuse
with — the melody's onsets are the only motion, so even an un-shifted melody grid
segregates by being the only thing that moves. **Net Lena prediction:** the melody
will read as the foreground because it is both loudest AND the only animated voice
over a static block bed — but it will feel like a foregrounded line by *default of
a still bed*, not by rhythmic independence the way example's does. If Lena's
figuration ever resolves to an animated bed on some other image, the level-only
default will fuse. WATCH.

**2. TWO DIFFERENT PIECES, NOT ONE TRANSPOSED — PASS (cautious).**
The change stacks three distinctions now: (a) theme vs no-theme (example recalls a
real motif in its Statement section, Lena free-selects top chord tones on every
step), (b) full-lift vs default-lift assertiveness (+13 over bed and de-fused grid
vs +9 and co-located grid), and (c) animated alberti bed vs static block bed. That
is a genuinely different LISTEN: example should read as an assertive, hooky,
rhythmically-active piece with a recurring figure; Lena as a gentler, wandering,
exposed line over a still bed with no home to return to. They will no longer be
"the same piece in a different key." The residual sibling-risk is shared *gait* —
both still sit on the same Bass-sustained-root + Fill-sustained-inner foundation
and the same sectional clock — but with the melody now the perceptual subject, the
ear is no longer identifying the piece BY that shared bed. PASS, with the
acknowledgment that the deeper "shared skeleton" worry is the S13 macro-form
question, out of scope for this lever.

**3. BED RECEDES BUT DOES NOT VANISH — PASS.**
Both profiles recess HarmonicFill and Pad to 0.40 (below neutral 0.5, above the
0.25 vanish floor). Realized HarmonicFill 89 on both — it has dropped from being
the LOUDEST role (101/94 in S42) to sitting clearly under the melody, which is
exactly the inversion that needed fixing, without going hollow. Bass stays neutral
0.50 (foundation intact). The texture still supports. No Edit-3 Fill bias was
shipped, and the realized numbers say it is not needed — the weight recession did
the job. PASS.

**4. EXPOSED-MELODY VOICE-LEADING / CADENTIAL HOMECOMING — WATCH (honest limit).**
This is where structure cannot substitute for the ear, and I will not overclaim.
A louder, more-exposed melody mathematically reveals voice-leading that was
forgivable when buried. Two grounded observations: (i) the melody free-selects the
**top chord tone** via `role_pitch` on every non-theme step (and on ALL of Lena),
so by construction it lands ON a chord tone — there is no "wrong note" risk, but a
top-chord-tone-tracking line can read as arpeggio-following rather than as a melody
with its own contour, which a louder level now exposes. (ii) The cadence path is
the **unshifted** arpeggio acceleration (`pre_cadence ||` disjunct, prom_shift
never applied to it), so the cadential gesture IS present in the melody as a
4-onset drive into the cadence — the homecoming is structurally in the melody, not
only the harmony bed. But whether the *pitch* of the cadential arrival reads as a
satisfying melodic resolution (scale-degree 1 or 3 landing, not just "a chord tone
that happens to be on the downbeat") is not something the realized-event structure
guarantees — that is an ear call. WATCH.

---

## The acceptance gate — what the operator should listen for on the re-listen

1. **example: is there a TUNE you can hum back?** The fix predicts a followable,
   rhythmically-active line that sits clearly on top. If you can hum example's
   melody after one listen but not Lena's, that is the two-tier design working
   AS DESIGNED (assertive hook vs wandering line) — not a Lena failure.
2. **Lena: does the melody read as a SEPARATE voice, or as a loud top note of the
   chords?** This is THE acceptance test for the level-only default. Specifically:
   does Lena's melody feel like it has its own rhythm, or does it feel locked to
   the bed's pulse? If it feels locked / "loud but still part of the texture," the
   level-only lift was insufficient and the de-fusion lever (below) should arm.
3. **Did the over-loud inner Fill disappear as the loudest thing?** On both
   renders, confirm you are no longer hearing a middle voice shouting over the
   tune. (Realized says yes; confirm by ear.)
4. **example AND Lena: does the melody actually LAND at the cadence** — does the
   phrase ending feel resolved IN THE MELODY (a sense of arrival/homecoming in the
   tune), not only because the harmony underneath resolved? Listen at section
   boundaries.
5. **Do the two pieces feel like different pieces, not one transposed?** The gross
   test from S42 — if they still feel like "same piece, different key," the lever
   did not move the needle and the diagnosis was wrong about salience being the
   root cause (I do not predict this, but it is the disconfirming observation).

---

## Pre-staged levers (do NOT apply now — armed for the operator's ear)

- **PRIMARY (addresses the sharpest risk): extend the rhythm-band de-fusion to the
  `melody_forward` default, not only `subject_melody`.** The asymmetry is the gap:
  the default lifts LEVEL but its prom_shift (+0.028) is too small to move the
  melody off the bed's grid, so Lena's segregation currently depends entirely on
  her bed happening to be a static block. Two ways to arm if Lena reads as
  "loud-but-fused":
    (a) raise `melody_forward` Melody weight 0.78 → ~0.85, which both widens the
        level gap (+5→+6 nudge) AND raises prom_shift to (0.85−0.5)×0.10 = +0.035
        (dotted cutoff → 0.215) — a bigger but still-modest grid nudge; OR
    (b) the more targeted fix — decouple the de-fusion from level: give the melody
        a floor of rhythmic-grid independence in the default profile so a
        foregrounded melody ALWAYS subdivides off the bed regardless of how much
        its level was lifted. This is the cleaner aesthetic answer because grid
        independence (not loudness) is what makes Lena's line read as a tune over
        a still bed AND protects against the case where the default sits over an
        animated bed.
  Recommendation: re-listen first. If Lena's melody hums as its own line, ship as
  is. If it reads as a loud top note, arm (a) as the one-number ear-tune; reach for
  (b) only if (a) still fuses.
- **SECONDARY (only if example shouts):** lower `subject_melody` is NOT indicated
  — +13 over bed is assertive but appropriate for a theme-bearing piece; leave it.
- **DO NOT** add the Edit-3 HarmonicFill negative bias — the weight recession
  already dropped Fill from loudest (101/94) to under the melody (89/89). Adding it
  now would over-recess and risk the hollow-bed regression (Criterion 3).

---

## Honest limits of this read

- I reasoned from realized velocities, onset grids, register, and the rhythm-band
  cutoff math — NOT from audio. Loudness/level segregation and grid de-fusion are
  high-confidence structural predictions of stream segregation; melodic
  memorability, contour quality, and cadential satisfaction-of-arrival are ear
  calls the structure cannot settle.
- Lena's saving grace (block bed → no competing grid) is image-specific. The
  level-only default profile is robust on Lena BECAUSE of her static bed; it is
  NOT proven robust for a future image that gets both the default profile AND an
  animated bed. That combination is the latent failure mode the PRIMARY lever
  pre-empts.

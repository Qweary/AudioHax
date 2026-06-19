# S43 Salience Fix — Affect / Cross-Modal Taste-Gate Verdict

STANDING TASTE GATE (Perceptual / Cross-Modal Affect lens), sitting beside the
correctness gate on the S43 melody-foregrounding fix. This renders an
affect/perception verdict on a change that already passed correctness; it does
NOT author or modify any production content. The realized after-state numbers
below were measured through the real pipeline at `--seed 42` by the Test
Engineer and independently reconciled here against `assets/mappings.json` and
the velocity/register constants in `src/chord_engine.rs`
(`PROMINENCE_VEL_SPAN=18`, `PROMINENCE_REG_SPAN=4`, fixed role biases
`Melody +2 / Bass −1 / Pad −3`, HarmonicFill falls through `_ => {}` with no
bias). The `--json` resolved-structure path is not yet wired, so I reason from
the realized NoteEvent velocities/register/onset grids, not from a direct ear.

## VERDICT: **PASS-WITH-WATCH-ITEMS**

The fix structurally does the load-bearing thing the S42 diagnosis demanded: it
inverts the dynamic field so the melody is now the loudest voice on both images
(the HarmonicFill no longer floats to the top), and it opens a two-tier
assertiveness gap (example +9 / Lena +5) that, layered on theme/no-theme, is the
first per-image *energy* distinction the system has produced. That is a real,
direction-correct improvement and I expect the ear to confirm the melody now
reads as a figure. I do not clear it to unqualified PASS for two reasons the
numbers expose: Lena's +5 figure/ground gap is in the perceptually-timid zone
where the figure can read as merely "the top, slightly-louder note" rather than
"a tune," and the recessed-bed-with-no-Fill-bias choice leaves CounterMelody
(93) as a closer-than-ideal second voice on a melody that itself sits at only 98.
Both are ear-decidable, and both have a pre-staged lever.

---

## Per-criterion judgments (§3 taste-gate criteria)

**1. Foreground / subject-ness — PASS for example, WATCH for Lena.**
Example's Melody 102 over loudest-bed 93 = **+9** gap, plus +2 register lift and
the escalated profile's lowered rhythm-band cutoffs (melody subdivides off the
bed's grid), is comfortably above the threshold where loudness+rhythmic-
independence make a voice read as the *subject* rather than the top chord tone —
the single strongest stream-segregation cue (level) now points the right way.
Lena's Melody 98 over loudest-bed 93 = **+5** gap with only +1 register lift and
the *default* (un-escalated) rhythm cutoffs (melody still shares more of the
bed's grid) is the marginal case. +5 velocity is roughly one just-noticeable
loudness step over the bed; it will read as "on top" but may not read as "a line
to follow." This is exactly the §4 timidity risk. **Watch-item, lever pre-staged
below.**

**2. Per-image affect divergence — PASS (the strongest result).**
The two-tier gap (example full 1.0 / +9 / more-assertive lead; Lena default 0.78
/ +5 / milder lead) means the renders no longer differ only in key — they now
differ in *how assertively the melody leads*, and that difference is
salience-congruent: example carries the higher figure-ground contrast (0.136 vs
0.052) AND the only theme, and it is the one that leads harder. Higher
visual figure-ground → more assertive musical foreground is the correct
arousal/salience cross-modal mapping. example should read as "a piece with a
hook stated emphatically," Lena as "a piece with a softer, wandering exposed
line" — two different *kinds* of piece, which is the §3.2 goal. This is where the
fix most clearly buys the divergence the operator was missing.

**3. Affect regression from recessing the bed — PASS (low risk).**
Pad and Fill weights sit at 0.40 — recessed below neutral 0.5 but well above the
0.25 vanish floor, so the bed thins to support rather than hollowing out. Bass
held neutral at 0.50 keeps the foundation/affective floor intact. I see no
evidence the melody-loudest mix fights a valence/character intent: the bed still
sounds (Fill 89, Pad lower, Bass present), it has simply stopped *competing*.
The one thing the ear should confirm is that recessing the Pad's broken-chord
burst hasn't drained the "pleasant recurring changes" that were carrying the
pieces' warmth — but that surface is still present, just no longer the subject.

**4. CounterMelody as the new loudest non-melody role — PASS-WITH-WATCH.**
This is the correct two-tier *intent* (CounterMelody is a foreground-family
voice and should sit just under the lead, not in the bed), and on the S42 renders
CounterMelody did not even sound, so on *these two images* it is inert. BUT the
numbers show why it is a watch-item the moment a layer set includes one:
CounterMelody weight 0.58 → `(0.58−0.5)*18 = +1.4` nudge with no role bias →
realized 93, while Lena's Melody is only 98. A +5 lead over a +1.4 counter-voice
is a narrow two-tier separation; on Lena specifically (the timid-lead image) a
counter-melody could read as a co-equal duet partner rather than support. On
example (+9 lead) the separation is fine. **Watch:** if a CounterMelody-bearing
image renders on the Lena tier and the two top voices blur, the fix is
CounterMelody 0.58 → ~0.55, not a melody change.

---

## What the operator should listen for (the acceptance gate)

A/B the `--seed 42` renders: `/tmp/s43_example.wav` vs `/tmp/s43_lena.wav`
(and against the S42 before-renders if still held).

1. **example.jpg — is there a tune you can hum/follow, distinct from the chords?**
   The melody should now sit clearly on top and feel like *the* line. Confirm the
   HarmonicFill (the inner sustained tone) is no longer the thing your ear locks
   onto. This is the headline fix — it should be obvious.
2. **Lena.png — can you still find a foreground line, and does it read as a
   *line* and not just "the highest, slightly louder note"?** This is the
   make-or-break listen. If the Lena melody reads as a tune, ship as-is. If it
   reads as timid / still-part-of-the-texture, that is the pre-staged 0.85 lift
   (below).
3. **Do the two now sound like DIFFERENT PIECES, not the same piece in two keys?**
   Specifically: does example lead more *assertively/confidently* than Lena?
   Does example's line recur like a hook while Lena's wanders without a home?
   That energy/assertiveness divergence is the §3.2 success signal.
4. **Has the bed gone hollow?** Confirm the accompaniment still sounds warm and
   supporting under the line — recessed, not gutted. If the texture feels thin or
   the pieces lost their pleasantness, that's an over-recession signal.
5. **(Theory cross-watch the affect ear can also catch)** On the now-louder line,
   do exposed notes still land on chord tones and does the cadence resolve *in the
   melody* (you hear the homecoming in the tune, not only in the chords)? A louder
   voice exposes voice-leading roughness that was forgivable when buried.

---

## Pre-staged calibration levers (do NOT apply now — only if the ear finds the gap)

- **If Lena's lead is timid (criterion 1/2 fails on Lena):** raise
  `melody_forward` Melody weight 0.78 → **0.85** in `assets/mappings.json`
  (`prominence_catalogue`). At 0.85 the nudge is `(0.85−0.5)*18 ≈ +6.3` → Lena
  Melody ~99–100, gap ~+7, register lift +1→+1, narrowing the distance to the
  escalated tier while keeping `subject_melody (1.0) > melody_forward` two-tier
  intact. This is the §4 lever and the single highest-probability follow-up.
- **If the melody is on top but the inner Fill still competes anywhere:** ship the
  optional **Edit 3** — a HarmonicFill `−2` velocity bias in
  `src/chord_engine.rs:1371-1381` (`OrchestralRole::HarmonicFill if !is_cadence =>
  vel -= 2.0`). Not shipped in S43 (correctly — "ship 1+2, re-listen first"); it
  is belt-and-suspenders since `melody_forward` already recesses Fill by weight.
- **If a CounterMelody-bearing image renders with a blurred two-voice top
  (criterion 4 fails in the field):** CounterMelody 0.58 → ~0.55, tier-specific —
  not a melody change. Cannot be exercised on these two images.

## Honest limit
I cannot literally listen. The +9 example result is far enough above the
figure-ground threshold that I am confident the structural prediction (melody
reads as subject) holds. The +5 Lena result sits *on* the threshold where a
structural prediction genuinely cannot substitute for the ear — it is precisely
the case the §4 calibration note was written for, which is why I withhold
unqualified PASS and pre-stage the 0.85 lift rather than guessing the direction
for the operator.

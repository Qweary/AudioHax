# S42 Diagnosis — Music-Theory / Composition-Craft Lens

DESIGN/DIAGNOSIS DOCUMENT. No production code was changed. This reads the
S42 ground-truth trace (`docs/design-s42-trace.md`) through the
composition-craft lens and answers the three diagnostic questions. All
citations are to the current tree; the realization kernel `src/engine.rs` is
not touched by anything proposed here.

---

## HEADLINE

From a craft standpoint the melody is **not functioning as a melody** — it is
realized as a slightly-higher, slightly-louder inner voice, not as a figure the
ear can lift out of the ground. And the reason the two pieces sound like "the
same piece in a different key" is that **the accompaniment IS the perceived
composition**: an even, sustained, broken-chord bed that is image-invariant in
gait carries the identity, while the one thing that actually differs
between the images (the melodic line) is dynamically and figurally fused into
that bed. Foregrounding the melody is the higher-ranked lever; varying the bed
is second. The cheapest high-impact lever is to make the melody *audibly the
subject* — and that is a data-only change.

---

## Q1 — Is the melody actually functioning as a melody?

No. It satisfies the *bookkeeping* definition of a melody (it is the top voice,
it has the widest pitch range, it carries the theme pitches when a theme exists)
but it fails the *perceptual* definition. A line is heard as the melody — the
"subject," in the figure-vs-ground sense — only when it is differentiated from
the accompaniment along the dimensions the ear uses to segregate streams. Run
the trace against the four craft tests for melodic salience:

**1. Registral foreground — PASS (the one thing that works).** The melody median
sits at MIDI 69 / 82, clearly above HarmonicFill (~61/64), Pad (~62/63), and
Bass (~32/42); the register floor is the highest of the four roles
(`src/chord_engine.rs:1210-1212`). Pitch height alone, though, is the *weakest*
of the stream-segregation cues, and it is not enough on its own — a top voice
that is rhythmically and dynamically identical to the texture below it gets
heard as the *top of a chord*, not as a tune. This is exactly the failure mode
here.

**2. Dynamic foreground — FAIL, and inverted.** A melody must be the loudest
sustained voice; here it is not even close. Its velocity median (88 / 81) is
level with the Pad (82 / 88) and *below* the HarmonicFill (101 / 94) — the
inner voice is the loudest sustained role on both renders. The realizer gives
the melody only a fixed `+2` bias (`src/chord_engine.rs:1372`) while the Pad
takes `−3` (`:1379`) and the Bass `−1` (`:1373`), but the HarmonicFill takes
*no* negative bias (`:1380` falls through `_ => {}`), so the fill floats up to
the top of the dynamic field. The saliency lift that would open a real gap is
the centered nudge `(prominence_w − 0.5) * PROMINENCE_VEL_SPAN`
(`src/chord_engine.rs:1390-1392`), and with `prominence_w == 0.5` on every step
that term is exactly 0. Craft verdict: the foreground voice is quieter than a
background voice. That is a melody you cannot hear as a melody.

**3. Rhythmic differentiation from the bed — FAIL.** A melody earns its
independence by moving *against* the accompaniment's pulse — different
subdivision, different onset placement, suspensions/anticipations that cross the
bar. Here the melody's own figures are selected from the *same* global
`edge_activity` scalar (`src/chord_engine.rs:1504-1515`) that gates every other
role, and its richest figure — the even 4-onset arpeggio `(0,156,312,468)`
(`:1912-1924`) — is *rhythmically identical* to the Pad's alberti/broken-chord
burst `(0,156,313,469)`. When the melody arpeggiates, it does so in lockstep
with the bed's broken-chord figure: same grid, same even quarter-subdivision.
Two voices sharing an onset grid fuse into one stream. The melody is not
counterpointing the accompaniment; it is *doubling its rhythm an octave up*.

**4. Phrase shape that reads as a singing line — FAIL on these images.** The
messa-di-voce swell (`src/chord_engine.rs:1339-1342`) and metric accent
(`:1351-1358`) are applied to *every* role identically, so they do not single
out the melody. Worse, the melody's hold-ms median is 83 / 62 ms with onsets up
to 750 ms — the "note-length extremes" signature. A singing line has a
*coherent* note-length profile across a phrase (a sung or blown phrase breathes
in long values, ornaments in short ones, but does not whiplash between 62 ms and
750 ms step to step). The melody here is mostly very short detached notes
(STACCATO_FRAC on the arpeggio/dotted figures, `:1922/:1932/:1941`) punctuated by
the occasional 750 ms hold — that is the gesture-profile of figuration, not of
a cantabile theme.

**Craft conclusion:** three of the four salience cues fail and the one that
passes (register) is the weakest. The theme voice is, in compositional terms,
an *inner voice that happens to sit on top* — it has the pitch real estate of a
melody and none of the foregrounding. When `example` swaps a real motif into
that voice (the 11 theme-active steps, `src/chord_engine.rs:1134-1145`,
`:2765`) and `Lena` free-selects the top chord tone (`:1248-1263`), the listener
cannot tell, because *neither* reads as a figure against the ground. You cannot
hear the difference between a real theme and a non-theme when the channel that
would carry that difference is muted into the texture.

---

## Q2 — Why do the two pieces sound like "the same piece in a different key"?

Because the accompaniment is the perceived identity of both pieces, and the
accompaniment is nearly invariant.

The craft principle: **a listener identifies a piece by its most salient,
most-repeated surface layer.** When that layer is the accompaniment rather than
the tune, two pieces with different thematic content but the same accompaniment
gait will be heard as the same piece. Here the most salient, most-repeated layer
is unambiguously the bed:

- **It is the loudest layer** (HarmonicFill at vel ~101/94 is the loudest
  sustained role; the Pad burst is dense and present), so the ear weights it
  most.
- **It is the most repetitive layer.** On `example` the Pad runs the *same*
  4-onset broken-chord burst on 32 of 42 steps; the Bass holds *one* sustained
  root on ~34 of 42; the HarmonicFill holds *one* sustained inner tone on ~41 of
  42. That is a near-ostinato bed — and ostinato is the single strongest
  identity-fixing device in music. The ear latches onto the repeating gait and
  calls *that* the piece.
- **It does not encode the theme at all.** The Bass, Fill, and Pad rhythms read
  only the global `edge_activity` scalar and the coarse per-section figuration
  profile (`src/chord_engine.rs:1635-1816`); none of them reads "is this the
  theme" or "how different is this image." So the layer the ear is using to
  identify the piece is *exactly* the layer that carries no per-image identity.

Now the specific "dotted-quarter→eighth→triplet + occasional triad arpeggio"
the operator hears every time: that is the *cross-product of two facts*. (a) The
Pad's even broken-chord burst `(0,156,313,469)` is the constant — it is on most
steps of `example` (`:1797-1799` → `figured_bed`, `:2278`) and reads as the
steady gait. (b) The melody's own three active figures — arpeggio (`:1912`),
syncopated (`:1925`), dotted (`:1934`) — are all riding on top of that bed at
the *same dynamic level and on the same onset grid*, so the ear does not parse
them as a separate tune; it folds them into the texture as rhythmic decoration
of the bed. The "dominant recurring rhythm" the operator describes is the
*fused* perception of (bed + same-level melody) — which is why it is the same
across both images: the bed is the same kind of even broken-chord gait, and the
melody, being fused into it, contributes only more of the same flavor of
subdivision rather than a competing line.

Harmony does not rescue this. Two pieces can have genuinely different chord
content and *still* collapse to "same feel" when (1) the harmonic rhythm is the
same (here it is — chords change on the same coarse section grid on both), (2)
the voicing register and spacing are the same (here they are — same role floors,
same close inner-voice spacing), and (3) the figuration that *arpeggiates* those
chords is the same even broken-chord pattern (here it is). Different *notes*
inside an identical *gait, spacing, and figuration* read as "the same thing,
transposed." That is precisely the operator's report, and it is correct.

The key/register offset (`example` lower, `Lena` higher) is real but is the
*least* informative difference to the ear — transposition preserves all the
relationships a listener uses for identity. So the one difference that survives
to the surface is the one that says the least: "same piece, different key."

---

## Q3 — Salience vs Accompaniment-variation: craft ranking

**Ranked #1: Foreground the melody (make the theme the heard subject).**
**Ranked #2: Vary the accompaniment per image.**

Reasoning, from the craft standpoint:

**Foregrounding is ranked first because it fixes the actual reported defect and
because of an asymmetry the trace makes explicit.** The operator's complaint is
not "the accompaniment is boring" — it is "I cannot tell the two pieces apart,
and I cannot hear a tune." Both of those are *salience* failures. A foregrounded
melody over a constant bed is a completely legitimate, fully musical texture —
it is the texture of nearly all song, of the solo concerto, of the lead sheet:
melody + comping. A strong tune over an ostinato bed is *Pachelbel, Boléro, half
the standards repertoire.* So lifting the melody, even over the *current*
unchanged bed, produces a musical result *and* makes `example` (real theme)
audibly diverge from `Lena` (no theme) — because once the melody is the subject,
the presence/absence of a real motif is the most salient fact in the piece. One
lever, both problems.

**The asymmetry that settles the ranking:** a varied bed under an *inaudible*
melody just gives you *different churn* — the pieces would still lack a heard
subject, and you would still be identifying them by their (now-varied)
accompaniment, which is a weaker basis for musical identity than a tune. You
would have traded "same churn" for "different churn" without ever producing a
foreground. Whereas a foregrounded melody over even a *constant* bed gives you a
heard subject in both pieces and lets the thematic difference land. Salience
strictly dominates: it is necessary (nothing else makes the tune audible) and it
is partly sufficient (it alone differentiates the images via theme
presence/absence). Accompaniment variation is a real and worthwhile improvement
— it deepens per-image character and breaks the ostinato sameness — but it is a
*second-order* refinement that only pays off once there is a foreground for it
to sit behind. Fix the figure first, then enrich the ground.

A practical note for whoever implements: once the melody is foregrounded, the
*rhythmic doubling* between the melody arpeggio and the Pad broken-chord burst
(both `(0,~156,~313,~469)`) becomes the next craft problem to watch — a
foreground line should not share an onset grid with the bed, or it re-fuses.
The prominence profile already addresses this: lifting the melody also *lowers
its rhythm-band cutoffs* via `prom_shift` (`src/chord_engine.rs:1910-1911`), so a
foregrounded melody subdivides on a *different* threshold than the bed and is
more likely to break the lockstep. That is a further reason salience is the
higher-leverage lever: it attacks register, dynamics, *and* rhythmic
independence in one move.

### Cheapest high-impact craft lever

**Activate the already-built `subject_melody` foreground profile — a data-only
change in `assets/mappings.json`.** The entire foregrounding machine exists and
is dormant only because a single selection gate fails: the `subject_melody`
prominence rule fires only when `subject_size ∈ [0.05, 0.55]` AND
`fg_bg_contrast ≥ 0.25`, and both test images pass the size gate but fail the
contrast gate (`example` 0.136, `Lena` 0.052, vs the 0.25 floor — trace Part B
§3 / `src/composition.rs:1543-1545`). Relaxing that `fg_bg_contrast` floor
toward ~0.10, or adding a non-empty default prominence profile so the system
never falls back to neutral, switches on the full lift in one JSON edit:

- Melody velocity weight 1.0 → the `(prominence_w − 0.5) * PROMINENCE_VEL_SPAN`
  nudge at `src/chord_engine.rs:1390-1392` becomes `+9` instead of `0`, so the
  melody finally clears the bed dynamically;
- Melody register lift `+2`;
- Lowered rhythm-band cutoffs via `prom_shift` (`src/chord_engine.rs:1910-1911`)
  so the melody subdivides more freely than — and on a different grid than — the
  bed, breaking the lockstep;
- Pad 0.3 / Fill 0.4 → the bed (and the over-loud HarmonicFill) recedes beneath
  the line.

That is the highest craft-yield-per-edit move available: it converts the top
voice from an inner voice that sits on top into an actual foreground subject,
and it does so without touching a line of the frozen kernel. A one-line
companion edit — give the HarmonicFill a small negative velocity bias in the
`realize_velocity` role match (`src/chord_engine.rs:1371-1381`, where Fill
currently falls through `_ => {}`) so it stops being the loudest role — is the
cheapest correctness fix to pair with it and is worth doing in the same pass.

---

## Verification basis

Citations checked against the current tree: `realize_velocity` role biases and
the prominence nudge (`src/chord_engine.rs:1313-1395`); the four melody rhythm
bands and the `prom_shift` term (`src/chord_engine.rs:1896-1962`); register
floors, theme/free-select melody pitch, and the realization dispatch as cited in
trace Part B. The empirical per-role velocity/pitch/rhythm medians are taken
from `docs/design-s42-trace.md` Part A (seed 42, pure-Rust default build). No
code was modified.

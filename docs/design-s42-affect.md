# S42 Affect Diagnosis — Perceptual / Cross-Modal Lens

DESIGN / DIAGNOSIS DOCUMENT. No production code changed. This reads from the
shared S42 ground-truth trace (`docs/design-s42-trace.md`), `assets/mappings.json`,
and `src/composition.rs` (the affect composite + planner). It answers, through the
image-feature → perceived-character lens, *why* the operator heard two pieces as
"the same piece in a different key" and could not tell theme-bearing from
theme-less — and names the smallest freeze-safe lever that fixes it.

---

## Headline

The operator's verdict is a **SALIENCE failure first, a SAMENESS failure second** —
and the two are causally linked, not independent. The melody is in the right
register but psychoacoustically pinned to the accompaniment plane, so the ear has
no figure to lock onto; with no foreground voice, *all that is left to compare
between two pieces is the bed*, and the bed is driven by coarse, weakly-separated
selectors that land both images on the same gait. Fix the salience and you both
(a) give the ear a melodic figure and (b) make the theme/no-theme distinction
suddenly audible — which is most of the "they're the same piece" complaint, since
the theme is the single largest per-image difference the trace found.

The proximate cause is one mis-scaled threshold: the prominence gate
`fg_bg_contrast ≥ 0.25` is calibrated against a feature whose real-photograph
distribution sits far below 0.25. That is the same real-image-clustering pathology
already flagged for the rhythm-cell band edges. It is a JSON-only fix.

---

## Q1 — Salience failure, sameness failure, or both?

**Both, but salience is primary and is the upstream cause of most of the perceived
sameness.** The trace gives the perceptual diagnosis directly:

- The melody sits in the **highest register band** of all four voices (median MIDI
  69 / 82 vs Fill ~61/64, Bass ~32/42). Register is *not* the problem — the figure
  is spectrally placeable.
- But the melody is only **+2 velocity** over a neutral bed, while the
  **HarmonicFill is the loudest voice** on both renders (vel median 101 / 94). The
  intended foregrounding lift (+9 velocity, +2 register, freer rhythm) is gated off,
  so the prominence nudge resolves to exactly 0.

Read against auditory **stream-segregation / scene-analysis** theory: a listener
forms a "figure" stream and a "ground" stream from differences in the cues that
drive perceptual grouping — primarily **onset asynchrony, loudness/level, pitch
separation, and rhythmic independence**. The melody currently shares the bed's
metric grid, sits *at or below* the bed in level, and has no independent
articulation envelope. Pitch height alone is a weak segregation cue when every
other cue says "same stream" — the auditory system happily folds a high voice into
the texture if it is no louder, no rhythmically freer, and onset-aligned with the
chords. So the melody is heard as the **top note of the chords**, not as a tune.
That is textbook *failure to segregate a figure*: there is a melody in the signal,
but there is no melody in the percept. This is why the operator — a professional
ear — could not even find a line to evaluate, let alone judge whether it bore a
theme.

The sameness failure is real but **downstream**. The only large per-image
differences the trace isolates are (a) key/register offset, (b) one Pad figuration
choice (animated broken-chord bed vs plain block triad), and (c) theme presence in
the melody pitch content. Of these, (a) is exactly "a different key," (c) is
inaudible *because of the salience failure* (the theme rides at bed level so
swapping a real motif for a free-selected top note changes nothing isolable), and
(b) is a quiet inner-voice detail. Strip the salience failure away and (c) flips
from inaudible to obvious — the theme-bearing piece gains a stated, recalled,
foregrounded line and the theme-less piece does not. So a large fraction of the
sameness verdict is *salience failure wearing a sameness costume*.

**Verdict:** the melody cannot currently be heard as figure-vs-ground. That is the
load-bearing defect. Accompaniment sameness is a genuine secondary contributor but
is partly an artifact of having no figure to differentiate the pieces by.

---

## Q2 — The dormant prominence gate: miscalibration vs wrong feature

**It is a miscalibration, not a wrong feature — but the feature is also a weak
choice, so the right fix does BOTH: relax the threshold now, and stop letting that
single feature be the sole gate.**

First, the threshold. `fg_bg_contrast` is computed (`pure_analysis.rs:668`) as a
clamped sum of three normalized absolute differences between the single argmax
"subject" cell and the border ring: `|Δvalue|/100 + |Δsaturation|/100 + |Δedge|`.
For that to reach 0.25, a 3×3-binned natural photograph would need the most-salient
cell to differ from the *averaged border ring* by a large margin across value,
saturation, and edge energy combined. Real photographs — and especially a tightly-
framed portrait like the Lena case — are **spatially autocorrelated**: adjacent
regions resemble each other, so cell-vs-border-ring differences cluster low. The
measured values (0.136, 0.052) are not outliers; they are where the real-photo
distribution *lives*. A 0.25 floor was set as if the feature ranged uniformly over
[0,1], but its effective working range on photographs is roughly the bottom
quartile. **This is the identical pathology flagged for the rhythm-cell band edges
(DP-A): a threshold calibrated against the theoretical range, not the realized
real-image distribution, so the interesting branch never fires.** Same disease,
different table.

Second, the feature choice. Even recalibrated, `fg_bg_contrast` is doing two
unrelated jobs in the engine. On the affect side it is (correctly, at LOW–MEDIUM
confidence) a *valence* nudge via processing fluency — "the subject pops, the image
is easy to parse, mildly pleasant." It has **no grounding as the gate for whether
to foreground the melodic voice.** Whether a melody should be lifted above the bed
is a near-universal property of tonal music with a tune, not a property of how
visually separable the photo's subject is. Gating "should there be a melody at all"
on a fragile, low-distribution image statistic is why the system has no tune on
*any* normal photo.

From the affect/feature standpoint, what *should* drive melody prominence:

- **The default should be FOREGROUND, not uniform.** A non-empty default
  prominence profile (mild melody lift) is the perceptually correct floor: tonal
  music has a figure. "Uniform / no figure" should be the *rare, earned* state
  (e.g. genuinely textural/ambient images), not the default both real photos fall
  into.
- **Modulate the *degree* of lift** with an arousal-adjacent feature if you want
  per-image variation in how assertively the tune sits forward — `subject_energy`
  or the `arousal` composite are the defensible drivers (more energetic image →
  more assertive, more present melodic line; HIGH-confidence arousal→dynamics +
  arousal→register links). `fg_bg_contrast` can stay as a *secondary* booster
  (a genuinely subject-dominant image earns extra lift) but must not be the gate.

So: relaxing the gate is necessary and correct, but the cleaner perceptual fix is
to **make foreground the default and demote `fg_bg_contrast` to a modulator** rather
than leave a single fragile threshold guarding the entire foregrounding system.

---

## Q3 — Ranking + the smallest freeze-safe lever

**Ranking for PERCEIVED per-image distinctiveness: SALIENCE ≫ ACCOMPANIMENT-VARIATION.**

1. **Salience (foreground the melody) — rank 1, decisive.** It does triple duty:
   it gives the ear a figure (fixing the "I can't find a line" problem), it makes
   the theme/no-theme difference audible (fixing "I can't tell them apart" — the
   single largest per-image structural difference becomes a *heard* difference),
   and a foregrounded melody naturally carries the per-image pitch/contour
   variation the bed currently swallows. One lever, three perceptual wins.
2. **Accompaniment variation — rank 2, secondary.** Making two images pick visibly
   different Pad figures / bass gaits adds texture-level distinctiveness, but it
   varies the *ground* while the ear is still searching for a *figure*. Worth doing
   after salience as polish, not as the primary fix. On its own it would leave the
   "same feel, different decoration" impression largely intact, because the
   foreground is what the ear weights for "is this a different piece."

**Smallest freeze-safe lever (the pick): a `mappings.json`-only change to the
`composition.prominence` SelectTable that makes melody foreground the DEFAULT and
relaxes the gate.** No Rust, no schema change — it rides the existing
`prominence` / `prominence_catalogue` shape the loader already parses, and the
frozen realization kernel only *calls* the resolved weights. Concretely:

**(a) Add a mild always-on foreground profile to `prominence_catalogue`** (a softer
sibling of `subject_melody`, so the default lift is present but not as aggressive
as the full subject-dominant case):

```json
{ "id": "melody_forward", "layers": [
    { "role": "Melody",        "weight": 0.80 },
    { "role": "CounterMelody", "weight": 0.55 },
    { "role": "HarmonicFill",  "weight": 0.35 },
    { "role": "Pad",           "weight": 0.30 },
    { "role": "Bass",          "weight": 0.50 } ] }
```

**(b) Change the `prominence` table so foreground is the floor and
`fg_bg_contrast` only *escalates* to full subject-dominant:**

```json
"prominence": {
  "default": "melody_forward",
  "rules": [
    { "when": [ {"knob":"subject_size",  "op":"in_range","lo":0.05,"hi":0.55},
                {"knob":"fg_bg_contrast","op":"ge","lo":0.10,"hi":0.0} ],
      "pick": "subject_melody" }
  ]
}
```

Rationale for the two numbers:
- **`default: melody_forward`** is the load-bearing change — it guarantees the
  melody lifts on *every* image (both S42 photos go from neutral 0.500 to a real
  +velocity/+register/freer-rhythm figure), which is the direct cure for the
  primary defect. `0.80` (vs the full `1.0`) gives a clear, musical figure without
  over-driving the contrast on images that genuinely are not subject-dominant.
- **`fg_bg_contrast ≥ 0.10`** recalibrates the escalation gate to the realized
  real-photo distribution. `example` (0.136) now *earns* the full `subject_melody`
  treatment; `Lena` (0.052) gets the milder `melody_forward` default. That is
  perceptually correct: `example`'s subject is more separable, so its tune should
  sit more assertively forward — and the two images now diverge in *how much* the
  melody leads, an additional per-image distinction the operator can hear, layered
  on top of the theme/no-theme difference that salience just made audible.

This single table edit (one new catalogue row + two value changes) turns on the
entire dormant foregrounding system for both images, makes them diverge in melodic
prominence, and makes the theme/no-theme difference perceptible — the largest
perceptual return per unit of change, with zero engine risk.

**A close, optional second** (only if a one-line Rust touch is acceptable): give
`HarmonicFill` a small negative velocity bias in the `realize_velocity` role match
(`src/chord_engine.rs`, freeze-safe per the trace), since the Fill is currently the
*loudest* voice and is the single biggest competitor masking the melody. The
prominence-default change already recesses Fill via its `0.35` weight, so this is
belt-and-suspenders, not required. Recommend shipping (a)+(b) first and re-listening
before touching any Rust.

---

## Pure-feature vs ML line (honest, per effect)

- **Foregrounding the melody at all:** REACHABLE NOW — pure JSON, no new feature.
  It is a synthesis/orchestration choice (lift the figure over the ground), not an
  image-understanding problem.
- **Per-image variation in how assertively the melody leads:** REACHABLE NOW — from
  `fg_bg_contrast` / `subject_energy` / the `arousal` composite, all already
  computed pure-feature fields.
- **Making theme/no-theme audible:** REACHABLE NOW — it is a *consequence* of
  foregrounding; the theme content already exists, it just was not lifted.
- **Truly robust figure-ground saliency (knowing *what* the subject is, so the
  melody can track the real subject):** NOT reachable in pure features. The current
  `fg_bg_contrast` is a cheap value/saturation/edge proxy on a 3×3 grid; a real
  saliency/segmentation model is an opt-in ML tier, later. The lever above does not
  need it — it works on the existing proxy by recalibrating to that proxy's real
  distribution and defaulting to foreground.

---

## Risks / caveats

- **Load-bearing caveat preserved:** this change touches *prominence/orchestration
  only*. It does NOT touch the major/minor decision — valence still owns mode; hue
  stays a within-family garnish. No regression to the C6.6 valence-owns-the-third
  discipline.
- **`melody_forward` weights are tuned by ear, not from a landmark study.** The
  `0.80 / 0.55 / 0.35 / 0.30 / 0.50` split is a judgment call sized between neutral
  (0.5) and the existing full `subject_melody` profile; re-listen and adjust the
  Melody weight up/down if the figure is too timid or too shouty. The *direction*
  (melody loudest, Fill/Pad recessed below neutral) is HIGH-confidence
  (loudness/level is a primary stream-segregation cue); the exact magnitudes are
  ear-tunable.
- **The `fg_bg_contrast ≥ 0.10` floor is fitted to a two-image sample.** 0.10 cleanly
  splits the two S42 photos (example earns full, Lena gets default), but it should
  be re-checked against a wider image set so the full `subject_melody` escalation
  fires for genuinely subject-dominant images and not for cluttered ones. If only
  one threshold can be trusted, the **default-to-foreground change is the
  load-bearing half** and is robust on its own; the escalation gate is the
  refinement.
- **Do not also relax the *other* `fg_bg_contrast ≥ 0.25` gates** (key_scheme,
  texture, form) in the same pass without separate justification — those guard
  different musical decisions and have their own realized-distribution question.
  This lever is scoped to the prominence table only; widening the others is a
  separate, evidence-gated change.
- **Single-writer coordination:** these are affect/prominence rows in the shared
  `assets/mappings.json`. They are handed to the lead as a self-contained spec for
  the single writer to merge alongside the harmony tables — not committed here.
  `// TODO(S42): merge melody_forward profile + prominence default/gate change into
  mappings.json (coordinate with the harmony-table writer).`

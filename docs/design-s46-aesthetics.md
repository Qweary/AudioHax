# S46 Aesthetics Design — Figure-Ground as the IMAGE-Justified Lead (differentiate first, level last)

DESIGN / ASSESSMENT DOCUMENT. No source, test, or asset modified. This is the
composition & songwriting-aesthetics lens on the operator's S46 verdict: the
melody **"doesn't feel like the melody"** — it moves less than the other roles,
may not sit on top, the background takes "more main focus than the IMAGES
justify," cross-role balance is off, and the rhythms are stale and same across
roles. Read as one problem: **the piece has no clear FIGURE, and the GROUND has
crept into the foreground.** S42/S43 found the melody was buried by LEVEL
(velocity); S44 found the texture is flat across the form; S46 is the *next*
defect in that exact sequence — **the melody is buried by DIFFERENTIATION** (it
isn't the most active, isn't reliably on top, isn't rhythmically separated from
the bed), and turning it up cannot fix that.

The lens: figure-ground is a **songwriting/arrangement value**, not a mixing
parameter. "One thing in front, everything else supporting it" is what makes a
piece read as composed rather than as an undifferentiated wash of equal voices.
This document answers: *what does the image justify as the lead, how much should
the bed recede per image, how should the figure-ground breathe across the form,
and what should the scorecard reward so it does not encode a rigid "melody always
loudest/highest/busiest" that would itself sound mechanical.*

The trained-ear gate is the standard: the operator hears when no voice is in
charge, when a "melody" is just the top note of the chords, and when a louder
melody over an equally-busy bed is still mush.

---

## 0. The binding frame, restated as the design's spine

The lead's load-bearing counterpoint governs this whole document, so it is the
first thing stated, not a caveat at the end:

> **The fix is NOT "turn the melody up."** Differentiation — melody MOVES MORE
> than the bed + the bed RECEDES IN ACTIVITY + each role has its own RHYTHMIC
> IDENTITY — is the first 90%; level is the last 10%. **A loud melody over an
> equally-busy bed is still mush.** A high voice projects on its own; a low or
> inner figure needs the NON-LEVEL tools — articulation, timbre, rhythmic
> separation, register placement — because "turn it up" is the weakest of them.

And the lead's second counterpoint, which this design must NOT walk back:

> **The S45 CounterMelody is now an active background competing for figure.** Do
> NOT default to "remove S45." Resolve it as HIERARCHY — the counter recedes
> RELATIVE to a lifted melody, preserving S45's gain. Hard per-image counter
> recession only as a deliberate per-image hierarchy decision.

Three grounded facts from the code make this frame concrete and falsifiable, and
they are why this design exists:

1. **The melody's activity is gated by ONE global scalar; the counter's is
   GUARANTEED.** The Melody rhythm arm (`src/chord_engine.rs:1927–1974`) selects
   arpeggio/syncopated/dotted/sustained purely by `edge_activity` (shifted by a
   tiny `prom_shift`, `:1941–1942`). On real photographs `edge_activity` is low,
   so the melody falls into the **DOTTED or SUSTAINED** band — it barely moves.
   Meanwhile the CounterMelody MOVING mode (`:1899–1907`) fires a **GUARANTEED
   off-beat onset** *exactly when the melody is static* (`held_chord ||
   melody_static`, `:1899`). So on a calm image the bed-counter moves while the
   melody holds — **the background out-articulates the figure.** This is the
   operator's signal (2) and (7) in one mechanism, present in the tree. [CE]

2. **The melody is "on top" only by register FLOOR, with no guard that keeps it
   there.** Melody seats at `MELODY_REGISTER_FLOOR = 67` (G4, `:1222, :1271`);
   the CounterMelody, Pad, and HarmonicFill all seat at `FILL_REGISTER_FLOOR =
   55` (G3, `:1221, :1284–1310`). The melody's only upward prominence tool is
   `PROMINENCE_REG_SPAN = 4.0` → at most **±2 semitones** (`:1005, :1270`) — a
   token nudge. The counter is a *moving* contrapuntal line with no ceiling: it
   selects contrary/oblique chord tones (`:1894`) and can cross UP into or above
   the melody's register on any step. **Nothing enforces melody-on-top.** This is
   the operator's signal (3). [CE]

3. **Cross-role level is fixed bias + a centered nudge — no image-conditioned
   recession.** Velocity differentiation is the fixed `+2` Melody / `−3` Pad
   biases (`:1385–1392`) plus the prominence velocity nudge (`:1404`,
   `PROMINENCE_VEL_SPAN = 18.0`). The bed roles (HarmonicFill, CounterMelody)
   have **no negative velocity bias arm** — they fall through `_ => {}` (`:1393`)
   — so a background voice can float to the top of the level field, exactly the
   S42 finding. And the recession is one global `melody_forward` profile
   (`assets/mappings.json:373–378`), not derived from how much FIGURE the image
   justifies. This is signals (5) and (6). [JSON]+[CE]

The aesthetic conclusion that organizes everything below: **the next win is not a
louder melody — it is a melody the image JUSTIFIES as the lead, made the figure
by ACTIVITY and RHYTHMIC IDENTITY and REGISTER first, with the bed (including the
S45 counter) receding by an image-conditioned depth, deployed across the form so
the figure-ground breathes.** Level is the last, smallest lever.

---

## 1. WHICH ROLE THE IMAGE JUSTIFIES AS THE LEAD — the image→lead-role mapping

This is the operator's core question (signal 5): the backgrounds are "more main
focus than the IMAGES justify." That phrasing is precise and load-bearing — it
says the lead should be **what the image content justifies foregrounding**, not a
fixed rule that the melody is always the figure.

### 1.1 The songwriting principle: there is always ONE figure, but not always the same one

In any composed piece there is **one thing in front** — the voice the ear is
meant to follow — and everything else supports it. That is the figure-ground
contract, and it is what an undifferentiated equal-roles texture violates (§3).
But the *figure need not always be the melody.* A great deal of pleasing music
foregrounds something other than a tune:

- a **pulsing/ostinato bed** is the figure in minimalist, ambient, and
  groove-driven music (the texture itself is the subject);
- a **single sustained pad/drone** is the figure in an atmospheric piece (the
  field, not a line, is what you attend to);
- a **moving inner counter-line** can be the figure in a contrapuntal texture.

The arrangement value is *that there is a clear figure*, not *that the figure is
the melody.* So the right design is an **image→lead-role mapping**, not a global
"melody always wins."

### 1.2 The mapping — what the image content justifies as lead

The image's saliency + affect knobs already in the engine
(`subject_size`, `fg_bg_contrast`, `subject_energy`, `foreground_energy`,
`background_energy`, `arousal`, `valence`) carry exactly the information that
decides this. The aesthetic mapping:

| Image character (knob signature) | Reads as | IMAGE-justified lead | Why |
|---|---|---|---|
| **A clear SUBJECT** — sized, separated subject (`subject_size` mid-range, `fg_bg_contrast` high, `subject_energy` present) | "there is a thing I am looking AT" | **Melody is the figure**, strongly. | A subject image has a focal point; the music's focal point is the tune. This is the portrait/object case — the melody should lead clearly and the bed recede hard. |
| **A TEXTURE / FIELD** — low subject separation, even energy distribution (`fg_bg_contrast` low, `background_energy` ≈ `foreground_energy`), abstract/ambient | "there is no one thing — I am looking at a whole" | **A more EVEN texture; the figure may be a pulsing bed or a counter-line, not a spotlit melody.** | A field image has no focal point to spotlight; forcing a loud lead tune onto it is the inverse error — a figure the image does not justify. A clear-but-gentle melody over a present bed, or a foregrounded pulse, reads truer. |
| **A BUSY / panoramic / high-energy SCENE** — high `foreground_energy`/`background_energy`/`arousal`, low subject dominance | "lots going on, energetic, the eye travels" | **Melody leads but the bed is allowed to be ACTIVE** (the counter blooms) — a richer figure-ground with more competition, resolved by the melody being clearly MORE active still. | A busy image justifies an active bed; the discipline is that the melody must out-move it (the hierarchy, not a quiet bed). |

The throughline, in the operator's own terms: **the bed should take exactly as
much focus as the image justifies — no more.** A subject image → strong melody
lead, bed recedes hard. A field/abstract image → a more even texture is *correct*
(the bed legitimately shares focus). The amateur failure today is that EVERY
image gets the same near-even texture, so the subject images sound under-led and
the field images get a melody that's trying-but-failing to lead. **The fix is to
make the lead and the recession track the image, not to bolt a louder melody onto
everything.**

### 1.3 Where it lands in code

The lead-role decision is a SELECTION decision and belongs at the same seam as
the existing texture/prominence selection — **the per-plan selection in the
planner** (`composition.rs:1517` texture, `:1543` prominence), driven by the
saliency/affect knobs the `SelectTable` already reads. Concretely:

- The image→lead-role mapping is naturally expressed as a **richer prominence
  table** (`assets/mappings.json:365–387`): not one `melody_forward` default for
  all, but a set of prominence profiles selected by the saliency/affect knobs —
  a `melody_lead_strong` (subject image: high melody weight, hard bed recession),
  a `melody_lead_gentle` (field/abstract: clear-but-not-shouting melody, bed only
  mildly recessed — the present texture), and the existing `subject_melody` as
  the top escalation tier. **[JSON]** — this is the freeze-safe primary lever; it
  reuses the prominence machinery the realizer already consumes.
- The one piece prominence ALONE cannot express — *which role is the figure* when
  the figure is NOT the melody — is a later, larger move (a field image whose
  figure is the Pad/CounterMelody). For v1 the lead is always the Melody; the
  image-conditioned part is **how strongly it leads and how hard the bed
  recedes** (§2). A non-melody lead (foregrounded pulse for a pure texture-image)
  is a deliberate later slice, not v1 — flagged in §5. **[JSON]+[CE]** later.

---

## 2. HOW MUCH THE BED SHOULD RECEDE, PER IMAGE — image-conditioned recession depth

Signal (5) is not "recede the bed" globally; it is "the bed takes more focus than
*the IMAGES* justify." So recession must be **per-image**, derived from content,
not one global constant.

### 2.1 The aesthetic rule

- A **subject-rich image wants a clear single figure** → the bed recedes DEEP
  (the melody dominates; the counter and pad sit well under it). A portrait with
  one face should not have a busy competing inner line stealing the eye.
- An **abstract/ambient/field image tolerates a more even texture** → the bed
  recedes only SHALLOW (the texture legitimately shares focus; over-recessing it
  would hollow out the very thing the image is *about*). Forcing a hard
  figure-ground onto a field image is as wrong as forcing an even texture onto a
  portrait.

So recession depth is a **function of how much the image justifies a single
figure** — high `fg_bg_contrast` / clear subject → deep recession; low contrast /
even energy → shallow recession.

### 2.2 The knobs that drive recession depth, and the range

The recession driver should be the **same figure-strength signal that picks the
lead** — primarily `fg_bg_contrast` (how separated the subject is from its field)
modulated by `subject_size` and the foreground/background energy ratio. Mapped to
the bed roles' prominence weights:

| Image | Bed recession | Pad/Fill weight | Counter weight (relative to melody) | Why |
|---|---|---|---|---|
| **High fg_bg_contrast** (clear subject) | DEEP | ~0.30 (toward the `subject_melody` 0.3) | well below melody (e.g. 0.45 vs melody 0.90) | one figure; the bed is pure support |
| **Mid fg_bg_contrast** (some subject) | MEDIUM | ~0.40 (the current `melody_forward`) | mildly below melody (≈0.55 vs 0.78) | the present two-tier middle |
| **Low fg_bg_contrast** (field/abstract) | SHALLOW | ~0.45 (barely recessed) | near-even with melody (≈0.65 vs 0.72) | the texture legitimately shares focus; do not hollow it |

**Range:** bed-role weights span roughly **0.30 (deep recession) → 0.45 (shallow
recession)**, all staying `> 0.25` (the S42/S44 "bed recedes, does not vanish"
floor). The melody weight rises as the bed recedes deeper, so the GAP — the
figure-ground contrast — is what the image actually drives, not the melody level
alone.

### 2.3 Where the lever lives

This is the **prominence table** again (`assets/mappings.json:365–387`): the
recession depth IS the bed-role weights inside each image-selected prominence
profile. The existing `prominence` SelectTable (`:380–387`) already picks a
profile from `subject_size`/`fg_bg_contrast`; the v1 lever is to **add the
gentle/strong tiers and route them by the figure-strength knob** so the three
recession depths above are reachable. **[JSON]** — freeze-safe, no Rust. The
realizer already consumes the resolved weights at `chord_engine.rs:1308`
(register), `:1404` (velocity), `:1942` (rhythm-band shift).

**The one Rust-reachable gap (signal 4 / the non-level tools).** Prominence today
gives the bed only a *level* recession (velocity nudge) plus a tiny register
nudge and a melody-only rhythm shift. The operator is explicit that level is the
weakest tool. The deeper, freeze-reachable lever — deferred past v1 but flagged
here — is to extend the bed's recession to the **NON-LEVEL** dimensions:
articulation (the bed plays legato/sustained while the figure is more separated),
register (a real melody-stays-above guard, §3.3 / signal 3), and rhythmic
separation (the figure's onset grid is distinct from the bed's). These are [CE]
realizer arms, freeze-reachable, and they are where the operator's "turn it up is
weakest" insight cashes out. See §5.

---

## 3. FIGURE-GROUND AS A SONGWRITING / ARRANGEMENT VALUE

This section ties the mechanism to the operator's "doesn't feel like the melody"
(signal 1) — the most important and most subjective of the seven.

### 3.1 Why an undifferentiated equal-roles texture reads as amateur

The single most reliable marker of a *composed* piece versus a generated wash is
that **one voice is clearly in charge and the others know they are supporting.**
Listeners do not parse a texture by analyzing it; they lock onto the most salient
strand and hear everything else as frame. When no strand is most salient — when
the melody, the counter, the pad, and the fill all move about the same amount, at
about the same level, in about the same register band — the ear has **nothing to
lock onto**, and the result reads as amateur not because any note is wrong but
because **no voice is leading.** This is the precise content of the operator's
"doesn't feel like the melody": there IS a melody-role line, but it does not
*behave* like a melody — it doesn't move more, doesn't sit clearly on top, isn't
rhythmically distinct from the bed. A melody is not a register assignment; it is
a *behavior* (the most active, most separated, most-attended line). Today the
melody-role has the assignment without the behavior.

### 3.2 The arrangement principles, stated as the design's values

- **One thing in front.** Every section has exactly one figure. Equal salience
  across roles is the defect, not a neutral default.
- **Foreground/background CONTRAST is the value, not foreground LEVEL.** The
  figure is defined by how it DIFFERS from the ground — more motion, distinct
  rhythm, clear register, then level — not by how loud it is in isolation. (This
  is the operator's 90/10.)
- **Supporting roles support.** The bed's job is to make the figure legible: hold
  the harmony, keep time, recede. A bed that competes for attention is a bed
  doing the wrong job — which is exactly what the S45 counter risks (§3.4).
- **Arrangement clarity.** The average texture should be CLEAR — you can always
  find the figure. Density is spent deliberately (at a climax), not run flat.

### 3.3 The melody-on-top question (signal 3) as an arrangement value

"The melody may not be the highest voice" is an arrangement defect, not just a
register bug. In the overwhelming majority of pleasing arrangements the lead is
the **top sounding line** — height is a primary figure cue, and a lead that dips
below an inner voice momentarily loses the figure. The S45 counter is a *moving*
inner line seated at `FILL_REGISTER_FLOOR = 55` that selects contrary/oblique
tones and can rise above the melody on any step (§0 fact 2). **The arrangement
value: the figure stays on top.** The aesthetic resolution is NOT to silence the
counter but to **cap the counter's ceiling below the melody's floor** — a
register guard so the counter weaves UNDER the lead, never crossing it. This
preserves S45 (the counter still moves, still fills the empty periods) while
restoring the melody-on-top contract. [CE], freeze-reachable (a counter-register
ceiling in the counter arm, byte-neutral on identity which has no counter).

### 3.4 Resolving S45 as HIERARCHY, not removal (the lead's counterpoint, honored)

The operator's verdict that the background is too prominent is, in part, a verdict
on the S45 counter: it is now an active inner line competing for figure (it
out-moves the static melody, §0 fact 1, and can sit at the melody's height, fact
2). The naive fix is to walk back S45. **This design refuses that** and resolves
it as hierarchy:

- The counter recedes **RELATIVE to a lifted melody** — its weight stays below
  the melody's in every prominence profile (§2.2), so the gain S45 bought (a
  moving inner line that fills empty periods) is preserved while the melody
  becomes unambiguously the figure.
- The counter is held **UNDER the melody in register** (§3.3) and **less active
  than the melody** (§4 / the metric in §5) — so it is heard as a supporting
  second strand, not a co-lead.
- **Hard per-image counter recession** (dropping the counter weight aggressively,
  or routing back to the static `pad_bed` Fill) is reserved for a *deliberate
  per-image hierarchy decision* — a subject-image so focal that even a quiet
  counter steals from it. That is a per-image call (the deep-recession tier in
  §2.2), not a global walk-back.

The aesthetic statement: **S45 gave the bed a moving inner line; S46 puts that
line in its place in the hierarchy.** A moving counter UNDER a more-active melody
is richer than a static fill; a moving counter level with a static melody is the
mush the operator hears.

---

## 4. THE FIGURE-GROUND BALANCE ARC ACROSS THE FORM

Building on the S44 texture-ARC finding (`docs/design-s44-aesthetics.md` §1):
figure-ground is not a constant any more than texture-density is. It should
**breathe across the section roles**, and the S44 arc and this S46 arc are the
same departure-and-return shape seen from two angles.

| Role | Figure-ground posture | Why it is more pleasing |
|---|---|---|
| **Statement (A)** | Figure clearly established; bed lean and supportive. | The ear must *learn the figure* before the piece can play with it. A clear lead from bar one tells the listener what to follow. |
| **Contrast (B)** | The figure-ground may RELAX or HAND OFF. The bed can bloom slightly (the counter more active), OR the lead can hand to a different strand for the departure, so the return of the melody-lead feels like a homecoming. | Contrast in the FIGURE (who leads, how hard) is a real contrast device, not only key/density. A B where the bed shares more focus makes the A′ return of a clear lead feel like coming home to the tune. |
| **Return (A′)** | Figure-ground CLEAREST and the bed may bloom in DENSITY beneath it — the climax is a full bed UNDER an unmistakable lead, not an equal-voices tutti. | The return is where the lead should feel most *owned*. The bed blooms (S44) but the figure-ground GAP must WIDEN with it, or the climax becomes the mush — a loud everything. The climax is "the biggest the bed ever gets while the lead is still clearly in front." |
| **Coda** | Strip to the figure + minimal support; the bed recedes furthest. | The ending settles onto the lead alone (or lead + bass), so the final line is the last thing heard. |

The load-bearing arc value, which directly answers the S44→S46 progression: **the
bed may bloom at the climax (S44), but the figure-ground GAP must bloom WITH it.**
The single most dangerous interaction between the two designs is a climax that
adds the counter + thickens the bed (S44's "fullest Return") and thereby BURIES
the lead (S46's defect) at the very moment the lead should feel most owned. The
arc resolves this: **at the climax, the bed gets denser AND the melody's
figure-ground lead gets STRONGER** — the gap widens as the texture thickens.
A climax is the fullest the bed ever gets *while the lead is still unmistakably in
front.* (Carry-forward of the S44 §6.1 "climax bloom must not break the cadential
homecoming," now generalized: the climax bloom must not break the figure-ground.)

The figure CAN hand off (B section) as a deliberate contrast — that is a richer
arrangement than a melody that never yields — but it must hand BACK, and the A′
must restore the clear lead. A figure that hands off and never returns is the
through-composed anti-pattern in the figure-ground dimension.

---

## 5. AESTHETIC INPUT TO THE FIGURE-GROUND METRICS — avoiding a worse, more rigid result

This is the operator's explicit warning (and the most important guard in the
document): the scorecard must NOT enforce a rigid "melody always loudest /
highest / busiest," because **that would itself sound mechanical** and would
encode the field-image error from §1 as a *requirement.* The metrics extend the
S45 variety scorecard (`docs/spec-s45-variety-metrics.md` §3/§4/§7) which already
measures per-layer motion; S46 adds the **cross-role figure-ground relationships**
— but conditioned, not absolute.

### 5.1 What the scorecard SHOULD reward — image-conditioned, relational, breathing

1. **Melody-is-the-figure, IMAGE-CONDITIONED (not absolute).** Reward the melody
   being MORE active than the bed roles **by a margin that scales with the
   image's figure-strength** — large margin required on subject images (high
   `fg_bg_contrast`), SMALL or zero margin permitted on field/abstract images
   (low `fg_bg_contrast`). The metric is a *relationship gated by a knob*, not a
   constant. *Encodable:* `melody_motion_fraction − max(bed_motion_fraction) ≥
   f(fg_bg_contrast)`, where `f` is large for subject images and ≈0 for field
   images. This directly answers signal (2) WITHOUT mandating a busy melody on a
   still image. **Falsifiable by ear:** on a subject image the melody should be
   clearly the most-moving line; on an ambient image it need not be.

2. **Melody-stays-on-top as a RELATIONAL guard, not a fixed pitch floor.** Reward
   the melody's sounding pitch being `≥` every concurrent bed voice's on each
   step (signal 3 / §3.3), allowing deliberate brief crossings only as flagged
   exceptions. *Encodable:* fraction of steps where `melody_pitch ≥
   max(counter_pitch, pad_top, fill_pitch)` is `≥ 0.95`. Relational (melody vs the
   actual concurrent voices), so it does not freeze the melody into a register.

3. **Figure-ground GAP exists, sized by image (not a fixed loudness ranking).**
   Reward a per-step level gap between the figure and the loudest bed role that
   **scales with figure-strength** — deep on subject images, shallow on field
   images, never inverted. *Encodable:* `min over accented steps of
   (melody_velocity − max(bed_velocity)) ≥ g(fg_bg_contrast)`, `g` ≥ 0 always,
   large on subject images. This is the S42 "audible figure/ground gap" guard,
   now image-conditioned so it does not demand a shouting melody on a quiet image.

4. **Per-role RHYTHMIC IDENTITY (signal 7) — distinctness, not a fixed busyness
   order.** Reward each role having a DISTINCT onset-grid/rhythm profile from the
   others (the figure separated from the bed; the bed roles distinct from each
   other), and reward between-section rhythm change (the flat between-section
   rhythm complaint). Do NOT reward a fixed "melody busiest > counter > pad"
   ranking — reward *differentiation*. *Encodable:* the S45 M1.3 onset-distinct
   metric generalized to all role pairs (each pair's onset grids differ on `≥
   0.4` of steps); plus a between-section rhythm-profile-changes check. **This is
   the single most important S46 metric** — it operationalizes the "rhythms stale
   / same across roles" complaint as a relational distinctness property, the kind
   the operator's 90% lives in.

5. **The figure-ground ARC breathes (not a constant maximum).** Reward the
   figure-ground gap CHANGING across sections (lean Statement, possible B
   relaxation/handoff, widest-gap climax, settled Coda — §4), NOT a flat-maximum
   gap on every section. *Encodable:* the per-section figure-ground gap is not
   constant across roles; `argmax(gap)` is a climax-eligible section. A
   flat-maximum gap is itself a defect (the rigid-result the operator warns
   against — a melody that shouts equally hard the whole way is as mechanical as
   one that never leads).

### 5.2 The metric-rigidity caution, stated for the synthesis to encode

The four metrics above are deliberately **relational and image-conditioned**, and
the synthesis must NOT collapse them into absolutes. The specific failure modes to
avoid:

- **"Melody always busiest" (absolute)** → would force a busy melody onto a still,
  ambient image (the field-image error of §1 made mandatory) and would make every
  piece sound restless. Use the *image-conditioned margin* (§5.1.1) instead.
- **"Melody always loudest by a fixed margin"** → re-encodes the "turn it up"
  weak-tool the operator dismissed and would make every piece front-load the
  melody at the same level regardless of image. Use the *image-conditioned gap*
  (§5.1.3) and weight the NON-LEVEL tools (rhythm-distinctness §5.1.4, register
  §5.1.2) MORE heavily than level in the rollup — mirroring the operator's 90/10.
- **"Counter always recedes hard"** → would walk back S45 by metric fiat. Reward
  the counter being UNDER the melody (relational), not the counter being quiet
  (absolute) — preserving S45's moving line.
- **A flat-maximum figure-ground on every section** → mechanical. Reward the ARC
  (§5.1.5), not a constant.

The rollup should weight **rhythmic distinctness + register relation (the
non-level cues) ABOVE level**, so a build that wins by "turning the melody up"
scores WORSE than one that wins by making the melody move more and sit clearly on
top. That weighting is the scorecard encoding of the operator's 90/10 and is the
single most important instruction for the synthesis: **the metric should reward
DIFFERENTIATION, and only then level.**

---

## 6. AESTHETIC VERDICT + THE PER-IMAGE LEAD / RECESSION RECOMMENDATION

### 6.1 Verdict

The operator is right on all seven signals, and they are ONE defect: **the piece
has a melody-role but no FIGURE.** The melody has the register assignment and the
level bias of a lead but not the BEHAVIOR of one — it moves less than the
guaranteed-moving counter (§0.1), is not guarded on top (§0.2), and is separated
from the bed only by a token level bias (§0.3). The S42/S43 lineage fixed level;
S44 fixed texture-shape; **S46 must fix figure-ground DIFFERENTIATION** — and the
operator's trap is exactly right: doing it by level alone would not fix it. The
fix is the melody made the figure by ACTIVITY, RHYTHMIC IDENTITY, and REGISTER
first, with the bed (including the S45 counter) receding by an image-conditioned
depth, deployed across the form so the figure-ground breathes.

### 6.2 The per-image lead / recession recommendation (for the eventual build)

- **Lead role, v1:** Melody is the lead on every image; the image-conditioned part
  is HOW STRONGLY it leads and HOW HARD the bed recedes (a non-melody lead — a
  foregrounded pulse for a pure texture-image — is a deliberate later slice, §5).
- **The image→recession mapping (the v1 [JSON] lever):** replace the single
  `melody_forward` default with a **three-tier image-conditioned prominence
  family** routed by figure-strength (`fg_bg_contrast` + `subject_size`):
  - **subject image (high contrast)** → `melody_lead_strong`: deep bed recession
    (Pad/Fill ≈0.30, counter ≈0.45 well under melody ≈0.90) — one clear figure;
  - **mid image** → the existing `melody_forward` middle (≈0.78/0.40/0.58);
  - **field/abstract image (low contrast)** → `melody_lead_gentle`: shallow
    recession (Pad/Fill ≈0.45, counter ≈0.65 near-even, melody ≈0.72) — the
    texture legitimately shares focus;
  - `subject_melody` stays the top escalation tier.
- **The non-level differentiation (the high-value [CE] lever, slice 2):** the bed
  recession must reach the NON-LEVEL tools — a **melody-on-top register guard**
  (cap the counter's ceiling below the melody's floor, §3.3), **rhythmic
  separation** (the figure's onset grid distinct from the bed's; the melody NOT
  out-moved by the counter on calm images, §0.1), and **articulation** (the bed
  more legato/sustained, the figure more separated). This is where the operator's
  "turn it up is weakest" cashes out and is the slice that actually makes the
  melody *feel like the melody.*
- **S45 preserved as hierarchy:** the counter stays moving, kept UNDER the melody
  in level, register, and activity in every tier — never walked back globally;
  hard counter recession only on the deep-recession (subject) tier as a deliberate
  per-image call.
- **The form arc:** the figure-ground gap widens at the climax AS the bed blooms
  (S44), so the climax is "the fullest bed under an unmistakable lead," never an
  equal-voices tutti.

### 6.3 Cross-lens dependencies (for the synthesis)

- **Music Theory lens.** (a) The melody-on-top register guard must not break
  voice-leading — the capped counter still needs legal contrary/oblique motion in
  its compressed register band (it owns `realized_counter_pitch_with_prev`,
  `chord_engine.rs:1894`). (b) Making the melody MORE active (so it out-moves the
  counter on calm images, §0.1) means the melody subdivides on a still image —
  Theory must confirm the freer melody still lands on chord tones and resolves at
  cadences (carry-forward of the S42 §6.2 watch-item; a more-active foregrounded
  melody exposes any non-chord-tone roughness). (c) The cadential homecoming must
  be heard IN the melody under a climax bloom (S44 §6.1, carried).
- **Affect / Cross-Modal lens.** (a) Owns whether the figure-strength signal
  driving recession is `fg_bg_contrast` alone or a composite with `subject_size`
  and the foreground/background energy ratio — the per-image recession depth is an
  affect/saliency call. (b) Owns the field-image case: whether a pure
  texture-image should keep a gentle melody-lead (v1) or eventually get a
  non-melody figure (a foregrounded pulse) — the §5 deferred non-melody-lead
  decision is Affect's to steer. (c) Must confirm the image-conditioned metric
  `f`/`g` curves (§5.1) match the perceptual reality — how much MORE active a
  melody must be to read as the figure is a perceptual constant Affect owns.
- **Rust Architect lens.** Owns where the non-level recession tools land
  (articulation/register/rhythmic-separation arms in the realizer), whether the
  melody-on-top guard is a counter-ceiling clamp in the counter arm or a
  cross-voice post-pass, and whether the image-conditioned metric gating is
  cleanest as a prominence-table expansion ([JSON], v1) vs a realizer change
  ([CE], slice 2). The freeze constraint: every part is a compose-path / realizer
  change; the identity profile (no counter, empty prominence) stays byte-neutral —
  `engine.rs` sha256 unchanged.

---

## Appendix — freeze tiers and no `mappings.json` rows authored

**Freeze tiers for the levers in this design** (the S44 legend):

| Lever | Tier | Freeze verdict |
|---|---|---|
| Three-tier image-conditioned prominence family (recession depth per image, §2/§6.2) | **[JSON]** | FREEZE-SAFE — additive catalogue rows + SelectTable rules; the realizer already consumes resolved weights |
| Melody-on-top register guard (counter ceiling below melody floor, §3.3) | **[CE]** | FREEZE-REACHABLE — counter arm only; identity has no counter → byte-neutral |
| Rhythmic separation / melody out-moves counter on calm images (§0.1, signal 2/7) | **[CE]** | FREEZE-REACHABLE — melody/counter rhythm arms; identity path unchanged |
| Articulation recession of the bed (non-level tool, §2.3/§6.2) | **[CE]** | FREEZE-REACHABLE — bed role arms; centered on identity no-op |
| Negative velocity bias for HarmonicFill/CounterMelody (level, §0.3) | **[CE]** | FREEZE-REACHABLE — `realize_velocity` role match, `!is_cadence` guarded (the S42 Edit-3 pattern) |
| Image-conditioned figure-ground metrics (§5) | test-only | not source — scorecard extension in `tests/` |
| Change default `num_instruments` | **[FROZEN]** | NOT proposed by this design |

This is a design/assessment document. It does **NOT** author `mappings.json`
rows: the v1 prominence-family rows (the deep/mid/shallow recession tiers and
their SelectTable routing) should be authored in the BUILD slice so their exact
weights and the figure-strength routing thresholds are tuned against the real
image set and the operator's ear — and so they go through the single-writer
coordination with the Music Theory lens (shared `mappings.json`) exactly as S42
did. The §6.2 weights (`0.30/0.45/0.90` strong, `0.45/0.65/0.72` gentle) are
*starting-value sketches* sized between neutral (0.5), the existing
`melody_forward` (0.78/0.40/0.58), and `subject_melody` (1.0/0.3/0.6); the
operator's trained ear is the calibration gate. Flagged here so the lead routes
the row authorship to the BUILD slice, not this design.

`src/engine.rs` stays BYTE-FROZEN at sha256
`e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261` — untouched
and not proposed for edit. Types/sites verified against `src/composition.rs`
(ThematicRole `:406`, OrchestrationProfile `:487`, prominence resolve `:1543`,
per-section figuration arc `:1609–1636`) and `src/chord_engine.rs`
(register floors `:1220–1222`, role_pitch register seating `:1248–1311`,
PROMINENCE_REG_SPAN `:1005`, realize_velocity role bias `:1384–1405`, melody
rhythm bands `:1927–1974`, CounterMelody MOVING-mode guaranteed off-beat onset
`:1899–1907`, the counter pitch path `:1894`) and `assets/mappings.json`
(texture_catalogue `:262–276`, texture SelectTable `:337–364`,
prominence_catalogue `:365–379`, prominence SelectTable `:380–387`).

---

*End of S46 aesthetics design. Design-only: no source, test, or asset modified.*

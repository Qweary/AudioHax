# S46 — Figure-Ground / Role-Balance (Music-Theory Lens)

DESIGN / ASSESSMENT DOCUMENT. No source, test, or asset was modified to produce
it. This reads the current tree through the composition-craft / orchestration /
performance-practice lens and answers the S46 problem the operator surfaced after
S45 routed the CounterMelody into the default texture: **the melody no longer
reads as THE melody.** The seven verbatim operator signals are confronted
directly in §0; the binding TRAP (this is NOT "turn the melody up") governs the
whole ranking.

The realization kernel `src/engine.rs` is **byte-frozen** at sha256
`e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261` and is **not
touched by anything proposed here** — every lever lands in `assets/mappings.json`
(selection/catalogue tables) `[JSON]`, `src/chord_engine.rs` (the realizer the
frozen kernel only *calls*) `[CE]`, or `src/composition.rs` (the planner) `[COMP]`.
Each lever below is tagged. `[FROZEN]` marks the one decision that would require
a kernel edit.

Evidence base: a direct read this session of `chord_engine.rs` (`role_pitch`
register floors `:1220-1313`, `realize_rhythm` per-role arms `:1647-1994`,
`realize_velocity` role-bias + prominence nudge `:1326-1408`, the prominence
constants `:985-1015`, the CounterMelody species engine `:1831-1925`,
`:4039-4172`, the register band constants `:1220-1222` + `:3478`), the S45
metrics spec (`spec-s45-variety-metrics.md`), the S44 theory lens
(`design-s44-theory.md`) and synthesis (`design-s44-variety-nvoice.md`), the S42
salience diagnosis (`design-s42-salience-diagnosis.md`), and the live
`assets/mappings.json` texture/prominence tables (`:262-388`).

---

## 0. HEADLINE — and the seven signals, mapped

**The governing finding.** S45 did the right thing — it woke the richest line in
the engine (the CounterMelody species voice) into the default texture. But the
engine has **no enforced figure-ground HIERARCHY**: the melody and the counter
are now two moving inner-adjacent lines competing for the foreground, and on the
dimensions that actually decide which line the ear hears as the figure —
**relative ACTIVITY, relative REGISTER, and rhythmic SEPARATION** — the engine
either does nothing, or actively works *against* the melody.

The single sharpest, most concrete defect: **the melody and the counter can
occupy the SAME register, and on a dark image the melody can sit BELOW the
counter.** The counter band is `[FILL_REGISTER_FLOOR(55), COUNTER_CEILING(67))`
(`chord_engine.rs:4048`, `:3478`); the melody floor is `MELODY_REGISTER_FLOOR =
67` (`:1222`) but is shifted by brightness `lift = round(bright_octaves*12)`
which ranges **−12..+12** (`:1263`), summed then clamped (`:1271`). On a dark
image (`bright_octaves < 0`) the melody floor can fall to `67 − 12 + prom_lift(≤2)
= 57` — **inside the counter's `[55,67)` band**, and possibly below the counter's
actual sounding pitch. The "melody on top" guarantee is not enforced; it is an
accident of brightness. This is operator signal (3) in code.

The second sharpest: **the melody is NOT structurally guaranteed to move more
than the accompaniment, and the one prominence lever that touches melodic
activity is too weak to matter and is applied to the wrong target.** The melody's
rhythm band is gated by `edge_activity` with a `prom_shift` of at most `0.5 *
PROMINENCE_RHY_SHIFT(0.10) = 0.05` (`:1941-1942`, `:1015`) — a 0.05 nudge on the
0.25/0.55/0.80 band cutoffs. Meanwhile the CounterMelody arm gives itself a
**GUARANTEED off-beat onset at `step_ms/4` on every held/static step**
(`:1899-1907`) — an unconditional, structural source of motion the melody has no
equivalent of. Under a held chord with a static melody (the common case on a calm
image), the counter MOVES (guaranteed onset, forced pitch change) while the
melody SUSTAINS (`:1974-1992`, the low-activity branch is one long tone). **The
background literally moves more than the foreground.** That is operator signals
(1), (2) and (5) in one mechanism.

### The seven signals, each mapped to a mechanism

| # | Operator signal | Mechanism in code | Dimension |
|---|---|---|---|
| (1) | melody "doesn't feel like the melody" | no enforced hierarchy on activity/register/rhythm — the emergent property of (2)–(7) | all |
| (2) | melody MOVES LESS than other roles | counter has a guaranteed off-beat onset + forced pitch-move (`:1899-1907`, force_move `:4089`); melody's low-activity branch is one sustained tone (`:1974-1992`); melody activity lever is +0.05 max (`:1941`) | **ACTIVITY** |
| (3) | melody may not be the highest voice | melody floor `67 + lift(−12..+12) + prom(≤2)` clamped (`:1263-1271`) can fall into / below the counter band `[55,67)` (`:4048`); no ordering invariant | **REGISTER** |
| (4) | register-aware prominence is INVERSE compensation (low line needs MORE help, via non-level tools) | the engine has NO register-aware compensation; prominence is a flat per-role weight (`prominence_catalogue` `mappings.json:365-378`) blind to where in its range a line actually sits | **non-level tools** |
| (5) | backgrounds more focus than images justify | counter activity is unconditional; Pad runs a figure on every step; HarmonicFill holds (loud, was the S42 over-loud role); no activity recession applied to bed roles relative to the melody | **ACTIVITY** |
| (6) | volume balance off across roles | prominence weights are static and were tuned (S43) for a melody-vs-Pad world, BEFORE the counter became a live competitor (`melody_forward` CounterMelody 0.58 vs Melody 0.78 — only +0.20 separation; `mappings.json:373-378`) | **LEVEL** (the last 10%) |
| (7) | rhythms stale/same across roles; flat between-section rhythm | one global `edge_activity` scalar gates EVERY role's rhythm band (`:1517-1528`) → all roles subdivide together (fuse); per-section density nudge is the only differentiator and is 0.0 on identity (`:1526`) | **RHYTHM IDENTITY** |

**The TRAP, restated as the binding ranking law.** Signal (6)/level is the *last*
10%. Differentiation — melody moves MORE (activity), melody on TOP (register),
per-role rhythmic identity (separation) — is the first 90%. A loud melody over an
equally-busy, register-overlapping bed is still mush. **Every slice below ranks
the non-level tools above the level tweak**, exactly as S42/S43 ranked salience
above accompaniment-shuffle. The `prominence_catalogue` weight retune (signal 6)
is real and worth doing, but it is *belt-and-suspenders on top of* the hierarchy,
not the fix.

**the lead's counterpoint, confronted (the S45-counter question).** The counter is NOT
the enemy and must NOT be re-suppressed or walked back. S45's win — a moving inner
texture under the melody — is correct and must be KEPT. The defect is that the
counter currently competes *as an equal* with the melody for the figure. The
resolution is a **HIERARCHY, not a revert**: the counter recedes in ACTIVITY and
REGISTER *relative to a lifted melody*, while keeping its moving-inner-texture
gain. Concretely (§4): the melody's guaranteed-activity floor must exceed the
counter's; the melody's register floor must be enforced strictly above the
counter ceiling; the counter's off-beat onset stays (it is the inner-texture
engine) but the melody gets its OWN guaranteed surface-rhythm advantage so it
out-moves the counter. Hard per-image counter recession (dropping the counter to
oblique-sustain) is a *deliberate per-image hierarchy call* for a specific
texture, never a blanket S45 revert.

---

## 1. MELODY PRIMACY AS A CRAFT PROBLEM

What makes a line read as THE melody — the *figure* — over an accompaniment is a
solved problem in composition and orchestration. It is NOT primarily loudness.
The ear segregates streams and assigns "foreground" by a rank-ordered set of
cues; loudness is near the *bottom* of that order for a sustained texture (it is
the weakest because a held bed can be loud and still recede). The cues that
actually fix the figure, in descending craft-effectiveness for AudioHax:

### 1.1 The melody must MOVE MORE than every accompaniment role (activity hierarchy)

This is the single strongest figure-ground cue and the one the engine most
violates. In any tonal texture — a Bach chorale, a Chopin nocturne, a pop tune
over comping — **the melody carries the most surface rhythm and the most contour
change per unit time.** The accompaniment's job is to be *more static* than the
tune: a held pad, a repeating comp cell, a walking-but-regular bass. The ear
locks onto the line that is *changing the most* and calls it the subject. This is
why a soprano can be the figure even buried in the middle of a four-part texture
(the chorale "leading line") — it moves while the others hold.

**Where it lives in the engine, and why it's inverted today.**
- The melody's surface rhythm is selected in the `OrchestralRole::Melody` arm of
  `realize_rhythm` (`chord_engine.rs:1927-1993`): four `edge_activity` bands
  (sustained / dotted / syncopated / arpeggio), shifted by `prom_shift`
  (`:1941-1942`). On a calm image (`edge_activity` low — the common real-photo
  case, since `edge_density` ≈ 0.005–0.05 normalizes low) the melody falls into
  the **SUSTAINED branch: one long tone** (`:1974-1992`).
- The counter, on that same calm/held-chord step, takes its **MOVING branch: a
  guaranteed off-beat onset + a forced pitch change** (`:1899-1907`, `force_move`
  `:4089`, `:4171`). It is *built* to move when the melody is static — that was
  the S18 "fill the empty periods" design, and it is correct as inner-texture
  craft. But it means **the only guaranteed mover on a calm image is the
  background line.**
- Contour rate: the melody's pitch is the **top chord tone every step**
  (`role_pitch` Melody arm `:1258-1262`) — so on a held chord the melody pitch
  does not even change, while the counter steps through chord tones. The melody's
  contour rate is *lower* than the counter's on held harmony.

**The craft requirement:** the melody must have a *guaranteed activity floor that
exceeds the counter's* — its own version of the counter's guaranteed off-beat
onset, so that whenever the counter would move under a static melody, the melody
ALSO moves, and moves *more* (more onsets, or a contour step). This is the
non-level foreground tool with the highest yield (§4 P-A, §5 slice 1).

### 1.2 The melody on TOP (highest sounding voice) — or its register compensation

The second strongest cue. The outer voices of a texture are perceptually
privileged (the "outer-voice salience" of voice-leading pedagogy — the ear tracks
the top and bottom lines most easily). A melody that is the **highest sounding
voice** is foregrounded by register alone; this is the default expectation of
tonal writing and the reason descants soar *above* the tune.

**Where it lives, and why it's not guaranteed.** §0 covered the inversion risk:
the melody floor (`:1271`) is `67 + brightness_lift(−12..+12) + prom_lift(≤2)`,
clamped to `[24,96]`; the counter sits in `[55,67)` (`:4048`). On bright images
the melody floats well above the counter (brightness lift positive). **On dark
images the melody floor can fall to ≈57, INTO and possibly BELOW the counter
band.** There is no invariant `melody_pitch ≥ counter_pitch` anywhere in the
realizer. The "on top" property is emergent and brightness-dependent, not
enforced.

**The craft requirement:** enforce `melody_floor ≥ COUNTER_CEILING` strictly
(the melody register floor must never resolve below the counter's ceiling),
*independent of brightness*. The brightness lift may raise the melody but must
never push it below the bed's top line. This is a `role_pitch` clamp change `[CE]`
(§4 P-B, §5 slice 1).

### 1.3 The foreground/background ACTIVITY hierarchy (the bed must RECEDE in motion)

The complement of 1.1: not only must the melody move more, the **bed roles must
move LESS** than they do today. Today three things make the bed too active:
- the counter's unconditional off-beat onset (correct for inner texture, but it
  must sit *below* the melody's activity, not at/above it);
- the Pad runs its figuration cell on a large fraction of steps (the S42/S44
  ostinato finding — figure on ~76% of steps);
- the HarmonicFill holds a sustained inner tone every step and was the *loudest*
  role pre-S43 (S42 §1, `design-s42-salience-diagnosis.md:33-47`).

A first-class accompaniment is *quieter in activity* than the line it supports.
The hierarchy the ear needs is: **the bed is the ground (low activity, regular,
recessive); the melody is the figure (high activity, irregular/syncopated,
assertive); the counter is a SECONDARY figure (moving, but demonstrably less than
the melody and in a lower register).**

---

## 2. THE INVERSE-REGISTER PROMINENCE CRAFT

The operator's signal (4) is a precise orchestration insight and the engine has
**zero** of it. State it carefully because it inverts the naive intuition:

> A high note self-projects (its register IS its salience); a line LOW in its
> range needs MORE help to read as the figure — and the help must come from
> NON-LEVEL tools, because "turn it up" is the weakest tool.

### 2.1 Why a high line self-projects and a low line needs help

Two acoustic/perceptual facts. (a) **The ear's outer-voice bias**: the highest
sounding pitch is tracked preferentially regardless of loudness — a top line is
foregrounded *for free*. (b) **Spectral masking and register crowding**: a line
sitting low in its range overlaps the harmonic-fill / counter / upper-pad
spectrum and gets masked; raising its level just makes a *louder muddy middle*,
because the masking voices are at the same pitch height. A trombonist knows this
bodily: a melody in the low-middle of the horn disappears into an ensemble unless
you *separate* it — by articulation, by rhythm, by getting it out of the section's
register — not by blowing harder.

So the compensation must be **inverse to register**: the lower the melody sits in
its range on a given step/section, the MORE non-level help it needs. The engine
currently does the opposite — its only foreground tool that scales is the
*velocity* prominence nudge (`:1404`), the weakest tool, and it is **register-
blind** (a flat per-role weight, `prominence_catalogue` `mappings.json:365-378`).

### 2.2 The non-level tools, ranked by effectiveness for AudioHax, with code sites

| Rank | Non-level tool | Why it foregrounds | Where it lands in code | Tier |
|---|---|---|---|---|
| **1** | **Rhythmic separation / distinct onset grid** | Two voices on one onset grid FUSE into one stream (the S42 melody+Pad finding, `design-s42-salience-diagnosis.md:43-49`); a melody on a DIFFERENT grid/onset-phase than the bed segregates instantly. The strongest non-level cue for this engine because its bed is grid-locked. | give the melody a guaranteed onset phase/density distinct from the counter's `step_ms/4` and the Pad's downbeat figure; generalize `prom_shift` into a per-role rhythm BIAS so the melody subdivides on a different threshold (`:1941`, `PROMINENCE_RHY_SHIFT :1015`) | **[CE]** |
| **2** | **Register placement / separation** | Getting the melody OUT of the bed's register band (≥ a clear margin above `COUNTER_CEILING`) is the outer-voice cue made reliable. The most direct fix for the inversion (§1.2). | enforce `melody_floor ≥ COUNTER_CEILING` strictly in `role_pitch` (`:1271`); optionally add a register-aware *boost*: when the resolved melody pitch is low in its band, lift it further (the inverse-comp made literal) | **[CE]** |
| **3** | **Articulation contrast vs the bed** | A melody that is *detached* (marcato/staccato) against a *legato* bed — or legato over a detached comp — pops by contrast. Articulation contrast is a pure timbral/envelope segregation cue, level-independent. AudioHax already has an articulation curve (`base_frac`, `:1574-1586`) but it is driven by `edge_activity` identically for all non-Fill roles — NO melody-vs-bed contrast. | give the Melody arm an articulation offset distinct from the bed (e.g. a touch crisper attack than the Pad's `PAD_OVERLAP_FRAC` legato, or a marcato accent on melody downbeats) — a per-role `base_frac` bias beside the existing Fill special-case (`:1580`) | **[CE]** |
| **4** | **Rhythmic activity advantage** (the §1.1 mover) | covered as the activity hierarchy — it is both a primary figure-ground cue (§1.1) AND a register-independent foregrounding tool, so it doubles here. | melody guaranteed-activity floor > counter's (`:1927`, beside the counter's `:1899`) | **[CE]** |
| **5** | **Timbre / role assignment** | The classic orchestration tool (give the tune to a contrasting timbre — oboe over strings). AudioHax routes all roles to one synth/MIDI program per the kernel; per-role program assignment is a `midi_output.rs` concern OUTSIDE this specialist's ownership and likely a frozen/seam concern. **Flagged as cross-lens (Architect), not proposed here.** | `midi_output.rs` (EXCLUDED from this specialist) | **[FROZEN?]** cross-lens |
| **6** | **Level (velocity)** | The LAST 10%. Real but weakest; widens a gap the other tools already opened. | `realize_velocity` role bias `:1384-1394` + prominence nudge `:1404`; `prominence_catalogue` weights `mappings.json:365-378` | **[CE]** + **[JSON]** |

**The headline of this section:** tools 1–4 are all `[CE]` realizer changes, all
freeze-reachable, and ALL rank above the level tweak. The engine's *only* current
foreground tool is #6, the weakest — and it is register-blind, so it cannot do
the inverse compensation the operator asked for. The inverse-register craft is
implemented by making tools 1–3 *scale with how low the melody sits*, but even
the un-scaled, always-on versions (a flat melody activity floor + a strict
register-above-counter clamp + a melody articulation contrast) fix the dominant
defect; the register-*aware* scaling is the refinement on top (§4 P-B note).

---

## 3. PER-ROLE RHYTHMIC IDENTITY

Operator signal (7): rhythms are stale/same across roles, and between-section
rhythm reads flat. This is a direct consequence of the engine's architecture.

### 3.1 Why a shared bed-grid kills figure-ground

`realize_rhythm` computes ONE `edge_activity` scalar (`chord_engine.rs:1517-1528`)
and every role's rhythm-band selection reads it. So when an image is busy, *all*
roles subdivide together; when calm, *all* roles sustain together. They share a
pulse — and **voices that share an onset grid fuse into a single stream** (the
foundational stream-segregation fact, and the documented S42 melody+Pad failure,
`design-s42-salience-diagnosis.md:43-49`). A texture where bass, fill, pad, and
melody all change on the same grid is heard as *one thick voice*, not as a
foreground-plus-accompaniment. Real textures **STRATIFY**: a sustained pad under
a regular walking bass under a syncopated melody — each layer on its own
subdivision.

The only existing crack in the shared grid is the melody's `prom_shift`
(`:1941-1942`) — a +0.05 cutoff offset. It is the right *idea* (a per-role rhythm
bias) but far too small and applied to only one role.

### 3.2 What distinct per-role rhythmic profiles should look like

The orchestration-correct rhythmic stratification, role by role:

| Role | Rhythmic profile (craft) | vs the engine today |
|---|---|---|
| **Bass** | The slowest, most regular layer: roots on strong beats, an occasional walking pickup. Defines the pulse FLOOR. | today: one sustained root/step (`:1706-1724`) — correct in spirit but never re-articulates on the strong beat (drone, not pulse — S44 §1.1). Walking/pedal exist but are unrouted. |
| **Harmonic fill / pad** | Regular, recessive, OFF the melody's onsets: held tones, or a comp cell on weak beats (the "and" of the beat / beats 2&4) so it fills *between* the melody's onsets rather than doubling them. | today: Fill holds every step (`:1729-1750`); Pad runs its cell on the DOWNBEAT (offset 0, `:1816-1824`) — i.e. ON the melody's strong-beat onsets, the worst phase for segregation. |
| **Counter** | A SECONDARY moving line: moves, but in a distinct onset phase from the melody (the off-beat `step_ms/4`) and demonstrably LESS than the melody (fewer onsets per step). | today: guaranteed off-beat onset (`:1899-1907`) — the phase is GOOD (off-beat ≠ melody downbeat), but its activity is not capped *below* the melody's, so it can out-move the figure. |
| **Melody** | The MOST active, MOST irregular layer: syncopations, dotted figures, the most onsets, anticipations that cross the bar. Its irregularity vs the regular bed is the figure cue. | today: shares the global `edge_activity` grid; in the low-activity branch it is ONE sustained tone — LESS active than the counter (`:1974-1992` vs `:1899-1907`). |

### 3.3 Between-section rhythmic differentiation

The "flat between-section rhythm" is the per-section density seam (`Section.density`)
being pinned at 0.5 (DENSITY_NEUTRAL) on identity, so the `(density − 0.5) * GAIN`
activity nudge (`:1526`) is 0.0. The S44 synthesis already routes this (drive
density off region energy, `design-s44-variety-nvoice.md` slice 2/§3 row
"Per-section density"). For S46 the relevant point is narrower: **the
between-section rhythm should also differentiate the FORM** — a busier Contrast
section, a thinner Coda — which is the texture-arc item, S44 slice 2. S46 inherits
that; it is not the figure-ground core but it is the same `density` lever (`[COMP]`).

### 3.4 Map to `realize_rhythm`'s role arms

The fix is to **break the single `edge_activity` grid into per-role onset
profiles**, concretely:
- Generalize the melody's `prom_shift` (`:1941`) into a per-role rhythm bias so
  each role's band cutoffs differ — the melody biased toward subdivision (lower
  cutoffs → more onsets), the bed roles biased away (higher cutoffs → plainer).
  This is the existing seam, widened from one role to all and from ±0.05 to an
  audible spread.
- Phase-separate the onsets: keep the counter on `step_ms/4` (`:1903`), keep the
  Pad figure but consider moving its comp cell OFF the downbeat for the
  non-figured bed, and give the melody the strong-beat onset so the foreground
  owns beat 1.
- All of this is in the per-role `match role` arms of `realize_rhythm`
  (`:1647-1994`), `[CE]`, freeze-reachable (identity path has no Pad/Counter/
  Melody instruments → byte-neutral).

---

## 4. THE ROLE HIERARCHY THE ENGINE SHOULD ENFORCE ON A STEP

This is the heart of the spec and the part the Test Engineer must be able to
encode. State the hierarchy as **ordering invariants per dimension**, each a pure
function of the realized step.

The figure-ground ordering, foreground-last (the figure wins every dimension):

```
                 ACTIVITY        REGISTER (sounding pitch)     PROMINENCE (level)
Bass        <    bed/fill   <    Counter   <    Melody    |   Bass ≤ Counter < Pad/Fill < Melody (level)
(low,            (regular,        (moving,        (most         (the LAST tiebreak;
 sparse)          recessive)       secondary)      active,       widens the gap the
                                                   highest)      other 3 dims opened)
```

The four encodable invariants, with the precise relation each must satisfy:

### I-1 — MELODY-IS-MOST-ACTIVE (the load-bearing invariant)

**Relation:** over a render, the melody's onset count (and contour-change count)
per sounding step must be **≥ every other role's, and strictly > the counter's on
the steps where both sound.** Concretely the Test Engineer can encode, per step
where the melody sounds: `melody_onsets(step) ≥ counter_onsets(step)` with strict
`>` on at least a documented majority of co-sounding steps; and over the piece,
`mean_melody_onsets_per_step > mean_counter_onsets_per_step` and
`> mean_pad_onsets_per_step`. DETERMINISTIC (onset counts are the rhythm template,
RNG-free — see `spec-s45-variety-metrics.md` §0.2). **Today this FAILS**: on a
calm/held step the melody emits 1 sustained onset (`:1974-1992`) while the counter
emits its guaranteed off-beat onset and the Pad emits its multi-onset figure —
the foreground is the *least* active.

### I-2 — MELODY-IS-HIGHEST (register ordering)

**Relation:** on every step where both sound, `min(melody sounding pitches) ≥
max(counter sounding pitches)`, and `≥ max(pad/fill pitches)` — i.e. the melody's
LOWEST note this step is at or above the bed's HIGHEST. A strict version:
`melody_pitch ≥ COUNTER_CEILING(67)` and `counter_pitch < COUNTER_CEILING ≤
melody_pitch` by construction. DETERMINISTIC for the *floor* relation (register
floors are structural); SEEDED for exact pitch (chord draw) but the *band
ordering* (`melody_floor ≥ counter_ceiling`) is RNG-free. **Today this can
FAIL** on dark images (§0/§1.2): the melody floor can resolve to ≈57, below
`COUNTER_CEILING`.

### I-3 — BACKGROUND-RECESSION (activity AND level)

**Relation, activity:** `mean_pad_onsets`, `mean_fill_onsets`, `mean_bass_onsets`
each **< mean_melody_onsets** (the bed is quieter in motion than the figure).
**Relation, level:** resolved `melody_velocity ≥ every bed role's velocity on
accented steps` (the S43 carry-forward foreground invariant,
`design-s42-salience-diagnosis.md:289-295`), AND the counter's resolved velocity
**< the melody's** (the new S46 addition — the secondary figure recedes below the
primary). DETERMINISTIC on the structural side (prominence weights resolve RNG-
free); the velocity comparison reads the resolved `realize_velocity` output. The
counter-recedes-below-melody half is what the S45 `melody_forward` weights do NOT
yet guarantee enough of (CounterMelody 0.58 vs Melody 0.78 → only ≈+3.6 velocity
separation through `PROMINENCE_VEL_SPAN(18)`, `:995`).

### I-4 — PER-ROLE-RHYTHM-DISTINCTNESS

**Relation:** no two concurrently-sounding roles share an identical
`(onset_count, sorted offset_ms phase)` shape on a meaningful fraction of steps —
specifically the **melody and counter must differ in onset phase OR count on
≥ a threshold fraction of co-sounding steps** (the M1.3-style onset-distinctness
metric, `spec-s45-variety-metrics.md` §1 M1.3, generalized melody↔counter and
melody↔pad). And **between sections**, the mean onset-density must vary (the §3.3
form differentiation — `max_section_density − min_section_density ≥ threshold`,
the M5.2/M7.2 metric, `spec-s45-variety-metrics.md` §5, §7). DETERMINISTIC
(offset phase + onset count are RNG-free templates). **Today**: melody↔counter
onset distinctness is partially present (counter's off-beat phase vs melody's
on-beat) — this is the ONE dimension partly working, and it must be *preserved*
while the others are fixed (do not collapse the counter onto the melody's grid in
the name of "recession").

### The hierarchy as the scorecard's figure-ground extension

These four invariants ARE the figure-ground metrics the S45 scorecard
(`spec-s45-variety-metrics.md` §8) should grow. They extend it cleanly: §8's
rollup currently scores per-layer *variety*; S46 adds a cross-layer **HIERARCHY**
row — `FIGURE-GROUND: melody-most-active (I-1) | melody-highest (I-2) |
background-recession (I-3) | per-role-rhythm-distinct (I-4)` — each PASS/FAIL with
the relation above, run over the 6-image set. A render is figure-ground-FIRST-
CLASS only when all four pass simultaneously. **No image in the current build will
pass I-1 or I-2** — and that is the objective target each S46 slice moves.

---

## 5. RANKED SLICE CANDIDATES (highest figure-ground yield first)

Per the binding TRAP, every level lever ranks below the non-level differentiation
it depends on. Ranked by audible figure-ground gain per unit effort, freeze tier
and dependencies noted.

### Slice 1 (S46 first slice) — ENFORCE THE FIGURE-GROUND HIERARCHY: melody-most-active + melody-highest

**Two coupled `[CE]` changes, both freeze-reachable, both fixing the dominant
defect:**

1. **Melody-is-highest (I-2): clamp `melody_floor ≥ COUNTER_CEILING` strictly**
   in the `role_pitch` Melody arm (`chord_engine.rs:1271`). The brightness lift
   may still raise the melody, but the floor after summing lift+prom is clamped to
   never resolve below `COUNTER_CEILING(67)`. This kills the dark-image register
   inversion (§0/§1.2) — the operator's signal (3) — with a one-expression change.
   FREEZE: identity path has no Melody-vs-Counter coexistence and the melody on
   the identity render is the lone top voice; clamping a floor that is already ≥67
   on the bright/neutral identity path is byte-neutral (verify the identity
   render's brightness lift is non-negative; if a dark identity fixture exists,
   the clamp would change it — flag for the Test Engineer to confirm against the
   `engine_equivalence` goldens, but the counter is never on the identity path so
   the *relation* is moot there).

2. **Melody-is-most-active (I-1): give the melody a GUARANTEED activity floor that
   exceeds the counter's** in the `OrchestralRole::Melody` arm (`:1927`). Today the
   low-`edge_activity` branch is one sustained tone (`:1974-1992`); add a melody
   minimum-motion analogue of the counter's held-period activation — on a
   held/static step the melody gets at least the dotted/anticipation figure (≥2
   onsets), so it ALWAYS out-moves the counter's single off-beat onset. The
   melody owns the strong-beat onset; the counter keeps `step_ms/4`. This directly
   inverts the "background moves more" defect — signals (1), (2), (5).

**Why this is #1.** It attacks the two dimensions the ear weights most (activity,
register) — the first 90% — and it confronts the S45-counter question correctly:
it does NOT touch the counter (no revert, no re-suppress); it lifts the MELODY
above the counter on activity and register, resolving the competition as a
hierarchy. It is the exact S42/S43 move applied to the new (post-S45) world: S43
foregrounded the melody by LEVEL against the Pad; S46 must foreground it by
ACTIVITY and REGISTER against the now-live counter, because level alone (S43's
tool) cannot win against an equally-moving, register-overlapping competitor.
**Dependencies: none** — both levers are local realizer arms, freeze-reachable.

### Slice 2 — PER-ROLE RHYTHMIC IDENTITY: break the shared `edge_activity` grid

Generalize `prom_shift` (`:1941`) into a per-role rhythm bias so each role's band
cutoffs differ (melody biased toward subdivision, bed roles away), and
phase-separate the bed onsets off the melody's strong beat (§3.4). Add a melody
articulation contrast (`base_frac` per-role bias beside the Fill special-case
`:1580`) — tool #3 of §2.2. `[CE]`, freeze-reachable. **Depends on slice 1** (the
hierarchy must exist before the rhythmic stratification reinforces it). Buys
signal (7) and deepens I-4; the melody's irregular surface vs the regular bed is
the *third* figure cue stacked on slice 1's two.

### Slice 3 — THE LEVEL RETUNE (the last 10%) + inverse-register scaling

(a) Retune `prominence_catalogue` `melody_forward` / `subject_melody` weights
(`mappings.json:365-378`) now that the counter is a live competitor: widen the
Melody-vs-CounterMelody separation (Melody up and/or CounterMelody down — e.g.
the S44 §7 DP-8 carry-forward CounterMelody 0.58→0.55) so I-3's
counter-recedes-below-melody level relation holds with margin. `[JSON]`,
freeze-safe. (b) THEN, the inverse-register *scaling* (§2.1 made literal): make
the melody's non-level help (activity floor, articulation crispness, extra
register lift) *scale up* when the resolved melody pitch is LOW in its band — the
operator's signal (4) in its full form. `[CE]`, freeze-reachable. **Depends on
slices 1–2** (the tools to scale must exist first). This is explicitly LAST among
the figure-ground work because it is the weakest tool — exactly the TRAP
discipline.

### Slice 4 — BED ACTIVITY RECESSION (figuration density + Fill motion budget)

Reduce the bed's *motion* so the figure-ground activity gap widens from the
background side: cap the Pad figuration to weak-beat onsets (off the melody
downbeat) and/or reduce its per-step firing rate (the S44 ostinato finding), and
hold the HarmonicFill's velocity/activity budget firmly below the melody (the S42
over-loud-Fill residue). `[CE]` + `[JSON]`. **Depends on slices 1–3.** This is the
"background recedes in ACTIVITY" half of the operator's signal (5), complementing
slice 1's "foreground moves more" half. Ranked here (not first) because lifting
the figure is higher-yield than lowering the ground, but both are needed for the
full I-3.

### The S45-counter hierarchy question, resolved explicitly

The counter is KEPT as a moving inner texture (S45's gain) across all four slices.
It recedes *relative to a lifted melody* via slice 1 (melody out-moves and out-
registers it) and slice 3 (melody out-levels it), NOT via suppression. The ONE
place a deliberate per-image counter recession is appropriate: a texture where the
melody is intentionally sparse/sustained (a lament, a held-note climax) — there
the counter may carry more motion *by design*, and the hierarchy is satisfied by
register + level instead of activity. That is a per-image affect call (cross-lens
to Affect/Aesthetics), never a blanket revert of `pad_bed_counter` routing.

---

## 6. CROSS-LENS DEPENDENCIES

- **Architect:** I-2's register clamp and I-1's melody activity floor are local
  `role_pitch` / `realize_rhythm` arm edits, but the *register-aware* inverse
  scaling (slice 3b) wants to know "how low is the melody in its band this step"
  — a small derived quantity the realizer can compute inline (no seam change).
  Tool #5 (per-role TIMBRE/program) is a `midi_output.rs` concern OUTSIDE this
  specialist's ownership and likely frozen/seam-bound — **flag to Architect**;
  it is the one figure-ground tool this lens cannot reach.
- **Affect:** the hierarchy must be affect-conditioned — the deliberate-sparse-
  melody case (counter carries motion) is an affect call (§5 close). Affect owns
  the per-image decision of WHICH dimension wins the hierarchy when the melody is
  intentionally still. Affect's stream-segregation precondition
  (`design-s44-variety-nvoice.md` §4.3: level > rhythm-grid > register >
  articulation) is the SAME cue ordering this lens used in §2.2 — they must agree
  on the ranking (this lens ranks rhythm-grid #1 / register #2 for AudioHax
  specifically because the bed is grid-locked; reconcile with Affect's generic
  level-first ordering — the difference is that level is *already partly applied*
  here via S43, so the marginal next win is the rhythm/register tools).
- **Aesthetics / form:** I-4's between-section density arc (§3.3) is the S44 slice-2
  texture-arc item; S46 inherits it. The figure-ground hierarchy should also be
  *form-deployed* — a Return that lifts the melody further (a climactic
  foregrounding) — which is Aesthetics' territory.
- **Test Engineer:** §4's I-1–I-4 are the figure-ground scorecard extension to
  `spec-s45-variety-metrics.md` §8. They are mostly DETERMINISTIC (onset counts,
  offset phases, register floors are RNG-free); the velocity comparisons read
  resolved `realize_velocity`. Each is a regression gate once its slice lands.

---

*End of S46 music-theory design. No source, test, or asset modified.*

*Governing finding: S45 woke the right line (the CounterMelody) but the engine
enforces NO figure-ground hierarchy, so the melody and counter compete as equals
on the two dimensions the ear weights most — ACTIVITY and REGISTER — and on both
the engine currently favors the background (the counter has a guaranteed off-beat
mover the melody lacks; the melody's register floor can fall INTO the counter's
band on dark images). The fix is a hierarchy, not a counter revert: lift the
melody above the counter in activity (I-1) and register (I-2) first, stratify the
per-role rhythm second (I-4 / signal 7), and apply the LEVEL retune + inverse-
register scaling LAST (the TRAP discipline — level is the last 10%). The
role-hierarchy ordering to enforce per dimension is Bass < bed/fill < Counter <
Melody in ACTIVITY and REGISTER, and Bass ≤ Counter < Pad/Fill < Melody in LEVEL.*

*`src/engine.rs` sha256 re-verified UNCHANGED at session end:
`e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261`.*

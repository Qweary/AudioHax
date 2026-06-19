# S44 — Per-Layer Vocabulary + Meaningful Added Roles (Music-Theory Lens)

DESIGN/ASSESSMENT DOCUMENT. No source, test, or asset was modified to produce
it. This reads the current tree through the composition-craft lens and answers
the S44 coupled arc the operator forward-flagged after accepting the S43
melody-foregrounding fix: **(1) more variety at EVERY layer; (2) support >4
instrument lines as a future feature** — under the lead's binding frame that
**variety must precede voice-count**: a 5th/6th/7th line earns its existence
only by performing a real contrapuntal FUNCTION, never by doubling or
parallel-fattening a thin texture.

The realization kernel `src/engine.rs` is **byte-frozen** at sha256
`e50c7db1…2348261` and is **not touched by anything proposed here**. Every
lever lands in `assets/mappings.json` (selection/catalogue tables) or
`src/chord_engine.rs` / `src/composition.rs` (the realizer/planner the frozen
kernel only *calls*).

Evidence base: the S42 per-role realized-output tables
(`docs/design-s42-trace.md` Part A.1/A.2), the S42 theory lens
(`docs/design-s42-theory.md`), the S15/S14 vocabulary designs, and a direct
read of `chord_engine.rs` (roles, `instrument_role`/`assign_role`, `role_pitch`,
`realize_velocity`, `realize_rhythm`, the CounterMelody species engine,
`walking_bass`/`pedal_bass`/`figured_bed`), `composition.rs` (the
`OrchestrationProfile` / `LayerProminence` / `ThemeVariation` planning), and the
`texture_catalogue` / `figuration_catalogue` / `bass_pattern_catalogue` /
`prominence_catalogue` tables in `mappings.json`.

---

## 0. HEADLINE

The S43 fix did the right thing: it foregrounded the melody, and that
foregrounding is exactly what makes the **thinness underneath now audible** —
this is the predicted, healthy consequence of fixing salience first. Heard
honestly, the texture under a now-clear melody is **a static held bed**: the
HarmonicFill is a single sustained inner tone (one onset, the whole step, every
step), the Bass is a single sustained root, and the Pad is the same broken-chord
or block cell repeated as a near-ostinato. Three of the five roles do not
*move* — they sound a pitch and hold it.

The single most important craft finding is an **asymmetry in role maturity**:

- **CounterMelody is the only fully-developed independent line in the engine.**
  It is a real species-counterpoint voice — passing tones, neighbor tones,
  prepared-and-resolved suspensions, parallel-perfect avoidance, contrary/oblique
  motion against the melody, a cadential clausula, and held-period activation
  that *fills the empty bars under a static melody* (`pick_counter_figure`,
  `realized_counter_pitch_with_prev`, `is_legal_passing/neighbor/suspension`,
  `chord_engine.rs:4117–4660`). It is genuinely good.
- **But it is almost never summoned.** It rides only the `pad_bed_counter`
  texture profile, gated on `foreground_energy ≥ 0.35 AND fg_bg_contrast ≥ 0.2`
  (`mappings.json` `composition.texture`). Neither S42 image got it (trace:
  "no CounterMelody on either render"). The richest line in the system is dark
  on the very images the operator listens to.

So the variety problem is **not** that the engine lacks vocabulary everywhere —
it is that the **moving-line vocabulary is concentrated in one gated role while
the three always-on bed roles are static**, and the theme/harmony layer is
clamped to its two cheapest behaviors. The variety-first answer is therefore
*not* "add a 6th voice." It is: **wake the moving vocabulary that already
exists into the always-on roles, and route the existing CounterMelody so it
actually fires** — then, and only then, the N-voice question has a non-doubling
answer, because each new line has a differentiation model to follow.

---

## 1. PER-LAYER VOCABULARY AUDIT

For each layer: what it actually plays today (cited to the S42 trace + the
code), and the specific musical vocabulary missing that would add *real*
variety — not more notes. "Real variety" = the line moves differently, against
the others, in a way the ear segregates.

### 1.1 BASS — root-thump with a pre-cadence pickup; walking/pedal exist but rarely fire

**Plays today.** On both renders the Bass is **one sustained root `(0,)`** for
the whole step (×34 of 42 on `example`, ×27 of 36 on `Lena`), with a single
pre-cadence pickup `(0,468)` (`realize_rhythm` Bass `_` arm,
`chord_engine.rs:1693–1712`). Median pitch 32 / 42; it never moves between
chord roots within a step, and between steps it only changes when the chord
changes.

**Vocabulary that exists but is dormant.** `walking_bass`
(`chord_engine.rs:2059`) and `pedal_bass` (`:2190`) are real and good — walking
fills the gap between the current root and the *next* chord's root with diatonic
passing/neighbor motion and a leading-tone-or-step final approach; pedal holds a
tonic/dominant degree under changing harmony (a genuine pedal point, the
textbook device for tension under a static bass). Both are reachable only via the
`pad_walking` / `pad_pedal` texture profiles, and **no `texture` rule selects
either** — they are in the catalogue with no condition pointing at them.

**Vocabulary genuinely MISSING (theory-grounded):**
- **Stepwise root connection between chords** even in the default bed — the
  single most idiomatic non-walking bass enrichment: when two chord roots are a
  third or more apart, a diatonic passing tone on the weak half of the step
  (`I → [passing] → IV`) turns root-thump into a *line* without committing to a
  full walking bass. This is what a trombonist hears as "the bass is going
  somewhere" vs "the bass is parked."
- **Harmonic-rhythm-aware re-articulation** — the bass currently re-articulates
  only at the pre-cadence. A bass that restrikes the root on the metric strong
  beat (beat 3 in 4/4) reads as *pulse*; holding through reads as *drone*. Today
  it always drones except at cadence.
- **Arpeggiated/compound bass (the "boom-chuck" root-fifth alternation)** — root
  on the downbeat, fifth on the weak beat: the most common popular/folk bass and
  a distinct gait from both sustain and walking. Not present.

**The fix is mostly routing, not new code:** add `texture` rules that select
`pad_walking`/`pad_pedal` on the right images (high `subject_energy` →
walking; near-static/low-`arousal` field → pedal), and add a *default-bed
passing-tone* arm. Variety here is high-yield because the bass is the harmonic
floor — when it moves, the whole texture reads as music rather than as held
chords.

### 1.2 HARMONIC-FILL — a static sustain, and the thinnest layer in the engine

**Plays today.** **One sustained inner tone `(0,)`, ~every step** (×41 of 42 on
`example`, ×35 of 36 on `Lena`; `realize_rhythm` HarmonicFill arm,
`chord_engine.rs:1716–1737`). Its only vocabulary is *sound the tone* or
*rest-as-gesture* (one near-static-image escape). It does not move, does not
change which inner tone it holds within a section, does not ornament, does not
breathe with the phrase. Pre-S43 it was even the *loudest* role (the S42
inversion the fix corrected) — so the thinnest, most static layer was carrying
the most dynamic weight. **This is the single weakest layer and the operator's
"lifeless inner voice" will land here first.**

**Vocabulary genuinely MISSING:**
- **Inner-voice melodic motion** — the canonical role of an inner voice is the
  *smallest* motion (common-tone retention, then step) but it must still
  *resolve tendency tones*: a held chordal 7th should resolve down by step into
  the next chord; a suspension should prepare-suspend-resolve. The Fill holds
  blindly and lets the voice-leading layer re-seat it on the next chord — so
  4→3 suspensions, 7→3 resolutions, and 2-3 inner pedals never sound *in the
  Fill*, even though the engine knows how to spell them (the CounterMelody engine
  literally implements `is_legal_suspension`).
- **Passing/neighbor figuration on weak beats** — an inner voice that fills the
  third between two chord tones with a passing tone on the off-beat is the
  difference between a chord-stream and part-writing.
- **Rhythmic life independent of the bed** — currently Fill and Pad both hold;
  giving the Fill an off-beat re-articulation (the "and" of the beat) is the
  cheapest way to make the inner texture move without competing with the melody.

**The decisive structural point:** the Fill is the *static delegate* of a role
the engine already realizes richly — CounterMelody. The cleanest variety win is
not to reinvent inner-voice motion in the Fill arm but to **promote a moving
inner voice (CounterMelody) into the default texture** so the always-on inner
layer is the species engine, not the dead sustain (see §2.3 and §4).

### 1.3 PAD — a real figuration catalogue, but it reads as ostinato because nothing else moves

**Plays today.** Holds `pad_voices` (3) root-less inner tones, optionally
animated by a `figuration` cell. On `example` it ran the alberti broken-chord
burst `(0,156,313,469)` on 32 of 42 steps; on `Lena` it fell to the plain block
triad (`figured_bed`, `chord_engine.rs:2246`; trace A.1/A.2). The
`figuration_catalogue` is genuinely varied — alberti, broken-chord up/wave,
arp-waltz, block-comp 2&4, oom-pah, oom-pah-pah, stride (`mappings.json`). This
is the *best-stocked* layer.

**Why it still reads thin (the S42 finding, restated):** the Pad's figure is the
**most-repeated surface in the piece** — it runs the *same* cell on ~76% of
steps. A figure repeated unchanged for 32 bars is an **ostinato**, and ostinato
is the strongest identity-fixing device in music — so the ear latches onto the
comping cell as "the piece." The vocabulary is broad *across images* (different
images can pick different cells) but **flat within a piece** (one image gets one
cell, forever).

**Vocabulary genuinely MISSING:**
- **Per-section figuration change** — the single highest-yield Pad fix. A piece
  that comps in block chords in A, breaks them in B, and returns to blocks in
  A′ has *textural form*. Today the figure is selected once per plan and never
  varies by section. The catalogue already holds the cells; only the planner's
  one-figure-per-plan selection blocks it.
- **Density/voicing variation within the held bed** — the Pad always holds the
  same number of voices; thinning to two in a soft section and thickening to
  four at a climax is a free textural contrast (the voicing machinery already
  de-dups and spreads).
- **Registral spread of the bed** — all Pad tones sit in the one fill band
  `[55,67)`; open vs close voicing (spreading the bed across an octave-plus) is a
  real orchestration contrast the band-clamp currently forecloses.

### 1.4 MELODY — now foregrounded (S43), but its rhythmic vocabulary still doubles the bed's grid

**Plays today.** Four edge-activity bands: sustained / dotted `(0,416)` /
syncopated `(0,208,416)` / even arpeggio `(0,156,312,468)`
(`chord_engine.rs:1896–1962`), plus theme pitches in Statement/Return sections.
S43's prominence lift now makes it the loudest, highest voice and lowers its
rhythm-band cutoffs (`prom_shift`) so it subdivides on a different threshold
than the bed.

**Vocabulary genuinely MISSING (post-S43):**
- **Non-chord tones in the line** — the free-select melody takes the *top chord
  tone* every step (`role_pitch` Melody arm). A melody made only of chord tones
  is an arpeggio, not a tune. Passing tones, neighbor tones, appoggiaturas, and
  escape tones — *stepwise non-chord motion between chord tones* — are what make
  a line sing. The engine knows how to validate these (the species predicates)
  but the *melody* never uses them; only the CounterMelody does.
- **Rhythmic independence from the bed grid.** The S42 finding still partly
  holds: the melody's even-arpeggio band shares the Pad's even `(0,.25,.5,.75)`
  onset grid. S43's `prom_shift` helps it *reach* a different band, but within a
  band it still subdivides on the same even grid. Real melodic independence
  wants onsets that cross the bar against the comp (anticipations, tied-over
  syncopations) — vocabulary the melody bands don't currently contain.
- **Articulation contour as a phrase shape** — the continuous curve sets one
  hold-fraction per step from edge-activity; it does not *shape across a phrase*
  (legato into the apex, lift at the cadence). The note-length still whiplashes
  step-to-step (the S42 "extremes" signature) rather than breathing.

### 1.5 RHYTHM (cross-cutting) — one global activity scalar gates every role

**Plays today.** A single `edge_activity` scalar (per-step edge density,
normalized; the `(section.density − 0.5)` term is pinned to 0 on identity
sections) selects every role's rhythm band (`chord_engine.rs:1504–1515`). So all
roles subdivide *together* when the image gets busy — they share a pulse.

**Vocabulary genuinely MISSING:**
- **Harmonic-rhythm variation** — the rate of *chord change* never varies; chords
  change on the coarse section grid. Common practice accelerates harmonic rhythm
  into cadences (one chord for two beats, then one per beat, then faster). This
  is the difference between "calm" and "driving" beyond tempo, and it is the
  trajectory cue the S15 design flagged as TIER-1-but-unbuilt.
- **Per-role rhythmic stratification** — because one scalar gates all roles, the
  roles tend to move together (fuse). Real textures stratify: a sustained pad
  under a walking bass under a syncopated melody — *different subdivision per
  layer*. The `prom_shift` melody offset is the only existing crack in this;
  it should generalize to a per-role rhythm bias.
- **Rest and re-attack as gesture beyond the inner voice** — only the Fill (and
  CounterMelody) may rest. A melody that *breathes* (a rest before a phrase
  answer) is a strong shape cue absent from the melody arm.

### 1.6 HARMONY (cross-cutting) — per-step color exists; the theme-variation vocabulary is clamped dead

**Plays today.** S13 gives per-step harmonic color (7ths/9ths by saturation,
occasional secondary dominant, modal mixture). Sections carry progressions, and
the key-scheme can modulate (pivot/common-tone, `pivot_chord_events`).

**Vocabulary genuinely MISSING — and this is a sharp one:** the
`ThemeVariation` enum has **eight** variants (Identity, Transposed, Reharmonized,
Augmented, Diminished, Ornamented, Fragmented, Inverted, Retrograde —
`composition.rs:418`), but the planner **clamps every section to
`{Identity, Fragmented}`** (`clamp_variation_slice1`, `composition.rs:1786`).
So **six of the eight variation techniques are dead code at plan time.** A
returning theme that is *only ever literally restated or truncated* is the
thinnest possible theme treatment. The high-value, low-cost unlocks:
- **Reharmonization** — same melodic contour, new chords under it on the return.
  The single most powerful "this is the same tune but transformed" device, and
  the machinery (per-section progressions) already exists.
- **Ornamentation** — insert passing/neighbor tones between motif pitches (the
  species engine already does exactly this for the counter-line).
- **Augmentation/Diminution** — double/halve the motif's note values; very
  audible, cheap (scale the rhythm cell).
These are theory layer (harmony/theme) variety that needs *no new voice* — pure
within-layer vocabulary the operator's "more variety at every layer" asks for.

### 1.7 FORM (cross-cutting) — the form catalogue is rich; textural deployment of it is not

**Plays today.** A real `form_catalogue` (rounded binary, ternary, AABA, ABAC,
ABBAC, …) with per-section theme placement and boundary cadences. Form is the
*best-developed* macro layer.

**Vocabulary genuinely MISSING (the §5 dependency):** form is currently audible
only through *theme return + cadence*. It is **not** reinforced by **textural or
contrapuntal contrast** — B does not get a different orchestration, a different
figuration, a different number of moving voices than A. The richest use of the
added roles below is *form-deployed*: a descant that appears only in the A′
return, a countermelody that enters only in B, a thinned texture in a soft middle
— contrast and return rendered in the *texture*, not only the tune. (See §5
Aesthetics dependency.)

---

## 2. MEANINGFUL ADDED ROLES — the N-voice question, musically

The lead's frame is the governing law here: **a new line earns its existence
only by performing a contrapuntal FUNCTION that no existing line performs.**
Below, each candidate role is given its music-theory function, the image/affect
condition that would *call* for it, and an explicit verdict: REAL independent
line vs DOUBLING TRAP.

The engine today realizes five roles (Bass / HarmonicFill / Melody / Pad /
CounterMelody) but the default ensemble sounds four
(`[Bass, Pad, HarmonicFill, Melody]`); CounterMelody is the dormant fifth.
`assign_role` already supports an arbitrary layer list with a documented
clamp rule, so the *mechanism* for >5 voices exists — the question is purely
which roles are musically real.

### 2.1 The functions an added voice can fill (and which are traps)

| Candidate role | Music-theory function | When the image/affect calls for it | Verdict |
|---|---|---|---|
| **Descant** (a SECOND melodic line ABOVE the melody at the climax/return) | A soaring countersubject above the tune — the hymn-last-verse descant; moves in mostly contrary/oblique motion to the melody, lands on chord tones at cadences | High-arousal, bright, high `vertical_emphasis`/top-mass image; deployed **only** in the A′ return or climax as an intensification | **REAL** — distinct register (above melody) + distinct contour (contrary to melody). Must obey no-parallel-8ve/5th with the melody or it collapses into doubling. |
| **Independent inner alto** (a SECOND inner moving voice, between Fill and Melody) | True four-part part-writing: the alto resolves tendency tones the soprano doesn't, supplies suspensions, moves by step | Mid-complexity, harmonically rich (high saturation → 7ths) image where the inner harmony wants to *resolve* audibly | **REAL** *if* it is the CounterMelody species voice, not a second held Fill. A second *sustained* inner tone is a **DOUBLING TRAP.** |
| **Inner / dominant PEDAL voice** | A sustained tonic-or-dominant degree held *through* changing harmony as a tension device — distinct from the bass pedal because it sits in an inner/upper register | Near-static, low-`arousal`, high-tension field (fog, drone, ominous still); or under a developmental B | **REAL** — its function (stasis-under-change) is orthogonal to every moving voice. `pedal_bass` proves the device; an *upper* pedal is the new contribution. |
| **Split walking bass** (a walking line SEPARATE from a sustained root bass) | Two bass-register functions: a held root foundation + a walking connective line — the "bass + walking tenor" of a jazz/baroque texture | Energetic, forward-moving image (high `subject_energy`) where one bass voice can't both anchor and walk | **REAL but register-fraught** — two voices in the bass band collide unless one is octave-displaced or one walks in the tenor register. Earns its place only with strict register separation; otherwise a **mud TRAP.** |
| **Second contrasting countermelody** (a B-section counter-subject distinct from the A counter) | Gives the contrast section its own contrapuntal identity — a different motivic shape than A's counter | High `palette_bimodality` / `fg_bg_contrast` ("two things in the image") image, deployed in B | **REAL** — it is a *different line in a different section*, the very definition of contrast. The species engine already supports a fresh counter contour. |
| **Obbligato** (a free, prominent solo countersubject — e.g. a "flute" over the texture) | A featured second melodic voice with near-melody prominence but its own rhythm/contour — the Bach obbligato | A second strong subject in the image; a duet affect | **REAL** but the *most* demanding — it competes with the melody for foreground, so it needs register and rhythmic separation AND a prominence tier between melody and bed. The richest, last to ship. |
| A second **Pad** / doubled Fill / melody doubled at the octave | (none — adds loudness/thickness, not a line) | never (loudness is a mix decision, not a voice) | **DOUBLING TRAP — the anti-pattern.** This is exactly "raw voice count, empty." |

### 2.2 The ranking among REAL added roles (cheapest-meaningful first)

1. **CounterMelody-as-default-inner-voice** (already built; §4) — *zero new
   role*, just route the existing species voice into the always-on texture. This
   is the highest-value "added line" because the line already exists and is the
   most musically complete voice in the engine; it is "added" only in the sense
   of *being summoned*.
2. **Descant** — a clean win because it occupies an empty register (above the
   melody) and has a clear deployment rule (return/climax only), so it never
   competes with the melody for the *same* slot. Reuses the melody pitch path
   with a contrary-motion + no-parallel constraint against the melody.
3. **Upper pedal voice** — function is orthogonal to all moving voices, so it
   cannot doubling-collide; `pedal_bass` is the template.
4. **Second contrasting countermelody** (B-section) — reuses the species engine
   with a fresh contour; pure form-deployment.
5. **Split walking bass** and **obbligato** — both REAL but register/prominence
   demanding; ship last, behind strict separation rules (§3).

### 2.3 The trap, stated as a rule

**No added voice may be a sustained chord tone in an already-occupied band.** A
6th voice that holds an inner third the Pad already holds, or doubles the melody
an octave down, or thickens the bass with a second root, is loudness disguised
as counterpoint. The test for every added line is: *does it move on onsets the
existing voices don't, in a register the existing voices don't, with a contour
the existing voices don't?* If it fails any of the three, it is a doubling.

---

## 3. VOICE-LEADING / REGISTER GENERALIZATION TO N LINES

Stated as **PROPERTIES** (testable musical constraints), not code. These are the
rules that must hold so N distinct lines stay clean as the ensemble grows past
five. The engine already enforces several pairwise (CounterMelody-vs-Melody); the
generalization is making them hold across *all* pairs in an N-voice texture.

**P1 — Register stratification (no-crossing, ordered bands).** For voices
ordered low→high, each voice's sounding pitch is ≥ the voice below it on every
step: `Bass ≤ {tenor/walking} ≤ {inner voices} ≤ Melody ≤ Descant`. Pedal voices
are exempt only in that they hold a fixed degree, but they must still occupy a
declared band that does not cross a moving voice. (Today guaranteed by per-role
register floors; with N voices the floors must be *assigned per active line*,
not per role-type, so two inner voices get distinct sub-bands.)

**P2 — No parallel perfect fifths or octaves between ANY voice pair.** Currently
enforced only Counter-vs-Melody (`is_legal_*`). The property must generalize:
for every pair (i,j) of moving voices, consecutive verticals must not move from
a P5/P8 to the same P5/P8 by similar motion. The bed (held) voices are exempt
while held; the moment a voice *moves*, it is bound. This is the single most
important property and the one most likely to break when adding a descant above
the melody (parallel octaves melody↔descant is the classic failure).

**P3 — Common-tone retention and minimal motion for inner voices.** Any voice
not designated melodic (inner, pedal, fill) holds a common tone when the harmony
shares it, else moves by the smallest available step. Melodic voices (melody,
descant, obbligato, countermelody) are *exempt* — they are allowed leaps that
inner voices are not. This is the property that keeps inner voices "alive but not
busy" (the lifeless-inner-voice cure is motion *within* this constraint, not
unconstrained motion).

**P4 — Tendency-tone resolution is mandatory for every voice that holds one.**
The chordal 7th resolves down by step; the leading tone resolves up by half step
to the tonic at cadences (and is not doubled). Today only the cadence voicing and
the counter clausula honor this; with N voices, *whichever voice holds the 7th*
must resolve it. This is what makes added inner voices sound like part-writing
rather than held filler.

**P5 — Doubling rules.** In a triad with more voices than tones, double the root
(or the fifth), never the third in a major chord at a structural point, never the
leading tone, never an active tendency tone. This property is what *prevents the
doubling trap from being reachable even by accident* — it constrains how an
"extra" voice fills the chord so a 5th/6th voice is forced toward a useful
doubling (root) and forbidden the useless/harmful ones.

**P6 — One foreground at a time (prominence ordering).** At most one voice
occupies the melodic foreground per moment; a second melodic voice (descant,
obbligato) must be either (a) deployed in a *different section/step-range* than
the melody's foreground moments, or (b) assigned a distinct prominence tier so
the texture has a clear primary and secondary line, never two co-equal tunes
fighting. (Extends the existing two-tier prominence scheme to an N-tier ordering.)

**P7 — Contrary-motion preference between the outer voices.** As the soprano
(melody/descant) rises, the bass should tend to fall, and vice versa — contrary
motion between the outer voices is the strongest cue that they are *independent*
lines and the strongest guard against the whole texture sliding in parallel. A
testable bias, not an absolute.

**The generalization principle:** today's constraints are *pairwise and
role-typed* (Counter-vs-Melody). For N lines they must become *per-active-line
and position-ordered* — every moving line gets a register slot, a prominence
tier, and is bound by P2/P4/P7 against every other moving line. The encoding
boundary that makes this tractable is the existing `LayerProminence` /
`OrchestrationProfile.layers` list: extend it so a profile names not just *which*
roles but *which register slot and prominence tier* each occupies, and the
realizer enforces P1–P7 over the resolved slot ordering.

---

## 4. VARIETY-FIRST RANKING — what to build, in what order, and why

Per the lead's binding frame, every voice-count item is ranked *below* the
within-layer variety it depends on. The ranking is by **audible improvement per
unit effort**, with the variety-precedes-count discipline enforced by
construction.

### The single most impactful first slice (S45 candidate)

**Wake the static inner layer: route the existing CounterMelody species voice
into the default texture, and animate the Pad per section.**

Concretely, two coupled, freeze-safe, JSON-and-planner changes:
1. **Make a moving inner voice the default**, not the gated exception. Either
   (a) broaden the `pad_bed_counter` selection so it fires on ordinary images
   (relax/replace the `foreground_energy ≥ 0.35 AND fg_bg_contrast ≥ 0.2` gate —
   the same real-photo-clustering pathology the S42 prominence gate had), or
   (b) give the default `pad_bed` profile a CounterMelody layer in place of the
   static HarmonicFill. This swaps the single deadest layer (a static sustain)
   for the single richest existing voice (the species engine) at zero new
   realizer cost.
2. **Vary the Pad figuration per section** so the comping cell is not a 32-bar
   ostinato — block in A, broken in B, block in A′ (planner selects a figuration
   *per section* instead of once per plan; the catalogue cells already exist).

**Why this is #1.** It is the exact analogue of the S43 decision: the S42/S43
work foregrounded the melody; the *predicted* next audible defect was the
thinness underneath. The deadest thing under the melody is the static
HarmonicFill, and the richest unused thing in the engine is the CounterMelody.
Routing the latter into the former's slot is the highest variety-per-edit move
available, it is **variety not voice-count** (it makes an *existing* always-on
line *move*), and it needs no new role and no frozen-kernel edit. The
per-section figuration change is the cheapest way to kill the ostinato identity
the S42 trace blamed for "same piece, different key."

### The full ranking

1. **(S45) CounterMelody-as-default inner voice + per-section Pad figuration.**
   Inner layer stops being static; comp stops being an ostinato. *Variety,
   within existing layers, existing voices.*
2. **Unclamp the high-value theme variations** (Reharmonization, Ornamentation,
   Augmentation/Diminution) — lift `clamp_variation_slice1` for these three.
   *Harmony/theme-layer variety; the return becomes a transformation, not a
   photocopy. No new voice.*
3. **Default-bed bass passing tones + route `pad_walking`/`pad_pedal`.** The bass
   starts to *move*; walking/pedal finally fire. *Bass-layer variety; existing
   code, needs selection rules.*
4. **Melodic non-chord tones** (passing/neighbor/appoggiatura in the free-select
   and theme melody) — the melody stops being a pure arpeggio of chord tones.
   *Melody-layer variety; reuses the species predicates.*
5. **Harmonic-rhythm acceleration into cadences + per-role rhythm bias.** *Rhythm
   layer stops being one shared pulse.*
6. **THEN the first added voice: the Descant**, deployed only in the A′/climax,
   under P1/P2/P6/P7. *Now the texture has the differentiation model the new line
   must follow — voice-count, after variety.*
7. Upper pedal voice → second B-section countermelody → split walking bass →
   obbligato, each behind the N-line properties of §3.

**Why variety must precede voice-count (the binding frame, proven concretely):**
items 1–5 add zero voices and are *every one of them higher-yield* than any
added voice, because they make the four lines the listener already hears stop
being static. If the Descant (item 6) shipped *first*, it would soar over a
static held bed and a root-thumping bass — a fifth line decorating a thin
texture, which is precisely the lead's anti-pattern. The texture must first
*move in four voices* before a fifth has something to be independent *from*. The
CounterMelody routing (item 1) is the proof case: it is technically "a voice you
hear that you didn't before," but it lands first *only because the line already
exists fully-differentiated* — adding it is waking variety, not inflating count.

---

## 5. DEPENDENCIES — Architect, Affect, Aesthetics

**On the ARCHITECT (where the lever lands).**
- The N-line generalization (§3) needs the `OrchestrationProfile.layers` /
  `LayerProminence` shapes extended so a profile names a **register slot** and a
  **prominence tier** per line, and `assign_role` / `role_pitch` resolve a
  *per-active-line* register floor rather than a per-role-type constant
  (`BASS/FILL/MELODY_REGISTER_FLOOR`). This is the one structural type decision >5
  voices forces; it is freeze-safe (the kernel only calls `assign_role`).
- Per-section figuration (S45 item 1.2) and per-section theme-variation
  (item 2) need the planner to select *per section* what it currently selects
  *once per plan* — a planner change in `composition.rs`, no kernel touch.
- The P2/P4/P7 cross-pair voice-leading enforcement wants a place to run *after*
  all lines' pitches are chosen for a step — today pitches are realized per
  instrument independently; an N-voice texture needs a post-pass (or a shared
  voice-leading context) so pairwise constraints can see all the lines. Flag this
  as the deepest reconciliation item; it can be staged (P2 melody↔descant first,
  full N-pair later).

**On AFFECT (what energy/character the variety serves).** Each variety lever must
be *image-conditioned*, not always-on, or it becomes a new uniformity:
- The Descant is an *intensification* — it serves high-arousal/bright/triumphant
  affect and must NOT appear on a calm image (a descant over a lament is wrong).
- Walking bass serves forward-motion/energy; pedal serves stasis/tension/dread.
- Per-section figuration density should track the section's affective contour
  (busier B for an agitated middle). Affect owns the mapping from
  arousal/valence/energy knobs to *which* variety fires and *where*, so the added
  vocabulary reads as character, not as decoration.

**On AESTHETICS (form-level deployment of contrast/return).** The added roles pay
off most when **form-deployed** (§1.7): the descant in the return, the
countermelody entering in B, the texture thinning in a soft middle and
thickening at the climax. Aesthetics owns the whole-piece plan that decides
*which section* each added line and each figuration change belongs to, so that
voice-count and figuration variety reinforce statement→contrast→return rather
than spraying evenly across the piece. Texture is a form-bearing dimension; the
variety must be *composed*, not constant.

---

## 6. THE MUSICAL DECISION POINTS THE LEAD MUST RESOLVE

1. **The S45 inner-voice route (the load-bearing fork):** make CounterMelody the
   *default* inner voice by **(a)** broadening the `pad_bed_counter` selection
   gate, or **(b)** swapping the static HarmonicFill out of the default `pad_bed`
   profile for a CounterMelody layer? (a) is more conservative — HarmonicFill
   stays the fallback; (b) is more decisive — the default texture moves. I lean
   (a) first (keep the static Fill as a low-energy fallback, route the moving
   voice in for everything above a low activity floor), so the calmest images
   still get a quiet held bed.
2. **How far to unclamp `ThemeVariation`:** Reharmonization + Ornamentation +
   Augmentation/Diminution are the high-value three. Ship all three in item 2,
   or stage Reharmonization first (most powerful, idiomatic) and hold
   ornamentation/augmentation? My lean: all three — they are independent and each
   is a real within-layer variety win.
3. **Descant deployment scope:** return-and-climax only (my recommendation —
   keeps P6 trivially satisfiable, one foreground at a time), or also as a
   B-section voice? Broader deployment raises the parallel-octave risk (P2) and
   the two-foreground risk (P6).
4. **The N-line register model:** per-active-line register slots assigned by the
   orchestration profile (my recommendation, the clean generalization), or keep
   per-role-type floors and forbid more than one line per role-type? The former
   is what genuinely enables >5 distinct lines; the latter caps the ensemble at
   one-of-each-role.
5. **Split walking bass — ship at all?** It is REAL but the most register-fraught
   (two bass-band voices). Confirm whether the engagement wants it, or whether
   "bass moves" is satisfied by walking/pedal/passing-tones on the *single* bass
   voice (my lean: the single moving bass is enough; defer the split).
6. **Affect-conditioning floor:** confirm that every added voice and every new
   figuration is image-conditioned (fires only when the affect calls for it), not
   always-on — so the variety does not become a new uniformity. This is the
   binding-frame guard restated as a ship rule.

---

*End of S44 music-theory design. No source, test, or asset modified. The
governing finding: the moving-line vocabulary the engine needs largely EXISTS
(the CounterMelody species engine, walking/pedal bass, the figuration
catalogue, six dead `ThemeVariation` variants) but is concentrated in gated or
clamped paths while the three always-on bed roles are static — so the
variety-first answer is to WAKE that vocabulary into the default texture before
adding any voice. A 5th/6th/7th line earns its place only by a real contrapuntal
function (descant, upper pedal, second countermelody, obbligato, split walking
bass) under the N-line voice-leading properties P1–P7; a sustained chord tone in
an occupied band is the doubling trap and is forbidden by P5. The single
highest-yield first slice is routing the existing CounterMelody into the default
inner voice plus per-section Pad figuration — variety, within existing layers and
existing voices, exactly as the lead's frame requires.*

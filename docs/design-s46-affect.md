# S46 Affect / Cross-Modal Lens — Figure-Ground as Auditory Scene Analysis

DESIGN / ASSESSMENT DOCUMENT (Perceptual / Cross-Modal Affect lens). **No
production code, test, or asset file was changed to produce it.** This is a
taste/affect review voice in the S46 cadence (per the Specialist Marshaling
Gate); its perceptual judgment is load-bearing.

It reads from the realized evidence base — the S42 trace and lens set
(`docs/design-s42-trace.md`, `docs/design-s42-affect.md`,
`docs/design-s42-salience-diagnosis.md`), the S44 affect lens
(`docs/design-s44-affect.md`) and the S44 unified work order
(`docs/design-s44-variety-nvoice.md`), the S45 variety-metric spec
(`docs/spec-s45-variety-metrics.md`) — and against the **live code read directly
this session** (cited file:line throughout). `src/engine.rs` is **byte-frozen**
at sha256 `e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261`
(re-verified UNCHANGED at session start). Nothing proposed here touches it; this
is design-only.

> **Verification note (the stale-comment discipline).** The S44 affect lens read
> a STALE doc comment and wrongly called CounterMelody "stubbed / a HarmonicFill
> delegate." The S44 synthesis (`design-s44-variety-nvoice.md` §2) corrected
> this against the live realize path. **This session re-verified the live code:**
> the CounterMelody is a genuine moving species line (`chord_engine.rs:1831-1925`,
> the arm headed *"THE REAL COUNTER-LINE … not the HarmonicFill delegate the stub
> was"*), routed onto three images by `pad_bed_counter`
> (`mappings.json:265, 343-346`). Every claim below is grounded in code read THIS
> session, not in the prior lens's prose.

It answers the operator's S46 figure-ground / role-balance arc — the seven
verbatim signals — through ONE question only: **by what perceptual mechanisms
does a listener pull the MELODY out as the figure, which of those cues does the
engine control, and why is "turn it up" the weakest of them.**

---

## 0. HEADLINE — the operator is perceptually right, and the trap is real

The operator's central claim is **perceptually correct and is the load-bearing
finding of this lens: level (loudness) is the WEAKEST of the strong
figure-ground cues for a sustained-tone synthetic texture, and differentiation
in ACTIVITY — onsets, motion, rhythmic-grid identity — is what actually makes a
line read as the figure.** "Turn the melody up" is the last 10%, not the first
90%. A loud melody over an equally-busy bed is still mush, because the bed keeps
generating attention-capturing onset events that compete for figure status
regardless of their level.

This *refines* (does not contradict) the S42/S43 finding. S43 correctly made
level the FIRST fix because the melody was then *quieter than the bed* (the
HarmonicFill floated to the top — `design-s42-affect.md` Q1); you cannot have a
figure that is buried below the ground in level. **Fixing an inverted level is
necessary; it is not sufficient.** Once the gross level inversion is corrected
(S43 did this), the remaining figure-ground work is dominated by the
*activity/onset* cues, exactly as the operator hears. S43 got the sign right and
the magnitude partial; S46 is the activity-differentiation half S43 deferred.

And the operator's trap is the same one the S42 evidence already PROVED on
accompaniment: the beds already differed and sounded the same
(`design-s42-salience-diagnosis.md` RANK 2). Recede a competing voice in **level
only** and it still competes, because the percept of "a separate line" is built
mostly from its **onsets**, not its loudness. **Recession must be in activity to
register.**

**The S45 CounterMelody is the concrete instance of the trap-in-waiting today.**
The richest moving line in the engine was routed in at S45 (`pad_bed_counter`)
as a deliberate variety win — and it is now the operator's signals (2), (5), (6)
made flesh: a second moving line, seated in the FILL register (floor 55,
`chord_engine.rs:1284`) ~12 semitones below the melody, but only ~5.6 velocity
below it and **on its own active onset grid** (the held-period off-beat at
`step_ms/4`, `:1903`). It is, perceptually, a co-equal duet partner competing
for figure — exactly what S43 flagged at vel 93 vs 98. the lead's counterpoint is
correct: the resolution is HIERARCHY (counter recedes in activity/register
RELATIVE to a lifted melody), **not** walking back S45 — that would forfeit the
moving-inner-texture gain S45 bought.

---

## 1. FIGURE-GROUND AS AUDITORY SCENE ANALYSIS — the cues, ranked by strength

A listener does not hear "a melody and an accompaniment"; the auditory system
performs **auditory scene analysis** [Bregman 1990] — it segregates the
continuous spectrum into perceptual **streams** and then assigns one stream the
status of **figure** (attended foreground) and the rest **ground** (unattended
support). Which stream becomes the figure is decided by a handful of grouping
cues. The operator's seven signals are, collectively, a precise diagnosis that
the engine is mis-deploying these cues. Here they are, **ranked by their
strength for THIS listener and THIS texture** (sustained-tone synth, no timbral
differentiation between voices, deterministic render). The ranking is the
load-bearing contribution of this lens, because the build budget must follow it.

### The cue-strength ranking (strongest → weakest) for the AudioHax texture

| Rank | Segregation cue | Why it is strong/weak HERE | Engine lever | Tier |
|---|---|---|---|---|
| **1** | **Rhythmic-grid distinctness / onset-rate independence** | Streams that share an onset grid FUSE (the proven S42 melody+pad failure: identical `(0,156,312,468)` grids, `design-s42-trace.md` A.1). A voice on a *different subdivision or phase* generates its own onset stream the ear tracks separately. For a synth texture with no timbre cue, this is the **strongest available** figure-maker — it is the cue the listener's "this is the tune" judgment rests on. | Melody rhythm bands lowered by `prom_shift` (`:1941-1943`); counter held-period off-beat grid (`:1903`); per-role rhythm arms | **[CE]** |
| **2** | **Onset asynchrony / motion-rate contrast** (a sub-facet of 1, but distinct enough to rank) | The figure should generate MORE onset events than the ground per unit time (the melody should MOVE MOST — operator signal 2). Asynchronous onsets between voices break fusion even at equal level [Bregman 1990; Darwin 1997]. A figure that moves *less* than the ground is perceptually demoted no matter how loud. | Melody onset density via edge_activity band + `prom_shift`; bed activity floor (counter/pad onset rate) | **[CE]** + **[JSON]** |
| **3** | **Register / frequency separation** | A clear pitch gap aids segregation — BUT it is the **weakest of the strong cues alone**: the S42 melody was the *highest* voice and STILL fused (`design-s42-affect.md` Q1). Frequency-based streaming holds only when PAIRED with level or rhythm. Useful as the cue that decides *which* stream is figure once segregation exists (higher → more figure-like), not as the cue that *creates* segregation. | Register floors: Bass 36 / Fill 55 / Melody 67 (`:1220-1222`); brightness lift; prominence REG_SPAN (`:1270`) | **[CE]** |
| **4** | **Articulation / timbre contrast** | Distinct articulation envelope (staccato figure over legato bed, or vice-versa) is a real but *reinforcing* cue — it helps voices segregate and carries affect besides (§5). Weaker than rhythm/level here because the synth gives no timbre difference; articulation is the only timbre-adjacent handle. | Articulation curve / STACCATO/PORTATO/LEGATO fracs (`:1413-1415`); per-role artic | **[CE]** |
| **5** | **LEVEL / loudness** | The operator's claim, vindicated: **this is the WEAKEST strong cue for sustaining figure status in this texture.** Level decides figure when all else is equal AND it fixes a gross inversion (a figure quieter than the ground cannot be figure — the S42/S43 case). But once level is non-inverted, a small loudness gap does NOT hold a figure against an equally-busy, equally-articulated competitor: the ear re-assigns figure to whichever stream is generating the salient *onsets*. Level is a tiebreaker and an inversion-fixer, not a figure-maker. | Role vel bias (`:1384-1394`); prominence VEL_SPAN (`:1404`) | **[CE]** + **[JSON]** |

### Why the ranking inverts the engine's instinct

The engine's foregrounding apparatus is **level-and-register-heavy and
activity-light**: `melody_forward` lifts the melody +5 velocity and +1 register
(VEL_SPAN 18, REG_SPAN 4 — `:995, :1005`), but its *activity* lever
(`PROMINENCE_RHY_SHIFT = 0.10`, `:1015`) shifts the melody's rhythm-band cutoffs
by only **0.028 at weight 0.78** — a near-negligible nudge toward subdividing. So
the engine is spending its prominence budget on the two WEAKEST cues (level,
register) and barely touching the two STRONGEST (rhythm-grid, onset-rate). **That
is the perceptual root of the operator's signals (1)-(4): the melody is louder
and higher but not meaningfully MORE ACTIVE, so it does not read as the figure.**
The fix is to rebalance the prominence budget toward activity — and the levers to
do it already exist (`prom_shift` is wired; it is just weighted too weakly).

> **Grounding the operator's "level is the weakest tool" claim, in one sentence:**
> for a synthetic texture with no timbral contrast, figure status is assigned by
> *who is generating the salient onset stream*, and loudness only adjudicates ties
> or repairs inversions — so spending the foregrounding budget on level leaves the
> strongest figure-cue (onset/rhythm-grid distinctness) on the table.

---

## 2. WHY ACTIVITY RECESSION MATTERS MORE THAN LEVEL RECESSION

This is the perceptual justification of the operator's trap, and it is the
governing principle for receding the backgrounds (signals 5, 6).

**A background that recedes only in loudness but stays rhythmically busy still
competes for figure status, because every onset it generates is an
attention-capturing event.** Auditory attention is *event-driven*: a new onset
is a salient perceptual event that pulls attention toward its stream, and this
capture is **largely level-independent** within the normal mix range — a quiet
but busy line keeps "raising its hand" with each onset. So a bed turned down 3 dB
but still articulating on every beat continues to generate the onset stream the
ear tracks, and the foreground has to win the figure contest *onset-for-onset*,
not decibel-for-decibel. Turning the bed down does almost nothing to its
onset-generation rate; therefore it does almost nothing to its figure-competition
strength. **This is exactly why the S42 beds — which already differed in
figuration — read as the same piece: the difference was in the ground's
decoration, and the ground was still generating the onsets the ear was attending,
so nothing about the *figure contest* changed.**

### What "recede in ACTIVITY" must mean, concretely

Recession-in-activity is the operative move for the backgrounds. It has three
measurable components, in order of perceptual weight:

1. **Lower onset density (fewer onsets per unit time).** The single most
   important component. A receding voice should generate FEWER onset events than
   the figure — ideally sustaining (one onset per step or longer) while the figure
   subdivides. The HarmonicFill already does this (one sustained inner tone per
   step — `:1729-1737`); the routed-in CounterMelody does NOT (it guarantees an
   off-beat onset on every held/static step — `:1899-1907`), which is precisely
   why it competes.
2. **Lower motion rate (sustain pitch; change less often).** A voice that holds
   its pitch generates no *pitch-change* events; a voice that moves on every step
   generates a melodic-contour stream the ear follows as a tune. The bed should
   move LESS than the figure. (Operator signal 2 is the dual of this: the melody
   should move MOST.)
3. **Higher rhythmic regularity / predictability.** A perfectly regular,
   predictable onset pattern habituates — the ear stops attending it and folds it
   into ground. An *irregular* or *syncopated* pattern keeps re-capturing
   attention. So the figure should carry the irregular/syncopated rhythm
   (`:1956-1964`, the melody's syncopated band) and the ground the regular,
   predictable one. Giving the bed syncopation is anti-recession.

**The rule, stated for the build:** *a voice recedes from figure status by
reducing its onset density and motion rate and increasing its rhythmic
predictability — NOT primarily by reducing its level.* Level recession is the
finishing 10%; activity recession is the load-bearing 90%. This is the perceptual
content of the operator's "first 90% is differentiation, last 10% is level."

---

## 3. THE INVERSE-REGISTER COMPENSATION, PERCEPTUALLY

The operator's signal (4) is the subtlest and the most perceptually
sophisticated: **register-aware prominence is INVERSE compensation** — a melody
HIGH in the range self-projects and needs LESS help, while a melody LOW in the
range fuses into the bed and needs MORE help, delivered via NON-level cues. This
is correct, and the engine currently gets the *direction* of register-as-figure
right but does NOT implement the *compensation*.

### Why high notes self-segregate and low melodies fuse

**Frequency-based streaming** [Bregman 1990; van Noorden 1975]: voices that
occupy clearly separated frequency regions are easier to hold apart, and the
*highest* voice in a texture is perceptually privileged as a candidate figure
("high-voice superiority" — the soprano-line bias). A melody sitting well above
the accompaniment band gets segregation *for free* from its frequency position;
its register is doing the figure-work, so it needs little additional help.

But when the melody dips LOW — into or near the accompaniment's register band —
frequency separation collapses, and the melody must compete with the inner voices
for the *same* perceptual region. With no frequency gap, the ONLY cues left to
keep it as figure are the non-frequency ones: onset-rate/rhythm-grid distinctness,
articulation contrast, and (last) level. **This is the operator's exact point:
the low melody needs MORE help, and "turn it up" is the weakest way to give it —
the perceptually effective compensation is to make the low melody MORE ACTIVE
(subdivide more, move more) and MORE ARTICULATED (distinct envelope) than the bed
it is now sharing a register with.**

### What compensation each register band needs

| Melody register state | Self-segregation from frequency? | Compensation needed | Engine lever |
|---|---|---|---|
| **HIGH** (melody well above fill band, e.g. bright image lifts it to 79+) | STRONG — register does the work | LITTLE — a modest activity lead is enough; do NOT over-drive level | reduce prominence reliance; the existing +5 vel is already plenty |
| **MID** (melody near top of fill band, 67-72, dim image, no bright lift) | WEAK — melody crowds the inner voices | MORE — a clear onset-rate/rhythm-grid lead over the bed; modest articulation contrast | strengthen `prom_shift` (activity); articulation split |
| **LOW** (melody dropped by a dark image's `bright_octaves<0`, into/below 67) | NEGLIGIBLE — melody is IN the bed | MOST — strong activity lead + articulation contrast are mandatory; level is a poor substitute | maximal `prom_shift`; recede the bed in ACTIVITY (§2) so the contest is winnable |

The current engine register lift is brightness-driven and prominence-driven, both
*additive to the floor* (`:1263-1271`). It RAISES the melody but does not
*condition the activity compensation on how low the melody landed.* The
perceptually correct design is **inverse**: the lower the realized melody seat
relative to the bed, the MORE the activity/articulation lead it should receive —
because register stopped doing the figure-work, so the other cues must do more.
This is a freeze-reachable refinement to how `prom_shift` (`:1941`) is computed
(condition it on the realized melody-vs-bed register gap), entirely in
`chord_engine.rs` **[CE]**.

> **The perceptual inversion, named:** prominence help should be *anti-correlated*
> with the melody's realized register height — high melody, little help; low
> melody, maximal NON-LEVEL help. The engine today applies help roughly uniformly
> (and via the weak cues). This is the deepest of the operator's seven signals and
> the one most worth getting right.

---

## 4. THE FIGURE-GROUND METRIC DEFINITIONS — from the perception side

These define, perceptually, what each of the operator's figure-ground concerns
should MEASURE to predict the listener's verdict, so the synthesis / Test
Engineer can encode them on the realized NoteEvent streams (the same headless
plan→realize harness the S45 scorecard reads — `spec-s45-variety-metrics.md`
§0.1). Each is paired with a **perceptually-sufficient contrast threshold** where
one can be stated; thresholds are ear-tunable in magnitude, grounded in direction.

These EXTEND the S45 variety scorecard (which measures *whether each layer moves*)
with the orthogonal axis it does not cover: *whether the moving layers are in the
right figure-ground RELATIONSHIP.* A layer can pass S45 (it varies) and fail S46
(it varies as much as the figure → it competes). The S43 correctness invariants
(resolved Melody prominence > 0.5; melody velocity ≥ loudest bed role) are the
LEVEL-only floor; these are the ACTIVITY companions S43 deferred.

### FG-1 — "Melody is MOST-active" (operator signal 2; the strongest cue, rank 1-2)

**Perceptual definition:** the figure must generate the densest onset stream. The
melody's onset density (onsets per sounding step, summed over the render) must
exceed *every* other sounding role's by a clear margin.

- **Metric:** `melody_onset_density − max(other_role_onset_density)`, where
  onset_density = total onsets / sounding steps per role. *DETERMINISTIC* (onset
  counts are RNG-free rhythm-template outputs).
- **Perceptually-sufficient threshold:** the melody's onset density should exceed
  the busiest bed role's by **≥ ~0.3 onsets/step** (roughly: where the bed
  sustains ~1/step, the figure averages ≥ ~1.3, i.e. it subdivides on a
  meaningful share of steps the bed does not). A margin near zero means
  co-equal-activity = competition. **Sign is load-bearing: a NEGATIVE value
  (melody less active than a bed role — the operator's signal 2 literally) is a
  HARD figure-ground failure** and must fail the build, the activity analogue of
  the S43 "melody velocity ≥ loudest bed" level invariant.

### FG-2 — "Background-recession" (operator signals 5, 6; the §2 activity finding)

**Perceptual definition:** background roles must recede in ACTIVITY, not (only)
level. Measure recession on the dimension that matters.

- **Metric (primary, activity):** for each bed role, `bed_onset_density /
  melody_onset_density` (an activity ratio) AND `bed_motion_fraction /
  melody_motion_fraction` (a motion ratio). Both should be **< 1** (the bed is
  less active and moves less than the figure). *DETERMINISTIC for onset density;
  SEEDED for motion fraction.*
- **Metric (secondary, level):** the S43 invariant — bed velocity < melody
  velocity on accented steps. Keep it; it is the inversion-floor.
- **Perceptually-sufficient threshold:** activity ratio **≤ ~0.7** for a clearly
  receded bed (the bed generates ≤ ~70% of the figure's onsets); motion ratio
  **≤ ~0.7**. A ratio near 1.0 = co-active = the trap. **For the CounterMelody on
  counter-routed images this metric is the live watch-item** (see §6/FG-5).

### FG-3 — "Melody is highest" (operator signal 3; cue rank 3)

**Perceptual definition:** the figure should occupy the top of the texture
(high-voice superiority). On the vast majority of sounding steps the realized
melody pitch must be the highest sounding pitch.

- **Metric:** fraction of sounding steps where `melody_pitch == max(all sounding
  pitches this step)`. *SEEDED* (absolute pitch); an RNG-invariant floor exists
  via the register floors (Melody floor 67 > Fill/Counter floor 55 → structurally
  melody-on-top unless a dark `bright_octaves<0` drop or a high counter tone
  inverts it).
- **Perceptually-sufficient threshold:** melody-is-highest on **≥ ~0.9** of
  sounding steps. Note the engine's register floors *almost* guarantee this
  structurally — the failure modes are (a) a dark image dropping the melody an
  octave (`bright_octaves<0`, `:1254/1263`) and (b) a counter tone seated high in
  the fill band on a step the melody dipped. The metric exists to catch those
  inversions, which are exactly the operator's "may not be highest" intuition.

### FG-4 — "Register-aware velocity / inverse compensation" (operator signal 4; §3)

**Perceptual definition:** the prominence *help* the melody receives should be
INVERSE to its realized register height relative to the bed — low melody gets
more (non-level) help, high melody gets less.

- **Metric:** correlation (or a binned check) between, per section, the
  realized `melody_register_gap = melody_pitch − max(bed_pitch)` and the melody's
  applied `prom_shift` activity lead. A first-class engine shows **negative
  correlation** (smaller gap → larger activity lead). *SEEDED for the gap;
  DETERMINISTIC for the applied shift once conditioned.* Today the correlation is
  ~0 (uniform help) — the metric makes the missing compensation visible.
- **Perceptually-sufficient direction:** the sign (negative) is the
  load-bearing assertion; magnitude is ear-tuned.

### FG-5 — "Per-role-rhythm-distinctness" (operator signal 7; cue rank 1)

**Perceptual definition:** each sounding role must occupy a DISTINCT onset grid
from every other — distinct subdivision and/or phase — so the voices segregate
into separate streams (the anti-fusion guarantee). This is the encodable form of
the strongest cue and the direct fix for "rhythms stale/same across roles."

- **Metric (intra-step, anti-fusion):** for every pair of concurrently sounding
  roles on a step, their `offset_ms` sets must NOT be identical (the S42 fusion
  signature was identical grids). Metric: fraction of step×role-pairs with
  distinct onset offsets. *DETERMINISTIC* (offsets are RNG-free rhythm templates).
  This is the S45 spec's M1.3 (counter-vs-pad onset distinctness, ≥0.40)
  generalized to **all role pairs**.
- **Metric (cross-section, signal 7's "flat between-section rhythm"):** between
  sections, onset-density should vary per role (`max_section_density −
  min_section_density` per role) — the rhythm ARC the operator misses. This is
  S45 spec M5.2 (≥0.20 between-section spread); S46 adds that it must be
  measured PER ROLE, and that the *relationship* between roles' grids must be
  preserved across sections (the figure stays distinct from the bed in every
  section).
- **Perceptually-sufficient threshold:** all-role-pair onset-distinctness
  **≥ ~0.5** of concurrent step-pairs; between-section per-role density spread
  **≥ ~0.2**. Identical grids on a meaningful share of steps = fusion = the
  operator's "rhythms stale/same."

> **The figure-ground verdict, as a rollup:** a render is figure-ground
> FIRST-CLASS iff FG-1 (melody most active, margin ≥ threshold AND sign positive),
> FG-2 (every bed role activity-ratio ≤ ~0.7), FG-3 (melody-highest ≥ ~0.9), FG-4
> (compensation correlation negative), and FG-5 (all-pair onset-distinctness ≥
> ~0.5) all hold simultaneously — alongside the carried-forward S43 level floors.
> No current render passes; the value is naming WHICH cue each image fails on.

---

## 5. AFFECT-CONDITIONING — how the hierarchy should bend with image affect

The figure-ground hierarchy is itself an affective parameter. The strength of
figure-ground SEPARATION should track the image's arousal — but in a controlled
way that does NOT re-introduce the uniformity the whole pipeline exists to escape.

### The principle: separation tracks arousal; the figure always wins

| Image affect (`affect_arousal` / `affect_valence`) | Figure-ground separation | Confidence | Why |
|---|---|---|---|
| **High arousal** (sat+colorfulness+edge high) | SHARPER — strong activity lead for the figure, busier (but still receded) bed, wider rhythmic-grid contrast | HIGH (arousal→density/tempo cues additive) | An energetic image wants a vivid, forward, assertive figure; the contrast itself reads as energy |
| **Low arousal** (calm, low-sat) | GENTLER — a softer activity lead, a quieter and even more sustained bed, less rhythmic-grid contrast | HIGH | A calm image wants intimacy; an over-separated, spotlit melody over a recessed bed reads as aggressive/clinical, contradicting the affect |
| **High valence** (bright) | (affects mode/color, not separation directly); separation leans toward a singing legato figure | HIGH (valence→mode) | — |
| **Low valence** (dark/tense) | separation can stay present but DENSE-BUT-SOFT — see the musical-fear caveat (§7) | HIGH (mode); see caveat | — |

The governing constraint that prevents this from becoming a new uniformity:
**the figure ALWAYS wins the figure-ground contest — what bends is the MARGIN, not
the ordering.** A calm image gets a *gentler* separation (smaller activity lead,
smaller register/level gap), never an *inverted* or *absent* one. Sparseness is
expressive (the S44 lesson), but a calm piece still has a tune; "no figure" is
not a calm affect, it is the original defect. So FG-1's *sign* (melody most
active) must hold on every image; only FG-1's *margin* and FG-2's *ratio* scale
with arousal. This is freeze-reachable: scale `prom_shift` and the bed-activity
floor by the already-computed `affect_arousal` composite, the same composite that
already gates figuration choice (`texture` table, `mappings.json:347-362`) — **a
[JSON] gate + a [CE] scale, no new feature.**

### The S45 counter hierarchy — per-image: when does the counter recede vs stay forward?

This is the load-bearing affect-conditioning decision and the direct answer to
the lead's counterpoint. The CounterMelody is routed on the three structured images
(`AudioHaxImg1/2/3`, `mappings.json:343-346`). The resolution is **per-image
hierarchy, governed by arousal — counter recedes by DEFAULT, stays forward only
as a deliberate high-arousal call:**

- **DEFAULT (most counter-routed images): the counter RECEDES below the melody in
  ACTIVITY and stays in the fill register.** Concretely: the counter's onset
  density and motion fraction must sit *below* the melody's (FG-2 applied to the
  counter), and its prominence weight (currently 0.58 under `melody_forward`,
  yielding +1.44 vel and competing) should recede toward the bed band so the
  melody's figure status is unambiguous. This PRESERVES the S45 gain — the counter
  still MOVES (it is still a moving inner line, the variety win), it just moves
  *less than the figure* and sits clearly under it. **This is the hierarchy
  resolution: a recessed moving inner voice, not a silenced one, not a co-equal
  duet partner.**
- **FORWARD (deliberate, high-arousal only): the counter rises toward duet
  status** — for a genuinely energetic image where two interweaving forward lines
  read as richness, not mud, AND where the register/grid separation between melody
  and counter is wide enough to keep them segregated. This is the rare earned
  state, gated on high `affect_arousal`, decided per-image, ear-confirmed.

> **The per-image counter-hierarchy recommendation, stated:** recede the counter
> in ACTIVITY (and trim its prominence weight) below a lifted melody by DEFAULT on
> every counter-routed image, preserving its motion (the S45 gain) while
> subordinating it; promote it toward co-equal duet status ONLY as a deliberate
> per-image high-arousal hierarchy decision with confirmed melody↔counter
> grid/register separation. Do NOT walk back S45. Do NOT leave the counter at its
> current near-co-equal 0.58/+1.44/own-active-grid state — that IS the operator's
> signals (2)/(5)/(6) on the counter-routed images.

---

## 6. PURE-RUST vs ML LINE (per claim)

- **Activity-led figure-ground (rebalance prominence toward onset/rhythm-grid
  distinctness): REACHABLE NOW, pure Rust.** Every lever exists — `prom_shift`
  (`:1941`), the per-role rhythm arms (`:1647-1994`), the held-period activation
  (`:1899`). The work is to re-WEIGHT (strengthen the activity lever, add bed
  activity recession), not to extract anything. **[CE]** + **[JSON]**.
- **Background activity recession (counter/pad recede in onsets/motion):
  REACHABLE NOW, pure Rust** — a realization re-weighting in `chord_engine.rs` and
  a prominence/texture row retune in `mappings.json`. Freeze-safe.
- **Inverse-register compensation (condition the activity lead on realized
  melody-vs-bed register gap): REACHABLE NOW, pure Rust** — the realized seats are
  computed in `role_pitch` (`:1248-1299`); conditioning `prom_shift` on the gap is
  a [CE] refinement, freeze-reachable (identity path: gap term centered to no-op).
- **Affect-conditioned separation margin: REACHABLE NOW, pure Rust** — rides the
  existing `affect_arousal` composite already feeding the `texture` table.
- **Per-image counter forward/recede decision: REACHABLE NOW as a HEURISTIC** —
  gated on the arousal composite + the (cheap-proxy) `fg_bg_contrast` already
  routing the counter. **NOT reachable as a SEMANTIC call** ("this image depicts
  two equal subjects → two equal voices") — that is scene understanding, an opt-in
  ML tier, later. The heuristic (arousal-gated) is honest and sufficient.
- **Reliable warm=forward-figure / cool=recessed: NOT reliable** — hue→affect is
  weak and culturally contingent; let the arousal composite (saturation-led, HIGH
  confidence) govern separation margin, never hue.

Net: every figure-ground lever the operator's seven signals ask for is
**pure-Rust-reachable today, all outside the freeze.** The limits are (a) semantic
scene understanding (deferred ML) and (b) the perceptual discipline of ranking the
cues correctly (this lens) — a *design* constraint, not a capability gap.

---

## 7. RISKS / CAVEATS

- **Do NOT over-correct into level recession.** The temptation, reading the
  operator's signals (5)/(6), is to turn the backgrounds DOWN. That is the
  weakest fix (§1-§2) and risks hollowing the texture (the S43 watch-item: "the
  bed recedes but does not vanish"). Recede in ACTIVITY first; touch level last
  and modestly. Keep the S43 floor: bed roles recede but stay prominence > 0.25.
- **Do NOT silence or de-route the CounterMelody.** the lead's counterpoint is binding:
  the S45 routing is a variety GAIN; the fix is hierarchy (recede in
  activity/register), not removal. A silenced counter forfeits the moving-inner
  texture S45 bought and re-opens the "static bed" defect S45 closed.
- **Musical-fear = SOFT caveat — applies to the high-arousal/low-valence
  separation.** When sharpening figure-ground on a dark, energetic image, do not
  reflexively make it LOUD: musical fear = fast + minor + SOFT, distinct from
  anger = fast + minor + LOUD [Cespedes-Guevara & Eerola 2018]. A dense-but-soft,
  sharply-segregated texture is a valid target; a "sharp separation ⇒ loud
  figure" coupling would erase it. Separation is an ACTIVITY/register matter, not
  a level matter — so this is consistent with the whole-lens thesis.
- **Load-bearing-valence-owns-mode caveat — PRESERVED.** Nothing here touches the
  major/minor decision (`mode_valence_cuts {major_min:0.55, minor_max:0.45}`,
  `mappings.json:199`); figure-ground is an arousal-axis / orchestration concern
  and must not back-door a mode change.
- **The cue-strength ranking is texture-specific.** The level-is-weakest finding
  holds for THIS texture: sustained-tone synth, no timbral differentiation between
  voices, deterministic render. In a multi-timbre orchestration (distinct
  instruments per role), timbre would jump up the ranking and level would matter
  less still. The ranking is correct for the engine as built; revisit if real
  per-role timbres are ever added.
- **Thresholds are ear-tunable in magnitude, grounded in direction.** The FG-1..5
  numbers (onset-density margin ~0.3, activity ratio ~0.7, melody-highest ~0.9,
  onset-distinctness ~0.5) are starting values sized between the trivial and the
  extreme — the DIRECTIONS and the SIGNS (melody most active; bed ratio < 1;
  compensation correlation negative) are HIGH-confidence and load-bearing; the
  magnitudes want the operator's ear, exactly as the S43 `melody_forward` weights
  did.
- **Verified-not-assumed.** Unlike the S44 affect lens (which read a stale
  comment), every code claim here was read live this session: register floors
  `:1220-1222`; velocity role bias `:1384-1394` (HarmonicFill still has NO bias —
  the S42 optional Edit 3 was NOT applied); the recessive-bed-never-lowered
  register clamp `:1301-1308`; `prom_shift` `:1941-1943`; prominence constants
  VEL_SPAN 18 / REG_SPAN 4 / RHY_SHIFT 0.10 `:995/1005/1015`; the live counter arm
  `:1831-1925`; `pad_bed_counter` layers + gate `mappings.json:265, 343-346`;
  `melody_forward` weights `:373-378`. `engine.rs` sha256 re-verified UNCHANGED.

---

## 8. TASTE VERDICT + WATCH-ITEMS for the A/B ear-test

**Taste verdict (anticipatory, for the build's eventual A/B at `--seed 42`):** the
operator's diagnosis is perceptually sound and the fix is well-scoped. The piece
will be heard to improve when the listener can, on a counter-routed image
(`AudioHaxImg1/2/3`), **track the melody as the single clear figure** — not pick
it out by effort, but have it arrive as the attended line — while the moving
counter is audibly THERE but UNDERNEATH (a moving inner voice, not a second tune
fighting for the front). The success signature is *activity*, not loudness: the
melody should be the busiest, most-moving line; the bed should be present but
calmer; and the difference between two structured images should now read in their
FIGURES, not their decoration.

**Watch-items for the ear-test (what to listen for, and the fusion alarms):**

1. **Can the listener point to the melody as a separate thing and hum it?** If the
   melody and counter blur into one wandering texture → fusion; the counter did
   not recede enough in activity (FG-2 on the counter failed). Revisit the counter
   recession before touching level.
2. **Does the melody MOVE MOST?** If a bed role (counter, or a busy pad figuration)
   sounds busier than the tune → FG-1 failed at the ear; strengthen the melody
   activity lead (`prom_shift`), do not just raise its level.
3. **On a DARK image (melody dropped low), is the melody still the figure?** This
   is the inverse-compensation test (FG-4): a low melody must hold figure via
   activity/articulation. If it sinks into the bed → the compensation is missing
   or too weak.
4. **Did the backgrounds recede WITHOUT hollowing out?** If the texture sounds
   thin/empty → level recession went too far (the S43 "bed does not vanish"
   floor); back off level, keep the activity recession.
5. **Did receding the counter forfeit the S45 movement?** If the inner texture
   went static again → over-receded; the counter must still MOVE, just less than
   the figure. The target is *subordinate motion*, not stillness.
6. **Calm vs energetic image — does the separation MARGIN differ?** A calm image
   should sound gently separated (intimate), an energetic one sharply separated
   (vivid). If both sound identically spotlit → the affect-conditioning (§5) is
   flat; the figure-ground margin is not tracking arousal.
7. **The carried-forward S43 watch-item, re-based:** S43 pre-staged a CounterMelody
   weight tweak (0.58→0.55) against what was then a phantom. It is now a LIVE
   counter on three images. Re-evaluate that weight against the real counter as
   part of the recession decision (§5) — and note the weight (level) is the *last*
   10%; resolve the counter's ACTIVITY recession first, then ear-tune the weight.

---

## 9. SUMMARY FOR THE LEAD

**The affect bridge for this arc:** figure-ground is auditory scene analysis — the
listener assigns "figure" to whichever stream generates the salient ONSET stream,
and for this timbre-flat synth texture the segregation cues rank **rhythmic-grid
distinctness / onset-rate > register separation > articulation > LEVEL**. The
operator is perceptually right: level is the weakest figure-maker (it fixes
inversions and breaks ties, nothing more), so the engine — which spends its
prominence budget on level (+5 vel) and register (+1) while barely touching
activity (`RHY_SHIFT` nudges the melody's rhythm cutoffs by only 0.028) — is
foregrounding via the wrong cues. The fix is to rebalance toward ACTIVITY:
melody most-active, backgrounds receded in onset-density/motion (NOT just level),
per-role distinct onset grids, and inverse-register compensation (a low melody
gets MORE non-level help because it lost its frequency-segregation). The S45
CounterMelody is the live trap: routed in as a variety win, it now sits ~5.6 vel
below and ~12 semitones below the melody but on its OWN active off-beat grid — a
co-equal duet partner competing for figure. Resolve it as HIERARCHY (recede in
activity/register relative to a lifted melody, preserving its motion), NOT by
walking back S45. Everything is pure-Rust-reachable, all outside the freeze; the
only ML-gated piece is semantic scene-content understanding (deferred).

**Cue-strength ranking (the load-bearing finding):** rhythmic-grid/onset-rate
distinctness (1) > onset asynchrony/motion-rate (2) > register separation (3) >
articulation (4) > level (5). The engine is over-investing in 3 and 5, under-
investing in 1, 2, 4.

**Activity-vs-level finding:** a background receding only in loudness still
competes, because attention is event-driven and each onset re-captures it
largely independent of level. "Recede in activity" = lower onset density + lower
motion rate + higher rhythmic predictability. This is the perceptual content of
the operator's "differentiation is 90%, level is 10%."

**Figure-ground metric definitions (for the synthesis / Test Engineer):** FG-1
melody-most-active (onset-density margin ≥ ~0.3, **negative = HARD fail**); FG-2
background-recession (bed activity ratio ≤ ~0.7, motion ratio ≤ ~0.7, PLUS the S43
level floor); FG-3 melody-highest (≥ ~0.9 of steps); FG-4 inverse-register
compensation (negative correlation between melody-bed register gap and applied
activity lead); FG-5 per-role-rhythm-distinctness (all-pair onset-distinctness
≥ ~0.5; per-role between-section density spread ≥ ~0.2). These EXTEND the S45
variety scorecard (does it move?) with the figure-ground RELATIONSHIP axis (does
it move in the right hierarchy?), and add the ACTIVITY companions to the S43
LEVEL-only invariants.

**Per-image counter-hierarchy recommendation:** recede the counter in ACTIVITY
(and trim its 0.58 prominence weight) below a lifted melody by DEFAULT on every
counter-routed image — keep it MOVING (the S45 gain), make it move LESS than the
figure and sit clearly under it. Promote it toward co-equal duet status ONLY as a
deliberate per-image high-arousal call with confirmed melody↔counter
grid/register separation. Do not walk back S45; do not leave it at near-co-equal.

**Cross-lens dependencies:** (1) FG-1..5 want the realized NoteEvent streams from
the S45 headless harness — the Test Engineer extends that instrument, does not
build a new one. (2) The activity-lead and bed-recession re-weighting lands in
`chord_engine.rs` realization internals + `mappings.json` rows — owned by Music
Theory (realization) + the prominence/texture/affect rows under single-writer
coordination; this lens supplies the perceptual requirement and the cue ranking.
(3) Affect-conditioning rides the existing `affect_arousal` composite — coordinate
with the arousal-composite owner that the SAME composite now also scales the
figure-ground separation margin. (4) Every figure-ground lever must be
affect-conditioned (this lens, §5) or it becomes a new uniformity — a binding
ship-rule, not a tuning preference.

**Decision points for the lead:**
1. **Rebalance the prominence budget toward activity** (raise `PROMINENCE_RHY_SHIFT`
   from 0.10; add a bed-activity-recession term) so figure-ground rides the strong
   cues, not the weak ones? *Lens recommends YES — it is the direct fix for signals
   1-4, freeze-reachable [CE].*
2. **Implement inverse-register compensation** (condition the melody's activity
   lead on its realized register gap to the bed)? *Lens recommends YES — it is the
   deepest of the seven signals (4) and the engine currently applies help roughly
   uniformly. [CE], freeze-reachable.*
3. **Recede the S45 CounterMelody in activity by default** (per §5), preserving its
   motion? *Lens recommends YES, with the forward-promotion as a rare arousal-gated
   per-image call. Resolve activity recession BEFORE the 0.58→0.55 weight tweak.*
4. **Apply the optional S42 HarmonicFill velocity bias now?** It was never applied;
   on `pad_bed`-routed (non-counter) images the Fill still has no level recession
   beyond its 0.40 weight. *Lens recommends YES as the cheap level-floor companion
   — but it is the 10%; ship the activity re-weighting (DP-1) first and re-listen.*
5. **Adopt FG-1..5 as a standing figure-ground correctness gate** beside the S45
   variety scorecard and the S43 level invariants? *Lens recommends YES — FG-1's
   sign (melody most active) is the hard build gate, the activity analogue of the
   S43 "melody ≥ loudest bed" level gate.*

---

*End of S46 affect lens. Design-only: no source, test, or asset modified.
`src/engine.rs` sha256 re-verified UNCHANGED:
`e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261`.*

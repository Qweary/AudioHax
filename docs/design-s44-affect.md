# S44 Affect / Cross-Modal Lens — What Per-Layer Variety and Voice Count Actually SERVE

DESIGN / ASSESSMENT DOCUMENT (Perceptual / Cross-Modal Affect lens). No
production code or content changed to produce it. This reads from the realized
S42 evidence base (`docs/design-s42-trace.md`, `docs/design-s42-affect.md`,
`docs/design-s42-salience-diagnosis.md`), the S43 taste-gate verdict
(`docs/review-s43-taste-affect.md`), the composition-architecture design
(`docs/composition-architecture-musical.md`, `docs/assessment-composition-architecture.md`),
the S13 diversity spec (`docs/design-s13-diversity.md`), and the confirmed
field/enum/table shapes in `src/composition.rs`, `src/chord_engine.rs`, and
`assets/mappings.json` (READ-ONLY; field names verified below). The realization
kernel `src/engine.rs` is **byte-frozen** at sha256 `e50c7db1…2348261` and
nothing proposed here touches it; this is design-only and proposes no edits at
all.

It answers the operator's S44 forward-flagged arc — **deepen the per-layer/per-
voice musical VOCABULARY** (two coupled signals: more variety at every layer;
support >4 instrument lines as a future feature) — through one question only:
**what does per-layer variety and added voice count actually SERVE
perceptually, and what is the trap.**

---

## 0. HEADLINE — the load-bearing counterpoint, grounded

The lead's counterpoint is correct and it is squarely a perceptual claim, so
this lens owns the case for it: **raw voice count is cheap and perceptually
EMPTY without per-voice differentiation.** Texture-thickness (number of
simultaneously sounding voices) is a *real but MEDIUM-confidence* arousal cue —
but the cue only fires when the added voices are perceptually DISTINCT lines
that the auditory system can pull into separate streams. Pile on doublings or
register-clustered held tones and you raise **no** perceived arousal and you
**worsen** the exact failure S42 diagnosed: voices that do not segregate fuse
into one stream and read as mud, not richness.

So ">N voices meaningfully" is not a separate goal from "more variety" — it is a
*facet* of it. Both reduce to the same perceptual requirement: **differentiation
the listener can hear.** Variety that is theoretically present in the data but
inaudible (a fourth held tone three semitones from the third; a counter-line on
the melody's own onset grid; a "different" bass pattern under an inaudible bed)
buys nothing affectively and tests green while sounding identical — the
operator's explicitly-warned-against trap, the same one S42's bed-already-
differed evidence already proved on accompaniment.

The S42 lesson carries forward verbatim: the melody FUSED with the pad's onset
grid into one stream (`design-s42-trace.md` A.1, melody arpeggio
`(0,156,312,468)` rhythmically identical to the Pad alberti burst
`(0,156,313,469)`). **Adding voices without distinct register / rhythm-grid /
articulation = MORE fusion, not more richness.** Per-voice differentiation is
simultaneously what makes variety *perceptible* AND what makes density
*meaningful*. They are one requirement.

---

## 1. WHAT VARIETY SERVES PERCEPTUALLY — per layer, per dimension

The vocabulary surface is the five roles (`OrchestralRole` /`LayerRole` in
`src/composition.rs:474` and `src/chord_engine.rs:856`: **Bass, HarmonicFill,
Melody, CounterMelody, Pad**) plus the cross-cutting dimensions (rhythm /
harmony / form). For each, this section names which variety dimensions carry
affect, with a confidence level from the grounding tables, and — critically —
distinguishes variety the listener PERCEIVES from variety that is present in the
data but inaudible.

### 1.1 The five variety dimensions and the affect they carry

These are the levers any layer can vary. Confidence is preserved from the
Affect→Music table (the best-established link in the whole chain; cues combine
roughly additively [Eerola, Friberg & Bresin 2013]).

| Variety dimension | Affect carried | Direction | Confidence | Grounding |
|---|---|---|---|---|
| **Rhythmic density / onset rate** | AROUSAL | more onsets → higher arousal | HIGH | Juslin & Laukka 2003 |
| **Articulation contour** (staccato↔legato) | AROUSAL (+ a valence tinge) | staccato → higher arousal; legato → calmer | MEDIUM–HIGH | Juslin & Laukka 2003 |
| **Register spread / pitch height** | AROUSAL | higher / wider → higher arousal | HIGH | Hevner 1937; Eerola et al. 2013 |
| **Harmonic color** (7th/9th, mixture, dissonance) | VALENCE (consonance↔dissonance) | consonant → positive; dissonant → negative | HIGH | Gabrielsson & Lindström 2010 |
| **Dynamic contour** (level + swell) | AROUSAL | louder → higher arousal | HIGH | Juslin & Laukka 2003; Eerola et al. 2013 |

The fact that matters for "more variety at every layer": **four of the five
variety dimensions are AROUSAL-bearing, only harmonic color is primarily
VALENCE-bearing.** So adding variety, undirected, tends to push perceived
arousal up. That is fine for an energetic image and *wrong* for a calm one (see
§4) — variety is not free; it has an affective sign.

### 1.2 Per-layer — which variety the listener actually hears

**MELODY (the figure).** Variety here is the highest-payoff variety in the whole
engine, because S43 just made the melody the *attended* stream
(`review-s43-taste-affect.md`: melody now loudest on both renders, +9/+5
figure-ground gap). The listener is *listening to this line*, so every variety
dimension on it is PERCEIVED at full weight:
- **Rhythmic-density + articulation variety on the melody = HIGH-confidence
  perceived affect.** A melody that varies its onset rate and staccato/legato
  across sections reads directly as the piece's energy contour. This is where
  variety pays the most.
- **The articulation-contour variety is also the cure for the S13 "note-length
  extremes / uniformly-short computer notes" complaint** (`design-s13-diversity.md`;
  the S42 trace A.3 confirms melody hold-ms median 83/62 ms — staccato-dominated).
  A *continuous, image-driven* articulation curve on the foregrounded line is
  perceived; a fixed clamp is not variety, it is a constant.
- **Register-arc variety (octave lift at a climax) is PERCEIVED** as the
  intensity peak (`composition-architecture-musical.md` §6.2 places it end-of-B).

**COUNTERMELODY (the second figure-family voice).** This is the single most
important layer for the ">4 voices" question, and it is currently a TRAP in
waiting: `CounterMelody`'s realization is **STUBBED — it delegates to the
HarmonicFill figure** (`src/chord_engine.rs:869-872`). A counter-melody that
renders as a HarmonicFill held tone is **not a second line — it is a doubled
bed voice.** Adding it raises NO perceived arousal and risks fusion with the
melody (the S43 verdict already flagged this: CounterMelody realized vel 93 vs
Lena melody 98, a narrow two-tier gap that "could read as a co-equal duet
partner rather than support"). A counter-line creates real perceived richness
ONLY when it is a *distinct line*: its own contour, its own rhythm-grid offset
from the melody, its own register band. **Un-stubbing CounterMelody as a genuine
second line is the highest-value voice addition available** (confidence HIGH that
a distinct countermelody adds perceived richness; MEDIUM that the current stub
adds anything but mud). A "fast distinct countermelody, a busy obbligato" is the
canonical example of a voice addition that *does* raise perceived arousal.

**HARMONICFILL (the inner held bed).** Variety here is mostly INAUDIBLE. The
Fill is a sustained inner tone (`src/chord_engine.rs:1716-1737`, one sustained
`(0,)`); the ear treats it as ground. Varying *which* inner tone it holds, or
its exact hold length, is theoretically-present-but-inaudible variety — the
classic trap. The one Fill change that *is* perceived is its LEVEL relative to
the melody (S42's headline defect was the Fill floating to the top; S43 recessed
it). Beyond level, Fill variety is low-payoff. **Do not invest variety budget
here.**

**PAD (the harmony bed / figuration).** This is where the operator's trap was
already *proven*. The S42 beds ALREADY differed (`example` got the animated
alberti broken-chord bed, `Lena` got the plain block triad —
`design-s42-trace.md` A.1/A.2) and the operator heard them as the same piece
(`design-s42-salience-diagnosis.md` §2 RANK 2). Pad-figuration variety
(`figuration_catalogue` already holds alberti / broken_chord_up /
broken_chord_wave / arp_waltz / oom_pah / stride … — 11 figures, `assets/mappings.json`)
is a genuine SECOND-ORDER refinement that deepens per-image character — **but
only once an audible figure sits in front of it.** As bed variety it is
perceived weakly (the ear is not attending the comping). The honest perceptual
statement: Pad-figuration variety carries affect (a busy stride bed reads more
aroused than a held block — MEDIUM confidence via density+articulation), but
that affect lands as *texture coloring under the figure*, not as the thing the
ear identifies the piece by.

**BASS (the foundation).** Bass-pattern variety (sustained / walking / pedal —
`bass_pattern_catalogue`, `pad_walking` / `pad_pedal` profiles) carries a real
but specific affect: a **walking** bass reads as forward motion / higher arousal
(MEDIUM confidence, density-driven); a **pedal** reads as stasis / suspension. It
is PERCEIVED more than Fill variety because gait is a low-frequency rhythmic
anchor the ear tracks — but less than melody variety. Worth a modest budget.

### 1.3 The cross-cutting dimensions (rhythm / harmony / form)

- **Rhythm variety (across the whole arc, not per-step).** The S13 engine gave
  per-step rhythmic variety (`design-s13-diversity.md`) and the operator still
  heard "structureless." The lesson: per-step rhythmic variety on undifferentiated
  voices is *wash*, not perceived variety. Rhythm variety becomes perceived when
  it is **organized by section** (the variation surface of a Theme-and-Variations
  form — `composition-architecture-musical.md` §1.1) so the busyness reads as
  "more to say about one idea" rather than noise. Confidence HIGH that section-
  organized rhythm variety is perceived; HIGH that undirected per-step rhythm
  variety is not.
- **Harmonic variety (color).** Already substantial per-step (7th/9th by
  saturation, mixture, secondary dominants — S13). It carries VALENCE
  (consonance↔dissonance, HIGH confidence). The missing perceived part is
  *trajectory* — color concentrated at the B departure and resolved at the return
  (`composition-architecture-musical.md` §6.1). Per-step color without trajectory
  is "varied but directionless," another face of ethereal. Adding *more* per-step
  color is the inaudible-variety trap; organizing existing color by section is the
  perceived win.
- **Form variety.** The single largest perceived-variety lever, because **return
  + contrast + cadential articulation** (`composition-architecture-musical.md`
  §1) is how the ear knows it is hearing a *different piece*. The `key_scheme`
  table already carries form vocabulary (rounded_binary / ternary_aba /
  theme_and_variations / rondo …). Form variety is PERCEIVED at the highest
  weight of any dimension and carries affect indirectly (a contrasting B section
  in the parallel mode is a valence excursion the ear hears as the piece's
  emotional arc). Confidence HIGH.

### 1.4 The perceived-vs-present summary (the operator's real question)

| Variety investment | Perceived by listener? | Affect payoff | Verdict |
|---|---|---|---|
| Melody rhythm/articulation/register variety | YES (it is the attended stream) | HIGH | **Invest first** |
| Un-stubbing CounterMelody as a distinct line | YES (a new stream) | HIGH | **Invest — the real >4-voice win** |
| Form variety (section-organized return/contrast) | YES (defines "different piece") | HIGH (arc) | **Invest** |
| Harmonic-color *trajectory* (concentrate at B) | YES | HIGH (valence arc) | Invest |
| Bass-pattern variety (walking/pedal) | PARTLY | MEDIUM | Modest budget |
| Pad-figuration variety (already shipped) | WEAKLY (ground, not figure) | MEDIUM (texture color) | Second-order, after a figure exists |
| HarmonicFill inner-tone variety | NO (inaudible inner held tone) | ~none | **Do not invest (trap)** |
| Per-step rhythm/color variety w/o section organization | NO (reads as wash) | ~none | **Do not invest (the S13 trap)** |
| Adding a doubling / register-clustered held voice | NO (fuses) | ~none / negative | **Do not invest (the fusion trap)** |

---

## 2. THE DENSITY / VOICE-COUNT AFFECT TRUTH

**The cue is real but conditional. Texture density (# of voices) is a
MEDIUM-confidence arousal cue [Webster & Weir 2005] — and the condition is
non-negotiable: the added voices must be perceptually DISTINCT.**

### 2.1 When adding a voice RAISES perceived arousal / richness

A voice addition pays off perceptually when the new voice forms its **own
auditory stream** — i.e. it is distinct in at least one strong segregation cue
(register band, rhythm-grid, articulation/timbre — §3). Canonical payoff cases:

- **A fast, distinct countermelody / busy obbligato.** Distinct rhythm-grid +
  distinct contour → a second attended stream → genuinely higher perceived
  arousal and richness. (Confidence HIGH for the perceptual win, conditional on
  real differentiation; this is exactly the un-stub opportunity in §1.2.)
- **A walking bass under a held bed.** Distinct rhythm-grid (steady quarter
  motion) vs the sustained voices → perceived as added drive. (MEDIUM.)
- **A genuinely separate register-band line** (e.g. a high obbligato well above
  the melody, or a tenor counter-line in the gap between Fill and Bass) — the
  register separation alone can carry stream segregation. (MEDIUM–HIGH.)

### 2.2 When adding a voice is INERT or HARMFUL

- **A doubled pad / a doubled melody octave.** Doublings reinforce the existing
  stream; they raise no perceived arousal and add no information. Perceptually
  INERT (raises loudness slightly, which is itself an arousal cue — but that is
  the loudness cue, not the density cue, and is cheaper to get by raising level).
- **A held tone register-clustered with existing held tones.** Voices within ~a
  third of each other, on the same onset grid, FUSE — they read as one thicker
  chord, not as more lines. This is *worse* than inert: it muddies the texture
  and degrades stream segregation, the exact S42 melody+pad fusion failure
  re-created on purpose. Perceptually HARMFUL.
- **A CounterMelody that delegates to the HarmonicFill figure** (the current
  stub). It is a held inner tone wearing a melodic name. It does not segregate
  from the Fill/Pad bed; it adds mud and risks fusing with the melody from
  *below*. HARMFUL until un-stubbed.

### 2.3 The precondition, stated as a rule

> **A voice may be added to the perceived-arousal/richness account ONLY if it is
> distinct from every already-sounding voice in at least one of {register band,
> rhythm-grid phase/subdivision, articulation/timbre}. A voice that fails this
> test is, perceptually, not a voice — it is a thickening of an existing one, and
> it contributes mud, not richness.**

This is the perceptual case for the lead's variety-first counterpoint, in one
sentence: **count is free in the data and expensive in the percept; pay for
differentiation, not for count.** Ship a swarm-supported >4-voice mechanism only
behind a differentiation guarantee — otherwise the feature tests green
(N voices present) and sounds like the same piece with more mud (N voices
fused), which is the trap in its purest form.

---

## 3. PER-VOICE DIFFERENTIATION AS A STREAM-SEGREGATION REQUIREMENT

Tying directly to the S42 fusion finding (`design-s42-affect.md` Q1: "Pitch
height alone is a weak segregation cue when every other cue says 'same stream' —
the auditory system happily folds a high voice into the texture if it is no
louder, no rhythmically freer, and onset-aligned with the chords"). Auditory
scene analysis groups sound into streams from a handful of cues; for N voices to
be *heard* as N lines, they must differ on these. In rough order of strength:

1. **LOUDNESS / LEVEL** — the strongest single cue and the one S43 just fixed for
   the melody. A foreground voice that is audibly louder than the bed segregates.
   (HIGH.) This is why S43 worked structurally even before the ear confirmed it.
2. **RHYTHMIC INDEPENDENCE / ONSET-GRID DE-FUSION** — voices that share an onset
   grid fuse (the S42 melody+pad failure: identical `(0,156,312,468)` grids).
   Voices on *different* subdivisions or phase-offset grids segregate. The S43
   escalation profile already exploits this via lowered rhythm-band cutoffs so the
   melody "subdivides on a different grid than the bed"
   (`design-s42-salience-diagnosis.md` §1). For a counter-line, a deliberately
   offset grid is the cheapest, strongest differentiation after level. (HIGH.)
3. **REGISTER / PITCH SEPARATION** — a clear pitch gap between voices aids
   segregation, but it is the WEAKEST of the strong cues *alone* (S42: the melody
   was in the highest register and still fused). Register separation must be
   PAIRED with level or rhythm to hold. For a 5th voice, give it its own register
   band that does not overlap an existing voice's working range. (MEDIUM.)
4. **ARTICULATION / TIMBRE CONTRAST** — distinct articulation envelope (one
   staccato, one legato) or distinct timbre helps voices segregate and carries
   affect besides (§1.1). Useful as a *reinforcing* cue. (MEDIUM.)

**The requirement, stated for the >4-voice feature:** every voice beyond the
foundational bed must be assigned a differentiation budget across these cues
*before* it is allowed to sound. The orchestration profile (`OrchestrationProfile`
in `src/composition.rs:487`, the `layers: Vec<LayerRole>` list + per-section
`prominence` from the `prominence` SelectTable) is exactly the right place to
encode this: each added layer needs a prominence weight (level), a rhythm-grid
assignment (grid/subdivision), and a register slot. **A `layers` list that adds a
fifth role without giving it a distinct prominence weight + rhythm-grid + register
slot will produce a fused fifth voice — the trap by construction.** This is a
design constraint on whatever mechanism implements ">4 voices," not a runtime
tuning.

---

## 4. AFFECT-DRIVEN VARIETY DEPLOYMENT — which images get more, which stay sparse

**Sparseness is itself expressive. A calm, low-arousal image should NOT get max
voices or max variety — sparseness IS the affective content.** Variety is an
arousal-bearing resource (§1.1: four of five dimensions push arousal up), so
deploying it uniformly over-energizes calm images and flattens the affect range
the whole pipeline exists to express. The correct deployment couples variety
*budget* to the arousal composite.

Map to the confirmed affect fields on `ImageUnderstanding` (`src/composition.rs`):
`affect_arousal` (the S22 planner-computed composite, sentinel `-1.0` until
filled), `affect_valence`, and the constituents the arousal composite already
pools — `avg_saturation` (0.45 weight), `colorfulness` (0.25), `edge_activity`
(0.20), `complexity` (0.10) per `assets/mappings.json` `affect.arousal_weights`.

| Image affect state (`affect_arousal` / `affect_valence`) | Variety / voice deployment | Confidence | Why |
|---|---|---|---|
| **High arousal** (sat+colorfulness+edge high) | More voices, busier rhythm, more articulation contrast, denser figuration | HIGH (arousal→density/tempo/dynamics) | The added arousal-bearing variety is *congruent* with the image's energy |
| **Low arousal** (calm, low-sat, low-edge) | FEWER voices, sparse texture, sustained articulation, held bed | HIGH | Sparseness expresses the calm; piling on voices contradicts the affect AND risks fusion-mud with no payoff |
| **High valence** (bright) | Major family already (valence owns mode); variety leans consonant color, lilting figuration | HIGH (valence→mode/consonance) | — |
| **Low valence** (dark/tense) | Variety leans dissonant color, but keep density honest to arousal (musical fear = fast+minor+SOFT, NOT loud — see §6) | HIGH (mode); see caveat | — |

This is the direct extension of the S22 arousal composite the engine already
computes: **the composite that drives tempo/character should ALSO gate the
variety/voice budget.** The same `affect_arousal` that picks `scherzo` vs
`ballad` (`character` SelectTable) should scale how many distinct lines sound and
how much each varies. The `texture` SelectTable already gestures at this
(`pad_stride` gated on `arousal ≥ 0.75`, `pad_block_comp` on
`arousal ≥ 0.70 + valence ≥ 0.55`) — the design principle is to extend that
arousal-gating to *voice count and per-layer variety depth*, not just figuration
choice.

**The anti-pattern to forbid:** a fixed "always use all N voices, always max
variety" deployment. That is the ceremony-inflation of the musical surface — it
destroys the calm↔energetic affect axis and re-introduces fusion-mud on the
images that least want it. Variety deployment must be affect-conditional.

---

## 5. THE TRAP, NAMED — thin multiplication and how to detect it

**THIN MULTIPLICATION:** increasing voice count or per-layer variety in the data
without increasing what the listener can *hear* as distinct. Its perceptual
signature is precise:

- **The render tests green on every objective metric** (N voices present, M
  distinct figuration choices, K rhythm patterns used) **and sounds the same** —
  the S42 bed-already-differed evidence is the canonical instance.
- **The texture gets THICKER but not RICHER** — more energy in the signal, no
  more lines in the percept. A spectrogram shows more notes; the ear hears one
  blurred mass.
- **It muddies rather than enriches** — undifferentiated added voices degrade
  stream segregation, so the foreground gets *harder* to follow (the S42 melody
  fused into the bed). Thin multiplication can make a piece sound WORSE while
  every count goes up.

**THE DETECTION TEST (the one question):** *Would a listener hear the
difference?* Concretely, the gate is the A/B ear test S43 established — render
with and without the added variety/voice at `--seed 42` and ask:

1. Can the listener point to the new line as a *separate thing* (hum it, track
   it independently)? If no → it fused; it is thin multiplication.
2. Did the perceived energy/richness change in the *direction the affect
   intended*? If the count went up but the felt energy did not → inert
   multiplication.
3. Did the foreground get HARDER to follow? If yes → harmful multiplication
   (fusion); revert.

**The encodable proxy guard (pairs with the ear, does not replace it):** for any
added voice, assert it differs from every concurrent voice in at least one strong
segregation cue — distinct prominence weight (level gap ≥ one JND over the
nearest voice), OR distinct rhythm-grid (non-identical onset offsets), OR
non-overlapping register band. A voice that passes none of these three is thin
multiplication *by construction* and should fail the build before it reaches the
ear. This is the §3 requirement restated as a CI guard — it catches the trap at
the moment of the defect, the same way the S43 correctness invariants
(resolved Melody prominence > 0.5; melody velocity ≥ loudest bed role) caught
the salience defect.

---

## 6. PURE-RUST vs ML LINE (per claim)

- **Per-layer variety depth (rhythm / articulation / register / dynamics /
  harmonic color on each role): REACHABLE NOW, pure Rust.** It is an
  orchestration/realization choice driven by the already-computed affect
  composite + per-section plan. No image understanding beyond the existing
  low-level features is needed. The S13 levers already exist; the S44 work is to
  *organize* them by section and gate them by arousal, not to extract anything new.
- **Un-stubbing CounterMelody as a genuine distinct line: REACHABLE NOW, pure
  Rust** — a realization change in `chord_engine.rs` (a real counter-line instead
  of the HarmonicFill delegate), entirely freeze-safe (it is downstream of the
  frozen kernel, which only *calls* `realize_step`). This is a music-craft build
  (Music Theory + Rust Architect own the realization internals); this lens
  specifies only the *perceptual requirement* (distinct grid/register/level).
- **>4 distinct voices: REACHABLE NOW in mechanism, pure Rust** — the `layers`
  Vec and per-layer `prominence` already generalize past 4; the constraint is
  perceptual (§3 differentiation), not architectural. The pure-Rust engine can
  voice and differentiate N lines. What it CANNOT do without ML is decide *which
  N* a scene musically warrants from its *content* (see below).
- **Affect-conditional variety deployment (calm→sparse, energetic→dense):
  REACHABLE NOW, pure Rust** — it rides the existing `affect_arousal` composite.
- **Knowing the image *content* to choose voices semantically** (e.g. "this is a
  crowd scene → many independent lines"; "this is a single subject on empty
  ground → one line + sparse bed): NOT reachable in pure Rust — that is
  scene/object recognition, an opt-in ML tier, later. The pure-Rust path
  approximates it with `subject_size` / `fg_bg_contrast` / region-energy fields
  (`subject_energy`, `foreground_energy`, `background_energy`) as cheap proxies —
  honest, but a proxy. The `texture` table already does exactly this with
  `subject_energy` / `foreground_energy` gates.
- **Reliable warm=many-voices / cool=few-voices: NOT reliable** — hue→affect is
  weak and culturally contingent; do not let hue gate voice count. Let the
  arousal composite (saturation-led, HIGH confidence) do it.

Net: every variety/voice lever the operator's arc asks for is **largely
reachable in pure Rust today**. The limits are (a) semantic scene understanding
(deferred ML) and (b) the perceptual differentiation discipline of §3 — which is
a *design constraint*, not a capability gap.

---

## 7. RISKS / CAVEATS

- **Load-bearing-valence-owns-mode caveat — PRESERVED.** Nothing here touches the
  major/minor decision; valence owns the third (`ModeValenceCuts`,
  `mode_valence_cuts: {major_min:0.55, minor_max:0.45}`), hue stays a within-
  family garnish. Variety/voice deployment is an arousal-axis and orchestration
  concern; it must not be allowed to back-door a mode change.
- **Musical-fear = SOFT caveat — applies to voice/density deployment.** When
  deploying density on a low-valence high-arousal image, do not equate "more
  voices / more energy" with "louder" by reflex: musical fear = fast + minor +
  SOFT/low loudness, distinct from anger = fast + minor + LOUD
  [Cespedes-Guevara & Eerola 2018]. A dense-but-soft texture is a valid and
  important affect target; a naive "dense ⇒ loud" coupling would erase it.
- **The density→arousal link is MEDIUM confidence, conditional on
  differentiation.** Do not over-weight voice count as an arousal driver relative
  to the HIGH-confidence cues (tempo, loudness, register, rhythmic density on the
  *attended* line). If forced to choose, get arousal from the HIGH-confidence
  melody-line cues first; treat added voices as enrichment, not as the primary
  energy lever.
- **CounterMelody is currently a stub — treat any CounterMelody-bearing profile
  as a fusion risk until un-stubbed.** The `pad_bed_counter` profile
  (`layers: ["Bass","Pad","CounterMelody","Melody"]`) exists in the catalogue and
  will, today, render the CounterMelody as a HarmonicFill held tone — a mud voice.
  Either un-stub it before relying on it, or do not gate variety on it.
- **Tuned by ear, not from a landmark study:** the exact differentiation
  thresholds (how large a level gap / how offset a grid / how wide a register
  band is "enough" to segregate) are ear-tunable judgment calls. The *directions*
  (level > rhythm-grid > register > articulation in segregation strength; calm→
  sparse; count needs differentiation) are grounded; the magnitudes want the
  operator's ear, exactly as the S43 `melody_forward` weights did.
- **Scope discipline.** This is an assessment; it proposes no edits. The
  build that follows is a music-craft + architecture build (Music Theory owns
  realization internals incl. the CounterMelody un-stub; the Rust Architect owns
  where the variety budget is computed; this lens supplies the perceptual
  requirements and the affect-gating). `src/engine.rs` stays byte-frozen; all
  reachable levers land in `chord_engine.rs` / `composition.rs` / `mappings.json`,
  per the S42 trace freeze table.

---

## 8. SUMMARY FOR THE LEAD

**The affect bridge for this arc:** per-layer variety and added voice count serve
perceived arousal/richness ONLY through *differentiation the listener can hear* —
they are one requirement, not two. Four of the five variety dimensions (rhythmic
density, articulation, register, dynamics) carry arousal; harmonic color carries
valence; all pay off most on the *attended* stream (the melody S43 just
foregrounded) and least on the inner bed. The highest-value voice addition is
un-stubbing CounterMelody into a genuinely distinct line; the highest-value
variety is section-organizing the rhythm/harmony/form the S13 engine already
produces per-step (per-step variety on undifferentiated voices is wash — the
documented S13 trap). Density (# voices) is a MEDIUM-confidence arousal cue gated
on a non-negotiable precondition: each added voice must segregate by level OR
rhythm-grid OR register, or it fuses into mud — the S42 melody+pad failure
re-created. Variety must be affect-conditional: calm images stay sparse
(sparseness is expressive), energetic images get more — driven by the existing
`affect_arousal` composite. The trap is thin multiplication: counts go up,
percept doesn't; detect it with the one question "would a listener hear the
difference?" plus an encodable per-voice differentiation guard. Everything is
pure-Rust-reachable today except semantic scene-content understanding (deferred
ML); the real limit is differentiation *discipline*, not capability.

**Decision points for the lead:**
1. **Un-stub CounterMelody?** This is the single highest-value voice addition and
   the precondition for trusting any CounterMelody-bearing profile (`pad_bed_counter`
   today renders mud). Recommend YES as the first slice of the voice-vocabulary
   build — it is a music-craft realization change, freeze-safe, and turns an
   already-catalogued profile from harmful to valuable. Route to Music Theory +
   Rust Architect; this lens supplies the distinct-grid/register/level requirement.
2. **Affect-gate the variety/voice budget on `affect_arousal`?** Recommend YES —
   extend the existing arousal-gating from figuration choice (`texture` table) to
   voice count and per-layer variety depth, so calm images stay sparse. Pure-Rust,
   rides the existing composite.
3. **Adopt the §5 per-voice differentiation CI guard** (every added voice must
   differ in ≥1 strong segregation cue) as a build gate beside the ear test?
   Recommend YES — it catches thin multiplication at the moment of the defect, the
   way the S43 salience invariants caught the salience defect.
4. **Sequence vs S43 watch-items:** the S43 verdict left a pre-staged Lena melody
   lift (0.78→0.85) and a CounterMelody-blur watch-item (0.58→0.55). The
   CounterMelody un-stub (DP-1) *changes the perceptual basis* of that watch-item —
   resolve the un-stub first, then re-evaluate the 0.58 weight against a *real*
   counter-line, not the stub. Do not apply the 0.55 tweak before un-stubbing.

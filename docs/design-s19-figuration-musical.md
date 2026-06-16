# Design S19 — Accompaniment Figuration: animating held harmony (the multi-voice rhythmic-pattern layer)

**Author role:** Music Theory Specialist (DESIGN ONLY — no source, test, or asset modified by this document; `docs/` only).
**Date:** 2026-06-15.
**Status:** PROPOSE-FOR-ITERATION. This is the musical design for "Slice 3" of the Composition Architecture arc — the fuller **accompaniment-figuration system** that S18 §7 explicitly deferred ("Alberti bass, comping patterns, on/off-beat placement, beat-position- and style-dependent figuration"). The S18 single counter-line is the one-line down-payment on this layer; this doc designs the layer it down-paid on.
**Builds on (read these first):** `spec-s18-slice2-build.md` (the saliency reader + counter-melody contract this rides on), `review-S18.md` (the QG PASS, incl. the two SOUND sound-deviations: locked center-surround weights 0.5/0.35/0.15, and the counter keeping the root pc de-prioritized-but-legal), `design-s16-texture-musical.md` / `design-s16-texture-engine.md` (the 4–5 layer texture model MELODY/COUNTER-MELODY/PAD/BASS/AMBIENCE and the saliency→layer throughline), and `assessment-composition-architecture.md` (the 10-stage roadmap; figuration is the accompaniment/texture completion of Stages 2.5 + 9).

**Grounded against the actual HEAD working tree (S18 as-built, verified — not trusted from the S16 docs which predate the build):**
- `src/chord_engine.rs`: `OrchestralRole::{Pad, CounterMelody}` (`:813`/`:818`); `assign_role` (`:918`, profile-aware, delegates to `instrument_role` under identity); `realize_step` (`:956`, PUBLIC signature FROZEN, threads `ctx` + `pad_voices`); `realize_rhythm` (`:1259`, private free fn, receives `pad_voices` + `ctx` — the additive-private-param precedent); the `Pad` arm (`:1419`, multi-NoteEvent inner-tone bed, `PAD_OVERLAP_FRAC` = 1.10) and the real `CounterMelody` arm (`:1480`); `role_pitch` (`:1045`, Pad/Counter share the inner-tone seat `:1089`); register floors `BASS_REGISTER_FLOOR`=36 / `FILL_REGISTER_FLOOR`=55 / `MELODY_REGISTER_FLOOR`=67 (`:1039`–`:1041`); the `sustained` closure (`:1349`, `(frac*rit).min(1.20)` cap); `has_parallel_perfects` (`:2055`, works on arbitrary-length voicings AS-IS), `upper_voice_candidates` (`:2021`), `voice_lead_one`, `counter_candidate_pitches`/`pick_counter_pitch`/`nearest_counter_tone` (S18 helpers `:2308`/`:2347`/`:2287`), `held_run_position`/`advancing_seed_counter` (the held-run rotation, `:2215`/`:2252`); `Chord { notes: Vec<u8> }` (`:34`); `StepPlan` (`:495`, `chord`/`position_in_phrase`/`phrase_len`/`position`); the cadence early-return (`:1370`); `FILL_REST_ACTIVITY`=0.10, `STACCATO_FRAC`=0.40, `PORTATO_FRAC`=0.70, `ARTIC_WINDOW_LO`=0.55, `ARTIC_WINDOW_HI`=1.10.
- `src/composition.rs`: `OrchestrationProfile { id, layers: Vec<LayerRole>, density, pad_voices }` (`:208`); `LayerRole { Bass, HarmonicFill, Melody, CounterMelody, Pad }` (`:196`); `Section.orchestration` (`:518`) + `Section.density` (`:511`, 0..1, currently a no-op default 0.5); the `Knob`/`Predicate`/`SelectRule`/`SelectTable` first-match-wins machinery (`:281`–`:401`); the S18 energy knobs `SubjectEnergy`/`ForegroundEnergy`/`BackgroundEnergy` (`:296`–`:298`) + their `Knob::read` arms; `texture`/`texture_catalogue` on `PlanMappings` (`:424`/`:430`); the planner's `texture.select(u)` (`:707`) selecting ONE profile per plan; `ImageUnderstanding` saliency fields incl. `subject_energy`/`foreground_energy`/`background_energy`/`fg_bg_contrast`/`subject_size`/`mass_centroid` (`:73`–`:87`).
- `src/pure_analysis.rs`: the S18 region reader filling those fields (3×3 rule-of-thirds, center-surround subject pick, energy == region `edge_energy`).
- `assets/mappings.json`: `texture_catalogue` = `identity` / `pad_bed` / `pad_bed_counter` (`:137`–`:140`); the `texture` SelectTable (`:142`–`:149`, default `pad_bed`, one rule → `pad_bed_counter` on `foreground_energy ≥ 0.35 ∧ fg_bg_contrast ≥ 0.20`).
- `src/main.rs`: the scheduler (`:481`–`:495`) sorts a step's events by absolute time and blocks until the LAST note_off — so every figure's onsets+holds MUST live within the step window (plus the ≤1.20× `sustained` cap the Pad/Counter already use). **No `main.rs` change. No scheduler change.**

---

## 0. Executive summary (read this first)

S18 gave the accompaniment ONE moving inner line (the CounterMelody) and a sustained PAD bed. Both currently emit a static rhythmic shape: the Pad holds `pad_voices` chord tones at offset 0 for the whole step; the counter sounds one note (offset 0, or offset `step_ms/4` in its moving/held-period mode). What is still missing is **figuration** — the rhythmic PATTERN by which a held harmony is *animated across the beat*: the Alberti bass's low-high-mid-high broken chord, the waltz oom-pah, the off-beat comp, the arpeggiated sweep, the syncopated anticipation. These are what make an accompaniment sound *played* rather than *sustained*. Today every supporting voice either holds or stabs once; the harmony exists but is rhythmically inert underneath the tune.

This design adds a **figuration vocabulary as data** (`mappings.json` rows) and a **deterministic first-match-wins selection** (a `SelectTable`-style ladder over `ImageUnderstanding` knobs, NO RNG) that chooses a figure per layer per section. The figure is realized by a NEW `realize_figure` helper the `Pad` arm (and optionally a future Comp role) calls — it expands ONE chord into a short ordered sequence of NoteEvents whose onsets/holds/pitch-selection encode the pattern, entirely within the step window. The **load-bearing operator requirement** — *"the most prevalent subject(s) play more of a role in the melody, the least prominent layer plays a larger role in the background"* — is met by making **per-layer figuration richness a function of the S18 saliency reading**: a salient, high-contrast subject drives the *foreground* layers (melody + a richer counter/comp) to a denser, more prominent figure, while low subject prominence pushes the interest down into a *sparser, subtler* background bed. The music "matches the image" because the image's *structure* (where the energy is) decides *which layer is busy*.

**Recommended first build slice (Slice 3a):** the **single-step broken-chord / Alberti figure on the PAD layer in 4/4**, selected by a small data ladder keyed on the existing `Section.density` + the saliency energies, with restraint guards (calm image ⇒ block/sustained, never figurated). Cheapest highest-value slice: it animates the one layer that today is most inert (the held Pad bed) with no new role, no meter dependency, no counter-melody change, and the engine byte-freeze fully preserved (figuration lives only on the compose path behind a non-identity profile, exactly the S17/S18 boundary).

---

## 1. The figuration vocabulary (the catalogue)

A figure is a **rhythmic-pattern template** that takes (a held chord, a step's ms budget, a target voice-count, a register band) and produces an ordered `Vec<NoteEvent>` *within one step* (or, where noted, spanning the metric unit once meter exists). Each figure below specifies: its **onset pattern** across the step, **which chord tone sounds when** (the pitch-selection rule), its **register**, its **voice count** (simultaneous notes per onset), and the **affect/style** it conveys.

The unit of all 4/4-ready figures is **one step = one beat** (the S13/S15 convention). Because the scheduler blocks until a step's last event (`main.rs:483`), a figure subdivides the *current step* into onsets; it does NOT span multiple steps. (Cross-step / per-measure figures — true Alberti over a 4-beat bar, oom-pah-pah over a 3/4 measure — are flagged **METER-DEFERRED** and belong to the Meter stage, Stage 3, which introduces `beats_per_measure` and `metric_position`.)

Notation below: a step is divided into `n` equal slots; `[t0 t1 …]` lists the onset slots; pitch tokens are chord-tone roles — **R** root, **3** third, **5** fifth, **7** seventh, **T** "top inner tone", **L/M/H** low/mid/high of the seated voicing.

### 1.1 The classic set — 4/4-ready (current mechanical default)

| Figure id | Onset pattern (per step) | Pitch per onset | Voices/onset | Register | Affect / style | Notes |
|---|---|---|---|---|---|---|
| **`block_sustained`** | `[0]`, held full step (≤1.10×) | all `pad_voices` inner tones together | `pad_voices` | fill band (G3+) | the S18 Pad bed — calm, choral, "background" | THE DEFAULT / restraint floor; identical to today's Pad arm. The vocabulary's no-op. |
| **`block_pulse`** | `[0 1 2 3]` (n=4), each short (PORTATO) | all inner tones together each onset | `pad_voices` | fill band | steady on-beat chordal pulse; hymn/march comp | re-strikes the same voicing each onset — restrained, NOT busy; use only on medium-density. |
| **`offbeat_comp`** | `[1 3]` of n=4 (the "and" of 1 and 2) | all inner tones together | `pad_voices` | fill band | jazz/pop off-beat comp; lifts against the downbeat | the downbeat is LEFT to bass+melody; the comp answers off the beat. Restraint-friendly (only 2 onsets). |
| **`broken_up`** | `[0 1 2 3]` (n=`pad_voices+`), ascending | L→M→H→(M) ascending through seated tones | 1 | fill band | rising arpeggiated bed; opening, lifting | one note per onset = a true broken chord, the canonical "animate the held chord". |
| **`broken_down`** | `[0 1 2 3]`, descending | H→M→L→(M) descending | 1 | fill band | settling, falling figure; resolving | mirror of `broken_up`; pairs with descending melodic contour. |
| **`alberti_step`** | `[0 1 2 3]` (n=4) | **L H M H** (low, high, mid, high) | 1 | fill band | the Alberti bass shape — flowing, classical, "kept-moving" inner figure | the single-step Alberti CELL (low-high-mid-high within one beat). The *bar-spanning* Alberti is METER-DEFERRED; this is its in-beat reduction. |
| **`arp_sweep`** | `[0 .. n-1]` (n = `pad_voices`, up to 4) | each seated tone in turn, low→high | 1 | fill band | a single quick arpeggiated rake of the chord | denser than `broken_up` (covers all voices once); the "harp roll" affect; gate to busy images. |
| **`anticipation`** | `[0]` held, PLUS a single onset at `3/4` that lands the NEXT chord's tone early | inner tone (anticipating) | 1 | fill band | syncopated push into the next harmony | the in-step syncopation lever; needs the next chord (re-derivable from `ctx.section.steps[si+1]`, the same recompute the counter uses for the prior step). |

### 1.2 METER-DEFERRED (need a measure > 1 beat; defer to Stage 3)

| Figure id | Needs | Affect | Why deferred |
|---|---|---|---|
| **`waltz_oompah`** | 3/4 measure (beat 1 bass, beats 2–3 chord) | lilting waltz | requires `metric_position` within a 3-beat measure; there is no meter today (a "step" is a flat beat, no measure grouping). |
| **`stride`** / **`ragtime`** | ≥2-beat measure, low bass on 1&3, chord on 2&4 | stride piano, ragtime | needs the alternating strong/weak *measure* beats; same blocker. |
| **`alberti_bar`** | 4-beat measure | full classical Alberti | the canonical Alberti spans the whole bar (beat1 low, beat2 high, beat3 mid, beat4 high); only its in-beat reduction (`alberti_step`) ships in 4/4. |

**Why the meter split is principled:** today `realize_rhythm` sees `step.position_in_phrase`/`phrase_len` but NO measure structure — every step is an undifferentiated beat. A figure whose identity is "bass on beat 1, chord on beats 2–3" cannot be expressed when the realizer cannot tell beat 1 from beat 2 of a measure. The in-step figures above are exactly the subset that DON'T need that distinction (they subdivide a single beat), so they ship now; the measure-spanning figures wait for the Stage-3 `metric_position` thread (which the assessment §A.4 already designs as "re-point accent from `position_in_phrase` to `metric_position`" — the same scalar a measure-figure would read).

---

## 2. Selection — deterministic, data-driven, no RNG

Figuration selection follows the project's standing **content-as-data** discipline exactly as `form`/`texture` already do: the vocabulary lives as `mappings.json` rows, and selection is a bounded **first-match-wins `Predicate` ladder** over `ImageUnderstanding` knobs. **No `thread_rng` anywhere in selection** (the QG net for S18 grepped this clean; this slice preserves it).

### 2.1 The data shape (mirrors `texture_catalogue` / `OrchestrationProfile`)

Add a `figuration_catalogue` (the vocabulary, §1.1) and a per-LAYER selection axis. Two clean options; **recommend Option A** (figure as a profile FIELD) for the first slice:

- **Option A — figure id attached to the orchestration profile (recommended Slice-3a).** Extend `OrchestrationProfile` with an optional `pad_figure: String` (default `"block_sustained"`, `#[serde(default)]` so old mappings parse unchanged → identical behavior). A profile then names not just *which layers* but *how the Pad layer is figurated*. Selection rides the EXISTING `texture` SelectTable: new profiles (`pad_bed_alberti`, `pad_bed_broken`) are selected by saliency/density predicates exactly as `pad_bed_counter` is today. **Zero new selection mechanism** — it reuses the shipped `texture.select(u)` path. This is the cheapest correct first slice.

- **Option B — a dedicated `figuration` SelectTable per layer (later slices, when several layers figurate).** A `figuration: { pad: SelectTable, comp: SelectTable }` axis mapping to `figuration_catalogue` ids, selected per layer. More expressive (independent per-layer choice) but adds a selection axis; defer until more than one layer figurates.

### 2.2 The selection AXES (which knobs decide the figure)

A figure is chosen along three principled axes, ALL data-expressible as `Predicate`s over existing knobs:

1. **Density-dependent** (the primary axis): ride the existing `Section.density` (0..1, currently a no-op) and/or the `OrchestrationProfile.density`. Low density ⇒ `block_sustained` (restraint). Medium ⇒ `block_pulse` / `offbeat_comp` / `broken_up`. High ⇒ `alberti_step` / `arp_sweep`. Density itself derives from `edge_activity` (the image's busyness) — already on `ImageUnderstanding`. This axis is what enforces "don't over-busy a calm image."

2. **Character/style-dependent** (METER-coupled, partly deferred): the `Character` enum (`Ballad`/`March`/`Waltz`/…) biases the family — Ballad → sustained/broken (legato); March → `block_pulse` (firm on-beat); Waltz → `waltz_oompah` (deferred). Slice-3a is Ballad-only (the current pin), so this axis ships as schema and activates with Stage-4 character. Within Ballad, `colorfulness`/`avg_saturation` can nudge broken-vs-Alberti (a richer palette → the more ornamental Alberti).

3. **Beat-position-dependent** (in-step, available NOW; full version METER-deferred): WITHIN a chosen figure, the realizer already reads `step.position_in_phrase`/`phrase_len`/`pre_cadence`. A figure can thin at phrase starts (let the harmony announce) and the existing `pre_cadence` acceleration already drives more onsets into the cadence. The FULL beat-position axis (figure varies by *measure beat*) is METER-deferred with §1.2; the *phrase-position* sub-axis ships now (it reuses fields the realizer already has).

### 2.3 Example `mappings.json` rows (illustrative — the Implementer is the sole writer of `mappings.json`)

```jsonc
// figuration ids carried on the profile (Option A). Old profiles omit pad_figure
// → serde default "block_sustained" → byte-identical to today's Pad bed.
"texture_catalogue": [
  { "id": "identity",         "layers": [],                                          "density": 0.5,  "pad_voices": 0 },
  { "id": "pad_bed",          "layers": ["Bass","Pad","HarmonicFill","Melody"],      "density": 0.55, "pad_voices": 3, "pad_figure": "block_sustained" },
  { "id": "pad_bed_counter",  "layers": ["Bass","Pad","CounterMelody","Melody"],     "density": 0.6,  "pad_voices": 3, "pad_figure": "block_sustained" },
  // NEW S19 — a foreground-busy subject earns a FIGURATED pad (broken chord),
  // animating the held harmony underneath the richer foreground.
  { "id": "pad_bed_broken",   "layers": ["Bass","Pad","CounterMelody","Melody"],     "density": 0.7,  "pad_voices": 3, "pad_figure": "broken_up" },
  { "id": "pad_bed_alberti",  "layers": ["Bass","Pad","CounterMelody","Melody"],     "density": 0.8,  "pad_voices": 3, "pad_figure": "alberti_step" }
],
"texture": {
  "default": "pad_bed",
  "rules": [
    // STRONG salient subject + busy foreground → the richer Alberti figure (most prominent
    // foreground interest). First-match-wins, so this beats the broken & counter rules.
    { "when": [ {"knob":"fg_bg_contrast","op":"ge","lo":0.40},
                {"knob":"foreground_energy","op":"ge","lo":0.55} ], "pick": "pad_bed_alberti" },
    // Real subject + busy foreground → a broken-chord bed + the counter line.
    { "when": [ {"knob":"foreground_energy","op":"ge","lo":0.35},
                {"knob":"fg_bg_contrast","op":"ge","lo":0.20} ], "pick": "pad_bed_broken" },
    // (the shipped S18 counter rule can remain as a lower-priority fallback if desired)
  ]
}
```

Restraint is encoded structurally: a calm image (low `foreground_energy`, low `fg_bg_contrast`) matches NO rule and falls to `pad_bed` (`block_sustained`) — the figure ladder only ever *adds* animation when the image's energy/structure warrants it. This is the data-level expression of §5's "restraint as a value."

---

## 3. ★ FIRST-CLASS — the image → layer-role mapping (the explicit operator requirement)

The operator's verdict, load-bearing: *"the most prevalent subject(s) may play more of a role in the melody, whereas the least prominent layer may play a larger role in the background."* This section designs exactly how the S18 saliency reading drives **which layer carries the figurative interest and how richly**, and is the reason the music "matches the image."

### 3.1 The principle: saliency → per-layer figuration BUDGET

Treat each layer as having a **figuration budget** — a 0..1 richness scalar that selects how dense/prominent that layer's figure is (from `block_sustained` at 0, through `broken`/`comp`, to `alberti`/`arp_sweep` at 1). The S18 region reader gives three energies plus the contrast/size gates; map them to per-layer budgets so the busiest layer tracks where the image's energy actually is:

| Layer | Figuration budget driven by | Direction | Musical consequence |
|---|---|---|---|
| **MELODY** (foreground/subject) | `subject_energy` × `fg_bg_contrast`, lifted by `subject_size` | salient subject ⇒ HIGH | the tune is the most prominent, most active line (already partly true — melody subdivides on high `edge_activity`; this makes it *explicitly* track the SUBJECT region, not whole-image average). |
| **COUNTER / COMP** (foreground) | `foreground_energy`, gated by `fg_bg_contrast ≥ 0.20` | busy foreground ⇒ MEDIUM-HIGH | the foreground's secondary energy becomes a *moving* counter / a *comping* figure — richer when the foreground is active, absent/sustained when it is quiet (the shipped S18 counter-presence rule, now extended to counter *richness*). |
| **PAD** (background bed) | inverse of subject prominence: high when `fg_bg_contrast` is LOW (flat field, no subject), low/sustained when a subject dominates | low-prominence image ⇒ the bed carries MORE | **this is the operator's "least prominent layer plays a larger role in the background"**: when there is no salient subject to carry the foreground, the *background bed* itself takes the figurative interest (a gently broken/animated pad), so a subject-less image still moves. When a subject DOES dominate, the pad recedes to a sustained bed so it does not compete with the prominent foreground. |
| **BASS** | `background_energy` (low, capped) | mostly steady | the floor stays sparse; a busy background may add at most a single pickup (the existing pre-cadence bass behavior), never a figure that muds the bottom. |

### 3.2 Why this is "the music matching the image"

The mapping makes **figurative prominence track the image's spatial energy distribution**, not its average:

- **A clear subject on a quiet ground** (high `fg_bg_contrast`, energy concentrated in `subject_energy`): the MELODY is prominent and active, a counter line weaves in the foreground, and the PAD recedes to a calm sustained bed — the natural "song" texture, the tune over a quiet accompaniment. The subject literally drives the most prominent layer.
- **A busy, subject-less abstract** (low `fg_bg_contrast`, energy spread): no single layer dominates, so the figurative interest sinks into the PAD/background — a broken or pulsing bed animates the harmony, the music stays moving without a false "subject" line. The least-prominent-element-in-the-image gets the larger background role.
- **A dark, low-energy image** (low all-energies): every budget is low ⇒ `block_sustained` everywhere ⇒ a calm, deep, sustained texture — restraint, because there is nothing in the image to animate.

This is the precise attack on the operator's earlier "ethereal / structureless / unrelated to the image" verdict: figuration density is now a *spatial* property of the image (where is the energy?) rather than a uniform wash.

### 3.3 How the budget is realized DETERMINISTICALLY (no RNG)

The budget is NOT a float multiplied at runtime — it is the same data-ladder selection as §2, expressed as predicates. The per-layer budget mapping above becomes `texture` SelectTable rows (e.g. the §2.3 rows: `fg_bg_contrast` high + `subject_energy` high ⇒ `pad_bed_broken`/`_alberti` which ALSO sets the pad figure; a flat field with energy ⇒ a future `pad_bed_animated_bg` profile). Each layer's figure id is chosen at plan-build time by first-match-wins predicates over the S18 knobs — fully deterministic, RNG-free, and tunable in JSON. The "budget" is a design *concept* that the discrete profile rows quantize; the runtime only ever does `texture.select(u)` + a catalogue lookup, exactly as today.

---

## 4. Interaction with existing craft

Figuration produces **multiple NoteEvents per step** (it animates held harmony), so it must coexist cleanly with the S18 counter-line, the pad bed, the bass, voice-leading, and parallel-perfects avoidance. The rules:

### 4.1 Where figuration LIVES (which voice carries it)

Figuration is the **PAD layer's job first** (Slice-3a). The Pad arm today seats the inner tones (root-skipped, §1.1 `block_sustained`); `realize_figure` replaces that single block emission with the chosen figure's onset sequence over the SAME seated inner tones. The Bass, Melody, and CounterMelody arms are **untouched** by Slice-3a — so the existing ≤1-event counter ceiling (S18 §3.6 test 7) and the melody's own articulation bands are preserved exactly. A later slice may add a dedicated **Comp** behavior to the CounterMelody/HarmonicFill register, at which point the per-layer §3 budget chooses comp richness; until then the counter remains the single moving line it is, and figuration is purely the pad's animation.

### 4.2 Avoiding mud — register separation and voicing

The seated figure tones come from the SAME `seat_pc_in_register(pc, FILL_REGISTER_FLOOR)` + de-dup the Pad arm already uses (`:1452`–`:1462`), so the figure stays in the **fill band [55, 67)**, under the melody (≥67) and above the bass (≈36). Discipline:
- **One-voice figures** (`broken_*`, `alberti_step`, `arp_sweep`): only ONE note sounds per onset, so they are *thinner* than the current block bed, not muddier — they trade simultaneity for motion. This is inherently anti-mud.
- **Multi-voice figures** (`block_pulse`, `offbeat_comp`): re-strike the full seated voicing, but only on the figure's onsets (2–4 per step), and the de-dup + root-skip already prevent unison collapse. Velocity is the Pad's supporting (quieter) level so it never competes with the melody.
- **The bass is never figurated** in this slice — the floor stays a single sustained root, so no two voices crowd the bottom octave.

### 4.3 Voice-leading and the held-run interaction

`voice_lead_one`/`voice_lead_sequence` voice the *chords* at plan-build time (vertical smoothness); figuration is a *horizontal/rhythmic* realization of the already-voiced chord at tick time — they compose cleanly, exactly as the S16 doc noted the counter-melody does ("no rewrite of voice leading; an additive realization on top"). The figure picks its onset pitches from `step.chord.notes` (already voice-led), so smoothness is inherited. The S18 **held-run rotation** (`advancing_seed_counter`) is the counter's; the pad figure does NOT use it — across a held chord, the pad simply repeats its figure (a broken chord re-broken each step is musically correct and is itself the "something moves underneath the held harmony" answer, complementary to the counter's rotating line). The two together — a rotating counter + a re-articulated broken pad — give a held chord *two* independent motions, which is exactly the rich-accompaniment goal.

### 4.4 Parallel perfects

Figuration is **monophonic-per-onset or homorhythmic** within the pad's own seated voicing; it introduces no NEW independent line against the melody (that is the counter's role, already guarded by the `has_parallel_perfects` call site at `:2400`). A broken-chord pad cannot form parallel perfects with the melody in the contrapuntal sense (it is an arpeggiation of a single harmony, not a second voice moving in parallel). **No new `has_parallel_perfects` call site is required for the pad figure.** If a later Comp role becomes a genuine second moving line, it inherits the counter's exact parallel-perfects guard.

---

## 5. Musical-correctness guardrails (intentional vs sloppy)

The operator's stated fear — *"naive extra chord stabs sound sloppy"* — is the design's central constraint. What makes figuration sound intentional rather than mechanical:

1. **Restraint is the default and a value.** `block_sustained` is the floor; a figure is selected ONLY when the image's energy/structure earns it (§2.2 density axis, §3 saliency budget). A calm image gets a calm sustained bed. The vocabulary is biased toward *under*-animating: the ladder adds motion conservatively, first-match-wins from the richest qualifying rule but with high thresholds (`fg_bg_contrast ≥ 0.40` for Alberti). **Never figurate a low-energy image.**

2. **Beat hierarchy.** Onsets land on metrically sensible positions: figures place onsets at slot boundaries (`0`, `n/4`, `n/2`, `3n/4`) — never at arbitrary ms offsets — so the figure reads as *rhythm*, not jitter. The downbeat (offset 0) is reserved for the structurally important onset; `offbeat_comp` deliberately AVOIDS the downbeat (leaving it to bass+melody) which is *why* it sounds like intentional comping, not a stab.

3. **Anticipation/resolution.** The `anticipation` figure resolves: it sounds the NEXT chord's tone early, which the next step then confirms — a planned syncopation, not a random early note. It is gated to medium-high density so it appears as a deliberate push, not noise.

4. **Don't re-strike for its own sake.** A broken chord (one note per onset) is preferred over a re-struck block (`block_pulse`) wherever motion is wanted, because re-striking the same voicing every onset is exactly the "pulsing block-chord" the S16 doc diagnosed as sloppy. `block_pulse` is reserved for explicitly firm characters (March) where the on-beat chord IS the affect.

5. **Velocity restraint.** The pad figure sounds at the Pad's supporting (quieter) level (the existing `velocity` thread), so even a busy figure stays *underneath* the melody — animation without competition. A figure that is as loud as the tune sounds sloppy regardless of its rhythm.

6. **One figure per layer per section** — the figure is stable within a section (selected once at plan-build), so the accompaniment has a *consistent* texture the ear can settle into, rather than thrashing between patterns step-to-step. Variation comes from the harmony moving under a stable figure, the way real accompaniment works.

---

## 6. Staged slicing recommendation

Figuration is large; decompose into shippable slices, cheapest-highest-value first, each byte-freeze-safe and independently hearable.

| Slice | What ships | New DATA vs MECHANISM | Hearable win | Independence |
|---|---|---|---|---|
| **3a — PAD broken-chord/Alberti in 4/4 (RECOMMENDED FIRST)** | `realize_figure` helper + the `block_sustained`/`broken_up`/`broken_down`/`alberti_step` figures on the Pad arm; `pad_figure` field on `OrchestrationProfile` (serde-default `"block_sustained"`); `pad_bed_broken`/`pad_bed_alberti` profiles + the saliency/density texture rules | **DATA:** figure ids on profiles + 2 texture rules. **MECHANISM:** one `realize_figure` fn + the Pad-arm call. | the held harmony stops being inert — a salient image's bed *moves* (broken/Alberti) under the tune, a calm image stays sustained | self-contained; touches only the Pad arm + `OrchestrationProfile` + `mappings.json` |
| **3b — comp figures + the per-layer budget** | `offbeat_comp`/`block_pulse`/`arp_sweep`; the per-LAYER `figuration` SelectTable (Option B); the §3 saliency→budget rows for melody/counter/bg | mostly DATA + the per-layer axis | the foreground vs background figuration split becomes audible; busy-abstract gets an animated bg bed | needs 3a's `realize_figure` |
| **3c — anticipation / in-step syncopation** | `anticipation` (reads the next chord via `ctx.section.steps[si+1]`) | MECHANISM: one next-chord recompute | syncopated push into chord changes | needs 3a |
| **3d (METER-GATED) — waltz/stride/bar-Alberti** | the §1.2 measure-spanning figures | DATA + reads `metric_position` | true oom-pah, stride, bar-Alberti | **gated on Stage 3 (meter)** — cannot ship until `metric_position` exists |

### 6.1 The recommended first slice (Slice 3a), precisely

**Build the single-step broken-chord / Alberti figure on the PAD layer, in 4/4, selected by a data ladder over the S18 saliency energies + density, with `block_sustained` as the restraint default.** Concretely:

- **`realize_figure(chord, seated_inner_tones, step_ms, velocity, figure_id) -> Vec<NoteEvent>`** — a NEW private helper in `chord_engine.rs`, Music-Theory-owned. It expands the figure id into the onset sequence (§1.1) over the seated inner tones (the SAME `seat_pc_in_register`/de-dup tones the Pad arm builds at `:1450`–`:1463`), entirely within the step window (onsets at slot boundaries, holds ≤ the `(frac*rit).min(1.20)` cap — same discipline as the Pad's `PAD_OVERLAP_FRAC`). `block_sustained` returns exactly today's Pad emission (byte-identical), so a profile with the default figure is unchanged.
- **The Pad arm (`:1419`) calls `realize_figure`** with `ctx.section.orchestration.pad_figure` instead of emitting the block directly. Under the identity profile no instrument is ever a Pad, so this is inert on the freeze path.
- **`OrchestrationProfile` gains `pad_figure: String` (`#[serde(default = ...)]` → `"block_sustained"`)** — additive, old mappings parse and behave identically.
- **`mappings.json`** gains `pad_bed_broken`/`pad_bed_alberti` profiles + 2 `texture` rules keyed on `fg_bg_contrast`/`foreground_energy` (§2.3). Restraint: a calm image matches no rule → `pad_bed` → `block_sustained`.

**Why this is the cheapest highest-value first slice:** it animates the single most-inert layer (the held Pad bed), needs **no new role**, **no meter**, **no counter-melody change**, **no scheduler change**, and **no new image feature** (rides the shipped S18 saliency knobs). It directly delivers the operator's "the bed should move when the image has a subject" intuition, and every later slice enriches a working `realize_figure`.

---

## 7. Hard constraints (stated where they bind)

- **Byte-freeze.** `engine_equivalence` goldens (240/114/84/36/79) stay byte-green: figuration is reachable ONLY through a non-identity `OrchestrationProfile` (`pad_voices > 0`, a named Pad layer), which `assign_role` never returns under the identity profile the net carries. The `single_section_default` identity path is untouched (figuration lives only on the compose path — the S17/S18 boundary). `block_sustained` makes the default figure byte-identical to today's Pad bed even when a Pad IS present, so any profile keeping the default figure is unmoved. `realize_step`'s PUBLIC signature is FROZEN — `realize_figure` is a NEW private helper the Pad arm calls; `realize_rhythm` already receives everything it needs (`ctx` + `pad_voices`, the additive-private-param precedent). No golden moves in Slice 3a.
- **Vocabulary as DATA.** Figures are `mappings.json` rows (figure ids on profiles, or a `figuration_catalogue`); selection is a bounded first-match-wins `Predicate` ladder over `ImageUnderstanding` knobs — **no RNG in selection** (preserves the S18 grep-clean property).
- **LOCKED-OFF files figuration must not touch:** the modem (`src/modem.rs`, `src/bin/modem_*`), the output/UI side (`src/synth_sink.rs`, `src/midi_output.rs`, `src/cli.rs`, `src/tui.rs`), and `main.rs` (the scheduler blocks until the step's last event — true cross-step sustain is a separate deferred concern; every figure lives within step timing + in-step off-beat onsets, the S18 pattern). The measure-spanning figures (§1.2) are METER-deferred precisely because expressing them inside one beat is impossible and reaching across beats would need a scheduler change.
- The figure realizer is `chord_engine.rs`-only; `OrchestrationProfile`/the selection axis are `composition.rs`; the rows are `mappings.json` (Implementer is the sole writer of `mappings.json`). No internal codenames appear in this doc.

---

## 8. Divergences from the S16 texture docs (found against the real S18 code)

1. **`TextureProfile` → `OrchestrationProfile`; `Section.texture` → `Section.orchestration`** (S17 rename; this doc uses the as-built names).
2. **The S16 docs put figuration density on `TextureProfile.density` as a runtime band-bias scalar.** The as-built `OrchestrationProfile.density` is still a no-op (reserved schema). This doc instead selects a DISCRETE figure id per profile (quantized budget) — more honest to the shipped first-match-wins selection and avoiding an un-wired continuous knob. `Section.density` remains available for the §2.2 density axis but is expressed as predicates, not a runtime multiplier.
3. **The S16 "5th AMBIENCE layer" / `num_instruments` widening is NOT required for figuration** — figuration animates an EXISTING layer (the Pad) within the existing ensemble width, exactly as `pad_bed` swaps one inner instrument to a Pad. No ensemble widening.
4. **Per-onset pitch selection reuses the Pad's seated inner tones**, not a fresh `voice_lead` call — the voicing is already done at plan-build; figuration is purely the rhythmic realization of it (a divergence from the S16 implication that figuration re-selects pitches).
5. **The full beat-position axis is METER-deferred** — the S16 docs assumed meter; the as-built tree has none, so the in-step figure subset ships now and the measure-figures wait for Stage 3 (§1.2).

---

*Design-only. No source, test, or asset modified by this document. Illustrative `mappings.json` rows and the `realize_figure` signature are non-binding; the Implementer owns the final `mappings.json` and the Rust Architect owns the final private signatures.*

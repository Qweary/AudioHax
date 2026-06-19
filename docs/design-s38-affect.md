# S38 — Affect / Cross-Modal Design Note

**Author:** Affect Specialist (Perceptual / Cross-Modal Affect)
**Session:** S38 — design/diagnosis only. No `src/*` edited. `engine.rs` is byte-frozen; nothing here requires touching it.
**Scope I own:** the bridge from an image's *affect* (valence/arousal) and direct cross-modal correspondences onto musical *character* and *expressive parameters* (tempo, dynamics, density, articulation, register, mode, dissonance). I consume features; I never compute pixels. Music Theory owns the contour/voice-leading vocabulary in `chord_engine.rs`; Aesthetics owns the taste call; the Rust Architect owns the seam shape.

**Grounding anchors used throughout (with confidence):**
- **Valdez & Mehrabian (1994)** — brightness dominates *valence*; saturation dominates *arousal*. (HIGH confidence; large-N controlled study; directly matches the existing `affect_composite` weights.)
- **Eerola, Friberg & Bresin (2013)** — affect→music expressive cues (tempo, dynamics, articulation, register, mode) combine **roughly additively** and each carries an independent, measurable effect. Mode/major-minor and tempo are the two strongest single cues; register and articulation are real but weaker. (HIGH confidence for direction; MEDIUM for exact magnitudes — their effect sizes are corpus-relative, not absolute MIDI numbers.)
- **Load-bearing caveat (carried from the engagement):** major/minor is owned by **valence**, NOT raw hue. The six-mode hue spread is *within-family garnish*. This is already correctly implemented by `valence_family_mode` (composition.rs:265) + `mode_valence_cuts` (mappings.json:167). Any new affect row must not re-introduce hue as the third-decider.

---

## 0. What the affect pipeline actually does today (code-grounded)

The composite is computed **once per plan** in `CompositionPlanner::plan` (composition.rs:1260) and seated on a working copy of `ImageUnderstanding` (`affect_arousal`, `affect_valence`), so every downstream `SelectTable` ladder reads real 0..1 values instead of the `-1.0` sentinel.

- **AROUSAL** = `0.45*(sat/100) + 0.25*colorfulness + 0.20*edge_activity + 0.10*complexity` (mappings.json:148). Saturation-led — correct per Valdez & Mehrabian.
- **VALENCE** = `0.70*(bright/100) + 0.20*(sat/100) + 0.10*(0.5+0.5*fg_bg_contrast)` (mappings.json:154). Brightness-led — correct.

Where affect currently *lands*:
- **Character** ladder (mappings.json:169) → ballad/scherzo/march/lament/hymn/nocturne. Both axes gate it.
- **Tempo** — `character_tempo_bpm` (composition.rs:372) clamps the brightness→BPM into the chosen character's window. So affect reaches tempo **only indirectly**, by picking the character whose window the raw BPM is then clamped into.
- **Texture / figuration** ladder (mappings.json:305) → arousal/valence gate which comping figure (block_comp, broken_up, stride, oom_pah…).
- **Mode family** — `valence_family_mode` (composition.rs:1297). Correct and load-bearing.

Where affect currently **does NOT reach at all** — this is the spine of two of the three findings:
- **Starting register / key center.** `home_root_midi = 60` is hardcoded (composition.rs:1302; legacy cli.rs:234). No affect, no feature, nothing moves it. Excursions move *section* roots via `offsets[i]`; the *home tonic* never leaves C4.
- **Density spine.** `Section.density` is set ONLY from per-region energy carried in `offsets[i]` (composition.rs:1416–1451): `f(e) = clamp(0.5 + 0.30*(e−0.5), 0.35, 0.65)`. **Home / home_only / identity sections are pinned to `HOME_ENERGY_NEUTRAL = 0.5` ⇒ density == 0.5 exactly** (the byte-stability proof at composition.rs:1221). **Arousal has no edge into density.** A calm-but-saturated image and a frenetic image produce the *same* home-section density. That is the inversion the operator hears.

**Two value corrections vs. the brief** (verified in source, matter for #3):
1. `FILL_REST_ACTIVITY = 0.10`, not 0.15 (chord_engine.rs:1436). The rest-as-gesture gate fires when normalized `edge_activity < 0.10` on a weak interior beat (chord_engine.rs:1703, 1860).
2. There IS already a density→activity coupling: `DENSITY_ACTIVITY_GAIN = 0.5` (chord_engine.rs:1403) nudges `edge_activity` by `(Section.density − 0.5)*0.5`, i.e. ≤ ±0.075. But because home-section density is pinned to 0.5, **that nudge is exactly 0 on every home section** — the coupling is dead where most of the piece lives.

---

## 1. Finding #1 — Every image starts on the same root/pitch (C4)

### Affect-lens diagnosis
The *missing signal* is twofold and they are separable:
- **(a) Register (octave).** There is no image→octave mapping. Every piece opens in the same C4-centered tessitura regardless of whether the image reads as a low, heavy, dark scene or a high, airy, bright one. Eerola/Friberg/Bresin treat register as an independent expressive cue: **high register → higher valence/lighter affect; low register → lower valence/heavier, more solemn affect.** We are leaving a real, additive cue at a constant.
- **(b) Key center (pitch class).** Hue→mode already chooses *which* mode; valence already chooses the *family*. But the absolute pitch-class anchor (C) is fixed. From the affect lens this matters far less than register — listeners do not hear "this image is in C vs. E♭" as an affect difference (absolute pitch is not an affective cue for non-AP listeners), but they **strongly** hear octave/register. So the two halves of this finding have very different leverage.

### Proposal (a): image → starting register (the part I own)

**Drive register from VALENCE, with a light arousal trim.** Grounding: brightness→valence (Valdez & Mehrabian) and valence→register (Eerola et al.) are the same direction — bright/pleasant images want a higher, lighter tessitura; dark/heavy images want a lower one. Using *valence* (which is brightness-led but already includes saturation + fluency) rather than raw brightness keeps the cue consistent with mode-family, so register and mode **reinforce** instead of fighting.

Proposed mapping (octave offset applied to the home root, **not** a new field — see coordination note):

```
home_octave_offset (semitones) = round( REGISTER_SPAN * (valence − 0.5) ) * 12   [snap to octaves]
   with a small arousal trim:   + round( REGISTER_AROUSAL_TRIM * (arousal − 0.5) ) * 12
   clamped to [REGISTER_FLOOR, REGISTER_CEIL]
```

Seed constants (all **garnish/tunable**, but the *direction* is load-bearing):
- `REGISTER_SPAN = 1.0` → valence alone moves the home root at most ±1 octave (valence 0 → −12, valence 1 → +12), snapping so most images land at −12 / 0 / +12. Center stays C4 at neutral valence — **preserves byte-stability** of the neutral/identity path (valence 0.5, arousal 0.5 ⇒ offset 0 ⇒ root 60).
- `REGISTER_AROUSAL_TRIM = 0.0` initially (garnish; can lift to ~0.3 later if the owner wants energetic images nudged up a register independent of brightness). Start at 0 so register is a *pure valence* cue we can A/B cleanly.
- `REGISTER_FLOOR = −12`, `REGISTER_CEIL = +12` (one octave each way). Keeps the line inside a singable/playable GM range and well clear of MIDI 0/127 clamping with the excursion offsets (max excursion +7 stacks safely on +12 → 79, fine).

Why octave-snapped, not continuous: a continuous register slide would fight the existing key-center logic and produce microtonal-feeling anchor drift. Snapping to octaves means the *pitch classes* (C, mode garnish, excursions) are untouched — only the tessitura moves. This is the smallest change that delivers the audible cue. **Confidence: HIGH on direction (valence→register is textbook), MEDIUM on the ±1-octave span being the sweet spot — the owner's ear should set the final span.**

**Subject-register note (deferred, lower confidence):** a more "correct" register cue would read the *subject's* vertical position (`vertical_emphasis`, composition.rs:71 — upper-mass emphasis) — a bird high in frame → high register, a foreground rock → low. This is a genuine cross-modal correspondence (spatial height ↔ pitch height is one of the most robust ones in the literature, confidence HIGH) and is arguably *more* faithful than valence→register. I recommend it as a **Phase 2** trim once valence→register is validated, because it interacts with subject saliency (Stage 9) and risks fighting valence on a dark-but-high-subject image. **Decision point for the operator (see §6).**

### Proposal (b): should the key CENTER carry affect?

**Recommendation: NO — leave the pitch-class anchor at C; move register instead.** Rationale: for non-AP listeners absolute key center is not an affective cue (confidence HIGH), whereas register is. Moving the *pitch class* would also collide with the excursion offsets (`relative_offset`, composition.rs:1578, and `region_related:b/c` schemes) which are computed relative to the home — changing the home PC just shifts everything rigidly with no perceptual payoff and new clamp-risk. The one thing worth changing about the "home" is its **octave**, which §1(a) does.

**How it composes with valence→mode-family:** cleanly, because they share the same driver (valence) in the same direction. valence high → major family (composition.rs:277) AND higher register → "bright + up" reinforce. valence low → minor family AND lower register → "dark + down" reinforce. There is no axis on which they can fight, which is exactly what we want and is why I route register off valence rather than off raw brightness or hue.

---

## 2. Finding #3 — Density feels reversed / too sparse (the part I own)

### Affect-lens diagnosis
**The arousal→density path does not exist.** The density spine is region-energy-driven and **pinned to 0.5 on every home/identity section** by design (composition.rs:1221–1225, the byte-stability proof). The only affect-reachable density lever — `DENSITY_ACTIVITY_GAIN` — multiplies `(density − 0.5)`, which is 0 there. So:
- Arousal cannot make a high-energy image fuller.
- The realizer's rest-as-gesture gate (`edge_activity < 0.10`) fires whenever normalized `edge_activity` is low — and `edge_activity` is only weakly lifted by density (and not at all on home sections). On a calm-but-coherent image (low edges, e.g. a smooth gradient or a calm portrait) the inner voices rest **constantly**, producing the "miles-davis scarcity."
- Net: density is not *inverted* so much as **un-wired and floored-low**. The neutral sits at 0.5 and the floor at 0.35, both of which read as sparse for a "fuller-by-default" target.

This is not a craft bug in `chord_engine` — it is a **missing affect→density edge** plus a **baseline set for a sparser aesthetic than the operator wants**.

### Proposed redesign (exact numbers)

Three moves, in order of leverage. All are **planner-side / constant changes** — `engine.rs` untouched.

**Move 1 (load-bearing): raise the baseline and re-center density fuller.** The operator wants "full by default, silence as a deliberate device." Today the algebra centers on 0.5. Re-center and lift:

| const | today | proposed | load-bearing? |
|---|---|---|---|
| `DENSITY_NEUTRAL` | 0.5 | **0.62** | LOAD-BEARING — this is the "fuller by default" knob |
| `DENSITY_FLOOR` | 0.35 | **0.45** | LOAD-BEARING — even calm images stay populated |
| `DENSITY_CEIL` | 0.65 | **0.80** | garnish — headroom for the busiest images |
| `DENSITY_ENERGY_SPAN` | 0.30 | **0.40** | garnish — slightly wider region-energy spread |

**Byte-stability caveat — this is the one real cost.** Raising `DENSITY_NEUTRAL` off 0.5 BREAKS the `HOME_ENERGY_NEUTRAL → f(0.5) == 0.5` identity that the engine_equivalence goldens lean on (composition.rs:1221). Two clean ways to handle it, operator's call (§6):
  - **(i) Re-baseline the goldens.** Accept that "fuller by default" is a deliberate new baseline and re-bless the equivalence net. Honest and simple; the goldens were freezing the *old* aesthetic, which is exactly what we're changing.
  - **(ii) Keep the identity, add a separate fill bias.** Leave `DENSITY_NEUTRAL = 0.5` (goldens hold) and introduce a NEW additive `DENSITY_FILL_BIAS = 0.12` applied *after* the identity check, so home sections still compute 0.5 internally but the realizer sees `density + bias`. More surgical, preserves the proof, but adds a concept. I lean (i) for honesty; the Rust Architect should weigh the golden-maintenance cost.

**Move 2 (load-bearing): give AROUSAL a direct edge into density.** This is the actual fix for "a frenetic image and a calm image sound equally sparse." Add an arousal term to the home/section density so arousal — not just region energy — drives fullness. Proposed planner formula (replacing the `section_density` computation at composition.rs:1450, conceptually):

```
arousal_term  = DENSITY_AROUSAL_SPAN * (arousal − 0.5)        // arousal lifts/lowers fullness
energy_term   = DENSITY_ENERGY_SPAN  * (energy_i − 0.5)        // existing region-energy spread
section_density = clamp(DENSITY_NEUTRAL + arousal_term + energy_term, FLOOR, CEIL)
```

with `DENSITY_AROUSAL_SPAN = 0.20` (**load-bearing direction, garnish magnitude**). Effect: a high-arousal image (arousal ~0.9) gets `+0.08`, pushing it toward the ceiling; a calm image (arousal ~0.2) gets `−0.06`, but the **raised floor (0.45)** stops it from emptying out. This is exactly "arousal drives density *without making calm images empty*." Confidence: HIGH on direction (arousal↔density/event-rate is one of the most robust affect-music cues, Eerola et al.), MEDIUM on the 0.20 span.

**Move 3 (garnish but recommended): make silence deliberate, not a low-activity accident.** Lower the rest-as-gesture gate so resting is rare and intentional rather than the default for any calm image:

| const | today | proposed | load-bearing? |
|---|---|---|---|
| `FILL_REST_ACTIVITY` | 0.10 | **0.05** | garnish — halves the rest-trigger band |

This makes the inner-voice rest a genuine *gesture* (only the very stillest textures) rather than a constant on calm images — directly serving "silence as a deliberate device." It is in `chord_engine.rs` (Music Theory's file), so it is a **coordination ask**, not mine to write (§7).

### Is the mapping under-powered or inverted?
**Under-powered, not inverted.** The *direction* of region-energy→density is right; the problems are (1) arousal never reaches it, (2) the home/identity floor pins most of the piece to a deliberately-sparse 0.5, and (3) the rest gate is too eager. Move 2 fixes (1); Move 1 fixes (2); Move 3 fixes (3). Highest single-item leverage inside #3 is **Move 1 re-centering** because it touches every section unconditionally; Move 2 is what makes density finally *track the image*.

---

## 3. Finding #2 — Too few distinct motifs (informing only)

### Affect-lens diagnosis
`pick_archetype` (composition.rs:1500) selects from only **4 of 8** archetypes via coarse hue buckets + one `edge_activity >= 0.6 → Ascent` override. Hue is doing the contour selection — which is the *same* mistake as the major/minor caveat: **hue is a colorist garnish, not an affect axis.** Two emotionally opposite images (a calm bright field vs. a tense bright field — same hue, different arousal) get the **same** contour. That is why only ~2 shapes are heard.

### Should affect spread the selection? YES — and this is the right division of labor
Music Theory owns the *contour vocabulary* (what Arch/Descent/Ascent/NeighborTurn etc. actually are). I own **whether affect spreads the selection across that vocabulary.** Recommendation: **re-key archetype selection off the affect quadrant, with hue as a within-quadrant tiebreak** — exactly mirroring the mode-family pattern that already works:

- **High arousal + high valence** → energetic-rising contours (Ascent-family).
- **High arousal + low valence** → agitated/angular contours (jagged, leaping — Music Theory names them).
- **Low arousal + high valence** → gentle arch / lyrical.
- **Low arousal + low valence** → descending / sighing / neighbor-turn-inward.

This is a 2×2 that immediately uses all four+ archetypes and guarantees emotionally-different images get audibly-different themes. Hue then picks *which* member of the quadrant's family — the garnish role it should have had all along. **I do not author the contour table** (Music Theory's call); I am asserting the **selection should be affect-quadrant-keyed, not hue-keyed**, and offering to supply the quadrant→family routing rows. Confidence: HIGH that affect-keying spreads the selection; the specific quadrant→shape pairings need Music Theory's craft sign-off.

---

## 4. The honest pure-Rust vs. ML line (per effect)

- **#1 register (valence→octave):** **Pure Rust, fully.** It is a threshold/scale on an already-computed scalar. No ML needed or wanted. The Phase-2 *subject-register* variant (vertical_emphasis) is also pure — the saliency it reads is already extracted.
- **#3 density (re-center + arousal term):** **Pure Rust, fully.** Arithmetic on existing scalars. No ML.
- **#2 motif spread:** **Pure Rust for the selection logic** (a 2×2 quadrant table). The *limit* of pure Rust is that it spreads across a **fixed hand-authored vocabulary** — it cannot invent new contours or read semantic content ("this is a soaring eagle → soaring line"). That semantic leap is where an ML/vision-embedding model would eventually help (image→contour as a learned mapping). **Honest line: heuristic-first gets us from 2 shapes to a faithful 6–8; semantic novelty is a future ML arc, not an S38 ask.** This matches the engagement's standing heuristic-first posture.

---

## 5. Highest-leverage finding (affect lens)

**#3 (density), and specifically Move 1 + Move 2 together.** Reasoning: density/event-rate is the cue that touches **every note of every section** unconditionally, and right now affect cannot move it at all on the home sections where most of the piece lives. It is also the finding the operator named as a *felt* problem ("miles-davis scarcity"), and the cheapest to fix decisively (constants + one added arousal term, no `chord_engine` craft, no `engine.rs`). #1 register is a close and very satisfying second (one octave of valence-driven tessitura is instantly audible), but it changes *where* the same number of notes sit, not *how full* the texture is. #2 is real but is gated on Music Theory's contour work and delivers variety-across-images rather than fixing a per-image quality complaint.

---

## 6. Decision points for the operator

1. **Density byte-stability (the one real cost):** re-baseline the engine_equivalence goldens (Move 1(i), honest) **or** keep `DENSITY_NEUTRAL = 0.5` and add a separate post-identity `DENSITY_FILL_BIAS` (Move 1(ii), surgical)? I lean (i); the Rust Architect should price the golden churn.
2. **Register driver:** valence-only (ship now, clean A/B) vs. add the subject *vertical_emphasis* trim now (more faithful, but interacts with saliency and can fight valence on dark-high-subject images). I recommend valence-only for S38, vertical_emphasis as Phase 2.
3. **Register span:** is ±1 octave the right reach, or does the owner (music-performance ear) want ±half-octave-snapped or a wider ±2? My seed is ±1; this is explicitly a taste call.
4. **Density target fullness:** `DENSITY_NEUTRAL = 0.62` is my read of "fuller by default" — the owner should confirm the number against their ear; it is the single most subjective constant here.

---

## 7. Coordination note — `mappings.json` is single-writer (shared with Music Theory)

I do **not** commit any of this. Proposed exact rows for whoever holds the `mappings.json` pen, grouped so Music Theory can review the contour-adjacent ones:

**New `composition.affect` register block** (mine; affect-owned):
```jsonc
"register": {
  "span": 1.0,            // valence → ±1 octave, snapped (REGISTER_SPAN)
  "arousal_trim": 0.0,    // off for S38 (REGISTER_AROUSAL_TRIM)
  "floor": -12,           // REGISTER_FLOOR (semitones off home)
  "ceil":  12             // REGISTER_CEIL
}
```
*(Planner reads these in place of the hardcoded `home_root_midi = 60` at composition.rs:1302 — the seed `60 + offset`. Neutral valence ⇒ offset 0 ⇒ root 60 ⇒ byte-stable on the neutral path.)*

**Density constants** (mine; affect-owned) — these are Rust `const`s in composition.rs:1226–1229, not JSON today. Proposal: either bump the consts directly **or** (cleaner) lift them into a `composition.affect.density` JSON block so the owner can tune by ear without a recompile:
```jsonc
"density": {
  "neutral": 0.62,        // was DENSITY_NEUTRAL 0.5      [LOAD-BEARING]
  "floor":   0.45,        // was DENSITY_FLOOR 0.35       [LOAD-BEARING]
  "ceil":    0.80,        // was DENSITY_CEIL 0.65        [garnish]
  "energy_span":  0.40,   // was DENSITY_ENERGY_SPAN 0.30 [garnish]
  "arousal_span": 0.20    // NEW — arousal → density edge [LOAD-BEARING direction]
}
```

**Motif quadrant table** (NEEDS Music Theory sign-off — they own the contour names; I only assert the affect-quadrant *keying*):
```jsonc
// composition.archetype (NEW SelectTable, replacing the hue-only pick_archetype):
//   keyed on {arousal, valence} quadrants; hue as within-quadrant tiebreak.
//   Exact archetype names are Music Theory's to fill — rows shown as intent, not final.
```

**`chord_engine.rs` ask (Music Theory's file, not mine to write):** lower `FILL_REST_ACTIVITY` from `0.10` to `0.05` (chord_engine.rs:1436) so rest-as-gesture is rare/deliberate rather than the calm-image default. This is part of "silence as a deliberate device."

**Sync flag:** `EDGE_ACTIVITY_RANGE_MAX` is mirrored in two places (composition.rs:26 and chord_engine.rs:1393) and must stay in sync; none of my proposals touch it, but anyone editing the density path near it should be aware.

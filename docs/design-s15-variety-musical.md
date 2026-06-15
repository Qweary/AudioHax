# Design S15 — Expanded Musical Vocabulary + Condition-Driven Selection

**Author role:** Music Theory Specialist (DESIGN ONLY — no source, test, or asset modified by this document).
**Date:** 2026-06-15
**Status:** Proposal for operator steer. Companion engineering envelope is being designed in parallel by the Rust Architect — §5 below flags every place this vocabulary needs a type-shape change so the two reconcile.
**Supersedes scope:** EXPANDS the small-vocabulary discipline of [`composition-architecture-musical.md`](./composition-architecture-musical.md) §1–§8 and the canonical [`assessment-composition-architecture.md`](./assessment-composition-architecture.md). The S14 assessment deliberately kept the catalog tiny (3 forms, 4 characters, 4 meters) to avoid sprawl. **The operator has consciously overridden that** — he wants more variety because variety is what makes one image's music feel unlike another's. This document delivers the expansion under one hard governor: **each dimension has a principled DEFAULT, and image features select departures via DETERMINISTIC CONDITIONS over perceptual knobs — never randomness.** Controlled variety, not chaos.

---

## 0. THE GOVERNING PRINCIPLE — "default standards, varying away on conditions"

Every dimension below is specified as:

1. **A DEFAULT** — the safe, pleasant, always-correct choice that fires when no condition triggers a departure. The default alone produces a complete, musical piece. This is the floor; nothing can break it.
2. **A CONDITION LADDER** — an ordered set of `if <perceptual condition> then <variant>` rules evaluated against the `ImageUnderstanding` knobs (assessment §C.2: `edge_activity`, `texture`, `complexity`, `dominant_hue`, `dominant_hue_mass`, `secondary_hue`, `palette_bimodality`, `colorfulness`, `value_key`, `avg_brightness`, `avg_saturation`, `mass_centroid`, `quadrant_contrast`, `aspect_ratio`, `vertical_emphasis`, `subject_size`, `subject_hue`, `subject_saturation`, `fg_bg_contrast`). **First matching rule wins** (priority order stated per dimension) — this makes selection a pure deterministic function of the knobs, fully testable, no `thread_rng` anywhere in SELECTION. The craft layer below (`pick_progression`, etc.) keeps its existing RNG — that is the documented boundary (assessment §C.5).

**Two cross-cutting discipline rules carried forward unchanged from the S14 ceiling:**
- **The honest ceiling stays:** "principled, fits the image, pleasant" — NOT "sounds hand-composed." Every entry below is musically defensible on its own; variety widens the *palette*, it does not raise the *ceiling*.
- **Each variant must be a real, audible, known-good musical object.** A bigger catalog of *defensible* choices is the goal. A bigger catalog of *gimmicks* is the failure mode, and the condition ladder is what prevents it (a variant only fires when the image genuinely warrants it).

A note on **threshold robustness.** Because selection is deterministic over continuous knobs, hard thresholds create cliff-edges (a tiny pixel change flips the whole form). Two mitigations, both stated as design intent for the implementer: (a) **hysteresis-free but coarse bins** — quantize each driving knob into 2–4 wide bands rather than reading raw floats at a knife-edge; (b) **priority short-circuit** — rarer/stronger variants are tested first against demanding compound conditions, so the common case falls through to the default. Neither needs new machinery; both are how the ladders below are written.

---

## 1. MACRO-FORM — expanded catalog

S14 shipped three forms (rounded binary default, ABA, theme-and-variations) and reserved two. The operator's examples — `ABA, ABBA, ABAC, ABBAC, AABA, theme-and-variations, rondo, and much much more` — call for a real catalog. A form is still **audible only through return + contrast + cadential articulation** (the §1.3 rule is inviolate and constrains every entry).

**A piece is a `Vec<Section>`; each Section has a `thematic_role` (Statement / Contrast / Return / Development / Coda), a key, a tempo, a character overlay, and a boundary cadence strength.** A "form" is therefore just a *recipe for the section sequence + which sections share/return material + the cadence plan*. That is the key insight that makes most of this catalog nearly free: **a new form is a new section-sequence template over the EXISTING per-section craft**, as long as it stays in 4/4 / home-key / no-new-variation-technique (TIER 1) vs. needs modulation or new variation machinery (TIER 2).

### 1.1 The form catalog

Notation: capital letters are sections; a prime (`A′`) is a varied return; subscript labels the thematic role. Cadence plan uses: **HC** = half cadence (comma, weak, open), **IAC** = imperfect authentic (medium), **PAC** = perfect authentic (period, strong), **DC** = deceptive (V–vi, a feint), **Plagal** = IV–I (a soft "amen" tag, for codas). The **structural close is always the single strongest PAC** in the piece (root position, soprano on tonic, + structural ritardando).

| Form | Section sequence | Where the theme returns | Cadence plan (per boundary) | Tier |
|---|---|---|---|---|
| **Rounded Binary `A B A′`** *(DEFAULT)* | A(Statement) B(Contrast) A′(Return) | A′ recaps A's theme, home key, abbreviated/light-varied | A: **HC** (question) · B: **HC or DC** (suspends) · A′: **PAC** (answer) | **TIER 1** |
| **Simple Binary `A B`** | A(Statement) B(Contrast/Continuation) | no literal return — B is a *response*, shares the theme's head (fragment), closes it | A: **HC** · B: **PAC** | **TIER 1** |
| **Ternary `A B A`** | A(Statement) B(Contrast) A(Return, complete) | A returns **complete** (not abbreviated); A is self-closing before B | A: **PAC** (self-contained) · B: **HC or DC** · A: **PAC** | **TIER 1** |
| **Arch `A B C B A`** | A B C(center) B A | A and B both return, mirror-symmetric around C | A:HC · B:IAC · C:**DC or HC** (apex, unresolved) · B:IAC · A:**PAC** | **TIER 1** (palindrome of existing sections) |
| **AABA (song form)** | A A B(bridge) A | A repeats then returns after the bridge; the bridge is the only contrast | A:IAC · A:IAC · B:**HC** (the bridge "leans") · A:**PAC** | **TIER 1** |
| **ABAC** | A B A C(new close) | A returns once mid-piece; C is a fresh, non-returning close | A:HC · B:IAC · A:HC · C:**PAC** | **TIER 1** |
| **ABBA** | A B B(intensified) A | A frames; B states then intensifies (B′ louder/denser) before A returns | A:HC · B:DC · B′:**HC** (the apex) · A:**PAC** | **TIER 1**\* |
| **ABBAC** | A B B A C(coda) | A returns after the doubled B; C is a coda tag | A:HC · B:DC · B′:HC · A:IAC · C:**PAC + Plagal tag** | **TIER 1**\* |
| **Rondo `A B A C A`** | A B(ep1) A C(ep2) A | A (the refrain) returns **twice** between contrasting episodes | A:IAC · B:HC · A:IAC · C:HC · A:**PAC** | **TIER 1**\* |
| **Extended Rondo `A B A C A B A`** / `ABACADA` | refrain alternates with N episodes | A returns between every episode | each episode:HC · each refrain:IAC · final A:**PAC** | **TIER 1**\* |
| **Theme & Variations `T V1 V2 (V3)`** | T(Statement) then 2–4 variations | theme's skeleton recurs in every variation, surface transformed | T:PAC · each V:IAC · final V:**PAC + ritard** | **TIER 2** (needs the variation techniques of §4) |
| **Strophic `A A A …`** | same section repeated, surface re-realized per pass | A every time, lightly re-voiced | each A:IAC · final A:**PAC** | **TIER 1** (degenerate T&V; no transform → free) |
| **Through-composed (continuous)** | A B C D … (no return) | NO return — gated behind **motivic continuity** (head-fragment recurs) so it develops, not rambles | internal:HC/IAC · final:**PAC** | **TIER 2** (needs fragmentation technique) |

**\*Footnote on the rondo/ABBA family Tier-1 claim:** these are Tier-1 *only* in the back-compat sense that their *mechanism* is pure section-sequencing + theme-placement + cadence-assignment over the existing 4/4/home-key craft — they need **no new realizer code**. BUT they expose a musical risk the simpler forms don't: with the theme in the **same home key every return** (no modulation, TIER 2), a 5–7-section rondo risks **monotony** (the same tune, same key, five times). So while the *engine mechanism* is Tier 1, I recommend the **multi-return forms (rondo, extended rondo, ABBA, ABBAC) be GATED behind at least character-overlay contrast between episodes** (different articulation/density per episode via §2) so the returns stay fresh without modulation. Without that contrast lever they should *not* ship in slice 1 even though they technically could. This is the one place where "the mechanism is free" and "it's musically ready" diverge, and I am flagging it honestly per the constraint.

### 1.2 The default + the form-selection condition ladder

**DEFAULT: Rounded Binary `A B A′`.** Fires whenever no stronger condition triggers. Smallest form delivering all three audibility cues; most forgiving. This is the §1 governor's floor.

**Condition ladder** (evaluated top to bottom over the knobs; first match wins; everything else → default):

1. **Theme & Variations** — `IF complexity ≥ 0.66 AND edge_activity ≥ 0.6` (a *busy, high-activity* image): much "to say about one idea"; variation surface absorbs the visual energy with structure. *(TIER 2 — until §4 lands, this condition routes to **Strophic** as the Tier-1 stand-in: same idea, re-realized, no transform.)*
2. **Rondo / Extended Rondo** — `IF subject_count ≥ 3` (from saliency, Stage 9) OR fallback `IF complexity ≥ 0.55 AND quadrant_contrast ≥ 0.5` (a "busy-corners, multiple-foci" image): each episode answers a region/focus. Episode count = `min(subject_count, 4)` or, on the fallback, 2 episodes (→ plain Rondo). *(Gated per §1.1 footnote: needs episode-contrast overlay.)*
3. **Arch `ABCBA`** — `IF quadrant_contrast ≤ 0.2 AND |mass_centroid.x − 0.5| ≤ 0.12 AND |mass_centroid.y − 0.5| ≤ 0.12` (a *strongly symmetric, centered* image): mirror form for mirror composition. The most literal balance→form mapping available without semantics.
4. **Ternary `ABA` (complete return)** — `IF fg_bg_contrast ≥ 0.5` (proxy: `center_saturation − border_saturation ≥ threshold`) — a *clear subject-vs-background* image: A "is" the subject, B "is" the ground, A returns to the subject **whole**. (Strong-contrast cousin of the default.)
5. **AABA (song form)** — `IF aspect_ratio is "wide/landscape" (≥ 1.6) AND palette_bimodality ≤ 0.3` (a *broad, palette-unified* image): the repeating A with a single bridge suits a panoramic, tonally-coherent scene.
6. **ABAC** — `IF vertical_emphasis is "top-heavy" (≥ 0.6)` (mass concentrated up top): one return, then a fresh descent to a new close — the "weight falls to a new resolution" reading.
7. **Through-composed** — `IF aspect_ratio is "tall/portrait" (≤ 0.62) AND vertical brightness gradient |top_half − bottom_half| ≥ 0.4` (a *strong directional gradient with no symmetry*): a genuine journey-without-return. *(TIER 2 — gated behind motivic continuity; until §4, this condition falls through to the default rather than ship a rambling form.)*
8. **ELSE → Rounded Binary `A B A′`** (default).

> **`ABBA` / `ABBAC`** are *not* on the primary ladder — they are a **modifier**, not a base form. When the default rounded-binary or a ternary is selected AND `edge_activity ≥ 0.7 AND quadrant_contrast ≥ 0.55` (an image with a *single dominant high-energy zone*), the B section is **doubled-and-intensified** (B B′) to give that energy a structural home, yielding ABBA (or ABBAC if a coda condition also fires). This keeps the catalog from combinatorially exploding while still honoring the operator's `ABBA`/`ABBAC` examples — they arise compositionally from a base form + an intensification modifier, which is more principled than treating them as peers.

> **Coda modifier:** any form gains a trailing `C(Coda)` (the `…C` in ABAC's sense of a *closing* section, realized as a brief tonic prolongation + Plagal tag after the structural PAC) when `value_key` is "low-key/dark" (≥ 0.6 toward dark) — a dark image earns a settling, resonant tail. This is how ABBAC arises from ABBA, ABAC's C-as-coda reading, etc.

---

## 2. CHARACTER / GENRE & METER — expanded catalog

S14 shipped four characters (Ballad default, Waltz, March, Lament) and reserved Scherzo. A character is still **a bundle of concrete parameters** (meter, tempo band, texture, rhythmic signature, articulation bias, dynamic posture), never a label. The operator wants the set fuller — and explicitly named the scoping tension: **meter beyond 4/4 is roadmap Stage 3, and Waltz/3-4 needs metric-accent machinery not yet built.** I mark each entry's mechanism honestly.

### 2.1 The character catalog

| Character | Meter | Tempo band (BPM) | Texture | Articulation bias (on §4 clamp) | Dynamic posture | Mechanism status |
|---|---|---|---|---|---|---|
| **Ballad** *(DEFAULT)* | 4/4 | 56–76 | full ensemble, melody-led, sustained fill | strongly legato → **1.10** | gentle messa-di-voce arches; soft | **TIER 1** — needs no per-beat gating; felt via accent + slow tempo |
| **Hymn / Chorale** | 4/4 | 60–80 | 4-part block, all roles move together, homorhythmic | legato, even → **1.00** | terraced, balanced, warm | **TIER 1** — block texture is the *existing* default behavior; no new mechanism |
| **Nocturne** | 4/4 | 50–68 | sparse melody over arpeggiated/rolled fill | legato, rubato-leaning → **1.08** | very soft, intimate, narrow | **TIER 1**\* (the *arpeggiated* fill uses the existing `edge_density>0.30` arpeggio behavior; no new code, but tuned via density bias) |
| **March** | 4/4 (or 2/4) | 96–120 | bass + melody prominent, fill steady | detached/marcato → **0.55** | firm, terraced; strong 1 & 3 | **TIER 1** for 4/4 accent; **role beat-masks (bass on 1&3) = TIER 2 (Stage 3)** |
| **Lament** | 4/4 *(or 6/8)* | 48–66 | melody + bass, fill thin/resting; suspensions | legato, weighted → **0.95**, exaggerated phrase-end ritard | dark, narrow-low; descending shapes; minor/Aeolian/Phrygian | **TIER 1** in 4/4; **6/8 felt = TIER 2 (Stage 3)** |
| **Waltz** | **3/4** | 96–144 | bass beat 1, fill 2–3 ("oom-pah-pah"), melody floats | portato bass → **0.75**, legato melody → **1.05** | lilting accent on beat 1 | **TIER 2 (Stage 3)** — needs `beats_per_measure=3` + per-beat role gating |
| **Scherzo** | 3/4 or 6/8 | 132–168 | light, melody-led, fill on off-beats | crisp staccato → **0.55** | playful, sudden contrasts (subito) | **TIER 2 (Stage 3+)** — needs 3/4 *and* off-beat gating; lowest priority (overlaps Waltz mechanically) |
| **Lilt / Pastorale** | **6/8** | 76–100 | flowing melody, gentle compound sway | legato → **1.05** | rolling, soft accents on 1 & 4 | **TIER 2 (Stage 3)** — needs 6/8 (2-felt) grouping |
| **Gigue / Romp** | **6/8** (or fast 2/4) | 116–160 | driving melody, active bass | detached → **0.60** | bright, bouncing | **TIER 2 (Stage 3)** — same 6/8 dependency |
| **Drone / Ambient** | 4/4 (free-feel) | 44–60 | pedal-point bass, slow harmonic rhythm, sparse melody | very legato → **1.10** | flat-soft, minimal contour | **TIER 1** — pedal point = sustained bass (existing); slow harmonic rhythm = density bias only |

**\*Nocturne / Hymn / Drone are the three NEW Tier-1 characters** — they ride entirely on the existing 4/4 accent + the existing texture/density/articulation knobs, so they ship in slice 1 alongside Ballad. They give immediate variety **without waiting on meter machinery.** This is the honest answer to the operator's "expand character now": you can have **four Tier-1 characters in slice 1** (Ballad, Hymn, Nocturne, Drone) — plus the 4/4 variants of March and Lament — and the genuinely metrical characters (Waltz, Scherzo, Lilt, Gigue) follow at Stage 3 when the meter mechanism lands.

### 2.2 The default + the character-selection condition ladder

**DEFAULT: Ballad** (4/4, slow, legato — safest "pleasant," needs the least machinery).

The canonical affect axes are **warmth** (`dominant_hue`: 0–60° & 300–360° warm, 60–300° cool), **brightness** (`avg_brightness` / `value_key`), and **energy** (`edge_activity` + `texture`). Selection ladder (first match wins; Tier-2 entries route to a Tier-1 fallback until Stage 3):

1. **Lament** — `IF warmth=cool AND value_key=dark (≥0.6) AND energy_low (edge_activity ≤ 0.3)` — somber, dark, still. *(Tier 1 in 4/4.)*
2. **March** — `IF energy_high (edge_activity ≥ 0.65) AND warmth=warm AND value_key=bright` — buoyant, driving, bright. *(Tier 1 for 4/4 accent; bass-mask deferred → until Stage 3 it's a "marcato 4/4," which is already audibly march-like via articulation 0.55 + accent.)*
3. **Drone / Ambient** — `IF complexity ≤ 0.2 AND edge_activity ≤ 0.2 AND quadrant_contrast ≤ 0.15` — a near-uniform, featureless field (sky, fog, color wash): nothing to subdivide → a sustained pedal-point ambient. *(Tier 1.)*
4. **Nocturne** — `IF value_key=dark (≥0.5) AND warmth ∈ cool/neutral AND energy_mid (0.3 < edge_activity < 0.6)` — dark but not still, intimate. *(Tier 1.)*
5. **Hymn / Chorale** — `IF colorfulness ≤ 0.3 AND palette_bimodality ≤ 0.25 AND energy_low-mid` — a tonally-unified, calm image (monochrome, muted palette): even, balanced, homorhythmic. *(Tier 1.)*
6. **Waltz** — `IF warmth=warm AND value_key=bright AND energy_mid AND aspect_ratio ≈ square-to-tall` — lilting, characterful. *(TIER 2 — until Stage 3, **falls back to Ballad** with a slightly brighter tempo-window center; the lilt waits for 3/4.)*
7. **Lilt / Gigue (6/8 family)** — `IF energy_mid-high AND warmth=warm AND a "flowing/rolling" texture signal (texture in a mid band, not spiky)` — compound-time sway. *(TIER 2 — falls back to Ballad (Lilt) or March (Gigue) in 4/4 until Stage 3.)*
8. **Scherzo** — `IF energy_very_high (edge_activity ≥ 0.8) AND value_key=bright AND complexity ≥ 0.6` — frantic, playful. *(TIER 2 — falls back to March until Stage 3+.)*
9. **ELSE → Ballad** (default).

### 2.3 The meter scoping tension — stated plainly

The S14 assessment is right: **meter beyond 4/4 is Stage 3, and it is the gate on half this catalog.** The mechanism that's missing is exactly two scalar inputs into the *existing* `realize_velocity`/`realize_rhythm` (assessment §A.4): `beats_per_measure` + `metric_position`, plus optional per-role beat-masks for oom-pah-pah/march gating. Until those land:

- **4/4 characters are fully shippable now** (Ballad, Hymn, Nocturne, Drone, March-as-marcato-4/4, Lament). That is **six Tier-1 characters** — already a real expansion past S14's "default to one."
- **3/4 and 6/8 characters (Waltz, Scherzo, Lilt, Gigue) are honestly TIER 2**, blocked on the Stage-3 meter mechanism. Their *selection conditions* can be written into the mappings now (so the ladder is complete and tested), but each routes to a 4/4 Tier-1 fallback under the default build, and only *activates its true meter* once `Meter::Three4`/`Meter::Six8` realization exists. **I will not pretend the lilt is free.**

---

## 3. MOOD / ENERGY / COLOR → MUSIC MAPPING — expanded

S13 drove tempo (brightness), harmonic color (saturation→7ths/9ths), mode (hue), density bias, and mixture. The operator wants *more dimensions driven → more distinct results*. The fuller mapping (each row is a default + a conditional departure over a knob):

| Image knob | Drives (musical target) | Default | Conditional departure |
|---|---|---|---|
| **`dominant_hue`** (not the muddy mean — the histogram argmax) | **mode** | Ionian (neutral) | warm (0–60/300–360) → Lydian/Ionian (bright); cool-blue (180–260) → Dorian/Aeolian; violet (260–300) → Phrygian; green (90–150) → Mixolydian. *The S13 diagnosis's recommended dominant-hue fix.* |
| **`avg_saturation`** | **harmonic complexity** | triads | `≥ 0.4` → +7ths; `≥ 0.7` → +9ths/extensions (S13, kept) |
| **`avg_brightness`** | **tempo position within the character window** | window center | bright → upper end of the character's BPM band; dark → lower end (S13, now *clamped by character*) |
| **`value_key`** (histogram-shape low/high key) | **register center + dynamic floor** | mid register, mid-soft | high-key → lift melody register +1 octave option, brighter dynamics; low-key → drop register, narrow dynamics down, favor minor modes |
| **`colorfulness`** (= `hue_spread`) | **modal mixture amount** | none/diatonic | `≥ 0.5` → enable bVI/borrowed-iv as B-section color; `≥ 0.75` → also chromatic passing in B (S13 mixture, now *concentrated structurally* not global) |
| **`edge_activity`** | **rhythmic density + articulation** | mid (clamped per §4) | high → more onsets/subdivision + shorter (toward 0.55); low → sustained + legato (toward 1.10) (S13 curve, clamped) |
| **`texture` (Laplacian var)** | **inner-voice activity + ornamentation rate** | static fill | high texture → fill moves more (passing tones); spiky texture → ornament the theme more; smooth → plain |
| **`complexity` (shape_complexity)** | **section/variation count + theme disjunctness** | 3 sections, stepwise theme | high → more sections/variations + wider theme leaps; low → fewer, conjunct |
| **`quadrant_contrast`** | **dynamic spread (range) + form symmetry** | moderate dynamic range | high → wide dynamic spread (soft B, loud climax) + asymmetric form; low → narrow dynamics + symmetric/arch form |
| **`palette_bimodality`** | **A-vs-B character/mode contrast strength** | mild contrast | high (two clear color populations) → strong parallel-mode or key contrast in B (the "two things in the image"); low → gentle contrast |
| **`fg_bg_contrast`** / center-vs-border | **B-section modulation depth** | stay near home | high → real key change in B (subject ≠ ground); low → B stays home (uniform image doesn't earn modulation) |
| **brightness gradient (top↔bottom)** | **theme contour direction** | arch (up-then-down) | bright-top → descending "from the light"; bright-bottom → ascending; flat → arch default |
| **`mass_centroid`** | **where weight/climax sits + register lean** | climax end-of-B | centroid-high → earlier/brighter climax, higher melody lean; centroid-low → later, grounded, bass-heavier |
| **`aspect_ratio`** | **phrase length / breath** | 4-step phrases | wide/landscape → longer 8-step phrases (broad, unhurried); tall/portrait → terser phrases, more directional |
| **`vertical_emphasis`** | **register trajectory across the piece** | level | top-heavy → overall ascending register plan; bottom-heavy → descending/grounded |
| **`subject_size`** (Stage 9) | **theme prominence (melody dynamic relative to accompaniment)** | balanced | large subject → melody forward, accompaniment recedes; small subject → melody intimate, texture richer |

**The principle that keeps this from being noise:** each mapping is **perceptually meaningful** (brightness=arousal, warmth=mode-color, contrast=dynamic-range, gradient-direction=melodic-direction, centroid=climax-weight) and each has a **stated default** so an absent/ambiguous signal degrades to the safe choice, never to randomness. The expansion is in *how many knobs are read* (S13 read ~4; this reads ~16), which is exactly what makes two different images diverge in more dimensions and therefore *feel* distinct — the operator's stated goal.

---

## 4. THEME BEHAVIOR — expanded variation vocabulary + the conditional theme-presence rules + motif encoding

### 4.1 The variation-technique vocabulary

S14 listed augmentation/diminution, transposition, reharmonization, ornamentation, fragmentation. That set is already good and **I keep it as the curated vocabulary** — the expansion the operator wants is in *when each fires and how they combine*, plus two additions that earn their place:

| Technique | What it does | Audibility / cost | Tier |
|---|---|---|---|
| **Identity** | literal restate | recognition anchor (cheapest) | **TIER 1** — already needed for the default A′ |
| **Transposition** | state from a new degree / in the new key | tied to the key plan | **TIER 2** (needs §A.5 modulation) for *key* transposition; **TIER 1** for diatonic degree-shift within home |
| **Augmentation / Diminution** | double / halve note values | very audible, cheap (scale the rhythm) | **TIER 2** (needs the motif-rhythm-scaling path) |
| **Reharmonization** | same contour, new chords beneath | powerful, idiomatic | **TIER 2** (needs per-section alt progression under a fixed motif) |
| **Ornamentation** | insert passing/neighbor tones between motif pitches | adds life, reuses voice-leading | **TIER 2** (needs motif-note interpolation) |
| **Fragmentation** | head-only (first 2 degrees), sequenced | the "developing" gesture | **TIER 2** (needs partial-motif replay) |
| **Inversion** *(new)* | mirror the contour (up↔down) | strong, recognizable transform | **TIER 2** |
| **Retrograde** *(new, reserve)* | reverse the contour | recognizable but harder to hear; reserve | **TIER 2** (lowest priority) |

**TIER-1 reality for slice 1:** the only theme behaviors that ship in the first slice are **Identity (A′ recap)** and **diatonic degree-transposition within the home key** + **presence/absence** (theme stated in A, absent or *head-fragmented* in B, recapped in A′). **Fragmentation is the one borderline case:** "head-only" replay needs the realizer to play a *subset* of the motif. I recommend the **simplest fragmentation — "play only the first half of the motif in B, then silence the melody role"** — which is achievable in slice 1 as a presence-gate (it's a truncation, not a new transform) and gives B real motivic continuity cheaply. Everything richer (augmentation, reharmonization, ornamentation, inversion, retrograde) is **TIER 2** and lands with Stage 7 (variation techniques + theme-and-variations form).

### 4.2 The conditional theme-presence rule (absent vs fragmented vs contrasting-second-theme)

This is the operator's open decision #6, resolved here as a *deterministic ladder* rather than a single global choice — which is exactly the "vary on conditions" he asked for. In the Contrast (B) section:

1. **Contrasting SECOND theme** — `IF palette_bimodality ≥ 0.6 AND fg_bg_contrast ≥ 0.5` (the image genuinely has *two distinct things*): B gets its own short generated motif (a *different* curated contour, seeded by `secondary_hue`). Maximum contrast, justified by the image literally having a second subject. *(TIER 2 — needs a second `ThemeSeed`; until then routes to "absent.")*
2. **Fragmented (head-only)** — `IF complexity ≥ 0.4` (a busy image wants continuity through the contrast): B sequences the motif's head. *(TIER 1 as truncation per §4.1.)*
3. **Fully absent (free-select melody)** — `ELSE` (calm/simple image): B is maximal contrast by *withholding* the theme entirely; the melody role free-selects chord tones as it does today. *(TIER 1 — this is literally current behavior.)*

The **default** (the safe floor) is **option 3, fully absent** — it's current behavior and always correct. The ladder *adds* unity (fragmentation) or contrast (second theme) only when the image earns it.

### 4.3 Motif encoding — the recommendation

This is open decision #9 and a real boundary: the encoding is what the engine's theme-replay reads. The three candidates:

- **(a) Absolute scale-degrees** `[1,3,2,1]` — simplest; the realizer maps degree→chord-tone-or-NCT in the current key. **Problem:** doesn't transpose cleanly across modes (degree 3 in Ionian vs Aeolian is a different interval), and "contour" is implicit, so variation-by-inversion is awkward.
- **(b) Intervals** `[+2,−1,−1]` (semitone or scale-step deltas) — transposes trivially (just pick a start pitch), inversion = negate, retrograde = reverse. **Problem:** a pure interval string is *unconstrained* — random intervals make ugly themes; you'd need a generator that only emits good ones, reintroducing the sprawl risk.
- **(c) Contour-from-a-curated-set** — the motif is **one archetype chosen from a small fixed catalog of known-good melodic shapes**, then *parameterized* (start degree, range, rhythm) by the image. This is the §A.6 lean and **my recommendation.**

**Recommendation: (c) contour-from-a-curated-set, encoded as a `MotifArchetype` enum + a small parameter struct, which the engine then *expands* into a concrete scale-degree sequence at plan time.** Rationale:

- It is the only encoding that **bounds the vocabulary to known-good shapes** while still letting the image select and parameterize — i.e. it is the §1-governor (default + conditional) applied at the motif level. Intervals (b) are too free (sprawl); absolute degrees (a) don't transpose. Contour-set gets (b)'s transpose/invert tractability *and* (a)'s safety.
- It gives the engine a **stable, tiny boundary to read**: the `ThemeSeed` carries `{ archetype: MotifArchetype, start_degree: i8, range: u8, rhythm: Vec<RhythmFigure> }`; the planner expands archetype+params → the concrete `Vec<MotifNote>` (scale-degree + duration) the realizer already knows how to play. **Variation operations are defined on the archetype** (inversion = the mirror archetype; augmentation = scale the rhythm; fragmentation = first-half of the expanded sequence), which keeps the §4.1 techniques clean.

**The curated contour set** (expanded from S14's 4 to a still-small 8, each a textbook melodic archetype):

| Archetype | Contour (do=1) | Character | Seed condition |
|---|---|---|---|
| **Arch** *(DEFAULT)* | 1 3 5 3 1 (up then down) | balanced, singable | the fallback when no gradient signal |
| **Inverted Arch (valley)** | 5 3 1 3 5 | settling then rising | bright-edges/dark-center images |
| **Descent** | 5 4 3 2 1 | "from the light," resolving | bright-top gradient |
| **Ascent** | 1 2 3 4 5 | rising, opening | bright-bottom gradient |
| **Neighbor-turn** | 1 2 1 7 1 | gentle, ornamental, calm | low edge_activity, low complexity |
| **Leap-and-step (gap-fill)** | 1 5 4 3 2 | a leap then a stepwise fill | mid-high edge_activity |
| **Pendulum** | 1 5 1 5 1 | oscillating, insistent | high `quadrant_contrast` (two-zone image) |
| **Rising sequence** | 1 2 3 / 2 3 4 (motivic sequence) | developmental, directional | high complexity + vertical gradient |

**How hue + edge-activity seed the choice (deterministic):**
- **`edge_activity` / `complexity` sets the RANGE band** (the §A.6 rule kept): low → conjunct (range ≤ 3 degrees, favors Neighbor-turn/Arch); high → wider leaps (range to 5+, favors Leap-and-step/Pendulum/Rising-sequence).
- **The brightness GRADIENT (top↔bottom) picks direction-bearing archetypes first** (Descent for bright-top, Ascent for bright-bottom) — this is the most literal, satisfying image→melody link.
- **`dominant_hue` is the tiebreaker among the non-directional archetypes** when no strong gradient exists: it indexes the curated set (warm hues → Arch/Ascent open shapes; cool hues → Inverted-arch/Neighbor-turn inward shapes), so different-hued images of similar energy still get different (but always good) contours.
- **`quadrant_contrast` ≥ 0.6 forces Pendulum** (an image with two strong zones gets an oscillating theme — the most direct two-zone→melody mapping).

This is fully deterministic, bounded to 8 known-good shapes, and gives the operator the variety he wants (8 contours × range-parameterization × rhythm-from-character × start-degree = a wide *audible* space) without a single random or potentially-ugly theme. **TIER-1 subset for slice 1:** Arch (default) + Descent + Ascent + Neighbor-turn (the original §A.6 four) ship first; Inverted-arch, Leap-and-step, Pendulum, Rising-sequence land with the variation-technique stage (they pair naturally with inversion/sequence operations).

---

## 5. OTHER DIMENSIONS where principled conditional variety adds distinctiveness

The operator explicitly expects there are more. Five I see, each default + conditional, each musically defensible:

1. **Harmonic rhythm (rate of chord change).** *Default:* one chord per measure. *Conditional:* `edge_activity`/`complexity` high → accelerate harmonic rhythm toward cadences (chord per 2 beats, then per beat into the close); low → slow, static (one chord per phrase, near-pedal). This is the §A.7 "harmonic trajectory" knob made image-conditional — and it's the difference between "calm" and "driving" beyond tempo alone. **TIER 1** (it's a per-section progression-length/repeat choice over existing chords; no new mechanism — the planner just repeats or subdivides progression entries).

2. **Texture density / role activation per section.** *Default:* all three roles (bass/fill/melody) active throughout. *Conditional:* `value_key`/`subject_size` → thin the texture in soft sections (drop fill in a Nocturne B), thicken at the climax (all roles + octave doublings). Makes sections *texturally* distinct, not just harmonically — a major contributor to "this section is OTHER." **TIER 1** (role-on/off per section is a beat-mask special case, and "all-on" is the default; muting a role is free). *Note: per-beat masks (waltz) are Tier 2; whole-section role on/off is Tier 1.*

3. **Cadence-type variety as a punctuation palette.** *Default:* HC inside, PAC at the structural close (the §6.3 hierarchy). *Conditional:* introduce **Deceptive cadences (V–vi)** as a "feint" at a section boundary when `palette_bimodality` is high (the image keeps surprising) and a **Plagal tag** (IV–I "amen") coda when `value_key` is dark/settling. Differentiated cadence *types* (not just strengths) give the form more varied punctuation. **TIER 1** for HC/IAC/PAC (existing); **deceptive + plagal = TIER 1.5** — the chords exist (`vi`, `IV`), but the planner must place them at the right boundary; small, low-risk.

4. **Register plan / tessitura arc across the piece.** *Default:* melody stays in its register band. *Conditional:* `vertical_emphasis`/`mass_centroid` → an overall ascending or descending tessitura plan across sections (top-heavy image → the melody climbs section-to-section toward the climax; bottom-heavy → it descends/grounds). A slow registral arc is a strong, subtle shape cue. **TIER 1** as a per-section register offset on the melody role (the realizer already re-seats into bands; a per-section band-center shift is a scalar). *Caution: touches register goldens — apply as plan scalar defaulting to 0 so the default plan stays byte-green.*

5. **Phrase-length / breath variety.** *Default:* 4-step phrases. *Conditional:* `aspect_ratio` wide → 8-step (broad, unhurried) phrases; tall → mix 4+2 (terser, more urgent). Phrase breath is a real character cue that's currently fixed. **TIER 1** (`PHRASE_LENGTHS` already supports 4 and 8; selecting per-image is free).

6. *(Reserve, Stage 9+)* **Sectional-tempo relationship driven by `quadrant_contrast`** — high-contrast image earns a faster or slower B (the §A.5 tempo-ratio idea, image-conditional). **TIER 2** (needs the per-section tempo plan depth of Stage 5).

---

## 6. THE EXPLICIT TIER-1-VS-TIER-2 SPLIT (with rationale)

The hard constraint: the **first build slice** must still land the spine — a non-looping sectioned plan + a returning theme + the S13 articulation clamp — behind a back-compat default that keeps `engine_equivalence` byte-green. Below, the expanded vocabulary is split honestly. **The test of Tier 1 is:** *does the mechanism reduce to section-sequencing + theme placement/truncation + cadence-assignment + plan-supplied scalars over the EXISTING 4/4, home-key, single-tempo craft?* If yes → Tier 1 (nearly free). If it needs meter beyond 4/4, real modulation, climax-pushing, or a new variation transform → Tier 2/later.

### TIER 1 — lands in (or is ready for) the first slice

- **Forms (section-sequence only, 4/4/home-key):** Rounded Binary (default), Simple Binary, Ternary ABA, Arch ABCBA, AABA, ABAC, Strophic. **Plus** the multi-return family (Rondo, Extended Rondo, ABBA, ABBAC) *mechanically* — **but GATED** behind §2 character-overlay episode-contrast so the same-key returns don't go monotone (the §1.1 footnote). For the **first slice specifically**, I recommend shipping the **default (Rounded Binary) + the simple non-modulating cousins it's easy to verify (Ternary ABA, AABA)** and holding the rondo family until the character overlays (themselves Tier 1, but a *second* slice) are in, so slice 1 stays small and the equivalence net is easy to reason about.
- **Characters (4/4, existing knobs):** Ballad (default), Hymn, Nocturne, Drone, March-as-marcato-4/4, Lament-in-4/4. Six characters with **no new mechanism** — but again, slice 1 itself ships **Ballad only** (the back-compat default), with the other five Tier-1 characters landing at Stage 4 as `CharacterOverlay` biases. They are *Tier 1* (no new machinery) but *not slice-1* (slice 1 is the spine; character is the next enrichment).
- **Theme:** generation from the curated contour set (the 4 original archetypes for slice 1), Identity recap in A′, presence/absence/head-fragment in B, diatonic degree-transposition within home. The **returning theme is the defining slice-1 deliverable.**
- **Mapping (§3):** the knobs that need no new mechanism — dominant_hue→mode, saturation→7ths/9ths, brightness→tempo-position, edge→density/articulation, gradient→contour-direction, complexity→theme-disjunctness. (Slice 1 uses the subset needed to seed the theme: hue + edge_activity, exactly as the spine requires no new extraction.)
- **Other dimensions:** harmonic-rhythm rate (#1), section role activation (#2), HC/IAC/PAC cadence assignment + deceptive/plagal placement (#3), register plan as plan-scalar (#4, defaulting to 0), phrase-length selection (#5).
- **The S13 articulation clamp (§7 below)** — folds into slice 1 as the ride-along.

**Rationale for the Tier-1 boundary:** all of the above are *data choices the planner makes and stamps onto `StepPlan`s*, consumed by the **unchanged** realizer. The single structural change (kill the `plan[step_idx % len]` loop → play the concatenated sections once) is what every one of these rides on. Crucially, **all biases are plan-supplied scalars defaulting to identity (×1.0 / +0 / role-all-on / register-offset-0)**, so the *default plan reproduces today's goldens exactly* and the equivalence net stays byte-green (assessment §C.6). That's why the catalog can be large while slice 1 stays safe: a big Tier-1 *catalog* is fine because slice 1 only *activates* the default end of it.

### TIER 2 / later stages — needs new mechanism

- **Meter beyond 4/4** (Waltz 3/4, Scherzo, Lilt/Gigue 6/8): needs `beats_per_measure` + `metric_position` re-pointing + per-beat role masks. **Stage 3.**
- **Real modulation / sectional key change** (B in dominant/relative; key-transposition of the theme): needs the applied-dominant pivot path + per-section root offset. **Stage 5.** (Slice 1 is all home-key.)
- **Per-section tempo relationships + structural ritardando depth** (#6). **Stage 5.**
- **Climax pushing** (register/dynamics/density/harmony to high ends at a marked step): **Stage 6.**
- **Variation techniques** (augmentation, diminution, reharmonization, ornamentation, inversion, retrograde) **and Theme-and-Variations / Through-composed forms** that depend on them: **Stage 7.** (Slice 1 has Identity + truncation-fragmentation only.)
- **Contrasting second theme in B** (needs a second `ThemeSeed`): **Stage 7.**
- **Form selection from balance/symmetry heuristics + region-saliency-driven section/episode count** (the rondo family's *true* driver, the Arch's symmetry condition, the second-theme's bimodality condition all read NEW heuristics): the **conditions** can be authored now, but several read **Stage-8/Stage-9 heuristics** (quadrant_contrast, mass_centroid, fg_bg_contrast, subject_count). Until those extractors exist, those ladder rungs are dead and the selection falls through to the default — **honest degradation, not breakage.**

**The honest summary of the split:** the operator gets a *large, principled catalog authored now*, but it **activates in stages** as the mechanism and the heuristics arrive. Slice 1 lands the **spine + default + the returning theme + the clamp**; the expanded catalog is the *ladder the spine grows into* across Stages 3–9. Nothing collapses into slice 1, and nothing pretends a 6/8 lilt or a real modulation is free.

---

## 7. THE S13 ARTICULATION-CLAMP SPEC (Music Theory Specialist owns this)

Carried forward from the assessment's Slice 0 (ride-along) and §9 fold-in, specified precisely:

- **Mechanism today:** S13's curve maps `edge_activity` 0→1 to a note-length fraction `LEGATO_FRAC_HI = 1.05` → `STACCATO_FRAC = 0.40`, clamped `0.30..=1.20` in `realize_rhythm` (~`chord_engine.rs:1144`–`:1182`). The extremes are unpleasant: below ~0.5 a note reads as a click rather than a tone; above ~1.15 successive notes mud together and lose articulation.
- **The clamp:** constrain the **non-cadence** hold fraction to a perceptually pleasant window — **`0.55 ≤ base_frac ≤ 1.10`**. This narrows the *output* range without flattening its *responsiveness*: `base_frac` still varies continuously with `edge_activity` across the window (the curve is re-scaled into the narrower range, not truncated — so a calm image is still more legato than a busy one, just within musical bounds).
- **The cadence ring stays byte-stable:** the cadence hold (1.20) is untouched — the clamp acts only on **non-cadence** steps. The cadence branch's golden (the 240 ms ring) does not move.
- **Per-character `articulation_bias` rides on TOP of the clamped window** (§2.1 column): the character's bias is applied to `base_frac` *before* (or as the center of) the clamp — Ballad biases toward 1.10, March toward 0.55, etc. — so character and clamp compose cleanly (the bias picks where in the 0.55–1.10 window this character centers; edge_activity then varies around that center, re-clamped). **In slice 1 the bias defaults to identity (Ballad), so the clamp is the only change.**
- **Golden discipline (the hard caution):** the clamp **deliberately moves the non-cadence articulation goldens.** Per S13 §7 precedent: **the moved non-cadence goldens must be hand-re-derived from the new documented formula in the same implementing commit, with a comment showing the derivation; never loosen an assert to silence it.** The cadence branch stays byte-stable. This is a Music-Theory-Specialist-owned, deliberate golden re-derivation — flagged here so the implementing commit budgets for it.

---

## 8. RECONCILIATION NOTES — where this vocabulary meets the engine type shells

For the Rust Architect designing the engineering envelope in parallel. Each note flags whether the existing type shell (assessment §C.3) holds the expanded vocabulary or needs a shape change.

1. **`Form` enum — NEEDS EXPANSION.** Assessment §C.3 has `Form { RoundedBinary, TernaryABA, ThemeAndVariations, ThroughComposed, Rondo }`. This design adds **SimpleBinary, Arch, AABA, ABAC, Strophic, ExtendedRondo**, and treats **ABBA/ABBAC as modifiers** not variants. **Recommendation:** either (a) widen the enum with the new variants, or — cleaner — (b) **replace the closed `Form` enum with a `FormTemplate { sections: Vec<SectionRole>, returns: Vec<(usize,usize)>, cadence_plan: Vec<CadenceStrength> }`** so a form is *data* (a section-sequence recipe), not a hard-coded variant. Option (b) makes the operator's "much much more" extensible without an enum edit per form, and the modifiers (ABBA-intensification, coda-tag) become *transforms on a FormTemplate*. **Flag for reconciliation: I lean (b); it changes the `plan.form` field from an enum to a small struct.** This is the single biggest type-shape decision this design raises.

2. **`Character` enum — NEEDS EXPANSION.** §C.3 has four variants; this adds Hymn, Nocturne, Drone (Tier 1) + Scherzo, Lilt, Gigue (Tier 2). The enum widening is mechanical. **But** the `CharacterOverlay` (the *bundle of biases*) is the load-bearing type, and it must carry: `articulation_bias: f32`, `dynamic_bias: f32`, `rhythm_band_shift: f32`, `harmonic_rhythm_rate: f32` (NEW, §5.1), `role_active: Map<OrchestralRole,bool>` (NEW, §5.2 whole-section activation — distinct from the Tier-2 per-beat `role_beat_masks`), `register_offset: i8` (NEW, §5.4), `phrase_len_pref: u8` (NEW, §5.5). **Flag: `CharacterOverlay` gains four fields beyond §A.7's list; all default to identity.**

3. **`Meter` enum — HOLDS.** §C.3 `Meter { Four4, Three4, Six8, Two4 }` covers everything this design needs. No change. (The work is in *realization* of 3/4 & 6/8 at Stage 3, not the type.)

4. **`Section` struct — MOSTLY HOLDS, two adds.** §C.3 `Section` already has `thematic_role`, `key_offset_semitones`, `ms_per_step`, `mode`, `progression`, `theme: Option<usize>`, `variation`, `boundary_cadence`, `density`. This design needs `boundary_cadence` to carry the **new cadence types** — confirm `CadenceStrength` includes **Deceptive and Plagal** (§5.3), not only Half/IAC/PAC. And the `progression` field must support a **per-section harmonic-rhythm count** (how many beats each chord holds) for §5.1 — either a parallel `Vec<u8>` of chord-durations or encoding repeats in the `progression` Vec. **Flag: `CadenceStrength` enum needs Deceptive + Plagal; harmonic-rhythm needs a per-chord duration vector on `Section`.**

5. **`ThemeSeed` / `MotifNote` — NEEDS A SHAPE CHANGE for contour-encoding.** §C.3 has `ThemeSeed { id, motif: Vec<MotifNote> }` with `MotifNote { degree, dur_steps }`. My §4.3 recommendation (contour-from-curated-set) means the **authored** seed is an archetype + params, expanded to `Vec<MotifNote>` at plan time. **Recommendation:** keep `ThemeSeed.motif: Vec<MotifNote>` as the **expanded** form the realizer reads (so the realizer boundary is unchanged — it still reads degree+duration), but add a `ThemeSeed.archetype: MotifArchetype` and `params` that the **planner** uses to *generate* that `Vec<MotifNote>` and to *drive variation operations* (inversion/fragmentation operate on the archetype, then re-expand). **Flag: `ThemeSeed` gains `archetype: MotifArchetype` (new enum, 8 variants) + a small param struct; `MotifNote` is unchanged — the realizer reads the same degree+dur it already does.** This keeps the engine's theme-replay boundary stable while making variation tractable.

6. **`ThemeVariation` enum — NEEDS TWO ADDS.** §C.3 has `Identity, Transposed, Reharmonized, Augmented, Diminished, Ornamented, Fragmented`. Add **Inverted, Retrograde** (§4.1). Mechanical.

7. **`PlanMappings` (the `mappings.json` range tables) — GROWS, no shape blocker.** All the condition ladders (§1.2, §2.2, §4.2, §4.3 seed conditions, §3 mapping) live here as threshold tables, per the assessment's "tunable without recompile" intent. **Flag: this is where the determinism lives — the ladders are data, not code branches, so the operator can re-tune thresholds without a recompile and the selection stays a pure function of (understanding, mappings).** The struct just needs sub-tables per dimension.

8. **`ImageUnderstanding` — HOLDS for Tier 1; some ladder rungs read fields populated only at later stages.** The §3 mapping and the form/character ladders read fields already in §C.2 (`edge_activity`, `complexity`, `dominant_hue`, `value_key`, `colorfulness`, `quadrant_contrast`, `mass_centroid`, `aspect_ratio`, `vertical_emphasis`, `palette_bimodality`, `fg_bg_contrast`). **No new field is required by the vocabulary itself** — the gap is *extraction maturity* (some fields are default-whole-image until the Stage 8/9 heuristics fill them), not type shape. **Flag: no `ImageUnderstanding` shape change; conditions on not-yet-extracted fields degrade to the default rung — confirm the planner treats default/sentinel field values as "condition not met."**

**Net for the Architect:** the only *structural* decisions this vocabulary forces are **(1) `Form`: enum-widen vs. `FormTemplate`-as-data (I lean data)**, **(2) `ThemeSeed` gains an `archetype` + params (realizer boundary unchanged)**, **(3) `CharacterOverlay` gains 4 identity-defaulting fields**, **(4) `CadenceStrength` gains Deceptive + Plagal**, **(5) `Section` gains a per-chord harmonic-rhythm duration vector**. Everything else is data in `mappings.json`. All adds are identity-defaulting so the back-compat default plan stays byte-green.

---

## 9. SUMMARY OF OPEN DECISIONS FOR THE OPERATOR (new/changed by this pass)

The S14 open decisions stand; this pass adds/sharpens:

- **A. Form representation:** closed `Form` enum (simple) vs. **`FormTemplate`-as-data** (extensible, my lean — lets you keep adding forms without code edits). *This is the decision that most enables "much much more."*
- **B. How many Tier-1 characters in slice 1 specifically:** I recommend **slice 1 = Ballad only** (the spine + default), with the other five Tier-1 characters (Hymn, Nocturne, Drone, March-4/4, Lament-4/4) as the *immediately-next* slice. Confirm you want the spine isolated first, or want 2–3 characters bundled into slice 1.
- **C. Multi-return-form gating:** confirm the rondo/ABBA family waits for character-overlay episode-contrast (my recommendation — same-key returns go monotone without it) rather than shipping on raw section-sequencing.
- **D. Motif encoding:** confirm **contour-from-curated-set (8 archetypes)** over absolute-degrees or raw-intervals. *(My strong recommendation; it's the only encoding that's both safe and variation-tractable.)*
- **E. Second-theme-in-B:** confirm the deterministic ladder (second theme when `palette_bimodality` high; else fragment; else absent) over a single global choice — this resolves S14 open-decision #6 as a *condition*, which is the operator's stated control style.
- **F. Threshold tuning surface:** confirm the ladders live as tables in `mappings.json` (tunable, no recompile) — and that you want to be the one tuning the thresholds by ear after slice 1 is hearable.

---

*End of S15 variety design. No source, test, or asset modified. The expansion honors the operator's override — substantially more variety in form, character/meter, mood/color mapping, theme behavior, and five further dimensions — under one governor: a principled DEFAULT per dimension + DETERMINISTIC conditional departures over the `ImageUnderstanding` knobs, never randomness. The honest ceiling ("principled, fits the image, pleasant") and the slice-1 spine (non-looping sectioned plan + returning theme + S13 articulation clamp, behind a byte-green back-compat default) are both preserved. The large catalog activates in stages; nothing collapses into slice 1.*

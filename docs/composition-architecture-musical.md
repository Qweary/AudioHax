# Composition Architecture — Musical (Section A)

**Author role:** Music Theory Specialist (DESIGN ONLY — no source, test, or asset modified by this document).
**Date:** 2026-06-14
**Companion:** Section (B), the engine/dataflow re-architecture, is authored in parallel by the Rust Architect. This document owns the **musical** contract: *what* a piece must contain to have form, character, themes, and a key scheme, and *how* those decisions should drive the existing per-step realizer. The Architect owns *how* a plan binds to the engine and the feasibility of extracting each image property.

**Standard:** the project owner has a music-performance degree (trombone) and a working ear. Everything below is written to survive a listener who hears every lifeless phrase and every structureless ramble. The discipline throughout is **small and coherent, not comprehensive** — a few forms × a few characters × a few devices, each tied to a robust *heuristic* image property. We are not building a tuning sandbox; we are building a piece.

---

## 0. THE VERDICT, RESTATED MUSICALLY, AND THE THESIS OF THIS DESIGN

S2–S13 built **bottom-up, note-level craft** and it is genuinely good: correct diatonic modes (`IONIAN`…`AEOLIAN` in `chord_engine.rs`), conservative voice leading with common-tone retention and parallel-perfect rejection (`voice_lead_sequence`/`voice_lead_one`), a real phrase model with cadences at boundaries (`plan_phrases`/`StepPlan`/`PhrasePosition`), an expressive performance layer (`realize_step` → `realize_velocity` + `realize_rhythm` + `role_pitch`, with orchestral roles Bass/HarmonicFill/Melody), and S13's per-image diversity (tempo from brightness, 7th/9th harmony from saturation, a continuous articulation curve, density bias, modal mixture).

And the operator's verdict still stands: **ethereal, structureless, unrelated to the image; only works for abstract art.** That is not a contradiction. It is the precise, predictable sound of *excellent local craft with no global plan*. The engine today is a **scan-sonifier**: it reads whole-image averages once, derives one mode + one randomly-chosen progression + one phrase plan, then emits a uniform left-to-right stream of realized chords. The `decide_instrument_action` kernel indexes `plan[step_idx % plan.len()]` — the plan **loops**. There is no beginning, middle, or end; no "this is the theme, here it comes back"; no "we left home and now we return"; no meter to push against. A texture, not a piece.

**The thesis:** the missing layer is a **CompositionPlan computed once, before any note is realized**, that imposes top-down architecture *over* the existing craft. The plan decides FORM (the sectional shape), CHARACTER (the genre preset), METER (which does not exist today at all), a KEY/TEMPO SCHEME across sections, and a THEMATIC PLAN (a motif that returns and is varied). Crucially, **the plan reuses every existing function** — it does not reinvent voice leading, realization, or phrasing. It *sequences and parameterizes* them. The single biggest lever against "structureless and ethereal" is **§5: a returning, varied theme** — the ear forgives almost anything if it recognizes a tune coming home.

A note on what to expect: this will be **audibly different from S13, in ways the operator will immediately hear** — repetition with return, sectional contrast, a downbeat you can feel. It will *not* sound like a hand-composed sonata. The honest target is the operator's own words: **follows music-theory principles, fits the image, and is pleasant.** That is achievable with a small vocabulary. "Great composition" is not a target this layer should chase, and we will say so explicitly in §10.

---

## 1. MACRO-FORM VOCABULARY

A form is **audible** only through three things: **return** (material the ear recognizes coming back), **contrast** (a section that is clearly *other*), and **cadential articulation** (a real close between sections, not a seam). The existing `plan_phrases` already gives us cadences at phrase boundaries; form is the layer that groups phrases into **sections** and decides which sections return.

**Vocabulary discipline:** ship **three** forms, default to one, hold a fourth in reserve. A piece is a list of **Sections**; each Section is a run of phrases (each phrase 4 or 8 steps, per the existing `PHRASE_LENGTHS`) that shares a key, tempo, character, and a **thematic role** (Statement / Contrast / Return / Development / Coda).

### 1.1 The three shipped forms

**(1) Rounded Binary — `A B A'` — THE DEFAULT.**
- **Shape:** A (statement, home key, closes with a half cadence or imperfect authentic cadence — a *question*), B (contrast: different key center and/or character, a *digression*), A′ (return of A's theme, now closing with a **perfect authentic cadence** — the *answer home*). A′ is shortened or lightly varied, never a literal copy.
- **Why default:** it is the smallest form that delivers all three audibility cues — statement, contrast, *and* a recognizable return — in ~3 sections. It is the most forgiving of imperfect material because the return does the heavy lifting. It is the textbook minimum viable "piece."
- **Size:** 3 sections; ~2 phrases each (so ~16–24 steps total). A′ may be 1 phrase.
- **Audible signature:** the listener hears the opening tune *come back* after a middle that went somewhere else, and the very end lands harder than anywhere prior (the PAC).

**(2) ABA / Ternary (da capo) — for strong contrast.**
- **Shape:** A (a complete, self-closing section, PAC at its end), B (a fully contrasting section — often the parallel mode, a new character, sometimes a new tempo), A (a *complete* restatement of A). Distinct from rounded binary in that A is **self-contained** (it closes firmly before B) and the return is **complete**, not abbreviated.
- **When right:** images with a clear **dominant region or subject vs. a distinct background** (foreground/background contrast) — the A material "is" the subject, B "is" the background, A returns to the subject. (This leans on region reading; see §8 — heuristic center-vs-border contrast is enough, true subject detection is "semantic, later.")
- **Size:** 3 sections, A larger (2 phrases), B contrasting (1–2 phrases), A returns.

**(3) Theme and Variations — `T V1 V2 (V3)` — for busy/complex images.**
- **Shape:** a Theme section, then 2–3 Variations that keep the theme's **harmonic skeleton and phrase length** but transform its surface (rhythm, register, articulation, ornamentation). This is the form where S13's per-step diversity becomes a *virtue* rather than a wash: each variation deliberately re-weights the realization knobs.
- **When right:** **high-complexity, high-edge-density images** — the visual busyness maps to "more to say about one idea," and the variation surface absorbs that energy with structure instead of formlessness.
- **Size:** Theme (1–2 phrases) + N variations of equal length; N from a complexity heuristic (§8).

### 1.2 Held in reserve (do NOT ship first)

**(4) Through-composed.** No return; continuous development. *This is what the engine accidentally produces today*, and it is exactly the structureless feel the operator rejected. Ship it only later, deliberately, for images that genuinely warrant a journey-without-return (a strong directional gradient — see §8) — and even then, gate it behind motivic continuity (recurring fragments) so it is "developing," not "rambling."

**Rondo (`A B A C A`)** is also reserved: it is just rounded binary's logic extended, useful once the three above are solid, ideal for images with **multiple distinct salient regions** (each episode = a region). Defer until region/saliency reading exists.

### 1.3 What makes a form audible — the cross-cutting rules (these are requirements on §4–§6)

- **Return must be recognizable:** A′/A reuses the *same theme* (§5) in the *same home key* (§4). Recognition is the whole point.
- **Contrast must be real:** B differs in **at least two** of {key center, mode/parallel-mode, character, tempo, register}. One axis is not enough to read as a new section.
- **Section boundaries must cadence:** the last phrase of every section ends on a cadence the existing `plan_phrases` already stamps; *between* sections, the boundary cadence is **stronger** than internal phrase cadences (a PAC closes a section; half cadences live inside). The Coda, if present, is a final tonic prolongation after the structural PAC.

---

## 2. CHARACTER / GENRE

A character is **not a label** — it is a **bundle of concrete musical parameters** applied across the piece. Each preset fixes: **meter** (§3), **tempo range** (BPM, intersecting the S13 brightness→tempo map), **texture** (which orchestral roles are active and how), **rhythmic signature** (the characteristic onset pattern, expressed in the existing `realize_rhythm` vocabulary), **articulation tendency** (a bias on the S13 continuous curve), and **dynamic posture** (a bias/shape on `realize_velocity`'s contour).

**Vocabulary discipline:** ship **four** characters, default to one. Each is a coherent preset, defensible to a trombonist's ear.

| Character | Meter | Tempo range (BPM) | Texture | Rhythmic signature | Articulation tendency | Dynamic posture |
|---|---|---|---|---|---|---|
| **Ballad** *(DEFAULT)* | 4/4 | 56–76 (slow) | full ensemble, melody-led, sustained fill | mostly sustained + the dotted "singing" figure; sparse onsets | strongly legato (curve biased toward `LEGATO_FRAC_HI`, overlapping) | gentle messa-di-voce arches; wide-but-soft; the safest "pleasant" default |
| **Waltz** | **3/4** | 96–144 | bass on beat 1, fill on 2–3 ("oom-pah-pah"), melody floats | strong-weak-weak; bass downbeat anchor every measure | portato bass, legato melody | lilting accent on beat 1, lighter 2–3 |
| **March** | 4/4 (or 2/4) | 96–120 | bass + melody prominent, fill steady | even, driving; arpeggio/eighth subdivision on melody; bass on 1 & 3 | detached/marcato (curve biased toward `STACCATO_FRAC`) | firm, terraced; strong beats 1 & 3 accented |
| **Lament** | 4/4 *(or 6/8 for a flowing dirge)* | 48–66 (very slow) | melody + bass, fill thin or resting | long values, suspensions, falling lines; rests-as-gesture | legato, weighted; phrase-end ritardando exaggerated | dark, narrow-low dynamics; descending dynamic shapes; favors minor/Aeolian/Phrygian modes |

**Reserved character: Scherzo** (fast 3/4 or 6/8, light staccato, playful) — defer; it overlaps Waltz mechanically and adds tuning surface without a new *audible* category for the first ship.

**How character binds to existing code (no new realizer needed):**
- *Tempo range* **clamps/centers** the S13 brightness→tempo result (§4.3) — character sets the window, brightness picks within it.
- *Texture* sets which `OrchestralRole`s are active and, for waltz/march, which roles sound on which beats (this is the **meter binding**, §3).
- *Rhythmic signature* selects/biases the branch taken in `realize_rhythm` (sustained vs dotted vs arpeggio vs syncopated) by **shifting the `edge_activity` band thresholds per character** — e.g. March lowers the arpeggio threshold so onsets subdivide readily; Ballad raises it so the line stays sustained.
- *Articulation tendency* applies a **per-character multiplier on `base_frac`** before the existing `0.30..1.20` clamp — Ballad ×(toward 1.05+), March ×(toward 0.40). (This is exactly the "global articulation bias" the S13 spec deferred for lack of a per-step `texture` value; the CompositionPlan supplies it as a *plan field*, so no seam change.)
- *Dynamic posture* biases `realize_velocity`'s `level_gain` and accent weights per character — but **only as a plan-supplied scalar**, leaving the function body's golden-pinned formula structure intact where the equivalence net pins it (see §7 caution).

---

## 3. TIME-SIGNATURE / METER MECHANISM

**There is no meter today.** The engine is a flat stream of equal "steps"; `decide_instrument_action` indexes `plan[step_idx % plan.len()]` and every step is metrically equivalent except for the phrase-position weighting `realize_velocity` already applies (`position_in_phrase == 0` gets +9, even positions +2, odd −6). That phrase-position accent is a *latent meter* — it already distinguishes strong from weak — but it is keyed to **phrase position, not measure position**, and there is no notion of "beats per measure." This section makes meter real.

### 3.1 The step↔beat↔measure relationship — the core definition

**Define: one STEP = one BEAT.** This is already the implicit S13 convention (`ms_per_step = 60000 / bpm`, "one step = one beat"). We keep it. Then:

- A **measure** is a fixed run of `beats_per_measure` consecutive steps.
- `beats_per_measure` comes from the character's meter: **4/4 → 4, 3/4 → 3, 6/8 → 2** (a 6/8 measure is felt in **2 dotted-quarter beats**; if we want the lilt audible at the eighth-note level we treat 6/8 as 6 steps grouped 3+3 — see 3.3).
- **A phrase is a whole number of measures.** Today phrases are 4 or 8 steps (`PHRASE_LENGTHS`). Under meter: a 4/4 phrase of 4 steps = **1 measure** (tight) or we redefine phrase length as *measures* (4-measure and 8-measure phrases = 16/32 steps in 4/4). **Recommendation: keep `PHRASE_LENGTHS` meaning *beats/steps* for the first ship** (so a 4-step phrase = one 4/4 measure, an 8-step phrase = two), and let **meter group the steps within the existing phrase**. This is the minimal change: meter becomes a *grouping overlay* on the existing step stream, not a restructuring of the phrase model.

### 3.2 Strong/weak beat hierarchy

Each step carries a **metric weight** derived from its position **within the measure** (`step_index_in_section % beats_per_measure`):

- **4/4:** beat 1 = STRONG (primary), beat 3 = STRONG (secondary), beats 2 & 4 = WEAK. (The textbook 4/4 hierarchy.)
- **3/4:** beat 1 = STRONG, beats 2 & 3 = WEAK. (The waltz downbeat.)
- **6/8 (felt in 2):** beat 1 = STRONG, beat 4 = secondary STRONG, the rest WEAK (the 3+3 grouping).

This metric weight is the input the realizer already wants: today `realize_velocity` accents by *phrase* position; under meter it accents by **measure** position. **Binding: replace (or augment) the `position_in_phrase`-based accent with a `metric_position`-based accent.** The S6 logic is already shaped exactly right — strong gets +, weak gets − — it simply needs to read the measure beat instead of the phrase index. This is a *re-pointing* of an existing mechanism, the cleanest possible meter introduction.

### 3.3 How character selects meter, and how the texture realizes it

Meter is not just accent — it is **which instrument sounds on which beat**, and that is where 3/4 *sounds* like a waltz rather than 4/4 with a different accent.

- **Waltz (3/4):** **Bass on beat 1 only**; **HarmonicFill on beats 2 and 3** ("oom-pah-pah"); Melody free. This is realized by gating each `OrchestralRole`'s onset by `metric_position` — the plan tells `realize_rhythm` "this role rests on these beats." (Today fill can already rest-as-gesture on weak beats; the waltz pattern is the *systematic* version of that.)
- **March (4/4):** Bass on beats 1 and 3; Melody subdivides (arpeggio/eighths); Fill sustains. Drum-like regularity.
- **Ballad (4/4):** no per-beat gating; the meter is felt only through the accent hierarchy and the slow tempo — the line is continuous. (This is why Ballad is the safe default: it needs the *least* new metric machinery to sound right.)
- **6/8 lilt (Lament option):** steps grouped 3+3, strong on 1 and 4, holds favor the dotted feel.

**The contract this puts on the plan (§7):** each Section carries a `meter: Meter { beats_per_measure, strong_beats: &[usize] }`, and each `OrchestralRole` carries an optional **beat-mask** (which beats it sounds on). `realize_rhythm`/`realize_velocity` consume `metric_position` (derivable from the global step index + the section's meter + the section's start step) instead of (or in addition to) `position_in_phrase`. **No new realizer function** — the meter is two new scalar inputs (beats-per-measure, metric position) threaded into the two functions that already accent and gate.

---

## 4. STRUCTURAL KEY / TEMPO PLAN

Today: **one key** (`root_midi = 60`, C4, constant), **one mode** (from `avg_hue`), **one tempo** (S13 brightness→BPM, applied once globally to `config.ms_per_step`). There is no modulation, no sectional tempo, no structural ritard/accel. That flatness is half of "structureless." This section makes key and tempo **evolve across sections** while *extending*, not fighting, the S13 image-driven choices.

### 4.1 The tonic plan (key across sections)

The **home key** is the existing image-derived tonic (root + mode from hue). The plan adds **sectional key relationships** that the ear reads as departure-and-return:

- **Rounded Binary / ABA:** A in the **home key**; **B modulates to a closely related key** — the standard choices, in priority order:
  1. **the dominant** (up a perfect fifth, `root + 7`) — the brightest, most idiomatic departure for a major-mode home;
  2. **the relative major/minor** (relative major = `root + 3` from a minor home; relative minor = `root − 3` from a major home) — same key signature, gentler;
  3. **the parallel mode** (same root, switch Ionian↔Aeolian) — strongest *character* contrast for the least pitch disruption, ideal for ABA where B "is" the shadow side.
  Then **A′/A returns to the home key.** The return-to-home *is* the form's resolution.
- **Theme and Variations:** stays in the **home key** (variations transform surface, not tonic) — except optionally **one variation in the parallel mode** (the classic "minore"/"maggiore" variation), a stock, beloved device that adds color without breaking the form.

**Modulation mechanics (reuse existing harmony):** between sections, insert a **pivot** — the existing `secondary_dominant_of` already builds an applied dominant; the dominant of the *new* key is exactly the pivot chord that tonicizes it. So a B-section in the dominant is reached by inserting **V/V** at the A→B seam (the engine already knows how to spell this). The return to home is reached by the new key's own dominant resolving back, or simply by restating A's opening in the home key (a "phrase modulation" — abrupt but valid, and the simplest first ship). **Tonicization, not full functional modulation, is the honest first target** — it is what the existing chord vocabulary supports without new machinery.

**Register/singability guard:** transposing `root_midi` must keep the bass register sane. The existing realizer re-seats every chord tone into role register bands (`BASS_REGISTER_FLOOR` etc.), so a transposed root is safe *as long as* the plan bounds transposition to ±7 semitones and the seating clamps to 24..=108 (it already does). Keep B's key within a perfect fifth of home.

### 4.2 The mode plan

Mode is the home mode (from hue) in A and A′. In B, mode either:
- **switches to the parallel** (Ionian↔Aeolian) for character contrast, or
- **stays** if the key already moved (don't change two things at once unless the image is genuinely high-contrast).

This *extends* the S13 hue→mode and colorfulness→mixture work: the global mode pick still seeds the home mode; the plan decides whether B borrows the parallel. The S13 mode-mixture (bVI, borrowed iv) becomes a **B-section coloring device** rather than a global wash — which is exactly how mixture is used in real music (locally, for shadow, not everywhere).

### 4.3 The tempo plan (tempo across sections, extending S13)

S13 derives **one** tempo from whole-image brightness. The plan keeps that as the **base tempo** and adds:

1. **Character window:** the character (§2) defines a BPM range; the S13 brightness→BPM result is **clamped/centered into that window**. (A "bright Ballad" is still a Ballad — slow — just at the bright end of the ballad range. This resolves the tension between "brightness wants fast" and "this is a ballad": character wins the *category*, brightness wins the *position within it*.)
2. **Sectional tempo relationship:** B may sit at a **proportionally related tempo** — the stock choices are *same tempo* (default, safest), *slower B* (a lyrical middle), or *faster B* (an agitated middle). Use a small set of ratios (e.g. ×1.0, ×0.85, ×1.18) so the relationship is audible but not jarring. Tie the choice to image contrast (§8): a high-contrast image earns a tempo change; a uniform one keeps one tempo.
3. **Structural ritardando:** the **final phrase of the final section** rings slower into the close. The realizer **already has** `RITARDANDO_FACTOR` applied at cadences — the plan promotes this to a **section-final ritardando** by lengthening `ms_per_step` (or the hold fractions) across the last measure. Minimal: reuse the existing cadence ritardando and let the plan mark the structural cadence as "the big one."

**Binding:** each Section carries `ms_per_step` (its own tempo). Since `ms_per_step` is already `EngineConfig` state and S13 already overwrites it once, the only change is that the **plan carries a per-section tempo** and the engine sets `config.ms_per_step` at each **section boundary** (or `decide_step` reads the current section's tempo). This is a small generalization of the S13 single-overwrite, owned on the Architect's side; musically the contract is "tempo is a section property, not a global constant."

---

## 5. THEMATIC MATERIAL — THE MOTIF THAT RETURNS AND IS VARIED

**This is the single most important section.** Return of a recognizable theme is what converts "a stream of pleasant chords" into "a piece." Everything else (form, key plan, meter) is scaffolding *for* the theme's journey. Today there is **no theme** — the melody role plays whatever chord tone its register band selects, per step, with no memory and no identity. We give the piece a tune.

### 5.1 What a theme *is* in this engine

A theme is a short **melodic gesture** carried by the **Melody `OrchestralRole`** over the first **statement** section's phrases — concretely, a **contour + rhythm over the section's chords**:

- a **pitch contour**: a sequence of scale-degree choices (e.g. `[1, 3, 2, 1]` = do-mi-re-do), realized against whatever chord is current via the existing chord-tone/non-chord-tone machinery;
- a **rhythmic profile**: which of the `realize_rhythm` figures each note uses (e.g. long-long-short-long);
- bound to the **phrase**: the theme spans one phrase (4 steps) and is the *thing the ear latches onto*.

The theme **lives in the plan**, not in the realizer — it is a small data structure (`MotifCell { degrees: Vec<i8>, rhythm: Vec<RhythmFigure> }`) the plan attaches to **Melody-role steps in thematic sections**. The realizer, when it sees a step that carries a motif note, plays *that degree* in the melody register instead of free-selecting a chord tone. **This is the key reuse:** `role_pitch`/`realize_rhythm` already place and articulate the melody; the motif just **dictates the melody's pitch-class choice and rhythm** for thematic steps, overriding the free pick. Non-thematic roles (bass, fill) are unchanged.

### 5.2 How the theme is GENERATED (from image seeds)

The motif must be **derived from robust heuristic image properties** so it "fits the image," and it must be **memorable** (short, mostly stepwise, clearly contoured). Generation seeds:

- **Contour direction** from a robust image gradient — e.g. overall **brightness gradient top↔bottom** (bright-top image → descending theme "from the light"; bright-bottom → ascending). Where no region/gradient signal exists yet, fall back to a **fixed pleasing contour** (an arch: up then down) seeded by `avg_hue` so different hues pick different *stock* contours from a small curated set (~4 contours). **Stay heuristic** — do not require subject recognition.
- **Contour *range*** (how far it leaps) from **edge density / complexity**: calm image → conjunct, stepwise theme (mostly `±1` scale degree); busy image → wider, more disjunct theme (allowing `±2`/`±3` leaps). This reuses the same activity signal S13 already normalizes.
- **Length** from phrase length (4-step theme for a 4-step phrase) — fixed, not image-driven, for memorability.
- **Rhythmic profile** from the character (§2): Ballad → long-note theme; March → even/subdivided; Waltz → strong-downbeat theme. So the *character* gives the theme its rhythmic identity, the *image* gives it its pitch contour. Clean separation.

**Curated, not generative-infinite:** the contour vocabulary is a **small fixed set** (arch, descent, ascent, neighbor-turn — ~4 shapes), each a known-good melodic archetype; the image *selects and parameterizes* one. This is the §counterpoint-3 discipline: a small coherent vocabulary, quality over breadth, no infinite tuning loop.

### 5.3 How the theme RETURNS (marks the form)

- In **rounded binary `A B A′`**: the theme states in A, is **absent or fragmented** in B (B is contrast — either a *new* contour or motivic fragments of A, never the full theme), and **returns recognizably in A′**. The return is in the **home key** (§4) so it is unmistakable.
- In **ABA**: theme states fully in A, B has its own (contrasting) material, A returns the theme **complete**.
- In **theme-and-variations**: the theme states in T, then **each variation transforms it** (next subsection) while keeping its skeleton recognizable.

**The plan marks which sections carry the theme** (`thematic_role: Statement | Return | Development | Contrast`), and the realizer plays the motif on Melody-role steps in Statement/Return/Development sections, free-selects in Contrast sections.

### 5.4 Variation techniques (when the theme returns)

When the theme returns or varies, apply **one or two** of these stock, audible transformations (a small curated set — do not stack many at once):

- **Rhythmic augmentation / diminution:** double or halve the note values (theme returns "broadened" or "quickened"). Cheapest, most audible. Realized by scaling the motif's `RhythmFigure` choices.
- **Transposition:** state the theme from a different scale degree or in the new key (used inherently by the key plan in §4).
- **Reharmonization:** keep the contour, change the chords under it (the existing progression machinery already varies; the plan can hand a thematic section a *different* progression while the same motif rides on top). Powerful and idiomatic.
- **Ornamentation:** add passing/neighbor tones between the motif's structural pitches (the engine's voice-leading layer already understands stepwise connection; ornamentation is inserting a step between two motif degrees).
- **Fragmentation:** in B or development, use only the **head** of the motif (first 2 degrees), repeated/sequenced — the "developing" gesture that keeps continuity without full restatement. This is what makes a *through-composed reserve form* (§1.2) sound developed rather than rambling.

**Discipline:** A′ in rounded binary uses **at most light variation** (a reharmonization or a small ornament) — the *recognizable* return is the goal; heavy variation defeats it. Save augmentation/diminution/fragmentation for theme-and-variations and development, where transformation *is* the point.

### 5.5 Theme vs. the current texture

Today all roles are essentially harmonic (block chord realized per role). The theme makes the **Melody role melodic** for the first time — a real top line with identity — over the **unchanged** bass + harmonic-fill accompaniment. This is the orchestrational payoff: a tune over an accompaniment, which is what "music" sounds like, versus the current homogeneous chord-stream. The bass and fill continue exactly as S6/S13 realize them; only the melody gains a will of its own.

---

## 6. MORPHING / PROGRESSIVE HARMONY

S13 gave **per-step** harmonic variety (7ths/9ths by saturation, occasional secondary dominant, mixture). What's missing is a **harmonic trajectory across the whole arc** — tension that builds and releases at structural points, a climax that *sits somewhere*, cadences whose strength signals where you are in the form. Without this, the harmony is "varied but directionless" — another face of "ethereal."

### 6.1 Harmonic trajectory by section

- **A (statement):** harmonically **stable** — mostly diatonic, the home key clearly established (start on tonic, the progression families already do this), close on a **weak/medium cadence** (half cadence or IAC) so the section feels *open*, posing a question.
- **B (contrast/departure):** harmonically **least stable / most tension** — this is where mixture, secondary dominants, and the key change live. B is the **harmonic high-water mark**: the most non-diatonic color, the furthest from home. The S13 mixture/secondary-dominant devices, which today fire randomly per-edge, are **concentrated here by the plan** — they belong to the departure.
- **A′/A (return):** harmonically **resolving** — back to diatonic home, ending on the **strongest cadence in the piece** (a root-position PAC, soprano on tonic). The existing `plan_phrases` already builds a PAC when a dominant precedes the final tonic; the plan ensures the structural cadence *is* a PAC.

### 6.2 Where the climax sits

The **climax** — the point of greatest intensity — should sit at a **principled, recurring location**, not randomly. Two defensible placements (pick one as default):

- **End of B / start of return** (recommended default): the tension of the departure peaks just before home reasserts. This is the most common-practice placement and reads clearly: maximum distance from home → the relief of return.
- **Golden-section point** (~62% through): a more sophisticated placement; defer.

Intensity is realized through the dimensions the engine already controls: **higher register** (melody octave lift — `role_pitch` brightness term, but plan-driven at the climax), **louder dynamics** (`realize_velocity` level), **denser rhythm** (more onsets — `realize_rhythm` bands), and **richest harmony** (the 7th/9th + secondary-dominant concentration). The plan **marks the climax step**; the realizer pushes all four knobs toward their high end there. This is a *structural* use of the per-step knobs S13 already built — they finally have somewhere to point.

### 6.3 Cadence strength as a structural signal

Cadences already exist (`HalfCadence`, `PerfectAuthenticCadence` in `plan_phrases`). The plan uses **cadence strength as punctuation**:

- **internal phrase boundaries:** half cadences (commas);
- **section boundaries:** stronger cadences — IAC or PAC (periods);
- **the structural close:** the **only** root-position PAC with melody on the tonic, plus the structural ritardando (full stop).

The existing cadence machinery supports all of this; the plan's job is to **assign the right cadence strength to the right boundary** rather than letting every phrase cadence identically. This single change — *differentiated* cadence strength — does enormous work for "this has a shape."

---

## 7. THE COMPOSITION PLAN (THE MUSICAL CONTRACT)

This pulls §1–§6 into the **one structure computed once, before any note is realized**, and states exactly what it carries and how it drives the existing realizer. This is the contract the Rust Architect binds to the engine.

### 7.1 Where it sits in the pipeline

Today: `set_features_global(global)` → (mode, progression, `generate_chords`, `plan_phrases`) → `Vec<StepPlan>` → `decide_instrument_action` loops `plan[step_idx % len]` → `realize_step`.

Proposed: `set_features_global(global)` → **`compute_composition_plan(global, [+regions])` → `CompositionPlan`** → the plan **expands into a flat `Vec<StepPlan>`** (one entry per step, *no longer looped* — the plan is the whole piece, played once start to finish) → `decide_instrument_action`/`realize_step` consume each `StepPlan`, now enriched with section/meter/motif context.

**Critical reuse:** `CompositionPlan` does **not** replace `StepPlan`; it **generates a richer sequence of them**. The realizer signature is preserved; `StepPlan` gains a few fields (below). The composition layer is a **planner that sits above `plan_phrases`**, calling `generate_chords`/`voice_lead_sequence`/`plan_phrases` **per section** (each section gets its own key/mode/progression/phrases) and concatenating the results into the piece's step list with section/meter/motif annotations.

### 7.2 The musical fields the plan must carry

```
CompositionPlan
  form: Form                      // RoundedBinary | Ternary | ThemeAndVariations | (reserve: ThroughComposed | Rondo)
  character: Character            // Ballad | Waltz | March | Lament
  home_key: KeyCenter             // { root_midi: u8, mode: ModeName }   ← from hue (existing)
  theme: Motif                    // the generated returning idea (§5)
  climax_step: usize              // absolute step index of the structural peak (§6.2)
  sections: Vec<Section>          // the ordered list that IS the piece

Section
  thematic_role: ThematicRole     // Statement | Contrast | Return | Development | Coda
  key: KeyCenter                  // this section's tonic+mode (home, dominant, parallel, relative…) (§4.1/4.2)
  tempo_ms_per_step: u64          // this section's tempo (character window ∩ image tempo) (§4.3)
  meter: Meter                    // { beats_per_measure: u8, strong_beats: Vec<u8> } (§3)
  character_overlay: CharacterOverlay  // per-section articulation/dynamic/rhythm biases (§2 binding)
  progression: Vec<String>        // Roman numerals for this section (may differ per section) (§6.1)
  phrases: Vec<PhraseSpec>        // phrase lengths + boundary cadence strength (§6.3)
  motif_active: bool              // does the Melody role play `theme` here, or free-select? (§5.3)
  motif_variation: Option<VariationOp>  // augmentation|diminution|reharmonization|ornament|fragment (§5.4)

Motif
  degrees: Vec<i8>                // scale-degree contour (e.g. [0,2,1,0]) — from a curated set, image-seeded (§5.2)
  rhythm:  Vec<RhythmFigure>      // per-note rhythmic profile (character-seeded)

Meter { beats_per_measure: u8, strong_beats: Vec<u8> }   // 4/4→{4,[0,2]}, 3/4→{3,[0]}, 6/8→{2,[0,3] over 6 steps}

CharacterOverlay
  articulation_bias: f32          // multiplier on realize_rhythm base_frac (Ballad>1, March<1) (§2)
  dynamic_bias: f32               // additive/scalar on realize_velocity level (§2)
  rhythm_band_shift: f32          // shifts edge_activity thresholds in realize_rhythm (March denser) (§2)
  role_beat_masks: Map<OrchestralRole, BeatMask>  // waltz oom-pah-pah, march 1&3 bass (§3.3)
```

`StepPlan` (existing) gains (additive, so `decide_instrument_action`/`realize_step` keep working):
```
StepPlan += {
  metric_position: u8,            // beat within the measure (drives accent/gating instead of position_in_phrase) (§3.2)
  beats_per_measure: u8,          // for metric weight (§3.2)
  motif_note: Option<i8>,         // if Some, the Melody role plays THIS scale degree (the theme), not a free pick (§5.1)
  is_climax: bool,                // push register/dynamics/density/harmony to the high end here (§6.2)
  character_overlay: CharacterOverlay (or an index into one),  // per-step access to the section's biases
}
```

### 7.3 How the plan drives the existing realizer (per dimension)

- **Form / sections:** the planner calls `generate_chords` + `plan_phrases` **once per section** (each with that section's key/mode/progression) and concatenates the `StepPlan`s, stamping `thematic_role`, `key`, `tempo`, `meter`, and the boundary cadence strength. `decide_instrument_action` plays the concatenated list **straight through** (drop the `% plan.len()` loop). *This is the structural fix: the plan is the whole piece, not a loop.*
- **Character:** `CharacterOverlay` rides each `StepPlan`; `realize_rhythm` multiplies `base_frac` by `articulation_bias` and shifts its `edge_activity` band thresholds by `rhythm_band_shift`; `realize_velocity` applies `dynamic_bias`. (Plan-supplied scalars only — see §7.4.)
- **Meter:** `realize_velocity`'s accent reads `metric_position`/`beats_per_measure` (strong beats accented) instead of (or alongside) `position_in_phrase`; `realize_rhythm` consults `role_beat_masks` to rest a role on its off-beats (waltz/march). Two new scalar inputs, existing logic re-pointed.
- **Key/tempo plan:** each section's `StepPlan`s already carry transposed chords (the planner transposed `root_midi` before `generate_chords`); the engine sets `ms_per_step` from the current section's `tempo_ms_per_step` (generalizing S13's single overwrite).
- **Theme:** on Melody-role steps where `motif_note.is_some()`, `role_pitch` seats **that scale degree** in the melody register instead of free-selecting the top chord tone; `motif_variation` transforms the motif before expansion. Bass/fill unaffected.
- **Morphing harmony / climax:** the planner concentrates mixture/secondary-dominant in the Contrast section's `progression` and marks `is_climax`; the realizer, at `is_climax`, pushes register/velocity/onset-count/complexity to their high ends. Cadence strength is set per boundary in `PhraseSpec`.

### 7.4 The one hard caution to the Architect (golden net)

`realize_velocity` and `realize_rhythm`'s **non-cadence** formulas are pinned by `tests/engine_equivalence.rs` golden constants (velocities 114/84, register 36/79, cadence hold 240 ms, etc.). The character/meter/climax biases **will move some of these**. Per the S13 precedent (its spec §7): **re-derive changed goldens by hand from the new documented formula in the same commit; never loosen an assert to silence it.** Where possible, apply biases as **plan-supplied scalars defaulting to identity (×1.0 / +0)** so that the *existing* default plan reproduces today's goldens exactly, and only a non-default character/section changes them. That keeps the regression net meaningful and the diff reviewable. The cadence branch (the 240 ms ring) should stay byte-stable — meter/character act on non-cadence steps.

---

## 8. IMAGE → MUSICAL-ARCHITECTURE MAPPING (the musical half)

For each architecture choice, the **robust heuristic image property** that should drive it, and **why the mapping is perceptually meaningful**. I own *which musical choice each property should map to*; the Architect owns whether/how each property is extractable. I flag anything that genuinely needs semantic recognition as **"semantic, later."** Per counterpoint #2: **no musical choice below *requires* semantic recognition to work** — each has a robust-heuristic driver, with semantic only as optional enrichment.

| Architecture choice | Heuristic image property | Why it's perceptually meaningful | Availability |
|---|---|---|---|
| **FORM** (rounded binary vs ABA vs theme-and-variations) | **composition balance / symmetry** (is the image balanced L/R or top/bottom? does it have one dominant region vs many?) + **complexity** (`shape_complexity`) | A symmetric, balanced image → a returning, balanced form (rounded binary/ABA: depart and return). A complex, busy image → theme-and-variations (much to say about one idea). A strong directional gradient with no symmetry → the through-composed reserve. Symmetry ≈ return; busyness ≈ variation. | balance/symmetry: **new heuristic** (cheap — compare quadrant/half feature means; the Architect can add it). complexity: **exists** (`shape_complexity`, 0.011–2.005, 180× spread — excellent). |
| **NUMBER OF SECTIONS / VARIATIONS** | **number of salient regions** (how many distinct foreground areas) | More distinct regions → more sections/episodes (rondo's natural driver); one subject → simple ternary. The piece's sectional count tracks the image's compositional count. | **region/saliency: does NOT exist today** (only whole-image avgs + a vertical scan strip). Use a **cheap 3-region proxy** (center/border/detail) first; true saliency is **semantic-ish, later**. Until then, drive section count from **complexity** (busy → more variations) as the robust fallback. |
| **CHARACTER** (ballad/waltz/march/lament) | **dominant palette warmth** (`avg_hue` warm vs cool) + **brightness** (`avg_brightness`) + **energy** (`edge_density`/`texture`) | Warmth and brightness are the canonical affect axes: warm+bright → buoyant (march/waltz); cool+dark → somber (lament); warm+soft+low-energy → ballad. High energy → march; low → ballad/lament; a lilting mid → waltz. This is film-scoring intuition, robust and well-calibrated. | **exists** (hue, brightness, edge density all extracted and well-spread). |
| **METER** (4/4 / 3/4 / 6/8) | follows from **character** (waltz→3/4, march→4/4, lilt→6/8) | Meter is a *property of character*, not an independent image read — coupling it to character keeps the vocabulary coherent (no incongruous "3/4 march"). | derived (no new extraction). |
| **TEMPO base** (BPM within the character window) | **brightness** (existing S13) + small **edge-density** nudge | Luminance = energy/arousal (S13's grounded choice); busier texture nudges faster. Character sets the window; image sets the position. | **exists** (S13). |
| **HOME KEY/MODE** | **dominant hue** (and `hue_spread` for mixture) | An image's characteristic color is its tonal color; the *dominant* hue (not the muddy mean) picks mode (the S13 diagnosis's recommended fix); wide palette → more mixture. | mode-from-mean **exists**; dominant-hue-from-histogram is a **recommended upgrade** (`hue_hist` exists per-bar; a global one is a small add). |
| **SECTIONAL KEY (B's modulation)** | **fg/bg or center/border contrast** (how different is the subject from its surround?) | High subject/background contrast → a real key change in B (the departure is "the other thing in the image"); low contrast → B stays near home (a uniform image doesn't earn a modulation). | **center/border contrast: new cheap heuristic** (compare center-region vs border-region feature means — no semantics needed). True fg/bg is **semantic, later**; the center/border proxy is the robust stand-in. |
| **TEMPO relationship (B faster/slower)** | overall **contrast / brightness dispersion** | A high-contrast image earns a tempo shift between sections; a flat one keeps one tempo. Contrast = "this image has dynamic range" = "this piece can change gears." | **brightness stddev / contrast: new cheap heuristic** (the S13 music diagnosis already asked for it). Fallback: same tempo (safe). |
| **THEME contour** | **brightness gradient direction** (top↔bottom) + **hue** (selects among curated contours) | A bright-top/dark-bottom image → a theme descending "from the light"; the dominant hue picks which stock contour archetype. Visual direction → melodic direction is the most literal, satisfying image→music link available without semantics. | gradient: **new cheap heuristic** (compare top-half vs bottom-half brightness — trivial). hue: **exists**. Until the gradient exists, seed contour from hue alone (still image-driven). |
| **THEME range/disjunctness** | **edge density / complexity** | Busy image → wider, leaping theme; calm → conjunct, stepwise. Visual activity → melodic activity (the engine's core, grounded mapping). | **exists** (S13 normalizes edge activity). |
| **CLIMAX intensity & harmonic high-water mark (B)** | **complexity / energy peak**; placement is **structural** (end of B) | The most non-diatonic, densest, highest, loudest moment belongs at maximum distance from home; energy/complexity sets *how* intense, structure sets *where*. | intensity: **exists**; placement: structural (no extraction). |
| **DYNAMIC posture / spread** | **brightness dispersion / contrast** | An image with wide tonal range → music with wide dynamic range; a flat image → narrow dynamics. Perceptually direct: visual contrast = dynamic contrast. | **new cheap heuristic** (brightness stddev); fallback to character default. |

**Heuristic-side summary for the Architect:** the high-value *new* heuristics, all cheap and semantics-free, are: **(1) quadrant/half feature means** (→ balance/symmetry → form; brightness gradient → theme contour), **(2) center-vs-border feature contrast** (→ B's modulation, the cheap "subject-pop" proxy), and **(3) brightness/contrast dispersion** (→ tempo-change and dynamic-spread). None requires ML. **True saliency / subject recognition / scene classification is "semantic, later"** and only *enriches* (better region count, literal subject matching); the form/character/key/theme vocabulary above all functions on the heuristic signals alone.

---

## 9. STAGED MUSICAL ROADMAP + THE HIGH-LEVERAGE FIRST SLICE

Each stage **lands audibly** — the operator can hear the difference at every step. Stages are ordered by **audible-structure-per-unit-effort**, and every stage is **pure-Rust, no semantic recognition** until the last optional one.

### The recommended FIRST slice (the single highest-leverage pure-Rust build)

**Build the returning theme over a fixed default form (rounded binary `A B A′`) in a single default character (Ballad, 4/4), all in the home key — i.e. §5 + the skeleton of §1 + §7, with §3/§4 held at defaults.**

Concretely, the first slice delivers:
1. A **`CompositionPlan` that lays out 3 sections** (A statement / B contrast / A′ return) and **expands to a non-looping flat `StepPlan` list** played once start to finish (kills the `% plan.len()` loop — the structural root cause).
2. A **generated motif** (from the curated contour set, image-seeded by hue + edge activity per §5.2) that the **Melody role plays in A and A′** and is **absent/fragmented in B**, with **differentiated cadence strength** (half cadence ends A, PAC ends A′ per §6.3).
3. **Default everything else:** one character (Ballad), one meter (4/4 via the existing phrase-accent, no per-beat gating yet), one key (home), one tempo (S13's). No modulation, no meter gating, no climax pushing yet.

**Why this is the highest leverage:** the operator's complaint is *"structureless and ethereal."* A **recognizable tune that states, departs, and returns** is the *defining* cure for structurelessness — it is the difference between "a piece" and "a texture" — and it requires **no new image extraction** (hue + edge activity already exist) and **no meter/modulation machinery**. It exercises the whole `CompositionPlan` → non-looping-`StepPlan` → realizer pathway end-to-end, so every later stage is an *enrichment of a working spine* rather than new plumbing. One build, and the engine stops sounding like a scanner.

### The full staged roadmap

- **Stage 1 — FIRST SLICE (above):** returning theme + rounded-binary skeleton + non-looping plan. *Audible: a tune that comes back; a real ending.*
- **Stage 2 — METER (§3):** introduce `beats_per_measure` + metric accent (re-point `realize_velocity` to `metric_position`); add **Waltz (3/4)** with oom-pah-pah role beat-masks. *Audible: a downbeat you can feel; 3/4 vs 4/4.*
- **Stage 3 — CHARACTER (§2):** the four presets (Ballad/Waltz/March/Lament) as `CharacterOverlay` biases on articulation/dynamics/rhythm; character selected from warmth/brightness/energy (§8). *Audible: the same image as a ballad vs a march.*
- **Stage 4 — KEY/TEMPO PLAN (§4):** sectional modulation (B to dominant/parallel via the existing applied-dominant pivot), return to home; per-section tempo; structural ritardando. *Audible: B "goes somewhere" and the return lands home.*
- **Stage 5 — MORPHING HARMONY + CLIMAX (§6):** concentrate mixture/secondary-dominant in B; mark and realize the climax; full cadence-strength hierarchy. *Audible: a peak; harmonic shape.*
- **Stage 6 — VARIATION TECHNIQUES + THEME-AND-VARIATIONS FORM (§5.4, §1):** augmentation/diminution/reharmonization/ornament/fragment; the third form. *Audible: development, not just repetition.*
- **Stage 7 — ABA + form selection from image (§1, §8):** wire form choice to balance/symmetry/complexity; add the cheap quadrant/center-border heuristics. *Audible: different images get different forms.*
- **Stage 8 (optional, later) — region/saliency reading:** the cheap 3-region proxy, then **semantic subject/scene** as pure enrichment (section count from real regions, literal subject matching). *This is the only stage that may need ML; everything above stands without it.*

### Fold-in: the S13 articulation-clamp carry-over (cheap, ride-along)

S13's continuous articulation curve produces **unpleasant extremes** — sometimes notes overlap too much (the `LEGATO_FRAC_HI = 1.05` legato end on calm images stacks/blurs), sometimes too short/staccato (the `STACCATO_FRAC = 0.40` busy end clips). **Musical clamp to apply (a sensible note-length range), folded into Stage 1 or 2:** constrain the realized **non-cadence** hold fraction to a perceptually pleasant window — **roughly `0.55 ≤ base_frac ≤ 1.10`** (a hair of overlap at the legato end for true legato, never below ~half-slot at the staccato end so even "detached" notes still *speak* on a sustaining patch). The cadence ring (1.20 cap) stays as-is. Rationale: below ~0.5 on a piano/brass patch a note reads as a click rather than a tone; above ~1.15 successive notes mud together and lose articulation. This narrows the curve's *output* range without flattening its *responsiveness* — it still varies with edge activity, just within a window the ear finds musical. Cheapest possible quality win; ride it along with the first structural build. *(Note: the per-character `articulation_bias` in §2 then rides on top of this clamped window — Ballad pushes toward 1.10, March toward 0.55 — so character and clamp compose cleanly.)*

---

## 10. OPEN MUSICAL DECISIONS FOR THE OPERATOR

Genuine forks where the operator's ear/taste should choose — not things to guess:

1. **Default form.** I recommend **rounded binary (`A B A′`)** — smallest form with statement+contrast+return, most forgiving. Alternatives: full **ABA** (stronger contrast, more literal return) or **theme-and-variations** (if he prefers "one idea explored" over "departure and return"). *His call which feels most "him."*
2. **Default character.** I recommend **Ballad** (slow, legato, the safest "pleasant" — and it needs the least new metric machinery for Stage 1). Alternative default: **Waltz** (more immediately characterful, but waits on Stage 2's meter). *Pick the first-impression sound.*
3. **How literal the image→music mapping should be.** A spectrum: **(a) abstract/affective** — image sets mood/energy/color, music is its own coherent piece (my lean; most robust, most musical); vs **(b) literal/descriptive** — brightness gradient *is* the melodic contour, region count *is* the section count, "you can hear the picture." Literal is more impressive when it lands and more wrong when it doesn't, and pushes toward semantic recognition sooner. *This single choice shapes how aggressively we chase region/saliency/semantic features.*
4. **Climax placement.** End-of-B (recommended, classic) vs golden-section (~62%) vs none-for-now. *A taste/sophistication call.*
5. **Modulation aggressiveness.** Tonicization-only (safe, my Stage-4 lean) vs real functional modulation with pivot chords (richer, more machinery, more chances to sound wrong). *How adventurous should B be.*
6. **Does the theme appear in B at all?** Fully absent (maximum contrast) vs fragmented (more unity, more "developed") vs a contrasting *second* theme. *Unity-vs-contrast taste.*
7. **The honest ceiling.** Confirm the target is **"principled, fits the image, pleasant"** (achievable with this small vocabulary) and **not** "sounds hand-composed / great." Setting the ceiling explicitly prevents an infinite tuning loop and keeps the vocabulary small (counterpoint #3). *I recommend stating this as a project ground-truth.*

---

*End of Section A (musical architecture). No source, test, or asset modified. The composition layer is a planner that sits ABOVE the existing craft: it computes one CompositionPlan (form → sections → per-section key/tempo/character/meter/thematic-role + a generated returning theme + a marked climax), expands it into a NON-LOOPING StepPlan sequence, and drives the unchanged realizer (`generate_chords`/`voice_lead_sequence`/`plan_phrases`/`realize_step`) with a few additive scalar fields. The single highest-leverage first build is the returning theme over a rounded-binary skeleton played once start to finish — the defining cure for "structureless," requiring no new image extraction and no meter/modulation machinery. Everything heuristic-first; semantic subject recognition is optional, last, and enriches only.*

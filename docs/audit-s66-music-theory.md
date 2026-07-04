# Music-Theory Gap Audit — Cross-Piece "Sameness"

**Scope.** A read-only music-theory audit of the composition engine against the cited
principle brief `docs/research-s66-sameness-principles.md`, on three axes:
**Axis 1 — melodic development / motivic transformation**, **Axis 2 — harmonic vocabulary
beyond diatonic**, and **Axis 4 — groove / feel / rhythm** (the priority axis, the one the
listener's own words point at). No code is modified; every source file is frozen for this
audit. Verdicts use the brief's three grades: **IMPLEMENTED**, **SHALLOW-FAKED** (the
mechanism exists in name but is applied so uniformly/thinly it does not register as variety),
**ABSENT**. Line numbers reference the current tree; corpus firing numbers are taken from the
already-committed diagnostics (`diagnostic-s62-motivic-derivation.md`,
`diagnostic-s63-vocabulary-breadth.md`, `diagnostic-s64-fixed-rhythm-key.md`), whose test
harnesses measured the six bundled images against the real planner.

---

## Axis 1 — Melodic Development / Motivic Transformation

The master principle (Schoenberg): a piece grows by **developing variation** — the same
motive returns in *transformed* guises — as opposed to **restatement**, where it returns in
the *same* guise. The engine has a motif object and a variation seam, so the machinery *looks*
present; the audit is whether it TRANSFORMS or merely RESTATES/TRUNCATES.

| Principle | Engine location | Verdict | Justification |
|---|---|---|---|
| **Developing variation** (restate-vs-develop, the master principle) | `ThemeVariation` enum `composition.rs:414-428`; live set clamped to `{Identity, Fragmented}` by `clamp_variation_slice1` `composition.rs:1968-1973` (and asserted `composition.rs:2632`) | **ABSENT** | The only two operations a section can apply to its subject are **Identity** (restate, then hold the final note — `chord_engine.rs:4148-4152`) and **Fragmented** (play a head, then rest). Neither is a transformation of the motive; the piece never presents its subject *changed*. |
| **Sequence** (restate a motive at a new pitch level) | Baked as one fixed contour `RisingSequence = [0,1,2,1,2,3]` `chord_engine.rs:3587` | **SHALLOW-FAKED** | Sequence exists only as one of eight *pre-authored fixed contours*, not as an operation that can sequence an arbitrary subject. It is a frozen shape selected or not; it cannot spin a stated motive up a step as continuation, which is what sequence *does* developmentally. |
| **Inversion** (reverse melodic direction) | Enum variant `ThemeVariation::Inverted` `composition.rs:426`; contour stored as degree-offsets expressly so "inversion (negate) … [is] clean" `chord_engine.rs:3568-3573` | **ABSENT** (locked off) | The representation was designed for inversion and the enum variant exists, but `clamp_variation_slice1` collapses `Inverted` to `Identity` (`composition.rs:1971`); the negation is never applied anywhere. A designed-but-disabled capability. |
| **Retrograde** (state the sequence backwards) | Enum variant `ThemeVariation::Retrograde` `composition.rs:427` | **ABSENT** (locked off) | Declared, never reachable — clamped to `Identity`. No code reverses a motif. |
| **Augmentation / diminution** (proportional lengthen/shorten of the motive's rhythm) | Enum variant `Augmented` `composition.rs:422` (locked off); per-archetype "broad/augmented" cell 1 and "busy/even" cell 2 in `rhythm_cells` `chord_engine.rs:3621-3695` | **SHALLOW-FAKED** | The `Augmented` *variation* is clamped off. Augmentation/diminution survive only as fixed **gait cells** chosen **once per piece** (`pick_piece_cell` `chord_engine.rs:3835`, one motto cell stamped on every section — see the vocabulary-breadth diagnostic §1 #2). Selecting a broad-or-busy gait for the whole piece is a per-piece *parameter*, not a *developing* augmentation of a stated idea across the span. |
| **Fragmentation** (isolate a sub-portion and work it) | `theme_melody_pitch` Fragmented arm `chord_engine.rs:4125-4147` | **SHALLOW-FAKED** | "Fragmented" plays the first half of the motif by note-count (`div_ceil(2)`, `:4131`) and then **rests** (`:4146`). Schoenberg's fragmentation *isolates and continues to work* a sub-cell; here the tail is silence, not development — which is why an ear-test of cadence-gesture fragmentation reported "jarring cuts of musical lines… silent breaks." It is head-truncation, not motivic working-out. |
| **Extension / liquidation** (spin-out; progressive stripping toward a cadence) | — | **ABSENT** | No spinning-out, and no liquidation: the phrase does not dissolve its motive toward the close. The cadence is a fixed structural gesture (a ritardando "ring," `chord_engine.rs` cadence early-return; see form/harmonic-rhythm diagnostic §1b), not a liquidation of the subject. |
| **Interval expansion / contraction** (widen/narrow the characteristic intervals) | `range_degrees = round(2 + edge_activity·5)` → 2..=7, scales the contour `resolve_motif_celled` `chord_engine.rs:3866+` | **SHALLOW-FAKED** | A single scalar stretches or compresses the *entire* fixed shape uniformly, chosen once from `edge_activity`. This is a global ambit scaling of one frozen contour, not a motivic interval-expansion operation applied developmentally (a rising third *becoming* a rising sixth on restatement). |
| **Motivic saturation / *Grundgestalt*** (nearly all material derived from one shape) | Theme substitutes only the **Melody** pitch, and only on Statement/Return `chord_engine.rs:4110-4120` | **ABSENT** | The subject touches melody alone; bass, harmonic fill, and accompaniment figuration are generated independently. The piece is not *derived from one shape*, so it cannot achieve unity-through-transformation — the surrogate for it is unison restatement, i.e. uniformity. |

**Axis 1 summary.** The engine RESTATES and TRUNCATES; it does not DEVELOP. Three of the
canonical transformation operations (inversion, retrograde, augmentation-as-variation) exist as
**declared-but-clamped-off enum variants** — the code was scaffolded for developing variation and
then pinned to a two-operation subset. The remaining "transformations" are per-piece *parameter
selections* on a frozen contour (which of eight shapes; how wide; how long; which gait), not
operations that carry a stated idea through changed guises. Per the brief, weak development is a
*compounding* rather than *leading* cause of the rhythmic-feel complaint — but it is the reason two
same-archetype pieces are heard as "scalings of one tune" (motivic-derivation diagnostic §1).

---

## Axis 2 — Harmonic Vocabulary Beyond Diatonic

The engine selects a progression by **mode family** (warm / cool / neutral) with a random draw
(`pick_progression` `chord_engine.rs:122-166`; families in `mappings.json:42-59`), then decorates
it with gated chromatic insertions in `generate_chords` (`chord_engine.rs:191-314`). The audit is
whether the chromatic devices reach real images and whether they change *identity* or only
*decorate*.

| Principle | Engine location | Verdict | Justification |
|---|---|---|---|
| **Secondary dominants (V/x), tonicization** | `secondary_dominant_of` `chord_engine.rs:363-394`; gated `edge_activity ≥ 0.55` `mappings.json:29-31` | **SHALLOW-FAKED** | The device is correct (major/dom7 a fifth above the target's root), but the gate clears on essentially one corpus image — normalized `edge_activity` is ≥ 0.55 only for `example` (0.719); the rest sit 0.106–0.509 (motivic-derivation diagnostic §3). It also inserts a chord *before* an existing diatonic chord; it never re-plans function. One image in six gets one applied dominant. |
| **Modal interchange / borrowed chords** | `borrowed_minor_iv` `:396-429`, `flat_submediant` (bVI) `:431-441`; cool-family progressions embed bVII/bVI/bIII `mappings.json:50-54` | **SHALLOW-FAKED** | Borrowed iv fires only where the drawn progression literally contains `IV` **and** `brightness_drop ≥ 0.25` (`:33`, `chord_engine.rs:290-294`); bVI is *appended* only when `colorfulness > 0.45` (`:306-311`). Where they fire they are single decorative swaps/appends onto an otherwise-diatonic skeleton, applied identically — a fixed garnish, not a plural palette that changes the progression's felt identity. Cool-mode pieces get their chromatic color from the *pre-authored* progression string, not from interchange logic. |
| **Tension extensions (9/11/13), altered dominants** | `HarmonicComplexity` from `avg_saturation` `chord_engine.rs:231-234`; applied in `roman_to_chord_complex` `:340-355`; levels `mappings.json:11-15` | **SHALLOW-FAKED** | One complexity level is computed **once per piece** and applied to **every** chord uniformly — TriadsOnly, or +diatonic-7th, or +diatonic-9th. There are no 11ths/13ths and no altered dominants (b9/#9/#11/b13). "Every chord a 9th" is precisely the brief's *uniform-color* new-sameness: a constant extension level is still a constant. |
| **Chromatic approach / voice-leading chords** | — | **ABSENT** | No passing diminished sevenths, no line-cliché, no chromatic approach chords between structural chords. Harmonic motion happens only at the structural chords the progression names. |
| **Modulation / tonicization of a new center** | `key_scheme` default `home_only` `mappings.json:211-221`; excursion schemes gated high `:214-219` | **SHALLOW-FAKED** | Excursion schemes exist but every non-default rule requires `fg_bg_contrast ≥ 0.25` plus a high second threshold; five of six images fall to `home_only` and never modulate (fixed-rhythm-key diagnostic §3). The tonal center is static within and across most pieces. |
| **Reharmonization** (same melody, different harmony) | — | **ABSENT** | The subject is derived *from* the key (`theme_pitch` `chord_engine.rs:4208+`), not *set against* alternative harmonizations. There is no mechanism to re-harmonize a melodic idea, so the axis's own proof-of-concept — that identical melody can be made distinct purely by changing the harmony under it — is unavailable. |

**Axis 2 summary.** The harmonic identity of a piece reduces to: hue → mode → family →
*random* pick from ~5–7 mostly-diatonic progressions, plus rarely-firing decorative chromatics and
a uniform per-piece extension level, with no modulation on 5/6 images. Two families reach the corpus
(neutral never fires — vocabulary-breadth diagnostic §1 #5). The *devices* are individually correct;
the *system* keeps every piece in one diatonic-family sound-world. Per the brief this is a strong
**global-sameness** driver and a weak **rhythm-specific** one.

---

## Axis 4 — Groove / Feel / Rhythm  *(priority)*

The brief's load-bearing claim: **tempo scales the surface; it does not change it.** The
surface-defining variables are subdivision grid, straight-vs-swung feel, syncopation *pattern*,
density, meter, and groove skeleton — none of which is tempo. This is the axis the listener named.

| Principle | Engine location | Verdict | Justification |
|---|---|---|---|
| **Subdivision grid** (duple vs. triple/compound) | — (no such concept in `realize_rhythm` `chord_engine.rs:2225+`) | **ABSENT** | There is **no subdivision-grid switch anywhere**. A repository-wide search for swing/shuffle/subdivision/triplet/compound logic returns nothing in the rhythm code. Note values are a continuous articulation *length* lerp (`chord_engine.rs:2282-2302`) plus integer `dur_steps`; every piece lives on one implicit straight grid. This is the single largest determinant of rhythmic-surface identity, and it is fixed. |
| **Straight vs. swung feel (swing ratio)** | — | **ABSENT** | No swing ratio, no long–short subdivision skew, no shuffle. Every piece is dead-straight. There is not even a swing flag to toggle. |
| **Syncopation** (off-grid accent; pattern, not just amount) | `apply_motto_onset_bias` with `MOTTO_ONSET_BIAS_DEPTH = 0.18` `chord_engine.rs:2188`; cell 3 "profiled/syncopated" `:3621-3695` | **SHALLOW-FAKED** | The motto displaces onsets a *fixed 0.18* off-grid, **count-preserving**, using one cell fixed for the whole piece (`pick_piece_cell` `:3835`; onset bias biases *placement within the band the ladder already chose*). This is a single low, constant syncopation amount — exactly the brief's "pinned at one amount, misses the inverted-U" — and it carries no per-piece *pattern* of where offbeats fall. |
| **Rhythmic density / event rate** | band ladder (dotted/arp/sync/sustained) driven by `edge_activity` `chord_engine.rs:2256-2267`, `:2728-2783` | **SHALLOW-FAKED** | Density varies somewhat cross-piece (band coverage 4/6 distinct — fixed-rhythm-key diagnostic §2, layer 7), but it changes the *number of onsets inside the one grid*, not the grid; and within a piece one motto cell is held throughout (vocabulary-breadth diagnostic §1 #2), so density does not develop. |
| **Metric variety** (meter + accent template) | `Meter` enum `composition.rs:444-448`; **selector `meter` has empty rules** `mappings.json:171`; triple figurations flagged inert `composition.rs:3472` | **SHALLOW-FAKED** | The enum offers Four4/Three4/Six8/Two4, but the `meter` SelectTable is `{ "default": "four4", "rules": [] }` — **every image resolves to 4/4** with one backbeat/accent template. Triple/compound accompaniment figures are explicitly un-selectable "until a triple-meter knob exists." One meter, one metric identity. |
| **Groove / pattern vocabulary** (the recurring onset skeleton) | fixed phrase/cadence scaffold + unconditional pre-cadence acceleration `chord_engine.rs:2277-2280` + cadence ring; figuration catalogue `mappings.json:238` | **SHALLOW-FAKED** | The de-facto groove skeleton is the **structural** scaffold: a 4-step period `[S · · h/P]`, an unconditional harmonic-rhythm acceleration on the pre-cadence step, and a fixed cadence ring — stamped on every role of every piece (fixed-rhythm-key diagnostic §1–2; this is the skeleton a trained ear transcribed unprompted). The 9-figure accompaniment ostinato vocabulary that *could* diversify the groove fires on ~1/6 images (vocabulary-breadth diagnostic §1 #1). One skeleton, lightly ornamented. |

**The load-bearing test, executed against the code.** *Does anything change the rhythmic surface
other than tempo?* Only `character_tempo` (`mappings.json:151-158`) changes cross-piece timing —
it scales one straight, 4/4, fixed-subdivision, fixed-skeleton surface faster or slower. The motto
and band ladder **relocate and scale onsets within that one grid**; they never change the grid, the
feel, the meter, or the swing. **The brief's claim is confirmed by the code: tempo scales the
surface, it does not change it — and tempo is essentially the only surface control the engine
exposes.** This is the direct mechanism behind "varying tempo/cadence doesn't fix the sameness."

---

## Ranked Gaps (this audit's three axes), by contribution to cross-piece sameness

1. **Axis 4 — absence of a subdivision-grid + feel dimension (LEADING).** There is *no* duple/triple
   switch, *no* swing, *one* meter (4/4 by empty rule-set), *one* groove skeleton. The rhythmic
   surface is a single container and tempo is the only knob that rides on top of it. This is the
   literal, code-level source of "the same common rhythm across every piece."
2. **Axis 4 — one groove skeleton + one fixed syncopation amount.** Even holding subdivision aside,
   the phrase/cadence scaffold and the fixed-0.18 count-preserving onset bias give every piece the
   same skeletal gait. *Important negative result:* varying the *cadence gesture* on this fixed grid
   was tried and ear-rejected (it fragmented lines without touching the sameness) — consistent with
   the brief's warning that relocating events within a fixed surface is not a surface change.
3. **Axis 1 — no developing variation.** Restatement + head-truncation only; inversion, retrograde,
   and augmentation-as-variation are declared enum variants clamped off. This collapses the melodic
   realization space so same-archetype pieces read as scalings of one tune (compounding, not
   leading, for the *rhythmic* complaint).
4. **Axis 2 — one diatonic-family sound-world per piece.** Random pick from a small mostly-diatonic
   set, uniform per-piece extension level, rarely-firing chromatics, no modulation (5/6), no
   reharmonization. Strong *global* sameness, weak *rhythmic* sameness.

---

## Single Highest-Leverage Recommendation (this audit's axes)

**The one architectural dimension whose absence most causes the "sameness" verdict is a
SUBDIVISION-GRID / FEEL vocabulary on Axis 4.** The engine today has exactly one rhythmic *surface*
— straight, 4/4, fixed subdivision, one groove skeleton — and everything the operator has varied
across the recent single-lever work (tempo, phrase period, cadence placement, cadence-gesture type)
is *content that rides inside that one surface*. The brief ranks this axis first for a complaint
worded as rhythmic feel, the listener's own words name it, and the code confirms tempo is the sole
surface control. It is therefore the dimension with the largest marginal return.

**What adding it means, in theory terms (not code).** Give the system a small, curated **vocabulary
of feels**, where a "feel" is a *coherent bundle*, not a single knob:

- a **subdivision policy** — the beat felt in duple (eighths/sixteenths) *or* triple/compound
  (triplets, 6/8) — this is the largest single determinant of rhythmic-surface identity;
- a **straight-or-swung setting** with a defined swing ratio when swung;
- a **meter + backbeat/accent template** (actually reaching the dormant Three4/Six8/Two4, whose
  enum already exists) — which also un-locks the currently-inert triple-meter accompaniment figures;
- a characteristic **groove/onset skeleton** for that feel, replacing the single universal scaffold
  as the piece's rhythmic ostinato;
- **syncopation as a per-feel pattern at a variable amount**, aimed at the middle of the pleasure
  inverted-U rather than pinned at one low fixed displacement.

Two pieces at the *same tempo* must then be able to present audibly different rhythmic surfaces —
that is the test the current engine fails. Crucially this is **orthogonal to, not a repeat of, the
cadence-gesture lever that was rejected**: cadence-gesture variation relocated events *within* the
one grid (and fragmented lines); a feel/subdivision vocabulary changes *what the grid is*. Selection
must be **deterministic from an image feature** over a **curated, authored** feel set — never from a
hash or RNG — so the pieces stay reproducible and every feel is a hand-vetted, musically-coherent
bundle rather than a synthesized surface. (A companion improvement on Axis 1 — un-clamping the
inversion/retrograde/augmentation variants into a genuine developing-variation trajectory — is the
second-most-leverage move, but it compounds the rhythmic fix rather than leading it.)

---

## Hypothesis under test — verdict

> *"The engine implements a handful of principles shallowly + uniformly → every piece = one
> archetype with parameter tweaks."*

**CONFIRMED on all three audited axes.** The engine possesses one contour vocabulary (eight fixed
shapes, no transformation), one gait cell fixed per piece, one diatonic-family progression with a
uniform per-piece extension level, and — decisively — **one rhythmic surface**: straight, 4/4, one
subdivision, one groove skeleton. Every dimension that would create a *new perceptual class* is
either **absent** (subdivision grid, swing, chromatic-approach harmony, modulation on 5/6,
reharmonization, motivic development) or **declared-but-locked-off** (the `Inverted`/`Retrograde`/
`Augmented` variation variants; the Three4/Six8/Two4 meters behind an empty rule-set). What varies
between pieces is a set of **parameter values on one architecture** — which contour, how wide, how
long, which gait cell, which random progression, which tempo, where the cadence sits. Per the
brief's synthesis, that moves each piece *within a single perceptual equivalence class*; the pieces
differ measurably yet cluster tightly — the operational definition of "they all sound the same." The
escape is categorical, not quantitative: the architecture must gain developmental/vocabulary
*dimensions*, and the highest-return one on these axes is the missing groove/feel/subdivision
dimension.

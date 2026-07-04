# Audit S66 — Composition & Songwriting Aesthetics (the gestalt / trained-ear lens)

**Lens:** Composition & Songwriting Aesthetics — the craft of form, key plan, pacing,
contrast, and resolution: whether a set of pieces *sound good and feel like distinct,
satisfying wholes* to a trained ear. **This is a READ-ONLY audit — no code is modified.**
`engine.rs` is frozen and untouched.

**Grading standard:** `docs/research-s66-sameness-principles.md` (the cited five-axis
principle brief). Each principle on my axes is graded against the engine.

**My axes:** Axis 3 (Form + harmonic-rhythm variety, PRIMARY), Axis 5 (Arrangement /
orchestration arc, PRIMARY), Axis 4 (Groove / feel, from the LISTENER / gestalt angle —
the code mechanics belong to the theory lens; I judge the *perceptual* result).

**Corpus:** the six bundled images, real `CompositionPlanner::plan` path. Numbers cited
below are from the standing diagnostics (`diagnostic-s63-vocabulary-breadth.md`,
`diagnostic-s64-fixed-rhythm-key.md`, `design-s63-holistic-distinctness-aesthetics.md`) and
from direct reading of `src/composition.rs`, `src/chord_engine.rs`, and `assets/mappings.json`.

**The one sentence that frames the whole audit** (from the grading brief): *perceived
musical identity is carried by the STRUCTURE (the container), not by parameter values (the
content); re-rolling values on a fixed container changes the notes but not the felt
identity.* Five sessions of single-lever work re-rolled content (pitch, tempo, which cell,
where the cadence sits). The container the ear tracks has barely moved.

---

## AXIS 3 — Form + Harmonic-Rhythm Variety (PRIMARY)

### Principle 3a — Phrase-length variety (4-bar/8-bar are defaults to be *varied*)

**Engine location:** `chord_engine.rs:579` `PHRASE_LENGTHS = [4,8]`; the S64 spread lives at
`chord_engine.rs:599` `enum PhraseScheme` (Period4 / Sentence8 / Phrase5 / Group6),
`:668 phrase_len_for`, `:858 plan_phrases_scheme`; selected by `mappings.json:374
phrase_scheme` over complexity + edge_activity.

**Verdict: SHALLOW-FAKED (partially remediated, still narrow).** Before S64 this was ABSENT —
`plan_phrases` collapsed every section to a 4-step period on the whole corpus (the
`S..h S..P` skeleton a trained ear transcribed unprompted, `diagnostic-s64` §1a). S64 added
a genuine curated menu of four real archetypes and reached all four across the corpus
(Img1/Img2/magic → period_4, Lena → phrase_5, Img3 → group_6, example → sentence_8). That is
real, honest phrase-length variety and it is authored, not random — credit where due. But it
remains SHALLOW for two reasons the ear will find: (1) **3 of 6 pieces still resolve to the
legacy `period_4`**, so half the corpus still shares the exact cadence periodicity; (2) the
schemes vary only the *nominal phrase length* fed to one unchanged partition-and-cadence
stamper — the *within-phrase* content (interior run + pre-cadence lift + cadence ring) is
identical across schemes. It is a real but thin spread on one sub-parameter of the container,
not plural phrase *behavior*.

### Principle 3b — Formal variety (form itself is a variable, not one template)

**Engine location:** `mappings.json:80 form_catalogue` (6 authored forms), `:110 form` rules;
selector `composition.rs:1457 form.select(u)` (once per piece).

**Verdict: SHALLOW-FAKED (rich catalogue, dead selector).** Six genuine forms are authored
(rounded_binary, ternary_aba, aaba, abac, abbac, theme_and_variations) — a good vocabulary.
But every non-default rule is gated above the corpus's observed feature range
(`ternary_aba` needs `quadrant_contrast ≥ 0.6` vs corpus max 0.253; `abac` needs
`vertical_emphasis ≥ 0.6` vs max 0.383; `diagnostic-s64` §4), so **5 of 6 pieces fall to
`rounded_binary`** and the sixth to `theme_and_variations`. Two forms reached out of six;
the macro shape is a near-constant. Critically — and this is the load-bearing finding for my
axes — `diagnostic-s64` §4 *proved* that spreading the form selector will **not** break the
perceived sameness: `example.jpg` is the one image that escapes to a different form, yet its
sections still realize the same `S..h S..P` phrase scaffold, because every section of every
form is re-partitioned into the same periods downstream. Form is a real gap for *macro*
variety but a weak lever against the gestalt sameness. Do not spend the primary fix here.

### Principle 3c — Harmonic rhythm as an independent variable (rate of chord change)

**Engine location:** `composition.rs:1536 steps_per_section = 8 + round(edge_activity*8)`;
`:1714 generate_chords` produces one chord per step; `:1737 plan_phrases_scheme` emits one
`StepPlan` per chord. There is **no** chords-per-step rate parameter anywhere.

**Verdict: ABSENT.** This is the single largest untouched wall on my axes. Every piece, every
section, changes harmony at exactly **one chord per step** — a flat, uniform harmonic tempo
from the first bar to the last, identical across all six pieces. The research (`Axis 3`,
principle 1) names a fixed harmonic-rhythm rate "one of the most insidious sameness sources
because it is rarely noticed consciously yet strongly shapes perceived pace." It is *never*
noticed by name and it is *completely* constant here. Five sessions varied tempo, notes, and
cadence position — all *content inside* this container — and never touched the rate the chords
themselves move, which is the container. Two pieces at different tempos with the same one-per-
step harmonic rhythm are heard as the same piece played faster or slower — which is precisely
the operator's standing verdict.

### Principle 3d — Harmonic-rhythm *variation within a piece* (acceleration toward cadence)

**Engine location:** the cadence machinery is `plan_phrases_scheme` (`chord_engine.rs:858`) +
the pre-cadence gesture at `chord_engine.rs:2122-2125` and the cadence ring at `:2252`.

**Verdict: ABSENT (a rhythmic-onset fake stands in for it).** Common practice accelerates the
*chord-change rate* approaching a cadence, so a phrase *arrives*. The engine has no such
acceleration: the chord rate is one-per-step through the cadence too. What fires instead
(`diagnostic-s64` §1b) is a *melodic-onset* acceleration on the second-to-last step (an
arpeggio run) plus a sustained "cadence ring" — a surface subdivision flourish, not a
harmonic-pace change, and it is stamped *identically* on every phrase of every piece. So the
directional "arrival" cue the ear most wants is missing, and `diagnostic-s64` §2 confirms the
audible consequence: phrases "do not *arrive* — they just stop and restart, which reads as
loopiness." This is the most *pleasing*-facing gap in the whole audit: its absence is not
just sameness, it is the absence of satisfaction.

### Principle 3e — Modulation / key-plan variety (departure and return)

**Engine location:** `mappings.json:172 key_scheme_catalogue` (9 schemes), `:211 key_scheme`
rules; resolved once at `composition.rs:1525 resolve_key_scheme`.

**Verdict: SHALLOW-FAKED (mostly static).** The home root is genuinely per-image (5 distinct
pitch classes, `diagnostic-s64` §3 — good, and not the sameness cause). But intra-piece
modulation fires `home_only` on 5 of 6 images, so **5 of 6 pieces never leave the home key**:
the "departure and return" that gives a piece a satisfying distinct tonal shape is absent for
almost the whole corpus. `diagnostic-s64` §3 also shows the six home roots cluster into a
4-semitone band (A3–C#4 on 5/6), so even the cross-piece home variety is compressed into one
tonal register. Real but secondary — a colorist garnish that is mostly switched off.

---

## AXIS 5 — Arrangement / Orchestration Arc (PRIMARY)

### Principle 5a — Voice roles (which roles present, how assigned)

**Engine location:** `composition.rs:487 OrchestrationProfile.layers`; texture profiles at
`mappings.json:222 texture_catalogue`; selected once per piece at `composition.rs:1586
texture.select(u)` and `.clone()`d onto every section (`:1762`).

**Verdict: SHALLOW-FAKED (one fixed role layout per piece, near-constant across the corpus).**
Every reachable profile carries the *same* role stack — Bass + Pad + (HarmonicFill or
CounterMelody) + Melody — always melody-on-top / comp-in-the-middle / bass-underneath. The
role *layout* never changes within a piece and barely changes across pieces (the only swing
is HarmonicFill vs CounterMelody). The research names exactly this — "a fixed role assignment
(always melody-over-block-chords-over-bass) is a textural constant." The S63 integration slice
made the roles *interlock* rhythmically (a real, credited win), but interlock is a
relationship *within* the fixed layout; it did not add role-layout variety.

### Principle 5b — Textural contrast *between sections*

**Engine location:** the only within-piece texture motion is the figuration arc at
`composition.rs:1746-1773` / `:2026 section_figuration_id`, plus a small per-section density
number at `:1743`.

**Verdict: SHALLOW-FAKED.** There is exactly one mechanism of sectional contrast: on
Contrast/Development sections the Pad's figuration cell swaps to a different onset-density
class (block↔broken), then returns on the Return. That is a genuine, tasteful A-B-A′ texture
gesture — but it is a *cell swap inside one fixed ensemble at one fixed voice count*. There is
no thinning to solo/duo, no thickening to tutti, no homophonic↔linear shift, no material moved
between registers — the arranging techniques the research names as what "makes a form feel
shaped rather than uniform." The texture at the opening, the middle, and the end are the same
weight; only the arpeggiation pattern flickers.

### Principle 5c — Instrumentation / orchestration variety

**Engine location:** `mappings.json:222 texture_catalogue` (13 profiles); `diagnostic-s63`
census; texture selector `mappings.json:298 texture` rules.

**Verdict: SHALLOW-FAKED (13-profile catalogue collapses to ~1 color).** `diagnostic-s63`
found 10 of 13 texture profiles unreachable on the corpus; after the S63 integration re-route,
**4 of 6 pieces land on the same `arp_waltz` base color**, the other two on broken-chord
variants. The three richer profiles (`pad_figured`, `pad_block_comp`, `pad_stride`) carry
thresholds *above the entire observed calm-corpus range* (e.g. `pad_block_comp` needs
`arousal ≥ 0.70`, `pad_stride` needs `arousal ≥ 0.75`; corpus arousal maxes ≈ 0.39), so they
never fire. The research calls fixed instrumentation "arguably the *most immediately audible*"
sameness source, "since timbre is registered pre-attentively and instantly." Four of six
pieces present the same arpeggiated wash. (The S63 gate-close already flagged this as the
next-in-line ear complaint: "Img1/2/3/Lena are all the same arpeggiated wash.")

### Principle 5d — Dynamic and textural ARC over time (build / climax / release)

**Engine location:** per-section density is set at `composition.rs:1743` from source-region
energy; there is no piece-level trajectory that rises to a climax and recedes.

**Verdict: ABSENT.** This is the second-largest untouched wall on my axes and the direct twin
of the harmonic-rhythm gap. A piece with a static homogeneous texture "has no arc; a piece
whose texture *develops* has a narrative" (Axis 5). Every piece here holds one intensity level
start to end — the density number wobbles slightly per section from region energy, but there
is no authored rise-to-a-peak-and-return, no structural climax, no arrival of *weight* to pair
with the (also-absent) harmonic arrival of 3d. Every piece is a level plateau. `design-s63`
Rank 4 named this directly: "Pieces don't *breathe* — no build, no thinning, no arrival."

---

## AXIS 4 — Groove / Feel (LISTENER / gestalt angle)

*The theory lens audits the code mechanics of these; I record the perceptual result, because
the operator's five-session verdict is a rhythm-feel verdict and the gestalt question is mine.*

### Principle 4a — Subdivision grid (duple vs triple/compound) — the single largest feel driver

**Engine location:** onset placement is straight fractional (`figuration_catalogue` onsets at
0/.25/.5/.75 or 0/.33/.66, `mappings.json:238`); the melody's per-step subdivision bands are
Sustained/Oblique(dotted)/Subdividing at `chord_engine.rs:1264-1305`.

**Perceptual verdict: ABSENT as a selectable feel.** There is no piece-level choice of a
subdivision grid: every piece is felt in the same straight duple subdivision. The bands vary
*how many* onsets a given step gets, but never the *grid those onsets sit on*. The research
names this "the single largest determinant of rhythmic-surface identity" — and it is a
constant. To the ear this is a large part of why "the tempo is varied but the rhythm is the
same": tempo scales a fixed grid; it does not change the grid.

### Principle 4b — Straight vs swung feel (swing ratio)

**Engine location:** searched — no swing-ratio / straight-vs-swung selection exists. The word
"swing" appears only as a density label (`chord_engine.rs:2147`) and as one cell's lopsided
long-short shape (`:3682`), neither a global feel axis.

**Perceptual verdict: ABSENT.** Every piece is straight. A swung feel over the identical tempo
and harmony is heard as a *different rhythm*; that categorical lever is not present.

### Principle 4c — Metric variety (time signature + accent template)

**Engine location:** `Meter` enum has Four4/Three4/Six8/Two4 (`composition.rs:444`) and
`parse_meter` handles all four (`:2370`) — **but** `mappings.json:171 meter` is
`{default: four4, rules: []}`.

**Perceptual verdict: SHALLOW-FAKED (machinery present, selector dead).** The vocabulary
exists in code and is wired to parse, yet the selector has zero rules, so **every piece is
4/4**. A catalogue locked to one meter with one accent template "has one metric identity." The
gate is dead exactly like the form gate.

### Principle 4d — Groove/pattern skeleton + syncopation

**Engine location:** the per-piece motto (`composition.rs:1944 pick_rhythm_cell`,
`chord_engine.rs:3774 RhythmMotto`) and the fixed cadence scaffold (accel `:2122` + ring
`:2252`).

**Perceptual verdict: SHALLOW-FAKED.** The per-piece motto is a real, healthy cross-piece
gait carrier (4/4 cells fire, `diagnostic-s63` #2) and the S64 phrase-scheme moved the cadence
periodicity somewhat. But the *groove skeleton* — the accel-into-ring cadence gesture — is
still stamped on every phrase, and syncopation is pinned to whatever the one motto cell
carries (no independent syncopation-*pattern* axis, and no acknowledgement of the research's
inverted-U: medium syncopation is the pleasure sweet spot, and nothing targets it). The single
attempt to vary the cadence *gesture type* directly (the S65 lever) was ear-rejected for
fragmenting the melodic line — an important constraint for the recommendation below.

---

## (1) THE GESTALT VERDICT — one perceptual equivalence class, and the strongest wall

**Yes. The six pieces occupy one perceptual equivalence class.** This is the operational
definition of "they all sound the same," and the engine satisfies it precisely: the pieces
differ measurably in the *content* variables five sessions have worked (pitch motif, tempo,
home root, which figuration cell, which phrase length, which cadence position) yet cluster
tightly on every *container* variable the ear uses to categorize a piece. Concretely, all six
share: one meter (4/4), one straight subdivision feel, one flat harmonic rhythm (one chord per
step, never accelerating), one flat intensity plateau (no build, no climax, no arrival), one
role layout (melody/comp/bass), and — on 4 of 6 — one texture color (the arpeggiated wash).
Re-rolling content inside that shared container moves each piece *within the class*; it cannot
move a piece *out of* it, because the class boundary is the container, not the content. That is
the exact mechanism of "varying the tempo doesn't fix it."

**The strongest wall of the class, on my axes:** the **flat, arc-less temporal-and-intensity
container** — every piece is the *same macro-shape*, a level plateau with a fixed harmonic pace
that never rises, drives, or arrives. I rank this the strongest wall my lens owns because it is
(a) *within-piece* constant (so it defines the shape of each piece) *and* (b) *cross-piece*
constant (so it is shared by all six), and (c) it is the shape variable the ear tracks as "the
form of the music" beneath melody and harmony. Its two faces are harmonic-rhythm (Axis 3c/3d,
ABSENT) and textural/dynamic arc (Axis 5d, ABSENT) — the two things that, in real music,
*shape a piece over time*, and here do not exist at all.

I record honestly that the theory lens owns a **co-dominant** wall I cannot fix from my seat:
the **subdivision-grid + meter feel** (Axis 4a/4c), which the grading brief ranks the #1 cause
of a rhythm-specific complaint. From the pure gestalt angle these are equally load-bearing;
the division of labor is that the *feel-surface* wall is the theory lens's mechanical fix and
the *macro-shape* wall is mine. Both must fall for the class to break; my recommendation
targets the one my axes can most pleasingly move.

## (2) RANKED gaps on my axes, by contribution to the sameness

1. **Harmonic rhythm is a flat constant (Axis 3c, ABSENT) + no acceleration toward cadence
   (Axis 3d, ABSENT).** The largest untouched container variable. One chord per step, every
   piece, never arriving. Highest rank because it is both a cross-piece constant *and* a
   within-piece constant *and* the direct cause of the "phrases stop rather than arrive"
   loopiness — it is a sameness cause and a dissatisfaction cause at once.
2. **No dynamic / textural ARC (Axis 5d, ABSENT).** Every piece is a level plateau; nothing
   builds or breathes. The macro-shape twin of #1; together they are "the piece has no shape."
3. **Instrumentation collapses to one color (Axis 5c, SHALLOW-FAKED — 4/6 the same wash).**
   The *most immediately noticeable* sameness (timbre registers first), but a routing/breadth
   problem more than a structural one, so ranked below the two absent structural dimensions.
4. **Form collapses to one template (Axis 3b, SHALLOW-FAKED — 5/6 rounded_binary).** A real
   macro-variety gap, but *proven* (S64 §4) to leave the felt skeleton intact — low leverage
   against the gestalt.
5. **Modulation mostly off (Axis 3e, SHALLOW-FAKED — 5/6 home_only).** A tonal-shape garnish
   that is switched off for most of the corpus; secondary.
6. **Phrase-length spread still narrow (Axis 3a, SHALLOW-FAKED — 3/6 still period_4).** The
   partially-landed S64 win; extend it, but it is a sub-parameter, not a new dimension.

## (3) THE SINGLE HIGHEST-LEVERAGE RECOMMENDATION — add a per-piece DRAMATIC ARC, anchored on shaped harmonic rhythm

**The one added structural dimension: a per-piece intensity/pace ARC — a shaped rise-and-return
in HARMONIC-RHYTHM RATE (Axis 3, the load-bearing core) reinforced by TEXTURAL DENSITY
(Axis 5, the visible amplifier) — so each piece has a distinct macro-shape (settle → build →
arrive → return) instead of a flat plateau.** The essential, must-ship core is the harmonic-
rhythm shaping; the texture-density arc rides on the same shape and multiplies the effect.

**Why this is the highest-leverage fix on my axes — the four tests the task sets:**

- **It multiplies perceptual equivalence classes.** Harmonic-rhythm rate is *orthogonal to
  every variable the five prior sessions touched* — pitch, tempo, key, cadence position,
  figuration cell. Two pieces with identical form, key, tempo, and motif but different
  harmonic pace (one broad and slow-changing, one urgent and accelerating) are heard as
  *different pieces*, because rate is a container variable and the ear categorizes by
  container. Adding it opens a genuinely new axis of between-piece distance — the thing the
  grading brief says actually escapes the class, as opposed to re-rolling content inside it.
  Coupling the texture-density arc to the same seed adds a second, reinforcing container axis
  at near-zero extra risk. This is the single change on my axes that adds *classes*, not
  *resolution within a class*.

- **It produces a MORE pleasing listen, not merely a different one.** This is the decisive
  point over the other candidates. Acceleration of harmonic rhythm into the cadence is *the*
  classic "arrival" gesture of tonal music, and its absence is exactly what `diagnostic-s64`
  §2 diagnosed as the loopiness ("phrases do not arrive — they just stop and restart"). Adding
  it does not just diversify — it makes phrases *land*, which is the most reliable single
  source of listener satisfaction in a short piece. A per-piece build-to-a-climax gives the
  same payoff at the macro level: the piece acquires a *narrative* (departure and return of
  intensity) instead of a plateau. Distinctness and satisfaction come from the same lever.

- **It cannot repeat the S65 fragmentation regression — it is coherence-safe by
  construction.** The S65 lesson is that variety which cuts the melodic line is a regression
  (the cadence-gesture-type variation "fragmented the lines… jarring cuts, silent breaks").
  Harmonic-rhythm shaping is structurally incapable of that failure: it operates on the *rate
  the chord layer changes underneath a continuous melodic line* — it re-paces the harmony
  beneath the tune, it never chops the tune. Likewise a textural density arc adds or removes
  *accompaniment weight around* the melody; the line flows on. Both act *under and around* the
  melodic line, not *on* it. This is why I recommend a pacing/intensity dimension rather than
  another surface-gesture dimension: it is the one high-leverage container variable that is
  inherently line-preserving.

- **It obeys the BALANCE LAW.** Implement as a small **curated menu of harmonic-rhythm
  profiles** — authored, common-practice shapes such as *broad* (one chord held across two
  steps, a calm/spacious pace), *normative* (the current one-per-step, the freeze default),
  and *driving/accelerando* (one-per-step opening that doubles to a faster change into each
  cadence) — **selected per image by an existing feature** (edge_activity or the arousal
  composite, the same knobs the form and phrase gates already read), fully deterministic, no
  RNG. The texture-density arc is the same: a curated rise-return envelope over the existing
  per-section density number, seeded by the same feature. Curated menu + feature seed +
  deterministic = exactly the doctrine that S62/S63/S64 established, and the legacy default
  ("normative", flat) preserves the frozen identity path byte-for-byte.

**Why this beats the other candidates on my axes.** Form-spread (Axis 3b) is *proven* weak
against the gestalt (S64 §4: form changes but the skeleton survives). Texture-color routing
(Axis 5c) is the most *noticeable* gap but is a breadth/routing fix the doctrine sequences
last — un-gating the three dead profiles gives a bigger stack of the same shape, not a new
dimension. Modulation and phrase-length are secondary garnish/sub-parameters. Harmonic-rhythm
is the *one absent container dimension on my axes that is simultaneously high-leverage,
intrinsically pleasing (arrival), and structurally coherence-safe* — the exact profile the
task asks for.

**Dependencies flagged for the theory lens (this design owns the shape; theory owns the
legality):** shaping harmonic rhythm means the chord layer must supply or hold chords at the
chosen rate — a *held* chord (broad profile) and a *doubled* change into a cadence both need
the voice-leading and cadential-approach to stay legal (the acceleration's added chord wants a
proper pre-dominant → dominant → tonic approach, not an arbitrary insert). The macro-climax's
harmonic and registral peak must also stay in-key and resolve home. My lens specifies *where*
the pace broadens and accelerates and *where* the climax lands; the theory lens keeps the held
chords, the inserted pre-cadential chords, and the peak voice-leading-legal and cleanly
resolved. And the co-dominant feel-surface wall (subdivision grid + meter, Axis 4a/4c) remains
the theory lens's parallel fix — the class breaks fastest if the macro-shape dimension (mine)
and a subdivision/meter feel dimension (theirs) land together.

---

*Public-repo note: role-language only; no codenames. Session tags (S62–S66) reference the
existing public diagnostic docs in this directory.*

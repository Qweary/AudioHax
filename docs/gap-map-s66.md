# Gap Map S66 — Unified Cross-Piece Sameness Diagnosis

**Status: diagnosis of record.** This document merges three read-only audit lenses —
music-theory (Axes 1, 2, 4), composition/aesthetics (Axes 3, 5, and the gestalt of 4), and
perceptual/affect (the image→character input) — against the cited principle brief
`docs/research-s66-sameness-principles.md`. It supersedes the per-lens gap lists as the single
ranked picture and names **one** recommended fundamental fix for the next build, with a real
alternative and a process fix. **No code is modified. `src/engine.rs` is frozen and untouched.**

The one sentence that governs the whole diagnosis, from the brief: *perceived musical identity
is carried by the STRUCTURE (the temporal container), not by the parameter VALUES (the content);
re-rolling values on a fixed container changes the notes but not the felt identity.* Five prior
sessions re-rolled content — pitch motif, tempo, home key, which figuration cell, which phrase
length, where the cadence sits. Every one of those is a value inside a container the ear tracks
as the piece. The container has barely moved. That is why "varying the tempo doesn't fix it,"
stated by the operator's ear and confirmed independently by all three lenses at code level.

---

## 1. Unified Ranked Gap Map

Merged across the three lenses and deduplicated (the Axis 4 feel-surface gaps that the theory
lens audits mechanically and the aesthetics lens audits perceptually are folded into single
rows). **Verdict** uses the brief's grades: IMPLEMENTED / SHALLOW-FAKED (present by name but
applied so uniformly it does not register as variety) / ABSENT (or ABSENT-LOCKED, meaning the
capability is declared in an enum but pinned off). **Contribution** to the *rhythm-feel /
sameness* verdict: LEADING / COMPOUNDING / MINOR.

| # | Gap | Axis | Engine location | Verdict | Contribution |
|---|---|---|---|---|---|
| 1 | **No subdivision-grid / feel surface** — no duple-vs-triple switch, no compound meter, one implicit straight grid for every piece | 4 | `chord_engine.rs:2225 realize_rhythm`; melody bands `:1264-1305`; figuration onsets `mappings.json:238` | **ABSENT** | **LEADING** |
| 2 | **No straight-vs-swung feel** — no swing ratio, no long-short skew; every piece dead-straight; not even a swing flag | 4 | (searched; absent — "swing" is only a density label `chord_engine.rs:2147` and one cell's shape `:3682`) | **ABSENT** | **LEADING** |
| 3 | **Harmonic-rhythm rate is a flat constant** — exactly one chord per step, every section, every piece, never varying between pieces | 3 | `composition.rs:1536 steps_per_section`, `:1714 generate_chords` (one chord/step), `:1737` one `StepPlan`/chord | **ABSENT** | **LEADING** |
| 4 | **No harmonic-rhythm acceleration toward cadence / no arrival** — phrases stop and restart rather than land; a melodic-onset flourish fakes it | 3 | cadence machinery `chord_engine.rs:858 plan_phrases_scheme` + pre-cadence gesture `:2122-2125` + ring `:2252` | **ABSENT** | **LEADING** (sameness *and* dissatisfaction) |
| 5 | **No dynamic / textural ARC** — every piece is a level intensity plateau; density wobbles per-section from region energy but never builds to a climax and recedes | 5 | per-section density `composition.rs:1743`; no piece-level trajectory | **ABSENT** | **LEADING** (macro-shape twin of #3) |
| 6 | **Meter locked to 4/4** — Four4/Three4/Six8/Two4 enum exists and parses, but the selector rule-set is empty, so one meter + one accent template for the whole corpus; triple-meter figures inert | 4 | `Meter` enum `composition.rs:444-448`; `parse_meter :2370`; selector `mappings.json:171 {default:four4, rules:[]}` | **SHALLOW-FAKED** (machinery live, selector dead) | **COMPOUNDING** |
| 7 | **One groove/onset skeleton + one fixed low syncopation** — a 4-step period, an unconditional pre-cadence accel and a fixed cadence ring stamped on every role of every piece; onset bias pinned at a constant 0.18, count-preserving, no per-piece pattern | 4 | motto `chord_engine.rs:3774 RhythmMotto`, `:3835 pick_piece_cell`, bias `:2188`; cadence scaffold `:2122`+`:2252` | **SHALLOW-FAKED** | **COMPOUNDING** |
| 8 | **Instrumentation collapses to ~one color** — 13 texture profiles, but 4 of 6 pieces land on the same arpeggiated wash; the three rich beds gate above the corpus's arousal range | 5 | `mappings.json:222 texture_catalogue`, `:298 texture` rules; selected `composition.rs:1586`, cloned per section `:1762` | **SHALLOW-FAKED** | **COMPOUNDING** (most *noticeable*, but routing/breadth) |
| 9 | **No developing variation** — the motive is restated (hold final note) or head-truncated then rested; inversion, retrograde, augmentation-as-variation are declared enum variants clamped off | 1 | `ThemeVariation` `composition.rs:414-428`, clamped to {Identity,Fragmented} `:1968-1973`; theme arms `chord_engine.rs:4110-4152` | **ABSENT-LOCKED** | **COMPOUNDING** |
| 10 | **One fixed role layout** — melody-over-comp-over-bass on every piece; only inner-voice swing is HarmonicFill↔CounterMelody | 5 | `OrchestrationProfile.layers` `composition.rs:487`; `texture.select :1586` | **SHALLOW-FAKED** | **COMPOUNDING** |
| 11 | **Form collapses to one template** — 6 authored forms, but 5 of 6 pieces fall to `rounded_binary`; *proven* to leave the felt skeleton intact when spread | 3 | `mappings.json:80 form_catalogue`, `:110 form`; select `composition.rs:1457` | **SHALLOW-FAKED** | **MINOR** (low leverage against the gestalt) |
| 12 | **One diatonic-family harmonic sound-world** — random pick from ~5-7 mostly-diatonic progressions, a uniform per-piece extension level, rarely-firing chromatics, no chromatic-approach chords, no reharmonization | 2 | `chord_engine.rs:122 pick_progression`, `:191 generate_chords`; secondary-dom `:363`, borrowed iv `:396`, bVI `:431` (all gated high) | **SHALLOW-FAKED** / **ABSENT** (approach chords, reharm) | **MINOR** for rhythm; strong for *global* sameness |
| 13 | **Modulation off** — `home_only` on 5 of 6 images; no departure-and-return of tonal center | 3 | `mappings.json:172 key_scheme_catalogue`, `:211` rules; `composition.rs:1525 resolve_key_scheme` | **SHALLOW-FAKED** | **MINOR** |
| 14 | **Phrase-length spread still narrow** — 4 curated schemes reach the corpus, but 3 of 6 still resolve to `period_4`; within-phrase content identical across schemes | 3 | `chord_engine.rs:599 PhraseScheme`, `:668 phrase_len_for`, `:858`; `mappings.json:374` | **SHALLOW-FAKED** | **MINOR** (sub-parameter, not a new dimension) |
| 15 | **Arousal input compressed** — 4 of 6 images bunch in a 0.016-wide arousal band; the high-energy textures are unreachable by any natural photo | affect (input) | `composition.rs affect_composite`; arousal weights sat 0.45 + colorfulness 0.25 + edge 0.20 + complexity 0.10 | **SHALLOW-FAKED** (input-side) | **MINOR** (companion, not root) |

**Reading of the map.** The five LEADING rows are not five separate problems. They are **two
walls**, each with two faces:

- **Wall A — the fixed feel surface** (rows 1, 2, 6, 7): one straight subdivision grid, no swing,
  one meter, one groove skeleton. This is the *rhythmic surface* the operator's ear named. The
  brief ranks it #1 for a rhythm-worded complaint; the code confirms tempo is the only surface
  control the engine exposes (`character_tempo mappings.json:151` scales one fixed grid).
- **Wall B — the flat temporal-and-intensity container** (rows 3, 4, 5): one chord per step
  forever, no acceleration into cadence, no build-to-climax. This is the *macro-shape* — the pace
  and arc the ear tracks as "the form of the music." It is both a within-piece constant (so it
  defines each piece's shape) and a cross-piece constant (so all six share it), and its absence is
  *both* a sameness cause and a dissatisfaction cause (phrases don't arrive; pieces don't breathe).

The affect lens's independent finding closes off the cheap escape: the input knobs *are* turning
(4 distinct characters, 66-132 BPM, valence owns mode), so the sameness is **not** input-collapse
— it is architecture-side, in Walls A and B, exactly where the two code lenses point. Rows 9-14
(development, harmony, form, modulation) are real but COMPOUNDING/MINOR — they rhyme the surfaces
without leading the rhythm-feel verdict, and the brief's adversarial check warns against spending
the single pointed build on them.

---

## 2. The Adjudication — the one fundamental fix for the next build

The two code lenses each recommend a different architecture-side dimension, and they are
genuinely complementary — one per wall:

- **Candidate A — Groove/feel subdivision-grid vocabulary** (music-theory lens, Wall A, Axis 4):
  a curated menu of *feels*, each a coherent bundle {subdivision duple/triple · straight-or-swung
  with a defined ratio · meter+backbeat reaching the dormant Three4/Six8/Two4 · a groove skeleton
  replacing the single universal scaffold · syncopation as a per-feel pattern at the pleasure
  inverted-U middle}. This is the literal answer to the operator's "same rhythm" words and the
  brief's #1-ranked axis.
- **Candidate B — Harmonic-rhythm dramatic-arc + textural density** (aesthetics lens, Wall B,
  Axes 3/5): a curated menu of harmonic-rhythm profiles (*broad* / *normative* / *driving-
  accelerando*) selected per image, giving each piece a shaped rise-and-return — settle → build →
  arrive → return — reinforced by a texture-density envelope, replacing the flat plateau. This is
  the answer to the loopiness and the flat container, and it claims structural fragmentation
  immunity.

Adjudicated on the four axes the task sets:

**(i) Leverage on the specific "sameness / rhythm-feel" verdict.**
Candidate A wins the *literal* wording — the listener said "rhythm," the brief ranks feel-surface
#1, and A is the only fix that changes what the ear hears *as the rhythm* (the grid, the swing,
the meter). Candidate B has *broad* leverage on the sameness gestalt and a unique second payoff —
it makes phrases *arrive* and pieces *breathe*, curing a dissatisfaction (loopiness) that is a
distinct facet of the complaint, not just a difference. **Edge: A on the literal words, B on the
total listen.** This is the crux tension and the reason section 3 exists.

**(ii) Risk of another ear-rejection — does it fragment or re-pace the melodic line?**
This is the decisive axis, because **S65 died by fragmentation** (the cadence-gesture lever cut
the melodic lines into jarring silences). The two candidates sit on opposite sides of that failure
mode:
- **B is structurally immune.** Harmonic-rhythm rate operates on *the rate the chord layer changes
  underneath a continuous melodic line* — it re-paces the harmony *beneath* the tune, it never
  chops the tune. Textural density adds or removes accompaniment weight *around* the melody. Both
  act *under and around* the line; neither can gap it. The immunity is by construction, not by
  care.
- **A is only *conditionally* immune.** A subdivision/feel change acts on the very surface where
  the melody lives. Applied correctly — as a coherent *whole-texture re-quantization* where every
  voice, melody included, re-feels together onto the new grid as a continuous line — it does not
  fragment (swing lilts a line, it doesn't cut it; triple subdivision re-groups continuously). But
  a groove skeleton that inserts rests or accent gaps into the *melodic* role is exactly the S65
  trap wearing a new name. A's safety depends on building it as a re-feel, never as a gapped
  skeleton stamped on the melody. **Edge: B — it removes the dominant risk by construction; A must
  be engineered around it.**

**(iii) Build size / freeze-safety.**
`src/engine.rs` is frozen; `StepPlan` carries literals there, so **any new `StepPlan` field breaks
the sha**, while an `OrchestrationProfile` built via `identity()` is freeze-safe.
- **B is freeze-friendlier.** Harmonic-rhythm shaping can be expressed as *chord content per
  existing step slot* — a held chord repeats the same chord value across steps; an accelerando
  places distinct chords in the pre-cadence steps — i.e. same `StepPlan` shape, different values,
  **no new field**. The texture-density envelope rides on the per-section density number and the
  `identity()`-built `OrchestrationProfile`, both freeze-safe. Must-ship core is small (the
  harmonic-rhythm profile menu; the density arc is a near-free rider on the same seed).
- **A is larger and has a heavier carrier burden.** A feel is a five-part bundle that must be
  selected per piece and threaded down into rhythm realization, meter activation, and swing timing.
  The per-piece feel selection *can* be carried freeze-safe on the `OrchestrationProfile` /
  plan-level struct, but the surface it touches (`realize_rhythm`, meter, onset placement) is
  deeper and wider, and the "coherent bundle" is more code than a profile menu. **Edge: B.**

**(iv) Balance-law cleanliness.**
Both are clean and identical here: a curated, authored menu (feels for A; harmonic-rhythm profiles
+ density envelopes for B) selected **deterministically from an existing image feature**
(edge_activity or the arousal composite — the same knobs the form and phrase gates already read),
no RNG, no hash. Both preserve the frozen identity path byte-for-byte via a legacy default (A: the
straight-duple/4-4 feel; B: the `normative` flat one-per-step profile). **Even.**

### PRIMARY recommendation: **Candidate B — the harmonic-rhythm dramatic-arc + textural-density fix.**

Rationale, in priority order:
1. **Fragmentation immunity is structural, not conditional.** After two consecutive rhythm-surface
   interventions were ear-rejected (S65) and with the incremental strategy already judged at its
   ceiling, the single most important thing the next build must not do is fragment the line again.
   B *cannot*; A can if built carelessly. This dominates.
2. **It buys arrival and breath, not merely difference.** B fixes the "phrases stop and restart"
   loopiness and the level-plateau flatness — distinct dissatisfactions the ear reports — so a
   positive verdict is more robust than "different but still same-y." Distinctness and satisfaction
   come from one lever.
3. **It is the smaller, freeze-friendlier build** (chord-content-per-step + `identity()`-carried
   density), which is the right shape for a pointed single next build.

### ALTERNATIVE (a real second option): **Candidate A — the groove/feel subdivision-grid vocabulary.**

Choose A over B if the operator's ear weights the *literal* rhythmic-surface complaint above the
pace/shape complaint. A is the only fix that changes what is heard *as the rhythm*; it is the
brief's #1 axis and the on-the-nose answer to the words. Its tradeoff is the mirror of B's
strengths: it must be built as a coherent whole-texture re-feel (never a gapped melodic skeleton —
the S65 trap lives here), and it is the larger, deeper build with the heavier freeze-carrier
burden. It is not a weaker idea — it is the more direct one carrying more risk.

### On a joint slice: **recommended against.**
Both audits note the perceptual class breaks fastest if *both* walls fall together — but shipping
A **and** B in one build doubles the build surface *and* the ear-rejection surface, which is
exactly the "rebuild-everything" over-scope the operator forbade. The disciplined path is to
**sequence**: ship the primary in the next build, gate it on the ear, then take the other wall in
the following build. One pointed dimension at a time is what the strategic reset asked for, and it
keeps each ear-verdict cleanly attributable to one change.

---

## 3. The Ear-Before-Build Proof-of-Concept (the process fix)

**The core process failure across the last five sessions:** the taste gates reason from CODE and
cannot HEAR, so each session passed a change the operator's ear then rejected. The fix is not a
better code gate — it is to put the operator's ear on the *flavor* of the recommended direction
**before** paying for a full engine build.

**Why the S63 gate-liveness scorecard was insufficient.** It measured selector *coverage* — which
cells and routes fire across the corpus (4 characters reached, 4 phrase schemes reached, etc.).
Coverage counts *distinct selections in code*; it does not measure *perceived distance*. A catalogue
can spread across four characters and four phrase schemes (coverage looks healthy) while every piece
still shares the container the ear tracks — which is precisely what happened: the scorecard would
have passed S64 and S65 because the selectors fired, yet the ear rejected the result. Coverage is
necessary but not sufficient; **the only valid gate is an actual ear on actual audio.**

**The proof-of-concept for the primary (Candidate B), concretely.** Before building the profile
menu into the engine, hand-produce a tiny A/B render set — a fraction of a full build's cost —
from **one** bundled image (recommend a mid-corpus piece such as AudioHaxImg3 or Lena, not the
outlier `example`):

- **Control:** the current S64 baseline render of that image (flat one-chord-per-step, level
  texture) — the thing the operator's ear already knows.
- **Variant B-broad:** the same image, same key/tempo/motif, but with the chord-per-step sequence
  hand-edited so selected chords are *held across two steps* (a spacious, slow-changing pace), and
  the per-section density hand-set to a gentle open→fill→open shape.
- **Variant B-driving:** the same image again, but with the chord change *doubled into each cadence*
  (a proper pre-dominant → dominant → tonic approach, hand-voiced) and a texture-density envelope
  hand-set to *thin open → thicken to a mid-piece peak → recede* to the close.

All three render through the existing MIDI→FluidSynth path with **no engine change** — the
chord-per-step content and the per-section density are hand-authored into the plan (or the MIDI
hand-edited). Total cost: a few authored renders, hours not a build.

- **Greenlight the S67 build if:** the operator hears B-broad / B-driving as *arriving* and having
  *shape / breath* versus the flat control — **and, decisively, as reducing the "sameness"** (two
  pieces with the same tempo and motif now feel like different pieces because their pace/shape
  differs).
- **Kill / switch if:** the operator hears "it arrives nicely, but the *rhythm* is still the same"
  — i.e. the arc improves satisfaction and macro-distinctness but does not touch the *literal*
  rhythmic-surface complaint. That verdict is the honest weakness of the primary, and it is the
  precise signal to switch to the alternative and PoC *that* instead: two renders of one image at
  the **same tempo**, one in straight duple (control) and one hand-quantized to a triplet/6-8 or
  swung feel — the cheapest possible test of whether changing the *grid itself* is what the ear is
  asking for.

This A/B is deliberately built to disambiguate the exact tension the adjudication could not
resolve on paper: whether the operator's "rhythm feel" complaint is answered by *re-pacing the
container* (B) or only by *changing the surface grid* (A). One cheap listening test settles it
before either full build is paid for.

**On a perceptual proxy metric (secondary, offered honestly).** A code-side proxy that correlates
with perceived sameness better than coverage would be a *container fingerprint*, not a content
count: e.g. a per-piece harmonic-rhythm-rate curve, an onset-interval / subdivision-grid signature,
and a per-piece intensity envelope — then measure whether two pieces' *container* signatures differ,
not their notes. This is more trustworthy than coverage because it measures the container the ear
tracks. But the audits show content-vs-container distance is subtle, and no metric has been
validated against this operator's ear; so the fingerprint is at best a cheap pre-filter, and the
authored A/B render remains the real gate. Do not let a green metric substitute for the ear again.

---

## 4. Risks & Open Tensions (for the theory / aesthetics lenses to resolve at build time)

**Freeze-safe carrier (binds whichever fix wins).** `StepPlan` carries literals inside the frozen
`src/engine.rs`; a new `StepPlan` field changes those bytes and breaks the sha. The winning fix must
carry its new per-piece selection on the `identity()`-built `OrchestrationProfile` (or a plan-level
struct assembled the same way) and express its *effect* through the *content* of existing `StepPlan`
slots — the chord value in a step (B) or the onset placement within a step (A) — never through a new
field. The legacy identity path (B: `normative` flat profile; A: straight-duple/4-4 feel) must remain
byte-identical so the frozen default is preserved. Confirm the exact carrier boundary against
`src/engine.rs` before authoring signatures.

**For the primary (B) — voice-leading legality of held and inserted chords.** Shaping harmonic
rhythm makes the chord layer supply or hold chords at a chosen rate: a *held* chord (broad profile)
must not produce a static or illegal inner-voice situation, and a *doubled* change into a cadence
(driving profile) must be a proper pre-dominant → dominant → tonic approach, not an arbitrary chord
inserted to fill a faster slot. The macro-climax's harmonic and registral peak must stay in-key and
resolve home. The aesthetics lens specifies *where* the pace broadens, accelerates, and peaks; the
theory lens keeps the held chords, the inserted pre-cadential chords, and the peak voice-leading-
legal and cleanly resolved.

**For the alternative (A) — groove-skeleton coherence and the fragmentation trap.** A feel is a
coherent bundle, not a stack of independent knobs; an incoherent combination (triple subdivision
against the wrong accent template with a swing ratio that fights the meter) reads as arbitrary, a
new kind of wrongness. Each authored feel must be a hand-vetted, internally-coherent gestalt. And
the melodic line must be re-felt *continuously* onto the new grid — never gapped by a skeleton that
inserts melodic rests, which is the S65 fragmentation regression under a new name.

**Coupled-wall tension (both fixes).** Walls A and B are coupled: a fixed one-chord-per-step
harmonic rhythm (Wall B) may keep even a re-felt groove (Wall A) sounding loopy, and vice-versa.
Fixing one wall may *reveal* the other as the next-in-line complaint rather than fully clearing the
verdict. This is expected and is why the recommendation sequences the two builds rather than
merging them — but the second build should be anticipated, not treated as a failure of the first.

**Companion, out of scope for the pointed build.** The affect lens's arousal re-curve (row 15) is a
cheap, balance-law-clean, deterministic input-side fix that unlocks the currently-unreachable
high-energy textures. It is explicitly a *resolution-within-class* improvement (which cell a piece
starts in), **not** the class-multiplying fundamental fix, and it must not displace the primary. It
can ride alongside later or wait; it is noted here so it is not forgotten, not so it is built now.

**Balance law (binds all of the above).** Every menu is curated and authored; every selection is
deterministic from an existing image feature; no randomness or hashing enters the surface. Curated
menu + feature seed + deterministic selection is the doctrine the last three sessions established,
and both candidate fixes obey it by construction.

---

*Public-repo note: role-language only, no codenames. The S-tags (S62-S66) reference the existing
public diagnostic and design docs in this directory. `src/engine.rs` remained frozen and untouched
throughout this audit.*

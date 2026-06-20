# Diagnosis S49 — The Recurring Rhythmic Signature (image-invariant "dotted-quarter / eighth → triplet → long note")

Author: Music Theory Specialist (read-only diagnosis — no code changed)
Scope: root-cause WHERE the operator's recurring rhythmic motif is generated and WHY it is image-invariant; assess whether the queued Lane B (per-role line-library expansion) fixes it.

## The defect (operator's ear)

Across EVERY generated piece, regardless of input image, the operator hears the same prominent figure:

> **[dotted-quarter → eighth] (×1–2) → triplet → long note**

…perceived as the loudest voice even when it is nominally a background bed, and repetitive enough that all pieces sound like variations of one thing. This is a *sameness-across-pieces* defect, distinct from the figure-ground (S46–S49) work, and was NOT fixed by that arc.

## TL;DR

The signature is **emergent, not a literal template** — but it is emergent from a pipeline that **collapses nearly every real image onto the same two rhythm decisions**:

1. **Macro grid (the planner):** every piece is **4/4, fixed `ms_per_step`, 8-step sections, theme cell 0 ("the S39 anchor")**, character **"ballad"**. None of these vary across real photos — the affect axes that *could* vary them (arousal/valence) sit in a narrow low-mid band for natural images, and meter/character default tables fire their default for all of them.
2. **Micro grid (the per-step realizer `realize_rhythm`):** `edge_activity` for real photos clusters at **~0.30–0.51**, which lands the Melody arm in the **DOTTED band** (long-short pair) almost every time, with the **ARPEGGIO band's `n=3` even subdivision = the "triplet"** appearing only at the pre-cadence acceleration / high-edge steps, and the theme's `dur_steps≥2` notes = the **"long note."**

So the heard motif is the literal sonification of: *theme cell 0's `[2,1,1,2]`-style duration pattern (the long notes) + the DOTTED band's long-short pair on its onset steps (the dotted-quarter/eighth) + the 3-onset ARPEGGIO/pre-cadence burst (the triplet) + the cadential/sustained ring (the long note)* — and because the inputs that pick all of those are pinned, the same motif prints on every image.

Root-cause class: **primarily TUNING + ARCHITECTURE; the library/variety axis is real but secondary.** Lane B (per-role *line* library) will **NOT** fix the image-invariant rhythmic signature — the sameness is produced upstream of where a line-library lives.

---

## 1. WHERE the [dotted, eighth, triplet, long] sequence comes from — traced mechanism

There are **two rhythm layers** and the heard motif is their superposition. Neither is a single literal "dotted/eighth/triplet/long" template; the figure is emergent from both layers being pinned.

### Layer A — the planner's THEME rhythm-cell vocabulary (macro durations: the "long notes" and the dotted *feel*)

`src/composition.rs:1481-1497` builds the theme once per plan:
- `pick_archetype(u)` → one of 8 contours (`src/composition.rs:1704`).
- `pick_rhythm_cell(u, archetype, K)` → a cell index 0..3 (`src/composition.rs:1791-1811`).
- `resolve_motif_celled(...)` (`src/chord_engine.rs:3338`) cycles the chosen cell's `dur_steps` weights across the contour.

The cell vocabulary is in `MotifArchetype::rhythm_cells` (`src/chord_engine.rs:3187-3261`). **Cell 0 is the "S39 anchor" for every archetype**, and it is a *frozen* long-short-short-long shape — e.g. Arch `[2,1,1,2]` (`:3194`), Descent `[1,1,1,1,2]` (`:3211`), Ascent `[1,1,2]` (`:3222`). A `dur_steps=2` motif note sounds as a note held across 2 step-slots (its continuation step is a `MotifStep::Continuation` REST, `src/chord_engine.rs:3457,3559`) — i.e. **a long note**. The static-tail fix (`:3397-3404`) extends the FINAL note to absorb the leftover budget — **another long note at the end of the head.** So cell 0 alone already delivers *long … (short short) … long* — the "long note" bookends the operator hears.

Crucially, **the profiled/dotted cells (cell 3 — the `[3,...]` dotted, `[2,1,1]` Lombard-snap shapes at `:3194,3236,3250,3255`) are reached only when `complexity ≥ 0.66`** (`pick_rhythm_cell`, `:1800`). Real photos have `complexity ≈ 0.005–0.23` (measured — see §2), so **cell 3 is essentially never selected** and the dotted *character* the operator hears does NOT come from the dotted cell — it comes from Layer B.

### Layer B — the per-step realizer `realize_rhythm` band ladder (micro onsets: the "dotted-quarter/eighth" pair and the "triplet")

Every melody step — including theme onset steps — is sized by `realize_rhythm` (`src/chord_engine.rs:2006`). The Melody arm (`:2584-2714`) selects one of four onset shapes by `edge_activity` against fixed cutoffs (`:1061-1063`, `ARP=0.80 / SYNC=0.55 / DOTTED=0.25`):

- **ARPEGGIO / pre-cadence acceleration** (`:2625-2638`): emits `n` evenly-spread onsets, `n=3` interior, `n=4` pre-cadence. **The interior `n=3` even subdivision of one 4/4 beat-slot IS the "triplet" the operator hears.** It fires on high-edge steps and — via `pre_cadence` (`:2058`) — on the step before every cadence, in every phrase, in every piece.
- **SYNCOPATED** (`:2639-2648`): onset at `step_ms/4` + a second at `3/4`.
- **DOTTED** (`:2649-2661`): `sustained(0, 2/3) , sustained(2/3, 1/3)` — **a long-short pair = the dotted-quarter→eighth.** This is the band real photos land in (§2).
- **SUSTAINED** (`:2662-2680`): one long tone (the calm "long note"), with `THEME_LONG_NOTE_SING` lengthening a `dur_steps>1` theme note (`:2675`) — **reinforcing the long note.**

Cadences always ring as a single sustained, ritardando-lengthened note (`:2188-2216`) — **the long note at phrase ends.**

**Synthesis:** the operator's `[dotted-quarter, eighth] → triplet → long note` is exactly **DOTTED band (long-short) on the theme-onset steps → ARPEGGIO/pre-cadence `n=3` burst (triplet) → cadential/sustained ring + cell-0 `dur_steps=2` (long note)**, laid over the fixed 4/4 8-step grid and repeated phrase after phrase. It is **emergent from the band ladder + cell-0 durations cycling in a fixed order**, NOT a literal phrase template — but it is *deterministically the same* emergent figure because its two selectors are pinned.

---

## 2. WHY it is image-invariant — the pinned inputs (empirically measured)

I measured the actual `ImageUnderstanding` features on the six bundled images via `understand_image_pure` (scratch test, since removed):

| image | edge_activity | complexity | colorfulness | brightness | saturation |
|---|---|---|---|---|---|
| AudioHaxImg1.jpg | **0.301** | 0.005 | 0.011 | 29.3 | 32.9 |
| AudioHaxImg2.jpg | **0.509** | 0.015 | 0.080 | 81.1 | 30.1 |
| AudioHaxImg3.jpg | **0.475** | 0.229 | 0.423 | 65.7 | 37.2 |
| example.jpg | 0.719 | 0.905 | 0.685 | 49.9 | 64.5 |
| Lena.png | **0.471** | 0.164 | 0.122 | 70.5 | 51.9 |
| magicstudio-art.jpg | 0.106 | 1.000 | 0.287 | 45.3 | 43.6 |

Four of six real photos cluster at **edge_activity ≈ 0.30–0.51** and **complexity ≈ 0.005–0.23**. From those two numbers the entire rhythmic surface is decided, and it decides the SAME way for all of them:

1. **Per-step band → DOTTED for nearly every photo.** edge_activity 0.30–0.51 is inside the DOTTED band `[0.25, 0.55)` (`:2649`). The `floor_to_dotted` foreground floor (`:2615`) also routes any calm foreground melody INTO DOTTED. So the foreground figure is **DOTTED (the long-short pair) on essentially every real photo.** The ARPEGGIO band needs edge_activity > 0.80 (only `example.jpg` even approaches it), so the "triplet" comes overwhelmingly from the **pre-cadence acceleration**, which is structural and present in *every* phrase regardless of image.

2. **Theme cell → cell 0 for nearly every photo.** `pick_rhythm_cell` (`:1800-1808`): cell 3 requires `complexity ≥ 0.66` (photos fail it); then edge_activity in `[0.33, 0.66)` → **cell 0** (`:1804`). So the photos that aren't quite in DOTTED still take the **cell-0 long-short-short-long** macro durations. Cell 0 is also the *frozen S39 anchor* — the one cell the whole freeze discipline guarantees is byte-identical — so it is the de-facto universal default.

3. **Character → "ballad" for nearly every photo (constant tempo + constant window).** Arousal = `0.45·sat/100 + 0.25·colorful + 0.20·edge + 0.10·complexity` (`:328`). For the photos this computes to **arousal ≈ 0.21–0.39** (e.g. Img1≈0.21, Img2≈0.26, Img3≈0.39). The `character` table (`mappings.json composition/character`) needs **arousal ≥ 0.6** for scherzo/march; photos never reach it, so they fall to the **default "ballad"** (`:1804` analogue in the SelectTable). Ballad's tempo window is 56–96 BPM and the affect de-cap only widens *non-ballad* windows — so **tempo is also pinned** to the slow-mid ballad band. The slow tempo makes the same onset pattern even more conspicuously identical.

4. **Macro grid is hard-pinned.** `meter` default = "four4", **rules: []** → always 4/4 (`mappings.json`; `src/composition.rs:15,441,2219`). `ms_per_step` is section-stable (`:1190`). Sections are `BASE_STEPS_PER_SECTION = 8` (`:1357`) plus a small edge bonus. Phrase length is 4 or 8 (`PHRASE_LENGTHS`, `src/chord_engine.rs:579`). **There is no per-piece meter, no per-piece phrase length, no per-piece harmonic-rhythm plan** — so the *placement* of the dotted/triplet/long figure within the bar is identical piece to piece.

**Net:** real images vary in brightness/saturation/color (so harmony, register, and mode DO differ — which is why the operator hears "other things may be different"), but the **two numbers that drive rhythm (edge_activity, complexity) sit in a narrow band where every selector returns its default/anchor.** The rhythmic surface is therefore a near-constant function of the image set. The S13 normalization (`edge_density / 0.05`, `:2038`) was meant to spread real photos across the bands, but real Canny edge densities still land everyone in the DOTTED neighborhood rather than spreading them across SUSTAINED→DOTTED→SYNC→ARP.

---

## 3. WHY it's perceived as loudest even as a "background" role

Two mechanisms, and they sit **outside** the S47/S48 figure-ground level work:

1. **The figure IS the melody, and the melody is the foreground.** The DOTTED long-short pair + the pre-cadence triplet are emitted by the **Melody arm** of `realize_rhythm` (`:2584`), which is the declared foreground. The S46–S49 figure-ground arc made the *bed recede relative to the melody* (`pad_onset_cap`, `recede_pad_onsets`, the counter governor, the inverse-register comp). It never changed the **melody's own rhythm** — so the recurring figure the operator hears is precisely the voice the figure-ground arc was *protecting and promoting.* Making it more foreground (S47/S48) makes the repetitive figure MORE prominent, not less. This is why "figure-ground didn't fix it" — figure-ground is orthogonal to *which rhythm the figure plays.*

2. **Where the bed carries it, the bed's own onsets are conspicuous.** When edge_activity is low the melody floors to DOTTED (`floor_to_dotted`, `:2615`) and the **Pad's figured bed** (`figured_bed`, reached at `:2405-2408`) animates the inner tones into a multi-onset burst — `recede_pad_onsets` only caps the COUNT, it does not change the *cell* the burst plays, and the bed plays on the fixed grid. A bed that is rhythmically a smaller copy of the same long-short/triplet grid, at a low slow tempo, reads as "the same figure in the background." The S47 level/onset recession reduces its *count and downbeat fusion* but not its *rhythmic identity*.

So: the loudest-perceived figure is the **Melody's** DOTTED/ARPEGGIO output (foreground by design), reinforced by a bed whose rhythmic *shape* is the same grid. The interaction with S47/S48 is that those passes touched LEVEL/COUNT/OFFSET — not the rhythm *vocabulary* — so they cannot dislodge the signature.

---

## 4. Root-cause classification

Ranked by contribution to the *image-invariant sameness*:

1. **TUNING (primary).** The band cutoffs (`:1061-1063`, DOTTED 0.25 / SYNC 0.55 / ARP 0.80) and the cell-selection cuts (`CELL_EDGE_BROAD 0.33 / CELL_EDGE_BUSY 0.66`, `CELL_COMPLEXITY_PROFILED 0.66`) are positioned so that the **measured real-image feature band (edge 0.30–0.51, complexity ~0) maps to a single output** (DOTTED + cell 0). The cutoffs don't *cross* across the natural-image distribution. Same for the character arousal gate (0.6) vs the real arousal band (0.2–0.4). The mapping is monotone but its *active range* doesn't overlap the data — so it behaves like a constant.

2. **ARCHITECTURE (primary, co-equal).** There is **no per-piece rhythmic identity above the per-step band**: fixed 4/4, fixed `ms_per_step`, fixed 8-step sections, fixed 4/8 phrase lengths, one harmonic-rhythm policy, and the *pre-cadence acceleration is structural* (fires every phrase). The "triplet" and the cadential "long note" are emitted by phrase *structure*, not by the image at all — so they are invariant by construction. No meter variety, no per-piece phrase-length plan, no per-piece harmonic-rhythm curve, no piece-level "rhythmic theme" distinct from the pitch theme.

3. **LIBRARY / VARIETY (secondary, real but not the cause of *invariance*).** The cell vocabulary is K=4 per archetype and *three of the four* (broad/busy/profiled) are nearly unreachable for real photos, so in practice the system has **~1 effective rhythm cell** (cell 0). Enlarging the vocabulary helps *only if the selector can reach the new entries* — which §2 shows it currently cannot for real images. So variety is genuinely thin, but thin variety is downstream of the tuning/architecture pinning; adding entries a pinned selector never reaches changes nothing audible.

This is a **combination, dominated by tuning + architecture.** It is NOT primarily a too-small-library problem.

---

## 5. Does the queued Lane B (per-role line-library expansion) fix this? — HONEST verdict

**No — Lane B does not fix the image-invariant rhythmic signature.** Reasoning:

- A per-role **line library** expands the *pitch/figuration vocabulary per role* (what notes each role weaves). The recurring *rhythmic* signature is generated by (a) the per-step band ladder in `realize_rhythm` and (b) the theme rhythm-cell selection + the fixed macro grid. A line library sits at the pitch layer; it does not change which `edge_activity` band a step lands in, does not change `pick_rhythm_cell`, does not change the meter/phrase/pre-cadence structure, and does not change the ballad tempo pin.
- Even a richer per-role line will be **rhythmically realized through the same DOTTED/ARPEGGIO/SUSTAINED arms on the same fixed grid** — so two pieces with different lines will still clap identically. The operator already reports "other things may be different" while the *rhythm* is the same; Lane B adds more "other things," precisely the axis already varying, and leaves the invariant axis untouched.
- Lane B *would* help the moment the rhythm-selection layer is unpinned (then richer lines + richer reachable rhythms compound). But on its own it is mis-targeted at this specific defect.

**Recommendation: re-sequence — do a rhythm-variety slice BEFORE (or instead of, for this defect) Lane B.**

---

## 6. Candidate fix-directions (ranked) — what would actually introduce per-piece rhythmic variety

Each tagged with which lens owns it. **Affect/Aesthetics-owned** = the cut points / felt character are taste-subjective and must go through the taste/affect review gate; **Music-Theory/Architecture-buildable** = mechanically buildable to a correctness gate once the design is set.

### Rank 1 — RE-RANGE the rhythm selectors onto the real-image distribution (TUNING). *Cheapest, highest leverage.*
The cutoffs are fine in shape but wrong in *position* for natural images. Re-map `edge_activity` (and the cell/character cuts) so the measured real-image band (≈0.3–0.5 edge, ≈0.2–0.4 arousal, ≈0–0.2 complexity) **spreads across SUSTAINED→DOTTED→SYNC→ARP and across cells 0–3 / ballad–march**, instead of collapsing to DOTTED+cell0+ballad. Options: percentile/standardized normalization of `edge_activity` instead of the raw `/0.05`; lower `CELL_COMPLEXITY_PROFILED`; widen the arousal composite's dynamic range or recenter the character gates.
- **Lens:** the *re-positioned cut values* are an Affect/Aesthetics taste call (where does "busy enough to subdivide" sit perceptually?); the normalization mechanism is Music-Theory/Architecture-buildable. **Needs the taste lenses for the cut points.**
- Caveat: this *spreads* the signature across the catalog but, alone, every image still has ONE rhythm — it fixes *sameness-across-pieces* far more than *monotony-within-a-piece*. Pair with Rank 2.

### Rank 2 — A per-piece RHYTHMIC IDENTITY plan (ARCHITECTURE). *The structural fix.*
Introduce a piece-level rhythm plan parallel to the existing key/theme plan: image-derived **meter** (activate the already-present `meter` SelectTable — `four4/three4/six8` exist at `:2219-2221` but rules are empty), image-derived **phrase length / harmonic-rhythm curve**, and a **piece "rhythmic motto"** (a characteristic cell sequence chosen per image, distinct from the pitch theme) that the realizer honors instead of re-deriving the figure per step from one `edge_activity` number. Make the pre-cadence acceleration *vary* (not fire identically every phrase).
- **Lens:** the *mechanism* (a RhythmPlan struct, meter activation, plan→realizer seam) is Music-Theory/Architecture-buildable; the *feel* of "this image → 6/8 lilt vs 4/4 march" and the motto cuts are Affect/Aesthetics taste calls. **Needs both** — Architecture builds the seam, Affect/Aesthetics author the image→meter/motto mapping.

### Rank 3 — DECORRELATE the per-step band from `edge_activity` alone (TUNING + small ARCH).
Right now one scalar (`edge_activity`) drives the band for every step of every piece, so the band is near-constant. Feed the band selection additional, independent axes already present (`texture`, `colorfulness`, `complexity`, region energy) and/or add a per-step pseudo-random-but-deterministic *gait variation* seeded by the image so consecutive phrases don't all clap identically.
- **Lens:** mostly Music-Theory/Architecture-buildable; the *weighting* of which visual axis should push subdivision is an Affect/Aesthetics call (it must stay musically-meaningful — visual activity→rhythmic activity). **Light taste involvement.**

### Rank 4 — UNPIN the character→tempo path so real images leave the ballad window (TUNING/Affect).
Recenter the arousal composite or the character gates so mid-arousal real photos reach march/scherzo/nocturne, engaging the de-cap and giving tempo (and thus the felt rhythm) real per-piece spread.
- **Lens:** Affect/Aesthetics-owned (it is literally the affect bridge's job; the de-cap mechanism already exists). **Needs the affect lens.**

### Then — Lane B (per-role line library) compounds.
Once Ranks 1–2 unpin the rhythm layer, a richer per-role line library multiplies the gained variety. **Sequence Lane B AFTER the rhythm-selector re-range + the per-piece rhythm plan,** not before.

---

## Appendix — key file:line evidence index

- Band cutoffs: `src/chord_engine.rs:1061-1063` (ARP 0.80 / SYNC 0.55 / DOTTED 0.25).
- Melody band realization (the dotted long-short pair, the n=3 "triplet"): `src/chord_engine.rs:2624-2680`; pre-cadence n=4 at `:2630`; pre_cadence definition `:2058`.
- `edge_activity` normalization `/0.05`: `src/pure_analysis.rs:760`; `src/chord_engine.rs:2038`; range const `EDGE_ACTIVITY_RANGE_MAX = 0.05` `:1939`.
- Theme rhythm-cell vocabulary (cell 0 = frozen anchor; cells 1–3 broad/busy/profiled): `src/chord_engine.rs:3187-3261`; resolve `:3338`; static-tail long note `:3397-3404`.
- `dur_steps≥2` → long note / continuation rest: `src/chord_engine.rs:3457,3559`; sing-lengthen `:2675`.
- Cell selection cuts (complexity≥0.66 → cell 3; edge bands): `src/composition.rs:1770-1811`.
- Character / affect (arousal/valence composite + ballad default): `src/composition.rs:328-360`; `assets/mappings.json composition/affect` + `composition/character`.
- Fixed macro grid: meter always 4/4 `src/composition.rs:15,441,2219` + empty `meter` rules; `ms_per_step` section-stable `:1190`; `BASE_STEPS_PER_SECTION = 8` `:1357`; phrase lengths `src/chord_engine.rs:579`.
- Figure-ground passes are LEVEL/COUNT/OFFSET only (orthogonal to rhythm vocabulary): `pad_onset_cap`/`recede_pad_onsets` `src/chord_engine.rs:1385,2424-2447`; inverse-register comp `:2129-2158,2682-2712`.
- Measured real-image features: §2 table (via `understand_image_pure`, scratch test removed).

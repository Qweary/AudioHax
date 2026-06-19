# S38 — Composition & Songwriting Aesthetics Design Note

**Author:** Composition & Songwriting Aesthetics Specialist
**Session:** S38 (design / diagnosis only — no `src/*` edits this session)
**Scope of this lens:** the macro craft of composition — song form, key plan, motif identity & development, pacing/contrast/resolution. Whether the *whole shape sounds good and feels pleasing to a trained ear*. This lens does **not** own chords/voice-leading/modulation mechanics (Music Theory owns `chord_engine.rs`) nor pixels (extraction owns analysis). Where a finding needs a pivot, a cadence, or note-level voice-leading, it is handed to the Music Theory lens explicitly in §5.
**Ultimate gate:** falsifiable by listening. Every guard-rail below is stated as a property a trained ear (or a cheap automated proxy) can confirm or reject. The owner's ear is the final arbiter.

---

## 0. Orientation — what the composer actually does today

Read from `composition.rs::CompositionPlanner::plan` (1256) and `chord_engine.rs`:

- One plan is built per image: a `FormSpec` is selected from a 6-form catalogue, a `KeyTempoPlan` spine is computed, sections are expanded with per-section harmony (`generate_chords`/`plan_phrases`), and **zero or one** returning theme is resolved (`themes` is `Vec::new()` or `vec![ThemeSeed{ id:0, .. }]`, lines 1332–1342).
- The home key-center is hardcoded: `home_root_midi = 60` (1302). Key *excursions* are real and per-section (`key_scheme` / `resolve_key_scheme`, the `*_excursion` schemes), but they all depart from and return to **C**.
- The single theme's shape is one of `pick_archetype`'s **4 active archetypes** (Arch / NeighborTurn / Descent / Ascent — 1500–1512) chosen by a coarse hue quadrant + an `edge_activity ≥ 0.6 → Ascent` override. Four further archetypes exist in `chord_engine` (InvertedArch, LeapStep, Pendulum, RisingSequence — 2320–2338) but are **never reachable** from the planner.
- The motif is `degree + dur_steps` with `dur_steps` pinned to `1` (resolve_motif, 2415) — rhythm comes entirely from the realizer, not the motif.
- Density is a per-section scalar in `[0.35, 0.65]` centered on `0.5`, mapped from region energy (1446–1451) and nudged into realizer busyness with gain `0.5`, so the **whole audible density swing is ≤ ±0.075 of edge-activity** (chord_engine 1398–1403). The identity/home path is pinned to `0.5` for byte-freeze.

This orientation is what makes the three findings cohere: **the piece has real structure but almost no identity-bearing surface.** Form travels, key travels, density barely travels, and the *one* memorable element — the theme — is drawn from a 4-shape vocabulary by a coarse selector. The ear hears "a competent ambient sketch that could be any image" rather than "this image's song."

---

## 1. Finding #1 — every image starts on the same root/pitch (home = C, hardcoded)

### Aesthetic diagnosis

A fixed home key-center is **not** a problem of *correctness* — a piece in C is a perfectly good piece. It is a problem of **per-image identity and cross-image differentiation**. Two things a trained ear notices immediately on an A/B listen of two different images:

1. **Absolute register sameness.** Every piece opens in the same tessitura around C4. The *first impression* of a piece — the pitch the ear anchors to in the opening seconds — is identical across all images. Form and density variation are slower-acting; the opening pitch is the fastest identity signal we have and we are spending it on a constant.
2. **No "home of its own."** The owner's instinct ("this image has its own home") is correct musical intuition. A song's tonal home is part of its identity the way a key is part of a singer's signature. When every image lives in C, the *excursion-and-return* machinery (which IS per-image) is doing identity work against a fixed backdrop instead of with one.

Crucially: a varying home does **not** weaken the returning-home payoff, **provided the home is fixed *within* a piece.** The payoff of "return home" is a *relative* perception — the ear remembers where the piece started and feels the homecoming when it lands back there. It does not require that "home" be C; it requires that home be *constant for the duration of this piece* and *clearly established at the open and re-asserted at the close*. A piece that opens in E and returns to E pays off exactly as well as one in C.

**Verdict from this lens: image→key-center SHOULD be part of the form's identity.** It is a high-value, low-risk identity lever. It is *not* the highest-leverage finding (that is #2 — see §6), because key-center is an identity the ear reads slowly and somewhat unconsciously, whereas motif is the identity it reads consciously and remembers.

### The guard-rail that keeps "return" feeling like home

> **GR-1 (home invariance within a piece).** The home pitch class is selected **once per image** and is **constant for the whole piece**. Every section's `key_offset_semitones` remains an offset *relative to that per-image home*, never relative to a fixed C. The Statement opens unambiguously on the home tonic and the final Return/Coda cadences onto the **same** home pitch class it opened on.
>
> Falsifiable by listening: pick the home pitch by ear in the first 2 seconds; confirm the final chord resolves to that same pitch class. Falsifiable automatically: assert `sections.first()` and the final `Return` section both resolve to `home_root_midi mod 12`, and that `home_root_midi` is a pure function of the image (not the constant 60).

> **GR-2 (don't move the seed octave).** Vary the home **pitch class**, but keep the **register** stable (home_root in a single octave band, e.g. MIDI 55–66) so we change *which note is home* without making some images boomy and others shrill. Register is a separate expressive dimension (it belongs with density/orchestration), not a free rider on key-center selection.

### Design (v1)

Drive `home_root_midi` from the image instead of the constant `60`. The natural, already-present driver is **dominant hue → pitch class** (the same hue signal that already picks `home_mode` via `hue_to_mode`, composition 1294–1296). Hue is a circular quantity and pitch class is a circle of 12 — this is a musically honest, deterministic mapping (a "color wheel → chromatic wheel" the synesthetic tradition already uses).

- v1 mapping: `pc = round(dominant_hue / 30) mod 12`; `home_root_midi = 60 - ((60 mod 12) ... )` seated into the 55–66 band so the result is `seat_pc_in_register(pc, 55)`. Determinism preserved (pure function of `dominant_hue`).
- **Off-by-default spice:** a *secondary* identity nudge from `value_key` (dark images seat home a touch lower in the band, bright a touch higher). Keep OFF in v1 — it couples register to key-center, which GR-2 warns against; ship it only if the ear wants it after the pitch-class move lands.

This is a one-line *planner* change at composition.rs:1302 (constant → function of `u`). It does **not** touch `engine.rs`. The hue→mode coupling already works at this site, so the harmony machinery downstream (`generate_chords` is already called with `section_root_midi`, 1418) re-roots automatically.

### Risk handed to Music Theory (see §5)

Moving the *home* pitch class is upstream of every section root. The pivot/cadence machinery (`is_mod_boundary`, `tonic_triad`, 1423/1441) already builds at `section_root_midi` and is home-relative, so it should ride the change transparently — **but Music Theory must confirm that the opening Statement establishes the new home strongly** (a root-position tonic / authentic gesture in the first phrase) so the ear *latches the new home fast enough* for GR-1's return to pay off. That is a cadential-establishment question, not a key-selection question.

---

## 2. Finding #2 — too few distinct themes/motifs (THIS LENS'S CORE FINDING)

### Aesthetic diagnosis — what "only ~2 shapes" really means

The owner heard ~2 distinct shapes (one long sustained note ending in stabs; one pair of triad variations). Three separate causes stack, and naming them precisely is the whole job here:

**Cause A — vocabulary half-disabled.** `pick_archetype` (1500) can only ever return 4 of the 8 defined contours, and within those 4 the selection is by *coarse hue quadrant* (4 buckets of 90°) plus a single `edge_activity` override. Eight images with hues in the same quadrant get the **same** contour. So even before we talk about development, the *raw alphabet* the ear could distinguish is 4, and the *effective* alphabet most images draw from is ~2–3.

**Cause B — weak image→motif spreading.** The motif's *only* image-dependent dimensions besides contour are `range_degrees` (from `edge_activity`) and `length_steps` (from `complexity`) — both feeding `resolve_motif` (1338–1340). And `resolve_motif` *re-scales the same contour* into the range; it never changes the contour's identity (its own doc says so, 2385–2388). Worse, **`dur_steps` is pinned to 1** (2415) — so the motif carries **no rhythmic identity of its own.** Rhythm is entirely the realizer's, which is image-driven but *theme-agnostic*. A motif with no rhythm and a re-scaled-but-identical contour is barely a "theme"; it is a pitch sketch. Two images with the same contour and similar energy produce near-identical themes. This is why the owner heard "two triad variations" — they ARE variations of one shape, because the shape barely varies.

**Cause C — no development across sections.** The single theme is *stated* (Statement) and *recalled* (Return), and in B-sections it is at most `Fragmented` (head-then-rest, theme_melody_pitch 2495–2513). There is **no transformation** that makes a section feel like it is *doing something new with the same idea* — no inversion, no rhythmic augmentation/diminution, no sequence, no contour re-contouring. The InvertedArch and RisingSequence contours that *would* read as "development of the head" are exactly the 4 that `pick_archetype` cannot reach. So the piece never demonstrates the one thing that makes a 2-shape vocabulary feel rich in real music: **the same idea, transformed, so the ear hears unity AND variety.**

The deepest aesthetic point: **memorability comes from a single strong idea developed, not from many weak ideas listed.** The fix is therefore *not* primarily "more contours." It is (1) make the *one* idea actually distinctive per image, and (2) make the piece *develop* it so it earns its returns. Vocabulary breadth is the cheap third lever, useful mainly for cross-image differentiation.

### Ranked design

Ranking is by *aesthetic leverage per unit of risk*, highest first. v1 = ship now; SPICE = off-by-default, behind a mappings flag, ship after the ear approves v1.

---

**Rank 1 (v1) — Give the motif a rhythmic identity (kill the flat `dur_steps = 1`).**
A memorable motif is a *rhythm* as much as a pitch shape ("da-da-da-DUM" is remembered by its rhythm). Today rhythm is theme-agnostic, so every theme has the same gait. Let `resolve_motif` emit a small **rhythmic cell** — 2–3 durations drawn from the image (e.g. `complexity`/`edge_activity` pick from a tiny palette of cells like `[1,1,2]`, `[2,1,1]`, `[3,1]`, `[1,1,1,1]`). This is the single biggest "distinctness" win because rhythm is the most *memorable* and most *cheaply differentiable* dimension, and it is entirely inside `resolve_motif` (chord_engine 2379) — Music-Theory-owned, no `engine.rs` touch, the realizer already reads `dur_steps` it just always gets 1.
- **Guard-rail GR-3 (rhythmic identity):** two images that differ in `complexity` by ≥ 0.3 must produce themes whose duration sequences differ in at least one position. Falsifiable by listening: clap the two themes back-to-back — they should not have the same gait.

**Rank 2 (v1) — Enable all 8 archetypes + finer selection.**
Switch `pick_archetype` from 4 hue-quadrants to a selector over the full 8 contours, driven by **two** image axes so it is not collinear with hue alone: hue picks a *family* (rising/falling/oscillating/turning) and a second knob (`vertical_emphasis` or `mass_centroid.y`) picks *within* the family (e.g. upward-mass → Ascent/RisingSequence, downward-mass → Descent, balanced → Arch/InvertedArch, central/contained → NeighborTurn/Pendulum, leaping → LeapStep). This doubles the reachable alphabet and decorrelates it from the single hue axis, so two similar-hue images can still get different shapes.
- **Guard-rail GR-4 (vocabulary spread):** across a fixed test set of ≥ 12 varied images, at least 6 distinct archetypes are observed (no single contour above ~30% share). Falsifiable automatically by logging selected archetype per fixture.

**Rank 3 (v1) — Revive prominence→melody so the most-prevalent subject drives the melody.**
The `subject_melody` prominence profile already exists (mappings prominence_catalogue) and is selected when there's a clear subject (subject_size in 0.05–0.55 AND fg_bg_contrast ≥ 0.25). The S19 parked idea — "most-prevalent subject drives the MELODY" — is the *semantic* anchor that makes the melody feel *about the image* rather than about the whole-image average. Concretely: when a subject is present, derive the motif's **contour family and its range from the SUBJECT's properties** (`subject_hue` → contour family, `subject_size` → range/register prominence) rather than the whole-image `dominant_hue`/`edge_activity`. This is what stops the "image-unrelated / ethereal" complaint at the *melodic* level — the tune now tracks the thing the eye actually lands on.
- This is a **planner** change at the theme block (1335–1340): when `u` has a salient subject, feed `pick_archetype` and the range/length from the subject knobs. Determinism preserved.
- **Guard-rail GR-5 (subject anchoring):** for an image with one obvious subject, the melody's contour/register correlates with the subject (e.g. a high-in-frame bright subject → ascending/high motif). Falsifiable by listening: "does the tune feel like it's tracing the main thing in the picture?"

**Rank 4 (v1) — Develop the theme across sections (the unity-with-variety engine).**
Use the form's existing section roles to *transform* the one motif instead of only stating/fragmenting it. The cheapest high-value transforms, all expressible as a section-level `variation` the realizer applies to the *same* `ThemeSeed`:
- **Inversion** in a Development/Contrast section (negate the degree offsets — the contour comment at chord_engine 2344 explicitly notes negate==invert is clean on this data).
- **Sequence** (restate the head a step higher) in a developmental section — RisingSequence's logic generalized.
- **Augmentation** (double `dur_steps`) for a Coda/arrival; **diminution** (halve, floored at 1) for an intensifying section.
These give the ear "same idea, new light," which is the actual source of richness. This needs a small extension to the slice-1 `{Identity, Fragmented}` variation set and a transform applied in `theme_melody_pitch` (chord_engine 2472) — Music-Theory-owned realizer territory, still no `engine.rs` touch.
- **Guard-rail GR-6 (developed-but-recognizable):** a listener can identify the Development section's material as "the same theme, changed" — not as a brand-new tune and not as a literal repeat. Two-sided test: it must *fail* if the section sounds unrelated (over-transformed) AND fail if it sounds identical to the Statement (under-transformed).

---

**SPICE (OFF by default, ship after v1 lands):**

- **S-1 — A genuine second theme (B-theme).** Allow `themes` to hold 2 seeds; give the Contrast section its *own* short motif (contrasting contour family + register) instead of only fragmenting theme 0. This is the textbook "A has a tune, B has a different tune" and is very effective — but it raises the risk of the piece feeling *disjointed* if the two themes don't relate, so it is spice, gated behind the ear confirming v1's single-theme development first. (Today `themes` is hardcoded to 0-or-1 at 1332–1342.)
- **S-2 — Motivic liquidation in the Coda** (fragment the head down to a 2-note cell that dissolves). High-craft, easy to overdo, pure spice.
- **S-3 — value_key→register for the motif** (dark images sing the theme lower). Pairs with GR-2's caution; spice.

### The single testable "memorable & distinct" guard-rail (umbrella)

> **GR-7 (the differentiation test) — the listening acceptance gate for #2.** Take any two *clearly different* images. Generate both. Play the two **opening themes** back to back to a listener who has not seen the images. They must be able to say "those are two different tunes" on rhythm OR contour OR register grounds — not "those are two takes of the same ambient idea." Re-run with two *similar* images: the themes may be similar (that is correct — similar images SHOULD rhyme), but must not be byte-identical unless the images are.

GR-7 is the falsifiable-by-listening test the whole #2 design is built to pass. GR-3..GR-6 are the component sub-tests that, together, produce GR-7.

---

## 3. Finding #3 — density feels reversed (too sparse; "Miles-Davis scarcity")

### Aesthetic diagnosis

This is the **macro-pacing** half of my lens (the *note-level* density mechanics are Music Theory's, but *when the piece should be full vs sparse for dramatic shape* is mine).

The S16–S18 "empty periods feel full" judgment was made on the **prior** path; the actual composer is demonstrably sparser. The code shows why precisely:

1. **The whole density band is `[0.35, 0.65]` around `0.5` (composition 1226–1229), and the realizer gain is `0.5` (chord_engine 1403), so the *audible* edge-activity swing from density is ≤ ±0.075** (the comment at 1398–1399 states this outright). Density is doing almost nothing audible — it cannot make a section *feel* full. So the perceived overall fullness is governed by the **base** path, and the base path is built for byte-freeze neutrality (everything pinned at 0.5), i.e. *deliberately middling*, which on a real composer reads as thin.
2. **`DENSITY_NEUTRAL = 0.5` is the wrong center for "fuller-by-default."** The operator wants fullness as the *resting state* and silence as a *device*. The current center says "moderate everywhere, never commit to full, never commit to empty." That is the textbook recipe for "scarcity that doesn't read as intentional" — it is neither lush nor pointedly sparse, so the ear hears *under-arranged* rather than *artfully restrained*.
3. **Silence isn't placed; it's averaged.** True "deliberate silence" (the Miles-Davis effect the operator likes *in principle*) only works when the surrounding texture is FULL — silence is contrast, and contrast needs a loud thing to be quiet against. With everything at 0.5 there's nothing for a silence to land against, so any sparseness reads as absence, not as a rest.

### Macro-pacing / form design (fuller-by-default, silence as a device)

The fix is a **pacing curve over the form**, not a global "+0.2 everywhere." Music has a standard, ear-validated fullness arc: *establish → build → climax → resolve*, with the climax fullest and the moment *before a return* often pointedly thin so the return blooms. Map fullness to the **thematic role** the planner already assigns each section (`thematic_role`, composition 1456):

| Section role | Target fullness | Rationale (dramatic shape) |
|---|---|---|
| Statement (A) | **High (full)** | First impression should be rich and committed, not tentative. |
| Contrast / Development (B) | **Highest** at its peak, with a **pointed thin spot just before the return** | The build/climax is the fullest the piece gets; the pre-return thinning is the *one* place silence earns its keep — it sets up the homecoming. |
| Return (A') | **High**, blooming out of the thin pre-return | The return should feel like arriving *home and full*, contrasted against the thin lead-in. |
| Coda | **Tapering** | A controlled decrescendo of texture — the only other place deliberate sparseness reads as intentional (a settling, not an absence). |

Concrete, minimal changes (all **planner-side**, all consistent with the byte-freeze hinge because they move the *non-identity* path only):

- **D-1 (v1) — raise the resting center.** Move the effective default fullness up so "full" is the resting state: widen/shift the band toward the top (e.g. center on ~0.62, span the same ±0.15 → `[0.47, 0.77]`) **and** raise `DENSITY_ACTIVITY_GAIN` so the swing is actually audible (today ±0.075 is imperceptible; target a felt ±0.15–0.20). Keep the *identity/home_only* sections pinned at neutral for byte-freeze (the freeze hinge at chord_engine 1400–1402 keys on `density == 0.5` for the *identity* path — the compose path is free to use a different center, so this does **not** break the equivalence net as long as the identity carry stays 0.5).
- **D-2 (v1) — bias fullness by role.** Add a per-role fullness term so Statement/Return sit high, Contrast peaks higher, Coda tapers — implemented as a role→density-bias lookup applied where `section_density` is set (composition 1450). This is the macro-pacing curve in one place.
- **D-3 (v1) — place ONE deliberate thin spot.** In the section *immediately before a Return*, dip the last sub-span's density to a genuine low so the return blooms. This is the *only* sanctioned "silence as a device" in v1 — one well-placed thinning beats scattered sparseness everywhere.

> **GR-8 (fuller-by-default):** on a neutral/average image, the Statement and Return read as *full/committed*, not tentative. Falsifiable by listening: the opening should sound arranged, not like a sketch.
> **GR-9 (silence earns its keep):** any sparse passage must be *adjacent to* a full one and must read as a *setup or a settling* (pre-return dip, or Coda taper) — never as a generic thin stretch in the middle of a section. Falsifiable by listening: "did that quiet moment make the next loud moment better?" If no → it's not a device, it's a hole; remove it.

### Note on the byte-freeze contract

None of D-1..D-3 require touching `engine.rs`. They move constants and add a role-bias in `composition.rs` and possibly the gain in `chord_engine.rs`. The equivalence net is protected by the *identity-path* pin (`density == 0.5` on home/home_only/identity sections), which we preserve; the compose path is where all the new fullness lives. Music Theory should confirm the gain increase doesn't push the realizer's rhythm patterns into an unmusical onset density (a note-level concern).

---

## 4. Cross-finding synthesis — why these three reinforce each other

The three findings are not independent bugs; they are three facets of one aesthetic deficit: **the piece doesn't commit to an identity.** Key-center is constant (#1), the identity-bearing motif is thin and undeveloped (#2), and the texture refuses to be either full or pointedly empty (#3). Fixing all three turns "an ambient sketch that could be any image" into "this image's song": it has *its own home* (#1), *its own developed tune* (#2), and *a committed, shaped texture* (#3). Doing only one will under-deliver, because the ear reads identity from the *combination* — a strong tune in a committed texture in a key of its own.

---

## 5. RISKS / open tensions for the Music Theory lens to resolve

These are the points where my macro design needs note-level craft I deliberately do **not** own:

1. **(#1) New-home establishment & cadence.** Moving `home_root_midi` per image is mine to *specify*; making the opening phrase *establish* the new home strongly (so GR-1's return pays off) is a cadential/voice-leading question. Music Theory: confirm the Statement opens with a tonic-establishing gesture in the new key, and that the final Return cadences authentically onto the new home PC.
2. **(#1/#2) Pivots for any key-center move.** Excursion sections already pivot (`is_mod_boundary`, composition 1423). With a per-image home AND developed motifs that may be inverted/sequenced *across* an excursion, Music Theory owns whether the pivot V→I still lands and whether a transposed/inverted motif voice-leads cleanly into the destination key.
3. **(#2) Motif voice-leading under transformation.** Inversion/sequence/augmentation (Rank 4) change the degree sequence; Music Theory owns ensuring the transformed motif still sits in the mode (degree→pitch through `degree_to_pitch`, chord_engine 2426) and doesn't create awkward leaps or clashes with the section harmony.
4. **(#2) Rhythmic cell vs. realizer rhythm.** Giving the motif real `dur_steps` (Rank 1) means the motif rhythm and the realizer's rhythm layer now coexist. Music Theory + realizer owner must define precedence (theme rhythm wins on Statement/Return melody; realizer fills elsewhere) so they don't fight.
5. **(#3) Gain & onset musicality.** Raising `DENSITY_ACTIVITY_GAIN` (D-1) pushes the realizer's rhythm-pattern selection; Music Theory confirms the busier bands stay musical (no machine-gun onsets) at the new top of the band.
6. **(#3) Cadential homecoming vs. pre-return thin spot.** D-3's pre-return thinning must not collide with the cadence machinery that arms the land-home resolution (`resolution`/`ResolutionPolicy`, composition 1069/1467). Music Theory confirms the dip lands *before* the cadential approach, not on top of it.

---

## 6. Highest-leverage finding from this lens, and why

**Finding #2 (motif identity & development) is the highest-leverage from the composition-aesthetics lens.**

Reasoning:
- **The motif is the only element the ear consciously remembers and reports.** The owner's report itself is the evidence: they counted *shapes* ("~2 themes"), not keys or densities. Identity that the listener can *name back* lives in the tune. Key-center (#1) and density (#3) are real and worth fixing, but the ear reads them slowly and semi-consciously; the tune is read instantly and remembered.
- **It is the root cause of the "image-unrelated / ethereal" complaint at the surface level.** Reviving prominence→melody (Rank 3) is what makes the music feel *about the image* rather than about its average — the single most direct fix for "structureless / image-unrelated."
- **It is mostly inside `resolve_motif`/`pick_archetype`/`theme_melody_pitch`** — already-built, byte-freeze-safe seams. Rank 1 (rhythmic cell) and Rank 2 (full vocabulary) are small, low-risk, and immediately audible.
- **#1 and #3 are *amplifiers* of #2, not substitutes.** A developed, image-anchored tune (#2) in a key of its own (#1) and a committed texture (#3) is the whole win — but the tune is the thing you'd miss most if you fixed only one. Fix #2 first; let #1 and #3 frame it.

Recommended sequencing for the build sessions: **#2 Rank 1 + Rank 3 first** (rhythm + subject-anchoring — the fastest path to "it sounds like this image"), then **#2 Rank 2 + Rank 4** (vocabulary + development), then **#1** (per-image home), then **#3** (fuller-by-default pacing). Re-listen gate after each.

---

## 7. mappings.json coordination note (single-writer, shared with Music Theory)

`assets/mappings.json` is single-writer and shared with the Music Theory lens. I am **not** committing any rows — these are *proposed* additions for Music Theory to integrate, with the planner-side glue noted. All proposed rows live under `composition`.

**For #1 (home key-center) — proposed `composition.home_root` block** (new; consumed at composition.rs:1302):
```json
"home_root": {
  "default_midi": 60,
  "register_floor": 55,
  "register_ceil": 66,
  "rule": "hue_pc",
  "_comment": "home pitch class = round(dominant_hue/30) mod 12, seated into [register_floor, register_ceil]. Pure fn of dominant_hue. value_key nudge is OFF in v1 (spice S-3)."
}
```

**For #2 Rank 1 (motif rhythm) — proposed `composition.motif_rhythm` block** (new; consumed by `resolve_motif`, chord_engine 2379):
```json
"motif_rhythm": {
  "default_cell": [1, 1, 1, 1],
  "cells": [
    { "id": "even",     "durs": [1, 1, 1, 1] },
    { "id": "long_short", "durs": [3, 1] },
    { "id": "anapest",  "durs": [1, 1, 2] },
    { "id": "dotted",   "durs": [2, 1, 1] }
  ],
  "select": { "default": "even", "rules": [
    { "when": [{ "knob": "complexity", "op": "ge", "lo": 0.6, "hi": 0.0 }], "pick": "anapest" },
    { "when": [{ "knob": "edge_activity", "op": "ge", "lo": 0.6, "hi": 0.0 }], "pick": "dotted" },
    { "when": [{ "knob": "complexity", "op": "le", "lo": 0.25, "hi": 0.0 }], "pick": "long_short" }
  ]}
}
```

**For #2 Rank 2 (full vocabulary) — extend `pick_archetype` selection.** No new JSON row strictly required (the 8 contours live in code), but if the selector is to be data-driven, add a `composition.archetype` SelectTable mapping `(hue family, vertical_emphasis)` → one of the 8 archetype ids. Coordinate the id strings with Music Theory's `MotifArchetype` enum names (chord_engine 2320–2338): `arch, inverted_arch, descent, ascent, neighbor_turn, leap_step, pendulum, rising_sequence`.

**For #2 Rank 4 (development) — extend the variation vocabulary.** The `variation` field today is `{Identity, Fragmented}` (clamped, composition 1516). Adding `Inverted | Sequenced | Augmented | Diminished` is a **code+JSON** change Music Theory owns (it touches `theme_melody_pitch`). I propose the *form-catalogue* rows gain these on the Development/Contrast/Coda sections, e.g. `"variation": "Inverted"` on a B/Contrast section — but the enum + realizer support is Music Theory's to land first.

**For #3 (fuller-by-default pacing) — proposed `composition.density` tuning block** (new; consumed at composition 1450 and the gain at chord_engine 1403):
```json
"density": {
  "neutral": 0.5,
  "compose_center": 0.62,
  "span": 0.15,
  "floor": 0.47,
  "ceil": 0.77,
  "activity_gain": 0.9,
  "role_bias": {
    "Statement": 0.10,
    "Contrast": 0.15,
    "Development": 0.15,
    "Return": 0.10,
    "Coda": -0.10
  },
  "pre_return_dip": 0.20,
  "_comment": "IDENTITY/home_only sections stay pinned at neutral 0.5 for the byte-freeze hinge; compose-path sections use compose_center + role_bias. pre_return_dip subtracts on the last sub-span of the section before a Return."
}
```

**Coordination ask to Music Theory:** the `density` activity_gain and the `motif_rhythm` precedence are the two places our lenses overlap most — please confirm the gain stays musical at the realizer (§5.5) and define motif-rhythm-vs-realizer-rhythm precedence (§5.4) before either of us writes those rows. I will not touch `mappings.json`; treat these as a spec for your single-writer pass.

---

*End of S38 design note. Every guard-rail (GR-1 … GR-9) is stated to be confirmable or rejectable by the owner's trained ear; the automated proxies are conveniences, not the gate.*

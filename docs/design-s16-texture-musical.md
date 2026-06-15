# Design S16 — Texture & Density: From a Thin Line to a Full Texture (saliency-driven)

**Author role:** Music Theory Specialist (DESIGN ONLY — no source, test, or asset modified by this document).
**Date:** 2026-06-15
**Status:** Proposal for operator steer + Rust Architect engine synthesis. Companion engine design (the `pure_analysis` saliency extension + the realizer plumbing) is being authored in parallel by the Rust Architect; §3 and §5 flag every place this design needs a type-shape or seam change so the two reconcile.
**Builds on:** the now-shipped S15 PLAN-FIRST composer ([`design-s15-variety-musical.md`](./design-s15-variety-musical.md) Slice 1, `composition.rs` + the returning-theme seam in `chord_engine.rs`) and the canonical [`assessment-composition-architecture.md`](./assessment-composition-architecture.md) 10-stage roadmap. This document opens a roadmap dimension the S14 assessment never enumerated: **TEXTURE/DENSITY**, and shows it is the same problem as the roadmap's **Stage 9 SALIENCY** — they merge.

---

## EXECUTIVE SUMMARY (read this first)

The S15 output is **distinct per-image and has form/return** — but it sounds **near-monophonic and ethereal** because, on a default 4-instrument run, only the single Melody instrument carries a real line, the single Bass sounds one root, and the 2 HarmonicFill instruments **frequently rest** (`rest-as-gesture`) or sustain one inner tone with no independent motion. There is **no counterpoint** (no second moving line) and **no sustained harmonic pad** (the fill stabs at the step rate instead of holding under the phrase). The operator's verdict — "missing at least half the melody, all the counterpoint, all the harmony, all the background" — maps precisely onto these four code mechanisms. The cure is a **target texture model of 4–5 stratified layers** (MELODY · COUNTER-MELODY · HARMONIC PAD · BASS · optional AMBIENCE), where density itself becomes a **sectional and phrase-positional compositional device** (full in A/A′, thinner in B), and where a **3-region subject/foreground/background reading of the image drives which layers sound and how** — unifying "more texture" with the operator's "tie it to subject/fg/bg" into one mapping. The single biggest-bang first slice is **Slice 1: the SUSTAINED HARMONIC PAD** — give HarmonicFill a phrase-length held chord under the melody, killing rest-as-gesture and the per-step stab. That one change directly answers "all the harmony and all the background" with the least new machinery, no new image feature, and a back-compat default that keeps `engine_equivalence` byte-green. Texture-density becomes a **new roadmap Stage (call it Stage 2.5 / 9-merged)**, deeply coupled to Stage 9 saliency.

---

## 1. WHY IT SOUNDS SPARSE — grounded in the code

### 1.1 How many voices actually sound per step today

The default run is **4 instruments** (`EngineConfig::default` → `num_instruments: 4`, `engine.rs:201`; main.rs:286). `instrument_role(inst_idx, num_instruments)` (`chord_engine.rs:863`) stratifies them: index 0 → **Bass**, index 3 → **Melody**, indices 1–2 → **HarmonicFill**. So the nominal texture is Bass + 2×Fill + Melody = four sounding parts. That *should* be a full-enough texture. It is not, because of what each role actually emits in `realize_step`/`realize_rhythm`. The four failures below are independent and each subtracts a layer the operator named.

### 1.2 "Missing all the HARMONY and all the BACKGROUND" → the HarmonicFill is a stab, not a pad, and often a REST

The HarmonicFill arm of `realize_rhythm` (`chord_engine.rs:1283`–`1296`) is the worst offender:

```rust
let weak_interior = !step.position_in_phrase.is_multiple_of(2);
if edge < 0.15 && weak_interior {
    Vec::new()                               // rest-as-gesture: emit NO event
} else {
    vec![sustained(0, step_ms, base_frac)]   // ONE note, ONE step long
}
```

Two defects make the harmony vanish:

- **`rest-as-gesture` fires constantly on real photos.** The `edge < 0.15` guard reads the **raw, un-normalized** per-bar edge density (`features.edge_density.clamp(0,1)`, line 1173), and real photos carry raw edge ≈ 0.005–0.05 (documented all over S13: `EDGE_ACTIVITY_RANGE_MAX = 0.05`). So `edge < 0.15` is **true for essentially every real image**, and on every weak interior beat **both fill voices go silent**. The inner harmony — the body of the chord — is dropped roughly half the time. That is the literal mechanism behind "missing all the harmony."
- **When it does sound, it's a one-step stab, not a pad.** Even on the strong beats where it sounds, the fill emits `sustained(0, step_ms, base_frac)` — a single note exactly **one step long**, re-articulated every step. A harmonic pad is defined by *sustaining across the harmonic rhythm* — holding the chord under several melody notes. Re-striking the inner voice every step at the same rate as everything else gives a pulsing block-chord, not a background bed. There is **no layer that holds longer than one step**, anywhere, except the cadence ring. Hence "all the background" is missing: nothing sits underneath.

### 1.3 "Missing all the COUNTERPOINT" → there is no independent second line, by construction

Counterpoint = **two or more simultaneously sounding, rhythmically and melodically independent lines.** The code has exactly **one** melodic line (the single Melody instrument) and everything else is vertical chord-tone doubling:

- `role_pitch` (`chord_engine.rs:980`) gives Melody the **top chord tone**, Bass the **root**, Fill an **inner chord tone** — all drawn from the *same* current chord. They are a voicing of one chord, not independent lines. The fill never moves *against* the melody; it has no contour of its own.
- The S15 theme seam (`theme_melody_pitch`, line 1549) gates **only on `role == Melody`** (line 1558: `if role != OrchestralRole::Melody { return None; }`). So the returning theme — the one genuinely melodic gesture in the system — is carried by **one voice only**. No voice imitates, answers, or counters it.
- The voice-leading engine (`voice_lead_sequence`/`voice_lead_one`, lines 539/1816) is excellent *vertical* craft — smooth chord-to-chord connection, common-tone retention, no parallel perfects — but it re-voices **chords**, it does not generate **lines**. It is homophony done well, which is exactly the ceiling: at best block-chord homophony, never polyphony. So "all the counterpoint" is missing because **no mechanism produces a second independent line.**

### 1.4 "Missing at least half the MELODY" → the melody itself is gapped by articulation + rhythm

Even the one real line is thinned by three mechanisms that, stacked, hollow it out:

- **Staccato hold-fractions on the active figures.** The Melody arpeggio/syncopation/dotted arms (`chord_engine.rs:1304`–`1334`) use `STACCATO_FRAC = 0.40` for the short notes — sounding only 40% of their slot. On busy images (`edge_activity > 0.55/0.80`) the melody is mostly **silence between short attacks**: the "uniformly-short computer notes" again, now in a sparse texture so the gaps read as missing notes.
- **The articulation window low is 0.55.** `ARTIC_WINDOW_LO = 0.55` (line 1139): on busy images the sustained-melody fraction floors at 0.55, so even the "long" notes leave 45% of the slot empty. In a one-line texture every gap is audible as a hole.
- **The Fragmented (B) section rests the melody entirely past the head.** `theme_melody_pitch` returns `Some(None)` for Fragmented sections past the half-motif (line 1585), so the **Melody role emits nothing** for much of B — and because nothing else fills, B is the sparsest section of all. The operator literally loses the melody for a whole section.

### 1.5 The compounding effect

Each layer's thinning is individually defensible (rest-as-gesture is a real articulation; staccato is a real articulation; Fragmented-rests are a real developmental device). But they were authored **assuming a full texture underneath** — and there is none. With no pad to hold the harmony and no counter-line to fill the melody's gaps, every individual "tasteful silence" becomes a **hole in an already-thin fabric**. The fix is not to remove the silences; it is to **build the layers they were silences *within*.**

---

## 2. THE TARGET TEXTURE MODEL

### 2.1 The five layers (registers, rhythmic role, density)

A "full but not muddy" texture for this engine is **4 sounding strata, optionally 5**, with clear register separation (the orchestration principle: wide spacing low, close spacing high, no crowding one octave). Naming and binding to existing roles:

| Layer | Register (MIDI floor) | Pitch material | Rhythmic role | Maps to today |
|---|---|---|---|---|
| **MELODY** | `MELODY_REGISTER_FLOOR = 67` (G4) ± brightness | the theme / free-selected top tone | most active; the tune; carries the motif | `OrchestralRole::Melody` (exists) |
| **COUNTER-MELODY** | between fill and melody, ~60–67 (C4–G4) | an *independent* diatonic line, contrary/oblique to the melody | moderately active; moves when melody holds, holds when melody moves | **NEW sub-role of HarmonicFill** (§2.3) |
| **HARMONIC PAD (inner voices)** | `FILL_REGISTER_FLOOR = 55` (G3) | sustained inner chord tones (3rd/5th/7th) | **least active — HOLDS across the harmonic rhythm** | `OrchestralRole::HarmonicFill` (exists, but stabs/rests) |
| **BASS** | `BASS_REGISTER_FLOOR = 36` (C2) | chord root, occasional stepwise passing | steady, sparse; the harmonic floor | `OrchestralRole::Bass` (exists) |
| **AMBIENCE** *(optional, 5th)* | very low (≤ 36) or very high (≥ 84) | a pedal tone (tonic/dominant) or a high shimmer | sustained drone; sounds only when the image warrants a "background" | **NEW, optional** (§2.4) |

**How many simultaneous voices is "full but not muddy"?** For this register span (C2–C8) and a mostly-diatonic vocabulary, **4 sounding pitches at once is the sweet spot** (root + two inner + melody, the classic four-part texture), with a **5th (counter-melody or ambient pedal) added only in the densest sections.** Beyond 5 simultaneous lines in this narrow harmonic language muds quickly — the discipline is *register separation and rhythmic differentiation*, not voice count. The rule of thumb to hand the implementer: **never two voices on the same pitch class in the same octave** (the existing `upper_voices_well_spaced`, line 1651, already enforces the unison-collapse floor — extend its spirit to the new layers), and **keep ≥ a third between the two inner voices.**

### 2.2 Density as a compositional device (sectional + phrase-positional)

Density must **vary**, not be constant — a constant full texture is as monotonous as a constant thin one. Two axes, both already have a home in the data model:

- **Sectional density** rides the existing `Section.density: f32` field (`composition.rs:421`, currently a no-op default 0.5) and the `thematic_role`:
  - **A (Statement):** medium-full — establish the texture (melody + pad + bass; counter-melody enters on the repeat).
  - **B (Contrast):** **thinner and differently colored** — drop the pad to a pedal, or drop the bass and float the upper voices; this is what makes B *contrast* texturally, not just harmonically. (And it gives the Fragmented melody-rests somewhere to breathe instead of becoming holes.)
  - **A′ (Return):** **fullest** — all layers including counter-melody; this is the texture climax that makes the return *land*. Ties directly to §A.7's "A′ is the harmonic high-water-mark of resolution."
- **Phrase-positional density** rides `position_in_phrase`/`PhrasePosition`: thin at phrase starts (let the melody announce), thicken into the interior, and **the pad sustains *through* the phrase** so the harmonic rhythm is felt as a held bed under the melodic activity. Counterpoint enters mid-phrase (the melody has stated, now answer it).

The governing principle (carried from S15 §0): **a principled DEFAULT density per (role, section, phrase-position), with image-conditioned departures.** The default alone is a complete full texture; saliency (§3) varies it away.

### 2.3 Counter-melody — concrete, bounded rules (NOT "add counterpoint")

The counter-melody is a **new sub-behavior of the HarmonicFill role** (so it needs no new orchestral role and no new instrument), produced as **first-species-leaning, then sparingly second-species, motion against the melody.** It must reuse the existing voice-leading craft, not reinvent it. Bounded rules for the implementer:

1. **It is a real line:** it has its own pitch from step to step, selected as a **chord tone nearest its previous pitch** (reuse `upper_voice_candidates` / the `voice_lead_one` nearest-tone search, lines 1748/1816) — so it is conjunct and connected, never a random inner tone.
2. **Independence by contrary/oblique motion against the melody** (the single most important counterpoint rule): when the **melody moves up**, the counter-line **moves down or holds**; when the melody **holds**, the counter-line **moves**. This is computable because in compose mode both lines are known at plan time (the melody is the theme motif; see §4) — the counter-line is selected *relative to the melody's next move*. Concrete bound: **forbid similar motion into a perfect fifth or octave** between melody and counter-line (extend `has_parallel_perfects`, line 1782, which already computes this for voicings — apply it to the (melody, counter) pair across T→T+1).
3. **Rhythmic complementarity (the "fill the gaps" rule):** the counter-line preferentially **sounds where the melody rests or holds** — when the melody is in a staccato arpeggio (gaps), the counter-line sustains; when the melody sustains, the counter-line may move. This is what makes the texture sound continuous instead of pulsing in lockstep, and it directly repairs the §1.4 "half the melody missing" hole by **filling it with a second voice** rather than by lengthening the melody (which would fight the articulation craft).
4. **Conservatism:** first-species (note-against-note, consonant) is the floor; allow **passing tones** (the existing non-chord-tone vocabulary the engine documents but barely uses) on weak beats only, resolving by step. No suspensions/appoggiaturas in the first cut — those are a later refinement.
5. **It is OPTIONAL by density:** counter-melody sounds only in medium-full and full sections (A on repeat, A′), and only when there are ≥ 3 instruments so a fill slot exists. With 2 instruments it does not appear (the texture is just melody + bass), preserving the existing degenerate-count behavior.

### 2.4 The relationship to the existing voice-leading craft

Critically: **counterpoint is a NEW independent line, not the same thing as the existing voice leading.** The existing `voice_lead_sequence` connects the *chords* (vertical sonorities) smoothly; it does not create *horizontal* independence. The counter-melody is horizontal: it is selected as its own line that happens to use the chord's tones. They compose cleanly — the pad/bass voicing comes from `voice_lead_sequence` (unchanged), and the counter-melody is a *separate selection* layered on top, constrained against the melody by the rules above. **No rewrite of voice leading; an additive line above it.**

---

## 3. THE SALIENCY → TEXTURE MAPPING (the operator's core request)

The operator: *"a lot of this will need to be integrated into the subject/foreground/background analysis/scanning."* This is the unification. We define a **3-region reading** of the image and map each region to a **layer of the texture**, so "more texture" and "tie it to subject/fg/bg" become one coherent mapping.

### 3.1 The three regions (heuristic, pure-Rust, no ML)

Per the roadmap's Stage-9 §B.2.d cheap proxy (no learned model, no semantics):

- **SUBJECT** — the salient foreground object. Cheap proxy: the **center region** (and/or the highest-saliency blob via a DoG center-surround mask, `imageproc::gaussian_blur_f32`). Already stubbed on `ImageUnderstanding`: `subject_size`, `subject_hue`, `subject_saturation` (`composition.rs:74`–`82`).
- **FOREGROUND/MIDGROUND** — the active region around the subject: the **border-minus-subject** band, carrying secondary detail/energy. Proxy: `quadrant_contrast`, `secondary_hue`, the non-center mass.
- **BACKGROUND** — the low-saliency field: the **outer border / low-detail region**. Proxy: `border_saturation`, `value_key`, `fg_bg_contrast` (`composition.rs:81`).

### 3.2 The mapping (region → layer)

| Image region | Drives this layer | Concrete musical decision | Image property (knob) |
|---|---|---|---|
| **SUBJECT** | **MELODY** (register, contour, prominence) | melody register raised when subject is high in frame; melody **louder/more prominent** when subject pops (high `fg_bg_contrast`); theme **contour range** from subject size (big subject → wider, bolder line) | `subject_size`, `subject_hue` (→ which scale degree the motif centers on), `fg_bg_contrast` (→ melody velocity bias), `vertical_emphasis` (→ register) |
| **FOREGROUND** | **COUNTER-MELODY / active inner voices** | counter-melody **present and active** when the foreground is busy; its **density** tracks foreground energy; it is **absent** (texture falls to melody+pad+bass) when the foreground is quiet | `quadrant_contrast`, `edge_activity`, `complexity` |
| **BACKGROUND** | **HARMONIC PAD + BASS + AMBIENCE** | a **darker/low-key** background → fuller, lower, more sustained pad + an **ambient pedal** (the 5th layer, §2.4); a **bright/empty** background → thinner, higher pad, no pedal; background hue can **color** the pad (a borrowed chord, reusing S13 mixture) | `value_key`, `avg_brightness`, `dominant_hue_mass`, `colorfulness` |
| **DETAIL / energy (cross-region)** | **rhythmic activity & ornamentation** | high overall `edge_activity`/`texture` → more onsets, passing tones, denser counter-line (the existing `realize_rhythm` band selection, recalibrated per layer) | `edge_activity`, `texture` |

The throughline: **subject → the tune; foreground → the counter-line; background → the harmony/bed; detail → the rhythm.** A representational photo with a clear subject on a quiet ground gets melody-forward-over-pad (the natural "song" texture); a busy abstract gets an active counter-line and dense rhythm; a dark moody image gets a deep sustained bed with a pedal. **That is the music tracking the image's structure instead of its average** — the precise attack on "unrelated to the image."

### 3.3 What image features are NEEDED but don't exist yet

For the Rust Architect to spec the `pure_analysis` extension (all pure-Rust-heuristic, no ML — staying inside the §B.1 heuristic tier):

- **`subject_size` / `subject_hue` / `subject_saturation`** — currently **default stubs** (`subject_size: 1.0`, `subject_hue == dominant_hue`, `subject_saturation == avg_saturation`, `pure_analysis.rs:491`–`493`). Need the **center-region vs border-region** computation (cheap crop math) and optionally a **DoG saliency mask**. This is Stage-9 work; texture needs it.
- **`fg_bg_contrast`** — currently `0.0` stub (`pure_analysis.rs:494`). Need `center_saturation − border_saturation` (or mask-inside vs mask-outside). This is the single most load-bearing new knob (it gates counter-melody presence and melody prominence).
- **`border_saturation` / `center_saturation`** — not on `ImageUnderstanding` at all yet; needed as the cheap proxy underlying `fg_bg_contrast` and the pad coloring.
- **`quadrant_contrast`, `vertical_emphasis`, `mass_centroid`** — stubbed (`composition.rs:65`–`72`); §B.2.c cheap quadrant/half means. Texture uses `quadrant_contrast` for foreground energy and `vertical_emphasis` for melody register.

**Latent seam already present (important):** the per-step scan already splits each scan-bar into **N horizontal sections** (`pure_analysis.rs::scan_steps`, the `num_bars` inner loop, line 658), and **the section index already maps to a vertical band of the image** — top sections vs bottom sections. The instrument stratification (Bass=0=lowest band, Melody=highest index=top band) means **the engine is already reading vertical image regions per instrument; it just isn't using them as fg/bg.** A cheap first cut of "background = bottom band, subject = a salient middle/top band" can ride this existing geometry **before** the full saliency mask lands — worth flagging to the Architect as the lowest-cost saliency bootstrap.

---

## 4. MAKING THE RETURNING THEME/MOTIF MORE MUSICAL & READABLE

The operator: *"it even seemed like I heard recurring motifs (but they were not so musical so it was hard to tell for sure)."* The motif machinery (S15, `resolve_motif`/`theme_melody_pitch`) is structurally correct but **under-projected**. Four concrete fixes — all in the existing functions, no new types:

1. **Give the motif a RHYTHMIC IDENTITY.** Today `resolve_motif` hard-codes `dur_steps: 1` for every motif note (`chord_engine.rs:1492`) — the motif is a pure pitch contour with **no rhythm of its own**, so its rhythm is whatever `realize_rhythm` happens to assign that step, which **changes between statement and return** (different `edge_activity` per bar). A motif with no fixed rhythm is **not recognizable** — rhythm is the most memorable feature of a tune. **Fix:** let `resolve_motif` assign a real `dur_steps` pattern (e.g. long-short-short-long, or the character's rhythmic signature) so the theme has a *fixed rhythmic profile* that recurs identically in A and A′. (The §A.6 "rhythmic profile from the character" already anticipated this; S15 deferred it to keep `dur_steps` augmentable in Stage 7 — but a *fixed* rhythm is required for recognizability and is compatible with later augmentation.)
2. **PROTECT the motif's register and prominence on return.** On the recap (A′), the melody must be **clearly on top and clearly louder** than the accompaniment so the ear locks onto it. Add a **melody-prominence bias in `realize_velocity`** for theme-carrying steps in Statement/Return sections (a few velocity points above the texture), and ensure the melody register floor is not pushed down by a dark image on the return (clamp the brightness register-drop for theme steps). Recognizability needs the tune to be *the loudest, highest thing.*
3. **Don't let the motif's notes be swallowed as non-chord tones.** `theme_pitch` (line 1603) already prefers the chord's seating when the degree is a chord tone — good. But when it *isn't*, the theme note is a dissonance that the sparse texture exposes harshly. **Fix:** on the recap specifically, **reharmonize lightly so the motif's structural notes are consonant** (bias `pick_progression` / the section's progression so A′'s downbeats support the motif's degrees) — this is the cheapest "reharmonization" variation and makes the return sound *intended*, not accidental.
4. **Repetition + immediate sequence within the statement.** A motif heard **once** in A is hard to remember by A′. **Fix:** state the motif, then **immediately repeat or sequence it** within section A (the existing `RisingSequence` archetype, line 1437, already encodes a sequenced cell — use that pattern: state the head, restate it up/down a step). Repetition is how the ear *learns* the tune before it has to *recognize* it. This is a `composition.rs` planner change (lay the motif twice in A), not a realizer change.

The combined effect: the motif gets a **fixed rhythm (1) + guaranteed prominence (2) + harmonic support (3) + repetition (4)** — the four classic recognizability levers. With them, "I think I heard motifs" becomes "the tune came back."

---

## 5. STAGING — one hearable slice at a time

### 5.1 The constraint

Each session builds **one** stage; the `engine_equivalence` byte-freeze must stay green (the behavior-neutral default plan reproduces today's goldens exactly), and any deliberate golden move is hand-re-derived in the same commit with a comment (S13/S15 discipline). Vocabulary that can be **data** lives in `mappings.json` (per-layer density defaults, per-section density bias rows); only true new **mechanism** is code.

### 5.2 The slice order (climbing out of sparsity)

Ordered by **bang-per-buck against the operator's specific complaints**, with each slice mostly independent:

| Slice | What changes musically | Hearable win | Independence | Answers |
|---|---|---|---|---|
| **S16-1: SUSTAINED HARMONIC PAD** *(RECOMMENDED FIRST)* | HarmonicFill **holds the chord across the phrase** (multi-step sustain, not a 1-step stab) and **stops resting** on real photos (fix the raw-edge `rest-as-gesture` bug); inner voices become a true bed | "all the harmony and all the background" appear — the texture gains a floor under the melody | Fully independent; touches only the HarmonicFill arm of `realize_rhythm` + the rest-guard threshold | "missing all the harmony / all the background" |
| **S16-2: COUNTER-MELODY** | a second independent line in the fill register, contrary/oblique to the melody, filling its gaps (§2.3 bounded rules) | "all the counterpoint" appears; the melody's holes fill in | Independent of pad once pad exists; needs ≥3 instruments | "missing all the counterpoint / half the melody" |
| **S16-3: MOTIF READABILITY** | fixed motif rhythm + prominence + repetition + light recap reharmonization (§4) | "the tune comes back" — motifs become legible | Independent; touches `resolve_motif` + `realize_velocity` + planner motif-layout | "motifs not musical / hard to tell" |
| **S16-4: SALIENCY → DENSITY (the Stage-9 merge)** | the 3-region reading drives sectional/phrase density: subject→melody, fg→counter, bg→pad/pedal; B thins, A′ fills | "the music tracks the subject, not the average" | Depends on S16-1..3 existing as the layers it modulates; needs the `pure_analysis` saliency extension | "integrate into subject/fg/bg analysis" |

### 5.3 The single biggest-bang FIRST slice

**S16-1: the SUSTAINED HARMONIC PAD.** Rationale:

- It answers **two** of the four named-missing layers at once ("all the harmony" + "all the background").
- It is the **cheapest** — it touches only the HarmonicFill arm of `realize_rhythm` (lines 1283–1296): (a) fix the `rest-as-gesture` guard to read the **normalized** `edge_activity` (so it stops firing on every photo — this is arguably a latent *bug*, not just a design change), and (b) emit a **phrase-length sustained** event for the fill instead of a one-step stab (hold the inner chord tone under the phrase's worth of steps, releasing at the next chord change).
- It needs **no new image feature** and **no new type** — pure realizer craft, exactly the Music-Theory-owned surface.
- It is **back-compat-safe:** the change is gated to the *non-cadence* HarmonicFill path; the cadence ring and the Bass/Melody arms are byte-untouched, so only the fill goldens move (re-derived by hand). The `engine_equivalence` default-plan anchor for Bass/Melody stays green.
- The hearable win is **immediate and exactly what the operator asked for:** a held harmonic bed appears under the melody, and the piece stops sounding hollow. Everything after (counterpoint, saliency) enriches a texture that now has a floor.

One honest tension to flag: a true *cross-step* sustain means the fill NoteEvent's `hold_ms` exceeds one `ms_per_step`, which the realizer can express (`hold_ms: u64` is unbounded and the cadence ring already overlaps via the 1.20 cap) — but the **adapter's per-step note_on/note_off scheduling** (main.rs:489–492) currently pairs every note_on with a note_off at step end. Sustaining across steps requires the adapter to **defer the note_off** until the pad's `hold_ms` elapses (or until the chord changes). This is a **seam coordination point with the Rust Architect** (the realizer expresses the long hold; the scheduler must honor it) — flagged here so it lands in the engine design, not as a surprise. If deferring note_off is out of scope for slice 1, the fallback is a **re-articulated pad with legato overlap** (`base_frac` ≥ 1.0 so consecutive fill notes tie) — less ideal but no scheduler change, and still a vast improvement over rest-as-gesture.

### 5.4 Reconciliation with the existing 10-stage roadmap

**Texture/density is a NEW dimension the S14 assessment did not enumerate** (the roadmap had Bass/Fill/Melody roles but assumed they produced a full texture — the listen proved they don't). It should be inserted as **Stage 2.5 (TEXTURE & DENSITY), immediately after the Stage-2 sectioned-plan slice that just shipped**, because a full texture is a prerequisite for every later stage being *audible* (meter, character, and climax all land harder on a full texture than a thin one). And **it merges with Stage 9 (SALIENCY):** Stage 9 was scoped as "region-saliency upgrade … melody-vs-accompaniment color split + theme prominence from the actual subject" — which is **exactly the §3 mapping**. The two are the same work approached from two ends (texture from the music side, saliency from the image side), so:

- **S16-1..3** (pad, counter-melody, motif readability) are the **texture half** — they build the layers, drivable by the *existing* knobs and section/phrase position. They become **Stage 2.5**.
- **S16-4** (saliency → density) is the **Stage 9 merge** — it wires the layers to the 3-region image reading, and pulls Stage 9 *earlier* in the roadmap because texture needs it to be more than sectional.

Net roadmap edit to recommend to the Architect: **insert Stage 2.5 (texture: pad → counterpoint → motif-readability) and fold the §3 saliency-mapping into Stage 9, noting Stage 9 now also *completes* the texture dimension.**

---

## RECOMMENDED SLICE 1

**Build the SUSTAINED HARMONIC PAD.** In one landed unit, in `chord_engine.rs`'s `realize_rhythm` HarmonicFill arm only:

1. **Kill the spurious rest:** change the `rest-as-gesture` guard from the raw `edge < 0.15` (which fires on every real photo) to the **normalized** `edge_activity < ~0.15` so a deliberate inner-voice silence is rare and intentional, not constant. (This is a latent bug fix as much as a design change — the threshold was authored against a normalization that the raw value never reaches.)
2. **Make the fill SUSTAIN:** emit a held inner-chord-tone event whose `hold_ms` spans the **phrase-length harmonic unit** (hold under the melody, release at the chord change), rather than `sustained(0, step_ms, base_frac)` (one step). Express the long hold in the NoteEvent; coordinate the deferred note_off with the Rust Architect (§5.3), with the legato-overlap re-articulation as the no-scheduler-change fallback.
3. **Keep it back-compat:** gate to the non-cadence HarmonicFill path; Bass/Melody/cadence-ring byte-untouched; re-derive the moved fill goldens by hand with a comment; `engine_equivalence` default-plan anchor stays green.

**Hearable result:** a held harmonic bed appears under the melody for the first time — the piece stops sounding hollow and "missing all the harmony / all the background." It is the smallest change that most directly answers the operator's dominant complaint, needs no new image feature, and gives every subsequent slice (counterpoint, motif readability, saliency) a real texture to enrich.

**Honest ceiling reminder (unchanged):** the target stays "principled, fits the image, pleasant" — a full, coherent texture tied to the image's structure, not "hand-composed." Adding layers widens the *fabric*; it does not raise the *ceiling*.

---

*End of S16 texture design. Design-only: no source, test, or asset modified by this document. Companion engine design (the `pure_analysis` saliency/region extension and the realizer/scheduler sustain plumbing) is the Rust Architect's; §3.3 and §5.3 enumerate the seam changes this design requires.*

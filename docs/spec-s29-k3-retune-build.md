# Spec S29 — K3 RE-TUNE BUILD: confirm the destination key + build MX-4 second-contrast + dom7 pivot

**Author role:** Rust Architect. **DESIGN ONLY.** This document modifies no source, test, or asset.
It produces exactly one artifact — this file. It is the buildable, per-file, per-lever contract that
an Implementer + a Music Theory Specialist transcribe against to make the K3 modulation
PERCEPTUALLY VISIBLE.

**Date:** 2026-06-16. **HEAD:** `dfcfb4c` (S28/K3 BUILT & CLOSED — "realizer pivot/common-tone
modulation + land-home cadence"; Quality Gate PASS, `docs/review-S28.md`). Every line number,
signature, and sha below is **re-verified against the working tree at `dfcfb4c`.**

**The problem this slice fixes (operator re-listen, verbatim):** *"Nothing sounded forced. The key
changes were not striking, and I was unsure if I even heard it. I could definitely hear the chord
changes, and those sounded normal."* Two read-only diagnostics converged: **K3 shipped the
ANNOUNCEMENT of a modulation (the pivot V at step 0) but neither the CONFIRMATION (a cadence in the
new key) nor the SCENE CHANGE (a second dimension of contrast).** This slice builds exactly the
confirmation + the second dimension, and nothing bolder.

**Builds on (do not restate):**
- `docs/spec-s28-k3-build.md` — the K3 build contract this re-tunes.
- `docs/input-s28-k3-pivot-harmony.md` — the V-of-destination pivot harmony (Lever 3 extends it).
- `docs/design-s26-multiexcursion-aesthetics.md` §1.3 / **Rule MX-4** — the second-dimension rule
  Lever 2 finally builds (specified S26, never wired).
- `docs/review-S28.md` — the K3 Quality Gate (PASS; the byte-anchors below are its verified anchors).

**APPROVED SCOPE — design EXACTLY these three levers. The bolder "striking" levers are explicitly
HELD for a later re-listen and MUST NOT be designed here:** no mode-change-at-B, no wider key menus,
no large dwell increases. (Risk §7 records why dwell stays a held knob.)

---

## 0. BYTE-FREEZE ANCHORS (re-verified at `dfcfb4c`, the load-bearing constraint)

| File | sha256 at `dfcfb4c` | S29 disposition |
|---|---|---|
| `src/engine.rs` | `e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261` | **MUST STAY FROZEN at this exact anchor.** This is the K3-re-baselined anchor (`review-S28` CHECK 1). S29 designs so engine.rs is **not edited at all** — see §5. |
| `src/chord_engine.rs` | `a891d4785d6836ddcb117b4c041d58725dc2f8b4fa77d1fde9e724d55ebce85f` | **sha WILL change** (Lever 3 voicing add + Lever 1 voice-leading rule). Freeze is BEHAVIORAL, witnessed by `engine_equivalence` 9/9 + the K3 identity test + `no_inversion_invariant`. |
| `src/composition.rs` | (not sha-anchored) | **Edited** (Lever 1 forced-tonic + opening-PAC; Lever 2 density wiring). Not a frozen file; witnessed behaviorally on the identity path. |
| `assets/mappings.json` | (not sha-anchored) | **Possibly edited** (Lever 2 may add ONE `feature_normalization`/coupling constant; Lever 1/3 add no data). Single-writer rule applies (§6). |

Frozen test guards that MUST stay GREEN (all re-pointed to the `e50c7db1…` anchor in K3):
`tests/engine_equivalence.rs` (9/9, goldens **240/114/84/36/79**), `tests/keyplan_s25.rs`
(`no_inversion_invariant`, `engine_equivalence_byte_green`), `tests/prominence_s23.rs`
(`no_inversion_invariant`, `engine_freeze_diff_empty`), `tests/affect_s22.rs`
(`byte_freeze_witness_locked_files_unmoved`), `tests/keyplan_k3.rs`
(`pivot_inserts_nothing_on_identity`, the K3 positive tests), `tests/keyplan_k2b.rs`
(`no_routed_image_ends_off_home`).

---

## 1. CURRENT STATE ANALYSIS — the exact mechanisms causing invisibility

### 1.1 Lever 1 — the destination key is ANNOUNCED but never CONFIRMED

**Mechanism (the V never resolves to I in the new key).** At a modulating `pivot:true` boundary the
realizer inserts the destination dominant (V/dest) at `step_in_section == 0`
(`chord_engine.rs:2188-2266`, `pivot_chord_events`; the bass sounds `(dest_root_pc + 7) % 12`, the
destination's V). That is the **announcement**. But step 1 onward is the section's own
`Vec<StepPlan>`, built at plan time by:

```
composition.rs:1205   let progression = chord_engine.pick_progression(&home_mode);
composition.rs:1206   let chords = chord_engine.generate_chords(&progression, section_root_midi, …);
composition.rs:1215   let steps = chord_engine.plan_phrases(&chords);
```

- `pick_progression` (`chord_engine.rs:119-145`) selects a progression string **with
  `thread_rng()`** (`:132`). The first symbol is therefore **not deterministically "I"** — it is
  whatever the RNG drew from the mode's family (e.g. `I-vi-IV-V` *or* a family whose head is not the
  tonic), and `generate_chords` may even prepend a secondary-dominant before symbol 0
  (`chord_engine.rs:240-256`, the `edge_activity > trigger` branch inserts V/next BEFORE the chord).
  **So `chords[0]` — the chord the pivot V is supposed to resolve INTO — is not guaranteed to be the
  destination tonic.** The pivot V→? is a non-resolution: the ear hears a dominant, then an arbitrary
  diatonic chord re-rooted at the new key. That reads as a **tonicization** (a momentary lean), not a
  **modulation** (an arrival in a new key).
- `plan_phrases` (`chord_engine.rs:637-758`) only stamps a `PerfectAuthenticCadence` at the section's
  **FINAL** phrase, and only when the chord immediately before it `is_dominant_name` (`:716-722`).
  There is **no opening cadence** confirming the new key early in the section. The new key is never
  cadentially closed until — at best — the section's end, many steps later, by which point the ear has
  long since lost the modulation as an event.

**Net:** the modulation is invisible because V/dest is sounded once and then abandoned; the new key
is asserted but never *cadentially confirmed*.

### 1.2 Lever 2 — there is no second dimension of contrast (and `Section.density` is DEAD)

**Mechanism (the key change carries 100% of the contrast, and a key change alone is the thinnest
contrast).** `docs/design-s26-multiexcursion-aesthetics.md` §1.3 / Rule MX-4 (`:162-169`) specified
that each excursion must differ in **≥ 2 dimensions, key + density mandatory**, and that the
energy-ordered region's energy should drive `Section.density`. **That coupling was never built.**
Verified:

- `composition.rs:1232` — `Section.density` is **HARDCODED to `0.5`** for every section in the
  planner loop. (Also `0.5` in the engine legacy fixture `engine.rs:766` and test fixtures.)
- **`Section.density` is WRITE-ONLY — it is never READ by any production code.**
  `grep -rn '\.density' src/` over a `Section` value returns **zero realizer reads** (the only
  `.density` reads are off `OrchestrationProfile.density`, a *different, also-unused* field at
  `composition.rs:390`). There is therefore no "density double-write against the S23 path" today —
  there is no reader at all. (This sharpens, not removes, the coordination risk: §7.)
- The data MX-4 needs is **computed and discarded.** `resolve_key_scheme` (`composition.rs:1459-1523`)
  ranks the two non-subject regions by `RegionAffect.energy` (`:1485-1491`, `bg.energy > fg.energy`)
  to pick *which* region is B vs C — then returns **only the offsets** (`Vec<i8>`). The per-section
  **energy itself is thrown away.** The "louder region sounds busier" link is unbuilt.
- The realizer's actual density/busyness knob is **`features.edge_density`** → `edge_activity`
  (`chord_engine.rs:1417`, `realize_rhythm`), which is the **GLOBAL per-bar image edge scan**
  delivered through `PerfFeatures` by the engine (`engine.rs:498-503`, built from
  `scan_bar_features`). The realizer has **no per-section density input wired at all.**

**Net:** the modulation lands in the same texture it left — same room, just a different chord root —
so the operator's ear, with no second cue, cannot register a *scene change*.

### 1.3 Lever 3 — the pivot is a bare triad, not unambiguously a dominant

**Mechanism.** `pivot_chord_events` (`chord_engine.rs:2214-2257`) voices the destination dominant as a
**triad only**: bass = `dom_root_pc`, fill = the common-tone hinge (+ implicitly the leading tone in
a 3+ ensemble), melody = `dom_fifth_pc`. There is **no chordal seventh** in the pivot. A bare V triad
is a weaker dominant signal than a V7 — the tritone (3rd↔7th) that makes a dominant *pull* is absent,
so even the announcement is softer than it could be. The dom7 pitch class
`(dom_root_pc + 10) % 12` is computed nowhere in the pivot.

---

## 2. PROPOSED CHANGES — per file, per lever

> **One-rule discipline (inherited from K3):** the pivot is ONE unified rule (V-of-destination) for
> all menu offsets `{+7,+5,+3,−3}`. S29 keeps that — it does not add per-offset tables.

### 2.1 LEVER 1 — confirm the destination key (forced tonic + opening tonicizing PAC)

**Owner:** the planner structure is the **Rust Implementer's** (`composition.rs`); the
voice-leading RULE for the V→I is the **Music Theory Specialist's** (`chord_engine.rs`).

**2.1(a) Force `chords[0]` to the destination ROOT-POSITION tonic — `composition.rs`.** In the
planner section loop, for a **modulating `pivot:true` section** (the section whose offset differs
from its predecessor), the first chord MUST be the destination tonic `I`, so the step-0 pivot V
resolves V→I into the new key on the step-1 downbeat. The cleanest transcribable rule reuses the
existing chord builder:

- Compute, once where `scheme_pivot`/`scheme_resolution` are resolved (`composition.rs:1108-1109`),
  the per-section "is this a modulating boundary" predicate from the resolved `offsets` vector:
  `let is_mod_boundary_i = scheme_pivot && i > 0 && offsets[i] != offsets[i-1];`
- When `is_mod_boundary_i`, after `generate_chords` returns, OVERWRITE `chords[0]` with a
  deterministic root-position destination tonic built at `section_root_midi`:

```rust
// composition.rs, in the section loop, AFTER `let chords = chord_engine.generate_chords(...)`:
let mut chords = chords;                       // make mutable
if is_mod_boundary {
    // Force the destination TONIC as the section's opening chord so the step-0 pivot V
    // resolves V->I in the new key. Root-position I built at the section root; the
    // Music Theory rule (§2.1(b)) governs its voicing/voice-leading inside chord_engine.
    if let Some(first) = chords.first_mut() {
        *first = chord_engine.tonic_triad(section_root_midi, &home_mode);
    }
}
```

This needs ONE new public helper on `ChordEngine` (deterministic, no RNG) — the Implementer adds the
skeleton, the Music Theory Specialist confirms the chord content:

```rust
// chord_engine.rs — NEW public helper (deterministic root-position destination tonic).
/// Build the ROOT-POSITION tonic triad ("I") at `root_midi` in `mode`, with no RNG and no
/// secondary-dominant/mode-mixture additions — the deterministic destination-tonic the S29
/// opening tonicizing PAC requires. Reuses the same scale/`roman_to_chord_complex` machinery
/// `generate_chords` uses, at `HarmonicComplexity::Triad`, so the chord tones are identical to
/// a free-selected "I" but the SELECTION is forced. Name == "I".
pub fn tonic_triad(&self, root_midi: u8, mode: &str) -> Chord;
```

*Transcription note:* the body selects the mode scale (same `match mode { … }` as `generate_chords`
`:170` head) and calls the existing private `roman_to_chord_complex("I", root_midi, &scale,
HarmonicComplexity::Triad)` (`chord_engine.rs:302-340`). This is a deterministic re-use, not new
harmony.

**2.1(b) Stamp an OPENING tonicizing PAC so the new key is cadentially confirmed early —
`chord_engine.rs` (`plan_phrases`).** Today `plan_phrases` stamps a PAC only at the section's FINAL
phrase (`:709-722`). Lever 1 requires the OPENING phrase of a modulating section to also be readable
as an authentic cadence in the new key: the step-0 pivot supplies the dominant; step-1 `chords[0]` is
now the forced destination I (§2.1(a)). The opening V→I must be *honored* as a confirming cadence.

Two transcribable options — the Music Theory Specialist picks; **default = Option A** (no
`plan_phrases` structural change, minimal blast radius):

- **Option A (DEFAULT) — honor the opening V→I via VOICE-LEADING only, no new stamp.** The pivot
  already lands V/dest at step 0; the forced I lands at step 1. The Music Theory Specialist adds the
  **voice-leading requirement** so this reads as a true authentic cadence rather than two
  root-position triads stacked (which a trombonist hears as parallel octaves):
  - the **new key's leading tone resolves UP by semitone to the new tonic** (the V's major third →
    the I's root);
  - any **chordal 7th resolves DOWN by step** (the dom7 added in Lever 3 → the I's third);
  - the bass moves V-root → I-root (root-position to root-position is acceptable for the BASS; the
    parallel-octave hazard is in the UPPER voices, which the leading-tone/7th resolution above
    prevents).
  This is realized at the step-1 chord by a small voice-leading constraint on the forced-tonic
  voicing relative to the prior pivot chord. It adds NO step and NO stamp — it constrains the pitches
  of an already-present chord. **This is the recommended path: it keeps `plan_phrases` byte-stable
  for every non-modulating section.**

- **Option B (only if the ear demands an explicit opening cadence label) — stamp `PhrasePosition::
  PerfectAuthenticCadence` at the opening boundary of a modulating section.** This would require
  `plan_phrases` to know the section is a modulating pivot section (a new param threaded from the
  planner) and to stamp the opening phrase's resolution chord as a PAC. **Larger blast radius**
  (changes `plan_phrases`'s public-ish behavior and risks the velocity/structural goldens). Held as
  the fallback; do NOT build it unless Option A's voice-leading proves insufficient on re-listen.

**Why Lever 1 needs no engine.rs touch:** §2.1(a) is a plan-time chord overwrite in `composition.rs`;
§2.1(b) Option A is a voice-leading rule inside `chord_engine.rs`. Both are gated on
`is_mod_boundary` / the pivot path, which is dead on every identity/home_only/`pivot:false` section.

### 2.2 LEVER 2 — build MX-4: drive `Section.density` from region energy AND make it AUDIBLE

**Owner:** **Rust Implementer** (all of it — planner structure + the realizer density read).
`composition.rs` only, plus possibly ONE `chord_engine.rs` read and ONE `mappings.json` constant.

This lever has TWO halves, because today `Section.density` is **dead** (§1.2): (i) SET it from
energy (the MX-4 planner coupling), and (ii) READ it in the realizer so it is actually audible.
**Both are required** — building only (i) reproduces the existing dead field.

**2.2(i) Propagate per-section energy out of `resolve_key_scheme` and SET `Section.density` ONCE —
`composition.rs`.** `resolve_key_scheme` already ranks regions by energy (`:1485-1491`) but returns
only `Vec<i8>`. Extend it to ALSO return, per section, the energy of the region that section's offset
was drawn from (whole-image fallback energy for home/fallback sections):

```rust
// composition.rs — resolve_key_scheme return type change (planner-internal fn, not public API):
//   BEFORE: fn resolve_key_scheme(...) -> Vec<i8>
//   AFTER:  fn resolve_key_scheme(...) -> Vec<(i8, f32)>   // (offset, source_region_energy_0..1)
// For OffsetRule::Home  → (0, HOME_ENERGY_NEUTRAL)         // a home section carries the neutral energy
// For OffsetRule::Excursion(rank) → (offset, ranked.get(rank).map(|r| r.energy).unwrap_or(0.0))
// The Resolve final-section force still sets offset.0 = 0 (energy untouched — a Coda home section
//   that was an excursion keeps its source energy so it can still carry a density bias if wanted;
//   but see the home-clamp below).
```

Set `Section.density` from that energy in the planner loop, **replacing the `0.5` hardcode at
`composition.rs:1232`** — set exactly once, here, and nowhere else:

```
density = f(energy)  where  f(e) = DENSITY_NEUTRAL + DENSITY_ENERGY_SPAN * (e - 0.5)
                            clamped to [DENSITY_FLOOR, DENSITY_CEIL]
```

with the BIAS deliberately MODEST ("a different room, not a different piece"):

| const | value | rationale |
|---|---|---|
| `DENSITY_NEUTRAL` | `0.5` | the byte-stable identity midpoint (what `0.5` meant) |
| `DENSITY_ENERGY_SPAN` | `0.30` | a high-energy region (e=1.0) → 0.65; a calm one (e=0.0) → 0.35 — a felt but small lift |
| `DENSITY_FLOOR` / `DENSITY_CEIL` | `0.35` / `0.65` | the room never gets so dense/sparse it reads as a different piece |

**CRITICAL single-writer / byte-stability rule (the coordination check the prompt demands):**
- `Section.density` is set in **EXACTLY ONE** place — the planner loop literal at `:1232`. No other
  code path writes it. The S23 prominence path writes `orchestration` / the prominence Vec, NOT
  `Section.density`; they are disjoint fields, so there is no double-write. (The dead
  `OrchestrationProfile.density` at `:390` is NOT touched by this lever.)
- **The home_only / identity path MUST keep `density == 0.5` byte-for-byte.** Guarantee: on
  `home_only`, `resolve_key_scheme` returns all `(0, HOME_ENERGY_NEUTRAL)` where
  `HOME_ENERGY_NEUTRAL == 0.5`, so `f(0.5) == DENSITY_NEUTRAL == 0.5` exactly. The hardcoded-`0.5`
  fixtures (`engine.rs:766`, test fixtures) stay `0.5` and remain valid. **Set
  `HOME_ENERGY_NEUTRAL = 0.5` precisely so `f(HOME_ENERGY_NEUTRAL)` is the algebraic identity — this
  is the byte-stability proof.**
- **Home sections of a modulating piece also stay `0.5`.** A `home`-rule section maps to
  `(0, 0.5)` → density 0.5, so only the EXCURSION sections (where the new key arrives) carry a
  contrasting density — which is exactly the MX-4 intent: the excursion is a change of scene; home is
  home.

**2.2(ii) READ `Section.density` in the realizer so it is AUDIBLE — `chord_engine.rs` (gated).**
`Section.density` must modulate the realizer's busyness. The realizer's busyness knob is
`edge_activity` in `realize_rhythm` (`chord_engine.rs:1417`). Bias it by the section density, read
zero-copy off the already-borrowed `ctx.section.density`:

```rust
// chord_engine.rs, realize_rhythm, replacing the bare edge_activity computation at :1417:
//   BEFORE: let edge_activity = (features.edge_density / EDGE_ACTIVITY_RANGE_MAX).clamp(0.0, 1.0);
//   AFTER:
let edge_activity = {
    let base = (features.edge_density / EDGE_ACTIVITY_RANGE_MAX).clamp(0.0, 1.0);
    // S29/MX-4: a denser section nudges activity UP, a sparser one DOWN. ctx.section.density
    // is 0.5 (DENSITY_NEUTRAL) on EVERY identity/home/home_only section, so this term is
    // EXACTLY 0.0 there → edge_activity is byte-identical to pre-S29 on the identity path.
    let density_nudge = (ctx.section.density - 0.5) * DENSITY_ACTIVITY_GAIN;
    (base + density_nudge).clamp(0.0, 1.0)
};
```

with `DENSITY_ACTIVITY_GAIN` (a `chord_engine.rs` const, e.g. `0.5`) sized so the ±0.15 density swing
(0.35..0.65) shifts activity by ≤ ±0.075 — modest. `DENSITY_NEUTRAL == 0.5` makes
`density_nudge == 0.0` on the identity path, which is the byte-freeze hinge for this read.

> **Texture/figuration (the OPTIONAL second half of Lever 2, §2.2 of the prompt, "if clean"):** the
> prompt allows ALSO contrasting texture/figuration for the Contrast/Development section role. This is
> **DEFERRED within this slice** unless trivially clean: the existing `OrchestrationProfile` / texture
> `SelectTable` path (`composition.rs:763`, `texture_catalogue`) is the right home for it, but driving
> it from energy is a second planner coupling with its own byte surface. **Recommendation: ship the
> density read (audible, low-risk) in S29; hold texture/figuration as a named fast-follow** so the
> first re-listen judges density-as-scene-change alone (cleaner attribution). If the lead wants both,
> it is additive and disjoint, but it is NOT required for MX-4's "key + density mandatory" floor.

### 2.3 LEVER 3 — dominant 7th in the pivot (cheap realizer add) — `chord_engine.rs`

**Owner:** **Music Theory Specialist** (it is harmony, inside `pivot_chord_events`).

Add the dominant 7th `(dom_root_pc + 10) % 12` to the pivot voicing so it is unambiguously a
dominant. **Which voice carries it, without breaking the no-inversion frame (bass < fill < melody):**
the **FILL/inner register**, alongside (or in place of, for a 2-instrument ensemble) the common-tone
hinge — the 7th is an inner-voice color tone by nature, and seating it via
`seat_pc_in_register(dom_seventh_pc, FILL_REGISTER_FLOOR …)` keeps it strictly between the bass (dom
root) and the melody (dom fifth), so `no_inversion_invariant` holds by construction.

```rust
// chord_engine.rs, pivot_chord_events, after dom_fifth_pc (:2216):
let dom_seventh_pc = (dom_root_pc + 10) % 12;  // minor 7th above the dominant root = the V7 color
```

Voice assignment rule (transcribable):
- **Bass** — unchanged: `dom_root_pc` at the bass floor (root-position V7).
- **Melody** — unchanged: `dom_fifth_pc` at the melody floor (a stable top tone over the V7).
- **Fill / inner (HarmonicFill | Pad | CounterMelody)** — sound the **dom7** (`dom_seventh_pc`)
  seated at the fill floor when the ensemble has a dedicated inner voice; the common-tone hinge
  (§2.2 of `input-s28`) is preserved by giving the 7th to the inner voice and keeping the hinge as
  the *resolution target* (the 7th resolves DOWN to the destination tonic's third on the step-1 I —
  which is exactly the Lever 1 §2.1(b) voice-leading rule, so Lever 1 and Lever 3 dovetail).
  For a 1- or 2-instrument ensemble (no dedicated fill), the 7th is omitted (no inner voice to carry
  it) — the bare-triad pivot remains for those, which is acceptable and byte-irrelevant to the
  no-inversion guard.

**Voice-leading the 7th's resolution (the dovetail with Lever 1):** the dom7 (the leading dissonance)
must resolve DOWN by step into the step-1 forced tonic — this is the SAME rule as §2.1(b)'s "any 7th
resolves DOWN," realized across the pivot→I boundary. Specify it once, in the Music Theory input doc,
covering both the pivot's 7th and the opening cadence.

---

## 3. INTERFACE DEFINITIONS — complete Rust signatures (no bodies)

```rust
// ── src/chord_engine.rs ─────────────────────────────────────────────────────────

/// NEW (Lever 1) — deterministic root-position destination tonic for the forced opening I.
/// No RNG, no secondary-dominant/mode-mixture; reuses `roman_to_chord_complex("I", …, Triad)`.
pub fn tonic_triad(&self, root_midi: u8, mode: &str) -> Chord;

/// NEW const (Lever 2 read) — gain mapping (section.density − 0.5) into an edge_activity nudge.
/// 0.5 density (the identity midpoint) → 0.0 nudge → byte-identical edge_activity.
const DENSITY_ACTIVITY_GAIN: f32 = 0.5;

/// NEW const (Lever 3) — none needed; the dom7 pitch class is computed inline as
/// `(dom_root_pc + 10) % 12` inside `pivot_chord_events`.

// `pivot_chord_events` SIGNATURE IS UNCHANGED (Lever 3 is a body-internal voicing add):
//   fn pivot_chord_events(ctx, role, features, ms_per_step) -> Option<Vec<NoteEvent>>
// `realize_rhythm` SIGNATURE IS UNCHANGED (Lever 2 read uses the already-borrowed `ctx`).
// `plan_phrases` SIGNATURE IS UNCHANGED under Option A (Lever 1 voice-leading only).

// ── src/composition.rs ──────────────────────────────────────────────────────────

/// CHANGED (Lever 2) — resolve_key_scheme now returns (offset, source_region_energy) per section.
/// Planner-internal fn; not public API. The energy is 0..1; home/fallback sections carry
/// HOME_ENERGY_NEUTRAL so the density map is the algebraic identity on the home path.
fn resolve_key_scheme(
    scheme: Option<&KeyScheme>,
    sections: &[SectionTemplate],
    u: &ImageUnderstanding,
    home_mode: &str,
) -> Vec<(i8, f32)>;   // was: Vec<i8>

/// NEW consts (Lever 2 set) — the modest energy→density map and its byte-stable neutral.
const HOME_ENERGY_NEUTRAL: f32 = 0.5; // f(this) == DENSITY_NEUTRAL exactly (byte-stability proof)
const DENSITY_NEUTRAL: f32 = 0.5;
const DENSITY_ENERGY_SPAN: f32 = 0.30;
const DENSITY_FLOOR: f32 = 0.35;
const DENSITY_CEIL: f32 = 0.65;

// `Section` struct is UNCHANGED — `pivot`, `resolution`, `density` already exist (K3/slice-1).
// `StepContext` is UNCHANGED — `prev_key_offset_semitones` already exists (K3). Lever 2's read
//   uses `ctx.section.density`, already reachable. NO new ctx field, so NO engine.rs ctx-build edit.
```

**Callers of `resolve_key_scheme` to update for the return-type change** (Implementer; verified at
`dfcfb4c`): the planner call site `composition.rs:1110` (now destructures `(offset, energy)` per
section), and the unit tests at `composition.rs:2010/2058/2069/2084/2095/2121` (assert on the `.0`
offset element, or `.map(|t| t.0)` to keep existing offset assertions). These are mechanical.

---

## 4. DATA FLOW (ASCII)

```
                          ┌──────────────────── PLAN TIME (composition.rs) ───────────────────────┐
ImageUnderstanding u ───► resolve_key_scheme(scheme, sections, u, mode)
  (foreground_energy,        │  ranks fg/bg by RegionAffect.energy (EXISTING :1485)
   background_energy,        │  returns Vec<(offset_i8, source_region_energy_f32)>  ◄── LEVER 2(i) CHANGE
   …)                        ▼
                        section loop  (for each section i):
                          offset_i, energy_i = resolved[i]
                          section_root_midi = home_root + offset_i
                          progression = pick_progression(mode)            (RNG)
                          chords = generate_chords(progression, section_root_midi, …)
                          is_mod_boundary = pivot && i>0 && offset_i != offset_{i-1}
                          if is_mod_boundary:                              ◄── LEVER 1(a)
                              chords[0] = chord_engine.tonic_triad(section_root_midi, mode)
                          steps = plan_phrases(chords)   ── Lever 1(b) Option A voice-leads V→I
                          Section {
                              …,
                              density: f(energy_i)        ◄── LEVER 2(i) SET (ONCE, here only)
                                       = clamp(0.5 + 0.30*(energy_i − 0.5), 0.35, 0.65)
                                       ;  energy_i == 0.5 on home/home_only ⇒ density == 0.5 (byte-stable)
                              steps, pivot, resolution, …
                          }
                          └────────────────────────────────────────────────────────────────────────┘
                                                          │ CompositionPlan (sections carry density)
                                                          ▼
        ┌──────────────────────── RUN TIME (engine.rs — UNCHANGED, FROZEN) ───────────────────────┐
        │  decide_step → locate(step_idx) → (section, step_in_section)                              │
        │  ctx = StepContext::with_prev(section, step_in_section, theme, &key_tempo, prev_offset)   │  ◄ no edit
        │  PerfFeatures built from scan_bar_features (edge_density = GLOBAL per-bar) ──────────────┐│
        └────────────────────────────────────────────────────────────────────────────────────────┘│
                                                          │ ctx (carries ctx.section.density)        │
                                                          ▼                                          ▼
        ┌──────────────────────── REALIZER (chord_engine.rs — sha changes, BEHAVIORALLY frozen) ───┐
        │  realize_step(step, …, features, ms_per_step, ctx)                                        │
        │    └► pivot_chord_events(ctx, role, features, ms)  ── LEVER 3: + dom7 in inner voice ─────┤
        │    └► realize_rhythm(…, features, …, ctx)                                                 │
        │         edge_activity = clamp( features.edge_density/RANGE                                │
        │                                 + (ctx.section.density − 0.5)*DENSITY_ACTIVITY_GAIN, 0,1) │  ◄ LEVER 2(ii) READ
        │         (density 0.5 ⇒ nudge 0.0 ⇒ byte-identical on identity)                            │
        └──────────────────────────────────────────────────────────────────────────────────────────┘
```

**Where density is set:** exactly once, `composition.rs:1232` (the planner Section literal). **Where
density is read:** exactly once, `chord_engine.rs:1417` (the `realize_rhythm` edge_activity term).
The two are the only touch points; no other code path reads or writes `Section.density`.

---

## 5. BYTE-FREEZE ARGUMENT (load-bearing — the Quality Gate checks this)

**Claim: `src/engine.rs` is NOT EDITED and stays at `e50c7db1…`.**

- Lever 1 lives in `composition.rs` (the `chords[0]` overwrite) + `chord_engine.rs` (the
  `tonic_triad` helper + the V→I voice-leading rule). **No engine.rs change.**
- Lever 2 lives in `composition.rs` (resolve_key_scheme return type + the density SET) +
  `chord_engine.rs` (the density READ in `realize_rhythm`). The read uses **`ctx.section.density`**,
  which is **already on the ctx** the engine builds today (K3's `with_prev`). **No new ctx field, so
  no engine.rs ctx-build edit.** This is the single most important freeze decision: by riding the
  EXISTING `Section.density` field (dead until now) rather than adding a new field, S29 touches no
  engine.rs surface at all.
- Lever 3 lives entirely in `pivot_chord_events` (`chord_engine.rs`). **No engine.rs change.**
- The engine legacy fixture `engine.rs:766` (`density: 0.5`) is unchanged — it is already 0.5 and
  stays 0.5; it compiles unchanged because `Section.density` already exists.

Therefore the engine.rs sha **does not move** — no re-baseline, no lead sign-off needed for engine.rs
this slice. **Run `sha256sum src/engine.rs` after the slice: it MUST still equal `e50c7db1…`.** If it
moved, an engine.rs edit leaked in — revert it (the design requires none).

**Claim: `engine_equivalence` 9/9 + the 3 freeze guards stay GREEN.**

- The goldens (240/114/84/36/79) run the IDENTITY ctx via `single_section_default` over a hand-built
  `Section { key_offset_semitones: 0, density: 0.5, pivot: false, … }`. On that fixture:
  - Lever 1: `is_mod_boundary` is false (planner-side; the equivalence net doesn't even run the
    planner — it hand-builds sections), and `pivot:false` ⇒ no forced tonic, no opening-PAC
    voice-leading. **Inert.**
  - Lever 2 READ: `ctx.section.density == 0.5` ⇒ `density_nudge == (0.5 − 0.5)*GAIN == 0.0` ⇒
    `edge_activity` is byte-identical to pre-S29. **The articulation/rhythm goldens cannot move.**
  - Lever 3: `pivot_chord_events` returns `None` on `pivot:false` (K3 gate `chord_engine.rs:2195`),
    so the dom7 add is never reached. **Inert.**
- The 3 re-pointed guards (`keyplan_s25`/`prominence_s23`/`affect_s22`) shell `sha256sum
  src/engine.rs` against `e50c7db1…`; since engine.rs is untouched, they pass unchanged.

**Claim: every chord_engine.rs change is gated to be `None`/byte-identical on the identity path.**

| change | gate | identity behavior |
|---|---|---|
| Lever 3 dom7 in `pivot_chord_events` | the K3 gate (`!ctx.section.pivot \|\| step_in_section != 0` → `None`; `prev == dest` → `None`) | `None` on every identity/home_only/`pivot:false` step — the dom7 line is never executed |
| Lever 2 density nudge in `realize_rhythm` | `(ctx.section.density − 0.5) * GAIN` | `0.0` exactly when `density == 0.5`, which is EVERY identity/home/home_only section (proof: `f(HOME_ENERGY_NEUTRAL)==0.5`) → `edge_activity` byte-identical |
| Lever 1 `tonic_triad` helper | only CALLED from `composition.rs` under `is_mod_boundary` | never called on the identity path; as a standalone pub fn it is dead code under identity |
| Lever 1 V→I voice-leading (Option A) | applies only to the forced-tonic step of a modulating section | non-modulating sections' voicing untouched (the rule keys on the modulating boundary) |

**Witness set (machine-checkable — the Quality Gate runs ALL):**
- `sha256sum src/engine.rs` == `e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261` (UNMOVED — no re-baseline).
- `cargo test --test engine_equivalence` → 9/9 byte-green, goldens 240/114/84/36/79 unmoved.
- `realize_step` PUBLIC 7-param signature byte-identical worktree vs HEAD.
- `tests/keyplan_k3.rs::pivot_inserts_nothing_on_identity` green (now ALSO witnessing the dom7 add is
  dead on identity, and density-nudge==0 on identity — extend its assertions, §6).
- `no_inversion_invariant` green in BOTH `keyplan_s25.rs` and `prominence_s23.rs` (the dom7 in the
  fill register cannot invert bass<fill<melody).
- `no_routed_image_ends_off_home` (`keyplan_k2b.rs`) green (operator lock: Open scheme unrouted).
- full default net `cargo test` + `cargo test --lib --no-default-features` green.

---

## 6. WORK SPLIT (respecting single-writer on mappings.json)

**Music Theory Specialist (`chord_engine.rs` harmony/voice-leading; a file-disjoint INPUT DOC, e.g.
`docs/input-s29-k3-retune-harmony.md`):**
- **Lever 1(b)** the V→I opening-cadence VOICE-LEADING rule (leading tone up, 7th down — the
  parallel-octave avoidance the trombonist will hear), Option A default; the `plan_phrases` Option-B
  fallback decision.
- **Lever 1(a)** confirm `tonic_triad` chord content (root-position I, Triad complexity).
- **Lever 3** the dom7 voice assignment + its downward resolution across pivot→I (dovetails 1(b)).
- Confirms it owns and commits `chord_engine.rs` (the K3 default: Music Theory commits chord_engine).

**Rust Implementer (`composition.rs` planner structure + `Section.density` wiring + the realizer
density READ if assigned chord_engine commit; commits `composition.rs`, and `mappings.json` if any
data row is added):**
- **Lever 1(a)** the `is_mod_boundary` predicate + the `chords[0]` overwrite calling `tonic_triad`;
  the `tonic_triad` skeleton.
- **Lever 2(i)** `resolve_key_scheme` return-type change `Vec<i8> → Vec<(i8, f32)>` + all caller
  updates; the energy→density `f(energy)` SET at `:1232` (replacing the `0.5` hardcode) + the five
  new consts.
- **Lever 2(ii)** the `edge_activity` density nudge in `realize_rhythm` + `DENSITY_ACTIVITY_GAIN`
  (this is a chord_engine.rs read — the committer of chord_engine.rs lands it; coordinate via the
  input doc so only ONE writer touches chord_engine.rs).
- **mappings.json single-writer:** S29 adds **no required data row** — the density map is code
  consts, the pivot/cadence are code. *If* the lead elects an optional texture/figuration fast-follow
  (deferred, §2.2), that would add a `texture`-axis row and the Implementer is the SOLE
  mappings.json committer. For THIS slice, mappings.json is **untouched** (no flip set change either —
  the six Resolve schemes already carry `pivot:true` from K3; the Open scheme stays `pivot:false`,
  unrouted — operator lock).

**Test Engineer (tests only; no production code) — the surface to ADD:**
- **`opening_pac_confirms_destination_key`** — a routed modulating section now has its step-0 pivot V
  resolve to a step-1 destination ROOT-POSITION I (assert `chords[0].name == "I"` and the step-1
  bass pitch class == `dest_root_pc`), i.e. a V→I authentic cadence lands in the destination key
  early in the section.
- **`pivot_voicing_carries_dom7`** — a modulating boundary's pivot inner voice sounds
  `(dom_root_pc + 10) % 12` for a 3+ ensemble (and the no-inversion frame still holds:
  bass<fill<melody).
- **`density_varies_between_home_and_excursion`** — for a two-excursion image, the excursion sections'
  `Section.density != 0.5` while home sections == 0.5, AND the realized `edge_activity`/onset count
  differs measurably between a high-energy excursion and home.
- **Extend `pivot_inserts_nothing_on_identity`** (or add `density_nudge_zero_on_identity`) — assert
  `Section.density == 0.5` ⇒ realized stream byte-identical to the `single_section_default` baseline
  (witnesses Lever 2(ii) is inert on identity), AND the dom7 add is never reached on identity.
- Re-assert `no_inversion_invariant` across the pivot path WITH the dom7 (extend the K3
  `no_inversion_under_pivot_path` sweep), and confirm `engine_equivalence` 9/9 +
  `no_routed_image_ends_off_home` stay green.

**Quality Gate LAST** — runs the §5 witness set (esp. `sha256sum src/engine.rs` == `e50c7db1…`
UNMOVED), the module-boundary audit (chord_engine reads `ctx`/`features` only, names no pixel type;
composition has no pixel type; the density map is pure data), the codename scrub, the full net.

---

## 7. RISKS & TRADE-OFFS

1. **Dwell tension (Music Theory: dwell adequate / Aesthetics: dwell short).** The resolution is that
   **confirmation (Lever 1) + contrast (Lever 2) come FIRST; dwell is a HELD secondary knob.** Do NOT
   increase section dwell / `BASE_STEPS_PER_SECTION` / per-section step length in this slice — that is
   the explicitly-held "large dwell increases" lever. Re-listen after S29; if the modulation is now
   visible but still feels rushed, dwell becomes its own slice. **No dwell change here.**

2. **The "forced" over-correction risk.** The operator said *"nothing sounded forced"* — that is a
   NEGATIVE result we are deliberately leaving room around. The bias is intentionally MODEST
   (density span ±0.15, activity nudge ≤ ±0.075; the pivot/cadence already exist). Over-correcting
   (huge density swing, an aggressively stamped opening PAC via Option B, a dwell increase) risks
   swinging from "invisible" to "forced/mechanical." Ship the modest version; let the ear ask for
   more. This is why Option A (voice-leading, no new stamp) is the default over Option B.

3. **Density double-write / dead-field hazard (the prompt's named coordination check).** Today
   `Section.density` is **dead** (write-only, §1.2), so the failure mode is not "double-write" but
   **"set it and it's still dead"** — building Lever 2(i) without 2(ii) reproduces the invisible
   field. The mitigation is the §2.2 requirement that 2(i) and 2(ii) ship together, plus the
   single-writer rule: density is SET in exactly one place (`:1232`) and READ in exactly one place
   (`realize_rhythm:1417`); the S23 prominence path writes disjoint fields (`orchestration`/the
   prominence Vec), so there is provably no double-write. The byte-stability proof
   (`f(HOME_ENERGY_NEUTRAL) == DENSITY_NEUTRAL == 0.5`) is what keeps the identity path frozen.

4. **`pick_progression` RNG vs the forced tonic.** Forcing `chords[0]` overwrites one RNG-drawn
   symbol; the rest of the progression stays RNG. This is intentional (we only need the FIRST chord
   to confirm the key). Risk: if `generate_chords` prepended a secondary-dominant before the original
   symbol 0, the overwrite replaces that prepended V/x with I — acceptable (we WANT a clean V→I at the
   boundary, and the step-0 pivot already supplies the V). The Test Engineer asserts `chords[0].name
   == "I"` to pin this.

5. **`resolve_key_scheme` return-type change ripple.** Changing `Vec<i8> → Vec<(i8, f32)>` touches
   ~6 unit-test call sites. Mechanical, but the Implementer must update each (assert on `.0`). Low
   risk; caught at compile time. An alternative (a parallel `resolve_section_energies` fn) avoids the
   ripple but duplicates the ranking logic and risks the two drifting — the tuple return is the
   single-source-of-truth choice and is recommended.

6. **Texture/figuration deferral.** Shipping density-only for MX-4's "key + density mandatory" floor
   is sufficient and is what MX-4 mandates; texture is the named optional upgrade (§2.2). The trade is
   cleaner re-listen attribution (density alone) vs. a richer scene change. Recommend density-first;
   add texture as a fast-follow only if the ear still wants more scene change after S29.

7. **Engine.rs stays frozen — confidence HIGH.** The decisive design choice (riding the existing
   dead `Section.density` field instead of adding a ctx field) means engine.rs is not edited. The only
   way the freeze breaks is if an implementer adds a NEW ctx/Section field and threads it through the
   engine ctx-build — which this design explicitly avoids. If a future need forces a new ctx field,
   that is a SEPARATE, freeze-sensitive slice; flag it loudly and do not fold it into S29.

---

*Design-only. No source, test, or asset modified by this document. All line numbers, signatures, and
the byte anchors are verified against the working tree at HEAD `dfcfb4c`. Signatures + types + doc
comments are binding shapes; bodies belong to the slice implementers; the pivot/cadence harmonic
RULES belong to the Music Theory Specialist. Build-role titles (Architect, Implementer, Music Theory
Specialist, Test Engineer, Quality Gate) are the S21/S24/S26/S28 domain titles.*

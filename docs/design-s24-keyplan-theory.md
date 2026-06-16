# Design S24 — Structural Key Plan: the music-theory layer of "image as musical form"

**Author role:** Music Theory Specialist — THEORY pass. **DESIGN ONLY:** this document modifies no
source, test, or asset. It produces exactly one artifact — this file. It is the theory layer of a
larger S24 design; a parallel Rust-Architect threading pass and an aesthetics pass will sit beside
it (the open tensions for the aesthetics lens are collected at the end).
**Date:** 2026-06-16.
**Convention** (carried from `docs/design-s21-affective-fidelity.md`): Decision/Risk format; new
vocabulary lands as `mappings.json` rows parsed backward-compatibly via `#[serde(default)]`; the
neutral/identity path stays byte-identical to the frozen kernel; module boundaries hold
(`pure_analysis` = pixels, `composition` = planner, `chord_engine` = realizer, image-blind).

**Verified against the working tree** (not trusted from the brief):

- The transpose seam is real and applies as a UNIFORM pitch-class shift before voicing.
  `chord_engine.rs:2105`: `tonic_pc = ((home_root_midi + key_offset_semitones).rem_euclid(12))`.
  The same `key_offset` reaches the chord builder through `home_root_midi` →
  `generate_chords(root_midi, …)` (`chord_engine.rs:170`), so melody and harmony share one tonic.
- The seam is locked at zero: `composition.rs:937` `home_root_midi = 60` (C4 seed);
  `composition.rs:1038` `key_offset_semitones: 0 // LOCKED slice 1`; `composition.rs:916`
  `_key_scheme_id = …key_scheme.select(u)` is **selected then discarded**; `mappings.json`
  `composition.key_scheme = {"default":"home_only","rules":[]}`.
- The threading is END-TO-END today: `KeyTempoPlan.home_root_midi` (`composition.rs:743`) and
  `Section.key_offset_semitones` (`composition.rs:765`) already flow into the realizer via
  `StepContext.key_tempo`/`StepContext.section`. Nothing is missing structurally.
- **The forms already exist as data.** `mappings.json composition.form_catalogue` ships
  `ternary_aba` = [A:Statement/Perfect, B:Contrast/Half, A:Return/Perfect] and
  `abac` = [A:Statement/Half, B:Contrast/Imperfect, A:Return/Half, C:Coda/Perfect], plus
  `rounded_binary`, `aaba`, `abbac`, `theme_and_variations`. The `form` SelectTable already picks
  `abac` on `vertical_emphasis ≥ 0.6` and `ternary_aba` on `quadrant_contrast ≥ 0.6`. Each section
  template carries `boundary_cadence` and a `theme` slot. **A multi-section form with returns is
  already a fully representable structure — S24 only fills the key_offsets, it adds no new form.**
- **The region split is genuinely three-way.** `ImageUnderstanding` (`composition.rs:73–87`)
  exposes `subject_size`, `subject_hue`, `subject_saturation`, `fg_bg_contrast`, plus the energy
  triplet `subject_energy` / `foreground_energy` / `background_energy`. So "subject vs foreground
  vs background" is **three separable regions today** — the operator's three-way split needs NO new
  image analysis. (Their hues/saturations are the channels S24 reads; §Decision 2.)
- Register safety is structural: transpose is `.rem_euclid(12)` — a pitch-CLASS rotation, never an
  octave move; every pitch still clamps to `24..=108` (`chord_engine.rs:2120`, role_pitch
  `:1168`). `MELODY_REGISTER_FLOOR = 67`. A key change cannot push a voice out of register or
  invert figure-ground, because it shifts every voice by the SAME pre-voice-leading amount.
- Mode comes from hue today (`global.hue_to_mode`, `composition.rs:935`), and affect owns the
  major/minor *lean* (S21 valence). Neither is the tonic. The key plan is orthogonal to both
  (§Decision 4).

> **One ground-truth correction to the brief.** The brief says "every image renders in a FIXED key
> (C)" and "Tonic is hardcoded at `composition.rs:937`." That is the *home root* — correct, it is
> pinned to 60. But the home *mode* is NOT fixed: `home_mode` is already hue-derived
> (`composition.rs:934`, Phrygian/Lydian/Ionian/Dorian/Aeolian/Mixolydian). So today's pieces vary
> in MODE but not in TONIC, and never modulate. S24 adds the missing axis: a per-section TONIC
> plan (the `key_offset` spine), leaving mode where S21 put it.

---

## 0. Executive summary (read first)

Every piece today sits in one key for its whole duration: the home root is C4, every section's
`key_offset_semitones` is 0, and the `key_scheme` ladder is selected-then-discarded. Multi-section
forms (ABA, ABAC) already PLAY — they just play all in one key, so the "image as form" is a form of
texture and cadence, never of TONAL ARCHITECTURE. A trained ear hears a piece that states, contrasts,
and returns but never *travels*. The fix is to fill the key spine that is already threaded.

**The design, in one page:**

- **Form (Decision 1).** Keep the existing `form` ladder; do NOT invent a form. For the
  *key-plan* layer, **`ternary_aba` is the v1 home** (the clearest "depart and return" — A states
  the home key, B modulates away, A returns home) and **`abac` is the v1 stretch goal** (A home,
  B → related-key-1, A home, C → related-key-2, a "rondo-like" two-excursion reading of the eye
  sweeping the image). ABAC's two distinct excursions are exactly the operator's vision. Both are
  already in the catalogue; S24 ships a `key_scheme` per form, not a new form.

- **Region → key-area (Decision 2).** The **subject sets the home key A** — but A's *tonic* stays
  the pinned C (register-safe, identity-preserving); the subject instead sets A's MODE via the
  existing hue→mode path, so the "subject is home" is realized through the home mode the piece is
  already in. The **background and foreground select B and C** as RELATED-KEY OFFSETS from a small,
  music-theory-constrained menu. CRITICAL move: the region does not pick an arbitrary key — its own
  affect (its relation to the subject) picks a *direction and distance* on the related-key menu, so
  B/C are simultaneously image-driven AND modulation-sound.

- **The related-key menu (Decision 3).** Constrained to the keys a tonal ear accepts as "closely
  related" to A: **dominant (+7), subdominant (+5), relative (±3 across the major/minor divide),
  supermediant/submediant (±9 ≈ relative neighbors), and parallel (mode flip, +0).** Ranked by
  smoothness. v1 ships **dominant, subdominant, and relative** only — the three textbook
  first-degree-of-relation keys — and reserves the chromatic-mediant relations for a later slice.

- **Modulation mechanics (Decision 4).** v1 uses **direct (phrase) modulation at section
  boundaries** — the existing per-section `key_offset` IS a phrase modulation, and the section's
  `boundary_cadence` (already in the form data: B closes Half/Imperfect, returns close Perfect)
  prepares the ear. Pivot-chord modulation is a Stage-2 deepening that needs a one-bar pivot insert
  in `chord_engine`; it is designed here but DEFERRED. The home return is automatic: A's offset is
  always 0, so every "A" section snaps back to the pinned tonic — the strongest possible return.

- **Seam + byte-freeze (Decision 5).** Rides the EXISTING `key_scheme` SelectTable + the
  per-section `key_offset_semitones` field. The neutral default — `home_only`, all offsets 0 — is
  the shipped state, so goldens stay byte-identical with ZERO new default behavior. The
  `key_scheme` Vec already exists on `KeyTempoPlan`; S24 fills it instead of zeroing it.

---

## 1. The decisions (each PINNED)

### Decision 1 — FORM: ternary_aba is the v1 key-plan home; abac is the stretch. No new form. (PINNED)

**The question the brief poses:** ABAC vs ABA vs AABA vs ABAB vs through-composed, for an
image-driven piece.

**The catalogue already answers most of it.** S24 does NOT author a form — `ternary_aba` and
`abac` are shipped rows, picked by the `form` ladder from image structure (`abac` on high
`vertical_emphasis`, `ternary_aba` on high `quadrant_contrast`). S24's only job is to attach a
**key_scheme to each form** so those sections travel tonally. So the real decision is *which forms
get a non-trivial key scheme in v1, and what the scheme is.*

**Evaluation, through a key-plan lens (does the form's section sequence map cleanly onto a tonal
journey?):**

| Form | Section key-journey reading | Verdict |
|---|---|---|
| **ternary_aba** | **A(home) → B(away) → A(home).** The textbook departure-and-return. ONE modulation, ONE resolution. The cleanest possible "image has a center, an excursion, and a homecoming." | **v1 PRIMARY.** Smallest sound surface, hardest to get wrong, the canonical statement of tonal travel. |
| **abac** *(operator's example)* | **A(home) → B(rel-1) → A(home) → C(rel-2) → coda home.** TWO distinct excursions to two DIFFERENT related keys, each resolving home — a rondo-like reading where the eye sweeps the image twice and finds two different "elsewheres." This is precisely the operator's "background sets B, foreground sets C, both relate to A" vision. | **v1 STRETCH (Slice 2).** Richer and exactly the operator's design, but two modulations double the risk; ship after ABA proves the menu. |
| rounded_binary | A → B(contrast) → A'(0.75-len return). B already closes Half. A natural "open then half-close then resolve." | v1 SECONDARY (gets the ABA scheme — A=0, B=away, A'=0). Cheap to include since it's the default form. |
| AABA | A A B A — the B is a single bridge. Only ONE key area to choose (B), like ABA but with a doubled, more-established home. | Reserve; identical mechanics to ABA, lower marginal value. |
| ABAB | Not in the catalogue. Alternating two key areas with no final home resolution risks ending "away" — tonally unresolved, which a trained ear reads as incomplete. | **Reject** for v1: weak closure. |
| through-composed | No returns → no home to leave and come back to → the modulation has no anchor; every section a new key reads as aimless wandering, the opposite of "sensible modulation." | **Reject:** defeats the constrained-relation premise. |

**Decision: ternary_aba (PRIMARY) and rounded_binary (the default form, gets the same A/B/A'
scheme) ship the key plan in Slice 1; abac ships its two-excursion scheme in Slice 2.** AABA is a
documented reserve (same mechanics as ABA). ABAB and through-composed are rejected for v1 on
closure grounds.

**Dwell length / where returns fall:** unchanged — the planner already sizes sections from
`rel_len` (`composition.rs:1009–1018`) and the form data already places the returns. A trained ear
wants the home statement and the return to be *at least as long* as the excursion so the home
frames the journey; the catalogue satisfies this (ABA = 1.0/1.0/1.0; ABAC = 1.0/1.0/1.0/1.0 with a
0.75 coda in `abbac`). **No section-length change is needed or proposed.**

### Decision 2 — REGION → KEY-AREA: subject sets home MODE; background→B, foreground→C as offsets (PINNED)

**The brief's proposal:** subject → A's tonic; background → B; foreground → C.

**The theory refinement (two changes, both load-bearing):**

1. **Subject sets the home MODE, not a moved tonic.** Moving A's tonic off C buys nothing musical
   (a piece in F-major sounds identical to one in C-major to an unaccompanied ear — there is no
   reference pitch) and it costs the byte-freeze identity (the goldens are at root 60). So **A's
   tonic stays the pinned C**, and the subject expresses itself through the **home mode**, which is
   ALREADY subject-adjacent: today `home_mode` comes from `dominant_hue` via `hue_to_mode`. S24
   redirects that lookup to read `subject_hue` instead of `dominant_hue` when a real subject exists
   (`fg_bg_contrast` high enough) — so **the salient subject's hue now picks the home mode**, which
   is the strongest tonal statement of "the subject is home" that costs no register move. Where
   there is no subject (`subject_hue` defaults to `dominant_hue` anyway, `composition.rs:121`), this
   is a no-op — byte-stable.

2. **Background and foreground select B and C as RELATED-KEY OFFSETS, and the region's own affect
   picks WHICH related key.** This is the operator's "B/C are constrained AND image-driven"
   resolved cleanly. The mechanism:

   - **The menu is fixed** (Decision 3): {dominant +7, subdominant +5, relative ±3}.
   - **The region's relation to the subject picks the menu entry**, via two signals already
     present:
     - **Direction (brighter/darker than home) → sharp-side vs flat-side.** A region brighter
       than the subject (`region_brightness > subject_brightness`, derivable from
       `subject_saturation`/`avg` deltas, or simply `background_energy`/`foreground_energy` as a
       proxy in v1) modulates to the **sharp side = dominant (+7)** (the "brighter, more tense"
       neighbor). A darker region modulates to the **flat side = subdominant (+5)** (the
       "softer, more relaxed" neighbor). This is the standard sharp=bright / flat=dark
       affective reading of the circle of fifths, and it makes the modulation *direction*
       image-driven.
     - **Distance / contrast → near vs relative.** When the region's hue is close to the
       subject's hue (small `|subject_hue − region_hue|`), pick the **near key** (dominant or
       subdominant per direction); when the region's hue is far / its mode would differ (the
       region is a strong contrast), pick the **relative key (±3)** — the mode-crossing
       neighbor — so a strongly-contrasting region reads as a major↔minor relative shift, the
       most "different but still related" move.
   - **B (from background) and C (from foreground) read DIFFERENT regions**, so in ABAC they pick
     different menu entries → two genuinely distinct excursions. Foreground (nearer the eye, more
     prominent) tends to pick the stronger/closer relation (dominant); background (recessive) tends
     to the softer (subdominant or relative). This makes C feel like a *bolder* second excursion
     than B — the natural rondo escalation.

**Concrete derivation rule (v1, ABA — B only):**

```
home_mode  = hue_to_mode(subject_hue  if fg_bg_contrast ≥ τ_subj  else dominant_hue)   // Decision 2.1
B_offset:
  bright   = background_energy  (proxy v1) OR (region_brightness − subject_brightness)
  contrast = |subject_hue − background_hue| normalized, AND mode-crossing test
  if contrast HIGH  →  B_offset = relative_offset(home_mode)        // ±3, the contrast key
  elif bright       →  B_offset = +7  (dominant, sharp/bright)
  else              →  B_offset = +5  (subdominant, flat/dark)
A_offset, A'_offset = 0   // home always returns to the pinned tonic
```

**ABAC adds (Slice 2):** `C_offset` read from FOREGROUND the same way, with the foreground biased
toward the *near/strong* relation (dominant) so C escalates past B. Guard: if B and C would pick the
SAME offset, nudge C to the next-ranked menu entry so the two excursions stay distinct (a piece that
"modulates" to the same place twice has no second journey).

**`relative_offset(mode)`** is mode-aware: from a major/Ionian-family home the relative minor is
−3 (down a minor third); from a minor/Aeolian-family home the relative major is +3. This is
computed from `home_mode`, not hardcoded, so it composes with the hue-selected mode.

### Decision 3 — THE RELATED-KEY MENU: closely-related keys only, ranked; v1 ships three (PINNED)

The brief's central theory question. The menu of keys a tonal listener accepts as a *sensible*
modulation from home key A, ranked by smoothness/strength of relation (fewest accidentals changed
= closest = smoothest), with the `key_offset_semitones` each implies:

| Rank | Relation | `key_offset` | Why it's smooth (shared pitch material) | v1? |
|---|---|---|---|---|
| 1 | **Dominant** (V) | **+7** | Shares all but one pitch with home (one sharp added); the single most common modulation in tonal music; the "raise the tension / move outward" excursion. | **YES** |
| 2 | **Subdominant** (IV) | **+5** | Shares all but one pitch (one flat added); the "relax / settle" neighbor; the plagal-side counterweight to the dominant. | **YES** |
| 3 | **Relative** (vi from major / III from minor) | **−3 (maj→min) / +3 (min→maj)** | SAME key signature — zero accidentals changed; the smoothest possible *mode-crossing* move; the canonical major↔minor "same notes, different center." | **YES** |
| 4 | **Submediant / Supermediant** (chromatic-mediant) | ±9 / ±4, ±8 | One or two pitches changed; a "color" modulation — striking but still relatable via a common tone; Romantic-era favorite. | Reserve (Slice 3) |
| 5 | **Parallel** (same tonic, mode flip) | **+0**, mode change only | Zero tonic move; reads as a *mode* shift, not a key shift — which S21's valence axis already owns. Excluded as a KEY move to avoid fighting affect (Decision 4). | Excluded from key menu |
| — | **Supertonic / leading-tone / tritone** (+2, +11, +6) | distant | Two-or-more accidentals; reads as abrupt/foreign without careful pivoting; "bad modulation practice" for an unprepared phrase modulation. | **Reject** v1 |

**Decision: v1 menu = {dominant +7, subdominant +5, relative ±3}** — the three first-degree
related keys every theory text lists as the closest, each implementable as a single
`key_offset_semitones` with no engine change. Chromatic mediants are reserved for Slice 3 (they
*sound* great but need pivot/common-tone preparation to not jar, which is itself deferred). Parallel
is excluded from the KEY menu because mode is the affect axis's territory (Decision 4) — letting the
key plan flip mode too would double-drive the major/minor choice.

**Does the region's own affect pick B from within A's menu? YES — and this is the preferred
design** (Decision 2): the region's brightness picks sharp(+7)/flat(+5) direction, the region's
contrast picks near(dominant/subdominant) vs relative(±3). So B and C are *always* one of the three
menu entries — image-driven in WHICH entry, theory-constrained in that the entries are exactly the
closely-related set. The image can never select a foreign key.

### Decision 4 — MODULATION MECHANICS: direct phrase modulation in v1; pivot deferred (PINNED)

**What's implementable on the existing engine with NO kernel change:**

- **Direct (phrase) modulation.** The per-section `key_offset_semitones` IS already a direct
  modulation: at the section boundary the tonic jumps by the offset and `generate_chords` builds the
  next section's progression in the new key (it takes `root_midi`, `chord_engine.rs:170`). No
  insert, no engine touch — the offset simply becomes non-zero. **This is v1.** Direct modulation is
  *convincing* when the destination is closely related (our menu guarantees that) and the LEAVING
  section ends on a cadence that doesn't over-commit to home — and the form data already gives us
  that: B is reached after A closes on a **Half/Imperfect** cadence (`abac`: A closes Half; `ternary_aba`:
  A closes Perfect — see Risk 2), which is exactly the "leave a door open" preparation.

- **The home return is the strongest possible.** Every "A" section's offset is pinned 0, so the
  return is an *exact* snap back to the home tonic — not an approximate re-modulation. A trained ear
  reads the literal return of the home pitch center as a true recapitulation. The form's Return
  sections close **Perfect** (`abac` A-return is Half — see Risk 2; `ternary_aba` Return is
  Perfect), so the homecoming lands with a full cadence.

**What's designed-but-DEFERRED (Slice 3):**

- **Pivot-chord modulation.** The smoothest classical modulation reinterprets a chord common to
  both keys (e.g. home's vi = destination's ii) as the hinge. Implementable as a one-bar pivot
  chord inserted at the section boundary BY `chord_engine` — but it changes the realized note stream
  at the seam, so it is freeze-sensitive and gets its own witnessed slice.
- **Common-tone modulation.** Hold a single shared pitch across the boundary while the harmony
  reinterprets it — also a realizer insert, also deferred.

**Decision: v1 = direct phrase modulation at section boundaries, using the form's existing boundary
cadences as preparation, with an exact pinned-tonic home return. Pivot and common-tone are designed
and deferred to Slice 3.** Direct modulation to a closely-related key is fully idiomatic — it is
how thousands of folk tunes, hymns, and pop songs change key — and it is the only mechanic that
moves ZERO kernel bytes.

### Decision 5 — SEAM + BYTE-FREEZE: ride the existing key_scheme + key_offset; neutral default unchanged (PINNED)

**The seam, exactly:**

- `key_scheme` SelectTable (`composition.rs:660`, `mappings.json composition.key_scheme`) is
  selected at `composition.rs:916` and currently discarded (`_key_scheme_id`). S24 **stops
  discarding it**: the selected id looks up a **key-scheme catalogue** (new `#[serde(default)]`
  rows, exactly parallel to `form_catalogue`/`prominence_catalogue`) giving a per-section offset
  rule, which fills `Section.key_offset_semitones` (`composition.rs:1038`) and the
  `KeyTempoPlan.key_scheme` Vec (`composition.rs:1053`) instead of zeroing them.
- The offset then flows UNCHANGED through the already-wired path: `key_offset_semitones` +
  `home_root_midi` → `tonic_pc` (`chord_engine.rs:2105`) for melody, and `home_root_midi` (+offset
  via the same arithmetic) → `generate_chords(root_midi)` for harmony. **No new threading; the
  pipe already exists and is tested zero.**

**The neutral default that keeps goldens byte-identical:**

- The SHIPPED `key_scheme` is `{"default":"home_only","rules":[]}`. The `home_only` catalogue entry
  is **all-zero offsets**. So with no image rule firing — and on the entire equivalence net, which
  hand-builds sections with `key_offset_semitones: 0` (`composition.rs:1434`, `chord_engine.rs:4507`
  etc.) — every offset stays 0, `tonic_pc` stays `60.rem_euclid(12) = 0`, and **every emitted note
  is bit-for-bit today's.** The new behavior is reachable ONLY when a non-`home_only` scheme is
  selected by an image rule, which the net never triggers. This is the same discipline as
  `key_scheme: home_only` already uses — S24 changes nothing about the default.
- `KeyScheme` mappings get `#[serde(default)]` → `Default` = the single `home_only` all-zero scheme,
  so a `mappings.json` with no key-scheme catalogue parses to the byte-stable state (the
  `AffectMappings::default()` precedent, `composition.rs:178`). The `mapping_loader` mirror
  obligation applies: the new `key_scheme_catalogue` field must be added to `CompositionMappings`
  with `#[serde(default)]` AND mapped in `From<CompositionMappings> for PlanMappings` (the S21
  Risk 7 trap).

**Register / no-inversion safety (the brief's explicit ask):**

- Transpose is a **pitch-CLASS** operation: `tonic_pc = (home_root_midi + offset).rem_euclid(12)`
  (`chord_engine.rs:2105`). The `.rem_euclid(12)` means a `+7` offset rotates the tonic class but
  the SOUNDING register is set independently by `MELODY_REGISTER_FLOOR`/`role_pitch`/the
  brightness `bright_octaves` lift — all of which clamp to `24..=108`. **A key change therefore
  cannot move any voice into a new octave, cannot push the melody out of range, and cannot invert
  figure-ground** — it is a uniform pre-voice-leading shift of the tonal center, applied identically
  to every voice. This is strictly safer than the S21 prominence register nudges (which DID move
  octaves and needed the `no_inversion_invariant` sweep). The home root stays clamped in its safe
  octave by construction (it is the pinned 60). **No new register guard is required**, but the
  Slice-1 test suite re-asserts the existing `mean_pitch(Bass) < bed/fill < mean_pitch(Melody)`
  invariant across all key offsets to make the safety explicit.

---

## 2. The data surface (mappings.json rows — additive, disjoint keys)

S24 owns ONE new `composition.*` block (the key-scheme catalogue) + fills the existing `key_scheme`
SelectTable rules. Disjoint from S21–S23's keys (`affect`, `character`, `prominence`, `texture`).

```jsonc
// (a) the key-scheme catalogue (NEW) — parallel to form_catalogue/prominence_catalogue.
//     Each scheme is per-section offset rules; "home_only" is the all-zero identity.
"key_scheme_catalogue": [
  { "id": "home_only", "sections": [] },                       // identity: every section offset 0
  { "id": "aba_excursion", "sections": [                       // ternary_aba / rounded_binary
      { "label": "A",  "offset_rule": "home" },
      { "label": "B",  "offset_rule": "region_related:background" },
      { "label": "A",  "offset_rule": "home" },
      { "label": "A'", "offset_rule": "home" } ] },
  { "id": "abac_rondo", "sections": [                           // abac (Slice 2)
      { "label": "A", "offset_rule": "home" },
      { "label": "B", "offset_rule": "region_related:background" },
      { "label": "A", "offset_rule": "home" },
      { "label": "C", "offset_rule": "region_related:foreground" } ] }
],

// (b) fill the EXISTING (currently-empty) key_scheme SelectTable rules.
//     A real subject/ground stratification → travel; else home_only (byte-stable).
"key_scheme": {
  "default": "home_only",
  "rules": [
    { "when": [ {"knob":"fg_bg_contrast","op":"ge","lo":0.25,"hi":0.0} ], "pick": "aba_excursion" }
    // Slice 2 adds a rule picking "abac_rondo" when form==abac is selected (or on a stronger
    // fg_bg_contrast + a foreground-distinct-from-background test).
  ]
}
```

`offset_rule` is resolved IN the planner (it needs the live region knobs + `home_mode`), exactly
like the prominence resolve (`composition.rs:1001`): `home` → 0; `region_related:<region>` →
the Decision-2 brightness/contrast → {+7, +5, ±3} computation. The catalogue carries the RULE
(byte-stable, data); the planner computes the NUMBER once per plan. New planner fn signature
(no body — Architect pins it):

```rust
/// Resolve a key-scheme's per-section offset rules against the region knobs + home_mode.
/// Returns the per-section Vec<i8> of key_offset_semitones; "home" → 0, "region_related" →
/// the closely-related menu entry (Decision 2/3). Pure; no clock, no RNG.
fn resolve_key_scheme(scheme: &KeyScheme, u: &ImageUnderstanding, home_mode: &str) -> Vec<i8>;
```

---

## 3. THE STAGED BUILD PLAN (independently-shippable, independently-HEARABLE)

Cadence per slice (the S21 pattern): **Rust Architect (buildable spec) → Rust Implementer ∥ Music
Theory Specialist (file-disjoint) → Test Engineer → Quality Gate LAST.**

### Slice 1 — ABA single modulation *(RECOMMENDED FIRST SLICE)*

- **Scope:** the `aba_excursion` scheme on `ternary_aba` + `rounded_binary`; ONE modulation (B),
  exact home return. Menu {+7, +5, ±3}. Subject_hue → home_mode redirect (Decision 2.1).
- **Files:** `composition.rs` (KeyScheme types, `resolve_key_scheme`, fill `key_offset_semitones`
  / `key_scheme` Vec from the resolve instead of literal 0, redirect the home_mode hue source);
  `mapping_loader.rs` (`key_scheme_catalogue` mirror, `#[serde(default)]`, `From` arm);
  `mappings.json` (§2 (a)(b)). **`chord_engine.rs` and `engine.rs` NOT touched** — direct
  modulation rides the existing offset→tonic_pc/root_midi path; no realizer code moves.
- **Byte-freeze argument (one line):** `home_only` is the shipped default and the net's hand-built
  sections all carry `key_offset_semitones: 0`; `resolve_key_scheme("home_only")` returns all-zero;
  so `tonic_pc`/`root_midi` are unchanged and goldens (240/114/84/36/79) cannot move — the new
  offsets are reachable only via an image-selected non-`home_only` scheme the net never selects.
- **Tests:** (1) `home_only_keeps_offsets_zero` — no-rule / absent-catalogue → all offsets 0 →
  `git diff HEAD -- src/engine.rs src/chord_engine.rs tests/engine_equivalence.rs` EMPTY.
  (2) `aba_returns_to_home` — A and A' offsets == 0, B offset != 0. (3) `B_offset_in_menu` — B's
  offset ∈ {+7,+5,+3,−3} for any image. (4) `bright_bg_picks_sharp` / `dark_bg_picks_flat` /
  `contrasting_bg_picks_relative` — the Decision-2 direction/contrast mapping. (5)
  `no_inversion_across_all_offsets` — `mean_pitch(Bass) < fill < mean_pitch(Melody)` and every note
  ∈ 24..=108 for every menu offset and every character (the explicit register-safety witness).
- **What the owner hears:** *a piece with a clear subject/ground now LEAVES home for its B section
  — to the dominant, subdominant, or relative key depending on the background's brightness/contrast
  — and RETURNS home for the recap. The piece travels and comes back, instead of sitting in one key.
  A subjectless field still stays home (byte-stable).*

### Slice 2 — ABAC two-excursion rondo

- **Scope:** the `abac_rondo` scheme on `abac`; B from background, C from foreground, the
  distinct-excursion guard (C escalates past B; nudge if equal). The operator's literal vision.
- **Files:** `composition.rs` (the foreground branch of `resolve_key_scheme` + the B≠C guard);
  `mappings.json` (the `abac_rondo` catalogue row + the Slice-2 `key_scheme` rule). No engine/realizer.
- **Byte-freeze:** identical argument — reachable only via an image-selected `abac_rondo`; net never
  selects it.
- **Tests:** `abac_two_distinct_keys` (B_offset != C_offset), `abac_all_A_home` (every A == 0),
  `foreground_escalates` (C tends to the stronger/nearer relation than B), plus the Slice-1 register
  sweep across the 4-section form.
- **What the owner hears:** *the eye sweeps the image twice — B visits one related key (the
  background's), A returns, C visits a DIFFERENT related key (the foreground's, bolder), and the
  coda lands home. Two distinct tonal excursions, both sensible neighbors of the subject's home.*

### Slice 3 — pivot / common-tone modulation + chromatic-mediant menu *(depth, freeze-sensitive)*

- **Scope:** the smoother modulation mechanics (a one-bar pivot chord inserted at the boundary by
  `chord_engine`) and the chromatic-mediant menu entries (±9/±4/±8) that NEED that preparation.
- **Files:** `chord_engine.rs` (the witnessed pivot-insert at the section seam) + `mappings.json`
  (the extended menu). The one freeze-sensitive slice — the pivot changes the realized note stream
  at the seam, so it carries an explicit byte-freeze argument (the pivot is reachable only when a
  non-`home_only` scheme + a `pivot:true` flag is set; identity path inserts nothing).
- **What the owner hears:** *the key changes stop being abrupt jumps and become prepared, hinged
  modulations — the destination is foreshadowed a beat before it arrives — and the menu opens to the
  striking chromatic-mediant relations a trained ear loves.*

**Sequencing rationale.** Slice 1 is the decisive, lowest-risk win — data + planner only, the
literal "the piece modulates and comes home" fix — and it proves the related-key menu + the
region→direction mapping before any realizer code moves. Slice 2 is the operator's full ABAC vision,
pure planner. Slice 3 is the smoothness/color depth pass and the only freeze-sensitive change.
Slices 1 and 2 are file-disjoint from the realizer entirely.

---

## 4. Risks / trade-offs

1. **Moving-tonic buys nothing audible without a reference (the reason A stays C).** An
   unaccompanied piece in F-major is indistinguishable from one in C-major to the ear — only
   *relative* motion (the modulation itself) is audible. The design exploits this: A is pinned to C
   (free byte-freeze identity), and ALL musical meaning is in the B/C *offsets* relative to it. The
   trade-off: "the subject sets the home key" is realized as MODE, not absolute pitch — which is
   the right call, but worth stating so the owner isn't surprised that two subject-distinct images
   share the same home pitch C (they differ in mode and in where they travel).
2. **`abac`'s A-return closes HALF, not Perfect (catalogue fact).** `abac` A-return =
   `boundary_cadence: Half`; `ternary_aba` Return = Perfect. A *Half* cadence on a home return is a
   weak homecoming — the ear wants the recap to resolve. This is why **`ternary_aba` is the v1
   primary** (its Perfect return lands the home properly) and ABAC is Slice 2. **Flag for the
   aesthetics lens:** should ABAC's A-return cadence be strengthened to Perfect for the key-plan to
   feel resolved, or does the Half cadence + the literal pinned-tonic return suffice? (Touching the
   form data is out of S24's lane; this is a question, not a change.)
3. **Direct modulation can jar without preparation.** A `+7` jump straight into the B section, with
   no pivot, is idiomatic for closely-related keys but is the bluntest mechanic. The menu
   constraint (closely-related ONLY) keeps it acceptable in v1; Slice 3's pivot is the smoothing
   pass. If the owner's ear finds even closely-related direct modulation too abrupt, Slice 3 moves
   up the queue.
4. **The brightness→sharp/flat mapping is a convention, tunable by ear.** "Brighter region →
   dominant (sharp side), darker → subdominant (flat side)" is the standard affective reading of the
   circle of fifths, but it is a STARTING calibration — the directions are from theory, the
   thresholds (τ_subj, the contrast cutoff) are seeds. The owner's ear is the gate.
5. **Region-energy as a brightness proxy is coarse (v1).** v1 uses `background_energy` /
   `foreground_energy` as the direction proxy because per-region *brightness* is not yet a first-class
   field (only `subject_saturation`/`subject_hue` are). If the ear wants true brightness-driven
   direction, a small `pure_analysis` add (`background_brightness`/`foreground_brightness`) is the
   minimal prerequisite — **flagged as an optional Slice-1.5 image-analysis sub-slice, NOT a
   blocker** (the energy proxy ships v1).
6. **mapping_loader mirror is easy to forget** (S21 Risk 7, recurring): the new
   `key_scheme_catalogue` MUST land on `CompositionMappings` + the `From` impl, or the catalogue is
   silently dropped at load and every scheme degrades to `home_only`. The `home_only_keeps_offsets_zero`
   test is necessary but NOT sufficient to catch this (it passes either way); add a
   `key_scheme_catalogue_round_trips` load test as the witness.
7. **Composing with affect's mode (Decision 4 / S21).** Affect (valence) owns the major/minor
   *lean*; hue owns the home mode; the key plan owns the tonic offsets. These are three orthogonal
   axes by construction — the key plan moves the tonic, never the mode — but the **relative-key
   offset (±3) CROSSES the major/minor divide**, so a relative modulation implicitly flips the mode
   family for that section. This is correct theory (the relative key IS the other mode), but it
   means the B section's mode is set by the modulation, potentially overriding the hue/affect mode
   for that section only. **Flag for the aesthetics lens:** when B modulates to the relative key,
   should B's mode follow the relative (correct theory, the section genuinely changes color) or
   should the relative offset be skipped if it would contradict a strongly affect-selected mode?
   The design's position: follow the relative (it is the point of the move), but this is the sharpest
   key↔mode interaction and the one most worth an aesthetic ruling.

---

## 5. Recommended FIRST BUILD SLICE

**Slice 1 — ABA single modulation.** Data + planner only (`composition.rs`, `mapping_loader.rs`,
`assets/mappings.json`); **`chord_engine.rs` and `engine.rs` untouched.** It is the biggest tonal
win with the least risk — it fills the already-threaded `key_scheme`/`key_offset_semitones` spine so
a piece with a clear subject leaves home for a closely-related key (dominant/subdominant/relative,
chosen by the background's brightness/contrast) and returns home on the form's Perfect cadence. The
byte-freeze holds in one line: `home_only` is the shipped default and the equivalence net's sections
all carry offset 0, so `tonic_pc`/`root_midi` are unchanged and goldens cannot move. Predicted
output: *a subject/ground image now modulates and recapitulates; a subjectless field stays home.*

*Design-only. No source, test, or asset modified by this document. The seam citations
(`composition.rs:916/937/1038/1053`, `chord_engine.rs:2105/170`, `mappings.json composition.key_scheme`)
are verified against the working tree. Signatures are binding shapes; bodies are the slice
implementers'. Build-role titles (Rust Architect, Rust Implementer, Music Theory Specialist, Test
Engineer, Quality Gate) are the S21 domain titles.*

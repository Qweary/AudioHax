# Design S26 — Multi-Excursion Travel & the Open-Ending Policy: the SONGWRITING / AESTHETICS layer

**Author role:** Composition & Songwriting Aesthetics Specialist — DESIGN ONLY. This document
modifies no source, test, or asset. It writes `docs/` prose and proposes `mappings.json` rows,
planner-fill behaviour, and property invariants as *binding shapes for a later implementer to
transcribe*; it commits nothing. Its lens is distinct from the Music Theory lens (which guarantees a
modulation is theory-*correct* — a legal pivot, a clean cadence) and from the image lens (which
*computes* the perceptual features). THIS role owns whether the whole macro shape *sounds good and
feels pleasing* — pacing, payoff, contrast that rewards, a return that lands, and — new in S26 — an
open ending that feels INTENTIONAL rather than abandoned.

**Date:** 2026-06-16. **Builds on:** `docs/design-s24-keyplan-aesthetics.md` (my predecessor design),
`docs/design-s24-image-as-form-key-plan.md` (the locked Architect reconciliation), `docs/review-S25.md`
(the K1 Quality Gate + the operator's re-listen).

**Verified against the working tree (HEAD `9cd9681`), not trusted from prose:**

- The spine types are exactly: `ThematicRole = {Statement, Contrast, Return, Development, Coda}`
  (`composition.rs:277`); `CadenceStrength = {Half, Imperfect, Perfect, Deceptive, Plagal}` (`:304`);
  `SectionTemplate { label, role, rel_len, theme, variation, boundary_cadence }` (`:512`);
  `FormSpec { id, sections }` (`:530`); `KeyScheme { id, sections: Vec<KeySchemeSection> }` (`:461`)
  and `KeySchemeSection { label, offset_rule }` (`:450`); the per-section
  `Section.key_offset_semitones: i8` (`:794`) and the `KeyTempoPlan.key_scheme: Vec<i8>` spine (`:778`).
- The affect/saliency knobs on `ImageUnderstanding` (`:39`) are real: `subject_size` (`:75`),
  `subject_hue` (`:77`), `subject_saturation` (`:79`), `fg_bg_contrast` (`:81`), `subject_energy`
  (`:83`), `foreground_energy` (`:85`), `background_energy` (`:87`), `value_key` (`:59`),
  `vertical_emphasis` (`:72`), `quadrant_contrast` (`:68`), `secondary_hue` (`:53`), and the
  planner-computed `affect_valence`/`affect_arousal` (`:95`–`97`). **CONFIRMED LIMITATION (review-S25
  note 2 / S24 Risk 3): there is NO per-region valence/hue/brightness — only per-region ENERGY.**
- K1 shipped: `resolve_key_scheme` (`:1260`), `excursion_offset` (`:1215`), `relative_offset`
  (`:1183`), `lookup_key_scheme` (`:1175`). The menu is `{+7 dominant, +5 subdominant, +3/−3
  relative}`, mode-family-aware. The `region_related:c` branch ALREADY routes to `excursion_offset`
  (`:1281`) — so K2's C is structurally a SECOND excursion today.
- **The live data contradiction the operator just resolved:** `assets/mappings.json`
  `key_scheme_catalogue` ships `abac_rondo` with C as `{"offset_rule": "region_related:c"}` —
  i.e. **C ends OFF-home.** This directly contradicts S24 Invariant A ("C resolves home even when the
  theme is new"). The operator has DELIBERATELY chosen the off-home reading as a first-class option;
  this doc owns the *taste* of when that is right and when it is wrong, and replaces the hard
  Invariant A with a CONDITIONAL one.
- The `form` ladder routes `abac` on `vertical_emphasis ≥ 0.6` and `abbac` on `edge_activity ≥ 0.7 &
  value_key ≥ 0.6`; the `key_scheme` ladder currently fires only `aba_excursion` on
  `fg_bg_contrast ≥ 0.25` (no `abac_rondo` rule wired yet). Both verified in `assets/mappings.json`.

> Convention (carried from S21/S24): the plan lands as **data** (`key_scheme` SelectTable rules +
> `key_scheme_catalogue`), parsed backward-compatibly via `#[serde(default)]`; pure-Rust default;
> the planner fills `key_offset_semitones`/`key_scheme: Vec<i8>` from the selected scheme; **no
> realizer/engine edit** for the planner-only slices because the transpose seam is already wired and
> exercised. `"home_only"` stays the identity/byte-freeze anchor. The pivot/cadential-homecoming work
> is the ONE freeze-sensitive piece and is owned by the Music Theory + Architect lenses.

---

## 0. Executive summary (read first)

K1 (S25) made a piece LEAVE home and COME BACK on a single trip — the ABA departure-and-return. S26
opens the scope the operator asked for: **the WHOLE form catalogue carries genuine tonal travel**, so
the variety we drew at the top of the pipeline becomes *audible across forms*, not an abac-only
special case. And it makes the **open / off-home ending a deliberate expressive option**, not a defect
to suppress. The hard "always resolve home" rule (S24 Invariant A) becomes a **conditional** one I
define here on image grounds.

**The aesthetic design, in one page:**

- **The eye sweeps twice → the music travels twice, and it must feel like a JOURNEY, not wandering.**
  A multi-excursion piece is satisfying when (a) each excursion is *audibly distinct* (different key
  AND different density/register, tracking the per-region affect), (b) the destinations sit at a
  *rewarding* tonal distance from each other (related-but-distinct, not a micro-step and not a jarring
  leap), and (c) there are *few enough* of them that each is an event. The cap is **≤ 2 distinct
  non-home destinations**; a third excursion in a short piece reads as restless. (§1.)

- **THE OPEN-ENDING POLICY (the heart of S26): home vs open is decided by the IMAGE, testably.**
  A piece ends HOME when the image has a **clear subject** (`subject_size` in a mid band AND
  `fg_bg_contrast ≥ τ_subject`) — a resolved scene with a thing the eye rests on. A piece may end
  OPEN when the image is a **subjectless / panoramic field** (`fg_bg_contrast < τ_subject` AND a
  *travelling* read — `vertical_emphasis ≥ 0.6` selecting an episodic form) — there is no resting
  place in the picture, so the music shouldn't manufacture one. This is the exact condition wired as
  the conditional `resolves_home` guard. (§2.)

- **The form catalogue earns its keep by a PERCEPTUAL reason, not randomness.** Returning forms
  (`rounded_binary`, `ternary_aba`, `aaba`) are selected — and want home — when there IS a subject to
  return to; episodic forms (`abac`, `abbac`) are selected — and MAY stay open — when the eye travels;
  `theme_and_variations` is selected for a single dense busy field (one idea, recolored). The form
  ladder already does this selection; S26 binds the key plan + the open/home decision to it in
  lockstep. (§3.)

- **An earned return and an intentional open ending are SYMMETRIC craft problems.** A return is earned
  by a cadential approach (the B/penultimate section leaves on a Half/weak cadence so the ear *wants*
  home) and a contrast payoff (home returns *brighter/clearer* against the travelled keys). An open
  ending is intentional when the final excursion lands on a *related* key (not a random one) and the
  final section *settles* on a soft cadence (Plagal/Imperfect, never a Perfect cadence in a non-home
  key — that would sound like the piece resolved to the wrong place). Both depend on the Music Theory
  lens for the actual cadence. (§4.)

- **Guard-rails are testable properties, updated for the multi-excursion + open-ending world.** The
  identity path stays byte-frozen. `resolves_home` becomes CONDITIONAL on the image. Home sections are
  always home. Distinct excursions must actually differ. Modulation count is capped. (§5.)

**Recommended FIRST build slice (S26-A): the conditional open-ending decision on ABAC.** It is the
single highest-payoff audible win — it makes the operator's "the eye sweeps twice and may not come
home" real, and it is planner-only (no realizer move) once the conditional guard is wired. (§6.)

---

## 1. The panoramic / multi-excursion satisfaction model (PINNED)

**What the operator is reaching for.** "The eye sweeps twice (or more) and the music travels with
it." The risk this opens is the oldest one in episodic music: travel without shape is *wandering*. A
piece that modulates to a new key every section, each unrelated to the last, with no contrast in
texture, sounds like a tour of unconnected rooms — restless, not journeying. The satisfaction model
is the set of rules that keep travel feeling like a *route* with stops, not a drift.

### 1.1 How many excursions before it feels like wandering — the count cap

| Excursion count | What the ear hears | Verdict |
|---|---|---|
| 0 (home_only) | static, one room | the *old* complaint; reserved for genuinely uniform images |
| 1 (ABA) | leave, come back — the classic single trip | the safe, universally satisfying default (K1) |
| **2 (ABAC, ABBAC)** | **two distinct stops — a real journey** | **the multi-excursion sweet spot; the S26 target** |
| 3+ | a tour; each stop loses weight; restless | OFF — a third destination in a ~60–120 s piece dilutes every move |

**Rule MX-1 (count cap).** A piece carries **at most TWO distinct non-home destinations.** *Why more
pleasing:* in a short piece, listener memory holds two contrasting "elsewheres" and the home they
frame; a third is forgotten as the fourth arrives, so each move stops being an event. This is the
existing `at_most_two_distinct_non_home_keys` guard (review-S25 §5.5) promoted from a K2-safety cap to
the *governing* pacing rule for the whole catalogue.

### 1.2 How far apart the destinations should sit — contrast that rewards vs jarring

The two excursions must be far enough apart to feel like *different* places, but both close enough to
home that the home frame still binds them. The K1 menu (`{+7 dominant, +5 subdominant, +3/−3
relative}`) is exactly the closely-related set — every entry shares 6–7 of 7 pitch classes with home,
so any *pair* drawn from it is automatically "related-but-distinct."

**Rule MX-2 (distinct destinations).** When a form has two excursions (B and C), **B and C must
resolve to DIFFERENT offsets.** *Why more pleasing:* two stops at the *same* key is not a journey —
it is one excursion stated twice with a home visit in between, which the ear hears as a failed second
move. If the energy-ordered region pick (Decision 2) and the menu math happen to collapse B and C to
the same offset, the planner nudges C to the next-ranked menu entry (the distinct-excursion guard
named in S24 Slice K2). The *ranking* of "next" is: from the dominant (+7) → the relative; from the
subdominant (+5) → the dominant; from the relative (±3) → the dominant. Each fallback keeps the move
inside the closely-related set, so the nudge never makes C *more* remote than B — it makes it
*different*, not *wilder*.

**Rule MX-3 (no adjacent modulation).** Two consecutive non-home sections never both modulate to a
*new* key without a home section between them — the existing ABAC/ABBAC shapes guarantee this (a home
`Return` always sits between B and C). *Why:* back-to-back key changes with no home anchor read as
aimless; the ear loses the reference that makes each move legible.

### 1.3 How each excursion should differ AUDIBLY — key + density + register

A key change alone is the *thinnest* form of contrast (review-S25 honestly flags that two images with
the same whole-image valence but different fg/bg energy currently get the *same* B offset). For a
multi-excursion sweep to feel like the eye landing on a *different region*, each excursion should
differ in **more than one dimension**:

| Dimension | Source knob | How it should differ per excursion | Why pleasing |
|---|---|---|---|
| **Key** | `excursion_offset` (valence + hue-distance) | B and C distinct offsets (MX-2) | the tonal "where am I" |
| **Density** | the energy-ordered region's energy (`background_energy` vs `foreground_energy`) | the *more energetic* region's excursion is denser/busier | the busier region of the image SOUNDS busier — the eye-to-ear link |
| **Register** | (depends on the realizer's role-pitch lift) | optional later: a higher-energy region can sit brighter | reinforces "a different place" without a key change carrying all the weight |

**Rule MX-4 (excursions differ in ≥ 2 dimensions, key + density mandatory in v1).** The energy-ordered
region pick (Decision 2) already names *which* region is B vs C; S26 requires the implementer to also
let that energy drive the section's `density` bias so the louder region's excursion is audibly busier.
*Why more pleasing:* without this, "the eye sweeps twice" is only a tonal abstraction the operator's
ear may not perceive as two *places*; coupling key to density makes the second sweep a felt change of
scene. **This is the one new planner coupling S26 asks for beyond K1** — and it is the minimal fix for
review-S25 note 2 (the energy-order is wired but not yet audibly differentiated). It needs NO new image
field (it reads the existing per-region energy); it is planner-only.

> **Dependency flag (image lens).** True per-excursion *key-color* differentiation (B and C taking
> genuinely different directions because their regions have different valence/hue) needs a per-region
> valence/hue/brightness add to `pure_analysis`, which does not exist (review-S25 note 2). Until then,
> S26's per-excursion contrast rides **density** (real today) + the **distinct-offset nudge** (MX-2).
> The per-region affect add is the named optional sub-slice that upgrades MX-4 from density-only to
> full key-color differentiation. It is NOT an S26 blocker.

---

## 2. THE OPEN-ENDING POLICY (PINNED — the heart of S26)

S24's Invariant A was a *hard* "always resolve home." The operator has overridden it: open / off-home
endings are now a deliberate expressive option, "in line with edge cases of music that behave the
same." My job is the taste — when the unresolved ending is the satisfying choice, and when forcing it
would make a piece sound broken. The answer must be **image-driven and testable**, so the generator can
never (a) leave a clear-subject piece unresolved, nor (b) force-home a panoramic one.

### 2.1 The governing intuition

The ending mirrors **whether the image has a place the eye rests.** A photograph with a clear subject
— a face, a single object, a focal point — is a *resolved scene*: the eye travels and returns to the
subject, so the music must come home. A panoramic field — a horizon, a texture, a crowd, a vista with
no single subject — is an *open scene*: the eye sweeps and never settles, so a manufactured homecoming
would be a lie the ear hears as either pat or arbitrary. **Resolution in the music = a resting place in
the image.**

### 2.2 The decision, as a function of the image knobs (the conditional `resolves_home`)

```
clear_subject  :=  (SUBJECT_SIZE_LO ≤ subject_size ≤ SUBJECT_SIZE_HI)   // a real subject, not the whole frame, not a speck
                   AND  fg_bg_contrast ≥ τ_subject                       // it stands out from the ground
travelling_read := vertical_emphasis ≥ 0.6                              // the form ladder's episodic trigger (selects abac)
                   OR  (the selected form is episodic: abac / abbac)

resolves_home  :=  clear_subject
                   OR  (NOT travelling_read)        // any non-episodic / returning form ALWAYS resolves home
                   OR  fg_bg_contrast ≥ τ_subject   // a strong subject overrides "panoramic" even on a tall image

open_ending    :=  (NOT clear_subject)              // no resting subject
                   AND travelling_read              // an episodic, sweeping read
                   AND fg_bg_contrast < τ_subject   // genuinely low subject/ground separation
```

**Seed thresholds (tuned by ear; directions are craft, exact values are seeds):**

| Constant | Seed | Meaning | Why this value |
|---|---|---|---|
| `τ_subject` | **0.25** | the K1 `key_scheme` gate already in `assets/mappings.json` | reuse the existing "is there a subject/ground stratification" line; one threshold, not two |
| `SUBJECT_SIZE_LO` | **0.10** | a subject must be more than a speck | a tiny salient dot is not a scene's anchor |
| `SUBJECT_SIZE_HI` | **0.85** | a subject filling the frame IS the whole image (panorama of one thing) | when subject ≈ whole frame there is no "ground" to travel away from — treat as a field |

### 2.3 Worked cases (the taste check)

| Image | `subject_size` | `fg_bg_contrast` | `vertical_emphasis` | Decision | Why it's the satisfying choice |
|---|---|---|---|---|---|
| Portrait, single face | 0.4 | 0.6 | 0.4 | **HOME** | a clear subject — the eye rests on the face; the music must return to it |
| Wide landscape horizon | 0.95 (sky fills) | 0.1 | 0.7 | **OPEN** | no subject, tall travelling read, low contrast — a vista has no resting point |
| Tall waterfall, top-to-bottom | 0.3 | 0.15 | 0.8 | **OPEN** | the eye genuinely travels down and doesn't return; an open ending mirrors the descent |
| Busy abstract, no focus | 0.5 | 0.2 | 0.5 | **HOME** (not travelling) | low contrast but NOT an episodic/travelling read → returning form → resolves home (safe default) |
| Object on contrasting ground, tall frame | 0.4 | 0.7 | 0.7 | **HOME** | strong subject overrides the tall frame — `fg_bg_contrast ≥ τ_subject` wins |

**The two protections this guarantees (the operator's exact requirement):**

> **PROTECTION 1 — never leave a clear-subject piece unresolved.** If `clear_subject` is true,
> `open_ending` is forced false regardless of form. A face, an object, a focal point ALWAYS comes home.
>
> **PROTECTION 2 — never force-home a panoramic field.** If `open_ending` is true, the final section's
> offset is NOT zeroed — the piece is permitted (not required) to end on its last excursion's related
> key. The realizer/Music-Theory lens then settles it on a *soft* non-home cadence (§4.2), not a
> Perfect cadence in the wrong key.

### 2.4 Which forms WANT home vs MAY stay open

| Form | Ending policy | Why |
|---|---|---|
| `rounded_binary`, `ternary_aba` | **ALWAYS home** | their final role is `Return` with a **Perfect** cadence — the form IS a homecoming; an open ending would fight the structure |
| `aaba` | **ALWAYS home** | the songwriter's form: the hook returns; the whole point is the familiar A landing home |
| `theme_and_variations` | **ALWAYS home** | the final variation re-grounds the theme; T&V is a single idea recolored, not a journey away |
| `abac` | **MAY stay open** | C is a `Coda` of *new material*; if the image is a travelling field, C ending on its related key is the satisfying open ending; if there's a subject, C resolves home |
| `abbac` | **MAY stay open** | same as abac — the C coda is the open/closed hinge; its catalogue cadence is **Plagal** (a soft "amen"), which suits BOTH a gentle homecoming and a soft open landing |

**The mechanical realization (data).** The conditional is expressed by making the episodic schemes'
**final section's `offset_rule` conditional**: rather than a fixed `"home"` (force-home, S24's
Invariant A) or a fixed `"region_related:c"` (the current always-open data), the C row uses a new
**`"home_if_subject"`** rule tag — resolves to `0` when `clear_subject`/`NOT open_ending`, and to the
C excursion offset when `open_ending`. The returning forms keep their hard `"home"` final row
unchanged. This is one new rule tag in `resolve_key_scheme` and one edited data row; it is byte-stable
because the identity path (`home_only`) never reaches it.

---

## 3. Form-library → image-variety mapping (PINNED) — the variety is MEANINGFUL

The catalogue earns its keep only if a *perceptual reason* connects each image character to its form.
The `form` ladder already encodes most of this; S26 states the aesthetic rationale and binds the key
plan + open-ending decision to it so the escalation is in lockstep, not random.

| Image character | Selected form (ladder, verified) | Travel class | Ending | The perceptual reason |
|---|---|---|---|---|
| **clear subject, mid contrast** (default) | `rounded_binary` | one-trip (ABA) | HOME | a scene with a focal point: leave it, come back to it |
| **strong quadrant contrast** (`quadrant_contrast ≥ 0.6`) | `ternary_aba` | one-trip (ABA) | HOME | a composition split into clearly contrasting halves wants a clean statement→contrast→return |
| **wide, low-bimodality** (`aspect_ratio ≥ 1.6 & bimodality ≤ 0.3`) | `aaba` | one-trip, hook-repeated | HOME | a wide uniform image is "songlike" — one memorable idea stated, departed, returned |
| **tall / vertically-travelling** (`vertical_emphasis ≥ 0.6`) | `abac` | **two-trip (episodic)** | **HOME-IF-SUBJECT / OPEN** | the eye sweeps top-to-bottom — two stops; ends open IF no subject |
| **high-edge, dark** (`edge_activity ≥ 0.7 & value_key ≥ 0.6`) | `abbac` | **two-trip (episodic)** | **HOME-IF-SUBJECT / OPEN** | a busy dark field — a longer episodic sweep; soft Plagal landing |
| **very busy, complex** (`complexity ≥ 0.66 & edge_activity ≥ 0.6`) | `theme_and_variations` | recolor-in-place | HOME | one dense idea seen many ways — not a journey *away*, a journey *through* one thing; re-grounds |

**Rule FL-1 (escalation in lockstep).** The richer the image's structure (more vertical travel / more
edge activity / more distinct fg-bg energy), the more the form ladder escalates one-trip → two-trip,
**and the key plan escalates `aba_excursion` → `abac_rondo` in the same step.** S26 wires the missing
`key_scheme` ladder rule: pick `abac_rondo` when the form ladder selects `abac` (i.e. when
`vertical_emphasis ≥ 0.6`) AND the fg/bg regions are energy-distinct (so the two excursions have
genuinely different source regions, satisfying MX-2/MX-4). This rule is ordered BEFORE the
`aba_excursion` rule so the episodic case wins when both could match. *Why:* the variety is meaningful
precisely when the form and the tonal travel agree — a two-trip key plan on a one-trip form (or vice
versa) is the "random variety" the operator warned against.

**Rule FL-2 (no orphan travel).** A returning form NEVER gets a two-excursion scheme, and an episodic
form NEVER gets a force-home-only scheme when the image is a travelling field. The form-class and the
key-scheme-class are bound by the same image triggers, so they cannot disagree.

---

## 4. "EARNED" — return and open ending as symmetric craft (PINNED)

### 4.1 What makes a multi-excursion RETURN feel earned

A return is *arrived at* by zeroing the final offset; it is *earned* by three gestures, all of which
already live in the catalogue data and the realizer — the key plan leans on them:

1. **The cadential approach.** The penultimate (B or A-Return-waypoint) section leaves on a *weak*
   cadence — ABAC's B closes `Imperfect`, ABBAC's B closes `Deceptive`, the mid `Return` closes `Half`
   — so the ear is left *wanting* resolution. *Why pleasing:* a home that arrives after the ear has
   been left hanging feels inevitable; a home that arrives after a full stop feels redundant.
2. **The contrast payoff.** Home returns having been *framed* by two distinct travelled keys (MX-2);
   the more the excursions contrasted, the more the home tonic *means* on return. *Why pleasing:*
   the return's weight is borrowed from the distance travelled — this is the whole reason MX-2 forbids
   two identical excursions.
3. **The Perfect-cadence landing.** The final `Return`/`Coda` section in the home-resolving forms
   carries a **Perfect** cadence (rounded_binary/ternary/aaba) — a full V→I *in the home key*. *Why
   pleasing:* the cadential homecoming is what makes "offset 0" *sound* like home rather than merely
   *be* home.

> **Dependency flag (Music Theory lens).** Gestures 1 and 3 are CADENCES the realizer renders; this
> design pins *that they must be in the home key* and *that the approach must be weak*. Whether the
> modulation INTO the final home uses a pivot/applied chord (so it sounds prepared, not spliced) is the
> Music Theory lens's K3 pivot work (S24 Decision 5). A multi-excursion return with a hard-cut back to
> home is the same "tape splice" risk K1 carried — closely-related keys keep it tolerable, the pivot
> makes it smooth.

### 4.2 What makes an OPEN ending feel intentional, not abandoned

The symmetric problem: an open ending must read as a *deliberate* unresolved question, not a piece that
forgot to finish. Three rules:

1. **Land on a RELATED key, never a remote one.** The open ending's final offset is a v1-menu entry
   (`{+7, +5, +3, −3}`) — the closely-related set. *Why pleasing:* an unresolved ending on the
   *dominant* sounds like a question (it *wants* home, and withholding home is the expressive act); an
   unresolved ending on a tritone sounds like a mistake. Open ≠ remote.
2. **Settle on a SOFT cadence, never a Perfect cadence in the wrong key.** The final section closes on
   `Plagal` (ABBAC's C, already in the data) or `Imperfect`/`Half` — a gesture that *rests* without
   *resolving*. *Why pleasing:* a Perfect cadence in a non-home key sounds like the piece resolved to
   the WRONG home (a definite ending in the wrong place); a soft cadence on a related key sounds like a
   deliberate fade on an open question. **A Perfect cadence on a non-home final section is FORBIDDEN**
   (guard-rail §5).
3. **The prior home visit must have happened.** ABAC/ABBAC both contain an A `Return` to home BEFORE
   the open C — so the listener HAS heard home; the open ending withholds a *second* homecoming, it
   doesn't deny home entirely. *Why pleasing:* you can only leave a question open if the answer was
   once in view; a piece that never touched home and ends away has no "open" — it just never arrived.

> **Dependency flag (Music Theory lens).** The soft non-home cadence (rule 2) is a realizer cadence in
> a *non-home* key — the theory lens must confirm a Plagal/Imperfect cadence is voiced correctly in the
> excursion key, and that the approach to C does not accidentally tonicize home (which would collapse
> the open ending back to a homecoming). This is the open-ending analogue of the pivot dependency.

---

## 5. Guard-rails as testable properties (PINNED) — the "pleasing" invariants for the multi-excursion + open-ending world

The Test Engineer owns these; the Quality Gate runs them LAST. They UPDATE the K1 suite
(`tests/keyplan_s25.rs`, review-S25 §5) for the new world. Each maps to a property the generator must
never violate.

1. **`identity_path_byte_frozen` (unchanged, hard).** A mappings.json with no firing `key_scheme` rule
   (or `home_only` selected) → every `key_offset_semitones == 0`, `KeyTempoPlan.key_scheme` all-zero;
   `git diff HEAD -- src/engine.rs src/chord_engine.rs tests/engine_equivalence.rs` EMPTY;
   `sha256sum src/engine.rs == 7a07fb8568fcd94536dd3ec2e4dc71c8154f9d9678599a6106e3a542a4343c23`;
   `engine_equivalence` 9/9; goldens 240/114/84/36/79 unmoved. *The identity path stays byte-frozen.*

2. **`resolves_home_when_subject` (CONDITIONAL — replaces the hard S24 Invariant A).** For any image
   where `clear_subject` is true (`SUBJECT_SIZE_LO ≤ subject_size ≤ SUBJECT_SIZE_HI AND fg_bg_contrast
   ≥ τ_subject`), OR the selected form is a returning form (`rounded_binary`/`ternary_aba`/`aaba`/
   `theme_and_variations`), the FINAL section's `key_offset_semitones == 0`. *Protection 1: a
   clear-subject piece is never left unresolved.*

3. **`open_ending_only_when_panoramic` (CONDITIONAL, hard).** The final section's offset is non-zero
   ONLY when `open_ending` is true (`NOT clear_subject AND travelling_read AND fg_bg_contrast <
   τ_subject`) AND the form is episodic (`abac`/`abbac`). For every NON-`open_ending` image the final
   offset is 0. *Protection 2 inverted: a panoramic field is never force-homed; a subject scene is never
   left open.*

4. **`open_ending_lands_on_related_key` (hard).** When the final offset is non-zero (open ending), it
   ∈ the v1 menu `{+7, +5, +3, −3}` — never a remote/off-menu key. *Open ≠ remote (§4.2 rule 1).*

5. **`no_perfect_cadence_off_home` (hard, NEW).** No section with a non-zero `key_offset_semitones`
   carries a `Perfect` `boundary_cadence`. *A definite ending in the wrong key is forbidden (§4.2 rule
   2).* (In the current catalogue this already holds — ABAC's C is `Perfect` but resolves home in the
   home case; the guard bites if a future open-ending scheme leaves a Perfect-cadence section off-home.)

6. **`home_sections_are_home` (unchanged, hard).** Every section with `thematic_role ∈
   {Statement, Return}` has `key_offset == 0`. *Never modulate a section the ear hears as home
   (Invariant B).*

7. **`at_most_two_distinct_non_home_keys` (hard, promoted from K2-safety to governing).** The count of
   *distinct non-zero* offsets across `KeyTempoPlan.key_scheme` is ≤ 2 for ANY image. *MX-1: more than
   two stops is wandering.*

8. **`distinct_excursions_actually_differ` (hard, NEW).** In a two-excursion form (`abac`/`abbac`),
   B's offset ≠ C's offset (after the distinct-excursion nudge). *MX-2: two identical stops is not a
   journey.*

9. **`excursion_couples_density` (soft→hard, NEW — the §1.3 win).** In a two-excursion form, the
   section sourced from the *more energetic* region carries a higher `density` bias than the section
   from the calmer region. *MX-4: the busier region of the image sounds busier — the eye-to-ear link.*

10. **`smooth_keys_only` (unchanged, hard).** Every non-zero offset ∈ `{+7, +5, +3, −3}` for ANY image
    without an explicit OFF-by-default opt-in.

11. **`no_inversion_invariant` (unchanged, hard — re-run because pitch classes move).** Across ALL
    menu offsets `{0, +7, +5, +3, −3}` and ALL characters: `mean_pitch(Bass) < bed/fill <
    mean_pitch(Melody)`, every note ∈ `24..=108`.

12. **`key_scheme_catalogue_round_trips` (unchanged, the mirror witness).** Load the shipped
    `assets/mappings.json`; assert a firing episodic image resolves a two-excursion scheme to two
    distinct non-zero offsets through `plan()`. *Proves the `From<CompositionMappings>` mirror is wired
    AND the abac_rondo ladder rule fires (FL-1).*

---

## 6. Sliceability (PINNED) — one audible win per session

The decisive, lowest-risk ordering. Each slice is independently HEARABLE; each leans on the K1 spine
already shipped; only the named theory/realizer pieces are freeze-sensitive.

### Slice S26-A — the CONDITIONAL open-ending decision on ABAC *(RECOMMENDED FIRST; the heart of S26)*

- **Scope:** the conditional `resolves_home`. Add the `"home_if_subject"` rule tag to
  `resolve_key_scheme` (resolves to 0 when `NOT open_ending`, to the C excursion offset when
  `open_ending`); compute `open_ending` from the §2.2 knobs; edit `abac_rondo`'s C row from
  `"region_related:c"` to `"home_if_subject"`; wire the `key_scheme` ladder rule that picks
  `abac_rondo` on the abac trigger (FL-1). **Files (data + planner ONLY):** `assets/mappings.json`
  (the C row + the ladder rule), `src/composition.rs` (the new rule tag + `open_ending` predicate).
  **`chord_engine.rs`/`engine.rs` untouched** — a non-zero final offset rides the existing transpose
  seam exactly as B does. **Byte-frozen:** reachable only via an image-selected `abac_rondo`; the
  equivalence net never selects it; `home_only` stays all-zero.
- **What the owner hears:** *a tall, subjectless, low-contrast field (a horizon, a waterfall) now
  sweeps through two keys and ENDS OPEN on a related key — it does not manufacture a homecoming. A
  tall image WITH a clear subject sweeps twice and DOES come home. The open ending is now a deliberate
  choice the image earns.*
- **v1-essential.** This is the operator's literal new requirement, planner-only, lowest risk.

### Slice S26-B — multi-excursion contrast: distinct-offset nudge + density coupling *(follow-on, planner-only)*

- **Scope:** MX-2 (the B≠C distinct-excursion nudge, S24 Slice K2) + MX-4 (couple the energy-ordered
  region's energy to the section `density` bias). **Files:** `src/composition.rs` only (the nudge in
  `resolve_key_scheme`/its caller; the density coupling in the section loop). No data, no realizer.
- **What the owner hears:** *the two sweeps now feel like two DIFFERENT places — different key AND the
  busier region's sweep is audibly busier — instead of two tonal abstractions. Fixes review-S25 note
  2's "wired but not audibly differentiated."*

### Slice S26-C — cadential homecoming + open-ending soft-cadence (the freeze-sensitive depth) *(later, ear-gated)*

- **Scope:** the §4 "earned" gestures that touch the REALIZER — the pivot/applied chord into the home
  return (S24 K3 pivot), and confirming the open ending's final section voices a SOFT non-home cadence
  (Plagal/Imperfect) not a Perfect one. **Owned by the Music Theory + Architect lenses**, not this
  lane. **Files:** `chord_engine.rs` (the witnessed pivot/cadence insert) — **the one freeze-sensitive
  slice**, carries its own byte-freeze argument + `no_inversion`/golden re-witness.
- **What the owner hears:** *the key changes stop being abrupt and become hinged; the open ending
  settles softly instead of cutting off — the homecoming and the open question both land cleanly.*

### Open tensions the Music Theory + Architect lenses must resolve

1. **(Music Theory)** The open-ending soft cadence is a cadence in a NON-HOME key — confirm a
   Plagal/Imperfect cadence voices correctly in the excursion key and does NOT accidentally tonicize
   home (which would collapse the deliberate open ending into a homecoming). The open-ending analogue
   of the pivot problem.
2. **(Music Theory)** The multi-excursion return needs a pivot into the final home (the K3 pivot), or a
   two-stop journey home risks the same hard-cut splice K1 carried — closely-related keys keep it
   tolerable, the pivot makes it smooth.
3. **(Architect)** The §1.3 / MX-4 density coupling is a NEW planner read (per-region energy →
   section `density`); confirm it composes with the S23 prominence/density path without double-writing
   `Section.density`, and that it stays byte-stable (it must not fire on `home_only`).
4. **(Image lens, NOT a blocker)** True per-excursion key-COLOR differentiation (B and C taking
   different *directions*, not just different *offsets*) needs a per-region valence/hue/brightness add
   to `pure_analysis` — the named optional sub-slice (review-S25 note 2). Until then S26's
   per-excursion contrast rides density + the distinct-offset nudge, which is sufficient for the
   operator's first re-listen.

---

*Design-only. No source, test, or asset modified by this document. The proposed `mappings.json` rows
(the `abac_rondo` C-row tag change, the `abac_rondo` ladder rule) and planner predicates
(`open_ending`, the `"home_if_subject"` rule tag, the distinct-excursion nudge, the density coupling)
are binding shapes for a later implementer to transcribe; bodies and the single-writer mappings.json
commit are deferred to the slice implementer, coordinated with the Music Theory lens through the lead.
The seam citations (`composition.rs:277/304/450/461/512/530/770/786/794/1175/1183/1215/1260`,
`assets/mappings.json` `form_catalogue`/`key_scheme`/`key_scheme_catalogue`/`form`) are verified
against the working tree at HEAD `9cd9681`. The build-role titles (Rust Architect, Rust Implementer,
Music Theory Specialist, Test Engineer, Quality Gate) are the domain titles carried from the S21/S24
docs.*

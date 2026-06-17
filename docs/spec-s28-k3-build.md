# Spec S28 — Slice K3 BUILD: the realizer pivot/common-tone modulation + land-home cadence

**Author role:** Rust Architect. **DESIGN ONLY.** This document modifies no source, test, or
asset. It produces exactly one artifact — this file. It is the buildable, per-file consolidation of
the K3 sections of `docs/design-s26-multiexcursion-keyplan-engine.md` (§4, §4.1, §4.2, §4.3, §4.4,
§6 "Slice K3", §7 Risk 1), re-anchored against the CURRENT tree.

**Date:** 2026-06-16. **HEAD:** `9fd46ad` (K2b BUILT & CLOSED; `336c66a` K2a; `9cd9681` K1). The
S26 design was cited against `9cd9681`; K2a (`336c66a`) and K2b (`9fd46ad`) have since landed, so
every line number and signature below is **re-verified against `9fd46ad`**.

**Builds on (do not restate):**
- `docs/design-s26-multiexcursion-keyplan-engine.md` — the parent design. This spec CONSOLIDATES its
  §4 seam, §4.2 signatures, §4.3 byte-freeze argument, §4.4 realization decision, §6 Slice K3 owners.
- `docs/review-S27.md` — the K2b Quality Gate (PASS; the two non-blocking carry-forwards reconciled
  below).

---

## 0. WHAT HAS ALREADY LANDED (re-anchoring — read before building)

Three design assumptions from S26 §4 are **already true at `9fd46ad`** and must NOT be re-built:

1. **§4.1(i) PLANNER RE-ROOT IS DONE (K2a).** `composition.rs:1141-1153` already generates each
   section's chords at the per-section root, NOT the literal home root:
   ```rust
   let section_offset = offsets.get(i).copied().unwrap_or(0);
   let section_root_midi = (home_root_midi as i16 + section_offset as i16).clamp(0, 127) as u8;
   let progression = chord_engine.pick_progression(&home_mode);
   let chords = chord_engine.generate_chords(&progression, section_root_midi, &home_mode, ...);
   ```
   The harmony already travels with the melody. **§4.1(i) is NOT pending and is OUT OF SCOPE for
   K3.** Option (ii) (realize-time transpose) was correctly never built. This means K3's only
   realizer job is the PIVOT and the LAND-HOME cadence VOICING — not transposition.

2. **`KeyScheme.{resolution, pivot}` types exist (K2a).** `composition.rs:489-528`:
   `ResolutionPolicy{Resolve, Open}` (`Default = Resolve`) and `KeyScheme { id, sections,
   resolution, pivot }`, all `#[serde(default)]`. The `pivot` field is parsed but **not yet read by
   anything** — K3 is its first consumer.

3. **All seven catalogue schemes ship with `pivot: false` (K2b).** `assets/mappings.json:172-203`.
   The Open scheme `theme_and_variations_excursion` is present-in-data but routed by NO rule
   (`assets/mappings.json:207-215`); its Resolve twin `theme_and_variations_resolve` is what the
   T&V form rule selects.

What is therefore LEFT for K3: (a) carry `pivot`/`resolution` onto `Section`; (b) add the additive
`StepContext.prev_key_offset_semitones`; (c) build it in the engine compose-path ctx; (d) the two
realizer fns in `chord_engine.rs`; (e) flip the chosen schemes to `pivot:true` in `mappings.json`.

---

## 1. CURRENT-STATE ANCHORS (verified at `9fd46ad`)

### 1.1 The two sha256 byte-freeze anchors (verified now)

| File | sha256 at `9fd46ad` | K3 disposition |
|---|---|---|
| `src/engine.rs` | `7a07fb8568fcd94536dd3ec2e4dc71c8154f9d9678599a6106e3a542a4343c23` | **MUST STAY FROZEN.** Matches the S26/K1/K2b anchor exactly. The compose-path ctx build edit must be byte-irrelevant to the golden sweep (see §4). |
| `src/chord_engine.rs` | `b448d9363499234e7e5ddce18fbb3017b754acbea3af51126cd5e51b1215e39b` | **sha WILL change** (two new fns + a guarded hook). Freeze is BEHAVIORAL for this file (§4), witnessed by `engine_equivalence` + a new identity test. |

### 1.2 The realizer seams (verified line/signature at `9fd46ad`)

| Element | Location | Verified shape |
|---|---|---|
| `realize_step` (PUBLIC, FROZEN sig) | `chord_engine.rs:1021-1028` | `pub fn realize_step(step: &StepPlan, inst_idx: usize, num_instruments: usize, features: &PerfFeatures, ms_per_step: u64, ctx: &crate::composition::StepContext) -> Vec<NoteEvent>` — 7 params, unchanged. |
| `theme_pitch` (reads the offset) | `chord_engine.rs:2098-2121` | `tonic_pc = ((ctx.key_tempo.home_root_midi as i16 + ctx.section.key_offset_semitones as i16).rem_euclid(12)) as u8` — already shifts the theme tonic. |
| `generate_chords` (PUBLIC) | `chord_engine.rs:170-179` | `pub fn generate_chords(&self, progression: &[String], root_midi: u8, mode: &str, edge_complexity: f32, brightness_drop: f32, saturation01_raw: f32, colorfulness_raw: f32) -> Vec<Chord>` — already called with `section_root_midi`. |
| Cadence stamping (DATA-DRIVEN, present) | `chord_engine.rs:476-482` (`PhrasePosition`), `:722` (Perfect at final), `:743` (V_CADENCE re-spell) | `plan_phrases` stamps `HalfCadence`/`PerfectAuthenticCadence` and re-spells the boundary chord. K3's land-home only STRENGTHENS the VOICING of the already-stamped Perfect cadence; it adds no step. |
| `is_cadence`/`is_phrase_start` read | `chord_engine.rs:1041-1045` | `matches!(step.position, PhrasePosition::HalfCadence | PhrasePosition::PerfectAuthenticCadence)`. |
| `CadenceStrength` | `composition.rs:327` (enum), carried on `SectionTemplate` `:584` → `Section.boundary_cadence` `:878` | Perfect is the final's cadence (`:722`, `:1167`). |

### 1.3 The planner / context seams (verified at `9fd46ad`)

| Element | Location | Verified shape |
|---|---|---|
| `StepContext` (4 fields TODAY) | `composition.rs:941-949` | `pub struct StepContext<'a> { section: &'a Section, step_in_section: usize, theme: Option<&'a ThemeSeed>, key_tempo: &'a KeyTempoPlan }` — **NO `prev_key_offset_semitones` yet.** |
| `single_section_default` | `composition.rs:956-968` | Builds `{ section, step_in_section: 0, theme: None, key_tempo }`. The equivalence/legacy anchor. |
| `Section` (NO pivot/resolution TODAY) | `composition.rs:858-882` | Has `key_offset_semitones`, `boundary_cadence`, `orchestration`, … but **no `pivot`/`resolution` carry.** K3 adds them. |
| `Section { … }` construction (planner) | `composition.rs:1158-1172` | The single literal where K3 sets the new carry fields. |
| `KeyScheme.{resolution,pivot}` | `composition.rs:511-528` | Already exist; `scheme.pivot`/`scheme.resolution` available where the scheme is resolved. |
| `resolve_key_scheme` | `composition.rs:1395` | Resolves the offsets; the scheme (with its `pivot`/`resolution`) is in scope at the call site that drives section construction. |
| engine compose-path ctx build | `engine.rs:543-560` | `if let Some((section, step_in_section)) = comp.locate(step_idx) { … StepContext { section, step_in_section, theme, key_tempo: &comp.key_tempo } … }`. **This is the ONLY writer that must populate `prev_key_offset_semitones` with a real value.** |
| engine LEGACY flat-path ctx | `engine.rs:576-583` | `StepContext::single_section_default(&section, &key_tempo)` — must keep `prev: None`. |
| engine unit-test ctx builds | `engine.rs:928/939/949` | All `single_section_default` → `prev: None`. |

### 1.4 The equivalence net (the goldens that cannot move)

`tests/engine_equivalence.rs` — **9 `#[test]` fns**. Goldens, verified at `9fd46ad`:
- `G_BASS_NOTE = 36` (`:130`, C2 floor), `G_MELODY_NOTE = 79` (`:135`, G5-area).
- cadence hold `240` (`:278`, `1.20*200`), bass velocity `114` (sat100, `:274`) / `84` (sat0, `:290`).
- Every test builds its ctx via `StepContext::single_section_default(&sec, &kt)` over a
  hand-built `Section { key_offset_semitones: 0, … }` (`:86`). **All goldens live on the identity
  ctx path.** This is the structural reason the pivot guard is dead in the equivalence net.

### 1.5 The `no_inversion_invariant` register guards (the chord_engine behavioral witnesses)

Two existing guards exercise the realizer's register correctness across offsets and must be re-run:
- `tests/keyplan_s25.rs:658` `no_inversion_invariant` — across the K1 menu offsets × home modes.
- `tests/prominence_s23.rs:415` `no_inversion_invariant` — the original hard register guard.

---

## 2. PER-FILE BUILD PLAN (exact owners)

The cadence is BINDING (S21/S24/S26 §6): **Architect spec (this) → Rust Implementer ∥ Music Theory
input (file-disjoint) → Test Engineer → Quality Gate LAST.** Each owner's surface:

### 2.1 `src/chord_engine.rs` — Rust Implementer owns the wiring; Music Theory owns the bodies

Add TWO free functions (module-level, near `theme_pitch`) and ONE guarded hook line at the top of
`realize_step`. The PUBLIC `realize_step` signature is FROZEN (§3). New data rides `ctx`.

**(a) The guarded hook** — first executable lines of `realize_step` body (after the existing
`let role`/`prominence_w`/`pad_voices` reads are fine to follow, but the pivot guard SHOULD be the
first branch so the identity path falls straight through — place it immediately after the doc/sig,
before `assign_role`). Recommended placement: the very top of the body:

```rust
// S28/K3 — pivot/common-tone modulation. Reachable ONLY on a non-`home_only`, `pivot:true`
// scheme at a real key-change boundary. Returns Some(events) → those ARE this step's events
// (the boundary step is realized as the pivot). Returns None on the identity / home_only /
// pivot:false path → fall straight through to the FROZEN free-select/theme path below.
if let Some(pivot) = pivot_chord_events(ctx, features, ms_per_step) {
    return pivot;
}
```

**(b) `pivot_chord_events`** — signature + doc binding; BODY is the Music Theory lens'.

```rust
/// A witnessed pivot / common-tone chord inserted at a MODULATING section boundary (S28/K3).
/// The chord prepares the move from the previous section's key to this section's
/// `key_offset_semitones` so a direct modulation no longer sounds like a splice. Returns the
/// pivot's note events for THIS step, or `None` when no pivot applies (the byte-freeze gate).
///
/// Returns `Some` ONLY when ALL hold (else `None`):
///   (a) the active scheme is non-`home_only` AND `pivot == true` — carried onto the section via
///       the new `Section.pivot` flag the planner sets (§2.2);
///   (b) this is the FIRST step of the section (`ctx.step_in_section == 0`);
///   (c) this section's key differs from the previous section's:
///       `ctx.section.key_offset_semitones != ctx.prev_key_offset_semitones.unwrap_or(/*self*/ ...)`
///       (a `None` prev — first section / identity — is NEVER a key change → `None`).
/// Under `home_only` every offset is 0 and equals its predecessor → `None` → NOTHING inserted.
///
/// Data consumed (all zero-copy off `ctx`): `ctx.section.key_offset_semitones` (destination key),
/// `ctx.prev_key_offset_semitones` (the new `StepContext` field, `None` on the first section /
/// identity), `ctx.key_tempo.home_root_midi`/`home_mode` (the home anchor), `ctx.step_in_section`.
/// The pivot's harmonic RULE (which common-tone / pivot chord, its voicing, its DURATION within
/// the boundary step) is the Music Theory lens' to specify — see §5 hand-off. Pure.
fn pivot_chord_events(
    ctx: &crate::composition::StepContext,
    features: &PerfFeatures,
    ms_per_step: u64,
) -> Option<Vec<NoteEvent>>;
```

**(c) `land_home_is_armed`** — signature + doc binding; BODY (the predicate) is the Music Theory
lens' to confirm; the consuming VOICING change is theirs too.

```rust
/// Is the land-home authentic cadence armed at THIS step (S28/K3)? `true` only when the scheme's
/// `ResolutionPolicy::Resolve` forced the final section to offset 0 AND `pivot == true` AND this
/// is the final section's closing Perfect cadence step. When armed, the realizer STRENGTHENS the
/// VOICING of the already-stamped Perfect cadence into an explicit V→I in the HOME key — it does
/// NOT re-author cadence DATA (`plan_phrases` already stamps `PerfectAuthenticCadence`, and adds
/// no step / moves no boundary). Reads `ctx.section.{boundary_cadence, key_offset_semitones,
/// resolution, pivot}` + `step.position`. Pure. Returns `false` on the identity / `home_only` /
/// `pivot:false` / `Open` path → the voicing is untouched and byte-identical to pre-K3.
fn land_home_is_armed(ctx: &crate::composition::StepContext) -> bool;
```

The `land_home_is_armed` consumer is a small branch inside `realize_step`'s existing cadence
handling (`chord_engine.rs:1041-1045` region): when armed, the Music Theory lens specifies the
re-voicing (e.g. root-position V→I with soprano on the home tonic) for the final boundary's chord
tones — within the existing note path, adding no event count change beyond the voicing.

**Owner split for chord_engine.rs:** the Rust Implementer wires the hook, the fn skeletons, and the
`ctx`/`Section` field reads; the **Music Theory Specialist** authors the BODIES of
`pivot_chord_events` (the common-tone selection + voicing + duration) and `land_home_is_armed` +
the land-home re-voicing. File-disjoint coordination is via the Music Theory input doc (§5) — only
ONE writer commits `chord_engine.rs`; per the S26 §6 discipline the Implementer is the committer and
folds in the Music Theory rule as specified, OR the Music Theory lens commits chord_engine.rs and
the Implementer owns the planner/engine threading. The lead picks one committer; default: **Music
Theory commits `chord_engine.rs` (it owns the harmonic bodies), Implementer commits
`composition.rs` + `engine.rs` + `mappings.json`.**

### 2.2 `src/composition.rs` — Rust Implementer

**(a) Additive `StepContext` field.** `composition.rs:941-949`:

```rust
pub struct StepContext<'a> {
    pub section: &'a Section,
    pub step_in_section: usize,
    pub theme: Option<&'a ThemeSeed>,
    pub key_tempo: &'a KeyTempoPlan,
    /// NEW S28/K3 — the PREVIOUS section's `key_offset_semitones`, or `None` on the first
    /// section / the legacy identity path. The ONLY signal `pivot_chord_events` needs to detect a
    /// modulating boundary. `None` is never a key change (so the identity/legacy ctx is inert).
    pub prev_key_offset_semitones: Option<i8>,
}
```

**(b) `single_section_default` defaults it to `None`** (`composition.rs:956-968`):

```rust
StepContext {
    section,
    step_in_section: 0,
    theme: None,
    key_tempo,
    prev_key_offset_semitones: None, // NEW S28/K3 — identity path never modulates
}
```

This is the §4.3 guarantee-2 contingency made CONCRETE and DEFAULT: by defaulting `None` in the
constructor that the equivalence net and the legacy flat path both use, the engine's compose-path
build is the ONLY writer of a real value (§2.3). If a direct struct-literal in the engine
compose-path proves to perturb the engine.rs sha, switch the engine to ALSO build via a constructor
that defaults the field — see §4 decision rule.

**(c) Carry `pivot`/`resolution` onto `Section`.** `composition.rs:858-882` struct — add after
`boundary_cadence` (`:878`):

```rust
/// NEW S28/K3 — the active scheme's pivot opt-in, copied onto each section so the realizer can
/// read it zero-copy off `ctx.section`. `false` on every legacy/identity/`pivot:false` section →
/// the pivot guard is dead. Set by the planner from `scheme.pivot`.
pub pivot: bool,
/// NEW S28/K3 — the active scheme's resolution policy, copied onto each section so
/// `land_home_is_armed` can tell a Resolve final-return (arm land-home) from an Open ending (do
/// not arm). Defaults to `ResolutionPolicy::Resolve` on legacy/identity sections.
pub resolution: ResolutionPolicy,
```

**(d) Set them in the `Section { … }` planner literal** (`composition.rs:1158-1172`). The scheme is
in scope at the section-construction call site (it drove `resolve_key_scheme`); thread
`scheme.map(|s| s.pivot).unwrap_or(false)` and `scheme.map(|s| s.resolution).unwrap_or_default()`
into the literal. If the scheme handle is not currently held at that exact site, capture
`let scheme_pivot = ...; let scheme_resolution = ...;` once where the scheme is resolved (near the
`resolve_key_scheme` call) and use them in the loop.

**(e) Update the OTHER `Section { … }` literals.** There are test/helper literals that build a
`Section` (e.g. `composition.rs:1501`, `:1797`, `:2075` build `boundary_cadence: Perfect`). Each
must add `pivot: false, resolution: ResolutionPolicy::Resolve` (the identity values) to compile.
These are NON-realizer construction sites; they keep the identity path. (Implementer: a quick
`grep -n "Section {" src/composition.rs src/engine.rs` finds every literal; engine.rs's
`legacy_default_section` at `engine.rs:743` region is one — see §2.3.)

**Module boundary held:** these are planner-side DATA carries; no music logic, no pixels.

### 2.3 `src/engine.rs` — Rust Implementer (BYTE-CRITICAL — see §4)

**(a) Compose-path ctx build** (`engine.rs:547-555`): populate the new field with the PREVIOUS
section's offset. The previous section is the one whose index is `section_index - 1`; the engine
already has `comp` and resolves `(section, step_in_section)` via `comp.locate(step_idx)`. Compute
the prev offset from the composition's section list (the section index is derivable from `locate`,
or a tiny helper on `CompositionPlan` returns the prior section's `key_offset_semitones`). Set:

```rust
let ctx = StepContext {
    section,
    step_in_section,
    theme,
    key_tempo: &comp.key_tempo,
    prev_key_offset_semitones: prev_offset, // NEW S28/K3 — None for section 0
};
```

where `prev_offset: Option<i8>` is `None` when the located section is index 0, else
`Some(comp.sections[idx-1].key_offset_semitones)`.

**(b) `legacy_default_section` carry** (`engine.rs:743` region): the helper that builds the
throwaway legacy `Section` must add `pivot: false, resolution: ResolutionPolicy::Resolve` to
compile (it feeds `single_section_default`, which sets `prev: None`). This keeps the legacy flat
path on the identity branch.

**(c) The legacy flat-path ctx and all engine unit-test ctx builds** stay on
`single_section_default` (`engine.rs:581`, `:928`, `:939`, `:949`) → `prev: None`, untouched.

**This is the ONLY behavior `engine.rs` adds: building one additive `Option<i8>` field on the
compose path.** It is byte-irrelevant to the golden sweep because the goldens run the identity ctx
(§4). The Implementer MUST re-witness the engine.rs sha after the edit (§4 decision rule).

### 2.4 `assets/mappings.json` — Rust Implementer (SOLE committer for the flips)

Flip `"pivot": false → true` on the **returning-Resolve** schemes only — the schemes whose final
section resolves HOME so a prepared modulation + land-home cadence is the audible win:

| scheme id (`mappings.json`) | line | flip to `pivot:true`? | why |
|---|---|---|---|
| `rounded_binary_excursion` | :172 | **YES** | A–B–A', B departs and returns → pivot in, land home. |
| `ternary_aba_excursion` | :176 | **YES** | classic ternary departure/return. |
| `aaba_excursion` | :180 | **YES** | the bridge departs, A returns home. |
| `abac_rondo` | :185 | **YES** | two excursions, both resolve home (Invariant A). |
| `abbac_excursion` | :190 | **YES** | two diverging excursions, returns home. |
| `theme_and_variations_resolve` | :196 | **YES** | the ROUTED T&V twin; ends home. |
| `theme_and_variations_excursion` (Open) | :200 | **NO — STAYS `pivot:false`, STAYS UNROUTED** | the operator lock: the Open scheme must NOT sound until a re-listen un-gates it (review-S27 nit N-K2b-2). Keeping `pivot:false` AND unrouted is double-safe. |

`home_only` has no `pivot` key (defaults false) — untouched. Do NOT touch the `key_scheme.rules`
selector (the Open scheme stays unrouted; review-S27 CHECK 4 tripwire `no_routed_image_ends_off_home`
must keep passing).

> **DECISION POINT for the lead / Music Theory:** flipping all six Resolve schemes at once maximizes
> the audible win but also the ear-test surface. A conservative alternative is to flip ONE
> well-understood scheme first (recommend `ternary_aba_excursion` — the cleanest single
> departure/return) for the first re-listen, then flip the rest in a fast follow once the pivot
> sounds right. The Music Theory lens picks; default = flip all six Resolve schemes, re-listen,
> revert any that sound worse than the K2b abrupt-but-honest splice.

### 2.5 LOCKOFF set (binding — NOT touched by K3)

`src/midi_output.rs`, `src/synth*` (all synth modules), `src/cli.rs`, `src/tui.rs`, `src/main.rs`,
`src/modem.rs`, `src/bin/*`, `src/lib.rs` are NOT modified. **`src/main.rs` is locked** — the §4.4
realization is the no-touch legato-overlap fallback (§4.4 below), so the scheduler is untouched. No
cross-step-sustain scheduler change. `src/pure_analysis.rs` is also untouched (K3 adds no perceptual
field; the per-region affect already landed in K2a).

---

## 3. THE BYTE-FREEZE ARGUMENT FOR K3 (explicit)

The realizer change is reachable ONLY through a non-`home_only`, `pivot:true` scheme AT a key-change
boundary. The identity / `home_only` / `pivot:false` path is byte-identical, by these guarantees:

1. **The pivot is gated on three ANDed conditions** (`pivot_chord_events` §2.1(b)): scheme
   `pivot == true` (defaulted `false`) AND `ctx.step_in_section == 0` AND
   `ctx.section.key_offset_semitones != prev`. Under `home_only` every offset is 0 and every section
   equals its predecessor (and `prev` is `None` on section 0) → the guard returns `None` → the
   `return pivot;` line is never taken → control falls into the FROZEN free-select/theme path.

2. **`realize_step`'s PUBLIC signature is FROZEN** (`chord_engine.rs:1021-1028`, 7 params). The new
   data rides `ctx` (the additive-PRIVATE-ctx route) exactly as the S15 pad/S18/S23 prominence
   slices threaded behavior without touching the signature. The Quality Gate's CHECK-1-style
   "`realize_step` public signature byte-identical worktree vs HEAD" must pass.

3. **`engine.rs` MUST stay sha `7a07fb8568fcd94536dd3ec2e4dc71c8154f9d9678599a6106e3a542a4343c23`.**
   The ONLY engine.rs change is building the additive `prev_key_offset_semitones` into the
   COMPOSE-path ctx (`engine.rs:547-555`). The golden sweep
   (`tests/engine_equivalence.rs`) runs the IDENTITY ctx via `single_section_default` over a
   hand-built `Section { key_offset_semitones: 0, … }` — it NEVER touches the compose path. So the
   goldens 240/114/84/36/79 cannot move regardless of the compose-path ctx edit.
   **The §4.3 guarantee-2 contingency, concretely (and ALREADY the default route here):** the field
   is defaulted to `None` IN THE `single_section_default` CONSTRUCTOR (§2.2(b)), so the equivalence
   net and the legacy flat path are the only `single_section_default` callers and are untouched;
   the engine's compose-path struct-literal is the SOLE writer of a real value.
   **DECISION RULE for WHEN to apply the contingency further:** after the engine.rs edit, run
   `sha256sum src/engine.rs`. **If it still equals the anchor → done, no further action.** **If it
   differs** (e.g. adding the struct-literal field or a `prev_offset` local nudged the byte layout),
   then move the compose-path build to ALSO go through a constructor — e.g. extend the existing
   `single_section_default` pattern with a sibling `with_prev(section, key_tempo, step_in_section,
   theme, prev)` constructor in `composition.rs`, so `engine.rs` calls a function rather than
   open-coding the literal, minimizing the engine.rs textual delta until the sha re-matches. The
   goal state is: engine.rs sha re-equals the anchor after the slice. (If, after exhausting the
   constructor route, the sha STILL cannot be held — because adding ANY real compose-path value is
   inherently a textual change — the freeze is re-witnessed BEHAVIORALLY via the equivalence net
   9/9 staying byte-green and the new identity test green, and the sha-anchor is formally
   re-baselined with a one-line note in the Quality Gate review. Prefer the constructor route first;
   re-baseline is the last resort, and the lead must sign off.)

4. **`chord_engine.rs` carries its OWN freeze re-witness (BEHAVIORAL, not byte).** Its sha256 WILL
   change (two new fns + the hook). The witnesses:
   - a NEW `pivot_inserts_nothing_on_identity` test (Test Engineer, §6) — for every `home_only` /
     `pivot:false` plan, the realized note stream is byte-identical to a pre-K3 capture (or,
     equivalently, `pivot_chord_events` returns `None` and `land_home_is_armed` returns `false`
     for every step of an identity plan);
   - the `engine_equivalence` 9/9 goldens stay 240/114/84/36/79 (they exercise only the identity
     ctx);
   - `no_inversion_invariant` re-run (BOTH `tests/keyplan_s25.rs:658` and
     `tests/prominence_s23.rs:415`) stays green — the register guard cannot break under the pivot.

5. **The land-home cadence inserts NOTHING structurally.** `land_home_is_armed` only STRENGTHENS the
   VOICING of an already-stamped `PerfectAuthenticCadence` (`plan_phrases` already stamps it,
   `chord_engine.rs:722`); it adds no step, moves no boundary, changes no event count. It is armed
   ONLY under `pivot:true` + `Resolve` + the final boundary. Under identity it is `false`.

**Witness set (machine-checkable, the Quality Gate must run ALL):**
- `sha256sum src/engine.rs` == `7a07fb8568fcd94536dd3ec2e4dc71c8154f9d9678599a6106e3a542a4343c23`
  (or the contingency in guarantee 3 applied + signed off).
- `cargo test --test engine_equivalence` → 9/9 byte-green, goldens unmoved.
- `realize_step` PUBLIC signature byte-identical worktree vs HEAD.
- `pivot_inserts_nothing_on_identity` green.
- `no_inversion_invariant` green in BOTH `keyplan_s25.rs` and `prominence_s23.rs`.
- the K2b operator-lock tripwire `no_routed_image_ends_off_home` (`tests/keyplan_k2b.rs`) still
  green (the Open scheme stays unrouted, `pivot:false`).
- full default net `cargo test` green.

---

## 4. THE §4.4 REALIZATION DECISION — no-touch legato-overlap (DEFAULT, BINDING)

`src/main.rs` schedules per-step `note_on`/`note_off` pairs, each note bounded within its own step's
`hold_ms`. A pivot chord that wanted to SUSTAIN a common tone ACROSS the section boundary cannot
extend a note past its step under the current scheduler — and **K3 does NOT change the scheduler.**

**The DEFAULT and ONLY in-scope realization:** the pivot is realized ENTIRELY WITHIN the boundary
step (the FIRST step of the modulating section, `step_in_section == 0`). `pivot_chord_events`
returns the pivot chord's `NoteEvent`s for that one step; those notes hold to the end of THEIR step
(per the existing per-step `hold_ms` envelope, near `realize_step`'s existing duration handling),
and the NEXT step's downbeat OVERLAPS naturally because consecutive steps' note windows already abut
(the legato-overlap the adapter already produces). The common-tone "hold" is heard as the pivot
chord ringing into the new section's downbeat — NOT as a literal cross-step tied note.

**Specify to the Implementer/Music Theory lens, precisely so no one reaches for a scheduler change:**
- The pivot occupies the boundary step only. It does NOT request a note longer than `ms_per_step`.
- The common tone is voiced as a note WITHIN the boundary step that lands on the same pitch the new
  section's downbeat will sound, so the ear connects them across the abutting step windows.
- A true cross-step TIED sustain (one `note_on` spanning two steps with the `note_off` deferred) is
  **OUT OF SCOPE for K3** — it would require a `main.rs` scheduler change and is its own future,
  freeze-sensitive slice if the ear demands it. `main.rs` STAYS LOCKED.

This keeps `main.rs`, the synth sinks, and the MIDI output untouched.

---

## 5. HAND-OFF TO THE MUSIC THEORY SPECIALIST (the harmonic RULES)

The Music Theory lens owns a file-disjoint INPUT DOC (e.g. `docs/input-s28-k3-pivot-harmony.md`) and
the BODIES of `pivot_chord_events` / `land_home_is_armed` + the land-home re-voicing. It must specify:

1. **The pivot/common-tone chord.** Given `home_root_midi`, `home_mode`, the PREVIOUS section's key
   (`home_root + prev_offset`), and the DESTINATION key (`home_root + key_offset_semitones`): which
   chord is the pivot? Options the lens chooses among — a true common-chord pivot (a triad diatonic
   to BOTH keys, the textbook common-chord modulation), or a common-TONE pivot (a single shared
   pitch class bridging the keys when no common chord exists, e.g. for a ±relative or distant move),
   or a dominant-prep (V/destination) when neither is clean. The menu offsets are `{+7,+5,+3,−3}`
   (dominant, subdominant, +relative, −relative), so the lens should give a RULE per offset class.
2. **Its voicing.** Register, spacing, which voice carries the common tone — respecting the existing
   no-inversion register frame (`no_inversion_invariant` must stay green; the pivot's notes stay in
   the realizer's existing 24..=108 / role-register bounds).
3. **Its duration WITHIN the boundary step.** Per §4: the pivot occupies the boundary step; the lens
   says whether it sounds for the full `ms_per_step` or a portion, and how the common tone seats so
   the legato-overlap to the next downbeat reads as a hinge, not a splice.
4. **The land-home cadence.** Confirm `land_home_is_armed`'s predicate (Resolve + pivot + final
   Perfect-cadence step) and specify the V→I re-voicing in the HOME key (root-position V→I, soprano
   on the home tonic for a true Perfect Authentic Cadence) — STRENGTHENING the already-stamped
   Perfect cadence's voicing, adding no step.
5. **Which schemes deserve `pivot:true`** (confirm or trim §2.4's six) and whether the all-at-once
   vs one-first decision point (§2.4) should be conservative for the first re-listen.

The Music Theory lens does NOT touch `engine.rs`/`composition.rs` threading or `main.rs`. Its
mappings.json surface is only confirming the `pivot:true` flip set (the Implementer commits it).

---

## 6. HAND-OFFS — Implementer & Test Engineer

**Rust Implementer owns:**
- `composition.rs`: the additive `StepContext.prev_key_offset_semitones` (+ `None` default in
  `single_section_default`); the `Section.{pivot, resolution}` carry + setting them in the planner
  `Section { … }` literal from the resolved scheme; updating every other `Section { … }` literal to
  the identity values so the tree compiles.
- `engine.rs`: building `prev_key_offset_semitones` on the COMPOSE path only (+ the
  `legacy_default_section` identity carry); re-witnessing the engine.rs sha (§3 guarantee 3).
- `assets/mappings.json`: the `pivot:true` flips per §2.4 (SOLE committer).
- The `pivot_chord_events`/`land_home_is_armed` SKELETONS + the guarded hook line (if the lead
  assigns chord_engine.rs commit to the Implementer rather than Music Theory; default is Music
  Theory commits chord_engine.rs — §2.1).
- Opportunistic cleanup (§7): prune/confirm the dead `excursion_offset` in `composition.rs:1368`.

**Test Engineer must witness (tests only; no production code):**
- `pivot_inserts_nothing_on_identity` (NEW) — `home_only` and any `pivot:false` plan: every step's
  realized stream is byte-identical to a pre-K3 capture (or `pivot_chord_events` is `None` /
  `land_home_is_armed` is `false` for every step). The PRIMARY byte-freeze behavioral witness.
- a `pivot_fires_on_modulating_boundary` POSITIVE test — a `pivot:true` scheme on a firing image
  produces a non-empty pivot at the first step of a section whose offset differs from its
  predecessor, and produces NOTHING at non-boundary steps and at same-key boundaries.
- `land_home_voicing_on_resolve_final` — a Resolve + `pivot:true` form's final boundary is voiced
  V→I in the home key (the cadence chord's notes are the home V then home I), and the event COUNT is
  unchanged from the K2b stamp (no added step).
- re-run / re-assert `no_inversion_invariant` across the pivot path (extend the existing guards or
  add a pivot-aware sibling) — no register inversion under any pivot.
- confirm `cargo test --test engine_equivalence` 9/9 and the K2b `no_routed_image_ends_off_home`
  tripwire stay green.

**Quality Gate LAST** — runs the §3 witness set, the module-boundary audit (chord_engine.rs has no
image logic; the pivot reads `ctx` only and names no pixel type; engine.rs adds no note-selection),
the codename scrub, the full net, and the engine.rs sha re-witness; verdict PASS / PASS WITH ISSUES
/ FAIL.

---

## 7. THE TWO K2b NON-BLOCKERS (review-S27 carry-forwards)

1. **N-K2b-1 — dead `excursion_offset` under `--no-default-features`** (`composition.rs:1368`, a
   WARNING not an error). K3 touches `composition.rs` heavily, so it MAY opportunistically clean
   this in-file: confirm whether `excursion_offset` is still reachable under the default feature set
   (it is the K1 shim that `region_excursion_offset` subsumed — §2.2 of the S26 design); if no
   default-feature caller remains, prune it or `#[cfg_attr(not(feature = "..."), allow(dead_code))]`
   it. This is OPTIONAL and IN-FILE only; it must not perturb the byte-freeze witnesses (it is in
   `composition.rs`, not `engine.rs`/`chord_engine.rs`, and removing dead code changes neither
   sha-anchored file). If touching it risks the slice, DEFER it.
2. **N-K2b-2 — Open-ending abruptness re-listen.** This is precisely what K3 unblocks IN PRINCIPLE,
   but the operator lock keeps the Open scheme (`theme_and_variations_excursion`) UNROUTED and
   `pivot:false` (§2.4). Do NOT un-gate the Open scheme in K3. After K3's pivot/cadence sounds right
   on the Resolve schemes, a SEPARATE future re-listen decides whether to route + `pivot:true` the
   Open scheme. **The bin `--no-default-features` break (review-S27 CHECK 8 — `modem.rs`/`main.rs`/
   `bin/*` feature-gating, S11-era) is LOCKED-file territory and is LEFT to a future bin-owning
   slice — K3 touches no locked file and is not chargeable for it.**

---

*Design-only. No source, test, or asset modified by this document. All line numbers, signatures, and
the two sha256 anchors are verified against the working tree at HEAD `9fd46ad`. Signatures + types +
doc comments are binding shapes; bodies belong to the slice implementers; the pivot's harmonic RULES
belong to the Music Theory lens. The build-role titles (Architect, Implementer, Music Theory
Specialist, Test Engineer, Quality Gate) are the S21/S24/S26 domain titles.*

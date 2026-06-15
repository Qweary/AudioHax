# S15 — The Expanded-Variety Composition Engine (engineering envelope)

**Author role:** Rust Architect (DESIGN ONLY — no source, test, or asset modified by this document; `docs/` only).
**Date:** 2026-06-15.
**Status:** PROPOSE-FOR-ITERATION. Goes back to the operator. A Music Theory Specialist is, in parallel, proposing the actual expanded *musical* vocabulary + selection rules; this document owns whether/how that vocabulary fits the engine cleanly, in scope, without breaking the `engine_equivalence` byte-freeze.
**Grounded against** the working tree at the S13 head: `src/engine.rs`, `src/chord_engine.rs`, `src/pure_analysis.rs`, `tests/engine_equivalence.rs`, and the canonical assessment `docs/assessment-composition-architecture.md` (Section C signatures, Section D roadmap, the recommended first build slice, the open decisions).

> This pass adapts the S14 baseline under **one consciously-accepted operator override of the S14 R3 "vocabulary discipline" mitigation:** the operator wants MORE variety across every dimension — many forms, expanded character/meter, richer mood/energy/colour mappings, richer theme behaviour — controlled by **"default standards + vary-on-conditions"** (deterministic threshold/range selection over image features, never randomness). The S14 assessment deliberately kept the vocabulary small and closed; the operator has overridden that. The architectural question this raises is no longer "how do we keep the vocabulary small" but **"how do we let the vocabulary grow without an enum edit per addition, while staying type-safe, deterministic, and byte-freeze-safe."** That is §1 below, and it is the central call.

---

## 0. Executive summary (read first)

1. **Type representation (§1) — the central call.** Keep `Character` and `Meter` as **closed enums** (they bind to *mechanism*, not to vocabulary size — see §1.3). Replace the closed **`Form` enum with a data-driven `FormSpec`**: a form becomes a `Vec<SectionTemplate>` (each a `ThematicRole` + relative length + theme directive + boundary-cadence directive) **loaded from `mappings.json`**. Adding ABBA / ABAC / AABA / rondo / theme-and-variations / … becomes a **JSON table entry, not a Rust enum edit or a recompile.** `Form` survives only as a small *named-handle* enum-or-string used for logging/selection; the *structure* lives in data. This is the one representation change versus S14 §C.3, and it is exactly what the operator's "more forms" directive demands.
2. **Selection mechanism (§2).** Formalize `PlanMappings` as a set of **ordered conditional-departure tables** over the `ImageUnderstanding` knobs. Each musical axis (form / character / meter / key-scheme / theme-behaviour) is `{ default: <id>, rules: [ {when: [ {knob, op, threshold} ...], pick: <id>} ... ] }`. The planner walks each axis's rules **in order, first-match-wins, falling back to `default`** — fully deterministic given `(understanding, mappings.json)`. Tunable without recompile (the S13 `mappings.json` precedent). No randomness in the *selection*; the only non-determinism is the pre-existing `pick_progression` `thread_rng`, which the equivalence net already isolates.
3. **Back-compat (§3) — non-negotiable, unchanged from S14 §C.6.** The expanded vocabulary lands behind the behaviour-neutral default `StepContext` (single section, no theme, home key, identity variation). `tests/engine_equivalence.rs` stays byte-green by being fed that default context. **OPEN DECISION #12 resolved: `StepContext<'a>` BORROWED, with a small `tick` restructure** (compute `ctx` from an immutable borrow *before* the `&mut self` advance — §3.3). Landing order is types → `ctx` param defaulted at every call site (incl. the net) → `compose_from_image` producing >1 section only when called.
4. **Scope (§4) — the load-bearing judgment.** TIER 1 (slice 1) ships **exactly the spine plus the variety that is free given the spine:** the non-looping sectioned plan, a returning theme, `StepContext` threading behind the freeze, the S13 articulation clamp — and, as the *only* expanded variety, **the data-driven `FormSpec` carrying the section-sequencing-only forms** (ABA′ rounded binary, ternary ABA, AABA, ABAC, ABBA, theme-and-variations-as-section-list) **all at 4/4, home key, Ballad, identity/light theme variation.** Everything needing *new mechanism* defers: meter ≠ 4/4 (Stage 3), real modulation (Stage 5), climax marking (Stage 6), new variation *techniques* (Stage 7), full character presets (Stage 4). The `FormSpec` representation is built in slice 1; its *contents* grow over later stages without engine edits.
5. **Reconciliation (§5).** The Music Theory vocabulary fills: `FormSpec` table rows (the form catalogue), the `PlanMappings` rule thresholds (what image property picks what), `ThemeSeed.motif` encoding, the `CharacterOverlay`/`Meter` musical contents. The one place their proposal *can* force a type-shape change is flagged: if a form needs a **per-section variable meter or a within-section tempo ramp** in slice 1 (it should not), that is a `StepContext`/`Section` field addition we reconcile in synthesis rather than absorb silently.

---

# 1. Type representation — closed enums vs a data-driven form spec

## 1.1 The question, precisely

S14 §C.3 fixed `Form`, `Character`, `Meter` as **closed `enum`s** with a small curated variant set, *because* the S14 R3 mitigation was "keep the vocabulary small." The operator has overridden the smallness goal for variety. So for each of the three axes the question is: **does "much larger vocabulary" mean "just more enum variants" (cheap, type-safe, but an enum edit + recompile per addition), or does it call for a data-driven representation that grows without touching Rust?**

The answer is *different per axis*, because the three axes differ in how tightly their vocabulary binds to engine *mechanism*.

## 1.2 `Form` → **data-driven `FormSpec`** (the representation change)

A form, mechanically, is **nothing but a section sequence**: an ordered list of `(ThematicRole, relative-length, which-theme, how-the-theme-varies, boundary-cadence-strength)`. ABA′, AABA, ABAC, ABBA, ABABA-rondo, T-V1-V2-V3 — these differ *only* in that list. The engine does not branch on "is this a rondo"; it walks `sections[]`. That means **the form vocabulary has zero coupling to engine mechanism** — it is pure data the planner expands into `Vec<Section>`.

Therefore: **represent the form catalogue as data, loaded from `mappings.json`.** Adding a form is adding a JSON row. No enum edit, no recompile, no engine change — which is precisely the operator's "many forms" requirement made cheap.

```rust
/// One section's role in a FORM TEMPLATE — pure structure, no music content yet.
/// The planner expands a FormSpec's templates into concrete `Section`s by filling
/// key/tempo/mode/progression from the KeyTempoPlan + chord_engine craft.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct SectionTemplate {
    /// "A" / "B" / "A'" / "T" / "V1" … — label carried to the snapshot/observer.
    pub label: String,
    /// Statement | Contrast | Return | Development | Coda (drives theme + harmony posture).
    pub role: ThematicRole,
    /// Relative weight; the planner scales these to fill `total_steps` (so total step
    /// budget is image/character-driven, the PROPORTIONS are the form's).
    pub rel_len: f32,
    /// Which theme this section states/recalls, by theme slot index, or none.
    pub theme: Option<usize>,
    /// How it varies its theme on recall. Identity for a first statement. (TIER-1 set
    /// is {Identity, Transposed-by-key-plan, Fragmented}; richer techniques are Stage 7.)
    pub variation: ThemeVariation,
    /// The cadence that CLOSES this section — half inside, PAC at the structural close.
    pub boundary_cadence: CadenceStrength,
}

/// A FORM = an ordered section template list + a stable handle for logging/selection.
/// THE FORM VOCABULARY LIVES HERE, IN mappings.json — adding ABBA/AABA/rondo/T+V is a
/// JSON row, NOT a Rust enum edit. This is the S15 representation change vs S14 §C.3's
/// closed `Form` enum (the operator's "more forms without recompile" directive).
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct FormSpec {
    /// Stable id, e.g. "rounded_binary" / "ternary_aba" / "aaba" / "theme_and_variations".
    pub id: String,
    /// The ordered section templates that ARE this form.
    pub sections: Vec<SectionTemplate>,
}
```

`CompositionPlan.form` then carries the **selected `FormSpec`'s id** (a `String` handle) for the snapshot/observer, while `CompositionPlan.sections: Vec<Section>` (unchanged from S14) is the *expanded, concrete* sequence the time cursor walks. The S14 `enum Form { RoundedBinary, TernaryABA, … }` is **deleted** as a structural type and survives, if desired, only as a non-authoritative convenience constant set for tests — the authority moves to the JSON catalogue.

**Type-safety is preserved** despite the move to data: `ThematicRole`, `ThemeVariation`, `CadenceStrength` stay closed enums (they bind to mechanism — see §1.3), so a `SectionTemplate` cannot name a role/variation/cadence the realizer doesn't implement; `serde` rejects an unknown variant at load with a clear error. The *open* part (how many forms, what their section sequences are) is data; the *closed* part (what a section can mechanically do) is types. That is the right seam.

**Why not just add enum variants?** Adding `Form::AABA`, `Form::Rondo`, … is type-safe but (a) requires a Rust edit + recompile per form, defeating "tunable without recompile," and (b) tempts a `match self.form { … }` somewhere, which is the coupling we want to *avoid* — the engine should never branch on form identity, only walk sections. Data-driven `FormSpec` structurally prevents that branch from being written.

## 1.3 `Character` and `Meter` → **stay closed enums**

These are the opposite case: their vocabulary **binds directly to engine mechanism**, so each new variant is *not* free — it needs code.

- **`Meter`** (`Four4 / Three4 / Six8 / Two4`) drives `beats_per_measure` and the metric-accent re-pointing (S14 §A.4) and the per-role beat-masks. A new meter is a new accent pattern + mask — *mechanism*, not data. Keep it a closed enum; it grows by deliberate Stage-3+ code, and there is no operator value in "arbitrary meters from JSON" (an unbounded meter surface is musically meaningless and mechanically expensive). **The expanded meter vocabulary the operator wants is still a small closed set — it just gets *built out* over Stage 3, not opened to JSON.**
- **`Character`** is a *bundle of concrete biases* (tempo window, articulation multiplier, texture/beat-mask selection, dynamic posture). The biases themselves are data (`CharacterOverlay`, below); but *which biases exist* — the set of scalars the realizer reads — is mechanism. So **`Character` stays a closed enum that selects a `CharacterOverlay`**, and the overlay's *values* live in `mappings.json` (tunable). New character → a new overlay row in JSON *if* it reuses the existing bias knobs; a new enum variant + code *only if* it needs a new kind of bias. This gives the operator "richer character mappings" (re-tune the overlays freely) without opening the bias *vocabulary* to sprawl.

```rust
/// The per-character bias bundle — VALUES live in mappings.json (tunable, no recompile),
/// the SET OF KNOBS is fixed mechanism. Defaults are identity (×1.0/+0) so the back-compat
/// default plan reproduces today's goldens exactly (§3).
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct CharacterOverlay {
    pub tempo_lo_ms: u64, pub tempo_hi_ms: u64,   // character tempo window (clamps S13 result)
    pub articulation_mult: f32,                   // ×base_frac BEFORE the §3 clamp (default 1.0)
    pub velocity_level_bias: i8,                  // +/- on realize_velocity level (default 0)
    pub meter: Meter,                             // character SELECTS meter (Stage 3+)
    // beat-mask / texture selection added at Stage 3-4; default = "no per-beat gating".
}
```

## 1.4 Net of §1

| Axis | Representation | Grows by | Type-safety |
|---|---|---|---|
| **Form** | **data-driven `FormSpec` (JSON)** | JSON row, no recompile | closed `ThematicRole`/`ThemeVariation`/`CadenceStrength` inside each row |
| **Meter** | **closed enum** | Stage-3+ code (new accent/mask) | full |
| **Character** | **closed enum → `CharacterOverlay` (JSON values)** | JSON overlay row (re-tune) or enum+code (new bias kind) | full |

`Section`, `ThemeSeed`, `MotifNote`, `KeyTempoPlan`, `StepContext`, `CompositionPlan` keep their S14 §C shapes — the only field-level change is `CompositionPlan.form: Form` (closed enum) **→ `form: String`** (the selected `FormSpec.id` handle) plus the new `FormSpec`/`SectionTemplate`/`CharacterOverlay` types. The expanded *form* vocabulary therefore costs **zero new structural Rust types beyond these three serde structs**, and grows entirely in data.

---

# 2. The selection mechanism — `PlanMappings` / `mappings.json` ("default + conditional-departure")

## 2.1 The shape

The operator's control mechanism is **"default standards + vary-on-conditions":** every musical axis has a default, and image features deterministically select a departure from threshold/range tables. Encode that literally. `PlanMappings` is a set of **per-axis conditional tables**; each table is `{ default, rules }`, each rule is `{ when: [predicates], pick }`, evaluated **in declaration order, first match wins, else `default`.**

```rust
/// Curated plan-selection tables over the ImageUnderstanding knobs. Loaded from
/// mappings.json (tunable, no recompile — S13 precedent). Encodes the operator's
/// "default + conditional-departure" control: each axis has a default id and an ordered
/// rule list; the planner picks the FIRST rule whose predicates all hold, else default.
/// DETERMINISTIC given (understanding, this).
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct PlanMappings {
    pub form: SelectTable,          // → a FormSpec.id from the form_catalogue
    pub character: SelectTable,     // → a Character variant
    pub meter: SelectTable,         // → (informational; character also implies meter)
    pub key_scheme: SelectTable,    // → a named key-relation scheme id (home/dominant/relative/parallel)
    pub theme_behaviour: SelectTable, // → how the theme behaves in B (absent/fragment/second-theme)
    pub form_catalogue: Vec<FormSpec>,            // the data-driven form vocabulary (§1.2)
    pub character_overlays: Vec<(Character, CharacterOverlay)>, // §1.3 values
    pub key_schemes: Vec<NamedKeyScheme>,         // (id → Vec<i8> section offsets)
}

/// One axis's "default + ordered conditional departures." `pick`/`default` are string ids
/// resolved against the relevant catalogue (FormSpec.id, Character name, key-scheme id, …).
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct SelectTable {
    pub default: String,
    pub rules: Vec<SelectRule>,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct SelectRule {
    pub when: Vec<Predicate>,   // ALL must hold (AND); rules are tried in order (first-match)
    pub pick: String,
}

/// A single threshold/range test over one ImageUnderstanding knob. Closed op set keeps it
/// type-safe and bounded — NOT an expression language (R3 discipline survives the override:
/// the VOCABULARY grew, the SELECTION-LANGUAGE stayed a bounded threshold table).
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct Predicate {
    pub knob: Knob,             // closed enum naming an ImageUnderstanding field
    pub op: CmpOp,              // Lt | Le | Gt | Ge | InRange
    pub lo: f32,                // threshold (InRange uses lo..=hi)
    pub hi: f32,                // unused except InRange
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
pub enum CmpOp { Lt, Le, Gt, Ge, InRange }

/// Closed handle naming a selectable ImageUnderstanding knob. New knob → enum + a getter
/// arm; this is the ONE deliberate coupling between the table and the struct, and it keeps
/// JSON from naming a field that doesn't exist (serde rejects unknown variants at load).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
pub enum Knob {
    EdgeActivity, Texture, Complexity, Colorfulness, ValueKey, AvgBrightness, AvgSaturation,
    DominantHue, PaletteBimodality, QuadrantContrast, VerticalEmphasis, AspectRatio,
    SubjectSize, FgBgContrast,
}
```

## 2.2 How `CompositionPlanner::plan` consumes it (deterministically)

```text
fn plan(&self, u: &ImageUnderstanding) -> CompositionPlan:
  1. form_id   = self.plan_mappings.form.select(u)        // first-match-wins, else default
  2. character = self.plan_mappings.character.select(u)   // → Character variant
  3. overlay   = lookup(character_overlays, character)
  4. key_id    = self.plan_mappings.key_scheme.select(u)  // (TIER-1: forced "home_only")
  5. theme_beh = self.plan_mappings.theme_behaviour.select(u)
  6. form_spec = lookup(form_catalogue, form_id)
  7. KeyTempoPlan: home_root from dominant_hue lookup; base_ms from brightness clamped to
       overlay.tempo_lo/hi; key_scheme from key_id (TIER-1: all zeros); tempo_scheme flat (TIER-1)
  8. total_steps from a base budget × form section count (image-influenced; deterministic)
  9. expand form_spec.sections → Vec<Section> : scale rel_len → step_len to fill total_steps;
       fill each Section.{key_offset, ms_per_step, mode, progression} from KeyTempoPlan +
       per-section chord_engine craft (pick_progression/generate_chords/plan_phrases over the
       section's chords); set theme/variation/boundary_cadence from the SectionTemplate
  10. themes: generate ThemeSeed(s) from u (hue + edge_activity → curated contour) per theme_beh
  11. assemble CompositionPlan { form: form_id, character, meter: overlay.meter, key_tempo,
       sections, themes, total_steps }
```

`SelectTable::select(u)` is a pure scan: for each rule, if every `Predicate` holds against `u`'s knob, return `pick`; else fall to `default`. **Determinism:** every input to `select` is `(u, table)`; no clock, no RNG. The *only* non-determinism in `plan()` is the delegated `pick_progression` (`thread_rng`) — exactly the boundary S9/S13 documented, and the equivalence net never exercises this path (it pins the realizer on a fixed hand-built plan). The selection layer the operator tunes is **fully deterministic;** re-running the same image + same `mappings.json` yields the same form/character/meter/key/theme choices.

## 2.3 Why this honours the override *and* the spirit of R3

The operator overrode "small vocabulary," not "no sprawl ever." The discipline that survives: **the vocabulary (forms, characters, key-schemes) is open and lives in data; the selection *language* is a bounded threshold table, NOT a Turing-complete composition DSL.** `Predicate` has five comparison ops over named knobs — no arithmetic, no nesting beyond AND-of-predicates, no user-defined functions. That is the right place to hold the line: rich *content*, bounded *control surface*. Adding a form or re-tuning a threshold is safe and recompile-free; the engine can never be asked to evaluate an arbitrary expression.

---

# 3. Back-compat byte-freeze landing + the `StepContext` borrow resolution

## 3.1 The freeze, restated from the actual test

`tests/engine_equivalence.rs` calls `decide_instrument_action(&f, inst, step, num, &plan, MS_PER_STEP)` — **six args, no `ctx`** — and pins P3: `plan[step_idx % plan.len()]` (modulo wrap), the golden pitches `G_BASS_NOTE=36`/`G_MELODY_NOTE=79`, cadence velocity `114`/`84`, cadence hold `240 ms`. The expanded engine must keep this byte-green. The mechanism is unchanged from S14 §C.6: a **behaviour-neutral default `StepContext`** (single `Section` {label "A", `step_len = total`, `key_offset = 0`, home mode, `theme = None`, `variation = Identity`, `boundary_cadence` = today's}, constant `key_scheme`/`tempo_scheme`) under which the kernel does exactly what it does today — no transposition, no theme, home key, same `ms_per_step`.

## 3.2 Landing order (each step keeps the net green)

1. **Types only.** Add `composition.rs` (`CompositionPlan`, `Section`, `KeyTempoPlan`, `ThemeSeed`, `MotifNote`, `StepContext`, `FormSpec`, `SectionTemplate`, `CharacterOverlay`, `PlanMappings` + the enums) and the `engine::ImageUnderstanding` mirror + boundary copy in `pure_analysis.rs`. **No behaviour change → net GREEN.**
2. **`ctx` parameter, defaulted everywhere.** Add `ctx: &StepContext` to `decide_instrument_action` (now 7 args). Provide `StepContext::single_section_default(plan_len, ms_per_step, …) -> StepContext<'static>` (or owning helper, see §3.3). Wire the default at **every call site:** `decide_step` (engine.rs:481), and — critically — **update `tests/engine_equivalence.rs` to pass the default `ctx`.** Because the default applies zero transposition / no theme / home key, **the goldens (240 ms, 114/84, 36/79) do not move; net GREEN.** This is the S9 discipline: extend the signature without changing the output at the legacy operating point. *Note: updating the equivalence test's call sites to pass the default `ctx` is the one allowed touch of `tests/engine_equivalence.rs` in slice 1 — it adds an argument, it does not relax an assert. The asserts are unchanged.*
3. **`compose_from_image` / `CompositionPlanner` producing >1 section ONLY when called.** The new sectioned behaviour is reachable only via the new compose path (analogue of S13's `set_features_global` boundary — the net never calls it). `set_features_global` still produces a single-section-equivalent plan. The golden moves only when a slice deliberately re-derives it — which in slice 1 is **only** the §3.4 articulation clamp.

## 3.3 OPEN DECISION #12 — `StepContext` borrowed vs owned: **BORROWED, with a small `tick` restructure**

**Recommendation: `StepContext<'a>` borrowed (zero-copy), as S14 §C.5 shows.** The owned alternative deep-copies a `Section` (which contains a `Vec<String> progression` and `String`s) per step per instrument — wasteful and, worse, it would make `StepContext` carry cloned music data that can silently drift from `self.plan`. Borrowed is correct.

**The borrow-checker detail, concretely against the real code.** Today `tick` (engine.rs:405) takes `&mut self`, calls `self.decide_step(source, step_idx)` (which borrows `&self.plan` immutably inside, fine because `decide_step` is `&self`), then at **line 448 reads `self.plan[step_idx % self.plan.len()].position`** for the phrase snapshot, then at **line 457 mutates `self.step_index`**. With `StepContext<'a>` borrowing `&self.plan`'s `Section`/`ThemeSeed`/`KeyTempoPlan`, the conflict is: we want to build `ctx` (an immutable borrow of `self.plan`/`self.key_tempo`) and hand it to `decide_step`, but `tick` later does `self.step_index = step_idx + 1` (a `&mut self` field write). Rust allows this **as long as the immutable borrow held by `ctx` has ended before the mutable field write.** It does — `decide_step` returns owned `Vec<InstrumentDecision>`, so `ctx` (and the borrow) is dropped at the end of the `decide_step` call expression, well before line 457. **The clean structure:**

```text
fn tick(&mut self, source, sink):
    if paused { return … }
    let step_idx = self.step_index;
    // Resolve section + ctx from an IMMUTABLE borrow, fully scoped to decide_step.
    let decisions = {
        let ctx = self.step_context(step_idx);     // borrows &self.plan/&self.key_tempo
        self.decide_step_ctx(source, step_idx, &ctx) // returns OWNED Vec; borrow ends here
    };
    // … sink sends (uses owned `decisions`) …
    // phrase snapshot: read via a SHORT immutable borrow, then drop, then advance:
    let phrase = self.snapshot_phrase(step_idx);    // returns owned PhrasePosition
    self.last_phrase = phrase;
    let total = source.step_count().max(1);
    self.step_index = step_idx + 1;                  // &mut field write — no live borrow now
    …
```

The restructure is small and additive: `decide_step` gains a `_ctx`-passing sibling (or simply takes `ctx: &StepContext`), and the section/phrase resolution moves into a `fn step_context(&self, step_idx) -> StepContext` + a `fn snapshot_phrase(&self, step_idx) -> PhrasePosition` helper. No method on `PipelineEngine` holds a `self.plan` borrow across the `step_index` write. **This compiles under NLL with no `unsafe`, no `RefCell`, no clone.** The `single_section_default` helper for the equivalence test builds a `StepContext` from a locally-owned `Section`/`KeyTempoPlan` the test holds (the test owns the lifetime), so the borrowed `StepContext<'a>` is happy there too — the test constructs the backing `Section`/`KeyTempoPlan` as locals and borrows them into the `ctx`, exactly as it already constructs `fixed_plan()` as a local.

**One subtlety for the implementer (flag, not a blocker):** `decide_step` currently iterates `row.iter().enumerate()` and pushes per instrument; the `ctx` is the SAME for all instruments of a step (it is step-relative, not instrument-relative), so it is built once per step and borrowed by each `decide_instrument_action` call — no per-instrument rebuild. Good.

## 3.4 The S13 articulation clamp (slice-1 ride-along, the one deliberate golden move)

The clamp lives at `chord_engine.rs:1164`: `base_frac … .clamp(0.30, 1.20)`, fed by `curve_frac = LEGATO_FRAC_HI(1.05) + (STACCATO_FRAC(0.40) - 1.05)*edge_activity`. The §D fix narrows the **non-cadence** window to roughly `0.55 ≤ base_frac ≤ 1.10` (kills the click-short and the mud-long extremes without flattening responsiveness). This **deliberately moves the non-cadence articulation goldens** — re-derive the affected constants **by hand in the same commit** with a comment pointing at this section (S13 §7 discipline), and **keep the cadence branch byte-stable** (the `sustained` helper's `(frac*rit).min(1.20)` and the `240 ms` cadence hold are untouched — the clamp change is on the non-cadence `base_frac` path only). The per-character `articulation_mult` (§1.3) then rides *on top of* this clamped window in later stages (Ballad→toward 1.10, March→toward 0.55), defaulting to ×1.0 in slice 1 so it is a no-op at the default plan. **Owner: Music Theory Specialist** (articulation craft); this doc only fixes *where* and *that the cadence branch stays frozen*.

---

# 4. SCOPE — TIER 1 (slice 1) vs TIER 2+ (later)

The partition rule is mechanical, not aesthetic: **TIER 1 = variety whose mechanism is JUST section-sequencing + theme placement + cadence-plan over the existing 4/4 / home-key / Ballad craft.** TIER 2+ = anything that needs a *new mechanism surface*. The first slice must still ship the spine; the expanded-variety addition it can absorb *for free* is **the data-driven form catalogue**, because expanding the form vocabulary is, by §1.2, purely a longer `sections[]` list — no new mechanism.

## 4.1 TIER 1 — lands in slice 1

**The spine (non-negotiable, from the canonical first build slice):**
- `ImageUnderstanding` re-exposing the four dead features (`hue_spread`/`texture_laplacian_var`/`shape_complexity`/`aspect_ratio`) + the cheap palette/balance knobs that slice-1 selection actually reads. Trivial; mostly re-exposure.
- A minimal `CompositionPlanner` that lays out the selected form's sections and **expands to a non-looping flat `StepPlan` sequence played once start-to-finish** — this **kills `plan[step_idx % plan.len()]`** (the structural root cause). The realizer still walks plan steps, but the engine drives `step_index` 0→`total_steps` once, never wrapping; the per-section `&[StepPlan]` is the section's own filled phrase plan.
- A **generated returning theme** (curated contour set, image-seeded by hue + edge_activity) the **Melody role plays in Statement/Return sections and is absent/fragmented in Contrast** (per `theme_behaviour` selection).
- **Differentiated cadence strength** from each `SectionTemplate.boundary_cadence` (half cadence closes a question section; PAC closes the structural return).
- The engine `compose_from_image` / `set_plan` / `StepContext` threading behind the **back-compat default plan** keeping `engine_equivalence` byte-green (§3).
- The **S13 articulation clamp** (§3.4).

**The expanded-variety addition slice 1 absorbs (the only one — it's free):**
- **The data-driven `FormSpec` representation (§1.2) + a TIER-1 form catalogue in `mappings.json`** carrying the **section-sequencing-only forms:** rounded binary `A B A′`, ternary `A B A`, `A A B A`, `A B A C`, `A B B A C`, and **theme-and-variations-as-a-section-list** (`T V1 V2 [V3]` where each variation section is, in slice 1, the theme restated with **light/identity variation** — the *technique* set is TIER-1-limited, the *form* is present). All at **4/4, home key, Ballad defaults, identity-or-light theme variation.** These cost nothing beyond data because the planner just expands a longer `sections[]`; the time cursor and realizer already handle "more sections."
- **The `PlanMappings` selection scaffold (§2)** with the `form` / `theme_behaviour` tables live (the operator can tune which image → which form, which theme behaviour). `character` / `meter` / `key_scheme` tables are *present in the schema* but **TIER-1-pinned to their defaults** (Ballad / 4/4 / home_only) so no un-built mechanism is reachable.

**Why theme-and-variations is TIER-1 as a *form* but not as a *technique*:** the form is a section list (free); the *variation techniques* (augmentation/diminution/reharmonization/ornamentation) are new realizer mechanism (Stage 7). So slice 1 ships T-V1-V2 as a *form* where the variations are, mechanically, identity-or-fragment restatements over their own per-section chord plans — audibly "the theme again, slightly different harmony" — and the rich transformations land at Stage 7 *into the same form rows*, no engine change.

## 4.2 TIER 2+ — defers (each needs new mechanism)

| Deferred capability | Why it needs new mechanism | Stage |
|---|---|---|
| **Meter ≠ 4/4** (3/4 waltz, 6/8, 2/4) | `beats_per_measure` + re-point `realize_velocity` accent from `position_in_phrase` to `metric_position` + per-role beat-masks | Stage 3 |
| **Full character presets** (Waltz/March/Lament active) | `CharacterOverlay` biases must actually drive tempo-window/articulation_mult/velocity_bias/beat-mask in the realizer | Stage 4 |
| **Real modulation** (B "goes somewhere") | applied-dominant pivot + non-zero `key_scheme` + transposition in the realizer driven by `section.key_offset` | Stage 5 |
| **Climax marking** | `is_climax` field + realizer pushing register/dynamics/density to high ends at a structural point | Stage 6 |
| **New variation *techniques*** (augment/diminish/reharmonize/ornament/fragment-as-development) | new transforms on the recalled motif in the realizer | Stage 7 |
| **Within-section tempo ramp** (closing ritardando across a section) | a per-step tempo field on `StepContext` (OPEN DECISION #10) | defer unless music side asks |
| **Region-saliency upgrade** (DoG subject mask) | new `pure_analysis` saliency submodule | Stage 9 |
| **Semantic tier** | `candle` + model, feature-gated | Stage 10 |

**The scope guard, stated plainly so it does not balloon:** slice 1 adds **exactly one** new mechanism surface beyond the spine — *none*. The expanded form vocabulary rides on the spine's own section-sequencing. Every other expanded-variety axis the operator wants (meter, character, key, climax, variation techniques) is **already staged** in the canonical roadmap and stays there. The `FormSpec`/`PlanMappings`/`CharacterOverlay` *types* are built in slice 1 (so later stages fill data/values, not types), but only the *form* and *theme-behaviour* tables are *active*; the rest are schema-present, default-pinned. This is how the operator gets "much more variety" in slice 1 (many forms, deterministic theme behaviour) **without** the slice's mechanism surface growing past the spine.

---

# 5. Reconciliation hooks for the Music Theory vocabulary

The Music Theory Specialist's parallel proposal fills these shells; flagged here so synthesis is mechanical.

1. **The form catalogue (`form_catalogue: Vec<FormSpec>`).** The music side authors the actual `SectionTemplate` rows for each form — the `rel_len` proportions, which sections carry the theme, the `variation` per section, and the `boundary_cadence` per section (the cadence hierarchy: half inside, PAC at the structural close). The engine guarantees the rows are *expandable and walkable*; the music side owns *what the rows are*. **No type change** — pure data.
2. **The selection thresholds (`PlanMappings.*.rules`).** The music side (with the heuristic mapping table §B.4 of the assessment) authors the `Predicate` thresholds: which `ImageUnderstanding` knob, which `CmpOp`, which value picks which form/character/key-scheme/theme-behaviour. This is the "fits the image" contract. **No type change** unless they need a knob not in the `Knob` enum → then add one `Knob` variant + one getter arm (flagged below).
3. **`ThemeSeed.motif` / `MotifNote` encoding (OPEN DECISION #9).** The shells (`Vec<MotifNote { degree, dur_steps }>`, key-relative) are present. If the music side leans **contour-anchors** rather than absolute degrees, that is a `MotifNote` field reshape (e.g. `degree: i8` → a `ContourStep` enum) — **a type-shape change we reconcile in synthesis**, not absorb silently. Recommended floor stays degree-contour (transposes cleanly by `key_offset`).
4. **`CharacterOverlay` values + `Meter` musical contents.** The music side fills the overlay scalars (tempo windows, articulation multipliers, velocity bias, beat-masks) and the meter accent patterns. Slice 1 only needs the **Ballad** overlay = identity (so the default plan reproduces goldens); the rest are Stage-4 data.
5. **Theme behaviour in B (OPEN DECISION #6).** The `theme_behaviour` `SelectTable` resolves to `absent` / `fragment` / `second_theme`. The engine supports `theme: Option<usize>` per section + a `Fragmented` variation; if the music side wants a *contrasting second theme*, that is `themes: Vec<ThemeSeed>` with len ≥ 2 and a Contrast section pointing at slot 1 — **already expressible, no type change.**

**The one place their proposal can force a type-shape change** (so we watch for it in synthesis): if a TIER-1 form needs **per-section meter** or a **within-section tempo ramp** in slice 1, that adds a field to `Section`/`StepContext` (`meter_override: Option<Meter>` or a per-step tempo). The recommendation is that slice 1 does **not** need either (all TIER-1 forms are 4/4, section-stable tempo); if the music side disagrees, we reconcile the field addition explicitly rather than letting it leak in.

---

## Decision points for the operator (consolidated)

- **Confirm the `Form`-as-data representation (§1.2).** This is the central S15 call: forms become JSON rows, not Rust enum variants, so the vocabulary grows without recompile. Recommended — it is the literal mechanism for your "more forms" directive. (`Character`/`Meter` stay closed enums because they bind to mechanism, not vocabulary size.)
- **Confirm the bounded selection language (§2.3).** Rich vocabulary in data; selection stays a threshold table (`Knob` × `CmpOp` × value, AND-of-predicates), *not* a composition DSL. This is where the R3 discipline survives your vocabulary override.
- **Confirm the TIER-1 form set for slice 1 (§4.1):** rounded-binary, ternary ABA, AABA, ABAC, ABBAC, theme-and-variations-as-section-list — all 4/4 / home-key / Ballad / identity-or-light theme variation. Anything needing meter/modulation/character-presets/climax/variation-techniques is staged later (§4.2). Is that the right "more variety now, free given the spine" line, or do you want a specific deferred axis pulled forward (cost: it brings its mechanism into slice 1)?
- **Note resolved #12:** `StepContext` is **borrowed**, with the small `tick` restructure in §3.3 — no operator action, recorded for the implementer.
- **Still open from the assessment, unchanged:** #8 compose-as-default (recommend yes once slice 1 is hearable), #9 motif encoding (degree-contour floor; reconcile if music wants contour-anchors), #11 semantic tier go/no-go (decide after Stage 9).

*Design-only. No source, test, or asset modified by this document.*

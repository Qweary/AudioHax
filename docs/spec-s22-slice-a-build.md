# Spec S22 — Slice A build: AFFECT → CHARACTER + TEMPO DE-CAP (Implementer-ready)

**Author role:** Rust Architect — BUILDABLE-SPEC pass. DESIGN/SPEC ONLY: this document
modifies no source, test, or asset. Its single artifact is this file. It translates the
**locked** S21 design (`docs/design-s21-affective-fidelity.md` and its three companion
streams) into exact, unambiguous build instructions so two implementers — a Rust plumbing
implementer (the `.rs` files) and a data-author (the one `.json` file) — and a Test Engineer
can each build their half with NO further questions.
**Date:** 2026-06-15. **Repo HEAD this spec is pinned against:** `30b4ce2`. **Default build:
pure-Rust.**

> The S21 design is LOCKED. This spec does not re-design — it pins. Every signature,
> JSON row, and line reference below is verified against the working tree at HEAD, not
> trusted from the design prose. Where the prose and the code disagree, the code wins and
> the discrepancy is called out.

---

## 0. Scope, in one paragraph (read first)

Slice A breaks the "every image is a slow ballad" failure with **DATA + planner only**.
It (1) pools existing perceptual scalars into two continuous affect axes — **arousal** and
**valence** — via one new pure fn `affect_composite`; (2) exposes them to the existing
`SelectTable` ladder as two new `Knob` variants; (3) fills the empty `character` ladder so a
bright/energetic image selects `Scherzo` and a dark/calm one selects `Lament`/`Nocturne`;
(4) replaces the single hard Ballad tempo clamp with a **per-character tempo window** so a
bright image can exceed 96 BPM and a dark one can fall below 56 BPM. **`chord_engine.rs` and
`engine.rs` are NOT touched.** Character drives **tempo + character-selection** in Slice A;
the per-character articulation/rhythm/harmonic realization and the valence→major/minor mode
lean are explicitly deferred to later slices (see §7). The `engine_equivalence` byte-freeze
(goldens 240/114/84/36/79) is provably unmoved (§5).

---

## 1. Ownership & single-writer discipline (DISJOINT)

Two implementers build Slice A; their file sets are **disjoint** and may run in parallel.

| Builder | OWNS (may modify) | MUST NOT touch |
|---|---|---|
| **Rust Implementer (plumbing)** | `src/composition.rs`, `src/mapping_loader.rs`, `src/pure_analysis.rs` (sentinels ONLY) | `assets/mappings.json`, everything in the Lock List (§6) |
| **Data-author** | `assets/mappings.json` (the §4 rows ONLY) | every `.rs` file |
| **Test Engineer** | `tests/affect_s22.rs` (new) | all production `src/*` and `assets/*` |

`assets/mappings.json` is single-writer **this slice = the data-author**. The Rust
Implementer never edits it. The Music Theory Specialist supplies the per-character tempo-window
numbers and the ladder-threshold confirmation as a **file-disjoint input doc**, NOT a direct
edit — the data-author transcribes those numbers into the rows in §4. One writer, one commit
per file.

---

## 2. Verified code anchors (cited so the Implementer lands precisely)

All confirmed at HEAD `30b4ce2`:

| Anchor | File:line | Fact (verified) |
|---|---|---|
| `struct ImageUnderstanding` | `composition.rs:38–88` | 22 fields; last three are `subject_energy`, `foreground_energy`, `background_energy`. **Add the two new sentinel fields at the END of the struct.** |
| `ImageUnderstanding::neutral()` | `composition.rs:93–118` | sets every field; **add the two new fields here = `-1.0`.** |
| `enum Character` | `composition.rs:177–188` | `Ballad, Hymn, Nocturne, Drone, March, Lament, Waltz, Scherzo, Lilt, Gigue` — **all ten already exist. `Scherzo` exists. DO NOT add a variant.** |
| `enum Knob` | `composition.rs:338–356` | 17 variants `EdgeActivity..BackgroundEnergy`, `#[serde(rename_all="snake_case")]`. **Add `Arousal`, `Valence` at the END.** |
| `Knob::read` | `composition.rs:361–381` | one arm per variant. **Add two arms.** |
| `enum CmpOp` | `composition.rs:387–393` | `Lt, Le, Gt, Ge, InRange` — the ops the JSON rules use; no new op needed. |
| `struct PlanMappings` | `composition.rs:465–493` | has `form, character, meter, key_scheme, theme_behaviour, texture (#[serde(default)]), form_catalogue, texture_catalogue (#[serde(default)]), figuration_catalogue (#[serde(default)])`. **Add `affect` `#[serde(default)]`.** |
| `From<CompositionMappings> for PlanMappings` | `composition.rs:495–512` | maps every field 1:1. **Add `affect: c.affect`.** |
| planner select+store of `character` | `composition.rs:701` | `let character = parse_character(&self.plan_mappings.character.select(u));` — the ladder runs here; **the affect composite must be computed and written into `u` BEFORE this line** (see §3.6). Note `u: &ImageUnderstanding` is borrowed `&` here — see §3.6 for the mutation discipline. |
| tempo block | `composition.rs:725–728` | `let bpm = interp_tempo_bpm(...)` (`:725`); `let bpm = bpm.clamp(BALLAD_BPM_MIN, BALLAD_BPM_MAX);` (`:727`); `base_ms_per_step = (60_000.0/bpm.max(1.0)).round() as u64;` (`:728`). **Replace `:726–727` (the comment + clamp) with the `character_tempo_bpm` call.** |
| `const BALLAD_BPM_MIN/MAX` | `composition.rs:851–852` | `56.0`/`96.0`; **only reader is `:727`.** DELETE both consts (incl. the `:850` doc comment); values move into `affect.character_tempo.ballad`. |
| `fn interp_tempo_bpm` | `composition.rs:949–978` | UNCHANGED — still produces the raw brightness→BPM. |
| `understand_image_pure` construction | `pure_analysis.rs:738–` | builds `ImageUnderstanding { ... }`; last set field is `subject_size: subj.area_frac` at `:754`+. **Add the two new sentinel fields = `-1.0` in this literal.** |
| `struct CompositionMappings` | `mapping_loader.rs:110–138` | the actual deserialize target; mirrors `PlanMappings`. **Add `affect` `#[serde(default)]`.** |
| `brightness_to_tempo_bpm` | `mappings.json:16–20` | `0-30:60, 31-70:90, 71-100:120` — DE-CAP per §4(a). |
| `"character"` | `mappings.json:134` | `{ "default": "ballad", "rules": [] }` — FILL per §4(c). |
| nesting | `mappings.json` `composition` block | `character`/`meter`/`texture_catalogue` live INSIDE the top-level `"composition": { ... }` object (sibling of `global`). The new `affect` block is a sibling key inside `composition`. |

---

## 3. The Rust surface (composition.rs + mapping_loader.rs) — exact signatures

The plumbing Implementer writes the BODIES; this spec pins every SIGNATURE, every field, and
the composite formula numerically. Signatures only below — no bodies.

### 3.1 The `Affect` struct + `AffectMappings`/`CharacterTempo` types (composition.rs)

```rust
/// The affect composite — the image's valence/arousal coordinates, each 0..1 (0.5 neutral),
/// derived purely from the perceptual scalars already on `ImageUnderstanding`. NO new image
/// extraction. Computed ONCE per plan in `composition.rs` (the planner's module). Pure: no
/// pixels, no RNG, no clock.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Affect {
    /// Arousal / energy, 0 (calm) .. 1 (energetic). Saturation-led.
    pub arousal: f32,
    /// Valence / mood, 0 (dark/tense) .. 1 (bright/pleasant). Brightness-led.
    pub valence: f32,
}

/// One character's tempo window (BPM), loaded from `affect.character_tempo.<character>`.
#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize)]
pub struct CharacterTempo {
    pub bpm_min: f32,
    pub bpm_max: f32,
}

/// The `affect` mapping block: composite weights + per-character tempo windows. All fields
/// `#[serde(default)]` so a partial/absent block still parses. NEW S22.
#[derive(Debug, Clone, PartialEq, serde::Deserialize, Default)]
pub struct AffectMappings {
    /// Weight per ImageUnderstanding field name (snake_case JSON keys) for the arousal blend.
    #[serde(default)]
    pub arousal_weights: std::collections::HashMap<String, f32>,
    /// Weight per ImageUnderstanding field name for the valence blend. The `fg_bg_contrast`
    /// term is fed through the `0.5 + 0.5*x` fluency transform INSIDE `affect_composite`
    /// (NOT pre-transformed in JSON).
    #[serde(default)]
    pub valence_weights: std::collections::HashMap<String, f32>,
    /// Per-character tempo windows, keyed by lowercase character name ("ballad","scherzo",…).
    #[serde(default)]
    pub character_tempo: std::collections::HashMap<String, CharacterTempo>,
}
```

**`AffectMappings::default()` is load-bearing (the byte-freeze floor).** `#[derive(Default)]`
on the struct yields empty maps — which is WRONG, because the no-`affect`-block path must
ship the legacy `ballad:{56,96}` window so `character_tempo_bpm` reproduces the old clamp. So
**do NOT derive `Default`; hand-implement it** to seed the single legacy window:

```rust
impl Default for AffectMappings {
    /// The no-`affect`-block floor: empty weight maps (the composite then degenerates to a
    /// neutral 0.5/0.5 — harmless, since with no `affect` block the character ladder is also
    /// empty and the plan stays Ballad) AND the SINGLE legacy `ballad:{56,96}` tempo window,
    /// so `character_tempo_bpm(raw, Ballad, default)` == the old `clamp(56,96)` byte-for-byte.
    fn default() -> Self { /* arousal_weights: empty; valence_weights: empty;
        character_tempo: { "ballad": CharacterTempo { bpm_min: 56.0, bpm_max: 96.0 } } */ }
}
```

(Remove the `Default` from the `#[derive(...)]` list above when hand-implementing; the derive
line then reads `#[derive(Debug, Clone, PartialEq, serde::Deserialize)]`.)

### 3.2 `affect_composite` — the pure composite fn (composition.rs)

```rust
/// Pure. Weighted blend of EXISTING ImageUnderstanding scalars under the JSON weights.
/// The two HSV scalars (`avg_saturation`, `avg_brightness`) are 0..100 and divided by 100;
/// the rest are already 0..1. Output each clamped to 0..1.
///
/// AROUSAL = 0.45*(avg_saturation/100) + 0.25*colorfulness + 0.20*edge_activity + 0.10*complexity
/// VALENCE = 0.70*(avg_brightness/100) + 0.20*(avg_saturation/100) + 0.10*(0.5 + 0.5*fg_bg_contrast)
///
/// The weights come from `weights.arousal_weights` / `weights.valence_weights` keyed by the
/// snake_case field name. For each weighted field the term is `weight * normalized_field`,
/// where normalization is: avg_saturation→/100, avg_brightness→/100, fg_bg_contrast→fluency
/// transform (0.5 + 0.5*x), all others→identity. Sum, then clamp 0..1. When a weight map is
/// EMPTY (the default floor / no-affect-block path) the corresponding axis returns the neutral
/// 0.5 (an empty blend has no terms; seed it to 0.5 so a Ge/Le rule reads "neutral").
fn affect_composite(u: &ImageUnderstanding, weights: &AffectMappings) -> Affect;
```

**Field-name ⇆ struct-field mapping the Implementer must honor** (verified against
`ImageUnderstanding` at `composition.rs:38–88`):

| JSON weight key | `ImageUnderstanding` field | Normalization in `affect_composite` |
|---|---|---|
| `avg_saturation` | `u.avg_saturation` (0..100) | `/ 100.0` |
| `colorfulness` | `u.colorfulness` (0..1) | identity |
| `edge_activity` | `u.edge_activity` (0..1) | identity |
| `complexity` | `u.complexity` (0..1) | identity |
| `avg_brightness` | `u.avg_brightness` (0..100) | `/ 100.0` |
| `fg_bg_contrast` | `u.fg_bg_contrast` (0..1) | fluency: `0.5 + 0.5 * x` |

These six are the only keys the §4 weight rows use. The composite formula is the
**authoritative reconciled formula** (unified design §Decision 3); the `fluency` transform is
applied inside the fn, NOT expressed as a raw weight.

### 3.3 The two `Knob` variants + their `read()` arms (composition.rs)

Append to `enum Knob` (after `BackgroundEnergy`, `composition.rs:355`):

```rust
    /// NEW S22 — the planner-computed arousal composite (0..1). Reads the runtime-only
    /// `affect_arousal` field the planner fills via `affect_composite` (NOT a pixel field).
    Arousal,
    /// NEW S22 — the planner-computed valence composite (0..1). Same discipline.
    Valence,
```

Append to `Knob::read` (after the `BackgroundEnergy` arm, `composition.rs:379`):

```rust
            Knob::Arousal => u.affect_arousal,
            Knob::Valence => u.affect_valence,
```

`#[serde(rename_all="snake_case")]` on `Knob` means JSON spells these `"arousal"` / `"valence"`
— matching the §4(c) ladder rows. serde rejects any unknown knob name (closed enum preserved).

### 3.4 The two `ImageUnderstanding` sentinel fields + `neutral()` value (composition.rs)

Append to `struct ImageUnderstanding` (after `background_energy`, `composition.rs:87`):

```rust
    /// NEW S22 — the planner-computed arousal composite (0..1). NOT extracted from pixels and
    /// NOT deserialized; `pure_analysis::understand_image_pure` and `neutral()` leave it at the
    /// `-1.0` sentinel ("not yet computed"), and the planner overwrites it via `affect_composite`
    /// before the character/tempo ladders run. `Knob::Arousal` reads this. Keeping it off the
    /// pixel producer holds the module boundary (`pure_analysis.rs` writes the sentinel, never
    /// a real value). The `-1.0` sentinel is below any real 0..1 value, so a `Ge`/`Gt` ladder
    /// rule reading an unfilled composite never spuriously fires.
    pub affect_arousal: f32,
    /// NEW S22 — the planner-computed valence composite (0..1). Same sentinel discipline.
    pub affect_valence: f32,
```

In `ImageUnderstanding::neutral()` (`composition.rs:93–118`), add to the constructor literal:

```rust
            affect_arousal: -1.0,
            affect_valence: -1.0,
```

In `understand_image_pure` (`pure_analysis.rs:738` literal), add the SAME two fields = `-1.0`:

```rust
        affect_arousal: -1.0,
        affect_valence: -1.0,
```

The pixel producer NEVER computes a real affect value — it writes only the sentinel. This is
the only change to `pure_analysis.rs` in Slice A.

### 3.5 `character_tempo_bpm` + the tempo-block replacement (composition.rs)

```rust
/// Clamp the raw brightness→BPM into the selected character's window from
/// `affect.character_tempo.<character>`. An ABSENT window (character name not in the map)
/// means "no clamp" — return `raw_bpm` unchanged (the legacy flat-path behaviour, which never
/// clamped). With the default `AffectMappings` (no-affect-block floor), the only window present
/// is `ballad:{56,96}`, so `character_tempo_bpm(raw, Ballad, default)` == the old
/// `clamp(56,96)` byte-for-byte. Pure. Replaces the hard clamp at composition.rs:727.
fn character_tempo_bpm(raw_bpm: f32, character: Character, affect: &AffectMappings) -> f32;
```

**The exact replacement for the `:725–728` tempo block.** Before (HEAD):

```rust
        let bpm = interp_tempo_bpm(&mappings.global.brightness_to_tempo_bpm, u.avg_brightness);
        // Ballad tempo window (slice 1 character==Ballad): keep the BPM musical (slow-to-mid).
        let bpm = bpm.clamp(BALLAD_BPM_MIN, BALLAD_BPM_MAX);
        let base_ms_per_step = (60_000.0 / bpm.max(1.0)).round() as u64;
```

After (Slice A) — replace the two middle lines (`:726–727`) ONLY; the `:725` raw read and the
`:728` ms-per-step line keep their shape:

```rust
        let raw_bpm = interp_tempo_bpm(&mappings.global.brightness_to_tempo_bpm, u.avg_brightness);
        // Per-character tempo window (de-caps the legacy Ballad 56..96 clamp): the chosen
        // character selects the window; brightness positions BPM within it. Absent window → no clamp.
        let bpm = character_tempo_bpm(raw_bpm, character, &self.plan_mappings.affect);
        let base_ms_per_step = (60_000.0 / bpm.max(1.0)).round() as u64;
```

(Note: `character` is already in scope — it was selected at `:701`. `self.plan_mappings.affect`
is the new field from §3.7. The variable was named `bpm` twice via shadowing in the original;
the replacement renames the first to `raw_bpm` for clarity — purely local, no external reader.)

**DELETE** `composition.rs:850–852` (the doc comment + both consts):

```rust
/// Ballad tempo window (slice 1 character == Ballad): keep brightness→BPM musical.
const BALLAD_BPM_MIN: f32 = 56.0;
const BALLAD_BPM_MAX: f32 = 96.0;
```

Grep-verify before deleting: `grep -n "BALLAD_BPM_MIN\|BALLAD_BPM_MAX" src/composition.rs`
must show only the definition (`:851–852`) and the `:727` clamp being replaced. No other reader.

### 3.6 Computing & writing the composite BEFORE the ladder (composition.rs `plan`)

The character ladder at `:701` reads `u: &ImageUnderstanding` (shared borrow — see the `plan`
signature `composition.rs:697`: `pub fn plan(&self, u: &ImageUnderstanding, …)`). The two new
`Knob` arms read `u.affect_arousal` / `u.affect_valence`, so those fields must hold the REAL
composite by the time `.select(u)` runs at `:701`. Since `u` is `&` (not `&mut`), the
Implementer makes a **local owned copy at the top of `plan`**, fills its affect fields, and
uses that copy for every ladder `.select(...)` and every `u.<field>` read in `plan`:

```rust
    pub fn plan(&self, u: &ImageUnderstanding, mappings: &MappingTable) -> CompositionPlan {
        // S22: compute the affect composite once and seat it on a local working copy so the
        // character/tempo ladders read the real arousal/valence (the input `u` is borrowed `&`,
        // and the pixel producer left the affect fields at the -1.0 sentinel).
        let affect = affect_composite(u, &self.plan_mappings.affect);
        let mut u = u.clone();
        u.affect_arousal = affect.arousal;
        u.affect_valence = affect.valence;
        let u = &u; // shadow back to a shared borrow for the rest of plan (minimal blast radius)
        // … existing body unchanged from here: form/character/meter selection at :699–704 …
```

This is the **S20 `figuration_resolved` planner-fills-a-resolved-field precedent**, applied to
the affect sentinels. `ImageUnderstanding` derives `Clone` (`composition.rs:38`) so the copy is
free of trait work. No other call site changes — the rest of `plan` already reads `u` by shared
borrow. (If the Implementer prefers, the same effect is achievable by binding
`let u = { let mut u = u.clone(); u.affect_arousal = …; u.affect_valence = …; u };` — either
shape is acceptable; the binding name `u` must remain so the ~200 downstream `u.` reads are
untouched.)

### 3.7 `PlanMappings` + the `From` impl (composition.rs)

Append to `struct PlanMappings` (after `figuration_catalogue`, `composition.rs:492`):

```rust
    /// NEW S22 — the affect weights + per-character tempo windows (§3.1). `#[serde(default)]`
    /// so an OLD mappings.json (no `affect` key) parses → `AffectMappings::default()`, which
    /// ships the legacy `ballad:{56,96}` window → the compose-path tempo is bit-identical.
    #[serde(default)]
    pub affect: AffectMappings,
```

Append to the `From<CompositionMappings> for PlanMappings` impl (`composition.rs:500–510`),
inside the `PlanMappings { … }` literal:

```rust
            affect: c.affect,
```

### 3.8 The mapping_loader mirror (mapping_loader.rs) — LOAD-BEARING

`CompositionMappings` (`mapping_loader.rs:110–138`) is the struct that actually deserializes
`assets/mappings.json`. Per the S20 two-mirror discipline, **every new `PlanMappings` field
MUST also be added here**, or the `affect` block is silently dropped at load (no panic, falls
to default) and the de-cap never fires. Append to `struct CompositionMappings` (after
`figuration_catalogue`, `mapping_loader.rs:137`):

```rust
    /// S22 — the affect weights + per-character tempo windows. `#[serde(default)]` back-compat
    /// floor: absent → `AffectMappings::default()` (legacy Ballad window). Carried onto
    /// `PlanMappings` by the `From<CompositionMappings>` impl in composition.rs.
    #[serde(default)]
    pub affect: crate::composition::AffectMappings,
```

`AffectMappings`, `CharacterTempo`, and `Affect` are DEFINED in `composition.rs` (the
structural authority) and re-used here via `crate::composition::AffectMappings` — one
definition, not two, exactly as `SelectTable`/`OrchestrationProfile` already are. The
`#[serde(skip)]`-style resolved fields (`affect_arousal`/`affect_valence` on
`ImageUnderstanding`) are NOT in `CompositionMappings` — they are planner-filled, never
deserialized.

The witness that the mirror is wired is the test `affect_absent_block_keeps_ballad_window`
(§5/§6 test (a)) — if the `From` line or the `CompositionMappings` field is forgotten, that
test fails because the de-cap silently no-ops AND the legacy window is dropped.

---

## 4. The `assets/mappings.json` rows (data-author owns; copy-pastable)

These three edits land inside the existing `"composition"` block (the de-cap is in the
top-level `"global"` block). Content-only — NO schema change. Backward-compatible: an old
file omitting these still parses (`#[serde(default)]`). The per-character window NUMBERS and
the ladder thresholds are the **Music Theory Specialist's to confirm** via a file-disjoint
input doc; the data-author transcribes the confirmed values into these rows.

### 4(a) — DE-CAP `brightness_to_tempo_bpm` (in the `"global"` block, `mappings.json:16–20`)

Replace the existing three anchors:

```jsonc
"brightness_to_tempo_bpm": {
  "0-30": 72,
  "31-70": 108,
  "71-100": 150
}
```

Top anchor raised from 120→150 so brightness is no longer a hard 120 ceiling once arousal is
the primary tempo driver; the hard musical range is now owned by the per-character window, not
this table.

### 4(b) — the `affect` block (NEW key inside the `"composition"` block)

Add as a sibling key of `"character"` inside `"composition"`. AUTHORITATIVE reconciled
affect-design weights + the six core-character windows:

```jsonc
"affect": {
  "arousal_weights": {
    "avg_saturation": 0.45,
    "colorfulness":   0.25,
    "edge_activity":  0.20,
    "complexity":     0.10
  },
  "valence_weights": {
    "avg_brightness":  0.70,
    "avg_saturation":  0.20,
    "fg_bg_contrast":  0.10
  },
  "character_tempo": {
    "ballad":   { "bpm_min": 56,  "bpm_max": 96  },
    "scherzo":  { "bpm_min": 120, "bpm_max": 168 },
    "march":    { "bpm_min": 96,  "bpm_max": 132 },
    "lament":   { "bpm_min": 44,  "bpm_max": 66  },
    "hymn":     { "bpm_min": 60,  "bpm_max": 92  },
    "nocturne": { "bpm_min": 50,  "bpm_max": 80  }
  }
}
```

The `fg_bg_contrast` valence weight is a RAW-field weight; the `0.5 + 0.5*x` fluency transform
is applied inside `affect_composite` (§3.2), NOT here. `ballad:{56,96}` is the legacy window
preserved exactly (matches `AffectMappings::default()` and the old `clamp(56,96)`).

### 4(c) — the FILLED `character` ladder (replace `mappings.json:134`)

Replace `"character": { "default": "ballad", "rules": [] }` with the six-rule ladder. Every
`pick` is an EXISTING `Character` enum variant (verified §2: `scherzo`, `march`, `lament`,
`hymn`, `nocturne`, `ballad` all exist; **`Scherzo` is the energetic corner — NOT a new
variant**). First-match-wins; `ballad` is the safe catch-all default:

```jsonc
"character": {
  "default": "ballad",
  "rules": [
    { "when": [ {"knob":"arousal","op":"ge","lo":0.60,"hi":0.0}, {"knob":"valence","op":"ge","lo":0.55,"hi":0.0} ], "pick": "scherzo" },
    { "when": [ {"knob":"arousal","op":"ge","lo":0.60,"hi":0.0}, {"knob":"valence","op":"lt","lo":0.45,"hi":0.0} ], "pick": "march" },
    { "when": [ {"knob":"arousal","op":"le","lo":0.30,"hi":0.0}, {"knob":"valence","op":"lt","lo":0.35,"hi":0.0} ], "pick": "lament" },
    { "when": [ {"knob":"arousal","op":"le","lo":0.30,"hi":0.0}, {"knob":"valence","op":"ge","lo":0.50,"hi":0.0} ], "pick": "hymn" },
    { "when": [ {"knob":"arousal","op":"le","lo":0.35,"hi":0.0}, {"knob":"valence","op":"in_range","lo":0.35,"hi":0.50} ], "pick": "nocturne" }
  ]
}
```

The `hi` field is required by the `Predicate` schema (`composition.rs:405–406`, `#[serde(default)]`
so `0.0` is the no-op upper bound for non-`InRange` ops); the `nocturne` rule uses `InRange`
with a real `hi: 0.50`. These thresholds are the principled STARTING calibration — the owner's
ear is the gate (unified design Risk 4). No `march`/`lament`/`hymn`/`nocturne` tempo-window or
character-realization beyond tempo is wired in Slice A (§7).

---

## 5. The byte-freeze argument (tight)

The entire Slice A change lives on the **compose path** (`CompositionPlanner::plan`) plus an
**additive surface** (`Affect`/`AffectMappings`/`CharacterTempo` types, two `Knob` variants,
two `ImageUnderstanding` sentinel fields, the `affect` `PlanMappings` field) that the
`engine_equivalence` net **never constructs**: the net hand-builds `Section`/`StepContext`
literals and passes an explicit `MS_PER_STEP = 200` into every `decide_instrument_action`
call, so the planner's tempo derivation — and the cap, and the character ladder — are off the
net's path entirely (the net never calls `compose_from_image`/`plan`). No realizer or engine
code is touched: `chord_engine.rs` and `engine.rs` are not in Slice A's file set, so no note
emission changes. The one back-compat subtlety is honored by construction:
`AffectMappings::default()` ships the single legacy `ballad:{56,96}` window, so a mapping with
no `affect` block makes `character_tempo_bpm(raw, Ballad, default)` reproduce the old
`clamp(56,96)` byte-for-byte. **Therefore goldens 240/114/84/36/79 cannot move.** The
mechanical witness: after the slice,

```
git diff HEAD -- src/engine.rs src/chord_engine.rs tests/engine_equivalence.rs
```

**MUST be empty** (none of those three files is touched), and `cargo test` keeps
`engine_equivalence` 9/9 green.

---

## 6. Property-test list for the Test Engineer (new `tests/affect_s22.rs`, default features)

New integration file `tests/affect_s22.rs`, imports the crate (`use audiohax::composition::*;`
as the surface is `pub`). Each test is a measurable property, not a brittle golden. Concrete
names + assertion shapes:

**(a) `affect_absent_block_keeps_ballad_window`** — back-compat / mirror witness. Build a
`PlanMappings`/`MappingTable` whose `composition` block has NO `affect` key (or load a fixture
JSON without it) → it parses to `AffectMappings::default()`. For a bright `ImageUnderstanding`
(e.g. `avg_brightness = 100.0`) whose raw brightness→BPM exceeds 96, assert the compose-path
`base_ms_per_step` (or the `bpm` `character_tempo_bpm` returns for `Character::Ballad`) equals
the OLD `clamp(56,96)` result: `assert_eq!(character_tempo_bpm(raw, Character::Ballad, &AffectMappings::default()), raw.clamp(56.0, 96.0));`
Witnesses the loader mirror (§3.8) AND the default-window floor (§3.1).

**(b) `affect_composite_monotone`** — composite direction. Holding all other fields fixed,
arousal strictly increases as `avg_saturation` increases; valence strictly increases as
`avg_brightness` increases. Shape:
`assert!(affect_composite(&u_hi_sat, &w).arousal > affect_composite(&u_lo_sat, &w).arousal);`
plus the brightness/valence pair. Also pin one exact vector: for an
`ImageUnderstanding` with `avg_saturation=80, colorfulness=0.90, edge_activity=1.0,
complexity=0.75, avg_brightness=62, fg_bg_contrast=0.15` and the §4(b) weights, assert
`arousal ≈ 0.86` and `valence ≈ 0.65` within `1e-2` (the `example.jpg` prediction, unified
design §4): `assert!((a.arousal - 0.86).abs() < 0.02);`

**(c) `bright_energetic_picks_scherzo`** — character ladder. For the high-sat/colorfulness/edge
vector in (b) run through the planner (compute composite, run the §4(c) ladder), the selected
character == `Character::Scherzo` AND the resolved BPM is in the Scherzo window (`> 96` and
within `120..=168`, ~150). Shape:
`assert_eq!(selected_character, Character::Scherzo);`
`assert!(bpm > 96.0 && bpm >= 120.0 && bpm <= 168.0);`

**(d) `calm_dark_picks_slow_minor_character`** — the opposite corner. A calm/dark vector
(`s≈0.15, c≈0.20, e≈0.25, x≈0.30, b≈0.20, fg_bg_contrast≈0.40` → arousal≈0.20, valence≈0.24)
selects `Character::Lament` (or `Nocturne` for a mid-valence calm vector) AND the BPM goes
BELOW the old 56 floor (into the Lament window, `<= 66`, reaching ~44–66). Shape:
`assert_eq!(selected_character, Character::Lament);`
`assert!(bpm < 56.0 && bpm >= 44.0 && bpm <= 66.0);`
A neutral vector (`ImageUnderstanding::neutral()`-like mid values firing no rule) → `Ballad`.

> **NOTE on the spawn-prompt's test (d) "valence→major on bright, minor on dark".** Mode
> selection is **NOT in Slice A's scope.** The locked design pins Slice A as **tempo +
> character-selection only**; the valence→major/minor mode lean rides the `key_scheme`/mode
> path and is explicitly deferred to a later slice (engine-reframe §1.3 "key… untouched in
> Slice A", §2.5 "a `Valence`-driven major/minor lean could LATER ride the existing
> `key_scheme` ladder"; unified design §3 Slice A touches `chord_engine.rs`/`engine.rs`
> NOT at all). `home_mode` continues to derive from `dominant_hue` at `composition.rs:721`,
> unchanged. Test (d) as worded therefore belongs to **Slice B/C**, not Slice A — this spec
> replaces it with `calm_dark_picks_slow_minor_character` above, which validates the
> character-corner the design DOES wire in Slice A (Lament is the minor-leaning character; its
> mode realization arrives with its preset bundle in Slice C). The Test Engineer should NOT
> add a mode-assertion test in Slice A; doing so would assert behaviour the slice does not
> ship.

**(e) `scherzo_tempo_exceeds_ballad_cap`** — the de-cap, isolated at the fn level. For an
identical bright raw BPM, `character_tempo_bpm(raw, Character::Scherzo, &affect)` exceeds the
old 96 ceiling, and `character_tempo_bpm(raw, Character::Lament, &affect)` falls below the old
56 floor, where `affect` is built from the §4(b) windows. Shape:
`assert!(character_tempo_bpm(160.0, Character::Scherzo, &affect) > 96.0);`
`assert!(character_tempo_bpm(50.0, Character::Lament, &affect) < 56.0);`

**(f) `byte_freeze_witness`** — the freeze. (i) `engine_equivalence` is unmoved: assert the
in-tree goldens are still `240/114/84/36/79` (or simply that `cargo test engine_equivalence`
passes 9/9 — the CI invocation). (ii) A shell/`std::process::Command` witness that
`git diff HEAD -- src/engine.rs src/chord_engine.rs tests/engine_equivalence.rs` produces
empty stdout. Shape: `let out = Command::new("git").args(["diff","HEAD","--","src/engine.rs","src/chord_engine.rs","tests/engine_equivalence.rs"]).output()…; assert!(out.stdout.is_empty());`
If a process-spawning test is undesirable in the suite, the data-author/lead runs the
`git diff` check manually at Quality-Gate time and the test reduces to the
`engine_equivalence` 9/9 assertion.

---

## 7. Register-separation NOTE (Slice B inherits it)

The register-separation invariant (S21 Decision 8 / unified-design Risk 1 —
`mean_pitch(Bass) < bed/fill < mean_pitch(Melody)`, no figure-ground inversion, melody retains
range at maximum stacked lift) is a **SLICE B concern, not Slice A**. Slice A changes
**tempo + character-selection only** — no register nudge, no realizer touch — so it cannot
break the invariant. Slice B (the saliency→role prominence work, which adds the
melody-register lift) inherits this invariant and MUST enforce it via the
`no_inversion_invariant` sweep test specified in the locked design. Recorded here so Slice B
does not lose the inheritance.

---

## 8. Cadence (the build order for this slice)

Rust Architect (this spec, pins the rows + signatures) → **Rust Implementer** (composition.rs /
mapping_loader.rs / pure_analysis.rs sentinels) ∥ **Data-author** (mappings.json §4 rows,
file-disjoint) — the Music Theory Specialist supplies the tempo-window numbers + confirms the
ladder thresholds as a file-disjoint input doc, no source edit → **Test Engineer**
(`tests/affect_s22.rs`, the six tests above) → **Quality Gate LAST** (`engine_equivalence`
9/9 green + the §5 `git diff HEAD` empty on the three locked files + mappings.json
backward-compat + value-range check: all BPM musical, weights in 0..1, every `pick` an
existing `Character` variant).

---

## LOCK LIST — OFF-LIMITS for Slice A

The following are **not** to be created, modified, or have their public surface changed in
Slice A:

- `src/chord_engine.rs` — NOT touched.
- `src/engine.rs` — NOT touched.
- `src/midi_output.rs`, `src/synth_sink.rs`, `src/cli.rs`, `src/tui.rs`, `main.rs`,
  `src/modem.rs`, `src/bin/*` — NOT touched.
- `realize_step` PUBLIC signature — FROZEN.
- `engine_equivalence` goldens **240 / 114 / 84 / 36 / 79** — UNMOVED.
- `tests/engine_equivalence.rs` — NOT edited.
- The `Character` enum — NO new variant (`Scherzo` already exists; the energetic corner uses it).
- Mode / `key_scheme` / `home_mode` derivation — UNTOUCHED in Slice A (deferred per §7/§6 note).

*Spec-only. No source, test, or asset modified by this document. Signatures are binding
shapes; bodies are the Implementer's. Role titles (Rust Architect, Rust Implementer, Music
Theory Specialist, Test Engineer, Quality Gate, the data-author) are the only agent labels
used; no orchestration-framework or swarm name appears.*

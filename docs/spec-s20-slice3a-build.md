# S20 — Slice 3a Build Spec: ACCOMPANIMENT FIGURATION (Alberti on the Pad layer, saliency-gated)

**Author role:** Rust Architect — BUILD SPEC (DESIGN ONLY — this document modifies no source, test, or asset; `docs/` only).
**Date:** 2026-06-15.
**Status:** BUILDABLE. This is the single, unambiguous build contract for Slice 3a. The implementer and the music-theory pass each build their half from this with no further questions.
**Supersedes for build purposes:** `docs/design-s19-figuration-synthesis.md` (the reconciled plan), `docs/design-s19-figuration-musical.md` (vocabulary + tone-index-modulo rule), `docs/design-s19-figuration-engine.md` (realize-path placement + byte-freeze). Where the three S19 docs left an illustrative type or row "non-binding," THIS doc pins it to the as-built code.
**Grounded against git HEAD `49e0821`** — every line/symbol below was re-verified in the working tree, not trusted from the S19 docs.

---

## 0. Locked scope (the 8 settled S19 steers — DO NOT re-litigate or expand)

1. **First figure = Alberti only.** Onsets at step fractions `{0:root, ¼:5th, ½:3rd, ¾:5th}` WITHIN one step — a held chord expands into a bounded burst inside the single step. No scheduler/`main.rs` change; holds under the existing ≤1.2× legato cap.
2. **Layer = Pad only** in 3a.
3. **No S18 counter** on the figured profile in 3a (register-overlap avoidance) — the figured profile's 3rd slot is `HarmonicFill`, not `CounterMelody`.
4. **Per-plan selection** in 3a (one figuration choice for the whole plan; per-section is 3b).
5. **Saliency gate ladder (first-match-wins):** `subject_energy ≥ 0.45 ∧ fg_bg_contrast ≥ 0.25` → `pad_figured`; ELSE `foreground_energy ≥ 0.35 ∧ fg_bg_contrast ≥ 0.20` → `pad_bed_counter` (the unchanged S18 rule); ELSE default `pad_bed` (the sustained block bed).
6. **Name = "figuration"** (`FigurationSpec` / `FigurationOnset` / `figuration_catalogue`). No collision with the S17 `OrchestrationProfile.texture`/`ImageUnderstanding.texture: f32` family. Grep at HEAD confirms NO `figuration`/`figured`/`alberti` identifier exists in `src/` or `assets/` today (only prose comments in `saliency_s18.rs` and `chord_engine.rs` that name "Slice-3 figuration" as a future diff).
7. **Schema = catalogue-by-handle.** `Option<String> figuration` handle on `OrchestrationProfile` → resolved against a `figuration_catalogue: Vec<FigurationSpec>` row; NOT an inline struct. Both new pieces `#[serde(default)]` so old `mappings.json` parses unchanged → byte-identical to S18. Unresolved/missing id falls back to the block bed; NO panic.
8. **Meter-dependent figures DEFERRED.** A "step" is a flat beat today.

**One music-owned build-time rule confirmed here (S19 Decisions §8):** on non-triad chords the onset `tone` index cycles **modulo the seated voice count**. Specified exactly in §5.4 so the catalogue rows and the mapper agree.

---

## 1. As-built anchors (verified at HEAD `49e0821`)

| Symbol | File:line | Relevant fact |
|---|---|---|
| `OrchestrationProfile { id, layers, density (#[serde(default="half_f32")]), pad_voices (#[serde(default)]) }` | `src/composition.rs:208–222` | The struct the new `figuration` field is added to. `#[derive(Debug, Clone, PartialEq, serde::Deserialize)]`. |
| `OrchestrationProfile::identity()` literal | `src/composition.rs:233–240` | Constructs `{ id:"identity", layers:Vec::new(), density:0.5, pad_voices:0 }`. **Must gain `figuration: None`.** |
| `is_identity()` | `src/composition.rs:244–246` | `self.pad_voices == 0 && self.layers.is_empty()` — **UNTOUCHED**. A figured profile always has `pad_voices > 0`, so it is never mistaken for identity. |
| `LayerRole { Bass, HarmonicFill, Melody, CounterMelody, Pad }` | `src/composition.rs:196–202` | **UNTOUCHED** in 3a (no new role). |
| `half_f32()` serde-default helper | `src/composition.rs:225–227` | Precedent for the new `one_u8`/`one_f32` default-fn helpers (§2.1). |
| `Knob` enum + `Knob::read` | `src/composition.rs:281–325` | `SubjectEnergy`/`ForegroundEnergy`/`BackgroundEnergy`/`SubjectSize`/`FgBgContrast` all wired. **No new knob needed** — the gate is pure JSON. |
| `Predicate`/`CmpOp`/`SelectRule`/`SelectTable` first-match-wins | `src/composition.rs:328–401` | `SelectTable::select` returns the first matching rule's `pick`, else `default`. The figuration gate ladder is expressed entirely in the EXISTING `texture` SelectTable. |
| `PlanMappings { form, character, meter, key_scheme, theme_behaviour, texture (#[serde(default)]), form_catalogue, texture_catalogue (#[serde(default)]) }` | `src/composition.rs:407–431` | **Gains `figuration_catalogue: Vec<FigurationSpec>` `#[serde(default)]`.** |
| `From<CompositionMappings> for PlanMappings` | `src/composition.rs:433–449` | **Must map the new `figuration_catalogue` field** (see §2.2 — TWO mirror structs). |
| `CompositionMappings` (loader mirror) | `src/mapping_loader.rs:110–132` | **Also gains `figuration_catalogue: Vec<...> #[serde(default)]`** — this is the struct that actually deserializes `assets/mappings.json`. |
| `Section { …, orchestration: OrchestrationProfile, steps: Vec<StepPlan> }` | `src/composition.rs:518–522` | **UNTOUCHED** (the resolved spec rides on `orchestration`, see §3). |
| `StepContext<'a> { section, step_in_section, theme, key_tempo }` | `src/composition.rs:573–582` | **UNTOUCHED.** The mapper reads the spec off `ctx.section.orchestration` — the borrow is already there. |
| planner texture resolution | `src/composition.rs:707–711` | `let texture_id = self.plan_mappings.texture.select(u); let orchestration = lookup_orchestration(&self.plan_mappings.texture_catalogue, &texture_id).cloned().unwrap_or_else(OrchestrationProfile::identity);` — **the figuration handle resolution attaches HERE** (§3). |
| `orchestration.clone()` onto every section | `src/composition.rs:754` | Per-plan: one profile cloned onto each `Section`. |
| `lookup_orchestration` | `src/composition.rs:818–823` | The find-by-id helper to mirror for figuration resolution. |
| `realize_step(step, inst_idx, num_instruments, features, ms_per_step, ctx)` | `src/chord_engine.rs:956–963` | **PUBLIC signature FROZEN.** Already reads `pad_voices = ctx.section.orchestration.pad_voices` at `:968`. |
| `realize_rhythm(note, velocity, role, features, ms_per_step, is_cadence, is_phrase_start, step, pad_voices, ctx)` | `src/chord_engine.rs:1259–1277` | Private free fn; already receives `ctx` + `pad_voices`. **No new param** — the mapper reads `ctx.section.orchestration.figuration`. |
| the cadence early-return | `src/chord_engine.rs:1370–1372` | `if is_cadence { return vec![sustained(0, step_ms, LEGATO_FRAC)]; }` — fires BEFORE the role match, so a cadence step is NEVER figured (correct: the arrival rings, it does not arpeggiate). The figured arm lives inside the `Pad` match arm, reached only on non-cadence steps. |
| the `Pad` match arm (the block bed) | `src/chord_engine.rs:1419–1478` | Seats `notes[1..]` (root-skipped) up to `pad_voices`, de-dups via `seat_pc_in_register(tone%12, FILL_REGISTER_FLOOR)` (`:1452`), emits each at `offset_ms:0`, `hold_ms = step_ms × PAD_OVERLAP_FRAC`. The figured-bed sub-branch goes INSIDE this arm. |
| `seat_pc_in_register(pc, floor)` | `src/chord_engine.rs:1113–1120` | Seats a pitch class at/above a floor, clamped 24..=108. REUSED unchanged. |
| `NoteEvent { note, velocity, hold_ms, offset_ms }` | `src/chord_engine.rs:849–859` | `#[derive(Debug, Clone, Copy, PartialEq, Eq)]`. Maps 1:1 onto `main.rs`'s `(note, vel, hold_ms, offset_ms)`. |
| `FILL_REGISTER_FLOOR=55`, `MELODY_REGISTER_FLOOR=67`, `BASS_REGISTER_FLOOR=36` | `src/chord_engine.rs:1039–1041` | The fill band is `[55, 67)`. |
| `PAD_OVERLAP_FRAC = ARTIC_WINDOW_HI = 1.10` | `src/chord_engine.rs:1253` / `:1218` | The bed's per-event hold factor. The absolute hold ceiling in the `sustained` helper is `.min(1.20)` (`:1350`). See §5.6 for the exact reconciliation of "≤1.2× cap." |
| `instrument_role` returns only `Bass`/`HarmonicFill`/`Melody` | `src/chord_engine.rs:874–...` | Under identity, `assign_role` delegates here → **`Pad` is never returned** → the figured arm is structurally unreachable on the equivalence net. |
| `engine_equivalence` goldens | `tests/engine_equivalence.rs:124–135,257–279` | `MS_PER_STEP=200`, `G_BASS_NOTE=36`, `G_MELODY_NOTE=79`, cadence vel `114`/`84`, cadence hold `240`. **MUST STAY UNMOVED.** |
| the S18 counter ceiling tests | `tests/saliency_s18.rs:732`, `src/chord_engine.rs:~4797` | `test_counter_at_most_one_event_*` — the ≤1-event counter ceiling. **UNTOUCHED** (the counter arm is not modified). |

---

## 2. THE DATA TYPES — `composition.rs` (implementer owns)

### 2.1 `FigurationSpec` and `FigurationOnset` (NEW serde structs)

Add to `src/composition.rs` next to `OrchestrationProfile` (after `:247`, the `impl` block). Final, buildable signatures (these REPLACE the "illustrative" S19 versions):

```rust
/// One named accompaniment-figuration pattern — pure STRUCTURE, no note content. Animates a
/// held chord into a bounded rhythmic burst within ONE step. Lives as a row in
/// `figuration_catalogue`; an [`OrchestrationProfile`] references it BY ID. Adding a pattern is a
/// JSON row, NOT a Rust edit (the `FormSpec`/`OrchestrationProfile` content-as-data discipline). NEW S20.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct FigurationSpec {
    /// Stable id, e.g. "alberti" / "block" (block == the no-op sustained bed: empty `onsets`).
    pub id: String,
    /// Per-step onset template, in TIME ORDER (ascending `at`). 2..=4 entries (the bounded
    /// burst). Empty == a block bed (no-op). serde rejects a malformed `FigurationOnset` entry.
    #[serde(default)]
    pub onsets: Vec<FigurationOnset>,
    /// How many DISTINCT inner chord tones the figure draws from (Alberti = 3). The mapper
    /// clamps this to the seated inner-tone count at realize time. Default 1.
    #[serde(default = "one_u8")]
    pub voices: u8,
}

/// One onset of a figure: WHEN it sounds (fraction of step_ms), WHICH seated inner-voice index
/// it sounds (cycled modulo the seated voice count, §5.4), and how long it holds. NEW S20.
#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize)]
pub struct FigurationOnset {
    /// Onset time as a fraction of step_ms, 0.0..1.0 (0.0 == downbeat, 0.25 == the S18 off-beat).
    pub at: f32,
    /// Seated inner-voice index this onset sounds; cycled modulo the seated voice count (§5.4).
    pub tone: u8,
    /// Hold as a fraction of the GAP to the next onset (0.0..1.0), the in-step articulation.
    /// Default 1.0 (legato: fill the whole gap up to the per-onset cap, §5.6).
    #[serde(default = "one_f32")]
    pub hold_frac: f32,
}

/// serde default for `FigurationSpec::voices`.
fn one_u8() -> u8 { 1 }
/// serde default for `FigurationOnset::hold_frac`.
fn one_f32() -> f32 { 1.0 }
```

**Why `voices` AND a per-onset `tone` index both exist:** `voices` is the figure's declared distinct-tone budget (used to author the catalogue and to assert "the figure is a 3-voice Alberti"); `tone` is the per-onset selector INTO the seated tones. The mapper (§5.4) seats `min(voices, available_inner_tones)` tones, then each onset's `tone % seated_count` picks one. They are consistent by construction: the Alberti row declares `voices:3` and uses tone indices `{0,2,1,2}` which all fall in `0..3`.

> **No `register_floor` field.** The S19 engine doc proposed an optional `register_floor` on `FigurationSpec`. It is DROPPED from 3a: the figured bed seats in `FILL_REGISTER_FLOOR` (55) identically to the block bed, hard-coded in the mapper, exactly as the block Pad does. A per-figure register band is a 3b voicing concern (the §2 register-split in the synthesis doc). Keeping it out of the 3a type avoids an unused, untested field.

### 2.2 The new fields on the carriers (BOTH mirror structs)

`OrchestrationProfile` (`src/composition.rs:208`) gains ONE additive field:

```rust
pub struct OrchestrationProfile {
    pub id: String,
    pub layers: Vec<LayerRole>,
    #[serde(default = "half_f32")] pub density: f32,
    #[serde(default)] pub pad_voices: u8,
    /// NEW S20 — id of a `figuration_catalogue` row this profile's Pad animates with, or None
    /// for the S17 block bed. `#[serde(default)]` (== None) so EVERY old profile parses unchanged
    /// → byte-identical to S18. The planner resolves this handle at §3; the realizer reads the
    /// RESOLVED spec, never the raw handle.
    #[serde(default)]
    pub figuration: Option<String>,
}
```

**`OrchestrationProfile::identity()` (`:233`) gains `figuration: None`** in the literal.

**There are TWO mirror structs that must each gain `figuration_catalogue`** — this is the load-bearing build detail the S19 docs under-specified:

1. `PlanMappings` (`src/composition.rs:407`) — the planner type:
```rust
    /// NEW S20 — the figuration vocabulary, parallel to `form_catalogue`/`texture_catalogue`.
    /// `#[serde(default)]` (empty Vec) so an OLD mappings.json with no `figuration_catalogue`
    /// key parses; an unresolved profile handle then falls back to the block bed.
    #[serde(default)]
    pub figuration_catalogue: Vec<FigurationSpec>,
```

2. `CompositionMappings` (`src/mapping_loader.rs:110`) — the struct that ACTUALLY deserializes `assets/mappings.json`:
```rust
    /// S20 — the figuration vocabulary. `#[serde(default)]` back-compat floor.
    #[serde(default)]
    pub figuration_catalogue: Vec<crate::composition::FigurationSpec>,
```

3. The `From<CompositionMappings> for PlanMappings` impl (`src/composition.rs:437`) **must map the new field**:
```rust
            figuration_catalogue: c.figuration_catalogue,
```

Forgetting any of the three breaks the build or silently drops the catalogue at load. `mapping_loader.rs:110` is NOT in the locked-off set, so editing it is permitted (it is the implementer's loader-mirror lane, file-disjoint from the music-theory `chord_engine.rs` lane).

### 2.3 Serde back-compat proof (byte-identical to S18)

- `OrchestrationProfile.figuration: Option<String>` `#[serde(default)]` → `None` on every existing profile (`identity`/`pad_bed`/`pad_bed_counter`) → the realizer takes the block-bed path → byte-identical.
- `figuration_catalogue` `#[serde(default)]` on BOTH mirrors → empty Vec when the JSON key is absent.
- Resolution is total (§3): `Some(id)` with no matching catalogue row → block bed; `None` → block bed. **Never panics.**
- `is_identity()` unchanged; a figured profile has `pad_voices > 0` so identity detection is unaffected; the byte-freeze anchor holds.

---

## 3. THE RESOLUTION SEAM — handle → resolved spec, threaded via the section's orchestration clone

The figuration handle is resolved **once per plan, at the existing texture-resolution point** (`src/composition.rs:707–711`), and the resolved spec is threaded to `chord_engine` **without touching any function signature and without touching `chord_engine.rs` from the implementer's side** — it rides the already-borrowed `ctx.section.orchestration`.

### 3.1 The chosen mechanism (pin one of the two S19 options)

The S19 synthesis offered two: (a) add a resolved-spec field to `Section`, or (b) store the resolved spec on the cloned `OrchestrationProfile`. **PIN OPTION (b) with a thin variant: the resolver REWRITES the handle in place is NOT allowed (the handle is `Option<String>`, not `Option<FigurationSpec>`). Instead, resolve at realize time off the catalogue carried no further than the planner is NOT possible (the catalogue is not on `ctx`).**

Therefore the buildable mechanism is: **the realizer (the music-theory `Pad` arm) resolves the spec, but it needs the catalogue.** Two are file-disjoint-clean; PIN this one:

> **The planner resolves the handle to a concrete `Option<FigurationSpec>` and stores the RESOLVED spec on the section's `OrchestrationProfile` clone, in a SECOND additive field that is NOT serde-loaded.**

Concretely, add a second field to `OrchestrationProfile`:

```rust
    /// NEW S20 — the RESOLVED figuration spec for this section, filled by the planner from
    /// `figuration` against `figuration_catalogue`. NOT loaded from JSON (`#[serde(skip)]` →
    /// always `None` at deserialize); the planner sets it. The realizer reads THIS, never the
    /// raw `figuration` handle. `#[serde(skip)]` keeps mappings.json byte-shape unchanged and
    /// keeps `PartialEq`/`Clone` total.
    #[serde(skip)]
    pub figuration_resolved: Option<FigurationSpec>,
```

`identity()` sets `figuration_resolved: None`.

At `composition.rs:707–711`, AFTER the `orchestration` is resolved and BEFORE the section loop, the planner fills it:

```rust
    let texture_id = self.plan_mappings.texture.select(u);
    let mut orchestration =
        lookup_orchestration(&self.plan_mappings.texture_catalogue, &texture_id)
            .cloned()
            .unwrap_or_else(OrchestrationProfile::identity);
    // S20: resolve the figuration handle ONCE per plan against the catalogue. An unresolved /
    // None handle leaves `figuration_resolved == None` → the realizer takes the block bed.
    orchestration.figuration_resolved = orchestration
        .figuration
        .as_deref()
        .and_then(|id| lookup_figuration(&self.plan_mappings.figuration_catalogue, id))
        .cloned();
```

with a new finder mirroring `lookup_orchestration`:

```rust
fn lookup_figuration<'a>(catalogue: &'a [FigurationSpec], id: &str) -> Option<&'a FigurationSpec> {
    catalogue.iter().find(|f| f.id == id)
}
```

`orchestration.clone()` at `:754` already deep-clones `figuration_resolved` onto each section (it derives `Clone`). The realizer reads `ctx.section.orchestration.figuration_resolved`.

### 3.2 Why this is the clean seam between the two lanes

- The **implementer** owns the whole resolution: the two serde types, the two carrier fields (`figuration` handle + `figuration_resolved`), the catalogue on BOTH mirrors, the `From` map, the `lookup_figuration` finder, and the 4-line resolve block. All in `composition.rs` + `mapping_loader.rs`. Plus the sole-writer `mappings.json` rows (§4).
- The **music-theory pass** owns ONLY the `chord_engine.rs` figured-bed sub-branch + the mapper (§5). It reads `ctx.section.orchestration.figuration_resolved: Option<FigurationSpec>` — a field guaranteed to exist by the implementer's lane. It NEVER touches `composition.rs`, the catalogue, or the handle.
- The seam is exactly one type (`Option<FigurationSpec>` reachable off the already-borrowed `ctx`). No function signature changes; `realize_step` FROZEN; `realize_rhythm` already carries `ctx`.

> **Build-order note:** the implementer's `FigurationSpec` type must compile before the music-theory pass can name it. The two lanes are file-disjoint but the TYPE is a shared dependency. Resolve by: the implementer lands `composition.rs` types FIRST (or both work against this spec's exact signature and the type is added in the implementer's commit, which the music-theory commit imports via `use crate::composition::FigurationSpec;`). They do not edit the same file.

---

## 4. `mappings.json` DELTAS — implementer is SOLE writer (music-theory hands the values)

All three deltas go in the `composition` block of `assets/mappings.json` (currently ending at `:149`). Music-theory supplies the onset/threshold NUMBERS; the implementer writes the JSON.

### 4.1 The `figuration_catalogue` array (NEW key)

```jsonc
"figuration_catalogue": [
  { "id": "block",   "onsets": [] },
  { "id": "alberti", "voices": 3,
    "onsets": [
      { "at": 0.0,  "tone": 0 },
      { "at": 0.25, "tone": 2 },
      { "at": 0.5,  "tone": 1 },
      { "at": 0.75, "tone": 2 }
    ] }
]
```

The Alberti onset/tone mapping, stated against the locked steer `{0:root, ¼:5th, ½:3rd, ¾:5th}`:
- The bed seats the **inner tones, root-skipped** (`notes[1..]`, the as-built Pad voicing at `chord_engine.rs:1444`). So seated index 0 == the 3rd, index 1 == the 5th, index 2 == the 7th (on a triad, only indices 0,1 exist → 3rd, 5th).
- The steer's "root/5th/3rd/5th" is the CLASSICAL Alberti description over a full triad-with-bass. In THIS root-less inner bed, the figure animates the inner voicing the bed already plays. The locked cell `{0:tone0, ¼:tone2, ½:tone1, ¾:tone2}` (low-high-mid-high over the seated inner tones) is the in-bed realization the synthesis doc §3 and the musical doc §1.1 both pin. **This is the canonical row; music-theory confirms the exact index ordering against the seated-tone semantics in §5.4 before it is written.** (See §7 Ambiguity A1 — this is the one place the steer's pitch names and the as-built root-less bed must be reconciled; resolved there.)

### 4.2 The `pad_figured` OrchestrationProfile row (added to `texture_catalogue`)

```jsonc
"texture_catalogue": [
  { "id": "identity",        "layers": [], "density": 0.5, "pad_voices": 0 },
  { "id": "pad_bed",         "layers": ["Bass","Pad","HarmonicFill","Melody"], "density": 0.55, "pad_voices": 3 },
  { "id": "pad_bed_counter", "layers": ["Bass","Pad","CounterMelody","Melody"], "density": 0.6,  "pad_voices": 3 },
  { "id": "pad_figured",     "layers": ["Bass","Pad","HarmonicFill","Melody"], "density": 0.62, "pad_voices": 3,
    "figuration": "alberti" }
]
```

The `pad_figured` row is the SAME bed as `pad_bed` (`HarmonicFill` in the 3rd slot — **NO `CounterMelody`**, locked steer 3) but carries `"figuration": "alberti"`. `pad_voices: 3` so it is non-identity and the Pad arm fires.

### 4.3 The gate ladder (extends the existing `texture` SelectTable — NO new axis)

```jsonc
"texture": {
  "default": "pad_bed",
  "rules": [
    { "when": [ {"knob":"subject_energy",   "op":"ge","lo":0.45,"hi":0.0},
                {"knob":"fg_bg_contrast",   "op":"ge","lo":0.25,"hi":0.0} ],
      "pick": "pad_figured" },
    { "when": [ {"knob":"foreground_energy","op":"ge","lo":0.35,"hi":0.0},
                {"knob":"fg_bg_contrast",   "op":"ge","lo":0.20,"hi":0.0} ],
      "pick": "pad_bed_counter" }
  ]
}
```

`SelectTable::select` is first-match-wins (`composition.rs:393–400`), so the order — `pad_figured` rule FIRST, then the unchanged S18 `pad_bed_counter` rule, then the `default: "pad_bed"` — IS the locked three-way ladder. The figuration gate is expressed entirely in the existing `texture` schema; **no new `figuration` SelectTable, no new PlanMappings axis.** The S18 rule's predicate values are byte-unchanged; only the new figured rule is prepended.

---

## 5. THE onset → NoteEvent MAPPER CONTRACT — `chord_engine.rs` (music-theory pass owns)

### 5.1 Where it lives

A NEW private helper inside `src/chord_engine.rs`, called from a NEW sub-branch INSIDE the existing `Pad` match arm (`:1419`). The block-bed path (`:1441–1477`) stays **byte-untouched**: the sub-branch only redirects when a non-empty resolved figuration is present.

```rust
OrchestralRole::Pad => {
    let notes = &step.chord.notes;
    if notes.is_empty() || pad_voices == 0 {
        vec![sustained(0, step_ms, PAD_OVERLAP_FRAC)]   // unchanged defensive path
    } else {
        // seat the inner tones EXACTLY as the block bed does (:1444–1463) — REUSE verbatim.
        let seated: Vec<u8> = /* the existing :1444–1463 seating, unchanged */;
        match ctx.section.orchestration.figuration_resolved.as_ref() {
            Some(spec) if !spec.onsets.is_empty() => {
                figured_bed(spec, &seated, velocity, step_ms)
            }
            _ => {
                // the EXISTING block emission (:1464–1476), byte-identical
                seated.into_iter().map(|n| NoteEvent {
                    note: n, velocity,
                    hold_ms: ((step_ms as f32) * PAD_OVERLAP_FRAC).round().max(1.0) as u64,
                    offset_ms: 0,
                }).collect()
            }
        }
    }
}
```

> The block path must remain BIT-FOR-BIT what it is today on the `None`/empty branch — the `block_bed_unchanged_when_figuration_none` test (§6) is the witness. The cleanest implementation factors the seating once and shares it between both branches (as sketched); a refactor that changes the emitted bytes of the `None` path is a defect.

### 5.2 The helper signature

```rust
/// Expand ONE held chord into the figure's bounded multi-onset burst within a single step.
/// `spec`   — the resolved figuration (non-empty `onsets`, 2..=4 entries).
/// `seated` — the inner tones the block bed already seated (root-skipped, fill register, de-duped).
/// `velocity` — the Pad's supporting velocity for this step.
/// `step_ms` — the step duration.
/// Returns a bounded `Vec<NoteEvent>` of `spec.onsets.len()` events (2..=4), all WITHIN the step.
fn figured_bed(
    spec: &crate::composition::FigurationSpec,
    seated: &[u8],
    velocity: u8,
    step_ms: u64,
) -> Vec<NoteEvent>
```

### 5.3 Inputs / output (the exact contract)

- **Inputs:** the resolved `&FigurationSpec` (read off `ctx.section.orchestration.figuration_resolved`), the already-seated inner tones `&[u8]` (the SAME `seat_pc_in_register(tone%12, FILL_REGISTER_FLOOR)`/de-dup tones the block bed builds at `:1444–1463`), `velocity`, `step_ms`.
- **Output:** `Vec<NoteEvent>` with **exactly `spec.onsets.len()` events, bounded 2..=4** (the catalogue guarantees 2..=4; the helper MAY defensively clamp/truncate to 4 but must NOT emit an unbounded count). Each event is a chord-tone-only note in the fill band `[55, 67)`.
- **Empty/`None` is handled by the CALLER** (§5.1): `figured_bed` is only called with a non-empty `onsets`, so it never needs to reproduce the block path.

### 5.4 Per-onset realization (the precise rules)

For each `onset` in `spec.onsets`, in order:

1. **Offset:** `offset_ms = (onset.at.clamp(0.0, 1.0) * step_ms as f32).round() as u64`.
   - For Alberti: `0, step_ms/4, step_ms/2, 3·step_ms/4`.
2. **Tone selection (the non-triad modulo rule — LOCKED):**
   ```
   let n = seated.len();                 // 1..=pad_voices, post-clamp/de-dup
   let idx = (onset.tone as usize) % n;  // cycle modulo the SEATED voice count
   let note = seated[idx];
   ```
   - This is the music-owned rule confirmed from S19 §8: **the onset `tone` index cycles modulo the seated voice count.** On a triad the bed seats 2 inner tones (3rd, 5th) → a 4-onset Alberti `{0,2,1,2}` reads `0,2%2=0,1,2%2=0` → tones `[s0, s0, s1, s0]`. On a 7th chord the bed seats 3 (3rd, 5th, 7th) → `{0,2,1,2}` reads `s0, s2, s1, s2` (the full low-high-mid-high Alberti cell). The catalogue Alberti row is authored for the 3-voice case; the modulo guarantees it degrades sanely on a triad and never indexes out of bounds.
   - Guard: if `seated` is empty the caller's `if notes.is_empty() || pad_voices == 0` branch already returned, so `n >= 1` here; the `% n` is always safe.
3. **Hold (in-step legato, capped):**
   - Gap to next onset: `gap_ms = next_offset_ms - offset_ms` for all but the last onset; for the LAST onset, `gap_ms = step_ms - offset_ms` (it fills to the step end).
   - Raw hold: `hold_ms = (gap_ms as f32 * onset.hold_frac.clamp(0.0, 1.0)).round() as u64`.
   - **Per-onset cap:** clamp so `offset_ms + hold_ms ≤ (step_ms as f32 * PAD_OVERLAP_FRAC).round() as u64` (i.e. ≤ `step_ms × 1.10`, the established Pad ceiling — see §5.6). Concretely: `hold_ms = hold_ms.min(cap.saturating_sub(offset_ms)).max(1)` where `cap = ((step_ms as f32) * PAD_OVERLAP_FRAC).round() as u64`.
   - With `hold_frac` default 1.0, each note holds to the next onset (continuous figure); only the last onset's hold may reach the cap.
4. **Emit:** `NoteEvent { note, velocity, hold_ms, offset_ms }`.

**Velocity:** use the `velocity` the arm already passes (the Pad's supporting level). The S19 musical doc §5.5 calls for the figure to stay UNDER the melody; the existing Pad velocity already is the supporting level (no `-3` adjustment is in the as-built Pad arm — the block bed uses `velocity` as-is). **PIN: use `velocity` unchanged**, identical to the block bed, so the only audible delta 3a introduces is rhythm, not dynamics. (A `-3` supporting trim is a 3b dynamics tweak, deferred — see §7 Ambiguity A2.)

### 5.5 `None` behavior (back-compat)

When `figuration_resolved` is `None` OR `onsets` is empty, the `Pad` arm takes the EXISTING block emission (`:1464–1476`) — `pad_voices` simultaneous inner tones at `offset_ms: 0`, `hold_ms = step_ms × PAD_OVERLAP_FRAC`. Byte-identical to S18. The `figured_bed` helper is never entered. This is the chord-tone-only line in the pad band when figured; the sustained block when not.

### 5.6 The "≤1.2× legato cap" reconciliation (exact)

The locked steer says "≤1.2× legato cap." The as-built code has TWO related numbers:
- `PAD_OVERLAP_FRAC = 1.10` — the block bed's per-event hold factor (`:1473`).
- `.min(1.20)` — the absolute ceiling inside the `sustained` helper (`:1350`).

**The figured bed uses `PAD_OVERLAP_FRAC` (1.10) as its per-onset hold cap**, identical to the block bed, so the figure never over-runs the step more than the block bed already does (≤10%). 1.10 ≤ 1.20 satisfies the "≤1.2×" steer with margin. The test `figuration_onsets_are_in_step` (§6) asserts `offset_ms + hold_ms ≤ step_ms × 1.2` (the looser absolute ceiling, so the test passes whether the implementer uses 1.10 or up to 1.20). **The build target is 1.10 (matches the block bed); the test allows up to 1.20.**

---

## 6. THE TEST PLAN — the new figuration net (test engineer owns)

A NEW test file `tests/figuration_s20.rs` (run under default features, like `tests/saliency_s18.rs` — the integration harness builds the feature-gated bin, so `--no-default-features` cannot RUN it; see `saliency_s18.rs:750`). Drive everything through the PUBLIC `realize_step`, hand-building an RNG-free `Section`/`StepContext` exactly as `saliency_s18.rs:399–423` does. **Fixtures must set the two new `OrchestrationProfile` fields** (`figuration` + `figuration_resolved`) in the struct literal — the serde defaults do NOT apply to hand-built literals. A `pad_figured()` fixture builds `OrchestrationProfile { id:"pad_figured", layers:[Bass,Pad,HarmonicFill,Melody], density:0.62, pad_voices:3, figuration:Some("alberti".into()), figuration_resolved:Some(<the Alberti FigurationSpec>) }`. (The existing `saliency_s18.rs` fixtures that build `OrchestrationProfile` literals — e.g. `pad_bed_counter()` at `:380` — will ALSO need the two new fields added; that is an expected, mechanical fixture update in the test lane, NOT a behavior change.)

| Test | Asserts | Fixture |
|---|---|---|
| `figuration_emits_bounded_burst` | the Pad inst under `pad_figured` emits exactly `onsets.len()` events, `2..=4`; never an unbounded count. | `pad_figured`, a held non-cadence interior step. |
| `figuration_onsets_are_in_step` | for every event, `offset_ms + hold_ms ≤ (step_ms as f32 * 1.2).round() as u64`; the last onset never overhangs the step. | same. |
| `figuration_tones_are_chord_tones_in_band` | every event's `note` is a chord tone (its pc ∈ `step.chord.notes` pcs) seated in `[FILL_REGISTER_FLOOR, MELODY_REGISTER_FLOOR)` = `[55,67)`. | a known triad (C major), assert each note pc ∈ {E,G} (the root-skipped inner tones). |
| `figured_bed_off_beat` | ≥1 event has `offset_ms > 0` (the off-beat witness — distinguishes the figure from the offset-0 block bed; the Alberti `¼/½/¾` onsets guarantee it). | same. |
| `block_bed_unchanged_when_figuration_none` | a `pad_bed`-style profile (`figuration_resolved: None`) emits the S17 block: all events `offset_ms == 0`, count == `pad_voices`, `hold_ms == round(step_ms*1.10)`. The BACK-COMPAT witness. | `pad_bed` fixture. |
| `texture_selects_pad_figured_on_salient_subject` | `texture.select(u)` returns `"pad_figured"` on `ImageUnderstanding { subject_energy:0.5, fg_bg_contrast:0.3, .. }`, and `"pad_bed"` (NOT `pad_figured`) on a calm one (`subject_energy:0.1, fg_bg_contrast:0.05`); and `"pad_bed_counter"` on the S18 case (`foreground_energy:0.4, fg_bg_contrast:0.25, subject_energy:0.0`) to prove the ladder order is unbroken. | build the SelectTable + catalogue from the §4 rows (or load `mappings.json`). |
| `unresolved_figuration_id_falls_to_block` | a profile with `figuration: Some("nope")` resolving to `figuration_resolved: None` (because the catalogue has no "nope") takes the block bed — no panic. | a `pad_figured`-shaped profile with `figuration_resolved: None`. |
| `tone_index_cycles_modulo_seated` (RECOMMENDED, pins §5.4) | on a TRIAD (2 seated inner tones) the Alberti `{0,2,1,2}` produces notes from indices `{0,0,1,0}` (each `% 2`); on a 7th chord (3 seated) `{0,2,1,2}`. No out-of-bounds. | a triad fixture and a 7th-chord fixture. |

**Freeze witnesses (handed to the test engineer):**
- `git diff HEAD -- tests/engine_equivalence.rs` is EMPTY (the equivalence net is byte-untouched).
- `cargo test` (default features): `engine_equivalence` green (all goldens in-file confirmed), the S18 counter-ceiling tests (`test_counter_at_most_one_event_*` in `tests/saliency_s18.rs` and `src/chord_engine.rs`) still green.
- The goldens `MS_PER_STEP=200`, `G_BASS_NOTE=36`, `G_MELODY_NOTE=79`, cadence vel `114`/`84`, cadence hold `240` UNMOVED in `tests/engine_equivalence.rs:124–135,257–279`.

---

## 7. BYTE-FREEZE ARGUMENT + WITNESSES

### 7.1 Why `engine_equivalence` stays byte-green

The figured arm is reachable ONLY through a `pad_figured` profile on the compose path. `engine_equivalence.rs` builds its plan by hand and realizes under `OrchestrationProfile::identity()` (carried by `single_section_default`). Under identity:
1. `assign_role` sees `prof.is_identity() == true` → delegates to `instrument_role`, which returns ONLY `Bass`/`HarmonicFill`/`Melody` (verified `chord_engine.rs:874`). **`Pad` is never returned → the figured sub-branch is structurally unreachable**, independent of the green test.
2. `pad_voices == 0` under identity → even the block Pad path holds nothing; `figuration_resolved == None` under identity → the block branch is taken regardless.
3. `FigurationSpec`/`FigurationOnset` are serde/`#[serde(skip)]`-only; the equivalence net constructs none.
4. `single_section_default`'s identity path keeps role assignment to Bass/Melody/HarmonicFill, so the figured arm is unreachable on the equivalence net — the locked invariant holds.
5. `realize_step` PUBLIC signature unchanged; `realize_rhythm` already carries `ctx` (no new param). The cadence early-return (`:1370`) fires before the role match, so even a figured profile never figures a cadence step.

**No golden re-derivation. The goldens 240/114/84/36/79 do not move.**

### 7.2 The freeze witnesses to hand to the test engineer

1. `engine_equivalence` 9/9 green; goldens confirmed in-file at `tests/engine_equivalence.rs`.
2. `git diff HEAD -- tests/engine_equivalence.rs` EMPTY.
3. **sha256 of every LOCKED-OFF file == `git show HEAD:<path>`** for: `src/modem.rs`, `src/bin/modem_encode.rs`, `src/bin/modem_decode.rs`, `src/bin/channel_sim.rs`, `src/bin/make_packetized.rs`, `src/bin/make_tiled_payload.rs`, `src/bin/unpack_tiled_payload.rs`, `src/bin/audiohax-tui.rs`, `src/synth_sink.rs`, `src/midi_output.rs`, `src/cli.rs`, `src/tui.rs`, `src/main.rs`, `src/engine.rs`. All MATCH `49e0821` (the change touches none of them).
   ```sh
   export PATH="$HOME/.cargo/bin:$PATH"
   for f in src/modem.rs src/bin/modem_encode.rs src/bin/modem_decode.rs \
            src/bin/channel_sim.rs src/bin/make_packetized.rs src/bin/make_tiled_payload.rs \
            src/bin/unpack_tiled_payload.rs src/bin/audiohax-tui.rs src/synth_sink.rs \
            src/midi_output.rs src/cli.rs src/tui.rs src/main.rs src/engine.rs; do
     a=$(git show 49e0821:"$f" | sha256sum | cut -d' ' -f1)
     b=$(sha256sum "$f" | cut -d' ' -f1)
     [ "$a" = "$b" ] && echo "OK   $f" || echo "DRIFT $f"
   done
   ```
4. Re-derive `assign_role`/`instrument_role` to confirm `Pad` is unreachable under identity (the S18 method).

> **`src/engine.rs` is in the locked-off set** (the engine realize-driver). The figured-bed change lives entirely in the `chord_engine.rs` realizer body + `composition.rs`/`mapping_loader.rs` data + `assets/mappings.json` — `engine.rs` is untouched and its sha256 is a freeze witness.

---

## 8. FILE-OWNERSHIP SPLIT (disjoint, parallel-safe)

| Lane | Owns (writes) | Reads (does not write) |
|---|---|---|
| **Implementer** | `src/composition.rs` (`FigurationSpec`/`FigurationOnset` types + `one_u8`/`one_f32`; `figuration` handle + `figuration_resolved` `#[serde(skip)]` fields on `OrchestrationProfile`; `identity()` literal; `figuration_catalogue` on `PlanMappings`; the `From` map; `lookup_figuration`; the 4-line resolve block at `:707`). `src/mapping_loader.rs` (`figuration_catalogue` on `CompositionMappings`). `assets/mappings.json` (the §4 catalogue + `pad_figured` profile + the gate rule — SOLE writer). | `src/chord_engine.rs` (to understand the seam; does NOT edit it). |
| **Music-theory pass** | `src/chord_engine.rs` (the figured-bed sub-branch INSIDE the `Pad` arm + the `figured_bed` mapper helper; the block path stays byte-untouched). | `composition.rs` (`use crate::composition::FigurationSpec;` + reads `ctx.section.orchestration.figuration_resolved`). Hands the onset/tone/threshold VALUES to the implementer for §4. |
| **Test engineer** | `tests/figuration_s20.rs` (NEW). Mechanical fixture updates in `tests/saliency_s18.rs` IF its `OrchestrationProfile` literals need the two new fields (additive, no behavior change). | `tests/engine_equivalence.rs` — **NEVER edited** (the `git diff EMPTY` witness). |

The implementer and music-theory lanes are **file-disjoint** (`composition.rs`+`mapping_loader.rs`+`mappings.json` vs `chord_engine.rs`). The shared seam is the single type `Option<FigurationSpec>` on the borrowed `ctx`. No function signature changes.

---

## 9. AMBIGUITIES RESOLVED (flagged; none silently diverge from the locked steers)

- **A1 — the Alberti pitch names vs the root-less inner bed.** The locked steer 1 names the cell `{0:root, ¼:5th, ½:3rd, ¾:5th}`. The as-built Pad bed is **root-less** (`notes[1..]`, root deliberately skipped, `chord_engine.rs:1428–1429`). A literal "root" onset is impossible in the seated tones. **Resolution:** the figure animates the SEATED INNER tones; the catalogue Alberti cell is `{0:tone0, ¼:tone2, ½:tone1, ¾:tone2}` (low-high-mid-high over the seated inner voicing), which is exactly the in-bed Alberti shape the S19 synthesis §3 and musical §1.1 both already pin. This is NOT a divergence from the steer — it is the steer's classical pitch-name description realized in the root-less bed the project already ships. The music-theory pass confirms the index ordering in §5.4 before the row is written. (Flagged because the steer's literal pitch names and the as-built voicing are reconciled here, not contradicted.)
- **A2 — supporting velocity (`-3`?).** The prompt mentions "the `-3` supporting velocity precedent or as the music design specifies." The as-built Pad block bed uses `velocity` UNCHANGED (no `-3` is in the Pad arm). **Resolution:** 3a uses `velocity` unchanged (identical to the block bed) so the only audible delta is rhythm. A velocity trim is deferred to a dynamics slice. Not a divergence — the steer offered the trim as optional ("or as the music design specifies").
- **A3 — the resolution-threading mechanism.** The S19 synthesis left two options open (a `Section` field vs storing the resolved spec on the profile clone) and its "recommended" phrasing was ambiguous about WHERE the catalogue is reachable. **Resolution:** pinned to a `#[serde(skip)] figuration_resolved: Option<FigurationSpec>` second field on `OrchestrationProfile`, filled by the planner at the existing `:707` resolution point (§3.1). This keeps the realizer reading off `ctx.section.orchestration` with NO new param, keeps the catalogue confined to the planner, and keeps `mappings.json` byte-shape unchanged (`#[serde(skip)]` never deserializes). Resolves the S19 "either way" hand-wave into one buildable mechanism.
- **A4 — the `register_floor` field on `FigurationSpec`.** The S19 engine doc proposed it; the synthesis doc dropped it. **Resolution:** dropped from 3a (the bed seats in `FILL_REGISTER_FLOOR` hard-coded, identical to the block bed). A per-figure register band is a 3b voicing concern. Avoids an unused/untested field.
- **A5 — the TWO mirror structs.** The S19 docs spoke of "the `PlanMappings` field" as if singular. **Resolution (load-bearing, not a divergence):** `figuration_catalogue` must be added to BOTH `PlanMappings` (`composition.rs:407`) AND `CompositionMappings` (`mapping_loader.rs:110`), with the `From` impl mapping it. The latter is what actually deserializes `mappings.json`; omitting it silently drops the catalogue at load. Pinned in §2.2.

**Nothing contradicts the 8 locked steers.** Every steer is honored exactly: Alberti-only (§4.1), Pad-only (§5), no counter (§4.2), per-plan (§3.1 resolves once at `:707`), the saliency ladder (§4.3), the "figuration" name (§2), catalogue-by-handle + `#[serde(default)]` back-compat (§2), meter-deferred (out of scope), tone-index-modulo (§5.4).

---

*Design-only build spec. No source, test, or asset modified by this document. The Rust signatures and `mappings.json` rows here are BINDING for Slice 3a (this is the build contract, not a sketch): the implementer builds `composition.rs`/`mapping_loader.rs`/`mappings.json` against §2–§4, the music-theory pass builds the `chord_engine.rs` mapper against §5, the test engineer builds `tests/figuration_s20.rs` against §6, and §7's freeze witnesses gate the merge.*

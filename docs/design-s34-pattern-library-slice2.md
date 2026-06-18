# Design — S34 Pattern-Library Arc, Slice 2 (oom-pah/stride via `register_octaves` + walking-bass/pedal-point generators)

Date: 2026-06-18
Status: DESIGN ONLY — no implementation in this document. Specifies a FULL-SLICE build across two file-disjoint lanes.
Grounding (all read before writing): `docs/design-s30-pattern-library-slice1.md` (the locked arc + the Slice 2 decomposition), `docs/design-s33-gap4-penult-rework.md` (the additive-without-breaking-freeze pattern emulated here), `docs/spec-s20-slice3a-build.md` (the figuration generator+catalogue+mapping-rule+onset→NoteEvent pattern Slice 2 follows).
Verified against working tree HEAD `20fc407`; `src/engine.rs` sha256 confirmed `e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261` (the byte-freeze anchor) at design time. Every line/symbol below was re-read in the working tree, not trusted from the predecessor docs (Slice 1's line numbers have drifted; this doc pins to HEAD `20fc407`).

This document delivers a complete, test-gated build contract for **Slice 2**, the slice S30 §1 named as "Generator-Backed Accompaniment Idioms." It has two parts:

- **Part (A)** — oom-pah / stride accompaniment textures, which need a NEW `register_octaves` onset field (the part flagged freeze-sensitive). **This is the load-bearing deliverable: §2 scopes its freeze-safe path first and most carefully.**
- **Part (B)** — walking-bass and pedal-point generators: pure-ish additive `composition.rs`/`mappings.json` data + generator work.

---

## 0. HEADLINE VERDICT (read this first)

**`register_octaves` lands freeze-safe via PATH 1 (data carried in the onset spec, realized inside the existing `chord_engine.rs` `figured_bed` texture mapper). `src/engine.rs` stays BYTE-FROZEN; `realize_step`'s PUBLIC 7-param signature stays FROZEN; the goldens 240/114/84/36/79 and the `engine_equivalence` 9/9 byte-witness do NOT move; the counter-OFF identity (PT-0) is untouched.**

The decisive structural fact, re-verified at HEAD `20fc407`: **`realize_step` does NOT live in `engine.rs`.** It lives in `src/chord_engine.rs:1055–1062`. `engine.rs` is the *driver*: it builds the `StepContext` and *calls* `chord_engine::realize_step(...)` at `engine.rs:740` with the frozen 7-arg shape. The freeze on `engine.rs` therefore means **the call seam and the StepContext construction cannot change** — no new arg at the `engine.rs:740` call, no new field threaded through the `StepContext`/`Section` that `engine.rs` constructs. `register_octaves` requires NEITHER: it is a field on `FigurationOnset` (a `composition.rs` serde type), carried into the realizer on the ALREADY-RESOLVED `FigurationSpec` that the planner stamped onto `Section.orchestration.figuration_resolved` (the S20 mechanism, `composition.rs:405–406`), and read off `ctx.section.orchestration.figuration_resolved` inside `figured_bed` (`chord_engine.rs:1894`). The realizer touch is one arithmetic line inside `figured_bed` — a `chord_engine.rs` texture-mapper edit, explicitly the music-theory lane, carrying its own re-witness plan (§6). **engine.rs is never opened.**

This is the same discipline S33 GAP-4 used (additive behavior off already-threaded context, no new param, no engine.rs touch) and the same discipline S20 figuration used (data on the resolved spec, read off the borrowed `ctx`). Part (A) is *more* freeze-safe than S33's case, because S33 needed a cross-step lookahead (`next_chord`) whereas `register_octaves` needs only a field that travels INSIDE the spec already on `ctx`.

---

## 1. CURRENT-STATE ANALYSIS (verified at HEAD `20fc407`)

### 1.1 The texture-realization path (where part A lands)

| Symbol | File:line | Current fact |
|---|---|---|
| `realize_step(step, inst_idx, num_instruments, features, ms_per_step, ctx)` | `chord_engine.rs:1055–1062` | **PUBLIC 7-param signature — FROZEN.** Reads `ctx.section.orchestration.pad_voices` at `:1084`. This is the realizer; it is NOT in engine.rs. |
| `engine.rs` call site | `engine.rs:740` | `chord_engine::realize_step(step, inst_idx, num_instruments, &features, ms_per_step, ctx)` — the frozen 7-arg call. engine.rs is byte-frozen; this call shape cannot change. |
| `OrchestralRole::Pad` arm of `realize_rhythm` | `chord_engine.rs:1651–1728` | The bed: seats inner tones root-skipped (`notes[1..]`, `:1676`), de-dups via `seat_pc_in_register(tone % 12, FILL_REGISTER_FLOOR)` (`:1684`), then at `:1709` matches `ctx.section.orchestration.figuration_resolved`: `Some(spec) if !spec.onsets.is_empty()` → `figured_bed(spec, &seated, velocity, step_ms)`; else the byte-identical block emission (`:1713–1725`). |
| `figured_bed(spec, seated, velocity, step_ms) -> Vec<NoteEvent>` | `chord_engine.rs:1894–1946` | The onset→NoteEvent mapper. Per onset: `offset_ms = round(at·step_ms)` (`:1920`); `idx = onset.tone % n` (`:1922`); **`note = seated[idx]` (`:1923`)** — THIS is the single line a register shift modifies; `hold_ms` capped to `step_ms·PAD_OVERLAP_FRAC` (`:1937`). Returns exactly `onsets.len()` events (≤4). |
| `seat_pc_in_register(pc, floor)` | `chord_engine.rs:1282` | Seats a pitch class at/above a floor, clamped `24..=108`. Reused unchanged. |
| `FILL_REGISTER_FLOOR = 55`, `BASS_REGISTER_FLOOR = 36`, `MELODY_REGISTER_FLOOR = 67` | `chord_engine.rs:1189–1190` (+ `MELODY` nearby) | The fill band is `[55, 67)`; the bass band floor is 36. |
| `PAD_OVERLAP_FRAC = ARTIC_WINDOW_HI = 1.10` | `chord_engine.rs:1447` | The bed's per-event hold cap. |
| `NoteEvent { note, velocity, hold_ms, offset_ms }` | `chord_engine.rs` (struct) | `#[derive(Debug, Clone, Copy, PartialEq, Eq)]`. Maps 1:1 onto `(note, vel, hold_ms, offset_ms)`. |
| `assign_role` / identity gate | `chord_engine.rs:915–934` + `:1063` | Under the identity profile no instrument is a `Pad` → the figured arm is structurally unreachable on the equivalence net → byte-freeze intact. |

### 1.2 The figuration data types + resolution seam (the S20 carrier part A rides)

| Symbol | File:line | Current fact |
|---|---|---|
| `FigurationSpec { id, onsets: Vec<FigurationOnset> (#[serde(default)]), voices: u8 (#[serde(default = "one_u8")]) }` | `composition.rs:531–543` | `#[derive(Debug, Clone, PartialEq, serde::Deserialize)]`. The row type. |
| `FigurationOnset { at: f32, tone: u8, hold_frac: f32 (#[serde(default = "one_f32")]) }` | `composition.rs:547–557` | `#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize)]`. **The struct part (A) adds `register_octaves` to.** |
| `one_u8()` / `one_f32()` | `composition.rs:560 / 565` | serde-default helpers; precedent for a new `zero_i8`-style default (or `#[serde(default)]` on an `i8`, which is 0 — no helper needed). |
| `OrchestrationProfile.figuration: Option<String>` | `composition.rs:394–399` | `#[serde(default)]` handle. |
| `OrchestrationProfile.figuration_resolved: Option<FigurationSpec>` | `composition.rs:400–406` | `#[serde(skip)]` — filled by the planner, read by the realizer. **The carrier part (A) rides; no new field needed here.** |
| planner resolve block | `composition.rs:1174–1179` | `orchestration.figuration_resolved = orchestration.figuration.as_deref().and_then(|id| lookup_figuration(&self.plan_mappings.figuration_catalogue, id)).cloned();` — resolves the handle ONCE per plan. A `register_octaves` on each onset travels through this `.cloned()` automatically (it is a field of `FigurationOnset`, which is a field of the cloned `FigurationSpec`). |
| `lookup_figuration` | `composition.rs:1339` | `catalogue.iter().find(|f| f.id == id)`. Unchanged. |
| `figuration_catalogue` on `PlanMappings` / `CompositionMappings` | `composition.rs:774` / `mapping_loader.rs:~110` | Both `#[serde(default)]`; the `From` map at `composition.rs:810`. A new field on `FigurationOnset` flows through both mirrors with zero edit (it is interior to `FigurationSpec`). |
| `Section.orchestration` | `composition.rs:894` | Each section carries an `OrchestrationProfile` clone; `figuration_resolved` (with its onsets' `register_octaves`) deep-clones onto every section automatically. |
| `StepContext.section` | `composition.rs:973` | `&'a Section` — the borrow the realizer reads `figuration_resolved` off. Built by `engine.rs` (`with_prev` / `single_section_default`); **`register_octaves` adds NOTHING to this borrow's shape.** |
| back-compat pin | `composition.rs:2272` (`s30_figuration_backward_compat_old_rows_unchanged`) | Currently asserts `FigurationOnset` has no `register_octaves`; its doc-comment (`:2268`) must be updated when the field is added (see §6/§7). The functional assertions (alberti `voices==3`, `onsets.len()==4`, block present) stay green because the new field is `#[serde(default)]`. |

### 1.3 The generator seam (where part B lands)

| Symbol | File:line | Current fact |
|---|---|---|
| `OrchestralRole::Bass` arm of `realize_rhythm` | `chord_engine.rs:1607–1626` | Today: one sustained root for the whole step (`sustained(0, step_ms, base_frac)`, `:1624`), or a 2-onset pre-cadence pickup (`:1611–1619`). **The comment at `:1609` already names "a walking pickup" — walking-bass is the planned deepening of exactly this arm.** The pitch is the chord ROOT (set upstream in `role_pitch`/`base_note`, `:1218`), seated `BASS_REGISTER_FLOOR`. |
| `role_pitch` Bass arm | `chord_engine.rs:1218` | `seat_pc_in_register(root_pc, floor)` with a brightness-derived dark-drop. The bass sounds the chord root; walking-bass overrides this with passing tones (§3). |
| next-chord lookahead reachability | `chord_engine.rs:1740` (counter arm precedent) | `ctx.section.steps.get(si + 1)` is the proven in-realizer next-step read (S33 used `.map(|s| &s.chord)`). **Walking-bass's target (next chord root) is reachable identically — no seam change.** `si = ctx.step_in_section` (`:1738`). |
| `shape_to_ostinato` mapping | `mapping_loader.rs:96, 231` (HashMap) + `mappings.json:96–100` | `{circle:PedalPoint, triangle:AscendingOstinato, square:DescendingOstinato}` is parsed into a `HashMap<String,String>` but **NEVER realized into note generation** (grep confirms no `Ostinato`/`PedalPoint` code path in `src/chord_engine.rs` or `src/engine.rs`). So pedal-point is genuinely NEW generator work — the S30 phrase "extends the existing `PedalPoint` ostinato" refers to this UNWIRED data string, which has no behavior today. |
| `texture_catalogue` / `texture` SelectTable | `mappings.json:229–294` | The four S30 Slice-1 figured profiles (`pad_broken_up`/`pad_broken_wave`/`pad_arp_waltz`/`pad_block_comp`) plus `pad_figured`/`pad_bed`/`pad_bed_counter`, gated by the `texture` SelectTable rules. Part (A)/(B) profiles are appended here. |
| `progression_families` | `mappings.json:42–59` | The Slice-1 Area-2 rows landed; part (B) adds none (it is accompaniment/bass texture, not progression). |
| `Predicate`/`SelectTable` | `composition.rs:~328` | first-match-wins; `Knob` enum has `arousal`/`valence`/`colorfulness`/`subject_energy`/`foreground_energy`/`fg_bg_contrast`/etc. all wired. No new knob needed for part (A) or (B). |

**Slice-1 status (verified):** the four no-new-field idioms (`broken_chord_up`, `broken_chord_wave`, `arp_waltz`, `block_comp_24`) and their profiles + SelectTable rules ARE built and landed (`mappings.json:248–293`, tests at `composition.rs:2272–2369`). Slice 2 is purely additive on top of this.

---

## 2. FREEZE-SAFE VERDICT FOR `register_octaves` (THE HEADLINE) — PATH 1

**Path 1 applies in full. `register_octaves` is carried as DATA in the onset spec (`mappings.json` + `FigurationOnset`), realized inside the existing `figured_bed` texture mapper (the S20 figured-bed sub-branch). `engine.rs` and `realize_step` stay frozen. No value must reach the realizer that is not already on the resolved spec borrowed off `ctx`.**

### 2.1 The complete data path (mappings.json row → composition.rs → chord_engine.rs realize sub-branch, NO engine.rs touch)

```
assets/mappings.json
   figuration_catalogue: [ { "id":"oom_pah", "voices":3, "onsets":[
        { "at":0.0, "tone":0, "register_octaves":-1 },   // the "oom": bass note, an octave DOWN
        { "at":0.5, "tone":1, "register_octaves":0  } ] } ]   // the "pah": inner stab, in-band
        │
        │  serde deserialize (register_octaves #[serde(default)] => 0 on every existing row)
        ▼
src/composition.rs
   FigurationOnset { at, tone, hold_frac, register_octaves: i8 }   ← NEW field, #[serde(default)] (==0)
        │   (interior to FigurationSpec → flows through BOTH mirror structs
        │    PlanMappings/CompositionMappings + the From map with ZERO edit there)
        ▼
   planner resolve block (composition.rs:1174–1179): figuration_resolved = …lookup…cloned()
        │   register_octaves rides inside the cloned FigurationSpec onto Section.orchestration
        ▼
   Section.orchestration.figuration_resolved : Option<FigurationSpec>   (composition.rs:894/405)
        │   deep-cloned onto every Section by orchestration.clone()
        ▼
src/engine.rs  (BYTE-FROZEN — only BORROWS the section into StepContext; reads nothing new)
   StepContext { section: &Section, … }   ← register_octaves changes NOTHING about this borrow's shape
        │   engine.rs:740 calls chord_engine::realize_step(…, ctx)  ← frozen 7-arg call, UNCHANGED
        ▼
src/chord_engine.rs   (music-theory lane — the ONE realizer touch)
   realize_step → Pad arm (:1709) → figured_bed(spec, &seated, velocity, step_ms)
        │
        ▼   figured_bed (:1923) — the ONE arithmetic line that changes:
            // BEFORE:  let note = seated[idx];
            // AFTER:   let note = apply_register_octaves(seated[idx], onset.register_octaves);
            //          where apply_register_octaves shifts by 12·octaves, clamped to [24,108]
```

**Every input the register shift needs (`onset.register_octaves`) is INSIDE the `spec` already borrowed off `ctx` at `chord_engine.rs:1709`.** Nothing new is threaded through `engine.rs`, `StepContext`, or `Section`'s shape. This is strictly inside the S20 seam.

### 2.2 Explicit freeze statement

- **`engine.rs`: NOT TOUCHED.** It only borrows `&Section` into the `StepContext` and calls `realize_step` with the frozen 7-arg shape. The `register_octaves` field is interior to `FigurationOnset`; the borrow shape (`section: &Section`) is byte-identical. sha256 stays `e50c7db…2348261`.
- **`realize_step`: PUBLIC 7-param signature FROZEN.** No new param. `figured_bed`'s signature is also unchanged (`register_octaves` is read off the `spec: &FigurationSpec` it already receives). The new `apply_register_octaves` helper is a PRIVATE free fn.
- **Goldens 240/114/84/36/79 + `engine_equivalence` 9/9 byte-green: UNMOVED.** The figured arm is reachable only via a `Pad` instrument, which the identity profile never assigns (`assign_role` → `instrument_role` returns only Bass/HarmonicFill/Melody under identity, `chord_engine.rs:915`). Every equivalence step is downstream-untouched. `register_octaves` is `#[serde(default)]` (==0) so every EXISTING figuration row (alberti, the four Slice-1 idioms, block) realizes byte-identically — a 0 shift is `note = seated[idx]` exactly as today (PT-FREEZE/`block_bed_unchanged_when_figuration_none` and the figured-bed witnesses hold).
- **Counter-OFF identity (PT-0): UNTOUCHED.** The Pad/figured path is not the counter path; the identity profile assigns neither.

### 2.3 Why paths 2 and 3 do NOT apply (stated for the record)

- **Path 2 (ride an existing threaded field to the realize seam) is NOT NEEDED.** It would be the fallback if `register_octaves` had to reach the realizer as a *separately-threaded* value (the way S33 needed `next_chord`). It does not: the value travels INSIDE the `FigurationSpec` that the S20 seam already delivers to `figured_bed`. Path 1 subsumes it.
- **Path 3 (cannot land freeze-safe) does NOT apply.** There is no datum the register shift needs that is not already on the borrowed spec. No golden moves; the anchor does not re-baseline; `realize_step` gains no param. The "land B clean while deferring A" fallback is therefore unnecessary — but it is documented in §8 as a contingency only if the music-theory lane discovers a realization surprise during build (it should not).

### 2.4 The one honest freeze-sensitivity flag (LOUD, per the constraint)

The single freeze-sensitive element is the **one-line edit inside `figured_bed` at `chord_engine.rs:1923`** (`note = seated[idx]` → register-shifted). This is a `chord_engine.rs` texture-realization touch — acceptable per the constraint IF it carries its own re-witness plan, which §6 provides in full. It is freeze-sensitive ONLY in the sense that it edits a realizer body; it does NOT move any golden, because:
1. it is downstream of the Pad arm (identity-unreachable), and
2. it is a no-op (shift 0) on every existing row.

**The flag is raised; the mitigation is the §6 re-witness battery + the §2.2 default-zero argument.** The block-bed path (`:1713–1725`) and the counter/Bass paths are byte-untouched (the edit is confined to the single `let note = …` line inside `figured_bed`).

---

## 3. PROPOSED CHANGES — per file (complete signatures, no bodies)

### 3.1 Part (A) — oom-pah / stride

#### `src/composition.rs` (implementer lane) — the `register_octaves` onset field

```rust
/// One onset of a figure: WHEN it sounds (fraction of step_ms), WHICH seated inner-voice
/// index it sounds (cycled modulo the seated voice count), how long it holds, AND an
/// optional whole-octave register shift applied to the seated pitch. NEW S20; `register_octaves` NEW S34.
#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize)]
pub struct FigurationOnset {
    /// Onset time as a fraction of step_ms, 0.0..1.0 (0.0 == downbeat, 0.25 == the off-beat).
    pub at: f32,
    /// Seated inner-voice index this onset sounds; cycled modulo the seated voice count.
    pub tone: u8,
    /// Hold as a fraction of the GAP to the next onset (0.0..1.0), the in-step articulation.
    /// Default 1.0 (legato: fill the whole gap up to the per-onset cap).
    #[serde(default = "one_f32")]
    pub hold_frac: f32,
    /// NEW S34 — whole-octave register shift applied to the seated pitch for THIS onset:
    /// `-1` drops the tone an octave (the oom-pah "oom" / stride bass), `+1` raises it (a
    /// high stride stab), `0` == in-band (every existing row). The mapper applies
    /// `seated[idx] + 12·register_octaves`, CLAMPED to the engine's `[24, 108]` MIDI range
    /// (the same clamp `seat_pc_in_register` already uses). `#[serde(default)]` (== 0) so
    /// EVERY existing row (alberti, broken_chord_up/wave, arp_waltz, block_comp_24, block)
    /// deserializes byte-identically and realizes with NO pitch change — the byte-freeze
    /// default. Read ONLY in `chord_engine.rs::figured_bed`; never reaches engine.rs.
    #[serde(default)]
    pub register_octaves: i8,
}
```

No new serde-default helper is required: `i8`'s `Default` is `0`, and `#[serde(default)]` (no path) uses `Default::default()`. (Contrast `voices`/`hold_frac`, which need non-zero defaults and so carry `one_u8`/`one_f32`.)

`FigurationSpec`, `OrchestrationProfile`, the two mirror structs, the `From` map, `lookup_figuration`, and the planner resolve block are **UNCHANGED** — `register_octaves` is interior to `FigurationOnset` and flows through all of them via the existing `.cloned()`.

#### `src/chord_engine.rs` (music-theory lane) — the register-shift application

ONE new private helper + ONE changed line inside `figured_bed`. `figured_bed`'s signature is UNCHANGED.

```rust
/// Apply a whole-octave register shift to a seated pitch, clamped to the engine's MIDI
/// range `[24, 108]` (identical to `seat_pc_in_register`'s clamp, so a shifted tone can
/// never escape the synthesizable range). `octaves == 0` returns `pitch` unchanged (the
/// byte-freeze default for every non-register-split figure). NEW S34. Private free fn.
///
/// theory: the oom-pah/stride idiom places the "oom"/stride-bass an octave BELOW the inner
/// bed (`octaves == -1`) so the accompaniment reads as bass-vs-chord, while the "pah"/stab
/// stays in the fill band (`octaves == 0`). The shift is whole-octave ONLY — it preserves
/// pitch class, so the figure remains chord-tones-only (the §6 chord-tone witness holds).
fn apply_register_octaves(pitch: u8, octaves: i8) -> u8;
```

The changed line in `figured_bed` (`chord_engine.rs:1923`):

```rust
// BEFORE:  let note = seated[idx];
// AFTER:   let note = apply_register_octaves(seated[idx], onset.register_octaves);
```

Everything else in `figured_bed` (offset, hold cap, the modulo tone rule, the ≤4 truncation) is unchanged.

> **Register-band note (music-theory):** a `-1` shift on a tone seated at `FILL_REGISTER_FLOOR` (55) lands at 43 (G2) — between `BASS_REGISTER_FLOOR` (36) and the fill floor (55), the correct "oom" register, below the inner stabs and above the true Bass root. This is intentional and correct for oom-pah (the bass alternation), and is NOT required to stay inside `[55,67)` (that band constraint is the COUNTER voice's, not the Pad bed's — the Pad already seats freely and the figured bed inherits that freedom). The §6 chord-tone witness asserts pitch-CLASS membership, not band membership, for register-shifted onsets.

#### `assets/mappings.json` (implementer lane) — oom-pah + stride catalogue rows, profiles, SelectTable

```jsonc
"figuration_catalogue": [
  // … existing rows (block, alberti, broken_chord_up/wave, arp_waltz, block_comp_24) UNCHANGED …

  // NEW S34 — OOM-PAH (the classic waltz/polka left hand): low "oom" bass on the downbeat,
  // higher "pah" inner stab(s) on the off-beats. register_octaves:-1 drops the oom an octave.
  { "id": "oom_pah", "voices": 3,
    "onsets": [
      { "at": 0.0, "tone": 0, "register_octaves": -1, "hold_frac": 0.4 },   // oom (bass, octave down)
      { "at": 0.5, "tone": 1, "register_octaves": 0,  "hold_frac": 0.4 } ] },// pah (inner stab)

  // NEW S34 — OOM-PAH-PAH (triple-meter waltz): one low oom, two off-beat pahs.
  { "id": "oom_pah_pah", "voices": 3,
    "onsets": [
      { "at": 0.0,  "tone": 0, "register_octaves": -1, "hold_frac": 0.3 },  // oom
      { "at": 0.33, "tone": 1, "register_octaves": 0,  "hold_frac": 0.3 },  // pah
      { "at": 0.66, "tone": 2, "register_octaves": 0,  "hold_frac": 0.3 } ] },// pah

  // NEW S34 — STRIDE (the jazz/ragtime left hand): low bass on 1 & 3, mid-register chord
  // stab on 2 & 4. The bass strides an octave down; the chord stabs sit in the bed.
  { "id": "stride", "voices": 3,
    "onsets": [
      { "at": 0.0,  "tone": 0, "register_octaves": -1, "hold_frac": 0.4 },  // bass (1)
      { "at": 0.25, "tone": 1, "register_octaves": 0,  "hold_frac": 0.4 },  // stab (2)
      { "at": 0.5,  "tone": 2, "register_octaves": -1, "hold_frac": 0.4 },  // bass (3) — alt root/5th
      { "at": 0.75, "tone": 1, "register_octaves": 0,  "hold_frac": 0.4 } ] }// stab (4)
]
```

```jsonc
"texture_catalogue": [
  // … existing rows UNCHANGED …
  { "id": "pad_oom_pah",     "layers": ["Bass","Pad","HarmonicFill","Melody"], "density": 0.6,  "pad_voices": 3, "figuration": "oom_pah" },
  { "id": "pad_oom_pah_pah", "layers": ["Bass","Pad","HarmonicFill","Melody"], "density": 0.6,  "pad_voices": 3, "figuration": "oom_pah_pah" },
  { "id": "pad_stride",      "layers": ["Bass","Pad","HarmonicFill","Melody"], "density": 0.64, "pad_voices": 3, "figuration": "stride" }
]
```

```jsonc
// APPENDED to the existing "texture" SelectTable rules (first-match-wins; appended AFTER the
// Slice-1 rules so existing image→texture behavior is byte-stable unless a feature crosses a
// NEW threshold). Affect mapping per §8 OPEN DECISION: oom-pah → bright + danceable; stride →
// high-arousal + colorful. These are the implementer's proposed gates; the lead may retune.
"texture": {
  "default": "pad_bed",
  "rules": [
    // … existing Slice-1 rules UNCHANGED (pad_figured, pad_bed_counter, pad_block_comp,
    //    pad_broken_up, pad_arp_waltz, pad_broken_wave) …
    { "when": [ {"knob":"valence","op":"ge","lo":0.60,"hi":0.0},
                {"knob":"arousal","op":"in_range","lo":0.40,"hi":0.65} ],
      "pick": "pad_oom_pah" },
    { "when": [ {"knob":"arousal","op":"ge","lo":0.75,"hi":0.0},
                {"knob":"colorfulness","op":"ge","lo":0.55,"hi":0.0} ],
      "pick": "pad_stride" }
  ]
}
```

(`oom_pah_pah` is a triple-meter variant; until a triple meter exists it reads as a 3-onset arpeggiation in 4/4 — harmless, authored now so the row is ready. Its profile `pad_oom_pah_pah` need not be gated by a `texture` rule yet — leave it un-selected, or gate it identically to `pad_oom_pah` behind a triple-meter knob when meter grows. **Recommend: author the row + profile, leave it un-selected, per the "inert until a ladder selects it" S30 R-C discipline.**)

### 3.2 Part (B) — walking-bass + pedal-point

Both are **generator** idioms (logic, not a static onset table), per S30 contested-decision #3. They are NOT figurations of the Pad bed — they reshape the **Bass** line. The cleanest, freeze-safe placement mirrors the figured-bed seam: a NEW **bass-pattern** catalogue + a resolved-spec field on the section, read inside the Bass arm of `realize_rhythm`/`role_pitch`.

#### Design choice for part (B): a `bass_pattern` resolved-spec seam parallel to figuration

To keep part (B) freeze-safe AND file-disjoint from part (A), it rides the SAME mechanism class as S20 figuration: a data-named pattern resolved once per plan onto the section's orchestration, read off `ctx` in the realizer. Two sub-options were considered:

- **(B-i) — a tiny `BassPatternSpec` enum-tagged catalogue + `bass_pattern_resolved` on `OrchestrationProfile`** (parallel to `figuration`/`figuration_resolved`). The realizer's Bass arm reads `ctx.section.orchestration.bass_pattern_resolved` and, when present, calls a generator that uses `ctx.section.steps.get(si+1)` (walking) or holds a pinned pitch (pedal). **PREFERRED** — it is the proven S20 pattern, file-disjoint (data in composition.rs, generator in chord_engine.rs), and needs NO engine.rs/`realize_step`/`StepContext` change (the next-chord lookahead is the same in-realizer read S33 used).
- **(B-ii) — overload the `figuration` seam** by letting a `FigurationSpec` target the Bass. REJECTED: figuration is Pad-scoped by construction (seats inner tones root-skipped); the bass line is a different voicing domain. Overloading muddies the §1 module boundary.

PIN **(B-i).** Complete signatures:

```rust
// src/composition.rs (implementer lane) — NEW serde types + carrier field + resolution.

/// A named BASS-LINE generator pattern — pure STRUCTURE/POLICY, no pitch content. Unlike a
/// `FigurationSpec` (a static onset table over the Pad bed), a bass pattern is a small POLICY
/// the realizer's Bass arm interprets against the live chord stream (walking needs the NEXT
/// chord root; pedal holds one pitch under changing harmony). Lives in `bass_pattern_catalogue`;
/// an [`OrchestrationProfile`] references it BY ID. Adding a pattern is a JSON row, NOT a Rust
/// edit (the `FigurationSpec`/`FormSpec` discipline). NEW S34.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct BassPatternSpec {
    /// Stable id, e.g. "sustained" (the no-op default == today's one-root bass) / "walking" / "pedal".
    pub id: String,
    /// Which generator the Bass arm runs. serde rejects an unknown kind. Default `Sustained`
    /// (== the byte-stable current bass).
    #[serde(default)]
    pub kind: BassPatternKind,
    /// For `Walking`: onsets-per-step (the walking-bass note density; 2 == half-step walk,
    /// 4 == quarter-note walk). Ignored by other kinds. Default 2.
    #[serde(default = "two_u8")]
    pub density: u8,
    /// For `Pedal`: which scale degree to PIN under the changing harmony — `1` (tonic pedal,
    /// the default) or `5` (dominant pedal). Ignored by other kinds. Default 1.
    #[serde(default = "one_u8")]
    pub pedal_degree: u8,
}

/// The bass-line generator kinds. `Sustained` is the byte-stable default (today's behavior:
/// one grounded root per step). NEW S34.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BassPatternKind {
    /// One sustained chord root for the whole step (today's `OrchestralRole::Bass` arm). The
    /// realizer takes its byte-identical legacy path under this kind — the freeze default.
    #[default]
    Sustained,
    /// A target-seeking stepwise bass: fill `density` onsets between THIS chord root and the
    /// NEXT chord root with diatonic passing/neighbor tones, arriving ON the next root at the
    /// next downbeat (strong-beat = chord tone, weak-beat = passing tone).
    Walking,
    /// Hold ONE pitch (the `pedal_degree` of the section's key) under the changing harmony —
    /// the pedal point. The harmony changes above; the bass does not move.
    Pedal,
}

/// serde default for `BassPatternSpec::density` (the walking-bass note count).
fn two_u8() -> u8 { 2 }
// `one_u8` (composition.rs:560) is reused for `pedal_degree`'s default.
```

```rust
// OrchestrationProfile gains a parallel handle + resolved field (the S20 pattern, additive):
pub struct OrchestrationProfile {
    // … id, layers, density, pad_voices, figuration, figuration_resolved, prominence …
    /// NEW S34 — id of a `bass_pattern_catalogue` row this profile's Bass uses, or None for the
    /// sustained default. `#[serde(default)]` (== None) → every existing profile is byte-stable.
    #[serde(default)]
    pub bass_pattern: Option<String>,
    /// NEW S34 — the RESOLVED bass pattern, filled by the planner from `bass_pattern` against
    /// `bass_pattern_catalogue`. `#[serde(skip)]` (never deserialized); the realizer reads THIS.
    /// `None` == the sustained default == byte-stable Bass arm.
    #[serde(skip)]
    pub bass_pattern_resolved: Option<BassPatternSpec>,
}
```

`OrchestrationProfile::identity()` gains `bass_pattern: None, bass_pattern_resolved: None`.

```rust
// PlanMappings + CompositionMappings each gain (the TWO-mirror discipline, like figuration):
    /// NEW S34 — the bass-pattern vocabulary. `#[serde(default)]` empty Vec (back-compat floor).
    #[serde(default)]
    pub bass_pattern_catalogue: Vec<BassPatternSpec>,
// … and the From<CompositionMappings> map gains: bass_pattern_catalogue: c.bass_pattern_catalogue,

// A finder mirroring lookup_figuration:
fn lookup_bass_pattern<'a>(catalogue: &'a [BassPatternSpec], id: &str) -> Option<&'a BassPatternSpec>;

// The planner resolve block (next to the figuration resolve at composition.rs:1174) gains:
//   orchestration.bass_pattern_resolved = orchestration.bass_pattern.as_deref()
//       .and_then(|id| lookup_bass_pattern(&self.plan_mappings.bass_pattern_catalogue, id)).cloned();
```

```rust
// src/chord_engine.rs (music-theory lane) — the Bass generators. Both are PRIVATE; called from
// a NEW sub-branch INSIDE the existing OrchestralRole::Bass arm (realize_rhythm :1607) AND a
// pitch override in the Bass arm of role_pitch/realize_step. The sustained default path stays
// BYTE-UNCHANGED (the `bass_pattern_resolved.is_none()` branch).

/// Generate a target-seeking stepwise WALKING bass for one step: `density` onsets that connect
/// THIS chord's root to the NEXT chord's root by diatonic step (strong beat == chord tone, weak
/// beats == passing/neighbor tones), arriving so the next downbeat lands on the next root. The
/// next chord is read off `ctx.section.steps.get(si+1)` (the S33 in-realizer lookahead — NO seam
/// change). At a section's last step (`next` is None) it falls back to the sustained root (the
/// §R-B end-of-section fallback). All notes seated `BASS_REGISTER_FLOOR`, chord-tones-or-passing.
/// Returns a bounded `Vec<NoteEvent>` of `density` events within the step. NEW S34.
fn walking_bass(
    chord: &Chord,
    next_chord: Option<&Chord>,
    mode: &str,                 // section's mode/key for diatonic passing-tone derivation
    key_offset_semitones: i8,   // the section's home offset (read off ctx.section), for the tonic
    velocity: u8,
    step_ms: u64,
    density: u8,
) -> Vec<NoteEvent>;

/// Generate a PEDAL-POINT bass for one step: hold the section-key `pedal_degree` (1 == tonic,
/// 5 == dominant) as ONE sustained low pitch, IGNORING the step's chord (the harmony changes
/// above; the pedal does not). Seated `BASS_REGISTER_FLOOR`. Returns one sustained `NoteEvent`.
/// theory: the pedal is a NON-chord bass that the upper voices' harmony is heard AGAINST; it is
/// the one sanctioned standing dissonance in common practice. NEW S34.
fn pedal_bass(
    mode: &str,
    key_offset_semitones: i8,
    pedal_degree: u8,
    velocity: u8,
    step_ms: u64,
    base_frac: f32,
) -> Vec<NoteEvent>;
```

The Bass arm of `realize_rhythm` (`chord_engine.rs:1607`) gains a sub-branch BEFORE the existing body:

```rust
OrchestralRole::Bass => {
    match ctx.section.orchestration.bass_pattern_resolved.as_ref().map(|b| b.kind) {
        Some(BassPatternKind::Walking) if !is_cadence => { /* call walking_bass(…) */ }
        Some(BassPatternKind::Pedal)   if !is_cadence => { /* call pedal_bass(…) */ }
        // Sustained, None, OR any cadence step → the EXISTING body, BYTE-UNCHANGED:
        _ => { /* the current :1611–1625 pre_cadence / sustained-root logic, verbatim */ }
    }
}
```

> **Pitch seam for walking/pedal (music-theory):** the existing Bass pitch is set upstream in `role_pitch` (`base_note`, `:1218`) and the rhythm arm only sets durations. Walking/pedal need to set PITCH per onset, so the generator must emit `NoteEvent`s with their own `note` values (it cannot rely on the single upstream `base_note`). This is exactly what `figured_bed` already does for the Pad (it ignores the single `base_note` and emits its own seated pitches), so the precedent is clean: the walking/pedal generators OWN their pitches, emitted directly as `NoteEvent { note, … }`, and the Bass arm's sub-branch returns them. The non-walking/pedal path still uses the upstream `base_note` exactly as today. **No `role_pitch` edit is needed** — the generators compute their own pitches from the chord(s) + mode, identical to how `figured_bed` seats from `seated`. Confirm during build that the Bass arm has `step.chord`, `ctx`, `velocity`, `step_ms`, `base_frac`, and `si` in scope (it does: `si` is computable as `ctx.step_in_section`, the same value the counter arm reads at `:1738`).

```jsonc
// assets/mappings.json — NEW bass_pattern_catalogue + profiles + SelectTable rules.
"bass_pattern_catalogue": [
  { "id": "sustained", "kind": "sustained" },                          // the no-op default (optional to list)
  { "id": "walking",   "kind": "walking", "density": 2 },              // half-note walk
  { "id": "walking_q", "kind": "walking", "density": 4 },              // quarter-note walk
  { "id": "pedal",     "kind": "pedal",   "pedal_degree": 1 },         // tonic pedal
  { "id": "pedal_dom", "kind": "pedal",   "pedal_degree": 5 }          // dominant pedal
],
// Profiles that pair a bed/figuration with a bass pattern (a profile can carry BOTH a
// figuration AND a bass_pattern — they are independent voices):
"texture_catalogue": [
  // … existing + the part-A oom-pah/stride profiles …
  { "id": "pad_walking",  "layers": ["Bass","Pad","HarmonicFill","Melody"], "density": 0.6, "pad_voices": 3, "bass_pattern": "walking" },
  { "id": "pad_pedal",    "layers": ["Bass","Pad","HarmonicFill","Melody"], "density": 0.6, "pad_voices": 3, "bass_pattern": "pedal" }
],
// SelectTable rules APPENDED after Slice-1/part-A (first-match-wins; inert until crossed):
// walking → propulsive/march affect; pedal → static/drone (low arousal + low complexity).
//   { "when":[{"knob":"arousal","op":"ge","lo":0.55,"hi":0.0},{"knob":"complexity","op":"ge","lo":0.5,"hi":0.0}], "pick":"pad_walking" },
//   { "when":[{"knob":"arousal","op":"le","lo":0.25,"hi":0.0},{"knob":"complexity","op":"le","lo":0.25,"hi":0.0}], "pick":"pad_pedal" }
```

### 3.3 Removals

NONE. Every change is additive (new fields `#[serde(default)]`/`#[serde(skip)]`, new private fns, new catalogue rows). No existing type, fn, field, or row is removed or re-signatured.

---

## 4. INTERFACE DEFINITIONS (complete signatures, no bodies)

All Rust signatures + struct defs + doc-comments are given inline in §3 (the `FigurationOnset.register_octaves` field, `apply_register_octaves`, `BassPatternSpec`/`BassPatternKind`, `walking_bass`, `pedal_bass`, `lookup_bass_pattern`, the two-mirror `bass_pattern_catalogue` fields, the `OrchestrationProfile` carrier fields). The mappings.json schema additions, example catalogue rows, and SelectTable selections are likewise given inline in §3.1 (part A) and §3.2 (part B). No lifetimes/bounds beyond the `<'a>` on `lookup_bass_pattern` (mirroring `lookup_figuration`) and the `&str`/`&Chord` borrows on the generators. No trait bounds are introduced.

**Schema summary (the JSON contract):**
- `FigurationOnset` gains `"register_octaves": i8` (optional, default 0).
- NEW top-level `composition` key `"bass_pattern_catalogue": [ { "id", "kind"?, "density"?, "pedal_degree"? } ]`.
- `texture_catalogue` rows gain an optional `"bass_pattern": "<id>"` (independent of `"figuration"`; a profile may carry both, either, or neither).
- `texture` SelectTable gains appended rules selecting the new profiles (existing rules byte-unchanged).

---

## 5. FILE-DISJOINT IMPLEMENTATION SPLIT

Three lanes; the implementer and music-theory lanes are **file-disjoint** and run in parallel. The test lane runs after both land.

| Lane | OWNS (writes) | READS (does not write) |
|---|---|---|
| **Rust Implementer** | `src/composition.rs` — the `register_octaves` field on `FigurationOnset`; the `BassPatternSpec`/`BassPatternKind` types + `two_u8`; the `bass_pattern`/`bass_pattern_resolved` fields on `OrchestrationProfile` + the `identity()` literal; `bass_pattern_catalogue` on BOTH mirror structs (`PlanMappings` here + `CompositionMappings` in `mapping_loader.rs`) + the `From` map + `lookup_bass_pattern` + the planner resolve line. `src/mapping_loader.rs` — the `bass_pattern_catalogue` mirror field. `assets/mappings.json` — **SOLE writer** of ALL new catalogue rows (oom_pah/stride figurations, bass_pattern_catalogue), the new `texture_catalogue` profiles, and the appended `texture` SelectTable rules. | `src/chord_engine.rs` (to understand the seams; does NOT edit it). |
| **Music Theory Specialist** | `src/chord_engine.rs` — **SOLE writer.** The `apply_register_octaves` helper + the ONE changed `let note = …` line in `figured_bed` (part A). The `walking_bass` + `pedal_bass` generators + the Bass-arm sub-branch dispatch (part B). The block-bed path, the counter path, the Pad block path, and the existing Bass sustained/pre-cadence path stay BYTE-UNTOUCHED. Hands the implementer the onset/`register_octaves`/`density`/`pedal_degree`/threshold VALUES for the §3 JSON rows. | `src/composition.rs` (`use crate::composition::{FigurationSpec, BassPatternSpec, BassPatternKind};` + reads `ctx.section.orchestration.{figuration_resolved, bass_pattern_resolved}`). |
| **Test Engineer** | `tests/` — a NEW `tests/pattern_library_s34.rs` (the generator-property + register-shift + freeze witnesses). Mechanical fixture updates wherever a hand-built `OrchestrationProfile`/`FigurationOnset` literal needs the new fields (additive, no behavior change). | `tests/engine_equivalence.rs` — **NEVER edited** (the `git diff EMPTY` witness). |

**The seam between the two build lanes is exactly two resolved-spec types reachable off the borrowed `ctx`:** `Option<FigurationSpec>` (already exists; part A only adds an interior field) and `Option<BassPatternSpec>` (new, part B). No function signature crosses the seam; `realize_step` is FROZEN; `figured_bed`'s signature is unchanged.

**Do BOTH lanes touch `chord_engine.rs`?** No — only the Music Theory lane writes `chord_engine.rs`; the Implementer only reads it. They are genuinely file-disjoint. **The one shared dependency is the TYPE:** `BassPatternSpec`/`BassPatternKind` must compile (in `composition.rs`, the Implementer's file) before the Music Theory lane can `use` them. Resolve by build-ORDER, not file-sharing: **the Implementer lands the `composition.rs` types FIRST** (or both build against this spec's exact signatures and the type lands in the Implementer's commit, which the Music Theory commit imports). They never edit the same file. `register_octaves` (part A) has the same micro-dependency: the field must exist on `FigurationOnset` before `figured_bed` reads `onset.register_octaves` — same resolution (Implementer-first on `composition.rs`).

> **Recommended serialization if the lead wants a single-writer ordering for safety:** Implementer-first on `composition.rs`/`mapping_loader.rs`/`mappings.json` (lands the types + data, all `#[serde(default)]`/`#[serde(skip)]` so the build is green with the realizer still on its legacy path), THEN Music Theory on `chord_engine.rs` (wires the realizer to read the new fields), THEN Test Engineer. This is the §8 migration order. The two build lanes CAN overlap (file-disjoint) but the type-compile dependency makes Implementer-first the lower-risk sequence.

---

## 6. BYTE-FREEZE RE-WITNESS PLAN (the checks QG/Test Engineer runs to prove the freeze held)

Run from the repo root with `export PATH="$HOME/.cargo/bin:$PATH"` prefixed.

1. **engine.rs sha256 == the anchor:**
   ```sh
   sha256sum src/engine.rs
   # MUST equal e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261
   ```
2. **`engine_equivalence` 9/9 byte-green, goldens unmoved:**
   ```sh
   cargo test --test engine_equivalence
   # 9/9 pass; G_BASS_NOTE=36, G_MELODY_NOTE=79, cadence vel 114/84, cadence hold 240,
   # MS_PER_STEP=200 are in-file constants — confirm they are UNEDITED.
   git diff HEAD -- tests/engine_equivalence.rs   # EMPTY (the net is never touched)
   ```
3. **`realize_step` PUBLIC 7-param signature FROZEN:** `git diff HEAD -- src/chord_engine.rs` shows NO change to the `pub fn realize_step(step, inst_idx, num_instruments, features, ms_per_step, ctx)` signature line (`:1055–1062`); `figured_bed`'s signature is also unchanged (only its `:1923` body line changes). New fns (`apply_register_octaves`, `walking_bass`, `pedal_bass`) are PRIVATE.
4. **Counter-OFF identity (PT-0):** the existing PT-0 / counter byte-preservation test stays green — the new branches are reachable only via Pad (figured) or a `bass_pattern_resolved`-bearing Bass profile, neither of which the identity profile assigns.
5. **block/counter/Pad-block/Bass-sustained paths byte-untouched:** the existing figured-bed witnesses (`block_bed_unchanged_when_figuration_none`, `figuration_*` in `tests/figuration_s20.rs`) stay green; a NEW witness asserts every EXISTING figuration row (`register_octaves` defaulted to 0) realizes byte-identically to pre-S34 (the §2.2 default-zero guarantee).
6. **Whole suite green across feature sets:**
   ```sh
   cargo build
   cargo test                              # default features — engine_equivalence + figuration_s20 + counterpoint + new s34 net
   cargo test --lib --no-default-features  # headless lib path green
   cargo clippy -- -W clippy::all          # no new correctness warnings
   ```
7. **Locked-off file sha sweep (optional, the S20 §7.2 method):** confirm `src/engine.rs`, `src/synth_sink.rs`, `src/midi_output.rs`, `src/cli.rs`, `src/tui.rs`, `src/main.rs`, `src/modem.rs` sha256 == `git show HEAD:<path>` (the change touches none of them).

Do NOT run bare `cargo fmt` (reflows non-deliverable files — the S32/S33 standing note). Format only the touched files: `cargo fmt -- src/chord_engine.rs src/composition.rs src/mapping_loader.rs`.

---

## 7. TEST PLAN OUTLINE (the Test Engineer authors these in `tests/pattern_library_s34.rs`)

Drive everything through the PUBLIC `realize_step`, hand-building an RNG-free `Section`/`StepContext` exactly as `tests/figuration_s20.rs` / `tests/saliency_s18.rs` do. **Fixtures must set the new struct fields** (`register_octaves` on onsets, `bass_pattern`/`bass_pattern_resolved` on the profile) in literals — serde defaults do not apply to hand-built literals.

**Part (A) — register_octaves / oom-pah / stride:**
- `register_octaves_zero_is_byte_identical` — a figured profile whose onsets all have `register_octaves: 0` realizes the IDENTICAL `NoteEvent` sequence as the same row pre-field (the §2.2 default-zero freeze guard). Pin against the recorded as-built sequence for alberti/broken_chord_up.
- `oom_pah_register_split` — under `pad_oom_pah`, the `at:0.0` "oom" event's `note == apply_register_octaves(seated[0], -1)` (an octave below the inner stab) and the `at:0.5` "pah" event sits in `[55,67)`; assert the oom is strictly LOWER than the pah.
- `stride_alternates_bass_and_stab` — under `pad_stride`, events at `at∈{0.0,0.5}` are register-shifted down (the stride bass), events at `at∈{0.25,0.75}` are in-band (the stabs).
- `register_shift_stays_chord_tone` — every register-shifted onset's pitch CLASS ∈ the chord's pitch classes (whole-octave shift preserves pc → still chord-tones-only).
- `register_shift_clamped_to_midi_range` — an adversarial `register_octaves: -9` (or `+9`) clamps into `[24,108]`, never panics, never wraps.
- `oom_pah_bounded_burst` — the figure emits exactly `onsets.len()` events (≤4), all within the step + the ≤10% Pad over-run cap.

**Part (B) — walking-bass / pedal-point:**
- `walking_bass_arrives_on_next_root` — under `pad_walking` over a chord pair `(C, G)`, the walking line's LAST onset (or the next step's downbeat) lands on the NEXT chord's root pc; strong-beat onsets are chord tones, weak-beat onsets are diatonic passing tones between the two roots.
- `walking_bass_end_of_section_falls_back` — on a section's LAST step (`next_chord == None`), the Bass arm returns the sustained root (the §R-B fallback), no panic.
- `walking_bass_legal_bass_line` — no onset leaps by a dissonant melodic interval; the line is stepwise-or-chord-tone (the bass-line legality property S30 §1 named).
- `pedal_point_holds_under_changing_harmony` — under `pad_pedal` over a 3-chord stream, the Bass note is the SAME pinned `pedal_degree` pitch on every step regardless of the chord; the upper voices' chords change above it.
- `pedal_degree_selects_tonic_or_dominant` — `pedal` pins scale-degree 1; `pedal_dom` pins degree 5.
- `bass_pattern_none_is_byte_identical` — a profile with `bass_pattern_resolved: None` realizes the IDENTICAL Bass events as today (the sustained-root freeze guard).

**Data-validity + selection (implementer-lane surface):**
- `s34_bass_pattern_catalogue_parses` — every `bass_pattern_catalogue` row deserializes; `walking`/`pedal` kinds resolve; `density`/`pedal_degree` defaults apply.
- `s34_texture_rules_reach_new_profiles` — the appended `texture` rules select `pad_oom_pah`/`pad_stride`/`pad_walking`/`pad_pedal` on their gate features and NONE on the `neutral()` sentinel (the Slice-1 `s30_texture_rules_reach_new_figured_profiles` discipline, extended).
- `s34_old_rows_unchanged` — update the existing `s30_figuration_backward_compat_old_rows_unchanged` doc-comment (`composition.rs:2268`) to reflect that `FigurationOnset` NOW has `register_octaves` (default 0); its functional assertions (alberti `voices==3`/`onsets==4`, block present) stay green.

**Freeze witnesses (the §6 battery, asserted in CI):** engine.rs sha == anchor; `engine_equivalence` 9/9 byte-green + goldens unmoved; `git diff HEAD -- tests/engine_equivalence.rs` EMPTY; PT-0 green; the existing `figuration_s20`/counterpoint nets green.

---

## 8. RISKS / TRADE-OFFS / OPEN DECISION POINTS

### Freeze-safe verdict recap (no risk here)
Part (A) `register_octaves` is **Path 1 freeze-safe** with NO engine.rs touch, NO `realize_step` change, NO golden movement — the value rides the S20 resolved-spec seam already on `ctx`. The only realizer edit is one line in `figured_bed`, no-op on every existing row. Part (B) walking/pedal ride the SAME seam class (a new `bass_pattern_resolved` parallel to `figuration_resolved`) with the same freeze properties; the next-chord lookahead walking needs is the proven S33 in-realizer `ctx.section.steps.get(si+1)` read. **Neither part can move the byte-freeze.**

### Open decision points needing the lead's / operator's input

- **OD-1 (musical taste — the headline fork): how do oom-pah and stride map from image features?** §3.1 proposes: oom-pah ← bright + danceable (`valence ≥ 0.60, arousal ∈ [0.40,0.65]`); stride ← high-arousal + colorful (`arousal ≥ 0.75, colorfulness ≥ 0.55`). These are *placeholders the music owner should confirm or retune* — the affect→idiom mapping is a taste call the property net cannot adjudicate (it can prove the figure is *legal*, not that it *reads as* the intended character). **Needs the lead's sign-off on the SelectTable thresholds before they ship**, or a deferral to leave the profiles inert (authored, un-selected) until an ear pass tunes them.
- **OD-2 (walking-bass passing-tone policy): diatonic-only, or chromatic-approach-allowed?** §3.2 specifies diatonic passing/neighbor tones between roots (the conservative, always-legal choice). A jazz walking bass often uses a CHROMATIC approach tone into the next root (a half-step below/above). Chromatic approach is more idiomatic but introduces a non-diatonic bass pitch. **Recommend diatonic-only for Slice 2 (property-testable, never wrong); flag chromatic approach as a Slice-2.5 ear-gated option.** Needs a lead nod on which to ship.
- **OD-3 (pedal-degree default + selection): tonic pedal only, or both tonic + dominant?** §3.2 authors both (`pedal` = tonic, `pedal_dom` = dominant) but the SelectTable only needs to gate one initially. Dominant pedals are more tension-laden (a standing V under the harmony). **Recommend ship the tonic pedal selected, author the dominant pedal un-selected.** Needs a lead nod on whether to gate the dominant pedal now.
- **OD-4 (triple-meter readiness): `oom_pah_pah` un-selected until meter grows?** The 3-onset waltz row reads as a 4/4 arpeggiation today. **Recommend authoring it + its profile but leaving it un-selected** (the S30 R-C "inert until a ladder selects it" discipline) until a triple meter exists. Needs only a lead acknowledgment, not a decision.
- **OD-5 (the `bass_pattern` seam scope): a NEW parallel catalogue, or fold into figuration?** §3.2 PINS a new `BassPatternSpec`/`bass_pattern_resolved` seam (option B-i) over overloading figuration (B-ii rejected on module-boundary grounds: Pad-voicing vs bass-voicing are different domains). This is a structural call, not musical — flagged so the lead can confirm the new seam is acceptable (it adds one small serde type + one resolved field, mirroring the proven figuration mechanism exactly). If the lead prefers a smaller surface, part (B) could ship walking/pedal as a single enum tag directly on `OrchestrationProfile` (no catalogue) — but that loses the `density`/`pedal_degree` data-tuning the catalogue gives. **Recommend the catalogue (B-i).**

### Trade-offs (decided, noted for transparency)
- **Register band for the "oom":** a `-1` shift puts the oom at ~43 (G2), between the bass floor (36) and fill floor (55). This deliberately overlaps the register ABOVE the true Bass root and BELOW the inner stabs — the correct oom-pah register (the alternating-bass left hand). It is NOT constrained to `[55,67)` (that is the counter voice's band; the Pad bed already seats freely). Decided; the §6 chord-tone witness asserts pitch-class, not band, for shifted onsets.
- **Two voices on the bass register (oom + walking/pedal):** an oom-pah Pad profile and a walking/pedal Bass profile are INDEPENDENT (a profile may carry `figuration: "oom_pah"` AND `bass_pattern: "walking"`). If both place notes low, they could collide. **Mitigation:** the SelectTable rules should not select an oom-pah figuration AND a walking bass on the same profile in Slice 2 (keep the part-A and part-B profiles distinct); flagged so the implementer does not author a combined profile without an ear check. Property-wise both are legal; the collision is a taste concern (OD-1-adjacent).

---

## Appendix — file/line index of cited existing code (read-only, HEAD `20fc407`)

| Symbol | File | Lines |
|---|---|---|
| `realize_step` (PUBLIC 7-param, in chord_engine NOT engine) | chord_engine.rs | 1055–1062 |
| engine.rs `realize_step` call seam (frozen) | engine.rs | 740 |
| `OrchestralRole::Pad` rhythm arm + figured-bed dispatch | chord_engine.rs | 1651–1728 (dispatch :1709) |
| `figured_bed` (the onset→NoteEvent mapper; `let note = seated[idx]` at :1923) | chord_engine.rs | 1894–1946 |
| `OrchestralRole::Bass` rhythm arm (walking/pedal land here) | chord_engine.rs | 1607–1626 |
| Bass `role_pitch` arm | chord_engine.rs | 1218 |
| next-chord lookahead precedent (`ctx.section.steps.get(si+1)`) | chord_engine.rs | 1740 |
| `seat_pc_in_register` | chord_engine.rs | 1282 |
| `FILL_REGISTER_FLOOR=55` / `BASS_REGISTER_FLOOR=36` | chord_engine.rs | 1189–1190 |
| `PAD_OVERLAP_FRAC=1.10` | chord_engine.rs | 1447 |
| `assign_role` / identity gate | chord_engine.rs | 915–934 / 1063 |
| `FigurationOnset` (gains `register_octaves`) | composition.rs | 547–557 |
| `FigurationSpec` | composition.rs | 531–543 |
| `OrchestrationProfile` (+ `figuration_resolved` carrier) | composition.rs | 380–413 |
| `OrchestrationProfile::identity()` | composition.rs | 424–434 |
| planner figuration resolve block | composition.rs | 1174–1179 |
| `lookup_figuration` | composition.rs | 1339 |
| `figuration_catalogue` on PlanMappings | composition.rs | 774 |
| `From<CompositionMappings>` map | composition.rs | 810 |
| `Section.orchestration` / `.steps` | composition.rs | 894 / 898 |
| `StepContext` | composition.rs | 971–979 |
| back-compat pin (`s30_figuration_backward_compat_…`) | composition.rs | 2272 (doc :2268) |
| `CompositionMappings` loader mirror | mapping_loader.rs | ~110 |
| `shape_to_ostinato` (parsed, UNWIRED) | mapping_loader.rs / mappings.json | 96,231 / 96–100 |
| `figuration_catalogue` / `texture_catalogue` / `texture` SelectTable | mappings.json | 239–273 / 229–238 / 274–294 |
| engine.rs sha256 anchor | src/engine.rs | `e50c7db…2348261` |

---

*Design-only build spec. No source, test, or asset modified by this document (docs/ only). The Rust signatures and `mappings.json` rows here are the BINDING build contract for Slice 2: the Implementer builds `composition.rs`/`mapping_loader.rs`/`mappings.json` against §3.1(impl)+§3.2(impl), the Music Theory Specialist builds the `chord_engine.rs` `figured_bed` register-shift + the walking/pedal generators against §3.1(music)+§3.2(music), the Test Engineer builds `tests/pattern_library_s34.rs` against §7, and §6's freeze witnesses gate the merge. The headline verdict (§0/§2): `register_octaves` is Path-1 freeze-safe — engine.rs and realize_step stay frozen.*

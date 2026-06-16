# S19 — The Accompaniment Figuration Engine (Slice 3 of the Composition Architecture arc)

**Author role:** Rust Architect (DESIGN ONLY — no source, test, or asset modified by this document; `docs/` only).
**Date:** 2026-06-15.
**Status:** PROPOSE-FOR-ITERATION. This document owns the ENGINE architecture for the FIGURATION layer: where multi-voice rhythmic figuration lives in the realize path, how the figuration vocabulary threads through as DATA (reusing the S15/S17 content-as-data machinery — no parallel mechanism), how the `engine_equivalence` byte-freeze is preserved unmoved, the saliency→layer-role engine seam the operator asked for, and the staged build. The Music Theory Specialist owns, in parallel, the MUSICAL figuration contract (what each pattern *is* — the Alberti order, the comping placement, the broken-chord shape, the register banding, the voice-leading of the figuration tones). §2/§4/§6 flag every seam where the two reconcile.

**What figuration is (the one-sentence scope):** a multi-voice rhythmic-PATTERN layer that animates a held harmony *across the beat* — Alberti bass (low-high-mid-high), broken chords, comping stabs, on/off-beat oom-pah. The S18 single counter-line is the **one-line down-payment**: one moving inner voice, ≤1 `NoteEvent`/step. Figuration is its **multi-voice generalization**: a bounded BURST of `NoteEvent`s per step (off-beat onsets at `step_ms` fractions) that turns a single held chord into a rhythmically alive accompaniment without changing the harmony.

---

## 0. Executive summary (read first)

The S17 Pad bed made the harmony *present* (a held chord under the tune); the S18 counter-line made *one* inner voice *move* (the "empty periods" fix, ≤1 event/step). What is still missing is **rhythmic animation of the held chord itself** — when a chord holds, the Pad sounds it as a static block and the counter steps a single tone through it, but there is no Alberti/broken-chord/comping FIGURE breaking the chord into a moving multi-onset pattern. That is the figuration layer.

The whole layer is reachable by **exactly the precedent the S17 Pad and the S18 counter already proved**: a new realize arm reached ONLY through a non-identity `OrchestrationProfile` on the compose path; the figuration vocabulary is **data in `mappings.json`** parsed by a new `#[serde(default)]`-additive `FigurationSpec` row and selected by the existing `SelectTable`; the byte-freeze holds because `single_section_default`'s identity profile never names a figuration-bearing layer, so the arm is structurally unreachable on the `engine_equivalence` net. `realize_step`'s public signature is UNCHANGED (the figuration arm reads the chord via `step.chord` and the spec via `ctx.section.orchestration` — both already in scope, exactly as the Pad's `pad_voices` and the counter's `ctx` are).

**Recommended FIRST slice: the ALBERTI/BROKEN-CHORD FIGURE on the Pad layer only.** Re-skin the existing `Pad` arm so that when its section's profile carries a `figuration` spec, instead of emitting `pad_voices` simultaneous held tones at offset 0 it emits the spec's onset/tone template (e.g. 4 onsets at `0, step_ms/4, step_ms/2, 3·step_ms/4`, each one inner chord tone). One data row, one new realize sub-branch, one new profile (`pad_figured`), one `SelectTable` rule keyed on the saliency knobs, the byte-freeze untouched, headless-testable through a hand-built profile. The counter line is left exactly as S18 shipped (it is the melodic figuration; the Pad figuration is the harmonic figuration — they are file-disjoint and compose cleanly). Files touched: `src/chord_engine.rs` (the figuration sub-branch + the spec reader), `src/composition.rs` (the `FigurationSpec` type + the field on `OrchestrationProfile`), `assets/mappings.json` (the `pad_figured` row + rule), `tests/` (a new figuration net).

---

## 1. WHERE FIGURATION LIVES — the realize-path placement

### 1.1 The current realize path (grounded at HEAD, post-S18)

`realize_step` (`chord_engine.rs:956`, signature FROZEN) computes `role = assign_role(inst_idx, num, ctx)` then `pad_voices = ctx.section.orchestration.pad_voices`, derives `base_note` (the theme/free-select melody seam), `velocity`, and dispatches to the private `realize_rhythm` (`:1259`), which after the `is_cadence` early-return (`:1370`) `match`es on `role`:

- `Bass` (`:1375`) — one sustained root (two onsets only `pre_cadence`).
- `HarmonicFill` (`:1396`) — one sustained inner tone, or a rest-as-gesture.
- `Pad` (`:1419`) — **N simultaneous held inner tones at offset 0** (the S17 bed): `pad_voices` inner chord tones (`notes[1..]`, root-skipped, de-duped, seated in the fill register), each held `step_ms × PAD_OVERLAP_FRAC`.
- `CounterMelody` (`:1480`) — **the S18 moving inner line**: ≤1 `NoteEvent`/step, off-beat (`step_ms/4`) in held/static periods, the contrary-motion pick.
- `Melody` (`:1571`) — the subdividing tune (arpeggio/syncopated/dotted/sustained bands).

The decisive read: **the Pad and CounterMelody arms are already the two figuration seats.** The Pad arm already emits a *multi-event* burst (it is the only role that emits >1 `NoteEvent`) — but all at offset 0 (a block, not a figure). The CounterMelody arm already emits an *off-beat* single onset. **Figuration is the union: a multi-event burst placed at off-beat `step_ms` fractions.** It belongs in the realize path AT THE SAME ALTITUDE AS THE PAD ARM — a sibling `match role` branch (a new `OrchestralRole::Figuration`, §3), or, cheaper for the first slice, a *parameterized variant of the existing Pad arm* gated on whether the section's profile carries a figuration spec.

### 1.2 The placement decision: a Pad-arm sub-branch first, a sibling role later

Two architecturally clean placements; the recommendation is to take them in order across slices:

- **Slice-3a (recommended FIRST): figured Pad.** The Pad arm already owns "this role re-derives its whole voicing off `step.chord`" — it is the only arm that does not sound the single `base_note`. Adding `if let Some(fig) = &ctx.section.orchestration.figuration { … emit the figured burst … } else { … the S17 block bed … }` INSIDE the existing `Pad` arm is the smallest reachable change: the Pad role already exists, is already selected by `pad_bed`-style profiles, already emits multiple events, and is already unreachable under identity. The figuration is a *rhythmic re-skin of the bed* — same tones, animated onsets. No new `OrchestralRole`, no new `assign_role` mapping, no `LayerRole` add. This is the same "additive sub-branch, no new role" move the S17 rest-fix used inside HarmonicFill.

- **Slice-3b (later, if a SECOND independent figured voice is wanted): a sibling `Figuration` role.** If the operator wants figuration to occupy its *own* instrument slot (e.g. melody + counter + figured-comping + bass = 4 independent strata), add `OrchestralRole::Figuration` + `LayerRole::Figuration` (the S17 enum-pair precedent) and a `figured_bed` profile that names it. This is a clean future diff precisely because Slice-3a kept the figuration *spec* on the profile (not hard-coded into the Pad arm), so 3b reads the same spec from a new arm.

**The recommendation is 3a first** because it animates the harmony with zero new role vocabulary and rides the already-selected Pad layer; 3b is a width/independence question deferred until the operator has heard 3a.

### 1.3 The bounded multi-event burst WITHIN the locked scheduler — the load-bearing constraint

`main.rs` (LOCKED OFF) blocks until the step's LAST event completes, so **true cross-step sustain is out of scope** (the S17 Pad already accepts this — `PAD_OVERLAP_FRAC = 1.10` keeps each bed within a ≤10% wall-clock over-run, never the N× catastrophe of a multi-step hold). Figuration inherits the SAME discipline and is in fact *more* scheduler-friendly than the block Pad, because its onsets land at `step_ms` fractions *within* the step:

- **Onsets are at `step_ms` fractions, never beyond the step.** The figuration template places onsets at `frac · step_ms` for `frac ∈ {0, 1/4, 1/2, 3/4}` (the S18 `step_ms/4` precedent generalized to a per-onset list). The LAST onset's `offset + hold` must not exceed `step_ms × PAD_OVERLAP_FRAC` (the established ≤1.2× ceiling the `sustained` helper already enforces) so the scheduler's block-until-last over-runs each step by ≤10%, exactly as the block Pad does today. A figure of 4 onsets each holding `step_ms/4 × frac` naturally fits — the burst FILLS the step rather than overhanging it.

- **Per-onset hold ≤ the gap to the next onset (legato within the step).** Each figured note holds up to (next_onset − this_onset) × an articulation fraction, so successive figure notes connect without the last one overhanging the step boundary. This is the in-step analogue of the `sustained` helper; the figuration arm reuses the same `(frac · rit).min(1.20)` cap discipline.

- **No `hold_ms` spans a step.** The figure is COMPLETE within its step. Cross-step continuity (an Alberti pattern that flows across a held run) is achieved the way the S18 counter already achieves a moving held line — by the *plan position* selecting which tone each step's figure starts on (§4.4), NOT by a `hold_ms` that crosses the boundary. The scheduler never sees an over-long note.

This is why figuration is scheduler-safe: it is a denser packing of onsets *inside* the step the scheduler already budgets, not a longer note that breaks its block-until-last contract.

### 1.4 Bounding the event count (the S18 ceiling lifted, but bounded)

The S18 counter ceiling was ≤1 event/step (pinned by `test_counter_at_most_one_event_*`). Figuration deliberately lifts that ceiling FOR THE FIGURATION LAYER ONLY, but bounds it: a figuration burst emits **`onset_count` events, where `onset_count` is a small fixed field on the `FigurationSpec` (LOCK: 2..=4)**. The bound is data (the spec row), enforced by a `min`-clamp in the arm and pinned by a test (`figuration_emits_at_most_max_onsets`). The counter and every other arm keep their existing ceilings — only the figured Pad/Figuration arm may emit the bounded burst. This keeps "figuration is not unbounded arpeggiation" a structural property, the same way the S18 ≤1 ceiling kept "the counter is not figuration."

---

## 2. DATA-AS-VOCABULARY THREADING — the figuration catalogue as content-as-data

The governing discipline (S15 §1.2, re-affirmed by S17/S18): **open CONTENT is data in `mappings.json`; bounded new MECHANISM is code.** Figuration patterns are open content — there are many (Alberti, broken-chord-up, broken-chord-updown, comping-offbeat, oom-pah, waltz-pah-pah) and the operator will want to add/tune them without a recompile. So the pattern vocabulary is DATA, selected by the existing `SelectTable`, attached to the existing `OrchestrationProfile`. **No parallel mechanism is invented** — figuration reuses the exact `texture_catalogue`/`texture`-`SelectTable`/`OrchestrationProfile` machinery S17 shipped.

### 2.1 The `FigurationSpec` row schema (NEW serde struct, `composition.rs`)

A figuration spec is pure structure: an onset template (when, and which chord-tone seat), a voice count, a register band. It lives as an OPTIONAL field on `OrchestrationProfile` (so a profile either carries a figured bed or the S17 block bed):

```rust
// ── src/composition.rs (NEW serde struct) ─────────────────────────────────────
/// One named accompaniment-figuration pattern — pure STRUCTURE, no note content. Animates a
/// held chord into a rhythmic multi-onset figure. Attached (optionally) to an
/// [`OrchestrationProfile`]; the realizer's figured-bed branch reads it. Adding a pattern is a
/// JSON row, NOT a Rust edit (the S15 FormSpec / S17 OrchestrationProfile discipline). NEW S19.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct FigurationSpec {
    /// Stable id, e.g. "alberti" / "broken_up" / "comping_offbeat" / "block" (block == no-op).
    pub id: String,
    /// The per-step onset template, in time order. EACH entry is one note of the figure: WHEN
    /// it onsets (as a fraction of step_ms) and WHICH chord-tone seat it sounds. 2..=4 entries
    /// (the bounded burst, §1.4). serde rejects a malformed entry. Empty == a block bed (no-op).
    #[serde(default)]
    pub onsets: Vec<FigurationOnset>,
    /// How many distinct inner chord tones the figure draws from (the "voice count" — Alberti
    /// uses 3: low-high-mid-high over {0,1,2}). Clamped to the chord's available band tones.
    #[serde(default = "one_u8")]
    pub voices: u8,
    /// Register-band floor the figure seats in (MIDI). Default == the fill register (G3=55),
    /// where the inner figuration lives, under the melody. Reuses the seat machinery; the music
    /// layer reads this as a normalized band choice, NEVER an image type.
    #[serde(default = "fill_floor_u8")]
    pub register_floor: u8,
}

/// One onset of a figuration figure: its onset time (fraction of the step) and which inner
/// chord-tone INDEX it sounds (0 == lowest band tone, 1 == next, …; modulo the seated voice
/// count, so a 4-onset Alberti over 3 voices cycles 0,2,1,2). NEW S19.
#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize)]
pub struct FigurationOnset {
    /// Onset time as a fraction of step_ms, 0.0..1.0 (0 == downbeat, 0.25 == the S18 off-beat).
    pub at: f32,
    /// Which seated inner-voice index this onset sounds (cycled modulo the voice count).
    pub tone: u8,
    /// Hold fraction of the gap-to-next-onset, 0.0..1.0 (articulation within the figure).
    #[serde(default = "one_f32")]
    pub hold_frac: f32,
}
```

`OrchestrationProfile` (`composition.rs:209`) gains ONE additive `#[serde(default)]` field:

```rust
pub struct OrchestrationProfile {
    pub id: String,
    pub layers: Vec<LayerRole>,
    #[serde(default = "half_f32")] pub density: f32,
    #[serde(default)] pub pad_voices: u8,
    /// NEW S19 — the figuration pattern this profile's bed animates with, or None for the S17
    /// block bed. `#[serde(default)]` (== None) so EVERY old mappings.json profile (identity,
    /// pad_bed, pad_bed_counter) still parses unchanged and keeps the static-bed behaviour.
    #[serde(default)]
    pub figuration: Option<FigurationSpec>,
}
```

### 2.2 Back-compat additivity (the load-bearing serde discipline)

This is the same proof S17/S18 used, applied to the new field:

- **`figuration: Option<FigurationSpec>` is `#[serde(default)]`.** An OLD `mappings.json` (no `figuration` key on any profile) parses with `figuration: None` on every profile → the realizer takes the S17 block-bed path → byte-identical to S18. The three shipped profiles (`identity`, `pad_bed`, `pad_bed_counter`) are unchanged on disk and parse exactly as before.
- **Every nested field is `#[serde(default)]`.** `onsets` defaults empty (→ block bed, the no-op figure), `voices` to 1, `register_floor` to 55, `hold_frac` to 1.0. So a partially-specified figuration row still parses; a malformed *closed* member (a non-numeric `at`) is a serde reject at load (fail-fast, the desired behaviour for a hand-edited catalogue).
- **`identity()` and `is_identity()` are UNTOUCHED in semantics.** `identity()` constructs `figuration: None` (the new default), and `is_identity()` still keys only on `pad_voices == 0 && layers.is_empty()` — a figured profile always has `pad_voices > 0` (it animates a bed), so it is never mistaken for identity. The byte-freeze anchor is unchanged.

### 2.3 Selection: the existing `SelectTable`, a new catalogue row, NO new axis

Figuration selection reuses the `texture` `SelectTable` and the `texture_catalogue` exactly — a figured profile is just another `OrchestrationProfile` row. NO new `PlanMappings` axis is added (no `figuration` `SelectTable`); the figuration *is* a property of the selected texture profile, so selecting `pad_figured` (a profile that carries a `figuration`) IS selecting figuration. The `mappings.json` add for the first slice:

```jsonc
"texture_catalogue": [
  { "id": "identity", "layers": [], "density": 0.5, "pad_voices": 0 },
  { "id": "pad_bed",  "layers": ["Bass","Pad","HarmonicFill","Melody"], "density": 0.55, "pad_voices": 3 },
  { "id": "pad_bed_counter", "layers": ["Bass","Pad","CounterMelody","Melody"], "density": 0.6, "pad_voices": 3 },
  // NEW S19 — the SAME pad bed, but the Pad layer ANIMATES with an Alberti figure instead of a
  // static block. Selected when the subject is salient enough to warrant a livelier accompaniment
  // (a strong subject earns a moving bed under it). Music Theory owns the onset/tone values.
  { "id": "pad_figured",
    "layers": ["Bass","Pad","CounterMelody","Melody"], "density": 0.62, "pad_voices": 3,
    "figuration": {
      "id": "alberti", "voices": 3,
      "onsets": [ {"at":0.0,"tone":0}, {"at":0.25,"tone":2}, {"at":0.5,"tone":1}, {"at":0.75,"tone":2} ]
    } }
],
"texture": {
  "default": "pad_bed",
  "rules": [
    // S19: a STRONG, ENERGETIC subject earns the figured bed (the richest accompaniment). Checked
    // FIRST (most specific). subject_energy busy AND fg_bg_contrast shows a real subject.
    { "when": [ {"knob":"subject_energy","op":"ge","lo":0.45,"hi":0.0},
                {"knob":"fg_bg_contrast","op":"ge","lo":0.25,"hi":0.0} ], "pick": "pad_figured" },
    // S18 (unchanged): a busy foreground over a real subject earns the counter line.
    { "when": [ {"knob":"foreground_energy","op":"ge","lo":0.35,"hi":0.0},
                {"knob":"fg_bg_contrast","op":"ge","lo":0.20,"hi":0.0} ], "pick": "pad_bed_counter" }
  ]
}
```

(The Music Theory Specialist authors the `onsets`/`voices`/threshold musical values; the Rust Implementer is the SOLE writer of `mappings.json`, exactly as in S18.)

### 2.4 Where the spec attaches and threads (zero new thread)

The figuration spec rides the EXACT path the S17 `pad_voices` already rides: `OrchestrationProfile` → `Section.orchestration` (the planner clones the selected profile onto each section, `composition.rs:754`) → `StepContext.section` (borrowed, already in scope in `realize_step` and `realize_rhythm`) → read in the Pad arm as `ctx.section.orchestration.figuration`. **No new parameter on any function**, public or private — the figured-bed branch reads the spec off the borrowed `ctx` the S18 counter already threads. This is the cleanest possible thread: the borrow is already there.

---

## 3. BYTE-FREEZE GUARANTEE — the goldens 240/114/84/36/79 stay UNMOVED

The proven pattern (S13/S15/S17/S18): new behaviour reachable ONLY on the compose path through a non-identity profile; `single_section_default`'s identity profile keeps the realizer's role assignment to Bass/Melody/HarmonicFill so no figuration arm is ever exercised by `engine_equivalence`.

### 3.1 The unreachability argument, re-derived (not trusted)

`engine_equivalence.rs` builds a FIXED `&[StepPlan]` by hand and calls the realizer with `OrchestrationProfile::identity()` (the `default_section` carries it, S18 review §1). Under identity:

1. `assign_role` (`:918`) sees `prof.is_identity() == true` → delegates to `instrument_role`, which returns ONLY `Bass`/`HarmonicFill`/`Melody` (grep-verified in S18 review §1; `CounterMelody`/`Pad`/`Figuration` are NOT reachable). So the realizer never enters the Pad arm, hence never reads `figuration`, hence the figured-bed sub-branch is **structurally unreachable** independent of the green test.
2. `pad_voices` is `0` under identity → even if some future identity-mapped instrument WERE a Pad, the `pad_voices == 0` defensive path holds nothing. `figuration` is `None` under identity → the block path is taken regardless.
3. `figuration: Option<FigurationSpec>` is an ADDITIVE field; the net constructs no `OrchestrationProfile` carrying it (it uses `identity()`), and the new `FigurationSpec`/`FigurationOnset` types/`Knob` reads are serde/getter-only — never read at the default operating point.

So the figured-bed code is added entirely behind a profile the equivalence net never carries. **Goldens `G_BASS_NOTE=36`, `G_MELODY_NOTE=79`, cadence vel `114`/`84`, hold `240`, `MS_PER_STEP=200` are UNMOVED.** No golden re-derivation in this slice (same as S17/S18 — figuration moves NO golden).

### 3.2 `realize_step`'s public signature is UNCHANGED

The figuration arm needs the chord (`step.chord`, already in scope), the spec (`ctx.section.orchestration.figuration`, already in scope via the borrowed `ctx` the S18 counter threads), `velocity` and `step_ms` (already local in `realize_rhythm`). **Zero new parameter** on `realize_step` (public, FROZEN) OR on the private `realize_rhythm` — unlike S18, which had to thread `ctx` into `realize_rhythm`, S19 needs NO new thread because S18 already put `ctx` there. The figured-bed branch is a pure body addition inside the existing `Pad` arm. (Slice-3b's sibling `Figuration` role would add only a `match` arm + a `LayerRole`/`OrchestralRole` variant — still no signature change, same as the S17 Pad add.)

### 3.3 The verification approach (the S18 method, repeated)

The Quality Gate verifies the freeze by:
- `git diff HEAD -- tests/engine_equivalence.rs` is EMPTY (the net is byte-untouched).
- `cargo test` (default features) — `engine_equivalence` green (9/9), goldens confirmed in-file unmoved.
- **sha256 of every off-limits file vs `git show HEAD:`** — `engine.rs`, `main.rs`, `modem.rs`, `synth_sink.rs`, `midi_output.rs`, `cli.rs`, `tui.rs`, `bin/modem_*`, `bin/audiohax-tui.rs` — all MATCH (the figuration touches none of them).
- Re-derive `assign_role`/`instrument_role` to confirm `Pad`/`Figuration` is unreachable under identity (the S18 method, independent of the green test).
- A throwaway example driving the figured-bed arm directly to confirm the burst is bounded, in-step, and off-beat (the S18 held-run probe method).

### 3.4 The byte-freeze table (consolidated)

| S19 change | Default-path behaviour | Net impact |
|---|---|---|
| `FigurationSpec`/`FigurationOnset` types | serde-only; the net constructs none | GREEN |
| `OrchestrationProfile.figuration: Option<…>` (`#[serde(default)]`) | identity carries `None`; the net uses `identity()` | GREEN — additive, defaulted |
| `pad_figured` profile + texture rule | `#[serde(default)]` catalogue; net builds its plan by hand, never loads it | GREEN |
| the figured-bed sub-branch in the `Pad` arm | unreachable under identity (`assign_role` never returns `Pad`); `figuration: None` falls to the S17 block bed even if reached | GREEN |
| (Slice-3b) `OrchestralRole::Figuration` + `LayerRole::Figuration` | new `match` arm + enum variant; `instrument_role` never returns it under identity | GREEN — additive arm, unreachable at default |

---

## 4. THE SALIENCY → LAYER-ROLE COUPLING — the engine seam (operator requirement)

The operator wants **subject-salience to drive which layer carries the richer figuration and how dense it is.** The engine seam is already 90% built by S18 — the `ImageUnderstanding` saliency knobs exist and the `SelectTable` already reads them. S19 wires them to figuration selection + intensity through the SAME path, keeping `engine.rs`'s pure-no-image-type constraint intact (the music layer reads normalized knobs, never image types).

### 4.1 The knobs that already exist (S18, in `composition.rs` + `Knob::read`)

`ImageUnderstanding` carries `subject_energy`, `foreground_energy`, `background_energy`, `fg_bg_contrast`, `subject_size` (all 0..1 normalized scalars filled by `pure_analysis.rs::understand_image_pure` on the compose path), and `Knob::read` (`composition.rs:304`) already has the arms for every one. **No new knob, no new `Knob` variant, no new field is needed** — S18 shipped the full saliency knob surface precisely so S19's selection rules are a pure JSON add (the S18 spec §2.2 said this explicitly). This is the payoff of S18 having shipped the energy triplet ahead of need.

### 4.2 The mapping: salience → WHICH layer carries the richer figuration

The throughline (design-s16 §3.2, now realized): **subject → the figured bed; foreground → the counter line; background → the static pad depth.** Concretely, as ordered first-match-wins `SelectTable` rules over the saliency knobs (§2.3):

- **Strong, energetic SUBJECT** (`subject_energy ≥ 0.45 ∧ fg_bg_contrast ≥ 0.25`) → `pad_figured`: the bed ANIMATES (Alberti/broken-chord) under the prominent subject — the richest accompaniment for the most salient image. The subject earns the moving harmonic figure.
- **Busy FOREGROUND, real subject** (`foreground_energy ≥ 0.35 ∧ fg_bg_contrast ≥ 0.20`) → `pad_bed_counter` (S18, unchanged): the counter line moves, the bed stays a block — a busy mid-ground earns the second melodic line.
- **Otherwise** → `pad_bed` (S17 default): static bed, no counter, no figure — a quiet image gets the calm accompaniment.

The conjunctive `fg_bg_contrast` guard on the figured rule is the same structure-not-average discipline S18 used: figuration only enters when there is a real subject/ground stratification for it to ornament — a busy abstract with no subject does NOT trigger the figure (its bed stays a block; the rhythm lives in the melody).

### 4.3 The intensity coupling — salience → HOW dense the figuration is

"How dense" is the figure's `onset_count` / `voices`. Two clean engine seams, in priority order:

- **Discrete (Slice-3a, recommended): salience selects the PROFILE, the profile fixes the density.** A `pad_figured` row carries a 4-onset Alberti; a future `pad_figured_sparse` row carries a 2-onset broken-chord; the `SelectTable` picks between them on a `subject_energy` threshold ladder (more subject energy → the denser profile). This is pure DATA (more catalogue rows + more rules), no Rust edit, and keeps "density is a discrete, curated choice" — the vocabulary-discipline posture (no continuous knob to over-tune).
- **Continuous (deferred, Slice-3c): a per-section figuration-intensity scalar.** If the operator wants the figure's onset count to *scale continuously* with `subject_energy`, the seam is the existing `OrchestrationProfile.density` field (reserved schema since S17, currently a no-op) — the figured-bed arm would read `ctx.section.orchestration.density` and drop/keep onsets above a threshold (e.g. density < 0.5 → use only the on-beat onsets of the template). This is ONE bounded scalar read in the arm, the same minimal-mechanism move S16 §3.4 proposed for the Pad. **Deferred** to keep Slice-3a to one new mechanism; the knob is already there when wanted.

### 4.4 Per-LAYER selection, not per-plan — the one engine change the seam needs

There IS one real engine seam to flag. Today the planner selects the texture profile **ONCE per plan** (`composition.rs:707`) and clones it onto EVERY section (`:754`). For the operator's "subject drives which layer is rich" to vary *across sections of one piece* (e.g. a figured bed in the high-salience A section, a plain bed in the quiet B), the selection must move to **per-section**:

- **Slice-3a stays per-plan** (the figured bed is selected once for the whole piece on the whole-image subject knobs) — this is the cheapest first cut and already realizes "this image's accompaniment is figured" vs "this image's is plain." It matches how S18 shipped (S18 selects the counter once per plan, `composition.rs:707` comment: "section-conditioned selection is a later slice").
- **Slice-3b/c (per-section)** moves the `texture.select(u)` call INSIDE the per-section loop (`composition.rs:715`), selecting a profile per section with a per-section understanding. This is the "later slice" S18 already named. It is a small planner change (one line moves into the loop) — but it needs a per-section `ImageUnderstanding`, which is itself the region-saliency-per-section work (Stage 9's deeper end). **Flag, not Slice-3a scope.** The whole-image subject knobs select the piece's figuration in 3a; per-section figuration is 3b/c.

The pure-no-image-type constraint is intact throughout: the `SelectTable` reads normalized `subject_energy`/`fg_bg_contrast` scalars off `ImageUnderstanding` (the image-free mirror), never an image type. `engine.rs` and `composition.rs` see only `f32` knobs; `pure_analysis.rs` (which DOES touch pixels) fills them at the boundary, exactly as today.

---

## 5. STAGED BUILD SLICES

Each slice builds + tests headless + keeps `engine_equivalence` byte-green. Sequenced cheapest-high-value first.

### 5.1 Slice-3a — FIGURED PAD (the Alberti/broken-chord bed) — RECOMMENDED FIRST

**The cheapest high-value cut: re-skin the existing Pad bed into an animated figure, selected by subject salience, byte-freeze untouched, one pattern subset (Alberti) on one layer (Pad).**

- **Pattern subset:** ONE figure — Alberti/broken-chord (`{0:tone0, ¼:tone2, ½:tone1, ¾:tone2}` over 3 inner voices). One profile (`pad_figured`), one `SelectTable` rule (subject-energy-gated).
- **Layer:** the existing **Pad** layer only (no new role). The figured-bed sub-branch lives inside the `Pad` arm of `realize_rhythm`; `figuration: None` → the S17 block bed (byte-unchanged); `figuration: Some(spec)` → the figured burst.
- **Files touched:**
  - `src/composition.rs` — `FigurationSpec` + `FigurationOnset` serde types; the `figuration: Option<…>` `#[serde(default)]` field on `OrchestrationProfile`; the `identity()` literal gains `figuration: None`. (Rust Implementer.)
  - `src/chord_engine.rs` — the figured-bed sub-branch inside the `Pad` arm: seat the `voices` inner tones (reuse the existing Pad seating, `:1444`–`:1463`), then map the spec's `onsets` to `NoteEvent`s at `at · step_ms` with the bounded-burst + in-step-hold discipline (§1.3/§1.4). New private helper `figured_bed(spec, &chord.notes, velocity, step_ms)`. The block path is untouched. (Music Theory owns the figure body; Rust Implementer owns the seam.)
  - `assets/mappings.json` — the `pad_figured` row + the figured `SelectTable` rule (§2.3). (Rust Implementer, SOLE writer; Music Theory hands the onset values.)
  - `tests/` — a new figuration net (§5.4).
- **New tests:** `figuration_emits_bounded_burst` (2..=4 events, never the unbounded count); `figuration_onsets_are_in_step` (every `offset + hold ≤ step_ms × 1.2`, the last onset never overhangs); `figuration_tones_are_chord_tones_in_band` (each figure note is a chord tone seated in the fill register); `figured_bed_off_beat` (the figure has ≥1 off-downbeat onset, distinguishing it from the block bed); `block_bed_unchanged_when_figuration_none` (a `pad_bed`-style profile with `figuration: None` still emits the S17 simultaneous-at-offset-0 block — the back-compat witness); `texture_selects_pad_figured_on_salient_subject` (the `SelectTable` rule fires on a hand-built `ImageUnderstanding`). Plus the freeze witness: `engine_equivalence` green, goldens unmoved.
- **Independence/byte-freeze:** fully self-contained; reachable only via `pad_figured` on the compose path; identity never names it; `realize_step` signature unchanged; NO golden moves.
- **Hearable:** for the first time the harmony MOVES rhythmically under a salient subject — a real accompaniment figure (Alberti/broken-chord) instead of a static block, animating the held chords the S17 bed only sustained.

### 5.2 Slice-3b — a SECOND figured voice / per-section selection / a sibling `Figuration` role

If the operator wants figuration on its OWN instrument slot (melody + counter + figured-comping + bass) and/or figuration that varies per SECTION: add `OrchestralRole::Figuration` + `LayerRole::Figuration` (the S17 enum-pair precedent) and a `figured_bed` profile naming it; move `texture.select` into the per-section loop (§4.4) for per-section figuration. Files: `chord_engine.rs` (the new arm reading the SAME `FigurationSpec`), `composition.rs` (the enum variant + the per-section selection move), `mappings.json` (the new profile). Needs Slice-3a's spec type. Byte-freeze: additive arm, unreachable under identity.

### 5.3 Slice-3c — figuration-intensity from `density` + a wider pattern catalogue

Wire the continuous `OrchestrationProfile.density` scalar (§4.3) into the figured-bed arm (drop onsets below a density threshold) and add more pattern rows (comping-offbeat, oom-pah, broken-up-down) with finer saliency thresholds. Mostly DATA + one bounded scalar read. No new role. Needs Slice-3a.

### 5.4 Per-slice net summary

| Slice | Files | New DATA vs MECHANISM | Freeze |
|---|---|---|---|
| **3a** | `composition.rs` (`FigurationSpec`/`FigurationOnset` + the field), `chord_engine.rs` (figured-bed sub-branch + helper), `mappings.json` (`pad_figured` + rule), `tests/` | DATA: the figure pattern + the saliency rule. MECHANISM: the figured-bed sub-branch + the onset→NoteEvent mapper. | GREEN — figured arm unreachable under identity; `realize_step` sig unchanged; NO golden moves |
| **3b** | `chord_engine.rs` (`Figuration` arm), `composition.rs` (`LayerRole::Figuration` + per-section select), `mappings.json` (`figured_bed`) | MECHANISM: the new role arm + the per-section selection move. | GREEN — additive arm, unreachable at default |
| **3c** | `chord_engine.rs` (density read), `composition.rs`/`mappings.json` (more rows/rules) | mostly DATA + one density scalar read | GREEN |

---

## 6. RISKS / OPEN DECISIONS

1. **Naming collision (the S17 lesson — load-bearing).** S17 renamed `TextureProfile` → `OrchestrationProfile` to avoid colliding with `ImageUnderstanding.texture: f32`. S19 must NOT introduce a `Figuration` name that collides with an existing token. Checked: there is no `figuration`/`Figuration` identifier in the tree today (the word "figure" appears only in prose comments, never as a type/variable/field name) — so the new names do not collide. LOCK the name `FigurationSpec`/`FigurationOnset`/`OrchestralRole::Figuration`/`LayerRole::Figuration` and the `figuration` JSON key; do NOT reuse `density` (already a reserved no-op field) for the figure's intensity until Slice-3c deliberately wires it. **Operator/Music-Theory steer:** confirm the layer is called "figuration" (vs "accompaniment pattern" / "comping") so the vocabulary is stable before the type lands.

2. **The locked scheduler tension (named, bounded, not blocking).** The scheduler blocks until the step's last event (`main.rs` LOCKED OFF). Figuration is scheduler-SAFE because its onsets land within the step and the last onset's `offset + hold ≤ step_ms × 1.2` (the established Pad ceiling) — §1.3. The ONE thing this forbids: a figure whose last onset's hold spans into the next step (true cross-step legato). That is OUT OF SCOPE (same constraint the S17 Pad and S18 counter already accept). **No operator steer needed** — it is the same accepted limit; flagged so it lands in the build, not as a surprise. A reverberant external synth (the actual listening path, per the PAD_OVERLAP_FRAC note) smooths the in-step re-attacks into a continuous figure.

3. **Per-plan vs per-section figuration selection (a genuine fork).** Slice-3a selects figuration once per plan (whole-image subject knobs); the operator's "which LAYER is rich, per the subject" is fully realized per-piece but not per-section. Per-section figuration (a figured A, a plain B) is Slice-3b/c and needs a per-section `ImageUnderstanding` (region-saliency-per-section, the deeper Stage-9 end). **Operator steer:** is per-piece figuration enough for the first cut (recommended — it is the S18 posture and immediately hearable), or is per-section figuration a Slice-3a requirement (more machinery, the region-per-section reader pulled forward)?

4. **The figure's tone-index semantics (Music-Theory decision, flagged for the engine seam).** The `FigurationOnset.tone` is an index into the seated inner voices (0 == lowest band tone). Whether Alberti's canonical low-high-mid-high maps to `{0,2,1,2}` over a root-skipped triad, and how the index behaves on a 7th chord (4 band tones) vs a triad (2 non-root tones), is the Music Theory Specialist's contract — the engine just seats `voices` inner tones and cycles the index modulo the seated count. **Flag:** confirm the index→seat mapping with Music Theory so the data rows are authored against a stable semantics (the same coordination S18 had for the counter's ring-tone rotation).

5. **Interaction with the S18 counter on the same profile.** `pad_figured` names BOTH a figured Pad AND a CounterMelody (the §2.3 row keeps the counter). The figured bed (harmonic figuration, fill register) and the counter (melodic figuration, counter register `[55,67)`) overlap in register — flag to Music Theory that the figured bed should de-prioritize the counter's current band tone, or seat a touch lower, to avoid the figure and the counter colliding on the same pitch. This is a VOICING coordination (Music-Theory-owned), not an engine seam — the engine emits both bursts; their non-collision is the music contract. **Operator/Music-Theory steer:** does the figured profile carry the counter, or does figuration REPLACE the counter (a simpler, cleaner first cut: `pad_figured` uses `HarmonicFill` for the 3rd slot, not `CounterMelody`, so only the bed is animated and there is no register overlap)? **Recommended first cut: figuration WITHOUT the counter on the same profile** (the `pad_figured` row above keeps the counter; the simpler alternative swaps it to `HarmonicFill`), deferring the counter+figure stack until both are individually heard.

---

*Design-only. No source, test, or asset modified by this document. Illustrative Rust signatures are non-binding; the Music Theory Specialist owns the figuration patterns' musical contracts (the Alberti/broken-chord/comping onset-and-tone values, the register banding, the figure↔counter voicing) and the Implementer owns the final signatures.*

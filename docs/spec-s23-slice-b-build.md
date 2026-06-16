# Spec S23 — Slice B Build Order: saliency → role prominence

**Author role:** Rust Architect — BUILDABLE SPEC. This document modifies no source, test, or asset; it is the single consolidated build order for **Slice B** so the Rust Implementer, the Music Theory Specialist, and the Test Engineer each build their file-disjoint half with no further design questions.
**Date:** 2026-06-15. **HEAD:** `89087f9` (Slice A shipped: affect→character + tempo de-cap; `AffectMappings`, the filled `character` ladder, and `character_tempo_bpm` are in tree).
**Authoritative design inputs (built from, not restated):** `docs/design-s21-affective-fidelity.md` §2.4/§2.5/§2.6(d)/§3/§4-Dec4/§5-Risk1; `docs/design-s21-musical-craft.md` Part B + §C.1.3/§C.3; `docs/design-s21-engine-reframe.md` Part 3 + §4.3/§4.4.
**engine.rs freeze witness (verified this session):** `sha256(src/engine.rs) = 7a07fb8568fcd94536dd3ec2e4dc71c8154f9d9678599a6106e3a542a4343c23`. Slice B does NOT touch `src/engine.rs`.

---

## 0. Scope + what stays frozen

Slice B is the FIRST witnessed realizer change. A salient image subject pushes the **Melody** forward (louder, higher, rhythmically freer) while recessive regions recede into a background bed, by **reweighting the existing five orchestration roles** (Melody / CounterMelody / HarmonicFill / Pad / Bass). NO new role, NO new image feature — the saliency knobs (`subject_size`, `fg_bg_contrast`, the `*_energy` triplet) already exist on `ImageUnderstanding`.

The mechanism: a planner-resolved `#[serde(skip)]` `prominence: Vec<LayerProminence>` on `OrchestrationProfile`, filled from a `prominence` SelectTable over `prominence_catalogue`, consumed by the realizer as **three CENTERED nudges each exactly `0.0` at weight `0.5`**. Under identity the field is empty → `prominence_weight` returns `0.5` → every nudge is `(0.5-0.5)*SPAN == 0.0` exactly → every emitted `NoteEvent` is bit-for-bit today's, independent of SPAN magnitudes.

**Frozen / locked off (verified anchors):**
- `src/engine.rs` — untouched; sha256 above stays the witness.
- `realize_step` PUBLIC signature (chord_engine.rs:957–963) — frozen. The weight reaches `realize_velocity`/`realize_rhythm`/`role_pitch` via the already-blessed **additive-private-param route** (the `pad_voices` precedent at chord_engine.rs:968,1029,1271).
- `OrchestrationProfile::is_identity()` (composition.rs:408–410) — UNCHANGED (keys on `pad_voices == 0 && layers.is_empty()`).
- The cadence ring (`is_cadence` early return, the 240 ms hold), the structural velocity floor, `assign_role`→`instrument_role` delegation under identity.
- `engine_equivalence` goldens 240/114/84/36/79 — UNMOVED. `src/midi_output.rs`, `src/synth_sink.rs`, `src/cli.rs`, `src/tui.rs`, `main.rs`, `src/modem.rs`, `src/bin/*` — untouched.

**The naming decision (resolved — §1.1):** `LayerProminence.role` is **`LayerRole`** (composition.rs:343–351), NOT a new enum and NOT `OrchestralRole`. `LayerRole` already exists, already derives `Deserialize` + `PascalCase` serde + `Copy`/`Eq`, already deserializes role strings `"Melody"`/`"CounterMelody"`/`"HarmonicFill"`/`"Pad"`/`"Bass"`, is already the element type of `OrchestrationProfile.layers`, and already has a total bridge to `OrchestralRole` (`to_orchestral_role`, chord_engine.rs:893–902). This is the least-surface option: zero new deserialization code, the §2.6(d) JSON role strings parse as-is, and the realizer bridges via the existing `to_orchestral_role`.

---

## 1. RUST IMPLEMENTER WORK-ORDER (composition.rs + mapping_loader.rs + mappings.json)

Owns `src/composition.rs` (planner), `src/mapping_loader.rs`, `assets/mappings.json`. Single-writer of the `prominence`/`prominence_catalogue` keys (disjoint from Slice A's `affect`/`character`/`brightness_to_tempo_bpm`).

### 1.1 Prominence types (composition.rs)

Add next to `OrchestrationProfile` / `FigurationSpec` (composition.rs ~358–420 region). `LayerRole` already exists at composition.rs:343 — reuse it, do NOT define a new role enum.

```rust
/// One layer's resolved prominence weight for a section — the saliency "who is foreground"
/// signal. `role` reuses the EXISTING planner layer vocabulary (composition.rs:343); the
/// realizer bridges it to OrchestralRole via the existing `to_orchestral_role`. NEW S23.
#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize)]
pub struct LayerProminence {
    /// Which layer this weight applies to. serde rejects an unknown LayerRole name; the
    /// §2.6(d) JSON strings "Melody"/"CounterMelody"/"HarmonicFill"/"Pad"/"Bass" parse 1:1
    /// (LayerRole is `#[serde(rename_all = "PascalCase")]`, composition.rs:344).
    pub role: LayerRole,
    /// 0..1 prominence; 0.5 == neutral (every nudge is a no-op at exactly 0.5). 1.0 ==
    /// fully foreground (Melody louder/higher/freer); 0.0 == fully recessive.
    pub weight: f32,
}

/// One named prominence profile — pure structure. Selected by the `prominence` SelectTable;
/// the planner copies its `layers` onto the section's `OrchestrationProfile.prominence`.
/// Adding a profile is a JSON row, NOT a Rust edit (the FigurationSpec / FormSpec discipline).
/// NEW S23.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct ProminenceProfile {
    pub id: String,
    pub layers: Vec<LayerProminence>,
}
```

Add ONE additive `#[serde(skip)]` field to `OrchestrationProfile` (composition.rs:358–384), exactly mirroring the `figuration_resolved` precedent at composition.rs:382–383:

```rust
pub struct OrchestrationProfile {
    // …id, layers, density, pad_voices, figuration, figuration_resolved UNCHANGED…
    /// NEW S23 — the RESOLVED per-layer prominence for this section, filled by the planner
    /// from the `prominence` SelectTable (§1.3). NOT loaded from JSON (`#[serde(skip)]` →
    /// always empty at deserialize). EMPTY == the uniform/identity sentinel: the realizer
    /// takes its byte-stable legacy path. The realizer reads THIS.
    #[serde(skip)]
    pub prominence: Vec<LayerProminence>,
}
```

In `OrchestrationProfile::identity()` (composition.rs:395–404) add `prominence: Vec::new()` to the literal (alongside `figuration_resolved: None`). **`is_identity()` (composition.rs:408–410) is UNCHANGED** — an empty `prominence` is implied by identity; a non-empty `prominence` only ever rides a composed (non-identity) profile.

The finder, mirroring `lookup_figuration`/`lookup_orchestration`:

```rust
fn lookup_prominence<'a>(catalogue: &'a [ProminenceProfile], id: &str) -> Option<&'a ProminenceProfile>;
```

### 1.2 PlanMappings fields (composition.rs:621–654)

Add after `affect` (composition.rs:653), both `#[serde(default)]`:

```rust
pub struct PlanMappings {
    // …form, character, meter, key_scheme, theme_behaviour, texture, form_catalogue,
    //   texture_catalogue, figuration_catalogue, affect UNCHANGED…
    /// NEW S23 — selects a `prominence_catalogue` id from the saliency knobs (§1.3).
    /// `#[serde(default)]` empty SelectTable → "" → uniform (byte-stable legacy realization).
    #[serde(default)]
    pub prominence: SelectTable,
    /// NEW S23 — the prominence-profile vocabulary (id → per-layer weights). Parallel to
    /// `texture_catalogue`/`figuration_catalogue`. `#[serde(default)]` empty Vec.
    #[serde(default)]
    pub prominence_catalogue: Vec<ProminenceProfile>,
}
```

### 1.3 The planner resolve block (composition.rs, immediately after the figuration resolve)

Model EXACTLY on the `figuration_resolved` resolve precedent at **composition.rs:947–953**. Insert directly after that block (after line 953), still operating on the `orchestration` local before the section loop builds sections from it:

```rust
// S23: resolve saliency → prominence ONCE per plan, immediately after the figuration
// resolve (composition.rs:947–953). The `prominence` SelectTable picks a catalogue id from
// the saliency knobs (subject_size, fg_bg_contrast); an absent/unmatched/`uniform` id leaves
// `prominence` empty → the realizer takes its byte-stable uniform path.
let prom_id = self.plan_mappings.prominence.select(u);
orchestration.prominence =
    lookup_prominence(&self.plan_mappings.prominence_catalogue, &prom_id)
        .map(|p| p.layers.clone())
        .unwrap_or_default();
```

`orchestration.clone()` at composition.rs:824 (the per-section clone) already deep-clones the new `prominence` Vec onto each `Section`. No section-loop edit is needed — the resolve happens once on the shared `orchestration` local before the loop, exactly as figuration does.

### 1.4 mapping_loader.rs mirror (mapping_loader.rs:109–143 + the From impl at composition.rs:656–674)

Add to `CompositionMappings` (mapping_loader.rs, after `affect` at :141–142), both `#[serde(default)]`:

```rust
    /// S23 — the prominence SelectTable. `#[serde(default)]` back-compat floor: absent →
    /// empty → planner falls to uniform (byte-stable). Carried onto PlanMappings by the
    /// From<CompositionMappings> impl in composition.rs.
    #[serde(default)]
    pub prominence: crate::composition::SelectTable,
    /// S23 — the prominence-profile vocabulary. `#[serde(default)]` empty Vec.
    #[serde(default)]
    pub prominence_catalogue: Vec<crate::composition::ProminenceProfile>,
```

Add the two carry lines to `From<CompositionMappings> for PlanMappings` (composition.rs:660–673), after `affect: c.affect,` (line 671):

```rust
            prominence: c.prominence,
            prominence_catalogue: c.prominence_catalogue,
```

The `#[serde(skip)]` resolved `OrchestrationProfile.prominence` field is **NOT** in `CompositionMappings` (it is planner-filled, never deserialized) — same discipline as `figuration_resolved` and the affect sentinels.

### 1.5 mappings.json rows (§2.6(d) verbatim — disjoint keys from Slice A)

Add inside the `composition` block (alongside `texture`/`texture_catalogue`/`affect`; do NOT touch any Slice A key):

```jsonc
"prominence_catalogue": [
  { "id": "uniform",        "layers": [] },
  { "id": "subject_melody", "layers": [
      { "role": "Melody",        "weight": 1.0 },
      { "role": "CounterMelody", "weight": 0.6 },
      { "role": "HarmonicFill",  "weight": 0.4 },
      { "role": "Pad",           "weight": 0.3 },
      { "role": "Bass",          "weight": 0.5 } ] }
],
"prominence": {
  "default": "uniform",
  "rules": [
    { "when": [ {"knob":"subject_size",  "op":"in_range","lo":0.05,"hi":0.55},
                {"knob":"fg_bg_contrast","op":"ge","lo":0.25,"hi":0.0} ],
      "pick": "subject_melody" }
  ]
}
```

A real, distinct subject (small-to-mid area, high fg/bg contrast) → `subject_melody`; a uniform/subjectless field (e.g. `example.jpg`, `fg_bg_contrast≈0.15`) → `uniform` (empty) → byte-stable. The `subject_size in_range 0.05–0.55` upper bound excludes "subject fills the frame" (no figure-ground), the lower bound excludes "no subject." `subject_size`/`fg_bg_contrast` are already in the `Knob` enum and on `ImageUnderstanding`.

---

## 2. MUSIC THEORY SPECIALIST WORK-ORDER (chord_engine.rs)

Owns `src/chord_engine.rs` ONLY — the realizer nudges + the SPAN const magnitudes. File-disjoint from the Implementer.

### 2.0 The helper + the threading route (no realize_step signature change)

Add a pure helper near `instrument_role`/`to_orchestral_role` (chord_engine.rs ~887–902). It reuses the existing `to_orchestral_role` bridge so the planner's `LayerRole` matches the realizer's `OrchestralRole`:

```rust
/// New consts — magnitudes are this slice's to finalize by ear (seeds below). The
/// NEUTRAL is load-bearing: it is the value returned under identity, where every nudge
/// becomes (0.5-0.5)*SPAN == 0.0 exactly.
const PROMINENCE_NEUTRAL: f32 = 0.5;
const PROMINENCE_VEL_SPAN: f32 = 16.0;  // SEED (§2.4)
const PROMINENCE_REG_SPAN: f32 = 4.0;   // SEED, in semitones (§2.4)

/// Prominence weight (0..1) for `role`, read off `ctx.section.orchestration.prominence`.
/// Returns PROMINENCE_NEUTRAL (0.5) when the section's prominence is EMPTY (identity/uniform)
/// OR the role is unlisted — so the legacy realization is byte-identical when prominence is
/// absent. Pure. Bridges LayerRole→OrchestralRole via the existing `to_orchestral_role`.
fn prominence_weight(ctx: &crate::composition::StepContext, role: OrchestralRole) -> f32 {
    let prom = &ctx.section.orchestration.prominence;
    if prom.is_empty() {
        return PROMINENCE_NEUTRAL;
    }
    prom.iter()
        .find(|lp| to_orchestral_role(lp.role) == role)
        .map(|lp| lp.weight)
        .unwrap_or(PROMINENCE_NEUTRAL)
}
```

**Threading route (the already-blessed additive-private-param precedent — the `pad_voices`/`ctx` pattern at chord_engine.rs:968,1029,1271).** In `realize_step` (chord_engine.rs:957–1032), `ctx` and `role` are both in scope. Compute the weight once after `role` is known and pass it down as an additive private param to the three private free fns. `realize_step`'s public signature is UNCHANGED.

- `realize_velocity` (chord_engine.rs:1123–1128) — add a private param `prominence_w: f32`; the call site is chord_engine.rs:1012. Pass `prominence_weight(ctx, role)`.
- `role_pitch` (chord_engine.rs:1045–1051) — add a private param `prominence_w: f32`; the call site is chord_engine.rs:1000 (and the second call at chord_engine.rs:2259, the theme path — pass the same weight there). Pass `prominence_weight(ctx, role)`.
- `realize_rhythm` (chord_engine.rs:1259–1277) — already takes `ctx`; it can call `prominence_weight(ctx, role)` itself (it has `role` and `ctx` already), OR take an additive `prominence_w` param. Prefer **computing it inside `realize_rhythm` from the already-borrowed `ctx`** (no new param), to minimize the call-site change at chord_engine.rs:1020–1031.

Net path: `engine_equivalence` builds `OrchestrationProfile::identity()` → empty `prominence` → `prominence_weight` returns `0.5` for every role → all three nudges are `0.0` exactly.

### 2.1 Velocity nudge (centered, 0 at w==0.5)

**Insertion point:** in `realize_velocity`, immediately after the per-role bias `match` block (chord_engine.rs:1177–1187) and BEFORE the final `vel.round().clamp(1.0, 127.0)` at chord_engine.rs:1189:

```rust
    // S23 prominence: centered velocity nudge. Exactly 0 at w==0.5 (identity); a foreground
    // role (w>0.5) gets louder, a recessive role (w<0.5) quieter. Cadence-exempt is implicit:
    // the existing contour additions above are already gated on !is_cadence where they matter,
    // and the clamp keeps it in band. Adding it here (before the clamp) lets it ride on top of
    // the existing +2 Melody / -3 Pad biases — saliency WIDENS the gap the realizer already has.
    vel += (prominence_w - 0.5) * PROMINENCE_VEL_SPAN;
```

(`vel.round().clamp(1.0, 127.0)` already follows, satisfying "clamp 1..=127".) At `w==0.5`, `(0.5-0.5)*SPAN == 0.0`. **Cadence note:** to keep the cadence golden byte-stable, guard the nudge on `!is_cadence` (the cadence step velocities 114/84 are goldens). Recommended: `if !is_cadence { vel += (prominence_w - 0.5) * PROMINENCE_VEL_SPAN; }`. Under identity the guard is moot (term is 0 anyway), but it documents the cadence exemption and is defensive against future non-0.5 cadence weights.

### 2.2 Register nudge (centered, 0 at w==0.5; BASS EXEMPT; Risk-1 sum-clamp)

**Insertion point:** in `role_pitch` (chord_engine.rs:1045–1110), fold the centered term into the `bright_octaves`-derived `lift`. Two arms only — Melody (chord_engine.rs:1073–1081) and the fill group (chord_engine.rs:1089–1108). **The Bass arm (chord_engine.rs:1064–1072) is EXEMPT** — it must stay the harmonic floor; do NOT add any upward prominence lift to Bass (consistent with `bright_octaves` already exempting Bass from upward shift).

Melody arm (chord_engine.rs:1078–1079) becomes:

```rust
            let lift = (bright_octaves * 12.0).round() as i16;
            // S23 prominence: a foreground melody (w>0.5) lifts UP; recessive never lowers the
            // bed. Risk-1 (design-s21 §5): clamp the SUM of (brightness lift + prominence lift),
            // never each independently, and lift the foreground only.
            let prom_lift = ((prominence_w - 0.5) * PROMINENCE_REG_SPAN).round() as i16;
            let floor = (MELODY_REGISTER_FLOOR as i16 + lift + prom_lift).clamp(24, 96) as u8;
```

Fill group arm (chord_engine.rs:1105–1106) — Pad/HarmonicFill/CounterMelody. Per Risk 1 ("lift the melody, NEVER lower the bed"), a recessive bed (`w<0.5`) must NOT be pushed below its current floor. Use `max(0, prom_lift)` so prominence only ever RAISES (it never deepens the bed into the bass), and for a recessive layer it is a no-op:

```rust
            let lift = ((bright_octaves * 6.0).round() as i16).clamp(-12, 12);
            // S23 prominence: a recessive bed (w<0.5) is NEVER lowered (Risk-1); only a
            // foreground (w>0.5) could rise. Clamp the prominence lift at >=0 for the bed.
            let prom_lift = (((prominence_w - 0.5) * PROMINENCE_REG_SPAN).round() as i16).max(0);
            let floor = (FILL_REGISTER_FLOOR as i16 + lift + prom_lift).clamp(24, 96) as u8;
```

**Risk-1 sum-clamp (design-s21 §5, the invariant most at risk):** the three upward forces — a Scherzo bright register, a bright image's `bright_octaves` lift, and the saliency `+` foreground lift — must clamp as a SUM, not independently. Above, both arms add `lift + prom_lift` then apply a SINGLE `.clamp(24, 96)` (the existing melody/fill clamp). That single clamp on the summed lift IS the sum-clamp. The melody floor of 67 (G4) + max `lift` 12 + max seed `prom_lift` 2 (`PROMINENCE_REG_SPAN=4` ⇒ `0.5*4=2`) = 81, well under 96, so the melody retains range — `no_inversion_invariant` (§4 test 4) cannot break at the seed magnitude. **Keep `PROMINENCE_REG_SPAN` small** (≤6 semitones span; `0.5*6=3` max lift) precisely so the melody never clamps flat at the top of 24..=108.

### 2.3 Rhythm nudge (centered, 0 at w==0.5; shift Melody/CounterMelody edge_activity band thresholds)

**Insertion point:** in `realize_rhythm`, the Melody arm (chord_engine.rs:1589–1632). The exact thresholds to shift are at chord_engine.rs:1595 (`edge_activity > 0.80` → arpeggio), :1608 (`edge_activity > 0.55` → syncopated), :1617 (`edge_activity > 0.25` → dotted). Lowering these thresholds for a foreground role (w>0.5) makes the melody subdivide more readily ("rhythmically freer"); raising them for a recessive role keeps it plainer.

Compute the centered shift once at the top of the Melody arm (and the CounterMelody arm if it has its own bands — currently CounterMelody delegates to the HarmonicFill figure, chord_engine.rs:814–818, so it has no independent band; apply the shift to the Melody arm and, when the CounterMelody arm gains real bands in a later slice, the same term applies):

```rust
        OrchestralRole::Melody => {
            // S23 prominence: shift the band cutoffs by a centered term. w>0.5 lowers the
            // cutoffs (the foreground melody subdivides more readily — rhythmically freer);
            // w==0.5 → shift 0.0 (byte-identical); w<0.5 raises them (plainer). Magnitude
            // PROMINENCE_RHY_SHIFT is this slice's to finalize (seed 0.10).
            let w = prominence_weight(ctx, role);            // or the threaded prominence_w
            let shift = (w - 0.5) * PROMINENCE_RHY_SHIFT;     // +shift LOWERS the effective cutoff
            if pre_cadence || edge_activity > (0.80 - shift) {
                // …arpeggio (chord_engine.rs:1595–1607) UNCHANGED body…
            } else if edge_activity > (0.55 - shift) {
                // …syncopated (chord_engine.rs:1608–1616) UNCHANGED body…
            } else if edge_activity > (0.25 - shift) {
                // …dotted (chord_engine.rs:1617–1625) UNCHANGED body…
            } else {
                // …sustained (chord_engine.rs:1626+) UNCHANGED body…
            }
        }
```

Add the seed const `const PROMINENCE_RHY_SHIFT: f32 = 0.10;` near the other prominence consts. At `w==0.5`, `shift == 0.0` and the cutoffs are exactly `0.80 / 0.55 / 0.25` — byte-identical. **Do NOT shift the cadence/pre_cadence branch** (`pre_cadence ||` is kept as the first disjunct unchanged, so the cadence acceleration path is untouched — protects the cadence golden).

### 2.4 SPAN const seed magnitudes (this slice finalizes by ear)

| Const | Seed | Principled rationale |
|---|---|---|
| `PROMINENCE_NEUTRAL` | `0.5` | FIXED, not tunable. The freeze pivot — every nudge is `(0.5-0.5)*SPAN == 0.0` here. |
| `PROMINENCE_VEL_SPAN` | `16.0` | At max prominence (w=1.0) the foreground gains `0.5*16 = +8` velocity over neutral and a fully recessive layer (w=0.3 for Pad) loses `0.2*16 ≈ -3.2`, so the Melody-vs-Pad gap widens by ~11 over the existing +2/-3 baseline — audible but non-clipping (96-floor cadences aside, typical mid-velocities ~60–90 stay in band after the 1..=127 clamp). Matches the music-craft design's "+2 + round(prom*6) ≈ up to +8" intent (design-s21-musical-craft §B.4.1). |
| `PROMINENCE_REG_SPAN` | `4.0` | semitones of span. Max foreground lift `0.5*4 = +2` semitones. Deliberately small so MELODY_FLOOR(67)+bright_lift(≤12)+prom_lift(≤2)=81 ≪ 96 — the melody never clamps flat at the top of 24..=108, so `no_inversion_invariant` cannot break. (The music-craft design floated +5; the sum-clamp risk argues for a smaller seed — the ear can raise it toward 5–6 max if range holds.) |
| `PROMINENCE_RHY_SHIFT` | `0.10` | At max prominence the melody arpeggio cutoff drops 0.80→0.75 and the dotted cutoff 0.25→0.20 — a modest, audible bias toward subdivision without collapsing the bands. |

These four are the Music Theory Specialist's to finalize by ear; the byte-freeze holds for ANY magnitude (the freeze depends only on `(0.5-0.5)*SPAN == 0.0`), so re-tuning is risk-free against the goldens. The only ear-gated constraint is keeping `PROMINENCE_REG_SPAN` small enough that `no_inversion_invariant` holds at max stacked lift.

---

## 3. THE BYTE-FREEZE PROOF (the earned one — verified against the real code)

**Claim:** under identity, every emitted `NoteEvent` is bit-for-bit today's, independent of SPAN magnitudes.

1. The equivalence net (`tests/engine_equivalence.rs`) hand-builds a `Section` carrying `OrchestrationProfile::identity()` (verified: identity sets `prominence: Vec::new()` per §1.1) and drives `decide_instrument_action`/`realize_step` directly — it never runs the compose path (`CompositionPlanner::plan`), so the §1.3 resolve block is OFF the net entirely.
2. Under identity, `OrchestrationProfile.prominence` is EMPTY. `prominence_weight(ctx, role)` (§2.0) short-circuits on `prom.is_empty()` and returns `PROMINENCE_NEUTRAL = 0.5` for EVERY role.
3. Velocity (§2.1): `vel += (0.5-0.5)*PROMINENCE_VEL_SPAN == vel += 0.0`. `round(x + 0.0) == round(x)`. Cadence velocities 114/84 (additionally `!is_cadence`-guarded) untouched.
4. Register (§2.2): both Melody and fill arms add `prom_lift = round((0.5-0.5)*PROMINENCE_REG_SPAN) == 0`; `floor + lift + 0 == floor + lift`. Bass arm has NO prominence term at all (exempt). Goldens 36 (Bass) / 79 (Melody) untouched.
5. Rhythm (§2.3): `shift = (0.5-0.5)*PROMINENCE_RHY_SHIFT == 0.0`; cutoffs are exactly `0.80 / 0.55 / 0.25`; the `pre_cadence ||` disjunct and the cadence early-return are untouched, so the 240 ms cadence hold is byte-stable.
6. The prominence arms are reachable ONLY through a composed `subject_melody` profile (a non-identity profile with a non-empty `prominence`), which requires the compose path's resolve block AND a `Pad`/`CounterMelody` role. Under identity `assign_role` delegates to `instrument_role` (chord_engine.rs:874–887), which returns only `Bass`/`HarmonicFill`/`Melody` — and even those get weight `0.5` since `prominence` is empty. The equivalence net never constructs a non-identity profile.

**Verified against the actual code paths:** the three insertion points (1189-adjacent, 1078–1079 + 1105–1106, 1595/1608/1617) each evaluate their new term to exactly `0.0` at `w==0.5`, `round()` absorbs `+0.0`, the single `.clamp(24,96)` on the summed register lift is unchanged when `prom_lift==0`, and the cadence ring is never on a prominence path. The freeze holds for all SPAN magnitudes. **Witness:** `git diff HEAD -- src/engine.rs tests/engine_equivalence.rs` MUST be EMPTY; `sha256(src/engine.rs)` MUST equal `7a07fb8…43c23`.

---

## 4. THE 5 PROPERTY TESTS (Test Engineer — build with no questions)

All headless/synth-independent: build `ImageUnderstanding`/`Section`/`StepPlan` literals, drive `realize_step`/`realize_rhythm`, never the synth. To exercise a non-uniform profile, build an `OrchestrationProfile` with an explicit non-empty `prominence` Vec (the planner resolve is bypassed in-test, same as the figuration tests construct `figuration_resolved` directly).

1. **`prominence_neutral_is_byte_identical`** — realize Melody/Bass/cadence steps under (a) `OrchestrationProfile::identity()` (empty prominence) and (b) the SAME profile but with an explicit `prominence` listing EVERY role at `weight: 0.5`. Assert the two `Vec<NoteEvent>` are EQUAL. (Proves the centered-nudge-zero property directly — every nudge is `(0.5-0.5)*SPAN == 0.0` for both empty and all-0.5.)

2. **`high_saliency_melody_louder`** — a strong-subject profile (`subject_melody`: Melody 1.0, Pad 0.3). On a fixed chord/StepPlan, `mean_vel(Melody) - mean_vel(Pad)` MUST exceed the same gap under all-0.5 (`baseline_gap`). I.e. `gap_high > baseline_gap + EPS`. (The melody gains `+0.5*VEL_SPAN`, the Pad loses `+0.2*VEL_SPAN`, widening the existing +2/-3.)

3. **`high_saliency_melody_higher_wider`** — same `subject_melody` profile. `mean_pitch(Melody) > mean_pitch(Pad)`, AND the Melody-Pad pitch separation at high prominence (Melody 1.0) is strictly GREATER than at neutral (Melody 0.5). (Encodes the register lift; the Pad bed is never lowered, so the separation grows by the melody rising.)

4. **`no_inversion_invariant`** (the HARD guard, Risk 1 / §C.3) — sweep ALL prominence values (Melody weight across `{0.0, 0.25, 0.5, 0.75, 1.0}`) × ALL characters (Ballad…Scherzo, including Scherzo's bright register + a bright-image `bright_octaves` lift, so the lifts STACK). For every step assert: `mean_pitch(Bass) < mean_pitch(any HarmonicFill/Pad bed voice) < mean_pitch(Melody)` AND every emitted note ∈ `24..=108`. Also assert the Melody retains measurable range (max-min pitch > 0) at the maximum stacked lift (it must not clamp flat at the top). This is the test the register seed magnitude (`PROMINENCE_REG_SPAN`) is tuned against.

5. **Freeze diff** — `git diff HEAD -- src/engine.rs tests/engine_equivalence.rs` is EMPTY, and `sha256(src/engine.rs) == 7a07fb8568fcd94536dd3ec2e4dc71c8154f9d9678599a6106e3a542a4343c23`. (Plus the standing `engine_equivalence` 9/9 green and a `prominence_round_trips_from_json` mirror witness recommended — see §1.4 — to confirm the loader mirror is wired and the data is not silently dropped at load.)

---

## 5. FILE-DISJOINTNESS + LOCKED-OFF TABLE

| Owner | Files / surface | Notes |
|---|---|---|
| **Rust Implementer** | `src/composition.rs` (the prominence types §1.1, PlanMappings fields §1.2, resolve block §1.3, `identity()` literal, `lookup_prominence`), `src/mapping_loader.rs` (CompositionMappings mirror §1.4), `assets/mappings.json` (`prominence` + `prominence_catalogue` §1.5) | Single-writer of the `prominence`/`prominence_catalogue` JSON keys — DISJOINT from Slice A's `affect`/`character`/`brightness_to_tempo_bpm`/`character_tempo`. The `From` impl two carry lines (composition.rs:671-adjacent) are the Implementer's. |
| **Music Theory Specialist** | `src/chord_engine.rs` ONLY (the `prominence_weight` helper §2.0, the three nudges §2.1/§2.2/§2.3, the four new consts §2.4) | Reuses the existing `to_orchestral_role` bridge. Finalizes the SPAN seed magnitudes by ear; the freeze holds for any magnitude. |
| **Test Engineer** | the 5 tests of §4 (new test module / `tests/` file, NOT `tests/engine_equivalence.rs`) | Constructs non-uniform profiles in-test; never edits the frozen net. |
| **LOCKED OFF — everyone** | `src/engine.rs` (sha256 `7a07fb8…43c23` freeze witness), `tests/engine_equivalence.rs` (goldens 240/114/84/36/79 UNMOVED), `src/midi_output.rs`, `src/synth_sink.rs`, `src/cli.rs`, `src/tui.rs`, `main.rs`, `src/modem.rs`, `src/bin/*` | `realize_step` PUBLIC signature FROZEN (additive-private-param route only). `is_identity()` UNCHANGED. The cadence ring + structural velocity floor untouched. |

**Collision-freedom:** the Implementer touches composition.rs/mapping_loader.rs/mappings.json; the Music Theory Specialist touches chord_engine.rs only; the Test Engineer touches a new test file only. No shared file. The single cross-module contract is the `LayerRole`/`to_orchestral_role` bridge (both already in tree) and the `OrchestrationProfile.prominence` field shape (Implementer defines, Music Theory reads via `ctx.section.orchestration.prominence`). Build order: Implementer ∥ Music Theory (disjoint) → Test Engineer → Quality Gate LAST.

*Buildable spec only. No source, test, or asset modified by this document. Anchors are file:line against HEAD 89087f9.*

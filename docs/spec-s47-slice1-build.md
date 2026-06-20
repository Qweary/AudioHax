# S47 Slice 1 — The Buildable Per-File Spec: THE FIGURE-GROUND HIERARCHY

**Author role:** Rust Architect (DESIGN ONLY — no source/test/asset modified by this document; `docs/` only).
**Date:** 2026-06-19.
**Status:** LOCKED for build. This is the single contract the two implementers (Music Theory Specialist for `chord_engine.rs` + `mappings.json` musical content; Test Engineer for the scorecard) build against. The operator's seven confirmed decisions (§0) are settled — this doc encodes them; it does not re-litigate them.

**Synthesizes** the S46 design cadence into the S46 work-order's Slice 1 (`docs/design-s46-figure-ground.md` §5): `design-s46-architect.md` (the `ActivityClass`/seat-guard seam), `design-s46-theory.md` (the role-ordering law Bass < bed/fill < Counter < Melody in ACTIVITY and REGISTER), `design-s46-affect.md` (the cue-strength ranking + magnitude leans), `design-s46-aesthetics.md` (the recession-tier magnitudes + MIN_FIGURE_GAP lean), and `spec-s46-figure-ground-metrics.md` (the F1–F5 the scorecard reads). Mirrors `docs/spec-s15-slice1-build.md` in shape.

**Grounded against the live working tree at HEAD — every file:line below was re-read THIS session against the live file (the S46 doc line numbers were verified, not trusted blindly; where the live file had drifted from a lens's cite it is noted):**
`src/chord_engine.rs` (Melody seat + `.clamp(24,96)` `:1258-1272`; Melody rhythm arm 4-band ladder `:1927-1993`, SUSTAINED arm `:1974-1992`, `prom_shift` `:1941-1942`; CounterMelody arm `:1831-1924`, the governor predicate `held_chord || melody_static` `:1899`, OBLIQUE arm `:1908-1912`, rest-as-gesture arm `:1913-1923`; `realize_rhythm` signature with the `pad_voices` + `ctx` private params `:1494-1512`; `realize_velocity` role-bias arms `:1384-1394`; register floors `:1220-1222`; `COUNTER_CEILING = MELODY_REGISTER_FLOOR` (67) `:3478`; the prominence constants `PROMINENCE_NEUTRAL=0.5` `:985`, `PROMINENCE_VEL_SPAN=18` `:995`, `PROMINENCE_REG_SPAN=4` `:1005`, `PROMINENCE_RHY_SHIFT=0.10` `:1015`; `prominence_weight(ctx, role)` reader `:1018-1031`; `melody_pitch_for` `:3577`),
`assets/mappings.json` (`prominence_catalogue` + `melody_forward` default + the `prominence` SelectTable `:365-387`; `pad_bed_counter` orchestration row `:265`; the counter routing gate `:343-346`),
`src/composition.rs` (`ImageUnderstanding.subject_size` `:77` / `.fg_bg_contrast` `:83`; `LayerProminence { role, weight }` `:566-575`; `ProminenceProfile { id, layers }` `:581-585`; `Knob::SubjectSize` `:792` / `Knob::FgBgContrast` `:793`; `Knob::read` arms `:829-830`; `SelectTable`/`SelectRule`/`Predicate`/`CmpOp` carry-forward from S15),
`tests/engine_equivalence.rs` (`G_MELODY_NOTE=79` `:138`, `G_BASS_NOTE` 36, the cadence goldens velocity 114/84 `:277/:293`, cadence hold 240 ms `:281`),
`tests/variety_scorecard_s45.rs` (`ENGINE_SHA256` guard `:68`; the F1–F5 land via `scorecard_for`/`LayerVerdicts` per `spec-s46-figure-ground-metrics.md`).

**THE FREEZE (binding):** `src/engine.rs` is BYTE-FROZEN at sha256 `e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261` (re-verified UNCHANGED this session via `sha256sum src/engine.rs`). **NOTHING in this slice touches `engine.rs`.** Every lever is in `chord_engine.rs` (realizer the frozen kernel only *calls*) + `assets/mappings.json` (data) + `tests/` (scorecard). Every per-role/per-step term is centered on the identity no-op (no Counter instrument under identity → the governor is never consulted on the freeze path; prominence neutral 0.5 → every `(0.5−0.5)*SPAN` nudge == 0 and the activity floor is a no-op; `serde(default)` new fields == today's behavior; the seat guard sits BELOW the legacy bright/neutral melody seat under identity so it is a no-op).

---

## 0. The operator's confirmed decisions (settled — build to these; do NOT re-litigate)

1. **THREE recession tiers** — deep (subject) / mid / shallow (field).
2. **Counter recession in slice 1 = ACTIVITY GOVERNOR ONLY.** The 0.58→0.55 counter weight trim is DEFERRED to slice 3 (DP-6). Do NOT spec a weight change.
3. **DP-3 inverse-register compensation = DEFERRED to slice 3.** NOT in this slice.
4. **DP-4 per-role timbre = DEFERRED, separate future arc.** NOT in this slice.
5. **Seat-guard `MIN_FIGURE_GAP` = a small POSITIVE margin** (a clear seat above the counter ceiling, not a tie). Magnitude ear-tuned — spec'd as a named const with a recommended start + flagged ear-tunable.
6. **DP-6 counter weight 0.58→0.55 = AFTER slice 1.** NOT this slice.
7. **Hard-fail margin curve `f(fg_bg_contrast)`: the SIGN is load-bearing** — a NEGATIVE margin fails everywhere; a POSITIVE required margin only on subject images. Magnitude ear-tuned to the recession tiers.

**SCOPE — exactly these three coupled changes, nothing more.** (a) [CE] `ActivityClass` ordering + `melody_activity_class` helper → governs the CounterMelody activation at `:1899` (routing into its EXISTING oblique/rest modes) + a prominence-keyed melody activity FLOOR at the Melody arm `:1943-1992`. (b) [CE] a seat-order guard so the realized melody seat > counter ceiling, folded UNDER the existing `.clamp(24,96)` at `:1271`. (c) [JSON] an image-conditioned prominence FAMILY at `mappings.json:365-387` replacing the single `melody_forward` default with deep/mid/shallow tiers via the existing SelectTable.

---

## 1. CURRENT-STATE GROUND TRUTH — the three inversion sites, pinned to CONFIRMED live lines

Re-read this session; the S46 docs were right within ±a few lines. The CONFIRMED current sites:

| # | Site | CONFIRMED live line(s) | What is there today |
|---|---|---|---|
| **(a-gov)** | CounterMelody activation predicate | `chord_engine.rs:1899` (`if held_chord \|\| melody_static`) inside the arm `:1831-1924` | When the chord is held OR the melody is static, the counter takes its **MOVING** branch: a GUARANTEED off-beat onset at `step_ms/4` (`:1900-1907`). The OBLIQUE branch (one sustained tone, `edge_activity > 0.55`) is `:1908-1912`; the rest-as-gesture / one-sustained-tone branch is `:1913-1923`. So on a calm image the counter MOVES exactly when the melody holds — the inversion. |
| **(a-floor)** | Melody rhythm 4-band ladder | `chord_engine.rs:1943` (`if pre_cadence \|\| edge_activity > (0.80 - prom_shift)`), `:1956` (`> 0.55 - prom_shift`), `:1965` (`> 0.25 - prom_shift`), SUSTAINED `else` arm `:1974-1992`; `prom_shift` computed `:1941-1942` | On low `edge_activity` the melody falls to the SUSTAINED arm — ONE long tone, offset 0. `prom_shift = (prominence_weight(ctx,role) − 0.5) * PROMINENCE_RHY_SHIFT(0.10)` only nudges the melody's OWN cutoffs (≤0.05 at full weight) — it cannot lift a calm melody out of SUSTAINED. |
| **(b)** | Melody seat / register | `chord_engine.rs:1258-1272`; the single `.clamp(24, 96)` is `:1271` | `floor = (MELODY_REGISTER_FLOOR(67) + lift + prom_lift).clamp(24,96)`, where `lift = (bright_octaves*12).round()` ∈ [−12,+12] (`:1263`) and `prom_lift = ((w−0.5)*PROMINENCE_REG_SPAN(4)).round()` ≤ +2 (`:1269-1270`). On a dark image `lift` can be −12 → floor ≈ 55-57, INSIDE the counter band. `COUNTER_CEILING = MELODY_REGISTER_FLOOR = 67` (`:3478`); counter band `[FILL_REGISTER_FLOOR(55), 67)`. **No `melody_seat > counter_ceiling` invariant anywhere.** |
| **(c)** | Prominence family | `mappings.json:365-387` — `prominence_catalogue` (`uniform`, `subject_melody`, `melody_forward`) + the `prominence` SelectTable (`default: "melody_forward"`, one `subject_melody` rule) | One `melody_forward` default (Melody 0.78 / CounterMelody 0.58 / HarmonicFill 0.40 / Pad 0.40 / Bass 0.50) for nearly every image; the realizer consumes resolved weights at `:1271` (register), `:1404` (velocity), `:1942` (melody rhythm shift). The SelectTable already routes by `subject_size`/`fg_bg_contrast` (`Knob::SubjectSize :792`, `Knob::FgBgContrast :793`). |

**Supporting seam facts (confirmed, load-bearing for the build):**
- `realize_rhythm` (`:1494-1512`) already carries TWO private additive params — `pad_voices: u8` and `ctx: &composition::StepContext` — set by `realize_step`; `realize_step`'s public signature is UNCHANGED. The governor's inputs (`edge_activity`, `prom_shift`, `pre_cadence`, and the melody-vs-counter relationship) are all already in scope in `realize_rhythm`. **No public-seam change is needed for any part of this slice.**
- `prominence_weight(ctx, role)` (`:1018-1031`) returns `PROMINENCE_NEUTRAL(0.5)` on an empty/absent prominence vec — the identity no-op anchor for both the floor and the family.
- The counter arm (`:1831-1924`) is unreachable under identity (no Counter instrument; `pad_voices == 0`, empty `layers`), so anything keyed off the counter is byte-neutral on the freeze path.

---

## 2. THE EXACT EDIT SITES — types, signatures, decision tables (NO bodies)

All NEW Rust lives in `src/chord_engine.rs` (Music Theory Specialist owns). NO new module. NO `realize_step`/`role_pitch` public-signature change. All JSON lives in `assets/mappings.json` (Music Theory Specialist authors weights + routing; the schema is additive `serde(default)`-clean).

### 2(a) — THE ACTIVITY HIERARCHY: `ActivityClass` + `melody_activity_class` + the governor + the floor

#### 2(a).1 The ordering type (NEW; `chord_engine.rs`, near the prominence constants `:985-1015`)

```rust
/// The realized rhythmic-activity CLASS of a voice on a step — coarse, RNG-free, and
/// structural (a function of `edge_activity` + the arm's band, never of absolute pitch).
/// Ordering IS the figure-ground rank: Sustained < Oblique < Subdividing. The Counter arm
/// reads the MELODY's class to stay strictly BELOW it on a HOLDING melody (the hierarchy
/// invariant), so the background never out-moves the foreground. Under identity there is no
/// Counter instrument and prominence is neutral, so this is never consulted on the freeze
/// path → byte-neutral. `Ord` is derived so the governor can compare classes directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ActivityClass { Sustained, Oblique, Subdividing }
```

#### 2(a).2 The pure helper (NEW; `chord_engine.rs`, free fn beside `realize_rhythm`)

```rust
/// The melody's activity class for THIS step, derived from the SAME 4-band ladder the Melody
/// arm uses (`chord_engine.rs:1943-1992`) — extracted so the Counter arm can govern off it
/// WITHOUT duplicating the cutoff logic. Pure; reads only `edge_activity`, the prominence
/// rhythm shift `prom_shift`, and `pre_cadence`. Returns:
///   - `Subdividing` when `pre_cadence || edge_activity > (0.80 - prom_shift)` (the ARPEGGIO
///     band, :1943) OR `edge_activity > (0.55 - prom_shift)` (the SYNCOPATED band, :1956);
///   - `Oblique` when `edge_activity > (0.25 - prom_shift)` (the DOTTED band, :1965);
///   - `Sustained` otherwise (the SUSTAINED arm, :1974).
/// The cutoffs MUST stay 1:1 with the live Melody-arm cutoffs (any future cutoff change must
/// move both — flagged in §8). The `prom_shift` argument is the SAME value the Melody arm
/// computes at :1941-1942 (`(prominence_weight(ctx, role) - PROMINENCE_NEUTRAL) *
/// PROMINENCE_RHY_SHIFT`), passed in so the helper is pure (no `ctx` read).
fn melody_activity_class(edge_activity: f32, prom_shift: f32, pre_cadence: bool) -> ActivityClass;
```

> **Mapping note (DOTTED→Oblique, SYNCOPATED→Subdividing).** The Melody arm has FOUR bands (arpeggio / syncopated / dotted / sustained) but the figure-ground rank has THREE classes. The mapping above is the load-bearing music-theory call (locked per the theory lens §3.2): the DOTTED band (a long-short pair, 2 onsets) is the melody's "Oblique-equivalent" minimum-real-motion; the SYNCOPATED and ARPEGGIO bands are both "Subdividing" (≥2 onsets pushing the meter / spreading the beat). The Music Theory Specialist confirms this mapping at build (it is the only interpretive degree of freedom in the helper).

#### 2(a).3 THE GOVERNOR — rewrite of the counter activation predicate (`chord_engine.rs:1899`)

The counter arm's three existing rhythm branches are KEPT; only the *selection* between them is governed by the melody's class. The decision table the producer codes to (the melody class is computed once via `melody_activity_class` with the counter's own in-scope `edge_activity`/`prom_shift`/`pre_cadence` — NOTE the prominence weight read for the counter's `prom_shift` is the MELODY role's weight, since the governor asks "how active is the melody," not the counter; the producer reads `prominence_weight(ctx, OrchestralRole::Melody)` for this one computation):

| Melody `ActivityClass` this step | Counter routes into | Existing branch reused | Why |
|---|---|---|---|
| **Subdividing** (melody moves — S45's active passage) | **MOVING** — the guaranteed off-beat onset at `step_ms/4` | the current `:1900-1907` body | The melody is the busiest line; the counter may keep its moving inner texture (PRESERVE S45 — when the melody moves, the counter moves). |
| **Oblique** (melody dotted — minimal real motion) | **OBLIQUE** — one sustained tone, onset 0 | the current `:1908-1912` body | The melody has ≥2 onsets (dotted); the counter recedes one rank below to a single sustained tone so it stays under the figure. |
| **Sustained** (melody holds — the calm-image inversion case) | **OBLIQUE or REST-as-gesture** (one rank below Sustained-with-a-guaranteed-onset, i.e. the counter does NOT take the guaranteed off-beat onset) | the current `:1908-1923` bodies (the OBLIQUE arm, falling to rest-as-gesture on a weak interior beat per the existing `FILL_REST_ACTIVITY` gate at `:1919`) | **THE FIX:** when the melody holds, the counter MUST NOT get the guaranteed onset that out-moves the held foreground. It recedes to a sustained tone (or the existing breathing-rest), never silenced. |

**The governing condition, stated precisely (the producer writes the body; this is the rule):**

```text
let m_class = melody_activity_class(edge_activity, melody_prom_shift, pre_cadence);
match m_class {
    Subdividing => <existing MOVING branch, :1900-1907>,   // S45 preserved
    Oblique     => <existing OBLIQUE branch, :1908-1912>,
    Sustained   => <existing OBLIQUE/rest branch, :1908-1923 — NEVER the guaranteed onset>,
}
```

The current arm's *outer* `if held_chord || melody_static` (`:1899`) is what fires the guaranteed onset; the governor REPLACES that predicate with the class match above. The key inversion-kill: `melody_static`/`held_chord` no longer routes the counter to MOVING — only a `Subdividing` melody does. **The counter is NEVER silenced** (rest-as-gesture remains gated exactly as today on `edge_activity < FILL_REST_ACTIVITY && weak_interior`, `:1919`); a holding-melody step gives the counter a sustained tone, not a guaranteed onset.

#### 2(a).4 THE MELODY ACTIVITY FLOOR — at the Melody arm SUSTAINED branch (`chord_engine.rs:1943-1992`)

A prominence-keyed FLOOR so a FOREGROUND (>0.5 weight) melody never falls all the way to SUSTAINED on a calm image. The rule (producer writes the body):

```text
// At the Melody arm, BEFORE the existing band ladder selects SUSTAINED:
// if the resolved Melody prominence weight is foreground (> ACTIVITY_FLOOR_THRESHOLD) AND
// the band ladder would otherwise select SUSTAINED (i.e. melody_activity_class == Sustained
// on the un-floored cutoffs), FLOOR the melody to the DOTTED (Oblique-rank, ≥2 onsets) band
// instead — i.e. take the :1965-1973 DOTTED arm rather than the :1974-1992 SUSTAINED arm.
// A neutral-weight (== 0.5) or recessive (< 0.5) melody is UNAFFECTED → byte-identical.
```

- The floor lifts the melody one rank (Sustained → Oblique/dotted), giving it ≥2 onsets so it ALWAYS out-moves the governed counter's single sustained tone on a calm image — closing the activity inversion from the foreground side.
- **Identity neutrality:** the floor only bites when `prominence_weight(ctx, Melody) > ACTIVITY_FLOOR_THRESHOLD`. Under identity the weight is `PROMINENCE_NEUTRAL = 0.5` and the threshold start (§3) is `0.5`, so the strict `>` is FALSE → the SUSTAINED arm runs exactly as today → byte-identical.
- **No new JSON field is required for the floor** (the locked decision is the activity GOVERNOR + floor keyed off the EXISTING resolved prominence weight; the threshold is a single CE const). The S46 architect doc floated an optional `activity_floor: Option<u8>` serde field on `ProminenceProfile` — that is NOT needed for slice 1 and is NOT spec'd here (it would be an over-build; the prominence weight already carries the foreground/recessive signal). See §8 RISKS.

### 2(b) — THE SEAT-ORDER GUARD (`chord_engine.rs:1258-1272`, folded under the `.clamp(24,96)` at `:1271`)

A named const + an additive `.max(...)` folded UNDER the existing single sum-clamp, so the realized melody seat is structurally above the counter ceiling.

```rust
/// The minimum clear margin (in semitones) the melody seat must hold ABOVE the counter
/// ceiling — a POSITIVE gap (operator decision 5: a clear seat, not a tie). High-voice
/// superiority wants the figure unambiguously on top, so the floor is COUNTER_CEILING + this.
/// Ear-tunable (the taste gate sizes it). NEW S47; near MELODY_REGISTER_FLOOR :1222.
const MIN_FIGURE_GAP: u8 = 2; // recommended start; range [1, 5] — see §3 ear-tunable knobs
```

The edit to the Melody seat expression (`:1271`), additive, folded under the same clamp:

```text
// today (:1271):
//   let floor = (MELODY_REGISTER_FLOOR as i16 + lift + prom_lift).clamp(24, 96) as u8;
// S47 — fold a seat-order floor UNDER the existing single sum-clamp, so a dark-image lift
// can never seat the melody into the counter band:
//   let raw = MELODY_REGISTER_FLOOR as i16 + lift + prom_lift;
//   let floor = raw.max(COUNTER_CEILING as i16 + MIN_FIGURE_GAP as i16).clamp(24, 96) as u8;
```

- `COUNTER_CEILING` is already defined (`:3478`, == `MELODY_REGISTER_FLOOR` == 67) and in scope at module level. The guard makes "melody on top" structural: the realized seat floor is never below `67 + MIN_FIGURE_GAP`.
- **Identity neutrality (the WITNESS, §4):** under the identity render the melody is the bright/neutral lone top voice. On the bright/neutral path `bright_octaves ≥ 0` so `lift ≥ 0`, and `prom_lift == 0` at neutral weight, so `raw = 67 + lift + 0 ≥ 67 ≥ 67 + MIN_FIGURE_GAP` is FALSE only if `lift < MIN_FIGURE_GAP`. **This is the one place the guard could perturb the freeze if a low-but-non-negative `lift` exists on the identity path.** The build MUST verify the witness in §4 before landing. The guard's `.max(...)` is a no-op iff `lift ≥ MIN_FIGURE_GAP` on every step of every `engine_equivalence` golden render. `G_MELODY_NOTE = 79` (= 67 + 12) confirms the canonical golden seats with `lift = +12 ≥ MIN_FIGURE_GAP(2)` → the guard is a no-op there. The Test Engineer confirms across all goldens (§4).

### 2(c) — THE IMAGE-CONDITIONED PROMINENCE FAMILY (`assets/mappings.json:365-387`)

Replace the single `melody_forward` default with deep/mid/shallow recession tiers, routed by `fg_bg_contrast` (+ `subject_size`) via the EXISTING first-match-wins `SelectTable`. Additive + `serde(default)` back-compat: the old `mappings.json` still parses (the loader already tolerates an absent/empty SelectTable → `uniform` / neutral 0.5); the identity/empty profile → neutral 0.5. The realizer logic is UNCHANGED — only WHICH weights it reads changes.

**New `prominence_catalogue` rows (Music Theory authors the exact weights against the ear; these are starting-value sketches from the aesthetics lens §6.2):**

```jsonc
"prominence_catalogue": [
  { "id": "uniform",        "layers": [] },                       // UNCHANGED — the identity/no-op profile
  { "id": "subject_melody", "layers": [ /* UNCHANGED — the top escalation tier */
      { "role": "Melody", "weight": 1.0 }, { "role": "CounterMelody", "weight": 0.6 },
      { "role": "HarmonicFill", "weight": 0.4 }, { "role": "Pad", "weight": 0.3 },
      { "role": "Bass", "weight": 0.5 } ] },
  { "id": "melody_lead_strong", "layers": [   // DEEP tier (subject image): one clear figure
      { "role": "Melody", "weight": 0.90 }, { "role": "CounterMelody", "weight": 0.45 },
      { "role": "HarmonicFill", "weight": 0.30 }, { "role": "Pad", "weight": 0.30 },
      { "role": "Bass", "weight": 0.50 } ] },
  { "id": "melody_forward", "layers": [        // MID tier — UNCHANGED weights (today's default)
      { "role": "Melody", "weight": 0.78 }, { "role": "CounterMelody", "weight": 0.58 },
      { "role": "HarmonicFill", "weight": 0.40 }, { "role": "Pad", "weight": 0.40 },
      { "role": "Bass", "weight": 0.50 } ] },
  { "id": "melody_lead_gentle", "layers": [    // SHALLOW tier (field image): texture shares focus
      { "role": "Melody", "weight": 0.72 }, { "role": "CounterMelody", "weight": 0.65 },
      { "role": "HarmonicFill", "weight": 0.45 }, { "role": "Pad", "weight": 0.45 },
      { "role": "Bass", "weight": 0.50 } ] }
]
```

> **DECISION-2 compliance (counter recession = governor only, NOT a weight change).** Note the `melody_forward` MID row keeps CounterMelody at **0.58 — UNCHANGED**. The deep/shallow tiers carry DIFFERENT counter weights (0.45 / 0.65) only as the natural consequence of the per-image recession depth, NOT as the slice-3 DP-6 0.58→0.55 trim of the MID tier. The mid tier (the most-routed image class) is byte-stable in level; the activity recession (the governor, 2(a)) is the load-bearing counter fix this slice. The producer MUST NOT change the MID `melody_forward` counter weight.

**The `prominence` SelectTable — routed by figure-strength, first-match-wins (Music Theory authors the thresholds against the live `fg_bg_contrast` range 0.052–0.341):**

```jsonc
"prominence": {
  "default": "melody_forward",                            // MID — the fall-through (back-compat)
  "rules": [
    { "_comment": "DEEP/subject: a clear, separated subject justifies a strong lead + deep bed recession.",
      "when": [ {"knob":"fg_bg_contrast","op":"ge","lo":0.25,"hi":0.0} ], "pick": "melody_lead_strong" },
    { "_comment": "The existing subject_melody escalation tier — kept; a small, separated subject.",
      "when": [ {"knob":"subject_size","op":"in_range","lo":0.05,"hi":0.55},
                {"knob":"fg_bg_contrast","op":"ge","lo":0.10,"hi":0.0} ], "pick": "subject_melody" },
    { "_comment": "SHALLOW/field: low contrast, even energy — the texture legitimately shares focus.",
      "when": [ {"knob":"fg_bg_contrast","op":"lt","lo":0.10,"hi":0.0} ], "pick": "melody_lead_gentle" }
  ]
}
```

- Routing predicates use the EXISTING `Knob::FgBgContrast`/`Knob::SubjectSize` and the closed `CmpOp` set (`ge`/`lt`/`in_range`); no Rust change. First-match-wins → ORDER matters: DEEP (high contrast) first, then the existing `subject_melody` rule (preserved verbatim), then SHALLOW (low contrast), else MID default. Music Theory tunes the 0.25 / 0.10 thresholds against the ear and the 6-image spread.
- **Identity / back-compat:** an identity render carries an empty/`uniform` prominence (no rule matches a sentinel-default `ImageUnderstanding`, or the loader yields `uniform`), so every weight resolves to neutral 0.5 → no-op. The old `mappings.json` (with only `melody_forward`/`subject_melody`) still parses and routes to `melody_forward` as before.

---

## 3. THE EAR-TUNABLE KNOBS — every magnitude the taste gate sizes (with recommended starts)

Every knob below ships with a concrete recommended starting value so the producer can build immediately, AND is flagged EAR-TUNABLE for the standing taste/affect gate (per the Specialist Marshaling Gate; the magnitudes are sized between the trivial and the extreme — the SIGNS/DIRECTIONS are load-bearing and fixed, the magnitudes want the operator's ear).

| Knob | Where | Recommended START | Range | Sign/direction (FIXED) | Source lens |
|---|---|---|---|---|---|
| **`MIN_FIGURE_GAP`** | `chord_engine.rs` const (2b) | **2** semitones | [1, 5] | POSITIVE (a clear seat above the counter ceiling, never a tie — operator decision 5) | aesthetics + theory |
| **`ACTIVITY_FLOOR_THRESHOLD`** | `chord_engine.rs` const (2a.4) — the melody-prominence weight above which the floor bites | **0.50** (strict `>`; so exactly-neutral 0.5 is a no-op, every foreground weight floors) | [0.50, 0.60] | a FOREGROUND (>0.5) melody never falls to SUSTAINED; neutral 0.5 is a no-op | affect (activity-floor sizing) |
| **deep tier (`melody_lead_strong`) weights** | `mappings.json` (2c) — Melody / CounterMelody / Pad·Fill | **Melody 0.90 / Counter 0.45 / Pad·Fill 0.30** | Melody [0.85,0.95]; bed [0.28,0.45] (all > 0.25 floor) | bed recedes DEEP; melody leads strongly; gap widest | aesthetics §6.2 |
| **mid tier (`melody_forward`) weights** | `mappings.json` (2c) | **0.78 / 0.58 / 0.40 — UNCHANGED** | locked (byte-stable; do NOT change in slice 1) | — | carry-forward |
| **shallow tier (`melody_lead_gentle`) weights** | `mappings.json` (2c) | **Melody 0.72 / Counter 0.65 / Pad·Fill 0.45** | Melody [0.68,0.74]; counter [0.60,0.68]; bed [0.42,0.48] (> 0.25) | bed recedes SHALLOW; near-even texture; counter near-even (still under melody) | aesthetics §6.2 |
| **`f(fg_bg_contrast)` routing thresholds** | `mappings.json` SelectTable (2c) — the DEEP gate `ge 0.25` and the SHALLOW gate `lt 0.10` | **DEEP ≥ 0.25; SHALLOW < 0.10** (MID in between) | DEEP [0.20,0.30]; SHALLOW [0.08,0.12] | the SIGN is load-bearing: higher contrast → deeper tier; the required figure-ground MARGIN is positive only on subject (deep) images, ≈0 on field (shallow) — operator decision 7 | affect §4 + aesthetics §2.2 |

> **The `f(fg_bg_contrast)` hard-fail margin (the scorecard side, operator decision 7).** The build's prominence FAMILY routing thresholds above and the scorecard's F1 image-conditioned margin `f(fg_bg_contrast)` (`spec-s46-figure-ground-metrics.md` §1 F1(b)) are the SAME figure-strength binning: SUBJECT (high contrast, `f ≈ +0.3` onsets/step required), MID (`f` moderate), FIELD (low contrast, `f ≈ 0`). The HARD floor on EVERY image is `F1_margin ≥ 0` (a negative margin — bed busier than melody — fails everywhere); the positive required margin applies only on subject images. The Test Engineer sizes `f` to match these recession tiers (the magnitudes are ear-tuned; the sign is fixed).

---

## 4. FREEZE-NEUTRALITY WITNESS — per edit, re-grounded on the confirmed lines

`engine.rs` is byte-frozen at sha256 `e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261`; the `ENGINE_SHA256` guard (`tests/variety_scorecard_s45.rs:68`) asserts it and is untouched. The **9/9 `engine_equivalence` goldens that must stay byte-green** are the cadence hold **240** ms (`:281`), the cadence velocities **114** / **84** (`:277`/`:293`), the bass note **36** (`G_BASS_NOTE`), and the melody note **79** (`G_MELODY_NOTE`, `:138`/`:243`/`:341`) — across the equivalence net's golden renders.

| Edit | Why it is identity-byte-neutral (re-grounded on the confirmed live lines) |
|---|---|
| **2(a).1 `ActivityClass` enum** | A new pure type; never instantiated on the identity path (no Counter instrument; the floor's `melody_activity_class` call is gated behind the foreground-weight check, which is FALSE at neutral 0.5). Zero byte impact. |
| **2(a).2 `melody_activity_class` helper** | Pure fn; only CALLED by the governor (counter arm — unreachable under identity) and the floor (gated on `prominence_weight(ctx, Melody) > 0.50`, FALSE at neutral 0.5). Never invoked on the freeze path. |
| **2(a).3 governor (`:1899`)** | The counter arm (`:1831-1924`) is unreachable under identity (`pad_voices == 0`, empty `layers` → no Counter instrument). The predicate rewrite changes which branch the counter takes — moot when the arm is never entered. PRESERVES S45 (Subdividing melody → counter MOVING). |
| **2(a).4 melody activity floor (`:1943-1992`)** | Gated on `prominence_weight(ctx, Melody) > ACTIVITY_FLOOR_THRESHOLD(0.50)`. Under identity `prominence_weight` returns `PROMINENCE_NEUTRAL = 0.5` (empty vec, `:1026`), so strict `>` is FALSE → the existing SUSTAINED arm (`:1974-1992`) runs unchanged → byte-identical. The 79 / 36 / 240 / 114 / 84 goldens are unaffected. |
| **2(b) seat-order guard (`:1271`)** | Additive `.max(COUNTER_CEILING + MIN_FIGURE_GAP)` folded UNDER the EXISTING single `.clamp(24,96)` (the Risk-1 sum-clamp). **WITNESS (must be verified before landing):** the guard is a no-op iff `MELODY_REGISTER_FLOOR(67) + lift + prom_lift ≥ 67 + MIN_FIGURE_GAP(2)` on every golden step, i.e. iff `lift + prom_lift ≥ 2`. On the identity/golden path `prom_lift == 0` (neutral weight) so the condition is `lift ≥ 2`. `G_MELODY_NOTE = 79 = 67 + 12` confirms the canonical golden render seats with `lift = +12 ≥ 2` → the guard is a no-op. **The Test Engineer MUST confirm `lift ≥ MIN_FIGURE_GAP` (i.e. `bright_octaves` is non-negative enough) on EVERY `engine_equivalence` golden render before the const lands**; if any golden seats with `0 ≤ lift < 2`, lower `MIN_FIGURE_GAP` to ≤ that `lift` or hand-re-derive that golden in the same commit (S13 §7 discipline). The counter is never on the identity path, so the melody-vs-counter RELATION is moot there — only the self-floor magnitude matters. |
| **2(c) prominence family (JSON)** | Additive catalogue rows + SelectTable rules. The realizer already consumes resolved weights; the identity/empty profile → neutral 0.5 → every centered nudge is `(0.5−0.5)*SPAN == 0` (register `:1271`, velocity `:1404`, melody rhythm `:1942`). The old `mappings.json` still parses (loader tolerates absent rows; `serde(default)`). Zero byte impact on identity. The MID `melody_forward` row is UNCHANGED, so any non-identity render that already routed to `melody_forward` is also byte-stable. |

---

## 5. WHAT THE SCORECARD READS — the seam exposes nothing new

The producer's three changes are observable through the EXACT per-role `StampedEvent` streams `scorecard_for` already collects (`tests/variety_scorecard_s45.rs` `render()` `:173`, `per_step_pitch` `:268`, `step_shape_key` `:288`, `motion_dir` `:262`). **No new render path, no new image type, no audio, no OpenCV** — F1–F5 (`spec-s46-figure-ground-metrics.md` §1) read the same streams:

- **F1 (melody-most-active)** and **F5b (bg-activity-recession)** read per-(step,role) ONSET COUNTS (the first element of `step_shape_key`) — exactly what the governor (counter no longer out-onsets a holding melody) and the floor (foreground melody floors to ≥2 onsets) drive. The governor + floor are the levers that drive **F5b `bg_recession_violations` toward 0** (the HARD regression gate, `spec-s46-figure-ground-metrics.md` §2): on every co-sounding step a bed role must satisfy `bed_onsets(step) ≤ melody_onsets(step)`.
- **F3 (melody-highest)** reads `per_step_pitch` per role — the seat-order guard (2b) drives `F3_frac → 1.0` by making `melody_seat ≥ COUNTER_CEILING + MIN_FIGURE_GAP` structural.
- **F2 (bg-recession ratio)** reads the same onset densities; the family (2c) conditions the margin per image via `fg_bg_contrast` (already read in `scorecard_for` for the knobs row).
- **F4 (inverse-comp)** is REPORTED only (slice 3 territory) — unaffected by this slice; the seam exposes the same data it already does.

**The producer adds NOTHING to the render path.** The Test Engineer adds the F1–F5 computations + the F5b regression assertion + the `LayerVerdicts` fields (`figure_ground`, `melody_most_active_margin`, `melody_highest_frac`, `bg_recession_violations`, `rhythm_distinct_frac`) per `spec-s46-figure-ground-metrics.md` §3 — all from streams already in hand. **The F5b invariant** (`bed_onsets ≤ melody_onsets` per co-sounding step) is the precise quantity the governor + floor drive to 0; baseline it on the pre-fix tree (the inverted residual), then tighten the bound WITH this slice (the staged-bound M1.4 discipline).

---

## 6. PRESERVE-S45 STATEMENT (binding)

The CounterMelody routed in at S45 is a GAIN (the inner texture finally moves) and MUST be preserved. The governor (2a.3) recedes the counter ONLY relative to a lifted/holding melody:
- When the melody **MOVES** (`ActivityClass::Subdividing`), the counter takes its MOVING branch (the guaranteed off-beat onset) — **the counter still moves.**
- When the melody **HOLDS** (`Sustained`), the counter recedes to its OBLIQUE sustained tone (or the existing breathing rest-as-gesture) — **subordinate motion / a sustained inner voice, never silence.**
A blanket counter re-suppression or a `pad_bed_counter` de-route is FORBIDDEN. The one legitimate hard per-image recession is the deep-tier (subject) prominence weight (2c, `melody_lead_strong` counter 0.45) — a deliberate per-image hierarchy call, still a moving line, never a mute.

---

## 7. CLIMAX-BLOOM CROSS-ARC INVARIANT (out of scope for slice 1 — state it so it is not broken)

The climax-bloom is the texture-arc slice's concern (S44 slice 2), NOT slice 1. But slice 1 establishes the figure-ground MARGIN as a first-class quantity (F1/F3/F5), so the producer must not encode anything that would prevent the later climax guard. **The cross-arc invariant (do not break):** at the climax/Return the bed may bloom in density ONLY IF the figure-ground gap blooms WITH it — encodable later as "F1 margin at the climax section ≥ F1 margin at the Statement section." Slice 1 must keep the margin a per-section-measurable quantity (it does — F1/F5 are per-step/per-section) and must NOT make the activity floor or governor section-blind in a way that pins a constant margin. (The current spec is section-agnostic per step, which is correct — the arc rides on top later.)

---

## 8. RISKS / OPEN MICRO-DECISIONS for the producer

1. **The 4-band → 3-class mapping (2a.2) is the one interpretive call.** DOTTED→Oblique, SYNCOPATED+ARPEGGIO→Subdividing is locked here per the theory lens, but the Music Theory Specialist owns confirming it sounds right (whether a dotted melody is "active enough" to let the counter move, or should still recede the counter). **Lead decision not required** — it is the producer's craft call within the locked mapping; flag if the ear wants DOTTED→Subdividing instead.
2. **Cutoff-duplication coupling.** `melody_activity_class` (2a.2) MUST keep its cutoffs (`0.80/0.55/0.25 - prom_shift`) 1:1 with the live Melody arm (`:1943/:1956/:1965`). A future cutoff change must move BOTH. Recommend the producer extract the cutoffs to shared consts (`MELODY_ARP_CUTOFF` etc.) so the helper and the arm read one source — a small, freeze-neutral refactor (the arm's behavior is unchanged). **Producer's discretion; recommended.**
3. **No `activity_floor` JSON field.** The S46 architect doc floated an optional `serde(default) activity_floor: Option<u8>` on `ProminenceProfile`. This spec deliberately does NOT use it — the floor keys off the EXISTING resolved prominence weight (a foreground melody is already `> 0.5`), so a new field is redundant over-build. If the producer finds the per-image floor magnitude needs to differ from a single global threshold, that is a slice-3 refinement, not slice 1.
4. **The `prom_shift` source in the governor (2a.3).** The governor asks "how active is the MELODY," so it computes `melody_activity_class` using the MELODY role's prominence weight (`prominence_weight(ctx, OrchestralRole::Melody)`), NOT the counter's — both reads are cheap and already available in `realize_rhythm` via `ctx`. Pinned here so the producer does not accidentally pass the counter's weight. **No ambiguity remains; stated to prevent the slip.**
5. **Seat-guard witness gating the const value (2b/§4).** The ONLY freeze risk in the slice. The Test Engineer MUST run the witness (`lift ≥ MIN_FIGURE_GAP` on every golden) before the const lands. If a golden seats with `lift < 2`, either lower `MIN_FIGURE_GAP` or hand-re-derive that golden in-commit. **This is the one item the lead should confirm is gated in the build order** (witness FIRST, then the const).
6. **Single-writer on `mappings.json`.** The prominence family (2c) and any other `mappings.json` touch in this engagement go through single-writer coordination (the S42 discipline) — Music Theory owns the musical rows; do not let two writers race the file.

---

*Design-only. No source, test, or asset modified by this document. `src/engine.rs` sha256 re-verified UNCHANGED this session: `e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261`.*

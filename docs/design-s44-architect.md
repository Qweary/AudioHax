# S44 — Per-Layer Variety & N-Voice Architecture (Rust Architect lens)

**Author role:** Rust Architect (DESIGN ONLY — no source modified by this document; `docs/`
only). All proposed Rust is signatures / types / doc comments — **no bodies**.
**Date:** 2026-06-19
**Grounded against** the working tree at HEAD: `src/engine.rs` (BYTE-FROZEN, sha256
`e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261`, re-verified this session),
`src/chord_engine.rs`, `src/composition.rs`, `src/cli.rs`, `src/main.rs`, `assets/mappings.json`.
Precedent read first: `design-s42-trace.md`, `design-s42-salience-diagnosis.md`,
`design-s13-diversity.md`, `design-s15-variety-engine.md`, `design-s15-variety-musical.md`,
`composition-architecture-engine.md`, `assessment-composition-architecture.md`.

> **The binding frame this doc carries (the lead's load-bearing counterpoint).** Raw voice count
> is architecturally cheap — `num_instruments` is already a free parameter and the CLI already
> accepts `6` (`src/cli.rs:701`). It is also musically EMPTY without per-voice differentiation.
> "Support >N voices meaningfully" is a **facet** of "more per-layer variety," not a second
> feature. A naive `num_instruments`-bump is the **anti-pattern**: it multiplies a thin texture
> (the "thicker but undifferentiated" trap). This doc ranks **variety-first** and treats the
> bump as the thing to *guard against*, not the thing to ship.

---

## 0. Executive summary (read first)

1. **`num_instruments` is NOT the ceiling.** It is a live, free parameter end-to-end
   (`--instruments N` → `EngineConfig.num_instruments` → `realize_step(inst_idx, num_instruments)`).
   `4` is a *default* (`src/engine.rs:188`, `:203`; `src/cli.rs:95`), not a clamp.

2. **The real ceiling is a THREE-WAY narrowness, all freeze-reachable (none in `engine.rs`):**
   - **Role vocabulary:** 5 `OrchestralRole` variants and a role-assignment that, past
     5 voices, produces undifferentiated copies (identity path: every inner instrument is the
     *same* `HarmonicFill`; profile path: every extra instrument *clamps onto the last named
     layer* — Melody — `src/chord_engine.rs:1053`).
   - **Register bands:** only three floors (Bass 36 / Fill 55 / Melody 67) plus the Counter band
     `[55,67)`. Every inner voice collapses into the same ~12-semitone window
     (`src/chord_engine.rs:1210-1212`, `:3370`), and the inner-tone `pick` wraps modulo
     chord-tone count (`:1279-1286`) so a 4th/5th inner voice **doubles** an existing tone.
   - **Prominence catalogue:** every profile names exactly the 5 roles; there is no
     per-*instance* prominence (two HarmonicFills get one identical weight).

3. **The variety levers fall on a clean three-tier seam** that the S42 trace already mapped and
   this doc extends to N voices: `assets/mappings.json` (zero-Rust, **freeze-safe**) →
   `src/chord_engine.rs` / `src/composition.rs` (Rust, **freeze-reachable**) → `src/engine.rs`
   (**FROZEN**, flagged loud). **Nothing variety-first needs `engine.rs`.**

4. **Recommendation on the default voice count: do NOT change it.** Keep the default at 4.
   The default-count change is a frozen-kernel *value* decision (`src/engine.rs:203`) that earns
   nothing musically until per-voice differentiation exists to fill the added lines. Ship variety
   first; raise the default (or expose richer N via `--instruments`, which already works) only
   after a differentiation precondition is in place (§5).

5. **Smallest freeze-safe first slice (S45 candidate):** a **per-instance figuration/register
   offset within the inner voices** — make the existing HarmonicFill/Pad voices *differ from each
   other* before adding more of them. This is the differentiation precondition that turns every
   later voice-count increase from "thin multiplication" into "earned counterpoint." Detail in §6.

---

## 1. CURRENT STATE — how voices and roles are actually wired

### 1.1 The `num_instruments` flow (CLI / runtime → realize path)

```
--instruments N                     src/cli.rs:65 (PipelineCliArgs.instruments, default 4 @:95)
  └─ pipeline_to_engine_config      src/cli.rs:242  → EngineConfig{ num_instruments: args.instruments, .. }
       └─ EngineConfig              src/engine.rs:188 (field), :203 (Default = 4)   [FROZEN file]
            └─ decide_step          src/engine.rs:527,532  let num_instruments = self.config.num_instruments;
                 └─ decide_instrument_action  src/engine.rs:699,703,740
                      └─ chord_engine::realize_step(step, inst_idx, num_instruments, …)  src/chord_engine.rs:1076
                           └─ assign_role(inst_idx, num_instruments, ctx)  src/chord_engine.rs:1084 → :1038
```

Runtime mutation: `EngineCommand::SetInstruments(n)` → `self.config.num_instruments = n`
(`src/engine.rs:633`). `decide_step` re-reads `self.config.num_instruments` every step
(`:532`), so a live `SetInstruments` takes effect on the next step with no replan. The CLI test
at `src/cli.rs:701-708` already constructs `instruments: 6` and asserts
`cfg.num_instruments == 6` — **N>4 already builds and runs; it is the *musical* result that is
degenerate, not the plumbing.**

`src/engine.rs:496` and `:850` (and the test mock at `:1000`) truncate/pad the per-step scan-bar
row to exactly `num_instruments`, so the feature row is always N-wide — every instrument gets a
scan-bar feature regardless of N. No clamp on N anywhere in the feature path.

### 1.2 Role assignment — the two paths, and where they degenerate

There are **two** role-assignment functions; `realize_step` calls `assign_role` (`:1084`),
which branches:

```rust
// src/chord_engine.rs:1038
pub fn assign_role(inst_idx: usize, num_instruments: usize, ctx: &StepContext) -> OrchestralRole {
    if prof.is_identity() { return instrument_role(inst_idx, num_instruments); } // legacy/flat path
    let layers = &prof.layers;
    let clamped = inst_idx.min(layers.len().saturating_sub(1));                  // profile path
    to_orchestral_role(layers[clamped])
}
```

**Path A — identity / flat (`instrument_role`, `src/chord_engine.rs:929`).**
```
n<=1 → Melody
idx 0 → Bass ; idx == n-1 → Melody ; everything between → HarmonicFill
```
So for n=6 identity: `[Bass, Fill, Fill, Fill, Fill, Melody]` — **four identical HarmonicFills.**
The four inner voices are the same role, and (per §1.3) get the same register treatment and
prominence. They differ only by the inner-tone `pick` spread, which itself wraps (§1.3).

**Path B — non-identity orchestration profile (clamp, `:1053`).** Every texture profile in
`assets/mappings.json` (`pad_bed`, `pad_figured`, `pad_broken_*`, `pad_oom_pah`, `pad_walking`,
…, 12 profiles) names **exactly 4 layers** `[Bass, Pad, HarmonicFill|CounterMelody, Melody]`.
`assign_role` clamps `inst_idx` onto `layers.len()-1`, so for n=6 with `pad_bed`:
`[Bass, Pad, HarmonicFill, Melody, Melody, Melody]` — **three Melodies in unison.** The doc
comment at `:1033-1037` explicitly anticipates this ("a 5th instrument would extend the Melody
layer") and accepts it as a *safe* edge — safe in the sense of not muddying the bass, but
musically it is unison doubling, not a new line.

`to_orchestral_role` (`:948`) and `prominence_weight`'s bridge (`:1013-1019`) are total 1:1 maps
over the same 5-variant `LayerRole`/`OrchestralRole` pair (`src/composition.rs:474`,
`src/chord_engine.rs:856`). **The vocabulary is 5 roles; there is no sixth structural line.**

### 1.3 What happens RIGHT NOW at n>5 — doubling, not differentiation

| Layer / dimension | Per-voice differentiated today? | Mechanism / file:line |
|---|---|---|
| **Bass (idx 0)** | N/A — single voice | `role_pitch` Bass arm `:1239-1247`; one root in `[36..]` |
| **Melody (idx n-1)** | N/A — single voice (but profile path makes idx≥n-1 **unison-double** Melody) | `:1248-1262`; profile clamp `:1053` |
| **Inner voices (HarmonicFill / Pad / Counter)** | **Partially, then it WRAPS** | `:1271-1297` |
| **Register band of inner voices** | **NO — all share `[55,67)`** | floors `:1211-1212`, counter band `:3370` |
| **Prominence weight** | **NO — one weight per ROLE, not per instance** | `prominence_weight` `:1013`; catalogue keyed by role |
| **Rhythm / figuration** | Per-ROLE, not per-instance | `realize_rhythm` `:1481`; Pad reads one `figuration_resolved` `:1739` |

The inner-tone spread is the *only* per-instance differentiation that exists:
```rust
// src/chord_engine.rs:1279-1286  (HarmonicFill | Pad | CounterMelody arm of role_pitch)
let pick = if num_instruments >= 3 {
    let fill_rank = inst_idx.saturating_sub(1);
    1 + (fill_rank % inner_count.saturating_sub(1).max(1))   // <-- wraps modulo chord-tone count
} else { inner_count / 2 };
```
With a triad (`inner_count == 3`) the divisor is `2`, so `fill_rank` 0,1,2,3 → picks 1,2,1,2:
**the 3rd and 4th inner voices double the 1st and 2nd.** Past the chord's tone count the spread
provides nothing — added inner voices are tone-duplicates in the same register band at the same
prominence, played with the same Pad/Fill rhythm. **That is the thin-multiplication trap, already
realized in code at n≥5.**

### 1.4 The macro picture (carry-over from `composition-architecture-engine.md` §0)

The S44 variety arc sits *under* the standing macro-form gap, not in place of it. The engine
still SONIFIES A SCAN: there is no per-instance voice plan, no contrapuntal relationship between
inner voices, no theme memory across voices. Per-layer variety + N voices is a **horizontal**
enrichment (more differentiated simultaneity); macro-form is the **vertical/temporal**
enrichment. They are orthogonal and S44 is the cheaper, freeze-safer of the two. Do not let the
N-voice work absorb the macro-form scope.

---

## 2. PER-LAYER VARIETY GAP MAP — where each lever physically lands

Tiers, repeated for every row: **[JSON]** = `assets/mappings.json`, zero-Rust, freeze-safe ·
**[CE]** = `src/chord_engine.rs`, Rust, freeze-reachable · **[COMP]** = `src/composition.rs`,
Rust planner, freeze-reachable · **[FROZEN]** = `src/engine.rs`, frozen-kernel decision.

| Layer | Variety gap today | Lever | Seam (file:line) | Tier |
|---|---|---|---|---|
| **Melody (rhythm)** | 4 fixed bands keyed off one global `edge_activity` scalar; same grid as the bed | New onset tables / band cutoffs | `realize_rhythm` Melody arm `chord_engine.rs:1896-1960`; cutoffs are consts | **[CE]** (cutoffs) / **[JSON]** if cutoffs lifted to a catalogue (not today) |
| **Melody (prominence/dynamics)** | Per-image, but two-tier only (S43) | More prominence profiles + finer gate | `prominence_catalogue` + `prominence` SelectTable | **[JSON]** |
| **CounterMelody** | Real line (S18+), but **only present in `pad_bed_counter`**, gated on `foreground_energy≥0.35` | Add Counter to more texture profiles; widen its gate | `texture_catalogue` rows + `texture` rules | **[JSON]** |
| **HarmonicFill** | Sustained inner tone; rest-as-gesture only | More fill figures / per-instance offset | `realize_rhythm` Fill arm `:1716-1737`; `role_pitch` `:1271` | **[CE]** |
| **Pad (figuration)** | 9 figures exist; chosen per-SECTION, ONE per section | More figures; per-instance figure variation | `figuration_catalogue` (JSON) + `figured_bed` `:2278` | **[JSON]** for new figures; **[CE]** for per-instance |
| **Bass (pattern)** | 5 patterns (sustained/walking/pedal); ONE per section | More patterns | `bass_pattern_catalogue` + dispatch `:1635-1714` | **[JSON]** for new patterns (dispatch exists) |
| **Harmony / figuration selection** | `texture` SelectTable reads 6 knobs but every pick is a 4-voice profile | New profiles; richer rules | `texture_catalogue` + `texture` rules | **[JSON]** |
| **Per-section density** | `section.density` pinned at 0.5 → the `(density-0.5)*GAIN` term in `edge_activity` is dead (S42 trace §B.1) | Drive density off the image | planner sets per-section `density` | **[COMP]** |
| **Register spread of inner voices** | All inner voices share `[55,67)` | Sub-band the inner register by instance | `role_pitch` `:1271-1297`, floors `:1211-1212` | **[CE]** |
| **Per-instance role/prominence** | None — weight is per-role | Per-instance prominence + sub-role | `prominence_weight` `:1013`; `assign_role` `:1038` | **[CE]** (+ **[JSON]** for the data) |
| **Ensemble width** | `--instruments` works; default 4 | Change default | `EngineConfig::default` `engine.rs:203` | **[FROZEN]** ⚠ |

**Concrete seam note on the FROZEN boundary.** `engine.rs` holds exactly three N-voice-relevant
things: the `num_instruments` *field* and *default value* (`:188`, `:203`), the `SetInstruments`
command handler (`:633`), and the scan-row truncate/pad to N (`:496`, `:850`). All three already
*support arbitrary N* — they are width-agnostic plumbing. **The only frozen thing that is a
musical decision is the default value `4`.** Everything that makes N voices *sound different*
lives outside the freeze. This is the load-bearing freeze fact for the whole arc.

---

## 3. N-VOICE ARCHITECTURE — what "support N voices MEANINGFULLY" requires

The goal is **N distinct lines**, not N copies. Three things must generalize from "5 roles +
3 register bands" to "N differentiated voices." All are freeze-reachable in `chord_engine.rs` /
`composition.rs`; **none requires touching `engine.rs`.** Signatures only — no bodies.

### 3.1 Generalize role assignment past the 5-role clamp

The defect is that both paths degenerate past their named layers (unison-Melody on the profile
path; identical-Fill on the flat path). Introduce a **per-instance VOICE ASSIGNMENT** that, for
inner voices, carries an *index within the inner group* so the realizer can differentiate them.
This is additive to the existing `OrchestralRole` (the 5 roles stay; we add per-instance context),
preserving every existing call site and the byte-freeze identity path.

```rust
// src/chord_engine.rs — NEW, additive. The 5-variant OrchestralRole is UNCHANGED.
//
/// A realized voice's role PLUS its position within voices sharing that role.
/// `role` is the existing structural role; `voice_in_role` is 0-based within the
/// set of instruments assigned the same role this step (0 for the sole Bass/Melody;
/// 0..k for the k inner voices). `voices_in_role` is that set's size. This is the
/// handle every per-voice differentiation lever (register sub-band, figuration
/// rotation, prominence instance) reads — it is what makes the 3rd inner voice
/// DIFFER from the 1st instead of doubling it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoiceAssignment {
    pub role: OrchestralRole,
    pub voice_in_role: u8,
    pub voices_in_role: u8,
}

/// PLAN-AWARE, per-instance. Supersedes `assign_role` as the realizer's entry; the
/// existing `assign_role`/`instrument_role` become the `.role` projection so all
/// current call sites and goldens are byte-stable. For N > named-layers, instead of
/// clamping every extra instrument onto the last layer, it distributes the surplus
/// across the INNER roles (HarmonicFill/Pad/CounterMelody) with increasing
/// `voice_in_role`, never unison-doubling Melody and never wrapping onto Bass.
pub fn assign_voice(inst_idx: usize, num_instruments: usize, ctx: &StepContext) -> VoiceAssignment;
```

`realize_step`'s **public signature is unchanged** (it still takes `inst_idx, num_instruments,
ctx`); internally it would call `assign_voice` instead of `assign_role` and thread the
`VoiceAssignment` down the *already-blessed additive-private-param route* (the `pad_voices` /
`prominence_w` precedent at `:1101-1105`). The identity path returns `voices_in_role == 1` for
every voice → every per-instance term is a no-op → **byte-frozen identity holds** exactly as the
prominence-neutral pivot does today.

### 3.2 Generalize register seating to N voices (the band-collapse fix)

Today the inner voices share `[FILL_REGISTER_FLOOR(55), MELODY_REGISTER_FLOOR(67))` and the
`pick` wraps modulo chord-tone count (`:1279-1286`). Generalize `role_pitch` to take the
`VoiceAssignment` and SUB-DIVIDE the inner band by `voice_in_role / voices_in_role`, and to
SPREAD across chord tones AND octaves (not just the bare triad) so the 4th inner voice lands a
chord tone an octave up rather than a duplicate.

```rust
// src/chord_engine.rs — MODIFY (private fn; realize_step's PUBLIC sig unchanged).
// BEFORE: fn role_pitch(role, chord, inst_idx, num_instruments, features, prominence_w) -> u8
// AFTER:  fn role_pitch(voice: VoiceAssignment, chord, inst_idx, num_instruments,
//                       features, prominence_w) -> u8
//
// Inner-voice seating becomes a function of voice_in_role/voices_in_role: partition
// the [55,67) inner band into `voices_in_role` sub-bands (or extend upward by octave
// when voices_in_role exceeds the chord-tone count) so two inner voices never seat the
// same pitch. The Bass/Melody arms are unchanged (voices_in_role == 1 → identity seat).
```

Risk-1 sum-clamp (`:1255-1261`, `:1288-1296`) is preserved: every inner sub-band still rides the
single `.clamp(24,96)`; the recessive-never-lowered guard (`:1294-1295 .max(0)`) is unchanged.

### 3.3 Generalize prominence / voice-leading / catalogues to N

- **Prominence:** today one weight per role (`prominence_weight`, `:1013`). For N inner voices
  add an OPTIONAL per-instance taper so inner voices recede progressively (voice 0 supports,
  voice k whispers) rather than all sitting at the role's single weight. Data-side this is a
  **[JSON]** addition to `prominence_catalogue` (an optional `per_voice_falloff: f32`);
  consume-side a **[CE]** read in `prominence_weight` keyed off `VoiceAssignment`.
  ```rust
  // src/chord_engine.rs — MODIFY
  // BEFORE: fn prominence_weight(ctx, role) -> f32
  // AFTER:  fn prominence_weight(ctx, voice: VoiceAssignment) -> f32   // role + instance falloff
  ```
- **Voice leading:** the CounterMelody arm already does real contrary/oblique motion against the
  Melody (`:1818-1893`) and stays in `[55,67)` (`COUNTER_CEILING`, `:3370`). Generalizing to N
  inner moving voices means each inner voice checks parallel-perfects against the voice *below*
  it, not only against the Melody — a **[CE]** extension of `realized_counter_pitch_with_prev`
  parameterized by `VoiceAssignment`. This is the genuinely hard musical seam; flag it as the
  cross-lens dependency (§7).
- **Texture catalogue:** every profile is 4-wide. Generalize the schema so a profile can name an
  inner-voice *group* with a width hint, e.g. `Pad×k`, instead of one `Pad` entry. This is a
  **[JSON]** schema addition consumed by `assign_voice`. Keeps `LayerRole` (5 variants)
  unchanged; adds a repeat-count, not a sixth role.
  ```rust
  // src/composition.rs — additive field on OrchestrationProfile (serde default keeps every
  // existing JSON profile byte-shape-stable, exactly like figuration/bass_pattern did):
  /// Optional per-layer voice multiplicity. None/absent == 1 each (today's behavior).
  /// e.g. [Bass:1, Pad:1, HarmonicFill:3, Melody:1] realizes 3 differentiated fills.
  #[serde(default)]
  pub layer_widths: Option<Vec<u8>>,
  ```

### 3.4 What is freeze-reachable vs frozen

| N-voice change | Site | Tier |
|---|---|---|
| `VoiceAssignment` type + `assign_voice` | `chord_engine.rs` | **[CE]** freeze-reachable |
| `role_pitch` inner-band sub-division | `chord_engine.rs` | **[CE]** freeze-reachable |
| per-instance `prominence_weight` + falloff data | `chord_engine.rs` + `mappings.json` | **[CE]+[JSON]** |
| N-voice voice-leading (inner vs inner) | `chord_engine.rs` | **[CE]** freeze-reachable |
| `layer_widths` schema | `composition.rs` + `mappings.json` | **[COMP]+[JSON]** |
| `realize_step` public signature | `chord_engine.rs` | **UNCHANGED** (engine.rs calls it; signature must not move) |
| **default `num_instruments` value** | `engine.rs:203` | **[FROZEN]** ⚠ frozen-kernel decision |
| `SetInstruments` / width plumbing | `engine.rs:633,496,850` | **[FROZEN] but already N-agnostic — no edit needed** |

**The only frozen-kernel question in the entire arc is whether to change the default `4`** — and
§4 recommends NOT changing it.

---

## 4. FREEZE IMPLICATIONS — per-change verdict

| Change | Site | Touches `engine.rs`? | Verdict |
|---|---|---|---|
| New prominence/texture profiles, new figuration/bass patterns | `mappings.json` | NO | **FREEZE-SAFE** — additive Vec rows, loader parses the shape |
| Widen CounterMelody gates / add Counter to more profiles | `mappings.json` `texture` rules | NO | **FREEZE-SAFE** |
| `layer_widths` optional field | `composition.rs` (serde default) + `mappings.json` | NO | **FREEZE-SAFE schema** (the figuration/bass_pattern precedent: `#[serde(default)]` keeps every old profile byte-shape-stable) |
| `VoiceAssignment` + `assign_voice` + per-instance `role_pitch`/`prominence_weight` | `chord_engine.rs` | NO | **FREEZE-REACHABLE** — `realize_step` public sig unchanged; identity path returns `voices_in_role==1` → every per-instance term is the centered no-op, identity bytes preserved |
| N-voice inner voice-leading | `chord_engine.rs` | NO | **FREEZE-REACHABLE** — unreachable under identity (no extra inner voices), byte-neutral on the freeze path |
| Per-section `density` driven off image | `composition.rs` planner | NO | **FREEZE-REACHABLE** — realizer already reads `ctx.section.density` zero-copy |
| **Change default `num_instruments` 4 → N** | `engine.rs:203` | **YES** | **FROZEN-KERNEL DECISION** ⚠ — must be a deliberate, flagged freeze edit; **recommended AGAINST** (§4 rationale) |

**Recommendation on the default voice count.** **Keep it at 4.** Rationale: (a) `--instruments N`
and `SetInstruments(n)` already let any caller request more voices *without* editing the frozen
file — the capability is reachable today; (b) raising the default changes the byte-output of every
existing render and would need the engine.rs byte-freeze re-baselined (a real cost the arc does
not need to pay to deliver variety); (c) the default should only rise once §5's differentiation
precondition holds, otherwise it ships the thin-multiplication trap to every user by default.
If, after the differentiation work lands, the operator's ear wants a richer default, *that*
becomes a clean, justified, one-value frozen-kernel edit — made deliberately, not assumed.

---

## 5. RISKS / TRADE-OFFS — the thin-multiplication trap and its architectural guard

**The trap (the lead's counterpoint, now located in code).** Bumping `num_instruments` to 6 today
yields, per §1.3, either four identical HarmonicFills (flat path) or three unison Melodies
(profile path), with inner voices doubling chord tones in one register band at one prominence and
one rhythm. The texture is *thicker* and *more masked* (more energy competing with the melody the
S42/S43 arc just fought to foreground) but **not more musical**. Adding voices before
differentiation is strictly negative: it re-buries the melody S43 just surfaced.

**The architectural guard — make a voice EARN its place.** A voice should only be realized as a
*distinct line* if it differs from its neighbors on at least one perceptual axis: register
sub-band, chord-tone/octave, rhythm/figuration, or prominence. The `VoiceAssignment` of §3.1 is
exactly that guard made structural: `voices_in_role > 1` is the *signal* that differentiation must
be applied, and every per-instance lever keys off `voice_in_role`. The encodable invariant
(pairs with a taste-gate ear test, mirroring the S43 correctness guards):

> **Voice-distinctness invariant.** For any two instruments `i ≠ j` realized on the same step,
> their realized `(register-band, primary chord tone, onset grid)` triple must not be identical
> unless they are a deliberate octave doubling. Today this is VIOLATED at n≥5; the guard makes the
> violation a test failure, so added voices cannot regress to copies.

**Other trade-offs:**
- **Masking vs. clarity.** More inner voices raise the noise floor under the melody. The
  per-instance prominence falloff (§3.3) is the mitigation: inner voices recede progressively, so
  width adds *body*, not *competition*. Without it, N voices undo S43.
- **Voice-leading combinatorics.** N independent moving lines is the genuinely hard part
  (parallel-perfects across all pairs). Mitigation: keep most added inner voices as *held/Pad*
  (low voice-leading burden), and add at most one extra *moving* CounterMelody per section. This
  is a musical-craft call → cross-lens dependency (§7).
- **Determinism / goldens.** Every per-instance term must be centered on the identity (`voices_in_role==1`)
  no-op, exactly as prominence centers on 0.5. This is the discipline that has kept `engine.rs`
  byte-frozen across S17–S43 and must hold here.
- **Schema creep.** `layer_widths` is the *only* new schema field; resist adding a sixth
  `LayerRole`. Width is a multiplicity, not a new structural role — keeping the 5-role vocabulary
  fixed is what keeps `to_orchestral_role`/`prominence_weight` total and the freeze argument clean.

---

## 6. SLICEABILITY — variety-first ranking and the smallest first slice

Ranked **variety-first** (the bump is last, and frozen-gated):

1. **[JSON, freeze-safe] Widen the existing per-SECTION vocabulary.** Add figuration patterns,
   bass patterns, texture profiles, and prominence profiles; widen the CounterMelody gate so a
   real second line appears on more images. Zero Rust, zero freeze risk, immediate per-image
   variety. *Unlocks:* more distinct gaits per image with no architectural change. (This is the
   direct continuation of the S42/S43 mappings.json work.)

2. **[CE, freeze-reachable] Per-INSTANCE inner-voice differentiation — THE S45 CANDIDATE.**
   Introduce `VoiceAssignment` (§3.1) and make `role_pitch` sub-divide the inner register band
   and spread chord tones/octaves per instance (§3.2), with the voice-distinctness invariant
   (§5) as the correctness guard. **This is the smallest slice that makes voices DIFFER, and it
   is the precondition that turns every later voice-count increase from multiplication into
   counterpoint.** It improves the *existing* 4-voice render too (the 3rd inner voice already
   doubles today — §1.3), so it pays off before any N increase. *Unlocks:* meaningful N — after
   this, `--instruments 6` produces six differentiated lines, not six copies.

3. **[JSON+COMP, freeze-safe/-reachable] `layer_widths` + image-driven per-section density.**
   Let a texture profile request `HarmonicFill×3`, and unpin `section.density` so the bed busyness
   varies per image (the dead `(density-0.5)*GAIN` term, S42 trace §B.1). *Unlocks:* the planner
   can *choose* a wide differentiated texture per image.

4. **[CE, freeze-reachable] N-voice inner voice-leading.** Generalize the Counter line's
   contrary-motion gates to inner-vs-inner pairs (§3.3). The hard musical slice; do last, gated
   on the Music Theory lens. *Unlocks:* added moving voices that are contrapuntally correct.

5. **[FROZEN] Raise the default voice count.** Only after slices 2–4. A deliberate, flagged
   one-value freeze edit, or simply leave it at 4 and let `--instruments` carry richer N.
   *Recommended: leave the default at 4.*

**Smallest freeze-safe first slice (S45):** if the lead wants the absolute minimum, slice 1 is
pure JSON and ships variety today. But the slice that *unblocks the N-voice goal* — and the one
this doc recommends as the S45 build — is **slice 2** (the per-instance differentiation +
voice-distinctness invariant), because it is the architectural precondition the whole "support >N
voices meaningfully" signal depends on, and it improves the current render immediately. It is
freeze-reachable (`chord_engine.rs` only), keeps `realize_step`'s public signature, and centers
every per-instance term on the identity no-op so the byte-freeze holds.

---

## 7. Decision points for the lead

1. **Frozen-kernel question (the only one):** change the default `num_instruments` from 4?
   **Architect recommendation: NO** — `--instruments`/`SetInstruments` already deliver richer N
   without a freeze edit, and raising the default before differentiation ships the
   thin-multiplication trap by default. Revisit only after slice 2, as a deliberate one-value edit.
2. **First-slice choice (S45):** pure-JSON vocabulary widening (slice 1, fastest, but doesn't
   touch the N-voice goal) vs. per-instance differentiation (slice 2, the precondition for
   meaningful N, freeze-reachable). Architect recommends **slice 2** as the S45 build.
3. **Schema shape:** approve `layer_widths: Option<Vec<u8>>` as the multiplicity mechanism and
   the decision to **keep the 5-role `LayerRole` vocabulary fixed** (no sixth role).
4. **Cross-lens dependency (Music Theory / Affect / Aesthetics):** the inner-voice register
   sub-banding (§3.2), the per-instance prominence falloff curve (§3.3), and the N-voice
   voice-leading rules (§3.3, slice 4) are musical-craft decisions, not architecture decisions.
   The architecture provides the `VoiceAssignment` handle and the freeze-safe seams; **how** the
   inner voices should differ (which chord tones, how far they recede, what counterpoint they
   keep) needs the Music Theory lens, and **whether the result reads as richer-vs-muddier** needs
   the Affect/Aesthetics taste gate — exactly the standing taste-gate-beside-correctness cadence
   the S43 work used.

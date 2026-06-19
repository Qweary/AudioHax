# S44 — Per-Layer Variety & Meaningful N-Voice: Unified Assessment + Work Order

**Author role:** Rust Architect (SYNTHESIS round of the S44 design cadence).
**DESIGN ONLY — no source, test, or asset modified by this document.** All Rust shown is
signatures / types / doc comments — **no bodies**.
**Date:** 2026-06-19
**Synthesizes** the four S44 lens designs: `docs/design-s44-architect.md` (Architecture),
`docs/design-s44-theory.md` (Music Theory), `docs/design-s44-affect.md` (Affect/Cross-Modal),
`docs/design-s44-aesthetics.md` (Aesthetics).
**Grounded against** the working tree at HEAD: `src/engine.rs` (**BYTE-FROZEN**, sha256
`e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261` — re-verified unchanged at
the start AND end of this session), `src/chord_engine.rs`, `src/composition.rs`, `src/cli.rs`,
`assets/mappings.json`. Every cited file:line below was read directly this session to
adjudicate the cross-lens discrepancy in §2.

> **The binding frame this synthesis carries (the lead's load-bearing counterpoint).** Raw voice
> count is architecturally cheap (`num_instruments` is a free parameter; `--instruments 6` already
> builds and runs — `src/cli.rs:701`) and musically EMPTY without per-voice differentiation.
> **">N voices meaningfully" is a FACET of "more variety," not a second feature.** Adding voices
> before differentiation multiplies a thin texture — the trap, already realized in code at n≥5.
> This synthesis ranks **variety-first** and treats the naive `num_instruments`-bump as the
> explicit anti-pattern, placed LAST and frozen-gated.

---

## 1. EXECUTIVE SYNTHESIS — the one governing finding across all four lenses

**The variety the operator wants is largely ALREADY IN THE ENGINE, but it is dormant, gated,
cloned-flat, or hidden behind a stale "stubbed" comment. The S44 gap is differentiation +
deployment, not raw capability — and voice-count is strictly downstream of both.** All four
lenses converge on this from different angles:

- **Architecture** finds `num_instruments` is not the ceiling; the ceiling is a three-way
  narrowness (5-role vocabulary that degenerates past 5 voices, a single shared inner register
  band `[55,67)`, and per-role-not-per-instance prominence) — **all freeze-reachable, none in
  `engine.rs`.**
- **Music Theory** finds a "maturity asymmetry": a rich *moving* vocabulary EXISTS (the
  CounterMelody species engine, walking/pedal bass, the 11-figure figuration catalogue, 7 dead
  `ThemeVariation` variants) but is **gated/clamped/unrouted** while the three always-on bed roles
  (Bass, HarmonicFill, Pad) sit static.
- **Affect** finds variety and voice-count reduce to ONE perceptual requirement —
  *differentiation the listener can hear* — and that density only pays off behind a
  stream-segregation precondition; per-step variety on undifferentiated voices is wash (the
  documented S13 trap).
- **Aesthetics** finds the planner picks ONE orchestration profile and `.clone()`s it onto every
  section (`src/composition.rs:1623`, selection at `:1517`), so the piece has texture but **no
  texture ARC** — variety is not deployed in time.

The synthesis: **wake / route / deploy the variety that exists, in that order, before adding any
voice.** The single S45 first slice (§6) is the change that all four lenses' first-slice
candidates collapse into once the §2 ground truth is settled.

---

## 2. THE RECONCILED CODE GROUND TRUTH — the CounterMelody discrepancy, resolved

**The discrepancy.** Affect (`design-s44-affect.md` §1.2, §2.3, §7) states CounterMelody is
**STUBBED — it delegates to the HarmonicFill held tone at `src/chord_engine.rs:869-872`** and is
therefore "a mud voice" today. Music Theory (`design-s44-theory.md` §0, §1.2) states CounterMelody
is **the only fully-developed independent line in the engine** — a real species-counterpoint voice
(passing/neighbor/suspension, parallel-perfect avoidance, contrary/oblique motion, cadential
clausula, held-period activation) at `src/chord_engine.rs:4117–4660`. Both read the same file.

**GROUND TRUTH (read directly this session): Music Theory is correct. Affect is reading a STALE
DOC COMMENT, not the live realization path.** Precisely:

1. **The "stubbed" prose is a stale Pass-A artifact in two places.** The `OrchestralRole::CounterMelody`
   enum doc comment (`src/chord_engine.rs:869-873`) and the `role_pitch` inner-arm comment
   (`:1264-1270`) both say the CounterMelody "delegates to the HarmonicFill figure" / "stub
   delegates to the fill figure for now." These comments describe the *original* Pass-A state and
   were **never updated** after the real line was wired in.

2. **The real species voice IS wired into the live default realize path — in `realize_rhythm`, not
   `role_pitch`.** `realize_step` (`:1076`) computes a `base_note` via `role_pitch` (`:1137`) — and
   for the CounterMelody role that single seat IS still the stale inner-fill anchor (`:1271-1297`).
   **But that anchor is immediately OVERWRITTEN.** Control flows to `realize_rhythm` (`:1191`), whose
   `OrchestralRole::CounterMelody` arm (`:1818-1894`) is explicitly headed *"THE REAL COUNTER-LINE
   (S18 §3) — a genuine second moving line, not the HarmonicFill delegate the stub was."* That arm:
   - recomputes the melody pitch this/prev step (`melody_pitch_for` / `melody_pitch_for_step`,
     `:1833-1834`) for contrary-motion,
   - seeds the realized previous counter pitch by deterministic replay
     (`realized_prev_counter`, `:1859`; the S30-CP-FIX realized-prev memory),
   - selects the actual counter pitch through the SHARED
     `realized_counter_pitch_with_prev` (`:1863`, defined `:3538`), which dispatches the
     fifth-species figure driver `pick_counter_figure` (`:4652`) gated by the species predicates
     `is_legal_passing/neighbor/suspension/cambiata` (`:4141`–`:4259`) and the consonant-sustain
     scorer `pick_counter_pitch` (`:3815`),
   - and **rebinds every emitted event's pitch to that counter pitch** via
     `with_note(ev) = NoteEvent { note: cnt, ..ev }` (`:1865, :1875, :1881, :1891`),
   - with held-period activation (`:1868-1876`) — the guaranteed off-beat onset that "fills the
     operator's empty period" — and oblique/rest modes.
   The cadence ring is handled separately and ALSO species-correct: the `is_cadence` branch at
   `:1604-1630` recomputes the counter's contrapuntal cadence pitch through the same shared path
   before ringing it (S30-CP-FIX GAP-3).

3. **So the `role_pitch` seat is a dead anchor for CounterMelody — overwritten on every emitted
   event.** The "stub" lives ONLY in the comment and in the unused `role_pitch` return value; the
   *sounding* CounterMelody is the full species line.

**The reconciliation that matters for the work order:** the question "un-stub vs route the existing
gated voice into the default" — **these are the SAME edit described two ways, and neither is a
realization rewrite.** The species realization already exists and already fires *whenever a
CounterMelody-role instrument is present*. The thing that is missing is **selection**: the DEFAULT
texture profile `pad_bed` (`assets/mappings.json:264`) carries layers
`["Bass","Pad","HarmonicFill","Melody"]` — **no CounterMelody** — and the only profile that names
CounterMelody, `pad_bed_counter` (`:265`), is gated on `foreground_energy ≥ 0.35 AND
fg_bg_contrast ≥ 0.20` (`texture` SelectTable, `:343-345`), which neither S42 image cleared. **The
richest line in the engine is dark on the very images the operator listens to — because it is never
SELECTED, not because it is stubbed.** Affect's *perceptual requirement* (distinct
grid/register/level) is correct and already satisfied by the species arm; Affect's *factual claim
that it renders as a Fill held tone* is wrong against the live code. The first slice is therefore a
**routing/selection** change (freeze-safe JSON + a one-line `role_pitch` anchor cleanup), not a
realization build.

**Side correction for the ledger:** Music Theory says "6 of 8 `ThemeVariation` variants dead." The
enum actually has **9 variants** (`src/composition.rs:415-425`: Identity, Transposed, Reharmonized,
Augmented, Diminished, Ornamented, Fragmented, Inverted, Retrograde); `clamp_variation_slice1`
(`:1786-1791`) passes only `Fragmented` and forces all others to `Identity` — so **7 of 9 are dead
at plan time** (Transposed, Reharmonized, Augmented, Diminished, Ornamented, Inverted, Retrograde).
The substance of the finding is unchanged and sharper: the theme-variation vocabulary is the most
clamped layer in the engine.

### 2.1 What is actually dormant vs static vs missing (all five roles + rhythm/harmony/form)

| Layer / dimension | State today | Precise evidence (file:line) |
|---|---|---|
| **CounterMelody** | **DORMANT** — full species line built and live, but the default profile never selects it (gated) | realize arm `chord_engine.rs:1818-1894`; cadence `:1604-1630`; species engine `:3538`,`:4141-4259`,`:4652`; gate `mappings.json:343-345`; default `pad_bed` omits it `:264` |
| **HarmonicFill** | **STATIC** — one sustained inner tone, ~every step; "rest-as-gesture" the only variety | rhythm arm `chord_engine.rs:1716-1737` (Fill); is the default inner layer in `pad_bed` `mappings.json:264` |
| **Pad** | **DORMANT-then-CLONED** — 11 figures exist, ONE chosen per plan, then run as a ~32-bar ostinato | `figuration_catalogue` `mappings.json`; resolved once per plan `composition.rs:1525-1530`; cloned flat `:1623` |
| **Bass** | **DORMANT** — walking/pedal generators built and dispatched, but no `texture` rule selects `pad_walking`/`pad_pedal`; default is sustained root | dispatch `chord_engine.rs:1646`; generators `walking_bass`/`pedal_bass`; default sustained `_` arm; no selecting rule in `mappings.json` texture table |
| **Melody** | **FOREGROUNDED (S43) but vocabulary-thin** — chord-tones only, rhythm shares the bed grid within a band | rhythm arm `chord_engine.rs:1896+`; pitch is top chord tone `role_pitch` Melody arm `:1248-1262` |
| **Rhythm (cross-cut)** | **STATIC-ish** — one global `edge_activity` scalar gates all roles; per-section density nudge exists but is 0.5 on identity | `edge_activity` `chord_engine.rs:1504-1515`; density term `:1513` |
| **Harmony / theme (cross-cut)** | **CLAMPED** — per-step color is rich (S13); 7 of 9 `ThemeVariation` variants dead at plan time | enum `composition.rs:415-425`; `clamp_variation_slice1` `:1786-1791` |
| **Form / texture (cross-cut)** | **CLONED FLAT** — one orchestration profile selected per plan, `.clone()`d onto every section → NO texture arc | selection `composition.rs:1517`; clone `:1623`; per-section `density` seam exists `:1606/1624` |

**Net:** only ONE thing the operator's arc asks for is genuinely *missing* (per-instance inner-voice
differentiation past the chord-tone count — Architect §1.3). Everything else is **dormant, static,
clamped, or cloned** — present capability that is not being selected or deployed.

---

## 3. PER-LAYER VARIETY GAP MAP (merged across the four lenses)

Tier legend (the load-bearing freeze seam, identical across all four lenses): **[JSON]** =
`assets/mappings.json`, zero-Rust, **freeze-safe** · **[CE]** = `src/chord_engine.rs`, Rust
realizer, **freeze-reachable** (kernel only calls it; identity path byte-neutral) · **[COMP]** =
`src/composition.rs`, Rust planner, **freeze-reachable** · **[FROZEN]** = `src/engine.rs`,
frozen-kernel value decision.

| Layer | Variety gap (merged) | Lever | Where the lever lands (file:line) | Tier |
|---|---|---|---|---|
| **CounterMelody** | Built, live, species-correct — but never selected on ordinary images | Route it into the default inner slot: relax the `pad_bed_counter` gate and/or give `pad_bed` a CounterMelody layer; clean the stale `role_pitch` anchor comment | gate `mappings.json:343-345` (+`:264-265`); anchor `chord_engine.rs:1264-1297` | **[JSON]** (route) + **[CE]** (anchor cleanup) |
| **HarmonicFill** | Static sustain — the deadest layer; affect says do NOT invest variety budget here | Replace it as the *default* inner voice with the moving CounterMelody (above); keep Fill as the low-energy fallback | `mappings.json` texture rows; rhythm arm `chord_engine.rs:1716-1737` | **[JSON]** primarily |
| **Pad (figuration)** | One figure per plan → 32-bar ostinato (the "same piece" identity-fixer) | Select figuration **per section** instead of once per plan (block A → broken B → block A′) | resolve `composition.rs:1525-1530`; clone `:1623` | **[COMP]** (per-section selection) + **[JSON]** (any new figures) |
| **Bass** | walking/pedal generators dormant; default-bed bass never moves between roots | Add `texture` rules selecting `pad_walking`/`pad_pedal`; add a default-bed passing-tone arm | `mappings.json` texture rules; `chord_engine.rs:1646` bass dispatch | **[JSON]** (route) + **[CE]** (passing-tone arm) |
| **Melody** | chord-tones only; rhythm fused to bed grid within a band | Non-chord tones (passing/neighbor/appoggiatura — reuse species predicates); per-role rhythm bias generalizing `prom_shift` | melody arms `chord_engine.rs:1896+`; pitch `:1248-1262` | **[CE]** |
| **Theme variation** | 7 of 9 variants dead at plan time | Unclamp the high-value three (Reharmonized, Ornamented, Augmented/Diminished) | `clamp_variation_slice1` `composition.rs:1786-1791` | **[COMP]** |
| **Per-section density** | computed but flat 0.5 on identity; the `(density−0.5)*GAIN` activity term is dormant | Drive density off image region energy (already half-wired, `:1614-1624`) and let it shape figuration activity per section | `composition.rs:1606/1614-1624`; activity nudge `chord_engine.rs:1513` | **[COMP]** |
| **Texture ARC (form)** | one profile cloned flat onto every section → no departure-and-return in the fabric | Select/derive the orchestration profile **per section role** (sparse Statement → thin Contrast → full Return → stripped Coda) | selection `composition.rs:1517`; clone `:1623`; role enum `:406-412` | **[COMP]** (+ **[JSON]** rows for new profiles) |
| **Inner-voice per-instance differentiation** | the one genuinely MISSING piece: 3rd/4th inner voice doubles tones in one band at one prominence | `VoiceAssignment` handle + sub-band the inner register + per-instance prominence falloff | `role_pitch`/`assign_role` `chord_engine.rs:1038/1216-1297`; prominence `:1013` | **[CE]** (+ **[JSON]** data) |
| **Ensemble width default** | `--instruments` works; default 4 | Change the default value (recommended AGAINST until differentiation lands) | `EngineConfig::default` `engine.rs:203` | **[FROZEN]** ⚠ |

**The load-bearing freeze fact (all four lenses agree):** `engine.rs` holds only width-AGNOSTIC
plumbing for N voices — the `num_instruments` field/default (`:188/:203`), the `SetInstruments`
handler (`:633`), and the scan-row truncate/pad to N (`:496/:850`). **The only frozen thing that is
a musical decision is the default value `4`.** Everything that makes voices SOUND different lives
outside the freeze.

---

## 4. THE N-VOICE QUESTION, ANSWERED

**">N voices meaningfully" requires four things, contributed one per lens, and ALL are downstream
of variety-first:**

1. **(Architecture) A per-instance differentiation handle + a distinctness invariant.** The
   `VoiceAssignment { role, voice_in_role, voices_in_role }` type (Architect §3.1) carries each
   inner voice's index within its role-group so the realizer can make the 3rd inner voice DIFFER
   from the 1st (sub-band register, spread chord tones/octaves) instead of wrapping modulo
   chord-tone count (`role_pitch` `:1279-1286`, which today doubles tones at n≥5). The encodable
   **voice-distinctness invariant**: no two instruments on a step share an identical
   `(register-band, primary chord tone, onset grid)` triple unless a deliberate octave doubling.

2. **(Music Theory) Real added ROLES with contrapuntal function, not copies.** A new line earns
   its place only by a function no existing line performs: **Descant** (above the melody, climax
   only), **upper pedal** (stasis under change), **second B-section countermelody**, **split
   walking bass**, **obbligato** — each under the N-line voice-leading properties P1–P7 (register
   stratification, no parallel perfects across ALL pairs, common-tone retention, tendency-tone
   resolution, doubling rules, one-foreground-at-a-time, outer-voice contrary-motion bias). A
   sustained chord tone in an occupied band is the doubling trap, forbidden by P5.

3. **(Affect) A stream-segregation precondition.** Each added voice must differ from every
   concurrent voice in ≥1 strong segregation cue — **level (loudness) > rhythm-grid > register >
   articulation** — or it FUSES into mud (the S42 melody+pad failure recreated). Density is only a
   MEDIUM-confidence arousal cue, conditional on this; and the budget must be **affect-gated** off
   the existing `affect_arousal` composite (calm → sparse, energetic → dense), so variety does not
   become a new uniformity.

4. **(Aesthetics) Deployment in time.** Added voices pay off only DEPLOYED at the climax — a 5th
   line that enters at the Return is an arrival; one present everywhere is back to the flat trap. A
   constant full texture is as monotonous as a constant thin one and has no dynamic range left to
   bloom.

**The frozen-default recommendation (unanimous): KEEP the default `num_instruments` at 4.**
`--instruments N` and `SetInstruments(n)` already deliver richer N without editing the frozen file;
raising the default would re-baseline the `engine.rs` byte-freeze and would ship the
thin-multiplication trap to every render *before* the differentiation precondition exists. Revisit
only after slices 2–4 land, as a deliberate, flagged, one-value freeze edit — or leave it at 4 and
let `--instruments` carry richer N.

**Why N is downstream of variety-first:** the texture must first MOVE in four differentiated voices
before a fifth has something to be independent *from*. A descant over a static held bed and a
root-thumping bass is a fifth line decorating a thin texture — the anti-pattern with a bigger
number. Items in §6 slices 1–4 add zero net voices (or wake one already-built voice) and are each
higher-yield than any added voice.

---

## 5. FREEZE LEDGER — per proposed change

| Proposed change | Site | Touches `engine.rs`? | Verdict |
|---|---|---|---|
| Route CounterMelody into the default (relax `pad_bed_counter` gate and/or `pad_bed` layer swap) | `mappings.json` | NO | **FREEZE-SAFE** — additive/edited catalogue + SelectTable rows; loader parses the shape |
| Clean the stale `role_pitch`/enum "stub" comments + (optional) dead anchor | `chord_engine.rs:869-873, 1264-1270` | NO | **FREEZE-REACHABLE** — comment-only / dead-value; the species realize path is unchanged; identity path has no Counter inst (byte-neutral) |
| Per-section figuration selection | `composition.rs` planner | NO | **FREEZE-REACHABLE** — realizer already reads resolved figuration per section; identity profile resolves to None (byte-stable) |
| Unclamp high-value `ThemeVariation` (3 variants) | `composition.rs:1786-1791` | NO | **FREEZE-REACHABLE** — identity sections carry `Identity`/`Fragmented`; unclamping only affects non-identity compose-path sections |
| Image-driven per-section density | `composition.rs:1614-1624` | NO | **FREEZE-REACHABLE** — `f(0.5)==0.5` on identity → byte-stable; activity nudge already centered at 0 |
| Role-aware texture ARC (per-section profile select/derive) | `composition.rs:1517/1623` | NO | **FREEZE-SAFE shape** — identity plan still clones one identity profile; only the non-identity compose path gains per-role selection |
| Route `pad_walking`/`pad_pedal` + default-bed bass passing tones | `mappings.json` + `chord_engine.rs:1646` | NO | **FREEZE-SAFE** (rules) + **FREEZE-REACHABLE** (passing-tone arm; `_` arm is the freeze default, byte-unchanged) |
| Melodic non-chord tones | `chord_engine.rs` melody arms | NO | **FREEZE-REACHABLE** — additive; identity melody path unchanged |
| `VoiceAssignment` + `assign_voice` + per-instance `role_pitch`/`prominence_weight` | `chord_engine.rs` | NO | **FREEZE-REACHABLE** — `realize_step` PUBLIC sig unchanged; identity returns `voices_in_role==1` → every per-instance term is the centered no-op, identity bytes preserved (the prominence-neutral precedent) |
| `layer_widths: Option<Vec<u8>>` schema | `composition.rs` (serde default) + `mappings.json` | NO | **FREEZE-SAFE schema** — `#[serde(default)]` keeps every existing profile byte-shape-stable (the figuration/bass_pattern precedent) |
| N-voice inner-vs-inner voice-leading | `chord_engine.rs` | NO | **FREEZE-REACHABLE** — unreachable under identity; byte-neutral on the freeze path |
| **Change default `num_instruments` 4 → N** | `engine.rs:203` | **YES** | **FROZEN-KERNEL DECISION** ⚠ — must be a deliberate, flagged freeze edit; **recommended AGAINST** until §6 slices 2–4 land |

**The only frozen-kernel question in the entire arc is whether to change the default `4`** — and
§4 + §6 recommend NOT changing it.

---

## 6. RANKED, SLICED BUILD PLAN (the heart) — variety-first, anti-pattern last

The four lenses each proposed a first-slice candidate. They reconcile cleanly because they are
addressing different *layers* of the same dormancy, and the §2 ground truth settles the one that
looked like a realization build into a routing change:

- **Music Theory:** route CounterMelody into the default inner slot + per-section Pad figuration.
- **Affect:** un-stub CounterMelody (→ §2: it is already built; this IS the routing change).
- **Aesthetics:** role-aware texture ARC (Statement→Return texture deployment).
- **Architecture:** per-instance inner-voice differentiation + voice-distinctness invariant.

These are **one ranked sequence**, not four competing proposals. The ranking is variety-first
(audible improvement per unit effort, with voice-count strictly after differentiation):

| # | Slice | Tier / Freeze | Dependencies | Audible win it buys | Owner |
|---|---|---|---|---|---|
| **1 (S45)** | **Route CounterMelody into the default inner voice + per-section Pad figuration.** Relax the `pad_bed_counter` gate (or swap `pad_bed`'s HarmonicFill for a CounterMelody layer above a low activity floor, keeping Fill as the calm fallback) so the already-built species line FIRES on ordinary images; select Pad figuration per section so the comp stops being a 32-bar ostinato. Includes the stale-comment cleanup at `chord_engine.rs:869-873, 1264-1270`. | **[JSON]** (route) + **[COMP]** (per-section figure) + **[CE]** (comment cleanup) — **FREEZE-SAFE / FREEZE-REACHABLE** | none (everything it needs is built) | The deadest layer (static Fill) is replaced by the richest existing line (the species counter); the comp stops repeating one cell — the inner texture MOVES and the piece stops sounding like "same piece, different key." | **Music Theory** (realization internals + figure-per-section) with **Affect** taste gate (distinct grid/register/level) |
| **2** | **Role-aware texture ARC.** Select/derive the orchestration profile per section role (sparse Statement → thinner/re-colored Contrast → fullest Return → stripped Coda) at the `composition.rs:1517` seam instead of cloning one flat profile (`:1623`); drive per-section density off image energy (`:1614-1624`). Aesthetics recommends **derivation** (image picks the climax profile; the form derives the sparser non-climax profiles) so the operator's tuned image→texture mapping is preserved and the arc layers on top. | **[COMP]** (+ **[JSON]** rows authored in this slice) — **FREEZE-SAFE shape** | builds on #1 (the inner voice the arc deploys must already move) | Departure-and-return in the FABRIC, not just the harmony: the piece opens with room, contrasts in its middle, and BLOOMS at the return. Gives the eventual added voices somewhere to ARRIVE. | **Aesthetics** (arc shape) + **Architecture** (select-vs-derive seam) |
| **3** | **Unclamp the high-value theme variations + route walking/pedal bass + melodic non-chord tones.** Lift `clamp_variation_slice1` for Reharmonized/Ornamented/Augmentation-Diminution (`composition.rs:1786`); add `texture` rules selecting `pad_walking`/`pad_pedal` and a default-bed bass passing-tone arm; add passing/neighbor/appoggiatura to the melody (reusing the species predicates). | **[COMP]** + **[JSON]** + **[CE]** — **FREEZE-REACHABLE** | builds on #1–#2 (variety organized by the arc reads as "more to say," not wash — Affect's S13 lesson) | The return becomes a transformation not a photocopy; the bass starts to MOVE; the melody stops being a pure arpeggio of chord tones. | **Music Theory** (theme/harmony/melody craft) + **Affect** (affect-gating each lever) |
| **4** | **Per-instance inner-voice differentiation (`VoiceAssignment`) + the first REAL added voice (Descant), deployed at the climax.** Introduce `VoiceAssignment` and sub-band the inner register / spread chord tones per instance with the voice-distinctness invariant (the one genuinely MISSING capability); then add the Descant at the Return/climax only, under P1/P2/P6/P7 and the Affect segregation guard. | **[CE]** (+ **[JSON]** data) — **FREEZE-REACHABLE** (identity returns `voices_in_role==1` no-op) | builds on #1–#3 (a new line needs a moving 4-voice texture to be independent FROM, and an arc to enter into) | `--instruments 6` produces differentiated lines, not copies; the Return gains a soaring descant — voice-count, AFTER variety. | **Architecture** (`VoiceAssignment`/distinctness) + **Music Theory** (descant counterpoint) + **Affect/Aesthetics** taste gate |
| **5 (ANTI-PATTERN — LAST, FROZEN-GATED)** | **Raise the default `num_instruments` above 4.** Only ever after slices 1–4. A deliberate, flagged one-value frozen-kernel edit (`engine.rs:203`) — or simply leave the default at 4 and let `--instruments` carry richer N. **Naive `num_instruments`-bump without slices 1–4 is the explicit anti-pattern: it multiplies a thin texture and re-buries the melody S43 surfaced.** | **[FROZEN]** ⚠ | ALL of slices 1–4 | (none on its own — value-only) | Operator decision; **recommended: leave at 4** |

### The single recommended S45 first slice

**Slice 1 — route the existing CounterMelody species voice into the default inner voice, plus
per-section Pad figuration.**

**Justification.** It is the exact analogue of the S43 decision: S42/S43 foregrounded the melody,
and the predicted next audible defect was the thinness underneath. The §2 ground truth makes this
slice cheap and decisive: the richest, most musically complete line in the engine ALREADY EXISTS,
is ALREADY species-correct, and ALREADY fires whenever selected — it is dark only because the
default profile never selects it. Routing it in is the highest variety-per-edit move available; it
is **variety, not voice-count** (it makes an *existing always-on line MOVE*); it needs no new role,
no realization rewrite, and no frozen-kernel edit; and the per-section figuration change is the
cheapest kill for the ostinato identity the S42 trace blamed for "same piece, different key." Three
of four lenses named exactly this (Music Theory and Affect explicitly; Aesthetics' texture-arc is
its natural successor in slice 2); Architecture's per-instance differentiation is correctly slotted
later (slice 4) because it is the precondition for N voices, not for the *next audible win*.

---

## 7. CROSS-LENS DEPENDENCIES + OPEN DECISIONS FOR THE OPERATOR

**Cross-lens dependencies (load-bearing ordering):**
- Slice 4's added voices DEPEND on slices 1–3: a new line needs a moving 4-voice texture to be
  independent from (Music Theory + Affect) and an arc to enter into (Aesthetics).
- The N-line voice-leading enforcement (P2/P4/P7) wants a post-pass / shared voice-leading context
  that sees all lines on a step — today pitches are realized per-instrument independently. This is
  the deepest reconciliation item; it can be STAGED (P2 melody↔descant first, full N-pair later).
- Every variety lever must be **affect-conditioned** (Affect) and **form-deployed** (Aesthetics),
  or it becomes a new uniformity — this is a binding ship-rule, not a tuning preference.
- `mappings.json` is a shared writer across Music Theory and Aesthetics; row authorship for new
  texture profiles is deferred to the BUILD slice under single-writer coordination (Aesthetics
  appendix), exactly as S42 did.

**Open decisions for the operator:**
1. **S45 inner-voice route — the load-bearing fork (Music Theory DP-1):** (a) relax the
   `pad_bed_counter` selection gate (conservative — HarmonicFill stays the calm fallback), or (b)
   swap the static HarmonicFill out of the default `pad_bed` for a CounterMelody layer (decisive).
   *Lens lean: (a) first* — keep a quiet held bed for the calmest images, route the moving voice in
   above a low activity floor.
2. **Frozen default voice count:** keep at 4? *Unanimous lens recommendation: YES, keep at 4.*
3. **Texture-arc mechanism (Aesthetics §5.1 / Architect):** role-keyed ladder in the `texture`
   table, or planner-side derivation from the image-selected climax profile? *Aesthetics leans
   derivation (b)*; Architecture to confirm which is cleaner in the type model.
4. **How far to unclamp `ThemeVariation`:** all three high-value variants at once
   (Reharmonized + Ornamented + Augmentation/Diminution), or stage Reharmonized first? *Music
   Theory leans all three* (independent, each a real within-layer win).
5. **Descant deployment scope (slice 4):** Return/climax only (keeps P6 trivially satisfiable), or
   also B? *Music Theory leans return/climax only.*
6. **Split walking bass — ship at all?** REAL but the most register-fraught. *Music Theory leans
   defer* (single moving bass via walking/pedal/passing-tones is enough).
7. **N-line register model:** per-active-line register slots assigned by the orchestration profile
   (enables >5 distinct lines), or keep per-role-type floors (caps at one-of-each-role)? *Architect
   + Music Theory lean per-active-line slots* — the clean generalization.
8. **Affect/aesthetics taste-gate watch-item carry-forward:** the S43 verdict pre-staged a Lena
   melody lift (0.78→0.85) and a CounterMelody-blur weight tweak (0.58→0.55). The slice-1 routing
   CHANGES the perceptual basis of the 0.55 tweak (it now applies against a *live* counter line, no
   longer a phantom) — *resolve slice 1 first, then re-evaluate the weight against a real counter,
   per Affect DP-4.*

---

## 8. GATES — the in-class cadence per build slice

Every build slice runs the **standing S43 cadence**, unchanged:
1. **Correctness gate** — the encodable invariants for the slice: the **voice-distinctness
   invariant** (slice 4), the **texture-arc guard-rails** (Aesthetics §4: texture-actually-varies,
   sparseness-floor, variety-serves-contrast, no-uniform-tutti, climax-is-earned, coda-recedes,
   bed-never-vanishes, identity-path byte-frozen — slice 2), the **per-voice differentiation CI
   guard** (Affect §5: every added voice differs in ≥1 of level / rhythm-grid / register —
   slices 1 & 4), and the carry-forward S43 foreground invariants (resolved Melody prominence > 0.5;
   bed roles recede but stay > 0.25 — every slice).
2. **The standing taste/affect + aesthetics gate** — summoned into the cadence as a standing gate
   beside correctness (per the Specialist Marshaling Gate), NOT an optional end-of-slice ear-test.
   This is generative/aesthetic work; the relevant specialists (Affect/Cross-Modal review +
   Aesthetics review) are in inventory and MUST be in the build cadence.
3. **The operator ear-test** — the A/B render at `--seed 42` with and without the change, against
   the one detection question: *would a listener hear the difference?* (Affect §5): can they point
   to the new/moved line as a separate thing; did perceived energy move in the affect-intended
   direction; did the foreground get HARDER to follow (→ fusion, revert). Exactly as S43 did.

**Freeze discipline on every slice:** `src/engine.rs` stays byte-frozen at sha256
`e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261`; every per-instance/per-section
term is centered on the identity no-op so the byte-freeze holds, the discipline that has kept the
kernel frozen across S17–S43.

---

*End of S44 unified synthesis. Design-only: no source, test, or asset modified. `src/engine.rs`
sha256 re-verified UNCHANGED at session end: `e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261`.*

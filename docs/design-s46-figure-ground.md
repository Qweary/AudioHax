# S46 — Figure-Ground / Role-Balance: Unified Assessment + Work Order

**Author role:** Rust Architect (SYNTHESIS round of the S46 design cadence).
**DESIGN ONLY — no source, test, or asset modified by this document.** All Rust shown is
signatures / types / doc comments — **no bodies**.
**Date:** 2026-06-19
**Synthesizes** the four S46 lens designs: `docs/design-s46-architect.md` (Architecture),
`docs/design-s46-theory.md` (Music Theory), `docs/design-s46-affect.md` (Affect/Cross-Modal),
`docs/design-s46-aesthetics.md` (Aesthetics). Mirrors the S44 unified work-order
`docs/design-s44-variety-nvoice.md` in shape.
**Grounded against** the working tree at HEAD: `src/engine.rs` (**BYTE-FROZEN**, sha256
`e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261` — re-verified unchanged at
the **start AND end** of this session), `src/chord_engine.rs`, `src/composition.rs`,
`assets/mappings.json`, `tests/variety_scorecard_s45.rs`. Every cited file:line below was read
directly this session to adjudicate the cross-lens code claims in §2.

> **The binding frame this synthesis carries (the lead's load-bearing counterpoint).** The fix
> is NOT "turn the melody up." Level is the **last 10%** and is the one axis the engine ALREADY
> balances (S43). The first 90% is **DIFFERENTIATION**: the melody MOVES MORE (activity) + the
> backgrounds RECEDE IN ACTIVITY + each role has its own RHYTHMIC IDENTITY + the melody sits ON
> TOP. A loud melody over an equally-busy bed is still mush. And: **do NOT walk back S45** — the
> CounterMelody S45 routed in is a *gain* (the inner texture finally moves). Resolve the
> inversion as a **HIERARCHY** — the counter recedes in activity/register *relative to a melody
> lifted above it*, preserving the moving inner texture while the melody wins the foreground. A
> blanket per-image counter re-suppression is forbidden; a deliberate per-image hierarchy
> recession is the only legitimate recession.

---

## 1. EXECUTIVE SYNTHESIS — the one governing finding across all four lenses

**S45 woke the right line (the CounterMelody), but the engine enforces NO figure-ground
HIERARCHY — so the melody and the counter now compete as EQUALS on the two cues the ear weights
most, ACTIVITY and REGISTER-ORDER, and on BOTH the engine currently favors the background. The
S46 defect is therefore an ACTIVITY-and-REGISTER-ORDER inversion, NOT a level inversion — and
level is the one axis already balanced.** All four lenses converge on this from different angles:

- **Architecture** locates three concrete, freeze-reachable inversions, none in `engine.rs`:
  (1) an **ACTIVITY inversion** — on a calm image the Melody arm falls to SUSTAINED
  (`chord_engine.rs:1974-1992`) while the CounterMelody arm takes a GUARANTEED off-beat onset
  (`:1899-1907`) on exactly the held/static steps, so the background moves and the foreground
  holds; (2) a **REGISTER-ORDER fragility** — `MELODY_REGISTER_FLOOR=67` (`:1222`) abuts
  `COUNTER_CEILING=67` (`:3478`) with no melody≥counter invariant, and a dark-image brightness
  lift can be negative (`:1263`), pulling the melody into the counter band; (3) an
  **INVERSE-COMPENSATION absence** — the prominence register nudge (`:1269-1271`) is forward and
  register-blind.
- **Music Theory** states the same fact as a missing ordering law: the role hierarchy the engine
  should enforce per dimension is **Bass < bed/fill < Counter < Melody** in ACTIVITY and
  REGISTER, and **Bass ≤ Counter < Pad/Fill < Melody** in LEVEL — and on activity/register the
  engine today does nothing or works against the melody.
- **Affect** supplies the perceptual ranking that proves "level is weakest": for this
  timbre-flat synth texture the segregation cues rank **rhythmic-grid/onset-rate (1) >
  onset-asynchrony/motion (2) > register (3) > articulation (4) > LEVEL (5)** — the engine
  over-invests cues 3+5 and barely touches cue 1 (`PROMINENCE_RHY_SHIFT=0.10` yields only a
  0.028 cutoff shift at weight 0.78), and attention is **event-driven** so a quiet-but-busy bed
  keeps competing onset-for-onset.
- **Aesthetics** names the lived defect: the piece has a melody-ROLE but no FIGURE — the melody
  has the register assignment and level bias of a lead but not the *behavior* of one — and warns
  that the fix must be **image-conditioned**: a clear-subject image justifies a strong lead, a
  field/abstract image justifies a more even texture, so the scorecard must reward RELATIONAL,
  image-conditioned differentiation, never an absolute "melody always busiest/highest/loudest"
  (which would itself sound mechanical).

The synthesis: **establish the figure-ground HIERARCHY on the strong cues first (activity, then
register-order), preserve S45's moving counter by receding it RELATIVE to a lifted melody,
condition the whole thing on the image, and touch level last.** The single S47 first slice (§5)
is the change all four lenses' first-slice candidates collapse into once §2's ground truth is
settled. `engine.rs` stays byte-frozen — every lever is in `chord_engine.rs` (realizer) +
`assets/mappings.json` (data) + `tests/` (scorecard).

---

## 2. THE RECONCILED CODE GROUND TRUTH — the inversion sites, pinned

The four lenses agree on the mechanism; they differed only in three secondary code claims, each
adjudicated below by direct read this session. (Unlike S44, there is **no stubbed-vs-live
discrepancy** to resolve — all four lenses read the CounterMelody as the live species line, the
S44 correction having stuck.)

### 2.1 The three inversion sites (pinned, read directly)

**(A) ACTIVITY inversion — the DOMINANT defect.** Traced end-to-end on a calm image (low
`edge_activity`), confirmed at the cited lines:

```
realize_rhythm(role = Melody, …)            chord_engine.rs:1927
  edge_activity low → falls through the 0.80/0.55/0.25 cutoffs (:1943/:1956/:1965, each
  shifted by prom_shift :1941-1942) → SUSTAINED arm (:1974-1992) → ONE long held tone, offset 0.

realize_rhythm(role = CounterMelody, …)     chord_engine.rs:1831
  held_chord || melody_static  (:1899)      ← TRUE precisely when the melody is calm/static
  → MOVING mode: a GUARANTEED off-beat onset at step_ms/4 (:1903-1907) → the counter MOVES.
```

The two arms read the **same** `edge_activity` and resolve OPPOSITELY: low activity makes the
melody hold (`:1991`) and makes the counter move (`:1907`). The counter's held-period activation
was designed to "fill the operator's empty period" (`:1902`) — but the empty period it fills is
the *melody's*, so it fills the foreground's silence with a *background* line. This is the
figure-ground inversion in two adjacent match arms. Note the counter's OBLIQUE arm (`:1908-1912`,
one sustained tone when `edge_activity > 0.55`, i.e. the melody is active) and its
rest-as-gesture arm (`:1919-1920`) already exist — they are the recession modes the governor will
route into when the melody holds.

**(B) REGISTER-ORDER fragility.** `MELODY_REGISTER_FLOOR = 67` (`:1222`); the melody seats UP
from 67 with `lift = (bright_octaves * 12.0).round()` where `bright_octaves ∈ [−1,+1]` (`:1246`,
`:1263`), then `prom_lift` (≤ +2 at full foreground, `:1269-1270`), summed under a single
`.clamp(24, 96)` (`:1271`). `COUNTER_CEILING = MELODY_REGISTER_FLOOR = 67` (`:3478`); the counter
band is `[FILL_REGISTER_FLOOR(55), 67)`. On a dark image `lift` can be `−12`, so the melody floor
resolves to `67 − 12 + prom_lift ≈ 55–57`, **inside the counter band**. There is **no
`melody_seat > counter_ceiling` invariant** anywhere — "melody on top" is an emergent accident of
brightness, not enforced.

**(C) INVERSE-COMPENSATION absence.** The only register-scaling tool is `prom_lift`
(`:1269-1270`), a FORWARD, fixed, weight-keyed lift that is **blind to where the melody actually
landed**. A melody seated LOW self-projects LEAST and most needs help, but nothing routes more
NON-LEVEL help (articulation/rhythmic separation) to a low-seated melody. The compensation the
operator wants is INVERSE to the realized seat; the engine has only a forward, register-blind
lift.

### 2.2 The three secondary cross-lens claims, adjudicated by direct read

1. **"Prominence is under-deployed" (kickoff) vs "fully deployed but on the wrong axes"
   (Architecture).** ADJUDICATED: Architecture is right. `prominence_weight` is the table
   DEFAULT `melody_forward` (`mappings.json:381`), wired on all three axes — velocity (`:1404`),
   register (`:1269-1271`), and the melody's own rhythm-band cutoffs (`:1941-1942`). It is
   **deployed, not dormant.** The sharper truth: it operates on axes that cannot fix
   figure-ground — it shifts the melody's OWN band cutoffs against a fixed scalar (never relative
   to the counter, `:1942`), it nudges a floor (never enforces seat order), and it has no inverse
   compensation. The S46 fix adds the two axes prominence was never built to carry: **relative
   rhythmic activity** and **seat-order/inverse-register**.

2. **The magnitude of the activity lever.** Affect computes the live shift precisely: at melody
   weight 0.78, `prom_shift = (0.78 − 0.5) * PROMINENCE_RHY_SHIFT(0.10) = 0.028` (constants
   confirmed at `:1015`, `:985`). A 0.028 nudge on the 0.25/0.55/0.80 cutoffs cannot lift a
   calm-image melody out of the SUSTAINED band, so the melody still holds while the counter's
   activation fires unconditionally. Both lenses agree; the number is correct.

3. **The velocity-bias asymmetry (Affect/Aesthetics).** CONFIRMED at `:1384-1394`: Melody `+2`,
   Bass `−1`, Pad `−3`, and **HarmonicFill + CounterMelody fall through `_ => {}` (`:1393`) with
   NO bias arm.** So the level field is melody-forward via the Pad recession + the prominence
   nudge (`:1404`), but the counter has no negative level bias — its recession is only its 0.58
   weight. This matters for the level-tier slice (§5), not the first slice.

### 2.3 What this means for the work order

The dominant defect (activity) and the highest-correctness-per-line defect (register-order) are
BOTH `chord_engine.rs`-local, freeze-reachable, identity-byte-neutral, and **independent of each
other** — they can ship as one coupled slice. The level retune and the inverse-register
*scaling* are downstream refinements. The image-conditioning (Aesthetics) is a `mappings.json`
prominence-family expansion that rides ALONGSIDE the first slice (it changes WHICH weights the
realizer reads, not the realizer). `engine.rs` is untouched.

---

## 3. PER-DIMENSION FIGURE-GROUND GAP MAP (merged across the four lenses)

Tier legend (the load-bearing freeze seam, identical to S44): **[JSON]** = `assets/mappings.json`,
zero-Rust, **freeze-safe** · **[CE]** = `src/chord_engine.rs`, Rust realizer, **freeze-reachable**
(kernel only calls it; identity path byte-neutral) · **[COMP]** = `src/composition.rs`, Rust
planner, **freeze-reachable** · **[FROZEN]** = `src/engine.rs`, frozen-kernel value decision.

| Dimension (operator signal) | Today | First-class | Lever | Where it lands (file:line) | Tier |
|---|---|---|---|---|---|
| **ACTIVITY** (sig 2) — melody must move MOST | Melody → SUSTAINED on calm images; counter has a guaranteed off-beat onset → background out-moves foreground | Melody is the most-active line by construction; counter recedes one rank below it but keeps moving | `ActivityClass` ordering + `melody_activity_class`; govern the counter activation off the melody class; melody activity FLOOR | melody arm `chord_engine.rs:1943-1992`; counter activation `:1899-1924`; floor data `mappings.json` prominence | **[CE]** + **[JSON]** (floor field) |
| **REGISTER-ORDER** (sig 3) — melody on TOP | Seat order emergent: abutting floors 67/67, dark-image lift can be −12 → melody crosses into counter band | `melody_seat > counter_ceiling` is structural, not emergent | Seat-order guard: `.max(COUNTER_CEILING + MIN_FIGURE_GAP)` on the realized melody seat | melody seat `chord_engine.rs:1258-1272`; counter ceiling `:3478` | **[CE]** |
| **LEVEL** (sig 6) — the LAST 10% | ALREADY balanced: Melody +2 / Pad −3 bias + prominence nudge; melody_forward 0.78/0.58/0.40 | A small image-conditioned gap that WIDENS at climax; counter level recedes below melody | Retune `melody_forward` / add image-tiered profiles; add Counter/Fill negative bias arm | velocity bias `:1384-1394`; nudge `:1404`; weights `mappings.json:373-378` | **[JSON]** + **[CE]** |
| **PER-ROLE RHYTHM** (sig 7) — distinct onset grids | Five distinct arms exist but all key off ONE shared `edge_activity`; between-role/section reads flat | Each role on a distinct onset grid; between-section density arc | Per-role rhythm bias (generalize `prom_shift`); phase-separate bed onsets; per-section density off image | role arms `chord_engine.rs:1647-1994`; `edge_activity` `:1517`; density nudge `:1526` | **[CE]** + **[COMP]** |
| **BACKGROUND-RECESSION (activity)** (sig 5) | Bed recedes in LEVEL (Pad −3) but NOT in activity; counter's activation gives the bed a guaranteed onset the melody lacks | Bed onset-density + motion < melody's, image-conditioned depth | Counter activity governor (above) + Pad figuration weak-beat cap + Fill motion budget | counter `:1899-1924`; Pad `:1810-1828`; Fill `:1729-1746` | **[CE]** + **[JSON]** |
| **INVERSE-COMPENSATION** (sig 4) | MISSING — register nudge is forward + register-blind | A melody seated LOW gets MORE non-level help (articulation/rhythm separation), anti-correlated with seat height | `inverse_register_compensation(seat) -> f32` routed to articulation/rhythm, NOT level | new helper consumed in melody arm `chord_engine.rs` | **[CE]** + **[JSON]** (curve) |

**Confirmation for the ledger (correcting the kickoff).** Prominence is **fully deployed, not
under-deployed** (§2.2.1). The accurate statement: it is deployed on the LEVEL axis and weakly on
register/rhythm-band, but it operates on the WRONG axes for figure-ground. The fix is the two
missing axes (relative activity; seat-order/inverse-register), not a prominence re-deploy.

---

## 4. FREEZE LEDGER — per proposed change

| Proposed change | Site | Touches `engine.rs`? | Verdict (identity path byte-neutral) |
|---|---|---|---|
| `ActivityClass` enum + `melody_activity_class` pure helper | `chord_engine.rs` | NO | **FREEZE-REACHABLE** — pure helper; no Counter & neutral prominence under identity → never consulted on the freeze path |
| Govern the CounterMelody activation off the melody's activity class (hierarchy rule at `:1899`) | `chord_engine.rs:1899-1924` | NO | **FREEZE-REACHABLE** — counter arm is unreachable under identity (no Counter inst); byte-neutral; PRESERVES S45 (counter still moves when melody moves; recedes only relative to a holding melody) |
| Melody `activity_floor` (foreground melody never falls to Sustained) | `chord_engine.rs:1943-1992` (consume) + `mappings.json` prominence data (declare) | NO | **FREEZE-SAFE/REACHABLE** — `serde(default)` None == today; floor only bites for a >0.5 Melody weight; identity weight is 0.5 → no-op, byte-identical |
| Seat-order guard (`melody_seat > counter_ceiling`) | `chord_engine.rs:1258-1272` | NO | **FREEZE-REACHABLE** — additive `.max(...)` summed under the existing `.clamp(24,96)`; below the legacy bright/neutral seat under identity → no-op. **WATCH:** verify the identity render's `bright_octaves ≥ 0` against the `engine_equivalence` goldens; the counter is never on the identity path so the *relation* is moot there |
| `inverse_register_compensation(seat) -> f32` → articulation/rhythm separation (NOT level) | `chord_engine.rs` (melody arm) | NO | **FREEZE-REACHABLE** — additive, `!is_cadence`-guarded; neutral weight + high legacy seat → negligible/no-op on the freeze path |
| Image-conditioned prominence family (deep/mid/shallow recession tiers + SelectTable routing) | `assets/mappings.json:365-387` | NO | **FREEZE-SAFE** — additive catalogue rows + SelectTable rules; realizer already consumes resolved weights; identity profile (empty prominence) → neutral 0.5 |
| Counter/Fill negative velocity bias arm (level tier) | `chord_engine.rs:1384-1394` | NO | **FREEZE-REACHABLE** — `realize_velocity` role match, `!is_cadence` guarded (the S42 Edit-3 pattern); unreachable under identity |
| New scorecard figure-ground metrics F1–F5 + the F5 regression assertion | `tests/variety_scorecard_s45.rs` | NO | **FREEZE-SAFE** — test-only; the existing `engine.rs` sha guard (`:68`/`ENGINE_SHA256`) is unchanged and still passes |
| **Raise melody prominence weight 0.78→higher (the level lever, alone)** | `mappings.json:374` | NO | **FREEZE-SAFE but NOT the fix** — moves the weakest cue; do NOT lead with it (the operator's trap) |
| **Any edit to `engine.rs`** | `engine.rs` | YES | **FROZEN-KERNEL** ⚠ — none proposed; S46 needs none |

**The freeze fact for S46:** every figure-ground lever is freeze-reachable in `chord_engine.rs`
(realizer) + data in `assets/mappings.json` + test in `tests/`. `engine.rs` is not touched; sha
stays `e50c7db1…2348261`. (Contrast: the S45 spec §1b forward note flagged a *cross-section
counter-continuity* item that WOULD touch the frozen `StepContext` constructor — that is the one
deferred figure-ground-adjacent item that is NOT freeze-reachable, and it is explicitly NOT in
this slice plan; see §6 tension 4 of the S45 spec carry-forward.)

---

## 5. RANKED, SLICED BUILD PLAN (the heart) — figure-ground-first, level last

The four lenses each proposed a first-slice candidate. They reconcile into ONE ranked sequence,
not four competing proposals, because they address different *layers of the same hierarchy
absence* and the §2 ground truth settles the mechanism:

- **Architecture:** activity hierarchy (`ActivityClass` governor + melody activity floor).
- **Music Theory:** two coupled `[CE]` clamps — register floor ≥ counter ceiling + melody
  activity floor exceeding the counter's.
- **Affect:** re-weight the prominence budget toward cue-1 (rhythm/onset), recede the counter in
  activity by default.
- **Aesthetics:** an image-conditioned `[JSON]` prominence-recession family first.

**The reconciliation (tension #1).** These are NOT one undifferentiated slice and NOT four
separate slices. They are: a **first-class `[CE]` hierarchy core** (the first 90%) that the
**`[JSON]` image-conditioning rides alongside**, then ranked downstream refinements. The `[CE]`
hierarchy is the first-90% because it adds the two axes prominence cannot carry (relative
activity + seat order) — the cues the ear actually uses (Affect ranks them 1–3, above level).
The `[JSON]` recession family is freeze-safer and is the Aesthetics image-conditioning vehicle,
but ALONE it only re-weights LEVEL+register-nudge+the-melody's-own-cutoffs — the exact axes
§2.2.1 proved cannot fix the inversion. So the `[JSON]` family is necessary (it carries the
image-conditioning the metrics require, tension #2) but not sufficient; it ships **with**, not
before, the `[CE]` core.

| # | Slice | Tier / Freeze | Dependencies | Audible win it buys | Owner |
|---|---|---|---|---|---|
| **1 (S47)** | **THE FIGURE-GROUND HIERARCHY: melody-most-active + melody-on-top, image-conditioned.** Two coupled `[CE]` changes + the `[JSON]` family riding alongside: (a) `ActivityClass` ordering + `melody_activity_class` helper; govern the CounterMelody activation (`:1899`) so it recedes one rank below a holding melody (taking its existing oblique/rest modes), never out-moving the figure; add the prominence-keyed melody **activity FLOOR** (`:1943-1992`) so a foreground melody never falls to SUSTAINED on a calm image. (b) **Seat-order guard** `melody_seat > counter_ceiling` (`:1271`) so melody-on-top is structural. (c) The **image-conditioned prominence family** (`mappings.json:365-387`): replace the single `melody_forward` default with deep (subject) / mid / shallow (field) recession tiers routed by `fg_bg_contrast`+`subject_size`, so the hierarchy MARGIN tracks the image (Aesthetics + Affect §5). | **[CE]** (governor, floor, seat guard) + **[JSON]** (recession family + floor field) — **FREEZE-REACHABLE / FREEZE-SAFE** | none (everything is built or freeze-safe data) | The DOMINANT defect (activity inversion) dies on the STRONGEST cue, melody-on-top becomes guaranteed, and the separation MARGIN is image-justified — the melody arrives as the figure on subject images while a field image keeps its even texture. PRESERVES S45 (the counter still moves; it recedes only relative to the melody). | **Music Theory** (realization internals + role hierarchy) + **Architecture** (`ActivityClass`/seat-guard seam) with the **Affect + Aesthetics taste/affect gate** standing in the cadence |
| **2** | **PER-ROLE RHYTHMIC IDENTITY + between-section rhythm arc.** Generalize `prom_shift` (`:1941`) into a per-role rhythm BIAS so each role's band cutoffs differ (melody toward subdivision, bed roles away); phase-separate the bed onsets off the melody's strong beat; add a melody articulation contrast (a `base_frac` per-role bias beside the Fill special-case, `:1580`); drive per-section density off image region energy (`:1526`, `composition.rs` planner). | **[CE]** + **[COMP]** — **FREEZE-REACHABLE** | builds on #1 (the hierarchy must exist before the rhythmic stratification reinforces it) | The melody's irregular surface vs the regular bed is the THIRD figure cue stacked on slice 1's two; between-section rhythm stops reading flat (signal 7). | **Music Theory** (rhythm craft) + **Affect** (cue ranking) |
| **3** | **INVERSE-REGISTER COMPENSATION via NON-level tools + the level retune (the last 10%).** (a) Route `inverse_register_compensation(seat)` (`:1258-1272` realized seat) into the melody arm's articulation/rhythmic separation so a low-seated melody pops out without a louder level (operator signal 4 — the SUBTLE one). (b) THEN, only if the ear still wants it: widen the `melody_forward` Melody-vs-Counter gap (`mappings.json:373-378`) and add the Counter/Fill negative velocity bias arm (`:1393`). | **[CE]** + **[JSON]** — **FREEZE-REACHABLE / FREEZE-SAFE** | builds on #1–#2 (the melody must already lead in activity and sit on top before the low-seat compensation is audible) | The low-seated melody holds figure via separation, not loudness; the counter's LEVEL recedes below the melody with margin — the finishing differentiation. | **Affect/Aesthetics** taste gate (sizes the compensation curve + the weight) + **Music Theory** (consume) |
| **4** | **BED ACTIVITY RECESSION (figuration density + Fill motion budget).** Cap the Pad figuration to weak-beat onsets (off the melody downbeat) and/or reduce its per-step firing rate; hold the HarmonicFill activity budget firmly below the melody. | **[CE]** + **[JSON]** — **FREEZE-REACHABLE** | builds on #1–#3 (lifting the figure is higher-yield than lowering the ground; this is the complement that widens the gap from the bed side) | The "background recedes in ACTIVITY" half of signal 5, completing the full background-recession invariant (F5). | **Music Theory** + **Affect** |
| **5 (LEVEL-ONLY, ANTI-PATTERN IF LED WITH)** | **Raise the melody prominence weight alone.** Only ever as a finishing touch after 1–4 if the ear asks. Leading with it is the operator's confirmed trap (a loud melody over an equally-busy bed is still mush). | **[JSON]** — **FREEZE-SAFE** | ALL of 1–4 | (none on its own — the weakest cue) | Operator decision; **recommended: do not ship unless the ear asks after 1–4** |

### The single recommended S47 first slice

**Slice 1 — the figure-ground hierarchy: melody-most-active (activity governor + floor) +
melody-on-top (seat-order guard), with the image-conditioned prominence family riding
alongside.** Freeze tier: **[CE]** (governor, activity floor consume, seat guard) **+ [JSON]**
(recession family + the activity-floor data field). `engine.rs` untouched.

**Justification.** It is the exact figure-ground analogue of the S43 salience decision and the
natural successor to S45. S42/S43 made the melody the LOUDEST line (level); S45 then correctly
added inner motion (the counter); the predicted-and-now-observed next defect is that the inner
motion out-moves the held foreground. Slice 1 makes the melody the **most-active** line and the
**top** line — the two cues the ear weights ABOVE level (Affect ranks them 1 and 3) — while
keeping S45's moving inner texture (the counter recedes only *relative to* the melody, never a
blanket revert). It attacks the DOMINANT defect (activity, §2.1A) and the
highest-correctness-per-line defect (register-order, §2.1B) together; both are independent,
`chord_engine.rs`-local, and identity-byte-neutral. The `[JSON]` family is bundled because the
metrics require image-conditioning to be honest (tension #2) and because Aesthetics' field-image
warning forbids a flat-maximum hierarchy — the conditioning must be present from the first slice,
not bolted on later. Three of four lenses named exactly the `[CE]` hierarchy as slice 1 (Music
Theory and Architecture explicitly; Affect as the activity re-weighting); Aesthetics contributed
the image-conditioning that turns it from a mechanical rule into an image-justified one.

### S45 preserved (hierarchy not revert) — stated explicitly

Across ALL slices the counter is KEPT as a moving inner texture (S45's gain). It recedes
*relative to a lifted melody*: slice 1 makes the melody out-MOVE it (the governor lets the
counter move whenever the melody also moves; it recedes only when the melody holds) and out-SIT
it (seat guard); slice 3 makes the melody out-LEVEL it. The ONE legitimate hard per-image counter
recession is a deliberate hierarchy call on a deep-recession (subject) tier — never a blanket
`pad_bed_counter` de-route. A silenced counter forfeits S45 and re-opens the static-bed defect.

### The climax-bloom (S44 slice 2) vs figure-ground (S46) collision — placed (tension #5)

Aesthetics' resolution is CONFIRMED and adopted: at the climax/Return the bed may bloom in
density (S44) **only if the figure-ground GAP blooms WITH it** — the climax is "the fullest the
bed ever gets while the lead is still unmistakably in front," never an equal-voices tutti. This
is a CROSS-ARC ship-rule, not a separate slice: it constrains how S44 slice 2 (texture arc) and
S46 slice 1 (hierarchy) compose. Placement: it becomes a **guard-rail on the texture-arc slice**
(whichever of S44-slice-2 / S46-slice-1 lands second must assert the gap widens, not narrows, as
the bed thickens). Because S46 slice 1 establishes the hierarchy MARGIN as a first-class quantity
(F1/F2/F5), the climax guard is encodable as "F1 margin at the climax section ≥ F1 margin at the
Statement section" — folded into the form-arc work, not slice 1.

---

## 6. CROSS-LENS DEPENDENCIES + OPEN DECISIONS FOR THE OPERATOR

### 6.1 The six cross-lens tensions, resolved

1. **Slice-1 mechanism reconciliation — RESOLVED (§5).** The `[CE]` activity-hierarchy +
   seat-order guard is the first-90% core; the `[JSON]` image-conditioned recession family rides
   alongside it (it carries the image-conditioning the metrics need but ALONE only moves the
   weak axes). One slice, two tiers, shipped together. The level retune and inverse-register
   *scaling* are downstream (slices 3, then 5).

2. **Metric-rigidity caution vs hard-fail metrics — RESOLVED.** The hard-fail bar is made
   RELATIONAL and IMAGE-CONDITIONED, reconciling Affect (FG-1 negative = HARD fail) with
   Aesthetics (≈0 margin allowed on field images). Concretely: the melody-most-active SIGN
   requirement (melody onset-density ≥ the busiest bed role) is required only **to the degree the
   image justifies a strong figure** — the required margin is `f(fg_bg_contrast)`: large on
   subject images, ≈0 on field images. The HARD assertion is the WEAKEST-image form: the melody
   must not be *strictly less active* than the bed (margin ≥ 0) on ANY image, AND must clear the
   image-conditioned margin `f(fg_bg_contrast)` on subject images. A field image with melody
   onset-density ≈ bed PASSES (margin ≥ 0 holds; `f≈0`); only a NEGATIVE margin (background
   strictly busier — the literal operator signal 2) HARD-fails everywhere. This is encoded in the
   scorecard exactly per the spec (§B), and the rollup weights rhythmic-distinctness + register
   ABOVE level so a "turn-it-up" win scores WORSE (Aesthetics §5.2). The full
   image-conditioning mechanics are in `spec-s46-figure-ground-metrics.md`.

3. **Inverse-register compensation magnitude + which non-level tool leads — OPERATOR DECISION
   (lens lean stated).** Architecture routes the compensation to articulation/rhythmic
   separation (operator signal 4 forbids level-leading); the CURVE (how much separation per
   semitone of low seating) and whether register-placement should also carry it is a taste/affect
   call. **Lens lean:** rhythmic separation FIRST (Affect cue rank 1), articulation SECOND
   (rank 4), level NEVER (signal 4). This is generative/aesthetic work → the standing taste/affect
   gate sizes it in the build cadence (Specialist Marshaling Gate); it is NOT an architecture
   default. Marked open decision DP-3.

4. **Timbre tool (per-role MIDI program) — RULED OUT OF SCOPE for the figure-ground slices.**
   Music Theory flagged it (cue rank 5) as living in `midi_output.rs`, outside its ownership. The
   ruling: per-role timbre is a real figure-ground tool but `midi_output.rs` is a separate module
   with its own freeze/seam status, and the kernel routes all roles to one program per the current
   architecture; introducing per-role programs is a distinct, larger arc (a midi-output capability,
   not a realizer change) and is **DEFERRED**, not in any S46 slice. The figure-ground hierarchy
   is fully reachable WITHOUT it (slices 1–4 are all `chord_engine.rs`/`mappings.json`). When a
   per-role-timbre arc is opened, it docks onto the hierarchy as an additional segregation cue.
   Marked open decision DP-4 (a future arc, not an S46 fork).

5. **Climax-bloom (S44) vs figure-ground (S46) collision — RESOLVED (§5).** Gap widens as the
   bed thickens; encoded as a guard-rail on the texture-arc slice (F1-margin-at-climax ≥
   F1-margin-at-Statement), not a separate slice.

6. **The S45-counter question — RESOLVED, unanimous.** Hierarchy, not revert. The counter
   recedes in activity (slice 1 governor) and register (it is already below the counter ceiling;
   the seat guard lifts the melody, widening the gap) and level (slice 3) RELATIVE to a lifted
   melody, while keeping its motion. Hard per-image counter recession only as a deliberate
   per-image hierarchy call on the deep-recession tier. The S43-pre-staged 0.58→0.55 counter
   weight tweak is now a LIVE-counter decision — resolve the ACTIVITY recession (slice 1) FIRST,
   then re-evaluate the weight against the real counter (slice 3). Marked open decision DP-6.

### 6.2 Open decisions for the operator (numbered, with lens leans)

1. **The image-conditioned recession family — how many tiers?** Three (deep/mid/shallow, routed
   by `fg_bg_contrast`+`subject_size`), or stage with two (mid + deep) first? *Lens lean
   (Aesthetics): three* — the field-image shallow tier is what stops a forced lead on abstract
   images; it is the point of the image-conditioning.
2. **Counter recession DEFAULT — activity governor only, or also a weight trim in slice 1?**
   *Lens lean (Affect/Aesthetics): governor only in slice 1* (the activity recession is the
   load-bearing fix; resolve the 0.58 weight in slice 3 against the now-recessed counter, DP-6).
3. **(DP-3) Inverse-register compensation — which non-level tool leads, and the curve magnitude?**
   *Lens lean: rhythmic separation first, articulation second, level never; magnitude ear-tuned.*
   Generative/aesthetic → standing taste gate sizes it in the build.
4. **(DP-4) Per-role timbre (`midi_output.rs`) — open a separate arc later?** *Lens lean: defer;*
   not an S46 slice; dock it onto the hierarchy when a midi-output capability arc is opened.
5. **The seat-order guard's `MIN_FIGURE_GAP`** — 0 (melody floor exactly at counter ceiling) or a
   small positive margin (a clear seat above)? *Lens lean (Music Theory + Affect): a small
   positive gap* — high-voice superiority wants a clear margin, not a tie; ear-tuned.
6. **(DP-6) The S43-pre-staged CounterMelody weight 0.58→0.55** — apply now or after slice 1?
   *Lens lean: after slice 1* — it now applies against a live, activity-recessed counter, a
   different perceptual basis than when it was pre-staged.
7. **Hard-fail margin curve `f(fg_bg_contrast)`** — exact shape (linear, stepped by tier)?
   *Lens lean (Affect): direction/sign is load-bearing (negative = fail everywhere; positive
   margin required only on subject images); magnitude ear-tuned — match the recession-family
   tiers.*

---

## 7. GATES — the in-class cadence per build slice

Every build slice runs the **standing S43/S44 cadence**, unchanged:

1. **Correctness gate — the figure-ground invariants** (the new F1–F5 in
   `spec-s46-figure-ground-metrics.md`, EXTENDING the S45 scorecard): the **F5
   background-activity-recession regression assertion** (the hard gate — the S46 analogue of
   M1.4), the carried-forward S43 level floors (resolved Melody prominence > 0.5; bed roles
   recede but stay > 0.25), and the carried-forward S45 variety + M1.4 parallel-perfect guard.
   F1/F2/F3/F4 are REPORTED (per the scorecard's report-don't-red-bar discipline for layers known
   to be inverted pre-fix) and tighten toward pass as the build slice lands.
2. **The standing taste/affect + aesthetics gate** — summoned into the cadence as a STANDING gate
   beside correctness (per the Specialist Marshaling Gate), NOT an optional end-of-slice ear-test.
   This is generative/aesthetic work whose acceptance turns on a perceptual judgment a correctness
   gate cannot render (does the melody *arrive* as the figure?); the relevant specialists
   (Affect/Cross-Modal review + Aesthetics review) are in inventory and MUST be in the build
   cadence. They size the open taste decisions (DP-3 compensation curve, DP-6 counter weight, the
   recession-family magnitudes, the `f`/`g` margin curves — all flagged "ear-tuned").
3. **The operator A/B ear-test** — the render at `--seed 42` with and without the change, against
   the figure-ground watch-items (Affect §8): can the listener track the melody as the single
   clear figure and hum it; does the melody MOVE MOST; on a DARK image does the low melody still
   hold figure (inverse-comp); did the backgrounds recede WITHOUT hollowing; did receding the
   counter forfeit S45's movement (→ over-receded, the counter must still move); does the
   separation MARGIN differ between a calm and an energetic image (affect-conditioning).

**Freeze discipline on every slice:** `src/engine.rs` stays byte-frozen at sha256
`e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261`; every per-role/per-step term
is centered on the identity no-op (no Counter instrument under identity; prominence neutral 0.5 →
every nudge is `(0.5−0.5)*SPAN == 0`; `serde(default)` activity-floor None == today; seat guard
below the legacy bright/neutral seat) so the byte-freeze holds — the discipline that has kept the
kernel frozen across S17–S45.

---

*End of S46 unified synthesis. Design-only: no source, test, or asset modified. `src/engine.rs`
sha256 re-verified UNCHANGED at session end:
`e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261`.*

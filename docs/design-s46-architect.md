# S46 — Figure-Ground / Role-Balance (Rust Architect lens)

**Author role:** Rust Architect (DESIGN ONLY — no source, test, or asset modified by this
document; `docs/` only). All proposed Rust is signatures / types / doc comments — **no bodies**.
**Date:** 2026-06-19
**Grounded against** the working tree at HEAD, every file:line cited below read directly this
session: `src/engine.rs` (BYTE-FROZEN, sha256
`e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261`, re-verified at start AND end
of this session), `src/chord_engine.rs`, `src/composition.rs`, `assets/mappings.json`,
`tests/variety_scorecard_s45.rs`.
Precedent read first: `design-s44-architect.md` (the lens shape), `design-s44-variety-nvoice.md`
(the FREEZE LEDGER + per-layer lever table), `design-s42-salience-diagnosis.md` (the
prominence/salience system this doc builds on), `spec-s45-variety-metrics.md` +
`tests/variety_scorecard_s45.rs` (the scorecard this doc extends).

> **The binding frame this doc carries (the lead's load-bearing counterpoint).** The fix is NOT
> "turn the melody up." Level is the LAST 10%. The first 90% is DIFFERENTIATION:
> melody-moves-MORE + backgrounds-recede-in-ACTIVITY + per-role rhythmic identity. A loud melody
> over an equally-busy bed is still mush. And: do NOT walk back S45 — the counter line S45 routed
> in is a *gain* (inner texture finally moves). Resolve the inversion as a HIERARCHY problem
> (counter recedes in activity/register *relative to* a melody lifted above it), preserving the
> moving inner texture while the melody wins the foreground. A blanket per-image counter
> re-suppression is forbidden; a deliberate per-image hierarchy recession is the only legitimate
> recession.

---

## 0. Executive summary (read first)

1. **The prominence/velocity/register system EXISTS and is live — the kickoff's claim is CORRECT
   and now even stronger than S42 left it.** S43 shipped it: the `melody_forward` default
   prominence profile is in `assets/mappings.json:373-378` and IS the table default
   (`:381`), and the realizer consumes it on all three axes — velocity (`chord_engine.rs:1404`),
   register (`:1269-1271`), and the melody's rhythm-band cutoffs (`:1941-1942`). It is **deployed,
   not dormant.** The S46 figure-ground defect is therefore NOT "the prominence system is off"; it
   is **three structural inversions the prominence system, as built, cannot fix** because they live
   on axes prominence does not move.

2. **The governing finding: the figure-ground failure is a RHYTHMIC-ACTIVITY and REGISTER-ORDER
   inversion, not a level inversion — and level is the one axis the engine ALREADY balances.**
   Three concrete inversions, all freeze-reachable, none in `engine.rs`:

   - **ACTIVITY INVERSION (the dominant defect — operator signals 2, 5, 7).** On a CALM image the
     **melody goes SUSTAINED** (one long tone — `realize_rhythm` Melody arm `chord_engine.rs:1974-1992`)
     while the **CounterMelody is given a GUARANTEED off-beat onset** on every held/static step
     ("held-period activation," `:1899-1907`). So on exactly the quiet images the operator listens
     to, *the background moves and the foreground holds.* This is S45's gain turned against the
     melody: the counter's motion now competes for figure status. Level cannot fix it — the busiest
     line wins the ear regardless of which is loudest (the S42 stream-segregation lesson:
     rhythm-grid is a STRONGER segregation cue than level).

   - **REGISTER-ORDER FRAGILITY (operator signal 3).** Melody and CounterMelody are seated by
     ADJACENT, ABUTTING floors with NO guaranteed separation: melody floor `MELODY_REGISTER_FLOOR
     = 67` (`:1222`), counter ceiling `COUNTER_CEILING = MELODY_REGISTER_FLOOR = 67` (`:3478`). The
     counter is bounded *below* 67 and the melody seated *at/above* 67 — but the melody's actual
     pitch is the chord's TOP tone seated up from 67 with a brightness lift that can be **negative
     on a dark image** (`:1262-1263`, `bright_octaves` down to −1 octave), so a dark-image melody
     can seat at/below 67 and **cross into the counter's band.** "Melody may not be the highest
     voice" is real and located: the seat ORDER is not invariant; it is an emergent consequence of
     two independently-clamped floors.

   - **INVERSE-COMPENSATION ABSENCE (operator signal 4 — the subtle, important one).** The
     prominence register lift is in the WRONG direction for figure-ground. A melody seated LOW in
     its range (dark image, low chord-top) self-projects LEAST and most needs help — but the
     register nudge `prom_lift` (`:1269-1271`) only ever pushes the floor UP by a fixed weight-keyed
     amount independent of *where the melody actually landed*. There is no "this melody seated low,
     so compensate harder via non-level tools (articulation / rhythmic separation / register
     placement)." The compensation the operator wants is INVERSE to the realized register; the
     engine has only a FORWARD, register-blind lift.

3. **Level (operator signals 1, 6) is the SMALLEST part and is already balanced.** `melody_forward`
   sets Melody 0.78 / Counter 0.58 / Fill 0.40 / Pad 0.40 (`:373-378`); the per-role velocity bias
   adds +2 Melody / −3 Pad (`:1385-1392`); the centered nudge widens the gap (`:1404`). The level
   field is *already* melody-forward. The reason the melody "doesn't feel like the melody" despite
   leading in level is precisely that **the other two cues (activity, register-order) say
   background**, and they outvote level. This is the architectural proof of the operator's trap
   warning: turning 0.78→0.85 moves the weakest cue and leaves the inversion intact.

4. **The smallest highest-yield slice (S46 candidate): give the MELODY rhythmic-activity priority
   over inner voices, and put the COUNTER on an activity governor RELATIVE to the melody.** This is
   the hierarchy resolution the lead asked for: it does not re-suppress the counter (it keeps the
   counter moving), it makes the melody the MOST-active line by construction, and it is
   freeze-reachable in `chord_engine.rs` only. Detail in §1.1 / §5. It is **the activity analogue of
   what S43 did for level** — and the activity axis is the one the ear actually uses to pick the
   figure.

5. **The frozen-default question does not arise this slice.** Everything S46 needs lives in
   `chord_engine.rs` (realizer) + `assets/mappings.json` (data) + optionally `composition.rs`
   (planner). `engine.rs` stays byte-frozen.

---

## 1. WHERE EACH LEVER LIVES — the six figure-ground levers, located and freeze-tiered

Tier legend (identical to S44): **[JSON]** = `assets/mappings.json`, zero-Rust, **freeze-safe** ·
**[CE]** = `src/chord_engine.rs`, Rust realizer, **freeze-reachable** (kernel only calls it;
identity path byte-neutral) · **[COMP]** = `src/composition.rs`, Rust planner, **freeze-reachable**
· **[FROZEN]** = `src/engine.rs`, frozen-kernel decision.

| Lever (operator signal) | Capability today | Exact code path (file:line) | Tier |
|---|---|---|---|
| **Melody activity / motion** (sig 2) | EXISTS but is the WRONG default for calm images: the melody goes SUSTAINED (one held tone) when `edge_activity` is low; nothing guarantees the melody is the *most-active* line | `realize_rhythm` Melody arm, the 4-band ladder `chord_engine.rs:1943/1956/1965/1974`; calm→sustained at `:1974-1992`; band cutoffs shifted by `prom_shift` `:1941-1942` | **[CE]** (band logic) / **[JSON]** if a new "melody activity floor" knob is added to the prominence data |
| **Voice-on-top / register placement** (sig 3) | PARTIAL — seat ORDER is emergent, not invariant: abutting floors `MELODY_REGISTER_FLOOR=67` and `COUNTER_CEILING=67` with a brightness lift that can pull the melody DOWN below 67 | melody seat `chord_engine.rs:1258-1272` (lift `:1263` can be negative); counter band `:3478`, `:3663`; fill/counter floor `:1284-1310` | **[CE]** |
| **Register-aware INVERSE-compensation prominence** (sig 4) | **MISSING** — the register nudge is FORWARD and register-BLIND (fixed weight-keyed lift regardless of where the melody actually seated); no "low melody → compensate harder via non-level tools" | `prom_lift` in `role_pitch` Melody arm `chord_engine.rs:1269-1271`; velocity nudge `:1404`; NO reader of the *realized* seat | **[CE]** (consume) + **[JSON]** (the compensation curve data) |
| **Per-role rhythmic identity** (sig 7) | PARTIAL — each role HAS a distinct rhythm arm (Bass `:1648`, Fill `:1729`, Pad `:1810`, Counter `:1831`, Melody `:1927`) but they all key off ONE shared `edge_activity` scalar `:1517-1528`, so between-role and between-section rhythm reads flat | the five role arms in `realize_rhythm`; the single `edge_activity` driver `:1517`; per-section density nudge `:1526` (the only between-section term) | **[CE]** (per-role activity bias) + **[COMP]** (per-section density off image) |
| **Background activity-recession (not just level)** (sig 5) | **MISSING** — backgrounds recede in LEVEL (Pad −3, prominence weights) but there is NO activity recession: the counter's held-period activation gives the BACKGROUND a guaranteed onset the melody lacks | counter activation `chord_engine.rs:1899-1907`; Fill rest-as-gesture `:1735-1746` (the only existing activity recession, Fill-only) | **[CE]** |
| **Cross-role volume balance** (sig 6) | EXISTS (the least-broken axis) — per-role velocity bias + centered prominence nudge | `realize_velocity` role match `chord_engine.rs:1384-1394`; nudge `:1404`; weights `mappings.json:373-378` | **[JSON]** (weights) + **[CE]** (bias arms) |

**Confirmation of the kickoff's prominence claim.** The kickoff says the prominence system "already
exists (per S42/S43) and is under-deployed not missing." **CORRECTION for the ledger: it is no
longer under-deployed — S43 deployed it.** It is the table DEFAULT (`mappings.json:381`,
`"default": "melody_forward"`), so it applies to *every* image, and all three of its axes are wired
(`chord_engine.rs:1404` velocity, `:1269-1271` register, `:1941-1942` rhythm-band). The accurate
S46 statement is sharper: **the prominence system is fully deployed on the LEVEL and a little on
register/rhythm-band, but it operates on the wrong axes for figure-ground** — it cannot make the
melody the *most-active* line (it only shifts the melody's *own* band cutoffs, never relative to the
counter), it cannot enforce seat ORDER (it nudges a floor, it does not guarantee melody > counter),
and it has no inverse-register compensation. The figure-ground fix is NOT re-deploying prominence;
it is adding the two axes prominence was never built to carry: **relative rhythmic activity** and
**seat-order/inverse-register**.

### 1.1 The activity inversion, in code (the dominant defect)

The mechanism, traced end to end on a calm image (low `edge_activity`):

```
realize_rhythm(role = Melody, …)            chord_engine.rs:1927
  edge_activity low  →  falls through 0.80 / 0.55 / 0.25 cutoffs (:1943/1956/1965)
  →  SUSTAINED arm  (:1974-1992)  →  ONE long held tone, offset 0.

realize_rhythm(role = CounterMelody, …)     chord_engine.rs:1831
  held_chord || melody_static  (:1899)      ← TRUE precisely when the melody is calm/static
  →  GUARANTEED off-beat onset at step_ms/4 (:1900-1907)  →  the counter MOVES.
```

The two arms read the **same** `edge_activity` but resolve OPPOSITELY: low activity makes the
melody hold and makes the counter move (the held-period activation is explicitly designed to "fill
the operator's empty period" — `:1902` — but the empty period it fills is the melody's, so it fills
the foreground's silence with a *background* line). **This is the figure-ground inversion in two
adjacent match arms.** The fix is architectural and local: the counter's activation must be
GOVERNED by the melody's activity (recede when the melody holds, not move *because* it holds), and
the melody must have an activity FLOOR that keeps it the most-active line. Both are
`chord_engine.rs`-local, freeze-reachable, and identity-byte-neutral (no Counter or non-neutral
prominence under identity).

---

## 2. THE PROMINENCE / VELOCITY / REGISTER SEAM IN DETAIL

### 2.1 How prominence is computed and applied today — per-ROLE, per-step, centered on a freeze pivot

`prominence_weight(ctx, role) -> f32` (`chord_engine.rs:1023-1032`) reads
`ctx.section.orchestration.prominence` (a `Vec<LayerProminence>`, `composition.rs:530`), finds the
row whose `LayerRole` bridges (via `to_orchestral_role`) to `role`, and returns its `weight`;
EMPTY vec or unlisted role → `PROMINENCE_NEUTRAL = 0.5` (`:985`). It is **per-role, not
per-instance** (a single weight for the whole Melody role; no notion of "this melody seated low").
The planner copies the selected `ProminenceProfile.layers` onto every section's
`orchestration.prominence` (`composition.rs` planner, the `prominence` SelectTable, default
`melody_forward`). The weight is consumed on three axes, each as a CENTERED nudge that is exactly
`0` at `w == 0.5` (the byte-freeze pivot — `design-s42 §`, re-confirmed):

- **Velocity** (`realize_velocity:1404`): `vel += (w − 0.5) * PROMINENCE_VEL_SPAN(18)`, applied
  AFTER the per-role bias (`:1384-1394`, Melody +2 / Bass −1 / Pad −3 / Fill 0), `!is_cadence`
  guarded.
- **Register** (`role_pitch:1269-1271` Melody; `:1307-1309` inner): `prom_lift = ((w − 0.5) *
  PROMINENCE_REG_SPAN(4)).round()`; for the inner voices `.max(0)` so a recessive bed is **never
  lowered** (the Risk-1 "never invert figure-ground downward" guard, `:1301-1308`). Summed under a
  single `.clamp(24,96)` (the Risk-1 sum-clamp).
- **Rhythm bands** (Melody only, `realize_rhythm:1941-1942`): `prom_shift = (w − 0.5) *
  PROMINENCE_RHY_SHIFT(0.10)`; lowers the melody's OWN band cutoffs so a foreground melody
  subdivides sooner. **Note: this shifts the melody's cutoffs against a FIXED scalar — it never
  compares the melody's activity to the counter's.** This is exactly why prominence cannot fix the
  activity inversion: `prom_shift` makes the melody *more likely to subdivide than it otherwise
  would*, but on a calm image even a lowered 0.25 cutoff is still above the image's `edge_activity`,
  so the melody still falls to SUSTAINED while the counter's held-period activation fires
  unconditionally.

### 2.2 Where the register floors live

`BASS_REGISTER_FLOOR = 36` (`:1220`), `FILL_REGISTER_FLOOR = 55` (`:1221`), `MELODY_REGISTER_FLOOR
= 67` (`:1222`), `COUNTER_CEILING = MELODY_REGISTER_FLOOR = 67` (`:3478`). The counter's working
band is `[FILL_REGISTER_FLOOR(55), COUNTER_CEILING(67))` with anchor `(55+67)/2 = 61` (`:3663`,
`:3916`, `:3993`). The melody seats UP from `67` with the brightness lift `lift = (bright_octaves *
12).round()` (`:1263`), `bright_octaves ∈ [−1, +1]` (`:1246`). **The seat-order invariant
"melody > counter" is NOT enforced anywhere** — it emerges from `67 ≥ 67` only when the melody's
lift is `≥ 0`. On a dark image (`bright_octaves < 0`) the melody floor can drop to `67 − 12 + prom_lift
= 55+`, landing IN the counter's band. This is the located cause of "melody may not be the highest
voice."

### 2.3 Where the rhythm template keys off role

`realize_rhythm` (`:1494`) computes ONE `edge_activity` (`:1517-1528`) — `features.edge_density /
EDGE_ACTIVITY_RANGE_MAX`, plus the per-section `(density − 0.5) * GAIN` nudge (`:1526`, the only
between-section rhythm term) — then dispatches on `role` (`:1647`) into five arms with HARD-CODED,
NON-prominence-keyed activity behaviors: Bass sparse (`:1648-1727`), Fill sustained + rest-as-gesture
(`:1729-1828`), Pad block/figured (`:1810-1828`), Counter held-period-activation/oblique
(`:1831-1925`), Melody 4-band ladder (`:1927-1993`). **The per-role rhythmic *identity* exists in
these five arms, but the per-role *activity level* is not a tunable** — only the Melody arm reads
prominence (its band cutoffs), and even that is self-referential, never relative to the counter.

### 2.4 The precise seam where the two new capabilities land — byte-neutral on identity

**(a) Relative rhythmic activity (the figure-ground core).** The seam is the CounterMelody arm's
activation predicate (`:1899`) and the Melody arm's band ladder (`:1943-1992`). Introduce a single
per-step **activity-hierarchy** signal — the melody's realized activity class for THIS step,
computed once where the Melody arm already computes it — and pass it (additive private param, the
blessed `pad_voices`/`ctx` precedent) to the counter arm so the counter's activation is GOVERNED:

```rust
// src/chord_engine.rs — NEW additive type + private threading. realize_step's PUBLIC sig UNCHANGED.
//
/// The realized rhythmic-activity class of a voice on a step, coarse enough to be RNG-free
/// and structural (it is a function of `edge_activity` + the arm's band, not of pitch).
/// Ordering is the figure-ground rank: Sustained < Oblique < Subdividing. The COUNTER arm
/// reads the MELODY's class to stay strictly BELOW it (the hierarchy invariant), so the
/// background never out-moves the foreground. Under identity there is no Counter instrument
/// and prominence is neutral, so this is never consulted on the freeze path → byte-neutral.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ActivityClass { Sustained, Oblique, Subdividing }

/// The melody's activity class for this step, derived from the SAME band ladder the Melody
/// arm uses (chord_engine.rs:1943-1992) — extracted so the Counter arm can govern off it.
/// Pure; reads only `edge_activity`, the prominence rhythm shift, and `pre_cadence`.
fn melody_activity_class(edge_activity: f32, prom_shift: f32, pre_cadence: bool) -> ActivityClass;
```

The counter arm's `:1899` predicate becomes governed: it may take its moving off-beat onset ONLY
when `melody_activity_class >= Subdividing` is FALSE *and* a per-step "melody is holding, so the
counter recedes one rank below it" rule holds — i.e. when the melody SUSTAINS, the counter does NOT
get the guaranteed onset; it takes the OBLIQUE held tone (which it already has at `:1908-1912`) or
the rest-as-gesture (`:1919-1920`). This **preserves S45's gain** (the counter still moves whenever
the melody is also moving — inner texture is alive in active passages) while killing the inversion
(the counter never out-moves a holding melody). It is the activity analogue of the level hierarchy
`melody_forward` already encodes.

**(b) The melody activity FLOOR (make the melody the MOST-active line by construction).** Add a
prominence-keyed activity FLOOR so a foreground melody never falls all the way to SUSTAINED on an
otherwise-calm image — instead of the self-referential `prom_shift` that merely lowers cutoffs, give
the foreground melody a *minimum* activity class (e.g. a foreground melody floors at `Oblique`/dotted
rather than Sustained). This is the same centered-on-0.5 discipline (a neutral-weight melody floors
at Sustained = today's behavior, byte-identical; a foreground melody floors higher). Data-side a
**[JSON]** optional field on the prominence profile; consume-side a **[CE]** read in the Melody arm.

```rust
// src/composition.rs — additive serde-default field on ProminenceProfile / LayerProminence data,
// keeping every existing JSON profile byte-shape-stable (the figuration/bass_pattern precedent):
/// Optional minimum activity rank for a foreground voice — floors how SUSTAINED the melody may
/// fall on a calm image so the foreground stays the most-active line. None/absent == no floor
/// (today's behavior, byte-identical). Only meaningful for a foreground (>0.5) Melody weight.
#[serde(default)]
pub activity_floor: Option<u8>,   // 0=Sustained 1=Oblique 2=Subdividing; serde-defaulted None
```

**(c) Seat-order invariant + inverse-register compensation.** The seam is the Melody seat
(`:1258-1272`) and the counter band ceiling (`:3478`). Two additive changes:
- *Seat-order guard:* after computing the melody seat, ENFORCE `melody_seat > counter_ceiling`
  (raise the melody an octave if a dark-image lift dropped it into the counter band) — a
  `.max(COUNTER_CEILING + MIN_FIGURE_GAP)` style floor on the realized seat, summed under the
  existing `.clamp(24,96)`. Byte-neutral under identity (no counter → guard is a no-op there, and
  the legacy path has no counter ceiling interaction).
- *Inverse-register compensation:* make the NON-LEVEL compensation a function of WHERE the melody
  actually seated. The architecture handle is the realized seat (`role_pitch`'s return). The
  compensation should route to articulation / rhythmic separation, NOT to a bigger level bump
  (operator signal 4: "turn it up is the weakest tool"). Concretely: a melody that seated LOW in its
  range gets a stronger articulation-separation bias (shorter, more detached, off the bed's grid) so
  it pops out perceptually despite the low register. This reads the realized seat in the Melody
  rhythm/articulation path — additive, `!is_cadence`-guarded, neutral at neutral weight.

```rust
// src/chord_engine.rs — NEW pure helper, consumed in the Melody arm (articulation/rhythm bias).
/// INVERSE-register figure compensation: a melody seated LOW in its range (closer to the
/// inner band) needs MORE non-level help (articulation separation / rhythmic distinctness),
/// because high notes self-project and low ones do not (operator signal 4). Returns a
/// 0..1 compensation factor: 0 at the TOP of the melody range (self-projecting, no help),
/// rising toward 1 as the seat approaches COUNTER_CEILING. Level is intentionally NOT a
/// consumer — this drives articulation/rhythm separation only. Pure; identity-neutral (the
/// legacy path seats high enough that the factor is small, and prominence is neutral).
fn inverse_register_compensation(melody_seat: u8) -> f32;
```

**Why all of this is byte-neutral on identity:** under the identity profile there is no
CounterMelody instrument (`pad_voices == 0`, empty `layers` → `assign_role` delegates to
`instrument_role`, `:1054-1056`), and `prominence_weight` returns the neutral `0.5` (empty vec,
`:1025-1026`), so `prom_shift == 0`, the activity floor is `None`, the counter governor is never
reached, and the seat-order guard's `.max(...)` is below the legacy seat. Every new term centers on
the identity no-op exactly as the S23 prominence nudges do (`:977-980`). `engine.rs` only *calls*
`realize_step`/`realize_rhythm`/`role_pitch` and its public signatures do not move.

---

## 3. THE SCORECARD-EXTENSION SEAM

`tests/variety_scorecard_s45.rs` already renders all 6 probe images through the real seeded plan +
realizer, collects per-role `NoteEvent` streams (`render()`, `:173-232`), and computes per-layer
metrics. The figure-ground metrics SLOT INTO the same `scorecard_for` body and the same
`LayerVerdicts` struct (`:305-315`), reusing the existing helpers `per_step_pitch` (`:268`),
`distinct_pitches` (`:281`), `motion_dir` (`:262`), and `step_shape_key` (`:288`). All five new
metrics are computable from the streams already collected — no new render path. Per the
spec-s45 RNG-boundary discipline (`spec §0.2`), each is tagged DETERMINISTIC (structural / RNG-free)
or SEEDED (absolute pitch).

| New metric | What it measures | Computed from | DET vs SEEDED |
|---|---|---|---|
| **F1 melody-is-most-active** | melody onset-count per step ≥ every other concurrent role's, on ≥ X% of steps. The DIRECT figure-ground correctness gate. | per-(step,role) onset counts — already grouped at `:715-736` for M5.1; compare Melody vs each other role per step | **DETERMINISTIC** — onset count is the rhythm template (RNG-free), the same data M5.1/M3.2 read |
| **F2 melody-is-highest** | fraction of co-sounding steps where the melody's seat > every other role's seat (the seat-ORDER invariant). | per-step representative pitch per role (`per_step_pitch`, `:268`); compare Melody max vs others | **SEEDED** — absolute pitch (chord draw); pin under the existing `SEED = 42` (`:62`), exactly as M1.2/M4.1 are |
| **F3 inverse-compensation present** | when the melody seats LOW (seat near `COUNTER_CEILING`), is its articulation/rhythm MORE separated from the bed than when it seats high? (the compensation is *active*, not just declared) | melody seat (`per_step_pitch`) × melody `step_shape_key` onset-distinctness vs Pad/Fill (reuse the M1.3 onset-distinct pattern, `:400-430`) | **SEEDED** for the seat partition; the onset-distinctness *within* a partition is **DETERMINISTIC** |
| **F4 per-role-rhythm-distinctness** | distinct `step_shape_key` SETS per role differ across roles (Bass ≠ Fill ≠ Counter ≠ Melody shape vocabularies) AND between sections (signal 7's "between-section reads flat") | `step_shape_key` per role per section — already computed for M5.1/M5.2 (`:711-789`), partition by role and by section | **DETERMINISTIC** — onset/offset/hold shape is RNG-free |
| **F5 background-recession (activity)** | the counter's (and Fill's) onset-count per step ≤ the melody's on every step it co-sounds — the activity-recession invariant, the regression gate that pins S46's gain and forbids re-inversion | per-step onset counts (as F1), counter/fill vs melody | **DETERMINISTIC** |

**Where they land in the file.** F1/F5 fold into the per-(step,role) onset grouping the Rhythm
layer already builds (`:715-736`); F2/F3 fold into the per-step pitch + onset-distinct machinery the
CounterMelody layer already builds (`:380-465`); F4 reuses the `step_shape_key` set the Rhythm layer
builds (`:713`). Add a new `LayerVerdicts` field `figure_ground: Verdict` plus the raw counts
(`melody_most_active_frac`, `melody_highest_frac`, `bg_recession_violations`) for the printed row,
mirroring `parallel_perfect_count` (`:313`).

**The new HARD assertion (the regression gate).** F5 (background activity-recession) is the S46
analogue of the M1.4 parallel-perfect regression guard: assert
`bg_recession_violations <= documented_residual` so that ANY future change re-introducing a
background line out-moving the melody FAILS the harness — exactly the "do not walk back S45 but do
not let it re-invert" discipline encoded as a test. Like M1.4, the residual is measured-and-pinned
on the routed images (the counter is only present on AudioHaxImg1/2/3, `spec §0.3`), reported on the
others as `N/A — counter not routed`. **Verdict semantics:** today (pre-fix) F1/F2/F5 are EXPECTED
to FAIL on the counter-routed calm images (the inversion is live) — so, per the scorecard's
report-don't-red-bar discipline for known-dormant layers (`:38-39`), F1–F4 are REPORTED and only F5
carries the regression assertion, pinned to today's (inverted) residual so the BUILD slice tightens
it toward zero. That makes the scorecard the objective before/after instrument for the S46 build,
exactly as spec-s45 §8c intends.

**RNG discipline reused verbatim:** `set_composition_seed(Some(SEED))` before every `plan()`
(`:322`, `:951`); F2/F3's seat-dependent parts are SEEDED and replay byte-identically under SEED 42;
F1/F4/F5 are structural and need no seed. The engine.rs freeze guard (`:1157-1166`) is untouched and
still asserts the sha.

---

## 4. FREEZE LEDGER — per named change

| Change | Site | Touches `engine.rs`? | Verdict |
|---|---|---|---|
| `ActivityClass` enum + `melody_activity_class` helper | `chord_engine.rs` | NO | **FREEZE-REACHABLE** — pure helper; no Counter & neutral prominence under identity → never consulted on the freeze path |
| Govern the CounterMelody activation off the melody's activity class (hierarchy rule at `:1899`) | `chord_engine.rs:1899-1924` | NO | **FREEZE-REACHABLE** — the counter arm is unreachable under identity (no Counter inst); byte-neutral on the freeze path; PRESERVES S45 (counter still moves when melody moves) |
| Melody `activity_floor` (foreground melody never falls to Sustained) | `chord_engine.rs:1943-1992` (consume) + `mappings.json` prominence data (declare) | NO | **FREEZE-SAFE/REACHABLE** — `serde(default)` None == today; floor only bites for a >0.5 Melody weight, and identity weight is 0.5 → no-op, byte-identical |
| Seat-order guard (`melody_seat > counter_ceiling`) | `chord_engine.rs:1258-1272` | NO | **FREEZE-REACHABLE** — additive `.max(...)` summed under the existing `.clamp(24,96)`; below the legacy seat under identity → no-op |
| `inverse_register_compensation` → articulation/rhythm separation (NOT level) | `chord_engine.rs` (Melody arm) | NO | **FREEZE-REACHABLE** — additive, `!is_cadence`-guarded; neutral-weight + high legacy seat → negligible/no-op on the freeze path |
| New scorecard figure-ground metrics F1–F5 + the F5 regression assertion | `tests/variety_scorecard_s45.rs` | NO | **FREEZE-SAFE** — test-only; the existing engine.rs sha guard (`:1157`) is unchanged and still passes |
| **Raise melody prominence weight 0.78→higher (the level lever)** | `mappings.json:374` | NO | **FREEZE-SAFE but NOT the fix** — moves the weakest cue; do NOT lead with it (the operator's trap) |
| **Any edit to `engine.rs`** | `engine.rs` | YES | **FROZEN-KERNEL** ⚠ — none proposed; S46 needs none |

**The freeze fact for S46:** every figure-ground lever is freeze-reachable in `chord_engine.rs`
(realizer) + data in `assets/mappings.json` + test in `tests/`. `engine.rs` is not touched; sha
stays `e50c7db1…2348261`.

---

## 5. RANKED CANDIDATE SLICES (architecture view — figure-ground-yield per edit)

Ranked by figure-ground yield per unit edit, with the lead's "differentiation first, level last"
ordering baked in:

1. **[CE, freeze-reachable] Activity hierarchy: govern the counter off the melody's activity +
   floor the melody's activity — THE S46 SLICE-1 CANDIDATE.** Introduce `ActivityClass` +
   `melody_activity_class` (§2.4a), govern the counter activation (`:1899`) so it never out-moves a
   holding melody, and add the prominence-keyed melody `activity_floor` (§2.4b) so a foreground
   melody never falls to SUSTAINED on a calm image. **This is the single highest-yield change**
   because it attacks the DOMINANT defect (the activity inversion, §1.1) on the STRONGEST
   segregation cue (rhythm-grid > register > level), it PRESERVES S45 (the counter keeps moving in
   active passages — it recedes in activity only RELATIVE to the melody, never a blanket revert),
   and it is `chord_engine.rs`-only, identity-byte-neutral. *Dependencies:* none. *Freeze tier:*
   [CE] + a [JSON] data field for the floor. It is the activity analogue of S43's level fix.

2. **[CE, freeze-reachable] Seat-order invariant.** Enforce `melody_seat > counter_ceiling` in the
   Melody seat (§2.4c, first bullet) so "melody is the highest voice" becomes structural, not
   emergent — fixing the dark-image crossing. *Dependencies:* independent of slice 1 but most
   meaningful after it (a melody that also out-MOVES the counter and sits ABOVE it is
   unambiguously the figure). *Freeze tier:* [CE]. Highest correctness-per-line after slice 1;
   pin it with the F2 scorecard metric.

3. **[CE+JSON, freeze-reachable] Inverse-register compensation via NON-level tools.** Route the
   `inverse_register_compensation` factor (§2.4c, second bullet) into the Melody arm's
   articulation/rhythmic-separation, so a melody seated low pops out perceptually without a louder
   level. *Dependencies:* slices 1–2 (the melody must already lead in activity and sit on top before
   the subtler low-seat compensation is audible). *Freeze tier:* [CE] consume + [JSON] curve data.
   This is operator signal 4 directly, and it is the one the operator flagged as the SUBTLE,
   important capability — the engine genuinely lacks it (§1, MISSING).

4. **[CE+COMP, freeze-reachable] Per-role rhythmic identity + between-section rhythm arc.** Give
   each role a per-role activity bias (so Bass/Fill/Counter/Melody have distinct rhythmic
   FINGERPRINTS beyond the shared `edge_activity`) and drive per-section density off image region
   energy (the `(density−0.5)*GAIN` term that is currently flat at 0.5 — `:1526`,
   `composition.rs` planner) so between-section rhythm stops reading flat (operator signal 7,
   second half). *Dependencies:* slices 1–3 (differentiate the foreground first; then enrich the
   per-role identities under it). *Freeze tier:* [CE] (per-role bias) + [COMP] (per-section density).

5. **[JSON, freeze-safe] Level retune (LAST, the 10%).** Only after slices 1–4, if the ear still
   wants it: nudge the `melody_forward` weights (`mappings.json:373-378`). **Explicitly last and
   small** — leading with it is the operator's confirmed trap (a loud melody over an equally-busy
   bed is still mush). *Recommended: do not ship this slice unless the ear asks after 1–4.*

### The single recommended S46 first slice
**Slice 1 — the activity hierarchy** (govern the counter off the melody + floor the melody's
activity). It is the exact figure-ground analogue of the S43 salience decision: S42/S43 made the
melody the loudest line; S45 then (correctly) added inner motion; the predicted-and-now-observed
next defect is that the inner motion out-moves the held foreground. Slice 1 makes the melody the
*most-active* line — the cue the ear actually uses to pick the figure — while keeping S45's moving
inner texture, and it does so freeze-reachably in `chord_engine.rs` with every term centered on the
identity no-op.

---

## 6. CROSS-LENS DEPENDENCIES the synthesis must resolve

1. **The counter-governor curve is a MUSICAL-CRAFT decision, not an architecture one (Music Theory
   lens).** Architecture provides the `ActivityClass` ordering and the governing seam (counter ≤
   melody − 1 rank); *exactly how* the counter should recede (held oblique tone vs rest-as-gesture
   vs a sparser off-beat) when the melody holds is species-counterpoint craft. The counter arm
   already has all three modes (`:1908-1924`); Music Theory must pick the per-case recession so the
   inner texture stays musically alive, not merely quieter. This is the load-bearing cross-lens item.

2. **The inverse-register compensation magnitude + which non-level tool (Affect / Aesthetics
   lens).** Architecture routes the compensation to articulation/rhythmic separation (operator
   signal 4 forbids level); the CURVE (how much separation per semitone of low-seating) and whether
   timbre/register-placement should also carry it is a taste/affect judgment. This is generative
   /aesthetic work → the standing taste-affect gate beside correctness must size it (Specialist
   Marshaling Gate), not an architecture default.

3. **Per-image counter recession legitimacy (Aesthetics lens + operator).** The lead's frame allows
   a *deliberate per-image* hard counter recession as a hierarchy decision, never a blanket revert.
   Architecture exposes the per-step governor (slice 1); whether any image warrants a *harder*
   per-image recession (and which) is an aesthetics/operator call the synthesis must rule on — the
   architecture must NOT bake a blanket counter-suppression.

4. **The F5 regression-residual baseline (Test Engineer).** F5's pinned residual must be measured
   on the routed images under the current (pre-fix) tree, then tightened by the build slice — the
   same staged-bound discipline as M1.4 (`tests/variety_scorecard_s45.rs:1029-1043`). The synthesis
   should confirm the residual is measured BEFORE slice 1 lands so the gate brackets the gain.

---

*End of S46 architect lens. Design-only: no source, test, or asset modified. `src/engine.rs`
sha256 re-verified UNCHANGED at session end: `e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261`.*

# S48 Slice 3 — The Buildable Per-File Spec: INVERSE-REGISTER COMPENSATION + THE LEVEL FINISH

**Author role:** Rust Architect (DESIGN ONLY — no source/test/asset modified by this document; `docs/` only).
**Date:** 2026-06-19.
**Status:** LOCKED for build. This is the single contract the two implementers build against — the **Music Theory Specialist** (owns `chord_engine.rs` realizer internals + `assets/mappings.json` musical rows) and the **Test Engineer** (owns `tests/variety_scorecard_s45.rs` scorecard). The S46 work-order's Slice-3 row (`docs/design-s46-figure-ground.md` §5) + DP-3 / DP-6 (§6.2) are settled inputs; this doc encodes them, it does not re-litigate them.

**Synthesizes** the S46 design cadence's Slice-3 row into a build: `design-s46-figure-ground.md` §5 row 3 (the two coupled changes — inverse-register comp via NON-level tools FIRST, the level retune SECOND) + §6.2 DP-3 (the LOCKED non-level priority: rhythmic separation FIRST / articulation SECOND / level NEVER) + DP-6 (the CounterMelody mid-tier 0.58→0.55 trim, ear-gated/optional) + `spec-s46-figure-ground-metrics.md` §1 F4 (the INVERSE-COMPENSATION metric — NEGATIVE `correlation(register_gap, separation)`) + §2 (F5b stays the hard regression gate, asserted at 0). Mirrors `docs/spec-s47-slice1-build.md` in shape and rigor.

**Grounded against the LIVE working tree at HEAD — every file:line below was re-read THIS session against the live file. The tree DRIFTED after S47/Slice-4 landed; the S46/S47 doc line numbers are STALE and were NOT trusted — the confirmed live sites are below:**
`src/chord_engine.rs` —
- `realize_velocity` `:1614-1696`; the per-role velocity-bias `match role` is `:1672-1682` with arms `Melody +2 (:1673) / Bass −1 (:1674) / Pad −3 (:1680)` and the **`_ => {}` fall-through `:1681`** where **HarmonicFill + CounterMelody currently get NO bias**; the `!is_cadence` guards on every arm; the prominence velocity nudge `(prominence_w − PROMINENCE_NEUTRAL) * PROMINENCE_VEL_SPAN` applied `!is_cadence`-guarded at `:1691-1693` (`PROMINENCE_VEL_SPAN = 18.0` `:995`).
- `role_pitch` Melody arm `:1530-1561`: `raw = MELODY_REGISTER_FLOOR(67) + lift + prom_lift` `:1543`; the **S47 seat-order guard already landed** `:1544-1559` (`seat_floor = COUNTER_CEILING + MIN_FIGURE_GAP` gated `if counter_present`, else `i16::MIN`), folded under one `.clamp(24,96)` `:1559`. `role_pitch` signature `:1490-1508` carries the additive private params `prominence_w` `:1497` + **`counter_present: bool` `:1507`**.
- `realize_rhythm` `:1782-1800`: signature carries `note: u8 :1783` (the realized melody seat — used by the `sustained` closure), `pad_voices: u8 :1794`, `ctx: &StepContext :1799`. **It does NOT currently receive `counter_present`** (see §2b/§8). `edge_activity` `:1805-1816`, `pre_cadence` `:1826-1829`, `base_frac` (the ARTIC_WINDOW_LO/HI articulation curve) `:1862-1874`, the `sustained(offset, slot_ms, frac)` closure `:1883-1892`.
- Melody rhythm arm `:2274-2356`: `melody_w = prominence_weight(ctx, role)` `:2288`; `prom_shift` `:2289`; the S47 `floor_to_dotted` activity floor `:2300-2302`; the 4-band ladder ARPEGGIO `:2303-2315` / SYNCOPATED `:2316-2324` / DOTTED `:2325-2336` (`floor_to_dotted ||` routes calm-foreground here) / SUSTAINED `:2337-2355`.
- CounterMelody arm `:2142-2272`: the S47 activity governor (`melody_prom_shift` `:2229-2231`, `m_class` `:2232`, the `oblique_or_rest` closure `:2241-2248`, the `match m_class` routing Subdividing→MOVING `step_ms/4` `:2250-2259` / Oblique→onset 0 `:2260-2264` / Sustained→`oblique_or_rest()` `:2265-2270`).
- Helpers near `:1043-1213`: `ActivityClass { Sustained, Oblique, Subdividing }` (derives `Ord`) `:1050-1058`; `melody_activity_class(edge_activity, prom_shift, pre_cadence) -> ActivityClass` `:1073-1087`; `ACTIVITY_FLOOR_THRESHOLD = 0.50` `:1097`; `MIN_FIGURE_GAP: u8 = 2` `:1106`; the S47-Slice-4 Pad recession `melody_min_onsets` `:1177-1183` / `pad_onset_cap` `:1200-1213` / `recede_pad_onsets` `:1233-1270` (the onset-offset-displacement precedent for F5a anti-fusion); `prominence_weight(ctx, role)` `:1278-1287` (returns `PROMINENCE_NEUTRAL` on empty prominence).
- Consts: `MELODY_SYNC_CUTOFF = 0.55` `:1040`, `MELODY_DOTTED_CUTOFF = 0.25` `:1041`; `PROMINENCE_NEUTRAL = 0.5` `:985`, `PROMINENCE_RHY_SHIFT = 0.10` `:1015`, `PROMINENCE_REG_SPAN = 4.0` `:1005`; `FILL_REGISTER_FLOOR = 55` `:1485`, `MELODY_REGISTER_FLOOR = 67` `:1486`; `COUNTER_CEILING = MELODY_REGISTER_FLOOR` (== 67) `:3841`.

`assets/mappings.json` — `prominence_catalogue` `:365-392` (CONFIRMED live weights below) + the `prominence` SelectTable `:393-407` (routes deep→`melody_lead_strong` (ct ≥ 0.25), the preserved `subject_melody`, shallow→`melody_lead_gentle` (ct < 0.10), default→`melody_forward`).

`tests/variety_scorecard_s45.rs` — F4 `correlation(f4_gaps, f4_seps)` computed `:1164-1224` (`gap = mel_pitch − max(bed_pitch)` `:1199`; `sep = fraction of bed roles whose onset offset differs from the melody's` `:1200-1217`; `mel_off` = the FIRST melody event's `offset_ms` per step `:1172-1178`); the F4 line `f4_corr < 0.0 ? OK : FAIL`, **SEEDED / REPORTED-not-asserted** `:1220-1223`; `f4_ok = f4_corr < 0.0` folded into the rollup `:1312`; the figure_ground rollup `:1295-1326`; the F5b hard assertion `:1552-1572` (`v.bg_recession_violations <= s46_recession_bound(name)`, post-S47 residual driven to 0); `s46_recession_bound` `:1402`; `LayerVerdicts` `:445-469` (carries `figure_ground`, `melody_most_active_margin`, `melody_highest_frac`, `bg_recession_violations`, `rhythm_distinct_frac`, `fg_bg_contrast` — **NO `inverse_comp` field yet**).

**CONFIRMED current prominence weights (`mappings.json:365-392`), the level-bump baseline:**

| tier (id) | routed when | Melody | CounterMelody | HarmonicFill | Pad | Bass | mel−counter gap |
|---|---|---|---|---|---|---|---|
| **deep** `melody_lead_strong` | `fg_bg_contrast ≥ 0.25` | **0.90** | **0.45** | 0.30 | 0.30 | 0.50 | 0.45 |
| **mid** `melody_forward` (default) | else | **0.78** | **0.58** | 0.40 | 0.40 | 0.50 | 0.20 |
| **shallow** `melody_lead_gentle` | `fg_bg_contrast < 0.10` | **0.72** | **0.65** | 0.45 | 0.45 | 0.50 | 0.07 |
| `subject_melody` (escalation) | small separated subject | 1.0 | 0.6 | 0.4 | 0.3 | 0.5 | 0.40 |
| `uniform` | identity | (empty → neutral 0.5 all) | | | | | 0 |

**THE FREEZE (binding):** `src/engine.rs` is BYTE-FROZEN at sha256 `e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261` (re-verified UNCHANGED this session via `sha256sum src/engine.rs` — see §0). **NOTHING in this slice touches `engine.rs`.** Every lever is in `chord_engine.rs` (the realizer the frozen kernel only *calls*) + `assets/mappings.json` (data) + `tests/` (scorecard). Every per-role/per-step term is centered on the identity no-op: under identity there is **no CounterMelody instrument** (`counter_present == false`, `pad_voices == 0`, empty `layers`) and prominence is **neutral 0.5** (`prominence_weight` returns `PROMINENCE_NEUTRAL` on the empty prominence Vec), so every centered nudge `(0.5−0.5)*SPAN == 0` and every counter-gated term is unreachable. The 9 `engine_equivalence` goldens are synthetic NO-COUNTER bars (melody note 79, bass 36, cadence velocities 114/84, cadence hold 240 ms) — none of this slice's terms fire on them.

---

## 0. PRE-BUILD FREEZE VERIFICATION

```text
$ sha256sum src/engine.rs
e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261  src/engine.rs   ✓ MATCH (verified this session)
```

The `ENGINE_SHA256` guard in `tests/variety_scorecard_s45.rs` asserts this same digest and is UNTOUCHED by this slice. If the digest does not match at build time, STOP — the slice's freeze-neutrality witnesses (§4) are predicated on this exact byte image.

---

## 1. CURRENT-STATE GROUND TRUTH — the three slice-3 sites, pinned to CONFIRMED live lines

Re-read this session against the post-S47/Slice-4 tree. The CONFIRMED current sites the three changes touch:

| # | Site | CONFIRMED live line(s) | What is there today |
|---|---|---|---|
| **(a-i) LEVEL/JSON** | The deep/mid/shallow Melody prominence weights | `mappings.json:374-391` | deep `melody_lead_strong` Melody **0.90**; mid `melody_forward` Melody **0.78** (the **most-routed** default, byte-stable since S47); shallow `melody_lead_gentle` Melody **0.72**. The melody velocity nudge reads these at `realize_velocity:1691-1693` (`(w−0.5)*18`). The mel−counter LEVEL gap today is 0.45 / 0.20 / 0.07 deep/mid/shallow. |
| **(a-ii) LEVEL/CE** | The Counter+Fill velocity-bias gap | `realize_velocity:1672-1682`; the `_ => {}` fall-through is `:1681` | The per-role velocity bias is `Melody +2 (:1673) / Bass −1 (:1674) / Pad −3 (:1680)`, all `!is_cadence`-guarded. **HarmonicFill and CounterMelody fall through `_ => {}` (:1681) → NO velocity bias** — their level recession is carried ONLY by their prominence weight, not by a structural bias the way the Pad's −3 is. So the counter/fill have no structural level floor BELOW the melody beyond the prominence nudge. |
| **(b) INVERSE-COMP/CE** | The melody arm's onset OFFSET + articulation, register-blind | `realize_rhythm` melody arm `:2274-2356`; `sustained` closure `:1883-1892`; `base_frac` `:1862-1874` | The melody's onset offset is FIXED by band: ARPEGGIO spreads `k*slot` `:2310-2315`, SYNCOPATED delays to `step_ms/4` + `step_ms*3/4` `:2316-2324`, DOTTED onsets at `0` + `2/3` `:2325-2336`, **SUSTAINED onset 0** `:2354`. None of these read the realized seat `note` for offset/articulation — the help is **register-BLIND** (F4 corr ≈ 0, ABSENT). The realized melody seat is available as the closure-captured `note :1783`; the bed reference is `COUNTER_CEILING = 67 :3841`. |
| **(c) DP-6/JSON, optional** | The CounterMelody MID-tier weight | `mappings.json:382` | `melody_forward` CounterMelody = **0.58** (the S43-pre-staged 0.58→0.55 trim is DEFERRED-and-ear-gated). |

**Supporting seam facts (confirmed, load-bearing for the build):**
- `realize_rhythm` (`:1782-1800`) carries the additive private params `pad_voices :1794` and `ctx :1799` — the blessed "additive private param on the free fn, NOT a `realize_step` public-seam change" route (`role_pitch` and `realize_velocity` take the same kind of param). It does **NOT** carry `counter_present`. `counter_present` is computed ONCE in `realize_step:1377-1378` and is already passed to `role_pitch:1417` — it is trivially threadable into the `realize_rhythm` call at `:1465-1476` (see §2b decision).
- `prominence_weight(ctx, Melody)` (`:1278-1287`) returns `PROMINENCE_NEUTRAL(0.5)` on an empty/absent prominence Vec — the identity no-op anchor for every weight-gated term.
- The counter arm (`:2142-2272`) and the seat guard (`:1554`) are unreachable / no-op under identity (`counter_present == false`), so anything keyed off the counter is byte-neutral on the freeze path. The `recede_pad_onsets` helper (`:1233-1270`) ALREADY displaces a Pad block-stab off the downbeat to `PAD_WEAK_BEAT_FRAC * step_ms` while preserving onset COUNT — the EXACT precedent for the F4 offset-modulation tool (move WHERE an onset sits, never HOW MANY).

---

## 2. THE EXACT EDIT SITES — types, signatures, decision tables (NO bodies)

All NEW Rust lives in `src/chord_engine.rs` (Music Theory Specialist owns). NO new module. **NO `realize_step` PUBLIC-signature change** (a private param added to the free fn `realize_rhythm` is permitted, per the `pad_voices`/`ctx` precedent — see §2b). All JSON lives in `assets/mappings.json` (Music Theory authors weights; the schema is additive). The scorecard change lives in `tests/variety_scorecard_s45.rs` (Test Engineer; §5).

### 2(a) — LEVEL: widen the melody-vs-bed level gap (the operator's "bump the melody volume")

Two coordinated, ear-tunable levers. The SIGN of each is fixed; the magnitudes are ear-tunable starts. **This is the FINISH, not the differentiation** (§6): the activity governor (S47), the seat guard (S47), and the Pad recession (Slice-4) already did the figure-ground work — level only widens the already-won gap, it does not carry it.

#### 2(a).i — [JSON] Raise the Melody prominence weight in the deep/mid/shallow tiers (`mappings.json:374-391`)

Raise the `Melody` weight in each tier so the velocity nudge `(w−0.5)*PROMINENCE_VEL_SPAN(18)` at `realize_velocity:1691-1693` opens a wider melody-vs-bed level gap. Recommended starts + ranges (the bed weights are UNCHANGED in this lever — the gap widens from the melody side; the Counter/Fill negative bias in 2(a).ii recedes the bed):

| tier (id) | weight field `:line` | current | recommended START | [range] | sign |
|---|---|---|---|---|---|
| **deep** `melody_lead_strong` Melody | `:375` | 0.90 | **0.92** | [0.90, 0.95] | raise (↑) |
| **mid** `melody_forward` Melody | `:381` | 0.78 | **0.82** | [0.78, 0.85] | raise (↑) — see freeze note |
| **shallow** `melody_lead_gentle` Melody | `:387` | 0.72 | **0.74** | [0.72, 0.78] | raise (↑), kept GENTLEST (a field image must not force a hard lead — §6, Aesthetics) |

**THE MID-TIER FREEZE CONSEQUENCE — stated precisely (the load-bearing subtlety the prompt flags).** The mid `melody_forward` row is the **most-routed** image class (every image not gated deep/shallow) and was deliberately byte-stable in S47 (Slice-1 DECISION-2). Moving it is **freeze-SAFE** but is **NOT a no-op on already-routed non-identity renders**:
- **Freeze-SAFE:** the mid tier is **NOT on the identity path.** The identity/golden/`engine_equivalence` render carries an EMPTY (`uniform`) prominence Vec, so `prominence_weight` returns neutral 0.5 (`:1280-1281`) regardless of any catalogue row — the `melody_forward` weights are never read on the freeze path. Changing 0.78→0.82 cannot perturb the 9/9 goldens or the `ENGINE_SHA256` digest. **WITNESS in §4.**
- **CHANGES already-routed mid renders:** every real image that routes to `melody_forward` (e.g. `example.jpg` ct 0.136, `AudioHaxImg3` ct 0.203 — both MID per the affect review) WILL get a louder melody after the bump. This is the INTENDED effect (the operator's "bump the melody volume"), but it is a non-byte-stable change to the most-common render. **Recommendation: move the mid tier (the operator explicitly asked to bump the melody), but flag it as the single most-audible knob for the A/B taste gate** — it touches the most images. If the taste gate finds the mid bump over-loud on `AudioHaxImg3`/`example.jpg`, hold mid at 0.78 and carry the level finish on deep/shallow only. The producer MUST NOT silently leave mid at 0.78 "to be safe" — that under-delivers the operator's request; the decision is the taste gate's, with the bump as the recommended start.

#### 2(a).ii — [CE] Add the Counter+Fill NEGATIVE velocity-bias arm at the `_ => {}` fall-through (`realize_velocity:1681`)

Replace the `_ => {}` fall-through (`:1681`) with explicit `CounterMelody` and `HarmonicFill` arms carrying a NEGATIVE velocity bias, `!is_cadence`-guarded exactly as the existing arms (the S42 Edit-3 pattern the Melody/Bass/Pad arms already follow at `:1673/:1674/:1680`). This gives the counter/fill a STRUCTURAL level floor below the melody (mirroring the Pad's −3), so their level recedes below the melody independent of (and additive to) the prominence nudge.

The arm the producer writes (NO body beyond the bias values; this is the rule + the recommended magnitudes):

```text
// realize_velocity, the `match role` at :1672-1682 — REPLACE the `_ => {}` (:1681) with:
//   OrchestralRole::CounterMelody if !is_cadence => vel -= <COUNTER_VEL_BIAS>,   // recede below the melody
//   OrchestralRole::HarmonicFill  if !is_cadence => vel -= <FILL_VEL_BIAS>,      // a touch under, less than the counter
//   _ => {}
// (the existing Melody +2 / Bass -1 / Pad -3 arms are UNCHANGED; the final round().clamp(1,127) at :1695 holds the bound)
```

| Knob | recommended START | [range] | sign (FIXED) | rationale |
|---|---|---|---|---|
| `COUNTER_VEL_BIAS` (CounterMelody) | **2.0** (`vel -= 2.0`) | [1.0, 4.0] | **NEGATIVE** (recede below the melody) | the counter is now activity-recessed (S47 governor); a modest level recession completes the figure-ground gap. Kept SMALLER than the Pad's −3 (the counter is a moving line, not a flat bed — it must stay audible as a second voice, S45). |
| `FILL_VEL_BIAS` (HarmonicFill) | **1.0** (`vel -= 1.0`) | [0.5, 2.0] | **NEGATIVE** | the fill is the connective inner tissue; a gentle recession keeps it under the melody without hollowing the harmony. Smaller than the counter's bias (the fill is more recessive than the counter by role). |

> **Naming:** the producer MAY inline the magnitudes as literals (as the existing `+2.0 / -1.0 / -3.0` arms do) OR hoist them to named consts beside the velocity logic (`COUNTER_VEL_BIAS` / `FILL_VEL_BIAS`) for the ear-tuning loop — RECOMMENDED hoist (the taste gate sizes them; named consts make the A/B sweep one-line). Either is freeze-neutral.

### 2(b) — INVERSE-REGISTER COMPENSATION (`inverse_register_compensation`) (`realize_rhythm` melody arm `:2274-2356`)

Route MORE non-level SEPARATION to a melody seated LOW relative to the bed, anti-correlated with the realized seat height — so a low-seated melody holds figure via separation, NOT loudness (operator signal 4, the subtle one; level NEVER, DP-3). **LOCKED DP-3 priority: rhythmic separation FIRST, articulation SECOND, level NEVER.**

#### 2(b).0 — What the F4 metric actually measures (pin the target before designing the tool)

From `tests/variety_scorecard_s45.rs:1189-1220` (re-read this session): per co-sounding step, `f4_gaps[step] = melody_pitch − max(bed_pitch)` (`:1199`) and `f4_seps[step] = (# bed roles whose onset offset ≠ the melody's first onset offset) / (# bed roles)` (`:1200-1217`), where the melody's offset is `mel_off[step]` = the FIRST melody `NoteEvent.offset_ms` on that step (`:1172-1178`). `F4 = correlation(f4_gaps, f4_seps)` (`:1220`); a first-class engine is **NEGATIVE** (small/low gap → HIGH separation; large/high gap → LOW separation).

**The mechanical consequence for the tool (load-bearing):** to make the correlation NEGATIVE, the PRIMARY tool MUST modulate the melody's FIRST onset OFFSET as a function of the realized seat:
- **LOW seat (small register gap to the bed)** → push the melody's first onset OFF the bed's downbeat (offset 0) → `mel_off ≠ bed_off` on most beds → HIGH `sep`.
- **HIGH seat (large register gap)** → leave the melody on the downbeat (offset 0, matching the bed's offset-0 onsets) → LOW `sep`.

This is precisely the negative `correlation(gap, sep)` F4 wants. The SECONDARY tool (articulation) nudges `base_frac` toward more DETACHED (shorter) when low-seated — perceptual separation that the F4 metric does NOT directly read but the taste gate hears (DP-3 rank 4).

#### 2(b).1 The pure helper (NEW; `chord_engine.rs`, free fn beside `melody_activity_class:1073`)

```rust
/// The inverse-register compensation FACTOR for a melody seated at `seat` relative to the bed
/// ceiling — 0.0 when the melody sits clearly ABOVE the bed (no help needed), rising toward 1.0
/// as the realized seat approaches/drops into the bed band (a LOW-seated melody needs MORE
/// separation to hold figure without a louder level). Pure; RNG-free; a function of the realized
/// seat ONLY. The bed reference is COUNTER_CEILING (67) — the same ceiling the seat guard uses;
/// the band below it is [FILL_REGISTER_FLOOR(55), COUNTER_CEILING(67)). theory (DP-3): help is
/// INVERSE to register height (Affect FG-4) — the low melody gets the separation, the high one
/// does not. Returns 0.0 at/above (COUNTER_CEILING + MIN_FIGURE_GAP) (the guarded seat floor),
/// ramping to 1.0 at FILL_REGISTER_FLOOR. The MAPPING shape (linear vs stepped) is the producer's
/// craft call within "monotone decreasing in seat"; recommended LINEAR over the band (§3).
fn inverse_register_compensation(seat: u8) -> f32;
```

- **Domain/range:** input the realized melody seat `note: u8` (in scope in `realize_rhythm` as `note`, `:1783`); output `[0.0, 1.0]`. `0.0` for `seat ≥ COUNTER_CEILING + MIN_FIGURE_GAP` (== 69 at the recommended `MIN_FIGURE_GAP=2`); ramp UP as `seat` falls toward `FILL_REGISTER_FLOOR(55)`; cap at `1.0`. Monotone NON-INCREASING in `seat` (FIXED direction; the magnitude/shape is ear-tunable).
- **WHY the seat is the input, not the gap-to-`max(bed_pitch)`:** the realizer cannot cheaply know the bed's realized max pitch at the melody arm (the Pad/Counter pitches are computed in their own arms / `realize_rhythm` is per-instrument). `COUNTER_CEILING(67)` is the FIXED structural bed ceiling the whole figure-ground design references (the seat guard, the counter band) — it is the correct, in-scope, deterministic bed reference. The scorecard's F4 uses the realized `max(bed_pitch)`; with the seat guard live, the realized counter/fill seats sit in `[55,67)` and the melody seat sits `≥ 69` when high — so `seat − COUNTER_CEILING` tracks the realized `gap` closely enough that compensating off `seat` drives the F4 correlation negative. The producer confirms the proxy holds on the 6-image set at the taste gate.

#### 2(b).2 THE PRIMARY TOOL — onset-OFFSET modulation in the melody arm (`:2274-2356`)

Apply the compensation as an additive onset-OFFSET PUSH on the melody's FIRST emitted onset, scaled by `inverse_register_compensation(note)`, on the bands whose first onset is currently on the downbeat (offset 0) — i.e. the **DOTTED** band (`:2325-2336`, first onset at 0) and the **SUSTAINED** band (`:2337-2355`, onset 0). The producer writes the body; the rule:

```text
// In the melody arm (:2274-2356), AFTER the band ladder builds the event vec, BEFORE returning:
//   let comp = if <GATE — see 2(b).4> { inverse_register_compensation(note) } else { 0.0 };
//   if comp > 0.0 {
//       // push ONLY the FIRST onset (offset 0) of the DOTTED / SUSTAINED branch to a weak-beat
//       // fraction, scaled by comp: offset_push = (comp * COMP_OFFSET_FRAC * step_ms).round();
//       // re-fit that onset's hold so it does not ring across the step boundary (the sustained-
//       // closure :1885 hold logic / the same .min(1.20)-style ceiling the Pad displacement uses);
//       // the onset COUNT is UNCHANGED (push WHERE, never HOW MANY — the recede_pad_onsets precedent).
//   }
```

| Knob | recommended START | [range] | sign (FIXED) | notes |
|---|---|---|---|---|
| `COMP_OFFSET_FRAC` — the max fraction of `step_ms` the first onset is pushed off the downbeat at full comp (1.0) | **0.25** (push up to `step_ms/4` — the same off-beat the counter's MOVING mode uses, `:2256`) | [0.125, 0.375] | **POSITIVE** (push LATER, off the downbeat) | at full comp a low melody attacks on the "and"; at comp 0.0 it stays on the downbeat. Kept ≤ `step_ms/4` so it never crosses into the next beat or collides with the SYNCOPATED band's own `step_ms/4` displacement. |

- **WHICH bands are safe to modulate (the load-bearing constraint).** ONLY the **DOTTED** and **SUSTAINED** bands (first onset at offset 0). The **SYNCOPATED** band already delays its first onset to `step_ms/4` (`:2320-2322`) and the **ARPEGGIO** band already spreads onsets across the step (`:2310-2315`) — both are ALREADY off the bed downbeat (high `sep` already), and both are the `Subdividing` class where the counter MOVES. **Do NOT push the ARPEGGIO/SYNCOPATED first onset** — it would (a) double-displace an already-separated onset, (b) risk colliding with the counter's `step_ms/4` MOVING onset (re-fusing F5a), and (c) the F4 metric reads the FIRST onset offset, which on those bands is already non-zero, so the comp is redundant there. The DOTTED+SUSTAINED restriction is exactly the "low-activity melody that sits ON the downbeat" case — the case F4's correlation is computed across (the low-seat steps are the calm/dark images where the melody is DOTTED/SUSTAINED).
- **It MUST NOT reduce the melody's onset COUNT (F5b/F1 must not regress).** The push moves the FIRST onset's `offset_ms`; the DOTTED band's SECOND onset (at `2/3 step_ms`) and the SUSTAINED band's single onset keep their COUNT. Re-fit the pushed onset's hold so total events per step is IDENTICAL (1 for SUSTAINED, 2 for DOTTED). This preserves `melody_onsets(step)` → F5b (`bed_onsets ≤ melody_onsets`) and F1 (the melody-most-active margin) are UNTOUCHED. **WITNESS in §4.** (The `recede_pad_onsets` displacement at `:1255-1268` is the proven count-preserving-offset-move precedent.)
- **F5a anti-fusion interplay (S47 governor + Pad displacement).** Pushing the melody's DOTTED/SUSTAINED first onset to `step_ms/4` could in principle land it on the SAME offset as the counter's MOVING onset (also `step_ms/4`, `:2256`) — but the counter only takes MOVING when the melody is `Subdividing` (`:2250`), and the comp only fires on DOTTED/SUSTAINED (Oblique/Sustained classes) where the counter is at offset 0 or sustained (`:2260-2270`). So the pushed melody onset (≈`step_ms/4`) is DISTINCT from the counter's onset (0) on exactly the steps the comp fires → F5a separation is IMPROVED, never fused. The Pad recession's surviving stab sits at `PAD_WEAK_BEAT_FRAC * step_ms` (== `step_ms/2` at the default `0.5`, `:1165`) — distinct from `step_ms/4`. **The producer confirms no offset collision on the 6-image set at the taste gate; the math says it is safe.**

#### 2(b).3 THE SECONDARY TOOL — articulation (`base_frac`) detachment when low-seated (`base_frac` `:1862-1874`)

Nudge `base_frac` (the articulation fraction, `:1862-1874`) toward MORE DETACHED (smaller fraction → shorter, more separated notes) as `inverse_register_compensation(note)` rises. DP-3 rank 4 (SECONDARY to the offset push). The producer writes the body; the rule:

```text
// After base_frac is computed (:1874) and the comp factor is known (2(b).2's `comp`):
//   if comp > 0.0 { base_frac = (base_frac - comp * COMP_ARTIC_DETACH).max(ARTIC_WINDOW_LO); }
// (clamped to the existing ARTIC_WINDOW_LO(0.55) floor :1733 so it stays in the pleasant window;
//  a high-seated melody (comp 0.0) is byte-unchanged.)
```

| Knob | recommended START | [range] | sign (FIXED) | notes |
|---|---|---|---|---|
| `COMP_ARTIC_DETACH` — the max `base_frac` reduction at full comp | **0.10** | [0.05, 0.20] | **NEGATIVE on `base_frac`** (more detached when low) | a low melody gets crisper, more separated notes — perceptual figure-ground separation the ear hears (DP-3 rank 4). Floored at `ARTIC_WINDOW_LO(0.55)` so it never clicks. |

- **DO NOT apply the articulation detach to the CADENCE ring.** The cadence path early-returns at `:1904` (`is_cadence`) BEFORE the per-role match — it never reads `base_frac` (it uses `LEGATO_FRAC` via the `sustained` closure's `rit` cap, the 240 ms golden). The comp lives in the melody arm AFTER the cadence early-return, so the cadence ring is structurally untouched. **WITNESS in §4.**

#### 2(b).4 THE FREEZE GATING — does the comp need a new `realize_rhythm` param?

**DECISION (the lead must confirm): YES — add `counter_present: bool` as a new additive PRIVATE param to `realize_rhythm`.** Rationale + the alternatives weighed:

- **The gate (recommended):** the comp fires iff `counter_present && prominence_weight(ctx, Melody) > ACTIVITY_FLOOR_THRESHOLD(0.50) && !is_cadence`.
  - **`counter_present`** is the PRIMARY freeze witness (operator decision A, the seat-guard precedent at `:1554`). The `engine_equivalence` goldens are synthetic NO-COUNTER bars → `counter_present == false` → the comp is a NO-OP on every golden → 9/9 byte-green. This is the same gate that makes the seat guard freeze-neutral, so the freeze argument is IDENTICAL and already proven.
  - **`prominence_weight(ctx, Melody) > 0.50`** (already in scope via `ctx`) is the SECONDARY gate: a foreground melody gets the comp; a neutral (0.5, identity) or recessive melody does not. At neutral 0.5 the strict `>` is FALSE → no comp. (Belt-and-suspenders with `counter_present`; both are FALSE on the identity path.)
  - **`!is_cadence`** keeps the cadence ring untouched (the cadence early-returns at `:1904` anyway, so this is defensive).
- **Why a new param is needed:** `realize_rhythm` does NOT currently receive `counter_present` (CONFIRMED `:1782-1800`). The comp's bed reference (`COUNTER_CEILING`) and freeze witness both depend on a counter actually being present. `counter_present` is ALREADY computed in `realize_step:1377-1378` and ALREADY threaded into `role_pitch:1417` — adding it to the `realize_rhythm` call at `:1465-1476` is a one-line additive thread of an EXISTING value. It is a PRIVATE param on the free fn `realize_rhythm`, exactly like `pad_voices`/`ctx` (`:1794/:1799`) — **`realize_step`'s PUBLIC signature is UNCHANGED**, and `role_pitch`'s signature is unchanged.
- **Alternative WEIGHED and REJECTED (gate on prominence-weight ONLY, no new param):** one could fire the comp on `prominence_weight(ctx, Melody) > 0.50 && !is_cadence` alone (both in scope, no param). This is freeze-safe (neutral 0.5 → no-op). BUT it would also fire on a high-prominence NO-counter render (a `subject_melody`/`melody_forward` profile with no CounterMelody instrument) — a case that exists in the in-place renders. There the bed reference `COUNTER_CEILING` is less meaningful (no counter), and more importantly the FREEZE WITNESS is weaker (it leans only on the 0.5 no-op, not on the structural no-counter fact the goldens exercise). The `counter_present` gate is the cleaner, already-proven witness and matches the seat-guard precedent the comp pairs with. **RECOMMENDATION: add the param.** (If the lead prefers minimal seam churn, the prominence-only gate is freeze-correct — but state the weaker witness explicitly; the param is the recommended, lower-risk choice.)
- **`role_pitch`/`realize_rhythm` signatures may change (private params only); `realize_step`'s PUBLIC signature MUST NOT.** Confirmed: the only seam touched is the private `realize_rhythm` param list + its single call site in `realize_step`.

### 2(c) — [JSON, OPTIONAL] DP-6: CounterMelody mid-tier weight 0.58→0.55 (`mappings.json:382`)

Spec'd as an OPTIONAL, ear-gated knob. **RECOMMENDATION: DEFER to the taste gate / operator A/B — do NOT bundle into the build by default.**

| Knob | site | current | the OPTIONAL change | gate |
|---|---|---|---|---|
| `melody_forward` CounterMelody | `mappings.json:382` | 0.58 | → **0.55** (DP-6) | EAR-GATED — apply only if the taste gate hears the mid-tier counter still competing after the activity recession (S47) + the 2(a).ii negative velocity bias |

- **Rationale for deferring (DP-6, design §6.2):** the S43-pre-staged 0.58→0.55 trim now applies against a counter that is ALREADY activity-recessed (the S47 governor) AND about to get a structural negative velocity bias (2(a).ii). The perceptual basis is different from when it was pre-staged — the counter may no longer need a weight trim. Resolve against the LIVE recessed counter at the taste gate, not at build planning.
- **FREEZE CONSEQUENCE (freeze-SAFE):** the mid `melody_forward` tier is NOT on the identity path (the identity render carries an empty `uniform` prominence Vec → neutral 0.5 → the row is never read). Changing 0.58→0.55 cannot perturb the goldens or the digest. It DOES change already-routed mid renders (a slightly quieter counter) — that is the point, ear-gated. **Same freeze argument as 2(a).i's mid-tier note.**

---

## 3. THE EAR-TUNABLE KNOBS — every magnitude the taste gate sizes (recommended starts)

Every knob ships with a concrete START so the producer builds immediately, AND is flagged EAR-TUNABLE for the standing taste/affect gate (Specialist Marshaling Gate — this is generative/aesthetic work whose acceptance turns on "does the low melody hold as the figure without getting louder?"). The SIGNS/DIRECTIONS are load-bearing and FIXED; the magnitudes want the operator's ear.

| Knob | Where | recommended START | Range | Sign/direction (FIXED) | Source lens |
|---|---|---|---|---|---|
| **deep tier Melody weight** | `mappings.json:375` (2a.i) | **0.92** | [0.90, 0.95] | raise (↑) | aesthetics + affect (level finish) |
| **mid tier Melody weight** | `mappings.json:381` (2a.i) | **0.82** | [0.78, 0.85] | raise (↑) — most-routed; flag for A/B | aesthetics + affect |
| **shallow tier Melody weight** | `mappings.json:387` (2a.i) | **0.74** | [0.72, 0.78] | raise (↑), kept GENTLEST | aesthetics (no forced field lead) |
| **`COUNTER_VEL_BIAS`** | `realize_velocity:1681` arm (2a.ii) | **2.0** (`vel −= 2.0`) | [1.0, 4.0] | **NEGATIVE** (recede below melody) | affect (level recession) |
| **`FILL_VEL_BIAS`** | `realize_velocity:1681` arm (2a.ii) | **1.0** (`vel −= 1.0`) | [0.5, 2.0] | **NEGATIVE**, < counter | affect |
| **`COMP_OFFSET_FRAC`** | melody arm (2b.2) — first-onset push at full comp | **0.25** (`step_ms/4`) | [0.125, 0.375] | **POSITIVE** (push off the downbeat) | DP-3 rank 1 (rhythmic separation FIRST) |
| **`COMP_ARTIC_DETACH`** | `base_frac` (2b.3) — detach at full comp | **0.10** | [0.05, 0.20] | **NEGATIVE on `base_frac`** (more detached low) | DP-3 rank 4 (articulation SECOND) |
| **`inverse_register_compensation` shape** | helper (2b.1) — seat→factor ramp | **LINEAR** over `[FILL_REGISTER_FLOOR(55), COUNTER_CEILING+MIN_FIGURE_GAP(69)]`, factor `1.0→0.0` | shape ear-tunable (linear / stepped / eased) | monotone NON-increasing in seat (FIXED) | affect (DP-3 curve) |
| **`melody_forward` CounterMelody (DP-6)** | `mappings.json:382` (2c) | **0.58 — UNCHANGED (defer)** | optional → 0.55 | NEGATIVE if applied | carry-forward / ear-gated |

> **DP-3 priority is FIXED (not ear-tunable): rhythmic separation FIRST (2b.2), articulation SECOND (2b.3), level NEVER.** The comp NEVER touches the melody's velocity/level — level recession of the bed is the SEPARATE 2(a) lever (bed-side, not melody-side compensation). The taste gate sizes the magnitudes of the offset push and the articulation detach; it does NOT reorder the priority or add a level term to the comp.

---

## 4. FREEZE-NEUTRALITY WITNESS — per edit, re-grounded on the confirmed lines

`engine.rs` is byte-frozen at sha256 `e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261`; the `ENGINE_SHA256` guard (`tests/variety_scorecard_s45.rs`) asserts it and is untouched. The **9/9 `engine_equivalence` goldens** are synthetic NO-COUNTER bars: melody note **79** (= 67 + 12, `lift = +12`), bass **36**, cadence velocities **114 / 84**, cadence hold **240 ms**.

| Edit | Why it is identity-byte-neutral (re-grounded on the confirmed live lines) |
|---|---|
| **2(a).i deep/mid/shallow Melody weight ↑ (JSON)** | The identity/golden render carries an EMPTY `uniform` prominence Vec → `prominence_weight` returns `PROMINENCE_NEUTRAL(0.5)` (`:1280-1281`) REGARDLESS of any catalogue row → the velocity nudge `(0.5−0.5)*18 == 0` (`:1692`). The `melody_lead_strong`/`melody_forward`/`melody_lead_gentle` rows are NEVER read on the freeze path. **The mid bump is freeze-SAFE precisely because mid is NOT on the identity path** (it changes already-routed non-identity renders only — §2a.i). Zero byte impact on the 9/9 goldens. |
| **2(a).ii Counter+Fill negative velocity bias (`:1681`)** | The new arms match `OrchestralRole::CounterMelody`/`HarmonicFill`. Under identity there is NO CounterMelody instrument (`assign_role` never yields it on the identity path, `:1377-1378`); a HarmonicFill MAY exist on the identity path, BUT the bias is `!is_cadence`-guarded and the existing `engine_equivalence` goldens are the synthetic NO-COUNTER bars whose role assignments do not include a HarmonicFill (the goldens are melody-79 + bass-36 bars). **WITNESS the Test Engineer MUST confirm:** re-run `engine_equivalence` after the arm lands — if ANY golden bar emits a HarmonicFill event, the `FILL_VEL_BIAS` would perturb it. The goldens are NO-COUNTER melody/bass bars (`G_MELODY_NOTE=79`, `G_BASS_NOTE=36`) → no Fill role → byte-green. If a golden DOES carry a Fill, gate the Fill bias on `counter_present` too (it is then in scope only if the param is threaded — but the velocity fn does not take it; in that case drop the Fill arm or hand-re-derive). **The melody/bass cadence goldens (114/84, 240 ms) are untouched** — Melody/Bass arms are unchanged and the bias arms are `!is_cadence`-guarded. |
| **2(b).1 `inverse_register_compensation` helper** | A new pure fn; CALLED only from the melody arm under the §2b.4 gate (`counter_present && prominence_weight(ctx, Melody) > 0.50 && !is_cadence`), ALL FALSE on the identity path. Never invoked on the freeze path → zero byte impact. |
| **2(b).2 onset-offset modulation (melody arm `:2274-2356`)** | Gated on `counter_present` (FALSE on the no-counter goldens → no-op) AND `prominence_weight(ctx, Melody) > 0.50` (FALSE at neutral 0.5 → no-op). On the golden render BOTH are false → the DOTTED/SUSTAINED branches emit their EXACT current offsets (0) → the melody-79 bars are byte-identical. The push preserves onset COUNT (1 SUSTAINED / 2 DOTTED), so even a non-identity render keeps F5b/F1. |
| **2(b).3 articulation detach (`base_frac` `:1862-1874`)** | Same gate as 2(b).2 (counter_present + foreground weight + !is_cadence). On the golden path the gate is false → `base_frac` is byte-unchanged. The cadence ring early-returns at `:1904` BEFORE the melody arm → never reads the detached `base_frac` → the 240 ms cadence hold is structurally untouched. |
| **2(b).4 new `counter_present` param on `realize_rhythm`** | Threads an EXISTING value (`:1377-1378`) into the free-fn call (`:1465-1476`) — the `pad_voices`/`ctx` private-param precedent. `realize_step`'s PUBLIC signature is UNCHANGED. On the identity path `counter_present == false` (same value `role_pitch` already receives) → every comp term short-circuits. Zero byte impact. |
| **2(c) DP-6 mid counter weight (JSON, optional)** | If applied: same as 2(a).i — mid `melody_forward` is NOT on the identity path (empty `uniform` → neutral 0.5) → never read on the freeze path → freeze-SAFE. Changes already-routed mid renders only. |

**The one freeze item the lead MUST gate in the build order (§8):** the **2(a).ii Fill velocity bias witness** — confirm the `engine_equivalence` goldens emit NO HarmonicFill event (they are synthetic melody/bass NO-COUNTER bars, so they do not) BEFORE the arm lands. The CounterMelody bias is freeze-trivially safe (no counter on the goldens); the Fill bias is the only velocity-arm term that could in principle touch a non-counter golden. The witness is run FIRST (Test Engineer), then the arm lands.

---

## 5. WHAT THE SCORECARD READS — the F4 instrument promotes from REPORTED toward a sign-asserted gate

The producer's changes are observable through the EXACT per-role `StampedEvent` streams `scorecard_for` already collects (per-(step,role) `offset_ms`, pitch, onset counts). **No new render path, no new image type, no audio, no OpenCV.** The Test Engineer's slice-3 work on `tests/variety_scorecard_s45.rs`:

- **F4 (inverse-comp) is the slice-3 instrument** (`:1164-1224`). Today it is SEEDED + REPORTED-not-asserted (`f4_tag = f4_corr < 0.0 ? OK : FAIL`, printed only; folded into the rollup at `:1312` but NOT red-barred). The comp (2b) drives `correlation(register_gap, separation)` NEGATIVE: on the low-seat (DOTTED/SUSTAINED) steps the melody's first onset is now pushed off the downbeat (`mel_off ≠ bed_off` → high `sep`), while high-seat steps stay on the downbeat (low `sep`) → the correlation goes negative. **Test Engineer adds `inverse_comp_corr: f32` to `LayerVerdicts`** (`:445-469`, mirroring `melody_most_active_margin`) carrying `f4_corr`, and PROMOTES F4 from reported to a SIGN assertion (`corr < 0.0`) on the counter-routed images per the metrics-spec promotion discipline (`spec-s46-figure-ground-metrics.md` §2: "as the inverse-comp slice lands, F4 can be PROMOTED from reported to asserted"). Magnitude stays ear-tuned (sign only is asserted).
- **F5b (the HARD regression gate) MUST stay at 0** (`:1552-1572`, `v.bg_recession_violations <= s46_recession_bound(name)`, post-S47 residual driven to 0). The comp is COUNT-PRESERVING (2b.2 moves WHERE the melody onset sits, never HOW MANY) → `melody_onsets(step)` is unchanged → F5b is untouched. The Test Engineer baselines F5b on the pre-slice-3 tree (still 0) and confirms it stays 0 after the comp + level changes.
- **F1 (melody-most-active)** reads onset COUNTS — unchanged by the comp (count-preserving) and unchanged by the level changes (level ≠ onset count). The 2(a) level changes do NOT regress F1.
- **F3 (melody-highest)** reads `per_step_pitch` — unchanged (the comp touches OFFSET/articulation, never pitch; the seat is the comp's INPUT, not output).
- **F5a (rhythm-distinctness)** can only IMPROVE: pushing the melody's DOTTED/SUSTAINED first onset off offset 0 increases melody-vs-bed offset distinctness on the comp steps (§2b.2).

**The producer adds NOTHING to the render path.** The Test Engineer adds the `inverse_comp_corr` field + the F4 sign promotion, and re-confirms F5b == 0 + F1 margins hold. **Baseline-then-tighten:** baseline F4's corr on the pre-comp tree (≈ 0 / positive = ABSENT), then assert `< 0.0` WITH the comp slice (the staged-bound M1.4 discipline).

---

## 6. PRESERVE STATEMENT (binding) — S45 + the S47 figure-ground gains

- **S45 (the counter MOVES — never silence it).** The 2(a).ii negative velocity bias RECEDES the counter's LEVEL below the melody — it does NOT silence it (a bounded `vel −= 2.0`, floored by `round().clamp(1,127)` at `:1695`). The counter still moves (the S47 governor's MOVING/oblique modes are untouched). DP-6 (2c) is ear-gated and at most trims 0.58→0.55 — still a sounding, moving line. A blanket counter re-suppression or `pad_bed_counter` de-route remains FORBIDDEN.
- **The S47 figure-ground gains (F5b stays 0; melody stays most-active + on top).** Level is the FINISH, not the differentiation — the activity governor + seat guard + Pad recession already made the melody most-active and on-top. The 2(a) level bump MUST NOT undo the 90/10: it is count-neutral (no F1/F5b impact) and pitch-neutral (no F3 impact). **The level bump does NOT carry the figure-ground — differentiation already did the work; level only widens the already-won gap** (operator's explicit framing). If the level bump is removed, the figure-ground hierarchy still holds (it rides on activity + register, not level).
- The comp (2b) holds figure via SEPARATION, not loudness (DP-3: level NEVER in the comp). A low-seated melody pops out through onset-offset + articulation, exactly as operator signal 4 wants.

---

## 7. CROSS-ARC INVARIANTS (out of scope for slice 3 — state them so they are not broken)

- **CLIMAX-BLOOM (S44 slice 2 / S46 tension #5).** The bed may bloom in density at the climax ONLY IF the figure-ground gap blooms WITH it. Slice 3's level + comp changes are per-step and section-agnostic (the comp keys off the realized seat; the level off the per-image tier) — they do NOT pin a constant margin and do NOT make the gap section-blind. The Pad recession's `melody_min_onsets`-relative cap (`:1177-1213`) already lets the bed track the melody's bloom. Slice 3 keeps the margin per-section-measurable (it does — level/comp ride on top of the per-step F1/F3/F4). DO NOT encode anything that prevents the later climax guard ("F1 margin at climax ≥ F1 margin at Statement").
- **F5a anti-fusion (S42) + the S47 governor.** The comp's offset push is confined to DOTTED/SUSTAINED (Oblique/Sustained) steps where the counter is at offset 0 / sustained — DISTINCT from the pushed melody onset (≈`step_ms/4`), so F5a separation improves, never fuses (§2b.2). DO NOT push the ARPEGGIO/SYNCOPATED first onset (the `Subdividing` steps where the counter MOVES at `step_ms/4`).

---

## 8. S47 EAR-GATED WATCH-ITEMS carried forward (design notes for the taste gate — do NOT build fixes unless trivially coupled)

These are S47/Slice-1 watch-items the taste gate checks at the slice-3 A/B. **Do NOT build fixes here unless trivially coupled** to a slice-3 change:

1. **The seat-lift leap (`MIN_FIGURE_GAP` 2→1).** If the seat guard (`:1554`) leaps the melody jarringly on a dark counter image, drop `MIN_FIGURE_GAP(:1106)` 2→1. NOT a slice-3 build item — but NOTE: `inverse_register_compensation` (2b.1) ramps to 0.0 at `COUNTER_CEILING + MIN_FIGURE_GAP`, so if the taste gate moves `MIN_FIGURE_GAP`, the comp's high-seat zero-crossing moves WITH it (1:1 coupled, no extra edit — the helper reads the same const). Flag this coupling to the taste gate.
2. **Deep-bed thinness on busy-subject held passages.** Fix from the bed-FLOOR side (`PAD_ONSET_FLOOR :1154`), NOT the ceiling. NOT a slice-3 item (the 2(a) level bump does not touch onset counts, so it neither causes nor fixes this). Carry as a taste-gate listen-item.
3. **Img3 mid-tier counter blur (deep gate → 0.20).** `AudioHaxImg3` (ct 0.203) routes MID and gets only the 0.20 mel−counter gap with a MOVING counter — the affect review's flagged perceptual mis-bin. The 2(a).ii negative velocity bias + the (optional) DP-6 trim BOTH widen the mid mel−counter gap, which may RESOLVE the blur WITHOUT moving the deep gate. **Taste-gate decision:** A/B `AudioHaxImg3` after the 2(a)/2(c) level changes; if the melody still blurs into the counter, lower the deep gate (`mappings.json:397` `fg_bg_contrast ge 0.25` → 0.20) so Img3 routes DEEP. NOT a default build change — ear-gated, but trivially coupled to the level finish (the level bump is the cheaper fix to try first).

---

## 9. RISKS / OPEN MICRO-DECISIONS for the producer

1. **The `seat → comp` proxy (2b.1).** The comp keys off `COUNTER_CEILING(67)` as the bed reference, not the realized `max(bed_pitch)` the F4 metric uses. With the seat guard live the realized bed sits in `[55,67)` and the high melody at `≥69`, so the proxy tracks the metric closely — but the producer CONFIRMS at the taste gate that the F4 corr actually goes negative on the 6-image set (the metric is the before/after instrument). If a deep-tier image seats the melody above 69 on every step (no low-seat steps), F4 has no low-gap samples to correlate — that is expected (the comp fires on the LOW-seat dark/calm images, which are the ones with a small register gap). **Producer confirms; not a lead decision.**
2. **The DOTTED/SUSTAINED-only push restriction (2b.2) is load-bearing.** Pushing the ARPEGGIO/SYNCOPATED first onset would double-displace and risk F5a re-fusion with the counter's MOVING onset. LOCKED to DOTTED+SUSTAINED here. Flag if the ear wants the comp on more bands (it should not — those bands are already separated).
3. **The mid-tier level bump (2a.i) touches the most-routed images.** It is the operator's explicit request and freeze-safe, but it is the single most-audible knob. **The lead/taste gate confirms the mid bump magnitude** (0.78→0.82 start; hold at 0.78 if over-loud on Img3/example.jpg). Stated so the producer does not silently skip it (under-delivering) NOR over-bump it.
4. **`COUNTER_VEL_BIAS` / `FILL_VEL_BIAS` floor interaction.** The biases ride the existing `round().clamp(1,127)` at `:1695` — a deeply-recessed counter on a quiet (low-saturation) image could clamp toward 1. The bias magnitudes ([1,4] / [0.5,2]) are sized so a normal render stays well above 1; the producer confirms no counter clips to silence at the taste gate (S45 forbids a silent counter).
5. **DP-6 deferral (2c).** RECOMMENDED defer to the taste gate. If the lead wants it bundled, it is freeze-safe (mid not on identity path) — but the design RECOMMENDS resolving it against the live recessed + velocity-biased counter, not at build planning.
6. **Single-writer on `mappings.json`.** The level weights (2a.i) and the optional DP-6 (2c) go through single-writer coordination (the S42 discipline) — Music Theory owns the rows; do not race the file.

---

*Design-only. No source, test, or asset modified by this document. `src/engine.rs` sha256 re-verified UNCHANGED this session: `e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261`.*

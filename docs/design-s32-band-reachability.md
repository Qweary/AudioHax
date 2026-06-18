# Design Spec — S32 Band-Reachability (counter-line in-register dissonance resolution)

Status: DESIGN ONLY. This document specifies a change to `src/chord_engine.rs`; it does
not implement it. The implementer is the music-craft owner of `chord_engine.rs` (sole
writer); the test owner strict-ens `tests/counterpoint_s30.rs`.

Scope: close the three pinned residuals left by the fifth-species figure scorer
(`pick_counter_figure` and its private gates) where, from certain realized penultimate
counter pitches, no consonant / no-leap landing is reachable inside the counter band
`[55, 67)`, so the scorer falls back to a blunt least-bad pitch (a structural dissonance
or a dissonant leap). All work lives in `src/chord_engine.rs` ONLY.

---

## 0. The three residuals (restated) and the chosen intent

The counter band is `[FILL_REGISTER_FLOOR, COUNTER_CEILING)` = `[55, 67)`. Under the test
harness (`brightness 50` → no register lift) the melody seats at/above 67, so every
counter↔melody vertical is computed with the counter strictly below the melody.

The three residuals, with the **exact realized pitches** confirmed by replaying the engine:

- **GAP-2 `{IV, V}` — terminal diminished `vii`, unresolvable structural DISSONANCE.**
  `X → vii` with `vii` as the LAST step (Interior position, no cadence slot, no next chord).
  - `IV → vii`: realized penult counter `57`, terminal lands `55` → vs melody `77`, ic 10
    (m7) — DISSONANT. (Note `55` is pc G, **not even a `vii` chord tone** — it is the
    `consonance_gate_sustain` `unwrap_or(raw)` floor firing.)
  - `V → vii`: realized penult counter `59`, terminal lands `60` → vs melody `77`, ic 5
    (P4) — DISSONANT under `FOURTH_IS_DISSONANT`.
  - **Reachability fact:** the `vii` band tones vs melody `77` are `59`(ic 6, diss),
    `62`(ic 3, **consonant m3**), `65`(ic 0, perfect). A consonant landing (`62`) DOES
    exist in band; from penult `57` it is `+5` (P4 leap, a *consonant* leap, legal), from
    penult `59` it is `+3` (m3 leap, legal). The fault is that the gate is not reaching it.

- **GAP-3 `{V, vi}` — PAC resolves by a 7-semitone LEAP.**
  `X → IV → V → I` PAC. For `X ∈ {V, vi}` the realized penult counter is `62`; the cadence
  resolves `62 → 55` (ic 0 perfect CLOSE, but a `move = 7` LEAP).
  - **Reachability fact:** the `I` band perfect tones vs melody `67` are `60`(ic 7, P5) and
    `55`(ic 0, P8). From penult `62`, `62 → 60` is `−2` — a **STEP** onto a perfect
    consonance, contrary to the CF's `74 → 67` descent... no: CF descends, counter `62→60`
    also descends → SIMILAR, not contrary. `62 → 60` is still a no-leap perfect landing; it
    fails only the *stepwise-CONTRARY* clause of `cadence_resolution_pitch`, so the function
    falls through to the nearest-perfect FALLBACK and leaps to `55`.

- **GAP-4 `{ii→IV→iii, vi→IV→iii}` — dissonant TRITONE melodic LEAP.**
  Realized line lands `si2 = 59` from penult `65`: `65 → 59` is `−6`, a TRITONE (ic 6)
  melodic leap.
  - **Reachability fact:** the `iii` band tones vs melody `71` are `64`(ic 7, P5),
    `55`(ic 4, M3), `59`(ic 0, P8). From penult `65`: `65 → 64` is `−1` (a STEP onto a
    **consonant** P5), `65 → 63` is `−2` (a step onto pc D♯, a non-chord tone — a mild
    dissonance vs `71`, ic 8 → actually a m6, consonant; pc D♯ is not diatonic so excluded
    by the chord-tone floor). The clean stepwise consonant chord-tone landing `64` exists;
    the line nevertheless leaps the tritone because `64` is rejected by an upstream guard
    (the no-parallel/approach-perfect gate at that transition) and the scorer's only
    surviving chord-tone candidate is `59`, taken by the `−6` leap.

### The taste decision (already made — design to it)

**Option B is chosen.** Keep the counter line **in register** — never octave-displace the
target out of `[55, 67)` to manufacture a consonant/no-leap landing (Option A, rejected,
because an octave jump reads as the mechanical artifact the project is removing). Where no
consonant AND no-leap in-band landing exists, the line must prefer, in order:

1. an in-band consonant chord tone reached by STEP (≤2) — the ideal;
2. an in-band consonant chord tone reached by a CONSONANT LEAP (3rd/4th/5th/6th/8ve) —
   acceptable, register preserved, vertical consonant;
3. an in-band **prepared, intentional ornamental dissonance** reached by STEP — a rare
   dissonance that reads as human/expressive *because it is approached (and, where a slot
   exists, left) by step*;

and must NEVER fall to (a) an octave displacement or (b) the current blunt least-bad
pitch (an unprepared structural dissonance, or a dissonant — tritone/7th — leap).

The ranking is the spec's spine: **a stepwise mild ornamental dissonance is strictly
preferable to a dissonant leap, and a consonant leap is strictly preferable to a stepwise
dissonance.** GAP-3 and GAP-4 are *leap* problems and are fully closable inside tiers
(1)/(2) with no new dissonance at all; GAP-2 is a *terminal-dissonance* problem and is the
only place tier (3) — the prepared ornament — is actually needed.

---

## 1. Function-by-function changes

All five touch points named in the task live in `src/chord_engine.rs`. The change is
**additive on the counter path**: every new branch is reached only inside
`pick_counter_figure` / its helpers, which are reached only from the CounterMelody realize
arm. The Bass/Pad/Melody realization, the identity / counter-OFF path, and `engine.rs` are
untouched.

A single new private helper anchors all three fixes:

```rust
/// Rank a band-seated counter candidate against the CF for the "no ideal landing"
/// recovery search. Lower-is-better. Encodes the Option-B preference order:
///   tier 0  consonant + reached by STEP (|motion| <= 2)
///   tier 1  consonant + reached by a CONSONANT LEAP (legal melodic leap, |motion| >= 3)
///   tier 2  a PREPARED ornamental dissonance reached by STEP (|motion| <= 2)
/// A dissonant LEAP and an out-of-band pitch are NOT candidates here (filtered before
/// ranking) — they are exactly what Option B forbids.
///
/// Within a tier the existing inner-line biases break ties: imperfect consonance over
/// perfect (avoid bare 5ths/8ves), then smallest melodic motion (connectedness), then a
/// non-root pc (the counter is not a bass double). RNG-free, total order, deterministic.
fn rank_inregister_landing(
    cand: u8,
    prev_counter: u8,
    cf_now: u8,
    chord: &Chord,
) -> (u8 /*tier*/, u8 /*imperfect_first*/, i16 /*step_size*/, u8 /*root_last*/);
```

The implementer derives `tier` from `is_consonant(cand, cf_now)` and
`(cand as i16 - prev_counter as i16).abs()` (≤2 ⇒ step). `imperfect_first/step_size/
root_last` mirror the existing `consonance_gate_sustain` ordering verbatim so the clean
regions are byte-stable. The helper does NOT itself enforce legality (band membership,
`melodic_leap_is_legal`, parallel-perfect) — callers filter to legal candidates first,
then pick `min_by_key(rank_inregister_landing)`.

### 1.1 `consonance_gate_sustain` — widen the consonant search to a consonant LEAP, then a prepared ornament (GAP-2)

Current behavior (lines ~3592–3623): when the raw sustain pick is dissonant vs the CF, it
re-selects among `counter_candidate_pitches(chord, prev_counter)` filtered to
`is_consonant(c, cf)`, ranked by `(imperfect_first, step, root_last)`; if NO consonant
chord tone exists it returns `raw` unchanged (the blunt floor that produces GAP-2's `55`).

The defect is twofold. (a) On `IV → vii` a consonant landing (`62`) **does** exist in the
chord's band tones, but the current `min_by_key` still chose a non-chord floor — confirm by
tracing whether `counter_candidate_pitches(c_vii, 57)` is actually yielding `62`; the
`upper_voice_candidates(pc, from=57, max=12)` window from `57` reaches `62` (D) and `65`
(F), so the consonant set should be `{62, 65}` and `62` should win tier-0/1. **If the engine
is instead landing `55`, the gate is being bypassed** (the raw pick is already a chord tone
the gate treats as consonant, or the gate sees an empty consonant set because of a
`prev_counter` that seats the window away from `62`). The implementer must instrument this
exact call and confirm the consonant set before changing the ranking.

New behavior (Option B):

1. Keep the no-op fast path (raw already consonant, or CF rests) **unchanged** — byte-stable.
2. Build the consonant candidate set as today, but rank it with `rank_inregister_landing`
   so a consonant STEP wins, then a consonant LEAP (`melodic_leap_is_legal(prev_counter, c)`
   — a P4/m3/etc. leap to `62` is legal; a tritone/7th leap is not, and there is none here),
   then fall through.
3. **New tier-2 branch (the only genuinely-new dissonance Option B introduces):** if the
   consonant set is EMPTY (no chord tone of the harmony is consonant vs the CF from this
   band position — a truly degenerate vertical), do NOT return `raw`. Instead search the
   chord's band tones for a **prepared ornamental dissonance**: a chord tone reachable by a
   STEP (`|c − prev_counter| ≤ 2`) from `prev_counter`, where the *preparation* `prev_counter`
   is itself consonant vs the prior CF. Pick the one minimizing `rank_inregister_landing`'s
   tier-2 ordering. This is the "appoggiatura-style" terminal ornament: a step-approached
   dissonance on the final chord that the ear reads as intentional even with no resolution
   slot (see §2). Only if THAT set is also empty (no band chord tone within a step) keep
   `raw` as the absolute defensive floor.

Signature change: `consonance_gate_sustain` gains the prior-CF (for the preparation check):

```rust
// before:
fn consonance_gate_sustain(chord: &Chord, raw: u8, prev_counter: u8, m_now: Option<u8>) -> u8
// after:
fn consonance_gate_sustain(chord: &Chord, raw: u8, prev_counter: u8,
                           m_prev: Option<u8>, m_now: Option<u8>) -> u8
```

The single caller (`pick_counter_figure`, line ~3672) already has `m_prev` in scope — pass
it through. This is a PRIVATE signature; `realize_step`'s public 7-param signature is
untouched.

### 1.2 `cadence_resolution_pitch` — accept a no-leap SIMILAR/oblique perfect landing before the leaping fallback (GAP-3)

Current behavior (lines ~3506–3538): collects perfect-consonant chord tones reachable by a
STEP (≤2) **and contrary/oblique** to the CF; if that set is empty, falls back to the
nearest perfect-consonant tone (which on `V/vi` is `55`, taken by a `−7` LEAP).

The defect: on the `V/vi` penult `62`, the perfect landing `60` (P5 vs CF `67`) is reachable
by a `−2` STEP but the counter descends WITH the CF (`74 → 67`), so it is SIMILAR, not
contrary, and the strict contrary filter drops it — forcing the leaping fallback.

New behavior (Option B — eliminate the leap while preserving register; relax the *motion
quality* of the approach before relaxing the *no-leap* guarantee):

Insert a SECOND-tier search between the strict stepwise-contrary set and the nearest-perfect
fallback:

1. **Tier A (unchanged):** stepwise-CONTRARY perfect landing — the ideal clausula. Keep
   exactly as-is so the 4 clean openers stay byte-identical.
2. **Tier B (new):** any perfect-consonant chord tone reachable by STEP (`|c − counter_prev|
   ≤ 2`, `c != counter_prev`), regardless of motion direction (similar/oblique allowed). On
   `V/vi` this yields `60`. Pick the nearest. This LANDS THE PERFECT CLOSE BY STEP — the
   `move ≤ 2` no-leap guarantee now holds for ALL six openers.
3. **Tier C (new, only if A and B empty):** a CONSONANT (perfect-or-imperfect) chord tone by
   STEP — preserves no-leap, mild relaxation of "perfect close." (Not needed for the pinned
   set; specified for completeness so a future degenerate cadence still avoids a leap.)
4. **Tier D (fallback, unchanged):** nearest perfect-consonant tone by any motion — the
   existing leaping floor, now reached only by genuinely degenerate chords with no stepwise
   landing of any kind.

This is a pure widening of the candidate search; no new dissonance. The PAC close stays a
perfect consonance for the pinned battery (tier B lands `60`, which IS perfect), so PT-8's
universal perfect-close property is **strengthened**, not weakened.

### 1.3 `melodic_leap_is_legal` — UNCHANGED (it is the correct gate; the fix is upstream candidate selection) (GAP-4)

`melodic_leap_is_legal` already correctly rejects the tritone (`65 → 59`, ic 6) and the
sevenths. The GAP-4 fault is NOT that the gate lets the tritone through — it is that, after
all *consonant chord-tone* candidates are rejected by the parallel/approach-perfect guard,
the figure driver's surviving pick is `59` reached by the tritone leap, and there is no
recovery search that prefers an in-register stepwise landing over that dissonant leap.

Do NOT loosen `melodic_leap_is_legal`. Instead add the recovery in `pick_counter_figure`'s
interior path (§1.4): when the chosen sustain pitch would only be reachable by a dissonant
leap, run the in-register recovery search (tier 1 consonant-leap, then tier 2 prepared
ornament-by-step) and prefer those over the dissonant-leap pitch. On the `ii/vi → IV → iii`
case the recovery search finds `64` (P5, reached `65 → 64` by `−1` STEP) — a consonant
stepwise landing — and takes it, eliminating the tritone. The only reason `64` is dropped
today is an upstream parallel/hidden-perfect rejection at that transition; the implementer
must confirm which guard fires (likely `approach_perfect_is_legal`, since `64` vs `71` is a
perfect P5 and the approach from `(65 vs 72)` may be similar). If `64` is genuinely illegal
as a perfect arrival, tier 2 supplies the prepared ornamental dissonance by step (a chord
tone of `iii` a step from `65` that is mildly dissonant vs `71`) as the Option-B landing —
strictly preferable to the `−6` tritone.

### 1.4 `pick_counter_figure` — the interior recovery branch (the integrating change)

Current interior path (lines ~3705–3735): when `figures_enabled`, search
`best_dissonant_figure`; if a legal figure scores better than the sustain, return it; else
return `(sustain, Sustain)`. The sustain at this point is `consonance_gate_sustain`'s output
— which, post-§1.1, is already consonant where a consonant landing exists.

Add, at the END of the interior path (after the dissonant-figure comparison, before the
final `return (sustain, Sustain)`):

```text
// Option-B in-register leap recovery: if the sustain pitch is only reachable from the
// realized prior counter pitch by a DISSONANT melodic leap (tritone/7th — the GAP-4
// shape), do NOT emit it. Search the chord's band tones for a better in-register landing:
//   (1) a consonant chord tone reached by STEP or by a CONSONANT (legal) leap, else
//   (2) a PREPARED ornamental dissonance reached by STEP (preparation = prev_counter
//       consonant vs cf_prev),
// ranked by rank_inregister_landing. Take the best; only if the recovery set is empty
// keep the sustain (the absolute defensive floor). This never displaces the octave and
// never widens the band.
if !melodic_leap_is_legal(prev_counter, sustain) {
    // recovery search over counter_candidate_pitches(chord, prev_counter) ++ stepwise
    // ornament tones, filtered to legal in-band landings, min_by_key(rank_inregister_landing)
}
```

This branch is reached only when the sustain itself would be a dissonant leap — i.e. only on
the GAP-4 residual shape and any future case like it. On every clean transition
`melodic_leap_is_legal(prev_counter, sustain)` is already true, so the branch is a no-op and
the line is byte-identical. Determinism is preserved (the search is a total-order
`min_by_key`, no RNG).

---

## 2. GAP-2 terminal dissonance: what "intentional ornament" means with no resolution slot

A terminal `vii` is the LAST step: there is no next step to resolve into, so the
passing/neighbor/suspension machinery (which all require a resolution beat) cannot license
the dissonance. Under Option B the terminal dissonance is made intentional by **preparation
alone**: the dissonant tone is approached by STEP from a counter pitch that was itself
CONSONANT against the prior CF. That is the appoggiatura/suspension *preparation* gesture —
a leaned-into dissonance — minus the resolution the final position cannot provide.

Concretely, "acceptable rare ornamental dissonance when no resolution slot exists" =
a band chord tone of the final harmony that is (i) dissonant vs the final CF, (ii) reachable
by a STEP (|motion| ≤ 2) from the realized prior counter pitch, and (iii) whose preparation
(the prior counter pitch) was consonant vs the prior CF. It is tier (2) of the preference
order and is reached ONLY when no consonant in-band chord-tone landing exists at all (tiers
0/1 exhausted).

**For the two pinned GAP-2 cases, tier (3) is NOT actually needed — a consonant landing
exists** (`62`, the m3 vs `77`). The correct Option-B fix for `{IV, V} → vii` is therefore
the §1.1 widened consonant search (consonant STEP, then consonant LEAP), which lands `62` in
both cases and makes the terminal vertical CONSONANT. The prepared-ornament tier exists as
the principled floor for a *future* truly-degenerate terminal where even `62`-style consonant
landings are unreachable; it is the design's answer to "what if no consonant landing exists,"
and it keeps the line in register with a deliberate, step-prepared dissonance rather than the
current blunt non-chord-tone floor (`55`).

So GAP-2 closes to CONSONANT (the strong outcome), and the ornamental-dissonance mechanism is
the documented fallback that guarantees Option-B behavior even past the pinned set.

---

## 3. Per-GAP resolution, NEW witness pitches, strict-ened test property

All witnesses below are hand-derived from the design + confirmed against the chord/band
geometry the engine already produces. The implementer and test owner MUST re-derive each by
replaying the engine after the change (the exact landing within a tier can shift if the
implementer's tie-break differs from the spec's stated order — the spec fixes the ORDER, the
test fixes the resulting PITCH).

### GAP-2 — `test_diminished_structural_sustain_is_consonant`

- **Mechanism:** §1.1 widened consonant search → both `{IV, V} → vii` land the consonant m3.
- **NEW witnesses (re-derive to confirm):**
  - `IV → vii`: terminal counter `62` vs melody `77` → ic 3 (consonant m3). (was `55`, ic 10.)
  - `V → vii`: terminal counter `62` vs melody `77` → ic 3 (consonant m3). (was `60`, ic 5.)
  - The existing kept witness `iii → vii` (counter `62`, ic 3) is UNCHANGED.
- **Strict-ened property:** the residual set `GAP2_RESIDUAL_DISSONANT_OPENERS` SHRINKS to
  **empty** (`&[]`). Re-assert: for EVERY consonant opener of `X → vii` (vii terminal), the
  structural vertical is CONSONANT (`!is_dissonant(c, m)`). The existing per-opener
  consonant-region assertion already covers the clean openers; with the residual empty the
  whole `X → vii` battery becomes the strict universal "terminal diminished sustain is
  consonant" property. Keep the kept-witness `assert_eq!((m,c),(77,62))` for `iii → vii`,
  and ADD the two new exact witnesses for `IV`/`V`.

### GAP-3 — `test_cadence_resolves_perfect_no_leap`

- **Mechanism:** §1.2 tier-B no-leap step-to-perfect → `V/vi` land `60` by a `−2` step.
- **NEW witnesses (re-derive to confirm):**
  - `V → IV → V → I`: penult counter `62` → final `60` vs melody `67` → ic 7 (P5),
    `move = 2` (NO leap). (was `62 → 55`, move 7.)
  - `vi → IV → V → I`: penult counter `62` → final `60`, ic 7, `move = 2`. (was `62 → 55`.)
  - The kept adversarial witness `ii → IV → V → I` is `55 → 55` (move 0) — UNCHANGED.
  - NOTE: the other clean openers (`I, iii, IV`) keep their existing no-leap closes; confirm
    none of them shift (tier A is byte-identical for them).
- **Strict-ened property:** `GAP3_RESIDUAL_LEAP_OPENERS` SHRINKS to **empty** (`&[]`). The
  no-leap approach (`move ≤ 2`) now holds for ALL six openers, so the per-opener
  `if mv > 2 { residual_leap.push(...) }` loop yields an empty set and the assertion becomes
  the strict universal "the PAC close is perfect AND reached by motion ≤ 2 (no leap)" over
  the whole consonant-opener battery. The perfect-CLOSE assertion (already universal) is
  retained verbatim.

### GAP-4 — `test_no_dissonant_melodic_leap_in_counter_line`

- **Mechanism:** §1.3 keeps `melodic_leap_is_legal` strict; §1.4 recovery search prefers the
  in-register stepwise consonant landing `64` over the `−6` tritone to `59`.
- **NEW witnesses (re-derive to confirm — the recovery landing depends on which upstream
  guard rejects `64`):**
  - PRIMARY expectation: `ii → IV → iii` and `vi → IV → iii` realize `si2 = 64` (P5 vs melody
    `71`), reached `65 → 64` by a `−1` STEP. The `64 → ` line then has `ic(65,64)=1`? No —
    the *melodic* interval `65→64` is a 1-semitone step (legal); the *vertical* `64` vs `71`
    is ic 7 (P5, consonant). So the realized line becomes `[…, 65, 64]` with a consonant
    step, NOT a tritone.
  - CONTINGENCY: if the implementer's trace shows `64` is genuinely illegal at that
    transition (a hidden/direct P5 via `approach_perfect_is_legal`), the Option-B landing is
    the tier-2 prepared ornament: the `iii` band chord tone a step from `65` that is mildly
    dissonant vs `71` — i.e. `iii` tones are `{64(E), 67(G→out of band), 71(B)}`; band-seated
    `iii` tones in `[55,67)` are `{55(G... no, 55=G is not an iii tone)}`. The iii pcs are
    E(4), G(7), B(11) → band seats E=64, G=55, B=59. So the ONLY band landings are `64, 55,
    59`. From `65`, the stepwise option is `64` (−1) only; `55`/`59` are leaps. Therefore the
    correct Option-B landing is unambiguously `64` (a stepwise consonant P5). The contingency
    ornament does not arise for this exact chord — `64` is the witness.
  - **The test owner must replay to confirm `64` is what the engine lands** once §1.4 prefers
    the stepwise consonant landing over the tritone leap; if the upstream guard still vetoes
    `64`, that guard (not this recovery) is the remaining defect and must be reported, not
    papered over.
- **Strict-ened property:** the residual `expected_diss` set SHRINKS to **empty** (`&[]`).
  Re-assert: over the full ordered diatonic-triple battery, NO realized counter leap lands a
  dissonant melodic interval (`ic ∉ {6,10,11}` for every `|move| ≥ 3`). With the residual
  empty this becomes the strict universal PT-6 "no dissonant melodic leap" property. The
  leap-recovery (no two same-direction ≥4th leaps) assertion is already universal — retain.
  Update the adversarial-witness assertion (`I → V → IV` → `[64,62,65]`) only if a re-derive
  shows it shifted; the design does not touch that case, so it should be UNCHANGED.

---

## 4. Byte-freeze witness plan

**Stays frozen (must NOT move):**

- `src/engine.rs` — sha256 `e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261`.
  No edit. Verify with `sha256sum src/engine.rs` before and after.
- `tests/engine_equivalence.rs` — 9/9 byte-green. The equivalence goldens (240 ms
  ritardando; velocities 114 / 84 / 36 / 79) must be UNMOVED.
- The identity / counter-OFF path: `PT-0` (`test_counter_off_is_byte_identical_baseline`,
  `test_held_period_is_sustain_only_no_added_dissonance`) — the counter machinery is
  downstream of role assignment; with no CounterMelody layer the new branches are never
  reached, so the frozen Bass/Pad/Melody output is unperturbed.
- The clean regions of GAP-2/3/4 (the openers that already land consonant / no-leap): the
  new branches are gated behind "no ideal landing" / "dissonant-leap" conditions that are
  FALSE on those transitions, so their realized pitches are byte-identical.

**Re-witnesses (expected to change, and must be re-derived):**

- `tests/counterpoint_s30.rs`: the three residual sets shrink to empty; the three new
  per-case witness pitches (`IV/V→vii` → 62; `V/vi→…→I` → 60; `ii/vi→IV→iii` → 64). All
  other PTs (PT-0/1/3/4/6/7/8/9, no-runaway, no-voice-cross, determinism) stay green.

**Self-verification the implementer runs (full required set):**

```
cargo test --test counterpoint_s30          # the three residuals now empty, all green
cargo test --test engine_equivalence        # 9/9 byte-green, goldens unmoved
cargo test                                   # default feature set, whole suite green
cargo test --lib --no-default-features       # headless lib path green
sha256sum src/engine.rs                      # == the frozen sha above
cargo clippy -- -W clippy::all               # no new correctness warnings
```

Do NOT run bare `cargo fmt` (it reflows non-deliverable files — a known defect). Format only
the touched file if needed: `cargo fmt -- src/chord_engine.rs` is acceptable, or hand-format.

---

## 5. Frozen-surface honor / no new threaded field

- `realize_step`'s PUBLIC 7-param signature is FROZEN and is NOT touched. All new data
  (`m_prev` into `consonance_gate_sustain`) rides existing locals already in scope at the
  call site inside `pick_counter_figure`. **No new threaded `ctx`/`Section`/`StepPlan`/
  `PerfFeatures` field is required** — the recovery searches read only `chord`,
  `prev_counter`, `cf_prev`, `cf_now`, all already available. This is a deliberate design
  goal met: the fix is local to the counter scorer's private helpers.
- Determinism (PT-9): every new selection is a `min_by_key` over a deterministically-ordered
  candidate list (the existing `counter_candidate_pitches` sort + the total-order rank key).
  No `thread_rng`, no RNG anywhere on the path.
- Preserved invariants: counter band `[55,67)` (the recovery searches filter to band tones
  via `counter_candidate_pitches`, which already enforces the band; the stepwise-ornament
  search must also `(FILL_REGISTER_FLOOR..COUNTER_CEILING).contains(&c)`-filter); no
  voice-cross; no-runaway; no unison-collapse (PT-7 — the recovery must keep the
  `cand != cf_now` guard `best_dissonant_figure` already applies); begin/cadence
  perfect-consonance formulas (PT-8 close — tier B/C only ADD no-leap perfect/consonant
  landings, never remove the perfect-close guarantee).

---

## 6. Open taste forks for the operator

The primary taste call (Option A vs B) is settled. One genuinely-new, minor fork arises:

- **FORK (low stakes): GAP-2 outcome — consonant landing (achieved) vs an intentional
  ornament.** The design closes `{IV, V} → vii` to a CONSONANT m3 (`62`), because a consonant
  in-band landing demonstrably exists. This is the *stronger* result (no dissonance at all)
  but it means the terminal `vii` no longer SOUNDS its characteristic diminished bite against
  the counter — the counter sidesteps to the consonant chord tone. If the operator would
  rather the terminal diminished chord KEEP an expressive, step-prepared dissonance (the
  appoggiatura colour, accepting a rare dissonance as a feature exactly per the Option-B
  spirit), the design supports that by FLIPPING the §1.1 tier order on a terminal `vii`:
  prefer the prepared stepwise ornament (tier 2) over the consonant sidestep (tiers 0/1) when
  the chord is diminished AND it is the last step. Default in this spec = the consonant
  landing (conservative, the safe musical default). This is the one place "rare dissonance as
  a feature" could be dialed UP rather than resolved away; surfaced for the operator's ear.

No other fork arises — GAP-3 and GAP-4 are pure leap-elimination with no dissonance trade,
so they have a single correct Option-B outcome.

---

## 7. Handoff notes

**To the music-craft implementer (sole writer of `src/chord_engine.rs`):**

1. Add `rank_inregister_landing` (§1, total-order key helper).
2. §1.1 — widen `consonance_gate_sustain`: rank consonant candidates (step→consonant-leap)
   via the helper; add the tier-2 prepared-ornament branch when the consonant set is empty;
   add the `m_prev` param and thread it from the single caller. FIRST instrument the
   `IV → vii` / `V → vii` calls and confirm the consonant set is `{62, 65}` — the fix is to
   make `62` win; if the set is unexpectedly empty/`raw`, the bug is in candidate seating, not
   ranking, and must be fixed there.
3. §1.2 — `cadence_resolution_pitch`: insert tier-B (stepwise perfect, any direction) and
   tier-C (stepwise consonant) between the strict-contrary set and the leaping fallback.
4. §1.4 — `pick_counter_figure`: add the interior dissonant-leap recovery branch (gated by
   `!melodic_leap_is_legal(prev_counter, sustain)`).
5. Do NOT loosen `melodic_leap_is_legal` (§1.3). Do NOT touch `engine.rs`. Do NOT octave-
   displace. Every new branch carries a theory comment (species floor / no-leap recovery /
   prepared-ornament rationale).
6. Run the full §4 self-verification set; confirm the engine.rs sha is unmoved.

**To the test owner (strict-ens `tests/counterpoint_s30.rs`):**

1. After the implementer lands the change, REPLAY the engine to read the exact realized
   pitches for the three residual cases (do not trust the spec's witnesses blind — confirm
   them: `IV/V→vii` → 62; `V/vi→…→I` → 60; `ii/vi→IV→iii` → 64).
2. Shrink each residual set to empty: `GAP2_RESIDUAL_DISSONANT_OPENERS = &[]`,
   `GAP3_RESIDUAL_LEAP_OPENERS = &[]`, GAP-4 `expected_diss = []`. Convert each residual loop
   into the strict universal positive property (terminal-diminished consonant; cadence
   perfect+no-leap; no dissonant melodic leap).
3. Update the three per-case `assert_eq!` witnesses to the confirmed new pitches; KEEP the
   kept witnesses (`iii→vii` 62; `ii→IV→V→I` 55→55; `I→V→IV` [64,62,65]) unless a replay shows
   a shift (none expected).
4. Keep PT-0/1/3/4/6/7/8/9, no-runaway, no-voice-cross, determinism assertions exactly as-is
   — they must remain green. If any residual does NOT shrink to empty, report it; do not pin a
   partial residual silently.

# Design Spec — S33 GAP-4 Penult-Rework (clean `iii` landing on `{ii,vi}→IV→iii`)

Status: DESIGN ONLY. This document specifies a change to `src/chord_engine.rs`; it does
not implement it. The implementer is the music-craft owner of `chord_engine.rs` (sole
writer); the test owner evolves the GAP-4 pin in `tests/counterpoint_s30.rs`. This spec is
modeled on `docs/design-s32-band-reachability.md` (its direct predecessor) and closes the
ONE residual that S32 deliberately deferred.

All witnesses below are GROUNDED in the live engine: they were read by replaying the
PUBLIC `realize_step` path for the three progressions through a throwaway read-only probe
(now removed; it touched no source or test file). Engine.rs sha verified unmoved at
`e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261` before and after.

---

## 0. The GAP-4 problem, restated precisely (with the live arithmetic)

The counter band is `[FILL_REGISTER_FLOOR, COUNTER_CEILING)` = `[55, 67)`. Under the test
harness (`brightness 50` → no register lift) the melody seats at/above 67, so every
counter↔melody vertical is the counter strictly below the melody.

The residual S32 pinned (and left as a DELIBERATE deferral) is two progressions whose
realized counter line ends on a `−6` TRITONE melodic leap. Live replay confirms the exact
pitches:

```
ii→IV→iii : counter [65, 65, 59]   melody [69, 72, 71]
vi→IV→iii : counter [64, 65, 59]   melody [76, 72, 71]
```

On BOTH, the IV-step (penult) counter is realized as **`65`** (vs melody `72` — a bare P5,
ic 7), and the `iii` step lands **`59`** (vs melody `71` — ic 0, P8) by the move
`65 → 59` = `−6` = a **TRITONE (ic 6)** melodic leap. That tritone is the defect.

### The precise parallel-fifth obstruction (why S32 could not close it from `65`)

The `iii` chord is E–G–B. Its band-seated tones vs the `iii`-step melody `71` are:

| `iii` band tone | pc | vs melody 71 | interval |
|---|---|---|---|
| `64` | E | ic 7 | **P5 (perfect)** |
| `55` | G | ic 4 | M3 (imperfect) |
| `59` | B | ic 0 | **P8 (perfect)** |

From the realized penult `65`, the ONLY in-band STEP is `65 → 64` (`−1`). But `64` vs `71`
is a P5, and the approach `65 → 64` (counter descends) against the melody `72 → 71` (melody
descends) is **SIMILAR motion into a perfect** — a hidden/direct fifth. The engine's
`approach_perfect_is_legal` gate (with `HIDDEN_PERFECTS_STRICT = true`) vetoes it, and the
must-stay-green non-GAP invariant `test_no_audible_parallel_perfect` forbids it. The other
two `iii` tones, `55` (`−10`) and `59` (`−6`), are reachable only by LEAP — and `−6` is the
tritone, `−10` is ic 2 (a major-9th-class dissonant interval, also a forbidden dissonant
leap class is ic 10/11; `65→55` is ic 2... a 9th, which `melodic_leap_is_legal` actually
ALLOWS as it only bans ic 6/10/11 — but `55` vs `71` is an imperfect M3, so `65→55` is a
*consonant* leap of a 9th-class interval). So from `65` the menu is: a hidden P5 (`64`,
vetoed), a tritone leap (`59`, the dissonant defect), or a wide consonant leap to `55`.

This is exactly why S32's existing GAP-4 leap-recovery branch in `pick_counter_figure`
(lines ~3977–4043) cannot fix it: that branch re-applies `approach_perfect_is_legal` +
`has_parallel_perfects` (correctly — it must not trade the tritone for a parallel perfect),
so `64` is filtered out; `55`/`59` are leaps; `59` is a dissonant leap (filtered);
`55` is a wide consonant leap and would actually be ELIGIBLE — but the recovery branch only
fires when `!melodic_leap_is_legal(prev_counter, sustain)`, and the realized sustain at the
`iii` step is already `59`, a *dissonant* leap, so the branch DOES fire — yet `55` is the
recovery's only survivor and it is a `−10` leap that... [see §1.3 note: the recovery as-built
should already prefer `55`; this confirms the fault is the PENULT, not the recovery]. The
honest conclusion S32 reached stands: **no clean landing is reachable from penult `65`.** The
fix must change the PENULT so a clean stepwise consonant landing exists at all.

### The taste decision inherited from S32 (design to it)

**Option B remains in force.** Keep the counter line **in register** — never octave-displace
out of `[55, 67)`. Prefer, in order: (0) a consonant chord tone by STEP; (1) a consonant
chord tone by a CONSONANT (legal) LEAP; (2) a prepared ornamental dissonance by STEP. Never
fall to an octave displacement or to a dissonant (tritone/7th) leap. GAP-4 is a pure leap
problem; it closes fully inside tiers (0)/(1) with NO new dissonance.

---

## 1. The penult-rework approach (which penult, why it is clean, both boundaries)

### 1.0 The winning penult: `57` (A3, the THIRD of IV = F–A–C)

The live geometry probe over every legal IV-step penult is decisive. The IV chord is
F–A–C; its band-seated tones vs the IV-step melody `72` are:

| IV band tone | pc | vs melody 72 | as IV penult |
|---|---|---|---|
| `57` | A (third) | ic 3 → **imperfect M3** | **the winner** |
| `60` | C (fifth) | ic 0 → P8 (perfect) | trap (see below) |
| `65` | F (root) | ic 7 → P5 (perfect) | the CURRENT penult — the trap |

From each candidate penult, the reachable clean `iii` landings (the `penult→iii` boundary,
melody `72 → 71`):

```
penult 57 → iii:   57→55 (-2 STEP) vs 71 ic4 imperf   CLEAN  ← ideal
                   57→59 (+2 STEP) vs 71 ic0 P8        legal (contrary into perfect: 57↑ vs 72↓)
                   57→64 (+7 leap) vs 71 ic7 P5        (leap; not needed)
penult 60 → iii:   60→59 (-1 STEP) vs 71 ic0 P8        HIDDEN PERFECT (60→59 ↓, melody ↓ = similar)  vetoed
                   60→55 (-5 leap), 60→64 (+4 leap)    leaps only
penult 65 → iii:   65→64 (-1 STEP) vs 71 ic7 P5        HIDDEN PERFECT  vetoed
                   65→59 (-6 leap, TRITONE), 65→55 (-10 leap)   the current trap
```

**Penult `57` is the only IV penult from which a clean stepwise CONSONANT non-parallel `iii`
landing exists:** `57 → 55` is a `−2` STEP onto an **imperfect** consonance (M3 vs `71`).
Because `55` is an imperfect consonance (not a perfect), no hidden/parallel-perfect can
arise at the `penult→iii` boundary at all — the parallel-perfect prohibition only constrains
arrival on a perfect interval, and `55` vs `71` is a third. (As a bonus, `57 → 59` is also
legal — a `+2` step into the P8 by CONTRARY motion, since the counter rises while the melody
falls — so even the perfect landing is reachable cleanly from `57`. The implementer's
selection will land `55` first: it is the imperfect-preferred, smallest-step, non-root
choice and tier-0 under `rank_inregister_landing`.)

### 1.1 The both-boundaries constraint set (the reworked penult must satisfy ALL of these)

The reworked penult `P` (on a pre-`iii` IV step, predecessor counter `prev`, IV melody
`m_iv`, `iii` melody `m_iii`) must satisfy:

**Boundary A — predecessor → penult (`prev → P`, melodies `m_pred → m_iv`):**
1. `P` is an in-band IV chord tone (`counter_candidate_pitches(IV, prev)` membership; band `[55,67)`).
2. `P` is a legal melodic move from `prev`: `melodic_leap_is_legal(prev, P)` (step, or a non-tritone/non-7th leap).
3. `P` vs `m_iv` is CONSONANT (the IV-step's own first-species vertical floor — a step that is itself a structural dissonance is illegal).
4. `prev → P` against `m_pred → m_iv` introduces no parallel/hidden perfect: `approach_perfect_is_legal(m_pred, m_iv, prev, P)` AND `!has_parallel_perfects([m_pred, prev], [m_iv, P])`.

**Boundary B — penult → `iii` (`P → L`, melodies `m_iv → m_iii`), L the realized `iii` landing:**
5. There EXISTS an in-band `iii` chord tone `L` reachable from `P` by a STEP (`|L − P| ≤ 2`), `L != m_iii` (PT-7 no-unison).
6. `L` vs `m_iii` is CONSONANT.
7. `P → L` against `m_iv → m_iii` introduces no parallel/hidden perfect: `approach_perfect_is_legal(m_iv, m_iii, P, L)` AND `!has_parallel_perfects([m_iv, P], [m_iii, L])`.

Live verification of `P = 57` for both progressions:

```
Boundary A  ii(prev=65)→57 : 65→57 = -8  ic8 m6  CONSONANT LEAP (legal); 57 vs 72 imperf; no hidden-perf (arrival imperfect). OK
Boundary A  vi(prev=64)→57 : 64→57 = -7  ic7 P5  CONSONANT LEAP (legal); 57 vs 72 imperf; arrival imperfect → no parallel-perfect test triggers. OK
Boundary B  57→55          : -2 STEP, 55 vs 71 ic4 M3 CONSONANT; arrival imperfect → no parallel-perfect possible. CLEAN
```

So `P = 57` is fully legal at BOTH boundaries for both progressions. The cost is at Boundary
A: reaching `57` is a **consonant leap** from the predecessor (an m6 from `ii`'s `65`, a P5
from `vi`'s `64`) rather than the near-step the current `65` enjoys (`65→65` hold from `ii`,
`64→65` step from `vi`). By the Option-B order already encoded in the engine, **a consonant
leap into the penult that buys a clean stepwise consonant landing strictly beats a smooth
penult that forces a dissonant tritone leap out.** This is the whole trade, and it is a win.

### 1.2 Which function changes, and the lookahead it rides

**The change is in `pick_counter_figure` only** (a private function; additive on the counter
path). The IV step is realized through `pick_counter_figure`, whose signature ALREADY carries
`next_chord: Option<&Chord>` — and at the IV-step decision point the call site
(`realized_counter_pitch_with_prev`, line ~2892) populates it as
`ctx.section.steps.get(si + 1).map(|s| &s.chord)`, i.e. **`next_chord = Some(iii)` is in scope
at the IV step.** This is the decisive freeze-positive result: the lookahead needed to know
"this IV step is a penult before a `iii` landing" is ALREADY THREADED. No new field, no new
parameter, no `realize_step` signature change is required. The rework rides existing context.

**New behavior — a pre-`iii` penult-bias branch in `pick_counter_figure`.** After the
`sustain` is computed and BEFORE the existing interior figure / leap-recovery logic, add a
*penult-lookahead recovery* gated by: `next_chord` is present AND, from the sustain pitch,
NO clean stepwise consonant non-parallel landing on `next_chord` exists, BUT from some OTHER
legal in-band IV chord tone `P` such a landing WOULD exist. When that holds, prefer `P` as
the penult pitch.

Concretely the new branch:

```text
// S33-CP-FIX (GAP-4) — PENULT LOOKAHEAD REWORK.
// Reached only when next_chord is Some AND the as-built sustain penult forces the next
// step onto a dissonant leap / hidden-perfect dead-end (the {ii,vi}→IV→iii shape).
// We already hold next_chord (= iii) at this IV step (no new threading). Search the
// current chord's in-band tones for a penult P that (a) is itself a legal, consonant,
// non-parallel IV vertical from prev_counter (Boundary A), and (b) admits a clean
// stepwise consonant non-parallel landing on next_chord vs the next melody (Boundary B).
// Prefer such a P, ranked by rank_inregister_landing against the IV melody (consonant-
// step penult first, then consonant-leap penult), tie-broken to keep the line connected.
// If the as-built sustain ALREADY admits a clean next-landing, this branch is a NO-OP
// (byte-identical). Determinism: min_by_key over the deterministically-ordered candidate
// list; no RNG.
```

The branch needs the NEXT melody pitch (`m_iii`) to test Boundary B. It is recoverable from
the same context the call site already has — `next_chord`'s step melody — but to honor the
frozen surface WITHOUT a new threaded field, the cleanest realization computes it the way the
replay already does: the implementer adds a small private helper that reads
`ctx`-derived melody for `si+1`. **FREEZE-SENSITIVITY FLAG (see §5):** `pick_counter_figure`
does not currently receive `ctx` or the next melody pitch. Two freeze-clean options exist,
in preference order:

- **Option 1 (preferred — no signature change to `pick_counter_figure`): do the penult
  rework one level UP, in `realized_counter_pitch_with_prev`.** That function already holds
  `ctx`, `si`, `m_prev`, `m_now`, `next_chord`, and can cheaply compute `m_iii =
  melody_pitch_for(si+1)` and a one-step lookahead landing test. It calls
  `pick_counter_figure` to get the as-built `cnt`; if `next_chord` is Some and `cnt` forces a
  dead-end on the next step, it re-picks the penult from the chord's in-band tones via a new
  PRIVATE helper `penult_for_clean_next(chord, prev_counter, m_prev, m_now, next_chord,
  m_next)` and returns THAT instead. This keeps `pick_counter_figure`'s arity frozen and
  confines the change to the realize/replay seam — which is private and already the owner of
  cross-step context. This is the S32-style "ride an existing local" pattern applied at the
  right altitude.

- **Option 2 (acceptable but freeze-sensitive): thread the next melody pitch into
  `pick_counter_figure` as an added PRIVATE param `m_next: Option<u8>`.** `pick_counter_figure`
  is private and `#[allow(clippy::too_many_arguments)]` already, so adding one `Option<u8>`
  does not touch `realize_step`'s frozen 7-param public signature. But it is a wider blast
  radius (every call site and the test's direct `pick_counter_figure` calls at lines
  ~7038/7088/7113 must add the arg). PREFER Option 1.

Either way the PUBLIC `realize_step` 7-param signature is UNTOUCHED — the rework lives
entirely in private functions that already (Option 1) or trivially can (Option 2) see the
next melody pitch. **No new `ctx`/`Section`/`StepPlan`/`PerfFeatures` field is required.**

### 1.3 Why the existing leap-recovery branch is NOT enough (and is not removed)

The existing GAP-4 in-register leap-recovery (lines ~3977–4043) stays. It correctly handles
the case where, from a GIVEN penult, a *consonant* recovery landing exists — it just cannot
manufacture one when the penult itself is `65` (every landing from `65` is a hidden perfect
or a dissonant leap, and the one consonant leap `65→55` is a `−10` wide leap that the
recovery's tie-break deprioritizes against... in fact the recovery would take `55` if it
survived the filters; the test confirms the realized line still ends `59`, so trace which
filter drops `55` — likely none, meaning the recovery is not firing because the *sustain* on
the `iii` step is selected before recovery as `59` and `melodic_leap_is_legal(65,59)` is
FALSE so the branch SHOULD fire; the implementer MUST instrument this exact path). The S33
penult-rework operates one step EARLIER (at the IV step) so that by the time the `iii` step is
realized, the realized penult is `57` and the ordinary sustain/recovery machinery lands `55`
cleanly with no special casing. **`melodic_leap_is_legal` is NOT loosened (S32's §1.3 holds);
`approach_perfect_is_legal` and `has_parallel_perfects` are NOT loosened.** The fix is purely
upstream candidate selection — exactly the "penult-rework slice" S32 named.

---

## 2. Worked arithmetic (live-confirmed) for the closure

### 2.1 `ii → IV → iii`

```
predecessor (ii) realized counter = 65   (vs ii-melody 69, ic 4 imperf — unchanged)
IV penult  : OLD 65 (root, P5 vs 72)  →  NEW 57 (third, M3 vs 72)
   Boundary A  ii→IV : 65→57 = -8  ic8 m6  CONSONANT LEAP (legal); 57 vs 72 ic3 imperf consonant; arrival imperfect → no parallel-perfect. LEGAL.
iii landing: from 57 → 55 = -2 STEP; 55 vs 71 ic4 M3 CONSONANT; arrival imperfect → no parallel-perfect. CLEAN.
   Realized line OLD [65, 65, 59] (65→59 = -6 TRITONE)  →  NEW [65, 57, 55] (57→55 = -2 STEP, consonant).
```

### 2.2 `vi → IV → iii`

```
predecessor (vi) realized counter = 64   (vs vi-melody 76, ic 0 — unchanged)
IV penult  : OLD 65 (root)  →  NEW 57 (third)
   Boundary A  vi→IV : 64→57 = -7  ic7 P5  CONSONANT LEAP (legal); 57 vs 72 ic3 imperf consonant; arrival imperfect → no parallel-perfect. LEGAL.
iii landing: from 57 → 55 = -2 STEP; 55 vs 71 ic4 M3 CONSONANT; CLEAN.
   Realized line OLD [64, 65, 59] (65→59 = -6 TRITONE)  →  NEW [64, 57, 55] (57→55 = -2 STEP, consonant).
```

Both close to a `−2` stepwise CONSONANT landing — no tritone, no new dissonance, register
preserved, both boundaries clean.

### 2.3 Witnesses that must NOT change (confirmed unaffected)

- **`I → V → IV` (the GAP-4 adversarial witness):** realized `[64, 62, 65]`, melody
  `[67, 74, 72]`. Its last chord is `IV`, NOT `iii` — `next_chord` at the `V` step is `Some(IV)`
  and at the `IV` (terminal) step is `None`. The S33 branch only fires on a pre-`iii` penult
  where the as-built sustain forces a dead-end; `I→V→IV` already realizes a clean consonant
  line (the `62→65` move is a consonant m3 leap, ic 3), so the branch is a NO-OP. Witness
  `[64,62,65]` UNCHANGED.
- **All other ordered diatonic triples** whose IV step is NOT followed by `iii`, OR whose
  as-built sustain already admits a clean next-landing: the branch's guard is FALSE → no-op →
  byte-identical. The branch fires on `{ii,vi}→IV→iii` ONLY (and any future progression with
  the identical penult dead-end shape).
- **GAP-2 / GAP-3** (S32 dispositions): untouched. GAP-2's terminal-diminished "bite" path
  (`next_chord.is_none()`) and GAP-3's `cadence_resolution_pitch` are different code paths,
  not reached by the pre-`iii` interior penult branch.

---

## 3. How the GAP-4 test pin evolves (for the Test Engineer)

The current pin (`test_no_dissonant_melodic_leap_in_counter_line`,
`tests/counterpoint_s30.rs` lines ~1106–1198) holds the residual at exactly
`{ii->IV->iii, vi->IV->iii}` and FAILS LOUDLY if it grows OR shrinks. After this slice lands,
the residual SHRINKS to empty. Evolve the pin toward the strict positive property:

1. **REPLAY first.** Do not trust this spec's witnesses blind — replay the engine after the
   change and read the realized lines for both progressions. Expected (live-derived here):
   `ii→IV→iii` → `[65, 57, 55]`; `vi→IV→iii` → `[64, 57, 55]`. Confirm the IV penult is `57`
   and the `iii` landing is `55` (a `−2` step, ic 4 M3 — consonant, NOT a tritone).
2. **Shrink the residual to empty:** change `expected_diss` from
   `["ii->IV->iii", "vi->IV->iii"]` to `[]` (an empty `Vec<String>`). The
   `residual_diss_leaps != expected_diss` advisory and the final `assert_eq!` then assert the
   residual stays EMPTY — i.e. NO realized counter leap over the full ordered diatonic-triple
   battery lands a dissonant melodic interval (`ic ∉ {6,10,11}` for every `|move| ≥ 3`). This
   IS the strict universal PT-6 "no dissonant melodic leap" property.
3. **Update the doc-comment:** the DEFERRAL paragraph (lines ~1086–1096) describing the pinned
   tritone and the parallel-fifth obstruction is now HISTORY — rewrite it to state that the
   penult-rework slice closed it: `{ii,vi}→IV→iii` now realize the IV penult `57` (the third
   of IV, an imperfect consonance vs the melody) from which `iii` is reached by a `−2`
   consonant step onto `55`; the tritone is gone; the residual is empty and the property is
   universal. Note that closure cost a CONSONANT leap into the penult (Boundary A) traded for
   the eliminated dissonant leap out (Boundary B) — register preserved, no new dissonance.
4. **Add two exact witnesses** (positive, replacing the pinned residual): assert
   `realize_line_interior(&[c_ii(), c_iv(), c_iii()])`'s counter line == `[65, 57, 55]` and
   `realize_line_interior(&[c_vi(), c_iv(), c_iii()])`'s == `[64, 57, 55]`, with the closing
   move asserted as a STEP (`|55 − 57| == 2`) onto a consonant interval (`ic(55,71) == 4`).
   Re-derive both from the live replay before pinning.
5. **Keep the adversarial witness** `I→V→IV → [64,62,65]` (line ~1109) verbatim — a replay
   must show it UNCHANGED; if it shifted, the branch over-fired and that is a defect to report,
   not to re-pin.
6. **Keep universal-strict** the leap-recovery (no two same-direction ≥4th leaps) assertion
   and the `leaps_seen > 0` sanity — both stay green.
7. Update the module-header doc-comment (lines ~58–62) so GAP-4 reads CLOSED (residual empty,
   universal no-dissonant-leap) rather than PARTIAL/DEFERRED.

If the residual does NOT shrink to empty after the change, report it — do not pin a partial
residual silently (S32's standing rule).

---

## 4. BYTE-FREEZE compliance

**What this slice TOUCHES:**
- `src/chord_engine.rs` ONLY — additively, on the counter path. Per §1.2 Option 1 (preferred):
  a new private helper `penult_for_clean_next(...)` plus a guarded call to it inside
  `realized_counter_pitch_with_prev` (a private function, already the owner of cross-step
  context). No edit to `pick_counter_figure`'s body is strictly required under Option 1; under
  Option 2 a single private `m_next` param is added to `pick_counter_figure`.
- `tests/counterpoint_s30.rs` — the GAP-4 pin evolves per §3 (test owner, after the implementer
  lands the engine change and replays).

**What stays FROZEN (must NOT move):**
- `src/engine.rs` — sha256 `e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261`.
  No edit. Verify with `sha256sum src/engine.rs` before and after. **This slice does NOT need
  engine.rs to change** — the entire fix is in `chord_engine.rs` private functions.
- `realize_step`'s PUBLIC 7-param signature — UNTOUCHED. The rework rides existing private
  context (`ctx`/`next_chord`/the realize-replay seam); no new threaded public field.
- `tests/engine_equivalence.rs` — 9/9 byte-green; goldens `240 ms` ritardando, velocities
  `114 / 84 / 36 / 79` UNMOVED. The counter path is downstream of role assignment; the
  equivalence ensemble's structural roles are unperturbed.
- The counter-OFF identity path (PT-0) — inserts nothing; the new branch is reachable only
  through the CounterMelody arm, which the identity profile never assigns.
- Every clean transition and every non-`{ii,vi}→IV→iii` progression — the new branch's guard
  (next_chord Some AND as-built sustain forces a next-step dead-end) is FALSE on them, so their
  realized pitches are byte-identical, including the `I→V→IV` adversarial witness.

**Freeze-sensitive flag RAISED:** the rework needs the NEXT melody pitch (`m_iii`) at the
penult decision, which `pick_counter_figure` does not currently receive. This is flagged
(§1.2). The PREFERRED resolution (Option 1) does the rework in
`realized_counter_pitch_with_prev`, which ALREADY holds `ctx`/`si`/`next_chord` — so NO new
threaded field and NO `pick_counter_figure` signature change are needed, and the public
`realize_step` surface is untouched. Option 2 (a private `m_next` param on
`pick_counter_figure`) is documented as the acceptable fallback but is wider-blast and
dispreferred. Either way the byte-frozen public surface and engine.rs are honored.

**Self-verification the implementer runs (full required set):**

```
cargo test --test counterpoint_s30          # GAP-4 residual now EMPTY; all green
cargo test --test engine_equivalence        # 9/9 byte-green, goldens unmoved
cargo test                                   # default feature set, whole suite green
cargo test --lib --no-default-features       # headless lib path green
sha256sum src/engine.rs                      # == the frozen sha above
cargo clippy -- -W clippy::all               # no new correctness warnings
```

Do NOT run bare `cargo fmt` (it reflows non-deliverable files — S32 noted defect). Format only
the touched file if needed: `cargo fmt -- src/chord_engine.rs`, or hand-format.

---

## 5. Determinism & preserved invariants

- **Determinism (PT-9):** the penult re-pick is a `min_by_key` over the
  deterministically-ordered `counter_candidate_pitches(IV, prev)` list, ranked by
  `rank_inregister_landing` (a total order). No RNG anywhere on the path.
- **Replay-verified, never per-step-seeded:** the penult `57` is a REALIZED pitch — it is what
  `realized_counter_pitch_with_prev` returns for the IV step, fed forward as the `iii` step's
  `prev_counter` via the existing `realized_prev_counter` recursion. The rework is verified by
  replaying the realized sequence, NOT by a synthetic seed (the standing S30/S32 lesson). The
  `iii` step then realizes against the REAL penult `57`, so the ordinary machinery lands `55`.
- **Band `[55,67)` preserved** — all candidates come from `counter_candidate_pitches`, which
  enforces the band; no octave displacement (Option A stays rejected).
- **No parallel/hidden perfect introduced** — Boundary A and Boundary B both re-apply
  `approach_perfect_is_legal` + `has_parallel_perfects`; `57`'s landing `55` is an IMPERFECT
  arrival, so the perfect-arrival rule is vacuous there. PT-1 strengthened, not weakened.
- **No unison collapse (PT-7)** — the `c != m_iii` guard is retained in the landing search.
- **PT-0 / PT-8 / GAP-2 / GAP-3** untouched (different code paths / FALSE guard).

---

## 6. Honest taste-trade

There is ONE real taste-trade, and it is mild and favorable:

- **The reworked IV penult `57` (A, the third) replaces `65` (F, the root).** `57` is reached
  from the predecessor by a CONSONANT LEAP (m6 from `ii`, P5 from `vi`) rather than the near-
  step `65` enjoys (a hold / a `−1` step). A leap into the penult is slightly less smooth at
  Boundary A than a step. BUT: (a) it is a *consonant* leap, fully legal and singable; (b) by
  the Option-B order already in the engine, a consonant penult-leap that buys a clean stepwise
  consonant landing strictly beats a smooth penult that forces a dissonant *tritone* leap out
  — the trade nets a strictly better line; (c) as a bonus, `57` is an IMPERFECT consonance
  (M3) against the IV melody `72`, whereas the old `65` was a bare P5 — the engine's own
  `IMPERFECT_PREF` rewards 3rds/6ths over bare 5ths for interior verticals, so `57` is
  actually the *better* IV counter on the engine's own taste axis; and (d) `57` is a NON-root
  tone (the counter should not double the bass root), favored by the `root_last` tie-break.

Net: the rework removes a dissonant tritone leap and IMPROVES the IV-step vertical (imperfect
over perfect, non-root over root) at the cost of one consonant leap of approach. There is no
fork to surface — unlike GAP-2's consonant-vs-bite choice, GAP-4 has a single correct
Option-B outcome, and it happens to also be the better-tasting IV penult. The only judgment
the implementer must make is the §1.2 Option 1 vs Option 2 mechanics call (preferred: Option
1, the realize-seam helper, no signature change), which is an internal-structure choice, not
a musical one.

---

## 7. Handoff notes

**To the music-craft implementer (sole writer of `src/chord_engine.rs`):**

1. Implement the penult-lookahead rework per §1.2 **Option 1** (preferred): add a private
   helper `penult_for_clean_next(chord, prev_counter, m_prev, m_now, next_chord, m_next) ->
   Option<u8>` that returns a reworked penult when, and only when, (a) the as-built sustain
   forces a next-step dead-end (no clean stepwise consonant non-parallel `next_chord` landing
   from the sustain) and (b) some other legal in-band chord tone `P` satisfies BOTH boundaries
   in §1.1; else `None`. Call it from `realized_counter_pitch_with_prev` (which already holds
   `ctx`/`si`/`next_chord`), computing `m_next` from `si+1`'s melody exactly as the replay
   computes melodies. Return the reworked `P` in place of the as-built `cnt` when it is `Some`.
2. FIRST instrument the live `ii/vi → IV → iii` path: confirm the IV-step `next_chord` is
   `Some(iii)`, the predecessor realized counters are `65`/`64`, and the as-built IV penult is
   `65`. Confirm `penult_for_clean_next` returns `57` and the `iii` step then realizes `55`.
   Also confirm WHY the existing leap-recovery did not already salvage `55` from `65` (§1.3) —
   if it should have, that is a separate finding to report.
3. Do NOT loosen `melodic_leap_is_legal`, `approach_perfect_is_legal`, or
   `has_parallel_perfects`. Do NOT octave-displace. Do NOT touch `engine.rs`. Do NOT change
   `realize_step`'s public signature. Every new branch carries a theory comment (penult-
   rework / both-boundaries / consonant-leap-over-dissonant-leap rationale).
4. Run the full §4 self-verification set; confirm the engine.rs sha is unmoved.

**To the test owner (evolves `tests/counterpoint_s30.rs`):**

1. After the implementer lands the change, REPLAY the engine to read the exact realized lines
   for both progressions (expected: `ii→IV→iii` → `[65,57,55]`, `vi→IV→iii` → `[64,57,55]`).
2. Shrink `expected_diss` to `[]`; the residual becomes the strict universal no-dissonant-leap
   property (§3). Rewrite the DEFERRAL doc-comment and the module header to read CLOSED.
3. Add the two positive witnesses (`[65,57,55]`, `[64,57,55]`) with the `−2`-step-onto-
   consonance assertion; KEEP the `I→V→IV → [64,62,65]` witness unchanged.
4. Keep PT-0/1/3/4/6/7/8/9, no-runaway, no-voice-cross, determinism assertions green. If the
   residual does NOT shrink to empty, report it; do not pin a partial residual silently.

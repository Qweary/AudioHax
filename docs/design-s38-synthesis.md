# S38 — Unified Design Synthesis (buildable)

**Author:** Rust Architect (synthesis half of S38; design-only — no `src/*` edited this session)
**Inputs reconciled:** `design-s38-affect.md` (Perceptual/Affect), `design-s38-aesthetics.md` (Composition Aesthetics), `design-s38-theory.md` (Music Theory), `design-s38-architecture.md` (my own prior diagnosis).
**Freeze anchor honored throughout:** `src/engine.rs` byte-frozen at sha256 `e50c7db1…48261`; `engine_equivalence` 9/9 must stay green. Every fix below lands in `composition.rs`, `chord_engine.rs`, or `assets/mappings.json`. `engine.rs` is never opened. Any path that would require breaking that is flagged **FREEZE-BREAK** and must reach the operator *before* a build.

This doc is the session's primary deliverable. S39 builds the Slice-1 specified in §3.

---

## 1. Unified diagnosis — three findings across four lenses

All four lenses converge on one root cause, stated four ways:

- **Theory:** "the piece is over-governed toward a single safe default; the image-derived knobs that should perturb it are frozen, collapsed, or pointed at the wrong axis."
- **Aesthetics:** "the piece doesn't commit to an identity" — constant key (#1), thin/undeveloped motif (#2), texture that refuses to be full *or* pointedly empty (#3).
- **Affect:** affect reaches mode, character, tempo, figuration — but **not register and not density**, the two cues that carry "this image's feeling."
- **Architecture:** the derivations are all plan-time and already plumbed; the blockers are a hardcoded constant (#1), a collapsed selector + a pinned `dur_steps` (#2), and a byte-freeze hinge that pins density to 0.5 on the common image (#3).

### Per-finding reconciliation

#### Finding #1 — every image starts on C4

| Lens | Position |
|---|---|
| Architecture | Smallest, safest, most-contained fix; one line at `composition.rs:1302`; zero test cost. Recommends as Slice-1. |
| Theory | "Smallest real fix and genuinely safe." Confirms re-seating by *pitch class* (`seat_pc_in_register`, role floors 36/55/67) means moving the tonic rotates pitch classes without moving absolute register. **Safe home window: MIDI [57,68]**; do NOT exceed 68 without re-deriving headroom margins. |
| Aesthetics | Real *identity* lever (per-image "home of its own"), but **not** the highest — the ear reads key slowly/unconsciously. Adds **GR-1 (home invariance within a piece)** and **GR-2 (vary pitch class, hold register band)**. |
| Affect | Splits the finding: **register (octave) is the high-leverage half** (valence→register is textbook, instantly audible); **absolute key center is NOT an affect cue** for non-AP listeners. Recommends moving *register*, leaving pitch-class anchor where it is. |

**AGREEMENT (auto-apply candidates):** the locus is `composition.rs:1302`; the fix is plan-time, freeze-safe, single-file; the home must be **one value per piece** (GR-1) and the **register band must stay narrow** (Theory's [57,68] ≈ Aesthetics' GR-2 55–66 band ≈ Affect's "snap to octave, keep tessitura sane").

**CONFLICT (operator-surface):** *what drives the home, and is it pitch-class or register?*
- Aesthetics/Theory: **dominant_hue → pitch class**, seated in a narrow register band (a color-wheel→chromatic-wheel identity; reuses the hue signal already read at `composition.rs:1294`).
- Affect: **valence → register (octave)**, pitch class left at C; register is the affect-faithful cue, hue-as-pitch is "colorist garnish, not affect."

These are not mutually exclusive — hue→pitch-class (identity) and valence→register (affect) are orthogonal and *compose*. But they answer "what should #1 change" differently, and Affect explicitly warns against re-introducing hue as a third decider on the *affect* axis (the major/minor caveat). **This is a real divergence the operator must arbitrate** (§5, DP-2).

#### Finding #2 — too few distinct motifs

| Lens | Position |
|---|---|
| Aesthetics | **Highest-leverage finding from its lens.** The motif is the only element the ear consciously remembers and *reports* (operator counted shapes). Three stacked causes: vocabulary half-disabled (4/8), weak image→motif spreading (contour re-scaled but never re-identified), no cross-section development. Rank order: **(1) rhythmic identity, (2) full vocabulary, (3) subject→melody anchoring, (4) cross-section development.** |
| Theory | **Highest-leverage finding from its lens, and within it the RHYTHMIC PROFILE is the single cheapest high-impact lever.** Names three independent collapses: selection collapse (`edge_activity≥0.6 → Ascent` short-circuit swallows photos), rhythm flatness (`dur_steps:1` at `chord_engine.rs:2415`), sampling smear (static repeated tail). Owns contour voice-leading; all 8 contours legal off any root. |
| Affect | Real but gated on Music Theory's contour work; delivers *variety-across-images* not a per-image quality fix. Asserts selection should be **affect-quadrant-keyed** (2×2 arousal×valence), hue as within-quadrant tiebreak — mirroring the working mode-family pattern. |
| Architecture | Selection-spread is `composition.rs`-only, freeze-safe, ~15 LOC (S). All 8 contours already exist in `chord_engine.rs` and `resolve_motif` already handles them. **Watch-item:** giving the motif real `dur_steps` requires threading duration into `theme_melody_pitch` (today the realizer ignores motif duration) — a `chord_engine.rs` realize-side read, NOT an engine.rs seam. |

**AGREEMENT (auto-apply candidates — strong, three lenses independently):**
- **The motif's RHYTHMIC PROFILE is the highest-impact single lever.** Aesthetics Rank 1 == Theory §3.3 == the kickoff hypothesis. Both taste lenses name it independently. It adds *content*, not just texture.
- **Widen `pick_archetype` from 4→8 and kill the `edge_activity≥0.6 → Ascent` short-circuit.** Aesthetics Rank 2 == Theory §3.2 == Architecture's "highest-yield single change."
- **Selection must be driven by ≥2 decorrelated axes**, not hue alone (so similar-hue images can still diverge).

**CONFLICT (operator-surface):** *what is the second selection axis?*
- Affect: **arousal×valence quadrant** (affect-faithful, mirrors mode-family).
- Aesthetics: **hue family × `vertical_emphasis`/`mass_centroid.y`** (spatial-height↔pitch-height correspondence; also feeds Rank 3 subject-anchoring).
- Theory: routes via **hue + affect** but defers the exact cuts to Affect/Aesthetics; owns only that the targets are legal.

All three agree it must be ≥2 axes and decorrelated from hue; they differ on *which* second axis. Affect's spatial-height note and Aesthetics' `vertical_emphasis` actually *agree* with each other (both invoke height↔pitch); the live split is **affect-quadrant vs. spatial-height** as the primary spreader. Surface to operator (§5, DP-3).

#### Finding #3 — density feels reversed / too sparse

| Lens | Position |
|---|---|
| Affect | **Highest-leverage from its lens** (operator's named *felt* problem). Diagnosis: **arousal→density edge does not exist**; density pinned to 0.5 on home/identity sections; rest gate too eager. Fix is **un-wired + floored-low, not inverted.** Proposes raising `DENSITY_NEUTRAL 0.5→0.62`, `DENSITY_FLOOR 0.35→0.45`, adding `DENSITY_AROUSAL_SPAN=0.20`. Flags the byte-stability cost itself. |
| Aesthetics | Macro-pacing half: "fuller by default, silence as a *device*." Density band ±0.075 audible swing is imperceptible. Proposes role-bias fullness curve (Statement/Return high, Contrast peak, one pre-return thin spot, Coda taper) + raise gain so the swing is felt. Keeps identity path pinned at 0.5. |
| Theory | Craft half: three compounding causes — articulation busy-end too detached (`ARTIC_WINDOW_LO=0.55`), default bed has zero inner motion + `figured_bed` onset-capped, conservative bass. Proposes `ARTIC_WINDOW_LO 0.55→0.62`, activity-scaled figuration onset *prefix* (≤4, safe), optional default-bed breathing. **`FILL_REST_ACTIVITY` is well-tuned — do NOT loosen it** (directly contradicts Affect's Move 3). |
| Architecture | **The central tension.** The freeze hinge is `Section.density==0.5` on identity sections. **SAFE knobs** (`DENSITY_ENERGY_SPAN`, `GAIN`, `FLOOR`/`CEIL` *while bracketing 0.5*) by construction leave the `e=0.5` home case unchanged — so they **cannot** fix a home-only image's sparseness, because that image *is* the frozen `e=0.5` case. **DANGER knobs** (`DENSITY_NEUTRAL`, `HOME_ENERGY_NEUTRAL`, raising `FLOOR` above 0.5) break the freeze. `FILL_REST_ACTIVITY` is read against raw `edge_activity`, ungated by the identity hinge — a *downward* move is likely safe (goldens use `edge_density:0.5 ≫ 0.10`), an upward move is not; must verify against fixtures. |

**AGREEMENT (auto-apply candidates):**
- The density *audible swing* is too small (`±0.075`) — both Affect and Aesthetics want it felt; Theory's `ARTIC_WINDOW_LO 0.55→0.62` and activity-scaled figuration-onset-prefix are **freeze-safe craft levers that increase felt fullness without touching the identity hinge** (Architecture confirms onset-prefix stays ≤4 by construction).
- "Silence as a deliberate device, fuller by default" is the shared target.

**CONFLICT (operator-surface — two of them):**
1. **The satisfying fix is a FREEZE-BREAK.** Affect's `DENSITY_NEUTRAL 0.5→0.62` and `FLOOR 0.35→0.45` (above 0.5? no — 0.45<0.5, that one is borderline-safe; but the NEUTRAL move is the break) **break the `engine_equivalence` goldens** (Architecture DANGER-knob analysis confirms). Affect names two outs: (i) re-baseline the goldens, (ii) keep NEUTRAL=0.5 + a post-identity `DENSITY_FILL_BIAS=0.12`. Architecture independently identifies this as the central tension and the only operator-sign-off item among the three findings.
2. **`FILL_REST_ACTIVITY`: Affect wants 0.10→0.05; Theory says do NOT touch it** (it fixed the old "harmony vanishes" bug). Architecture: a downward move is *likely* freeze-safe but must be verified against fixtures. **Direct specialist conflict — surface, do not auto-pick** (§5, DP-4).

---

## 2. Slice-1 recommendation — defended against the divergence

### The divergence, stated honestly

- **#1 (key/register)** is the cheapest, safest, most-contained, freeze-trivially-safe, zero-test-cost slice. My prior note and the kickoff both pre-recommended it.
- **#2-rhythmic-profile** is independently judged the **highest-leverage finding by BOTH taste lenses** (Aesthetics and Theory), and Theory names the rhythmic profile *specifically* as the single cheapest high-impact lever. The operator's own report is the evidence: they *counted motif shapes (~2)*. The motif is the identity the ear consciously remembers and reports back.
- **#3 (density)** is the operator's named *felt* problem, but its satisfying fix is a **FREEZE-BREAK** requiring sign-off — disqualifying it as a clean Slice-1.

### The decision criterion (stated explicitly)

> **Slice-1 should be the change that best closes the gap between what the operator consciously reported and what the system produces, subject to being freeze-safe and buildable in one slice.**

This criterion deliberately weights **"what the operator actually reported"** above **"cheapest/smallest blast radius."** Cheapness is a tiebreaker, not the objective — a maximally cheap slice that doesn't move the needle the operator pointed at is a worse slice than a slightly larger one that does. #3 fails the freeze-safe clause (its satisfying form). Between #1 and #2, #2 wins the criterion: the operator *counted shapes*, not keys; two of three other lenses rank #2 highest; and Theory isolates a sub-lever (rhythmic profile) that is freeze-safe and S-sized.

### Where I change my earlier recommendation

My architecture note recommended **#1-first on contained-ness grounds.** I am **changing that recommendation to #2-first (rhythmic profile + vocabulary spread)**, because the taste-led case is stronger under the stated criterion and the cost delta is small:

- Contained-ness was my whole #1 case. But #2's rhythmic-profile slice is *also* S-sized and freeze-safe (Architecture §Finding 2: plan-time selection + a `chord_engine.rs` realize-side duration read, no engine.rs touch, no golden moves on the theme-less identity fixtures). The contained-ness gap between #1 and #2-Slice-1 is **small**, not decisive.
- The leverage gap is **large and points the other way**: the operator reported motif sameness directly; #1's payoff (Affect: "absolute key is not an affective cue"; Aesthetics: "the ear reads key slowly/unconsciously") is real but second-order.
- #2 also carries Aesthetics' Rank 3 (subject→melody) as the most direct fix for the standing "image-unrelated/ethereal" complaint — the deeper problem behind this whole arc.

### RECOMMENDED SLICE-1

> **Slice-1 = Finding #2, minimal form: (a) give the motif a rhythmic profile (kill `dur_steps:1`) + thread duration into the realizer, and (b) widen `pick_archetype` to all 8 contours and drop the `edge_activity≥0.6 → Ascent` short-circuit.**

This is the smallest slice that delivers the highest-leverage, most-reported, freeze-safe win. It is two tightly-coupled sub-levers (rhythm gives each contour an identity; vocabulary gives more contours to differentiate) — building (b) without (a) leaves distinct contours arriving as identical even-note streams (Theory §3.1), so they ship together.

### Operator choice I AM surfacing

Because the kickoff and my own prior note both pre-recommended #1, and #1 is genuinely the lower-risk slice, the operator should get a crisp either/or rather than have me silently override the pre-recommendation:

- **Option A — #1-first (key/register).** Lowest risk, smallest blast radius, freeze-trivial, ~30 LOC + 1 JSON block. Delivers a second-order-but-real identity gain. Frames #2 nicely (a developed tune *in a key of its own* — Aesthetics §4). **Choose if the operator prioritizes a guaranteed-safe confidence-building first slice.**
- **Option B — #2-first (motif rhythm + vocabulary). ← MY SINGLE RECOMMENDATION.** Slightly larger (adds the realizer duration-thread, ~S/borderline-M), still freeze-safe, directly closes the gap the operator reported (counted shapes). **Choose if the operator prioritizes hitting the loudest reported defect first.**

I recommend **Option B**. If the operator prefers the safe-first posture, Option A is a clean fallback and the §4 sequence simply swaps Slice-1↔Slice-2.

---

## 3. RECOMMENDED Slice-1 BUILD SPEC (Finding #2: motif rhythm + vocabulary)

Concrete enough for an Implementer in S39.

### 3a. Sub-lever (a) — motif rhythmic profile + realizer duration thread

**Files & functions:**
- `src/chord_engine.rs` — `resolve_motif` (`:2379`): replace the hardcoded `dur_steps: 1` (`:2415`) with a per-archetype durational profile cycled across the sampled notes.
- `src/chord_engine.rs` — `theme_melody_pitch` / theme realize branch (`:2472–2549`): **thread `dur_steps` into how long the melody holds each theme note.** Today the realizer substitutes only PITCH and is rhythm-agnostic (Theory §3.3(c), Architecture Finding-2 watch-item). This is the one piece of follow-on plumbing #2 requires and it MUST be scoped here.
- `assets/mappings.json` — optional `composition.motif_rhythm` block (durational templates as data) so profiles are tunable by ear without recompile. **Single-writer; Music Theory integrates** (the durational profiles are *theory data* — Music Theory owns them; Affect tunes only which archetype an image gets, not a given archetype's profile).

**Per-archetype rhythmic profiles (Music Theory's §3.3 table — theory-owned, load-bearing):**

| Archetype | `dur_steps` profile | Rationale |
|---|---|---|
| `Arch` | `[2,1,1,2]` | balanced sigh |
| `Descent` | `[1,1,1,1,2]` | even fall into a longer arrival |
| `Ascent` | `[1,1,2]` cycled | a lift that breathes |
| `NeighborTurn` | `[1,1,1,1,2]` | quick turn, held resolution |
| `LeapStep` | `[2,1,1,1,1]` | the leap LANDS long, gap-fill quick — sounds *announced* |
| `Pendulum` | `[2,2]` | even, weighty oscillation |
| `RisingSequence` | `[1,1,2]` per cell | rhythm articulates the 3-note sequence cell |
| `InvertedArch` | `[2,1,1,2]` | mirror of Arch |

**Constraints (Theory-owned, MUST hold):**
- `dur_steps >= 1` always (the `MotifNote` contract, `:2317`).
- **The sum of a section's motif `dur_steps` must not exceed `length_steps`** — emit notes until the durational sum reaches `length_steps` rather than emitting exactly `length_steps` notes. **This also fixes the §3.1(3) static-tail smear** (a longer final note replaces the repeated held tail).

**Image-feature → music data flow:** image → `ImageUnderstanding` (already populated by `understand_image_pure`) → `plan()` → `resolve_motif(archetype, range, length_steps, …)` at plan-build time (`composition.rs:1335`) → emits `Vec<MotifNote{degree, dur_steps}>` onto `ThemeSeed.motif` → consumed by the FROZEN realize path reading plan *data* via `theme_melody_pitch`. No new feature threading; `complexity`/`edge_activity` (which Aesthetics/Affect would use to *pick among* rhythm cells, if cells are image-selected rather than archetype-fixed) are already on `u` and in scope. **Decision: archetype-fixed profiles (Theory's table) vs. image-selected cells (Affect/Aesthetics' `motif_rhythm.select` rules).** Recommend **archetype-fixed for Slice-1** (simplest, each contour gets a stable rhythmic identity); image-selected cells are a Slice-2 garnish.

### 3b. Sub-lever (b) — widen `pick_archetype` to all 8 + drop the Ascent short-circuit

**Files & functions:**
- `src/composition.rs` — `pick_archetype` (`:1500–1512`): remove the `if edge_activity >= 0.6 { return Ascent }` short-circuit (`:1503`); replace the 4-way hue-quadrant ladder with a selector that can reach all 8 `MotifArchetype` variants. All 8 contours already exist in `chord_engine.rs` (`:2320–2363`) and `resolve_motif` already handles them — **no `chord_engine.rs` contour work needed for vocabulary.**
- `assets/mappings.json` — optional `composition.theme_archetype` SelectTable (the `form`/`character`/`texture`/`prominence` SelectTable pattern at `:1267–1391`) mapping `(axis1, axis2) → archetype id`. If data-driven, add a `parse_archetype(&str)` mapper (`MotifArchetype` is a closed Rust enum). **Minimal slice = broaden the Rust ladder inline; data-driven SelectTable is the cleaner M-sized variant.** Recommend inline-Rust for Slice-1 unless the operator wants the SelectTable now.

**Selection axes — SURFACE TO OPERATOR (DP-3), do not auto-pick.** The three lenses agree on ≥2 decorrelated axes but split on the second:
- Affect: `arousal × valence` quadrant (hue as within-quadrant tiebreak).
- Aesthetics: hue family × `vertical_emphasis`/`mass_centroid.y`.
- Theory: hue + affect (defers cuts to Affect/Aesthetics; owns legality).

The seed I'd build absent operator input (it satisfies all three constraints — ≥2 axes, decorrelated from pure hue, affect-reinforcing): **arousal×valence picks the contour FAMILY (Affect's 2×2), `vertical_emphasis` breaks within-family up/down (Aesthetics' height↔pitch), hue is the final tiebreak.** This composes Affect's and Aesthetics' second-axis proposals rather than choosing between them. But it is a taste call — surface it.

**Voice-leading constraints any selection MUST respect (Theory-owned):** largest native interval is the 5th (degree 0→4) in `LeapStep`/`Pendulum`/`InvertedArch`; leaps stepwise-recovered except `Pendulum` (both poles are chord tones); every contour begins on a stable degree (all 8 do).

### 3c. Proposed `mappings.json` rows (single-writer — Music Theory integrates)

`assets/mappings.json` is single-writer, shared between Music Theory and Affect/Aesthetics. **Music Theory holds the pen** for the contour-adjacent rows; Affect/Aesthetics supply selection-cut numbers. Proposed (from Theory §7 + Aesthetics §7):

```jsonc
"composition": {
  // (a) motif rhythm — THEORY-OWNED durational templates (load-bearing)
  "motif_rhythm": {
    "Arch":            [2,1,1,2],
    "Descent":         [1,1,1,1,2],
    "Ascent":          [1,1,2],
    "NeighborTurn":    [1,1,1,1,2],
    "LeapStep":        [2,1,1,1,1],
    "Pendulum":        [2,2],
    "RisingSequence":  [1,1,2],
    "InvertedArch":    [2,1,1,2]
  },
  // (b) archetype selection — AFFECT/AESTHETICS-OWNED cuts, THEORY-validated targets
  "theme_archetype": { /* SelectTable: (arousal×valence family, vertical_emphasis within-family, hue tiebreak) -> archetype id */
    "_axes": ["affect_quadrant", "vertical_emphasis", "dominant_hue"],
    "_ids":  ["arch","inverted_arch","descent","ascent","neighbor_turn","leap_step","pendulum","rising_sequence"]
  }
}
```
**Coordination note:** the `motif_rhythm` profiles are theory data (single-writer Music Theory); the `theme_archetype` cuts are Affect/Aesthetics' to fill, Theory validates legality. Build the inline-Rust ladder first if the SelectTable + parser is judged too large for Slice-1.

### 3d. Test net to preserve + new test

| Test | Status under Slice-1 | Why |
|---|---|---|
| `engine_equivalence` (9/9) | **unchanged green** | Hand-built fixed plan; identity/home fixtures carry `theme: None` — no motif path exercised. The freeze hinge is untouched. |
| `runtime_reachability_s37` | unchanged green | Asserts spine (tempo/mode/length), "never asserts per-step harmony realization." |
| `composition_s15`, `keyplan_*` | unchanged green | Structural/key invariants, motif-agnostic. |
| `diversity_s13` | unchanged green | `60`-arg `generate_chords` direct unit calls, motif-agnostic. |
| `pattern_library_s34`, `figuration_s20`, `prominence_s23` | re-run, expect green | Orchestration/figuration/bass catalogues — orthogonal to motif archetype selection, but re-run to confirm no realize-path interaction from the `theme_melody_pitch` duration thread. |

**NEW test (`tests/motif_s38.rs`):**
- **Rhythm identity (GR-3 proxy):** two fixtures differing in archetype produce `motif` `Vec`s whose `dur_steps` sequences differ in ≥1 position.
- **Vocabulary spread (GR-4 proxy):** across ≥12 varied image fixtures, ≥6 distinct archetypes observed, no single contour >~30% share (log archetype per fixture).
- **Static-tail fix:** a long-`length_steps` fixture's motif does NOT end in ≥2 identical held degrees (proves the durational-sum emit cap replaced the smear).
- **Duration-thread:** a theme-bearing section's realized melody note lengths reflect the motif `dur_steps` (not all equal).

### 3e. Freeze-safety verdict

**SAFE — no engine.rs edit, no golden move.** (1) `resolve_motif`/`pick_archetype` run at plan-build time (`:1335`), explicitly "build time only, never tick time" (`chord_engine.rs:2367`). (2) The `theme_melody_pitch` duration thread is on the realize path *called from* the frozen kernel, but the `engine_equivalence` fixtures carry no theme on the identity/home sections they pin, so the goldens are insensitive (Architecture Finding-2). (3) All 8 contours already exist and `resolve_motif` already handles them — no new degree ranges that could perturb the realizer's pitch map. **Watch-item carried forward:** if a *future* slice adds a NEW (9th+) contour whose degree span exceeds the realizer's pitch-map assumptions, validate `theme_melody_pitch` first (still no engine.rs edit). Not in scope for Slice-1.

### 3f. Size estimate

**S → borderline-M.** Sub-lever (b) inline-Rust ladder ~15 LOC. Sub-lever (a) profile table + `resolve_motif` durational-sum emit ~15–25 LOC + the `theme_melody_pitch` duration thread (the M-pushing piece) ~10–20 LOC + new test ~60 LOC. One build slice if the realizer thread is clean; split (a)/(b) across two if the duration thread surprises.

### 3g. Acceptance / listening gate

**Primary (Aesthetics GR-7, the differentiation test):** take any two *clearly different* images; generate both; play the two **opening themes** back-to-back to a listener who hasn't seen the images. They must say "two different tunes" on rhythm OR contour OR register grounds — not "two takes of the same ambient idea." Re-run with two *similar* images: themes may rhyme but must not be byte-identical unless the images are.
**Component (Theory §3.3 ear test):** clap two themes back-to-back — they must not have the same gait (rhythm now carries identity).
**The operator's ear is the final arbiter.** All automated proxies in §3d are conveniences, not the gate.

---

## 4. Sequenced approach for the other two findings

Ordered after Slice-1 (= #2 rhythm+vocabulary). Re-listen gate after each slice.

### Slice-2 — Finding #1 (per-image home), freeze-safe
**Shape:** replace `composition.rs:1302` `home_root_midi = 60` with an image-derived lookup (keep `60` as defensive fallback so a mappings.json without the block reproduces today byte-for-byte). Reword the stale "home_root_midi seed = 60" docstrings in `keyplan_k2a.rs:289/304` (consistency sweep, no assertion changes).
**Driver — operator decision required (DP-2):** hue→pitch-class (Aesthetics/Theory, identity) vs. valence→register (Affect, affect). These compose; the operator picks one for Slice-2 or builds both. If both: pitch-class seated in band [57,68] (Theory's safe window) + valence→octave snapped within REGISTER_FLOOR/CEIL = ±12.
**Constraints:** GR-1 (home constant within a piece), GR-2 (narrow register band; do NOT exceed MIDI 68 without re-deriving headroom — Theory §2.2). Theory confirms NO pivot/cadence rework needed (the home is a single fixed center; `key_scheme` excursions are a separate already-guarded axis).
**Freeze:** SAFE. No test asserts planner `home_root_midi` (Architecture test-net inventory: zero hits). `engine_equivalence` hand-built and immune. **No freeze-break, no sign-off.**
**Dependency:** none on Slice-1; freely reorderable (this is the Option-A fallback's Slice-1).
**Size:** S.

### Slice-3 — Finding #3 (density), SPLIT into freeze-safe now + freeze-break later

**Slice-3a (freeze-safe, build without sign-off) — craft-side felt fullness:**
- `ARTIC_WINDOW_LO 0.55→0.62` (`chord_engine.rs:1411`) — closes per-note silence on busy images (Theory §4.2; SAFE const, curve shape unchanged, cadence ring untouched).
- Activity-scaled figuration onset **prefix** (≤4 by construction, Theory §4.2) — busy→more inner onsets, stays under the bounded-burst safety proof (Architecture confirms).
- Optional default-bed single mid-step re-articulation for `density > neutral` sections, respecting `PAD_OVERLAP_FRAC`, byte-identical on the `density==0.5` identity path.
- **Freeze:** SAFE (all preserve `f(0.5)==0.5` and the `density==0.5` identity nudge). `figuration_s20`/`pattern_library_s34` must re-run.

**Slice-3b (FREEZE-BREAK — operator sign-off REQUIRED before build) — the satisfying "fuller by default":**
- The arousal→density edge + raised baseline that *actually* fixes the common home-only image: `DENSITY_NEUTRAL 0.5→0.62`, add `DENSITY_AROUSAL_SPAN=0.20` (Affect Move 1+2), role-bias pacing curve (Aesthetics D-1/D-2/D-3).
- **This breaks the `engine_equivalence` goldens** — the home-only image *is* the frozen `e=0.5` case; the safe knobs are inert on it by construction (Architecture's central tension). Two operator-choice outs:
  - **(i)** re-baseline/re-bless the `engine_equivalence` goldens (honest — they froze the *old* sparser aesthetic we're deliberately changing).
  - **(ii)** keep `DENSITY_NEUTRAL=0.5` + a post-identity `DENSITY_FILL_BIAS≈0.12` applied after the identity check (surgical, preserves the proof, adds a concept).
- **Also unresolved in Slice-3b:** `FILL_REST_ACTIVITY` direct conflict (Affect 0.10→0.05 vs. Theory "do not touch"). Architecture: downward move *likely* safe (goldens use `edge_density:0.5 ≫ 0.10`) but VERIFY against fixtures before touching.
- **Dependency:** Slice-3a is independent and should ship first regardless. Slice-3b is gated on the operator's freeze-break decision (DP-1).
- **Size:** 3a = S; 3b = M (plus golden re-baseline churn under out (i)).

---

## 5. Decision points for the operator

- **DP-1 (FREEZE-BREAK — highest priority, blocks Slice-3b):** the *satisfying* density fix ("fuller by default," `DENSITY_NEUTRAL 0.5→0.62` + arousal edge) **breaks the `engine_equivalence` goldens** because the common home-only image is exactly the frozen `e=0.5` case. Pick: **(i)** re-baseline the goldens (Affect leans here; honest), or **(ii)** keep NEUTRAL=0.5 + post-identity `DENSITY_FILL_BIAS≈0.12` (surgical). The freeze-safe craft levers (Slice-3a) ship regardless and need no sign-off.
- **DP-2 (Slice-2 #1 driver):** **hue→pitch-class** (Aesthetics/Theory — per-image *identity*) vs. **valence→register/octave** (Affect — *affect*-faithful, instantly audible). They compose; choose one for Slice-2 or build both. Affect warns: do not re-introduce hue as a third decider on the *affect* axis.
- **DP-3 (Slice-1 selection second axis):** the three lenses agree on ≥2 decorrelated axes but split the second: **arousal×valence quadrant** (Affect) vs. **`vertical_emphasis`/spatial-height** (Aesthetics). My build seed *composes* them (affect-quadrant → family, vertical_emphasis → within-family, hue → tiebreak); confirm or override.
- **DP-4 (`FILL_REST_ACTIVITY` — direct specialist conflict):** Affect wants 0.10→0.05 ("silence as a device, not a calm-image default"); Theory says **do NOT loosen it** (it fixed the "harmony vanishes" bug). Architecture: downward move likely freeze-safe but verify against fixtures. Operator arbitrates (deferrable to Slice-3b).
- **DP-5 (Slice-1 vs. Slice-2 order):** my single recommendation is **#2-first (Option B)** per §2's criterion; the kickoff and my prior note pre-recommended **#1-first (Option A, safe-first)**. If the operator prefers the guaranteed-safe confidence-builder, swap Slice-1↔Slice-2 — both are freeze-safe and independently buildable.
- **DP-6 (Slice-1 data vs. inline):** archetype-fixed rhythm profiles + inline-Rust selection ladder (minimal, recommended) vs. data-driven `motif_rhythm` + `theme_archetype` SelectTables + `parse_archetype` mapper (cleaner, M-sized). Recommend inline for Slice-1; promote to data later.

---

## 6. Freeze ledger

| Finding | Slice | Freeze-safe? | What a future engine.rs-touching version would need |
|---|---|---|---|
| #1 home root/register | Slice-2 | **YES** — plan-time `composition.rs:1302` only; no test asserts planner `home_root_midi`; `engine_equivalence` hand-built & immune; home window [57,68] preserves headroom margins | Only if root selection moved into the tick kernel (it should not — root is a plan-time spine decision). Would break engine.rs sha → operator sign-off. |
| #2 motif rhythm + vocabulary | **Slice-1** | **YES** — `resolve_motif`/`pick_archetype` are build-time; the `theme_melody_pitch` duration thread is on the realize path but identity/home goldens carry `theme: None` so are insensitive; all 8 contours already exist | A NEW (9th+) contour whose degree span exceeds the realizer pitch-map → validate `theme_melody_pitch` first (still no engine.rs edit). |
| #3 density — craft (artic/figuration prefix/bed breathing) | Slice-3a | **YES** — all preserve `f(0.5)==0.5` and the `density==0.5` identity nudge; figuration prefix stays ≤4 | n/a (stays off the frozen kernel). |
| #3 density — satisfying (NEUTRAL 0.5→0.62 + arousal edge + role bias) | Slice-3b | **NO — FREEZE-BREAK** | Re-baseline the `engine_equivalence` goldens (out i) OR add post-identity `DENSITY_FILL_BIAS` (out ii). `FILL_REST_ACTIVITY` change must be fixture-verified. **Operator sign-off (DP-1) before ANY build.** |

---

*End of S38 synthesis. S39 builds Slice-1 (§3) once the operator answers DP-5 (Slice order) and DP-3/DP-6 (Slice-1 internals); DP-1 gates Slice-3b only and does not block Slice-1.*

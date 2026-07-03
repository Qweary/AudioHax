# S64 Diagnostic — The Fixed Cross-Piece Rhythmic Skeleton & the Shared-Key Hearing

**Session 64, STEP 1 — READ-ONLY diagnostic.** No production code modified. `engine.rs` frozen
(untouched). One diagnostic test added: `tests/keydiag_s64.rs` (structural/integer asserts only).
All claims cite `file:line` and are grounded in the actual corpus (the 6 test images), measured with
the S63 gate-liveness scorecard and the new S64 key/form/phrase probe.

---

## 0. The operator's ear-test seed (verbatim)

> The S63 integration is a REAL improvement — layers cohere better. BUT an overarching "Wizard of
> Oz" cross-piece SAMENESS remains: every output feels like a variation of ONE archetypal piece.
> THE KEY FINDING — a FIXED RHYTHMIC SKELETON in ONE layer of EVERY output, transcribed:
> `dotted-quarter · eighth · quarter-note-triplet · half-note` (repeat the 4-event sequence), then
> `dotted-quarter · eighth` (repeat those last two events ~3 more times). "Every output has those
> rhythms; maybe minus one or two notes and/or slightly varied, but the skeleton is still present."
> Does this same signature ALSO recur at OTHER layers? And the KEYS sound shared too.

**Verdict up front.** The operator's ear is exactly right, and the skeleton is **not the motto**
(S53/S62/S63 — that varies per piece). It is the **phrase/cadence scaffold**: a hard-coded 4-step
phrase *period* whose cadence layout `[Start · Interior · Interior · Cadence]` is stamped onto every
section of every image, then rhythmically realized by two *unconditional, non-image-driven* gestures —
a **pre-cadence harmonic-rhythm acceleration** and a **cadence ring**. It is **fixed by design, not by
a dead gate, and — critically — it is NOT form-gated.** This corrects the STEP-2 working hypothesis
(see §4).

---

## 1. Fixed-rhythm source — located

### 1a. The scaffold: `plan_phrases` collapses every piece to a 4-step period

`PHRASE_LENGTHS` is a two-element constant `[4, 8]` (`chord_engine.rs:579`). `plan_phrases`
(`chord_engine.rs:692`) chooses a phrase length by *"prefer the LARGEST allowed length that divides
the sequence evenly AND still yields at least TWO phrases"* (`chord_engine.rs:712-724`):

```
phrase_len = largest of [4,8] with len<=n, n % len == 0, n/len >= 2   (else fallback, else 4)
```

Section length is `steps_per_section = BASE_STEPS_PER_SECTION(=8) + round(edge_activity*8)`
(`composition.rs:1387,1514`), so sections run ~8–14 steps. Against that range the rule **almost
always resolves to 4**: 8 is rejected (a single 8-step section yields only 1 phrase, failing
`n/len>=2`), and 9–14 are not multiples of 8, so `phrase_len = 4` (or the length-4 fallback). The
**4-step period is therefore universal**, and every longer section decomposes into repeated 4-step
periods.

`plan_phrases` then stamps a fixed structural role + velocity per position (`chord_engine.rs:736-810`):

- `V_START = 88`, `V_INTERIOR = 76`, `V_CADENCE = 96` — a fixed accent contour per phrase.
- position 0 → `PhraseStart`; interior → `Interior`; the phrase-final step → `HalfCadence` (non-final
  phrase, respelled to **V**) or `PerfectAuthenticCadence` (final phrase, respelled to **V–I**).

**Empirical confirmation (S64 probe, `tests/keydiag_s64.rs`).** The per-section phrase layout is
**nearly identical across all six images**. The recurring 8-step section (the "B" and "A′" of
rounded_binary, and example's "V2") is *always*:

```
[ S . . h  S . . P ]   =  antecedent (half cadence) + consequent (PAC)  — two 4-step periods
```

| image | form | home root | section layouts |
|---|---|---|---|
| AudioHaxImg1 | rounded_binary | C#4 | `[S.h]` `[Sh]` `[S..h]` |
| AudioHaxImg2 | rounded_binary | C#4 | `[S.P]` `[S..hS..P]` `[S..hS..P]` |
| AudioHaxImg3 | rounded_binary | C4  | `[S.P]` `[S..hS..P]` `[S..hS..P]` |
| example      | **theme_and_variations** | F#4 | `[S..hSP]` `[S..P]` `[S..hS..P]` |
| Lena         | rounded_binary | B3  | `[S.P]` `[S..hS..P]` `[S..hS..P]` |
| magicstudio  | rounded_binary | A3  | `[S.P]` `[S..hS..P]` `[S..hS..P]` |

The `S..hS..P` period repeats section after section, image after image. **This is the operator's
transcribed skeleton.**

### 1b. The rhythmic realization: two unconditional gestures stamp the transcription

Inside `realize_rhythm` (`chord_engine.rs:2070`), two phrase-boundary behaviors fire **with no image
feature in the gate** — they are structural constants:

1. **Pre-cadence acceleration** (`chord_engine.rs:2122-2125`): `pre_cadence = !is_cadence &&
   !is_phrase_start && position_in_phrase + 2 >= phrase_len`. For a 4-step period this is **always
   the 3rd step**. In the Melody arm it forces an **ARPEGGIO with `n = 4` even onsets**
   (`chord_engine.rs:2728-2741`) — a subdivided run the ear reads as the operator's **"quarter-note-
   triplet."** In the Bass arm it forces a **long root + pickup at 3/4** (`chord_engine.rs:2346-2354`).
2. **Cadence ring** (`chord_engine.rs:2252`): every phrase-final step returns a **single sustained,
   ritardando-lengthened note** (`RITARDANDO_FACTOR`, `chord_engine.rs:2228`) — the operator's
   **"half-note."** This early-returns **before the per-role match**, so it is **identical for every
   role** (Melody, Bass, Pad, Fill, Counter).

The interior steps (positions 0–1) are band-selected; the real-photo cluster's most common band is
**DOTTED** — a **long-short pair, holds 2/3 + 1/3** (`chord_engine.rs:2752-2764`) — which is exactly
the operator's **"dotted-quarter · eighth."** So one 4-step period realizes as:

```
pos0 DOTTED (dotted-Q · eighth) → pos1 DOTTED (…) → pos2 pre-cadence accel (triplet run) → pos3 cadence ring (half)
```

= **`dotted-quarter · eighth · quarter-triplet · half-note`**, the operator's 4-event group. The
**"dotted-quarter · eighth ×3" tail** is a passage of consecutive DOTTED interior steps before the
next cadence (e.g. the `S..h` antecedent of a `S..hS..P` period) — repeated long-short pairs. The
mapping is exact.

Note the DOTTED band itself is *not* the fixed part (bands do vary per image — see §2). The **fixed
part is the scaffold**: no matter which band the interiors take, **every 4-step period ends with the
same accel→ring gesture.** That gesture is the cross-piece constant.

### 1c. It is NOT a mappings.json template and NOT the motto

- Not a `mappings.json` rhythm array. `composition.motif_rhythm` is a documented **inert data
  mirror** (its own `_note`: *"resolve_motif_celled does NOT read this block … QG confirms inert"*).
  The authoritative rhythm cells are the per-archetype motto cells inline in `chord_engine.rs` — and
  those **vary per piece** (branch coverage 4/6 distinct cells; §2). The motto only *displaces WHERE
  onsets sit, count-preserving* (`apply_motto_onset_bias`, `chord_engine.rs:2871`) — it cannot create
  the scaffold.
- The scaffold lives in **code, unconditionally**: `PHRASE_LENGTHS` const, the `plan_phrases`
  divisibility rule, `pre_cadence`, and the cadence ring. None reads an image feature.

---

## 2. Cross-layer recurrence — the operator's explicit question, answered

**Yes — the same cross-piece-fixed signature recurs at multiple layers.** Enumerated, with the
fixed-by-design vs. fixed-by-dead-gate distinction:

| # | Layer | Cross-piece status | Source | fixed-by |
|---|---|---|---|---|
| 1 | **Phrase/cadence scaffold** (4-step period, `S..hS..P`) | **FIXED** — universal | `PHRASE_LENGTHS` `:579`; `plan_phrases` `:712-810` | **design** (const + rule) |
| 2 | **Cadence ring** (single sustained ritardando note at every boundary) | **FIXED** — *all roles* | `realize_rhythm:2252` | **design** |
| 3 | **Pre-cadence acceleration** (extra onsets on the 3rd step) | **FIXED** — Melody run + Bass pickup | `:2122-2125`, `:2728-2741`, `:2346-2354` | **design** |
| 4 | **Structural velocity contour** `[88,76,76,96]` per phrase | **FIXED** | `plan_phrases:736-808` | **design** |
| 5 | **Harmonic rhythm** (one chord/step; V/I cadence respell every 4th step) | **FIXED** rate | `plan_phrases:760-788` | **design** |
| 6 | **Bass gait** — `walking` on 4/6 images (even-quarter walk) | **mostly fixed** | `realize_rhythm:2302-2326`; texture handle | **dead gate** (texture selector) |
| 7 | **Realizer interior band** (dotted/arp/sync/sustained) | **VARIES** (4/6 distinct) | band ladder `:2728-2783` | — (this is the *live* part) |
| 8 | **Rhythm motto cell** (S53 gait) | **VARIES** (4/6 distinct) | `pick_rhythm_cell` | — (the S62/S63 win) |

So layers **1–5 are the same skeleton the operator isolated, present in every role** (the cadence
ring and harmonic rhythm are role-universal by construction; the pre-cadence accel hits Melody and
Bass). Layer 6 (walking bass on 4/6) is a *second* fixed rhythm — a shared bass gait — but it rides
the near-collapsed texture selector (a dead-gate, fixable by opening that gate). Layers 7–8 are the
parts that **already vary** (the S50 band re-range and the S53/S62/S63 motto). The operator was
hearing 1–6; the "cohere better" improvement they credited is 7–8 doing their job *on top of* an
unchanged 1–5 chassis.

---

## 3. Key story — confirmed with actual per-image roots

**The `home_root` block IS wired and IS NOT collapsed to the C4 default.** (`resolve_home_root_midi`,
`composition.rs:1075-1090`; the block is present in `assets/mappings.json` with a full 12-bucket
uniform `hue_to_pc` map + band `[57,68]`.) The S64 probe resolves, per image:

| image | dominant_hue | pitch class | home_root_midi | note |
|---|---|---|---|---|
| AudioHaxImg1 | 39.9 | 1 | 61 | **C#4** |
| AudioHaxImg2 | 36.6 | 1 | 61 | **C#4** |
| AudioHaxImg3 | 5.2  | 0 | 60 | **C4** (genuine pc 0, *not* the fallback) |
| example      | 198.6 | 6 | 66 | **F#4** |
| Lena         | 354.1 | 11 | 59 | **B3** |
| magicstudio  | 278.3 | 9 | 57 | **A3** |

- **All-C4-default? NO.** 5 distinct pitch classes {0,1,6,9,11}; 5 distinct roots. The `home_root`
  hue→pc mapping is doing real work.
- **So why does the operator still hear shared keys?** Two reinforcing reasons the ear is right about:
  1. **Register clustering.** Seating into the safe band `[57,68]` (`seat_pc_in_band`,
     `composition.rs:1045`) puts the warm-hue real-photo cluster into the **bottom** of the band:
     A3(57), B3(59), C4(60), C#4(61), C#4(61) — **five of six roots inside a 4-semitone span
     A3–C#4**, only example's F#4 outside. Different *pitch class*, near-identical *absolute tonal
     center*. Perceptually that reads as "same key-ish register."
  2. **No intra-piece modulation.** `key_scheme` fires `home_only` on 5/6 images (S63 scorecard:
     branch coverage 2 distinct, `home_only`×5). In the probe, **every section key-offset is 0**
     except AudioHaxImg2's B section (`key_off=7`). So for 5/6 pieces the key **never moves** — the
     home root is also the only root. Static tonal center within *and* across pieces.
  - Mode adds only partial relief (Dorian, Lydian, Mixolydian, Dorian, Ionian, Mixolydian — 4
    distinct, but Dorian×2 and Mixolydian×2), so two pieces near C#4 in similar modes sound tonally
    kindred.

**Key verdict:** the home root is *not* the C4 bug the hypothesis assumed. The residual "shared key"
hearing is **register-band clustering + a dead `key_scheme` (no modulation)** — a real but **secondary**
lever, and a *different fix* than "un-default from C."

---

## 4. Ranked verdict — the single deepest lever, with the hypothesis corrected

Ranked by **audibility** (what most makes it "the same piece"):

1. **Phrase/cadence scaffold (layers 1–5)** — *the operator's transcribed skeleton itself.* Highest
   audibility by definition: it's what a trained ear transcribed unprompted.
2. **Key register clustering + no modulation** — 5/6 roots in A3–C#4, 5/6 pieces never modulate. The
   "shared key" hearing.
3. **Form collapse** — 5/6 `rounded_binary` (form gate mostly dead: `ternary_aba` DEAD on
   `QuadrantContrast>=0.6` vs obs max 0.253; `abac` DEAD on `VerticalEmphasis>=0.6` vs obs max 0.383;
   S63 scorecard). Macro-structure sameness.
4. **Bass walking dominance** — a shared bass gait (4/6), rides the near-collapsed texture selector.

### The single lever, and the correction

**CORRECTION to the STEP-2 working hypothesis.** The hypothesis named **FORM-SELECTOR SPREAD** and
**KEY VARIETY** as the two deepest levers. The evidence overturns both as the *primary* target:

- **FORM SPREAD will NOT break the transcribed skeleton — proven.** `example.jpg` is the one image
  that escapes to a different form (`theme_and_variations`), and its sections **still realize
  `S..hS..P`** (probe, §1a). Form changes *section count / labels / repeats / key excursions*; it
  does **not** touch the within-phrase 4-step scaffold, because every section — whatever the form —
  is re-partitioned into 4-step periods by `plan_phrases`. Form spread is worth doing for **macro
  variety**, but it is the *least* effective lever against the specific skeleton the operator
  transcribed.
- **KEY VARIETY is largely ALREADY DONE.** The home root is wired and gives 5/6 distinct pitch
  classes (§3). The premise "drive the home root off an image feature … not default-C" is already
  satisfied. The remaining key win is **register-spread + turning on modulation** (open the dead
  `key_scheme` gate), which is a *secondary* lever, not the deepest.

**The single deepest lever is #1 — the PHRASE-PERIOD / CADENCE-SCHEME scaffold**, which no amount of
form or key spread reaches. It is the *only* lever that changes what the operator transcribed. Pull
it by breaking the universal 4-step period and the one-size cadence gesture:

**Recommended STEP-2 target (BALANCE-LAW compliant — open curated routes, never randomize):**

> **PHRASE-PERIOD SPREAD.** Replace the hard `PHRASE_LENGTHS = [4,8]` + "largest ≥2-phrase" collapse
> (`chord_engine.rs:579,712-724`) with a **selection over a curated phrase-length / cadence-layout
> menu** — authored common-practice options (e.g. the *sentence* 2+2+4, the asymmetric 3- and 5-bar
> phrase, the 6-step group) — **selected by an image feature** (complexity / edge_activity / aspect,
> the same knobs the form gate reads), deterministically. That varies the *cadence periodicity* the
> ear locks onto, so the `S..hS..P` recurrence breaks across pieces.
>
> **Secondary, same session or next:** author a small **cadence-gesture catalogue** so the
> unconditional pre-cadence accel (`:2728`) and cadence ring (`:2252`) are *selected* (feminine
> ending / suspension / elided cadence / the current accel-run) rather than stamped identically —
> feature- or character-seeded.
>
> **Then (macro variety, distinct from the skeleton):** open the dead **form** gate over the existing
> `form_catalogue` (re-scale the DEAD thresholds `QuadrantContrast>=0.6`, `VerticalEmphasis>=0.6`
> into the observed corpus ranges) and the dead **key_scheme** gate (turn on modulation for >1/6
> images), plus **re-seat the home-root register** so warm-hue photos don't all land in A3–C#4.

**Freeze discipline flag for STEP 2.** `plan_phrases` output feeds the `engine_equivalence` goldens
(the identity/legacy path). A phrase-period change must be **identity-path-neutral**: select the
phrase-length/cadence-scheme id in `composition.rs::plan` from the image feature against a curated
catalogue and thread it in as a *per-plan parameter that defaults to the legacy `[4,8]` behavior*
(the exact discipline used for `motto` / `prominence` / `bass_pattern` — a `#[serde(skip)]`/resolved
handle whose absence is byte-identical). `engine.rs` stays frozen; the change lives in `chord_engine`
+ `composition` craft layers with the legacy default preserved.

---

## Appendix — how to reproduce

```
export PATH="$HOME/.cargo/bin:$PATH"
cargo test --release --test gate_liveness_s63 -- --nocapture   # dead-gate scorecard (form/key/texture/bass)
cargo test --release --test keydiag_s64      -- --nocapture   # per-image home root / form / phrase layout
```

`tests/keydiag_s64.rs` asserts only structural/integer invariants (home root ∈ {60}∪[57,68];
phrase_len ∈ 1..=8 — the nominal period is 4/8, a section's short final phrase carries the
remainder 2/3; corpus fully processed) — no per-image float audio metrics (macOS-libm-safe).

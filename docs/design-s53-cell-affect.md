# Design — S53 / fix-direction-2 SLICE 1: Perceptual / Cross-Modal Affect pass on the per-piece rhythm-cell driver

**Status:** DESIGN ONLY (taste/affect review voice, beside correctness, in the S53 build cadence — Specialist Marshaling Gate). No source modified, no `mappings.json` written. This document sizes the seam spec's **DP-1** (the taste/affect call) and recommends exact consts for the Music Theory Producer to implement and ear-tune.
**Companion spec:** `docs/design-s53-cell-seam.md` (the Rust Architect's slice-1 seam — the carrier, the `pick_piece_cell` signature, the `RhythmMotto` value object, the freeze discipline). This doc owns only the **DRIVER** inside `pick_piece_cell` — which features select WHICH gait — not the carrier.
**Author role:** Perceptual / Cross-Modal Affect Specialist. I own the bridge from image affect/cross-modal correspondence → musical character. I do NOT own voice leading / harmony (Music Theory Specialist) and I do NOT compute pixels (image-extraction). I consume already-extracted features.
**Field names verified against:** `src/composition.rs:40–117` (`ImageUnderstanding`), `:337–366` (`affect_composite`), `:1395–1402` (the affect seat), `:1779–1826` (the reverted consts + `pick_rhythm_cell`); `src/chord_engine.rs:1071–1104` (`band_activity_spread` + its consts), `:1136` (`melody_activity_class`), `:1061–1063` (the MELODY band cutoffs), `:3241–3333` (the `rhythm_cells` vocabulary). Every value below is computed from the canonical six-probe feature table re-verified at three independent sources (`docs/design-s50-affect-cutpoints.md` table, `docs/design-s50-aesthetics-cutpoints.md` mirror, `docs/diag-s49-recurring-rhythm.md:61–69`).

> **No `docs/research-affect-crossmodal.md` exists in the tree.** My grounding brief is therefore the standing project literature already cited in the S50 affect-cutpoints doc (Hevner 1937 adjective circle; Eerola, Friberg & Bresin 2013; Juslin & Laukka 2003) plus the S50/S49 empirical probe measurements. I carry confidence levels on every claim and flag where the pure-Rust feature set is sufficient vs. where it is not.

---

## 0. The canonical six-probe feature table (the acceptance set)

These are the measured, pixel-derived, whole-image features for the six bundled probes. All three source docs agree; `affect_arousal` is the planner composite `0.45·(avg_saturation/100) + 0.25·colorfulness + 0.20·edge_activity + 0.10·complexity` (`composition.rs:337–366`, `mappings.json` affect.arousal_weights). These images **cluster low-complexity and compress the edge band into ≈0.30–0.51 for four of six** — this compression is the whole reason the cell axis went dormant.

| image | `edge_activity` | `complexity` | `colorfulness` | `avg_saturation` | `affect_arousal` |
|---|---|---|---|---|---|
| AudioHaxImg1 | 0.301 | 0.005 | 0.011 | 32.9 | 0.211 |
| AudioHaxImg2 | 0.509 | 0.015 | 0.080 | 30.1 | 0.259 |
| AudioHaxImg3 | 0.475 | 0.229 | 0.423 | 37.2 | 0.391 |
| example | 0.719 | 0.905 | 0.685 | 64.5 | 0.696 |
| Lena | 0.471 | 0.164 | 0.122 | 51.9 | 0.375 |
| magicstudio-art | 0.106 | 1.000 | 0.287 | 43.6 | 0.389 |

The `band_activity_spread` re-expansion (CENTER=0.40, GAIN_LOW=1.8, GAIN_HIGH=1.4, `chord_engine.rs:1071–1073/1097`) maps each `edge_activity` to a spread value:

| image | `edge_activity` | `band_activity_spread(edge)` | spread-band cell on the BROAD=0.33 / BUSY=0.66 ramp |
|---|---|---|---|
| magicstudio-art | 0.106 | **0.000** | < BROAD → cell **1** |
| AudioHaxImg1 | 0.301 | **0.222** | < BROAD → cell **1** |
| Lena | 0.471 | **0.499** | [BROAD, BUSY) → cell **0** |
| AudioHaxImg3 | 0.475 | **0.505** | [BROAD, BUSY) → cell **0** |
| AudioHaxImg2 | 0.509 | **0.553** | [BROAD, BUSY) → cell **0** |
| example | 0.719 | **0.847** | ≥ BUSY → cell **2** |

**The load-bearing observation up front:** the PRIMARY axis alone (spread-edge against the existing cuts) puts **three of the six probes — Lena, Img3, Img2 — on cell 0** and **two — magicstudio, Img1 — on cell 1**. Only example reaches cell 2; no probe reaches cell 3. The PRIMARY axis, unaided, produces just **three distinct cells {1, 0, 2} across six images** with a 3-way and a 2-way collapse. **Everything the SECONDARY axis must do is break those two collapses.** The DP-1 decision is therefore not cosmetic — it is the difference between the un-gated axis delivering real separation and re-creating the dormancy in disguise (seam spec R-4).

---

## 1. ANSWER 1 — is `affect_arousal` the right SECONDARY "rhythmic personality" axis?

### 1.1 The cross-modal chain, stated precisely

The defensible perceptual chain is: **a busier / higher-arousal image → a denser, more syncopated onset gait.** Arousal ↑ → tempo and articulation density ↑ is the single most robust affect→music finding in the literature (Juslin & Laukka 2003 meta-analysis; Eerola/Friberg/Bresin 2013 — tempo and articulation are the two strongest arousal cues). On that theory, arousal as the "rhythmic personality" axis is **prima facie correct**, and the seam spec's instinct to reach for it over raw `complexity` (the dormancy culprit) is sound *as a principle*.

**But the principle does not survive contact with the acceptance set, and the reason is structural, not aesthetic.** The verdict below is grounded, not stylistic.

### 1.2 Why `affect_arousal` FAILS as the cell-3 divert key on this set — the structural collision

`affect_arousal` and `band_activity_spread(edge_activity)` are **not independent axes** — they are strongly co-monotone on this set, because `edge_activity` is itself a 0.20-weighted term inside the arousal composite, and the dominant 0.45·saturation + 0.25·colorfulness terms happen to co-vary with edge for these particular photos. The consequence is decisive:

| image | spread-edge primary cell | `affect_arousal` | crosses ANY divert cut ≥ 0.40? |
|---|---|---|---|
| magicstudio | 1 | 0.389 | no |
| Img1 | 1 | 0.211 | no |
| Lena | 0 | 0.375 | no |
| Img3 | 0 | 0.391 | no |
| Img2 | 0 | 0.259 | no |
| example | 2 | **0.696** | **yes — the ONLY one** |

**`affect_arousal` clears a usable divert threshold for exactly one probe — `example` — and `example` is already the unique cell-2 image.** So the arousal divert:

1. **Adds zero separation** to the two collapsed clusters (Lena/Img3/Img2 on cell 0; magicstudio/Img1 on cell 1). It cannot, because no member of either collapsed cluster has arousal anywhere near a cut that doesn't also pull in the whole mid-band. The mid-band arousals (0.375, 0.391, 0.389) sit within 0.016 of each other — there is no cut that splits them.
2. **Is actively destructive at low cuts.** Any cut low enough to touch the mid-cluster (≈0.37–0.39) pulls magicstudio, Lena, AND Img3 all onto cell 3 simultaneously — a NEW 3-way collapse (verified: cut=0.35 → {magicstudio, Lena, Img3, example} all on cell 3). The decorrelating intent inverts into a re-collapse.
3. **Cannibalizes example's distinct landing.** At the only cuts that fire cleanly (≥0.40, catching example alone), the divert *moves example off its distinct cell-2 onto cell 3* — trading one distinct landing for another, net separation unchanged.

**Confidence: HIGH** that arousal is the wrong secondary *for this acceptance set*. This is an arithmetic fact about six measured feature vectors, not a taste judgment — I re-ran it across cuts 0.35/0.40/0.50/0.60 and every one either collapses the mid-cluster or touches only example.

### 1.3 What the SECONDARY axis actually has to discriminate — and which live feature does it

The job is to **split the cell-0 triple (Lena 0.471 / Img3 0.475 / Img2 0.509) and ideally the cell-1 pair**, using a feature that is (a) live (varies across the cluster), (b) perceptually defensible as a "which rhythmic gait" key, and (c) NOT one of the slice-1-pinned defaults (seam spec §2.3 exclusion list).

Within the cell-0 triple the candidate discriminators order as:

| feature | Img2 | Lena | Img3 | usable split inside the triple? |
|---|---|---|---|---|
| `affect_arousal` | 0.259 | 0.375 | 0.391 | **no** — Lena/Img3 within 0.016 (the load-bearing Img3/Lena pair is a TIE here) |
| `complexity` | 0.015 | 0.164 | 0.229 | **yes** — monotone, clean gaps (0.015 / 0.164 / 0.229) |
| `colorfulness` | 0.080 | 0.122 | 0.423 | **yes** — Img3 is a strong outlier (0.423); Img2/Lena close |
| `avg_saturation` | 30.1 | 51.9 | 37.2 | non-monotone vs. arousal; weak |

**The honest finding:** the ONLY live feature that cleanly splits the carried **Img3 ~ Lena watch-pair** at the cell level is `complexity` (0.229 vs 0.164) or `colorfulness` (0.423 vs 0.122). Arousal cannot — Img3 and Lena are an arousal tie. This is the crux: the seam spec proposed arousal *specifically to avoid* `complexity` (the S50 dormancy culprit), but on the acceptance set **`complexity` is the feature that carries the texture/intricacy → syncopation signal**, and it only failed in S50 because it was trapped behind the `complexity ≥ 0.4` theme gate. The S53 un-gating removes that trap. With the gate gone, `complexity` is no longer dormant — it is the correct divert key, exactly as the S50 author intended before the gating defeated it.

### 1.4 RECOMMENDATION (Answer 1)

**Do NOT use `affect_arousal` as the cell-3 divert key. Use `complexity` as the cell-3 divert key, un-gated, guarded so it cannot steal the busy/example cell-2 landing.** This is the S50 secondary axis (`CELL_COMPLEXITY_PROFILED`) — which was always perceptually correct (texture/intricacy → profiled/syncopated gait is the right cross-modal mapping) — finally made reachable by the un-gating.

Perceptual justification: the syncopated/profiled character cell (cell 3 — see `chord_engine.rs:3247/3253/3262/3281/3290/3302/3309` cell-3 doc lines: dotted launches, Lombard snaps, suspension-release, lopsided swing) is the gait of a *visually intricate/textured* surface, not merely a *saturated/energetic* one. Intricacy is a TEXTURE percept (spatial detail density); arousal is an ENERGY percept (chroma/motion). They are different cross-modal channels. Syncopation — onsets displaced off the grid — is the rhythmic analogue of spatial *irregularity/detail*, which `complexity` (shape_complexity / Laplacian texture) measures and `arousal` (saturation-dominant) does not. **Confidence: HIGH** that complexity/texture is the correct percept for the syncopation divert; **MEDIUM-HIGH** on the specific claim that this is preferable to a colorfulness-based key (colorfulness also works numerically and is a legitimate "chromatic energy" cross-modal channel — see §1.5).

> **Note for the Music Theory Producer — this CONTRADICTS the seam spec's DP-1 proposal.** The seam spec (`design-s53-cell-seam.md` §2.3) proposes `affect_arousal` as the secondary and introduces a new const `PIECE_AROUSAL_PROFILED`. My grounded finding is that on the acceptance set this collapses separation. The seam spec's *carrier, signature, and freeze discipline are all correct and unchanged* — but the `pick_piece_cell` signature should take **`complexity`** (or `colorfulness`) as its second scalar, not `affect_arousal`, and the new const should be `PIECE_COMPLEXITY_PROFILED`, not `PIECE_AROUSAL_PROFILED`. See §3 for the exact const recommendation. The seam author flagged this as a taste call to confirm; this is the confirmation: **arousal is the wrong key, and here is why, with the six-probe arithmetic.**

### 1.5 The one honest hedge — colorfulness as an alternative, and where pure-Rust runs out

`colorfulness` (0.011 / 0.080 / 0.122 / 0.287 / 0.423 / 0.685 across the set) is a fully live feature and a legitimate cross-modal "chromatic energy/vividness → rhythmic profile" channel (vividness reads as expressive emphasis). It splits Img3/Lena even more strongly than complexity (0.423 vs 0.122 — a 0.30 gap vs complexity's 0.065). **Either complexity OR colorfulness is a defensible secondary; both beat arousal decisively.** I recommend `complexity` as primary choice because (a) it is the S50 axis already named in the consts and tests, minimizing churn, and (b) "intricacy → syncopation" is a tighter perceptual claim than "vividness → syncopation." But I flag colorfulness as the **fallback the Producer should ear-test** if the complexity-driven cell-3 gait sounds wrong on Img3.

**Where pure-Rust is sufficient vs. not (honest boundary):** both `complexity` and `colorfulness` are pure pixel stats already computed — no new extraction, no OpenCV change, the whole driver is pure-Rust. **Pure-Rust IS sufficient for this slice.** Where it runs out is the *higher-order* percept the literature would actually want: rhythmic syncopation correlates best with perceived *visual rhythm/repetition-irregularity* (a Gestalt grouping percept), which neither `complexity` (a global shape/texture scalar) nor `colorfulness` captures — that would need a spatial-frequency or autocorrelation feature the extractor does not produce. **Confidence: HIGH** that this gap exists; **the honest position is that complexity is the best available proxy, not the ideal feature, and the cell-3 divert is a coarse 1-bit decision, not a graded mapping.** I do not recommend adding extraction this slice — the 1-bit divert is enough to break the collapse.

---

## 2. ANSWER 2 — the six-probe separation table (the load-bearing check)

### 2.1 Under the seam spec's PROPOSED driver (spread-edge PRIMARY + `affect_arousal` SECONDARY) — COLLAPSE

This is the driver as written in `design-s53-cell-seam.md` §2.3, evaluated on the acceptance set at the best-case arousal cut (any cut ≥ 0.40, which catches example alone):

| image | spread-edge | primary cell | `affect_arousal` | diverts to 3? | **final cell** |
|---|---|---|---|---|---|
| magicstudio | 0.000 | 1 | 0.389 | no | **1** |
| Img1 | 0.222 | 1 | 0.211 | no | **1** |
| Lena | 0.499 | 0 | 0.375 | no | **0** |
| Img3 | 0.505 | 0 | 0.391 | no | **0** |
| Img2 | 0.553 | 0 | 0.259 | no | **0** |
| example | 0.847 | 2 | 0.696 | **yes** | **3** |

**Cell distribution: {1: [magicstudio, Img1], 0: [Lena, Img3, Img2], 3: [example]} → only THREE distinct cells across six probes.**

**TWO COLLAPSES FLAGGED:**
- **CELL-0 TRIPLE COLLAPSE (severe):** Lena, Img3, Img2 all on cell 0. This is the load-bearing failure. It re-pins three of the six to one cell — the seam spec's own R-4 "dormancy in disguise" realized. In particular it **fails to split the carried Img3 ~ Lena watch-pair** (Answer 4b), which was the residual the cell axis was supposed to resolve.
- **CELL-1 PAIR COLLAPSE:** magicstudio + Img1 on cell 1. This **loses magicstudio's distinctive cell-3 gait** (Answer 4a) — magicstudio's strongest argument for reviving the axis is destroyed by this driver.

At lower arousal cuts the picture gets worse, not better (cut=0.35 → a 4-way cell-3 collapse {magicstudio, Lena, Img3, example}). **There is no arousal cut that separates the six.** The proposed driver fails the load-bearing check.

### 2.2 Under the RECOMMENDED driver (spread-edge PRIMARY + `complexity` SECONDARY, cell-2-guarded) — CLEAN

PRIMARY = `band_activity_spread(edge_activity)` against BROAD=0.33 / BUSY=0.66 (cells 1/0/2 ramp, unchanged). SECONDARY = `complexity ≥ PIECE_COMPLEXITY_PROFILED` diverts to cell 3 **only when the primary did not already land cell 2** (the guard — so the genuine busy outlier keeps its even-subdivided cell-2 gait and the profiled cell 3 stays reserved for the *textured-but-not-busiest* images). Cut = **0.20** (§3):

| image | spread-edge | primary cell | `complexity` | ≥ 0.20 & primary≠2 → divert? | **final cell** | gait character |
|---|---|---|---|---|---|---|
| magicstudio | 0.000 | 1 | 1.000 | **yes** | **3** | profiled/syncopated over a SUSTAINED band — "still but lurching" |
| Img1 | 0.222 | 1 | 0.005 | no | **1** | broadest/augmented — calmest gait |
| Lena | 0.499 | 0 | 0.164 | no | **0** | S39 anchor — broad-but-moving |
| Img3 | 0.505 | 0 | 0.229 | **yes** | **3** | profiled/syncopated — splits from Lena |
| Img2 | 0.509→0.553 | 0 | 0.015 | no | **0** | S39 anchor (shares cell 0 with Lena — see note) |
| example | 0.847 | 2 | 0.905 | guard blocks (primary=2) | **2** | busiest/even-subdivided — the genuine ARP outlier |

**Cell distribution: {3: [magicstudio, Img3], 1: [Img1], 0: [Lena, Img2], 2: [example]} → FOUR distinct cells {1, 0, 2, 3}, all occupied.**

**Residual collapses (both benign, both documented as acceptable):**
- **Lena + Img2 on cell 0.** This is NOT the Img3/Lena watch-pair — Img3 has been split OFF to cell 3. Lena and Img2 share cell 0, but they are **already separated on the BAND axis** (Lena → DOTTED band, Img2 → SYNC band per the S50 spread table) AND on **character** (Lena → scherzo, Img2 → hymn). The full (band, cell, character) tuple stays distinct. A shared *cell* between two images that diverge on band and character is the documented acceptable tie (`rhythm_variety_s50.rs:464` permits exactly the Img3/Lena tie historically; here the permitted tie moves to Lena/Img2, which is equally benign and equally well-separated on the other two axes).
- **magicstudio + Img3 on cell 3.** Same logic: magicstudio is SUSTAINED-band/march, Img3 is DOTTED-band/scherzo. Shared cell, divergent band+character → distinct tuple. This is exactly the S50-validated outcome (`design-s50-affect-cutpoints.md:137` — "Img3 and example BOTH take cell 3 ... but ... diverge on band AND character").

**This driver separates the six into four distinct cells AND preserves the full-tuple distinctness, with no collapse re-pinning a watch-pair onto a single identity. It passes the load-bearing check; the proposed arousal driver does not.**

**Confidence: HIGH** on the separation table itself (pure arithmetic on measured features). **Confidence: HIGH** that the recommended driver strictly dominates the proposed one on this acceptance set (4 distinct cells with watch-pairs split, vs. 3 distinct cells with both watch-pairs collapsed).

---

## 3. ANSWER 3 — the cut philosophy and the recommended PROFILED const

### 3.1 PRIMARY cuts (BROAD/BUSY) — confirm the spread spans them for the real cluster

The seam spec asks me to confirm the spread re-expansion actually spans CELL_EDGE_BROAD=0.33 / CELL_EDGE_BUSY=0.66 for the real-photo cluster (the S50 trap was cuts that never reached the cluster behind a pre-filter). **Confirmed — there is no pre-filter this slice, and the spread band genuinely straddles both cuts:**

- Spread values across the six: 0.000, 0.222, 0.499, 0.505, 0.553, 0.847. They span from below BROAD (0.33) — magicstudio 0.000, Img1 0.222 — through the [0.33, 0.66) middle band (Lena/Img3/Img2 at 0.499–0.553) — up past BUSY (0.66) — example 0.847. **All three ramp cells {1, 0, 2} are reached by the real cluster.** The cuts are correctly positioned relative to the SPREAD band; do NOT move them. (This is the S50 lesson honored: the cut values stay; the spread does the re-positioning; the un-gating makes the whole thing reachable.)
- One margin note for the Producer: the [0.33, 0.66) middle band is wide (0.499–0.553 all sit in it), which is *why* three images pile onto cell 0 before the secondary divert. The PRIMARY cannot split the mid-cluster on its own — that is structurally the SECONDARY's job, and it is why the secondary-key choice (Answer 1) is load-bearing rather than a tiebreak garnish.

### 3.2 The SECONDARY (PROFILED) cut — recommended const

**The const is `PIECE_COMPLEXITY_PROFILED`, NOT `PIECE_AROUSAL_PROFILED`** (per Answer 1 — the divert keys on `complexity`, not `affect_arousal`).

```rust
/// PER-PIECE cell-3 (PROFILED / SYNCOPATED) divert cut, keyed on `complexity`, applied
/// UN-GATED (no complexity>=0.4 theme pre-filter — the S53 un-gating removes that trap, which
/// is the only reason this cut now reaches the real-photo cluster). Distinct from the themed-path
/// `CELL_COMPLEXITY_PROFILED` (composition.rs:1792, still 0.66 for the synthetic theme path).
/// Diverts a visually-INTRICATE image onto the profiled/syncopated character gait. Guarded so
/// it does NOT fire when the primary (spread-edge) already chose the busiest cell 2 — the genuine
/// high-activity outlier keeps its even-subdivided gait; cell 3 is reserved for textured-but-not-
/// busiest images (so example stays cell 2, magicstudio/Img3 take cell 3).
const PIECE_COMPLEXITY_PROFILED: f32 = 0.20;   // recommended; ear-tune within [0.18, 0.23]
```

**Cut philosophy and the affect rationale for 0.20:**

- **Reachability floor (the hard constraint):** the cut MUST be ≤ 0.229, or Img3 (complexity 0.229) never reaches cell 3 and the Img3/Lena watch-pair never splits. This is the lower bound on usefulness.
- **Selectivity ceiling:** the cut must be > 0.164 (Lena) or Lena ALSO diverts and the cell-0 anchor empties. So the valid window that splits Img3 from Lena is **(0.164, 0.229]**.
- **0.20 sits in the middle of that window** — just above the calm-photo complexity floor (Img1 0.005, Img2 0.015, Lena 0.164 all stay on the density ramp) and just below the intricate images (Img3 0.229, magicstudio 1.0, example 0.905 would divert but example is guard-blocked). Perceptual reading: an image must read as *genuinely textured/intricate* — not merely edgy or saturated — before its piece adopts the syncopated character. 0.20 is where "intricate enough to deserve the profiled gait" sits for natural photos.
- **Ear-tune window [0.18, 0.23]:** toward 0.23 if three images sharing the cell-3 *gait* (magicstudio, Img3, and — if the guard is dropped — example) reads as a residual sameness (the S50 aesthetics flag); toward 0.18 if the divert feels under-applied and the mid-cluster sounds too uniform. **Affect-neutral across [0.18, 0.23]** — it is a by-ear call about how prominent the cell-3 gait sounds, exactly as the S50 doc flagged. Do not go below 0.18 (Lena starts diverting at 0.164) or above 0.23 (Img3 stops diverting at 0.229).

**The cell-2 guard is a separate, structural decision, not a tunable:** divert to cell 3 only when `primary_cell != 2`. Without it, example (complexity 0.905) diverts to cell 3 and you lose the cell-2 (busiest/even-subdivided) landing entirely — collapsing back to 3 distinct cells. **Recommend the guard be implemented as part of `pick_piece_cell`'s control flow** (compute the primary first; only consult the complexity divert if primary ∈ {0, 1}). Confidence: HIGH that the guard is necessary on this set (verified: without it, distinct-cell count drops 4 → 3).

---

## 4. ANSWER 4 — the two carried watch-items

### 4.1 (a) The magicstudio SUSTAINED + March tuple — does the driver give it a distinct gait?

**The strongest argument for reviving the cell axis.** magicstudio is the calmest image by edge (0.106 → spread 0.000 → SUSTAINED band) yet the most extreme by complexity (1.000) and lands march character (arousal 0.389 / valence 0.454). Its whole identity is the *paradox* — a still, low-energy surface that is nonetheless densely textured. The cell axis is the only knob that can express that paradox: it can give a SUSTAINED-band piece a *profiled/syncopated cell* so the held band is animated by a lurching, off-grid macro-gait ("still but lurching" — `chord_engine.rs:3304` Pendulum cell 3 = "lopsided, lurching gait", :3253 InvertedArch cell 3 = "weighted floor").

- **Under the PROPOSED (arousal) driver: FAILS.** magicstudio arousal 0.389 clears no usable divert cut, so it takes the PRIMARY cell 1 (spread 0.000 < BROAD) — the *broadest/augmented* gait. That is the OPPOSITE of profiled: it gives the most intricate image the calmest, most featureless macro-gait, AND collapses it onto cell 1 with Img1. The magicstudio watch-item is not resolved — it is actively mishandled. This is the single clearest demonstration that arousal is the wrong key.
- **Under the RECOMMENDED (complexity) driver: RESOLVED.** magicstudio complexity 1.000 ≥ 0.20 and its primary cell is 1 (not 2), so it diverts to **cell 3** — the profiled/syncopated gait — riding on its SUSTAINED band. It gets the "still but lurching" identity the paradox demands. **Verdict: the recommended driver gives magicstudio a distinct, perceptually-correct gait; the proposed driver does not.** Confidence: HIGH.

### 4.2 (b) The Img3 ~ Lena near-pair — does the driver split them?

The residual the cell axis is meant to split. Img3 and Lena are a near-tie on edge (0.475 / 0.471 → spread 0.505 / 0.499, both cell-0 band) AND on arousal (0.391 / 0.375, within 0.016). They differ on complexity (0.229 / 0.164) and colorfulness (0.423 / 0.122).

- **Under the PROPOSED (arousal) driver: FAILS.** Img3 arousal 0.391 and Lena arousal 0.375 — the difference is 0.016. No cut splits them: any cut below 0.375 diverts both; any cut above 0.391 diverts neither. They both stay on cell 0. The watch-pair is NOT split — it is exactly the collapse the un-gating was supposed to cure. **Arousal is constitutionally unable to split this pair** (they are an arousal tie); this is a structural, not a tuning, failure.
- **Under the RECOMMENDED (complexity) driver: SPLIT.** Img3 complexity 0.229 ≥ 0.20 → diverts to cell **3**; Lena complexity 0.164 < 0.20 → stays cell **0**. The watch-pair splits on the one axis that genuinely separates them. **Verdict: the recommended driver splits Img3 from Lena; the proposed driver cannot.** Confidence: HIGH.

  - **If colorfulness is chosen instead of complexity** (the §1.5 fallback): Img3 0.423 vs Lena 0.122 splits even more strongly (gap 0.301), with a cut anywhere in (0.122, 0.423], e.g. 0.20. Either feature resolves the watch-pair; arousal cannot.

### 4.3 What driver adjustment the proposed driver would need (since it fails both)

The seam spec asked, if the driver does NOT resolve the watch-items, what adjustment would. The answer is the whole of Answer 1: **replace the secondary key `affect_arousal` with `complexity` (recommended) or `colorfulness` (fallback), rename the const `PIECE_AROUSAL_PROFILED` → `PIECE_COMPLEXITY_PROFILED`, set it to 0.20, and add the cell-2 guard.** That single substitution converts both watch-item failures into resolutions and lifts the distinct-cell count from 3 to 4. The seam spec's carrier, signature shape, `RhythmMotto` object, and freeze discipline are all correct and unchanged — only the second scalar argument to `pick_piece_cell` and its cut const change.

---

## 5. HARD CONSTRAINTS — compliance check

- **Routes THROUGH the S46 figure-ground governor (`melody_activity_class`, `chord_engine.rs:1136`); never promotes a background voice's `ActivityClass` above the melody (preserve S46 6-VARIED).** My recommendation is purely about WHICH cell index the per-piece motto carries; it does not touch the per-voice onset realization. The motto biases onset placement *within* the band the band-ladder chose (seam spec I-4/I-5), and the governor clamp is unchanged. **The cell-3 (profiled) gait re-distributes onsets but cannot add onset COUNT beyond what the band permits for that role** — so a background voice carrying the same motto cannot exceed the melody's `ActivityClass`. The driver choice (complexity vs arousal) is governor-neutral: it changes which gait, not how many onsets the role is permitted. **Compliant — no figure-ground impact from the driver decision.** Confidence: HIGH (the driver sits entirely upstream of the governor).
- **Preserves the S50 cross-piece band/character/tempo spread.** The recommended driver reuses `band_activity_spread` unchanged (so the band-ladder spread is untouched) and keys the secondary on `complexity` (which does NOT feed the character/tempo SelectTables — those read arousal/valence, `mappings.json` character rules). So the cell decision is orthogonal to the band/character/tempo axes; it adds a fourth axis of variety without collapsing the existing three. The §2.2 table shows the full (band, cell, character) tuples staying distinct. **Compliant.** Confidence: HIGH.
- **DIMENSIONAL not categorical.** Both PRIMARY (graded spread-edge against cuts) and SECONDARY (graded complexity against a cut) are dimensional thresholds on continuous features, not categorical lookups. The cell-3 divert is a 1-bit thresholding of a continuous feature — coarse, but dimensional. (§1.5 honestly flags it as 1-bit, not graded — the ideal would be a graded syncopation amount, which the carrier does not yet support; deferred.) **Compliant within the slice's carrier limits.**
- **ACTUAL field names verified in composition.rs.** `edge_activity`, `complexity`, `colorfulness`, `avg_saturation`, `affect_arousal` all verified at `composition.rs:44/48/59/65/114`. `affect_arousal` is the planner composite seated at `:1399–1402` (the `-1.0` sentinel is overwritten before `plan()` proceeds), so it would be safe to read — it is simply the wrong key. `complexity` is a direct pixel stat (`:48`), available unconditionally. **Verified.**
- **Every rule carries a confidence level; pure-Rust sufficiency stated honestly.** Done throughout; §1.5 carries the explicit pure-Rust-sufficient-but-not-ideal boundary.

---

## 6. SUMMARY OF RECOMMENDATIONS FOR THE MUSIC THEORY PRODUCER (the exact consts to implement)

1. **`pick_piece_cell` secondary scalar = `complexity`, NOT `affect_arousal`.** Pass `u.complexity` as the second scalar (the seam spec's signature slot currently labeled `affect_arousal`). Fallback to `u.colorfulness` if the complexity-driven cell-3 gait sounds wrong on Img3 (ear-test).
2. **`const PIECE_COMPLEXITY_PROFILED: f32 = 0.20;`** — replaces the proposed `PIECE_AROUSAL_PROFILED`. Valid window (0.164, 0.229]; ear-tune within [0.18, 0.23]. Keeps the themed-path `CELL_COMPLEXITY_PROFILED = 0.66` (composition.rs:1792) UNCHANGED.
3. **PRIMARY cuts unchanged:** `CELL_EDGE_BROAD = 0.33`, `CELL_EDGE_BUSY = 0.66`, applied to `band_activity_spread(edge_activity)`. Confirmed to span the real cluster (§3.1) — do not move them.
4. **Add the cell-2 guard:** divert to cell 3 only when the primary cell ∈ {0, 1} (i.e. `primary != 2`), so the genuine busy outlier (example) keeps its even-subdivided cell-2 gait. Structural, not tunable.
5. **Result:** six probes → four distinct cells {Img1:1, Lena:0, Img2:0, example:2, magicstudio:3, Img3:3}, with magicstudio's paradoxical gait resolved and the Img3/Lena watch-pair split; the two shared-cell pairs (Lena/Img2, magicstudio/Img3) remain distinct on band and character.

---

## Appendix — files read for this design (absolute paths)

- `/home/qweary/working/audiohax-engagement/AudioHax/docs/design-s53-cell-seam.md` (the seam spec — DP-1, carrier, signatures, freeze discipline)
- `/home/qweary/working/audiohax-engagement/AudioHax/src/composition.rs` (`ImageUnderstanding` 40–117; `affect_composite` 337–366; affect seat 1399–1402; reverted consts 1779–1792; `pick_rhythm_cell` 1806–1826)
- `/home/qweary/working/audiohax-engagement/AudioHax/src/chord_engine.rs` (`band_activity_spread` + consts 1071–1104; `melody_activity_class` + `ActivityClass` 1106–1151; MELODY band cutoffs 1061–1063; `rhythm_cells` vocabulary 3241–3315; `rhythm_cell`/`rhythm_cell_count` 3321–3333)
- `/home/qweary/working/audiohax-engagement/AudioHax/assets/mappings.json` (affect.arousal_weights/valence_weights/character_tempo 138–169; character rules 160–169; theme_behaviour 120–123)
- `/home/qweary/working/audiohax-engagement/AudioHax/tests/rhythm_variety_s50.rs` (probe set 78–86; `understand`/`perf_for` 99–115; `band_for` 186–208; the permitted-tie machinery ~464)
- `/home/qweary/working/audiohax-engagement/AudioHax/docs/design-s50-affect-cutpoints.md` (canonical six-probe feature table + S50 cut tables + worked spread/cell/band/character tables)
- `/home/qweary/working/audiohax-engagement/AudioHax/docs/diag-s49-recurring-rhythm.md` (the full probe table incl. colorfulness/brightness/saturation, lines 61–69; the dormancy diagnosis)
- No `docs/research-affect-crossmodal.md` present (grounding taken from the S50 affect-cutpoints literature citations + S49/S50 empirical measurements).

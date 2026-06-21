# Design S50 — Affect Cut-Point VALUES (the taste-owned numbers for the rhythm-variety re-range)

Author: Perceptual / Cross-Modal Affect Specialist (DESIGN/VALUES ONLY — no src/asset/test file modified; the Music Theory Specialist is the single writer of `chord_engine.rs`/`composition.rs`/`mappings.json`)
Blueprint: `docs/diag-s49-recurring-rhythm.md` (fix-directions 1 + 4) · Seam spec: `docs/spec-s50-rhythm-variety.md` (§2.A/B/C — the TASTE-OWNED placeholders)
Scope: author the perceptual cut VALUES the spec left as placeholders, so distinct real images land on DISTINCT rhythmic surfaces (cross-piece sameness — the operator's recurring `[dotted-q, eighth]→triplet→long` motif). Within-piece monotony is fix-direction-2 (deferred).

## The affect bridge I am applying (my knowledge base)

The single load-bearing correspondence I own: **visual activity → rhythmic activity**, and **felt energy (arousal) → tempo/character**. The literature is unambiguous on the direction (it is the *placement* of the cut that is taste-subjective, not the sign):

- **Arousal ↑ → faster tempo, denser onsets, more subdivision, toward staccato** (Hevner 1937 adjective circle; Eerola, Friberg & Bresin 2013 "Emotional expression in music: contribution, linearity, and additivity"; Juslin & Laukka 2003 meta-analysis — tempo and articulation are the two strongest arousal cues). The arousal composite here is **saturation-dominant** (`avg_saturation` weight 0.45, the single largest term), which matches the cross-modal finding that chroma/saturation reads as energy.
- **Valence ↑ (brightness-led) → major mode, brighter character**; valence ↓ → minor, darker. This sets the scherzo(bright)/march(neutral-dark) and hymn(bright)/lament(dark) split on the calm and energetic sides.

The two perceptual questions every value below must answer, with a confidence level and no arbitrary numbers:

1. **Band / cell cuts:** *how visually active must an image be before its melody should SUBDIVIDE rather than SUSTAIN?*
2. **Character gate:** *how energetic must a photo FEEL before it should march/scherzo rather than sit in a slow ballad?*

The measured real-image distribution I am placing the cuts against (spec §0, verified against live weights):

| image | edge_activity | complexity | arousal | valence |
|---|---|---|---|---|
| AudioHaxImg1 | 0.301 | 0.005 | 0.211 | 0.321 |
| AudioHaxImg2 | 0.509 | 0.015 | 0.259 | 0.678 |
| AudioHaxImg3 | 0.475 | 0.229 | 0.391 | 0.584 |
| example | 0.719 | 0.905 | 0.696 | 0.528 |
| Lena | 0.471 | 0.164 | 0.375 | 0.647 |
| magicstudio-art | 0.106 | 1.000 | 0.389 | 0.454 |

The edge cluster is compressed into ≈0.30–0.51 (four of six), with `magic` low (0.106) and `example` high (0.719). Arousal is compressed into ≈0.21–0.39, with `example` the lone outlier at 0.696. **My job is to place cuts so the compressed cluster fans out, without over-driving the busy tail into a wall of fragmentation.**

---

## 1. THE BAND-SPREAD VALUES (spec §2.A — Family A, the per-step melody band)

The spread is `out = clamp(CENTER + (edge − CENTER) · slope, 0, 1)`, with `slope = GAIN_LOW` below the center and `GAIN_HIGH` above it, identity at `edge == CENTER` (the freeze witness). It is applied ONLY to the band-ladder comparison input (never the articulation curve, FILL_REST, or `/0.05` — the spec §3.3 freeze discipline). Band cuts stay at ARP=0.80 / SYNC=0.55 / DOTTED=0.25; the spread does the re-positioning.

| constant | value | valid range | confidence | perceptual rationale |
|---|---|---|---|---|
| `BAND_SPREAD_CENTER` | **0.40** | [0.30, 0.50] | **HIGH** | The pivot must sit at the perceptual centroid of the natural-photo activity cluster so the stretch is symmetric about "an ordinary photo." The four clustered edges (0.301, 0.471, 0.475, 0.509) have their mass at ≈0.42; 0.40 is the round value nearest that centroid and inside the freeze-neutral requirement (it equals the reference input). At 0.40 a "typically active" photo neither sustains nor over-subdivides — it sits on the DOTTED↔SYNC boundary, the perceptual neutral of "moving but not busy." |
| `BAND_SPREAD_GAIN_LOW` | **1.8** | [1.0, 3.0] | **HIGH** | Slope below center, opening the calm side toward SUSTAINED. Calm photos (`magic` 0.106, `Img1` 0.301) must be pushed down hard enough to clear the DOTTED floor (0.25) into genuine SUSTAINED — otherwise the calmest images still subdivide, which is the central defect. 1.8 maps Img1 (0.301) to 0.222 (just below DOTTED → SUSTAINED) and saturates magic to 0.0. A lower gain (≤1.5) leaves Img1 in DOTTED and the calm end fails to open; this is the load-bearing knob for the SUSTAINED band's existence. |
| `BAND_SPREAD_GAIN_HIGH` | **1.4** | [1.0, 3.0] | **HIGH (deliberately the lowest workable)** | Slope above center, toward SYNC/ARP. **Asymmetric and lower than GAIN_LOW on purpose**: the busy side must open just enough to separate Img2 (0.509→SYNC) and example (0.719→ARP) from the DOTTED pair (Img3, Lena), but NOT so much that mid-cluster photos get flung into ARPEGGIO — that is the inverse "computer-like fragmentation" failure (spec §7 risk 2). 1.4 is the minimum gain that puts exactly ONE image (example, the genuine outlier) into ARP and keeps the mid-cluster (Img3, Lena ≈0.47) in DOTTED. This is the over-drive guard expressed as a value. |

### Worked table — `band_activity_spread(edge)` and resulting band (cuts 0.80/0.55/0.25)

| image | edge | spread(edge) | band |
|---|---|---|---|
| magicstudio-art | 0.106 | 0.000 | **SUSTAINED** |
| AudioHaxImg1 | 0.301 | 0.222 | **SUSTAINED** |
| Lena | 0.471 | 0.499 | **DOTTED** |
| AudioHaxImg3 | 0.475 | 0.505 | **DOTTED** |
| AudioHaxImg2 | 0.509 | 0.553 | **SYNC** |
| example | 0.719 | 0.847 | **ARPEGGIO** |

All four bands occupied; the cluster fans SUSTAINED→DOTTED→SYNC→ARP. Before the spread, every one of these six landed in DOTTED (or SUSTAINED for magic) — a single band. After, the six occupy all four.

---

## 2. THE CELL VALUES (spec §2.B — Family B, the theme rhythm-cell)

`pick_rhythm_cell`: `complexity ≥ PROFILED && K>3 → cell 3` (the profiled/character gait); else `edge < BROAD → cell 1`; else `edge < BUSY → cell 0`; else `cell 2`. Re-positioning the three constants only — no new code path.

| constant | value | valid range | confidence | perceptual rationale |
|---|---|---|---|---|
| `CELL_COMPLEXITY_PROFILED` | **0.20** | [0.12, 0.66] | **MEDIUM-HIGH** | The complexity gate for the character/syncopated cell 3. The old 0.66 is unreachable for natural photos (real complexity 0.005–0.23 except the two synthetic-art images), so cell 3 was dead. 0.20 sits *just above* the calm-photo complexity floor (Img1/Img2/Lena at 0.005–0.164 stay on the density ramp) and *just below* the visually-intricate images (Img3 0.229, example 0.905, magic 1.0 divert to the character gait). Perceptually: an image must read as genuinely *textured/intricate* — not merely edgy — before its theme adopts the syncopated/profiled character; 0.20 is where "intricate enough to deserve the character gait" sits for photos. The decorrelating tiebreak now actually fires (Img3 and Lena have near-identical edge 0.475/0.471 but split: Img3→cell 3, Lena→cell 0). |
| `CELL_EDGE_BROAD` | **0.38** | [0.25, 0.42] | **MEDIUM** | Below this, a calm image takes the broadest/augmented gait (cell 1). Set just below the cluster floor (Img1 at 0.301) so Img1 takes the broad gait; tightened up from 0.33 toward the cluster so it actually separates the calm tail from the mid. |
| `CELL_EDGE_BUSY` | **0.50** | [0.45, 0.66], invariant BROAD<BUSY | **MEDIUM** | Above this, a busy image takes the even-subdivided gait (cell 2). Set at 0.50 so the mid-cluster (Img3 0.475, Lena 0.471 — those not diverted to cell 3) take the S39-anchor cell 0, while the busier Img2 (0.509) and example take cell 2. This splits the formerly-collapsed `[0.33,0.66)→cell 0` band into cell-0 (mid) vs cell-2 (busy). |

Invariant `BROAD(0.38) < BUSY(0.50)` holds.

### Worked table — cell per image (PROFILED=0.20, BROAD=0.38, BUSY=0.50, K=4)

| image | edge | complexity | path | cell |
|---|---|---|---|---|
| AudioHaxImg1 | 0.301 | 0.005 | comp<0.20; edge<0.38 → broad | **1** |
| AudioHaxImg2 | 0.509 | 0.015 | comp<0.20; edge≥0.50 → busy | **2** |
| Lena | 0.471 | 0.164 | comp<0.20; 0.38≤edge<0.50 → anchor | **0** |
| AudioHaxImg3 | 0.475 | 0.229 | comp≥0.20 → profiled | **3** |
| example | 0.719 | 0.905 | comp≥0.20 → profiled | **3** |
| magicstudio-art | 0.106 | 1.000 | comp≥0.20 → profiled | **3** |

All four cells {0,1,2,3} occupied. Three images take cell 3 (Img3, example, magic) but they each sit in a *different band* (DOTTED / ARP / SUSTAINED) and different character, so the full tuples stay distinct (§4).

---

## 3. THE CHARACTER GATE VALUES (spec §2.C, Option C1 — gate move in `mappings.json composition/character`)

Option C1 chosen by the lead (NOT C2 — no composite re-center; I do not touch the arousal NUMBER, only where the cut sits). The character family is squarely the affect bridge's domain (diagnosis fix-direction 4). I lower the scherzo/march arousal gate so mid-arousal real photos (0.21–0.39) can leave the ballad default, and I **confirm + adjust the valence splits** as the task asked.

### 3a. The arousal gate

| constant | value | valid range | confidence | perceptual rationale |
|---|---|---|---|---|
| scherzo/march `arousal ge` | **0.34** | [0.28, 0.60] | **HIGH** | "How energetic must a photo FEEL before it marches/scherzos rather than sits in a ballad?" The honest mid-arousal band is 0.21–0.39. The old 0.60 gate is unreachable, pinning everything to ballad. 0.34 sits at the *median* of the real arousal distribution, so it cleanly bisects the set: the lower-energy half (Img1 0.211, Img2 0.259) stays calm (lament/hymn/ballad), the upper-energy half (Img3 0.391, magic 0.389, Lena 0.375, example 0.696) crosses into march/scherzo. Placing it at the data median is the perceptually-defensible "above-average energy → it moves" call, and keeps a meaningful calm class instead of marching everything. |

### 3b. The valence splits — CONFIRMED + ONE ADJUSTMENT (task: "confirm the valence splits too")

Currently scherzo needs `valence ge 0.55`, march needs `valence lt 0.45`. **This leaves a deadzone [0.45, 0.55): a high-arousal image with neutral valence matches NEITHER and falls through to ballad.** That is a defect for the affect bridge: it dumps the *busiest image in the set* (`example`, arousal 0.696, valence 0.528) into the slow ballad — perceptually backwards. The fix is to close the deadzone.

| split | old | NEW | confidence | rationale |
|---|---|---|---|---|
| scherzo `valence ge` | 0.55 | **0.55 (UNCHANGED)** | HIGH | Scherzo is the *playful/bright* energetic character — it should require genuinely positive valence (bright image). 0.55 is correct; leave it. |
| march `valence lt` | 0.45 | **0.55 (RAISED to meet scherzo)** | MEDIUM-HIGH | March is the *driving/neutral-to-dark* energetic character. Raising its ceiling from 0.45 to 0.55 closes the [0.45,0.55) deadzone so the two energetic characters partition the valence axis with no gap: above 0.55 → scherzo (bright energy), below 0.55 → march (neutral/dark energy). This routes `example` (val 0.528, neutral) to **march** rather than ballad — correct: a high-arousal, neutral-valence image should drive, not lull. It does NOT make scherzo unreachable (Img3 0.584, Lena 0.647 still ≥ 0.55 → scherzo). |

This is the minimal edit that fixes the deadzone: the scherzo split is untouched; only march's ceiling moves up to abut it. (The calm-side splits — lament val<0.35, hymn val≥0.55, nocturne 0.35–0.47 — are unchanged and correct; they already partition the calm valence axis.)

### Worked table — character + tempo window per image (gate 0.34; scherzo val≥0.55, march val<0.55)

| image | arousal | valence | rule fired | character | tempo window (BPM) |
|---|---|---|---|---|---|
| AudioHaxImg1 | 0.211 | 0.321 | ar≤0.30 & val<0.35 | **lament** | 44–66 |
| AudioHaxImg2 | 0.259 | 0.678 | ar≤0.30 & val≥0.55 | **hymn** | 60–92 |
| Lena | 0.375 | 0.647 | ar≥0.34 & val≥0.55 | **scherzo** | 120–168 |
| AudioHaxImg3 | 0.391 | 0.584 | ar≥0.34 & val≥0.55 | **scherzo** | 120–168 |
| magicstudio-art | 0.389 | 0.454 | ar≥0.34 & val<0.55 | **march** | 96–132 |
| example | 0.696 | 0.528 | ar≥0.34 & val<0.55 | **march** | 96–132 |

4 distinct characters (lament, hymn, scherzo, march); the tempo windows take **4 distinct ranges** (44–66, 60–92, 96–132, 120–168) — far above the ≥3 floor. The character/tempo pin is broken: photos now span a slow-lament-to-fast-scherzo spread instead of all sitting in the 56–96 ballad window.

---

## 4. THE SPREAD CHECK — (band, cell, character) tuple per image

| image | band | cell | character | tuple |
|---|---|---|---|---|
| AudioHaxImg1 | SUSTAINED | 1 | lament | (SUSTAINED, 1, lament) |
| AudioHaxImg2 | SYNC | 2 | hymn | (SYNC, 2, hymn) |
| AudioHaxImg3 | DOTTED | 3 | scherzo | (DOTTED, 3, scherzo) |
| example | ARPEGGIO | 3 | march | (ARP, 3, march) |
| Lena | DOTTED | 0 | scherzo | (DOTTED, 0, scherzo) |
| magicstudio-art | SUSTAINED | 3 | march | (SUSTAINED, 3, march) |

**Distinct tuples: 6 / 6** — every image is unique. Floor is ≥4; achieved 6.

Directional sanity (spec §4 requirement): the busiest image `example` (edge 0.719 → ARPEGGIO band) does NOT share a band with the calmest `magic` (edge 0.106 → SUSTAINED band). ✔

Note the decorrelation working even where two families coincide: Img3 and example BOTH take cell 3 and BOTH could share a band-or-character, but Img3 is (DOTTED, scherzo) while example is (ARP, march) — they diverge on band AND character. Lena and Img3 have near-identical edge (0.471/0.475) but split on cell (0 vs 3) via the complexity tiebreak. magic and Img1 share SUSTAINED but split on cell (3 vs 1) and character (march vs lament). The three families are genuinely independent, which is why six images yield six tuples rather than collapsing.

---

## 5. LOAD-BEARING vs GARNISH

**Load-bearing (HIGH confidence — a re-collapse fails loudly if these move wrong):**
- `BAND_SPREAD_CENTER = 0.40` — the freeze-neutral reference AND the cluster centroid; both roles pin it.
- `BAND_SPREAD_GAIN_LOW = 1.8` — without enough low-side gain the SUSTAINED band never opens (Img1 stays DOTTED) and the calm end re-collapses. This is the single most consequential value for the calm half.
- `BAND_SPREAD_GAIN_HIGH = 1.4` — load-bearing as the *over-drive guard*: it is deliberately the lowest gain that still separates SYNC/ARP from DOTTED. Its job is as much to NOT push the mid-cluster into ARP as to spread.
- scherzo/march `arousal ge = 0.34` — the data-median bisector; the whole character/tempo un-pin hinges on it.
- march `valence lt = 0.55` — load-bearing because the OLD value (0.45) creates the deadzone that re-dumps the busiest image into ballad. This is a correctness-shaped fix, not a taste tweak.

**Tuned-by-ear / garnish (MEDIUM — a wide valid range, set against the six but adjustable on the ear-test):**
- `CELL_EDGE_BROAD = 0.38` / `CELL_EDGE_BUSY = 0.50` — these split the density ramp but only matter for the non-diverted images; the audible effect of cell-0 vs cell-2 is subtler than band or character. Tune by ear if Img2/Lena/Img3's gaits feel wrong.
- `CELL_COMPLEXITY_PROFILED = 0.20` — MEDIUM-HIGH. It is load-bearing for cell-3 *reachability* (must be ≤ ~0.23 or Img3 never reaches it), but the exact value in [0.12, 0.23] is by-ear: too low over-diverts every mildly-textured photo to the character gait (spec §7 risk 4). 0.20 keeps the divert selective (3 of 6); dial toward 0.23 if it feels over-applied.
- scherzo `valence ge = 0.55` — confirmed unchanged; garnish only in that the exact bright/neutral boundary is taste, but 0.55 is the established convention.

---

## 6. OVER-DRIVE GUARD (spec §7 risk 2 — "spreading the bands could over-drive into ARPEGGIO")

Verdict: **GUARDED — exactly one image reaches ARPEGGIO, and it is the genuine outlier.**

- Only `example` (edge 0.719, the single high-activity image, complexity 0.905) lands in ARP, at spread 0.847. Every mid-cluster photo (Img2 0.553, Img3 0.505, Lena 0.499) stays at or below SYNC — none is flung into ARP.
- The asymmetric gain is the guard: `GAIN_HIGH = 1.4` is the *minimum* that separates the busy tail. Raising it toward the spec ceiling (3.0) would start pulling Img2 (0.509) and even the mid pair toward ARP — re-introducing the "computer-like fragmentation" the operator already disliked (S13 re-listen note). I have deliberately chosen the lowest workable high-side gain.
- Margin check: Img2 spreads to 0.553, just over the SYNC cut (0.55) — it is the closest mid-image to the next band up, but it lands in SYNC (correct, it is the second-busiest), not ARP (needs >0.80). The next image down, Img3 at 0.505, sits safely in DOTTED. No mid-cluster photo is within reach of ARP.
- Calm-side inverse guard: GAIN_LOW = 1.8 saturates magic to 0.0 and pushes Img1 to 0.222 (SUSTAINED) — it opens the calm band without any image getting "stuck" mid-spread. No under-drive (everything-sustains) failure either: Lena/Img3 still subdivide (DOTTED).

---

## 7. OPEN TENSIONS for the Aesthetics lens

Where my affect call (the *correct emotional/energetic mapping*) might diverge from the Aesthetics call (*does the catalogue sound pleasing and varied to the ear*):

1. **The GAIN_HIGH ceiling (the central tension).** I set GAIN_HIGH = 1.4, the minimum that spreads, to protect against ARPEGGIO over-drive. Aesthetics may find the busy half *too tame* — that Img2/example don't sound energetic *enough* and want more subdivision (higher gain). My affect position: pushing the gain up risks the fragmentation the operator explicitly disliked. **Resolve by ear on example + Img2**: if they read as under-energized, raise GAIN_HIGH toward 1.6 and re-check that no THIRD image enters ARP. This is the one knob most likely to move after the ear-test.

2. **Three images on cell 3.** My complexity gate (0.20) routes Img3, example, AND magic to the character/syncopated cell. Affect-wise that is correct (all three are visually intricate). But Aesthetics may hear three pieces sharing the syncopated *gait* as a residual sameness — even though their bands/characters differ. If the cell-3 gait is distinctive enough to read as "the same trick three times," consider raising PROFILED toward 0.23 so only example+magic (the truly extreme-complexity pair) divert, leaving Img3 on the density ramp (cell 0). Affect-neutral either way; it is an ear call about how prominent the cell-3 gait sounds.

3. **example → march vs scherzo.** I route example (arousal 0.696, valence 0.528) to march via the deadzone-closing split. Its valence is a hair under 0.55. If Aesthetics hears example as *bright/playful* rather than *driving/neutral*, the scherzo split (0.55) is the lever — but I would not lower it below 0.50, or genuinely neutral images start reading as playful. Flagged as a borderline case, not a defect.

4. **The lament floor on Img1.** Img1 → lament (44–66 BPM), the slowest window, paired with the SUSTAINED band — a very sparse, very slow piece. Affect-correct (low arousal, low valence = somber/still). But Aesthetics should confirm it doesn't read as *empty* rather than *calm*; if so, that is a within-piece-density concern (fix-direction-2), not a cut-value concern — I flag it so it isn't mistaken for a mis-placed cut.

---

## Appendix — value summary (what the Music Theory Specialist writes)

| family | constant | value | file |
|---|---|---|---|
| A (band) | `BAND_SPREAD_CENTER` | 0.40 | chord_engine.rs |
| A (band) | `BAND_SPREAD_GAIN_LOW` | 1.8 | chord_engine.rs |
| A (band) | `BAND_SPREAD_GAIN_HIGH` | 1.4 | chord_engine.rs |
| B (cell) | `CELL_COMPLEXITY_PROFILED` | 0.20 | composition.rs |
| B (cell) | `CELL_EDGE_BROAD` | 0.38 | composition.rs |
| B (cell) | `CELL_EDGE_BUSY` | 0.50 | composition.rs |
| C1 (character) | scherzo/march `arousal ge` | 0.34 | mappings.json |
| C1 (character) | scherzo `valence ge` | 0.55 (unchanged) | mappings.json |
| C1 (character) | march `valence lt` | 0.55 (raised from 0.45) | mappings.json |

**Test-Engineer note (re-bless, NOT an expectation change):** lowering the arousal gate to 0.34 does not break `tests/affect_s22.rs` literal assertions — the scherzo corner fixture (`bright_energetic`, arousal≈0.93/valence≈0.64) still selects Scherzo, and the lament corner (`calm_dark`, arousal≈0.20/valence≈0.24) still selects Lament. BUT in `affect_arousal_monotone_in_saturation_downstream` the lo_sat fixture (arousal≈0.545, valence≈0.469) now selects **march** instead of ballad; the `assert_ne!(plo, Scherzo)` and `plan_bpm(phi) > plan_bpm(plo)` assertions still pass (march ≠ scherzo; scherzo 120–168 still faster than march 96–132), so the test stays GREEN, but the doc-comments referencing "the 0.60 Scherzo gate" go stale and should be updated to 0.34. No golden moves (spec §3.2 — character SelectTable is off the engine_equivalence golden path).

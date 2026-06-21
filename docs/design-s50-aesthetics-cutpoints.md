# Design S50 — Aesthetics Cut-Points for the Rhythm-Variety Re-Range

Author: Composition & Songwriting Aesthetics Specialist (DESIGN/VALUES ONLY — no src/asset/test file modified; the Music Theory Specialist is the single writer)
Lens: the *pleasing CATALOGUE* — does the set of six sound genuinely varied AND does each piece still breathe (not monotonous, not gratuitously fragmented/busy)?
Blueprint: `docs/diag-s49-recurring-rhythm.md`; seam: `docs/spec-s50-rhythm-variety.md` (the TASTE-OWNED placeholders + valid ranges)
Parallel author: the Affect specialist sets the same values from the affect-bridge lens; the lead reconciles where we differ (the S49 taste-gate reconciliation pattern). §7 lists my open tensions for that reconciliation.

---

## 0. The governing question and the two failure modes

I am not asking "what does THIS image's affect demand" (that is Affect's lens). I am asking: **across the six bundled images, does the resulting CATALOGUE sound varied, and does each individual piece still sound like music a person would write?**

Two failure modes bracket every value below:

- **TOO STATIC** (gains near 1.0, gates near the top): the six stay collapsed on DOTTED + cell-0 + ballad — the very monotony we are fixing. Pleasing variety requires the six to *actually diverge*; a listener A/B-ing two images must clearly hear different rhythmic character.
- **TOO BUSY** (gains too high, gates too low): mid-cluster photos get shoved into ARPEGGIO over-subdivision and over-fast character — the "restless, computer-like, gratuitously busy" inverse failure (spec §7 risk 2). A boring-but-musical piece beats an ambitious-but-fidgety one.

The sweet spot: variety **perceptible between pieces**, each piece **still breathing**. Because the measured real-image edge cluster is *compressed* (four of six between 0.301 and 0.509), even a modest stretch is enough to fan it across boundaries — so I deliberately sit on the **conservative side of every gain and gate**. We are moving off "boring" by a controlled amount, not sprinting toward "busy."

The measured distribution I design against (spec §0):

| image | edge_activity | complexity | arousal | valence |
|---|---|---|---|---|
| AudioHaxImg1 | 0.301 | 0.005 | 0.211 | 0.321 |
| AudioHaxImg2 | 0.509 | 0.015 | 0.259 | 0.678 |
| AudioHaxImg3 | 0.475 | 0.229 | 0.391 | 0.584 |
| example | 0.719 | 0.905 | 0.696 | 0.528 |
| Lena | 0.471 | 0.164 | 0.375 | 0.647 |
| magicstudio-art | 0.106 | 1.000 | 0.389 | 0.454 |

---

## 1. BAND SPREAD VALUES (§2.A)

Mechanism (Architecture's, fixed): `out = clamp(CENTER + (edge_activity - CENTER) * gain, 0, 1)`, where `gain = GAIN_LOW` below CENTER, `GAIN_HIGH` at/above CENTER; identity at `edge_activity == CENTER`. The *unmapped* `out` is compared against the unchanged cuts **ARP=0.80 / SYNC=0.55 / DOTTED=0.25** (prom_shift=0 on the neutral catalogue path).

### My values

| constant | value | valid range | placeholder |
|---|---|---|---|
| `BAND_SPREAD_CENTER` | **0.42** | [0.30, 0.50] | 0.40 |
| `BAND_SPREAD_GAIN_LOW` | **2.0** | [1.0, 3.0] | 1.8 |
| `BAND_SPREAD_GAIN_HIGH` | **1.35** | [1.0, 3.0] | 1.4 |

### Aesthetic rationale

- **CENTER = 0.42 (above the placeholder 0.40).** CENTER is the pivot *and* the freeze-neutral reference: the image whose edge sits exactly at CENTER is unmoved. I place it at **0.42** for two reasons. (1) It is the median of the four-photo natural cluster (0.301, 0.475, 0.509, plus Lena 0.471 → median ≈ 0.47–0.48; I pull slightly below to 0.42 to keep the bottom of the cluster firmly on the *opening* low-gain side). (2) Crucially, **DOTTED is the home/default texture and I want it to stay the catalogue's anchor band, not vanish.** With CENTER=0.42, the calm-side images open toward SUSTAINED while the cluster middle stays in DOTTED — DOTTED remains the gravitational center of the catalogue, which is correct: a varied catalogue should still have a *home* texture that several pieces share, with the outliers diverging. A catalogue where all six land in six different bands is its own kind of incoherence (no shared identity); the pleasing target is "a home with excursions."

- **GAIN_LOW = 2.0 (above placeholder 1.8).** The calm side is where the catalogue most needs to open: the lowest natural photo (magic, 0.106) and Img1 (0.301) must be pulled DOWN toward / into SUSTAINED so the calm end of the catalogue actually sounds calm (held tones) rather than the omnipresent DOTTED long-short. A gain of 2.0 is aggressive *downward* but downward motion toward SUSTAINED is the safe direction — it can never cause the busy/fidgety failure; the worst it does is make a calm image calmer, which is exactly what a calm image should be. So I spend my variety budget on the side that has no over-busy risk.

- **GAIN_HIGH = 1.35 (BELOW placeholder 1.4 — my deliberate conservatism).** This is the side that can over-drive into ARPEGGIO fragmentation (the "computer-like" inverse failure). I cap it *below* the placeholder. The reasoning is asymmetric on purpose: pushing a mid-cluster photo UP past SYNC (0.55) is musically fine (syncopation is pleasing and human), but pushing it past ARP (0.80) is the fragmentation cliff. I want the high side to comfortably reach SYNC for the genuinely busier photos (Img2 0.509, example 0.719) **without** dragging anything below `example` into ARPEGGIO. 1.35 reaches SYNC cleanly and keeps ARPEGGIO reserved for the one genuinely high-edge image (`example`). See §5 for the precise cliff arithmetic.

### Worked band table (CENTER=0.42, LOW=2.0, HIGH=1.35)

`out = 0.42 + (e − 0.42)·gain`, then band by ARP 0.80 / SYNC 0.55 / DOTTED 0.25.

| image | e | side | out | band |
|---|---|---|---|---|
| magicstudio-art | 0.106 | low | 0.42 + (−0.314)·2.0 = **−0.208 → 0.00** | **SUSTAINED** |
| AudioHaxImg1 | 0.301 | low | 0.42 + (−0.119)·2.0 = **0.182** | **SUSTAINED** (<0.25) |
| Lena | 0.471 | high | 0.42 + (0.051)·1.35 = **0.489** | **DOTTED** |
| AudioHaxImg3 | 0.475 | high | 0.42 + (0.055)·1.35 = **0.494** | **DOTTED** |
| AudioHaxImg2 | 0.509 | high | 0.42 + (0.089)·1.35 = **0.540** | **DOTTED** (just under 0.55) |
| example | 0.719 | high | 0.42 + (0.299)·1.35 = **0.824** | **ARPEGGIO** (>0.80) |

Band spread achieved: **SUSTAINED ×2, DOTTED ×3, ARPEGGIO ×1** — three distinct bands, with DOTTED as the shared home and the two extremes (calmest → SUSTAINED, busiest → ARPEGGIO) cleanly separated. Img2 at 0.540 sits a hair under the SYNC cut; see §5 + §7 for the open tension on whether to nudge it into SYNCOPATED.

**Why this is the more pleasing catalogue than the placeholders:** the placeholders (0.40 / 1.8 / 1.4) put Img1 at out = 0.40+(−0.099)·1.8 = 0.222 (still SUSTAINED, fine) but leave Img2 at 0.40+(0.109)·1.4 = 0.553 → just over SYNC, and push example to 0.40+(0.319)·1.4 = 0.847 (ARPEGGIO, same). My version keeps the *middle* of the catalogue anchored in DOTTED (a coherent home) while still spreading the extremes — the placeholder spreads slightly wider but at the cost of a thinner DOTTED home. Both are defensible; mine prioritizes catalogue coherence over maximal spread, which is the more pleasing listen for a *set*. (This is a genuine taste fork — flagged for the Affect lens in §7.)

---

## 2. CELL VALUES (§2.B)

Selector (composition.rs:1800-1808, unchanged in shape):
```
complexity >= CELL_COMPLEXITY_PROFILED && K>3 → cell 3   (profiled/character gait)
else edge_activity <  CELL_EDGE_BROAD          → cell 1   (broad/augmented)
else edge_activity <  CELL_EDGE_BUSY           → cell 0   (S39 anchor, broad-but-moving)
else                                           → cell 2   (busy/even-subdivided)
```

### My values

| constant | value | valid range | placeholder |
|---|---|---|---|
| `CELL_COMPLEXITY_PROFILED` | **0.20** | [0.12, 0.66] | 0.20 |
| `CELL_EDGE_BROAD` | **0.38** | [0.25, 0.42] | 0.38 |
| `CELL_EDGE_BUSY` | **0.50** | [0.45, 0.66] | 0.50 |

### Aesthetic rationale

- **CELL_COMPLEXITY_PROFILED = 0.20 (matches placeholder).** The cell-3 "character gait" is the *decorrelating diversion* (composition.rs:1796): a visually-intricate image gets a distinctive profiled rhythm *regardless* of its edge density. At 0.20 the divert is **selective**, not promiscuous: Img3 (0.229) and magic (1.000) qualify; Img1/Img2/Lena/example do not. That is exactly the pleasing outcome — the two visually-intricate images get a *characterful* macro rhythm distinct from the density ramp, and the rest stay on the ramp where edge density meaningfully separates them. Going lower (toward 0.12) would start diverting Lena (0.164) too, collapsing the ramp into the character cell — over-diverting (spec §7 risk 4). 0.20 is the floor that keeps the divert a *spice*, not the *staple*.

- **CELL_EDGE_BROAD = 0.38 (matches placeholder, top of its valid band).** I sit this at the *high end* of [0.25, 0.42] on purpose. Cell 1 (broad/augmented) is the slowest macro gait; I want it reserved for the genuinely calm images so the broad gait *means* calm. At 0.38, only Img1 (0.301, after the cell-3 check fails) takes cell 1 — a clean "calm image → broad gait" mapping. (magic also < 0.38 but is diverted to cell 3 first by its complexity.) A lower BROAD would push nothing extra into cell 1 here, but the high placement keeps the boundary clear for future images.

- **CELL_EDGE_BUSY = 0.50 (matches placeholder).** The cell-0 home band is `[0.38, 0.50)`; cell-2 (busy/even-subdivided) is `>= 0.50`. At 0.50 the cluster *splits cleanly*: Lena (0.471) and Img3-absent-divert would sit in cell 0, while Img2 (0.509) and example (0.719) reach the busy cell 2. This gives the cell axis the same "home + excursions" shape as the band axis: cell 0 stays the shared macro-rhythm home, with cell 1 (calm), cell 2 (busy), and cell 3 (character) as the three excursions. Pushing BUSY higher (toward 0.66) would re-collapse Img2 back onto cell 0 — re-creating the very pinning we are removing.

### Worked cell table

cell-3 check first (`complexity >= 0.20`), then edge cuts BROAD 0.38 / BUSY 0.50.

| image | complexity | cell-3? | edge | cell |
|---|---|---|---|---|
| AudioHaxImg1 | 0.005 | no | 0.301 < 0.38 | **cell 1** (broad) |
| magicstudio-art | 1.000 | **yes** | — | **cell 3** (character) |
| AudioHaxImg3 | 0.229 | **yes** | — | **cell 3** (character) |
| Lena | 0.164 | no | 0.471 ∈ [0.38,0.50) | **cell 0** (anchor) |
| AudioHaxImg2 | 0.015 | no | 0.509 >= 0.50 | **cell 2** (busy) |
| example | 0.905 | **yes** | — | **cell 3** (character) |

Cell spread: **cell 0 ×1, cell 1 ×1, cell 2 ×1, cell 3 ×3** — all four cells occupied. Note three images converge on cell 3 (the two intricate photos + magic + example). That is acceptable for *macro* variety because those three differ on the *band* axis (Img3 DOTTED, example ARPEGGIO, magic SUSTAINED) and on character/tempo — the tuple, not any single axis, is what the listener perceives. See §4.

---

## 3. CHARACTER GATE VALUE (§2.C, Option C1)

C1 = lower the scherzo/march arousal gate in `mappings.json composition/character`. Current: `arousal ge 0.6`. Default = ballad.

### My value

| constant | value | valid range | placeholder |
|---|---|---|---|
| character scherzo/march `arousal ge` | **0.36** | [0.28, 0.60] | 0.34 |

### Aesthetic rationale

- **0.36 (ABOVE the placeholder 0.34 — my deliberate conservatism on the tempo axis).** Character drives *tempo* (via the per-character window + de-cap), and tempo is the single most audible variety axis — but also the one most able to make a piece feel rushed/fidgety if a calm-looking photo is shoved into a fast march/scherzo. I want the march/scherzo (fast) characters reserved for photos that genuinely *read* energetic, and I want the calm photos to stay in the slower ballad/lament/nocturne windows.

  At **0.36**: example (0.696) clearly marches/scherzos; Img3 (0.391) and Lena (0.375) cross into march/scherzo; magic (0.389) crosses; Img2 (0.259) and Img1 (0.211) stay ballad/lament. That gives a **clean tempo bimodal**: the two genuinely-calm photos stay slow, the four mid-to-high photos get the faster window. 

  Why 0.36 not the placeholder 0.34: at 0.34, Img3 (0.391), Lena (0.375), magic (0.389) all still cross — *no image changes band between 0.34 and 0.36*. The two values are **behaviorally identical on this six-image set** (the nearest image, magic at 0.389, sits above both; the next-lowest, Img2 at 0.259, sits below both). I choose 0.36 purely as the more *defensible* perceptual floor — "a photo must read at least mid-energetic (0.36) to march" — and because it leaves a wider safety margin above Img2's 0.259, so a slightly-busier future natural photo near 0.30 won't tip into a fast tempo it can't musically support. Since the choice is behaviorally free on the actual six, I take the more conservative number. (This is a near-tie; if the Affect lens prefers 0.34 for affect-bridge symmetry, I do not object — see §7.)

### Worked character / tempo table (gate at 0.36; valence splits unchanged)

Existing valence-split rules from spec §1 Family C: scherzo `arousal ge gate & valence ge 0.55`; march `arousal ge gate & valence lt 0.45`; lament `arousal le 0.3 & valence lt 0.35`; hymn `arousal le 0.3 & valence ge 0.55`; nocturne `arousal le 0.35 & valence in [0.35,0.47]`; else ballad. (Note: the spec leaves the mid-valence band `[0.45,0.55)` at-or-above gate falling to ballad — a real photo can be energetic but neither bright-enough for scherzo nor dark-enough for march.)

| image | arousal | valence | character | tempo feel |
|---|---|---|---|---|
| AudioHaxImg1 | 0.211 | 0.321 | **lament** (ar≤0.3 & val<0.35) | slow |
| AudioHaxImg2 | 0.259 | 0.678 | **hymn** (ar≤0.3 & val≥0.55) | slow-mid |
| Lena | 0.375 | 0.647 | **scherzo** (ar≥0.36 & val≥0.55) | fast |
| AudioHaxImg3 | 0.391 | 0.584 | **scherzo** (ar≥0.36 & val≥0.55) | fast |
| magicstudio-art | 0.389 | 0.454 | **ballad** (ar≥0.36 but val∈[0.45,0.55) → default) | mid |
| example | 0.696 | 0.528 | **ballad** (ar≥0.36 but val∈[0.45,0.55) → default) | mid |

Character spread: **lament, hymn, scherzo ×2, ballad ×2** — four distinct characters, tempo spread across slow / slow-mid / fast / mid (≥3 distinct tempo windows, satisfying the spec §4 metric 3 floor). Note this is the *current* valence-split behavior; I am only moving the arousal gate. The two ballad fall-throughs (magic, example) are a real artifact of the mid-valence gap, not a defect of my gate value — flagged in §7 as an Affect-owned question (the valence splits are affect-bridge territory, not mine to move).

---

## 4. CATALOGUE SPREAD CHECK

The `(band, cell, character)` tuple per image (meter excluded — deferred per spec §2.D):

| image | band | cell | character | tuple |
|---|---|---|---|---|
| AudioHaxImg1 | SUSTAINED | 1 | lament | (SUS, 1, lament) |
| AudioHaxImg2 | DOTTED | 2 | hymn | (DOT, 2, hymn) |
| AudioHaxImg3 | DOTTED | 3 | scherzo | (DOT, 3, scherzo) |
| example | ARPEGGIO | 3 | ballad | (ARP, 3, ballad) |
| Lena | DOTTED | 0 | scherzo | (DOT, 0, scherzo) |
| magicstudio-art | SUSTAINED | 3 | ballad | (SUS, 3, ballad) |

**Distinct tuples: 6 of 6 — all unique.** Comfortably clears the spec §4 floor of ≥4. (Pre-S50 this was effectively 1.)

**Directional sanity (spec §4 metric 1):** busiest `example` (band ARPEGGIO) ≠ calmest `magic` (band SUSTAINED). PASS — the extremes are maximally separated on the band axis.

**No-over-busy check (my lens):** only ONE image (`example`, the genuinely high-edge 0.719 / high-complexity 0.905 photo) reaches ARPEGGIO. Nothing in the mid-cluster (Img1/2/3, Lena) is pushed into ARPEGGIO. The two SUSTAINED images are the two calmest. The DOTTED home holds three pieces. No image is simultaneously ARPEGGIO-band AND fast-character AND busy-cell — i.e. no piece stacks all three "busy" axes (example is ARP+cell3 but lands on ballad/mid tempo, not scherzo; Img3 is scherzo+cell3 but DOTTED, not ARP). **No piece is a subdivision stress-test.** PASS.

---

## 5. THE VARIETY-VS-FIDGETINESS BALANCE

The sweet spot lives almost entirely on the **GAIN_HIGH** and **character-gate** knobs, because those are the two that can manufacture the "busy/fidgety/computer-like" inverse failure. The other knobs (GAIN_LOW, CELL_*, CENTER) move pieces toward *calm* or toward *macro* distinctions that don't fragment the surface — they carry essentially no fidgetiness risk, so I let them spread freely.

**The ARPEGGIO cliff (the governing constraint).** ARPEGGIO fires at `out > 0.80`. With CENTER=0.42 and GAIN_HIGH=g, an image at edge e lands in ARPEGGIO when `0.42 + (e−0.42)·g > 0.80`, i.e. `e > 0.42 + 0.38/g`. 

- At g=1.35: threshold e > 0.42 + 0.281 = **0.701**. Only `example` (0.719) clears it. The next-busiest natural photo (Img2, 0.509) is far below. **Safe margin of 0.19 in edge-space.**
- At g=1.4 (placeholder): threshold e > 0.42 + 0.271 = **0.691**. Still only example. Safe, but the margin to a hypothetical busier photo shrinks.
- At g=1.6: threshold e > 0.42 + 0.238 = **0.658**. Still only example on *this* set, but a future natural photo at 0.66 would tip into ARPEGGIO — and 0.66 is a plausible busy-but-not-frantic photo. Too close.
- At g=2.0: threshold e > 0.42 + 0.19 = **0.61**. Now any moderately-busy photo fragments. **Over the cliff.**

So the gain-ceiling reasoning: **GAIN_HIGH must keep the ARPEGGIO threshold comfortably ABOVE the natural mid-cluster ceiling (~0.51) with margin for unseen photos.** I set 1.35 → threshold 0.701, leaving a ~0.19 cushion above the busiest mid-cluster photo. That is the ceiling: high enough that the busy side still spreads (Img2 climbs from raw 0.509 to 0.540, approaching SYNC; example reaches ARPEGGIO), low enough that ARPEGGIO stays the exclusive province of the one genuinely-frantic image.

**The gate floor.** The character gate (0.36) is held *above* the placeholder for the symmetric reason: it keeps the fast-tempo characters off the two calmest photos. The floor is "no photo below mid-energy (0.36) gets a fast window." I would not go below 0.34 (the placeholder), and I prefer 0.36, because dropping toward 0.28 would start pulling Img2 (0.259) — no, Img2 stays below even 0.28 — but it narrows the safety margin to near-zero for any future ~0.30 photo, which is precisely the band where a calm photo on a fast tempo reads as "rushed." The conservative floor costs nothing on the actual six (behaviorally identical to 0.34) and buys margin.

**Where the sweet spot sits, summarized:** spend the variety budget on the *no-risk* side (GAIN_LOW=2.0 opening the calm end toward SUSTAINED; the cell + character distinctions that vary *macro* feel without fragmenting the *micro* surface), and stay *conservative* on the two risk knobs (GAIN_HIGH=1.35 capping the ARPEGGIO cliff with margin; gate=0.36 keeping fast tempo off calm photos). The result: 6/6 distinct tuples, three bands with a coherent DOTTED home, four cells, four characters — perceptibly varied — while exactly one image (the genuinely busy one) is allowed into the fragmented ARPEGGIO band and no piece stacks all three busy axes. That is variety that rewards without a single piece becoming a stress-test.

---

## 6. V1-ESSENTIAL vs LATER

| value | V1-essential? | why |
|---|---|---|
| `BAND_SPREAD_GAIN_LOW = 2.0` | **ESSENTIAL** | The audible win. Opening the calm end into SUSTAINED is what makes magic/Img1 stop sounding like the omnipresent DOTTED long-short. Without it the calm half of the catalogue stays collapsed. |
| `BAND_SPREAD_CENTER = 0.42` | **ESSENTIAL** | Determines WHERE the cluster pivots; with the wrong center the spread fans the wrong images. Also the freeze reference. |
| `BAND_SPREAD_GAIN_HIGH = 1.35` | **ESSENTIAL (as a cap)** | Even 1.0 (no-op high side) would still give 5 distinct tuples, but the busy end (Img2/example separation) needs >1.0 to spread example into ARPEGGIO. Essential value; its *exact* level (1.35 vs 1.4) is refinement. |
| `CELL_COMPLEXITY_PROFILED = 0.20` | **ESSENTIAL** | This is what un-deads cell 3 — it is the single change that makes 3 of the 6 cells reachable. Without it the cell axis stays {0,1} and the macro-rhythm variety is half-gone. |
| `CELL_EDGE_BUSY = 0.50` | **ESSENTIAL** | Splits Img2/example off cell 0 into cell 2; without it the busy photos re-collapse onto the anchor. |
| `CELL_EDGE_BROAD = 0.38` | refinement | On the actual six it only re-labels Img1's already-distinct calm gait; the spread survives at any value in [0.25,0.42]. Keep at 0.38, but not the load-bearing value. |
| character gate = 0.36 | **ESSENTIAL** | Un-pins tempo (the most audible axis). Some value in [0.34,0.40] is essential; the exact 0.36 vs 0.34 is a free refinement (behaviorally identical on the six). |

The **two non-negotiable, audible-win values**: `BAND_SPREAD_GAIN_LOW` (opens the calm end) and `CELL_COMPLEXITY_PROFILED` (un-deads cell 3). If only two values shipped, those two move the needle most for a listener A/B-ing the set.

---

## 7. OPEN TENSIONS for the Affect lens

Where my conservative ceiling/floor may differ from the affect-driven value, stated explicitly for the lead's reconciliation:

1. **GAIN_HIGH: I cap at 1.35; Affect may want ≥1.4.** The affect bridge may argue that a high-arousal photo *should* feel more subdivided, pushing GAIN_HIGH up so more of the busy half reaches SYNCOPATED/ARPEGGIO. My catalogue-lens objection: above ~1.5 the ARPEGGIO threshold drops below 0.66 and risks fragmenting future natural photos. **Reconcilable middle: 1.4** (placeholder) keeps the ARPEGGIO threshold at 0.691 — still example-only on the six — and is the obvious compromise if Affect wants more high-side motion. I will not defend below 1.35 *down* (that's static) nor above 1.5 *up* (that's the cliff).

2. **Img2 sits at out=0.540, a hair under the SYNC cut (0.55).** Affect (reading Img2's bright valence 0.678 as "energetic/lively") may want Img2 to reach SYNCOPATED, which would require either GAIN_HIGH≈1.5 (threshold tension with #1) OR nudging CENTER down to ~0.40 (placeholder) so Img2 clears 0.55. This is the sharpest taste fork: my catalogue lens is content with Img2 in DOTTED (it is already distinguished by cell 2 + hymn character), but if Affect wants the band axis to also separate it, **CENTER=0.40 + GAIN_HIGH=1.4** pushes Img2 to 0.40+(0.109)·1.4=0.553 → SYNCOPATED, at the cost of thinning the DOTTED home to 2 pieces. Both are musical; the lead picks whether catalogue-home-coherence (mine) or band-axis-completeness (likely Affect's) wins.

3. **CENTER: I prefer 0.42 (coherent DOTTED home); Affect likely prefers ~0.40 (wider spread).** Directly coupled to #2. The lower CENTER spreads more aggressively but dissolves the shared home band. My recommendation if reconciling toward Affect: 0.40 is acceptable; I would resist going below 0.38 (DOTTED stops being any image's home → catalogue loses its center of gravity).

4. **Character gate: I prefer 0.36; Affect may prefer 0.34.** Behaviorally identical on the six (no image lies between). Pure free choice — I yield to whatever the Affect lens needs for affect-bridge symmetry. Not a real tension.

5. **The two ballad fall-throughs (magic, example) via the mid-valence gap [0.45,0.55).** Two energetic photos (arousal 0.389 / 0.696) land on slow-mid ballad purely because their valence sits in the uncovered mid-band — so the *highest-arousal image in the set gets a mid tempo*, which my ear flags as a mild catalogue oddity (the busiest-looking photo isn't the fastest). This is **Affect-owned** (valence splits are affect-bridge territory, outside my four taste-owned constants), but I surface it: if Affect can extend a mid-valence energetic character (or widen scherzo/march valence coverage), example would get the fast tempo its edge/complexity earns, tightening the "busy looks → busy sounds" coupling. Not blocking V1; a real refinement for the catalogue's internal logic.

---

## Summary value set

| constant | my value | placeholder | note |
|---|---|---|---|
| `BAND_SPREAD_CENTER` | 0.42 | 0.40 | coherent DOTTED home; tension #2/#3 |
| `BAND_SPREAD_GAIN_LOW` | 2.0 | 1.8 | spend budget on no-risk calm side |
| `BAND_SPREAD_GAIN_HIGH` | 1.35 | 1.4 | ARPEGGIO-cliff cap; tension #1 |
| `CELL_COMPLEXITY_PROFILED` | 0.20 | 0.20 | un-deads cell 3 (essential) |
| `CELL_EDGE_BROAD` | 0.38 | 0.38 | refinement |
| `CELL_EDGE_BUSY` | 0.50 | 0.50 | splits busy off cell 0 (essential) |
| character scherzo/march `arousal ge` | 0.36 | 0.34 | un-pins tempo; tension #4 (free) |

Catalogue spread: **6/6 distinct (band,cell,character) tuples**; busiest example (ARP) ≠ calmest magic (SUS); no piece stacks all three busy axes. Within spec floors (≥4 tuples, ≥3 tempo windows) with margin.

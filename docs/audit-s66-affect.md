# Audit S66 — The Affect Bridge and Cross-Piece Sameness (READ-ONLY)

**Axis:** image → affect (valence/arousal) → musical character. **Scope:** the affect
composite, the `character` / `texture` selectors, `character_tempo`, and `mode_valence_cuts`
(`assets/mappings.json` `affect`/`character`/`texture` blocks; `src/composition.rs`
`affect_composite`, `character_tempo_bpm`, `valence_family_mode`, `CompositionPlanner::plan`).
No code modified; `src/engine.rs` untouched. All numbers are the real pure-Rust
`understand_image_pure` values carried forward from `diagnostic-s62-motivic-derivation.md`
(feature/affect table) and `diagnostic-s64-fixed-rhythm-key.md` (per-image roots/modes), and
the tempo figures are computed against the current `brightness_to_tempo_bpm` anchors and the
per-character windows now in the mappings.

**The question this audit answers.** The engine is a stack of feature → selector mappings.
Cross-piece sameness can come from two directions: (i) the architecture has too few structural
dimensions downstream, or (ii) the input signal is collapsed — the six images' affect vectors
land in too narrow a region, so even a good selector set is fed near-identical inputs and every
piece lands in the same cell. This audit tests direction (ii) only: **are the affect knobs even
turning across the corpus?**

**Verdict up front:** the knobs ARE turning. The affect bridge is materially more alive than the
"always a ballad" framing assumes — the `character` selector spreads across four distinct cells,
zero images fall to the ballad default, final tempo spans 66–132 BPM (a ~2:1 ratio), and valence
(not hue) genuinely owns the major/minor split. Affect-input collapse is **MINOR** — a real but
secondary contributor, localized to the arousal axis — **not** the primary cause of sameness. The
dominant cause is architectural/downstream, exactly where the theory and aesthetics lenses point.

---

## 1. Corpus affect-spread table

Per image: the seated arousal/valence composite, the resulting `character` (first-match-wins over
the `character` rules), its tempo window, the brightness-derived raw BPM, and the final BPM after
the per-character window clamp. Character and mode were recomputed from the rules against the S62
affect numbers and match the characters measured on the real planner in `diagnostic-s63`
(Lament / Hymn / Scherzo / March / Scherzo / March) and the modes measured in `diagnostic-s64` —
so **confidence is HIGH** for every row (two independent measured diagnostics agree with the
recomputation).

| image | arousal | valence | character | tempo window | raw BPM (bri) | **final BPM** | mode / family | conf |
|---|---|---|---|---|---|---|---|---|
| AudioHaxImg1 | 0.211 | 0.338 | **Lament** | 44–66 | 86.5 | **66** | Dorian / minor (valence-forced) | HIGH |
| AudioHaxImg2 | 0.259 | 0.692 | **Hymn** | 60–92 | 144.7 | **92** | Lydian / major (valence-forced) | HIGH |
| AudioHaxImg3 | 0.391 | 0.594 | **Scherzo** | 120–168 | 126.2 | **126** | Mixolydian / major | HIGH |
| example | 0.696 | 0.535 | **March** | 96–132 | 107.4 | **107** | Dorian / neutral band → hue mode | HIGH |
| Lena | 0.375 | 0.650 | **Scherzo** | 120–168 | 132.0 | **132** | Ionian / major (valence-forced) | HIGH |
| magicstudio | 0.389 | 0.459 | **March** | 96–132 | 102.7 | **103** | Mixolydian / neutral band → hue mode | HIGH |

**What this table shows — the knobs are turning:**

- **Character spread: 4 distinct cells over 6 images** (Lament×1, Hymn×1, Scherzo×2, March×2).
  **Zero images land on the `ballad` default.** The "every image is a ballad" failure the affect
  bridge was created to fix is a **past state that S22–S64 work has already resolved**; the current
  `character` selector escapes ballad on 6/6 images.
- **Tempo spread: 66 → 132 BPM, a ~2:1 ratio.** This is a wide, audible tempo range, not a
  collapsed band. The per-character window clamp is doing real work *against* brightness: Img2 is a
  bright image (raw 145 BPM) pulled *down* to a 92-BPM Hymn; Img1 is dark (raw 86) pulled down to a
  66-BPM Lament. Affect, not brightness alone, is setting pace — which is precisely the corrected
  behavior the bridge was meant to install.
- **Valence owns mode (the load-bearing caveat, correctly implemented).** `mode_valence_cuts`
  (major_min 0.55, minor_max 0.45) is present and active: Img2 (0.692), Img3 (0.594), Lena (0.650)
  are forced MAJOR; Img1 (0.338) is forced MINOR; example (0.535) and magicstudio (0.459) fall in
  the neutral dead band and keep the hue-derived church mode as a colorist garnish. Hue is correctly
  demoted to within-family color; it does not decide the load-bearing third. This is the one caveat
  most likely to be gotten wrong, and it is right.
- **Affect is not narrowly wired.** Arousal/valence are consumed by three separate selectors, not
  one: `character` (→ tempo), `texture` (→ figuration; several rules gate on arousal/valence and the
  S63 integration re-tune routes the calm cluster to figured beds), and `mode_valence_cuts` (→
  major/minor). The affect signal reaches multiple downstream dimensions.

---

## 2. Verdict: **MINOR** (a real but secondary contributor, localized to the arousal axis)

Affect-input collapse is **not** the primary cause of cross-piece sameness. But it is not a clean
NOT-A-CAUSE either, because **one axis — arousal — is genuinely compressed** for the real-photo
cluster, and that compression starves the high-energy downstream routes. The two findings that fix
the verdict at MINOR:

**Finding A — valence spreads acceptably; arousal is compressed.**

- Valence range across the corpus: 0.338 – 0.692 (span 0.354), reasonably distributed. Valence is
  carrying the discrimination load.
- Arousal range: 0.211 – 0.696 (span 0.485) — but the distribution is deceptive. **Four of the six
  images sit in a razor-thin band: Img3 0.391, magicstudio 0.389, Lena 0.375, and Img2 0.259 /
  Img1 0.211 just below.** The three "typical photo" images that clear the mid-arousal line
  (Img3, Lena, magicstudio) do so at 0.375–0.391 — a **0.016-wide cluster** parked right on the
  0.34 character threshold. Only `example` (0.696), the one high-complexity abstract painting,
  reaches genuinely high arousal.
- Consequence 1: among the four mid-cluster images the **arousal axis barely discriminates at
  all** — their character split (Scherzo vs March) is decided entirely by *valence*, not arousal.
  Arousal is close to a constant for the bulk of the corpus.
- Consequence 2 (the load-bearing one): every downstream rule with an arousal threshold **above
  ~0.40 is reachable only by `example`.** The high-energy figured textures — `pad_broken_up`
  (arousal ≥ 0.60), `pad_block_comp` (≥ 0.70), `pad_stride` (≥ 0.75) — are structurally
  **unreachable by any ordinary photo**, because a real photo's arousal tops out near 0.39. The
  richest, busiest accompaniment textures in the catalogue are gated behind an arousal level the
  input signal never produces for five of six images.

**Finding B — the compression is upstream, in saturation.** Arousal weights are
saturation 0.45 (dominant) + colorfulness 0.25 + edge_activity 0.20 + complexity 0.10. The
real-photo saturations are 30–52 (normalized 0.30–0.52) — themselves a compressed mid-band, never
approaching the high end. With the dominant term compressed and the lighter terms unable to lift
the sum, arousal is structurally pinned in the low-mid for natural images. This is a genuine
input-collapse on one axis, and it does have a downstream cost (Consequence 2).

**Why it is nonetheless only MINOR, not material:** despite the arousal compression, the bridge
still produced 4 distinct characters, a 2:1 tempo spread, and a working valence→mode split — because
valence supplies the second axis and rescues the discrimination. The affect input is *turning
enough* to place the six images in distinct character/tempo/mode cells. The sameness the listener
keeps reporting is **not** "every piece has the same tempo/character/mode" — the table above proves
those differ. It is a sameness of *feel and development* that lives downstream of affect, in the
dimensions the other lenses audit: the within-piece flatness (one rhythmic gait per whole piece,
figuration arc firing on 1/6 pieces), the temporal container (the phrase/cadence scaffold, form
collapse), and the harmonic/motivic vocabulary. Those are architecture, not affect input.

Set against the grading standard (`research-s66-sameness-principles.md`): the affect bridge governs
*macro character selection* — which cell a piece occupies. The standard's own ranking puts the
sameness in the *temporal container* (Axis 4 groove/feel, Axis 3 form/harmonic-rhythm) and in
*within-piece development* (Axis 1, Axis 5's textural arc). Affect selection sets a piece's starting
coordinates well; it does not, and cannot, supply the developmental dimensions whose absence the
standard identifies as the root cause. A correctly-chosen character played through a container that
never develops still sounds like the same archetype — which is exactly what the ear reports.

---

## 3. The single highest-leverage affect-side fix (judged honestly against the architectural alternative)

**If** an affect-side fix is pursued, the highest-leverage one is: **re-curve the arousal composite
so the real-photo cluster spreads across the arousal range instead of bunching at 0.34–0.39.**
Concretely, apply a fixed, authored, monotonic expansion to the arousal blend — either a gamma/curve
on the compressed saturation input, or a corpus-calibrated re-weighting that gives colorfulness,
edge_activity, and complexity more pull relative to the compressed saturation term — so that ordinary
photos with real visual energy can climb past 0.40–0.60 and *reach* the high-arousal texture routes
(`pad_broken_up`, `pad_block_comp`, `pad_stride`) that only `example` currently reaches. This obeys
the balance law: it is a curated deterministic curve seeded by the existing features, no randomness
introduced. It would convert arousal from a near-constant into a live discriminating axis and unlock
three currently-dead high-energy accompaniment textures for the corpus.

**But this fix is honestly secondary, and here is the honest limit:** widening the arousal input only
helps to the extent that the dimensions it unlocks are themselves richly varied *and develop within a
piece*. The vocabulary-breadth diagnostic (`diagnostic-s63`) already showed the downstream is
container-uniform: one rhythmic motto cell cloned onto every section of a piece, the figuration arc
alive on 1/6 images, form collapsed to one skeleton on 5/6, the bass library 100% unused. Feeding a
wider arousal signal into a still-uniform container spreads *which* texture each piece starts on, but
does not make any single piece *develop*, and development-vs-restatement is what the grading standard
identifies as the sameness root. So the arousal re-curve is a worthwhile companion move — it removes
a genuine input-side starvation of the high-energy routes — but it is a *resolution-within-class*
improvement (which cell you land in), not a *class-multiplying* one (adding a developmental
dimension). The standard is explicit that only the latter changes the between-piece distance the
sameness verdict measures.

Two caveats bound even this secondary fix. First, the pure-Rust vs ML line: energy/arousal is fully
reachable now in pure Rust — the compression is a curve-shape problem, not a semantic-recognition
problem, so no ML tier is needed to spread it. What is *not* reachable is semantic joy (knowing the
scene depicts something happy) — that still needs scene recognition and stays out of scope. Second,
the musical-fear caveat still binds any expressive wiring downstream of arousal: in music, fear is
fast + minor + *soft*, distinct from anger (fast + minor + loud) — so a re-curved arousal must not be
wired naively as "more arousal = louder" across the board.

---

## 4. Recommendation for the S67 fundamental fix: **architecture-side (primary), affect-side arousal re-curve as a low-cost secondary companion**

The S67 fundamental fix should be **architecture-side.** The evidence is that the affect input is
already turning well enough: 4 distinct characters, a 2:1 tempo spread, a working valence→mode split,
and affect wired into three downstream selectors. The corpus does **not** collapse into one
character/tempo/mode cell — direction (ii) is not where the sameness originates. The listener's
persistent verdict is about *feel and lack of development*, which the grading standard and the sibling
diagnostics locate squarely in the temporal container and the within-piece developmental machinery:
the one-gait-per-piece rhythmic flatness, the figuration arc that fires on a single image, the phrase
scaffold and form collapse. Those are structural dimensions the engine is missing or leaving starved,
not affect inputs it is failing to read. Per the standard's own conclusion: parameter tuning adds
resolution within a perceptual class; only architectural pluralization — a developing texture, a
within-piece rhythmic arc, plural forms and phrase schemes actually reached — adds *classes*, and only
adding classes moves the sameness verdict.

The affect-side arousal re-curve should be carried as a **secondary companion fix, not the
fundamental one.** It is cheap, it is balance-law-clean, and it removes a real defect — the high-energy
textures are currently unreachable by any natural photo because arousal never climbs high enough. Doing
it *alongside* the architectural fix means that when the downstream container finally develops and its
starved routes open, the input signal is also wide enough to drive them across their full range rather
than pinning five of six images to the low-arousal corner. But on its own it would spread starting
coordinates without curing the developmental flatness the ear is actually reporting, so it must not be
mistaken for the fundamental fix.

**One-line answer:** the affect knobs are turning (character 4/6 distinct, tempo 66–132, valence owns
mode) — affect-input collapse is a MINOR, arousal-axis-only contributor; the S67 fundamental fix belongs
architecture-side, with a small deterministic arousal re-curve as a companion to unlock the currently
unreachable high-energy textures.

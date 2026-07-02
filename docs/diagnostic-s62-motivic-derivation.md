# Diagnostic S62 — Cross-Piece Motivic Sameness (READ-ONLY)

**Owner:** Music Theory Specialist · **Scope:** `src/composition.rs` (motif seam) + `src/chord_engine.rs` (`resolve_motif_celled`, `MotifArchetype`) · **No fix this session.** `src/engine.rs` is FROZEN and is not touched.

**Operator report (across several images):** *"pretty much the same motif with minor variations for each piece."*

**Verdict (one line):** The perceived sameness is REAL, and its dominant origin is not the archetype selector (which spreads well — 5 of 8 contours across 6 images) but the fact that **the pitched-motif seam is DEAD on 4 of the 6 bundled images**: the theme gate `complexity >= 0.4` emits no `ThemeSeed` at all for typical photos, so those pieces carry no per-image melodic subject and fall back to the realizer's generic melodization; and on the low-complexity images that a motif *would* apply to, `length_steps = 3 + complexity·5` collapses it to a 2–3-note tonic/leap skeleton too short to read as a distinct tune.

Empirical harness: `tests/motif_diag_s62.rs` (`cargo test --release motif_diag_s62 -- --nocapture`). The reconstruction of the private derivation helpers is **cross-validated byte-for-byte against the real planner** on every image whose theme actually fires (`motif_diag_cross_validates_real_planner`, 2/6 fired, both matched).

---

## 1. The derivation chain — image feature → archetype → concrete motif

The motif is built ONCE per plan in `CompositionPlanner::plan` (`composition.rs:1504-1528`), gated by `theme_behaviour`, and resolved by the single public contour→`MotifNote` function `chord_engine::resolve_motif_celled`.

```
ImageUnderstanding
   │
   ├─ affect_composite (composition.rs:337)  ── weights from assets/mappings.json ──
   │     arousal  = 0.45·sat/100 + 0.25·colorfulness + 0.20·edge_activity + 0.10·complexity
   │     valence  = 0.70·bri/100 + 0.20·sat/100     + 0.10·(0.5+0.5·fg_bg_contrast)
   │        │
   │        ▼
   ├─ pick_archetype (composition.rs:1767)   ── DRIVERS: affect_arousal, affect_valence,
   │     (arousal≥.5 , valence≥.5) → 1 of 4 FAMILIES           vertical_emphasis, dominant_hue
   │     within family: (vertical_emphasis≥.5  OR  hue<180°) → active vs settling member
   │        │  → one of 8 MotifArchetype  (contour is a FIXED &[i8], chord_engine.rs:3313)
   │        ▼
   ├─ range_degrees = (2 + edge_activity·5).round()   → 2..=7   ── DRIVER: edge_activity ONLY
   ├─ length_steps  = (3 + complexity·5).round()      → 3..=8   ── DRIVER: complexity ONLY
   ├─ cell_index    = pick_rhythm_cell (composition.rs:1869)     ── DRIVERS: edge_activity, complexity
   │     complexity≥.66 → cell 3 ; edge<.33 → 1 ; edge<.66 → 0 ; else 2   (theme path only)
   │        ▼
   └─ resolve_motif_celled(archetype, range, length, cell)  (chord_engine.rs:3661)
         contour scaled by range/4, sampled once, each degree given the next cell duration,
         Σ dur_steps capped at length_steps.  → Vec<MotifNote{degree,dur_steps}>
```

**True degrees of freedom the image has over the melodic material:**

| DOF | Source feature(s) | Cardinality | Nature |
|---|---|---|---|
| Contour shape | arousal, valence, vertical_emphasis, hue | 8 archetypes | discrete, coarse buckets |
| Ambit (range) | `edge_activity` only | 6 values (2..7) | scales the SAME shape wider/narrower |
| Length | `complexity` only | 6 values (3..8 steps) | truncates/extends the SAME shape |
| Rhythm cell | edge_activity, complexity | 4 cells — **but only when a theme fires** | gait of the SAME shape |

So the pitch identity of a piece is: **one of 8 fixed contours, stretched by one scalar (edge) and truncated by another scalar (complexity).** Interval content, opening gesture, and starting degree are NOT independently image-derived — they are properties of the chosen fixed contour. That is precisely "one motif re-parameterized," exactly as the operator intuited.

---

## 2. Decision-space census (measured, `motif_diag_decision_space_census`)

- Distinguishable **contours** the pipeline can emit: **8**. Occupied across the 6 bundled images: **5** — `Arch, Ascent, Descent, InvertedArch, LeapStep`.
- Distinguishable **(contour, cell)** identities: 8×4 = **32** possible. Occupied: **5** — `(Arch,0) (Ascent,3) (Descent,3) (InvertedArch,0) (LeapStep,1)`.

**How coarse are the feature→archetype buckets?** Very. The load-bearing axis is two binary thresholds — `arousal ≥ 0.5` and `valence ≥ 0.5` — carving all images into 4 quadrants. The within-family member is chosen by `vertical_emphasis ≥ 0.5 OR hue < 180°` — an **OR that biases toward the "active" member** (both conditions must fail to get the settling member). For all six bundled images `vertical_emphasis < 0.5`, so the second bit is decided purely by warm/cool hue. Net: a huge swath of images collapses to the same archetype whenever they share an affect quadrant and hue half. The archetype selector is nonetheless the *healthiest* part of the chain here (5/8 occupied) — it is **not** the primary bottleneck.

---

## 3. Empirical per-image motif table (the evidence)

Feature drivers (from the real `understand_image_pure` pure-Rust path):

| image | edge_activity | complexity | sat | bri | arousal | valence |
|---|---|---|---|---|---|---|
| AudioHaxImg1.jpg | 0.301 | 0.005 | 32.9 | 29.3 | 0.211 | 0.338 |
| AudioHaxImg2.jpg | 0.509 | 0.015 | 30.1 | 81.1 | 0.259 | 0.692 |
| AudioHaxImg3.jpg | 0.475 | 0.229 | 37.2 | 65.7 | 0.391 | 0.594 |
| example.jpg | 0.719 | 0.905 | 64.5 | 49.9 | 0.696 | 0.535 |
| Lena.png | 0.471 | 0.164 | 51.9 | 70.5 | 0.375 | 0.650 |
| magicstudio-art.jpg | 0.106 | 1.000 | 43.6 | 45.3 | 0.389 | 0.459 |

Derivation + concrete motif (reconstructed; cross-validated against the real planner where the theme fires):

| image | hue | vEmp | archetype | range | length | cell | MOTIF (scale-degree / dur_steps) |
|---|---|---|---|---|---|---|---|
| AudioHaxImg1.jpg | 39.9 | 0.25 | **LeapStep** | 4 | 3 | 1 | `deg[0,4] dur[2,1]` |
| AudioHaxImg2.jpg | 36.6 | 0.36 | **Arch** | 5 | 3 | 0 | `deg[0,3] dur[2,1]` |
| AudioHaxImg3.jpg | 5.2 | 0.33 | **Arch** | 4 | 4 | 0 | `deg[0,2,4] dur[2,1,1]` |
| example.jpg | 198.6 | 0.33 | **Ascent** | 6 | 8 | 3 | `deg[0,2,3,5,6] dur[1,1,1,2,3]` |
| Lena.png | 354.1 | 0.36 | **InvertedArch** | 4 | 4 | 0 | `deg[4,2,0] dur[2,1,1]` |
| magicstudio-art.jpg | 278.3 | 0.38 | **Descent** | 3 | 8 | 3 | `deg[3,2,2,1,0] dur[2,1,1,1,3]` |

**The authoritative planner seam — which images actually get a pitched motif:**

| image | real `plan.themes` |
|---|---|
| AudioHaxImg1.jpg | **EMPTY** — complexity 0.005 < 0.40, no pitched motif emitted |
| AudioHaxImg2.jpg | **EMPTY** — complexity 0.015 < 0.40 |
| AudioHaxImg3.jpg | **EMPTY** — complexity 0.229 < 0.40 |
| example.jpg | FIRED → `deg[0,2,3,5,6] dur[1,1,1,2,3]` |
| Lena.png | **EMPTY** — complexity 0.164 < 0.40 |
| magicstudio-art.jpg | FIRED → `deg[3,2,2,1,0] dur[2,1,1,1,3]` |

**Read this table carefully — it is the whole diagnosis:**

1. **4 of 6 images emit NO pitched motif at all.** The theme gate `complexity >= 0.4` (`theme_behaviour` in `mappings.json`) filters out every ordinary photo. On those pieces there is no per-image subject; the melody the operator hears is the realizer's default melodization over the harmonic ladder, which is uniform by construction. *This is what "the same motif on every piece" actually is* — the absence of a motif, heard as generic sameness.
2. **When the seam DOES fire (2/6), the motifs are genuinely distinct** — `Ascent 0-2-3-5-6` vs `Descent 3-2-2-1-0` are different contours and gaits. The machinery works; it just almost never runs.
3. **The reconstructed motifs on the dead images are 2–3-note skeletons** — `deg[0,4]` (tonic→fifth), `deg[0,3]`, `deg[4,2,0]`. Even if the gate let them through, `length_steps = 3 + complexity·5` gives complexity≈0 images only 3–4 steps, i.e. 2–3 notes. At that length `LeapStep`, `Arch`, and `InvertedArch` are perceptually "a tonic plus one leap/step" — indistinguishable to the ear even though the archetype label differs.

---

## 4. Diagnosis — why cross-piece motifs collapse toward sameness (ranked by impact)

1. **[DOMINANT] The pitched-motif seam is gated off for typical images.** `theme_behaviour` requires `complexity >= 0.4`; 4/6 bundled images are below it, so `plan.themes` is empty and no motivic subject is imprinted. The listener then hears the realizer's generic per-step melodization on those pieces, which has no per-image identity and therefore sounds like the same tune everywhere. (Note the asymmetry with rhythm: S53's D-CELL slice already **un-gated the rhythmic motto** to run per-piece, but the **PITCH motif seam was never un-gated** — the pitch half of the motivic identity is still dead on real photos.)
2. **[HIGH] Length is complexity-driven and floors at 3 steps.** `length_steps = 3 + complexity·5` means low-complexity images (the majority) get 3–4-step, 2–3-note fragments. A 2-note "motif" cannot carry a recognizable shape; distinct archetypes collapse to the same tonic-skeleton percept.
3. **[MEDIUM] Only two continuous DOF, and both merely SCALE one of 8 fixed shapes.** Range (edge) and length (complexity) stretch/truncate a fixed contour; interval content, opening degree, and micro-gesture are not independently image-derived. Two images sharing an affect quadrant + hue-half get the *same shape* differing only in width — "minor variations of the same tune."
4. **[MEDIUM] The rhythm cell is pinned to 0 off the theme path.** `pick_rhythm_cell` runs only when a theme fires; every non-firing image would have gait cell 0 (the `[2,1,…]` long-tonic launch), so even the attack/gait is uniform. On the two firing images the cell does vary (both hit cell 3), which helps — but that reach is tiny.
5. **[LOW] The archetype's within-family bit is an OR-biased pseudo-binary.** `vertical_emphasis ≥ 0.5 OR hue < 180°` biases toward the active member; with vEmp < 0.5 on all six images the choice reduces to warm/cool hue. Still, contour spread is the healthiest link (5/8 occupied), so this is the least of the causes.

---

## 5. Fix direction (DESIGN ONLY — not implemented this session)

Smallest high-impact changes, in priority order. All stay strictly inside the **authored, voice-leading-legal** vocabulary (`contour()` + `rhythm_cells()`); none opens interval content to arbitrary values. **`src/engine.rs` is FROZEN — the entire motif seam lives in `src/composition.rs` and `src/chord_engine.rs`, both editable; do not touch `engine.rs`.**

1. **Un-gate the pitched motif so every image gets a subject — the single biggest win.** Mirror exactly what S53 did for the rhythmic motto: derive a per-piece `ThemeSeed` from robust per-image features **independent of the `complexity >= 0.4` theme gate**, so ordinary photos stop falling back to identity-less melodization. Music-theory rationale: a short piece earns its identity from a hummable subject that recurs; today only intricate images get one. Risk: keep it seeding the SAME authored 8-contour vocabulary — this is a routing change (who gets a motif), not a new-material change.
2. **Floor the motif length and decorrelate it from `complexity`.** Give every motif a minimum singable length (≈5–6 steps / 4–5 notes) regardless of complexity, and drive length from a feature INDEPENDENT of the one that drives range (e.g. `aspect_ratio` or `subject_size`) so length and ambit stop moving together. Rationale: a 4–5-note statement is the smallest unit that reads as a *shape* rather than an interval. Risk: none musical; just ensure Σ dur_steps still ≤ the section budget (the existing cap already guarantees this).
3. **Give each archetype a small set of authored PITCH variants selected by an independent feature — the pitch analogue of `rhythm_cells`.** Today `contour()` is one fixed `&[i8]` per archetype. Author 3–4 legal variants per archetype (different opening degree / one characteristic leap / neighbor-decoration) and select among them with an independent per-image feature hash (e.g. hue-bucket ⊕ brightness-bucket ⊕ mass-centroid quadrant), just as cell selection already picks a gait. Rationale: this is what broke the S39 rhythm weld; do the same for pitch so two images on the same archetype still open differently. Risk (the one to guard): every variant must be hand-authored and voice-leading-legal — do **not** synthesize intervals from a hash directly, which would degenerate into unsingable noise. The curated-shape container is the safety rail; the fix adds more curated shapes + an independent selector, it does not randomize.
4. **[Optional, low priority] Make the archetype's within-family bit two real bits.** Replace `vertical_emphasis ≥ 0.5 OR hue < 180°` with independent selectors (vEmp picks up/down, hue picks colorist member) so all 8 contours are reached more evenly. Low impact — contour spread is already the healthiest link.

**Net:** fixes 1 + 2 alone would give every image a real, full-length, distinct subject; fix 3 is what makes same-archetype images stop being scalings of one shape. Together they convert "one motif re-parameterized" into "one authored motif *per image identity*," without leaving the good-motif vocabulary.

# Diagnosis S13 — Image-Feature Side & Feature→Engine Dataflow

**Author role:** Rust Architect (DESIGN / DIAGNOSIS ONLY — no source modified)
**Date:** 2026-06-14
**Scope:** the IMAGE-FEATURE side (`src/pure_analysis.rs`) and the feature→engine dataflow
(`src/engine.rs` seam, `src/main.rs` adapter, `src/mapping_loader.rs` + `assets/mappings.json`).
A parallel Music Theory Specialist owns the music-mapping + engine-consumption side; this
document deliberately stops at the seam and only characterizes which features *reach* music.

**Central hypothesis under test:** whole-image-AVERAGE features regress diverse images to the
mean — visually different photos collapse to similar feature vectors, so even a perfect mapping
would yield similar music.

**Verdict (headline):** CONFIRMED, with an important refinement. avg_hue actually discriminates
*well* (it is the one feature with real spread, and it is the one feature that drives the most
audible musical dimension — the mode). The collapse is in the *other* two music-active scalars
(saturation and edge_density) plus a structural fact: **only three image scalars reach the music
at all, and four of the eight global features are computed-but-never-consumed.** The cheapest
high-leverage fix is on the image side and does **not** require touching the byte-frozen
`engine.rs` decision logic.

---

## 1. FEATURE INVENTORY

`pure_analysis.rs` produces two struct families that mirror `engine::GlobalFeatures` /
`engine::ScanBarFeatures` field-for-field (the boundary mirror, `engine.rs:33-74`).

### 1.A Whole-image scalars — `GlobalFeatures` (`analyze_global_pure`, `pure_analysis.rs:423-448`)

| Feature | How computed | Spatial nature | Range | Consumed downstream? |
|---|---|---|---|---|
| `avg_hue` | **Circular** mean of per-pixel HSV hue over *every* pixel; mean of unit vectors at each hue angle, `atan2`, normalized 0..360 (`hsv_means`, `pure_analysis.rs:168-208`, default `compat_arithmetic=false`) | whole-image scalar | 0..360 deg | **YES** — `set_features_global` → `hue_to_mode` (`engine.rs:330`). The single most audible driver (selects the mode). |
| `avg_saturation` | Arithmetic mean of per-pixel HSV saturation over every pixel (`pure_analysis.rs:193`) | whole-image scalar | 0..100 | **YES, but indirectly via the per-bar copy** — see §1.B. The *global* `avg_saturation` field itself is **not** read by the engine. |
| `avg_brightness` | Arithmetic mean of per-pixel HSV value (`pure_analysis.rs:194`) | whole-image scalar | 0..100 | **NO** (global field). `mappings.json` declares `brightness_to_tempo_bpm` and `modal_interchange_trigger.brightness_drop_threshold`, but **neither is wired**: `set_features_global` hardcodes `brightness_drop = 0.0` (`engine.rs:343`) and tempo is `ms_per_step`, a CLI constant (see §1.C). |
| `edge_density` | imageproc Canny (50/150 hysteresis) on Rec.601 gray; non-zero / total pixels (`edge_density_pure`, `pure_analysis.rs:309-319`) | whole-image scalar | 0..1 | **YES** — passed as `edge_complexity` to `generate_chords` (`engine.rs:343`), gating secondary-dominant insertion at threshold **0.7** (`mappings.json:23`). With real photos this is *never* crossed (see §2). |
| `hue_spread` | Circular stddev of hue, `sqrt(-2 ln R)`, rescaled to OpenCV's `/90` heuristic (`hue_spread_pure`, `pure_analysis.rs:221-244`) | whole-image scalar | ~0..1 | **NO** — computed, mirrored, never read by any decision. |
| `texture_laplacian_var` | Population variance of a hand-rolled f64 3×3 Laplacian over gray (`laplacian_var_pure`, `pure_analysis.rs:333-386`) | whole-image scalar | 0..~unbounded (saw 327–1958) | **NO** — `texture_to_modal_color` mapping exists in JSON but is never consulted. |
| `shape_complexity` | Otsu threshold → 8-conn connected-component count / 1000 (`shape_complexity_pure`, `pure_analysis.rs:399-415`) | whole-image scalar | 0..~unbounded (saw 0.011–2.005) | **NO** — `shape_to_ostinato` mapping exists in JSON but is never consulted. |
| `aspect_ratio` | `w / h` (`pure_analysis.rs:436`) | whole-image scalar | >0 | **NO** — never read by a decision. |

### 1.B Per-scan-bar features — `ScanBarFeatures` (`analyze_section_pure`, `pure_analysis.rs:459-486`)

Computed per section of a scan bar (the geometry mirror of `image_analysis::scan_image`,
`pure_analysis.rs:550-664`). For each scan step `s` and instrument `i`, a sub-rectangle of the
image is cropped and analyzed:

| Field | How computed (per section) | Range | Consumed downstream? |
|---|---|---|---|
| `bar_index` | section index 0..num_instruments | usize | structural only |
| `avg_hue` | circular hue mean of the section | 0..360 | **NO** (the engine's per-step decision reads only sat/bright/edge — see `engine.rs:555-559`) |
| `avg_saturation` | arithmetic sat mean of the section | 0..100 | **YES** → `PerfFeatures.saturation` → velocity *level* (`chord_engine.rs:786-787`, ±velocity from saturation) |
| `avg_brightness` | arithmetic value mean of the section | 0..100 | **YES** → `PerfFeatures.brightness` (carried, but `realize_velocity`/`realize_rhythm` read saturation + edge_density; brightness's only consumer is the snapshot projection) |
| `edge_density` | section Canny density | 0..1 | **YES** → `PerfFeatures.edge_density` → articulation + rhythm pattern (`chord_engine.rs:863-882`) |
| `texture_laplacian_var` | section Laplacian var | 0..unbounded | **NO** — mirrored, never projected into `PerfFeatures` |
| `hue_hist` | normalized 8-bin hue histogram (`hue_histogram_pure`, `pure_analysis.rs:252-276`) | sums to 1 | **NO** — explicitly "unused by the music decision" (`engine.rs:72`) |

**The seam projection is the choke point.** `decide_instrument_action` (`engine.rs:555-559`)
builds `PerfFeatures { saturation, brightness, edge_density }` from each bar — **three scalars**.
Everything else in `ScanBarFeatures` is discarded at the seam. And of those three,
`chord_engine` reads saturation (velocity) and edge_density (rhythm/articulation); brightness is
carried but inert in the note decision.

### 1.C Is there a TEMPO / step-duration field in the feature path?

**No.** Tempo is **fixed, image-independent**. Step duration = `EngineConfig.ms_per_step`
(`engine.rs:179`), set from the CLI (`cli.rs:84,168`), default 250 ms (`engine.rs:194`). It is
passed to `realize_step`/`realize_rhythm` (`chord_engine.rs:858,864`) and never derived from any
image feature. `mappings.json:16-20` declares `brightness_to_tempo_bpm`, but grep shows **zero**
consumers of `brightness_to_tempo_bpm` anywhere in `chord_engine.rs`/`engine.rs`/`cli.rs`. So:
- Every image plays at the same tempo.
- The number of steps (`--steps`, default 40) is also a constant, not image-derived.

This is a major part of why "every image has the same feel": **tempo and step count — two of the
most viscerally identity-defining musical dimensions — are constant across all images.**

### 1.D Computed-but-never-consumed (the dead-feature list)

From the global struct: `avg_brightness`, `hue_spread`, `texture_laplacian_var`,
`shape_complexity`, `aspect_ratio` are computed and mirrored across the seam but read by **no**
decision. From the per-bar struct: `avg_hue`, `texture_laplacian_var`, `hue_hist` are dead. The
mapping file advertises rich behavior (`saturation_to_harmonic_complexity`,
`brightness_to_tempo_bpm`, `texture_to_modal_color`, `color_shift_to_chord_extension`,
`shape_to_ostinato`, `line_orientation_to_interval`, `contrast_to_articulation`) — **almost none
of it is wired**. The live wiring is: hue→mode, edge_density(global)→secondary-dominant gate,
per-bar saturation→velocity, per-bar edge_density→rhythm/articulation.

---

## 2. REGRESSION-TO-THE-MEAN VERDICT

**Empirically tested.** The `analyze` subcommand is not wired (`main.rs:246-249`), so I drove
the default pure-Rust `play` path on all six in-repo images and captured the printed
`Global features:` line (`main.rs:370`) before playback. Results:

| Image | avg_hue | mode | avg_sat | avg_bright | edge_density | hue_spread | tex_var | shape | aspect |
|---|---|---|---|---|---|---|---|---|---|
| AudioHaxImg1.jpg | 39.9 | Lydian | 32.9 | 29.3 | **0.015** | 0.011 | 1230 | 0.011 | 0.67 |
| AudioHaxImg2.jpg | 36.6 | Lydian | 30.1 | 81.1 | **0.025** | 0.080 | 1548 | 0.031 | 0.67 |
| AudioHaxImg3.jpg | 5.2 | Phrygian | 37.2 | 65.7 | **0.024** | 0.423 | 1100 | 0.458 | 1.5 |
| example.jpg | 198.6 | Dorian | 64.5 | 49.9 | **0.036** | 0.685 | 1958 | 1.81 | 2.05 |
| Lena.png | 354.1 | Ionian | 51.9 | 70.5 | **0.024** | 0.122 | 387 | 0.329 | 1.0 |
| magicstudio-art.jpg | 278.3 | Mixolydian | 43.6 | 45.3 | **0.005** | 0.287 | 328 | 2.005 | 1.0 |

**The operator's exact run is reproduced:** `magicstudio-art.jpg` → avg_hue 278.3 → Mixolydian,
edge_density 0.005. The operator's report was not a fluke; it is the analyzer's actual output.

**What the data shows:**

1. **avg_hue is the *good* feature.** It spans 5–354 and lands the six images in five
   different modes (Lydian, Phrygian, Dorian, Ionian, Mixolydian). Hue does NOT regress to the
   mean — and notably the circular mean (S11 IMAGEPROC PORT FIDELITY decision; `pure_analysis.rs:158-208`)
   is *correct* here: it avoids the red-wrap bug that would pull near-red images to cyan-180.
   The mode genuinely varies per image. (This matches the operator: "mode differs.")

2. **edge_density IS miscalibrated/saturated-low.** All six images fall in **0.005–0.036**, a
   ~7× spread but in absolute terms hugging zero. The two music-active edge thresholds are:
   - secondary-dominant gate at **0.7** (`mappings.json:23`) — *never* reached by any real photo;
   - articulation buckets `edge < 0.25 → legato`, `edge > 0.70 → staccato`, else portato
     (`chord_engine.rs:879-881`). **Every real image is < 0.25 → every image is legato.**
   So edge_density contributes **zero** musical variation across these images even though it has
   7× numeric spread — the thresholds were calibrated for a 0..1 feature whose realistic range is
   0..0.04. This is a genuine miscalibration: Canny edge *density* (fraction of edge pixels) on
   natural photos is intrinsically small; 0.005 is typical, not a bug. The mapping thresholds
   assume a feature that ranges much higher.

3. **saturation regresses to a tight band.** 30–65 on a 0..100 scale — the middle. Via
   `realize_velocity` (`chord_engine.rs:786-787`) this maps to a velocity level-gain of roughly
   −12 + (sat/100)·30, i.e. **−3 to +7.5 velocity** — a ~10-velocity window out of 127. Audibly
   nearly identical loudness across all images. (Note: the engine reads *per-bar* saturation, not
   the global; per-bar bands are similar.)

4. **The genuinely-spread features are dead.** `hue_spread` (0.01–0.69), `texture_laplacian_var`
   (328–1958, ~6× spread), `shape_complexity` (0.011–2.005, ~180× spread!), and `aspect_ratio`
   (0.67–2.05) all carry strong per-image signal — and **none of them reach the music** (§1.D).
   shape_complexity in particular discriminates images almost perfectly and is thrown away.

**Conclusion.** The regression-to-the-mean hypothesis is **half right and more precisely
diagnosable than stated**. It is not that *all* features collapse — hue is fine and is why mode
varies. The "same feel" comes from:
- **(a)** the only other two music-active scalars (saturation, edge_density) BOTH sit in a narrow
  central band for natural photos and BOTH fall on one side of every musical threshold, so they
  add ~no variation;
- **(b)** tempo and step-count, the most feel-defining dimensions, are **constant by construction**
  (§1.C); and
- **(c)** the four features that *do* discriminate strongly (shape_complexity, texture var,
  hue_spread, aspect) are computed and then discarded.

Whole-image averaging is a contributor to (a) — a single mean over a multi-colored photo
genuinely washes saturation toward the middle — but the dominant cause is **mapping/seam
under-utilization, not the averaging per se.** A multi-colored image's *hue* mean can still be
distinctive (the circular mean preserves the dominant direction); its *saturation/brightness*
means are what flatten.

---

## 3. DISCRIMINATION FIX (image side — proposals for `pure_analysis.rs`)

These are proposals to hand to a Rust Implementer. They keep image analysis free of music logic
(module boundary respected). Ordered cheapest-first.

### 3.1 Re-derive/expose features so the discriminating signal reaches music (cheapest)

The single biggest leverage is not new pixel math — it is **exposing features that already
discriminate** and giving them a music-active range. Concretely, on the image side:

- **Dominant hue instead of (or alongside) the mean hue.** A multi-color image's circular hue
  mean can drift to a hue that *no pixel actually has*. Compute the hue histogram (already exists,
  `hue_histogram_pure`) over more bins (e.g. 12–24) at the whole-image level and expose:
  - `dominant_hue` (0..360): the bin center of the argmax bin, mass-weighted within the bin;
  - `hue_bimodality`/`secondary_hue`: the second-largest bin, to distinguish a two-color image
    from a monochrome one. This is the operator's "subject vs background color" intuition in cheap
    form. Range: 0..360 each; a `dominant_hue_mass` 0..1 says how peaked the palette is.

- **Re-normalize edge_density to a usable range.** Canny density on photos is ~0.005–0.04. Either
  (i) expose a **rescaled** `edge_activity = clamp(edge_density / 0.05, 0, 1)` so the 0..1 range is
  actually populated, or (ii) expose the *raw* density and let the mapping thresholds be retuned
  (a Music-side change). The image-side cheap move is to add `edge_activity` (normalized 0..1)
  next to the raw value so downstream gets a feature that spans its threshold space. Across the
  six images this would yield 0.10, 0.51, 0.48, 0.72, 0.47, 0.11 — real spread that would actually
  cross articulation buckets.

- **Expose a spread feature to music.** `hue_spread`, `shape_complexity`, and
  `texture_laplacian_var` already discriminate; the image side simply needs to make them
  music-active candidates with sane normalized ranges:
  - `texture` = `clamp(texture_laplacian_var / 2000.0, 0, 1)` → 0..1 (saw 0.16–0.98 across the set);
  - `complexity` = `clamp(shape_complexity / 2.0, 0, 1)` → 0..1 (saw 0.006–1.0; superb spread);
  - `colorfulness` = `hue_spread` already 0..1 (saw 0.01–0.69).

These give a downstream consumer **five well-spread 0..1 knobs** (dominant_hue/hue spread, edge
activity, texture, complexity, colorfulness) instead of two clustered ones.

### 3.2 Richer per-region reading (medium — see §5 for the larger saliency variant)

Replace/augment the flat scalar means with a coarse **3-region split** (foreground center vs
border/background, plus a detail/high-frequency region) — cheaper than full saliency but captures
the operator's "subject vs background" intuition:

- `center_saturation` / `border_saturation`: mean saturation of the central 1/3-box vs the
  border ring. Their *difference* is a strong subject-pop signal that a whole-image mean destroys.
- `center_hue` vs `border_hue`: subject color vs background color (drives a melody/accompaniment
  color split).
- `detail_density`: edge density of the high-frequency (top-Laplacian) region only — separates
  "busy subject on plain background" from "uniformly busy" even when whole-image edge density is
  identical.

Definitions/ranges mirror the existing scalars (sat/hue/edge), just computed over sub-rectangles
the analyzer already knows how to crop (`crop_imm`, used at `pure_analysis.rs:656`).

---

## 4. DATAFLOW / SEAM ASSESSMENT

**Does the current seam carry enough per-image signal to drive more musical dimensions?**
Partially. The seam (`FeatureSource` → `GlobalFeatures`/`ScanBarFeatures`, `engine.rs:84-96`)
*already carries* eight global scalars and seven per-bar fields — four of which are discarded
unused (§1.D). So **a large amount of new musical variation can be unlocked with NO seam change
at all**, purely by the Music specialist wiring the *already-present* fields
(`hue_spread`, `texture_laplacian_var`, `shape_complexity`, `aspect_ratio`,
per-bar `avg_hue`/`texture_laplacian_var`/`hue_hist`) into decisions. The data shows these
fields discriminate strongly.

**Does anything force an `engine.rs` edit?** `engine.rs` has been byte-frozen S9–S12. Two cases:

- **No-edit path (preferred).** If new image features are added to existing struct fields'
  *meaning* (e.g. the image side fills the already-present `texture_laplacian_var` /
  `shape_complexity` / `hue_spread` fields, which it already does), then the only changes needed
  to make them musical are in **`chord_engine.rs` / `mappings.json`** (Music-side) plus the
  projection in `decide_instrument_action` (`engine.rs:555-559`). The projection currently copies
  only sat/bright/edge into `PerfFeatures`. **Wiring a fourth field into the music requires either
  widening `PerfFeatures` (a `chord_engine.rs` change, not engine.rs) and adding one line to the
  projection (engine.rs:555-559), OR routing it through the global path** (`set_features_global`
  already reads `avg_hue` and `edge_density` from `GlobalFeatures` at `engine.rs:330,343`; it could
  read more global fields there with no struct change).

  → **Cleanest no-struct-change win:** `set_features_global` (`engine.rs:328-357`) already has the
  whole `GlobalFeatures` in hand and already reads `avg_hue` and `edge_density` from it. It
  currently hardcodes `brightness_drop = 0.0` (`engine.rs:343`). Passing `global.avg_brightness`
  (or a derived drop) there, and deriving `ms_per_step`/progression-family weighting from
  `avg_brightness`/`shape_complexity`, would add per-image variation **using only fields that
  already cross the seam**. This is still an `engine.rs` line edit, so it must be flagged (below),
  but it adds **no new field** to the mirror structs.

- **Edit-forcing path.** If a NEW image feature must reach music as its own scalar (e.g.
  `dominant_hue`, `edge_activity`, `center_vs_border_saturation`), it must be added as a field to
  `GlobalFeatures`/`ScanBarFeatures` (`engine.rs:33-74`) AND its `pure_analysis.rs` mirror AND the
  OpenCV mirror copy (`main.rs:147-172`). That is a seam evolution.

**SEAM-EVOLUTION DECISION (explicit lead call required).** `engine.rs` is byte-frozen S9–S12 and
its decision *logic* (`decide_instrument_action`, `tick`, `decide_step`) is the
regression-equivalence anchor. Two distinct things can change, with very different risk:
- **Adding a field to the mirror structs** `GlobalFeatures`/`ScanBarFeatures` is a low-risk,
  additive seam change (the structs are plain data; adding a field does not alter decision logic,
  only the boundary copy must be kept in sync — the S9 §6 risk-2 the comments already flag).
- **Changing `decide_instrument_action` / the `PerfFeatures` projection** touches the frozen
  decision kernel and breaks the equivalence anchor — a real, higher-risk decision.

**Trade-off / recommendation for the seam:** the cheapest high-leverage change (§3.1 +
Music-side wiring) can be done **without adding any struct field** by consuming the
already-present-but-dead global fields inside `set_features_global` (which is not part of the
frozen decision kernel — it is the plan-derivation path, and is already non-deterministic via
`thread_rng`). That avoids both a struct change and a `decide_instrument_action` edit. The new
*image-side* features in §3.1 (dominant_hue, edge_activity) are nicer but require the additive
struct change; recommend doing the no-struct-change wins first and only evolving the seam once
the Music specialist confirms which new scalars actually earn their place.

---

## 5. SCAN-VS-SALIENCY

**Operator's design-history insight:** the original was a strip/chunk-per-instrument scan (a flat
left-to-right or top-to-bottom moving bar — exactly what `scan_steps` still does,
`pure_analysis.rs:550-664`). Humans don't read an image as a flat scan; they read SUBJECT →
foreground/background/detail. He wants a saliency/region reading.

**Is it feasible in pure-Rust (`image`/`imageproc`, no OpenCV)?** Yes, with caveats — `imageproc`
gives most of the primitives, but there is no turnkey saliency model.

- **Cheap region reading (recommended first step toward saliency):** the 3-region center/border/
  detail split of §3.2. Pure crop math (already used) + the existing HSV/edge/Laplacian kernels
  over sub-rects. Cost: ~3× the per-region passes the scan already does; trivial. Captures
  "subject vs background" coarsely with zero new dependencies. **This is the cheap proxy for the
  operator's intuition.**

- **True saliency (larger change):** a classic spectral-residual or center-surround saliency map
  is implementable in pure Rust:
  - center-surround contrast: difference-of-Gaussians on the Lab/gray channels (imageproc has
    `gaussian_blur_f32`), thresholded to a saliency mask; the masked region defines "subject."
  - or a coarse Itti-Koch-style combination of color/intensity/orientation contrast maps.
  - Then derive `subject_hue`, `subject_saturation`, `subject_size` (mask area fraction),
    `background_hue`, and a foreground/background `color_contrast` — a far richer, more
    human-aligned feature set.
  Cost: real implementation effort (a saliency module + tuning), some compute per image, and new
  feature definitions that DO force the additive seam change of §4. Accuracy without a learned
  model is modest but meaningfully better than a flat scan. No OpenCV needed; `image` +
  `imageproc` (gaussian, sobel, otsu, connected-components — all already in use) suffice.

**Assessment:** the saliency/region reading is the *right long-term direction* and matches how
the operator hears images, but it is a **larger change** (new module, new features, additive seam
evolution, tuning) than the §3.1 dominant-hue/expose-more-features fix. Sequence it second.

---

## 6. RECOMMENDATION (ordered by leverage-per-effort)

1. **[Cheapest, highest leverage — image side + Music wiring, NO seam struct change] Wire the
   already-discriminating, currently-dead features into music.** shape_complexity (180× spread!),
   hue_spread, texture_laplacian_var, and avg_brightness all cross the seam *today* and are
   discarded. Have the Music specialist consume them (e.g. brightness→tempo via the declared-but-
   dead `brightness_to_tempo_bpm`; complexity/texture→harmonic complexity or rhythmic density),
   and on the image side add normalized 0..1 mirrors (`texture`, `complexity`, `colorfulness`,
   `edge_activity`) so the values land in usable ranges. The one engine.rs touch (replacing the
   hardcoded `brightness_drop = 0.0` at `engine.rs:343` and/or deriving tempo in
   `set_features_global`) is in the **plan-derivation path, not the frozen decision kernel** —
   flag it but it does not break the equivalence anchor.

2. **[Cheap, image side] Add `dominant_hue` + `edge_activity` (re-normalized) features.** Use the
   existing multi-bin hue histogram for a true dominant hue (more faithful than the circular mean
   for multi-color images) and rescale edge_density into a populated 0..1 band. This DOES require
   the additive `GlobalFeatures`/`ScanBarFeatures` field add (low-risk seam evolution — additive,
   no decision-logic change) plus the boundary-copy sync (`main.rs:147-172`).

3. **[Medium, image side] 3-region center/border/detail split (§3.2).** Cheap proxy for the
   operator's subject→fg/bg/detail intuition; reuses existing crop + kernels; exposes subject-vs-
   background contrast features that whole-image means destroy. Additive seam change.

4. **[Larger, follow-on] True saliency/region reading (§5).** Pure-Rust DoG/spectral-residual
   saliency → subject mask → subject/background feature pairs. The right long-term match to how
   the operator hears images; defer until 1–3 prove the wiring and the seam appetite.

**Also flag to the Music specialist (their domain, surfaced here because it bounds image-side
leverage):** even a perfectly discriminating feature vector will still "feel the same" while
**tempo and step-count are constant** (§1.C) and while edge/saturation thresholds
(`mappings.json:23`, `chord_engine.rs:879-881`) are calibrated for ranges real photos never reach.
The image-side fix and the mapping-recalibration must land together.

---

### Module-boundary note

All proposals keep `pure_analysis.rs` free of music types (it names only `image`/`imageproc` and
the `engine::*` mirror structs it already implements) and keep `engine.rs` free of image types.
The seam-evolution items are explicitly called out as lead decisions, distinguishing the low-risk
additive struct change from the high-risk frozen-decision-kernel change.

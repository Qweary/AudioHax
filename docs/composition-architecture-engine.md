# Composition Architecture — Engine, Image-Understanding Feasibility & Roadmap

**Author role:** Rust Architect (DESIGN ONLY — no source modified by this document; `docs/` only).
**Date:** 2026-06-14
**Phase:** 1 of 2. This is the ENGINE / FEASIBILITY / ROADMAP half of the canonical
composition-architecture assessment. A Music Theory Specialist is authoring the MUSICAL half
(section A: forms × characters × structural devices, meter, structural key/tempo plan, returning
themes, morphing harmony, and the musical contract of an up-front `CompositionPlan`) in parallel.
**Phase 2** merges the two into the canonical assessment; the reconciliation points are enumerated
in §C.7 and §F.

**Grounded against** the working tree at `799a86d` (S13): `src/engine.rs` (1059 lines),
`src/chord_engine.rs` (3183 lines), `src/pure_analysis.rs` (1037 lines), `src/mapping_loader.rs`,
`assets/mappings.json`, and the prior design docs `design-s9-engine-seam-cli.md`,
`design-s13-diversity.md`, `diagnosis-s13-image-dataflow.md`, `diagnosis-s13-music-side.md`.

> Convention (carried from S9/S13): Rust signatures give the SHAPE of each seam. **No
> implementation bodies are written.** Web-grounded tooling facts in §B.2 cite live URLs;
> figures tagged **[est.]** are extrapolated, not read off a page. Where a field's musical
> *contents* belong to the Music Theory section, this doc defines the STRUCTURAL shape and marks
> the reconciliation point **[MERGE-§…]**.

---

## 0. Executive summary (read first)

**The verdict that opened this arc.** AudioHax has strong bottom-up note-level craft (S2–S13:
modes, voice leading, expressivity) and, since S13, genuine per-image *diversity* (tempo,
harmony, articulation, rhythm, mixture vary with image features). The operator listened through a
real engine: the diversity is REAL but the output is **ethereal, structureless, and unrelated to
the image** — it only "works" for abstract art. The missing thing is the **top-down architecture
of a piece**: there is no macro-form, no genre/character, no time signature, no structural
key/tempo plan, no returning themes, no morphing harmony, and no image-as-a-whole understanding to
drive any of it.

**The mechanism of the defect, located in code.** The engine SONIFIES A SCAN. `pure_analysis.rs`
reduces the whole image to eight whole-image *average* scalars (`analyze_global_pure`,
`pure_analysis.rs:423`) plus a flat left-to-right sequence of per-bar averages
(`scan_steps`, `pure_analysis.rs:550`). The engine emits a uniform stream: `PipelineEngine::tick`
(`engine.rs:405`) walks `step_index` 0→N and calls the per-step realizer
`decide_instrument_action` (`engine.rs:562`) → `chord_engine::realize_step`
(`chord_engine.rs:891`) once per bar. **No object above the per-step realizer plans the piece.**
`plan_phrases` (`chord_engine.rs:631`) groups steps into 4/8-step phrases with cadences at
boundaries — that is the *only* macro-structure that exists, and it is harmonic phrasing, not
form. There is confirmed **no meter, no key-change, no section, no theme memory** anywhere in the
tree (Explore audit of `chord_engine.rs`, item 15).

**The paradigm shift this doc designs.** From *scan sonifier* → *image-conditioned COMPOSER*: read
the image as a whole, derive a structural **CompositionPlan** up-front (form, character, meter,
key/tempo scheme, theme seeds, section list), and render a piece whose per-step realization is
DRIVEN BY that plan rather than by a bare left-to-right scan. The existing `chord_engine` craft
becomes the per-step realizer the planner drives — it is preserved, not replaced.

**The three load-bearing decisions in this doc:**

1. **Image understanding (§B): heuristic-first, semantic later and gated.** A pure-Rust heuristic
   composition layer (palette / composition-balance / region-saliency / complexity / energy) is
   buildable NOW by extending `pure_analysis.rs`, needs zero new dependency, and already has 4+
   strongly-discriminating-but-dead features sitting on the seam (S13 diagnosis §1.D). Semantic
   subject/scene recognition is a real capability leap; the web-grounded assessment (§B.2) is that
   **"pure-Rust semantic vision" is not cleanly achievable** — the lightest honest local path
   (`candle` CPU-only or `tract` + MobileNetV3-Small) still ships/downloads a model and, for
   `tract`, assembles SIMD kernels via `cc`. **Recommendation: ship heuristic features as the
   composer's whole-image understanding; defer semantic recognition to an optional, feature-gated,
   later layer that the planner consumes through the same neutral `ImageUnderstanding` struct.**

2. **Engine re-architecture (§C): a planner ABOVE the realizer, not a rewrite of it.** Introduce a
   `CompositionPlanner` that computes a `CompositionPlan` once, up-front, from an
   `ImageUnderstanding`. `PipelineEngine` gains a `plan: CompositionPlan` and threads it into the
   per-step realizer. **What stays:** the `chord_engine` craft, S13 expressivity, and the
   `FeatureSource`/`AudioSink`/`EngineObserver` seam — all of it. **What's new:** the planner, the
   `CompositionPlan`/`Section`/`ThemeSeed` types, and a `decide_instrument_action` that takes a
   plan/section context. The S9 byte-freeze and `engine_equivalence` golden are migrated by a
   **back-compat default plan** that reproduces today's behavior bit-for-bit when no planner runs
   (§C.6).

3. **Vocabulary (§3 of the scoping counterpoint): small, principled, coherent — not comprehensive.** The
   plan's enums (`Form`, `Character`, `Meter`) are a CURATED handful tied to robust image
   properties, chosen with the Music Theory section, NOT an open-ended config surface.

**Preliminary first BUILD slice (finalized in Phase 2, §G):** the **pure-Rust structural skeleton**
— `ImageUnderstanding` (heuristic, wiring the dead features) + a minimal `CompositionPlanner` that
emits a `CompositionPlan` with **sections and a structural key/tempo plan** (no themes yet, no
semantic recognition) + the engine threading + the back-compat default plan that keeps the golden
green. This makes *audible structure* (an image that opens, develops, and closes in distinct
sections at section-stable tempo/key) hearable with zero new dependency. The S13 articulation
clamp (§E.0) rides along as a cheap fix in the same slice.

---

# Part 1 — Current-state analysis

## A.1 The data path today (file:line)

```
pure_analysis.rs                         engine.rs                         chord_engine.rs
─────────────────                        ─────────                         ───────────────
analyze_global_pure  ──GlobalFeatures──► set_features_global  ──────────►  pick_progression (thread_rng)
  (8 avg scalars)        (:423)             (:328)                            (:115)  Vec<String>
                                              │                             generate_chords (:166) Vec<Chord>
                                              │ derives mode (hue→mode)      voice_lead_sequence (:533)
                                              │ S13 tempo (brightness→bpm)   plan_phrases (:631) Vec<StepPlan>
                                              ▼                                 │ (4/8-step phrases, cadence@bound)
                                            self.plan: Vec<StepPlan>  ◄─────────┘
scan_steps           ──Vec<Vec<Scan─────► tick (:405)  step 0→N
  (:550, flat L→R)       BarFeatures>>      └► decide_instrument_action (:562)  per bar, per instrument
                                              └► realize_step (:891) ──► Vec<NoteEvent>
                                                   instrument_role (:846) Bass/HarmonicFill/Melody
                                                   role_pitch / realize_velocity / realize_rhythm (:1116)
```

The whole pipeline is **per-step**. The only object that sees more than one step is `plan_phrases`,
and it sees a flat `&[Chord]`, not the image. There is **no `Piece`, no `Section`, no `Form`** — the
top of the structure tree is a `Vec<StepPlan>` whose length is `--steps` (default 40) and whose only
grouping is the phrase. (Explore audit, `chord_engine.rs` item 15: no meter, no key-change, no
section, no theme — confirmed.)

## A.2 The frozen kernel and its equivalence net

- `decide_instrument_action` (`engine.rs:562`) was lifted verbatim from `main.rs::worker_decide_action`
  at S9 and has been **byte-frozen** since (S13 broke the freeze only for the `set_features_global`
  tempo/modal-interchange edits, which are the plan-derivation path, not the kernel).
- `tests/engine_equivalence.rs` pins `decide_instrument_action` / `realize_step` / `realize_rhythm`
  against a **fixed `&[StepPlan]`** with hand-derived golden constants (cadence hold 240 ms;
  velocities 114/84; register `G_BASS_NOTE=36` / `G_MELODY_NOTE=79`). **Any musical-output change on
  that fixed plan trips the net on purpose.** The re-architecture's migration path (§C.6) must keep
  this net green except where a golden is deliberately, reviewably re-derived.

## A.3 The image-understanding deficit (from the S13 diagnoses)

The S13 image diagnosis (`diagnosis-s13-image-dataflow.md`) measured all six in-repo images and
established:

- **`avg_hue` discriminates well** (5°–354°, lands 6 images in 5 modes); it is the one good feature.
- **`edge_density` is saturated-low** (0.005–0.036) and below every musical threshold for real photos.
- **`saturation` regresses to a tight central band** (30–65).
- **Four globally-spread features are computed and DISCARDED:** `hue_spread` (0.01–0.69),
  `texture_laplacian_var` (328–1958), `shape_complexity` (0.011–2.005, **180× spread**),
  `aspect_ratio` (0.67–2.05). Plus per-bar `texture_laplacian_var` and the 8-bin `hue_hist`.
- **Tempo and step-count were constant by construction** until S13 wired brightness→tempo.

The diagnosis's own recommendation §5 ("scan vs saliency") flags the **subject→foreground/background/
detail region reading** as the right long-term direction and a larger rebuild. **This arc subsumes
that rebuild**: the heuristic `ImageUnderstanding` layer (§B.1) is exactly the region/saliency reading
that diagnosis recommended sequencing second.

## A.4 What `pure_analysis.rs` already gives us to build on

`analyze_global_pure` (`:423`) → `GlobalFeatures` {avg_hue, avg_saturation, avg_brightness,
edge_density, hue_spread, texture_laplacian_var, shape_complexity, aspect_ratio} — whole-image
averages only, **no region/quadrant/saliency analysis exists** (Explore audit, `pure_analysis.rs`
item 4). Primitives already in the tree and reusable for region work:

- RGB→HSV (`rgb_to_hsv`, `:121`), circular hue mean (`:168`), 8-bin hue histogram (`hue_histogram_pure`, `:252`).
- Canny edges (`imageproc::edges::canny`, `:316`), hand-rolled 3×3 Laplacian variance (`:333`),
  Otsu threshold + 8-connected components (`imageproc::region_labelling::connected_components`, `:410`).
- Crop math (`crop_imm`) already used by `scan_steps` (`:656`).

**Crucially: the dependencies needed for the heuristic layer — `image` and `imageproc` — are already
in the tree.** `imageproc` provides `gaussian_blur_f32`, sobel, otsu, connected-components — the full
toolkit for a center-surround / difference-of-Gaussians saliency map with **zero new crate**.

---

# Part B — Image-understanding layer (feasibility of the fork)

The composer needs to understand the image **as a whole** to derive a plan. There are two kinds of
"understanding," with very different cost:

- **Heuristic** — perceptual composition properties: palette, balance, region-saliency, complexity,
  energy. *Pure-Rust, doable now.*
- **Semantic** — what the image *is*: objects, scene, faces → literal subject-matching. *A
  capability + dependency leap.*

## B.1 Heuristic side — pure-Rust, buildable now

All of the following extend `pure_analysis.rs` using `image`/`imageproc` (already in the tree) and
respect the module boundary (no music logic in image analysis). They flow to the planner through the
new neutral `ImageUnderstanding` struct (§C.2), which is a *richer sibling* of `GlobalFeatures` — it
is computed once per image, whole-image, and is the planner's input.

### B.1.a Features that are FREE today (already computed, currently dead)

Zero new pixel math — these already cross the seam on `GlobalFeatures` and are discarded (A.3):
`hue_spread`, `texture_laplacian_var`, `shape_complexity`, `aspect_ratio`. Normalized to 0..1 knobs
(the S13 calibration: `texture = clamp(var/2000,0,1)`, `complexity = clamp(shape/2,0,1)`,
`colorfulness = hue_spread`), they give the planner **four well-spread whole-image knobs** for free.
**Effort: trivial** (re-expose; the values exist).

### B.1.b Palette features (cheap; new but small)

- `dominant_hue` (0..360) + `dominant_hue_mass` (0..1): argmax of a wider (12–24 bin) whole-image hue
  histogram — extends the existing `hue_histogram_pure`. A multi-color image's circular *mean* hue can
  land on a hue no pixel has; the dominant bin is the image's *characteristic* color.
- `secondary_hue` + `palette_bimodality`: second-largest bin → distinguishes a two-color image
  (subject vs background) from a monochrome one. The "subject vs background color" intuition, cheaply.
- `value_key` (0..1): low-key/high-key from the brightness histogram shape, not just its mean — a
  dark-dominant image with bright highlights reads differently from a uniformly mid image.

**Effort: low** (histogram post-processing on data the analyzer already produces).

### B.1.c Composition-balance features (cheap; pure crop math)

- `mass_centroid` (cx, cy in 0..1): luminance-weighted center of "stuff" — where the visual weight
  sits. Off-center → asymmetric/dynamic; centered → stable/symmetric.
- `horizon_split` / `vertical_emphasis`: ratio of upper-vs-lower and left-vs-right edge+saturation
  mass — landscape-like vs portrait-like composition.
- `quadrant_contrast`: variance of the 4-quadrant mean features — "uniform field" vs "one busy corner."

**Effort: low-medium** (4–9 sub-rect passes reusing the HSV/edge/Laplacian kernels over crops the
analyzer already knows how to take).

### B.1.d Region-saliency features (medium — the subject/background reading)

This is the operator's "humans read subject → foreground/background/detail, not a flat scan"
intuition (S13 diagnosis §5), in pure Rust:

- **Cheap proxy (recommended first):** 3-region center / border / detail split. `center_saturation`
  vs `border_saturation` (subject-pop), `center_hue` vs `border_hue` (color split),
  `detail_density` (edge density of the high-Laplacian region only).
- **True saliency (medium, follow-on):** center-surround / difference-of-Gaussians on gray (using
  `imageproc::gaussian_blur_f32`) → thresholded saliency mask → `subject_size` (mask area fraction),
  `subject_hue`, `subject_saturation`, `background_hue`, `foreground_background_contrast`. No
  learned model, no OpenCV — `image`+`imageproc` suffice. Accuracy is "modest but meaningfully better
  than a flat scan."

**Effort: 3-region proxy is low-medium; full DoG saliency is medium** (a small saliency submodule +
tuning). Both are pure-Rust, zero new dependency.

### B.1.e What the heuristic layer must EXPOSE to the music side

The planner consumes ONE neutral struct (the §C.2 `ImageUnderstanding`). The heuristic layer must
populate, as plain `f32`/small enums (no music types):

| Group | Fields | Drives (planner side, [MERGE-§A]) |
|---|---|---|
| Energy | `edge_activity`, `texture`, `complexity` (0..1) | tempo plan, rhythmic density, form busyness |
| Palette | `dominant_hue`, `secondary_hue`, `dominant_hue_mass`, `colorfulness`, `value_key` | key/mode scheme, character, mode-mixture |
| Balance | `mass_centroid`, `quadrant_contrast`, `aspect_ratio`, `vertical_emphasis` | form choice, section count, symmetry of return |
| Subject | `subject_size`, `subject_hue`, `subject_saturation`, `fg_bg_contrast` | melody-vs-accompaniment color split, theme prominence |

These are **perceptual, not semantic** — they say "there is a bright saturated centered subject on a
calm dark field," not "there is a dog." That is sufficient to drive form/character/key; semantic
recognition (§B.2) is a *refinement* the planner can fold in later, not a prerequisite.

## B.2 Semantic side — web-grounded current-tooling assessment

Can we run a vision model from Rust to get "what is in the image" — and does it survive the
pure-Rust-default / local-first posture? Web-verified, mid-2026:

| Tooling | Version / status | Pure-Rust? | Cross-platform CPU | Weight |
|---|---|---|---|---|
| **`candle`** (HF) | `0.10.2`, 2026-04-01, active | **Yes** — `default=[]`, CPU backend uses pure-Rust `gemm`; CUDA/Metal/MKL opt-in | Yes, no C/C++ step | ~25 deps; ships ResNet/CLIP/YOLOv8 examples (MobileNetV3 ✗, ships V4) |
| **`tract`** (Sonos) | `0.23.1`, 2026-06-10, active, in production | **Rust-first, NOT strictly cargo-only** — default `tract-linalg` uses `cc` to assemble SIMD `.S` kernels; portable Rust fallback + wasm path exist | Yes (asm on native, fallback otherwise) | self-contained runtime (no ORT/protobuf to link); ONNX load-only; MobileNet v2/v3 ✓, YOLOv8 ✓ |
| **`ort`** (pykeio) | `2.0.0-rc.12`, 2026-03-05, active, **no stable** | **No** — FFI over Microsoft's native C++ ONNX Runtime; `download-binaries` ON by default (downloads native lib at build) | Yes | pulls tens-of-MB C++ runtime; *cannot honestly call itself pure-Rust* |
| **`wonnx`** | `0.5.1` (2023-09-30); **repo archived 2025-05-07** | n/a | **No** — GPU-only (wgpu), no CPU fallback | **Do not use** (dead, GPU-mandatory, narrow ops) |

**Small CPU vision models** (web-verified params/sizes): MobileNetV3-Small 2.54M / 9.71 MB fp32 /
~2.5–3 MB int8 **[est.]** (ImageNet 1000-class, permissive license) is the **lightest usable
"what's in it" signal**, low-tens-of-ms CPU **[est.]**. CLIP ViT-B/32 ~151M / ~600 MB **[est.]** is
the open-vocabulary option (much heavier). YOLO11n 2.6M / 56.1 ms CPU (verified) gives boxes/counts
but is **AGPL-3.0** (copyleft — Enterprise license to embed closed-source).

**Cloud vision** (the gated, optional, later path): Claude / Gemini / GPT-5.x vision give the
strongest semantic understanding at ~half-a-cent-to-a-few-cents per image and **near-zero binary
weight** (HTTP client + key), but **send the user's image off-device** (privacy) and need network —
which is exactly why they belong behind an explicit opt-in, not in the default path.

Sources (web-verified): huggingface/candle tags + `candle-core/Cargo.toml`; sonos/tract crates.io +
`linalg/Cargo.toml`/`build.rs`; pykeio `ort` crates.io + `setup/linking.mdx`; webonnx/wonnx archived
repo; torchvision/ONNX-Model-Zoo/Ultralytics model cards; vendor vision-API docs.

## B.3 RECOMMENDATION — heuristic-first; semantic optional, gated, later

1. **Ship the heuristic `ImageUnderstanding` (§B.1) as the composer's whole-image understanding.**
   Pure-Rust, zero new dependency, subsumes the saliency rebuild the S13 diagnosis already wanted,
   and turns the four dead features into plan drivers. This alone closes most of the
   "unrelated-to-the-image" gap because it gives the planner *image-as-a-whole* signal (subject,
   palette, balance, energy) instead of a flat scan of averages.

2. **Defer semantic recognition to an optional, `cargo` feature-gated layer** (`--features semantic`,
   OFF by default). When enabled, it populates a small `SemanticTags` block (§C.2) the planner *may*
   consult to refine character/form, but the planner MUST produce a complete plan from heuristics
   alone when the feature is off. **Local default = `candle` CPU-only + MobileNetV3-Small** (closest
   to pure-Rust, ~2.5–3 MB model, no C/C++ step). **`ort` is rejected for the default** (native C++
   lib breaks the pure-Rust claim); cloud vision is a *second* optional gate behind explicit
   image-leaves-device consent.

3. **Honesty flag carried into Phase 2:** "pure-Rust semantic vision" with zero native toolchain
   *and* zero model file is **not achievable** today — even `candle` (no native lib) must ship/download
   a model. So the semantic tier is *always* a weight/consent decision, never free. The heuristic tier
   is the one that is genuinely free and local-first; that is why it is the default and the semantic
   tier is the gate. **Open decision for the operator (§G):** is the semantic tier worth building at
   all, or do heuristics + the planner deliver "fits the image" well enough? Recommend deciding AFTER
   the heuristic slice is hearable.

---

# Part C — Engine re-architecture (plan-first coexisting with the scan-bar engine)

## C.0 The one-sentence shape

A `CompositionPlanner` computes a `CompositionPlan` once from an `ImageUnderstanding`; `PipelineEngine`
holds that plan and threads the **current section + theme context** into the per-step realizer, which
is still `chord_engine`'s craft. The scan does not go away — it becomes the *time cursor that walks the
plan's sections* rather than a bare left-to-right sweep of average features.

## C.1 What STAYS vs what's NEW

**STAYS (preserved, not rewritten):**
- All of `chord_engine.rs`: modes, `pick_progression`/`generate_chords`, `voice_lead_sequence`,
  `plan_phrases`/`StepPlan`, `realize_step`/`PerfFeatures`/`NoteEvent`/`OrchestralRole`, S13 harmony
  & articulation. **The planner sits ABOVE this craft and drives it; the craft is the realizer.**
- The seam: `FeatureSource`, `AudioSink`, `AudioSinkError`, `EngineObserver`, `EngineSnapshot`,
  `EngineCommand`, `InteractionEvent` — **unchanged in shape.** The composer is image-conditioned;
  it does not need a new I/O boundary.
- `pure_analysis.rs` whole-image + per-bar extraction stays; it gains the `ImageUnderstanding`
  producer (§B.1) alongside the existing `GlobalFeatures` producer.

**NEW:**
- `src/composition.rs` (new lib module): `CompositionPlan`, `Section`, `ThemeSeed`, `KeyTempoPlan`,
  the `Form`/`Character`/`Meter` enums, and the `CompositionPlanner`. Pure-Rust, builds
  `--no-default-features`, **no image types and no OpenCV** (it consumes the neutral
  `ImageUnderstanding`), and **no pixel math** (boundary respected: image analysis has no music logic;
  composition has no image logic — it reads perceptual scalars, not pixels).
- `engine::ImageUnderstanding` mirror struct (image-free, the planner's input — same mirror discipline
  S9 used for `GlobalFeatures`).
- Engine state + threading: `PipelineEngine.plan: CompositionPlan` and a section/theme-aware
  per-step decision.

## C.2 The planner's input — `ImageUnderstanding` (image-free mirror)

Lives in `engine.rs` (or `composition.rs`) as the neutral whole-image understanding the
`pure_analysis.rs` adapter populates by field-copy at the boundary (same move as `GlobalFeatures`).
**No `Mat`, no OpenCV, no music type.**

```rust
/// Whole-image perceptual understanding — the COMPOSER'S input. A richer sibling of
/// `GlobalFeatures`: computed once per image, whole-image, all plain values. The
/// `pure_analysis.rs` adapter populates it (heuristic §B.1); the `CompositionPlanner`
/// consumes it. NO image/OpenCV type and NO music type appears here (boundary: image
/// analysis has no music logic; composition reads perceptual scalars, not pixels).
#[derive(Debug, Clone, PartialEq)]
pub struct ImageUnderstanding {
    // ── Energy (0..1 knobs; B.1.a — currently-dead features re-exposed) ──
    /// Normalized edge activity, clamp(edge_density / 0.05, 0, 1).
    pub edge_activity: f32,
    /// Normalized texture, clamp(texture_laplacian_var / 2000, 0, 1).
    pub texture: f32,
    /// Normalized structural complexity, clamp(shape_complexity / 2, 0, 1).
    pub complexity: f32,

    // ── Palette (B.1.b) ──
    /// Characteristic hue (argmax of the whole-image hue histogram), 0..360.
    pub dominant_hue: f32,
    /// How peaked the palette is around the dominant hue, 0..1.
    pub dominant_hue_mass: f32,
    /// Second-strongest hue, 0..360 (== dominant_hue if monochrome).
    pub secondary_hue: f32,
    /// Two-color-ness: secondary_mass / dominant_mass, 0..1.
    pub palette_bimodality: f32,
    /// Circular hue spread (colorfulness), 0..1 (== existing hue_spread).
    pub colorfulness: f32,
    /// Low-key/high-key from the brightness histogram shape, 0..1.
    pub value_key: f32,
    /// Whole-image mean brightness, 0..100 (carried raw for the tempo/key plan).
    pub avg_brightness: f32,
    /// Whole-image mean saturation, 0..100 (carried raw for harmonic richness).
    pub avg_saturation: f32,

    // ── Composition balance (B.1.c) ──
    /// Luminance-weighted visual-mass centroid, each component 0..1.
    pub mass_centroid: (f32, f32),
    /// Variance of the 4-quadrant feature means (uniform-field vs busy-corner), 0..1.
    pub quadrant_contrast: f32,
    /// width / height.
    pub aspect_ratio: f32,
    /// Upper-vs-lower mass ratio (landscape-vs-portrait emphasis), -1..1.
    pub vertical_emphasis: f32,

    // ── Subject / region-saliency (B.1.d; defaults = whole-image when saliency off) ──
    /// Saliency-mask area fraction (subject prominence), 0..1.
    pub subject_size: f32,
    /// Subject (salient region) hue, 0..360.
    pub subject_hue: f32,
    /// Subject saturation, 0..100.
    pub subject_saturation: f32,
    /// Foreground-vs-background contrast (subject pop), 0..1.
    pub fg_bg_contrast: f32,

    // ── Optional semantic refinement (§B.2/B.3; empty unless `semantic` feature on) ──
    /// Recognized tags with confidences; ALWAYS empty under the default build. The
    /// planner MUST produce a complete plan when this is empty.
    pub semantic: SemanticTags,
}

/// Optional semantic recognition output (`--features semantic`). Default-empty so the
/// planner is heuristic-complete without it (B.3 recommendation).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SemanticTags {
    /// (label, confidence 0..1), highest-confidence first; empty under default build.
    pub labels: Vec<(String, f32)>,
    /// Coarse scene class if a scene model ran (e.g. "landscape"/"portrait"/"indoor").
    pub scene: Option<String>,
}
```

**[MERGE-§A]:** The Music Theory section decides *which* of these knobs map to *which* musical
decisions (e.g. `value_key → key brightness`, `complexity → form busyness`, `subject_size → theme
prominence`). This doc fixes the STRUCTURAL availability; the mapping table is reconciled in Phase 2.

## C.3 The plan — `CompositionPlan` (structural shape; musical contents [MERGE-§A])

The up-front plan the planner computes once. This doc defines the STRUCTURAL fields and leaves the
*musical field contents* (the form vocabulary, the device set, theme pitch material) to be reconciled
with the Music Theory section. Where a field's enum/contents are the music section's, it is marked.

```rust
/// The up-front architectural plan for one piece — computed ONCE by the
/// `CompositionPlanner` from an `ImageUnderstanding`, then DRIVES per-step realization.
/// This is the object that did not exist before this arc: the top-down architecture of
/// the piece. Pure data; no image type, no OpenCV. (Musical CONTENTS of the enums are
/// reconciled with the Music Theory section in Phase 2 — [MERGE-§A].)
#[derive(Debug, Clone, PartialEq)]
pub struct CompositionPlan {
    /// Overall macro-form. A SMALL curated vocabulary (scoping counterpoint 3), chosen with
    /// the Music section — e.g. Binary, Ternary (ABA), Strophic, ThroughComposed, Arch.
    pub form: Form,
    /// Overall character/affect — the genre-ish identity (e.g. Calm, Driving, Stately,
    /// Playful, Dark). Curated; [MERGE-§A] owns the exact set + per-character defaults.
    pub character: Character,
    /// Meter / time signature for the piece (or per-section if a section overrides).
    pub meter: Meter,
    /// Structural key + tempo scheme across the piece (§C.4). Section-stable, planned
    /// up-front, NOT per-step-derived — this is what gives the piece a tonal spine.
    pub key_tempo: KeyTempoPlan,
    /// The ordered sections that realize `form`. Each section carries its own local
    /// harmonic/rhythmic identity and a reference to a theme.
    pub sections: Vec<Section>,
    /// The theme seeds the sections recall/vary (returning themes). Empty in the first
    /// BUILD slice (themes are slice 3); a section with `theme: None` is fully valid.
    pub themes: Vec<ThemeSeed>,
    /// Total steps the plan spans (== sum of section step-lengths); the time cursor's N.
    pub total_steps: usize,
}

/// Macro-form vocabulary — SMALL and curated (scoping counterpoint 3). Exact variants
/// finalized with the Music Theory section [MERGE-§A]; this is a representative floor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form { Strophic, Binary, TernaryABA, Arch, ThroughComposed }

/// Character/affect identity. Curated; [MERGE-§A] owns the set + the per-character
/// defaults (tempo band, articulation bias, harmonic palette, density).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Character { Calm, Flowing, Driving, Stately, Playful, Dark }

/// Meter / time signature. Curated small set; [MERGE-§A] may extend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Meter { Four4, Three4, Six8, Two4, Five4 }

/// One section of the piece — a span of steps with a local identity and a theme ref.
/// This is the unit the time cursor walks; the per-step realizer is parameterized by
/// the CURRENT section (§C.5).
#[derive(Debug, Clone, PartialEq)]
pub struct Section {
    /// Section label for the snapshot/observer (e.g. "A", "B", "A'").
    pub label: String,
    /// Number of scan steps this section spans.
    pub step_len: usize,
    /// Local key offset (semitones from the plan's home root) for this section — a
    /// modulation point in the structural key plan; 0 == home.
    pub key_offset_semitones: i8,
    /// Local tempo for this section (ms_per_step), from the KeyTempoPlan; section-stable.
    pub ms_per_step: u64,
    /// Local mode name for this section (may differ from the home mode — modal/key plan).
    pub mode: String,
    /// Index into `CompositionPlan.themes` this section states/varies, or None.
    pub theme: Option<usize>,
    /// How this section varies its theme on recall (§C.7). Identity for a first statement.
    pub variation: ThemeVariation,
    /// Local harmonic-rhythm / density bias for this section, 0..1 ([MERGE-§A]).
    pub density: f32,
}

/// A returning-theme seed — the pitch/rhythm material a section can state and later
/// recall in varied form. CONTENTS are the Music Theory section's [MERGE-§A]; the
/// STRUCTURAL shape is: an identifiable, replayable, transposable motif.
#[derive(Debug, Clone, PartialEq)]
pub struct ThemeSeed {
    /// Stable id used by sections' `theme: Option<usize>` references.
    pub id: usize,
    /// The motif as scale-degree + relative-duration steps (KEY-RELATIVE so a section
    /// can transpose it by its `key_offset_semitones`). Musical contents [MERGE-§A].
    pub motif: Vec<MotifNote>,
}

/// One note of a theme motif, key/scale-relative so it transposes cleanly. [MERGE-§A]
/// owns whether degrees, intervals, or contour anchors are the right encoding.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotifNote {
    /// Scale degree relative to the section root (0 == tonic), may be negative/octave+.
    pub degree: i8,
    /// Relative duration in steps (1 == one scan step).
    pub dur_steps: u8,
}

/// How a section transforms its recalled theme — the "varied return" of ABA/Arch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeVariation { Identity, Transposed, Inverted, Augmented, Diminished, Ornamented }
```

## C.4 The structural key/tempo plan — `KeyTempoPlan`

The piece's tonal + tempo SPINE, derived once and **section-stable**. This is what replaces S13's
per-image-but-otherwise-flat tempo with a *planned* scheme: a home key/tempo, a small set of related
key areas the sections move through, and a tempo shape across the form (e.g. a slow intro → faster
middle → ritard close).

```rust
/// The piece's structural key + tempo scheme — computed once, drives every section's
/// `mode`/`key_offset_semitones`/`ms_per_step`. Section-stable: tempo and key are
/// PLANNED at section granularity, not re-derived per step (the S13 per-step tempo
/// becomes a per-section value here). Musical choice of the key relations and tempo
/// curve is [MERGE-§A]; the shape is fixed here.
#[derive(Debug, Clone, PartialEq)]
pub struct KeyTempoPlan {
    /// Home root MIDI note (replaces EngineConfig.root_midi as the piece's tonal home).
    pub home_root_midi: u8,
    /// Home mode name (from the dominant-hue lookup, §B.1.b).
    pub home_mode: String,
    /// Base tempo (ms_per_step) the tempo curve is expressed relative to (from brightness).
    pub base_ms_per_step: u64,
    /// Ordered (section_index → key_offset_semitones) the structural key plan visits.
    /// A small curated relation set (home / relative / dominant / parallel) [MERGE-§A].
    pub key_scheme: Vec<i8>,
    /// Ordered (section_index → ms_per_step) tempo across the form (intro/dev/close shape).
    pub tempo_scheme: Vec<u64>,
}
```

## C.5 How the plan drives the realizer (the new seam, real signatures)

The per-step decision becomes **section/theme-aware**. The change is purely additive to the kernel's
parameter list: the realizer now receives a `StepContext` describing *where in the plan* this step
falls, in addition to the existing `&StepPlan`. The `chord_engine` craft inside is preserved.

```rust
/// The plan-relative context for one scan step — WHICH section we are in, the section's
/// theme/key/tempo, and the step's offset within the section. Threaded into the per-step
/// realizer so realization is DRIVEN BY the plan, not by a bare scan position. Pure data.
#[derive(Debug, Clone, PartialEq)]
pub struct StepContext<'a> {
    /// The section this step falls in.
    pub section: &'a Section,
    /// 0-based step index WITHIN the section (for theme-statement timing + phrase shape).
    pub step_in_section: usize,
    /// The theme this section states/varies, resolved from `section.theme`, or None.
    pub theme: Option<&'a ThemeSeed>,
    /// The plan's home key/tempo spine (for relative transposition).
    pub key_tempo: &'a KeyTempoPlan,
}

impl PipelineEngine {
    /// NEW: install a precomputed plan (the planner ran up-front in the adapter or in
    /// `set_features_global`). Replaces the bare `Vec<StepPlan>` as the engine's
    /// top-level musical state. The phrase-level `Vec<StepPlan>` becomes a PER-SECTION
    /// detail the realizer derives from `(section, chord_engine)`.
    pub fn set_plan(&mut self, plan: CompositionPlan);

    /// NEW: compute the plan from whole-image understanding, up-front. This is the
    /// composer entry point — the analogue of today's `set_features_global`, but it now
    /// derives a STRUCTURAL plan, not just a flat chord sequence. Calls the
    /// `CompositionPlanner`, then per-section calls the existing
    /// `pick_progression`/`generate_chords`/`plan_phrases` craft to fill each section's
    /// step detail. (Back-compat: §C.6 keeps a path that reproduces today's single
    /// section bit-for-bit.)
    pub fn compose_from_image(&mut self, understanding: &ImageUnderstanding);
}

/// The per-instrument decision kernel, now plan/section-aware. SAME body shape as today
/// (project ScanBarFeatures → PerfFeatures → realize_step) but parameterized by the
/// StepContext so the realizer can: transpose by section.key_offset, use the section's
/// mode/tempo, and state/recall the section's theme. When `ctx` is the back-compat
/// default (one section, no theme, home key), this is byte-identical to today (§C.6).
pub fn decide_instrument_action(
    f: &ScanBarFeatures,
    inst_idx: usize,
    step_idx: usize,
    num_instruments: usize,
    plan_steps: &[StepPlan],   // the section's filled phrase plan (unchanged type)
    ms_per_step: u64,          // now the SECTION's tempo (from ctx.section)
    ctx: &StepContext,         // NEW — the plan-relative context
) -> InstrumentDecision;
```

**The `CompositionPlanner` itself** (the new module's core), respecting the boundary (no pixels, no
music-craft duplication — it *orchestrates* `chord_engine` per section, it does not re-implement it):

```rust
/// Computes the up-front `CompositionPlan` from whole-image understanding. Pure-Rust,
/// `--no-default-features`-clean, NO image type. It reads perceptual scalars and emits
/// STRUCTURE (form/character/meter/key-tempo/sections/themes); per-section chord/phrase
/// CONTENT is delegated to the existing `chord_engine` craft (the planner calls it, does
/// not duplicate it). The form/character/meter MAPPING from understanding is [MERGE-§A].
pub struct CompositionPlanner {
    /// Mapping ranges (curated; from mappings.json — scoping counterpoint 3: small, not a
    /// config sprawl). The planner reads these to pick form/character/meter/key relations.
    plan_mappings: PlanMappings,
}

impl CompositionPlanner {
    pub fn new(plan_mappings: PlanMappings) -> Self;

    /// Derive the whole structural plan from the image understanding. Deterministic given
    /// (understanding, plan_mappings) EXCEPT where it delegates to RNG-bearing chord craft
    /// (`pick_progression` uses thread_rng — same non-determinism boundary S9 documented;
    /// the equivalence net pins the realizer on a fixed plan, never this path).
    pub fn plan(&self, understanding: &ImageUnderstanding) -> CompositionPlan;
}

/// Curated plan-selection ranges (form/character/meter/key thresholds over the
/// understanding knobs). Lives in mappings.json (tunable without recompile, per the S13
/// precedent). SMALL and principled (scoping counterpoint 3). [MERGE-§A] owns the contents.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct PlanMappings { /* form/character/meter/key range tables — [MERGE-§A] */ }
```

## C.6 Migration path — the S9 byte-freeze + the `engine_equivalence` golden

This is the highest-risk part of the re-architecture and is handled by a **back-compat default plan**:

1. **The kernel's `ctx` argument has a "today" value.** A `StepContext` over a single `Section`
   {label "A", `step_len = total`, `key_offset = 0`, `mode = home_mode`, `theme = None`,
   `variation = Identity`, `density` = today's} with `key_tempo` whose `key_scheme`/`tempo_scheme` are
   constant. Under this context, `decide_instrument_action` does EXACTLY what it does today:
   no transposition (offset 0), no theme statement (`theme = None`), home mode, the same `ms_per_step`.
   **The new `ctx` parameter is behavior-neutral at its default.**

2. **The equivalence net is preserved by feeding it the default context.** `tests/engine_equivalence.rs`
   constructs a fixed `&[StepPlan]` and calls the kernel; the migration adds the back-compat
   `StepContext::single_section_default(...)` to that call. Because the default context applies zero
   transposition / no theme / home key, **the golden constants (cadence 240 ms, vel 114/84, register
   36/79) are unchanged and the net stays green.** This is the same discipline S9 used: the kernel is
   pure and its output is pinned on a fixed plan; we extend the signature without changing the output
   at the legacy operating point.

3. **The S13 articulation goldens** (`realize_rhythm` non-cadence holds) are likewise unaffected by the
   plan threading — the plan changes *which section/key/tempo* a step uses, not the articulation curve.
   (The articulation *clamp* of §E.0 is a SEPARATE, deliberate golden re-derivation, handled in its own
   slice with hand-derived constants and a reviewer sign-off, exactly per the S13 spec §7 discipline.)

4. **`EngineConfig.root_midi` is superseded by `KeyTempoPlan.home_root_midi`** but not removed — it
   seeds the home root when no image has been composed yet (the TUI-seek-before-first-image path S13
   already flagged). Additive, not breaking.

5. **Landing order** (so each step keeps the net green): (a) add the new types in `composition.rs`
   + the `ImageUnderstanding` mirror, no behavior change; (b) add the `ctx` parameter with the
   back-compat default wired at every call site incl. the equivalence net — **net stays green, zero
   musical change**; (c) add `compose_from_image`/`CompositionPlanner` producing a >1-section plan
   ONLY when called (the batch path opts in); the legacy `set_features_global` path still produces the
   single-section plan. The golden only ever moves when a slice deliberately re-derives it.

## C.7 Reconciliation points to merge in Phase 2 ([MERGE-§A])

The Music Theory section owns the musical CONTENTS of the structural shells defined here. Phase 2
merges:

1. **`Form` / `Character` / `Meter` variant sets** — the curated vocabulary (this doc gives a
   representative floor; the music section finalizes the exact small set and their musical defaults).
2. **The understanding→plan MAPPING** — which `ImageUnderstanding` knob drives form vs character vs
   meter vs key-relation, and the threshold ranges (the `PlanMappings` contents). This is the heart of
   "fits the image" and is jointly owned: this doc guarantees the knobs are *available*; the music
   section decides what they *mean*.
3. **`ThemeSeed.motif` / `MotifNote` encoding** — degrees vs intervals vs contour anchors, and how a
   theme is generated from the image (e.g. from `subject_hue`/`mass_centroid`) and varied
   (`ThemeVariation` semantics).
4. **`KeyTempoPlan.key_scheme` relations** — the small curated key-relation set (home/relative/
   dominant/parallel) and the tempo-curve shapes per form/character.
5. **Morphing harmony** — how a section's harmony *interpolates* toward the next (the music section's
   "morphing harmony" item) maps onto the `Section` boundary; the structural hook is the per-section
   `mode`/`key_offset`/`density` + the cadence at section ends. Confirm whether morphing needs a
   per-step harmonic-target field on `StepContext` (an additive change if so).
6. **Per-section phrase fill** — confirm `plan_phrases` is called once per section over that section's
   chords (the planner's delegation), and that cadence placement at *section* ends (not just phrase
   ends) is the music section's call.

---

# Part D — Data-flow diagram (proposed architecture)

```
   ┌──────────────────────── pure_analysis.rs (image side — NO music logic) ─────────────────────────┐
   │  RgbImage ─► analyze_global_pure ─► GlobalFeatures (8 avg scalars)         [STAYS]               │
   │           └► scan_steps          ─► Vec<Vec<ScanBarFeatures>> (flat L→R)    [STAYS]               │
   │           └► analyze_understanding ─► ImageUnderstanding  (NEW §B.1: palette/balance/saliency/   │
   │                                       energy; semantic block empty unless --features semantic)   │
   │              [optional] semantic::recognize ─► SemanticTags  (NEW, gated; candle CPU + MobileNet)│
   └───────────────────────────────────────────┬───────────────────────────────────────────────────┘
                  field-copy at the BOUNDARY (no Mat crosses): image_analysis::* / understanding ─► engine::*
                                                │
   ════════════════════ HEADLESS / PURE-RUST LIBRARY BOUNDARY (--no-default-features) ════════════════════
                                                │
   ┌───────────────────── composition.rs (NEW — NO image logic, NO pixel math) ───────────────────────┐
   │  CompositionPlanner::plan(&ImageUnderstanding)  ─►  CompositionPlan { form, character, meter,     │
   │     reads PERCEPTUAL scalars only                    key_tempo, sections[], themes[] }            │
   │     delegates per-section chord content ──────────►  (calls chord_engine craft, does not dup it)  │
   └───────────────────────────────────────────┬───────────────────────────────────────────────────┘
                                                │ CompositionPlan
   ┌──────────────────────────────── engine.rs  PipelineEngine ───────────────────────────────────────┐
   │  compose_from_image(understanding) ─► planner ─► self.plan: CompositionPlan   [NEW]               │
   │  tick(source, sink):                                                                              │
   │     resolve current Section + StepContext from step_index over plan.sections    [NEW]             │
   │     source.scan_bar_features(step,n) ─► decide_instrument_action(.., ctx)  ─► PerfFeatures ─►      │
   │            chord_engine::realize_step (section mode/key/tempo + theme)  ─► Vec<NoteEvent>  [STAYS] │
   │     sink.note_on/.note_off ; advance time cursor ; return EngineTickOutput (+ section label)      │
   │  current_state() ─► EngineSnapshot (+ section/form for the observer)                              │
   └───────────────────────────────────────────┬───────────────────────────────────────────────────┘
                          &mut A: AudioSink ────┘ (MidiOut / future synth — UNCHANGED seam)
                          observers (CLI/TUI/GUI) ◄── EngineObserver::on_tick (now sees section/form)
```

Key: the OpenCV/`image` zone stays on the image side; `composition.rs` and `engine.rs` stay pure and
headless. The only new things crossing the boundary are the plain `ImageUnderstanding`/`SemanticTags`
value structs. **No `Mat`, ever.** Module boundaries hold: image analysis has no music logic; the
composition planner has no image logic (it reads perceptual scalars, not pixels); the chord engine
has no image logic (the planner hands it the *section's* musical params, never image data).

---

# Part E — Staged roadmap

Sequenced so the operator hears STRUCTURE as early as possible, each slice builds + tests headless +
is hearable, and the engine-equivalence net implications are flagged per slice. **Pure-Rust structure
first; semantic recognition is a late, optional, gated slice.**

## E.0 Slice 0 (ride-along) — clamp the S13 articulation extremes

**Folded into Slice 1, cheap.** The S13 articulation curve maps `edge_activity` 0→1 to a note-length
fraction `LEGATO_FRAC_HI = 1.05` → `STACCATO_FRAC = 0.40`, clamped to `0.30..=1.20` in the `sustained()`
helper inside `realize_rhythm` (`chord_engine.rs`, ~`:1144`–`:1182`). The operator flagged the
note-length *extremes* as unpleasant. **Fix:** tighten the musical band — raise the staccato floor and
lower the legato ceiling toward a pleasant range (e.g. `0.55..=1.05` instead of `0.30..=1.20`), and/or
gentle the curve's endpoints. **Owner:** Music Theory Specialist (it is articulation craft).
**Equivalence net:** this DELIBERATELY moves the non-cadence articulation goldens — re-derive the
affected constants by hand in the same commit with a comment pointing here (S13 spec §7 discipline);
keep the cadence branch byte-stable so the cadence golden stays 240 ms.
**Hearable:** notes stop having jarring too-short/too-long extremes — immediate listening win.
**Depends on:** nothing. Ships in Slice 1.

## E.1 Slice 1 — `ImageUnderstanding` (heuristic) + the dead-feature wiring

**Builds:** the heuristic whole-image understanding in `pure_analysis.rs` (§B.1.a free features +
§B.1.b palette + §B.1.c balance) → the `engine::ImageUnderstanding` mirror + boundary copy. No planner
yet; this slice just makes the rich image signal *available* and proves the boundary.
**Tested headless:** `analyze_understanding` over canned `RgbImage` literals asserts the knobs span
their ranges across the six in-repo images (the S13 measured set is the fixture); mirror round-trip
asserts field parity. No synth.
**Hearable:** wire `value_key`/`complexity` into existing S13 dials as an interim → modest immediate
diversity bump (optional; the real payoff is Slice 2).
**Equivalence net:** GREEN — additive struct + new producer; no kernel change.
**Depends on:** nothing. **+ Slice 0 rides along here.**

## E.2 Slice 2 — the structural skeleton: `CompositionPlan` with SECTIONS + key/tempo plan (NO themes, NO semantic) — **THE FIRST BUILD SLICE**

**Builds:** `composition.rs` with `CompositionPlan`/`Section`/`KeyTempoPlan`/`Form`/`Character`/`Meter`
+ a minimal `CompositionPlanner::plan` that, from `ImageUnderstanding`, picks a form + character +
meter and lays out **2–4 sections with section-stable key offsets and a tempo curve**; the engine
`compose_from_image` + `set_plan` + the `StepContext` threading + the back-compat default plan (§C.6).
Themes are `None`; semantic is off.
**Tested headless:** (a) the back-compat default plan keeps `engine_equivalence` GREEN
(byte-identical kernel at the default context); (b) NEW property tests: distinct images yield plans
with *different section counts / key schemes / tempo curves*; (c) a section-boundary test asserts the
realized key/tempo CHANGES at the planned section boundary and is STABLE within a section.
**Hearable:** **THIS is where the operator first hears STRUCTURE** — an image now opens in section A,
moves to a contrasting section B (different key area / tempo / density), and (for ABA/Arch) returns —
instead of a uniform left-to-right wash. Even without themes, sectioned key/tempo + cadences at
section ends produce an audible *architecture*.
**Equivalence net:** GREEN at default; the new >1-section behavior is exercised only via the new
`compose_from_image` path, which the net does not pin (same boundary as S13's `set_features_global`).
**Depends on:** Slice 1 (consumes `ImageUnderstanding`).

## E.3 Slice 3 — returning/varied THEMES

**Builds:** `ThemeSeed`/`MotifNote`/`ThemeVariation` population in the planner (generate a motif from
the image — e.g. `subject_hue`/`mass_centroid` → contour, [MERGE-§A]) + theme statement/recall in the
realizer via `StepContext.theme` (a section states its theme; a recapitulating section recalls it
transposed/varied). [MERGE-§A] owns the musical generation + variation.
**Tested headless:** a recapitulation section's melody is a recognizable transform of the exposition
theme (assert the degree-sequence is the `ThemeVariation` of the original, not a fresh random line).
**Hearable:** the piece now has *memory* — a tune you heard at the start comes back at the end, varied.
This is the single biggest "this is a composition, not a texture" cue.
**Equivalence net:** GREEN at default (default plan has no theme); new behavior on the compose path.
**Depends on:** Slice 2.

## E.4 Slice 4 — region-saliency `ImageUnderstanding` upgrade (subject/background)

**Builds:** the §B.1.d saliency layer (3-region proxy → optional DoG mask) populating
`subject_*`/`fg_bg_contrast` in `ImageUnderstanding`. Lets the planner drive a melody-vs-accompaniment
color split and theme prominence from the actual subject, not the whole-image average.
**Tested headless:** an image with a distinct centered subject yields `subject_size`/`fg_bg_contrast`
materially different from a uniform field; assert the plan's theme-prominence/voice-split responds.
**Hearable:** the music tracks the *subject* of the image, not its average — directly attacks
"unrelated to the image" for representational (non-abstract) photos.
**Equivalence net:** GREEN (image-side + planner-side; default plan unchanged).
**Depends on:** Slice 1–2 (extends `ImageUnderstanding`, fed by the planner).

## E.5 Slice 5 (optional, gated, LATE) — semantic recognition tier

**Builds:** `src/semantic.rs` behind `--features semantic` (default OFF): `candle` CPU-only +
MobileNetV3-Small ONNX → `SemanticTags` populating `ImageUnderstanding.semantic`; the planner *may*
refine `Character`/`Form` from tags but MUST be complete without them (§B.3). A second, separate
`--features cloud-vision` gate for the cloud path behind explicit image-leaves-device consent.
**Tested headless:** with the feature OFF (default), the planner produces a complete plan and all
prior nets are unchanged; with it ON, a fixture image yields non-empty `labels` and the plan's
character can shift. The model file is a build/runtime asset, not vendored into git.
**Hearable:** a recognizably "stormy seascape" vs "sunny meadow" can nudge character/form — but ONLY
as a refinement on top of the heuristic plan.
**Equivalence net:** GREEN (gated; default build has no semantic path).
**Depends on:** Slices 1–4 proving the heuristic plan is good enough that semantic is a *refinement*,
plus the operator's go/no-go on building the tier at all (§G open decision).

---

# Part F — Risks / trade-offs

- **R1 — equivalence-net breakage during threading.** The `ctx` parameter touches the frozen kernel's
  signature. *Mitigation (§C.6):* the back-compat default `StepContext` is behavior-neutral; the net is
  fed that default and stays byte-green. The only intentional golden move is Slice 0's articulation
  clamp, handled with hand-derived constants + reviewer sign-off (S13 §7 discipline). **Rejected
  alternative:** a parallel "v2 kernel" leaving the old one untouched — rejected because it forks the
  realizer and doubles the craft-maintenance surface; the additive-neutral-parameter path is cleaner.
- **R2 — scope: this is a near-rewrite of the generative core.** *Mitigation:* the staged roadmap (§E)
  makes each slice independently hearable + headless-testable; the planner sits ABOVE the preserved
  `chord_engine`, so the *craft* is not rewritten, only the *architecture above it* is added. The
  first BUILD slice (E.2) is self-contained and back-compat.
- **R3 — vocabulary sprawl ("fits the image + sounds nice" is unbounded).** *Mitigation (scoping
  counterpoint 3):* `Form`/`Character`/`Meter` are small curated enums; `PlanMappings` is a small
  range table in `mappings.json`, not an open config surface. The Phase-2 merge (§C.7) fixes the
  vocabulary deliberately small and coherent. **Rejected alternative:** a general-purpose
  "composition DSL" — rejected as exactly the unbounded surface to avoid.
- **R4 — semantic tier breaks the pure-Rust/local-first posture.** *Mitigation (§B.3):* it is
  `cargo`-feature-gated OFF by default; the planner is heuristic-complete without it; `ort`/cloud are
  rejected for the default. Honest cost: even the lightest local semantic path ships/downloads a model
  — never free. **This is an operator go/no-go (§G), not an assumed build.**
- **R5 — section/theme adds latency/state to the headless core.** *Mitigation:* the plan is computed
  ONCE up-front (`compose_from_image`); `tick` does an O(1) section lookup over `plan.sections`. The
  core stays single-threaded + deterministic (S9 D4); no new concurrency.
- **R6 — `[MERGE-§A]` divergence between this doc's structural shapes and the music section's contents.**
  *Mitigation:* §C.7 enumerates every reconciliation point; Phase 2 is exactly this merge. Structural
  fields here are deliberately content-neutral shells so the music section can fill them without
  reshaping the engine seam. **Trade-off accepted:** designing the shape before the contents risks a
  field that the music section doesn't need or needs differently; that is cheaper to fix in a merge
  than to entangle structure and musical content now.

---

# Part G — Preliminary first BUILD slice + open engineering decisions

## G.1 Recommended first BUILD slice

**Slice 2 (E.2): the structural skeleton — `CompositionPlan` with sections + a structural key/tempo
plan, threaded through a back-compat-default engine, with the S13 articulation clamp (E.0) riding
along.** Rationale: it is the first slice where the operator hears *architecture* (an image that opens,
develops, and closes in distinct sections), it is pure-Rust with zero new dependency, it is fully
headless-testable, and the back-compat default plan keeps the entire existing net green. Slice 1
(`ImageUnderstanding`) is its prerequisite and is trivial (it mostly re-exposes dead features), so the
*buildable unit* is "Slice 1 + Slice 2 + the E.0 clamp" landed together.

## G.2 Open engineering decisions for the operator (finalized in Phase 2 after the music merge)

1. **`compose_from_image` vs `set_features_global`** — does the batch path SWITCH to the composer
   entirely (every run is sectioned), or keep a flag for "legacy flat mode"? (Recommend: composer
   becomes default once Slice 2 is hearable; keep the default-plan path only as the equivalence anchor.)
2. **Where the planner runs** — inside `engine::compose_from_image` (lib, headless-testable,
   recommended) vs in the `main.rs` adapter. Recommend lib, for the same headless-determinism reason
   S9 put the engine in the lib.
3. **`StepContext` lifetime vs owned** — `StepContext<'a>` borrows the section/theme (zero-copy, shown
   above) vs an owned snapshot per step. Recommend borrowed; confirm it doesn't fight the `tick`
   borrow of `self.plan` (may need a small restructure of `tick` to compute the context before the
   mutable advance — a pure-Rust borrow-checker detail to resolve at implementation).
4. **Section granularity for tempo/key** — section-stable (recommended, this doc) vs allowing a tempo
   *ramp* within a section (ritardando across a closing section). The structural hook exists
   (`tempo_scheme` is per-section); a within-section ramp would add a per-step tempo field to
   `StepContext`. Defer unless the music section wants it.
5. **Semantic tier go/no-go (§B.3/R4)** — build the optional `--features semantic` tier at all, or
   declare heuristics + the planner sufficient? Recommend deciding AFTER Slice 4 (saliency) is heard.
6. **`EngineConfig.root_midi` / `--ms-per-step` semantics** — now superseded per-section by the
   `KeyTempoPlan`; confirm they remain as the pre-compose seed (recommended) vs are removed (breaking).
7. **`ThemeSeed.motif` encoding** — degrees vs intervals vs contour ([MERGE-§A], §C.7.3); flagged here
   because it is the boundary where the engine's theme-replay reads the music section's encoding.

---

*End of Phase 1. The seam the planner needs is defined: an image-free `ImageUnderstanding` in, a
`CompositionPlan` (form/character/meter/key-tempo/sections/themes) as the engine's top-level musical
state, and a section/theme-aware `StepContext` threaded into the PRESERVED `chord_engine` realizer via
a behavior-neutral default that keeps the S9/S13 equivalence net green. The image-understanding fork is
resolved heuristic-first (pure-Rust, builds on the dead features + the saliency rebuild the S13
diagnosis already wanted) with semantic recognition deferred to an optional, feature-gated, web-grounded
tier that does not survive the pure-Rust-default posture for free. The first BUILD slice is the
pure-Rust structural skeleton (sections + key/tempo plan), which makes audible structure hearable with
zero new dependency, with the S13 articulation clamp riding along. Phase 2 merges the Music Theory
section's vocabulary and mappings into the structural shells at the [MERGE-§A] points enumerated in
§C.7.*

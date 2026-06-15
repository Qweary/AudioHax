# Composition Architecture — Canonical Assessment

**Status:** CANONICAL. This is the operator-facing roadmap the project steers on.
**Date:** 2026-06-14 (Phase 2 synthesis)
**Authored by:** Rust Architect (engine/feasibility/roadmap), merging the Music Theory Specialist's musical architecture (Section A) faithfully.
**Backing material (kept in place, the detailed sources):**
- Musical architecture — full detail: [`composition-architecture-musical.md`](./composition-architecture-musical.md)
- Engine / feasibility / staged roadmap — full detail: [`composition-architecture-engine.md`](./composition-architecture-engine.md)

This document is the merge. Section (A) transcludes the Music Theory Specialist's judgments faithfully — that section is the domain authority for musical decisions, and where it is condensed for flow the full source is cited. Sections (B), (C), (D) are the engineering halves with real Rust signatures preserved. The document ends with the single reconciled **recommended first BUILD slice** and one consolidated **open-decisions** list.

---

## Framing — the paradigm shift, and the verdict that motivated it

AudioHax through S2–S13 built excellent **bottom-up, note-level craft**: correct diatonic modes, conservative voice leading (common-tone retention, parallel-perfect rejection), a real phrase model with cadences at boundaries, an expressive performance layer (velocity/rhythm/role-pitch with Bass/HarmonicFill/Melody roles), and — since S13 — genuine per-image *diversity* (tempo from brightness, 7th/9th harmony from saturation, a continuous articulation curve, density bias, modal mixture).

The operator listened through a real engine and delivered the verdict that opened this arc: the diversity is **real**, but the output is **ethereal, structureless, and unrelated to the image — it only "works" for abstract art.**

That is not a contradiction. It is the precise, predictable sound of **excellent local craft with no global plan.** Located in code: the engine **sonifies a scan**. `pure_analysis.rs` reduces the whole image to eight whole-image *average* scalars (`analyze_global_pure`, `:423`) plus a flat left-to-right per-bar sequence (`scan_steps`, `:550`). `PipelineEngine::tick` (`engine.rs:405`) walks `step_index` 0→N calling the per-step realizer `decide_instrument_action` (`:562`) → `chord_engine::realize_step` (`:891`). The only object above the per-step realizer is `plan_phrases` (`chord_engine.rs:631`), which groups steps into 4/8-step phrases with cadences — harmonic phrasing, not form. And `decide_instrument_action` indexes `plan[step_idx % plan.len()]` — **the plan loops.** There is confirmed no meter, no key-change, no section, no theme memory anywhere in the tree.

**The paradigm shift this assessment designs:** from *scan sonifier* → *image-conditioned COMPOSER*. Read the image as a whole, derive a structural **CompositionPlan up-front** (form, character, meter, key/tempo scheme, a returning theme, a section list), and render a piece whose per-step realization is **driven by that plan** rather than by a bare left-to-right scan. **The existing `chord_engine` craft is preserved, not replaced** — the planner sits *above* it and drives it. The single biggest lever against "structureless and ethereal" is a **returning, varied theme**: the ear forgives almost anything if it recognizes a tune coming home.

**The honest ceiling** (per Section A §10 and §B.3): the target is the operator's own words — **"follows music-theory principles, fits the image, and is pleasant."** That is achievable with a *small, coherent* vocabulary (a few forms × a few characters × a few devices, each tied to a robust *heuristic* image property). "Sounds hand-composed / great" is explicitly **not** a target this layer should chase; stating the ceiling keeps the vocabulary small and prevents an infinite tuning loop.

---

# Section (A) — MUSICAL ARCHITECTURE

*Faithful transclusion of the Music Theory Specialist's Section A. This section owns the musical contract. Condensed for flow; the full detail (every form's audible signature, the character parameter table, the complete image→music mapping table, all variation techniques) lives in [`composition-architecture-musical.md`](./composition-architecture-musical.md). The musical judgments below are NOT relitigated by the engineering sections.*

## A.1 The thesis

The missing layer is a **CompositionPlan computed once, before any note is realized**, that imposes top-down architecture *over* the existing craft. It decides FORM, CHARACTER, METER (which does not exist today at all), a KEY/TEMPO SCHEME across sections, and a THEMATIC PLAN (a motif that returns and is varied). Crucially, the plan **reuses every existing function** — it does not reinvent voice leading, realization, or phrasing; it *sequences and parameterizes* them. The single biggest lever is **A.5: a returning, varied theme.**

## A.2 Macro-form vocabulary

A form is **audible** only through three things: **return** (material the ear recognizes coming back), **contrast** (a section clearly *other*), and **cadential articulation** (a real close between sections). Discipline: **ship three forms, default to one, hold a fourth in reserve.** A piece is a list of **Sections**; each Section is a run of phrases sharing a key, tempo, character, and a **thematic role** (Statement / Contrast / Return / Development / Coda).

1. **Rounded Binary — `A B A′` — THE DEFAULT.** A (statement, home key, closes with a half/imperfect cadence — a *question*), B (contrast: different key center and/or character — a *digression*), A′ (return of A's theme, closing with a **perfect authentic cadence** — the *answer home*; shortened or lightly varied, never a literal copy). **Why default:** smallest form delivering all three audibility cues, most forgiving of imperfect material because the return does the heavy lifting. ~3 sections, ~16–24 steps.
2. **ABA / Ternary (da capo)** — for strong contrast. A is **self-contained** (PAC at its end), B fully contrasts (often parallel mode / new character / sometimes new tempo), A returns **complete**. For images with a clear subject-vs-background contrast.
3. **Theme and Variations — `T V1 V2 (V3)`** — for busy/complex images. A Theme then 2–3 Variations keeping the theme's **harmonic skeleton and phrase length** but transforming the surface. This is where S13's per-step diversity becomes a *virtue*: each variation deliberately re-weights the realization knobs.

**Held in reserve (do NOT ship first):** **Through-composed** (no return — *this is what the engine accidentally produces today*, exactly the rejected feel; ship later only for genuine directional gradients, gated behind motivic continuity). **Rondo `A B A C A`** (defer until region/saliency reading exists).

**Cross-cutting audibility rules (requirements on A.4–A.6):** return must reuse the *same theme in the same home key*; contrast must differ in **at least two** of {key center, mode/parallel-mode, character, tempo, register}; section boundaries must cadence, and the boundary cadence is **stronger** than internal phrase cadences (PAC closes a section; half cadences live inside).

## A.3 Character / genre

A character is **not a label** — it is a **bundle of concrete musical parameters** applied across the piece: meter, tempo range, texture (which roles are active and how), rhythmic signature, articulation tendency, dynamic posture. **Ship four, default to one.**

| Character | Meter | Tempo (BPM) | Texture | Articulation | Dynamic posture |
|---|---|---|---|---|---|
| **Ballad** *(DEFAULT)* | 4/4 | 56–76 | full ensemble, melody-led, sustained fill | strongly legato | gentle arches; soft; safest "pleasant" |
| **Waltz** | **3/4** | 96–144 | bass beat 1, fill 2–3 ("oom-pah-pah"), melody floats | portato bass, legato melody | lilting accent on 1 |
| **March** | 4/4 (or 2/4) | 96–120 | bass + melody prominent, fill steady | detached/marcato | firm, terraced; strong 1 & 3 |
| **Lament** | 4/4 (or 6/8) | 48–66 | melody + bass, fill thin | legato, weighted; exaggerated phrase-end ritard | dark, narrow-low; favors minor/Aeolian/Phrygian |

**Reserved: Scherzo** (defer — overlaps Waltz mechanically).

**How character binds to existing code (no new realizer):** tempo range *clamps/centers* the S13 brightness→tempo result; texture sets which `OrchestralRole`s sound on which beats (the meter binding, A.4); rhythmic signature *shifts the `edge_activity` band thresholds per character* in `realize_rhythm`; articulation tendency applies a **per-character multiplier on `base_frac`** before the existing clamp; dynamic posture biases `realize_velocity`'s level/accent **as a plan-supplied scalar**. (Full parameter detail: musical doc §2.)

## A.4 Meter mechanism

**There is no meter today** — a flat stream of equal "steps," with only a latent phrase-position accent in `realize_velocity` (position 0 gets +9, even +2, odd −6).

- **Define: one STEP = one BEAT** (already the S13 convention). A **measure** is a fixed run of `beats_per_measure` steps (4/4→4, 3/4→3, 6/8→2-felt). **Keep `PHRASE_LENGTHS` meaning beats/steps** for the first ship, and let **meter group the steps within the existing phrase** — meter is a *grouping overlay*, not a phrase-model restructuring.
- **Metric weight** from position within the measure (4/4: beats 1 STRONG, 3 secondary, 2&4 weak; 3/4: 1 STRONG, 2&3 weak; 6/8: 1 STRONG, 4 secondary). **Binding: re-point `realize_velocity`'s accent from `position_in_phrase` to `metric_position`** — the S6 strong-gets-+/weak-gets-− logic is already shaped right; it just reads the measure beat. The cleanest possible meter introduction.
- **Character selects meter, texture realizes it:** Waltz gates Bass→beat 1, Fill→beats 2&3; March gates Bass→1&3, Melody subdivides; Ballad needs *no* per-beat gating (felt through accent + slow tempo — why it is the safe default). Each `OrchestralRole` carries an optional **beat-mask**; `realize_rhythm`/`realize_velocity` consume `metric_position`. **No new realizer function** — two new scalar inputs threaded in.

## A.5 Structural key / tempo plan

Today: one key (C4 constant), one mode (from hue), one tempo (S13 brightness→BPM, applied once globally). That flatness is half of "structureless."

- **Tonic plan:** home key = the existing image-derived tonic. Rounded-binary/ABA: A in home, **B modulates to a closely related key** (priority: dominant `root+7`; relative major/minor `±3`; parallel mode same-root Ionian↔Aeolian), **A′/A returns home** — the return-to-home *is* the resolution. Theme-and-variations stays home (optionally one parallel-mode "minore/maggiore" variation).
- **Modulation mechanics (reuse existing harmony):** the existing `secondary_dominant_of` builds an applied dominant; the dominant of the *new* key is the pivot. **Tonicization, not full functional modulation, is the honest first target** — what the existing vocabulary supports without new machinery. Bound transposition to ±7 semitones (the realizer already re-seats into register bands and clamps 24..=108).
- **Mode plan:** home mode in A/A′; B either switches to parallel for contrast or stays if the key already moved (don't change two things at once unless the image is high-contrast). The S13 mode-mixture (bVI, borrowed iv) becomes a **B-section coloring device** — local shadow, the real-music use, not a global wash.
- **Tempo plan:** keep S13 brightness→tempo as the **base**, then: (1) **character window** clamps/centers it (a "bright Ballad" is still slow, just at the bright end — character wins the category, brightness the position); (2) **sectional tempo relationship** (small ratios ×1.0/×0.85/×1.18, tied to image contrast); (3) **structural ritardando** at the final section's last phrase (promote the existing cadence `RITARDANDO_FACTOR`). **Tempo is a section property, not a global constant.**

## A.6 Thematic material — the motif that returns and is varied

**The single most important section.** Return of a recognizable theme converts "a stream of pleasant chords" into "a piece." Today there is **no theme** — the Melody role plays whatever chord tone its register band selects, with no memory.

- **What a theme is:** a short melodic gesture (a **contour + rhythm over the section's chords**) carried by the **Melody `OrchestralRole`** over the statement section — a pitch contour (scale-degree sequence), a rhythmic profile, bound to the phrase. The theme **lives in the plan**, not the realizer: a small `MotifCell { degrees, rhythm }`. The realizer, on a step carrying a motif note, plays *that degree* in the melody register instead of free-selecting — `role_pitch`/`realize_rhythm` still place and articulate it. **Bass/fill unaffected.**
- **How it's generated (image-seeded, heuristic):** contour *direction* from a brightness gradient (bright-top → descending "from the light"); falls back to a **fixed pleasing contour** (arch) seeded by hue when no gradient signal exists. Contour *range* from edge density/complexity (calm → stepwise; busy → wider leaps). Length fixed (4-step) for memorability. Rhythmic profile from the **character**. **Curated, not generative-infinite:** ~4 contour archetypes (arch, descent, ascent, neighbor-turn), each a known-good shape the image *selects and parameterizes*.
- **How it returns (marks the form):** rounded binary — states in A, **absent/fragmented in B**, **returns recognizably in A′** in the home key. The plan marks which sections carry the theme (`thematic_role`); the realizer plays the motif in Statement/Return/Development sections, free-selects in Contrast.
- **Variation techniques (small curated set):** augmentation/diminution (cheapest, most audible), transposition (used by the key plan), reharmonization (keep contour, change chords — powerful), ornamentation (insert passing/neighbor tones), fragmentation (head-only, for development). **A′ uses at most light variation** — recognizability is the goal; save heavy transformation for theme-and-variations.
- **The orchestrational payoff:** the theme makes the Melody role *melodic* for the first time — a tune over the unchanged bass+fill accompaniment, which is what "music" sounds like versus the current homogeneous chord-stream.

## A.7 Morphing / progressive harmony

S13 gave per-step variety; what's missing is a **harmonic trajectory across the arc**.

- **A (statement):** harmonically **stable**, close on a weak/medium cadence (open question).
- **B (departure):** the **harmonic high-water mark** — the mixture/secondary-dominants that today fire randomly per-edge are **concentrated here** by the plan; they belong to the departure.
- **A′ (return):** **resolving**, ending on the **strongest cadence in the piece** (root-position PAC, soprano on tonic).
- **Climax:** at a **principled, recurring location** — recommended **end of B / start of return** (maximum distance from home → relief of return). The plan marks `is_climax`; the realizer pushes register/dynamics/density/harmony to their high ends there — a *structural* use of the per-step knobs S13 already built.
- **Cadence strength as punctuation:** half cadences = commas (internal); IAC/PAC = periods (section boundaries); the one root-position PAC + structural ritardando = full stop. *Differentiated* cadence strength does enormous work for "this has a shape."

## A.8 The CompositionPlan — the musical contract

Pipeline shift: `set_features_global` → **`compute_composition_plan(global, [+regions])` → `CompositionPlan`** → expands into a flat `Vec<StepPlan>` (one entry per step, **no longer looped — played once start to finish**) → the realizer consumes each enriched `StepPlan`.

**Critical reuse:** `CompositionPlan` does **not** replace `StepPlan`; it **generates a richer sequence of them**. The composition layer is a **planner that sits above `plan_phrases`**, calling `generate_chords`/`voice_lead_sequence`/`plan_phrases` **per section** and concatenating the results with section/meter/motif annotations. The musical contract the plan carries (full field list in musical doc §7.2): per-section `thematic_role`, `key` (tonic+mode), `tempo_ms_per_step`, `meter`, `character_overlay`, `progression`, `phrases` (with boundary cadence strength), `motif_active`, `motif_variation`; plus the piece-level `form`, `character`, `home_key`, `theme`, `climax_step`.

**The one hard caution (golden net):** `realize_velocity`/`realize_rhythm`'s non-cadence formulas are pinned by `tests/engine_equivalence.rs` goldens (velocities 114/84, register 36/79, cadence hold 240 ms). Character/meter/climax biases will move some. Per S13 precedent: **re-derive changed goldens by hand from the new documented formula in the same commit; never loosen an assert to silence it.** Apply biases as **plan-supplied scalars defaulting to identity (×1.0 / +0)** so the *default* plan reproduces today's goldens exactly. **The cadence branch stays byte-stable** — meter/character act on non-cadence steps.

*(Section A authority ends here. The image→music mapping table — which heuristic property drives which musical choice, and what is "semantic, later" — is shared with the engineering analysis and appears reconciled in §B.4. Full musical mapping rationale: musical doc §8.)*

---

# Section (B) — IMAGE-UNDERSTANDING LAYER

The composer needs to understand the image **as a whole** to derive a plan. Two kinds of understanding, with very different cost — this is the heuristic-vs-semantic fork.

## B.1 The fork

- **Heuristic** — perceptual composition properties (palette, balance, region-saliency, complexity, energy). *Pure-Rust, doable now, zero new dependency.*
- **Semantic** — what the image *is* (objects, scene, faces → literal subject-matching). *A capability + dependency leap.*

**Recommendation: heuristic-first; semantic optional, feature-gated, later.** The planner consumes ONE neutral `ImageUnderstanding` struct and **MUST produce a complete plan from heuristics alone**; semantic tags are an optional refinement, never a prerequisite.

## B.2 Heuristic side — pure-Rust, buildable now

All of the following extend `pure_analysis.rs` using `image`/`imageproc` (**already in the tree**) and respect the module boundary (no music logic in image analysis):

- **B.2.a — FREE today (computed, currently dead):** `hue_spread`, `texture_laplacian_var`, `shape_complexity` (0.011–2.005, **180× spread**), `aspect_ratio`. Normalized (S13 calibration: `texture=clamp(var/2000,0,1)`, `complexity=clamp(shape/2,0,1)`, `colorfulness=hue_spread`) → four well-spread knobs for free. **Effort: trivial.**
- **B.2.b — Palette (cheap):** `dominant_hue`+`dominant_hue_mass` (argmax of a wider hue histogram — a multi-color image's circular *mean* can land on a hue no pixel has; the dominant bin is its *characteristic* color), `secondary_hue`+`palette_bimodality` (the subject-vs-background-color intuition), `value_key` (low/high-key from the brightness-histogram *shape*). **Effort: low.**
- **B.2.c — Composition balance (cheap, pure crop math):** `mass_centroid` (luminance-weighted center of "stuff"), `vertical_emphasis` (upper-vs-lower mass), `quadrant_contrast` (variance of 4-quadrant means — "uniform field" vs "busy corner"). These are the **quadrant/half means** Section A asks for (→ form balance/symmetry + theme contour gradient). **Effort: low-medium.**
- **B.2.d — Region-saliency (medium — the subject/background reading):** the cheap **3-region center/border/detail proxy** first (`center_saturation` vs `border_saturation` = subject-pop — the **center-vs-border contrast** Section A asks for → B's modulation), then optional **true DoG saliency** (center-surround via `imageproc::gaussian_blur_f32` → mask → `subject_size`/`subject_hue`/`fg_bg_contrast`). No learned model, no OpenCV. **Effort: proxy low-medium; DoG medium.**

**The three high-value new heuristics Section A requested are all here and all cheap/semantics-free:** (1) quadrant/half feature means (→ form balance + theme contour gradient), (2) center-vs-border contrast (→ B's modulation), (3) brightness/contrast dispersion (→ tempo-change + dynamic spread, via `value_key`/`quadrant_contrast`). **None requires ML.**

## B.3 Semantic side — web-grounded current-tooling assessment

Can we run a vision model from Rust for "what is in the image" and survive the pure-Rust/local-first posture? Web-verified, mid-2026:

| Tooling | Status | Pure-Rust? | Weight |
|---|---|---|---|
| **`candle`** (HF) | `0.10.2`, active | **Yes** — CPU backend pure-Rust `gemm`, `default=[]` | ~25 deps; ships ResNet/CLIP/YOLOv8 examples |
| **`tract`** (Sonos) | `0.23.1`, active, in production | **Rust-first, NOT cargo-only** — default `tract-linalg` uses `cc` to assemble SIMD `.S`; portable fallback exists | self-contained runtime; ONNX load-only; MobileNet v2/v3 ✓ |
| **`ort`** (pykeio) | `2.0.0-rc.12`, no stable | **No** — FFI over MS native C++ ONNX Runtime; downloads native lib at build | tens-of-MB C++ runtime; *not pure-Rust* |
| **`wonnx`** | archived 2025-05-07 | n/a | **Do not use** (dead, GPU-only) |

**Lightest usable signal:** MobileNetV3-Small (2.54M params / ~2.5–3 MB int8 **[est.]**, ImageNet 1000-class, permissive license, low-tens-of-ms CPU **[est.]**). CLIP ViT-B/32 (~600 MB **[est.]**) is the open-vocabulary heavy option. YOLO11n is **AGPL-3.0** (copyleft — avoid embedding). **Cloud vision** (Claude/Gemini/GPT-5.x) gives the strongest understanding at near-zero binary weight but **sends the image off-device** — why it belongs behind explicit opt-in.

*Sources (web-verified): huggingface/candle tags + `candle-core/Cargo.toml`; sonos/tract crates.io + `linalg/Cargo.toml`/`build.rs`; pykeio `ort` crates.io + `setup/linking.mdx`; webonnx/wonnx archived repo; torchvision/ONNX-Model-Zoo/Ultralytics model cards; vendor vision-API docs.*

## B.4 Recommendation + the reconciled image→music mapping

1. **Ship the heuristic `ImageUnderstanding` (§B.2) as the composer's whole-image understanding.** Pure-Rust, zero new dependency, subsumes the saliency rebuild the S13 diagnosis already wanted, turns the four dead features into plan drivers. This alone closes most of the "unrelated-to-the-image" gap.
2. **Defer semantic recognition to an optional `--features semantic` layer (OFF by default).** Local default if built: **`candle` CPU-only + MobileNetV3-Small** (closest to pure-Rust, no C/C++ step). **`ort` rejected for the default** (native C++ breaks the pure-Rust claim); cloud is a *second* gate behind explicit image-leaves-device consent.
3. **Honesty flag:** "pure-Rust semantic vision" with zero native toolchain *and* zero model file is **not achievable** today — even `candle` must ship/download a model. The semantic tier is *always* a weight/consent decision, never free; the heuristic tier is the one genuinely free and local-first. **Operator go/no-go (§ open decisions); recommend deciding AFTER the heuristic slice is hearable.**

**Reconciled mapping (Section A owns the musical column; this doc guarantees the heuristic is *available*):**

| Musical choice (§A authority) | Heuristic property | Availability |
|---|---|---|
| FORM (rounded-binary / ABA / theme-and-variations) | composition balance/symmetry (quadrant/half means) + complexity | balance: **new cheap** (§B.2.c); complexity: **exists** (dead feature) |
| SECTION/VARIATION count | salient-region count → fallback **complexity** | regions: §B.2.d proxy; complexity fallback exists |
| CHARACTER (ballad/waltz/march/lament) | warmth (hue) + brightness + energy (edge/texture) | **exists** |
| METER | derived from character | derived |
| TEMPO base | brightness (S13) + edge nudge | **exists** |
| HOME key/mode | dominant hue (+ spread for mixture) | mean exists; **dominant-hue an upgrade** (§B.2.b) |
| SECTIONAL key (B's modulation) | center-vs-border contrast | **new cheap** (§B.2.d proxy) |
| TEMPO relationship (B faster/slower) | brightness/contrast dispersion | **new cheap** (`value_key`/`quadrant_contrast`) |
| THEME contour | brightness gradient (top↔bottom) + hue | gradient: **new cheap** (§B.2.c); hue exists |
| THEME range/disjunctness | edge density / complexity | **exists** |
| CLIMAX intensity | complexity/energy peak; placement structural | exists / structural |
| DYNAMIC spread | brightness dispersion | **new cheap** |

**True saliency / subject recognition / scene classification is "semantic, later"** and only *enriches* (better region count, literal subject matching). Every form/character/key/theme choice above functions on the heuristic signals alone.

---

# Section (C) — ENGINE RE-ARCHITECTURE

A `CompositionPlanner` computes a `CompositionPlan` once from an `ImageUnderstanding`; `PipelineEngine` holds that plan and threads the **current section + theme context** into the per-step realizer, which is still `chord_engine`'s craft. The scan does not go away — it becomes the *time cursor that walks the plan's sections* rather than a bare left-to-right sweep.

## C.1 What STAYS vs what's NEW

**STAYS (preserved, not rewritten):**
- All of `chord_engine.rs`: modes, `pick_progression`/`generate_chords`, `voice_lead_sequence`, `plan_phrases`/`StepPlan`, `realize_step`/`PerfFeatures`/`NoteEvent`/`OrchestralRole`, S13 harmony & articulation. **The planner sits ABOVE this craft and drives it.**
- The seam: `FeatureSource`, `AudioSink`, `AudioSinkError`, `EngineObserver`, `EngineSnapshot`, `EngineCommand`, `InteractionEvent` — **unchanged in shape.**
- `pure_analysis.rs` whole-image + per-bar extraction stays; it gains the `ImageUnderstanding` producer (§B.2) alongside the existing `GlobalFeatures` producer.

**NEW:**
- `src/composition.rs` (new lib module): `CompositionPlan`, `Section`, `ThemeSeed`, `KeyTempoPlan`, the `Form`/`Character`/`Meter` enums, and the `CompositionPlanner`. Pure-Rust, builds `--no-default-features`, **no image types, no OpenCV, no pixel math** (it reads perceptual scalars, not pixels).
- `engine::ImageUnderstanding` mirror struct (image-free — same mirror discipline S9 used for `GlobalFeatures`).
- Engine state + threading: `PipelineEngine.plan: CompositionPlan` and a section/theme-aware per-step decision.

## C.2 The planner's input — `ImageUnderstanding` (image-free mirror)

The neutral whole-image understanding the `pure_analysis.rs` adapter populates by field-copy at the boundary (same move as `GlobalFeatures`). **No `Mat`, no OpenCV, no music type.** (Full field list in engine doc §C.2; the load-bearing groups:)

```rust
/// Whole-image perceptual understanding — the COMPOSER'S input. A richer sibling of
/// `GlobalFeatures`: computed once per image, whole-image, all plain values.
#[derive(Debug, Clone, PartialEq)]
pub struct ImageUnderstanding {
    // ── Energy (0..1; dead features re-exposed) ──
    pub edge_activity: f32,        // clamp(edge_density / 0.05, 0, 1)
    pub texture: f32,              // clamp(texture_laplacian_var / 2000, 0, 1)
    pub complexity: f32,           // clamp(shape_complexity / 2, 0, 1)
    // ── Palette ──
    pub dominant_hue: f32,         // argmax of whole-image hue histogram, 0..360
    pub dominant_hue_mass: f32, pub secondary_hue: f32, pub palette_bimodality: f32,
    pub colorfulness: f32,         // == existing hue_spread
    pub value_key: f32,            // low/high-key from brightness-histogram shape, 0..1
    pub avg_brightness: f32, pub avg_saturation: f32,
    // ── Composition balance ──
    pub mass_centroid: (f32, f32), pub quadrant_contrast: f32,
    pub aspect_ratio: f32, pub vertical_emphasis: f32,
    // ── Subject / region-saliency (defaults = whole-image when saliency off) ──
    pub subject_size: f32, pub subject_hue: f32, pub subject_saturation: f32, pub fg_bg_contrast: f32,
    // ── Optional semantic refinement (empty unless `semantic` feature on) ──
    pub semantic: SemanticTags,    // ALWAYS empty under default build; planner MUST be complete without it
}

/// Optional semantic recognition output (`--features semantic`). Default-empty.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SemanticTags {
    pub labels: Vec<(String, f32)>,   // (label, confidence), highest first; empty under default
    pub scene: Option<String>,        // coarse scene class if a scene model ran
}
```

## C.3 The plan — `CompositionPlan` (structural shape; musical contents are §A's)

The up-front plan computed once. The **structural fields** are fixed here; the **musical contents** of the enums are Section A's (reconciled — §C.7).

```rust
/// The up-front architectural plan for one piece — computed ONCE by the
/// `CompositionPlanner` from an `ImageUnderstanding`, then DRIVES per-step realization.
#[derive(Debug, Clone, PartialEq)]
pub struct CompositionPlan {
    pub form: Form,            // macro-form — §A.2: RoundedBinary(default)/TernaryABA/ThemeAndVariations
    pub character: Character,  // §A.3: Ballad(default)/Waltz/March/Lament
    pub meter: Meter,          // §A.4 (per-section override allowed)
    pub key_tempo: KeyTempoPlan,   // §A.5 — section-stable tonal+tempo spine, planned up-front
    pub sections: Vec<Section>,    // the ordered sections that realize `form` — THIS IS THE PIECE
    pub themes: Vec<ThemeSeed>,    // returning themes (§A.6); a section with theme:None is valid
    pub total_steps: usize,        // == sum of section step-lengths; the time cursor's N
}

/// §A.2 vocabulary — SMALL and curated. RoundedBinary is the default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form { RoundedBinary, TernaryABA, ThemeAndVariations, /* reserve: */ ThroughComposed, Rondo }

/// §A.3 — curated; each carries tempo band / articulation bias / texture / dynamic posture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Character { Ballad, Waltz, March, Lament }

/// §A.4 — curated small set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Meter { Four4, Three4, Six8, Two4 }

/// One section — a span of steps with a local identity and a theme ref. The unit the time
/// cursor walks; the per-step realizer is parameterized by the CURRENT section.
#[derive(Debug, Clone, PartialEq)]
pub struct Section {
    pub label: String,                 // "A" / "B" / "A'"  — for the snapshot/observer
    pub step_len: usize,
    pub thematic_role: ThematicRole,   // §A.2/A.6: Statement|Contrast|Return|Development|Coda
    pub key_offset_semitones: i8,      // modulation point in the key plan; 0 == home (§A.5)
    pub ms_per_step: u64,              // section-stable tempo from KeyTempoPlan (§A.5)
    pub mode: String,                  // section mode (may differ from home — modal/key plan)
    pub progression: Vec<String>,      // Roman numerals for this section (mixture concentrated in B — §A.7)
    pub theme: Option<usize>,          // index into themes[] this section states/varies, or None
    pub variation: ThemeVariation,     // how it varies its theme on recall (Identity for a statement)
    pub boundary_cadence: CadenceStrength,  // §A.7 — half inside, PAC at the structural close
    pub density: f32,                  // local harmonic-rhythm/density bias, 0..1 (§A.3)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThematicRole { Statement, Contrast, Return, Development, Coda }

/// A returning-theme seed (§A.6). KEY-RELATIVE so a section transposes it by its key_offset.
#[derive(Debug, Clone, PartialEq)]
pub struct ThemeSeed { pub id: usize, pub motif: Vec<MotifNote> }

/// One motif note — scale/key-relative so it transposes cleanly. (degrees vs intervals vs
/// contour anchors is an open §A decision — see open decisions.)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotifNote { pub degree: i8, pub dur_steps: u8 }

/// §A.6.4 variation techniques.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeVariation { Identity, Transposed, Reharmonized, Augmented, Diminished, Ornamented, Fragmented }
```

## C.4 The structural key/tempo plan — `KeyTempoPlan`

The piece's tonal + tempo SPINE, derived once and **section-stable** — what replaces S13's per-image-but-flat tempo with a *planned* scheme (§A.5).

```rust
/// The piece's structural key + tempo scheme — computed once, drives every section's
/// mode/key_offset/ms_per_step. Section-stable: the S13 per-step tempo becomes a
/// per-section value here. Key relations and tempo curve are §A's (§A.5).
#[derive(Debug, Clone, PartialEq)]
pub struct KeyTempoPlan {
    pub home_root_midi: u8,        // tonal home (from dominant-hue lookup); seeds, then per-section offsets apply
    pub home_mode: String,
    pub base_ms_per_step: u64,     // base tempo (from brightness) the curve is relative to
    pub key_scheme: Vec<i8>,       // section_index → key_offset_semitones (home/relative/dominant/parallel — §A.5)
    pub tempo_scheme: Vec<u64>,    // section_index → ms_per_step (character window ∩ image; structural ritard at close)
}
```

## C.5 How the plan drives the realizer (the new seam, real signatures)

The per-step decision becomes **section/theme-aware** — purely additive to the kernel's parameter list. The realizer now receives a `StepContext` describing *where in the plan* this step falls. The `chord_engine` craft inside is preserved.

```rust
/// The plan-relative context for one scan step — WHICH section, the section's theme/key/
/// tempo, and the step's offset within the section. Threaded into the realizer so
/// realization is DRIVEN BY the plan, not by a bare scan position. Pure data.
#[derive(Debug, Clone, PartialEq)]
pub struct StepContext<'a> {
    pub section: &'a Section,
    pub step_in_section: usize,      // for theme-statement timing + phrase shape
    pub theme: Option<&'a ThemeSeed>,// resolved from section.theme, or None
    pub key_tempo: &'a KeyTempoPlan, // home spine, for relative transposition
}

impl PipelineEngine {
    /// Install a precomputed plan — replaces the bare `Vec<StepPlan>` as the engine's
    /// top-level musical state. The phrase-level `Vec<StepPlan>` becomes a PER-SECTION
    /// detail the realizer derives from (section, chord_engine).
    pub fn set_plan(&mut self, plan: CompositionPlan);

    /// Compute the plan from whole-image understanding, up-front — the composer entry
    /// point (analogue of today's set_features_global, but it derives STRUCTURE). Calls
    /// the CompositionPlanner, then per-section calls existing pick_progression/
    /// generate_chords/plan_phrases to fill each section's step detail. (Back-compat:
    /// §C.6 keeps a path that reproduces today's single section bit-for-bit.)
    pub fn compose_from_image(&mut self, understanding: &ImageUnderstanding);
}

/// The per-instrument decision kernel, now plan/section-aware. SAME body shape as today
/// (project ScanBarFeatures → PerfFeatures → realize_step) but parameterized by StepContext
/// so it can transpose by section.key_offset, use the section's mode/tempo, and state/recall
/// the theme. When `ctx` is the back-compat default (one section, no theme, home key), this
/// is byte-identical to today (§C.6).
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

The `CompositionPlanner` itself (boundary-respecting — no pixels, no music-craft duplication; it *orchestrates* `chord_engine` per section):

```rust
/// Computes the up-front CompositionPlan from whole-image understanding. Pure-Rust,
/// --no-default-features-clean, NO image type. Reads perceptual scalars and emits STRUCTURE;
/// per-section chord/phrase CONTENT is delegated to the existing chord_engine craft (calls
/// it, does not duplicate it). The form/character/meter MAPPING is §A's / §B.4.
pub struct CompositionPlanner { plan_mappings: PlanMappings }

impl CompositionPlanner {
    pub fn new(plan_mappings: PlanMappings) -> Self;
    /// Deterministic given (understanding, plan_mappings) EXCEPT where it delegates to
    /// RNG-bearing chord craft (pick_progression uses thread_rng — same boundary S9
    /// documented; the equivalence net pins the realizer on a fixed plan, never this path).
    pub fn plan(&self, understanding: &ImageUnderstanding) -> CompositionPlan;
}

/// Curated plan-selection ranges (form/character/meter/key thresholds over the
/// understanding knobs). Lives in mappings.json (tunable without recompile, per S13). SMALL
/// and principled (vocabulary discipline). Contents are §A's / §B.4.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct PlanMappings { /* form/character/meter/key range tables */ }
```

## C.6 Migration path — the S9 byte-freeze + the `engine_equivalence` golden

The highest-risk part, handled by a **back-compat default plan**:

1. **The kernel's `ctx` has a "today" value.** A `StepContext` over a single `Section` {label "A", `step_len = total`, `key_offset = 0`, home mode, `theme = None`, `variation = Identity`} with a constant `key_scheme`/`tempo_scheme`. Under it, `decide_instrument_action` does EXACTLY what it does today — no transposition, no theme, home mode, same `ms_per_step`. **The new parameter is behavior-neutral at its default.**
2. **The equivalence net is preserved by feeding it the default context.** `tests/engine_equivalence.rs` constructs a fixed `&[StepPlan]` and calls the kernel; the migration adds `StepContext::single_section_default(...)`. Zero transposition / no theme / home key → **the goldens (cadence 240 ms, vel 114/84, register 36/79) are unchanged, the net stays green.** Same discipline as S9: extend the signature without changing the output at the legacy operating point.
3. **The S13 articulation goldens** are unaffected by plan threading (the plan changes *which section/key/tempo*, not the articulation curve). The articulation *clamp* (§D ride-along) is a SEPARATE, deliberate golden re-derivation in its own slice with hand-derived constants + reviewer sign-off (S13 §7 discipline).
4. **`EngineConfig.root_midi` is superseded by `KeyTempoPlan.home_root_midi`** but **not removed** — it seeds the home root before any image is composed (the TUI-seek-before-first-image path). Additive, not breaking.
5. **Landing order (each step keeps the net green):** (a) add the new types + the `ImageUnderstanding` mirror — no behavior change; (b) add the `ctx` parameter with the back-compat default wired at every call site incl. the equivalence net — **net stays green, zero musical change**; (c) add `compose_from_image`/`CompositionPlanner` producing a >1-section plan ONLY when called. The golden only ever moves when a slice deliberately re-derives it.

## C.7 Reconciliation — structural shells filled by §A

The structural shells above are filled by Section A's musical contents at these points (all now resolved in this merged doc): `Form`/`Character`/`Meter` variant sets (§A.2/A.3/A.4); the understanding→plan mapping (§B.4 table); `ThemeSeed.motif` encoding (§A.6 — degree-contour, image-seeded; exact encoding is an open decision); `KeyTempoPlan.key_scheme` relations (§A.5 — home/relative/dominant/parallel); morphing harmony onto section boundaries (§A.7 — concentrated in B via the per-section `progression` + boundary cadence; confirm whether it needs a per-step harmonic-target field — open decision); per-section phrase fill (`plan_phrases` called once per section over that section's chords, cadence at section ends).

---

# Section (D) — STAGED ROADMAP

Sequenced so the operator hears STRUCTURE as early as possible; each stage builds + tests headless + is hearable; the equivalence-net implication is flagged per stage. **Pure-Rust structure first; semantic recognition is a late, optional, gated stage.** (Engine doc §E and musical doc §9 are the detailed sources; they are reconciled here into one arc.)

**Slice 0 (ride-along) — clamp the S13 articulation extremes.** The S13 curve maps `edge_activity` 0→1 to a note-length fraction `LEGATO_FRAC_HI=1.05`→`STACCATO_FRAC=0.40`, clamped `0.30..=1.20` in `realize_rhythm` (~`chord_engine.rs:1144`–`:1182`). The extremes are unpleasant (below ~0.5 a note reads as a click; above ~1.15 successive notes mud together). **Fix:** constrain the **non-cadence** hold fraction to a perceptually pleasant window — **roughly `0.55 ≤ base_frac ≤ 1.10`** — narrowing the *output* range without flattening its *responsiveness* (it still varies with edge activity). The cadence ring (1.20) stays as-is; the per-character `articulation_bias` (§A.3) then rides on top of this clamped window (Ballad→1.10, March→0.55). **Owner: Music Theory Specialist (articulation craft).** **Net:** DELIBERATELY moves the non-cadence articulation goldens — re-derive by hand in the same commit with a comment, keep the cadence branch byte-stable. **Folds into the first build slice.**

- **Stage 1 — `ImageUnderstanding` (heuristic) + dead-feature wiring (§B.2).** Make the rich image signal available; prove the boundary. *Net: GREEN (additive). Tested: knobs span their ranges across the six in-repo images; mirror round-trip parity.* Trivial — mostly re-exposes dead features. **Prerequisite of the first build slice.**
- **Stage 2 — THE FIRST BUILD SLICE (see below): the sectioned non-looping plan + the returning theme over a rounded-binary skeleton.**
- **Stage 3 — METER (§A.4):** `beats_per_measure` + metric accent (re-point `realize_velocity` to `metric_position`); add **Waltz (3/4)** oom-pah-pah beat-masks. *Audible: a downbeat you can feel; 3/4 vs 4/4.*
- **Stage 4 — CHARACTER (§A.3):** the four presets as `CharacterOverlay` biases; character selected from warmth/brightness/energy. *Audible: the same image as a ballad vs a march.*
- **Stage 5 — KEY/TEMPO PLAN depth (§A.5):** sectional modulation via the applied-dominant pivot, per-section tempo ratios, structural ritardando. *Audible: B "goes somewhere"; the return lands home.* (The first slice already ships a *minimal* sectioned key/tempo spine; this stage adds real modulation + tempo relationships.)
- **Stage 6 — MORPHING HARMONY + CLIMAX (§A.7):** concentrate mixture/secondary-dominant in B; mark + realize the climax; full cadence-strength hierarchy. *Audible: a peak; harmonic shape.*
- **Stage 7 — VARIATION TECHNIQUES + THEME-AND-VARIATIONS form (§A.6.4/A.2):** augmentation/diminution/reharmonization/ornament/fragment; the third form. *Audible: development, not just repetition.*
- **Stage 8 — ABA + form selection from image (§A.2/§B.4):** wire form choice to balance/symmetry/complexity; the quadrant/center-border heuristics. *Audible: different images get different forms.*
- **Stage 9 — region-saliency upgrade (§B.2.d):** 3-region proxy → optional DoG mask populating `subject_*`/`fg_bg_contrast`; melody-vs-accompaniment color split + theme prominence from the actual subject. *Audible: the music tracks the subject, not the average — attacks "unrelated to the image" for representational photos.*
- **Stage 10 (optional, gated, LATE) — semantic tier (§B.3):** `src/semantic.rs` behind `--features semantic` (default OFF): `candle` CPU + MobileNetV3-Small → `SemanticTags`; the planner *may* refine character/form but MUST be complete without them. A second `--features cloud-vision` gate behind explicit image-leaves-device consent. *Net: GREEN (gated). Operator go/no-go — recommend deciding after Stage 9.*

**Risk posture (engine doc §F):** R1 net-breakage — mitigated by the behavior-neutral default `StepContext`. R2 near-rewrite scope — mitigated because the planner sits *above* the preserved craft (the craft is not rewritten) and every slice is independently hearable. R3 vocabulary sprawl — mitigated by small curated enums + a small `mappings.json` range table, not an open config surface (no "composition DSL"). R4 semantic-tier posture — gated OFF by default. R5 latency — the plan is computed once; `tick` does an O(1) section lookup.

---

# RECOMMENDED FIRST BUILD SLICE

**Build, as one landed unit, the sectioned non-looping `CompositionPlan` AND the returning theme over a rounded-binary `A B A′` skeleton — in the default character (Ballad, 4/4) and the home key — with the S13 articulation clamp riding along.** Concretely: Stage 1 (`ImageUnderstanding` re-exposing the dead features) + a minimal `CompositionPlanner` that lays out 3 sections (A statement / B contrast / A′ return) and **expands to a non-looping flat `StepPlan` list played once start-to-finish** (this kills the `plan[step_idx % plan.len()]` loop — the structural root cause) + a **generated motif** (curated contour set, image-seeded by hue + edge activity) that the **Melody role plays in A and A′ and is absent/fragmented in B**, with **differentiated cadence strength** (half cadence ends A, PAC ends A′) + the engine `compose_from_image`/`set_plan`/`StepContext` threading behind the **back-compat default plan** that keeps `engine_equivalence` byte-green + the §D clamp.

This is the reconciliation of the two Phase-1 first-slice proposals: the engine side's "structural skeleton" supplies the non-looping sectioned plan and the back-compat-safe seam, and the music side's "returning theme" supplies the *defining* cure for structurelessness — and the theme is cheap to add on top of the spine because it needs **no new image extraction** (hue + edge activity already exist) and **no meter/modulation machinery** (held at defaults). The two together are what turns "different sections" into *a piece*, and they exercise the whole `CompositionPlan → non-looping StepPlan → realizer` pathway end-to-end so every later stage is an enrichment of a working spine.

- **Touches:** `pure_analysis.rs` (the `ImageUnderstanding` producer — re-expose dead features); new `src/composition.rs` (`CompositionPlan`/`Section`/`KeyTempoPlan`/`ThemeSeed`/the enums/the minimal `CompositionPlanner`); `engine.rs` (`ImageUnderstanding` mirror + boundary copy, `compose_from_image`/`set_plan`, the `StepContext` parameter on `decide_instrument_action` with the back-compat default at every call site); `chord_engine.rs` (the §D non-cadence articulation clamp, hand-re-derived goldens); `tests/engine_equivalence.rs` (feed the default context — stays green). **Zero new dependency.**
- **Tests headless:** (a) `engine_equivalence` GREEN at the default context (byte-identical kernel); (b) the §D clamp's deliberately re-derived non-cadence goldens (hand-derived, commented); (c) NEW property tests — distinct images yield plans with different motif contours / cadence placement; the realized plan is **non-looping** (length == `total_steps`, no modular wrap); the recapitulation (A′) melody is a recognizable transform of the A theme (degree-sequence match, not a fresh random line); the structural close is a PAC.
- **Becomes hearable:** the operator hears, for the first time, **a tune that states, departs, and returns, and a real ending** — an image that opens in A, goes elsewhere in B, brings the opening melody back in A′, and lands harder at the close than anywhere prior. The defining cure for "structureless," with zero new dependency and no meter/modulation machinery, and immediately-better note lengths from the clamp.

---

# OPEN DECISIONS FOR THE OPERATOR

Genuine forks where the operator's ear/taste or a build-direction call should decide — consolidated from both Phase-1 docs.

**Musical (Section A authority):**

1. **Default form.** Recommend **rounded binary (`A B A′`)** — smallest form with statement+contrast+return, most forgiving. Alternatives: full **ABA** (stronger contrast, complete return) or **theme-and-variations** ("one idea explored"). *Which feels most "him."*
2. **Default character.** Recommend **Ballad** (slow, legato, safest "pleasant," needs the least new metric machinery for the first slice). Alternative: **Waltz** (more immediately characterful, but waits on Stage 3's meter).
3. **How literal the image→music mapping should be.** **(a) abstract/affective** (image sets mood/energy/color, the music is its own coherent piece — the recommended lean, most robust) vs **(b) literal/descriptive** ("you can hear the picture" — brightness gradient *is* the contour, region count *is* the section count; more impressive when it lands, more wrong when it doesn't, pushes toward semantics sooner). *This single choice sets how aggressively we chase region/saliency/semantic features.*
4. **Climax placement.** End-of-B (recommended, classic) vs golden-section (~62%) vs none-for-now.
5. **Modulation aggressiveness.** Tonicization-only (recommended Stage-5 lean, safe) vs real functional modulation with pivot chords (richer, more machinery, more chances to sound wrong).
6. **Does the theme appear in B at all?** Fully absent (max contrast) vs fragmented (more unity, more "developed") vs a contrasting *second* theme.
7. **The honest ceiling.** Confirm the target is **"principled, fits the image, pleasant"** and **not** "sounds hand-composed / great." Recommend stating this as a project ground-truth to keep the vocabulary small and prevent an infinite tuning loop.

**Engineering (Section C authority):**

8. **`compose_from_image` replaces batch, or a legacy flag?** Does the batch path switch to the composer entirely (every run sectioned), or keep a flag for "legacy flat mode"? *Recommend: composer becomes default once the first slice is hearable; keep the default-plan path only as the equivalence anchor.*
9. **`ThemeSeed.motif` encoding.** Degrees vs intervals vs contour anchors — the boundary where the engine's theme-replay reads the music section's encoding. (This doc shows degree-contour as the floor; §A.6 leans contour-from-a-curated-set.)
10. **Section-stable vs within-section tempo ramps.** Section-stable (recommended) vs allowing a tempo *ramp* within a section (a ritardando across the closing section) — the latter adds a per-step tempo field to `StepContext`. *Defer unless the music section wants the within-section ritard.*
11. **Semantic-tier go/no-go (§B.3/§B.4).** Build the optional `--features semantic` tier at all, or declare heuristics + the planner sufficient? *Recommend deciding AFTER the saliency stage (Stage 9) is heard.*
12. **`StepContext` borrowed vs owned.** `StepContext<'a>` borrows the section/theme (zero-copy, recommended) vs an owned per-step snapshot. Recommend borrowed; confirm it doesn't fight `tick`'s borrow of `self.plan` (may need a small `tick` restructure to compute the context before the mutable advance — a borrow-checker detail for implementation).

---

*End of canonical assessment. Backing detail remains in [`composition-architecture-musical.md`](./composition-architecture-musical.md) (Section A's full musical reasoning) and [`composition-architecture-engine.md`](./composition-architecture-engine.md) (the full engine/feasibility/per-slice detail and web-sourced semantic tooling table). Design-only: no source, test, or asset modified by this document.*

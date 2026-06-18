# AudioHax Aesthetics Rule Inventory

## Purpose

This document is the **rule-inventory chunking surface** for the aesthetics-review arc. It is the ground-truth catalogue, authored through the **Music Theory lens**, of every music-producing decision in the AudioHax image-to-music pipeline across sessions S2 through S34. It is organized neutrally and completely into reviewable decision-clusters so that a separate **affective-first prioritization** can be overlaid on top of it without re-deriving the rules. Each rule carries the metadata (where it lives, what drives it, whether it connects the output to the image, its wiring state) that the affective ordering keys on.

The catalogue is grounded against the actual source — every identifier (function, const, struct, field, mappings.json key) was verified against `src/chord_engine.rs`, `src/composition.rs`, `src/pure_analysis.rs`, and `assets/mappings.json` and is reproduced verbatim. Line numbers reflect the state at S34 (HEAD `20fc407` plus the S34 `fa52795` lane; AudioHax repo).

**Standing failure mode the review arc exists to fix** (operator verdict, S13/S14, re-confirmed through S21): *"diversity real but ethereal / structureless / image-unrelated; only works for abstract art."* The closing section of this inventory notes, per cluster, how each plausibly contributes to that verdict — to seed the affective prioritization without performing it here.

> **Read the [Affective-First Review Chunking Plan](#affective-first-review-chunking-plan) below first** — it imposes the review ORDER (which decision-clusters the S36+ dual-agent passes attack first, ranked by how strongly each drives "reads as the image" and "shaped vs. ethereal/structureless"). It is the additive prioritization layer over the neutral catalogue that follows; it does not alter any catalogue entry.

### How to read a rule entry

Each rule carries:

- **Rule** — what the decision is.
- **Where** — file + function / const / table.
- **Session** — which session introduced or last changed it (S2–S34).
- **Driven by** — the image feature(s) that drive it, or `fixed` / `structural` if not image-driven.
- **Image-relatedness** — does this rule connect the OUTPUT to the IMAGE? `yes-strong` / `yes-weak` / `no`. (This is the tag the affective overlay keys on.)
- **Wiring state** — `LIVE` (runs on the default path), `SELECTED` (reachable, fires only when its SelectTable rule matches an image), or `AUTHORED-BUT-INERT` (in the codebase but no active rule selects it / it is a no-op on the default path).
- **MT note** — Music-Theory craft assessment in one line. Where a rule's quality is a *taste* judgment that Music Theory has flagged it cannot adjudicate (the "reads as intended" / "sounds good" layer), that is stated explicitly — those are prime aesthetics-review targets.

### Architecture in one paragraph (so the clusters make sense)

`pure_analysis.rs::understand_image_pure` extracts an `ImageUnderstanding` (28 perceptual scalars) from the image. `composition.rs::CompositionPlanner::plan` computes a `CompositionPlan` ONCE per image: it runs first-match-wins `SelectTable` ladders over those scalars to pick a form, character, meter, key-scheme, texture/orchestration profile, and prominence profile; it computes an affect composite (arousal/valence); it expands the form into a non-looping list of `Section`s, each with its own key offset, tempo, mode, progression, theme, density, and orchestration. The realizer `chord_engine.rs::realize_step` then turns each step of each section into `Vec<NoteEvent>` (the craft layer: voice leading, dynamics, rhythm, articulation, counterpoint, figuration, pivots). The planner sits ABOVE the preserved craft. An `engine_equivalence` byte-freeze (goldens 240ms / vel 114 / 84 / pitch 36 / 79) pins the realizer's behavior on an identity single-section plan, so every slice that touched the realizer had to prove it changes nothing on that path.

---

## Affective-First Review Chunking Plan

*(Authored through the Composition & Songwriting Aesthetics lens, S35. This section is ADDITIVE — it does not change any catalogue entry below; it imposes the review ORDER on them. It is the chunking layer Music Theory's catalogue and this aesthetics ordering co-own.)*

### What this section is

The catalogue below is organized by topic (correctness/craft), neutrally. This section re-orders it by AFFECT: which decision-clusters the upcoming chunked dual-agent passes (Aesthetics ∥ Music Theory) should review FIRST, ranked by how strongly each one controls the two halves of the standing verdict —

1. **Does the music READ AS THE IMAGE?** (image-relatedness — the "only works for abstract art / unrelated to the image" half), and
2. **Does it sound SHAPED / INTENTIONAL rather than ethereal / structureless?** (the "ethereal / structureless" half).

The ranking is not arbitrary. A cluster ranks high when changing it is the most likely thing the owner's ear will HEAR as fixing the verdict — either because it carries real per-image information the output currently ignores, or because it governs whether a piece floats vs. lands. Low-impact, well-crafted, or already-successful clusters drop to Tier 2 even when they are large, because reviewing them first would burn passes on things that sound fine. **The test of every ordering choice: would a trained-eared listener, hearing the before/after of this chunk on a real photo, say "yes, that is more like the picture" or "yes, that has a shape now"? If not, it is not Tier 1.**

Two structural facts from the catalogue shape the whole ordering. (i) The "ethereal" complaint is concentrated in a SMALL number of levers (note-length window C9.1, the missing major/minor mode C6.6, the affect tuning C6), while the "image-unrelated" complaint is SPREAD across many clusters (inert features C1, dead tables C2-tail, structural-but-image-blind shaping C7/C10/C12). So: attack the concentrated ethereal levers as tight early chunks (high payoff per pass), and attack image-relatedness by ACTIVATING latent image signal (C1 inert features, C11 inert generators) rather than by re-crafting already-sound structural shaping. (ii) Effect-size order from the affect research [Eerola et al. 2013] is **mode 0.29 > tempo 0.14 > register 0.08 > dynamics 0.04 > articulation 0.02** — mode and tempo are the loudest affect cues, which is why the unbuilt valence→mode (C6.6) and the affect/tempo tuning (C6) lead, and why image-independent dynamics (C7) and the already-tamed articulation extremes drop below them.

### Headline affective targets (the 3–5 highest-leverage findings, ranked)

These are the findings I would attack FIRST regardless of how the chunks are batched. They are ranked by expected audible impact on the verdict.

1. **`valence → major/minor mode` is designed but UNBUILT (C6.6).** *Attack first.* This is the single highest-leverage item in the entire inventory. Mode is the largest affect effect (0.29, ~2× tempo) and the most direct carrier of "joyful vs. not." Today valence drives character + tempo but the actual major/minor still comes from HUE (C4.1) — so an image the affect bridge correctly read as energetic/positive can still be voiced minor, which is *exactly* the "doesn't feel like the image" failure at its most audible. Building C6.6 is also the cleanest fix because the design already exists; the pass decides activation, not invention.
2. **The arousal/valence affect tuning — composite weights, character thresholds, tempo windows (C6.1–6.5).** *Attack second.* This cluster owns "energetic/fast/joyful images don't sound that way," and S22 already wired the de-cap and composite — but every weight, threshold, and tempo window in it is an ear-tuned value Music Theory explicitly defers to taste. This is the densest concentration of taste calls in the inventory and the place where small numeric moves change the felt energy of every image. It is the affect engine's calibration pass.
3. **The articulation / note-length window (C9.1) — the literal source of "ethereal."** *Attack third.* The hold-fraction window (`ARTIC_WINDOW_LO=0.55`, `ARTIC_WINDOW_HI=1.10`) governs whether notes sound sung, detached, or floaty. Too-long sustains read ethereal; too-short read mechanical. The catalogue names this the HEART of the ethereal verdict and a taste call MT cannot adjudicate. It is a tight, single-knob chunk with a high ear-payoff, and it is the most direct lever on the "ethereal" half specifically.
4. **Latent per-image signal thrown away (C1.12–1.16) + dead correspondence tables (C2.11–2.16).** *Attack fourth.* `subject_hue`, `subject_saturation`, `mass_centroid` are real per-image information extracted then discarded; orientation→interval, shape→ostinato, per-pixel pitch are documented intent the runtime silently ignores. Every discarded signal is a way the music fails to track the image. Lower than 1–3 because wiring new signal is a build, not a tuning, and its per-knob payoff is smaller than mode/tempo — but collectively it is the root of "image-unrelated."
5. **The inert orchestration generators — walking bass, pedal, oom_pah_pah (C11.7–11.9).** *Attack fifth.* Built and tested but no rule selects them; activating them with affect/energy triggers measurably increases rhythmic image-relatedness and is the catalogue's explicitly-assigned arc deliverable. Lowest of the five because the machinery exists and only the trigger predicates need authoring — high-confidence, but a smaller felt delta than mode or note-length.

**Net first move:** mode (C6.6) and the affect tuning (C6) together are the loudest, cheapest, most verdict-aligned wins — they lead. The note-length window (C9.1) is the dedicated "ethereal" fix. Wiring latent image signal (C1/C2-tail) and activating the inert generators (C11) are the dedicated "image-unrelated" fixes and follow once the affect engine that should DRIVE those triggers is itself tuned.

### The ranked review-chunk list

Ordered. TIER 1 chunks lead because they most directly attack the verdict; TIER 2 chunks are lower-impact mechanics or already-successful clusters reviewed after the verdict-movers are settled. Note the deliberate sequencing dependency: the affect engine (Chunks 1–2) is tuned BEFORE the chunks that consume affect as a trigger (Chunks 5–6), so those later passes select on a calibrated signal rather than a moving one.

---

**Chunk 1 — Valence owns major/minor (build C6.6; touches C4.1, C4.4).**
**Tier:** TIER 1 (lead).
**Why this priority:** Mode is the single largest affect cue (effect size 0.29). Today it is the one load-bearing affect parameter that is NOT affect-driven — major/minor still comes from hue (C4.1), which the research base rates LOW-confidence and culturally contingent. This is the most audible single mismatch between what the image expresses and what the listener hears, and the fix is designed already (C6.6). Highest expected ear-payoff per pass in the whole plan.
**What the pass will likely decide:** Should valence own the major/minor split (per the design caveat) with hue→mode demoted to a colorist garnish that only chooses *which* major-family or minor-family mode? Where is the valence threshold for the major/minor flip, and is there a neutral band (mirroring the character ladder's neutral→Ballad fall-through)? How does this reconcile with the byte-frozen realizer slice that deferred mode (the build constraint MT must clear)?

**Chunk 2 — Affect engine calibration (C6.1, C6.2, C6.3, C6.4, C6.5; with C6.7).**
**Tier:** TIER 1.
**Why this priority:** This cluster owns the "energetic/joyful/fast images don't sound that way" half of the verdict outright, and tempo is the second-largest affect cue (0.14). The mechanism (composite + de-cap) is built, but every number in it — the convex arousal weights (0.45/0.25/0.20/0.10), the valence weights, the character arousal×valence thresholds, the per-character tempo windows, the prominence nudge spans — is an ear-tuned taste value. Small moves here re-color every image, so it is the highest-density taste pass. Sequenced second so the mode flip from Chunk 1 is in place when the character/tempo ladders are re-judged on real images.
**What the pass will likely decide:** Do the arousal weights produce the right FELT energy on real photos (is saturation over-/under-weighted)? Apply the two flagged-but-un-applied tempo-window tweaks (march max 132→126, hymn max 92→84)? Are the character thresholds placed so the common photo lands somewhere expressive rather than defaulting to Ballad? Should the dead-center neutral keep falling to Ballad, or to a less "ethereal" default character?

**Chunk 3 — Articulation & note-length window (C9.1; with C9.4, C9.3).**
**Tier:** TIER 1.
**Why this priority:** This is the dedicated fix for the "ethereal" half. The LO/HI hold-fraction window is the literal control over sung-vs-floaty-vs-mechanical, named in the catalogue as the HEART of the ethereal verdict and the prime taste call MT defers. Tight single-knob chunk, very high ear-payoff, and it isolates the "ethereal" complaint from the affect work so the two can be judged independently.
**What the pass will likely decide:** Is the current window (0.55–1.10) right, or do the long sustains still read ethereal (lower HI) and the short detachments mechanical (raise LO)? Should the window be affect-conditioned (calmer/lower-arousal images sing longer, higher-arousal detach more — linking it to Chunk 2's arousal) rather than driven by `edge_activity` alone? Does the pad-overlap cap (C9.4) hold the harmony too short to feel sustained? *Anticipated Aesthetics-vs-Theory tension flagged below.*

**Chunk 4 — Macro-shape recognizability: returning theme + form/meter identity (C15.2, C3.5, C15.1; C3.2, C3.8).**
**Tier:** TIER 1.
**Why this priority:** This is the dedicated fix for the "structureless" half that the structural cure (C3.1, the killed loop) did NOT solve: the returning theme is present but, by the operator's own S16 verdict, "not so musical / hard to tell it recurred." A return you cannot hear is not a felt structure. Memorability/singability of the motif is exactly the "reads as intended" layer MT defers to taste. Meter frozen at 4/4 (C3.8) means rhythmic identity never varies by image — a structural sameness contributor. Tier 1 because a recognizable return is the strongest single source of "this has a shape."
**What the pass will likely decide:** What makes the motif MEMORABLE (rhythmic profile, contour distinctiveness, register placement) so its A′ return is audibly a homecoming? Should the contour archetype be more strongly image-seeded? Are the form-selection predicates (C3.2) aesthetically right (does a complex image "deserve" theme-and-variations), and should meter ever leave 4/4 (the closed enum awaiting code)?

**Chunk 5 — Image-relatedness I: wire latent per-image signal + retire/honor dead correspondences (C1.12–1.16; C2.11–2.16; C5.1).**
**Tier:** TIER 1.
**Why this priority:** The root of the "image-unrelated" half. Six extracted-or-reserved features are inert (C1.12–1.16) — `subject_hue`, `subject_saturation`, `mass_centroid` are real per-image information thrown away — and a long tail of legacy tables (C2.11–2.16) documents correspondences (orientation→interval, shape→ostinato, per-pixel pitch) the pipeline silently does not honor. The RNG-within-family progression choice (C5.1) means even harmony only weakly tracks the image. Sequenced after the affect engine because some of these signals should feed affect/character, which must be tuned first. Tier 1 because it is the densest source of "unrelated," but ranked below the concentrated ethereal/affect levers because each individual wiring is a smaller felt delta and is a build, not a tuning.
**What the pass will likely decide:** Which inert features to WIRE vs. delete (does subject_hue/subject_saturation drive a per-section color knob; does mass_centroid drive register?)? Are the dead legacy tables (C2.11–2.16) RESURRECTED, RE-SPECIFIED to the current plan+section model, or DELETED as misleading dead intent? Should progression selection become deterministic on a feature instead of RNG-within-family (C5.1), so the same image reliably yields the same harmony?

**Chunk 6 — Image-relatedness II: activate the inert orchestration generators (C11.7, C11.8, C11.9; with C11.6).**
**Tier:** TIER 1.
**Why this priority:** Walking bass, pedal, and oom_pah_pah are built, tested, and ready but no rule selects them — the catalogue names their activation the arc's explicit deliverable. Activating them with affect/energy triggers measurably increases rhythmic and textural image-relatedness. Last of Tier 1 because the machinery exists (only trigger predicates need authoring — lowest build risk) and it consumes the affect signal that Chunks 1–2 calibrate, so it must follow them.
**What the pass will likely decide:** Activate walking/pedal with which triggers — adopt MT's proposed predicates (walking: arousal≥0.55 ∧ subject_energy≥0.50; pedal: arousal≤0.28 ∧ colorfulness≤0.30) or tune them by ear? Route oom_pah_pah (and the inert dominant pedal `pedal_dom`) or leave parked? Do the new textures fight or reinforce the character chosen in Chunk 2 (e.g., does walking bass under a Nocturne read wrong)?

---

**Chunk 7 — Chromatic color & key-plan trigger tuning (C13.1–13.4; C14 trigger thresholds).**
**Tier:** TIER 2.
**Why this priority:** Both clusters are success stories per the catalogue — chromatic color is well image-tied and theory-sound (C13), and the key-plan arc (C14, S25–S29) already passed an operator re-listen for audible travel-and-return. They are NOT verdict-movers; they are well-crafted and image-tied already. Only the trigger thresholds (minor-iv 0.25, bVI 0.45, secondary-dom 0.55; fg_bg 0.25, valence cuts, hue 60°) are ear-tunable. Tier 2: refine after the verdict-movers, because moving these changes a system that already sounds intentional.
**What the pass will likely decide:** Are the chromatic-color trigger thresholds firing on the right images (too often = restless, too rare = bland)? Do the key-plan affective directions still reinforce (not fight) the mode flip introduced in Chunk 1?

**Chunk 8 — Image-independent expressive shaping: dynamics & rhythm mechanics (C7.2–7.7; C8.1–8.2).**
**Tier:** TIER 2.
**Why this priority:** Dynamics are well-crafted but almost entirely structural/image-independent — the same tasteful phrase shape regardless of image. It reinforces "image-unrelated" passively, but the contour itself sounds good (craft), so re-crafting it is low-payoff next to wiring NEW image signal (Chunk 5). Rhythm is one of the better image-tied clusters already; only band-threshold placement (C8.1) and vocabulary richness are open, both minor taste. Tier 2: the magnitudes (swell ±4, accents +9/+2/−6) are "reads as expressive vs. mechanical" taste calls worth a pass, but only after the verdict-movers.
**What the pass will likely decide:** Do the swell/accent magnitudes read as expressive or mechanical? Are the rhythm-pattern band thresholds (arpeggio >0.80, syncopated 0.55–0.80, etc.) placed well, and is the 5-pattern vocabulary rich enough?

**Chunk 9 — Voice-leading, counterpoint, cadence & phrase mechanics (C10.1–10.7; C12.1–12.3).**
**Tier:** TIER 2.
**Why this priority:** The most theory-rigorous, most QG-verified clusters in the inventory, and image-INDEPENDENT by design (craft, not correspondence). They do not contribute to "image-unrelated" — they were never meant to be image-driven — and the catalogue rates them sound. The one open taste call already resolved in the operator's favor (the GAP-2 "keep the bite" prepared dissonance, C10.7). Tier 2, reviewed last: this is craft to AUDIT for ear-acceptability, not a verdict lever to move.
**What the pass will likely decide:** Does the counterpoint, when it appears (saliency-gated), sound intentional to the ear or merely legal? Is the GAP-2 terminal "bite" still the operator's preference? Any cadence that lands weak to the ear despite being theory-correct?

**Chunk 10 — Plumbing & engine-equivalence constraints (C16; the byte-freeze).**
**Tier:** TIER 2 (reference, not a review target).
**Why this priority:** Not a music rule — it is the interface and the behavioral freeze that explains why so many image-tied rules are `SELECTED` not `LIVE`. No aesthetic decision lives here. Listed last only so each earlier chunk's pass knows its build must respect the identity-path byte-freeze (the standing constraint on every realizer-touching change).
**What the pass will likely decide:** Nothing aesthetic. It is the constraint each build chunk above must honor (the identity/equivalence path stays byte-frozen).

### Reconciliation note (operating rule for the chunked passes)

Each S36+ chunk is reviewed by BOTH lenses in parallel — Aesthetics (this lens: pleasing/shaped/reads-as-image) and Music Theory (correctness/craft/legality). The operating rule:

- **Where Aesthetics and Music Theory AGREE** → that becomes the auto-applied starting default for the build of that chunk.
- **Where they CONFLICT** → BOTH positions are surfaced to the operator and the operator's trained ear decides. Neither lens auto-picks over the other, and a taste call is never silently overridden by a legality claim (or vice-versa). The GAP-2 "keep the bite" precedent (C10.7) — taste overriding the species objection — is the standing model for how a conflict resolves: surfaced, then operator-chosen.

**Conflicts I already anticipate (operator: expect these):**

- **Chunk 3 (articulation window) — likely tension.** Taste may want an articulation FLOOR (a minimum hold-fraction so notes stop reading "ethereal/uniformly short") that the craft lens views as a blunt clamp overriding the continuous `edge_activity` curve. The same tension runs the other way at the top of the window: taste may want shorter sustains to kill the floaty feel where theory is content with the legato. Surface both window endpoints as operator ear-tests, not as a single auto-applied number.
- **Chunk 1 (valence→mode) — likely tension.** Affect may want to FORCE major on a high-valence image whose hue-selected mode is minor/dark (e.g., hue lands Aeolian but valence says joyful). The theory lens may prefer to honor the hue-seeded mode for tonal-color reasons. The research base sides with valence owning the major/minor split, but the *garnish* mode choice (which specific church mode within the major or minor family) is where theory keeps authority — surface the split-vs-color boundary to the operator.
- **Chunk 2 (affect tuning) — latent tension.** A "fear" region (fast + minor) must stay SOFT, not loud (the musical-fear caveat, C6.8) — so an arousal→loudness rule that taste wants to push louder for energy can collide with the affect-correct soft-fear case. Flag any loudness-from-arousal move against the fear=soft guard.
- **Chunk 5 (dead tables) — process tension, not taste.** Whether the legacy correspondences are resurrected, re-specified, or deleted is partly an aesthetic call (would honoring orientation→interval actually please?) and partly a theory/architecture call (can it even be expressed in the plan+section model?). Expect this chunk to split into an aesthetic "is it worth hearing" question and a theory "is it buildable as specified" question, both surfaced.

---

## Cluster 1 — Image-feature extraction & normalization (the "image-relatedness" substrate)

This cluster is the bridge: every other cluster's image-relatedness depends on whether it reads a feature that is (a) actually extracted, (b) actually consumed. Several extracted features are computed-then-discarded — flagged below as those are exactly the "image-unrelated" suspects.

| # | Rule | Where | Session | Driven by | Image-rel | Wiring | MT note |
|---|---|---|---|---|---|---|---|
| 1.1 | `ImageUnderstanding` = 28 perceptual scalars extracted pure-Rust (no OpenCV) | `pure_analysis.rs::understand_image_pure` → struct in `composition.rs:39-115` | S11/S13/S15 | image pixels | yes-strong | LIVE | The substrate. Whole-image scan, not subject-aware until S18 regions. |
| 1.2 | Feature normalization: edge `/0.05`, texture `/2000`, shape_complexity `/2`, hue_spread `/1.0`, brightness/sat `/100` → 0..1 | `mappings.json: global.feature_normalization`; consumed `chord_engine.rs:196-219` | S13 | fixed (calibration) | n/a | LIVE | S13 found raw edge (≈0.005–0.05 on real photos) never crossed thresholds; normalization is what made edge/texture/complexity *able* to drive anything. The `/0.05` edge max is a load-bearing calibration constant — set wrong, every edge-driven rule mis-fires. |
| 1.3 | `avg_brightness` (Rec.601 luma mean, 0..100) | `pure_analysis.rs` `hsv_means`/`to_gray` | S11 | brightness | yes-strong | LIVE | Drives tempo + valence. Sound. |
| 1.4 | `avg_saturation` (arithmetic HSV-S mean, 0..100) | `pure_analysis.rs` `hsv_means` | S11 | saturation | yes-strong | LIVE | Drives harmonic complexity + arousal. Sound. |
| 1.5 | `dominant_hue` (circular mean of pixel hues, 0..360) | `pure_analysis.rs` `hsv_means` (circular) | S11 | hue | yes-strong | LIVE | Drives mode + tonal home seed. Circular mean correct for hue. |
| 1.6 | `colorfulness` = circular stddev of hue (`hue_spread_pure`) | `pure_analysis.rs:222-245` | S13 | hue variety | yes-strong | LIVE | Drives bVI mixture + arousal. Correct hue-dispersion metric. |
| 1.7 | `edge_activity`, `texture`, `complexity` (Canny density / Laplacian var / connected-component count) | `pure_analysis.rs` `edge_density_pure`/`laplacian_var_pure`/`shape_complexity_pure` | S13 (revived; were dead) | image structure | yes-strong | LIVE | The "dead S13 features" — extracted long before, discarded until S13 wired them. complexity uses connected-components ≠ OpenCV contours (honest delta, tuned by ear — a taste call MT cannot adjudicate). |
| 1.8 | `value_key` = `clamp(1 - avg_brightness/100)` (darkness) | `composition.rs:58-59` | S13 | brightness | yes-strong | LIVE | Used in form/key-scheme selection. Sound inversion. |
| 1.9 | Region/saliency pass: 3×3 rule-of-thirds, `analyze_regions_pure` + `pick_subject_region` (center-surround blend, weights 0.5/0.35/0.15) | `pure_analysis.rs:451-586` | S18 | subject vs surround | yes-strong | LIVE | Fills `subject_size`/`subject_hue`/`subject_saturation`/`fg_bg_contrast`/`mass_centroid`/`quadrant_contrast`/`vertical_emphasis`. Locked weights mean a flat corner blob can only TIE the center (ties → center) — narrow honest blind spot. Whether 3×3 is fine enough to "read the subject" is a taste/perception call MT flags for aesthetics. |
| 1.10 | `subject_energy`/`foreground_energy`/`background_energy` (per-band edge density) | `pure_analysis.rs:719-735` | S18 | regional activity | yes-strong | LIVE | Drives texture/counter selection. Sound. |
| 1.11 | `foreground_brightness`/`background_brightness`/`foreground_hue`/`background_hue` (per-region affect via `band_affect`) | `pure_analysis.rs:801-834` | S26 | per-region color/value | yes-strong | LIVE | Lets B and C excursions read their OWN region (not whole-image avg). Reuses S18 cell stats, no new pass. Sound. |
| 1.12 | **`dominant_hue_mass` (hardcoded 1.0)** | `composition.rs:50-51` | S15 | fixed | **no** | AUTHORED-BUT-INERT | Computed-then-discarded; mono-color assumption. No Knob reads it. |
| 1.13 | **`secondary_hue` (== dominant_hue)** | `composition.rs:52-53` | S15 | fixed | **no** | AUTHORED-BUT-INERT | Fallback only; no Knob. |
| 1.14 | **`palette_bimodality` (hardcoded 0.0)** | `composition.rs:54-55` | S15 | fixed | **no** | AUTHORED-BUT-INERT | `Knob::PaletteBimodality` exists and the `aaba` form rule reads it, but the value is constant 0.0 → the predicate `le 0.3` is ALWAYS true, so it contributes nothing discriminating. Stage-8 upgrade deferred. |
| 1.15 | **`subject_hue` / `subject_saturation` (computed, no Knob)** | `composition.rs:76-79`; `pure_analysis.rs` region pass | S18 | subject region | **no** | AUTHORED-BUT-INERT | Extracted from the subject cell but no Knob/rule consumes them. Latent image signal left on the floor. |
| 1.16 | **`mass_centroid` (computed, no Knob)** | `composition.rs:65-66`; `pure_analysis.rs:675-690` | S18 | luminance centroid | **no** | AUTHORED-BUT-INERT | Computed, carried in struct, read by NO planner rule. Pure dead image signal. |
| 1.17 | **affect sentinels `affect_arousal`/`affect_valence` default -1.0** | `composition.rs:105-114, 150-151` | S22 | n/a (planner-filled) | n/a | LIVE | Not pixel-extracted; the planner computes them (Cluster 6) and seats them before the character/tempo ladders run. The -1.0 sentinel guarantees an unfilled value never spuriously matches a 0..1 predicate. |

**Failure-mode contribution:** This cluster is the ROOT of the "image-unrelated" half of the verdict. Six extracted-or-reserved signals (1.12–1.16) are inert — including `subject_hue`, `subject_saturation`, and `mass_centroid`, which are real per-image information thrown away. The more the music ignores extractable image content, the more it reads as image-unrelated. Prime aesthetics target: decide which inert features to wire.

---

## Cluster 2 — Image-feature → musical-parameter mappings (the direct correspondences)

The headline "which feature drives which parameter" layer. Each row is a single image→music correspondence; deeper mechanics live in their own clusters.

| # | Rule | Where | Session | Driven by | Image-rel | Wiring | MT note |
|---|---|---|---|---|---|---|---|
| 2.1 | hue → mode (6-band table: 0-30 Phrygian / 31-90 Lydian / 91-150 Ionian / 151-210 Dorian / 211-270 Aeolian / 271-330 Mixolydian) | `mappings.json: global.hue_to_mode`; `mapping_loader` lookup; planner `composition.rs:1186` | S2 (modes fixed); table S13 | `dominant_hue` | yes-strong | LIVE | The six-mode church-mode spread is **expressive convention, not validated affect science**. MT cannot adjudicate whether a given hue "should" map to a given mode on taste grounds — flagged repeatedly. Affect overlay should note: only the major/minor (valence) split is empirically grounded; hue→mode is colorist garnish. |
| 2.2 | saturation → harmonic complexity (0-30 TriadsOnly / 31-70 +7ths / 71-100 +7ths+extensions) | `mappings.json: global.saturation_to_harmonic_complexity`; `chord_engine.rs HarmonicComplexity::from_saturation01` | S13 | `avg_saturation` | yes-strong | LIVE | Musically meaningful (more color → richer chords). Diatonic 7ths/9ths, QG hand-verified. Sound. |
| 2.3 | brightness → tempo (continuous interp over anchors 72/108/150 BPM; dark ≈877ms → bright ≈502ms/step) | `mappings.json: global.brightness_to_tempo_bpm`; `composition.rs::interp_tempo_bpm:1749-1778` | S13 (de-capped S22) | `avg_brightness` | yes-strong | LIVE | S13 made tempo image-driven (was hardcoded); S22 de-capped (was 120-cap + Ballad clamp). Whether brightness is the *right* tempo driver vs arousal is a taste fork — see 6.x. |
| 2.4 | edge_activity → rhythmic density / note rate | `chord_engine.rs::realize_rhythm`; `mappings.json: instrument_section.edge_density_to_rhythm` | S6/S13 | `edge_activity` | yes-strong | LIVE | Visual activity → musical activity, perceptually meaningful. Mechanics in Cluster 8. |
| 2.5 | edge_activity → continuous articulation / note-length | `chord_engine.rs` artic curve | S13 | `edge_activity` | yes-strong | LIVE | The S13 fix for "uniformly-short computer notes." Mechanics in Cluster 9. The extremes were flagged unpleasant (S13 re-listen) — a taste call. |
| 2.6 | edge_complexity → secondary-dominant substitution (trigger 0.55) | `mappings.json: global.dominant_substitution_trigger`; `chord_engine.rs secondary_dominant_of` | S13 | `edge_activity` | yes-strong | LIVE | Mechanics in Cluster 13. |
| 2.7 | brightness_drop → modal interchange / borrowed chords (trigger 0.25; bVII/iv/bVI) | `mappings.json: global.modal_interchange_trigger`; `chord_engine.rs borrowed_minor_iv`; planner `brightness_drop:1241` | S13 | `avg_brightness` (drop) | yes-strong | LIVE | Mechanics in Cluster 13. |
| 2.8 | hue_spread / colorfulness → bVI mixture | `chord_engine.rs MODE_MIXTURE_THRESHOLD 0.45` | S13 | `colorfulness` | yes-strong | LIVE | Wide palette → borrowed bVI. Musically defensible. |
| 2.9 | saturation → velocity LEVEL (`-12 + (sat/100)*30` = -12..+18) | `chord_engine.rs::realize_velocity:1307` | S6 | `avg_saturation` | yes-weak | LIVE | Saturation sets the loudness baseline; the phrase CONTOUR (Cluster 7) shapes it. Image-relatedness is weak because it's a single scalar offset that the contour can swamp. |
| 2.10 | brightness → register / octave lift (melody +12/oct, fill +6/oct, bass -12 dark-only) | `chord_engine.rs role_pitch:1215,1232,1266` | S6/S13 | `avg_brightness` | yes-weak | LIVE | brighter → higher. A correct cross-modal correspondence, but small and per-role; weak audible image link. |
| 2.11 | stillness → cadence type (high_motion → Deceptive, low_motion → Authentic; thresh 0.15) | `mappings.json: global.cadence_trigger` | S13 | motion/edge | yes-weak | **AUTHORED-BUT-INERT** | The `cadence_trigger` table exists in mappings.json but the realizer's cadence type is set structurally by `plan_phrases`/`boundary_cadence` (Cluster 12), not by this table. No live consumer found. |
| 2.12 | line_orientation → interval (horizontal Stepwise / vertical Leaps / diagonal Mixed) | `mappings.json: instrument_section.line_orientation_to_interval` | S2-era | edge orientation | **no** | **AUTHORED-BUT-INERT** | Legacy pre-port mapping table; no edge-orientation feature is extracted by `understand_image_pure`, no consumer. Dead. |
| 2.13 | contrast → articulation (low Legato / med Portato / high Staccato) | `mappings.json: instrument_section.contrast_to_articulation` | S2-era | contrast | **no** | **AUTHORED-BUT-INERT** | Superseded by the S13 continuous curve (2.5); the discrete table is dead data. |
| 2.14 | color_shift → chord extension (add9/6, maj9/min9, 13/b13/#11) | `mappings.json: instrument_section.color_shift_to_chord_extension` | S2-era | color shift | **no** | **AUTHORED-BUT-INERT** | No consumer; extensions are driven by saturation complexity (2.2) instead. Dead. |
| 2.15 | texture → modal color (smooth StayInMode / medium BorrowFromParallel / rough SecondaryDominant) | `mappings.json: instrument_section.texture_to_modal_color` | S2-era | texture | **no** | **AUTHORED-BUT-INERT** | No live consumer; modal color is driven by 2.6/2.7. Dead. |
| 2.16 | pixel-Y → pitch height; pixel brightness → velocity (MapTopToHighPitch / DirectMapping) | `mappings.json: fine_detail` | S2-era | per-pixel | **no** | **AUTHORED-BUT-INERT** | The pre-port per-pixel scan model; the current pipeline is plan+section based. Entire `fine_detail` block (incl. `local_jaggedness_to_chromaticism`, `shape_to_ostinato`) is dead. |

**Failure-mode contribution:** The LIVE rows (2.1–2.10) are the real image→music spine and are mostly sound craft. But note the long tail of `AUTHORED-BUT-INERT` legacy tables (2.11–2.16) — they document INTENT (orientation→interval, shape→ostinato, per-pixel pitch) that the current pipeline does NOT honor. The gap between what the mappings.json *says* it does and what runs is itself a likely contributor to "image-unrelated": features like edge orientation and shape that a viewer expects to hear are silently unmapped. The two weak rows (2.9 saturation→level, 2.10 brightness→register) are weak precisely because they're scalar offsets a strong contour/floor can mask.

---

## Cluster 3 — Macro-form & structure

| # | Rule | Where | Session | Driven by | Image-rel | Wiring | MT note |
|---|---|---|---|---|---|---|---|
| 3.1 | Plan computed ONCE per image → non-looping `StepPlan` walked start-to-finish (the `plan[step_idx % len]` loop is DEAD) | `composition.rs CompositionPlan::locate:1005-1017`; engine cursor 0→total_steps | S15 | structural | n/a | LIVE | The structural cure. Before S15 the realizer looped a phrase modulo its length → structureless texture. `locate()` walks sections with no wrap. Load-bearing. |
| 3.2 | Form selected by SelectTable over image features; 6 forms (rounded_binary default / ternary_aba / aaba / abac / abbac / theme_and_variations) as DATA `FormSpec` rows | `mappings.json: composition.form_catalogue + form`; `composition.rs FormSpec, lookup_form` | S15 | complexity, edge_activity, quadrant_contrast, aspect_ratio, palette_bimodality, vertical_emphasis, value_key | yes-strong | SELECTED | Form-as-data, deterministic first-match-wins selection. The form-selection PREDICATES are a taste call (does a high-complexity image "deserve" theme-and-variations?) MT cannot adjudicate. Note 1.14: the `aaba` rule's `palette_bimodality le 0.3` predicate is always-true (constant feature). |
| 3.3 | Sections expanded with rel_len step allocation; last section absorbs rounding (cursor sums exactly to total_steps) | `composition.rs:1281-1358` | S15 | structural | n/a | LIVE | Sound. |
| 3.4 | `total_steps` = (8 base + edge_activity*8) × section count | `composition.rs BASE_STEPS_PER_SECTION=8, activity_bonus:1209-1212` | S15 | `edge_activity` | yes-weak | LIVE | Busier images get modestly longer pieces. Weak audible link. |
| 3.5 | Returning theme: stated in A (Statement), absent/head-fragment in B (Contrast), Identity recap in A′ (Return) | `chord_engine.rs theme_melody_pitch:2472`; `composition.rs ThemeSeed, themes` | S15 | structural (theme picked from image, Cluster 4) | yes-weak | SELECTED | The motif RETURNS (degree-sequence match, not regenerated) — QG-verified. But operator S16 verdict: motif "not so musical / hard to tell it recurred." The recognizability of the theme is a TASTE call MT flags — prime aesthetics target. |
| 3.6 | theme behaviour selection (absent / fragment) | `mappings.json: composition.theme_behaviour`; planner `:1217-1228` | S15 | `complexity` (ge 0.4 → fragment) | yes-weak | SELECTED | "second_theme" behaviour referenced in code paths but no active rule emits it. |
| 3.7 | Section density as a compositional device (thinner B, fuller A′) set from region energy | see 6.10 (MX-4) | S29 | region energy | yes-strong | LIVE | Cross-listed; the structural use of density. |
| 3.8 | Meter selection table (four4 default; three4/six8/two4 enum) | `mappings.json: composition.meter`; `composition.rs Meter, parse_meter` | S15 | none (pinned) | **no** | **AUTHORED-BUT-INERT** | `meter` SelectTable has empty `rules:[]` → always four4. Meter>4/4 is a closed enum awaiting Stage-3 code. Every piece is 4/4 today. |

**Failure-mode contribution:** This cluster is the structural cure (3.1 killed the loop) and is the strongest answer to the "structureless" half of the verdict. But two residuals feed it: the returning theme (3.5) is present-but-not-recognizable (operator's own complaint), and meter is frozen to 4/4 (3.8) so rhythmic identity never varies by image. Form selection predicates (3.2) are unvalidated taste.

---

## Cluster 4 — Mode & tonality

| # | Rule | Where | Session | Driven by | Image-rel | Wiring | MT note |
|---|---|---|---|---|---|---|---|
| 4.1 | Six mode interval patterns: Ionian [0,2,4,5,7,9,11] / Dorian [0,2,3,5,7,9,10] / Phrygian [0,1,3,5,7,8,10] / Lydian [0,2,4,6,7,9,11] / Mixolydian [0,2,4,5,7,9,10] / Aeolian [0,2,3,5,7,8,10] | `chord_engine.rs IONIAN..AEOLIAN consts:11-26` | S2 (mode-collapse fix) | hue (selects which) | yes-strong | LIVE | All six modes correct interval patterns; S2 fixed the mode-collapse bug + two numeral→degree dead-arms. Theory-correct. |
| 4.2 | Default tonal root = MIDI 60 (C4), seeded as home; re-rooted per section by key offset | `chord_engine.rs root_midi default 60`; `composition.rs home_root_midi:1188` | S2 | fixed (home), hue (mode) | yes-weak | LIVE | Home pitch is fixed C; only the MODE and key OFFSETS vary by image. Two images in the same hue band start on the same pitch — a weak image link MT notes. |
| 4.3 | Roman-numeral → scale degree mapping (I..vii → 0-based degree) | `chord_engine.rs roman_degree:96` | S2 | structural | n/a | LIVE | Correct after S2 dead-arm fixes (IV→deg3, iii→deg2). |
| 4.4 | Character is the load-bearing affect-character (NOT hue) — 10-variant `Character` enum; selected by arousal/valence | see Cluster 6 | S22 | affect | yes-strong | SELECTED | Cross-listed. The S22 fix: valence (not hue) owns the major/minor-feeling character. |

**Failure-mode contribution:** Modes are theory-correct, but the load-bearing caveat (carried since S21) is that hue→mode (4.1 selection) is *expressive convention*, not validated affect — relying on it for the image→emotion read is part of why bright/energetic images didn't feel right before S22 moved the affect read to arousal/valence. The fixed home pitch (4.2) weakens per-image tonal distinctness.

---

## Cluster 5 — Harmony & harmonic vocabulary

| # | Rule | Where | Session | Driven by | Image-rel | Wiring | MT note |
|---|---|---|---|---|---|---|---|
| 5.1 | Progression families warm/cool/neutral, mode-family-selected (`pick_progression`); chord-symbol progression lists | `chord_engine.rs pick_progression:119`; `mappings.json: global.progression_families` | S2 | mode (→ family) | yes-weak | LIVE | Family chosen by mode (warm=Ionian/Lydian/Mixolydian, cool=Dorian/Aeolian/Phrygian); WHICH progression within a family uses the craft-layer RNG (the documented S9 boundary the equivalence net isolates). So two same-mode images can get different progressions non-deterministically — image-relatedness is weak. The RNG-within-family is a known taste/identity question MT flags. |
| 5.2 | Triad → 7th → 9th by saturation (`roman_to_chord_complex`, `HarmonicComplexity`) | `chord_engine.rs:302, HarmonicComplexity:46-64` | S13 | `avg_saturation` | yes-strong | LIVE | Diatonic, QG hand-verified. Sound. |
| 5.3 | 7 idiomatic progression rows (axis rotations, circle-of-fifths, descending-thirds, doo-wop, lament/Andalusian approximations) | `mappings.json: global.progression_families` (warm/cool/neutral lists); `composition.rs` | S30 | mode-family | yes-weak | SELECTED | Deepened catalogue, deterministic selection, additive. Idiomatic content. Whether these read as intended on a given image = taste. |
| 5.4 | Secondary dominant tonicization | see Cluster 13 | S13 | edge_activity | yes-strong | LIVE | Cross-listed. |
| 5.5 | Modal interchange / minor iv / bVI | see Cluster 13 | S13 | brightness, colorfulness | yes-strong | LIVE | Cross-listed. |

**Failure-mode contribution:** Harmonic vocabulary is genuinely deepened (S30) and theory-sound, but the within-family RNG selection (5.1) means harmony is only *weakly* tied to the image — the family tracks mode/hue but the specific progression is random. For an operator listening for "this image → this music," that randomness dilutes image-relatedness. Aesthetics may want to make progression selection deterministic on a feature.

---

## Cluster 6 — Affect bridge (image → valence/arousal → character + tempo)

The S22 bridge that no earlier cluster owned. This is the cluster the affective overlay is most centrally about.

| # | Rule | Where | Session | Driven by | Image-rel | Wiring | MT note |
|---|---|---|---|---|---|---|---|
| 6.1 | Arousal composite = 0.45·(sat/100) + 0.25·colorfulness + 0.20·edge_activity + 0.10·complexity | `composition.rs affect_composite:231-260`; `mappings.json: composition.affect.arousal_weights` | S22 | saturation, colorfulness, edge, complexity | yes-strong | LIVE | The missing arousal pool — the direct fix for "always a ballad." Weights are convex (sum 1.0). Whether these weights produce the right *felt* energy is the central aesthetics-review question; MT defers the weight tuning to taste. |
| 6.2 | Valence composite = 0.70·(bright/100) + 0.20·(sat/100) + 0.10·(0.5+0.5·fg_bg_contrast) | `composition.rs affect_composite`; `mappings.json: composition.affect.valence_weights` | S22 | brightness, saturation, fg_bg_contrast | yes-strong | LIVE | Brightness-led valence (empirically grounded). The fg_bg fluency term is LOW-confidence garnish. Weight tuning = taste. |
| 6.3 | Character selected by arousal×valence ladder: Scherzo (a≥0.60,v≥0.55) / March (a≥0.60,v<0.45) / Lament (a≤0.30,v<0.35) / Hymn (a≤0.30,v≥0.55) / Nocturne (a≤0.35,v in 0.35-0.47) / else Ballad | `mappings.json: composition.character`; `composition.rs Character enum, parse_character` | S22 | affect (arousal,valence) | yes-strong | SELECTED | 6 of 10 enum variants reachable (Drone/Waltz/Lilt/Gigue declared, unrouted). The dead-center neutral falls through to Ballad (S22 calibration fix). The threshold placement is a taste call; MT concurred the neutral→Ballad intent. |
| 6.4 | Per-character tempo windows clamp raw BPM (ballad 56-96 / scherzo 120-168 / march 96-132 / lament 44-66 / hymn 60-92 / nocturne 50-80) | `mappings.json: composition.affect.character_tempo`; `composition.rs character_tempo_bpm:268-274` | S22 | affect (via character) + brightness (raw BPM) | yes-strong | LIVE | De-caps the old 120/Ballad clamp. Two MT ear-tuning candidates flagged un-applied: march max 132→126 (132 reads as galop), hymn max 92→84 (chorale tempo). Prime aesthetics targets. |
| 6.5 | Tempo de-cap: raw brightness→BPM (72/108/150 anchors) then per-character clamp; old `bpm.clamp(56,96)` Ballad window REMOVED | `composition.rs:725-728, interp_tempo_bpm`; `BALLAD_BPM_*` consts deleted | S22 | brightness | yes-strong | LIVE | The lifeless-ballad cap is gone. Whether a Scherzo at ~150 BPM "feels" joyful for example.jpg is the operator ear-test (still pending as of S22-S23 narrative). |
| 6.6 | Mode (major/minor) realization from valence — DEFERRED, not built | (design `design-s21-affect-mapping.md`) | S21 (designed) | valence | yes-strong | **AUTHORED-BUT-INERT** | The load-bearing caveat: valence SHOULD own major/minor, but S22 explicitly deferred MODE realization (chord_engine byte-frozen that slice). So today valence drives CHARACTER + tempo but the actual mode still comes from HUE (4.1). This is a known gap — character says "Scherzo" but the mode may be Aeolian if the hue lands there. Prime aesthetics/affect target. |
| 6.7 | Saliency → role prominence: salient subject pushes melody forward (louder/higher/freer), recessive regions recede | `mappings.json: composition.prominence + prominence_catalogue`; `chord_engine.rs prominence_weight:992` + 3 centered nudges (vel ±9, reg ±2, rhythm ±0.05) | S23 | `subject_size`, `fg_bg_contrast` | yes-strong | SELECTED | The first witnessed realizer change. Centered at weight 0.5 (byte-neutral). `subject_melody` profile selected on subject_size in 0.05-0.55 ∧ fg_bg ≥ 0.25. A subjectless field (example.jpg) realizes uniformly — by design, but it's also why example.jpg felt flat. The nudge SPANS (18/4/0.10) are ear-tuned values (VEL_SPAN finalized 16→18 by ear) — taste. |
| 6.8 | Fear-vs-anger caveat: in music fear = fast+minor+SOFT, anger = fast+minor+LOUD (don't naively map fear→loud) | (design caveat, `design-s21`) | S21 | — | — | AUTHORED-BUT-INERT (caveat, not a rule) | Documented design guard; not a code rule. Relevant to any future loudness-from-affect tuning. |

**Failure-mode contribution:** This is the cluster most directly targeting the "energetic/joyful/fast images don't sound that way" verdict, and S22/S23 are real progress. BUT: 6.6 is the biggest standing gap — valence drives character+tempo but NOT the actual major/minor mode (still hue-driven), so an "energetic" image can still be voiced minor. And the composite weights (6.1/6.2), threshold placements (6.3), tempo windows (6.4), and nudge spans (6.7) are ALL ear-tuned values MT explicitly defers to taste — making this the densest concentration of aesthetics-review targets in the inventory.

---

## Cluster 7 — Dynamics & velocity contour

| # | Rule | Where | Session | Driven by | Image-rel | Wiring | MT note |
|---|---|---|---|---|---|---|---|
| 7.1 | Velocity LEVEL from saturation (`-12 + (sat/100)*30`) | `chord_engine.rs realize_velocity:1307` | S6 | `avg_saturation` | yes-weak | LIVE | Baseline only; contour shapes it. (Cross-ref 2.9.) |
| 7.2 | Messa di voce phrase arch (`sin(PI·frac)·4.0`) | `chord_engine.rs:1320` | S6 | structural (phrase position) | no | LIVE | Phrase-level swell. Image-independent shaping. Theory-correct gesture. |
| 7.3 | Metric accent: +9 downbeat / +2 strong / -6 weak | `chord_engine.rs:1331-1336` | S6 | structural (meter) | no | LIVE | Correct accent pattern. Image-independent. |
| 7.4 | Phrase-end taper (-4 step before cadence) | `chord_engine.rs:1343` | S6 | structural | no | LIVE | Correct tapering gesture. |
| 7.5 | Structural velocity floors by phrase position (interior 76 / start 88 / cadence 96) | `chord_engine.rs V_INTERIOR/V_START/V_CADENCE:715-717` | S4 | structural | no | LIVE | Floor that the contour rides; verified realized variance > floor. |
| 7.6 | Role velocity offsets: melody +2, bass -1, pad -3 (non-cadence) | `chord_engine.rs:1351-1358` | S6/S17 | structural (role) | no | LIVE | Foregrounds melody, supports pad. Sound. |
| 7.7 | Cadence exemption (contour off at cadence) | `chord_engine.rs:1311` | S6 | structural | no | LIVE | Keeps cadential weight intact. |
| 7.8 | Prominence velocity nudge `+(w-0.5)·18` | see 6.7 | S23 | saliency | yes-strong | SELECTED | The only IMAGE-driven dynamics term. Cross-listed. |

**Failure-mode contribution:** Dynamics are well-crafted but almost entirely STRUCTURAL/image-independent (7.2–7.7 are `no`). Only saturation-level (weak) and the S23 prominence nudge tie dynamics to the image. A listener hears a tasteful phrase shape that is the SAME regardless of image — contributing to "image-unrelated." The contour itself sounds good (craft), but its image-blindness is an aesthetics concern. Whether the swell/accent magnitudes read as "expressive" vs "mechanical" is a taste call MT flags.

---

## Cluster 8 — Rhythm

| # | Rule | Where | Session | Driven by | Image-rel | Wiring | MT note |
|---|---|---|---|---|---|---|---|
| 8.1 | 5 rhythm patterns (sustained / arpeggio / dotted / syncopated / rest-as-gesture) selected by role × edge_activity × phrase-position | `chord_engine.rs realize_rhythm:1453, patterns:1569` | S6 | `edge_activity`, role | yes-strong | LIVE | Bands: arpeggio >0.80, syncopated 0.55-0.80, dotted 0.25-0.55, sustained <0.25. ≥3 distinct patterns per scan verified. Sound mechanism; band thresholds are tunable taste. |
| 8.2 | Harmonic-rhythm acceleration into cadences (2-onset root+pickup pre-cadence) | `chord_engine.rs:1669-1677` | S6 | structural | no | LIVE | Correct accelerando gesture. |
| 8.3 | HarmonicFill rest-as-gesture gated on normalized `edge_activity < FILL_REST_ACTIVITY (0.10)` | `chord_engine.rs FILL_REST_ACTIVITY:1436` | S17 (rest-bug fix) | `edge_activity` | yes-strong | LIVE | S17 fixed the bug where it read RAW edge (≈0.005-0.05) → inner voices silent nearly every step. Now calm images (≈0.08) keep the bed. |
| 8.4 | Section density → edge_activity nudge (`DENSITY_ACTIVITY_GAIN`) so excursions are audibly busier/sparser | `chord_engine.rs DENSITY_ACTIVITY_GAIN:1403` reads `Section.density` | S29 | region energy (via density) | yes-strong | LIVE | The MX-4 "scene change." Cross-ref 6.10/3.7. |
| 8.5 | Prominence rhythm-cutoff shift `(w-0.5)·0.10` on melody bands | see 6.7 | S23 | saliency | yes-strong | SELECTED | Cross-listed. |

**Failure-mode contribution:** Rhythm is one of the better image-tied clusters (edge_activity drives pattern + density). The main aesthetics question is band-threshold placement (8.1) and whether the 5-pattern vocabulary is rich enough — both taste. The "ethereal" complaint partly traces here historically (pre-S13 uniformly-short notes), largely addressed by the articulation curve (Cluster 9) + rest-bug fix (8.3).

---

## Cluster 9 — Articulation & note-length

| # | Rule | Where | Session | Driven by | Image-rel | Wiring | MT note |
|---|---|---|---|---|---|---|---|
| 9.1 | Continuous articulation/note-length curve: hold-fraction lerps `HI + (LO-HI)·edge_activity` (calm sings ~105% of step, busy detaches ~13%) | `chord_engine.rs ARTIC_WINDOW_LO=0.55, ARTIC_WINDOW_HI=1.10` (S15 re-scale) | S13 (curve), S15 (clamp 0.55-1.10) | `edge_activity` | yes-strong | LIVE | Replaced the 3-band staccato/portato/legato cutoff that produced "uniformly-short computer notes." The S15 clamp killed the unpleasant extremes (S13 re-listen). The exact LO/HI window is a TASTE call — too-long sustains read ethereal, too-short read mechanical; MT explicitly cannot adjudicate the felt result. PRIME aesthetics target. |
| 9.2 | Cadential ritardando (note lengthen `RITARDANDO_FACTOR=1.30`, cadence ring 1.20) | `chord_engine.rs RITARDANDO_FACTOR:1422` | S6 | structural | no | LIVE | Correct rit gesture at cadence. |
| 9.3 | Legacy discrete fractions STACCATO_FRAC 0.40 / PORTATO 0.70 / LEGATO 0.95 | `chord_engine.rs:1379-1381` | S6 | (superseded) | n/a | partially superseded | Constants retained; the continuous curve (9.1) is the live path. STACCATO_FRAC's old 0.40 was part of the S16 "half the melody missing" diagnosis. |
| 9.4 | Pad legato overlap cap `PAD_OVERLAP_FRAC=1.10` (beds tie step-to-step) | `chord_engine.rs PAD_OVERLAP_FRAC:1447` | S17 | structural | no | LIVE | The legato-overlap seam decision (no true cross-step sustain — scheduler blocks until last event; a multi-step hold would stretch tempo N×). A known fidelity compromise, documented. |

**Failure-mode contribution:** This cluster is at the HEART of the "ethereal" verdict. The articulation window (9.1) directly governs whether notes feel sung, detached, or floaty — and its calibration is exactly the taste call MT defers. The pad-overlap compromise (9.4) caps how sustained the harmony can feel. The aesthetics specialist's ear is the right adjudicator for the LO/HI window and the pad tie.

---

## Cluster 10 — Voice-leading & counterpoint

| # | Rule | Where | Session | Driven by | Image-rel | Wiring | MT note |
|---|---|---|---|---|---|---|---|
| 10.1 | Re-voicing pass: bass at index 0, upper voices ≤ P5 motion, common-tone retention, no parallel perfect 5ths/8ves compared at T AND T+1, per-voice state across the sequence | `chord_engine.rs voice_lead_sequence:573, voice_lead_one:4732, MAX_UPPER_VOICE_MOTION=7, has_parallel_perfects:2996` | S4 | structural | no | LIVE | Genuine voice leading (per-voice state, parallels at T AND T+1) — QG independently re-verified incl. throwaway re-impl. Theory-correct. |
| 10.2 | Upper-voice spacing hard-reject (no two upper voices share a MIDI note); fixed the IV=[65,65,65] unison collapse | `chord_engine.rs MIN_UPPER_VOICE_SPACING=1, upper_voices_well_spaced:2865` | S4/S6 | structural | no | LIVE | Correct. |
| 10.3 | Tendency-tone resolution = DELIBERATE LEAVE-EMERGENT (minimal-motion already resolves 7→1; no hard rule until 7th chords) | `chord_engine.rs` (design decision) | S4/S6 | structural | no | LIVE (by omission) | Documented decision; revisit when chordal 7ths are live. Sound rationale. |
| 10.4 | Independent counter-voice: fifth-species figure scorer `pick_counter_figure` (sustain/passing/neighbor/suspension), HARD-gated by counterpoint rules, PREF-scored contrary>oblique>similar>parallel | `chord_engine.rs pick_counter_figure:4307, harmonic_class:3639, rel_motion:3685, approach_perfect_is_legal:3738, melodic_leap_is_legal:3758` | S18 (stub), S30 (real) | structural + saliency (selection) | no (the line itself); yes-weak (when it appears) | SELECTED | The polyphony AudioHax never had. Contested decisions resolved: P4 dissonant only in 2-voice scorer; hidden/direct fifths strict; cambiata recognition-only. Theory-rigorous. |
| 10.5 | Counter cross-step memory via deterministic replay (`realized_prev_counter` recurses to section opening); gates the REALIZED line not a synthetic seed | `chord_engine.rs realized_prev_counter:3308, realized_counter_pitch_with_prev:3193` | S30 (critical fix) | structural | no | SELECTED | The S30 critical fix — gates were cosmetic until they checked the real prior pitch. Standing lesson: gate the sounding pitch, replay don't re-seed. |
| 10.6 | Held-period fill: off-beat onset at step_ms/4 + bounded held-run rotation (held C run sounds E→G→E→G, a MOVING inner line) | `chord_engine.rs held_run_position:3338, advancing_seed_counter:3375, consonance_gate_sustain:4112` | S18/S30 | structural | no | SELECTED | Answers the "empty periods" complaint. Deterministic rotation, no re-struck stab. |
| 10.7 | Band-reachability residuals GAP-2/3/4 disposition (counter band [55,67)) | `chord_engine.rs penult_for_clean_next:4257, clean_next_landing_exists:4202`, consonance gates | S30-S33 | structural | no | SELECTED | GAP-3 clean cadence; GAP-2 "keep the bite" (deliberate prepared terminal dissonance — an OPERATOR TASTE call, not MT); GAP-4 penult-rework (65→57 IV third, lands iii=59 contrary onto P8, no parallel fifth). GAP-2's "keep the bite" is explicitly a taste decision the operator made over MT's species objection. |

**Failure-mode contribution:** Voice-leading/counterpoint is the most theory-rigorous cluster and is image-INDEPENDENT by nature (it's craft, not correspondence — `no` on image-relatedness for the rules themselves). It does NOT contribute to "image-unrelated" (it was never supposed to be image-driven) but its very richness is gated behind saliency selection (10.4 only appears when foreground is busy+structured). The GAP-2 "bite" is the one place taste already overrode theory — a precedent for the aesthetics arc.

---

## Cluster 11 — Orchestration / texture / accompaniment patterns

| # | Rule | Where | Session | Driven by | Image-rel | Wiring | MT note |
|---|---|---|---|---|---|---|---|
| 11.1 | `OrchestralRole` 5-arm enum (Bass/HarmonicFill/Melody/Pad/CounterMelody); register floors C2/G3/G4 (36/55/67) | `chord_engine.rs OrchestralRole:835, role floors:1189-1191, role_pitch:1195` | S6 (3 roles), S17 (Pad), S18 (Counter) | structural (role) + brightness (register) | yes-weak | LIVE | Role registers derive from ROLE so a high instrument index can't alias onto bass. Sound stratification. |
| 11.2 | Plan-aware `assign_role` (delegates to `instrument_role` under identity; else maps inst_idx onto section orchestration layers) | `chord_engine.rs assign_role:1017`; `composition.rs to_orchestral_role bridge, LayerRole` | S17 | structural | no | LIVE | Identity path is the byte-freeze witness. |
| 11.3 | Sustained harmonic PAD: root-less inner tones (`notes[1..]` = 3rd/5th/7th, root skipped so bed never muds bass), held bed at -3 velocity, de-duped, ≤1.2× legato cap | `chord_engine.rs figured_bed/pad branch:1737, pad_voices` | S17 | structural | no | SELECTED | The S17 answer to "missing all the harmony / all the background." Sound voicing. |
| 11.4 | Orchestration-as-DATA: `OrchestrationProfile{id,layers,density,pad_voices,figuration,bass_pattern,prominence}` + `texture` SelectTable | `composition.rs OrchestrationProfile:380-425`; `mappings.json: composition.texture_catalogue + texture` | S17 | image (selection) | yes-strong | SELECTED | Texture selected per image. Profiles: pad_bed (default), pad_bed_counter, pad_figured, pad_broken_up/wave, pad_arp_waltz, pad_block_comp, pad_oom_pah, pad_stride, pad_walking, pad_pedal. Which texture for which image = taste. |
| 11.5 | Figuration (Alberti & accompaniment idioms): held chord animates into 2-4 onset burst WITHIN one step | `chord_engine.rs figured_bed:2206`; `composition.rs FigurationSpec/FigurationOnset`; `mappings.json figuration_catalogue` | S20 (Alberti), S30 (broken/waltz/comp), S34 (oom-pah/stride) | image (texture selection) | yes-weak | SELECTED | Figures: alberti, broken_chord_up/wave, arp_waltz, block_comp_24, oom_pah, oom_pah_pah, stride. `tone % seated.len()` degrades gracefully on non-triads. The held harmony MOVES (S20 win). |
| 11.6 | Texture selection rules (live): pad_figured (subject_energy≥0.45 ∧ fg_bg≥0.25), pad_bed_counter (foreground_energy≥0.35 ∧ fg_bg≥0.20), pad_block_comp (arousal≥0.70 ∧ valence≥0.55), pad_broken_up (arousal≥0.60), pad_arp_waltz (valence≥0.65 ∧ colorfulness≥0.50), pad_broken_wave (colorfulness≥0.40), pad_oom_pah (valence≥0.60 ∧ arousal 0.40-0.65), pad_stride (arousal≥0.75 ∧ colorfulness≥0.55) | `mappings.json: composition.texture.rules` | S17-S34 | subject/fg energy, fg_bg, affect, colorfulness | yes-strong | SELECTED | All thresholds are ear-tunable taste values. The oom-pah/stride gates (S34 OD-1) are the most recent. |
| 11.7 | **Walking bass** (diatonic target-seek next chord root via lookahead, arrives on next downbeat) | `chord_engine.rs walking_bass:2019`; `composition.rs BassPatternKind::Walking`; `mappings.json bass_pattern_catalogue: walking/walking_q`, profile `pad_walking` | S34 | (would be image-driven) | n/a | **AUTHORED-BUT-INERT** | Built and tested, but NO active texture rule selects `pad_walking`. MT's proposed (un-applied) trigger: arousal≥0.55 ∧ subject_energy≥0.50. Activation parked for the aesthetics arc — explicitly the arc's job. |
| 11.8 | **Pedal bass** (holds key's pedal_degree under changing harmony) | `chord_engine.rs pedal_bass:2150`; `BassPatternKind::Pedal`; `mappings.json bass_pattern_catalogue: pedal (degree 1)/pedal_dom (degree 5)`, profile `pad_pedal` | S34 | (would be image-driven) | n/a | **AUTHORED-BUT-INERT** | Built and tested; no active rule selects `pad_pedal`. MT's proposed trigger: arousal≤0.28 ∧ colorfulness≤0.30 (tonic pedal; dominant pedal `pedal_dom` inert). Activation parked for the aesthetics arc. |
| 11.9 | **oom_pah_pah figuration row** | `mappings.json figuration_catalogue: oom_pah_pah`; profile `pad_oom_pah_pah` | S34 | — | n/a | **AUTHORED-BUT-INERT** | Catalogue row + profile exist but no texture rule selects `pad_oom_pah_pah`. (oom_pah and stride ARE selected, 11.6.) |
| 11.10 | `register_octaves` octave-shift inside figured_bed (no-op at 0; oom-pah uses -1) | `chord_engine.rs` register shift; `FigurationOnset.register_octaves` | S34 | structural | no | SELECTED (within oom-pah/stride) | Lets oom-pah drop the "oom" an octave. Byte-safe at default 0. |

**Failure-mode contribution:** Texture is now a strong image-tied cluster (11.4/11.6 select per image) and answers the S16 "missing all the harmony/background" verdict. The standing issues: three generators are AUTHORED-BUT-INERT (11.7 walking, 11.8 pedal, 11.9 oom_pah_pah) — their activation triggers are the explicit deliverable of the aesthetics arc; and every texture-selection threshold (11.6) is unvalidated taste. Activating walking/pedal with chosen triggers will measurably increase rhythmic image-relatedness.

---

## Cluster 12 — Cadences & phrase structure

| # | Rule | Where | Session | Driven by | Image-rel | Wiring | MT note |
|---|---|---|---|---|---|---|---|
| 12.1 | Phrase model: 4/8-step phrases (`plan_phrases`/`StepPlan`), half cadence on V at antecedent boundary, PAC V-I at consequent boundary | `chord_engine.rs plan_phrases:671, PHRASE_LENGTHS=[4,8]` | S4 | structural | no | LIVE | Verified cadence-at-boundary (not mid-phrase). Theory-correct. |
| 12.2 | Per-section boundary cadence from `FormSpec` (Half/Imperfect/Perfect/Deceptive/Plagal) | `composition.rs CadenceStrength`; `mappings.json form_catalogue boundary_cadence` | S15 | structural (form) | no | SELECTED | Each form row specifies its section cadences. Differentiated A vs A′ (half ends A, PAC ends structural close). |
| 12.3 | Cadence resolution clausula for counter line (stepwise contrary to octave/unison) | `chord_engine.rs cadence_resolution_pitch:3945, opening_candidates:3917` | S30 | structural | no | SELECTED | Correct clausula. |

**Failure-mode contribution:** Phrase/cadence structure is theory-correct and image-INDEPENDENT (`no`) — it's the skeleton, not the image read. Not a direct "image-unrelated" contributor, but it's the same regardless of image, so it reinforces structural sameness across pieces. The cadence types DO vary by form (12.2), which is image-selected, so there's an indirect image link.

---

## Cluster 13 — Modal interchange, secondary dominants & chromatic color

| # | Rule | Where | Session | Driven by | Image-rel | Wiring | MT note |
|---|---|---|---|---|---|---|---|
| 13.1 | Minor iv on dark images (F-A-C → F-Ab-C, mode-general, no double-flatten) | `chord_engine.rs borrowed_minor_iv:390`; gated on `brightness_drop > 0.25` | S13 (no-op fixed same commit) | `avg_brightness` (drop) | yes-strong | LIVE | S13 QG found the original was a no-op; fixed to a TRUE minor iv. Optional minor-7th when complexity has 7th. Theory-correct borrowed chord. |
| 13.2 | bVI mixture appended on colorful (wide hue-spread) images (major triad +8 semitones, optional maj7) | `chord_engine.rs MODE_MIXTURE_THRESHOLD=0.45:285-288` | S13 | `colorfulness` | yes-strong | LIVE | Musically defensible mixture. |
| 13.3 | Secondary dominant tonicizes the REAL next chord (V/IV, V/V, V/vi distinct), look-ahead `next` now honored; major triad on target+7, optional dom7 | `chord_engine.rs secondary_dominant_of:351`; trigger `edge_activity > 0.55`; the chord_engine.rs:125 `next` lookahead | S13 (lookahead honored) | `edge_activity` | yes-strong | LIVE | S13 fixed the constant-home-V; now genuinely tonicizes. Theory-correct. |
| 13.4 | bVII / tritone-sub substitution options declared | `mappings.json: global.dominant_substitution_trigger.substitutions ["V/V","V/ii","tritone_sub"]`, `modal_interchange_trigger.borrowed_chords ["bVII","iv","bVI"]` | S13 | edge/brightness | yes-weak | partially LIVE | iv and bVI are live (13.1/13.2); V/V, V/ii, tritone_sub, bVII listed but the live secondary-dominant path tonicizes by lookahead (13.3) rather than these literal options — partial. |

**Failure-mode contribution:** Chromatic color is well image-tied (dark→minor-iv, colorful→bVI, busy→secondary-dominant) and theory-sound. Low contribution to the failure mode — this cluster is a success story. Aesthetics may want to tune the trigger thresholds (0.25/0.45/0.55) by ear.

---

## Cluster 14 — Key plan & modulation

| # | Rule | Where | Session | Driven by | Image-rel | Wiring | MT note |
|---|---|---|---|---|---|---|---|
| 14.1 | Image → home key + structural key plan: leaves home for a related key in B, returns home on the form's cadence | `composition.rs KeyScheme, resolve_key_scheme:1599-1700, key_scheme_catalogue`; `mappings.json key_scheme + catalogue` | S25 (K1) | `fg_bg_contrast` (gate), region affect (direction) | yes-strong | SELECTED | The tonal-travel win. v1 menu {dominant +7, subdominant +5, relative ±3}. Selected when fg_bg_contrast ≥ 0.25 (subject present); subjectless → home_only (byte-stable). |
| 14.2 | Per-region excursion direction: each excursion reads its OWN region's valence/hue (valence cut 0.40/0.60, hue-contrast 60° vs subject) | `composition.rs region_excursion_offset:1537, relative_offset:1464, parse_offset_rule` | S26 (K2a) | per-region brightness/hue | yes-strong | SELECTED | Closes the K1 "same whole-image valence → same offset" limitation. B and C can travel to distinct keys. |
| 14.3 | Planner re-root: chords generate at `home_root + offset` so HARMONY travels (not just theme melody) | `composition.rs §4.1(i):1302-1305, 1311-1319` | S26 (K2a) | key offset | yes-strong | LIVE | The load-bearing fix — K1 had modulated melody-only. |
| 14.4 | Multi-excursion reachable per form: catalogue rows for every form (rounded_binary/ternary/aaba/abbac/abac/T&V), routing order-isomorphic to the form ladder | `mappings.json key_scheme_catalogue + key_scheme rules` | S27 (K2b) | form features + fg_bg gate | yes-strong | SELECTED | Each form routes to its structural-twin scheme; `home_only` default. |
| 14.5 | `ResolutionPolicy{Resolve\|Open}` + `pivot` flag; Resolve forces final offset 0, Open ends off-home | `composition.rs ResolutionPolicy:501, KeyScheme.{resolution,pivot}` | S26 (K2b) | scheme | structural | LIVE | Resolve schemes land home; Open is the deliberate off-home ending. |
| 14.6 | **Open scheme `theme_and_variations_excursion`** (resolution:open, pivot:false) — the only Open scheme, UNROUTED | `mappings.json key_scheme_catalogue` | S27 | — | n/a | **AUTHORED-BUT-INERT** | OPERATOR LOCK: shipped but no active rule selects it; opt-in is a post-K3 operator decision. The one place a routed image could end off-home — deliberately gated off. |
| 14.7 | Realizer pivot/common-tone modulation: `pivot_chord_events` inserts V-of-destination at modulating boundary (triple-gated); dom7 = `(dom_root+10)%12` inner voice | `chord_engine.rs pivot_chord_events:2593, V_PIVOT=88` | S28 (K3), dom7 S29 | key plan | yes-strong | SELECTED | Off-home journeys arrive PREPARED/hinged. Triple-gate: pivot==true ∧ step_in_section==0 ∧ key≠prev. |
| 14.8 | Land-home PAC: Resolve scheme's final return strengthened to true home-key PAC (root-position I, soprano on tonic) | `chord_engine.rs land_home_is_armed:2708, land_home_pitch:2722` | S28 (K3) | key plan | yes-strong | SELECTED | Real V→I home arrival, not a splice. |
| 14.9 | Modulation made PERCEPTIBLE (S29 re-tune): (a) destination key CONFIRMED — force section chords[0] to dest tonic + re-voice step 1 as V→I authentic cadence in new key (`tonic_triad`, `pivot_resolution`); (b) MX-4 density SCENE-change; (c) dom7 in pivot | `chord_engine.rs pivot_resolution:2765+, tonic_triad`; `composition.rs density-from-energy` | S29 | key plan + region energy | yes-strong | SELECTED | S29 fixed "key changes not striking / couldn't hear it" — added CONFIRMATION (cadence in new key) + SCENE CHANGE (density). Operator re-listen PASSED. |
| 14.10 | MX-4: `Section.density` set from source region energy `f(e)=clamp(0.5+0.30·(e-0.5),0.35,0.65)`, READ as edge_activity nudge | `composition.rs density mapping:1336, HOME_ENERGY_NEUTRAL=0.5, DENSITY_*` | S29 (built; specified S26) | region energy | yes-strong | LIVE | Previously DEAD/write-only field, now drives audible density. f(0.5)=0.5 → byte-neutral on home sections. (Cross-ref 3.7/8.4.) |

**Failure-mode contribution:** Key-plan is a deep, image-tied, theory-sound success arc (S25-S29) that directly fights "structureless" — pieces now travel and come home, audibly (S29 operator re-listen passed). Low contribution to the failure mode; the one inert piece (14.6 Open scheme) is a deliberate operator lock, not a gap. Trigger thresholds (fg_bg 0.25, valence 0.40/0.60, hue 60°) are tunable taste.

---

## Cluster 15 — Motif / returning theme

| # | Rule | Where | Session | Driven by | Image-rel | Wiring | MT note |
|---|---|---|---|---|---|---|---|
| 15.1 | Motif from 8 contour archetypes (Arch/InvertedArch/Descent/Ascent/NeighborTurn/LeapStep/Pendulum/RisingSequence), hue+edge-seeded, resolved to degree+duration `MotifNote` at plan-build | `chord_engine.rs resolve_motif:2379, MotifArchetype:2321, MotifNote`; `composition.rs pick_archetype:1386` | S15 | `dominant_hue` (shape), `edge_activity` (motion), complexity (length) | yes-weak | SELECTED | Archetype is image-seeded (hue quadrant → broad shape; busy → motion). Range 2-7 degrees from edge, length 3-8 from complexity. The contour-to-pitch resolution biases toward chord tones. |
| 15.2 | Returning theme: stated A, head-fragment/absent B, Identity recap A′ (`theme_melody_pitch`) | `chord_engine.rs theme_melody_pitch:2472, theme_pitch:2526` | S15 | structural | yes-weak | SELECTED | Genuinely RETURNS (degree-sequence match). But operator S16 verdict: "not so musical / hard to tell it recurred." Whether the motif is MEMORABLE is a TASTE call MT cannot adjudicate. PRIME aesthetics target. |
| 15.3 | Fragmented variation plays only `(motif.len()+1)/2` notes then rests | `chord_engine.rs:2500` | S15 | structural | no | SELECTED | Head-motive fragmentation. Correct technique. |

**Failure-mode contribution:** The returning theme is the centerpiece of "structure," but 15.2 carries the operator's own complaint — present but not recognizable. The motif's memorability/singability is precisely the "reads as intended" layer MT defers to taste. High-priority aesthetics target.

---

## Cluster 16 — Realizer interface & output (structural, for completeness)

| # | Rule | Where | Session | Image-rel | Wiring | MT note |
|---|---|---|---|---|---|---|
| 16.1 | `realize_step(step, inst_idx, num_instruments, &PerfFeatures, ms_per_step, &StepContext) -> Vec<NoteEvent>` (7-param public sig, FROZEN since S6/S15) | `chord_engine.rs realize_step:1055` | S6 | n/a | LIVE | The realizer entry point. Public 7-param signature byte-frozen across every slice; new content rides the private `ctx`/`role` args. |
| 16.2 | `NoteEvent{note,velocity,hold_ms,offset_ms}` | `chord_engine.rs NoteEvent:884` | S6 | n/a | LIVE | The output unit per instrument per step. |
| 16.3 | `PerfFeatures{saturation,brightness,edge_density}` projection of image features into the realizer | `chord_engine.rs PerfFeatures:866` | S6 | yes-strong (carries image) | LIVE | The narrow channel through which raw image features reach the realizer (most image-driving now flows through `ctx`/plan instead). |
| 16.4 | `engine_equivalence` byte-freeze: goldens 240ms / vel 114 / 84 / pitch 36 / 79 on identity single-section plan | `tests/engine_equivalence`; `single_section_default` | S9+ | n/a | LIVE (test) | The behavioral freeze every realizer slice proved unmoved. Not a music rule, but it governs why so many rules are SELECTED (reachable only off the identity path). |

**Failure-mode contribution:** None directly — this is the plumbing. Noted so the reviewer understands why many image-tied rules are `SELECTED` not `LIVE`: they're unreachable on the identity/equivalence path and only fire on the compose path a real image takes.

---

## Known standing failure mode — per-cluster contribution

Operator verdict the arc exists to fix: **"diversity real but ethereal / structureless / image-unrelated; only works for abstract art."** One line per cluster on whether/how it plausibly contributes (seeds the affective prioritization; does NOT perform it):

- **C1 Feature extraction/normalization** — ROOT of "image-unrelated": `subject_hue`, `subject_saturation`, `mass_centroid`, `dominant_hue_mass`, `secondary_hue`, `palette_bimodality` are extracted-or-reserved but inert. Real image content thrown away.
- **C2 Feature→param mappings** — LIVE spine is sound, but a long tail of legacy `AUTHORED-BUT-INERT` tables (orientation→interval, shape→ostinato, per-pixel pitch) documents image correspondences the pipeline silently does NOT honor — a viewer expects to hear them.
- **C3 Macro-form/structure** — Strongest answer to "structureless" (S15 killed the loop), but theme is present-not-recognizable and meter is frozen 4/4 → rhythmic identity never varies by image.
- **C4 Mode & tonality** — Theory-correct, but hue→mode is unvalidated convention and home pitch is fixed C; weak per-image tonal distinctness; major/minor not yet valence-driven (see C6.6).
- **C5 Harmony** — Deepened + sound, but within-family progression is RNG → harmony only weakly tracks the image.
- **C6 Affect bridge** — The densest concentration of aesthetics targets: composite weights, character thresholds, tempo windows, nudge spans are ALL ear-tuned. BIGGEST GAP: valence drives character+tempo but NOT actual major/minor mode (6.6) → an "energetic" image can still voice minor. Directly owns the energetic/joyful verdict.
- **C7 Dynamics** — Well-crafted but almost entirely image-INDEPENDENT (structural) → same expressive shape regardless of image, reinforcing "image-unrelated."
- **C8 Rhythm** — One of the better image-tied clusters; main questions are band-threshold placement + vocabulary richness (taste).
- **C9 Articulation** — HEART of "ethereal": the LO/HI note-length window governs sung-vs-floaty-vs-mechanical and is exactly the taste call MT defers. Pad-overlap cap limits sustained-harmony feel.
- **C10 Voice-leading/counterpoint** — Theory-rigorous, image-independent by design (does NOT contribute to image-unrelated); richness gated behind saliency selection.
- **C11 Orchestration/texture** — Strong image link now, answers "missing harmony/background"; standing issue is the AUTHORED-BUT-INERT walking/pedal/oom_pah_pah generators whose triggers are the arc's explicit deliverable, plus unvalidated texture thresholds.
- **C12 Cadences/phrase** — Theory-correct, image-independent skeleton; reinforces structural sameness but cadence TYPES vary by (image-selected) form.
- **C13 Modal interchange/secondary dominants** — A success story: well image-tied + theory-sound; only trigger-threshold tuning.
- **C14 Key plan/modulation** — Deep image-tied success arc (S25-S29), directly fights "structureless," audibly travels (operator re-listen passed); low failure contribution; only the deliberately-locked Open scheme is inert.
- **C15 Motif/returning theme** — Carries the operator's own "not so musical / hard to tell it recurred" complaint; memorability is the "reads as intended" layer MT defers. High-priority.
- **C16 Realizer interface** — Plumbing; no direct contribution.

---

## Coverage check (S2 → S34 walked)

Confirmed each session's music-producing decisions are represented:

- **S2** — mode-collapse fix (6 modes, 4.1), numeral→degree dead-arm fixes (4.3), default root 60 (4.2), 23-test net foundation. ✓ (C4)
- **S3** — modem only (not music; out of scope for this inventory). ✓ (excluded by scope)
- **S4** — voice leading + phrase structure + structural velocity floors. ✓ (C10, C12, C7.5)
- **S5/S7** — modem only. ✓ (excluded)
- **S6** — dynamics + rhythm (5 patterns) + articulation + orchestration roles + phrase-model playback wiring. ✓ (C7, C8, C9, C11.1)
- **S9/S10/S11/S12** — WS-4 engine seam / TUI / pure-Rust collapse / MIDI output: infrastructure, no music-RULE change (S9 lifted `worker_decide_action` verbatim). ✓ (no new music rules; C16 interface)
- **S13** — diversity: tempo (2.3), harmonic complexity (2.2/5.2), continuous articulation (9.1), rhythm density, bVI mixture (13.2), secondary-dominant lookahead (13.3), minor-iv no-op fix (13.1), normalization (1.2), dead-feature revival (1.7). ✓ (C1, C2, C5, C8, C9, C13)
- **S14** — design-only (composition architecture vision); no rules. ✓ (no code)
- **S15** — non-looping plan (3.1), form-as-data (3.2), returning theme/motif (15.1/15.2/3.5), articulation clamp re-scale (9.1). ✓ (C3, C15, C9)
- **S16** — design-only (texture/saliency); no rules. ✓ (no code)
- **S17** — sustained pad (11.3), rest-bug fix (8.3), orchestration-as-data (11.4), FILL_REST_ACTIVITY (8.3), pad-overlap cap (9.4). ✓ (C11, C8, C9)
- **S18** — saliency reader (1.9/1.10), counter-melody (10.4 stub→), region selection (11.4). ✓ (C1, C10, C11)
- **S19** — design-only (figuration); no rules. ✓ (no code)
- **S20** — Alberti figuration (11.5); BUG-01 stuck-MIDI fix (output plumbing, not a music rule). ✓ (C11)
- **S21** — design-only (affect); Specialist 8 fabricated; no rules. ✓ (no code; 6.6/6.8 designed-not-built captured)
- **S22** — affect composite (6.1/6.2), character selection (6.3), tempo windows/de-cap (6.4/6.5). ✓ (C6)
- **S23** — saliency→prominence (6.7/7.8/8.5). ✓ (C6, C7, C8)
- **S24** — design-only (key plan); Specialist 9 fabricated; no rules. ✓ (no code)
- **S25** — K1 home key + ABA structural key plan (14.1). ✓ (C14)
- **S26** — K2a per-region affect + multi-excursion planner + harmony re-root (14.2/14.3). ✓ (C14)
- **S27** — K2b multi-excursion reachable + Open scheme + conditional resolves_home (14.4/14.5/14.6). ✓ (C14)
- **S28** — K3 realizer pivot + land-home PAC (14.7/14.8). ✓ (C14)
- **S29** — K3 re-tune: destination confirmation cadence + MX-4 density + pivot dom7 (14.9/14.10/8.4/3.7). ✓ (C14, C8, C3)
- **S30** — fifth-species counter scorer + cross-step memory (10.4/10.5/10.6) + idiomatic progression rows (5.3) + accompaniment figuration rows (11.5). ✓ (C10, C5, C11)
- **S31** — listening-loop unblock (`./play`, soundfont A/B, gain/reverb): audio-output tooling, NOT music engine (engine.rs unmoved). ✓ (no music rules)
- **S32** — band-reachability GAP-2/GAP-3 dispositions (10.7). ✓ (C10)
- **S33** — GAP-4 penult-rework (10.7). ✓ (C10)
- **S34** — oom-pah/stride SELECTED (11.5/11.6/11.10) + walking/pedal/oom_pah_pah AUTHORED-BUT-INERT (11.7/11.8/11.9). ✓ (C11)

### Decisions found in code but NOT cleanly tied to a session

- **Legacy `mappings.json` blocks** `instrument_section` (2.12-2.15), `fine_detail` (2.16), `cadence_trigger` (2.11) — these predate the captured S2 narrative (pre-port AudioHax original mapping tables). Tagged S2-era / `AUTHORED-BUT-INERT`; I could not tie them to a specific build session because they were never re-touched within S2-S34 (the live pipeline routes around them). They are the clearest "documented intent the runtime does not honor" finding.
- **`tonic_triad`** appears used by both the S15 plan-build (deterministic I) and S29 (destination tonic confirmation) — attributed to S29 where its key-plan use was wired, but its first introduction may predate; not a music-behavior ambiguity.

### Sessions whose music work I could NOT locate in code

None. Every session that the narrative claims touched a music rule was located in the source. Sessions S3/S5/S7 (modem) and S8-S12/S31 (infra/audio-output) are correctly NOT music-rule sessions and are out of this inventory's scope. The design-only sessions (S14/S16/S19/S21/S24) correctly produced no code rules; their DESIGNED-but-DEFERRED items that matter affectively (notably valence→mode, 6.6) are captured as `AUTHORED-BUT-INERT` so the affective overlay sees them.

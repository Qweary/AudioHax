# Diagnosis S13 — Music-Side: "Every image sounds the same"

**Author:** Music Theory Specialist
**Date:** 2026-06-14
**Scope:** READ-ONLY diagnosis of the image→music *mapping + engine consumption* side. The image-extraction side (richness of features) is diagnosed in parallel by the Rust Architect. No source was modified.
**Operator report:** Judged through FluidSynth+reverb (so this is about the NOTES, not the dry synth). Every image yields the same general feel; only the MODE seems to vary; TEMPO never differs between images; notes are uniformly SHORT and "computer-like."

---

## 0. Executive headline

**Yes — effectively only MODE is wired to the image.** Of the nine musical dimensions, exactly **one** (mode) varies per-image through a real feature→music path. Two more (a modal-interchange `iv` swap and a secondary-dominant insertion) are nominally feature-gated but are **dead in practice** because their trigger features are constant-ish near zero on the whole-image average. **Six dimensions are completely dead to the image:**

- **Tempo / step-duration is HARDCODED** at the CLI default `ms_per_step = 250` (`cli.rs:83`, `engine.rs:192`). No image feature touches it. `brightness_to_tempo_bpm` exists in `mappings.json` and is *loaded and clone-shuffled* but **never read by any decision** (confirmed: only references are the struct field + the deep-copy in `engine.rs:586`). This is the single most audible defect after "only mode varies," and it is the operator's exact complaint ("tempo does not differ").
- **Articulation / note-length, rhythm/density, dynamics, register, orchestration** *are* driven by features inside `realize_step` — but by the **per-step `ScanBarFeatures` row**, not the whole-image average. On the current default that path does carry per-step variation; however the *expressive bands are coarse and the same band is hit on most natural photos*, which (combined with constant tempo and constant mode-feel) produces the "uniformly short, computer-like, same general feel" result. See §1/§2 for the precise per-dimension verdict.

**Single most impactful fix:** drive **`ms_per_step` (tempo) from a per-image feature** (brightness or edge-density), wired through the existing-but-dead `brightness_to_tempo_bpm` map, and carry it into the engine. Tempo is the most globally perceptible musical dimension and is currently a literal constant; making it vary per image is the highest musicality-per-line-of-code change available. (Second: replace the single-mean-hue mode pick with a dominant-hue / histogram pick so mode itself stops regressing to the mean.)

---

## 1. CURRENT FEATURE → MUSIC DATAFLOW MAP

The whole pipeline runs `set_features_global(GlobalFeatures)` **once** to derive mode + progression + voice-led phrase plan (`engine.rs:328-357`), then `realize_step(...)` per step reads the **per-step** `ScanBarFeatures`→`PerfFeatures` projection (`engine.rs:532-565`).

| Musical dimension | Driving image feature (if any) | Where (file:line) | Status |
|---|---|---|---|
| **Mode** (Ionian…Aeolian) | `avg_hue` (whole-image mean) via `hue_to_mode` range map | `engine.rs:330` → `mapping_loader.rs:66` → `mappings.json:3-10` | **PER-IMAGE-VARYING** (but on a single mean — collapses, see §2c) |
| **Root / key** | none — fixed `root_midi = 60` (C4) | `engine.rs:194` (default), `cli.rs` `--root` is the only override | **CONSTANT** |
| **Tempo / step-duration** | none — CLI flag default 250 ms; `brightness_to_tempo_bpm` is dead | `cli.rs:83-84`, `engine.rs:192`, `engine.rs:457` passes `config.ms_per_step`; `brightness_to_tempo_bpm` only at `engine.rs:586` (clone, never read) | **CONSTANT** (hardcoded) |
| **Harmonic complexity** (triads / 7ths / extensions) | none — `saturation_to_harmonic_complexity` is dead; `roman_to_chord` always emits a bare root-position triad | map dead (`engine.rs:585` clone only); triad built `chord_engine.rs:164-173` | **CONSTANT** (always plain triads) |
| **Progression choice** | mode → warm/cool family, then **`thread_rng`** picks one of 2-3 | `chord_engine.rs:45-71` (`pick_progression`) | **RANDOM-not-image-driven** (within the mode's family) |
| **Note density / rhythm pattern** | per-step `edge_density` → pattern band (sustained/dotted/syncopated/arpeggio) | `chord_engine.rs:863, 877-882, 951-990` | **PER-IMAGE-VARYING** (per-step), but coarse 4-band cutoffs |
| **Articulation / note-length** | per-step `edge_density` → `STACCATO/PORTATO/LEGATO_FRAC` + role | `chord_engine.rs:842-844, 877-882` | **PER-IMAGE-VARYING** (per-step), but see §2a — bands cluster + jitter floor |
| **Dynamics / velocity** | per-step `saturation` (level) + structural floor + phrase contour | `chord_engine.rs:776-837` (`realize_velocity`); floor `chord_engine.rs:426-428` | **PER-IMAGE-VARYING** (per-step level) atop a constant structural contour |
| **Register / orchestration role** | per-step `brightness` (octave lift) + instrument index → role | `chord_engine.rs:705-763` (`role_pitch`), `608-621` (`instrument_role`) | **PER-IMAGE-VARYING** (per-step), bass exempt from lift |

### The crucial subtlety the operator is hearing

The realization layer (§dimensions 6-9) *is* feature-reactive — but **only to the per-step `ScanBarFeatures`, never to the whole image's identity**, and only through **coarse band cutoffs**. Meanwhile the two globally-perceived "what kind of piece is this" dimensions — **tempo and harmonic complexity — are hard constants**, and **mode** is the *only* per-image global that moves. So two images that differ a lot still get: the same tempo, the same triad-only harmony, the same phrase-velocity contour, the same C-tonic, and per-step articulation/density that lands in the same 1-2 edge-density bands for most natural photographs (whose global edge density the operator measured at ~0.005 — deep in the "low edge → LEGATO, sustained" band for every such image). The net percept is exactly "same general feel, only the mode (color) changes."

---

## 2. ROOT-CAUSE FINDINGS

### Hypothesis (a) — "Only mode is wired; tempo, density, articulation, dynamics are constant/hardcoded." → **CONFIRMED (with nuance).**

- **Mode is the only per-image GLOBAL musical driver.** `set_features_global` reads exactly one feature for the global plan: `avg_hue` → mode (`engine.rs:330`). `edge_density` is passed to `generate_chords` (`engine.rs:342`) and `brightness_drop` is passed as a literal `0.0` (`engine.rs:343`) — see the two dead triggers below.
- **Tempo is hardcoded.** `ms_per_step` is sourced from `EngineConfig`, which is built from the CLI (`cli.rs:168`, default 250 at `cli.rs:83`), never from a feature. The mapping author's intent — `brightness_to_tempo_bpm` (`mappings.json:16-20`) — is **completely unconsumed**: grep shows its only references are the `mapping_loader.rs:28` field declaration and the `engine.rs:586` deep-copy clone. **Dead code.** This is the operator's "tempo does not differ between images," verbatim.
- **Harmonic complexity is constant.** `roman_to_chord` always builds a 3-note root-position triad (`chord_engine.rs:164-173`); there is no 7th/extension path. `saturation_to_harmonic_complexity` (`mappings.json:11-15`) is **dead** (only the `engine.rs:585` clone). Every image, every chord: a bare triad. This is a major contributor to "computer-like / same feel" — real harmony breathes through 7ths, 9ths, suspensions; this engine never adds one.
- **The two "smart harmony" triggers are dead in practice:**
  - *Modal interchange* (`IV→iv` when `brightness_drop > 0.25`, `chord_engine.rs:102-113`): the caller passes `brightness_drop = 0.0` hardcoded (`engine.rs:343`). **Never fires.** No image can trigger it.
  - *Secondary-dominant insertion* (when `edge_complexity > 0.7`, `chord_engine.rs:116-130`): fed the whole-image `edge_density` (`engine.rs:342`). With natural-photo global edge density ~0.005 (operator's example) this is **two orders of magnitude below the 0.7 threshold** — it can only fire on near-pathologically busy images. **Effectively never fires.** (And even when it does, it has the bug in §3.)
- **Articulation / density / dynamics / register DO vary per step** (not constant), but: (1) only off the **per-step** row, never the image identity; (2) through **coarse bands** (`edge < 0.25 → LEGATO`, `> 0.70 → STACCATO`, else PORTATO — `chord_engine.rs:877-882`) that put most natural images in one band; (3) the **phrase-velocity contour and structural floor are image-independent constants** (`chord_engine.rs:426-428, 787-837`), so the dynamic *shape* is identical across images.

**Verdict (a): confirmed.** Per-image GLOBAL variation = mode only. Tempo, key, and harmonic complexity are dead to the image. The realization dimensions react per-step but not to image identity and through bands too coarse to differentiate typical photos.

### Hypothesis (b) — "The `chord_engine.rs:125` `next` is a dead voice-leading / dead-motion lead." → **CONFIRMED it is dead; see §3 for full treatment.** It is a dead *secondary-dominant look-ahead*, not voice-leading. Wiring it correctly would add genuine harmonic motion — but only if its trigger (§2a) is also fixed, since it currently almost never fires.

### Hypothesis (c) — "Whole-image averaging regresses diverse images to the mean; does the mapping collapse them even if features were richer?" → **CONFIRMED from the mapping side.**

Two independent collapse mechanisms, both on the mapping/consumption side (orthogonal to the extraction side the Architect is diagnosing):

1. **Single-scalar mode pick.** `hue_to_mode` consumes one number: `avg_hue` (`engine.rs:330`). A circular *mean* hue (computed in `pure_analysis.rs:196-205`) of a multi-colored image lands somewhere in the middle of the wheel regardless of the actual palette — a red+green image and a uniformly-yellow image can produce the same mean hue and therefore the same mode. The 6-bucket range map (`mappings.json:3-10`) then quantizes even that single number into one of six outcomes. The **`hue_hist` 8-bin histogram is already computed per section** (`pure_analysis.rs:472`) and explicitly documented as **"music-inert / unused by the music decision"** (`engine.rs:72`). The richer signal exists and is thrown away.
2. **Coarse global thresholds.** The two harmony triggers (§2a) read whole-image scalars against single fixed thresholds (0.7, 0.25). Whole-image averaging pushes edge/contrast toward small middling values, so the thresholds sit outside the realistic range and the triggers are effectively step-functions stuck on "off." Even a richer extractor feeding these *same* thresholds would mostly stay off.

**Verdict (c): confirmed.** Even with richer image features, the current mapping would collapse them: mode keys off one scalar mean, harmony triggers off whole-image scalars against unreachable thresholds, and the per-collection histogram that could diversify mode/key is computed and discarded.

---

## 3. THE `chord_engine.rs:125` `next` LEAD

```rust
// chord_engine.rs:116-130  (inside generate_chords)
if edge_complexity > self.mappings.global.dominant_substitution_trigger.edge_complexity_threshold {
    // heuristic: insert a V of the next chord
    if i + 1 < progression.len() {
        let next = &progression[i + 1];                       // :125  <-- computed, then DISCARDED
        // compute V of next and insert (very simplified)
        let v_chord = self.roman_to_chord("V", root_midi, &scale, "V");   // :127  uses literal "V", NOT next
        chords.push(v_chord);
    }
    let chord = ... // the real progression chord follows
}
```

**What it is:** a **secondary-dominant (applied-dominant) look-ahead**. The intent (per its own comment, "insert a V of the next chord") is *tonicization*: before progressing to the next chord, insert that chord's **own dominant** — i.e. a **V/x** ("five of x"). For a `…IV` continuation it would insert `V/IV` (the dominant of IV); before `vi`, `V/vi`; etc. This is the single most idiomatic way common-practice harmony generates forward pull: a chord borrowed from the next chord's key that resolves up a fourth into it. It is exactly the "motion" the operator says is missing.

**Why it's unused:** the author bound `next` to read the *target* of the tonicization, but then computed `roman_to_chord("V", ...)` with a **literal `"V"`** — the dominant of the *home* tonic, not of `next`. So `next` is never consumed (hence the compiler warning, confirmed live: `warning: unused variable: next --> src/chord_engine.rs:125:25`). The inserted chord is always the home-key V regardless of what follows it — a plain dominant filler, not a secondary dominant. The look-ahead was scaffolded and then left half-wired.

**Musical value of wiring it (correctly):** **High, and directly on-target for the complaint.** A correctly-wired `V/next` does three things constant triads cannot:
- introduces a **chromatic tone** (the raised third of the applied dominant — e.g. V/IV brings the b7 of the tonic; V/V brings the raised 4) — the first non-diatonic color this engine would ever produce;
- creates a **strong root-motion-by-fourth resolution** into the next chord — real harmonic propulsion;
- differentiates images: it only fires on busier images, so "busy image → harmonically richer, more driving piece" becomes an audible per-image axis.

**Two caveats for the lead:** (1) wiring `next` is necessary but **not sufficient** — the trigger (`edge_complexity > 0.7` on whole-image edge density ~0.005) must also be re-scaled or re-sourced or it will still never fire (§2a/§4); (2) the secondary dominant must be spelled relative to `next`'s scale degree, and the inserted chord should ideally be a dominant-*seventh* for the pull to land — which dovetails with adding the 7th path in §4.

---

## 4. FIX DESIGN (musical)

Goal: **distinct images → distinct music across multiple dimensions**, with every mapping perceptually grounded (visual activity → musical activity, color → tonal color, brightness → register/energy). Ordered by impact.

### 4.1 Tempo ← brightness (and/or edge density)  *(highest impact)*
Wire the dead `brightness_to_tempo_bpm` (`mappings.json:16-20`) into a per-image `ms_per_step`. Convert BPM→ms at a chosen subdivision (e.g. one step = one beat: `ms_per_step = 60000 / bpm`).
- **Mapping:** dark image → slow (60 BPM → 1000 ms/step), mid → 90, bright → 120 (240/667/500 ms... pick a step subdivision; or interpolate continuously rather than 3 buckets).
- **Theory:** brightness/luminance is the most natural visual correlate of *energy/arousal*; bright, high-key images read as fast/lively, dark low-key images as slow/somber. This is the textbook film-scoring intuition. A continuous map (not 3 buckets) is strongly preferred so two bright-but-different images still differ.
- **Refinement:** consider blending brightness (base tempo) with `edge_density` (a small +/- nudge: busier texture nudges faster) so two equally-bright images of different busyness still differ.
- **Requires the engine seam to carry a per-image tempo — see §5.** This is the one fix that does NOT fit inside the music files alone.

### 4.2 Harmonic complexity ← saturation  *(high impact, fixes "computer-like")*
Wire the dead `saturation_to_harmonic_complexity` (`mappings.json:11-15`) so `roman_to_chord` adds chord tones: low sat → triad, mid → add the diatonic 7th, high → 7th + a 9th/extension.
- **Theory:** saturation = vividness of color = richness of harmony. A washed-out image gets bare triads; a vivid image gets lush 7th/9th color. 7ths and 9ths are precisely what make harmony sound *human and breathing* rather than "computer triads" — this directly addresses the operator's "computer-like" note about character (alongside §4.3 for the literal note-length complaint).
- **Implementation note:** the diatonic 7th is `scale[(degree+6)%7]`, the 9th `scale[(degree+1)%7]` up an octave — both already derivable from the existing `scale` array in `roman_to_chord`. Voice leading (`voice_lead_one`) already generalizes over an N-note voicing, so adding a 4th tone flows through.

### 4.3 Articulation / note-length ← richer, continuous mapping  *(fixes "uniformly short")*
The operator hears "uniformly short, computer-like notes." Two things to fix:
- (1) **Don't cluster.** The current 3-band cutoff (`edge < 0.25 → LEGATO`, `> 0.70 → STACCATO`, `chord_engine.rs:877-882`) is fine in principle but the **bands are too wide** and natural images cluster in one. Replace the step function with a **continuous** hold-fraction: `frac = lerp(LEGATO_FRAC..STACCATO_FRAC, edge_density)` so note length varies smoothly with texture instead of snapping to one of three values.
- (2) **Raise the legato ceiling and add an image-global "base articulation."** `LEGATO_FRAC = 0.95` (`chord_engine.rs:844`) never overlaps; sustained "singing" lines on legato images would benefit from holds that *cross the step boundary* (frac > 1.0, already permitted on cadence at `chord_engine.rs:892`). For low-edge images, let the melody hold ≥ 1.0 so successive notes connect — this is what kills "uniformly short." Tie a **global** articulation bias to a whole-image feature (e.g. `texture_laplacian_var` or `hue_spread`): smooth/low-texture image → globally more legato; rough → globally more detached. That gives a *per-image* articulation character on top of the per-step variation.
- **Theory:** legato vs staccato is the single biggest contributor to whether a line sounds "played" vs "typed." Brass/strings phrasing (the operator's trombone ear) lives in connected note-to-note length; uniform short detached notes read as MIDI-grid.

### 4.4 Note density / rhythm ← edge density, but per-IMAGE not just per-step
The pattern selection (`chord_engine.rs:951-990`) is good but keys off the **per-step** edge band. Add a **per-image rhythmic-activity scalar** (whole-image `edge_density` or `texture_laplacian_var`) that shifts the *whole piece's* baseline density — e.g. a busy image makes even the "calm" steps subdivide more. Combine multiplicatively with the per-step band so both image identity and local texture register.

### 4.5 Mode / key ← dominant hue (histogram), not single mean  *(fixes mode collapse)*
Replace the `avg_hue → hue_to_mode` single-scalar pick (`engine.rs:330`) with a pick off the **already-computed 8-bin `hue_hist`** (`pure_analysis.rs:472`, currently discarded): use the **dominant (modal) hue bin** — or the top-2 bins — to choose mode, and optionally let a strong secondary hue choose the **root/key transposition** (currently a hard constant C4, §1 row 2). 
- **Theory:** an image's *characteristic* color is its most-present hue, not the circular mean of all of them (which averages a red-and-green image to a muddy middle). Letting the dominant hue pick mode and a secondary hue pick key gives **two** color-driven tonal axes instead of one, and stops diverse palettes from regressing to the same mode.
- **Bonus:** `hue_spread` (already computed, `pure_analysis.rs:429`) → if hue is very spread, lean toward modal-mixture/borrowed chords (revives the dead `modal_interchange_trigger` with a *real* feature instead of the hardcoded `0.0`).

### 4.6 Revive the two dead triggers with realistic thresholds
- **Modal interchange:** stop passing `0.0` (`engine.rs:343`); feed a real whole-image feature (e.g. a brightness-contrast or `hue_spread` measure) and re-scale the 0.25 threshold to that feature's realistic range.
- **Secondary dominant (the §3 `next` lead):** fix the `next` wiring AND re-scale `edge_complexity_threshold` from 0.7 to the realistic edge-density range (natural images ~0.005-0.05), or re-source it from a normalized 0..1 activity score so it actually fires on busy images.

### New image features wanted from the extraction side (name them for the Architect — see §7)
- A **normalized 0..1 "visual activity" scalar** (edge/texture combined and scaled so its realistic spread covers 0..1), so the harmony triggers and density mapping have a usable dynamic range instead of clustering near 0.005.
- A **dominant-hue / hue-histogram** surfaced at the *global* level (per-section `hue_hist` already exists; a whole-image one + a "dominant bin" + "secondary bin" would let §4.5 work).
- A **contrast / dynamic-range scalar** (e.g. brightness stddev) to drive dynamics spread and modal interchange honestly.

---

## 5. ENGINE-SEAM EVOLUTION  *(flagged EXPLICITLY for the lead — a decision, not an assumption)*

**Most of the fix fits inside the files the music side owns** (`chord_engine.rs` + `mapping_loader.rs` + `mappings.json`): §4.2 (harmonic complexity in `roman_to_chord`), §4.3 (articulation curve in `realize_rhythm`), §4.4 (density), §4.5 (mode/key off `hue_hist` — the data is already on `GlobalFeatures`/`ScanBarFeatures` so no new field), §4.6 modal-interchange (just stop passing `0.0` — a one-line change at the `engine.rs:343` call site, which the lead must approve as it touches `engine.rs`), and the §3 `next` fix.

**One fix REQUIRES a seam change and I am NOT assuming it:** **§4.1 tempo.** Today `ms_per_step` is **engine config**, set from the CLI before any image is seen (`EngineConfig.ms_per_step`, `engine.rs:178`; consumed at `engine.rs:457`/`chord_engine.rs:864`). For tempo to be image-driven, **`ms_per_step` must become a function of `GlobalFeatures`**, which means one of:
- **(Option A, smallest)** in `set_features_global` (`engine.rs:328`), derive a tempo from `global.avg_brightness` via `brightness_to_tempo_bpm` and **overwrite `self.config.ms_per_step`** — the engine already owns config and the global features at that point, so this is contained to `engine.rs` and needs no `decide_instrument_action` change. **Recommended.**
- **(Option B)** thread a per-step tempo through `decide_instrument_action`/`realize_step` (changes those signatures) — only needed if tempo should vary *within* an image (e.g. ritardando driven by per-step features). Heavier; defer.

`decide_instrument_action`'s **decision logic** does NOT need to change for any fix here — it already projects the per-step row into `PerfFeatures` and calls `realize_step`. The only seam question is whether tempo (Option A: a config overwrite inside `set_features_global`) or possibly a new `GlobalFeatures` field (a precomputed dominant-hue, if the Architect supplies it rather than the music side deriving it from the existing `hue_hist`) crosses the boundary. **Both are small and both are the lead's call.** Nothing requires touching the `FeatureSource`/`AudioSink` traits.

---

## 6. PROPERTY-TEST TARGETS (dry-synth-independent musical properties)

All testable headless via `cargo test --lib --no-default-features` against `decide_step`/`realize_step`/`generate_chords`, comparing two *distinct* `GlobalFeatures`/feature-vector inputs A and B:

1. **Tempo varies with the image:** two `GlobalFeatures` with different `avg_brightness` (e.g. dark vs bright) → the engine's effective `ms_per_step` (or total realized duration) **differs**. (Regression guard on the §4.1 fix; today this would be CONSTANT.)
2. **Harmonic complexity varies with saturation:** low-saturation input → chords have 3 notes; high-saturation input → chords have ≥4 notes (a 7th present). (§4.2)
3. **Mean note-length varies with texture/edge:** feature vector A (low edge) vs B (high edge) → the mean `hold_ms / ms_per_step` fraction **differs** by more than a small epsilon, AND A's mean fraction > B's (calm → longer). (§4.3 — directly tests "uniformly short" is gone.)
4. **Rhythm-pattern SET differs:** the multiset of onset counts per step for image A ≠ that for image B when their edge densities differ. (Not just "a pattern exists" — the *distribution* differs. §4.4.)
5. **Mode diversity off histogram:** two images with the same circular-*mean* hue but different dominant-hue bins → **different mode** (proves the §4.5 fix actually decoupled from the mean). And a red+green image ≠ a muddy-middle image in mode.
6. **The combined "distinctness" property (the headline acceptance test):** for two materially different feature vectors A and B, assert they differ in **≥3 of {tempo, mean note-length, rhythm-pattern set, chord-tone count, mode}** — *not* mode alone. This is the operator's complaint encoded as a test: "different image ⇒ different music in more than one dimension."
7. **Secondary dominant fires and is correct (§3):** a high-activity input produces an inserted chord that is the dominant **of the following chord** (a chromatic, non-home-key V/x), not always the home V; and `next` is consumed.
8. **Modal interchange can fire:** a qualifying whole-image feature produces a borrowed `iv` (today: never, because `0.0` is hardcoded). (§4.6)
9. **Determinism preserved where required:** with `pick_progression`'s RNG pinned (explicit progression), all of the above are deterministic functions of the feature vector (the existing test discipline at `chord_engine.rs:1371-1380` still holds).

---

## 7. COORDINATION NOTE — what the music side needs from the image-analysis side

For the fixes above to *bite*, the Architect's image-side diagnosis/fix should provide:

1. **A normalized 0..1 "visual activity" scalar** (combined edge+texture, scaled so the realistic spread across natural photos covers most of 0..1). The current whole-image `edge_density` clusters near ~0.005, which is why the 0.7 secondary-dominant trigger and the density mapping are effectively dead. The music side cannot fix that by re-thresholding alone without knowing the real distribution — we need either a normalized feature or the empirical min/max range to calibrate against. **(Blocks §4.4, §4.6-secondary-dominant.)**
2. **A whole-image dominant-hue signal** — either a global `hue_hist` (per-section histograms already exist at `pure_analysis.rs:472`; we need an image-level one) or precomputed "dominant bin" + "secondary bin." If the Architect would rather the music side derive this from an existing global histogram, just expose the global histogram on `GlobalFeatures` and we'll do the rest. **(Blocks §4.5 mode/key diversity.)**
3. **A contrast / brightness-dispersion scalar** (e.g. brightness stddev or hue_spread is usable) to drive dynamics spread and to give modal interchange an honest trigger feature instead of the hardcoded `0.0`. **(Blocks §4.6-modal-interchange, improves §4.3 global articulation bias.)**
4. **Confirmation of realistic value ranges** for `avg_brightness`, `avg_saturation`, `edge_density`, `texture_laplacian_var` across the operator's real image set — so tempo/complexity/articulation curves are calibrated to the data, not to the theoretical 0..100 / 0..1 endpoints (which natural images rarely span). **(Calibrates §4.1, §4.2, §4.3.)**

The seam decision in §5 (tempo via config-overwrite in `set_features_global` vs a new global field) should be made jointly once the Architect confirms whether dominant-hue/activity will be precomputed on the image side or derived on the music side from existing fields.

---

*End of music-side diagnosis. No source modified. Headline: only MODE is per-image-wired; tempo and harmonic complexity are hardcoded dead, and the realization layer reacts per-step but through bands too coarse (and too averaged) to distinguish typical images. Highest-impact single fix: image-driven tempo (§4.1) via the already-present-but-dead `brightness_to_tempo_bpm`, with the seam handled by Option A in §5.*

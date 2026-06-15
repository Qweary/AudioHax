# Design S13 — Image→Music Diversity (implementation-ready spec)

**Author role:** Rust Architect (DESIGN ONLY — no source modified by this document).
**Date:** 2026-06-14
**Inputs:** `docs/diagnosis-s13-music-side.md` (Music Theory Specialist) + `docs/diagnosis-s13-image-dataflow.md` (Rust Architect). Both read in full; their findings are the premises here and are not re-litigated.
**Operator approval carried into this spec:** implement the fix THIS session, *including* the one contained `engine.rs` change (tempo Option A — overwrite `config.ms_per_step` inside `set_features_global`). `engine.rs` has been byte-frozen since S9; this is a deliberate, approved break of that freeze, scoped to the single tempo edit plus the modal-interchange `0.0` removal.

This spec is executable by **two implementers in parallel** with **file-disjoint** ownership. Read §0 (the seam decision) and §5 (ownership + interface contract) before touching any file — they are the collision-avoidance contract.

---

## 0. THE SEAM / NORMALIZATION DECISION — *load-bearing; read first*

### Decision: **Option-NORM-MAP.** The music-mapping layer normalizes the raw already-crossing features using calibrated ranges. `engine.rs` is touched ONLY for tempo (Option A). **No struct field is added to `GlobalFeatures` / `ScanBarFeatures`. No `pure_analysis.rs` mirror work. No `main.rs` boundary-copy sync.**

### Why (justification — minimizes engine.rs churn AND seam risk):

Both diagnoses converge on the key fact: **the four strongly-discriminating features already cross the seam today.** `GlobalFeatures` carries `avg_brightness`, `hue_spread`, `texture_laplacian_var`, `shape_complexity`, `aspect_ratio` (`engine.rs:33-50`); `ScanBarFeatures` carries `texture_laplacian_var` and the 8-bin `hue_hist` (`engine.rs:59-74`). The image diagnosis §4/§6 explicitly states "**a large amount of new musical variation can be unlocked with NO seam change at all**" and recommends doing the no-struct-change wins first. The music diagnosis §5 agrees: every fix except tempo "fits inside the files the music side owns," and even mode/key off `hue_hist` needs no new field because the histogram is already on the structs.

Option-NORM-SEAM (adding normalized 0..1 mirror fields to the structs) would force **three** synchronized edits per field — `engine.rs` struct, `pure_analysis.rs` mirror, and the `main.rs` OpenCV boundary copy (`main.rs:147-172`) — exactly the "S9 §6 risk 2" de-sync hazard the struct comments warn about, plus it widens the byte-frozen seam for no behavioral gain the music layer can't get by normalizing the raw value itself. The normalization is a **deterministic, calibrated `clamp(raw / range_max, 0, 1)`** — there is no reason it must live on the image side; placing it in the music layer keeps the seam carrying *plain physical scalars* (the engine's stated boundary discipline: "engine carries plain scalars only") and lets the music layer own its own threshold calibration in `mappings.json`.

### Consequence for the implementers:

- **Implementer E's job SHRINKS to: the tempo touch (Option A in `set_features_global`) + the modal-interchange `0.0` removal at `engine.rs:343`.** No `pure_analysis.rs` change is required for the S13 diversity fix. (`pure_analysis.rs` is listed in E's ownership only as the boundary of "image-side exposure if it had been needed" — under NORM-MAP it is NOT touched. Stated explicitly so E does not invent work there.)
- **Implementer M owns all normalization**, performed in `chord_engine.rs` against calibrated ranges declared in `mappings.json`, consuming the raw features that already arrive on `GlobalFeatures` / `ScanBarFeatures`.

### Calibrated ranges (the empirical anchors — from image diagnosis §2/§3.1, the six-image measured set)

These are the `range_max` divisors the music layer normalizes against. They are declared in `mappings.json` (so they are tunable without a recompile) and consumed by `chord_engine.rs`. Each maps a raw physical feature to a usable 0..1 "knob":

| Knob (0..1) | Raw source feature | Normalization | Calibrated max | Measured raw spread (6 imgs) → knob spread |
|---|---|---|---|---|
| `edge_activity` | `edge_density` (global or per-bar, 0..~0.05) | `clamp(raw / 0.05, 0, 1)` | `0.05` | 0.005–0.036 → **0.10–0.72** |
| `texture` | `texture_laplacian_var` (0..~2000) | `clamp(raw / 2000.0, 0, 1)` | `2000.0` | 328–1958 → **0.16–0.98** |
| `complexity` | `shape_complexity` (0..~2.0) | `clamp(raw / 2.0, 0, 1)` | `2.0` | 0.011–2.005 → **0.006–1.0** (180× raw spread; superb) |
| `colorfulness` | `hue_spread` (already ~0..1) | `clamp(raw, 0, 1)` (identity) | `1.0` | 0.01–0.69 → **0.01–0.69** |
| `brightness01` | `avg_brightness` (0..100) | `raw / 100.0` | `100.0` | 29–81 → **0.29–0.81** |
| `saturation01` | `avg_saturation` (0..100) | `raw / 100.0` | `100.0` | 30–65 → **0.30–0.65** |

> **Calibration note for M:** these divisors are the empirical *max-of-six*; they are intentionally a touch generous so a busier-than-sample image still lands in range (the `clamp` protects the top). They live in `mappings.json` under a new `"feature_normalization"` block (§5.3) so re-tuning never needs a recompile. Do NOT bake them as `const` in `chord_engine.rs`.

---

## 1. WHAT CHANGES, IN ONE PICTURE

```
IMAGE SIDE (UNCHANGED under NORM-MAP)
  pure_analysis.rs / main.rs / GlobalFeatures / ScanBarFeatures structs
        │  (raw physical scalars already crossing the seam)
        ▼
ENGINE  (Implementer E — TWO edits only)
  set_features_global():
     • [E-1] derive ms_per_step from avg_brightness via brightness_to_tempo_bpm  (Option A)
     • [E-2] pass a real brightness-derived drop instead of hardcoded 0.0
        │  (engine still carries only plain scalars across decide_instrument_action)
        ▼
MUSIC SIDE (Implementer M — all normalization + all new musical dimensions)
  chord_engine.rs:
     • normalize raw features → 0..1 knobs using mappings.json ranges
     • harmonic complexity (7ths/9ths) from saturation01
     • continuous articulation curve from edge_activity (kills "uniformly short")
     • per-image rhythmic-density bias from edge_activity/texture
     • mode/key off hue_hist dominant bin (kills mode collapse)
     • the chord_engine.rs:125 `next` secondary-dominant fix + recalibrated trigger
  mapping_loader.rs:  parse the new feature_normalization + harmonic-complexity ranges
  mappings.json:      add feature_normalization block; recalibrate edge thresholds
```

---

## 2. EXACT FEATURE → MUSICAL-DIMENSION MAP

Each row is a dimension to drive. "Source" is the **raw** seam feature; "Knob" is its normalized form (§0); formulas use the knob unless noted. Continuous curves replace the dead 3-band cutoffs wherever the diagnoses call for it (especially articulation).

| Musical dimension | Source feature (raw) | Knob | Formula (input range → output range) | Music-theory rationale |
|---|---|---|---|---|
| **Tempo / step-duration** (E-owned) | `avg_brightness` (global) | `brightness01` | `bpm = lerp_over_map(brightness_to_tempo_bpm, avg_brightness)` then `ms_per_step = round(60000 / bpm)`. Interpolate **continuously** across the JSON anchor points (60/90/120 BPM at brightness 30/70/100), not 3 buckets. brightness 29→~60 BPM→1000 ms; 81→~106 BPM→566 ms. | Luminance is the canonical visual correlate of energy/arousal; bright→fast, dark→slow (film-scoring intuition). Continuous so two bright-but-different images still differ. |
| **Harmonic complexity** (chord-tone count) | `avg_saturation` (global) | `saturation01` | Threshold on `saturation_to_harmonic_complexity` ranges: `<0.31`→triad (3 notes); `0.31..0.71`→add diatonic 7th (4 notes); `≥0.71`→7th + 9th (5 notes). Diatonic 7th = `scale[(degree+6)%7]`; 9th = `scale[(degree+1)%7]` +12. | Saturation = vividness of color = richness of harmony. 7ths/9ths are what make harmony "breathe" instead of "computer triads" — directly targets the operator's "computer-like." |
| **Articulation / note-length** | per-bar `edge_density` | `edge_activity` (per-bar) | **Continuous, replacing the 3-band step:** `frac = lerp(LEGATO_FRAC_HI .. STACCATO_FRAC, edge_activity)` where `LEGATO_FRAC_HI = 1.05` (crosses the step boundary for connected lines) and `STACCATO_FRAC = 0.40`. So `edge_activity=0`→`1.05` (overlapping legato); `=1`→`0.40` (detached). Additionally bias by a **global** articulation scalar `g_artic = 1.0 - texture` (smooth image ⇒ globally more legato): `frac *= lerp(1.10, 0.90, texture)`. Final `frac` clamped to `0.30..=1.20` (the existing `sustained()` cap). | Legato↔staccato is the single biggest "played vs typed" cue. The old step function snapped every real photo (edge<0.25) to one LEGATO value AND capped legato at 0.95 (never overlapping) → uniform short detached feel. Continuous + ceiling>1.0 lets calm images sing across the bar. |
| **Note density / rhythm pattern** | per-bar `edge_density` (local) + global `edge_density`/`texture` (image bias) | `edge_activity` (both) | Keep the per-step pattern selection but **recalibrate against `edge_activity`** (not raw edge), and add a per-image density bias `g_density = clamp(0.5 + 0.5*global_edge_activity + 0.25*texture, 0.5, 1.5)`. Pattern band cutoffs become: arpeggio `edge_activity>0.80`; syncopated `>0.55`; dotted `>0.25`; else sustained. The onset count for the active patterns is scaled by `round(base_onsets * g_density)`. | Busy image → globally denser piece even on its calm steps; local texture still selects the per-step figure. Two equally-bright images of different busyness now differ in rhythm. Recalibration is essential: raw edge 0.005 was deep below every cutoff → every image legato/sustained. |
| **Dynamics / velocity** | per-bar `avg_saturation` | `saturation01` | UNCHANGED level formula (`-12 + sat/100*30`) — already per-step varying. *Optional* spread widening (defer): drive the `±` window by a global contrast scalar. **Not required for S13 acceptance; leave the velocity body alone to protect the equivalence golden** (see §7). | Saturation = boldness of tone. Already wired; the velocity golden in `engine_equivalence.rs` pins this exact formula — do not touch it. |
| **Register / orchestration role** | per-bar `avg_brightness` | (raw, 0..100) | UNCHANGED. brightness→octave lift in `role_pitch` is already per-step varying and is pinned by the golden (G_MELODY_NOTE derivation). Leave it. | Already wired and golden-pinned. |
| **Mode** | global `hue_hist` **dominant bin** (NOT `avg_hue`) | n/a (categorical) | Replace `lookup_range_map(hue_to_mode, avg_hue)` with: compute the argmax of the global `hue_hist` 8-bin histogram → bin-center hue (0..360) → `lookup_range_map(hue_to_mode, dominant_hue)`. Fall back to `avg_hue` if the histogram is empty/degenerate. | An image's *characteristic* color is its most-present hue, not the circular mean (which averages a red+green image to a muddy middle and regresses to one mode). |
| **Key / root transposition** | global `hue_hist` **secondary bin** (optional, gated) | n/a | If a clear secondary bin exists (mass ≥ 0.5× dominant mass), transpose `root_midi` by a small diatonic interval keyed off the secondary hue (e.g. map secondary-bin index → {0,+2,+4,+5,+7} semitones). Else keep `root_midi` (C4). **Bounded** so it never leaves a singable register. | Gives a *second* color-driven tonal axis so diverse palettes stop collapsing to one mode AND one key. Optional — land mode first; key transposition is the stretch goal of M's work. |
| **Modal interchange (borrowed `iv`)** (trigger revived by E-2) | global `avg_brightness`-derived drop | `brightness01` | E passes `brightness_drop = clamp((0.5 - brightness01)*2, 0, 1)` (dark image ⇒ larger drop) instead of `0.0`. M leaves the `>0.25` threshold OR retunes it in JSON to fire on the dark end of the real range. | Dark/low-key images borrow the minor subdominant — the textbook "shadow" color. Revives a dead trigger with an honest feature. |
| **Secondary dominant (V/next)** | global `edge_density` recalibrated | `edge_activity` (global) | See §4. Trigger recalibrated from raw `0.7` to `edge_activity > 0.55` (fires on the busy half of real photos), and the `next` look-ahead wired correctly so it inserts the dominant *of the following chord*. | First non-diatonic color + root-motion-by-fourth propulsion; "busy image → more driving harmony" becomes an audible per-image axis. |

---

## 3. THE ENGINE.RS TEMPO CHANGE (Implementer E — exact edit)

**Location:** `PipelineEngine::set_features_global` (`engine.rs:328-357`). **This is the ONLY function E changes for tempo.** `decide_instrument_action`, `decide_step`, `tick`, the `FeatureSource`/`AudioSink` traits, and the struct definitions are **UNTOUCHED.**

**Why this is safe relative to the freeze:** `set_features_global` is the *plan-derivation path*, not the decision kernel. It is already non-deterministic (`pick_progression` uses `thread_rng`) and the equivalence net deliberately does **not** exercise it (`engine_equivalence.rs` builds a fixed plan and passes `MS_PER_STEP` explicitly). Overwriting `self.config.ms_per_step` here does not alter `decide_instrument_action`, which receives `ms_per_step` as a parameter from `decide_step` (`engine.rs:457`) — the new value simply flows through unchanged plumbing.

### E-1: derive tempo from brightness

Add, near the top of `set_features_global` (after the existing `mode` lookup, before/after progression derivation — order does not matter, it only mutates `self.config`):

```rust
// S13: image-driven tempo. brightness → BPM via the (previously dead)
// brightness_to_tempo_bpm map, continuously interpolated, then BPM → ms/step
// (one step = one beat). This is the plan-derivation path, NOT the decision
// kernel; the new ms_per_step flows to decide_instrument_action through the
// existing parameter at decide_step (engine.rs:457). engine_equivalence.rs is
// unaffected (it passes MS_PER_STEP explicitly and never calls this path).
let bpm = interp_tempo_bpm(
    &self.mappings.global.brightness_to_tempo_bpm,
    global.avg_brightness,
);
self.config.ms_per_step = (60_000.0 / bpm.max(1.0)).round() as u64;
```

E adds a small free helper in `engine.rs` (the continuous interpolation over the JSON BPM anchors — `brightness_to_tempo_bpm` is `HashMap<String,u32>` keyed by `"0-30"`/`"31-70"`/`"71-100"`):

```rust
/// S13 helper: continuous brightness(0..100) → BPM over the JSON anchor map.
/// Parses each "lo-hi": bpm entry into (range-midpoint, bpm) anchor points,
/// sorts by midpoint, and linearly interpolates (clamped at the ends). A
/// continuous map (not 3 buckets) so two bright-but-different images differ.
fn interp_tempo_bpm(map: &std::collections::HashMap<String, u32>, brightness: f32) -> f32 {
    let mut anchors: Vec<(f32, f32)> = map
        .iter()
        .filter_map(|(k, v)| {
            let mut it = k.split('-');
            let lo: f32 = it.next()?.trim().parse().ok()?;
            let hi: f32 = it.next()?.trim().parse().ok()?;
            Some(((lo + hi) * 0.5, *v as f32))
        })
        .collect();
    anchors.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    if anchors.is_empty() {
        return 60_000.0 / 250.0; // == 240 BPM ⇒ preserves the legacy 250 ms default
    }
    if brightness <= anchors[0].0 {
        return anchors[0].1;
    }
    if brightness >= anchors[anchors.len() - 1].0 {
        return anchors[anchors.len() - 1].1;
    }
    for w in anchors.windows(2) {
        let (x0, y0) = w[0];
        let (x1, y1) = w[1];
        if brightness >= x0 && brightness <= x1 {
            let t = (brightness - x0) / (x1 - x0);
            return y0 + t * (y1 - y0);
        }
    }
    anchors[anchors.len() - 1].1
}
```

> **CLI-default note (FLAG, do not assume):** `--ms-per-step` (default 250, `cli.rs:83`) is now *overwritten* per image when an image is loaded. The CLI flag still seeds the initial `EngineConfig` and still governs any path that does NOT call `set_features_global` (e.g. a TUI seek before the first image feed). **E must NOT edit `cli.rs`.** If the operator wants the flag to act as a clamp/override rather than be silently superseded, that is a follow-up decision — flag it in the PR description, do not implement it now. (The fallback BPM in `interp_tempo_bpm` is deliberately 240 BPM = 250 ms so a degenerate/empty map preserves today's tempo.)

### E-2: revive modal interchange (replace the hardcoded `0.0`)

At `engine.rs:343`, change the `generate_chords` call's last argument from the literal `0.0` to a brightness-derived drop:

```rust
// S13: real modal-interchange trigger. Dark/low-key image ⇒ larger "drop" ⇒
// borrow the minor iv (the shadow subdominant). Was hardcoded 0.0 (never fired).
let brightness_drop = (0.5 - global.avg_brightness / 100.0).clamp(0.0, 1.0) * 2.0;
let chords = chord_engine.generate_chords(
    &progression,
    self.config.root_midi,
    &mode,
    global.edge_density,   // unchanged: M recalibrates the threshold side
    brightness_drop,       // S13: was 0.0
);
```

That is the **entire** engine.rs change surface: one helper fn + the tempo overwrite + the one-argument swap. `global.edge_density` continues to be passed raw; M owns recalibrating the threshold it is compared against (§4).

---

## 4. THE `chord_engine.rs:125` `next` FIX (Implementer M)

**Current (buggy) code (`chord_engine.rs:116-130`):** the secondary-dominant block computes `let next = &progression[i + 1];` then **discards it**, building `roman_to_chord("V", ...)` with a literal `"V"` (the home-key dominant) — hence the live `unused variable: next` warning and the "always plain home V" behavior.

**Fix — two coupled changes:**

1. **Honor `next` (the look-ahead).** Spell the inserted chord as the **dominant of `next`'s scale degree**, i.e. a true secondary dominant V/x. Add a private helper to `ChordEngine`:

```rust
/// S13: build the SECONDARY DOMINANT of `target` (the "V/x"): the major triad
/// (or dom7, per harmonic complexity) rooted a perfect fifth above target's
/// root. theory: tonicizing the next chord by inserting its own dominant gives
/// root-motion-by-fourth propulsion + the first non-diatonic (chromatic) tone
/// this engine produces. `with_seventh` adds the dominant 7th for a stronger pull.
fn secondary_dominant_of(
    &self,
    target_roman: &str,
    root_midi: u8,
    scale: &[i8; 7],
    with_seventh: bool,
) -> Chord {
    // degree of the target within the home scale
    let target_degree = roman_degree(target_roman); // factor the existing match out
    let target_root = (root_midi as i16 + scale[target_degree as usize] as i16) as u8;
    // V/x root = a perfect fifth (7 semitones) above the target's root
    let v_root = target_root.saturating_add(7);
    // a MAJOR triad on v_root: root, +4 (major 3rd), +7 (perfect 5th); dom7 adds +10
    let mut notes = vec![v_root, v_root.saturating_add(4), v_root.saturating_add(7)];
    if with_seventh {
        notes.push(v_root.saturating_add(10)); // minor 7th ⇒ dominant-seventh quality
    }
    Chord { name: format!("V/{target_roman}"), notes }
}
```

   (Factor the Roman→degree `match` currently inline in `roman_to_chord` into a free `fn roman_degree(roman: &str) -> u8` so both call sites share it — a pure refactor inside M's file.)

   Then in the block at `:124-128`:

```rust
if i + 1 < progression.len() {
    let next = &progression[i + 1];                       // NOW CONSUMED
    let v_of_next = self.secondary_dominant_of(next, root_midi, &scale, with_seventh);
    chords.push(v_of_next);
}
```

2. **Recalibrate the trigger so it actually fires.** The block is gated by `edge_complexity > dominant_substitution_trigger.edge_complexity_threshold` with the raw threshold `0.7` (`mappings.json:23`) — never reached by real photos (raw edge 0.005–0.036). Two options; **M chooses (a):**
   - **(a) [chosen] Lower the JSON threshold to the recalibrated `edge_activity` space.** Set `edge_complexity_threshold: 0.55` in `mappings.json` AND have `generate_chords` compare against the **normalized** `edge_activity` (= `clamp(edge_complexity / 0.05, 0, 1)`) rather than raw `edge_complexity`. Because `generate_chords` receives the raw value from E (unchanged), M normalizes at the top of `generate_chords`:
     ```rust
     let edge_activity = (edge_complexity / self.mappings.feature_normalization.edge_density_max).clamp(0.0, 1.0);
     // ... gate on `edge_activity > threshold`
     ```
     This fires on the busy half of the real set (example.jpg edge_activity 0.72, AudioHaxImg2 0.51) while staying off for calm images — exactly the intended "busy image → driving harmony" axis.

> **Interaction with harmonic complexity (§2):** `with_seventh` for the secondary dominant is wired to the same `saturation01` complexity decision as the diatonic chords — a vivid busy image gets a dominant-*seventh* secondary (strongest pull), a washed-out one gets a bare secondary triad. This dovetails the §3 (music doc) caveat that "the inserted chord should ideally be a dominant-seventh."

> **Voice-leading safety:** `voice_lead_sequence` / `plan_phrases` already generalize over N-note voicings (the music doc confirms `voice_lead_one` handles a 4th tone), so inserting a 3- or 4-note secondary dominant flows through the existing phrase planner without signature changes.

---

## 5. FILE-OWNERSHIP SPLIT (file-disjoint; the collision contract)

### Implementer E (engine / image seam) — **TWO edits in ONE file**
- **`src/engine.rs`** ONLY:
  - **E-1** tempo Option A: the `interp_tempo_bpm` helper fn + the `self.config.ms_per_step = …` overwrite inside `set_features_global`.
  - **E-2** modal-interchange: replace the `0.0` literal at the `generate_chords` call (`engine.rs:343`) with the `brightness_drop` expression.
- **`src/pure_analysis.rs`** — **NOT TOUCHED.** Under the chosen Option-NORM-MAP there is no image-side work. (Listed here only to state explicitly: E does **not** add normalized mirrors; that decision was made in §0.)

### Implementer M (music) — three files, all disjoint from E
- **`src/chord_engine.rs`:** normalization-at-consumption (raw → 0..1 knobs via `mappings.json` ranges); harmonic complexity (7ths/9ths in `roman_to_chord` / a complexity-aware builder); continuous articulation curve in `realize_rhythm` (replace the 3-band `base_frac`); per-image rhythmic-density bias; mode off `hue_hist` dominant bin (in `set_features_global`'s **caller-supplied data** — see contract below); optional key transposition; the §4 `secondary_dominant_of` + `roman_degree` refactor + `next` fix + trigger recalibration.
- **`src/mapping_loader.rs`:** add the `FeatureNormalization` struct + field on `MappingTable` (and into `rebuild_mapping_table` — see note); add a numeric range-map lookup helper if needed for harmonic complexity (the existing `lookup_range_map` returns `String`; complexity bands are strings already, so reuse it).
- **`assets/mappings.json`:** add the `feature_normalization` block; recalibrate `dominant_substitution_trigger.edge_complexity_threshold` to `0.55`; (optionally) retune `modal_interchange_trigger.brightness_drop_threshold`.

> **`rebuild_mapping_table` coordination (the one shared touch-point):** `rebuild_mapping_table` lives in **`engine.rs`** (`engine.rs:577-622`) and hand-copies every `MappingTable` field. If M adds a `feature_normalization` field to `MappingTable` in `mapping_loader.rs`, `rebuild_mapping_table` will **fail to compile** until that field is copied too. **Resolution to avoid an E/M collision in engine.rs:** make `feature_normalization` a `#[derive(Clone)]` struct in `mapping_loader.rs` and have **M** add the single line `feature_normalization: t.global.feature_normalization.clone()` to `rebuild_mapping_table`. This is the *only* line M writes in `engine.rs`, and it is in a different function (`rebuild_mapping_table`) than E's edits (`set_features_global` + the `interp_tempo_bpm` helper) — **disjoint by function, mergeable without conflict.** Sequence it after E lands (§7) so M edits a known-good `set_features_global`. *(Alternative if even that one shared file is unacceptable: add `#[derive(Clone)]` to `MappingTable` in `mapping_loader.rs` — the comment at `engine.rs:576` says doing so "collapses every call here to `table.clone()`," but that is a larger refactor; the single-line add is cleaner for S13.)*

### EXCLUDED from BOTH (do not touch):
`src/synth_sink.rs`, `src/midi_output.rs`, `src/cli.rs` (except the flagged tempo-default note — flag in PR, do NOT edit), `src/tui.rs`, `src/modem.rs`, `src/bin/modem_*`, `src/lib.rs`.

### THE INTERFACE CONTRACT (the field names / types / ranges E and M agree on)

The seam itself does **not** change (NORM-MAP). The contract is therefore about the **JSON `feature_normalization` block** (M authors; nothing E consumes) and the **scalars E feeds through unchanged**:

1. **E → M via `GlobalFeatures` (existing fields, raw, unchanged):** `avg_brightness: f32` (0..100), `avg_saturation: f32` (0..100), `edge_density: f32` (0..~0.05), `hue_spread: f32` (~0..1), `texture_laplacian_var: f32` (0..~2000), `shape_complexity: f32` (0..~2.0). These already arrive on the `&GlobalFeatures` M's harmony code sees in `set_features_global`.
2. **E → M via `ScanBarFeatures` (existing fields, raw, unchanged):** per-bar `edge_density` (0..~0.05), `texture_laplacian_var` (0..~2000), and the 8-bin `hue_hist: Vec<f32>` (sums to 1). M's `realize_rhythm` and mode logic read these as-is. *(Note: per-bar features reach `realize_step` only through the `PerfFeatures` projection in `decide_instrument_action`, which carries `saturation/brightness/edge_density`. For the per-bar articulation curve M needs only `edge_density`, which is already in `PerfFeatures` — no projection widening, no engine.rs touch. For `hue_hist`-driven mode, M reads the **global** `hue_hist`… see contract item 4.)*
3. **E → M via `EngineConfig.ms_per_step`:** E overwrites it per image; M's `realize_rhythm` receives it as the `ms_per_step` parameter exactly as today. No new field.
4. **The one data-availability item M must confirm:** mode currently keys off `global.avg_hue`. `GlobalFeatures` has **no `hue_hist` field** — only `ScanBarFeatures` does (per-section). For the §2 "mode off dominant hue bin" fix, M has two no-engine-edit options: **(i)** derive a dominant hue from the **per-bar** `hue_hist` rows the source already produces (aggregate the section histograms M can reach via the `FeatureSource` the engine already holds) — but `set_features_global` only receives `&GlobalFeatures`, not the per-bar rows; OR **(ii) [recommended] keep mode on `avg_hue` for S13** and instead break the *collapse* by also letting `hue_spread` (`colorfulness`) widen mode toward modal-mixture (revive `texture_to_modal_color` / borrowed chords) — this needs no new global histogram and no engine struct change. **Decision:** ship **(ii)** for S13 (no seam change, satisfies "decouple from the mean" via the secondary `colorfulness` axis); a true global `hue_hist` is an Option-NORM-SEAM follow-up explicitly deferred. *(This keeps the §0 promise of zero struct change. If M finds the per-bar aggregation reachable without an engine signature change, (i) is an acceptable upgrade — but it is NOT required and must not become a seam edit.)*

> **Net collision surface:** E writes only inside `set_features_global` + a new free helper in `engine.rs`. M writes in `chord_engine.rs` + `mapping_loader.rs` + `mappings.json`, plus **one line** in `engine.rs`'s `rebuild_mapping_table` (different function, sequenced after E). No two edits share a function. Mergeable.

---

## 6. PROPERTY-TEST TARGETS (Implementer T — dry-synth-independent)

All run headless via `cargo test --lib --no-default-features` (or as an integration test under `tests/`) against `generate_chords`, `realize_step` / `realize_rhythm`, and `decide_step` — **never** the synth. Construct distinct synthetic feature vectors **without OpenCV / a real image** by building `GlobalFeatures` / `ScanBarFeatures` literals directly (the `bar(...)` fixture pattern in `engine_equivalence.rs:74-84` is the template) and, for the tempo property, a tiny `FeatureSource` stub over canned rows (the `engine_seam.rs` pattern).

### Constructing two DISTINCT feature vectors (the fixture)

```rust
// "Calm dark" image A vs "busy bright vivid" image B — chosen so they differ on
// EVERY driven axis (tempo, articulation, rhythm, harmony, mode-mixture).
fn global_a() -> GlobalFeatures { GlobalFeatures {
    avg_hue: 40.0, avg_saturation: 20.0, avg_brightness: 25.0,
    edge_density: 0.004, hue_spread: 0.05, texture_laplacian_var: 300.0,
    shape_complexity: 0.02, aspect_ratio: 1.0 } }
fn global_b() -> GlobalFeatures { GlobalFeatures {
    avg_hue: 40.0, avg_saturation: 90.0, avg_brightness: 85.0,   // same hue on purpose
    edge_density: 0.040, hue_spread: 0.65, texture_laplacian_var: 1900.0,
    shape_complexity: 1.9, aspect_ratio: 1.0 } }
```

(Note A and B share `avg_hue=40` deliberately — proving diversity comes from the *other* axes, not just mode.)

### The assertions

1. **Tempo varies with the image (regression guard on E-1).** Build a `PipelineEngine`, `set_features_global(&global_a())` → record `engine.config().ms_per_step` as `ms_a`; same for B → `ms_b`. Assert `ms_a != ms_b` AND `ms_a > ms_b` (darker A is slower). *Today this is CONSTANT (always 250) — the test is RED before the fix.*
2. **Harmonic complexity varies with saturation (§2).** `generate_chords(prog, 60, "Ionian", edge=0.004, drop=0.0)` with the mapping's complexity at low-sat vs high-sat: assert low-sat chords all have `notes.len() == 3`; high-sat chords contain at least one with `notes.len() >= 4` (a 7th present). (Drive the saturation through the same path generate_chords reads, or test the chord builder directly with the complexity arg.)
3. **Mean note-length varies with edge — and A (calm) > B (busy) (§2 articulation; the "uniformly short" killer).** For each of A and B, run `realize_rhythm` (or `realize_step` for the melody role) across a fixed plan; compute mean `hold_ms / ms_per_step` over all emitted events. Assert `mean_frac_a - mean_frac_b > EPS` (calm holds longer) AND `mean_frac_a > 0.95` (A actually crosses into connected/legato territory — the old code capped at 0.95 and snapped everything there). *Use a FIXED `ms_per_step` for this test so tempo doesn't confound the fraction.*
4. **Rhythm-pattern SET differs (§2 density).** Collect the multiset of onset counts per step (`events.len()` per step) for A vs B over the same plan. Assert the two multisets are **not equal** (B, busier, has steps with more onsets). Not "a pattern exists" — the *distribution* differs.
5. **Mode/mixture diversity (§5 contract item 4, option ii).** Two globals with the **same `avg_hue`** but different `hue_spread` (A low, B high) → assert the generated chord *sets* differ in borrowed/mixture content (B contains at least one borrowed chord the low-spread A does not), proving mode-feel decoupled from the mean via the `colorfulness` axis. *(If M ships option (i) instead, assert two same-circular-mean / different-dominant-bin histograms yield different modes.)*
6. **THE HEADLINE ACCEPTANCE TEST — distinct images differ in ≥3 dimensions, not just mode.** For A vs B, assemble the tuple `(ms_per_step, mean_note_frac, onset-count-multiset, max-chord-tone-count, mode_or_mixture_signature)` and assert **at least 3 of the 5 components differ**. This is the operator's complaint encoded: "different image ⇒ different music in more than one dimension." This test is the gate for the whole session.
7. **Secondary dominant fires and is correct (§4).** A high-`edge_density` global (e.g. 0.045 → edge_activity 0.90) → `generate_chords` inserts a chord named `"V/<next>"` whose root is a perfect fifth above `next`'s root (a chromatic, non-home-key tone), AND the `next` variable is consumed (the `unused variable` warning is gone — verify via `cargo build` warning-free, or structurally by asserting the inserted chord differs for different `next` chords). A low-edge global inserts NO secondary dominant.
8. **Modal interchange can fire (§3 E-2).** A dark global (`avg_brightness` low → `brightness_drop > 0.25`) over a progression containing `"IV"` → the realized chord set contains the borrowed minor `iv` (today: never, `0.0` hardcoded).
9. **Determinism preserved.** With `pick_progression`'s RNG bypassed (call `generate_chords` on an explicit progression, never the `set_features_global` RNG path for the harmony asserts), all of the above are deterministic functions of the feature vector — the existing test discipline holds.

---

## 7. RISKS / SEQUENCING

### The golden regression net WILL change — deliberately, not silently

`tests/engine_equivalence.rs` pins `realize_step` / `realize_rhythm` golden constants. **The continuous-articulation change (§2) alters `realize_rhythm`'s `base_frac` path and therefore some golden holds.** Specifically:

- **Cadence golden (`G` cadence hold = 240 ms):** the cadence branch returns `sustained(0, step_ms, LEGATO_FRAC)` and is **independent of the new edge curve** (it hard-codes `LEGATO_FRAC`). If M leaves the cadence branch untouched (recommended — only change the *non-cadence* `base_frac`), **the cadence golden stays 240 and that test stays green.** Keep the cadence branch byte-stable.
- **Non-cadence holds / arpeggio onset counts:** any test asserting a *non-cadence* melody/bass hold fraction or onset count under a specific edge value may shift because (a) `base_frac` is now a continuous lerp and (b) the band cutoffs now compare `edge_activity` (normalized) not raw edge. Scanning the current net: `test_step_idx_wraps_via_modulo` asserts a **high-edge melody arpeggiates to 3 onsets at edge=0.9**. Under recalibration, raw 0.9 → edge_activity `clamp(0.9/0.05,0,1)=1.0` → still `> 0.80` → still 3 onsets. **That test survives** because 0.9 normalizes to the top. But any *new* assertion at realistic edge (0.005–0.04) lands in different bands than the old raw cutoffs — that is the intended fix, and the **net's golden constants for any non-cadence hold fraction must be re-derived by hand and updated in the same commit, with a comment pointing to this spec.** Do not silence a red golden by loosening the assert; re-derive the constant from the new formula and document the derivation (the net's own header demands hand-derived goldens).
- **Velocity goldens (P4/P6, the 114/84 cadence velocities):** **UNCHANGED** — §2 leaves `realize_velocity` alone. These must stay green; if they go red, M touched velocity by mistake.
- **Register goldens (G_BASS_NOTE=36 / G_MELODY_NOTE=79):** **UNCHANGED** — §2 leaves `role_pitch` alone. Must stay green.

**Action:** treat the engine_equivalence golden update as a reviewed, intentional diff. The reviewer (Quality Gate) confirms each changed golden is re-derived from the new documented formula, not loosened.

### Regression nets that must stay GREEN (no expected change):
- `cli_parse.rs` — no CLI edit (the tempo-default is flagged, not changed).
- `engine_seam.rs` — the `FeatureSource`→engine plumbing is unchanged (NORM-MAP).
- `tui_render.rs` — no `tui.rs` edit.
- `phase2_pure_pipeline.rs` — `pure_analysis.rs` is untouched under NORM-MAP; the pure pipeline output structs are byte-identical.
- `modem_realair.rs`, `modem_roundtrip.rs` — modem path wholly untouched.
- `qg_probe_band_isolation.rs` — confirm it does not pin an articulation band that moves; if it does, treat like the engine_equivalence goldens (re-derive, document).

### Other risks
- **Tempo overwriting the CLI flag silently** (§3 flag): documented, fallback BPM preserves 250 ms on a degenerate map; surface the "flag is now superseded per-image" behavior in the PR. Low risk, but it is an operator-visible semantics change.
- **Secondary-dominant register blowout:** `secondary_dominant_of` adds 7/10 semitones via `saturating_add` on `u8` — a high `next` root could clip at 127; `voice_lead_sequence` and the `role_pitch` register seating re-place tones into band 24..=108 downstream, so the raw chord notes are intermediate. Verify property-test 7 asserts the *final* realized notes stay in 24..=108.
- **Key transposition leaving singable register:** the optional §2 key shift is bounded to ±{0,2,4,5,7} semitones and must keep `root_midi` such that the bass register floor stays ≥24 — keep it optional and bounded; defer if it threatens goldens.
- **`rebuild_mapping_table` compile coupling** (§5): the single shared line. Sequenced so M adds it after E lands.

### SEQUENCING (the seam contract is fixed FIRST)

1. **Lock the JSON contract first (M, no code-behavior change yet):** add the `feature_normalization` block + the `dominant_substitution_trigger.edge_complexity_threshold: 0.55` retune to `mappings.json`, and the `FeatureNormalization` struct (+`#[derive(Clone)]`) to `mapping_loader.rs`. This freezes the field names/types/ranges both implementers reference. (Compiles once `rebuild_mapping_table` gets the one-line copy — step 2.)
2. **E lands the engine.rs tempo + modal-interchange edits** against the now-fixed contract (E does not depend on M's normalization — E only reads existing raw `GlobalFeatures` fields and overwrites `ms_per_step`). Then **M adds the single `rebuild_mapping_table` copy line.** engine.rs is now stable for the rest of the session.
3. **M lands the music-side dimensions in `chord_engine.rs`** (normalization-at-consumption, harmonic complexity, continuous articulation, density bias, mode-mixture, the `next` secondary-dominant fix + trigger recalibration). M re-derives and updates the affected `engine_equivalence.rs` non-cadence goldens in the SAME commit, documented.
4. **T writes the property tests** (§6) — ideally authored against the spec *before* step 3 so §6.1/§6.3/§6.6 are RED first, then GREEN after M lands (the music doc's "regression guard" intent).
5. **Full net pass:** `cargo test --lib --no-default-features` + the named integration nets all green; the only intentional diffs are the re-derived non-cadence goldens.

---

*End of spec. Implementers M and E are file-disjoint (M: chord_engine.rs / mapping_loader.rs / mappings.json + one sequenced line in rebuild_mapping_table; E: engine.rs set_features_global + helper). The seam does not change — Option-NORM-MAP normalizes raw already-crossing features in the music layer against calibrated ranges in mappings.json. The single highest-impact fix, image-driven tempo, lands via the approved engine.rs Option-A overwrite. The headline acceptance gate is property-test §6.6: two distinct feature vectors must differ in ≥3 musical dimensions, not just mode.*

# Quality-Gate Review — Session 13 "Lane 2: image→music diversity"

**Reviewer role:** Quality Gate (independent verification — last checkpoint before integration).
**Date:** 2026-06-14
**Scope reviewed:** `src/chord_engine.rs`, `src/mapping_loader.rs`, `assets/mappings.json` (Implementer M); `src/engine.rs` (Implementer E + M's one coordination line + M's 2 call-site args); `tests/diversity_s13.rs` (Test Engineer). Contract: `docs/design-s13-diversity.md`; background: the two `docs/diagnosis-s13-*.md`.
**Method:** mechanical build/test/lint; line-by-line boundary audit; independent re-derivation of the diatonic 7th/9th and one secondary-dominant root; test-quality audit; deviation + equivalence-drift judgment.

---

## 1. Compilation Status

- `cargo build` (default, pure-Rust): **PASS** (`Finished dev profile`). The only warnings are pre-existing and in files NOT under review (`src/bin/modem_encode.rs` unused `seq`, `make_tiled_payload`/`unpack_tiled_payload` dead fields). No warning on any changed file.

## 2. Lint Status

- `cargo clippy -- -W clippy::all`: **PASS for blocking purposes.** All warnings on the changed files are **style/pedantic** (`useless_vec` on the pre-existing `warm`/`cool` vecs, `doc_lazy_continuation` on the new doc-comments, `unnecessary_cast`, `len_zero`, etc.). **Zero `clippy::correctness` warnings.** Style warnings are NON-BLOCKING.
- **`chord_engine.rs:125` `unused variable: next` warning: RESOLVED / GONE.** Confirmed via `cargo build 2>&1 | grep "unused variable: .next"` → empty. The `next` look-ahead is now consumed by `secondary_dominant_of(next, …)`. This is the load-bearing music-theory fix and the warning that proves it is wired.

## 3. Test Results (per-net counts — all match the expected harness)

| Net | Expected | Actual | Result |
|---|---|---|---|
| lib (full) | 123 | 123 | PASS |
| cli_parse | 24 | 24 | PASS |
| diversity_s13 | 10 | 10 | PASS |
| engine_equivalence | 9 | 9 | PASS |
| engine_seam | 10 | 10 | PASS |
| modem_realair | 10 | 10 | PASS |
| modem_roundtrip | 17 | 17 | PASS |
| phase2_pure_pipeline | 7 | 7 | PASS |
| qg_probe_band_isolation | 1 | 1 | PASS |
| tui_render | 13 | 13 | PASS |
| **lib --no-default-features** | 95 | 95 | PASS |

Full suite green; headless lib green. No failures, no ignored.

## 4. Module Boundary Audit

- **`git diff --stat` confines changes to exactly the 4 authorized files** (`assets/mappings.json`, `src/chord_engine.rs`, `src/engine.rs`, `src/mapping_loader.rs`). Every EXCLUDED file confirmed **UNCHANGED** via `git diff --quiet`: `synth_sink.rs`, `midi_output.rs`, `cli.rs`, `tui.rs`, `modem.rs`, `pure_analysis.rs`, `main.rs`, `lib.rs`, and all `src/bin/*`. **CLEAN.**
- **`chord_engine.rs`:** carries NO image-processing types, NO MIDI-output types, NO OpenCV/`image`/`imageproc` references. It receives plain `f32` scalars (`edge_complexity`, `saturation01_raw`, `colorfulness_raw`, `brightness_drop`) and normalizes them itself via `FeatureNormalization::normalize(raw, range_max)` — a pure `clamp(raw/range_max, 0, 1)`. **Normalization is genuinely scalar-based; it reaches into no image data.** CLEAN.
- **`engine.rs`:** `decide_instrument_action`, `decide_step`, `tick`, the `FeatureSource`/`AudioSink`/`EngineObserver` trait surfaces, and the `GlobalFeatures`/`ScanBarFeatures` struct definitions are **UNTOUCHED** (verified by reading the full diff: it touches only `set_features_global`, the new free `interp_tempo_bpm` helper, the one `rebuild_mapping_table` clone line, and new `#[cfg(test)]` tests). The S9 byte-freeze break is **confined to exactly the authorized surface** — tempo overwrite + `brightness_drop` + M's 2 call-site args + the clone line. No struct field was added to `GlobalFeatures` (the new tempo test constructs it with its 8 existing fields — confirming Option-NORM-MAP's zero-struct-change promise held).
- **`mapping_loader.rs` / `mappings.json`:** the new `feature_normalization` block parses into a `#[derive(Clone)] FeatureNormalization`; no hardcoded musical values that should be data (divisors live in JSON, tunable without recompile, exactly as §0 requires). The canonical `assets/mappings.json` (the only mappings file in the repo) still loads cleanly — all 10 load sites pass.

## 5. Musical Logic Review (Stage 3 — re-derived independently)

- **HARMONIC COMPLEXITY — CORRECT.** `roman_to_chord_complex` builds the triad by stacking diatonic thirds: 3rd = `(deg+2)%7`, 5th = `(deg+4)%7`. The **7th = `scale[(deg+6)%7]`** and the **9th = `scale[(deg+1)%7] + 12`**. I re-derived: a diatonic seventh chord is root-3rd-5th-7th = degrees deg, deg+2, deg+4, deg+6; the 9th = the 2nd (deg+1) an octave up. **Both formulas are exactly right and genuinely diatonic to the mode** (drawn from the same `scale` array, not blind +10/+14 semitones). The independent unit test re-derives Ionian I7add9 = C-E-G-B-D = pcs {0,4,7,11,2} and passes. Complexity tracks saturation monotonically (`<0.31`→Triad/3, `0.31–0.71`→Seventh/4, `≥0.71`→Ninth/5) with sane JSON-driven thresholds and a mirrored fallback. **SOUND.**
- **ARTICULATION CURVE — CORRECT and genuinely CONTINUOUS.** `curve_frac = LEGATO_FRAC_HI + (STACCATO_FRAC − LEGATO_FRAC_HI) * edge_activity` = `1.05 + (0.40 − 1.05)*edge_activity` — a true linear lerp, NOT 3 relabeled bands. Calm (`edge_activity→0`) → 1.05 (overlapping legato across the step boundary); busy (`→1`) → 0.40 (detached). Clamped to `0.30..1.20` so note lengths stay musically sane (no zero/negative, overlap capped at the cadence-ring ceiling). The cadence branch is deliberately byte-stable (returns `sustained(0, step_ms, LEGATO_FRAC)`), protecting the equivalence golden. **SOUND.**
- **SECONDARY DOMINANT `next` FIX — CORRECT (the load-bearing fix).** `secondary_dominant_of` computes `target_root = root_midi + scale[roman_degree(target)]`, then `v_root = target_root + 7` (a P5 above), builds a MAJOR triad (+0/+4/+7), and adds +10 (minor 7th → dom7) when `with_seventh`. I re-derived V/IV in C-Ionian by hand: IV root = 60+5 = 65 (F); V/IV root = 65+7 = **72 (C)** — and C major IS the dominant of F. **Correct.** Different `next` yields a different root (V/IV root 72 ≠ V/V root 79), so it tonicizes the actual next chord, not a constant home V. It is gated on the **normalized** `edge_activity > 0.55` (fires on the busy half of real photos, off for calm). The +4 major third is the chromatic tone; +10 the tritone pull. All notes are legal MIDI (the test asserts ≤127; the register-safety test asserts the final voice-led notes stay in 24..=108). Voice leading is not broken — `voice_lead_sequence`/`plan_phrases` already generalize over N-note voicings and the busy+vivid end-to-end test passes the range check. **SOUND.**
- **MODAL INTERCHANGE — fires correctly, but is a SYMBOL-ONLY swap (NON-BLOCKING DEFECT, see §8).** The trigger is honest: a dark image's engine-derived `brightness_drop = (0.5 − b/100)*2` crosses the 0.25 threshold and swaps `"IV"`→`"iv"` (and only when intended — not always, not never; the old hardcoded `0.0` bug is fixed). BUT: `roman_to_chord_complex("iv", …)` calls `roman_degree("iv")` (case-insensitive) → degree 3 → builds the **diatonic** degree-3 triad. In Ionian that is F-A-C = **F MAJOR — pitch-identical to "IV".** The borrowed *minor* iv (the Ab/minor-third "shadow" the spec §2 promised) is NEVER produced; only the chord's `name` changes. The implementer's own test honestly documents this ("the 'borrowed' effect here is the SYMBOL swap … the minor-third spelling is a follow-up"). The trigger and the symbol axis are real and add a name-level diversity signal, but the **harmonic colour is not borrowed**. This is a correctness gap against the spec's stated intent, not a crash or a regression.
- **bVI MIXTURE — CORRECT.** `flat_submediant` builds a major triad rooted `root_midi + 8` (a minor sixth above the tonic) = bVI, with an optional +11 major 7th. In C, root 68 (Ab) + 4/+7 = Ab-C-Eb = **Ab major = bVI**, the stock parallel-minor mixture chord — correctly spelled and a defensible mixture. Appended only when `colorfulness > 0.45`, decoupling harmonic colour from the single mean-hue mode pick (the §2c collapse). **SOUND.**
- **NORMALIZATION CALIBRATION — SANE.** Divisors land the measured real-photo ranges into usable 0..1 bands, not one extreme: edge_density/0.05 maps the measured 0.005–0.036 → 0.10–0.72 (spans the 0.55 secondary-dominant trigger and the 0.25/0.55/0.80 articulation cutoffs); texture/2000 → 0.16–0.98; shape/2.0 → 0.006–1.0; hue_spread/1.0 identity → 0.01–0.69; brightness & saturation /100 → 0.29–0.81 / 0.30–0.65. `normalize` fails safe on a zero/negative `range_max` (returns 0, no divide-by-zero). The `test_normalization_real_photo_edge_range_is_usable` test confirms >1 rhythm pattern across the real edge spread. **SOUND.**

## 6. Test Quality Assessment

- **`tests/diversity_s13.rs` asserts MEANINGFUL divergence with asserted DIRECTION**, not "music is produced." The headline `test_distinct_images_differ_in_3_dimensions` measures all 5 axes (tempo, articulation, harmony, rhythm-multiset, mixture-signature) and requires **≥3 differ simultaneously**, plus directional guards (busy ⇒ faster, vivid ⇒ more tones). With the chosen fixtures all 5 differ, so the gate is robustly met.
- **Articulation-continuity test (`test_articulation_is_continuous_not_three_bands`)** genuinely proves continuity: 21 fine samples must yield **>3 distinct hold values** AND the largest adjacent jump must be `< step/5` (a 3-band step function would jump hundreds of ms) AND the calm end must reach ≥0.95 of the step. This is a real continuity proof, not a 3-value check.
- **No flakiness.** Every harmony/articulation assert calls `generate_chords`/`realize_step` on an EXPLICIT progression, never the `set_features_global → pick_progression` (`thread_rng`) path. `test_diversity_observables_are_deterministic` pins byte-identical repeat runs. Deterministic.
- **Implementer unit tests validate real properties, not `assert!(true)`.** They use an INDEPENDENT reference mode-interval table (`REF_IONIAN…`) to assert against ground truth; they re-derive the diatonic 7th/9th pitch classes and the V/IV root (72) by hand in the test body; they assert dom7-vs-triad quality, register safety (24..=108), and the normalization calibration arithmetic. Strong discipline.

## 7. Integration & Deviation Assessment

- **M's `engine.rs` call-site deviation (the 2 args): VERDICT — SOUND.** M was told not to touch `engine.rs`, but the frozen 5-arg `generate_chords` signature could not deliver the saturation/colorfulness that §2's harmonic-complexity and mode-mixture axes require. The deviation is a **clean, minimal, boundary-respecting argument pass-through**: it adds `global.avg_saturation` and `global.hue_spread` (two RAW scalars that already exist on `GlobalFeatures`) to the existing call inside `set_features_global`, with comments documenting the NORM-MAP rationale. It adds no struct field, touches no decision kernel, normalizes nothing engine-side (the seam still carries plain raw scalars — exactly the §0 discipline), and sits in the same function E already owns. The alternative (keeping the 5-arg signature and routing saturation/colorfulness some other way) would have been strictly worse: it would force either a struct field (the de-sync hazard §0 rejects) or a second seam path. The deviation is the *correct* minimal realization of §2, not a shortcut. **APPROVED.**
- **engine_equivalence stayed green — LEGITIMATE non-event, with one coverage note.** I verified each pinned golden against the changes: G_BASS_NOTE=36 / G_MELODY_NOTE=79 pin `role_pitch` (untouched); the cadence velocity 114 / hold 240 pin the `is_cadence` branch (deliberately byte-stable, `sustained(0, step_ms, LEGATO_FRAC)`, 0.95*1.30→min1.20→round(200*1.20)=240); `test_step_idx_wraps_via_modulo` pins a high-edge (0.9 → normalized 1.0) arpeggio that still lands in the >0.80 band. The continuous-articulation change altered only the **non-cadence `base_frac`** path, which the equivalence net **never pinned** — so it stays green because it doesn't exercise the changed path, NOT because it hides a regression. The new diversity_s13.rs net covers exactly that changed path. **This is the spec-anticipated outcome and a legitimate non-event.** (Coverage note flagged in §9.)
- **Flagged compromises — judged acceptable:** (1) global texture articulation bias omitted — correct call (texture is not on the per-step `PerfFeatures` seam; adding it would be a forbidden seam change; the per-bar edge curve alone removes uniformity). (2) `EDGE_ACTIVITY_RANGE_MAX` const mirrors the JSON `edge_density_max` — a real but reasonable constraint (`realize_rhythm` is a free fn with no `MappingTable` handle), well-documented with a keep-in-sync warning. (3) busy mean-fraction landing below the raw curve constant — expected (the busy melody arpeggiates to STACCATO_FRAC onsets). All three are acceptable; (1) and (2) are worth tracking as follow-ups.

## 8. Blocking Issues

**NONE.** Build passes, all 11 nets pass at expected counts, headless passes, no correctness lints, boundaries respected, the load-bearing music-theory fixes (diatonic 7th/9th, secondary-dominant `next` tonicization) are mathematically correct.

## 9. Non-Blocking Issues

1. **Modal interchange is a symbol-only swap, not a real borrowed minor iv (musical-correctness gap vs spec §2).** `"iv"` resolves through `roman_degree` to the diatonic degree-3 triad, which in a major mode is the *major* IV pitch set — the chord is renamed but its notes are unchanged, so the promised minor-third "shadow" colour is never sounded. The trigger and the name-level diversity axis are real; the harmony is not actually borrowed. Recommend a follow-up that lowers the third of a borrowed `iv` by a semitone (or routes through a borrowed-chords map) so the interchange is audible. (The implementer's own test documents this gap honestly.)
2. **`feature_normalization` is a required (non-`serde(default)`) field.** A hypothetical hand-edited *old* mappings file lacking the block would fail to deserialize. Not a regression for this repo (one canonical mappings.json, updated in this change; consistent with the existing non-default `dominant_substitution_trigger`), but a `#[serde(default)]` with sane fallback divisors would harden against external/older mapping files.
3. **engine_equivalence has no non-cadence articulation golden**, so it structurally cannot catch a regression in the changed `base_frac` curve. Coverage for that path now lives only in `diversity_s13.rs`. Consider adding one hand-derived non-cadence hold-fraction golden to the equivalence net so the regression anchor covers the path that S13 made dynamic. (Non-blocking; the path IS covered, just not in the equivalence anchor.)
4. **Style clippy warnings** on changed files (`useless_vec`, `doc_lazy_continuation`, `unnecessary_cast`) — cosmetic; clean up opportunistically.

## 10. Overall Verdict

# PASS WITH ISSUES

The S13 diversity fix is correct, well-tested, and respects every module boundary. The two load-bearing music-theory changes — diatonic 7th/9th harmonic complexity and the secondary-dominant `next` tonicization — are mathematically sound (re-derived independently). The continuous articulation curve genuinely replaces the 3-band step function and the headline acceptance gate (≥3 dimensions differ) is robustly met. M's `engine.rs` call-site deviation is a sound, minimal, boundary-respecting pass-through and is APPROVED; the equivalence-net non-drift is a legitimate spec-anticipated non-event. The only musical-correctness shortfall is that **modal interchange relabels rather than actually borrows the minor iv** — a NON-BLOCKING follow-up that does not affect the diversity gate (which does not depend on it). Cleared for integration.

# Quality Gate Review — S30 Slice 1 (Species-Counterpoint Voice + Pure-Data Catalogue Deepening)

Date: 2026-06-17
Reviewer: Quality Gate (independent verification — claims re-derived, not taken on the build lanes' word)
Scope under review: working-tree, uncommitted. `src/chord_engine.rs`, `assets/mappings.json`, `src/composition.rs` (modified); `tests/counterpoint_s30.rs`, `docs/design-s30-pattern-library-slice1.md`, `docs/research-s30-pattern-library.md`, `assets/images/magicstudio-art.jpg` (untracked).
Contract: `docs/design-s30-pattern-library-slice1.md`. Grounding: `docs/research-s30-pattern-library.md`.

**OVERALL VERDICT: PASS WITH ISSUES.**

The headline counterpoint fix is **genuine, not cosmetic**: the species two-point gates now bind the realized prev→now transition via a deterministic replay sharing one code path with live emission. The byte-freeze holds exactly (engine.rs sha unmoved, 9/9 equivalence byte-green). The deferred band-reachability residuals (GAP-2/3/4) are honestly characterized and pinned fail-loud on both regression and closure. The single non-blocking issues are style-grade (a recognition-only dead-code function and a minor docstring overstatement). No blocking issue.

---

## Compilation

- `cargo build` — **PASS.** Builds clean. The only warnings are pre-existing, in unrelated modem binaries (`modem_encode.rs`, `unpack_tiled_payload.rs`) — not in any file under review.
- `cargo build --lib --no-default-features` — **PASS.** The library compiles under the no-default-features feature set (4 pre-existing lib warnings, none from this slice).

## Lint

- `cargo clippy -- -W clippy::all` — **PASS (no blocking warnings).** Zero `error:`-level (deny/correctness) diagnostics across the whole workspace. All output is `warning:`-grade. The slice-relevant warnings are:
  - `function is_legal_cambiata is never used` (chord_engine.rs:3441) — the Cambiata figure is recognition-only in Slice 1 (emission is a documented Slice-4 widening). The function IS exercised by the `s30_cambiata_recognizer` unit test, but clippy's dead-code pass ignores test-only callers on a non-test build. The `Cambiata` enum variant carries `#[allow(dead_code)]` with an honest comment; the function itself does not, hence the warning. Non-blocking (documented, intentional).
  - `too many arguments (8/7)` is in `modem.rs`, NOT this slice. The slice's own 10-arg `pick_counter_figure` carries an explicit `#[allow(clippy::too_many_arguments)]`.
- `cargo fmt -- --check` — **non-blocking note.** The two reviewed Rust files (`chord_engine.rs`, `composition.rs`) are formatting-clean (`rustfmt --check` on those two files exits 0 with no diff). The project-wide `cargo fmt -- --check` reports diffs, but all of them are in pre-existing files OUTSIDE this slice (`make_tiled_payload.rs` and siblings) — a pre-existing condition, not introduced here. Per discipline, `cargo fmt` was NOT run (read-only check only).

## Test Results

Full suite green. Per-binary counts (default features):

| Binary | Tests | Result |
|---|---|---|
| lib (`src/lib.rs`) | 179 | PASS |
| `tests/counterpoint_s30.rs` (NEW) | 13 | PASS |
| `tests/engine_equivalence.rs` (byte-freeze) | 9 | PASS (byte-green) |
| `tests/figuration_s20.rs` | 8 | PASS |
| `tests/saliency_s18.rs` | 12 | PASS |
| `tests/affect_s22.rs` | 8 | PASS |
| `tests/composition_s15.rs` | 5 | PASS |
| all other integration nets | (modem_realair 10, modem_roundtrip 17, keyplan ×5, tui_render 13, cli_parse 24, diversity_s13 10, engine_seam 10, etc.) | PASS |

Library under `--no-default-features`: **144 tests PASS** (integration tests cannot RUN under `--no-default-features` because the bin pulls feature-gated deps — a documented pre-existing limitation, not a regression; the lib compiles and its unit suite passes). Zero failures anywhere.

## Module Boundary Audit

- **No image / MIDI / file-IO logic in `chord_engine.rs` additions.** A targeted grep of the chord_engine diff for image/MIDI/render/file-IO tokens (`image::`, `DynamicImage`, `pixel`, `note_on/off`, `File::`, `fs::`, `stdout`, etc.) returns empty. The new code is pure music-theory craft over MIDI integers.
- **`mappings.json` is data-only and backward-compatible.** Every pre-existing row (`I-vi-IV-V`, `ii-V-I`, `i-bVII-bVI-V`, `alberti`, `block`, …) is preserved verbatim; new rows are appended. No schema field was added — all four new figuration rows use the existing `{at, tone, hold_frac}` schema (the design's field-clean Slice-1 claim is upheld). The full suite parses the JSON, and `s30_figuration_backward_compat_old_rows_unchanged` asserts the old rows still load.
- **Lanes are file-disjoint and ownership was respected.** `git diff --name-only` shows exactly three modified files plus the untracked test/docs/asset. Music-theory touched ONLY `chord_engine.rs`; the implementer's `composition.rs` change is **100% test-additions inside `mod tests`** (verified — no production logic added) plus `mappings.json` data; the test lane wrote ONLY `tests/counterpoint_s30.rs`. No agent modified a file it did not own.
- **`realize_step` public 7-param signature UNCHANGED** (`step, inst_idx, num_instruments, features, ms_per_step, ctx`). All new items in `chord_engine.rs` are private functions, private types, and private constants reached only from the existing `CounterMelody` realize arm.

## Musical Logic Review

**Cross-step memory is REAL (the load-bearing finding).** The as-built defect was that the species two-point gates checked a *synthetic* prev (`seed_prev_counter`, re-derived off the prior chord) rather than the *realized* prior counter pitch. The fix:

- The realize arm now computes `prev_counter = realized_prev_counter(ctx, features, si)` for `si > 0` (and the §3.1 seed only at a section opening, preserving the opening byte-for-byte), then `realized_counter_pitch_with_prev(ctx, step, features, si, prev_counter)`.
- `realized_prev_counter` recovers a prior step's *actual sounding* pitch by deterministic replay: it recurses strictly downward toward the `si == 0` base case, feeding each step's realized pitch as the next step's `prev_counter`.
- The live emission and the replay both route through the **same** `realized_counter_pitch_with_prev` → `pick_counter_figure`, recomputing `held_run_index`, `held_target`, `figures_enabled`, and `next_chord` identically. They therefore **cannot diverge** — the value the arm emits for step `si` is exactly the value any later step's replay consumes as its `si-1`.
- I independently re-derived the witnesses from the **public** `realize_step` surface (a throwaway QG test, since removed): `IV→iii→I` realizes melody 71→67 against counter 59→60 (contrary, no parallel perfect — matches the pinned fixture); `I→V→IV` realizes the counter line `[64, 62, 65]` with the 62→65 move a consonant m3, not the old tritone. The gates genuinely bind the sounding line.
- The replay's ctx re-pointing is correct: `step_ctx.step_in_section = si` is used for the index-sensitive melody seam (`melody_pitch_for` / `melody_pitch_for_step`), while `held_run_position` and `next_chord` index explicitly off `si`, so passing `ctx.section` is right.

**Parallel-perfects checked at T AND T+1 on the realized line.** `has_parallel_perfects` and `approach_perfect_is_legal` are now reached with the realized prior pitch. PT-1's strict universal form holds over the full ordered diatonic-triple battery (GAP-1 fully closed); the property net checks every `(si, si+1)` pair, exercising the gate at both T and T+1.

**Sustain consonance-gate is a no-op on consonant triads.** `consonance_gate_sustain` returns `raw` unchanged when the plain pick is already consonant against the sounding CF (the overwhelmingly common case — every consonant triad), and only re-selects a consonant chord tone when the raw pick is a structural dissonance (the diminished-vii tritone case). The byte-freeze and PT-0 are preserved on the consonant path; the gate adds a consonance floor only where the as-built scorer would have left an unprepared structural dissonance. `s30_sustain_reduces_to_gated_sustain_when_figures_disabled` asserts both the gated reduction and the byte-identity on the consonant-already cases.

**Counterpoint correctness — spot-checks (all re-derived, all correct):**
- `harmonic_class` table: ic 0/7 → perfect, 3/4/8/9 → imperfect, 1/2/6/10/11 → dissonant, and the contested ic 5 (perfect fourth) → dissonant under `FOURTH_IS_DISSONANT = true`. The P4-as-dissonant ruling is correctly **scoped only to the two-voice counter scorer** (`harmonic_class`/`is_consonant`); the chordal `voice_lead_one` path does not call it and keeps its 4th-as-consonant behavior — the research's safe split is honored.
- `rel_motion`: oblique on any hold, contrary on opposed directions, parallel iff same direction AND interval-class preserved, else similar. `rel_motion_score` grades contrary < oblique < similar < parallel (strictly, verified by `s30_rel_motion_classification_and_gradient`).
- `melodic_leap_is_legal`: steps (≤2) always legal; tritone (ic 6) and sevenths (ic 10/11) rejected as melodic leaps; the octave (ic 0 across 12 semitones) correctly stays a legal consonant leap.
- Figure predicates resolve correctly: passing = step-in/step-out/same-direction with the candidate dissonant; neighbor = step-away/step-back to start, opposite directions; suspension = prep-consonant → held-same-pitch → dissonant-on-strong → resolves down −1/−2 to a consonance; cambiata = the canonical 5-note template (step-down / third-down / step-up / step-up, changing-note dissonant, frame consonant). All four unit tests assert real intervals.
- Dissonance is gated as an ornament: `best_dissonant_figure` only emits a dissonance that passes a figure predicate AND whose resolution lands a chord tone of the resolving harmony (R-B), and a dissonance is chosen over the sustain only when its scored figure bonus wins — the consonant frame stays the default.

**`mappings.json` rows musically sensible:** the seven new progression skeletons are diatonic (or borrowed numerals the existing roman parser already accepts), realized non-empty by `s30_new_progression_rows_realize`. The design's verbatim Andalusian duplicate (`i-bVII-bVI-V`, already in `cool`) was correctly NOT re-added — only `i-VII-VI-V` (lament approximation) and `iv-V` (Phrygian half-cadence tag) were appended to `cool`. The four figuration onset patterns are well-formed (2–4 strictly-ascending onsets in [0,1), valid tone/hold_frac), each referenced by a new texture profile gated by a `ge`-only SelectTable rule that is sentinel-safe (no new rule fires on `neutral()`).

## Test Quality

- The property net checks **specific** intervals and properties — no `assert!(true)`, no vacuous tests. Every assertion names exact pitches, interval classes, or motion types, with descriptive failure messages.
- The four inverted `test_*` properties assert real **positive** properties with adversarial witnesses re-derived at T and T+1, plus a broad ordered-triple battery: GAP-1 (no parallel perfect, fully universal), GAP-2 (consonant structural sustain), GAP-3 (perfect-consonant cadence close by no-leap), GAP-4 (no dissonant melodic leap). Each pins the witness with exact expected pitches and re-derives the motion classification independently.
- The residual pins are genuine fail-loud guards: each uses `assert_eq!(residual_set, expected)` so the test FAILS if the residual set **grows** (a new regression) OR **shrinks** (the lane closed it and the pin must be tightened), with a stderr advisory explaining the direction. Confirmed silent on this tree — the pinned sets ({IV,V}, {V,vi}, {ii→IV→iii, vi→IV→iii}) match current realized behavior exactly.
- Determinism (PT-9) is asserted both at the public surface (`test_determinism_of_realized_counter`) and the private driver (`s30_driver_is_deterministic`); the realization is RNG-free in the figure/voice selection.
- The unit tests added in `chord_engine.rs` `mod tests` exercise the private helpers directly (classifier table incl. contested fourth, relative-motion gradient, strict approach-to-perfect, melodic-leap legality, all four figure predicates, opening filter, and the byte-preservation reduction). These complement, do not duplicate, the black-box net.

## Integration

The slice integrates cleanly: the new counterpoint craft is reached only from the existing `CounterMelody` realize arm, which is activated by the existing `pad_bed_counter` texture profile (an implementer-domain SelectTable decision). The seam between lanes is the `OrchestrationProfile.layers` containing `CounterMelody` — set by data, consumed by craft — with no new shared field crossing it. All adjacent nets (saliency, figuration, affect, keyplan, texture) remain green.

## Byte-Freeze Verification (independent re-computation)

- **`sha256sum src/engine.rs` = `e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261`** — recomputed by the reviewer, **EXACTLY equals** the frozen anchor. `git diff --stat src/engine.rs` is empty (engine.rs unmodified). Re-checked after all review activity: still the frozen value.
- **`engine_equivalence` 9/9 byte-green** (re-run independently). The goldens **240 / 114 / 84 / 36 / 79** are all present in `tests/engine_equivalence.rs` and unmoved; the identity / `single_section_default` path inserts no `CounterMelody` events (the arm is unreachable with empty `layers`), confirmed by `test_counter_off_is_byte_identical_baseline` in the new net.
- **`realize_step` public 7-param signature unchanged** (verified by reading the definition at chord_engine.rs:1055).
- Working tree pristine after review (scratch verification test removed; only the three reviewed files + untracked docs/test/asset remain).

## Blocking Issues

**NONE.** The headline fix is verified genuine and load-bearing. The byte-freeze is intact.

## Non-Blocking Issues

1. **`is_legal_cambiata` is dead code on non-test builds (clippy warning, chord_engine.rs:3441).** Cambiata is recognition-only in Slice 1 by design (emission deferred to Slice 4); the function is called only by its unit test. Honest and documented. Optional cleanup: add `#[allow(dead_code)]` to the function (mirroring the `Cambiata` variant) to silence the warning, or leave it as a standing reminder that Slice-4 emission is pending. No action required for this slice.
2. **Minor docstring overstatement.** `realized_prev_counter`'s docstring references a "`seen`/depth guard" that "caps the recursion at the section length"; the implementation actually terminates by strict downward `checked_sub` recursion to the `si == 0` base case (with a defensive fallback on a malformed plan), not an explicit `seen` set. The behavior is correct and terminating; only the prose slightly over-describes the mechanism. Cosmetic.
3. **Project-wide `cargo fmt -- --check` reports diffs** — but exclusively in pre-existing files outside this slice. The reviewed files are fmt-clean. Pre-existing condition; not introduced here.

## Residual Honesty Assessment (per the KNOWN/ACCEPTED RESIDUALS brief)

The slice deliberately stops before the **band-reachability** residual, and characterizes it honestly:
- **GAP-1 fully closed** (no parallel perfect, universal over the battery).
- **GAP-2** (2/6 terminal-diminished openers, {IV, V}, still land a dissonant structural sustain), **GAP-3** (2/6 cadences, {V, vi}, resolve by leap), **GAP-4** (2/57 leaps, {ii→IV→iii, vi→IV→iii}, still tritone) — all attributable to one root: no consonant / no-leap target is band-reachable from the realized penult within the counter's register band.
- Each residual is pinned with a fail-loud `assert_eq!` that catches BOTH regression and closure, and the deferral to S31 (carrying a musical-taste dimension — octave-displacement vs. accept-rare-ornament — that wants the owner's ear) is appropriate. The residuals are NOT failures; they are honestly-scoped, owner-deferred, loudly-pinned known limitations. This is a legitimate PASS WITH ISSUES.

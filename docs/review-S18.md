# Quality-Gate Review — S18 Slice 2 (Saliency Region Reader + Real Counter-Melody)

**Reviewer role:** Quality Gate (independent verification — last stage before commit).
**Date:** 2026-06-15.
**Working tree:** `/home/qweary/working/audiohax-engagement/AudioHax`, HEAD `0f98d0f` + uncommitted Slice-2 work.
**Method:** Adversarial — re-derived formulas by hand, read every code path, re-ran the full default-feature suite, sha256'd the off-limits files against HEAD, and probed the held-run line empirically with a throwaway example (since removed). Nothing below is taken on the implementers' word.

---

## TOP-LINE VERDICT: **PASS**

All seven verdict criteria verified true; both documented deviations adjudicated **SOUND**. The byte-freeze is genuine (re-derived, not merely green), the saliency reader genuinely discriminates, and the counter-line is a real moving inner voice whose held-run pitch advances (`E→G→E→G`, off-root, off-beat) — the operator's load-bearing "empty periods" requirement is met. No blockers. Two minor non-blocking nits recorded; neither gates the commit.

---

## Per-criterion findings

### 1. BYTE-FREEZE REAL — PASS
- `tests/engine_equivalence.rs` ran green (9/9) in the full suite. The test file is **byte-untouched** (`git diff HEAD -- tests/engine_equivalence.rs` empty). Goldens confirmed in-file unmoved: `G_BASS_NOTE=36`, `G_MELODY_NOTE=79`, cadence vel `114`/`84`, hold `240`, `MS_PER_STEP=200`.
- **Unreachability re-derived, not trusted.** `assign_role` returns the legacy `instrument_role` stratification whenever `prof.is_identity()`. I read `instrument_role`: it only ever returns `Bass` / `Melody` / `HarmonicFill` (grep for `CounterMelody` inside it = 0). The equivalence net carries `OrchestrationProfile::identity()`, so the CounterMelody realize arm and the new saliency knobs are **structurally unreachable** on the freeze path — independent of the green test.
- **`realize_step` public signature UNCHANGED:** byte-identical to HEAD (the 6-arg form already carried `ctx` at S17). The only thread is `ctx` added to the *private* `realize_rhythm` call — confirmed in the diff. No public-seam change.

### 2. SALIENCY READER REAL — PASS
- **Zero new dependency, no ML, deterministic.** `analyze_regions_pure` reuses `to_gray` / `hsv_means` / `edge_density_pure` over `crop_imm(..).to_image()` sub-rectangles — the existing owned-buffer path. No RNG/clock in the diff (grep clean). Determinism is pinned by `test_saliency_reading_is_deterministic` (re-reads every field) and held across the suite.
- **Geometry sanity-checked by hand.** 3×3 thirds partition with the last row/col absorbing the remainder; areas sum to ~1.0 on both 30×30 (divisible) and 31×31 (non-divisible) — proven by `analyze_regions_pure_cell_count_and_geometry`, and the centroid formula `((x0 + cw/2)/w, (y0 + ch/2)/h)` is the correct cell-rect centroid.
- **One field formula re-derived independently:** `fg_bg_contrast = (|Δvalue|/100 + |Δsat|/100 + |Δedge|).clamp(0,1)` of subject-cell vs the everything-but-subject ring. A flat field → all three deltas 0 → `fg_bg_contrast 0`; this matches `test_saliency_spread_discriminates` (`flat < 0.05`). The cross-image discrimination (flat vs subject-on-quiet-ground vs busy-everywhere) is proven distinct, and `foreground_energy < subject_energy` for a subject on a quiet ground (action concentrates in the subject) holds.

### 3. COUNTER-MELODY REAL — PASS
- **Genuine moving line, chord-tones-in-band, ≤1 event/step.** `pick_counter_pitch` seats only `counter_candidate_pitches` (chord pitch classes seated in `[55,67)` via `upper_voice_candidates`), so every pick is a chord tone in band — pinned by `test_counter_is_chord_tone_in_counter_register` and the integration `test_counter_is_chord_tone_in_band`. The arm's three branches each build a single-element `vec!` (no figuration); `test_counter_at_most_one_event_*` (both nets) confirms the ≤1 ceiling across calm/busy and held/changing.
- **Contrary/oblique + no parallel perfects.** Scoring subtracts `CONTRARY_BONUS` for contrary/oblique motion vs the recomputed melody and never for similar; a **hard `continue`** rejects any candidate where `has_parallel_perfects(&[m_prev,prev_counter], &[m_now,cand])` fires — a NEW call site on the 2-voice pair, the fn itself unedited (confirmed). `test_counter_contrary_or_oblique_vs_melody` proves the realized counter never moves strictly up with a rising melody.
- **Held-period line ADVANCES — empirically confirmed.** I drove a 4-step C-major held run through `realize_step` directly: output `[(64,250),(55,250),(64,250),(55,250)]` — i.e. **E→G→E→G**, exactly the `[64,55,64]`-style advance the spec claims. Off-root (never C/60), off-beat (offset `250 = step_ms/4` on every step), ≥2 distinct pitches, adjacent steps always differ. The rotation is **bounded** (`HELD_RUN_SEED_CAP=4`, `(held_run_index-1) % ring.len()`) and **deterministic** (`advancing_seed_counter` is a pure function of plan position + chord; no `thread_rng`). This is the operator's load-bearing requirement and it is genuinely met — a woven inner line, not a re-struck stab.

### 4. SCOPE HELD — PASS
No Alberti/comping/figuration crept in: the only `alberti`/`comping`/`arpeggio` tokens in the diff are in comments (describing the deferred Slice-3 ceiling and when the melody subdivides). The counter emits at most one note per step (enforced + tested). No `num_instruments` widening — `pad_bed_counter` swaps one inner instrument to a CounterMelody at the existing width, exactly as `pad_bed` swaps one to a Pad. The slice is precisely saliency-reader + one counter line + the selection wiring.

### 5. OFF-LIMITS UNTOUCHED — PASS
`git diff --stat` shows only the seven expected paths (`src/pure_analysis.rs`, `src/composition.rs`, `assets/mappings.json`, `src/chord_engine.rs`, `tests/texture_s17.rs` modified; `tests/saliency_s18.rs`, `docs/spec-s18-slice2-build.md` new). I sha256'd every named off-limits file against `git show HEAD:` — all **MATCH**: `engine.rs`, `main.rs`, `modem.rs`, `synth_sink.rs`, `midi_output.rs`, `cli.rs`, `tui.rs`, `bin/modem_encode.rs`, `bin/modem_decode.rs`, `bin/audiohax-tui.rs`. Zero-byte-changed. (See Nit 1 re: an untracked asset.)

### 6. NO CODENAME LEAKS — PASS
A word-boundary grep over the upstream tooling's reserved-codename set, run across all changed/new source, the spec doc, and the test net, returned **no matches** (exit 1). This review uses role names only.

### 7. NETS GREEN + WARNINGS — PASS
Full headless default-feature suite green: 151 lib unit tests + `saliency_s18` 12 + `texture_s17` 7 + `engine_equivalence` 9 + `engine_seam` 10 + `composition_s15` 5 + `diversity_s13` 10 + `cli_parse` 24 + `modem_realair` 10 + `modem_roundtrip` 17 + `phase2_pure_pipeline` 7 + others — **0 failed** everywhere.
- **The 2 lib warnings are PRE-EXISTING, not introduced by this slice.** Both locate to `src/modem.rs` (lines 426, 983) — an off-limits file that is sha256-identical to HEAD. The remaining warnings are in `bin/modem_encode.rs` / `bin/unpack_tiled_payload.rs` (also off-limits, also pre-existing). This lane introduces **no new warnings**.

---

## Deviation adjudications

### Deviation A — locked center-surround weights (0.5/0.35/0.15) → corner blob can only TIE the center → **SOUND**
A corner cell's maximum score is `W_CONTRAST·1 + W_SAT·1 = 0.50`, which can only tie (never exceed) the center cell's `W_CENTER·1 = 0.50`; ties resolve to the most-central cell, so the *argmax* always claims the center on a flat-vs-corner contest. The implementer correctly changed the in-module test to honor the **locked weights** over the spec's looser "contrast beats center-bias" narrative — the right call (locked weights are the contract; the narrative was aspirational).

The deviation is sound because the reader **still discriminates real off-center salient subjects downstream**, which is what matters musically. I verified this is not a hand-wave: `fg_bg_contrast` is computed from the subject cell vs the *everything-but-subject* ring (not the geometric border), so a strong corner subject still produces real contrast even when the center prior wins the argmax; and `mass_centroid` is a luminance-weighted centroid over **all nine cells**, so off-center mass genuinely moves it. `test_offcenter_salient_subject_moves_the_reading` proves a corner blob moves `mass_centroid` by >0.02 and lifts `vertical_emphasis` above the centered case, with `fg_bg_contrast > 0`. The blind spot is narrow and honest: a *flat* corner blob ties (correctly — a flat patch is not salient); a *salient* corner subject is still read. Not a real defect.

### Deviation B — counter INCLUDES the root pc as a de-prioritized candidate (spec said "skip root pc") → **SOUND**
The justification is real: the narrow `[55,67)` counter band otherwise starves the moving line (a bare triad can fail to offer ≥2 non-root tones in 12 semitones). The implementer keeps the root *legal but de-prioritized* via a small `ROOT_PC_BIAS=2` tie-break (two orders of magnitude below `CONTRARY_BONUS=24`, so it only breaks ties), and `advancing_seed_counter` rotates through the **non-root** ring first, falling back to the full set only for a degenerate chord. The reasoning that a root pc seated at ≈55–66 is **not** a bass double (the actual bass sounds at C2≈36) is musically correct. With the held-run advance landed, the line both moves AND stays off-root — empirically `E→G→E→G`, never the root C. `test_held_period_fills_off_beat_on_a_non_root_tone` asserts `note%12 != root%12`. The deviation is now fully justified.

---

## Non-blocking nits (do not gate commit)

1. **Untracked asset `assets/images/magicstudio-art.jpg`** is present in the working tree and is NOT listed in the spec's deliverable set. It is a JPEG (not source/test/asset-config), so it does not affect any verdict criterion — but it should be either intentionally added (if a fixture/sample) or left out of the Slice-2 commit so the commit stays exactly the seven spec'd paths. Flag for the committer's `git add` discipline.
2. **`tests/counter_s18.rs` was not created as a separate file.** The spec §3.6 named a standalone `tests/counter_s18.rs`; the Test Engineer instead placed the §3.6 counter properties as in-module `chord_engine` tests **plus** the cross-lane `tests/saliency_s18.rs` Part 2. Coverage is equivalent or better (the in-module tests can see private helpers; the integration net pins the public surface), so this is a placement choice, not a gap — noted only so the spec/file map and reality are reconciled.

---

## What I ran (audit trail)
- `git diff --stat`, `git status --porcelain` — file set confirmation.
- sha256 of 10 off-limits files vs `git show HEAD:` — all MATCH.
- Word-boundary codename grep over 7 changed/new files — clean.
- `cargo test` (default features) — full suite green; warning locations traced to off-limits `modem.rs`/bins (pre-existing).
- Read of `assign_role` / `instrument_role` — proved CounterMelody unreachable under identity.
- `realize_step` signature diff vs HEAD — unchanged.
- Hand re-derivation of the 3×3 geometry and the `fg_bg_contrast` formula.
- Throwaway `examples/` probe driving a 4-step held C-major run through `realize_step` → observed `[64,55,64,55]` advance; example removed after capture.

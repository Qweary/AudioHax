# S20 — Slice 3a Build Review: ACCOMPANIMENT FIGURATION (Quality Gate, independent adversarial verification)

**Reviewer role:** Quality Gate — INDEPENDENT verification. Load-bearing claims re-derived from the working tree and the running test binaries; build agents' self-reports NOT trusted.
**Date:** 2026-06-15.
**Repo / base:** `/home/qweary/working/audiohax-engagement/AudioHax`, git HEAD `49e0821`.
**Contract verified against:** `docs/spec-s20-slice3a-build.md` + the 8 locked S19 steers.
**Method:** `git diff HEAD` per path, sha256 freeze check on the locked-off set, by-hand re-derivation of the figured burst, full `cargo test` net run, word-boundary codename grep, rustfmt check.

---

## VERDICT: **PASS**

The build implements the contract faithfully. The byte-freeze is real (engine.rs and all 14 locked-off files hash-identical to HEAD), the figured bed is a genuine Alberti animation that I re-derived by hand and matched event-for-event, the gate ladder is correct and first-match-wins, back-compat is structurally guaranteed by `#[serde(default)]`/`#[serde(skip)]`, and the whole headless suite is green (284 tests, 0 failed). No blockers. Two non-blocking nits (an untracked stray image; a pre-existing warning in a locked-off file) are carry-forward only.

---

## Per-item verification

### 1. Byte-freeze is REAL — PASS

- `git diff HEAD --stat` shows ONLY: `assets/mappings.json`, `src/chord_engine.rs`, `src/composition.rs`, `src/mapping_loader.rs`, `tests/saliency_s18.rs`, `tests/texture_s17.rs`. The two new files (`tests/figuration_s20.rs`, `docs/spec-s20-slice3a-build.md`) are untracked, hence absent from the `--stat` of tracked changes — correct, not a discrepancy.
- `git diff HEAD -- src/engine.rs` is **EMPTY** — the critical engine freeze holds.
- **sha256 of all 14 locked-off files == `git show HEAD:<path>`** — every one printed `OK` (modem.rs, the 6 modem/payload bins, audiohax-tui.rs, synth_sink.rs, midi_output.rs, cli.rs, tui.rs, main.rs, engine.rs). Zero drift.
- `realize_step` PUBLIC signature byte-identical HEAD vs now (6 params, `ctx: &StepContext`, `-> Vec<NoteEvent>`). The figuration threading is additive/private only (reads `ctx.section.orchestration.figuration_resolved` — no new param).

### 2. engine_equivalence byte-green, goldens UNMOVED — PASS

- `cargo test --test engine_equivalence` → **9 passed; 0 failed**.
- `git diff HEAD -- tests/engine_equivalence.rs` EMPTY.
- Goldens present and asserted in-file: `MS_PER_STEP=200` (:124), `G_BASS_NOTE=36` (:130), `G_MELODY_NOTE=79` (:135), cadence vel `114` (:274) / `84` (:288), cadence hold `240` (:278). None moved.
- **WHY the figured arm is unreachable here (both reasons verified in code, not taken on faith):**
  1. `instrument_role` (`chord_engine.rs:874`) returns ONLY `Bass` (inst 0), `Melody` (last inst / single), or `HarmonicFill` (interior) — **never `Pad`**. Under `single_section_default`'s identity profile, `assign_role` delegates here, so no instrument is ever a Pad → the figured sub-branch is structurally unreachable.
  2. The cadence early-return (`chord_engine.rs:1370–1372`, `if is_cadence { return vec![sustained(0, step_ms, LEGATO_FRAC)]; }`) fires BEFORE the `match role` block, so a cadence step is never figured even on a figured profile.
  Independently, identity has `pad_voices == 0` and `figuration_resolved == None`, so even the block-Pad path is defensive-only there.

### 3. The figured bed is REAL, not gamed — PASS (hand-derived, matches the build)

I read `figured_bed` in `chord_engine.rs` and re-derived the contract output by hand.

**Burst A — Alberti, seated=[64,55,70] (n=3), step_ms=200, PAD_OVERLAP_FRAC=1.10, cap=round(200×1.10)=220, hold_frac=1.0, onsets {0:t0, ¼:t2, ½:t1, ¾:t2}:**

| onset | at | offset=round(at·200) | idx=tone%3 | note=seated[idx] | next_off | gap | raw_hold | cap−off | hold |
|---|---|---|---|---|---|---|---|---|---|
| 0 | 0.00 | 0   | 0%3=0 | 64 | 50  | 50 | 50 | 220 | 50 |
| 1 | 0.25 | 50  | 2%3=2 | 70 | 100 | 50 | 50 | 170 | 50 |
| 2 | 0.50 | 100 | 1%3=1 | 55 | 150 | 50 | 50 | 120 | 50 |
| 3 | 0.75 | 150 | 2%3=2 | 70 | 200 | 50 | 50 | 70  | 50 |

→ **64@0, 70@50, 55@100, 70@150, all holds 50** — EXACTLY the build's claimed output. (Verified the loop: `offset_ms = round(at.clamp·step_ms)`, `idx = tone % n`, last-onset `next_offset_ms = step_ms`, `gap = next−offset` saturating, `hold = min(raw, cap−offset).max(1)`.)

**Burst B — TRIAD degrade, seated=[64,55] (n=2):**

| onset | offset | idx=tone%2 | note | hold |
|---|---|---|---|---|
| 0 | 0   | 0%2=0 | 64 | 50 |
| 1 | 50  | 2%2=0 | 64 | 50 |
| 2 | 100 | 1%2=1 | 55 | 50 |
| 3 | 150 | 2%2=0 | 64 | 50 |

→ **(64@0, 64@50, 55@100, 64@150)** — matches the claimed degrade. A real Alberti animation that gracefully reduces to the 2 available inner tones, in-band, NO out-of-bounds (the `% n` guarantees it).

- Onsets bounded **2..=4**: the helper slices `&spec.onsets[..len.min(4)]` (defensive truncate to 4); the catalogue row supplies 4. Verified.
- Line stays **chord-tones-only in the pad band**: the helper indexes ONLY into `seated` (the already-seated root-less inner tones in `[55,67)`), never re-derives a pitch — chord-tone-and-in-band by construction.

### 4. Block path BYTE-UNTOUCHED when figuration None — PASS

The Pad arm's `_ =>` (None / empty-onsets) arm emits, verbatim:
```rust
seated.into_iter().map(|n| NoteEvent {
    note: n, velocity,
    hold_ms: ((step_ms as f32) * PAD_OVERLAP_FRAC).round().max(1.0) as u64,
    offset_ms: 0,
}).collect()
```
This is character-identical to the pre-S20 block emission (the diff shows the original lines moved verbatim into the `_ =>` arm — same `note`, `velocity`, `hold_ms` formula, `offset_ms: 0`, same comment). The seating block above it (`:1444–1463`) is unchanged. The `block_bed_unchanged_when_figuration_none` and `unresolved_figuration_id_falls_to_block` tests both assert offset 0, count==pad_voices(3), hold==round(step_ms×1.10)=220 — and pass. Back-compat path is verbatim.

### 5. The gate ladder is correct — PASS

`assets/mappings.json` `texture` SelectTable (first-match-wins via `SelectTable::select`, `composition.rs`):
1. **FIRST:** `subject_energy ge 0.45 AND fg_bg_contrast ge 0.25` → `pad_figured`.
2. **THEN (unchanged S18):** `foreground_energy ge 0.35 AND fg_bg_contrast ge 0.20` → `pad_bed_counter` — predicate values byte-identical to S18; only the figured rule was prepended.
3. **default:** `pad_bed`.

- `pad_figured` layers = `["Bass","Pad","HarmonicFill","Melody"]` — **NO CounterMelody** (steer 3), `figuration:"alberti"`, `pad_voices:3`, `density:0.62`.
- alberti catalogue row onsets = `{0:t0, 0.25:t2, 0.5:t1, 0.75:t2}`, `voices:3` — matches steer 1.
- The S18 ladder still fires: `texture_selects_pad_figured_on_salient_subject` proves the S18 case (`foreground_energy:0.4, fg_bg_contrast:0.25, subject_energy:0.0`) still picks `pad_bed_counter` — the prepended rule does NOT shadow it because the S18 fixture's `subject_energy` defaults to 0.0 (`ImageUnderstanding::neutral()`), failing the figured rule's `subject_energy ge 0.45` gate. Calm image → `pad_bed`. Test loads the REAL `assets/mappings.json` (not a stub).

### 6. Back-compat: old mappings.json byte-identical — PASS

- `OrchestrationProfile.figuration: Option<String>` is `#[serde(default)]` (→ None); `figuration_resolved: Option<FigurationSpec>` is `#[serde(skip)]` (never deserialized, always None at load, planner sets it).
- `figuration_catalogue` is `#[serde(default)]` on BOTH mirror structs — `PlanMappings` (`composition.rs:485`) AND `CompositionMappings` (`mapping_loader.rs:131`, the struct that actually deserializes the JSON) — and the `From<CompositionMappings> for PlanMappings` impl maps it (`composition.rs:509`). All three present (the load-bearing A5 detail).
- `FigurationSpec.onsets` `#[serde(default)]`, `.voices` `#[serde(default="one_u8")]`; `FigurationOnset.hold_frac` `#[serde(default="one_f32")]`.
- Reasoning: an S18-era mappings.json (no figuration keys) → every profile's `figuration`==None, both catalogues empty → every handle resolves to None → realizer takes the block bed → byte-identical S18 behavior. Resolution is total (unmatched id → None → block bed, never panics — `unresolved_figuration_id_falls_to_block` is the witness). `is_identity()` untouched; figured profiles have `pad_voices>0` so identity detection is unaffected.

### 7. Full net green — PASS

Ran the whole headless suite myself:
- `cargo test --lib` → **151 passed**.
- Integration nets: cli_parse 24, composition_s15 5, diversity_s13 10, engine_equivalence 9, engine_seam 10, **figuration_s20 8**, modem_realair 10, modem_roundtrip 17, phase2_pure_pipeline 7, qg_probe_band_isolation 1, saliency_s18 12, texture_s17 7, tui_render 13.
- **TOTAL: 284 passed; 0 failed; 0 ignored.**

`figuration_s20`'s 8 tests assert what they claim (read in full):
- `figured_bed_off_beat` pins ≥1 event with `offset_ms > 0` (real off-beat witness).
- `figuration_tones_are_chord_tones_in_band` asserts `inner_pcs.contains(ev.note%12)` AND band `[55,67)` — seated membership, not a stub.
- `texture_selects_pad_figured_on_salient_subject` loads the real `assets/mappings.json` via `load_mappings`, resolves the handle through the catalogue, and pins the alberti `at` grid `[0.0,0.25,0.5,0.75]` — observed, not hardcoded.
- All witnesses drive the PUBLIC `realize_step`. `tone_index_cycles_modulo_seated` proves the triad (≤2 distinct pitches) vs 7th (≥2) modulo behavior with an OOB guard. Bounded-burst, in-step, and the two back-compat block witnesses all pass.

### 8. Scope held + hygiene — PASS

- **No new role enum arm:** `OrchestralRole` and `LayerRole` enums byte-unchanged (diffs empty for enum/new-variant). Figuration is a sub-branch INSIDE the existing `Pad` match arm, exactly as specified.
- **No scheduler/main.rs touch:** main.rs hash-identical to HEAD; no new param on `realize_step`/`realize_rhythm`; resolution rides the already-borrowed `ctx`.
- **No meter machinery:** none added (steer 8 deferred, honored).
- **Codename hygiene:** a word-boundary grep for the internal framework/agent codename set (run separately, not transcribed into this deliverable) across all changed + new files → **no matches** (clean). The spec doc and this review use plain role terms only.
- **rustfmt:** `rustfmt --edition 2021 --check` on all changed src+test files → exit 0, no diff. Clean.

---

## Deviations from spec / steers (each adjudicated)

None of the build's choices contradict the 8 locked steers. The spec itself flagged 5 ambiguity resolutions (A1–A5) and the build follows them:
- **A1 (root-less Alberti realization):** the figure animates the seated root-less inner tones (`{tone0, tone2, tone1, tone2}`), the steer's classical "root/5th/3rd/5th" realized in the as-built root-less bed. **SOUND** — this is the steer realized in the shipped voicing, not a divergence; I re-derived both the triad and 7th cases and they are musically sane (an Alberti broken-chord that degrades on triads).
- **A2 (velocity unchanged, no −3 trim):** `figured_bed` uses `velocity` as-is, identical to the block bed; only rhythm changes. **SOUND** — steer offered the trim as optional; deferring it to a dynamics slice is in-scope.
- **A3 (`#[serde(skip)] figuration_resolved` threading):** the planner resolves once at the texture-resolution point (`composition.rs:768`) and stores the resolved spec on the section's profile clone; the realizer reads it off `ctx`. **SOUND** — no signature change, catalogue confined to planner, mappings.json byte-shape preserved.
- **A4 (no `register_floor` field):** dropped; the bed seats in `FILL_REGISTER_FLOOR` hard-coded. **SOUND** — avoids an unused/untested field.
- **A5 (TWO mirror structs):** both `PlanMappings` and `CompositionMappings` gain `figuration_catalogue` + the `From` map. **SOUND and verified present** — omitting any of the three would have silently dropped the catalogue at load; all three are there.

---

## Non-blocking nits (carry-forward)

1. **Untracked stray image.** `assets/images/magicstudio-art.jpg` is present in the working tree, untracked, and outside this build's declared change set (never committed on any branch). It is unrelated to the figuration build — likely an operator test input. RECOMMEND: gitignore it or remove it before committing the slice, so it does not get swept into the commit. (Not a defect in the build itself.)
2. **Pre-existing warning in a locked-off file.** `cargo build` emits `warning: unused variable: total_shards` at `src/modem.rs:983`. `modem.rs` is in the locked-off set and is hash-identical to HEAD — so this warning PRE-DATES S20 and was not introduced here. No action required for this slice; noted for a future modem hygiene pass.

---

*Independent Quality Gate review. No source, test, or asset file was modified by this document (`docs/` only). Verdict: PASS — Slice 3a is correct, in-scope, byte-frozen where required, and fully green.*

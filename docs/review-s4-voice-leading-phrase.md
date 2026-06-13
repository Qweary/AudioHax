# Quality Gate Review — S4 Voice Leading + Phrase Structure

**Reviewer:** Quality Gate (independent verification)
**Date:** 2026-06-12
**Scope:** WS-1 music expressivity, first expressivity layer. New public API in `src/chord_engine.rs`: `voice_lead_sequence`, `voice_roles`, `plan_phrases`, types `StepPlan`/`VoiceRole`/`PhrasePosition`, consts `MAX_UPPER_VOICE_MOTION`/`PHRASE_LENGTHS`, plus a 5-test property net.
**Build profile:** headless library only (`--lib --no-default-features`); the binary cannot build here (no OpenCV/ALSA).

---

## Compilation Status

`cargo build --lib --no-default-features` — **PASS** (`Finished` in 0.34s).

6 build-level warnings, all `unused`:
- `src/chord_engine.rs:1` unused import `lookup_range_map`
- `src/chord_engine.rs:4` unused import `std::collections::HashMap`
- `src/chord_engine.rs:126` unused variable `next` (inside the **pre-existing** `generate_chords` secondary-dominant branch, not S4 code)
- `src/modem.rs:599`/`658` unused vars (pre-existing, unrelated module)

None are correctness defects. All NON-BLOCKING.

## Lint Status

`cargo clippy --lib --no-default-features -- -W clippy::all` — 35 warnings total, the overwhelming majority in `src/modem.rs` (pre-existing, out of scope). Warnings attributable to `chord_engine.rs`:
- 2 unused imports (lines 1, 4) — dead `use`s, NON-BLOCKING
- 1 unused variable `next` (line 126) — pre-existing `generate_chords` body, NON-BLOCKING
- 2 `useless_vec` (lines 47–48, `vec!["Ionian",...]` could be arrays) — pre-existing `pick_progression` body, NON-BLOCKING

No clippy **correctness** warnings in the new S4 code. The new free helpers and the two new methods produce no clippy findings of their own. All clippy findings NON-BLOCKING.

`rustfmt --edition 2021 --check src/chord_engine.rs` — **clean** (exit 0, no reformat needed).

## Test Results

`cargo test --lib --no-default-features` — **16 passed; 0 failed; 0 ignored** (matches the self-report exactly).

The 5 new property tests all green:
- `test_upper_voices_never_leap_beyond_perfect_fifth`
- `test_no_parallel_perfect_fifths_or_octaves`
- `test_common_tone_retained_in_same_voice`
- `test_cadences_sit_at_phrase_boundaries`
- `test_velocity_varies_within_a_phrase`

## Module Boundary Audit (per-file)

| File | Modified by S4? | Finding |
|---|---|---|
| `src/chord_engine.rs` | YES (the entire S4 delta) | No image/OpenCV, no MIDI-output, **no modem references**, no raw image-feature handling. Receives `&[Chord]` + scalar params only. Domain vocabulary "MIDI note number" appears but only as the chord-note representation, not as output I/O. CLEAN. |
| `src/main.rs` | Pre-existing S2 only | Diff is S2 lib-promotion plumbing; no S4 symbols (`voice_lead_sequence`/`plan_phrases`/`StepPlan`/`voice_roles`) referenced. |
| `src/lib.rs` | Pre-existing S2 only | Diff promotes `chord_engine` + `mapping_loader` to `pub mod`. No S4 additions. |
| `Cargo.toml` | Pre-existing S2 only | `[features]` default = opencv/midir/image made optional; `[dev-dependencies] rand_chacha`. The `rand_chacha` dev-dep is used by the **pre-existing** `tests/modem_roundtrip.rs` (verified), not by S4 — not dead, not S4-introduced. |
| `mapping_loader.rs`, `midi_output.rs`, `image_*.rs`, `modem.rs`, `bin/*`, `assets/mappings.json` | NOT modified | Confirmed via `git status`. |

`git status` shows exactly 4 tracked modifications (Cargo.toml, chord_engine.rs, lib.rs, main.rs) + untracked `Cargo.lock`, `docs/`, `tests/`. No file was modified by an agent that does not own it. The S4 delta is confined to `chord_engine.rs` as required.

> Note on git baseline: HEAD's `chord_engine.rs` is the pre-S2 126-line version; both S2 and S4 are uncommitted working-tree changes, so S2 and S4 cannot be separated by git alone. Separation was done by reading the diff content (lib/Cargo/main carry only S2 plumbing; all voice-leading/phrase code is in chord_engine.rs).

## Musical Logic Review

I verified each load-bearing check **at source** and additionally re-implemented the algorithm faithfully in a throwaway simulator to observe the actual voiced output for all four test progressions (Ionian, ROOT=60). The temp harness was removed after use; no production code or tests were modified.

### LB-1 — Per-voice state genuinely tracked across chords — **CONFIRMED**
`voice_lead_sequence` (src/chord_engine.rs:290) seats the first chord, then for each subsequent chord calls `voice_lead_one(prev, next)` where `prev = out.last()` — i.e. each chord is voiced **relative to the previous EMITTED voicing** (carried-forward state), not independently. `voice_lead_one` reads `prev.notes[vi]` as the source pitch for each upper voice and scores motion against it. This is real voice leading, not per-chord re-spelling.
- Index 0 is genuinely the bass: first chord seats the **root pitch class** at/near the lowest input note (`nearest_pc_to(root_pc, min(notes))`), uppers stacked above; thereafter the bass voice takes `next`'s root via `nearest_pc_to(root_pc, prev_bass)`. Index 0 = bass for the whole sequence.
- `voice_roles` classifies by **index** (0 = Bass, rest = Upper), matching the voice-alignment contract the tests rely on. Correct.

### LB-2 — Parallel detection compares a voice PAIR at T against the same pair at T+1 — **CONFIRMED**
`has_parallel_perfects(a, b)` (src/chord_engine.rs:620) iterates every voice pair `(i,j)`, computes `interval_class` of the pair at T (`a`) and at T+1 (`b`), and flags only when `ic_a == ic_b && (ic_a == 0 || ic_a == 7) && both_move`, where `both_move = a[i]!=b[i] && a[j]!=b[j]`. This is exactly the correct rule: same perfect class (P5=7 / P8-unison=0) sustained while BOTH voices move; oblique motion over a held common tone is allowed. It is NOT a within-single-chord check. The test (`test_no_parallel_perfect_fifths_or_octaves`) independently re-implements the same T→T+1 comparison and goes green. Simulation over `I-ii-iii-IV` (textbook parallel-fifth trap in root position) confirms the re-voiced output contains **no** parallel perfects.

### LB-3 — Cadences land at phrase boundaries, never mid-phrase — **CONFIRMED**
`plan_phrases` (src/chord_engine.rs:390):
- Phrase length chosen as the largest `PHRASE_LENGTHS` value that divides `n` AND yields ≥2 phrases (so a period forms), else largest dividing, else 4. An 8-chord input → two 4-step phrases. Verified by trace.
- A cadence is stamped ONLY when `at_boundary` (`position_in_phrase == this_len - 1`) and `this_len >= 2`. Start/Interior are the only other arms — structurally impossible for an interior/start step to carry a cadence label.
- Non-final boundary → `HalfCadence`, name re-spelled to `"V"`.
- Final boundary → `PerfectAuthenticCadence` with name `"I"` **only if** the already-stamped predecessor step is a dominant (`is_dominant_name`, which correctly excludes "vi"/"VI"); otherwise it honestly downgrades to `HalfCadence`/"V". HalfCadence carries "V"; PAC carries "I" preceded by "V". Verified by trace on `I-ii-IV-V-vi-IV-V-I`: step 3 = HalfCadence "V" at boundary; step 7 = PAC "I" at boundary, preceded by step 6 "V". The partition places boundaries exactly where cadences are stamped.

### Mode/scale constants — **UNCHANGED / CORRECT**
Ionian [0,2,4,5,7,9,11], Dorian [0,2,3,5,7,9,10], Phrygian [0,1,3,5,7,8,10], Lydian [0,2,4,6,7,9,11], Mixolydian [0,2,4,5,7,9,10], Aeolian [0,2,3,5,7,8,10] — all match canonical. The Roman-numeral→degree map is now correct (IV→degree 3 subdominant, iii→degree 2 mediant; the historical shadowing bugs are fixed and pinned by `test_iv_..`/`test_iii_..`).

### Common-tone retention is REAL — **CONFIRMED**
The scoring in `voice_lead_one` subtracts a heavy bonus (−100) when a voice holds its **exact** pitch (`prev.notes[vi] == voicing[slot+1]`, zero motion), which dwarfs any motion total so retention wins whenever available. Simulation of `I→vi` (shared pcs {0,4}): pc 4 is held in the same voice index across the change. The retention is driven by the scoring, not accidental.

### The two self-reported relaxations — **UNREACHABLE for test inputs (acceptable, defensive)**
- Relaxation A (`voice_lead_one`, ~line 679): if an upper voice has no in-cap chord tone, seat the nearest chord tone regardless of the ≤P5 cap.
- Relaxation B (~line 761): if every candidate combination is parallel (or there are no upper voices), fall back to minimal-motion ignoring the parallel rule.

I instrumented a faithful re-implementation and ran all four test progressions: **neither relaxation fires for any test input** (Relaxation A never fires; Relaxation B never fires). This is structurally expected — within a ±7-semitone window of any source pitch there is always at least one octave seating of some triad pitch class, so the candidate set is never empty for diatonic triads; and the diatonic search always finds a parallel-free voicing. The green is therefore **not hollow** — the tested properties hold on real algorithm output, not on a relaxed path. Acceptable.

### S4/S6 boundary (structural velocity is a floor, not expressive dynamics) — **CONFIRMED**
Velocity is three fixed values keyed purely to structural position: `V_INTERIOR=76`, `V_START=88`, `V_CADENCE=96`. No messa di voce, no crescendo/diminuendo contour, no accent patterns, no subito. It is a minimal phrase-position floor that guarantees intra-phrase variance > 0. All values are in valid MIDI range (1..=127). Clean S4-only layer; expressive dynamics correctly deferred to S6.

## Test Quality Assessment

The 5 new tests are **strong and honest**, not `is_ok()`/`!is_empty()`-grade:
- **Property 1** asserts a specific numeric bound (motion ≤ 7 semitones) per upper voice index, reading the bass-exemption from `voice_roles`. Real interval check.
- **Property 2** independently re-implements the T→T+1 pair comparison (does not call the production `has_parallel_perfects`), asserting no same-perfect-class-while-both-move. Genuine cross-time check.
- **Property 3** computes shared pitch classes between I and vi and requires a specific voice index to carry the shared pc at both T and T+1. Specific, would go red on a passthrough stub.
- **Property 4** asserts cadence-at-boundary, HalfCadence="V", PAC="I" preceded by "V", and "at least one of each" — and structurally that no interior step can be a cadence. Specific musical contract.
- **Property 5** checks real population **variance** > 0 within at least one phrase (not merely non-zero velocity).

Determinism contract is honored throughout: explicit Roman-numeral progressions (never `pick_progression`/RNG), `edge_complexity=0.0`, `brightness_drop=0.0`, fixed `ROOT=60`. The tests would genuinely go RED if the properties were violated (the doc comments even describe the stub-RED expectation). The implementation does not game a single literal: I confirmed the properties hold across all four distinct progressions used, and the algorithm is general (no hardcoded special-casing of a test progression).

The test module also retains a sound pre-existing net (range, chord-membership, mode-honored, numeral-resolution) — all green.

## Integration Assessment

- `voice_lead_sequence` returns `Vec<Chord>` with the same `Chord { name, notes: Vec<u8> }` shape `main.rs::worker_decide_action` consumes (it reads `chord.notes` / `chord.notes.len()` at lines 97, 124–125). Re-voicing is transparent — no signature or type break.
- `generate_chords` signature unchanged.
- `plan_phrases`/`StepPlan` is new isolated API **not yet wired into playback** — `main.rs` does not reference it (verified). This is a **clean deferral to S6**, not a dangling break: `main.rs` still calls `generate_chords` directly (line 422) and compiles/links fine. No TODO comments indicating incomplete integration were left in the new code.

## Blocking Issues

**None.**

## Non-Blocking Issues

1. **Stale doc comments (cosmetic, misleading).** The doc comments on `voice_lead_sequence` (lines ~287–289), `voice_roles` (~346–348), and `plan_phrases` (~386–389) still describe "Pass-1 stub behavior" ("returns the input sequence UNCHANGED", "places NO cadences", "assigns a FLAT velocity", "tests are expected to FAIL"). The real Pass-2 algorithm is implemented and the tests pass. These comments now contradict the code and should be updated to describe the shipped behavior. Likewise the velocity test comment (line 1364) references "a flat velocity (80)" which no longer matches (`V_INTERIOR=76` etc.).
2. **Two unused imports** in `chord_engine.rs` (`lookup_range_map`, `HashMap`) — trivially removable; harmless.
3. **Thin upper-voice voicings.** For several test progressions the two upper voices collapse to the same pitch (e.g. `IV=[65,65,65]`, final `I=[72,60,60]`), producing doubled/unison uppers. This satisfies all stated S4 properties (cap, no-parallels, common-tone) but is a weak voicing musically (loss of triad completeness / inner-voice independence). Not in scope for S4's stated contract; worth a follow-up for voicing-completeness quality (perhaps an S6/refinement constraint preferring distinct chord tones across the two uppers).
4. **Redundant common-tone clause** (line 719): `(prev.notes[vi] % 12) == (voicing[slot+1] % 12) && prev.notes[vi] == voicing[slot+1]` — the second (exact-equality) clause subsumes the first. Behaviorally correct (rewards exact held pitch only); the pc-class clause is dead. Cosmetic.

## Overall Verdict

**PASS WITH ISSUES.**

All three load-bearing musical checks are correct at source and confirmed by independent simulation: per-voice state is genuinely carried forward, parallel detection compares the same voice pair across T→T+1, and cadences land only at phrase boundaries with correct V/I identity. Common-tone retention is real, the ≤P5 cap holds, the two defensive relaxations are unreachable for the test inputs (green is not hollow), the velocity layer is a structural floor (not S6 expressive dynamics) in valid MIDI range, scale consts are canonical, module boundaries are respected (chord_engine touches no image/MIDI-out/modem code), and the new phrase API is a clean deferral that does not break main.rs. 16/16 tests pass, the 5 new property tests are specific and honest. The only issues are cosmetic/quality (stale "stub" doc comments, two dead imports, thin doubled-upper voicings, one redundant boolean clause) — none block integration.

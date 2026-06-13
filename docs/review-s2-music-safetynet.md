# Quality-Gate Review — WS-1 "Music Safety-Net" (Session 2)

**Reviewer:** Quality Gate (AudioHax)
**Scope:** Structural promotion of `chord_engine`/`mapping_loader` into the `audiohax` library, a property-test harness, and the music-theory fix (mode collapse + numeral mapping).
**Toolchain:** cargo 1.96.0 (userspace). All mechanical checks are **library-scoped** (`--lib --no-default-features`) by design — the binary target cannot build in this environment (no OpenCV/libclang/ALSA/cmake), an accepted condition, not a defect.

---

## Compilation Status

| Check | Command | Result |
|---|---|---|
| Library build | `cargo build --lib --no-default-features` | **PASS** (0.42s, 6 warnings, all pre-existing dead-code/unused-var) |
| Binary build | `cargo build` (full) | **NOT ATTEMPTED** — out of scope; would fail on OpenCV. Accepted coverage gap. |

No errors. The pure-Rust library (music + modem) compiles cleanly with system-library deps disabled — exactly the headless-buildability the promotion was meant to enable.

## Lint Status

| Check | Result |
|---|---|
| `cargo clippy --lib --no-default-features -- -W clippy::all` | PASS (warnings only, no errors) |
| `cargo fmt -- --check` | In-scope files **CLEAN**; pre-existing drift elsewhere (see Non-Blocking) |

Clippy notes touching in-scope files (`src/chord_engine.rs`), all **pre-existing**, none introduced by the fix:
- `unused_imports`: `std::collections::HashMap` (line 4) — dead before this session.
- `unused_imports`: `lookup_range_map` (line 1) — dead before this session.
- `unused_variable`: `next` (line 126) — pre-existing in the secondary-dominant branch.
- `useless_vec`: lines 47–48 (`vec!["Ionian",...]`) — pre-existing.

**fmt --check:** None of the four in-scope changed files (`Cargo.toml`, `src/lib.rs`, `src/main.rs`, `src/chord_engine.rs`) appear in the fmt diff set. All fmt diffs are in untouched pre-existing files (`src/bin/*`, `src/modem.rs`, `src/image_analysis.rs`, `src/image_source.rs`, `src/midi_output.rs`). The in-scope work is fmt-clean. Per instruction, no fixing `cargo fmt` was run.

## Test Results

`cargo test --lib --no-default-features` → **11 passed; 0 failed; 0 ignored.**

```
test chord_engine::tests::test_all_notes_within_playable_midi_range ... ok
test chord_engine::tests::test_ionian_triads_are_scale_derived_chord_tones ... ok
test chord_engine::tests::test_aeolian_triads_are_scale_derived_chord_tones ... ok
test chord_engine::tests::test_ionian_mode_honored_tonic_triad ... ok
test chord_engine::tests::test_aeolian_mode_honored_minor_tonic_triad ... ok
test chord_engine::tests::test_lydian_mode_honored_raised_fourth ... ok
test chord_engine::tests::test_mixolydian_mode_honored_flat_seventh ... ok
test chord_engine::tests::test_dorian_mode_honored_natural_sixth ... ok
test chord_engine::tests::test_phrygian_mode_honored_flat_second ... ok
test chord_engine::tests::test_iv_numeral_resolves_to_subdominant ... ok
test chord_engine::tests::test_iii_numeral_resolves_to_mediant ... ok
```

Expected count (11) matches exactly.

## Module Boundary Audit

**Git state matches expected.** `git status --porcelain`:
```
 M Cargo.toml
 M src/chord_engine.rs
 M src/lib.rs
 M src/main.rs
?? Cargo.lock
```
No out-of-scope src file is dirty. `modem.rs`, `image_*.rs`, `midi_output.rs`, `mapping_loader.rs` body, `bin/*`, `assets/mappings.json` are all untouched. **No boundary violation.**

**`src/chord_engine.rs` imports** (line 1–4) are exactly `crate::mapping_loader`, `rand`, and `std` — no image processing, no MIDI output, no modem references. Clean. (Two of those imports — `lookup_range_map`, `HashMap` — are pre-existing dead imports; noted under Non-Blocking, not a boundary breach.)

**Promotion is structural only:**
- `Cargo.toml`: `opencv`/`midir`/`image` are now `{ optional = true }` with a `[features] default = ["opencv", "midir", "image"]`. A normal `cargo build` is unchanged; `--no-default-features` yields the pure-Rust library. Correct.
- `src/lib.rs`: cleanly adds `pub mod chord_engine;` and `pub mod mapping_loader;` alongside the existing `pub mod modem;`. Correct.
- `src/main.rs`: repointed from `mod chord_engine;`/`mod mapping_loader;` to `use audiohax::{chord_engine, mapping_loader};`. The `use` paths line up with every in-body reference (`chord_engine::ChordEngine`, `chord_engine::Chord`, `mapping_loader::{load_mappings, lookup_range_map}`). **Verified behavior-identical** via `git diff -w`: the only non-whitespace changes are the `mod`→`use` repoint and import reordering; everything else is pure reformatting (brace expansion / line wrapping). No logic change.

**Coverage gap:** main.rs compilability cannot be verified here (OpenCV/ALSA absent). Stated as a known, accepted gap — not a failure. The `use audiohax::...` paths are confirmed by read-through to be internally consistent.

## Musical Logic Review

### Per-mode interval verification (semitone offsets vs. canonical reference)

| Mode | Production const | Reference | Match | Characteristic degree |
|---|---|---|---|---|
| Ionian | `[0,2,4,5,7,9,11]` | `[0,2,4,5,7,9,11]` | ✓ | major baseline |
| Dorian | `[0,2,3,5,7,9,10]` | `[0,2,3,5,7,9,10]` | ✓ | idx5 = +9 natural 6 |
| Phrygian | `[0,1,3,5,7,8,10]` | `[0,1,3,5,7,8,10]` | ✓ | idx1 = +1 b2 |
| Lydian | `[0,2,4,6,7,9,11]` | `[0,2,4,6,7,9,11]` | ✓ | idx3 = +6 #4 |
| Mixolydian | `[0,2,4,5,7,9,10]` | `[0,2,4,5,7,9,10]` | ✓ | idx6 = +10 b7 |
| Aeolian | `[0,2,3,5,7,8,10]` | `[0,2,3,5,7,8,10]` | ✓ | minor baseline |

All six offsets are **exactly correct** — every characteristic alteration sits at the right scale-degree index. **Bug 1 (mode collapse) is fixed:** the old 2-scale `if mode == "Ionian" || "Lydian" || "Mixolydian" { IONIAN } else { AEOLIAN }` is replaced by a 6-way `match` (lines 87–96) selecting the true scale, with an unrecognized-mode `_ => IONIAN` safe default.

### Numeral → degree mapping (lines 148–159)

`roman_to_chord` lowercases the numeral and exact-matches:

| Numeral | Degree | Correct |
|---|---|---|
| i | 0 (tonic) | ✓ |
| ii | 1 (supertonic) | ✓ |
| iii | 2 (mediant) | ✓ — was wrongly 1 (shadowed) |
| iv | 3 (subdominant) | ✓ — was wrongly 4 (shadowed) |
| v | 4 (dominant) | ✓ |
| vi | 5 (submediant) | ✓ |
| vii | 6 (leading tone) | ✓ |
| _ | 0 (safe default) | ✓ |

Case-insensitive (`to_lowercase()`), exhaustive, sane default. **Bug 2 fixed:** the order-shadowed `starts_with`/`len` chain is gone; the dead `"iv" => 3` / `"iii" => 2` arms are now reachable.

### Modal-interchange path

`generate_chords` (lines 103–114) rewrites `"IV"` → `"iv"` when `brightness_drop > threshold`. With the corrected numeral match, `"iv"` resolves to degree 3 (subdominant) — confirmed; the interchange still lands on the right scale degree after the fix.

### Scope discipline

This session is **scale + numeral correctness only**. No voice leading, phrase structure, dynamics, rhythm, articulation, or non-chord-tone machinery was introduced — triad construction remains the simple root/+2/+4-mod-7 it was. **No scope creep.**

## Test Quality Assessment

**Mode-honored tests assert specific MIDI values at characteristic degrees**, not weak liveness checks:
- Lydian #4: `ii` → `[62, 66, 69]` (66 = tonic+6, the #4; Ionian would give 65). Distinguishes from fallback. ✓
- Mixolydian b7: `vii` → `[70, 62, 65]` (70 = tonic+10; Ionian would give 71). ✓
- Dorian natural-6: `vi` → `[69, 60, 63]` (69 = tonic+9; Aeolian would give 68). ✓
- Phrygian b2: `ii` → `[61, 65, 68]` (61 = tonic+1; Aeolian would give 62). ✓

Each test names the exact wrong (collapsed-fallback) value it excludes, so none would pass against the pre-fix code. The four FAIL-then-PASS tests genuinely gate the fix. Ionian/Aeolian honored-tests assert exact triads too.

**Numeral tests corrected:** `test_iv_numeral_resolves_to_subdominant` asserts degree 3 (`IV != V`, `IV == expected_triad(REF_IONIAN, 3)`); `test_iii_numeral_resolves_to_mediant` asserts degree 2 (`iii != ii`, `iii == expected_triad(REF_IONIAN, 2)`). Both assert the **correct** post-fix degrees, not the old buggy ones. ✓

**Determinism contract held:** explicit progressions (never `pick_progression`), `edge_complexity = 0.0` (secondary-dominant branch never fires), `brightness_drop = 0.0` (modal-interchange branch never fires), fixed `root_midi = 60`. The `one_chord` helper asserts exactly one chord out, catching any stray RNG/inserted-chord. No `thread_rng` in the test path. ✓

**Reference scales independent of production constants** (`REF_*` consts in the test module) — the mode-honored tests assert against ground truth, not against whatever the engine currently does. Good practice.

## Integration Assessment

- The `chord_engine ↔ mapping_loader` boundary is type-consistent: `ChordEngine` holds `MappingTable`, `ChordEngine::new(mappings)` matches `load_mappings(...)` return, and the test harness loads real `assets/mappings.json` via `CARGO_MANIFEST_DIR` — exercising the live mapping types, not a stub.
- `chord_engine` and `mapping_loader` are both `pub mod` in lib.rs and successfully consumed from the in-file tests; they cohere as library modules.
- No leftover TODOs indicating incomplete integration in the changed code.
- **Binary-uncompilable coverage gap (explicit):** `src/main.rs` and the OpenCV/MIDI runtime path cannot be compiled or run in this environment. The structural repoint is verified by read-through and whitespace-diff, but end-to-end binary linkage and runtime behavior are unverified here. This is the accepted, known gap.

## Blocking Issues

**None.**

## Non-Blocking Issues

1. **Pre-existing dead imports in `chord_engine.rs`** (`HashMap` line 4, `lookup_range_map` line 1) — flagged by clippy, unused before this session, not introduced by the fix. Cheap cleanup for a future pass; not this session's scope.
2. **Pre-existing clippy nits** in `chord_engine.rs` (`useless_vec` lines 47–48, unused `next` line 126) — predate this work.
3. **main.rs diff is noisier than the minimal repoint** — the file was reformatted wholesale (brace expansion / line wrapping across the body). Verified behavior-identical via `git diff -w` (no logic change), but it enlarges the review surface. Worth a note to the lead so the broad diff isn't misread as behavioral.
4. **Repo-wide pre-existing fmt drift** in untouched files (`src/bin/*`, `modem.rs`, `image_analysis.rs`, etc.) — out of scope; do not fix from this session.

## Coverage Gaps

- **Binary target not buildable in this environment** (no OpenCV/libclang/ALSA/cmake) — accepted condition. main.rs structural correctness verified by read + whitespace-diff only; binary link/runtime unverified here.
- Modal-interchange and secondary-dominant branches are intentionally **not** exercised by the deterministic tests (both held at threshold-off). Their internal logic is unchanged this session, so this is acceptable, but those branches remain untested.

## Overall Verdict

**PASS**

The structural promotion is clean and behavior-preserving, the six diatonic mode scales are exactly correct, the numeral→degree mapping is exhaustive and de-shadowed, and the 11-test harness asserts specific, mode-distinguishing MIDI values under a sound determinism contract. Module boundaries are respected — no out-of-scope production file was modified. The only issues are pre-existing dead imports/clippy nits and a noisier-than-necessary (but behavior-identical) main.rs diff, all non-blocking. The single coverage gap — binary uncompilable in this environment — is the accepted, intentional condition of this lib-scoped review.

# Review S28 — BUILD K3 (Quality Gate)

**Slice:** K3 — the realizer pivot / common-tone modulation + land-home cadence slice. Adds the
additive `StepContext.prev_key_offset_semitones` + a `with_prev` constructor, carries
`Section.{pivot, resolution}` from the active scheme, the `CompositionPlan::prev_section_offset`
helper, two new realizer fns in `chord_engine.rs` (`pivot_chord_events`, `land_home_pitch`) + the
`land_home_is_armed` predicate + a guarded hook in `realize_step`, and flips six returning-Resolve
schemes to `pivot:true` in `mappings.json`. The Open `theme_and_variations_excursion` stays
`pivot:false` and unrouted (operator lock).

**HEAD:** `9fd46ad` (K2b BUILT & CLOSED). **Built in the working tree, UNCOMMITTED.**
**Surface verified:** `src/chord_engine.rs`, `src/composition.rs`, `src/engine.rs`,
`assets/mappings.json`, `tests/keyplan_k3.rs` (NEW), 10 existing test files (identity-carry edits),
`docs/spec-s28-k3-build.md` + `docs/input-s28-k3-pivot-harmony.md` (NEW).

## OVERALL VERDICT: PASS

All ten checks pass on independently re-derived evidence. The central re-baseline (the
lead-approved move of the engine.rs byte-anchor from `7a07fb85…` to `e50c7db1…`) is verified, the
new anchor matches the worktree exactly, the three re-pointed freeze guards are GENUINE (proven by
a mutate-and-restore that made the guard bite then restored engine.rs byte-identically), the
realizer change is behaviorally gated (None / false on every identity path, byte-identical to
pre-K3), the operator lock holds, and there are no codename leaks. The full default net is
all-green (28 binaries, 0 failures); `--lib --no-default-features` is 128/0. No blockers. Three
non-blocking carry-forward nits, none chargeable against the K3 slice.

---

## CHECK 1 — BYTE-FREEZE (central): PASS

- **`sha256sum src/engine.rs` = `e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261`**
  — matches the lead-approved NEW anchor exactly. `git show HEAD:src/engine.rs | sha256sum` =
  `7a07fb85…` (the OLD anchor), confirming HEAD predates the re-baseline and the worktree carries
  the new one. This is the documented "re-baseline as last resort" (spec §3 guarantee 3): the
  engine.rs change is the `use` import of `ResolutionPolicy`, the compose-path ctx switched to the
  `with_prev` constructor (the §3 contingency, minimizing textual delta), and the
  `legacy_default_section` identity-field carry. The constructor route minimized but could not
  eliminate the textual delta — `ResolutionPolicy` must be imported because it is now a `Section`
  field type — so the anchor was re-baselined, lead-approved per the prompt.
- **`cargo test --test engine_equivalence` → 9/9 byte-green.** Read the test source directly: the
  goldens **240** (cadence hold, `tests/engine_equivalence.rs:281`), **114 / 84** (bass velocity at
  sat100 / sat0, `:277`/`:293`), **36 / 79** (`G_BASS_NOTE` `:133` / `G_MELODY_NOTE` `:138`) are
  unmoved. The only diff to this test file is an additive identity-carry field add
  (`pivot: false, resolution: ResolutionPolicy::Resolve`) on the `default_section` fixture — no
  assertion, no golden, no behavior touched.
- **`realize_step` PUBLIC signature byte-identical worktree vs HEAD:** confirmed by diffing
  `git show HEAD:src/chord_engine.rs` against the worktree at the fn — same 7-param signature
  (`step, inst_idx, num_instruments, features, ms_per_step, ctx`) at the same line 1021. The new
  data rides `ctx` (the additive-PRIVATE-ctx route).
- **`sha256sum src/chord_engine.rs` CHANGED** (expected): `b448d936…` (HEAD) → `a891d478…`
  (worktree). The change is behaviorally gated (CHECK 2/3).

## CHECK 2 — IDENTITY INSERTS NOTHING (chord_engine behavioral freeze): PASS

- **`tests/keyplan_k3.rs::pivot_inserts_nothing_on_identity` is a REAL assertion** — read in full.
  It builds concrete `Section`/`StepContext` fixtures and `assert_eq!`s the full `Vec<NoteEvent>`
  stream (note, velocity, hold_ms, offset_ms) against the `single_section_default` baseline. It
  covers all three dead-path sub-cases over a 4-way role sweep (bass/fill/melody/lone-melody):
  (a) home-only / first-section identity (`prev:None`, offset 0, `pivot:false`), interior AND the
  PAC step; (b) a `pivot:true` boundary whose key EQUALS its predecessor (`prev == dest == +7`);
  (c) a `pivot:false` MODULATING boundary (`prev 0 → dest +7`). Not `assert!(true)`.
- **The gate confirmed by reading `pivot_chord_events`:** first branch `if !ctx.section.pivot ||
  ctx.step_in_section != 0 { return None; }`, then `let prev_off = ctx.prev_key_offset_semitones?;`
  (a `None` prev short-circuits to `None`), then `if prev_off == dest_off { return None; }`. The
  home_only / pivot:false / same-key / first-section paths therefore cannot fire.
- Test PASSES.

## CHECK 3 — PIVOT + LAND-HOME ARE REAL: PASS

- **`pivot_fires_on_modulating_boundary`** (read in full, non-vacuous): home C (pc 0), prev 0 →
  dest +7 (G). It asserts the bass pivot note's pitch class `== (dest_root_pc + 7) % 12 == 2` (D =
  the V of the destination G), `velocity == V_PIVOT (88)`, `hold_ms == ms_per_step`,
  `offset_ms == 0`, and that the chord is exactly one note per role. It then asserts a non-boundary
  step (`step_in_section == 1`) and a same-key boundary both equal the frozen baseline (no pivot).
- **`land_home_voicing_on_resolve_final`** (read in full, non-vacuous): asserts the armed final PAC
  step voices bass `% 12 == home_tonic_pc (0)` (root-position I) and melody/soprano `% 12 == 0`
  (the PAC marker), with event COUNT unchanged from the frozen single-note cadence. It includes
  negative arming guards (Open ending and `pivot:false` both fall back byte-identically).
- Both tests PASS. The `land_home_is_armed` predicate (read directly) requires all of: `pivot`,
  `resolution == Resolve`, `key_offset_semitones == 0`, `position == PerfectAuthenticCadence`.

## CHECK 4 — OPERATOR LOCK: PASS

- **`theme_and_variations_excursion` stays `pivot:false`:** confirmed in `assets/mappings.json` —
  the diff flips only the six returning-Resolve schemes; the Open scheme row is untouched (no
  `pivot:true` flip). It carries no `pivot` key change.
- **It is selected by NO routing rule:** the diff does not touch `key_scheme.rules`; the K2b
  unrouted state is preserved.
- **`cargo test --test keyplan_k2b` → `no_routed_image_ends_off_home` green** (14/14 in the
  binary, incl. `resolve_schemes_land_home`). The tripwire holds.

## CHECK 5 — NO REGISTER INVERSION: PASS

- **`no_inversion_invariant` green in BOTH** `tests/keyplan_s25.rs` and `tests/prominence_s23.rs`.
- **`no_inversion_under_pivot_path` (NEW, keyplan_k3) green** — sweeps every menu destination
  (`+7/+5/+3/−3`) × 3 brightnesses for the pivot boundary, plus the at-home Resolve final for
  land-home, asserting `bass < fill < melody` (mean pitch) and every note ∈ `24..=108`, with a
  combo-count assertion so the sweep cannot silently shrink. Confirmed by reading: the pivot/
  land-home voicings seat every pitch via the SAME `seat_pc_in_register(pc, role_floor)` helpers
  the free-select path uses, so the frame holds by construction.

## CHECK 6 — THE THREE RE-POINTED GUARDS ARE GENUINE: PASS

- All three read directly: `keyplan_s25.rs::engine_equivalence_byte_green`,
  `prominence_s23.rs::engine_freeze_diff_empty`, `affect_s22.rs::byte_freeze_witness_locked_files_unmoved`.
  Each shells out `sha256sum src/engine.rs`, compares against `const ENGINE_SHA256 =
  "e50c7db1…"` (the new anchor), and `assert_eq!`-fails on mismatch with a "drifted from the
  S28/K3 frozen anchor" message. (If `sha256sum` is unavailable they print an inconclusive notice
  and pass — a deliberate robustness fallback that still fails LOUDLY on a readable mismatching
  file, deferring to engine_equivalence + this QG diff as the authority. Not a no-op.)
- **The re-baseline is documented in-comment** in all three (e.g. "Re-baselined for the S28/K3
  slice (lead-approved, spec-s28-k3-build §3 guarantee 3)").
- **MUTATE-AND-RESTORE proof that the guard BITES:** I copied `src/engine.rs` aside, appended one
  comment line (sha → `9bb1255c…`), and ran `engine_equivalence_byte_green` — it FAILED with the
  exact "sha256 moved off the locked witness" panic at `keyplan_s25.rs:214`. I then restored from
  the copy and re-verified `sha256sum src/engine.rs` returned to `e50c7db1…` exactly. The guard is
  a true forward freeze, and the working tree is byte-identical to how I found it.

## CHECK 7 — MODULE BOUNDARIES: PASS

- **`chord_engine.rs` contains NO image/pixel type and NO `composition`/`engine` file edits leaked
  in:** grepping the added lines for `ImageUnderstanding|pixel|image_analysis|GlobalFeatures|hue`
  etc. → none. The pivot reads `ctx` only (`ctx.section.*`, `ctx.prev_key_offset_semitones`,
  `ctx.key_tempo.*`, `ctx.step_in_section`) plus `features` (the realizer's own `PerfFeatures`
  performance struct — the same brightness input the free-select path already uses; NOT an image
  type) and names no pixel type.
- **`engine.rs` adds no note-SELECTION logic** — only data threading (the `with_prev` ctx build
  carrying `comp.prev_section_offset(step_idx)`, and the `legacy_default_section` identity carry).
- **`realize_step` public signature unchanged; no public API broke** (CHECK 1). The 4th arg
  `role: OrchestralRole` on `pivot_chord_events` is a chord_engine.rs-internal PRIVATE fn arg —
  `pivot_chord_events`, `land_home_is_armed`, `land_home_pitch` are all module-private `fn`
  (the keyplan_k3 integration test drives them only through the public `realize_step`).

## CHECK 8 — LOCKOFF SET UNTOUCHED: PASS

- **`git diff --stat HEAD`** modified files: `assets/mappings.json`, `src/chord_engine.rs`,
  `src/composition.rs`, `src/engine.rs`, plus 10 existing test files. Untracked NEW:
  `tests/keyplan_k3.rs`, `docs/spec-s28-k3-build.md`, `docs/input-s28-k3-pivot-harmony.md`.
- **The 10 modified existing test files are ALL pure identity-carry** (verified line-by-line): each
  adds the `ResolutionPolicy` import + `pivot: false`, `resolution: ResolutionPolicy::Resolve`, and
  where a `StepContext` literal exists `prev_key_offset_semitones: None`, to existing `Section`/
  `StepContext` literals so the additive struct fields compile. No assertion or behavior changed.
  This is exactly the spec §2.2(e)/§6 requirement ("update every other literal to the identity
  values so the tree compiles") — in-scope, not creep.
- **`git diff --stat HEAD` for the LOCKOFF set** (`src/midi_output.rs`, `src/synth_sink.rs`,
  `src/cli.rs`, `src/tui.rs`, `src/main.rs`, `src/modem.rs`, `src/bin/*`, `src/lib.rs`,
  `src/pure_analysis.rs`, `src/image_analysis.rs`, `src/image_source.rs`, `src/mapping_loader.rs`):
  EMPTY — none modified.

## CHECK 9 — CODENAME SCRUB: PASS / CLEAN

- Grepped all changed code files (`chord_engine.rs`, `composition.rs`, `engine.rs`,
  `mappings.json`, `keyplan_k3.rs`) AND the two new docs for stray external project/tooling
  identifiers and process names, with common-word false positives filtered. Result:
  **CLEAN — zero leaks** (reported by category, no token quoted).

## CHECK 10 — FULL NET + SCOPE: PASS

- **`cargo test` (full default net) → ALL-GREEN.** 28 test binaries, 0 failures. Notable counts:
  lib 163/0, main 5/0, engine_equivalence 9/9, keyplan_k3 4/0, keyplan_s25 11/0, keyplan_k2b 14/0,
  keyplan_k2a 9/0, prominence_s23 5/0, affect_s22 8/0, modem_roundtrip 17/0, modem_realair 10/0,
  saliency_s18 12/0, all others green.
- **`cargo test --lib --no-default-features` → 128 passed; 0 failed** (exactly the spec target).
- **`cargo build --release` clean.**
- **Scope:** K3 stayed WITHIN the slice — the realizer pivot (V-of-destination, one rule for all
  menu offsets), the land-home PAC voicing, the additive `StepContext` field + `with_prev`
  constructor + `prev_section_offset` helper, the `Section.{pivot,resolution}` carry, and the six
  scheme flips. Honest statement of what K3 does NOT do: the Open `theme_and_variations_excursion`
  is still `pivot:false` and unrouted (operator lock); cross-step TIED sustain is OUT of scope
  (the pivot occupies the boundary step only, `hold_ms = ms_per_step`, relying on the existing
  legato-overlap — no `main.rs` scheduler change); `main.rs` / synth sinks / MIDI output untouched;
  the pivot is one unified rule (V/destination), not six bespoke common-chord tables — a deliberate
  conservative choice documented in the harmony input doc.

---

## BLOCKING ISSUES

None.

## NON-BLOCKING CARRY-FORWARD NITS

1. **N-K3-1 — one clippy style nit inside the K3 new code.** `src/chord_engine.rs:2295`
   (`land_home_pitch`): `(ctx.key_tempo.home_root_midi % 12) as u8` is an unnecessary cast —
   `home_root_midi` is already `u8` (`composition.rs:844`), so the cast is a no-op with zero
   behavioral effect. Style-only; consistent with the codebase's many pre-existing style warnings
   (clippy emits no `error` and no correctness lint anywhere). Optionally drop the `as u8`.
2. **N-K3-2 — pre-existing `cargo fmt --check` diffs in LOCKOFF files.** `make_tiled_payload.rs`,
   `unpack_tiled_payload.rs`, `image_analysis.rs`, `image_source.rs` show fmt diffs — NONE are
   K3-touched files, so these pre-date K3 and are not chargeable to this slice. No K3-touched file
   appears in the fmt diff (chord_engine / composition / engine / keyplan_k3 are all fmt-clean).
3. **N-K3-3 — stray untracked image artifact.** `assets/images/magicstudio-art.jpg` is untracked
   and referenced by NO K3 source/test/doc — a stray operator ear-test input sitting alongside the
   tracked sample images. Not part of the slice, no effect on correctness or the byte-freeze.
   Housekeeping only (decide whether to add or remove before commit).

> Note on the §7 K2b carry-forwards: N-K2b-1 (the dead `excursion_offset` cleanup) was OPTIONAL and
> appears not to have been actioned in this slice (no diff line touches it); per spec §7 it was
> explicitly "DEFER if it risks the slice" — leaving it is acceptable and non-blocking. N-K2b-2 (the
> Open-ending re-listen) is correctly deferred — the Open scheme stays locked, as designed.

---

*Quality Gate, S28 / K3. Independently re-derived: every sha computed, every witness test run, the
freeze guard proven to bite via mutate-and-restore (engine.rs restored byte-identically), the full
net and the no-default-features lib run, the module boundaries and codenames grepped. Working tree
left exactly as found — no edit, no stage, no commit. The build-role titles are the S21/S24/S26
domain titles.*

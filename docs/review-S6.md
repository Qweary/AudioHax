# Quality Gate Review — S6 (WS-1 Music Expressivity Layer 2)

**Scope:** `realize_step` + helpers, `instrument_role`, `PerfFeatures`/`NoteEvent`/`OrchestralRole`,
the voice-spacing fix (`upper_voices_well_spaced` + its wiring into `voice_lead_one`), the 7 new S6
property tests, and the `main.rs` thin-adapter rewrite. Reviewed against `docs/design-s6-expressivity.md`.

**Verdict: PASS WITH ISSUES** (no blocking issues; minor non-blocking cleanups noted).

---

## Compilation Status

- **Lib: GREEN.** `cargo build --lib --no-default-features` finishes clean (warnings only, none from S6 logic).
- **Binary (`main.rs`): typecheck/link DEFERRED.** The binary unconditionally imports OpenCV/ALSA/highgui,
  which cannot build on this machine (no OpenCV/ALSA/cmake toolchain). Per the brief, `main.rs` was reviewed
  **by inspection** — call sites walked against the lib signatures (see Integration Assessment). The full
  typecheck/link of the binary must be re-run on a machine with the OpenCV toolchain before ship.

## Lint Status

`cargo clippy --lib --no-default-features -- -W clippy::all`:

- **S6-introduced:** 3 × `doc_lazy_continuation` style warnings on the `PerfFeatures` doc block
  (`chord_engine.rs:562–564`). Pure documentation formatting — NON-BLOCKING, not a correctness issue.
- **Pre-existing (NOT S6):** `useless_vec` at `chord_engine.rs:46,47` and `unused variable: next` at
  `chord_engine.rs:125` — all inside the original `pick_progression`/`generate_chords`, untouched by S6.
- No correctness-class clippy warnings in any S6 code (`realize_step`, helpers, voice-spacing).
- `rustfmt --edition 2021 --check` on **both** `src/chord_engine.rs` and `src/main.rs`: **clean, no drift.**

## Test Results

`cargo test --lib --no-default-features chord_engine`: **23 passed; 0 failed; 0 ignored** (11 filtered out).
- 16 S4 tests + 7 S6 tests, all green.
- **Modem-lane RED is OUT OF SCOPE** and not present in this filter: the concurrent WS-2 work
  (`src/modem.rs`, `tests/modem_roundtrip.rs`) has 2 intentionally-failing test-first tests; they are
  not part of S6 and were correctly excluded by the `chord_engine` filter.

## Module Boundary Audit

- **`chord_engine.rs`:** imports only `mapping_loader::MappingTable` + `rand` (pre-existing, used by
  `pick_progression`). **No image/OpenCV/MIDI/modem import.** The S6 layer takes plain-scalar `PerfFeatures`
  in and emits plain `NoteEvent` data out — no rendering/vision/MIDI type crosses into the lib. Headless-testable. CLEAN.
- **`main.rs`:** `worker_decide_action` is a genuine thin adapter — (1) `plan[step_idx % len]` lookup with an
  empty-plan guard, (2) project `ScanBarFeatures → PerfFeatures`, (3) call `realize_step`, (4) map
  `NoteEvent → (note,vel,hold,offset)`. **No musical decision remains:** no velocity formula, no
  arpeggio/rhythm/register/octave branching, no `edge_density` branch. The only `edge_density`/`brightness`
  references are the feature projection and the unrelated `generate_chords` call. The dead
  `velocity_from_saturation` and the old arpeggio/octave logic are **fully removed.** CLEAN.
- **Ownership:** `git diff` shows S6 logic changes confined to `chord_engine.rs` (MTS + Test Engineer) and
  `main.rs` (Implementer). `image_analysis.rs`/`image_source.rs`/`midi_output.rs` are also dirty but their
  diffs are **pure rustfmt reflows** (whitespace/line-wrapping, zero logic) — not S6 music work and not a
  boundary violation. No agent edited another's production logic. CLEAN.

## Musical Logic Review (rigorous stage — mechanism confirmed, not just test names)

1. **Dynamics — REAL contour, not `saturation × constant`.** `realize_velocity` (`chord_engine.rs:776`)
   computes `floor + level_gain(saturation) + half-sine messa-di-voce(position) + metric accent(even/odd,
   downbeat strongest) + phrase-end taper`, cadence exempt. Independently traced with constant saturation:
   the floor series `[88,76,76,96]` (var 72.0) becomes `[100,76,80,99]` (var 117.7) — the contour adds
   variance ON TOP of the floor and reshapes the interior, it does not scale it. Strong beat (downbeat 100)
   > adjacent weak (76); realized start-minus-interior gap (24) strictly exceeds the floor gap (12). REAL.
2. **Rhythm — ≥3 genuinely distinct onset/duration sequences + cadence acceleration.** `realize_rhythm`
   (`chord_engine.rs:853`) emits distinct sequences: bass sustained (1 onset) vs pre-cadence bass (2 onsets
   at 0 and 3/4); fill sustained/rest; melody arpeggio (3–4 even onsets, staccato), syncopated (onset at 1/4
   + 3/4), dotted (onsets 0 and 2/3), sustained. Harmonic-rhythm acceleration is real: `pre_cadence` melody
   uses n=4 onsets vs 3 for an active-interior step, and the bass adds a pickup onset before a cadence — onset
   count on the pre-cadence step exceeds an early-interior step. ≥3 distinct shapes confirmed (test scan finds
   ≥3 over role × edge × position). REAL.
3. **Phrase-end ritardando increases actual hold_ms.** Cadence steps return a single sustained note with
   `LEGATO_FRAC × RITARDANDO_FACTOR (1.30)` (capped 1.20 of slot) over the FULL `step_ms` slot
   (`chord_engine.rs:912`), while interior melody/fill notes hold sub-slot fractions of shorter sub-slots.
   The cadence hold (≈1000ms+ at full slot) numerically exceeds the interior mean. REAL.
4. **Articulation fractions vary.** Three distinct constants — `STACCATO_FRAC 0.40`, `PORTATO_FRAC 0.70`,
   `LEGATO_FRAC 0.95` — selected by role/edge band, plus the ritardando multiplier. Not all ~0.9. REAL.
5. **Orchestration roles — register from ROLE, alias impossible.** `role_pitch` (`chord_engine.rs:705`)
   derives pitch from the ROLE: Bass = chord root seated at `BASS_REGISTER_FLOOR 36`; Melody = top chord tone
   at `MELODY_REGISTER_FLOOR 67` (+ brightness lift); Fill = inner tone at `FILL_REGISTER_FLOOR 55`. **It does
   NOT index `notes[idx % len]`,** so a 4th instrument cannot alias onto the bass pitch — the alias bug the
   brief flagged is structurally impossible. Register order bass < fill < melody holds by floor construction.
   Melody onset count ≥ bass by pattern design. `instrument_role` mapping (idx 0→Bass, num-1→Melody,
   middle→Fill; num≤1→Melody; num=2→Bass/Melody) is correct and total. REAL.
6. **Voice-spacing fix — real, complete, no regression.** `upper_voices_well_spaced` (`chord_engine.rs:1017`)
   does a true pairwise-distinct scan over indices `1..` (bass exempt), and is wired as a HARD reject in
   `voice_lead_one`'s candidate scan (`legal = !has_parallel_perfects(...) && upper_voices_well_spaced(...)`,
   line 1264). It covers ALL collapse cases (the test asserts the rule over EVERY chord of `["I","IV","V","I"]`,
   not just IV). S4 constraints are not regressed — all 5 S4 voice-leading tests stay green (≤P5, no
   parallels, common-tone retention, cadence placement). **Last-resort fallback ordering is musically correct:**
   it prefers a well-spaced voicing even at the cost of relaxing the *softer* parallel-perfects rule, and only
   collapses spacing in a truly-degenerate pass (2) when no spaced voicing exists at all — i.e. it relaxes the
   lesser sin first. REAL.
7. **`realize_step` is pure/deterministic.** No RNG, no I/O anywhere in the realization path (`rand`/`thread_rng`
   are only used by `pick_progression`, not reachable from `realize_step`). The property net depends on this and
   it holds. REAL.
8. **Clamping.** `seat_pc_in_register` clamps notes to `24..=108`; `realize_velocity` clamps to `1..=127`. CONFIRMED.

## Test Quality Assessment

The 7 S6 tests assert meaningful numeric properties, not `is_ok()`/`!is_empty()`:
- Dynamics-1 asserts realized variance **strictly exceeds** the floor variance (not merely non-zero) — the
  exact "not a scalar multiple" guard the design demands. Strong.
- Dynamics-2 asserts the realized strong-minus-weak gap **exceeds the structural-floor gap** — pins the accent
  adds on top of the floor, not just that start>interior. Strong.
- Rhythm asserts ≥3 distinct `(offset_ms,hold_ms)` signatures across role × edge × position. Strong.
- Articulation-1 asserts ≥2 distinct quantized hold fractions; Articulation-2 asserts cadence hold > phrase
  interior MEAN. Strong.
- Orchestration asserts register ordering (bass_min < melody_max), onset-count ordering, non-identity, AND the
  full `instrument_role` truth table incl. degenerate counts. Strong.
- Voice-spacing asserts distinct upper notes over **EVERY** chord in the progression, AND that
  `upper_voices_well_spaced` agrees with the actual-notes verdict. Strong — tests actual notes so it can't be
  gamed by the checker alone.

No weak tests found.

## Integration Assessment (cross-compile-deferred type-match walk + scheduler tolerance)

Walked every `main.rs` call site against the lib signatures (load-bearing since `main.rs` can't compile here):
- `engine.plan_phrases(&chords_vec) -> Vec<StepPlan>` (main:383) matches `pub fn plan_phrases(&self, &[Chord]) -> Vec<StepPlan>`. ✓
- `Arc::new(plan_vec)` → `play_scanned_steps_concurrent(.., plan: Arc<Vec<StepPlan>>, ..)` (main:103, call main:458). ✓
- `worker_decide_action(f, inst_idx, step_idx, num_instruments, &plan_cl, ms_per_step, h_threshold)` (main:157)
  matches the rewritten signature (main:60). `&plan_cl` is `&Arc<Vec<StepPlan>>` → param `&[StepPlan]` via deref
  coercion. ✓
- `PerfFeatures { saturation, brightness, edge_density }` (main:80) — all three lib fields are `f32`;
  `ScanBarFeatures.avg_saturation/avg_brightness/edge_density` are all `f32` with matching units
  (sat/bright 0..=100, edge 0..=1). No cast needed, projection is exact. ✓
- `realize_step(step, inst_idx, num_instruments, &features, ms_per_step) -> Vec<NoteEvent>` (main:88) matches
  the lib signature; `NoteEvent{note:u8,velocity:u8,hold_ms:u64,offset_ms:u64}` maps 1:1 onto the existing
  `(u8,u8,u64,u64)` tuple. ✓
- **Scheduler tolerance:** the coordinator loop (main:245–276) iterates `&action.events` — an **empty Vec**
  (fill rest-as-gesture) produces zero ScheduledEvents for that instrument with no special-casing, and
  **multiple non-zero `offset_ms`** events each get an independent `t0 + offset_ms` start and a sorted
  on/off pair. Both cases are handled with NO scheduler change required. ✓
- **No TODO/FIXME** comments left in either file.

## Blocking Issues

**None.**

## Non-Blocking Issues

1. **`_edge_threshold` param in `main.rs:67`** — underscore-prefixed, retained for call-site compatibility,
   no longer read (the worker no longer branches on edge density; `realize_step` owns it). **Acceptable as-is.**
   My call: leave it. Removing it touches the spawn call site and buys nothing musically; the underscore makes
   the intent explicit and a future caller may re-thread it. If a later pass cleans up the worker signature,
   drop it then.
2. **`doc_lazy_continuation` ×3 (`chord_engine.rs:562–564`)** — S6-introduced doc-comment style only. A blank
   line or a `cargo clippy --fix` would clear them. Cosmetic.
3. **Pre-existing lib warnings** (`useless_vec` 46/47, `unused next` 125) — not S6; can be swept opportunistically.
4. **`midi_output::play_chord_arpeggio` is now unused** (the old arpeggio path is gone) — but it's pre-existing
   public API and out of S6 scope; flag only so the lead can decide whether to retire it later.
5. **Binary typecheck DEFERRED** (restated): re-run `cargo build`/`cargo build --release` on an OpenCV-capable
   machine to confirm the seam links. Inspection says it will; a real compile is the final confirmation.

## Overall Verdict

**PASS WITH ISSUES.** The lib is green, all 23 `chord_engine` tests pass, clippy shows no S6 correctness
warnings, and both files are rustfmt-clean. All four expressive dimensions plus the voice-spacing fix are
implemented by genuine mechanisms — independently traced, not just test-passing. Module boundaries are clean
(no image/MIDI/modem type in the lib; no musical logic in `main.rs`). The only items are minor/cosmetic
non-blockers and the unavoidable binary-typecheck deferral. Recommend the lead integrate after a binary
compile on an OpenCV-capable machine.

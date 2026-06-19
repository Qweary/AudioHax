# Quality-Gate Review — S40 `play` Real-Time Scheduler Path Fix

**Reviewer role:** Quality Gate (independent verifier — verify only, no implementation).
**Scope:** the focused fix to the live `play` scheduler in `src/main.rs` that paces playback
on the same absolute grid `render --wav` uses, so live `play` no longer rushes (~1.8x) and no
longer eats rests.
**Date:** 2026-06-18
**Verdict:** **PASS** (no blocking issues; minor non-blocking notes below)

---

## Freeze Verification

- `sha256sum src/engine.rs` → `e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261`
  — **MATCHES** the freeze anchor. Re-checked after the full build/test/render sequence; still
  unchanged.
- `cargo test --test engine_equivalence` → **9 passed; 0 failed** → **engine_equivalence 9/9.**
- `git diff --stat` → `src/main.rs | 127 ++…`, **1 file changed, 113 insertions(+), 14 deletions(-)**.
  Only `src/main.rs` is uncommitted against HEAD (`c63b3b1`, Slice-2 committed). No `src/engine.rs`,
  `assets/*`, or `tests/*` modification. **Confirmed: this fix touches main.rs only.**

---

## Compilation

`cargo build --release` → **Finished** (success). The only warnings emitted are pre-existing and
in unrelated binaries (`unpack_tiled_payload`, `modem_encode`) — `unused_variables` /
`unused_assignments` on a `seq` counter — not attributable to this fix and not in `audiohax`'s
`play`/`render` code path.

---

## Lint

- `cargo fmt -- --check` → exit 0 — **clean** (non-blocking gate, but it passes).
- `cargo clippy -- -W clippy::all` → **0 errors.** Warnings exist, all of clippy *style/pedantic*
  class (`doc list item …`, `loop variable used to index`, `div_ceil`/`is_multiple_of` suggestions,
  `&Vec` vs `&[_]`, `io::Error::other`, etc.) and all in the **lib** (modem/FEC/synth code), not in
  the changed bin. Filtering clippy output for `src/main.rs` references returns **zero** lines — the
  changed file introduces no clippy warnings of any class. Pre-existing lib warnings are explicitly
  out of scope for this fix and are not blocking (correctness lints are the blocking gate; there are
  none).

---

## Test Results

| Suite | Command | Result |
|---|---|---|
| Library | `cargo test --lib --no-default-features` | **180 passed, 0 failed** |
| Binary (incl. onset helper) | `cargo test --bin audiohax` | **9 passed, 0 failed** |
| Engine equivalence | `cargo test --test engine_equivalence` | **9 passed, 0 failed (9/9)** |

The four new `step_onset_offset_ms` helper tests are present and green in the bin suite:
`onset_step_zero_is_zero`, `onset_equals_step_times_grid`, `onset_is_monotonic_nondecreasing`,
`onset_zero_grid_is_always_zero`.

---

## Scheduler Logic Review (the core)

Verified against the diff and the full `play`/`render` bodies:

1. **Plan-tempo bind (mirrors render).** `play` now binds the grid identically to render
   (main.rs ~:333 in `run_render_wav`):
   ```rust
   let (total_steps, grid_ms_per_step): (usize, u64) = match engine.composition() {
       Some(plan) => (plan.total_steps, plan.key_tempo.base_ms_per_step),
       None => (source.step_count(), ms_per_step), // legacy fallback (non-composed)
   };
   ```
   This is the same `engine.composition() => Some(plan) => plan.key_tempo.base_ms_per_step`
   read-only-accessor pattern render uses. **Confirmed.**

2. **Single epoch, captured once before the loop.** `let epoch = Instant::now();` sits *before*
   `for step_idx in 0..total_steps`. The pre-fix per-step `let t0 = Instant::now();` is **removed**
   (visible in the diff as a `-` line) and replaced with
   `let t0 = epoch + Duration::from_millis(step_onset_offset_ms(step_idx, grid_ms_per_step));`.
   Each step's onset is therefore anchored at `epoch + step_idx * grid_ms_per_step`, so a step whose
   voices end early — or rests entirely — still occupies its full grid slot (the inner
   `sleep(sev.at - now)` idles through the gap rather than skipping ahead). **Rests preserved, rush
   eliminated. Confirmed.**

3. **One shared formula, both paths.** `render`'s step base was switched from the inline
   `let step_base_ms = step_idx as u64 * grid_ms_per_step;` to
   `let step_base_ms = step_onset_offset_ms(step_idx, grid_ms_per_step);` (diff shows the `-`/`+`
   pair). `play` calls the *same* `step_onset_offset_ms`. There is now exactly one onset formula
   serving both paths. **Confirmed.**

4. **Non-composed fallback unchanged.** The `None` arm falls back to
   `(source.step_count(), ms_per_step)` — today's non-plan behavior. Note that for the legacy path,
   pre-fix `play` reset `t0` per step (emergent tempo), whereas post-fix it now also lays the legacy
   steps on `epoch + step_idx * ms_per_step`. This is a *behavioral improvement consistent with the
   fix's intent* (legacy `ms_per_step` becomes an honored grid instead of a header-only number) and
   does not silently alter the *composed* path or the engine's note decisions; see Non-Blocking note 1.

5. **Note decisions untouched.** Both paths still call `engine.decide_step(&source, step_idx)`
   identically; the within-step `ev.offset_ms` / `ev.hold_ms` scheduling and the `jitter_percent`
   application are byte-for-byte unchanged in `play`. The fix is **pacing-only.** **Confirmed.**

---

## Ctrl-C Responsiveness (regression risk introduced by the fix)

The fix adds the risk that waiting for a step's first onset is now potentially a full grid slot
(~900 ms) — sleeping that in one shot would delay shutdown. Verified mitigations:

- The wait-to-onset is **sliced**: `const SHUTDOWN_POLL_SLICE: Duration = Duration::from_millis(20);`
  and `std::thread::sleep(remaining.min(SHUTDOWN_POLL_SLICE));` inside a `loop` that recomputes
  `remaining = sev.at.checked_duration_since(now)` each pass and `break`s once the onset is reached.
- The shutdown `AtomicBool` is polled **between slices** (inside the loop, before each sleep) and
  again immediately after the loop, so Ctrl-C breaks within ≤~20 ms — well inside one step. BUG-01
  responsiveness is **preserved, not regressed.**
- The **pre-step** poll (`if shutdown.load(..) { break; }` at top of the `for step_idx` body) and the
  **per-event** poll (`if shutdown.load(..) { break; }` at the top of `for sev in events`) both
  remain. **Confirmed.**

---

## Path / Transport Integrity

The runtime sink selection block is untouched by the diff:

- `--output synth` (rustysynth + cpal `SynthSink::new_with_config`) and the A/B controls
  (`--soundfont` / `--reverb` / `--gain`) — unchanged.
- `--output midi` / `--midi-virtual` sink selection (`MidiOut::open_virtual` / `open_selector`,
  `AUDIOHAX_MIDI_PORT` env, `--midi-port`) — unchanged.
- `note_on` / `note_off` / `program_change` calls and the initial `prog = (i*7)%128` program-change
  scheme — unchanged.
- Jitter (`jitter_percent` on `hold_ms`) — unchanged.

Only **step pacing** changed. **Confirmed.**

---

## Test Quality

The four `step_onset_offset_ms` inline tests assert *real load-bearing properties*, not tautologies:

- `step0 == 0` for multiple grids (0, 250, 912) — the epoch anchor.
- `== step_idx * grid` for concrete cases (1×912, 3×250, 29×912) — locks the exact formula shared
  with render's `step_base_ms`.
- monotonic non-decreasing across 64 steps — the grid never compresses.
- degenerate zero-grid stays 0 with no panic — totality.

**Honest limitation (stated as required):** real-time `play` timing is *not* unit-testable headless
(no audio device, wall-clock sleeps). The validation chain is therefore: (a) the helper test locks
the shared math, (b) this code review confirms `play` and `render` both consume that math and that
only pacing changed, and (c) the operator re-listen is the terminal acceptance gate. The unit test
proves the *formula*; it cannot and does not prove the *realized audio tempo* — that rests on the
review + operator ear. This is the correct and honest scope for a headless QG.

---

## Non-Interactive Cross-Check (render reference tempo)

`cargo run --release -- render assets/images/AudioHaxImg1.jpg --wav /tmp/qg-verify.wav`:

- **30 steps, 4 instruments.**
- WAV duration **29.0 s** (font=bundled, reverb=on, gain=1).
- Implied grid: the 30 step onsets span `0 .. 29 * grid`, plus the final step's note hold + the
  fixed 1.5 s reverb tail. `29 * 0.917 s ≈ 26.6 s` of onset span + ~0.9 s final hold + 1.5 s tail
  ≈ **29.0 s** — consistent with the **~917 ms/step** plan tempo expected. This is the reference
  grid that live `play` now matches (both call `step_onset_offset_ms` with the same
  `plan.key_tempo.base_ms_per_step`).

**Real-time `play` cannot be exercised headless** (no audio output device / no operator ear in this
environment), so the realized live tempo is verified by formula-sharing + code review here and must
be confirmed by the operator re-listen.

---

## Blocking Issues

**None.**

---

## Non-Blocking Issues

1. **Legacy (`None`) path now grid-paces too.** In the non-composed fallback, post-fix `play` lays
   steps on `epoch + step_idx * ms_per_step` instead of the pre-fix per-step `Instant::now()` reset.
   This is consistent with the fix's intent (honor the configured grid) and is arguably a *fix* for
   the legacy path as well, but the work order framed the legacy arm as "preserves today's behavior."
   It preserves today's *step count and tempo source* and does not change note decisions or the
   composed path; the realized legacy pacing does change from emergent-to-grid. Flagging for the
   record — not a defect, but worth an operator note if any legacy/non-plan playback was being relied
   on for its old emergent timing.
2. **Pre-existing lib clippy/style warnings (41) and two unrelated-bin `unused` warnings** remain
   out of scope for this fix; none are in `audiohax`'s changed code.

---

## Overall Verdict

**PASS.** The fix does exactly what the work order specifies: it makes live `play` pace on the same
absolute grid `render --wav` uses, via one shared `step_onset_offset_ms` formula, a single
captured-once epoch, and a plan-tempo bind that mirrors render. Rests are preserved, the ~1.8x rush
is eliminated, note decisions are untouched (pacing-only), Ctrl-C responsiveness is preserved via
20 ms sliced sleeps, and all transport/sink paths are unchanged. Freeze anchor holds
(engine.rs sha matches, engine_equivalence 9/9). All test suites green (180 lib / 9 bin / 9 equiv).
The single non-blocking note (legacy-path grid pacing) is an improvement, not a regression. Final
acceptance of the *audible* tempo correctly rests with the operator re-listen, as real-time playback
is not headless-testable.

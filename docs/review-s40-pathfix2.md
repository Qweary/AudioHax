# QG Review — S40 Path-Fix #2 (shared `build_step_event_timeline`, `play` == `render`)

**Reviewer:** Quality Gate (independent verification; no implementation)
**Scope:** the second S40 main.rs refactor — unify `play` and `render --wav` onto ONE shared event timeline (`build_step_event_timeline`), finishing the unification the pacing fix started. Verifies the §3 Option A work order of `docs/design-s40-path-divergence-2.md`.
**Date:** 2026-06-19
**Working tree:** `src/main.rs` modified (uncommitted, carries pacing fix + this timeline refactor layered); Slice-2 committed at HEAD `c63b3b1`; path-divergence docs untracked.

---

## Overall Verdict: **PASS**

The refactor faithfully extracts render's prior event-emission into a single `build_step_event_timeline(...)` builder consumed by BOTH paths, and rewires `play` from a per-step batched drain to a single streaming walk of that one sorted timeline. `play == render` is genuinely achieved **by construction** at the timeline-mechanism level. The freeze holds, all gates are green, and the `thread_rng` non-determinism is a real, correctly-bounded property that does **not** undermine the equality claim. No blocking issues.

---

## Compilation

- `cargo build --release` — **PASS**. Clean build. The only warnings are pre-existing and unrelated to this fix: `src/bin/modem_encode.rs` (unused `seq`) and `src/bin/unpack_tiled_payload.rs`. Not in the fix's blast radius.

## Lint

- `cargo fmt -- --check` — **PASS** (no diff; non-blocking gate clean anyway).
- `cargo clippy --bin audiohax -- -W clippy::all` — **PASS for this fix.** The 2 warnings attributed to the audiohax bin resolve to `src/modem.rs` (`too many arguments 8/7` at modem.rs:1167), not `main.rs`. Lib-level warnings (41) are pre-existing and unrelated. **Zero clippy findings on the new `build_step_event_timeline` / the rewritten play loop.**

## Test Results

| Suite | Result |
|---|---|
| `cargo test --lib --no-default-features` | **180 passed**, 0 failed |
| `cargo test --bin audiohax` | **14 passed**, 0 failed (incl. all 5 new timeline tests) |
| `cargo test --test engine_equivalence` | **9 passed**, 0 failed — **9/9 ✓** |

All five new timeline tests pass: `timeline_is_sorted_by_at_ms`, `timeline_same_pitch_off_precedes_cross_step_on`, `timeline_preserves_cross_step_overlap`, `timeline_is_deterministic_single_source_for_both_paths`, `timeline_has_program_change_init_at_zero`.

---

## Shared-Timeline Logic Review — is `play == render` truly by construction?

**Yes — at the timeline-building mechanism, verified in code, not merely in name.**

- **ONE builder, BOTH paths.** `build_step_event_timeline` (main.rs:253-318) is the single source.
  - `run_render_wav` (main.rs:430-436) replaces its old inline `:345-384` accumulation with one call to the builder, then hands the result to `render_events_to_stereo` (main.rs:449). Confirmed render no longer accumulates inline.
  - The play loop (main.rs:783-789) calls the SAME builder once *before* the loop, into `timeline`, and streams it. The old per-step "build local `events` → sort → drain to completion → advance" structure is **gone** — replaced by a single `'stream: for tev in &timeline` walk (main.rs:810-875). This is the load-bearing change and it is real in code.
- **Builder body == render's old inline logic + idempotent sort.** The builder pushes per-channel `ProgramChange` at `at_ms=0` (prog `(i*7)%128`), then for each step `NoteOn` at `step_onset_offset_ms(step_idx, grid) + offset_ms` and `NoteOff` at `on_ms + hold_ms` with **NO grid clamp** (main.rs:283-299), exactly mirroring render's prior `:366`/`:375` emission. It then `sort_by_key(|e| e.at_ms)` (stable, main.rs:316). `render_events_to_stereo` *also* sorts by `at_ms` (synth_sink.rs:370, stable) — so the builder's sort is idempotent w.r.t. render and cannot move render's bytes. **The render side is a faithful, behavior-preserving extraction.**

## Overlap / Ring-Through

**Preserved.** `NoteOff.at_ms = step_base + offset_ms + hold_ms` with no clamp to the grid (main.rs:292-299). A hold of 1.10–1.20×grid therefore rings past the step boundary. Because the list is global and sorted, a step-N tail `NoteOff` at `(N+1.10)*grid` and a step-(N+1) head `NoteOn` at `(N+1)*grid` interleave in true time order — which is precisely the cross-step overlap the old per-step drain serialized away. In the streaming play loop they now fire at their absolute `epoch + at_ms` times, so the ring-through sounds live exactly as render synthesizes it offline.

**Off-before-on at coincident timestamps — correct, with a nuance worth stating precisely.** The prompt's STAGE-2 phrasing implies an explicit command-rank tiebreak. The implementation does **not** use a rank key; it relies on the **stable** sort over insertion order (documented at main.rs:304-316, and the developer notes they verified a rank-keyed sort moved the WAV md5 while at_ms-only stable sort kept it byte-identical). The desired off-before-on at equal `at_ms` still holds for the load-bearing case: a same-(channel,note) release is inserted in an *earlier* step than its retrigger's `NoteOn`, so the stable sort keeps the off ahead of the on → voice freed, no retrigger collision. This is the correct and (importantly) byte-preserving choice. Test #2 asserts exactly this invariant and passes.

## Render-Output-Unchanged Assessment

**Assessed equivalent — structurally, and via the step-count/duration reference.** Deliberately NOT gated on WAV md5 (see next section). The extraction is a pure refactor: builder body == old inline emission, and render re-sorts identically. The diagnosis reference reproduces exactly:

```
cargo run -- render assets/images/AudioHaxImg2.jpg --wav /tmp/qg2.wav
→ render --wav: 36 steps, 4 instruments → /tmp/qg2.wav
→ render --wav: wrote /tmp/qg2.wav (25.0s, font=bundled, reverb=on, gain=1)
```

**36 steps / 25.0 s** — exact match to the design doc's reference. Repeated runs held step-count and duration stable (36/25.0 every time).

## thread_rng Non-Determinism Finding — real? does it matter for the ear-gate?

**Real, confirmed, and correctly bounded — it does NOT undermine `play == render`.**

- Empirically confirmed: two separate `render` invocations of the same image produced **different WAV md5s** (`45e21885…` vs `ed6708b2…`), while **step-count and duration stayed identical** (36 steps / 25.0 s both runs). So the non-determinism is in note CHOICE, not in macro structure/tempo/density.
- Source confirmed at the cited frozen-engine boundary: `engine.rs:378` documents `pick_progression` uses `thread_rng`; `chord_engine.rs:132` is `let mut rng = thread_rng();`. This is the documented S9 boundary, inside the byte-frozen engine — out of scope for this main.rs refactor and correctly left untouched.
- **Why it does not break the claim:** `play == render` is a claim about the *mechanism* — both paths build their timeline by calling the identical `build_step_event_timeline`, so within any one process the on/off timeline a consumer receives is identical-by-construction. The `thread_rng` draw happens once per process when the plan is built, *upstream* of the builder; the builder then deterministically lays that one fixed plan onto the grid (test #4 proves the builder is deterministic given a fixed engine/source). Two *separate* CLI invocations differ only because each rolls its own progression — the same way two separate `render` runs already differ. A single timeline still feeds both consumers identically.
- **Impact on the operator's A/B method:** the correct gate is the **structural/perceptual** one (step-count, duration, density, ring-through, tempo), NOT a byte/md5 comparison between a `play` session and a separately-rendered WAV — those will differ in note selection by design. The takeaway the operator should hold: live `play` is now trustworthy for hold/density/tempo judgments (the structural axes converged), but do not expect a live session to be the *same notes* as a separately-run WAV. For a strict same-notes A/B the operator would need both consumers driven from one process/one plan; that is beyond this fix's scope and is the inherent S9 engine boundary, not a defect introduced here.

## Ctrl-C / Sink Integrity

- **Single epoch:** captured once at main.rs:794 (`let epoch = Instant::now();`), before the stream. Every event fires at `epoch + at_ms` (main.rs:846). ✓
- **Sliced shutdown-polled sleep:** the wait loop (main.rs:847-857) sleeps `remaining.min(SHUTDOWN_POLL_SLICE)` with `SHUTDOWN_POLL_SLICE = 20ms`, checking the `shutdown` AtomicBool each slice → Ctrl-C breaks within ~one slice. There is a top-of-loop poll (main.rs:813) and an in-wait `break 'stream` (main.rs:853-854). ✓ Clean graceful return → `MidiOut::Drop` fires the all-sound-off panic.
- **Sink selection + program init intact:** `--output synth/midi/midi-virtual` selection is upstream and unchanged; the eager per-channel `program_change` init (main.rs:738-744) is preserved for the MIDI-port "ready" handshake, and the program changes are ALSO in the timeline at `at_ms=0` (harmless idempotent re-set). ✓
- **Jitter dropped:** live `hold_ms` jitter removed so the timeline is deterministic and the monitor equals the gate (main.rs:779-782). `jitter_percent` is still parsed/printed for CLI compat (`let _ = jitter_percent;`) and provably no longer perturbs timing — it is not referenced anywhere in the builder or the stream loop. ✓
- **OpenCV overlay:** re-derived from `at_ms / grid` at dispatch time (main.rs:821-826), cosmetic/non-audible, behind `#[cfg(feature="opencv")]`. Does not touch audio timing.

## Test Quality

The five tests use a real `PipelineEngine` + a synthetic `FeatureSource` driving the genuine `decide_step` realize path (not mocked event lists), so they exercise the actual builder.

- **#1 sorted-by-at_ms** — asserts non-decreasing windows over real output, and that output is non-empty. Strong.
- **#2 same-pitch-off-before-on** — asserts no same-(channel,note) `NoteOff` appears *after* a coincident `NoteOn`. This is the correct stuck-voice invariant. Strong, and matches the stable-sort design.
- **#3 cross-step-overlap-preserved** — asserts ≥1 `NoteOff` lands at a non-grid-multiple `at_ms` in slot ≥1. **Slightly weak as written**: it proves a hold ends mid-slot, which strongly implies (but does not by itself strictly prove) the parent onset was in an earlier slot. Combined with the no-clamp code review it is adequate. Non-blocking note below.
- **#4 deterministic-single-source-both-paths** — builds twice with identical inputs, asserts `a == b`. This is the load-bearing determinism proof (no RNG/jitter in the builder). It does not literally invoke render's and play's call sites, but code review confirms both call this exact fn, so determinism is the only remaining gap and it is closed. Honest framing in the test comment.
- **#5 program-change-at-zero** — asserts PC count == instrument count and first N events are PCs at `at_ms=0`. Strong.

Honest caveat (also flagged by the implementer): real-time audio is not headless-testable. The trust chain is code review + these timeline tests + operator re-listen of `render --wav` (and now, with confidence, live `play`).

## Freeze Verification

- `sha256sum src/engine.rs` → `e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261` — **MATCHES anchor. Engine untouched.** ✓ (re-checked after all cargo runs)
- `cargo test --test engine_equivalence` → **9/9** ✓
- `git diff --stat` → `src/main.rs` is the **only** modified source (492 insertions, 129 deletions). `engine.rs`, `chord_engine.rs`, `composition.rs` untouched. ✓
- **No new event type introduced** — the builder reuses `synth_sink::TimedMidiEvent` / `MidiCmd`; no `struct`/`enum` of those names is defined in `main.rs`. ✓

---

## Blocking Issues

**None.**

## Non-Blocking Issues

1. **Test #3 (`timeline_preserves_cross_step_overlap`) is a proxy.** It asserts a `NoteOff` lands at a non-grid-aligned `at_ms` in a later slot, which evidences a non-clamped hold but does not strictly bind the off to a parent onset in an *earlier* step. A stronger version would map each `NoteOff` to its originating `NoteOn`'s step (track open notes per channel/note) and assert `off.at_ms > (on_step + 1) * grid` for at least one note. Current form is acceptable given the no-clamp code is directly verified; suggest hardening opportunistically.
2. **STAGE-2 spec wording vs. implementation (documentation, not a defect).** The work-order/prompt phrasing suggests "same-pitch note_off precedes a coincident note_on" implies a command-rank tiebreak in the sort. The implementation correctly achieves the same effect via at_ms-only **stable** sort over insertion order (and deliberately avoids a rank key because a rank-keyed sort moved render's WAV bytes). The code comment (main.rs:304-316) documents this well; no action needed, noted so a future reader doesn't "fix" the sort into a rank key and silently change render output.
3. **`thread_rng` two-invocation note-choice divergence** (analyzed above) — not introduced by this fix, is the frozen-engine S9 boundary. Operator A/B should be structural/perceptual, not byte-equality between a live session and a separately-rendered WAV. Worth carrying as a known property in the operator's ear-gate notes.
4. **Pre-existing lint debt** in `modem.rs` / lib / unrelated bins is untouched by this fix and out of scope.

---

## One-Paragraph Summary

This is a clean, faithful PASS. The refactor extracts render's exact event-emission into `build_step_event_timeline` and rewires `play` from a per-step batched drain to a single streaming walk of that one sorted timeline, eliminating the structural batching that serialized cross-step overlaps and made live `play` sparse. `play == render` is genuinely achieved by construction at the timeline-mechanism level: both paths call the identical deterministic builder, holds ring past step boundaries with no grid clamp, the stable sort preserves off-before-on at coincident timestamps (byte-preserving for render), and live jitter is removed so the monitor equals the gate. The engine freeze holds (`e50c7db1…48261`, 9/9 equivalence), the diff is `main.rs`-only with no new event type, and all suites are green (180 lib / 14 bin incl. 5 new timeline tests / 9 engine-equivalence). The `thread_rng` non-determinism is real but correctly bounded — it varies note CHOICE across *separate* invocations while leaving macro structure stable (36 steps / 25.0 s for AudioHaxImg2 every run), so it does not undermine the by-construction equality and only means the operator's A/B must be structural/perceptual rather than md5-based.

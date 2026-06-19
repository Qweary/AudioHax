# S40 — Render-Path Divergence Diagnosis #2 (note HOLD / overlap, `play` vs `render --wav`)

**Status:** DIAGNOSIS + DESIGN ONLY (Rust Architect). No `src/`, `assets/`, or `tests/` edited.
**Engine freeze:** `src/engine.rs` sha256 anchor `e50c7db1…48261` — untouched; all recommended fixes land in `src/main.rs` only (§3 freeze-safety verdict).
**Builds on:** `docs/design-s40-path-divergence.md` (the PACING divergence — FIXED) and `docs/review-s40-pathfix.md` (QG PASS of that pacing fix). The pacing fix (single epoch + `step_onset_offset_ms`) IS in the working tree and IS correct. This document diagnoses the **SECOND, independent** divergence the operator heard on the re-listen: `play --output synth` is still sparse and its **notes do not ring as long as in `render --wav`**.
**Repro convention:** DEFAULT features (pure-Rust); main bin builds pure-Rust on this box.

---

## 0. TL;DR

- **The pacing fix is real and correct, and is NOT the remaining bug.** Step ONSETS now match render exactly (`epoch + N*grid`, shared `step_onset_offset_ms`). The remaining defect is in **note HOLD / cross-step overlap**, a different axis from pacing.
- **Root cause: `play` schedules and DRAINS events one step at a time, in a per-step-local `events` vector that is fully consumed before the loop advances to the next step (`main.rs:754-822`).** `render` accumulates **every** note-on/note-off from **every** step into ONE global event list and synthesizes them together (`main.rs:345-384`). The two are NOT the same on/off timeline whenever a note's `hold_ms` exceeds one grid step — which is the COMMON case, not an edge case.
- **Holds routinely exceed one grid step.** `chord_engine.rs` rings pads at `PAD_OVERLAP_FRAC = ARTIC_WINDOW_HI = 1.10 × step_ms` (`:1789`), cadences up to `1.20 × step_ms` (`:1562`), and calm-image legato near `1.10 × step_ms` (`:1540`, `:1552`). These are deliberate cross-step **overlaps** (the comments at `:1448`, `:1785-1791` say so explicitly).
- **In `render` those overlaps are HONORED**: a step-N note whose off-time is `N*grid + 1.10*grid` rings concurrently with step-(N+1)'s notes, because all events share one timeline fed to one offline synth pass. The output is fuller — notes ring their full length and tie into the next step.
- **In `play` those overlaps are SERIALIZED, not concurrent**: step N's inner `for sev in events` loop must reach step N's LAST event — the longest note_off at `≈ (N+1.10)*grid` — before the outer `for step_idx` loop can build step N+1's events at all. Step N's tail note_off therefore fires *before* step N+1's note_on, so the intended ring-through-into-next-step never sounds as overlap. Perceptually: each step's notes get cut at/around the step boundary and the next step starts after them → shorter notes + gaps → exactly the "sparse, notes don't ring as long" character the operator heard.
- **Net:** the pacing axis converged; the **hold/overlap axis did not.** `render` is the faithful, fuller output; `play` is still degraded. `render --wav` remains the safe canonical ear-gate. The robust fix is to finish the unification the pacing helper started: build ONE shared event timeline in both paths.

---

## 1. The exact remaining divergence — file:line evidence (the on/off timeline)

### 1a. How `render --wav` computes each note's OFF time (`main.rs:344-384`)

Render builds a SINGLE flat `events: Vec<TimedMidiEvent>` (`:345`) and, across the WHOLE `for step_idx in 0..total_steps` loop (`:358`), pushes every note's on/off into that one vector:

```
let step_base_ms = step_onset_offset_ms(step_idx, grid_ms_per_step);   // :363  absolute grid anchor
for dec in engine.decide_step(&source, step_idx) {
    for ev in &dec.events {
        let on_ms = step_base_ms + ev.offset_ms;                        // :366  NOTE-ON
        events.push(TimedMidiEvent { at_ms: on_ms,           NoteOn });  // :367-374
        events.push(TimedMidiEvent { at_ms: on_ms + ev.hold_ms, NoteOff }); // :375-381  NOTE-OFF = on + FULL hold
    }
}
...
let interleaved = render_events_to_stereo(font_src, synth_config, sample_rate, events, 1_500)?;  // :397  ALL events, ONE pass
```

- **OFF time = `step_idx*grid + offset_ms + hold_ms`, with NO grid/step-boundary clamp.** A note with `hold_ms = 1.10*grid` legitimately ends at `(N+1.10)*grid + offset` — i.e. **0.10*grid INTO the next step**, where step N+1's notes are also sounding. The single offline synth pass renders both simultaneously → true overlap, full ring. ✅ This is the faithful timeline.

### 1b. How `play` computes each note's OFF time (`main.rs:746-822`)

Post-pacing-fix, `play` builds a FRESH, STEP-LOCAL `events: Vec<ScheduledEvent>` *inside* the `for step_idx` loop (`:754`), schedules this step's on/off into it, sorts it, and **drains it to completion** before the outer loop iterates:

```
for step_idx in 0..total_steps {                                        // :724  OUTER
    let decisions = engine.decide_step(&source, step_idx);              // :747
    let mut events: Vec<ScheduledEvent> = Vec::new();                   // :754  *** PER-STEP, RE-CREATED EACH STEP ***
    let t0 = epoch + Duration::from_millis(step_onset_offset_ms(step_idx, grid_ms_per_step)); // :755  correct onset anchor
    for dec in &decisions {
        for ev in &dec.events {
            let hold_ms_f = (base_hold * (1.0 + jitter)).max(8.0)...;   // :765  realized hold (jittered)
            let start_instant = t0 + Duration::from_millis(ev.offset_ms);   // :767  NOTE-ON
            events.push(ScheduledEvent { at: start_instant, on:true, ... }); // :768-774
            events.push(ScheduledEvent { at: start_instant + Duration::from_millis(hold_ms_f), on:false, ... }); // :775-781  OFF = on + hold (CORRECT value...)
        }
    }
    events.sort_by_key(|e| e.at);                                       // :786
    for sev in events {                                                 // :787  *** DRAIN THIS STEP FULLY ***
        ... sleep until sev.at ... fire note_on/note_off ...            // :800-820
    }
}                                                                       // :822  only NOW advance to step N+1
```

- The note_off `at` VALUE is computed correctly (`start_instant + hold_ms_f`, `:776`) — so the bug is **not** a wrong off-time arithmetic. The defect is **structural batching**: that note_off lives in step N's local vector, and the `for sev in events` drain (`:787`) runs to step N's last event before the outer loop builds step N+1's events.
- **Consequence — overlap is serialized, not concurrent.** When a step-N note has `hold_ms ≥ grid`, its note_off `at = (N + 1.10)*grid` is LATER than step N+1's onset `(N+1)*grid`. But step N+1's note_on is not even created until step N's drain finishes. So the runtime order is: …step N notes on → **step N tail note_off fires (at ≈(N+1.10)*grid)** → step N+1 note_on fires. The note that was supposed to *ring through* into step N+1 is instead turned OFF before step N+1 begins. The two steps never sound together.
- **Two audible symptoms, one cause:**
  1. **Notes don't ring as long.** A pad meant to tie 1.10*grid into the next step is heard as a note that ends, then the next step starts — its ring-through is amputated by the serialized drain.
  2. **Sparse / gaps.** Because step N+1's onset is deferred until step N's longest note_off has fired, and because the intended simultaneity (tail of N + head of N+1) is collapsed into sequence, the texture thins exactly where render is densest (the legato/pad ties).

### 1c. Realize-layer parity (investigation item #3) — CONFIRMED identical

Both paths read the SAME per-note hold from the SAME realize call: each calls `engine.decide_step(&source, step_idx)` (render `:364`, play `:747`) → same `NoteEvent { offset_ms, hold_ms, note, velocity }` (`chord_engine.rs:879-890`). The `hold_ms` VALUES are identical. The divergence is **purely** in main.rs's translation of that hold into a global-vs-per-step-batched on/off timeline — NOT in the engine. (One legitimate, sub-dominant difference: `play` applies live `jitter_percent` to `hold_ms` at `:765`; render does not — `:376` uses raw `hold_ms`. Jitter perturbs hold by a few percent but does NOT cause the structural cut; the batching does. Jitter should stay — it is intended live behavior — but note it is a second, minor source of play≠render that the shared-timeline option can optionally fold in or deliberately keep.)

### 1d. Synth-sink parity (investigation item #4) — NOT the cause

Both paths use rustysynth with `font = bundled`, reverb/chorus per the `--reverb` flag (offline `render_events_to_stereo`, `:397`; live `SynthSink` via `--output synth`). The sink does NOT re-derive, cap, or early-release voices on step advance — there is no `all_notes_off`/all-sound-off in the per-step loop (grep: the only all-sound-off is the sink's `Drop` at process end, not per step). The synth honors whatever note_on/note_off it is handed. Since `play` hands it serialized (cut-early) offs, the audible truncation is upstream in the main.rs batching, exactly as the work order's hypothesis #4 predicted ("suspect the main.rs note_off timing first, the sink second"). **Confirmed: main.rs is the defect; the sink is faithful.**

### 1e. The sliced-sleep / Ctrl-C loop (investigation item #5) — does NOT add an early note_off, but it CANNOT fix the batching

The `≤20ms` sliced sleep (`:799-810`) only governs *how* the loop waits until each `sev.at`; it fires each event at its scheduled time and adds no extra note_off and no step-boundary flush. So the Ctrl-C fix is clean. **However, it operates entirely INSIDE the per-step `for sev in events` drain — it cannot make step N+1's events coexist with step N's tail, because step N+1's events do not exist yet.** The slicing is correct but orthogonal; it neither caused nor can cure the hold/overlap divergence.

### 1f. Why diagnosis #1 missed this (and why it's a SEPARATE bug)

Diagnosis #1 correctly identified that pre-fix `play` reset `t0 = Instant::now()` every step and had no grid anchor (emergent tempo, rushed ~1.8×). The fix anchored `t0` on the absolute grid — which fixes ONSET pacing. But the fix kept the **per-step event batching** structure (`events` re-created and drained inside the loop). Onset-anchoring makes step starts correct; it does nothing for cross-step note RING-THROUGH, because that requires step N's tail and step N+1's head to be in the SAME schedulable timeline. They are not. This is genuinely a second divergence, on the hold/overlap axis, exposed only after pacing was fixed.

---

## 2. Root cause (single sentence)

**`play` plays the composition step-by-step, draining each step's note-on/note-off events to completion before constructing the next step's — so any note whose `hold_ms` exceeds one grid step (the common pad/cadence/legato case, `1.10–1.20 × step_ms`) has its ring-through into the next step serialized away (its note_off fires before the next step's note_on), whereas `render` puts all steps' events on one shared timeline and synthesizes them together, honoring the overlap; the result is `play`'s notes are cut short and the texture goes sparse exactly where `render` ties notes across the grid.**

Secondary (minor, intentional, not the cut): `play` jitters `hold_ms`, `render` does not (`:765` vs `:376`).

| Axis | Diagnosis #1 (FIXED) | Diagnosis #2 (THIS DOC) |
|---|---|---|
| Step ONSET timing | per-step `Instant::now()` reset → emergent rush | **FIXED** — `epoch + N*grid` shared formula |
| Note HOLD / cross-step overlap | (masked by the rush) | **OPEN** — per-step batched drain serializes overlaps → notes cut short, sparse |

---

## 3. RECOMMENDED FIX — work order (files, functions, signatures; NO bodies)

**Freeze-safety verdict: SAFE — `main.rs`-only.** The note decisions (`decide_step`/`realize_step`/`chord_engine`, `engine.rs` sha `e50c7db1…48261`) already emit correct `hold_ms`; the OFF-time values are already correct in both paths. The fix changes only HOW main.rs schedules/streams the already-correct events. No `engine.rs`, `chord_engine.rs`, or `composition.rs` edit. Mirrors the §4 verdict of diagnosis #1.

### Option A — ROBUST (RECOMMENDED, my pick): one shared event timeline, built once, for both paths

This finishes the unification the pacing helper started: instead of sharing only the onset *formula*, share the entire ordered event *list*, so the two paths are identical-by-construction (no future axis can silently diverge).

**WO-1 — Extract a shared timeline builder (`main.rs`, new free fn).**
- New function, signature only:
  `fn build_step_event_timeline(engine: &PipelineEngine, source: &PureAnalysisSource, total_steps: usize, grid_ms_per_step: u64, instrument_count: usize) -> Vec<TimedMidiEvent>`
- Body (not written here) = exactly render's current `:345-384` accumulation loop: initial per-channel `ProgramChange`s, then for each step push `NoteOn` at `step_onset_offset_ms(step_idx, grid) + offset_ms` and `NoteOff` at `on_ms + hold_ms` into ONE `Vec`, then `sort_by_key(at_ms)`. (Sorting the global list is what lets a step-N tail and a step-(N+1) head interleave correctly.)
- Decide jitter policy explicitly: either (a) keep the timeline jitter-free (then `play` becomes byte-deterministic w.r.t. render — cleanest, and the live monitor exactly equals the gate), or (b) add an optional `jitter_percent: f32` + `&mut impl Rng` parameter applied to `hold_ms` so live keeps its feel. **Recommend (a)** for a trustworthy monitor; jitter can be re-added later as a deliberate, documented divergence if wanted.

**WO-2 — `render --wav` consumes the shared builder (`run_render_wav`, `main.rs:344-384`).**
- Replace the inline `:345-384` accumulation with a single call to `build_step_event_timeline(...)`. No signature change to `run_render_wav`. Net behavior byte-identical to today (this is a pure refactor on the render side — its QG-passed output is the reference and must not move).

**WO-3 — `play` consumes the shared builder and STREAMS the global timeline (the play loop, `main.rs:724-822`).**
- Build the timeline ONCE before the loop: `let timeline = build_step_event_timeline(...);` (already sorted, absolute `at_ms` from the same `epoch` semantics).
- Replace the per-step `for step_idx { build local events; drain }` (`:724-822`) with a SINGLE streaming loop over `timeline`: for each `TimedMidiEvent`, sleep (in `≤20ms` shutdown-polled slices, reusing the existing `SHUTDOWN_POLL_SLICE` logic at `:799-810`) until `epoch + Duration::from_millis(ev.at_ms)`, then dispatch via the `AudioSink` trait (`note_on`/`note_off`/`program_change`). Because the loop now walks ONE ordered list, a step-N note_off at `(N+1.10)*grid` and a step-(N+1) note_on at `(N+1)*grid` interleave in true time order → the overlap rings exactly as render synthesizes it.
- Keep: the single `epoch` (already at `:722`), the SIGINT poll between slices and at top-of-loop, the program-change init (`:686-692`) — or fold the program-changes into the timeline per WO-1.
- Drop: the per-step local `events` vector (`:754`), the per-step `t0` (`:755` — `step_onset_offset_ms` now lives inside the builder), and the inner per-step `for sev in events` drain (`:787`). The OpenCV overlay (`:733-744`, opencv-only) can be re-driven by deriving `step_idx` from `at_ms / grid_ms_per_step` at dispatch time, or left on a parallel lightweight step tick; flag for the implementer (overlay is cosmetic, non-audible).

**WO-4 — verification guard (test-only, DEFAULT features per S11).**
- A `--test` target that calls `build_step_event_timeline` for a fixed image and asserts the produced `(at_ms, on/off, channel, note)` stream is identical whether reached via the render entry or the play entry (compare the lists; no audio device, no real-time sleep). With the jitter-free choice (WO-1a) this is an exact-equality test and locks play==render by construction. This upgrades diagnosis-#1's WO-4 from "same onset formula" to "same full timeline."

### Option B — MINIMAL (localized; closes the cut, weaker invariant)

If a full unification is undesired this cycle, the smallest change that restores cross-step overlap is to stop draining per-step and instead carry outstanding note_offs forward:

**WO-B1 (`main.rs` play loop, `:724-822`).** Hoist a SINGLE `events`/pending-off accumulator OUTSIDE the `for step_idx` loop. Each step pushes its on/off into the shared accumulator (offs at `epoch + N*grid + offset + hold`, possibly `>(N+1)*grid`), and a single time-ordered streaming dispatch (re-using the sliced-sleep) consumes them globally rather than per step. In effect this IS the global timeline, just built incrementally — which is why **Option A is preferred**: same outcome, but A also dedups render's builder and gives the exact-equality test. Option B leaves two scheduling code paths to keep in sync (the recurrence that produced this very bug).

### Pick

**Option A (shared timeline).** The pacing fix already proved the unification pattern (one shared `step_onset_offset_ms`); this bug exists precisely because only the onset, not the whole event list, was shared. Finishing the unification makes `play == render` *by construction*, eliminates the entire class of "Nth axis diverges" recurrences, and is still strictly `main.rs`-only. Option B is acceptable as a faster stopgap but re-introduces the dual-path drift that caused both S40 divergences.

**Freeze verdict (restated): SAFE — `main.rs`-only, expected.** Engine, chord_engine, composition all untouched; `hold_ms` values are already correct. Re-verify `sha256sum src/engine.rs == e50c7db1…48261` and `cargo test --test engine_equivalence` (9/9) after the change, per the established gate.

---

## 4. Does this finally make `play == render`? + interim ear-gate

- **After Option A: yes, `play` becomes the same composition as `render --wav` by construction** (same ordered event timeline; with WO-1a, identical to render up to the offline 1.5 s reverb tail and the live-vs-offline synth realization — the *event timeline* is provably equal). The real-time monitor becomes trustworthy: what you hear live is what the WAV gate certifies. (If jitter is kept via WO-1b, `play` equals render up to a few-percent per-note hold jitter — still faithful in tempo, density, and ring; the structural cut is gone either way.)
- **After Option B: yes for the audible overlap** (notes ring full length, gaps close), but the equality is behavioral, not by-construction, so it is one refactor away from drifting again.
- **Interim (until the fix lands): `render --wav` REMAINS the safe canonical ear-gate.** It already honors full holds and cross-step overlap on one timeline (`:345-397`) and is deterministic. Do NOT trust live `play` for density/hold judgments until WO-1..WO-3 land. Canonical command unchanged from diagnosis #1 §5:
  ```
  cargo run -- render <IMAGE> --wav /tmp/eargate.wav   # then play /tmp/eargate.wav in any player
  ```
  Reference image for this arc: `cargo run -- render assets/images/AudioHaxImg2.jpg --wav /tmp/img2.wav` → 36 steps, 25.0 s (font=bundled, reverb=on, gain=1), grid ≈ 653 ms/step.

---

## 5. Summary table

| Item | Finding |
|---|---|
| Remaining divergence | `play` drains note events per-step (`main.rs:754,786-787`); `render` puts all steps' events on one sorted global timeline (`main.rs:345-384`). |
| On/off evidence | render OFF = `N*grid + offset + hold` on one timeline, overlap honored (`:366`,`:376`,`:397`). play OFF value is correct (`:776`) but lives in a per-step vector drained before step N+1 is built (`:787` then loop iterates `:822`) → tail note_off fires before next step's note_on → no concurrent ring-through. |
| Root cause | per-step event batching serializes the deliberate `1.10–1.20×step_ms` cross-step holds (pads `:1789`, cadences `:1562`, legato `:1540/:1552`) → notes cut short + sparse. Engine/holds correct; main.rs scheduling wrong. |
| Sink | faithful; no per-step all-notes-off; not the cause (item #4 confirmed). |
| Sliced-sleep | clean; adds no early off, but can't fix batching (item #5 confirmed). |
| Minimal fix | global accumulator outside the step loop (WO-B1) — closes the cut, leaves dual paths. |
| Robust fix (PICK) | shared `build_step_event_timeline` consumed by both paths + streaming play loop (WO-1..WO-4) — `play==render` by construction. |
| Freeze | SAFE, `main.rs`-only; engine.rs sha must stay `e50c7db1…48261`. |
| play == render? | Yes after Option A (by construction). |
| Interim gate | `render --wav` remains the safe canonical ear-gate; do not trust live `play` for hold/density until the fix lands. |

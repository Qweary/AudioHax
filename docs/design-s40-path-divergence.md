# S40 — Render-Path Divergence Diagnosis (`play` vs `render --wav`)

**Status:** DIAGNOSIS + DESIGN ONLY (Rust Architect). No `src/`, `assets/`, or `tests/` edited.
**Engine freeze:** `src/engine.rs` sha256 verified `e50c7db1…48261` (frozen, untouched). All recommended fixes land in `src/main.rs` only — see §4 freeze-safety verdict.
**Repro build convention:** DEFAULT features (pure-Rust); main bin builds pure-Rust on this box.

---

## 0. TL;DR

- **The leading "wrong tempo" hypothesis is REFUTED in its literal form but CONFIRMED in spirit.** The `250 ms/step` the operator saw is a **scan-visualization print, not a playback clock** — it never times note playback on either path. So `play` does NOT literally play at 240 BPM.
- **The REAL divergence is structural: the two paths use two different step-timing models.**
  - `render --wav` lays steps on an **absolute, gap-preserving ms grid** at `step_idx * plan.key_tempo.base_ms_per_step` (`main.rs:348`). Every step occupies its FULL `base_ms_per_step` slot whether or not a note fills it.
  - `play` uses a **block-until-last-event live loop with NO inter-step pacing** (`main.rs:690-768`): each step resets `t0 = Instant::now()` and advances as soon as that step's *longest scheduled note* ends. There is no `sleep` of `base_ms_per_step` between steps and no `step_idx * base_ms_per_step` base offset anywhere in the loop.
- **Net effect:** `play`'s effective tempo is an *emergent property of the longest note per step*, not the plan tempo. It happens to land *near* plan tempo ONLY when some voice holds ≈ a full step (the calm-image case). On busier images — or any step where the longest voice is short or rests — `play` **rushes and eats the rests**, producing exactly the "sparse / rushed / wrong" character the operator heard, while `render --wav` of the *same image* preserves every slot.
- **Note DECISIONS are identical across both paths** (both call `engine.decide_step`, which passes the plan's `section.ms_per_step` into the realizer — `engine.rs:565`). The divergence is 100% in the **main.rs scheduler**, not in the engine.
- **The "sparse" complaint (Finding #3 / Slice-3b FREEZE-BREAK) is plausibly — at least partly — a `play`-path artifact.** Every past ear-gate done via `play` judged a tempo/rest-corrupted rendering. A clean re-listen via `render --wav` is required BEFORE committing to a density freeze-break.

---

## 1. The Divergence(s) Found — file:line evidence + clean same-image numbers

### Clean same-image numbers (the confound removed)

Both `render --wav` runs use DEFAULT features, deterministic offline synth (no jitter, no wall clock):

| Image | `render` plan steps | WAV duration | minus 1.5 s tail | **render ms/step (plan tempo)** | `play` printed "ms/step" |
|---|---|---|---|---|---|
| `AudioHaxImg1.jpg` | 30 | 28.86 s | ~27.36 s | **~912 ms/step** (~66 BPM) | 250 (scan-viz only) |
| `example.jpg` | 42 | 27.81 s | ~26.31 s | **~626 ms/step** (~96 BPM) | (n/a — different image) |

(Render adds a fixed **1.5 s reverb tail**, `main.rs:382`, so subtract it before dividing.)

**Reading the numbers:** `render(AudioHaxImg1)` ≈ **912 ms/step**, which is ~3.6× SLOWER than the `250` the operator read off the `play` console. That looked like a smoking gun for the "play runs at the wrong fast tempo" hypothesis. **It is not** — because `250` is not `play`'s playback clock (see Divergence A). `912 ms/step` is the genuine plan tempo for a dark image (brightness 29 → slow), and it is consistent with `render`. The two images differing (912 vs 626 ms/step) is EXPECTED — different brightness → different `brightness_to_tempo_bpm` → different `base_ms_per_step` (`composition.rs:1449-1453`). That part of the operator's A/B was pure confound.

### Divergence A — the `250 ms/step` print is scan-viz, not the playback clock (kills the literal hypothesis)

`main.rs:474-477`:
```
"Scan bar thickness = {:.2}, steps = {}, ms/step = {}, jitter% = {}"
   bar_thickness_frac, num_steps, ms_per_step, jitter_percent
```
where `ms_per_step = engine_config.ms_per_step` (`main.rs:470`) — the **CLI/config default** (`EngineConfig::default().ms_per_step = 250`, `engine.rs:204`; `steps` default `40`, `cli.rs:97`).

This value is printed for human/scan-overlay context. **It is never read by the `play` driver loop** (`main.rs:690-768`). Grep confirms: inside that loop, `ms_per_step`/`grid_ms_per_step` appear ZERO times. So `play` does not play at 250 ms/step. (Likewise the "steps = 40" print is the scan-grid granularity; both paths then OVERRIDE `total_steps` to `plan.total_steps = 30` — `main.rs:684` for play, `:318` for render — so both actually loop 30 plan steps. The operator's "Completed playback of 30 steps" was the NATURAL end of the 30-step plan, NOT an early Ctrl-C truncation. The Ctrl-C landed at/after the plan's real end.)

### Divergence B (ROOT) — two different step-timing models

**`render --wav` — absolute, gap-preserving grid** (`main.rs:316-369`):
```
let (total_steps, grid_ms_per_step) = match engine.composition() {
    Some(plan) => (plan.total_steps, plan.key_tempo.base_ms_per_step),   // :319
    None       => (source.step_count(), ms_per_step),                     // :320 legacy
};
...
let step_base_ms = step_idx as u64 * grid_ms_per_step;                     // :348
... on_ms = step_base_ms + ev.offset_ms;                                   // :351
```
Every step is anchored at `step_idx * base_ms_per_step`. A step whose notes end early still occupies its full slot — silence is preserved, the grid never compresses. **Render honors `plan.key_tempo.base_ms_per_step` exactly.** ✅

**`play` — block-until-last-event, NO inter-step pacing** (`main.rs:690-768`):
```
for step_idx in 0..total_steps {
    ...
    let decisions = engine.decide_step(&source, step_idx);   // :713
    let t0 = Instant::now();                                  // :717  RESET EVERY STEP
    for dec in &decisions {
        for ev in &dec.events {
            ... start_instant = t0 + Duration::from_millis(ev.offset_ms);  // :729
            ... at: start_instant + Duration::from_millis(hold_ms_f) ...   // :738
        }
    }
    events.sort_by_key(|e| e.at);                             // :748
    for sev in events { ... sleep(sev.at - now); ... }        // :756-758  blocks to last event ONLY
}
```
There is **no `sleep(base_ms_per_step)` between steps** and **no `step_idx * base_ms_per_step` anchor**. The loop blocks only until this step's last scheduled `note_off`, then immediately starts the next step at a fresh `t0`. So:

> **play's effective per-step duration = max over the step's events of (offset_ms + jittered hold_ms).**

The code comment at `main.rs:679-682` asserts "the plan's tempo is already honored inside each decision … only the total_steps swap is needed." **This is only true when the longest note in the step happens to span the whole `step_ms`.** It is NOT generally true:

- The intra-step timing IS plan-scaled — `decide_step` passes `section.ms_per_step` (the plan tempo) to the realizer (`engine.rs:558-567`), so `ev.offset_ms`/`ev.hold_ms` are sized to `step_ms`. ✅ for *within-step* spacing.
- BUT the **inter-step gap is whatever's left after the longest note** — and that is governed by the articulation curve, not the plan grid. Per `chord_engine.rs:1538-1556`, `base_frac` ∈ **[0.55, 1.10]** of `step_ms` (calm image → ~1.10, busy image → ~0.55), and the HarmonicFill voice can **rest entirely** on weak interior beats (`chord_engine.rs:1710-1712`). So on a busy/active image, the longest voice may hold only ~0.55 × `step_ms` (or, on a rest step, even less), and `play` advances at ~55% of plan tempo per step — **rushing ~1.8×** — while collapsing rest-steps to near-zero. The deliberate `PAD_OVERLAP_FRAC = 1.10` "block over-runs each step by ≤10%" comment (`chord_engine.rs:1785-1791`) is an explicit acknowledgement that the live scheduler's pacing is being *propped up* by note length — which only works when a near-full-step voice is present.

**For `AudioHaxImg1` (calm/dark: brightness 29, edge_density 0.015):** `edge_activity` is low → `base_frac` ≈ near 1.10, and HarmonicFill mostly does NOT rest. So `play` of THIS image runs at ≈ 1.0–1.10 × 912 ms/step ≈ near plan tempo by luck. **This is why a clean same-image A/B on AudioHaxImg1 alone would look "fine"** — the bug is masked on calm images. It bites on busier images and on any step with short/resting voices.

---

## 2. Root cause per divergence

| # | Divergence | Root cause |
|---|---|---|
| A | `play` console shows `250 ms/step` | It is `engine_config.ms_per_step` (the CLI/config DEFAULT, `engine.rs:204`) printed for scan-overlay context (`main.rs:474-476`). It is dead w.r.t. playback timing. Cosmetic/misleading, not a timing bug. |
| B | `play` and `render` time steps differently | **`play` has no grid clock.** `main.rs:690-768` resets `t0` per step and blocks only to the last note (`:717`, `:756-758`); it never sleeps `base_ms_per_step` between steps. `render` anchors each step at `step_idx * base_ms_per_step` (`:348`). So `play`'s tempo = emergent-from-note-length; `render`'s tempo = plan grid. |
| C (note) | "40 steps" vs "30 steps" | `40` = scan-grid `--steps` default (`cli.rs:97`); `30` = `plan.total_steps`. BOTH paths loop the plan's 30 (`:684`, `:318`). Not a real divergence — and the operator's run was NOT truncated early. |

**Composition-active parity (investigation item #3):** ✅ no divergence. Both paths take the SAME `match engine.composition()` and the same `Some`/`None` semantics: `render` `:318-321`, `play` `:684-687`. `compose_from_image` is called identically before each (`:306` / `:570`), and `assets/mappings.json` either has a `composition` block (→ both `Some`) or doesn't (→ both `None`). Neither path can hit `None` while the other hits `Some` for the same image+mappings. The `None` fallbacks differ only cosmetically (render falls back to a deterministic grid at config `ms_per_step`; play falls back to `source.step_count()` with the same block-until-event loop) — irrelevant here because the audible path is the `Some`/composed path on both.

**Note-decision parity (item #4):** ✅ identical. Both call `engine.decide_step(&source, step_idx)` (render `:349`, play `:713`) → same `realize_step`/`chord_engine` seam → same `NoteEvent`s. `docs/midi-routing.md:11` confirms: "AudioHax emits the same NoteEvents either way."

**`--output synth` specifics (item #5):** the in-process rustysynth+cpal sink and the offline render both consume the same `NoteEvent`s and the same bundled SoundFont with reverb/chorus on (`docs/midi-routing.md:3-4`). The synth path does NOT re-derive or shorten notes — but it IS subject to (a) the Divergence-B timing model above, (b) live `jitter_percent` on hold (`main.rs:721-727`, absent in render), and (c) real-time cpal under-runs (audible glitches but not the structural sparseness). The **timing model (B) is the dominant cause**, not the synth engine.

---

## 3. Is "sparse" a path artifact or a real engine property?

**Verdict: plausibly a `play`-PATH artifact in significant part — must be re-confirmed via `render --wav` before any density freeze-break.**

Reasoning:
- Every past ear-gate was done with `play` (the only path that makes live sound). `play` rushes and eats rests on any image where the longest per-step voice is short or rests (§1 Divergence B). "Rushed + dropped inner voices + collapsed rests" perceptually **reads as sparse/thin/ethereal** — which is exactly the complaint that opened this arc.
- The articulation/rest mechanics that drive the collapse (`base_frac` → 0.55 on busy images; HarmonicFill rest-as-gesture on weak interior beats, `chord_engine.rs:1710`) are *intended* musical behavior whose audible result is **distorted by the block-until-event scheduler** — under the correct gap-preserving grid those same decisions sound fuller because the slot time is preserved and the rest is heard as breathing space, not as acceleration.
- `render --wav` of `AudioHaxImg1` is 28.86 s of continuous, plan-paced audio over 30 steps — materially denser-in-time than the rushed `play` rendering.

**Therefore:** before spending the Slice-3b FREEZE-BREAK on density (which would risk `engine.rs`/`chord_engine.rs`), the operator should **re-judge density on the `render --wav` artifact**. If the rendered version sounds adequately dense, **the freeze-break is likely avoidable** — the perceived sparseness was a scheduler artifact, fixable entirely in `main.rs` (§4) with zero engine risk. If it STILL sounds sparse under faithful rendering, then Finding #3 is a genuine engine property and the freeze-break is justified — but at least it will be judged on real evidence, not a corrupted ear-gate.

This is high-leverage: it converts a risky engine freeze-break into a possibly-unnecessary one, gated behind a 10-second re-listen.

---

## 4. RECOMMENDED FIX — converge both paths onto one trustworthy timing model

**Goal:** make `play` honor the same absolute, gap-preserving grid `render --wav` uses, so the live ear-gate faithfully reproduces the composition (and matches the deterministic WAV up to jitter).

**Freeze-safety verdict: SAFE.** The entire fix lives in `src/main.rs` — specifically the `play` driver loop (`run_play` / the `Command::Play` body, ~`main.rs:690-768`). It does NOT touch `src/engine.rs` (frozen sha `e50c7db1…48261` — `decide_step`, `realize_step`, `chord_engine` all stay byte-identical) and does NOT touch `src/chord_engine.rs` or `src/composition.rs`. The note decisions are already correct; only the scheduler that consumes them changes.

### Work order (files, functions, signatures — NO bodies)

**WO-1 — Bind the plan grid in the `play` loop (REQUIRED).**
- File: `src/main.rs`, the `play` driver (the `Command::Play` arm / its `run_play` helper, ~`:690`).
- Add a plan-grid read mirroring render's `:318-321`:
  - `let grid_ms_per_step: u64 = match engine.composition() { Some(plan) => plan.key_tempo.base_ms_per_step, None => ms_per_step };`
  - (`total_steps` binding already exists at `:684` — keep it.)
- Anchor each step on the absolute grid instead of a fresh `Instant::now()`:
  - Capture ONE run epoch before the loop: `let epoch = Instant::now();`
  - Replace the per-step `let t0 = Instant::now();` (`:717`) with `let t0 = epoch + Duration::from_millis(step_idx as u64 * grid_ms_per_step);`
- This makes step N start at exactly `epoch + N * base_ms_per_step`, so steps whose notes end early still occupy their full slot (rest/breathing preserved), and the loop's `sleep(sev.at - now)` (`:757-758`) naturally idles through the gap. The within-step `ev.offset_ms`/`hold_ms` scheduling is unchanged.
- Keep the SIGINT graceful-stop poll (`:693`, `:753`) and the jitter (`:721-727`) as-is. Jitter on hold is fine under a fixed grid because the grid (not the note length) now sets step cadence.
- No signature changes required; `engine.decide_step` is called exactly as before.

**WO-2 — De-confuse the console prints (RECOMMENDED, cosmetic).**
- File: `src/main.rs`, `:474-476`.
- The `play` header should print the PLAN tempo + plan step count when composing, not the scan-viz `ms_per_step`/`--steps`. Either (a) print `grid_ms_per_step` and `total_steps` alongside the scan values with clear labels (e.g. `scan-viz ms/step` vs `playback ms/step`), or (b) move the existing print below the `engine.composition()` resolution and substitute plan values. This removes the `250` red herring that triggered the false hypothesis.

**WO-3 — (Optional) factor the shared grid math.**
- If desired, extract the `step_base_ms = step_idx * grid_ms_per_step` anchoring into a tiny free function in `main.rs` (e.g. `fn step_anchor_ms(step_idx: usize, grid_ms_per_step: u64) -> u64`) shared by both `run_render_wav` (`:348`) and the play loop, so the two paths provably use one formula. Pure refactor, `main.rs`-only.

**WO-4 — (Optional) verification guard.**
- A `--test`-target (DEFAULT features, per S11 convention) asserting that for a fixed image the `play`-scheduler's step-start times equal `render`'s `step_idx * base_ms_per_step` grid (compare the computed `at_ms` streams without real-time playback / without opening an audio device). This locks the convergence and prevents regression. Test-only; no `src` impact beyond what WO-1 adds.

**What is explicitly OUT of scope / forbidden:** any edit to `engine.rs`, `chord_engine.rs`, or the realizer seam. If a future investigation finds density genuinely too sparse AFTER WO-1 (judged on `render --wav` per §3), THAT is the only thing that should trigger a Slice-3b engine freeze-break — and it should be re-scoped then, separately.

---

## 5. Canonical re-listen command for ALL future ear-gates

Use the **deterministic, gap-preserving render** path (and play the WAV in any audio player) — this is the path that faithfully reproduces the composition at plan tempo with no scheduler distortion:

```
cargo run -- render <IMAGE> --wav /tmp/eargate.wav   # then play /tmp/eargate.wav in any player
```

Concretely for the arc's reference image:

```
cargo run -- render assets/images/AudioHaxImg1.jpg --wav /tmp/eargate.wav && (xdg-open /tmp/eargate.wav 2>/dev/null || aplay /tmp/eargate.wav)
```

(`tools/ab-render.sh <IMAGE> [SOUNDFONT.sf2]` renders several A/B configs at once — `docs/midi-routing.md:16`.)

**Until WO-1 lands, do NOT use `play` for ear-gates** — it judges a tempo/rest-distorted rendering. After WO-1 lands, `play` becomes trustworthy and can be used for live monitoring, but `render --wav` remains the deterministic gate of record.

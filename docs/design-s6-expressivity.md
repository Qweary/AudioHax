# S6 — Expressivity Layer: Design, Decisions & Wiring Spec (Pass A)

**Status:** Pass A — API surface defined as compiling naive stubs; this doc is
the design + wiring contract. Pass B implements the real expressive behavior
behind the SAME signatures.

**Owner of `chord_engine.rs`:** Music Theory Specialist (this lane).
**Owner of `main.rs`:** Rust Implementer (executes the Wiring Spec, §4, mechanically — no musical decisions).

All MIDI/music logic lives in `chord_engine.rs`. `main.rs` is a thin adapter:
extract scalars → look up the `StepPlan` → call the pure `realize_step` → map
the returned `NoteEvent`s into its existing event tuples. Nothing musical is
decided in `main.rs`.

---

## 1. The seam (what Pass A added to `chord_engine.rs`)

```rust
pub enum OrchestralRole { Bass, HarmonicFill, Melody }

pub struct PerfFeatures {
    pub saturation: f32,   // 0..=100  -> dynamic LEVEL
    pub brightness: f32,   // 0..=100  -> REGISTER (octave)
    pub edge_density: f32, // 0..=1    -> RHYTHMIC ACTIVITY
}

pub struct NoteEvent {
    pub note: u8,
    pub velocity: u8,
    pub hold_ms: u64,
    pub offset_ms: u64,
}

pub fn instrument_role(inst_idx: usize, num_instruments: usize) -> OrchestralRole;

pub fn realize_step(
    step: &StepPlan,
    inst_idx: usize,
    num_instruments: usize,
    features: &PerfFeatures,
    ms_per_step: u64,
) -> Vec<NoteEvent>;

pub const MIN_UPPER_VOICE_SPACING: u8 = 1;
pub fn upper_voices_well_spaced(notes: &[u8]) -> bool;
```

`NoteEvent` maps 1:1 onto `main.rs`'s existing `(note, vel, hold_ms, offset_ms)`
tuple. `PerfFeatures` is the music-domain projection of `ScanBarFeatures` — no
OpenCV/image type crosses into the lib crate, so the whole layer is
headless-testable.

**Pass-A stub behavior (so the property net goes RED on behavior, not symbols):**
- `realize_step` returns ONE flat note at `step.velocity`, hold = 90% of step,
  offset 0 — ignoring role, `features`, rhythm, articulation, harmonic rhythm.
- `instrument_role` is FINAL (real, not stubbed): the scheme below is fixed; only
  per-role *realization* is deferred.
- `upper_voices_well_spaced` returns `true` unconditionally (stub) so a test that
  feeds it the IV=[65,65,65] collapse goes red.

---

## 2. The four expressive dimensions (Pass B intent — what each must DO)

### 2.1 DYNAMICS — phrase-aware contour ON the structural floor
`StepPlan.velocity` is the structural FLOOR (76 interior / 88 start / 96 cadence).
Pass B SHAPES it; it does not replace it.
- **Overall LEVEL** comes from `features.saturation`: vivid color = bolder. Map
  saturation 0..=100 to a gain in roughly **[-12, +18] velocity** added to the
  floor (so a dull bar still respects the floor's phrase marking, a vivid bar
  pushes louder), then clamp to 1..=127.
- **Messa di voce within a phrase:** a gentle swell to the phrase's midpoint and
  a relaxation toward its end — a half-sine over `position_in_phrase /
  (phrase_len - 1)`, peak amplitude ~ ±8 velocity. The cadence step is exempt
  (its arrival weight comes from the floor).
- **Metric accent:** strong metric positions (position_in_phrase even, with the
  phrase downbeat strongest) get a small additive accent (+4..+8); weak
  positions a small subtraction. Keep accents SMALLER than the structural
  floor's start/cadence deltas so phrase structure still dominates.
- **Phrase-end taper:** the last interior step before a cadence eases off
  (the cadence itself remains the loudest point).
- **Hard requirement:** the realized velocities must show REAL within-phrase
  variation that is NOT `saturation × constant`. Saturation sets the level; the
  phrase contour shapes it. (The Test Engineer will pin: variance > 0 within a
  phrase, AND two steps with equal saturation but different phrase positions get
  different velocities.)

### 2.2 RHYTHM — ≥3 genuinely distinct onset/duration patterns + harmonic-rhythm acceleration
Beyond the current arpeggio/sustained binary. `features.edge_density` drives
rhythmic ACTIVITY; the orchestral role gates how much freedom each instrument
gets (see §2.4). At least **three** distinct patterns, selected by edge_density
bands (and role):
- **Sustained** (low edge_density): one onset at offset 0, hold ≈ full step.
- **Arpeggio** (high edge_density): chord tones spread evenly across the step,
  each onset offset = `k * ms_per_step / n`.
- **Dotted** (mid band): a long–short pair (e.g. onsets at 0 and `2/3` of the
  step; holds `2/3` and `1/3`).
- **Syncopated** (mid–high band, melody role only): onset DELAYED off the
  downbeat (e.g. first onset at `1/4` step), to push against the meter.
- **Rest-as-gesture** (occasional, interior weak positions, fill role): emit NO
  event (empty `Vec`) so silence becomes a deliberate articulation. *Never* on a
  phrase start or a cadence (those must always sound).
- **Harmonic-rhythm acceleration toward cadences:** as a phrase approaches its
  cadence boundary, increase onset DENSITY (more subdivisions per step) over the
  last 1–2 interior steps, so the music "drives" into the arrival. Pin: onset
  count on the pre-cadence step ≥ onset count on an early-interior step of the
  same phrase, given comparable edge_density.

### 2.3 ARTICULATION — hold_ms as a FRACTION of the step (not all ~90%)
Pass B sets `hold_ms` per note as a fraction of the per-note time budget:
- **Staccato** ≈ 0.30–0.50 of the slot (crisp, detached) — favor high
  edge_density / melodic activity.
- **Portato** ≈ 0.60–0.75 (slightly separated) — the neutral default.
- **Legato** ≈ 0.90–1.0 (connected, slightly overlapping) — favor low
  edge_density / sustained fill, and the approach to a cadence.
- **Phrase-end ritardando:** the final interior + cadence steps INCREASE actual
  `hold_ms` (notes ring longer as the phrase relaxes into arrival). Pin: the
  cadence step's note `hold_ms` > the mean interior `hold_ms` of its phrase.
- Pin: realized `hold_ms` fractions are NOT all ~0.9 — at least two distinct
  articulation fractions appear across a phrase.

### 2.4 ORCHESTRATION ROLES — bass vs melody vs fill differ in register, rhythm, motion
Driven by `instrument_role(inst_idx, num_instruments)` (§3) and `features.brightness`:
- **Bass:** sounds the chord ROOT (chord.notes[0]); LOW register (brightness
  pulls it down an octave when dark, never up); STEADY rhythm (sustained /
  simple, low onset count); minimal arpeggiation. Carries the harmonic floor.
- **Harmonic-fill:** inner chord tones (the upper voices); MIDDLE register;
  LEAST rhythmic activity (mostly sustained / portato); the place rest-as-gesture
  is allowed. Supports, never competes.
- **Melody:** TOP line (highest chord tone, or octave-up when bright); HIGH
  register; MOST rhythmic freedom (arpeggio, dotted, syncopation); carries the
  messa-di-voce peak. `features.brightness` raises/lowers the melody octave
  (bright = up).
- Pin: across a step with ≥3 instruments, the bass note < fill note(s) < melody
  note in pitch (register separation), and the melody's onset count ≥ the bass's
  (rhythmic-freedom separation), given comparable edge_density.

---

## 3. Orchestration-role assignment scheme (FINAL)

`instrument_role(inst_idx, num_instruments)`:

| num_instruments | inst_idx 0 | middle indices | inst_idx num-1 |
|---|---|---|---|
| 1 | — | — | **Melody** (the lone line is the tune, not a bare bass) |
| 2 | **Bass** | — | **Melody** (no fill) |
| ≥3 | **Bass** | **HarmonicFill** | **Melody** |

Lowest instrument = Bass; highest = Melody; everything between = HarmonicFill.
Total and pure over `(inst_idx, num_instruments)`. This is a DIFFERENT axis from
the existing `VoiceRole` (which classifies a voice WITHIN one triad voicing for
voice-leading constraints); `OrchestralRole` classifies an INSTRUMENT across the
ensemble for performance realization.

---

## 4. WIRING SPEC for `main.rs` (Rust Implementer — execute mechanically)

`main.rs` cannot compile headless (no OpenCV/ALSA), so the seam below is exact
and type-correct by construction. Make NO musical decision — every musical
choice lives in `chord_engine.rs`.

### 4.1 Compute the phrase plan ONCE and share it (replaces the raw chord Arc)
Currently (`main.rs` ~line 421–424):
```rust
let chords_vec =
    engine.generate_chords(&progression, 60, &mode, global_features.edge_density, 0.0);
let chords_arc = Arc::new(chords_vec);
```
**Change to:** plan the phrases once from the generated chords, and share the
PLAN instead of the raw chords:
```rust
let chords_vec =
    engine.generate_chords(&progression, 60, &mode, global_features.edge_density, 0.0);
let plan_vec = engine.plan_phrases(&chords_vec); // Vec<chord_engine::StepPlan>
let plan_arc = Arc::new(plan_vec);               // Arc<Vec<StepPlan>>
```
(`plan_phrases` already runs `voice_lead_sequence` internally, so the shared plan
carries the voice-led chords — no separate voice-leading call needed.)

### 4.2 Change `play_scanned_steps_concurrent`'s signature
Replace the `chords: Arc<Vec<chord_engine::Chord>>` parameter with:
```rust
plan: Arc<Vec<chord_engine::StepPlan>>,
```
At the call site (~line 494) pass `plan_arc` instead of `chords_arc`. Inside,
the per-worker clone becomes `let plan_cl = plan.clone();` (replacing
`chords_cl`). The `edge_threshold` local and all other params stay. Workers call
the new `worker_decide_action` (below). The coordinator loop, scheduling, jitter,
overlay, and barrier logic are UNCHANGED — it still consumes
`InstrumentAction { events: Vec<(u8,u8,u64,u64)> }`.

### 4.3 Rewrite `worker_decide_action` as a thin adapter
Replace the whole body of `worker_decide_action`. New signature:
```rust
fn worker_decide_action(
    f: &image_analysis::ScanBarFeatures,
    inst_idx: usize,
    step_idx: usize,
    num_instruments: usize,         // NEW — needed for instrument_role
    plan: &Vec<chord_engine::StepPlan>,
    ms_per_step: u64,
    _edge_threshold: f32,           // kept for call-site compatibility; unused now
) -> InstrumentAction
```
New body (mechanical — three moves: look up the step's plan, project features,
call the pure fn, map the result):
```rust
// 1) Look up THIS step's plan, wrapping like the old chords[step_idx % len].
let step = if plan.is_empty() {
    // defensive fallback: a lone tonic-ish StepPlan so playback never panics.
    // (Construct a minimal StepPlan with a C-major triad chord, phrase_index 0,
    //  position_in_phrase 0, phrase_len 1, PhrasePosition::PhraseStart, velocity 80.)
    // — or early-return an empty InstrumentAction; pick one and keep it dumb.
    return InstrumentAction { events: Vec::new() };
} else {
    &plan[step_idx % plan.len()]
};

// 2) Project the image features into the plain-scalar PerfFeatures
//    (no ScanBarFeatures crosses into the lib).
let features = chord_engine::PerfFeatures {
    saturation: f.avg_saturation,   // 0..=100
    brightness: f.avg_brightness,   // 0..=100
    edge_density: f.edge_density,   // 0..=1
};

// 3) Call the single pure entry point and map NoteEvent -> the existing tuple.
let note_events =
    chord_engine::realize_step(step, inst_idx, num_instruments, &features, ms_per_step);
let events = note_events
    .into_iter()
    .map(|e| (e.note, e.velocity, e.hold_ms, e.offset_ms))
    .collect();
InstrumentAction { events }
```
The worker-spawn call (~line 202) must now pass `num_instruments` and `&*plan_cl`
instead of `&*chords_cl`:
```rust
let action = worker_decide_action(
    f, inst_idx, step_idx, num_instruments, &*plan_cl, ms_per_step_local, h_threshold,
);
```

### 4.4 Things to DELETE / leave alone in `main.rs`
- `velocity_from_saturation` is now DEAD for playback (velocity comes from
  `realize_step`). Delete it, or leave it `#[allow(dead_code)]` — Implementer's
  choice; it makes no musical decision either way.
- Do NOT keep the old arpeggio/single-note/brightness-octave logic in
  `worker_decide_action` — all of it moves into `realize_step` (Pass B). The
  worker must not branch on `edge_density` itself.
- Overlay/scheduling/jitter/barrier code is untouched.

### 4.5 Summary of `main.rs` changes (for the Implementer)
- Compute `plan_phrases(&chords_vec)` once; wrap in `Arc<Vec<StepPlan>>`; pass
  that instead of `Arc<Vec<Chord>>`.
- Change `play_scanned_steps_concurrent` to take `plan: Arc<Vec<StepPlan>>`.
- Rewrite `worker_decide_action` to a 3-step adapter: `plan[step_idx % len]` →
  build `PerfFeatures` from `ScanBarFeatures` → `realize_step(...)` → map
  `NoteEvent`s to `(note,vel,hold_ms,offset_ms)`. Thread `num_instruments` in.
- Remove the musical branching (arpeggio/sustained/brightness-octave) and the
  now-dead `velocity_from_saturation` from the worker path.
- No musical decision remains in `main.rs`.

---

## 5. S4 carry-forward decisions

### 5.1 Voice-spacing constraint — RULE: no two UPPER voices on the same MIDI note
**Constant:** `MIN_UPPER_VOICE_SPACING = 1` (semitone). **Checker:**
`upper_voices_well_spaced(notes)` — true iff no two voices at indices `1..` share
the same MIDI note.

**Rationale (the musically-right call):** S4's `voice_lead_one` can collapse
upper voices to a literal unison (e.g. IV = [65, 65, 65]) when minimal-motion +
common-tone scoring drives two upper voices onto the same pitch. That silently
thins a triad to a dyad — a real defect. The MINIMAL correct fix forbids only the
literal unison between upper voices. A stricter "adjacent voices ≥ a minor third
apart" rule was considered and REJECTED: closed-position triads routinely place
inner voices a major/minor SECOND or THIRD apart (a perfectly idiomatic close
voicing), so a ≥ m3 floor would reject good voicings and force artificially open
spacing. The bass (index 0) is EXEMPT — a bass doubling an upper voice at the
octave is fine and common; only the inner/upper voices must not collide on one
pitch. **Pass B** wires `upper_voices_well_spaced` into `voice_lead_one`'s
candidate rejection (a voicing failing it is dropped from the search, alongside
the existing parallel-perfect rejection), keeping the documented last-resort
relaxation only if no spaced voicing exists. **Test Engineer pins:** every
voicing out of `voice_lead_sequence` satisfies `upper_voices_well_spaced`, and
specifically the IV that previously collapsed is now spaced.

### 5.2 Tendency-tone resolution — DECISION: LEAVE EMERGENT (do not hard-enforce)
**Decision:** Keep leading-tone→tonic resolution EMERGENT via the existing
minimal-motion + common-tone scorer. Do NOT add an explicit hard rule in Pass B.

**Rationale:**
1. The leading tone (scale degree 7) lies a SEMITONE below the tonic. The
   minimal-motion scorer already resolves it to the tonic in almost every case,
   because tonic is the nearest chord tone — enforcement would be redundant with
   the cost function that's already there.
2. A HARD rule fights voice-leading priorities the scorer balances correctly:
   the classic exception is V→vi (deceptive cadence) and inner-voice
   frustrated-leading-tone in four-part writing, where forcing 7→1 in an inner
   voice creates worse problems (incomplete chords, doubled thirds, parallels).
   We only have three voices and one of them is the bass; rigidly binding the
   single inner upper voice to 7→1 would frequently force a parallel or a leap
   the scorer would otherwise avoid.
3. Chordal sevenths don't exist yet (triads only), so chordal-7th resolution —
   the other place tendency-tone enforcement matters — is MOOT.
4. The professional standard here is "the line should *sound* resolved," which
   the smooth-connection scorer already delivers; a brittle hard rule would make
   the output worse in the exact corner cases (deceptive motion, three-voice
   spacing) where good practice DEPARTS from the textbook 7→1 default.

If a future pass adds dominant-seventh chords or four-part texture, REVISIT:
enforce chordal-7th → step-down and leading-tone → tonic *in outer voices only*,
with the deceptive-cadence exception coded explicitly. Not now.

---

## 6. Build / test discipline reminder
- Music subsystem builds/tests HEADLESS only:
  `cargo build --lib --no-default-features` / `cargo test --lib --no-default-features`
  / `cargo clippy --lib --no-default-features -- -W clippy::all`.
- Format ONLY `src/chord_engine.rs`: `rustfmt --edition 2021 src/chord_engine.rs`.
- The binary (with `main.rs`) cannot build here — the seam in §4 is exact so the
  Implementer can land it type-correct without a headless compile of `main.rs`.
- Keep the existing 16 `chord_engine` tests GREEN; Pass-A additions are purely
  additive API + stubs.

# S59 Lane H — `src/main.rs` feature-gating hygiene (pure-Rust headless app bin)

Status: implemented and verified.
Scope: `src/main.rs`, `Cargo.toml` (`[[bin]] audiohax` `required-features`). `src/engine.rs` untouched (frozen sha `e50c7db1…` verified before and after).

## 1. The root defect (precise diagnosis)

S58 fixed the *symptom* at the Cargo level: it declared `required-features =
["pure-analysis","synth","midi-out"]` on the `[[bin]] audiohax` table so that
`cargo test/build --no-default-features` would SKIP the app bin instead of failing
to compile it. That unblocked the headless test build but left the app bin
un-buildable under any pure-Rust (no-opencv, no-midir) feature set. The source had
three inconsistencies:

1. **Predicate mismatch on the pure-analysis image path.** The pure image-provider
   blocks were guarded on `#[cfg(not(feature = "opencv"))]` — i.e. keyed on the
   *absence of opencv* rather than the *presence of pure-analysis*. Under
   `--no-default-features` (opencv OFF but pure-analysis also OFF) the `not(opencv)`
   branch is compiled, yet it references `audiohax::pure_analysis`, which does not
   exist without the `pure-analysis` feature → `E0432`/`E0433`. Six sites (all the
   pure-path source acquisition + composer-install blocks):
   - image-source acquisition in `run_render_wav` (was line ~369)
   - composer install + fallback in `run_render_wav` (was lines ~407, ~409)
   - image-source acquisition in `main` (was line ~601)
   - composer install + fallback in `main` (was lines ~649, ~651)

2. **Unconditional `synth_sink` references.** `build_step_event_timeline` returns
   `Vec<audiohax::synth_sink::TimedMidiEvent>` and the live stream loop dispatches
   `audiohax::synth_sink::MidiCmd`. These are unconditional. They are only a *defect*
   if the app is expected to build without `synth`; see the design decision in §3 —
   the resolution is to keep `synth` a hard requirement, which makes these references
   honest rather than to gate them.

3. **Unconditional `midi_output::MidiOut` / `midir` reference.** `mod midi_output;`
   (was line ~44), `use midi_output::MidiOut;` (was line ~68), the
   `use audiohax::engine::AudioSinkError;` that only the MIDI impl needs (was line
   ~67), `impl AudioSink for MidiOut` (was lines ~80–92), the `--list-midi-ports`
   query (was lines ~519–534), and the MIDI arm of runtime sink selection (was lines
   ~676–692) were all compiled unconditionally. `midi_output.rs` opens with
   `use midir::{…}`, and `midir` is `optional = true` (enabled only by `midi-out`), so
   any feature set without `midi-out` failed to resolve `midir` when these sites
   compiled.

Net: even with S58's Cargo skip in place, the app bin could not be built under
`--features pure-analysis,synth`; the *only* buildable set was the full default set.

## 2. The fix, per site

### 2a. Pure-analysis image path — predicate corrected (defect #1)
All six `#[cfg(not(feature = "opencv"))]` guards on pure-path blocks became:

```rust
#[cfg(all(not(feature = "opencv"), feature = "pure-analysis"))]
```

This is "prefer opencv when `feature = "opencv"`, else use pure-analysis when
`feature = "pure-analysis"`" — the two image providers are now mutually exclusive by
preference, keyed on the RIGHT predicates. The opencv blocks stay
`#[cfg(feature = "opencv")]`. Behaviour is unchanged for every set that actually
builds the bin (under the default set opencv is OFF and pure-analysis is ON, so the
pure branch is selected exactly as before); the change only makes the pure branch
require the feature that supplies its symbols.

### 2b. MIDI output — gated on `midi-out` (defect #3)
Every `MidiOut`/`midir` reference is now `#[cfg(feature = "midi-out")]`:
- `mod midi_output;`
- `use midi_output::MidiOut;` and the MIDI-only `use audiohax::engine::AudioSinkError;`
- `impl AudioSink for MidiOut`
- the `--list-midi-ports` query gains a `#[cfg(not(feature = "midi-out"))]` sibling
  that prints a clear "no `midi-out` feature — rebuild with `--features midi-out`"
  message and returns, so the CLI flag still resolves cleanly.
- runtime sink selection is split into two `#[cfg]` arms (see §2c).

### 2c. Runtime sink selection — de-duplicated + gated
The in-process synth construction was extracted verbatim into a new
`fn build_synth_sink(&PlayArgs) -> Result<Box<dyn AudioSink>, …>` (byte-behaviour
identical to the previous inline path: no flags ⇒ `SynthConfig::default()` + bundled
font). Sink selection is then:
- `#[cfg(feature = "midi-out")]`: compute `want_midi` from `--output`/`--midi-virtual`,
  build `MidiOut` when requested, else `build_synth_sink(...)` — identical to the
  prior runtime behaviour.
- `#[cfg(not(feature = "midi-out"))]`: if the operator asked for MIDI, print a plain
  "ignored — using the in-process synth" note and fall back to `build_synth_sink(...)`.

Extracting the shared builder avoids duplicating the synth path across the two cfg
arms and avoids an `unreachable!()` placeholder.

### 2d. `synth_sink` references — intentionally left unconditional (defect #2)
See §3: `synth` is a hard requirement of the app bin, so these references are honest.

## 3. The minimal pure-Rust feature set + Cargo reconciliation

**Minimal buildable set for the app bin: `["pure-analysis", "synth"]`.** Proven with
`cargo build --no-default-features --bin audiohax --features pure-analysis,synth`
(compiles clean). `required-features` was lowered from
`["pure-analysis","synth","midi-out"]` to `["pure-analysis","synth"]`.

Rationale for each feature's disposition:

- **`synth` — kept REQUIRED (not gated out).** It is a hard dependency of the app's
  *event model*, not merely an output backend: `build_step_event_timeline` returns
  `synth_sink::TimedMidiEvent` and the stream loop dispatches `synth_sink::MidiCmd`
  on *every* sink, including the external-MIDI path. Making `synth` optional would
  require an alternate timeline representation and a second dispatch vocabulary —
  a structural change well outside Lane H's source-hygiene scope, and one that would
  touch the render/play convergence guarantee. Keeping `synth` required makes the
  unconditional `synth_sink` references (defect #2) honest.

- **`pure-analysis` — kept REQUIRED as the default image provider.** The default
  feature set has no `opencv`, so pure-analysis is the default (and, without opencv,
  the only) image provider. Requiring it also guarantees a defined feature `source`,
  so a stray `--features synth`-only invocation is cleanly SKIPPED rather than
  compiling to a "cannot find value `source`" error. The opencv escape hatch is
  normally ADDITIVE over the default set (`cargo build --features opencv` keeps
  pure-analysis + synth + midi-out and adds opencv), so opencv builds are unaffected.

- **`midi-out` — now TRULY OPTIONAL.** It remains in the DEFAULT set, so
  `cargo run`/`cargo build` compile `MidiOut` and select the sink at runtime exactly
  as before — **default runtime behaviour is byte-preserved.** A pure-Rust build
  (`--no-default-features --features pure-analysis,synth`) drops `midir` entirely and
  uses the in-process synth as the sole sink.

Feature-set behaviour matrix:

| Invocation | opencv | pure | synth | midi-out | app bin | image path | sink(s) |
|---|---|---|---|---|---|---|---|
| `cargo build` (default) | off | on | on | on | builds | pure | synth default, MIDI at runtime |
| `--no-default-features --features pure-analysis,synth` | off | on | on | off | **builds (Lane H win)** | pure | synth only |
| `--features opencv` (additive) | on | on | on | on | builds | opencv | synth default, MIDI at runtime |
| `--no-default-features` | off | off | off | off | **skipped** (pure absent) | — | — |

The last row is why `cargo test --no-default-features` stays green: the bin is
skipped, and the pure-Rust music+modem library/integration tests build and run.

## 4. Acceptance (all passed)

1. `sha256sum src/engine.rs` == `e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261` before AND after (engine.rs never edited).
2. `cargo build` (default) — clean.
3. `cargo build --no-default-features --bin audiohax --features pure-analysis,synth` — **compiles** (the Lane H win).
4. `cargo test` (default) — **580 passed / 0 failed** (exact S58 baseline; none dropped).
5. `cargo test --no-default-features` — compiles + green (S58 win preserved; app bin skipped).
6. `cargo clippy --all-targets` default + reduced — net-new warnings 0.
7. Files changed: `src/main.rs`, `Cargo.toml`. (`src/engine.rs` NOT changed.)
</content>
</invoke>

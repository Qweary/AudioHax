# Research: Audio Quality for the In-Process Synth

Status: research only — no code, Cargo, or asset changes. This document triages why
the default `--output synth` path sounds artificial, ranks the cheap wins, lists
license-safe soundfont alternatives to A/B, and proposes a minimal A/B harness.

Calibration note: claims about the audio *code path* and the synthesizer's behavior
are grounded in direct reading of the in-tree source and the resolved dependency
source (high confidence). Claims about which font *sounds* best are reputational and
must be settled by the owner's ear — they are flagged as such throughout.

---

## 0. Ground truth (verified)

- **Synth engine:** `rustysynth` resolved to **1.3.6** (`Cargo.lock`), pure-Rust SF2
  playback. `cpal` 0.16, `rtrb` 0.3. All pure Rust; `synth` is a default feature.
- **Audio path** (`src/synth_sink.rs`): engine thread enqueues `MidiCmd`
  (note_on / note_off / program_change) onto a lock-free SPSC ring; the cpal audio
  callback drains the ring, calls `synth.process_midi_message(...)`, then
  `synth.render(&mut left, &mut right)` and interleaves into the output buffer.
- **Synth construction is bare:** `SynthesizerSettings::new(sample_rate)` followed by
  `Synthesizer::new(&sound_font, &settings)`. **No field on `SynthesizerSettings` is
  overridden** — block size, polyphony, and the reverb/chorus toggle are all left at
  rustysynth's defaults.
- **No master gain / limiter / post-processing** is applied. The render output is
  written straight to cpal. (`master_volume` inside rustysynth is a private field
  hard-set to `0.5`; there is **no public setter** — see §1b.)
- **Soundfont:** GeneralUser GS v2.0.3 (S. Christian Collins), ~31 MB SF2,
  `include_bytes!`-embedded as `assets/soundfonts/default.sf2`, git-ignored pending a
  distribution decision (`assets/soundfonts/.gitignore`, `assets/soundfonts/README.md`).
  License: GeneralUser GS License v2.0 — permits bundled/binary and commercial
  distribution with a courtesy credit.
- **Runtime font swap already exists in the library** (`SynthSink::new` accepts
  `SoundFontSource::Path`/`Bytes`), **but the `audiohax` binary does not expose it** —
  `main.rs:419` hardcodes `SynthSink::with_bundled_soundfont()`. There is **no
  `--soundfont` and no `--reverb` CLI flag today.**
- **The external escape hatch exists and works:** `docs/midi-routing.md` documents
  `--output midi` into FluidSynth/Qsynth/a DAW with reverb + chorus.

Two pieces of in-repo prose are **stale relative to rustysynth 1.3.6** and should be
mentally discounted while reading the rest of this doc:
- `docs/midi-routing.md` says the synth path is "**dry** — a bare in-process SoundFont
  with no reverb or chorus." `src/cli.rs:111` likewise says "Dry; the default."
- In 1.3.6 this is not true: reverb/chorus is **on by default** and channels start with
  a moderate reverb send (see §1b). The prose was accurate for an earlier rustysynth
  (whose upstream README still lists Reverb/Chorus under "Todo"), but the version we
  actually compile against ships both effects.

---

## 1. Where does the "artificial" character come from?

Triaged in order of leverage. The honest headline: **the dominant cause is the GM
sample-playback ceiling, not a misconfigured mix.** The synth is closer to "correctly
configured but inherently General-MIDI" than to "broken/dry."

### 1a. The General-MIDI sample-playback ceiling — DOMINANT

GeneralUser GS is a genuinely good, carefully balanced GM bank, but it is still a
*general* bank: one (or a few velocity-layered) sample(s) per instrument, looped and
pitch-shifted across the keyboard. That is the same synthesis class as a 1990s
hardware GM module. For a trained ear — and especially on sustained, expressive,
or solo-instrument lines (strings, brass, winds) — this reads as "MIDI": the timbre
doesn't evolve with dynamics the way a real instrument does, vibrato/articulation are
canned, and transitions between notes are sample-crossfades rather than real
legato/portamento. No amount of reverb tuning removes this; it is the floor of the
approach. **Confidence: high that this is the largest single contributor.**

A second, *engine-side* contributor lives in this same bucket and is worth calling out
because it is fixable in our code: the way notes are driven. The engine emits
fixed-character note_on/note_off events; if velocities are uniform or compressed, and
if no expression/CC shaping is sent, even a great font sounds mechanical. (This doc is
scoped to the *synth/quality* layer; how the image→music mapping chooses velocities,
durations, and program assignments is a separate, larger lever owned elsewhere in the
pipeline. Flagging it here only so it isn't mistaken for a soundfont problem.)

### 1b. Mix / reverb / gain — REAL but SECONDARY, and partly already on

What rustysynth 1.3.6 actually does by default (verified in the resolved crate source):

- `SynthesizerSettings::enable_reverb_and_chorus` **defaults to `true`** — a Freeverb-
  style reverb and a chorus are instantiated and mixed in.
- Each channel is constructed via `Channel::new`, which calls `reset()`, which sets
  `reverb_send = 40` (chorus_send = 0). So notes **do** get a moderate reverb send out
  of the box, even though the engine never sends a reverb-send CC. The synth is **not
  bone-dry.**
- The reverb's character is fixed by crate constants (room ~0.5, wet ~0.33, etc.).

The mix limitations that *do* hurt us, and which we control:

1. **Reverb is binary from our side.** rustysynth exposes only the on/off toggle
   (`enable_reverb_and_chorus`) plus per-channel reverb/chorus *send* via MIDI CC
   (`0x5B` / `0x5D`). There is **no public API to set the reverb's room/wet/damp
   level** — those are private constants. So our only in-engine reverb levers are:
   (a) the global on/off, and (b) raising/lowering each channel's reverb/chorus send
   by emitting CC 0x5B/0x5D (we currently emit neither, so we sit at the default 40).
   A *little* more reverb send can add believable space; too much smears the GM
   artifacts into mush. **This is an ear decision.**
2. **No master gain / limiting / normalization.** Output goes straight to cpal at
   rustysynth's internal 0.5 master volume with no headroom management. Dense chords
   can clip or sound thin/peaky depending on the font. A simple post-render gain +
   soft-clip would make the level consistent and let us A/B fairly. We can implement
   this **in our own callback** (multiply the rendered `left`/`right` before
   interleaving) without any rustysynth API — rustysynth's lack of a public
   `set_master_volume` is therefore **not** a blocker.
3. **No stereo width / EQ.** Out of scope for cheap wins; mention only for completeness.

**Confidence: high** on the mechanism; **the audible size of the win is ear-dependent**
and is exactly what the A/B harness exists to measure.

### 1c. The synthesis approach itself — the hard ceiling

SF2 sample playback has a realism ceiling far below physical-modelling or sampled
multi-articulation libraries (the kind a DAW loads). Staying pure-Rust + GM SF2 means
accepting that ceiling. The realistic target for the default path is **"a clean,
pleasant, well-mixed GM render"**, not "indistinguishable from a real ensemble." For
the latter, the external-engine route (§4) is the answer, and it already exists.

---

## 2. Soundfont options (license-safe, redistributable)

All candidates below are **uncompressed SF2** (rustysynth 1.3.6 is **SF2-only — it does
not decode SF3/Ogg-Vorbis-compressed fonts**; verified against the crate and the
existing `assets/soundfonts/README.md` note). For any SF3 font listed, you must obtain
the **uncompressed `.sf2`** build.

Realism reputation is community consensus, not measured — treat the ranking as "where
to point your ears first," not a verdict.

| Font | Size (SF2) | License / redistribution | Realism reputation | Notes for us |
|---|---|---|---|---|
| **GeneralUser GS v2.0.3** (current default) | ~31 MB | GeneralUser GS License v2.0 — bundling/commercial OK, courtesy credit | Excellent *balance/mix*; modest raw realism | Baseline. Smallest. Same author as MuseScore_General. |
| **FluidR3_GM** (Frank Wen) | ~141 MB | **MIT** — redistribute in entirety with copyright + README | The open-source "reference" GM bank; richer/fuller than GeneralUser on many patches | Best first A/B candidate: well-known, permissive, clearly better-sampled on several instruments. ~4.5× the size. |
| **MuseScore_General** (S. C. Collins) | ~206 MB SF2 (also ships as SF3) | **MIT** — reproduce licence + copyright notice | Generally regarded as the highest-realism of the free GM banks; heavy velocity layering | Strongest *quality* candidate; heaviest. Use the **lossless/uncompressed `.sf2`**, not the `.sf3` (rustysynth can't read SF3). |
| **Timbres of Heaven** | ~399 MB | Freely redistributable (verify the bundled readme before *embedding*) | Very high realism, extensive velocity layering; orchestral favourite | Quality-ceiling reference; far too large to embed — load-by-path only. Confirm exact redistribution terms before any bundling. |
| **Arachno** | ~148 MB | Freeware; **check terms before redistributing/bundling** | "Punchy," strong drums/synths | Tonal alternative, not clearly more *realistic* than FluidR3. Lower priority. |

**Size / distribution tradeoff (load-bearing):** the engine currently *embeds* the
default font with `include_bytes!`, so swapping the **embedded** default to a 141 MB
(FluidR3) or 206 MB (MuseScore) font would bloat the binary 4–7×. The right pattern is
to **keep a small embedded default and load the heavy fonts by path at runtime** — which
the library already supports (`SoundFontSource::Path`) and the A/B harness in §5 uses.
If a heavier font is ever chosen as the *shipped* default, move to git-LFS or
fetch-on-build rather than embedding (this is the same open distribution decision the
soundfont README already flags).

**Confidence:** high on licenses for GeneralUser, FluidR3, and MuseScore_General (all
verified: the two alternatives are MIT). Lower on Timbres/Arachno redistribution
specifics — verify each font's own bundled readme before *embedding or redistributing*;
loading them locally by path for an ear test carries no distribution obligation.

---

## 3. In-process effects — the cheapest quality wins (no leaving pure Rust)

Ordered low-risk / high-leverage first. Each is implementable in our own crate.

1. **Swap the font (load-by-path), don't touch code.** The single biggest *audible*
   in-process win is almost certainly a better-sampled font (FluidR3 or
   MuseScore_General). Zero code risk once a `--soundfont PATH` flag exists (§5);
   today it requires the flag because the binary hardcodes the bundled font.
   *Leverage: high. Risk: none.*
2. **Add a master gain + soft-clip in our callback.** Multiply rendered `left`/`right`
   by a tunable gain and soft-clip before interleaving (a few lines in
   `build_f32_stream`). Gives consistent level across fonts, prevents clipping on
   dense chords, and makes A/B fair. Needs no rustysynth API. *Leverage: medium.
   Risk: low (purely our math on the output buffer).*
3. **Tune the reverb *send* via CC, and expose a reverb on/off.** We currently sit at
   the default per-channel reverb_send=40 and never adjust it. Emitting CC 0x5B at a
   chosen level per channel (or globally) lets us dial space up/down; the global
   `enable_reverb_and_chorus` toggle lets us A/B "dry vs wet" honestly. Note the hard
   limit: rustysynth gives no room/wet/damp control, so this is a coarse knob.
   *Leverage: medium, ear-dependent. Risk: low.*
4. **Pick better GM programs.** Confirm the image→music mapping is selecting flattering
   GM programs (e.g. choosing warm pads / good piano / string ensemble rather than the
   thin/buzzy GM patches). This is a mapping-layer change, not a synth change, but it
   is one of the largest perceived-quality levers and costs nothing at synthesis time.
   *Leverage: high. Risk: low, but lives outside this file's layer — flag to the
   mapping owner.*
5. **(Optional) light post-reverb send for sustained voices only.** If a global reverb
   send sounds muddy, raising send only on sustained/lead channels and keeping
   percussive channels drier is a cheap realism trick. *Leverage: low–medium. Risk:
   low.* Defer until 1–4 are heard.

A post-synth *convolution* reverb (an impulse-response reverb on the rendered buffer)
would beat rustysynth's algorithmic reverb in quality, but it pulls in a new
dependency/DSP and is **not** a cheap win — leave it for after the A/B settles whether
reverb is even the limiting factor (§1 says it usually isn't).

---

## 4. The honest ceiling: in-process vs external engine

**Recommended posture (bounded — not an engine hunt):**

> Make the in-process default *decent* (better font + master gain + sane reverb), and
> keep the documented external-MIDI route as the high-fidelity / presenting path.

Rationale:

- **In-process realistic ceiling:** with a better font + gain staging + tuned reverb,
  the default can go from "obviously MIDI" to "clean, pleasant GM render." It will not
  cross into "sounds like real players" — that is the SF2/GM ceiling (§1c), not a
  tuning gap.
- **`oxisynth` is not worth a switch for this goal.** It is the other pure-Rust
  FluidSynth-lineage synth and exposes reverb/chorus *parameters* (room/damp/width/
  level) that rustysynth hides — so it would give finer reverb control. But it is still
  **SF2 sample playback with the same realism ceiling**, it would be a non-trivial
  rewrite of the working `synth_sink.rs` hot path, and the payoff is "slightly better
  reverb control," not "a different class of sound." **Do not switch engines for
  realism.** (If, after the A/B, fine reverb control turns out to be the single thing
  blocking acceptance — unlikely per §1 — oxisynth becomes a *considered* option, not a
  default recommendation.)
- **External engine / DAW is the real high-fidelity answer and already exists.** For
  anything that has to sound good in front of an audience, route `--output midi` into
  FluidSynth/Qsynth (with reverb+chorus) or a DAW loading a proper sampled instrument
  (`docs/midi-routing.md`). That path has no realism ceiling we control — it is bounded
  only by the synth/instrument the owner points it at.

**Verdict:** pragmatic, two-tier. Default path = "good enough to demo and to test,"
external route = "good enough to present." No open-ended engine search.

---

## 5. A/B harness design (minimal, pure-Rust-first)

Goal: at a machine, the owner compares options **by ear in minutes**, apples-to-apples,
on the **same composition**. The library already supports everything needed; only thin
CLI plumbing is missing.

### Knobs to expose (smallest set that covers §1–§3)

| Flag | Maps to | Library support today |
|---|---|---|
| `--soundfont <PATH>` | `SoundFontSource::Path` instead of `Bundled` | **Already exists** in `SynthSink::new`; binary just needs to pass it. |
| `--reverb on\|off` | `SynthesizerSettings.enable_reverb_and_chorus` | Field exists; needs a constructor variant that sets it. |
| `--reverb-send <0-127>` (optional) | emit CC 0x5B per channel at startup | Reachable via existing `process_midi_message`; engine just needs to send it. |
| `--gain <f32>` (optional) | post-render multiply + soft-clip in `build_f32_stream` | New, ~5 lines, our own math. |
| `--engine synth\|midi` | already exists (`--output`) | Use the existing external route as the "ceiling" reference. |

Keep it pure-Rust-first: `--soundfont` + `--reverb` + `--gain` cover the whole
in-process matrix; `--output midi` is the external reference already documented.

### Apples-to-apples rendering

The cleanest ear test renders the **same** composition deterministically through each
config. Two options, in order of preference:

1. **Deterministic render-to-WAV per config (preferred).** Add a `--render-wav <OUT>`
   path that runs the same image→events sequence offline (no cpal device) and writes a
   WAV via the already-present `hound` dependency. Because the event stream is identical
   across configs, the only variable is the synth config — a true A/B. Bonus: WAVs can
   be loaded side-by-side in any editor for blind comparison and re-listened without
   re-running. *(The unit test `bundled_soundfont_renders_nonsilent_audio_for_a_note_on`
   already demonstrates offline rendering with `synth.render(...)` and no cpal — the
   offline render path is a short extension of that.)*
2. **Live playback with a fixed input.** Run the same image through `--output synth`
   with each flag combo. Faster to wire, but less rigorous (timing/host jitter) and
   you can't A/B without re-running. Fine as a first pass.

### A/B test matrix (configs × what to listen for)

Hold the **input image constant** across every row.

| # | Soundfont | Reverb | Gain | What to listen for |
|---|---|---|---|---|
| A0 | GeneralUser GS (baseline) | default (on, send 40) | none | The reference "artificial" sound. Anchor for every comparison. |
| A1 | GeneralUser GS | **off** | none | How much of the character is reverb vs. the font itself. Expect: drier, more exposed — confirms reverb is *not* the main problem. |
| A2 | GeneralUser GS | on | **+gain/soft-clip** | Does level/headroom alone clean it up? Listen for clipping gone, fuller body. |
| B0 | **FluidR3_GM** | default | none | First real font A/B. Listen for richer piano/strings/brass vs A0. |
| B1 | FluidR3_GM | on | +gain | The likely "good default" candidate, fully dressed. |
| C0 | **MuseScore_General** (.sf2) | default | none | Highest-realism free font. Listen for velocity-layer realism on sustained lines. |
| C1 | MuseScore_General | on | +gain | Quality ceiling of the pure-Rust path. |
| D0 | (best of B/C) | **higher reverb-send** | +gain | Does more space help or smear? Ear-only call. |
| X | (any font) via **`--output midi` → FluidSynth +reverb/chorus** | — | — | The external **ceiling reference**. Establishes how far above the in-process default the external route sits — sets honest expectations. |

How to read it: A0→A1 isolates reverb's contribution; A0→A2 isolates gain; A0→B0→C0
isolates the **font** (expected to be the biggest jump); the X row sets the ceiling.
If B0/C0 is the big mover (likely), the recommendation collapses to "ship a better
font + gain staging, keep reverb on, keep the external route documented."

---

## 6. Recommended cheap-wins list (ordered, low-risk first)

1. **Expose `--soundfont <PATH>`** (binary plumbing only; library already supports it).
   Unblocks every font A/B without a rebuild.
2. **A/B FluidR3_GM first, then MuseScore_General**, against GeneralUser GS, holding the
   image constant. Almost certainly the largest audible win.
3. **Add master gain + soft-clip** in the audio callback (our own math; no rustysynth
   API needed). Consistent level, no clipping, fair comparisons.
4. **Add `--reverb on|off`** (and optionally `--reverb-send`) to A/B space honestly;
   leave reverb **on** unless the ear says otherwise.
5. **Check GM program selection in the mapping layer** (separate owner) — flattering
   programs are a large perceived-quality lever at zero synthesis cost.
6. **Keep the external-MIDI route as the presenting path**; do **not** switch synth
   engines or chase a new one for realism.

Distribution reminder: keep a small embedded default; load heavy fonts (FluidR3 ~141 MB,
MuseScore ~206 MB) **by path**, not embedded, until/unless a heavier shipped default is
chosen — at which point move to git-LFS or fetch-on-build, consistent with the open
distribution decision already noted in `assets/soundfonts/README.md`.

---

## Sources

- In-tree: `src/synth_sink.rs`, `src/main.rs`, `src/cli.rs`, `Cargo.toml`,
  `Cargo.lock`, `assets/soundfonts/README.md`, `assets/soundfonts/.gitignore`,
  `docs/midi-routing.md`.
- Resolved dependency source (rustysynth 1.3.6): `synthesizer_settings.rs`
  (`enable_reverb_and_chorus` default `true`), `channel.rs` (`reset()` sets
  `reverb_send = 40`), `synthesizer.rs` (`master_volume = 0.5`, no public setter),
  `reverb.rs` / `chorus.rs` (effects present; parameters are private constants).
- rustysynth — pure-Rust SF2 synthesizer (SF2-only; upstream README still lists
  Reverb/Chorus under "Todo," but 1.3.6 ships both, confirmed in crate source):
  https://github.com/sinshu/rustysynth , https://crates.io/crates/rustysynth
- OxiSynth — pure-Rust FluidSynth-lineage synth with exposed reverb/chorus parameters:
  https://github.com/PolyMeilex/OxiSynth , https://docs.rs/oxisynth
- SoundFont3 format (Ogg-Vorbis-compressed SF2; not decoded by rustysynth):
  https://github.com/FluidSynth/fluidsynth/wiki/SoundFont3Format
- Free GM soundfont comparison (sizes/reputation):
  https://miditoolbox.com/posts/best-free-general-midi-soundfonts-2026
- FluidR3_GM — MIT license / redistribution terms:
  https://member.keymusician.com/Member/FluidR3_GM/README.html
- MuseScore_General — MIT license (S. C. Collins), SF2 + SF3:
  https://github.com/musescore/MuseScore/blob/master/share/sound/FluidR3Mono_License.md ,
  https://ftp.osuosl.org/pub/musescore/soundfont/MuseScore_General/MuseScore_General_License.md

# SoundFonts

The pure-Rust audio path (the default build, `synth` feature) renders MIDI in-process
with [`rustysynth`](https://crates.io/crates/rustysynth), which needs a SoundFont
(`.sf2`) — the file of instrument samples that *is* the sound. The build embeds
`default.sf2` at compile time via `include_bytes!`, so **the file must be present here
before you build.**

The `.sf2` itself is git-ignored for now (it is ~31 MB; the in-repo-vs-LFS-vs-fetch
distribution model is an open decision). Fetch the default before building:

```sh
curl -L -o assets/soundfonts/default.sf2 \
  https://raw.githubusercontent.com/mrbumpy409/GeneralUser-GS/main/GeneralUser-GS.sf2
```

## Default font

**GeneralUser GS v2.0.3** by S. Christian Collins — a full General-MIDI SoundFont
(261 presets + 13 drum kits) at a lean ~31 MB. Format: SoundFont 2 (`.sf2`,
uncompressed — `rustysynth` does not decode SF3/Ogg-Vorbis fonts).

License: *GeneralUser GS License v2.0* — permits use in software projects, including
commercial and bundled/binary distribution. Courtesy credit:
`GeneralUser GS by S. Christian Collins — schristiancollins.com`.

## Swapping fonts (no rebuild)

`SynthSink` accepts a runtime SoundFont path, so you can A/B a different `.sf2`
without recompiling — pass `--soundfont <path>` to `play` or `render`.

## Local A/B test set

A spread of fonts (light → balanced → full → realism-ceiling) is staged in this
directory for ear-testing. Each was sourced from the **signed Debian/Kali package
repositories** (the same trust root as the OS — not a third-party mirror), verified
as an uncompressed SoundFont 2 container (`RIFF…sfbk`, which `rustysynth` requires —
it cannot decode SF3/Ogg), and smoke-tested through the offline `render --wav` path.
All `.sf2` here are git-ignored (size + the open distribution decision); check any
font's exact license with `apt show <pkg>` or `/usr/share/doc/<pkg>/copyright`.

| File | Source (apt package / origin) | Size | Character |
|---|---|---|---|
| `default.sf2` | GeneralUser GS v2.0.3 — S. Christian Collins (the embedded default; GeneralUser GS License, bundling OK) | 31 MB | balanced GM, lean |
| `TimGM6mb.sf2` → `/usr/share/sounds/sf2/` | `timgm6mb-soundfont` | 6 MB | light / simple / fast-loading |
| `FluidR3_GM.sf2` → `/usr/share/sounds/sf2/` | `fluid-soundfont-gm` (MIT) | 142 MB | full; the classic FluidSynth default |
| `MuseScore_General.sf2` | `musescore-general-soundfont-lossless` → `MuseScore_General_Full.sf2` (MIT) | 467 MB | highest fidelity / realism ceiling / heaviest |

The two `→ /usr/share/sounds/sf2/` entries are symlinks to the already-installed,
distro-signed system fonts (no duplication); the other two are real files here.

A/B them — same composition, different timbre:

```sh
./play assets/images/example.jpg                                    # default (embedded)
./play assets/images/example.jpg --soundfont assets/soundfonts/TimGM6mb.sf2
./play assets/images/example.jpg --soundfont assets/soundfonts/FluidR3_GM.sf2
./play assets/images/example.jpg --soundfont assets/soundfonts/MuseScore_General.sf2

# offline, no audio HW (compare WAVs side by side):
./target/release/audiohax render assets/images/example.jpg --wav /tmp/fluid.wav \
  --soundfont assets/soundfonts/FluidR3_GM.sf2
tools/ab-render.sh assets/images/example.jpg            # renders several configs at once
```

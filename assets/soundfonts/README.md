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
without recompiling. Higher-fidelity (heavier) redistributable GM alternatives:

- **FluidR3_GM** (MIT, ~148 MB) — the classic FluidSynth default; closest to a
  historical FluidSynth setup if exact parity matters.
- **MuseScore_General.sf2** (MIT, ~206 MB, uncompressed `.sf2` build) — highest
  fidelity, heaviest. Use the uncompressed `.sf2`, not the `.sf3`.

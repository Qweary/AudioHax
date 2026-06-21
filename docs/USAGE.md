# AudioHax — Usage Reference

Full subcommand and flag reference. For the fast path, see the [README](../README.md).

All examples assume you are in the repo root with the toolchain on your `PATH`
(`export PATH="$HOME/.cargo/bin:$PATH"`) and that `assets/soundfonts/default.sf2` was
fetched before building (see the README Quick Start). `cargo run` with no `--bin`
resolves to the `audiohax` app (set via `default-run`).

```sh
cargo run --release -- --help            # top-level help
```

The top-level grammar advertises five subcommands, but **two are live today**:
`play` (fully wired) and `render --wav` (offline render). `analyze`, the top-level
`modem …`, and `tui` parse but print a "not yet wired" / "use the dedicated bin"
message — use the dedicated modem bins (below) for modem work.

---

## `play` — image → live audio

```sh
cargo run --release -- play [IMAGE] [OPTIONS]
./play [IMAGE] [OPTIONS]                    # wrapper: same thing, builds-if-stale
```

`[IMAGE]` is optional; omitting it uses the bundled example image
(`assets/images/example.jpg`).

**Pipeline (musical) flags** — shared with `render`:

| Flag | Default | Effect |
|---|---|---|
| `--instruments <n>` | `4` | number of voices in the ensemble |
| `--ms-per-step <ms>` | `250` | milliseconds per scan step (performance tempo) |
| `--steps <n>` | `40` | number of scan steps across the image |
| `--thickness <f>` | `0.10` | scan-bar thickness as a fraction of the scan axis |
| `--jitter-percent <f>` | `15` | per-event duration jitter, in percent (±) |
| `--seed <u64>` | *(absent)* | deterministic composition; same image+seed ⇒ identical result. Absent ⇒ varies run-to-run |

**Audio (in-process synth) flags** — shared with `render`; ignored under `--output midi`:

| Flag | Default | Effect |
|---|---|---|
| `--soundfont <path>` | bundled GM font | use a different `.sf2` (SF2-only; no SF3/compressed). Missing/invalid file fails loudly |
| `--reverb <on\|off>` | `on` | rustysynth ships reverb+chorus on; `off` is bone-dry |
| `--gain <f32>` | `1.0` | master gain (then `tanh` soft-clip); `1.0` is a bit-exact no-op |

**Output-sink flags:**

| Flag | Default | Effect |
|---|---|---|
| `--output <synth\|midi>` | `synth` | `synth` = built-in pure-Rust synth; `midi` = route to external port/DAW |
| `--midi-port <name\|index>` | first port / `$AUDIOHAX_MIDI_PORT` | (with `--output midi`) select by name substring or numeric index |
| `--list-midi-ports` | — | list available MIDI output ports and exit (no playback) |
| `--midi-virtual [NAME]` | name `AudioHaxOut` | create a virtual MIDI port (Linux ALSA / macOS CoreMIDI; not Windows). Forces `--output midi` |

Examples:

```sh
./play assets/images/Lena.png --instruments 6 --ms-per-step 180
cargo run --release -- play assets/images/example.jpg --reverb off --gain 1.5
cargo run --release -- play assets/images/example.jpg \
  --soundfont assets/soundfonts/FluidR3_GM.sf2
```

---

## `render` — image → WAV (offline)

```sh
cargo run --release -- render [IMAGE] --wav <PATH> [OPTIONS]
```

Renders the synthesized audio to a WAV at `<PATH>` with no audio device. **`--wav` is
required for `render` to do work** — without it, `render` prints a notice and exits.
The render is deterministic: the same image + config (and `--seed`) yields a
byte-identical WAV — ideal for blind A/B of fonts/effects. Accepts the same pipeline
and audio flags as `play` (`--seed`, `--instruments`, `--soundfont`, `--reverb`,
`--gain`, …).

```sh
cargo run --release -- render assets/images/example.jpg --wav out.wav --seed 42
cargo run --release -- render assets/images/example.jpg --wav fluid.wav \
  --soundfont assets/soundfonts/FluidR3_GM.sf2 --reverb off
```

Helper: `tools/ab-render.sh <IMAGE> [SOUNDFONT.sf2]` renders several configs at once
for ear-testing.

---

## Bundled assets

- **Example images** (tracked in git): `assets/images/example.jpg`,
  `AudioHaxImg1.jpg`, `AudioHaxImg2.jpg`, `AudioHaxImg3.jpg`, `Lena.png`,
  `magicstudio-art.jpg`.
- **SoundFonts** (`assets/soundfonts/`, not tracked in git — see its `README.md`):
  `default.sf2` (GeneralUser GS, the embedded default) plus a light→full A/B set
  (`TimGM6mb.sf2`, `FluidR3_GM.sf2`, `MuseScore_General.sf2`).
- **Mapping config**: `assets/mappings.json` — the image→music rules (editable).

---

## MFSK acoustic modem

The modem encodes a file into a WAV of MFSK tones (with gzip, AES-GCM, Reed-Solomon
FEC and interleaving options) and decodes it back. It runs via dedicated bins:

```sh
# Encode a file to a WAV
cargo run --release --bin modem_encode -- out.wav input.txt --compress --preset robust

# Decode a WAV back to a file (basename defaults to `payload`)
cargo run --release --bin modem_decode -- out.wav recovered

# Channel-simulation and packetization helpers
cargo run --release --bin channel_sim -- in.bin out.bin --mode acoustic
cargo run --release --bin make_packetized -- input.bin out.bin
```

See each bin's `--help` for the full option set.

---

## Configuration file (optional)

Pipeline defaults can be set in `audiohax.toml` in your platform config dir (precedence:
CLI flag > config file > built-in default). Fields: `instruments`, `thickness`,
`steps`, `ms_per_step`, `jitter_percent`, `midi_port`. Point at an explicit file with
`--config <FILE>`.

---

## External MIDI routing

For studio-grade effects, route into Qsynth / FluidSynth / a DAW with `--output midi`
or `--midi-virtual`. Full per-OS guide: [`docs/midi-routing.md`](midi-routing.md).
This is output plumbing only — not needed to hear sound.

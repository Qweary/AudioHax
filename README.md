# AudioHax

**Turn an image into music with one command.** AudioHax is a Rust command-line tool that scans an image, maps its colour, brightness, texture and shape to musical parameters (mode, harmony, tempo, rhythm), and plays the result through a built-in pure-Rust synthesizer — no MIDI cabling, no external synth, nothing to wire up. It also ships an MFSK acoustic data modem (encode a file to sound and back).

---

## Quick Start (Linux)

From an image to music in three steps.

```sh
# 1. Clone
git clone https://github.com/Qweary/AudioHax.git
cd AudioHax

# 2. Fetch the default SoundFont (REQUIRED — it is embedded into the binary at build
#    time, and is not stored in git). One-time, ~31 MB.
curl -L -o assets/soundfonts/default.sf2 \
  https://raw.githubusercontent.com/mrbumpy409/GeneralUser-GS/main/GeneralUser-GS.sf2

# 3. Play a bundled example image (builds on first run, then plays)
./play
```

That's it — `./play` builds the latest source and plays the bundled sample image
through the in-process synth. To play your own image:

```sh
./play path/to/your-image.jpg
```

> **`cargo: not found`?** Cargo is installed in your home dir. Run this once per shell:
> `export PATH="$HOME/.cargo/bin:$PATH"`

### The same thing without the wrapper

`./play` just drives `cargo run --release -- play`. The explicit form:

```sh
cargo run --release -- play assets/images/example.jpg
```

### Render to a WAV file (no audio device needed)

Great for a laptop with no/temperamental audio at a conference, or for sharing a file:

```sh
cargo run --release -- render assets/images/example.jpg --wav out.wav
```

This writes `out.wav` (44.1 kHz stereo) offline. Add `--seed <number>` for an
**exactly reproducible** result — the same image + seed always yields a byte-identical
WAV and the identical composition:

```sh
cargo run --release -- render assets/images/example.jpg --wav out.wav --seed 42
./play assets/images/example.jpg --seed 42          # ...and the same seed when playing live
```

### Run the desktop app

AudioHax also has a desktop window — open a picture, press Play, and hear it, with a few simple controls. The window is behind the optional `gui` feature, so you ask for it explicitly.

**One-time setup** is the same as the Quick Start above: clone the repo and fetch the SoundFont (it is built into the binary and is not stored in git).

On **Linux**, install the one audio build dependency first:

```sh
sudo apt-get install -y libasound2-dev     # Debian/Ubuntu; ALSA audio
```

(Windows and macOS need nothing extra.)

Then launch the window:

```sh
cargo run --release --features gui --bin audiohax-gui
```

The first build takes a few minutes; after that it opens straight away. Drop a picture onto the window (or use **File ▸ Open**), press **Play**, and **Save WAV** keeps your piece.

> **Note (Linux):** the Open and Save file windows use your desktop's file portal. On a minimal setup without one, drag a picture onto the window instead of using Open, and type a path when Save asks for one.

---

## Prerequisites

| Need | Why | Install (Debian/Ubuntu/Kali) |
|---|---|---|
| **Rust + Cargo** | builds the tool | [rustup.rs](https://rustup.rs) |
| **`libasound2-dev`** | the only Linux *build* dep — the audio (cpal) + MIDI (midir) backends link ALSA | `sudo apt install libasound2-dev` |
| **`default.sf2`** | the SoundFont is embedded into the binary at build time; **the build fails without it** | see Quick Start step 2 |

That is the whole list for the default build. It is **pure Rust** — you do **not** need
OpenCV, libclang, FluidSynth, loopMIDI, or any virtual-MIDI driver to build or to hear
sound. (Those are only relevant to the optional routes below.)

- **macOS / Windows:** install Rust + fetch `default.sf2`, then `./play` / `cargo run --release -- play <image>`. No system audio dev package is needed (CoreAudio / WASAPI are built in).

---

## What you can do

- **Image → live music** — `./play <image>` (or `cargo run --release -- play <image>`).
  Plays through the built-in synth, zero routing.
- **Image → WAV (offline render)** — `cargo run --release -- render <image> --wav out.wav`.
  Deterministic with `--seed`; honours `--soundfont` / `--reverb` / `--gain`.
- **Swap the instrument sound (no rebuild)** — `--soundfont <path/to/font.sf2>`.
  rustysynth is SF2-only (no SF3/compressed). A few fonts are staged under
  `assets/soundfonts/` — see that directory's `README.md`.
- **Shape the performance** — `--instruments <n>` (default 4), `--ms-per-step <ms>`
  (scan tempo, default 250), `--steps <n>` (default 40), `--reverb on|off`,
  `--gain <f32>`. Run `./play --help` or `cargo run --release -- play --help` for the full list.
- **Acoustic data modem** — encode a file to a WAV of MFSK tones and decode it back,
  with FEC and channel-simulation options, via the dedicated bins:
  ```sh
  cargo run --release --bin modem_encode -- out.wav secret.txt --compress
  cargo run --release --bin modem_decode -- out.wav recovered
  ```
  `audiohax modem …` appears in `--help` but is not wired yet: it prints a pointer and exits
  without writing anything. Use the `modem_encode` / `modem_decode` bins above.

### Optional: route into an external synth/DAW for studio-grade effects

The built-in synth is General-MIDI sample playback with reverb/chorus on by default.
For your own sampled instruments + effects, route AudioHax's note events into Qsynth /
FluidSynth / a DAW with `--output midi` (or `--midi-virtual` on Linux/macOS). This is
output plumbing only — you do **not** need it to hear sound. Full guide:
[`docs/midi-routing.md`](docs/midi-routing.md).

---

## Tweaking the music

The image→music mapping lives in [`assets/mappings.json`](assets/mappings.json) and is
plain JSON you can edit: hue → musical mode (Phrygian/Lydian/Ionian/…), saturation →
harmonic complexity, brightness → tempo, and more. Change a value, re-run `./play`, and
the same image speaks differently.

Where the mappings come from: `--mappings <file>` if you give it; otherwise
`./assets/mappings.json` in the directory you run from, if it exists; otherwise a copy built
into the binary. If something is at that path but cannot be read (a broken link, a folder),
you get an error rather than the built-in copy. `--preset default` always uses the built-in
copy. Every mappings file is checked before anything is composed (size, tempo range, section
count, MIDI program numbers), and a file that fails says which value is wrong.

---

## More usage

`docs/USAGE.md` has the full subcommand and flag reference. Quick pointers:

```sh
./play --help                              # the friendly wrapper's help
cargo run --release -- play --help         # every play flag
cargo run --release -- render --help       # every render flag
```

---

## License

MIT — see [LICENSE](LICENSE). Default SoundFont: *GeneralUser GS* by S. Christian
Collins (schristiancollins.com), used under the GeneralUser GS License.

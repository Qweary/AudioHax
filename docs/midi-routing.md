# Routing AudioHax MIDI into a real synth (Qsynth / FluidSynth / DAW)

The built-in `--output synth` path is **dry** — a bare in-process SoundFont with no
reverb or chorus. To judge the music properly (and to use your own synths/effects),
route AudioHax's note events into an external MIDI engine with `--output midi`.

This is **output plumbing only**: AudioHax emits the same NoteEvents either way; the
external engine just renders them through better synths + effects.

> All examples assume you are in the repo root and the toolchain is on your `PATH`
> (`export PATH="$HOME/.cargo/bin:$PATH"`). `cargo run` (no `--bin`) resolves to the
> `audiohax` app via `default-run`.

---

## 1. See what's available

```sh
cargo run -- play --list-midi-ports
```

Prints each MIDI output port as `[index] name` and exits without playing. The `index`
and any substring of the `name` are both valid selectors for `--midi-port`.

---

## 2. Linux — virtual port (the smooth path, recommended)

AudioHax can **create its own virtual MIDI port** (ALSA sequencer) that your synth
subscribes to — no pre-existing port required.

```sh
# Terminal A — start AudioHax with a virtual port named "AudioHaxOut".
cargo run -- play assets/images/example.jpg --midi-virtual
#   …or name it explicitly:  --midi-virtual MyPort
```

```sh
# Terminal B — start FluidSynth with reverb + chorus enabled, pointed at a SoundFont.
fluidsynth -a alsa -o synth.reverb.active=1 -o synth.chorus.active=1 /path/to/soundfont.sf2
```

Then wire AudioHax's port to FluidSynth's input. Either use **Qsynth**'s graphical
connections tab, or `aconnect` on the command line:

```sh
aconnect -l                       # list clients; find "AudioHaxOut" and "FLUID Synth"
aconnect 'AudioHaxOut' 'FLUID Synth'   # connect by name (or use the numeric client:port)
```

`--midi-virtual` forces `--output midi`, so you do not also need to pass `--output midi`.

---

## 3. Linux — connect to an existing port

Start your synth first (Qsynth/FluidSynth/a DAW) so its input port exists, then select
it by **name substring** or **numeric index**:

```sh
cargo run -- play assets/images/example.jpg --output midi --midi-port FLUID   # substring
cargo run -- play assets/images/example.jpg --output midi --midi-port 0       # index
```

With no `--midi-port` (and no `--midi-virtual`), AudioHax connects to the **first**
available port. You can also set `$AUDIOHAX_MIDI_PORT` as the default selector.

---

## 4. Windows — loopMIDI / a DAW

midir cannot create virtual ports on Windows, so `--midi-virtual` errors there with
guidance. Instead:

1. Install **loopMIDI** and create a port (e.g. `loopMIDI Port`).
2. Point your DAW's MIDI **input** at that loopMIDI port (and load an instrument with
   reverb/chorus).
3. Run AudioHax into it by name substring:

```sh
cargo run -- play example.jpg --output midi --midi-port loopMIDI
```

---

## 5. macOS — IAC Driver / virtual

`--midi-virtual` works on macOS (CoreMIDI):

```sh
cargo run -- play example.jpg --midi-virtual
```

Then subscribe your synth/DAW to the `AudioHaxOut` source. Alternatively, enable the
**IAC Driver** in *Audio MIDI Setup*, point your DAW at it, and use:

```sh
cargo run -- play example.jpg --output midi --midi-port IAC
```

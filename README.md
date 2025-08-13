# AudioHax

AudioHax is a proof-of-concept project that converts images into music by scanning and mapping pixel data to musical parameters, applying music theory logic, and outputting MIDI to a software synthesizer in real time.

---

## Features
- Image-to-MIDI mapping using OpenCV.
- Multi-instrument concurrent playback.
- Configurable SoundFonts with FluidSynth.
- Basic music theory–driven chord mapping.

---

## Requirements
- **Rust** (Cargo) – [Install Rust](https://rustup.rs)
- **OpenCV**  
  - Windows: Use prebuilt binaries from [OpenCV Releases](https://github.com/opencv/opencv/releases)  
  - macOS: Install via Homebrew (`brew install opencv`)  
  - Linux: Install via package manager (e.g., `sudo apt install libopencv-dev`)
- **loopMIDI** (Windows only) – [Download loopMIDI](https://www.tobias-erichsen.de/software/loopmidi.html)
- **Virtual MIDI Driver**  
  - macOS: [Built-in IAC Driver](https://support.apple.com/en-us/guide/audio-midi-setup/ams7cadc6d1/mac)  
  - Linux: ALSA MIDI (`sudo modprobe snd_virmidi`)
- **FluidSynth** – [FluidSynth Downloads](https://github.com/FluidSynth/fluidsynth/releases) or `brew install fluidsynth` / `sudo apt install fluidsynth`
- A General MIDI–compatible `.sf2` SoundFont file  
  Example: [GeneralUser GS](https://schristiancollins.com/generaluser.php)

---

## Project Structure

AudioHax/
│ Cargo.toml
│ .gitignore
├───assets
│  └───images
|    └───example.jpg
├───src
│ └───main.rs
│ └───image_analysis.rs
│ └───image_source.rs
│ └───chord_engine.rs
│ └───mapping_loader.rs
│ └───midi_output.rs

---

## Installation

### Windows
1. Install Rust & Cargo:
   ```powershell
   rustup-init.exe
2. Install OpenCV:

    Extract to C:\opencv

    Set environment variables:

        setx OPENCV_DIR "C:\opencv\build"
        setx PATH "$($Env:PATH);C:\opencv\build\x64\vc15\bin"

3. Install loopMIDI and create a port named AudioHaxOut.

4. Install FluidSynth (ensure fluidsynth.exe is in PATH).

---

### macOS

1. Install Rust:

    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

2. Install OpenCV:

    brew install opencv

3. Enable IAC Driver:

    Open Audio MIDI Setup → Window > Show MIDI Studio → Double-click IAC Driver → Enable device.

4. Install FluidSynth:

    brew install fluidsynth

---

### Linux (Debian/Ubuntu example)

1. Install Rust:

    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

2. Install OpenCV:

    sudo apt install libopencv-dev

3. Enable Virtual MIDI:

    sudo modprobe snd_virmidi

4. Install FluidSynth:

    sudo apt install fluidsynth

5. Install libclang:

   sudo apt install llvm-dev libclang-dev clang

6. Install ALSA:

   sudo apt install libasound2-dev

---

## Building

   cargo clean
   cargo update
   cargo build --release

---

## Running

---

### Start loopMIDI

Set "New port-name:" as "AudioHaxOut"

Click + to add port

Leave running

---

### Start FluidSynth

Replace PATH_TO_SF2 with your .sf2 file:

Windows:

   fluidsynth -a dsound -p AudioHaxOut -m winmidi "PATH_TO_SF2"

macOS:

  fluidsynth -a coreaudio -o midi.driver=coremidi -o midi.coremidi.id="AudioHaxOut" "PATH_TO_SF2"

Linux:

   fluidsynth -a alsa -o midi.driver=alsa_seq -o midi.alsa_seq.device=AudioHaxOut "PATH_TO_SF2"

---

### Run AudioHax

cargo run --release -- play

---

## How It Works

1. Image Loading – The image is read from assets/images/example.jpg.

2. Analysis – Pixel data is scanned and mapped to note/chord information.

3. Chord Engine – Music theory logic decides what notes/chords to play.

4. MIDI Output – Notes are sent to the virtual MIDI port.

5. Audio Rendering – FluidSynth plays the notes using your SoundFont.

---

## Data Flow Diagram

        ┌───────────────────────┐
        │   Input Image (.jpg)  │
        └───────────┬───────────┘
                    │
                    ▼
        ┌───────────────────────┐
        │ image_source.rs        │
        │ - Loads image          │
        │ - Prepares scan        │
        └───────────┬───────────┘
                    │
                    ▼
        ┌───────────────────────┐
        │ image_analysis.rs      │
        │ - Scans regions        │
        │ - Maps pixels → notes  │
        └───────────┬───────────┘
                    │
                    ▼
        ┌───────────────────────┐
        │ chord_engine.rs        │
        │ - Music theory logic   │
        └───────────┬───────────┘
                    │
                    ▼
        ┌───────────────────────┐
        │ midi_output.rs         │
        │ - Sends to MIDI port   │
        └───────────┬───────────┘
                    │
                    ▼
   loopMIDI / IAC / ALSA Virtual MIDI
                    │
                    ▼
         FluidSynth → Audio Output

---

## License

### MIT License – See LICENSE file for details.

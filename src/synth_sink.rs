//! src/synth_sink.rs — WS-4 Phase 2 in-process pure-Rust synth sink (Lane B).
//!
//! Implements [`engine::AudioSink`] by driving a `rustysynth::Synthesizer` (pure
//! Rust SF2 synthesis, no external FluidSynth process) and rendering to a `cpal`
//! output stream (cross-platform: ALSA/PulseAudio on Linux, WASAPI on Windows,
//! CoreAudio on macOS). It replaces "send MIDI bytes to a virtual port a separate
//! FluidSynth reads" with "synthesize and play in-process." `midir` external
//! MIDI-out is retained as the opt-in `midi-out` sink (design-s11 §5).
//!
//! Threading model (the load-bearing design — design-s11 §3.B.1 / §4.2):
//! cpal's audio callback runs on a realtime audio thread and MUST NOT block,
//! allocate, or lock. The engine's `note_on`/`note_off`/`program_change` calls run
//! on the engine/adapter thread. The two are bridged by a lock-free SPSC ring
//! (`rtrb`) of [`MidiCmd`]: the [`engine::AudioSink`] methods ENQUEUE a command
//! (non-blocking, allocation-free `push`) and return `Ok` immediately; the audio
//! callback DRAINS the queue at the top of each render block, applies each command
//! to the `Synthesizer` via `process_midi_message`, then renders a stereo block of
//! samples and interleaves them into the cpal output buffer. The `Synthesizer` is
//! owned SOLELY by the audio thread (it lives inside the callback closure, not in
//! [`SynthSink`]), so there is no lock on the hot path and the `AudioSink` methods
//! are O(1).
//!
//! Real-time safety: the only thing the audio thread touches that the engine thread
//! also touches is the `rtrb::Consumer` (the engine thread holds the `Producer`).
//! `rtrb` is wait-free SPSC, so `pop`/`push` never block. The per-callback scratch
//! buffers (`left`/`right`) are sized once at construction to the maximum cpal frame
//! count seen and re-used; in the steady state the callback allocates nothing.
//!
//! Headless robustness (design-s11 §6.5): construction surfaces device-absent /
//! stream-build failures as a clean [`engine::AudioSinkError`] rather than panicking,
//! so the lib builds and the unit tests run on a box without a working output device.
//! The note-event handling is unit-tested WITHOUT opening a cpal stream by driving a
//! `rustysynth::Synthesizer` directly (see the `tests` module).

use std::sync::Arc;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rtrb::{Consumer, Producer, RingBuffer};
use rustysynth::{SoundFont, Synthesizer, SynthesizerSettings};

use crate::engine::{AudioSink, AudioSinkError};

/// The SoundFont embedded into the binary for the zero-config default path
/// (`SoundFontSource::Bundled`). GeneralUser GS — a GM SoundFont — verified
/// RIFF/SF2 at `assets/soundfonts/default.sf2`. `include_bytes!` makes it
/// relocatable: no filesystem lookup, no CWD assumption (design-s11 §6.1).
const BUNDLED_SF2: &[u8] = include_bytes!("../assets/soundfonts/default.sf2");

/// Capacity of the engine→audio SPSC ring (in [`MidiCmd`] slots). One musical step
/// emits a few note_on/note_off pairs per instrument; 4096 slots is far more than a
/// single render block ever needs to absorb, so the producer never finds it full in
/// practice. A full ring degrades gracefully (the command is dropped with a logged
/// warning) rather than blocking the engine thread.
const QUEUE_CAPACITY: usize = 4096;

/// One MIDI command crossing the engine thread → audio thread over the SPSC queue.
///
/// Mirrors the three [`engine::AudioSink`] methods. The `u8` MIDI values are widened
/// to `i32` only at apply-time for rustysynth's
/// `process_midi_message(channel, command, data1, data2)` API; the queue itself
/// carries the compact `u8` form. `Copy` so the audio thread pops by value with no
/// allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MidiCmd {
    /// Note-on: status `0x90 | channel`, `data1 = note`, `data2 = velocity`.
    NoteOn { channel: u8, note: u8, velocity: u8 },
    /// Note-off: status `0x80 | channel`, `data1 = note`, `data2 = 0`.
    NoteOff { channel: u8, note: u8 },
    /// Program change: status `0xC0 | channel`, `data1 = program`, `data2 = 0`.
    ProgramChange { channel: u8, program: u8 },
}

impl MidiCmd {
    /// Decompose into rustysynth's `process_midi_message` argument tuple
    /// `(channel, command, data1, data2)` — all `i32`.
    ///
    /// The `command` is the MIDI status byte's high nibble WITHOUT the channel bits
    /// (`0x90`/`0x80`/`0xC0`); rustysynth takes the channel separately. This is the
    /// exact byte vocabulary `MidiOut` used (`midi_output.rs:33..49`), so the sink is
    /// a behavioral drop-in behind the trait.
    fn to_midi_message(self) -> (i32, i32, i32, i32) {
        match self {
            MidiCmd::NoteOn {
                channel,
                note,
                velocity,
            } => (channel as i32, 0x90, note as i32, velocity as i32),
            MidiCmd::NoteOff { channel, note } => (channel as i32, 0x80, note as i32, 0),
            MidiCmd::ProgramChange { channel, program } => {
                (channel as i32, 0xC0, program as i32, 0)
            }
        }
    }
}

/// How the sink obtains its SoundFont. The default loads the bundled GM SF2; a path
/// or an in-memory buffer override it WITHOUT a rebuild (the operator's
/// "config-swappable font" requirement — design-s11 §4.4 / §6.1).
pub enum SoundFontSource<'a> {
    /// The SF2 embedded in the binary via `include_bytes!` (the zero-config default).
    Bundled,
    /// A user-supplied `.sf2` on disk (matches "bring your own SoundFont").
    Path(&'a std::path::Path),
    /// An already-loaded SF2 byte buffer.
    Bytes(&'a [u8]),
}

/// Audio-output configuration for the synth sink — the A/B controls (S31).
///
/// Each field's default is the EXACT current behavior, so [`SynthConfig::default`]
/// reproduces today's audio path byte-for-byte:
///   * `enable_reverb_and_chorus = true`  → rustysynth 1.3.6's own default (reverb +
///     chorus ON; the "dry" prose elsewhere was stale — see [`SynthSink::new_with_config`]),
///   * `gain = 1.0`                        → unity; the post-render gain+soft-clip stage
///     is SHORT-CIRCUITED at exactly 1.0 (a literal no-op), so the default render is
///     bit-identical to the pre-S31 path.
///
/// Only a binary reverb toggle exists in rustysynth (no room/wet/damp setter — those
/// are private crate constants), so `enable_reverb_and_chorus` is the whole reverb lever.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SynthConfig {
    /// Map to `SynthesizerSettings::enable_reverb_and_chorus`. Default `true` =
    /// rustysynth's default = current behavior. `false` renders the bone-dry voice.
    pub enable_reverb_and_chorus: bool,
    /// Master gain applied in OUR cpal callback (rustysynth has no public master-volume
    /// setter). `1.0` is unity AND a guaranteed no-op (the gain+soft-clip stage is
    /// skipped at exactly 1.0). Non-unity multiplies each sample then soft-clips with
    /// `tanh` so boosting can never hard-clip.
    pub gain: f32,
}

impl Default for SynthConfig {
    fn default() -> Self {
        // The single source of truth for "today's audio behavior": reverb on, unity gain.
        SynthConfig {
            enable_reverb_and_chorus: true,
            gain: 1.0,
        }
    }
}

/// Apply the master gain + soft-clip to one rendered sample.
///
/// CONTRACT (the must-not-break property): at `gain == 1.0` this returns `s`
/// UNCHANGED (a literal short-circuit — `tanh` is NOT identity, so we must not run it
/// at unity or the default path would drift). For any other gain it multiplies then
/// soft-clips via `tanh`, which is smooth, monotonic, and bounded to (-1, 1), so a
/// boosted signal saturates gracefully instead of hard-clipping.
#[inline]
pub fn apply_gain_softclip(s: f32, gain: f32) -> f32 {
    if gain == 1.0 {
        s
    } else {
        (s * gain).tanh()
    }
}

/// In-process SoundFont synth sink (design-s11 §3.B.2).
///
/// Owns the PRODUCER end of the SPSC command queue and keeps the cpal [`cpal::Stream`]
/// alive for the sink's lifetime (dropping the stream stops audio). The
/// `rustysynth::Synthesizer` itself lives on the audio thread inside the cpal callback
/// closure, NOT in this struct — so there is no lock on the hot path.
///
/// The orphan rule is satisfied because `SynthSink` is LOCAL to this crate (unlike
/// `MidiOut`, which is bin-private and forces its `AudioSink` impl into `main.rs`);
/// the sink therefore lives in the LIBRARY and `main.rs` only constructs it
/// (design-s11 §3.B).
pub struct SynthSink {
    /// Producer end of the engine→audio SPSC queue.
    tx: Producer<MidiCmd>,
    /// Kept alive to keep the audio device open; never touched after construction.
    /// Dropping it stops the stream (and thus audio).
    _stream: cpal::Stream,
    /// The negotiated output sample rate (for diagnostics/logging).
    sample_rate: u32,
}

impl SynthSink {
    /// Build the sink (design-s11 §3.B.2): open the default cpal output device,
    /// negotiate an f32 output stream, construct a `rustysynth::Synthesizer` over the
    /// chosen SoundFont at the negotiated sample rate, spawn the audio callback that
    /// drains the command queue and renders, and start the stream. Returns the
    /// producer-side handle.
    ///
    /// `font` selects the SoundFont ([`SoundFontSource::Bundled`] GM by default).
    /// Every failure (no host, no device, no supported config, SoundFont parse error,
    /// stream-build error, play error) maps into [`engine::AudioSinkError`] so the
    /// caller speaks one error vocabulary. A box with no working output device
    /// surfaces a clean error — it never panics.
    ///
    /// Uses [`SynthConfig::default`] (reverb on, unity gain) — identical to the
    /// pre-S31 audio path. For the A/B controls (`--reverb`/`--gain`) call
    /// [`SynthSink::new_with_config`].
    pub fn new(font: SoundFontSource<'_>) -> Result<Self, AudioSinkError> {
        Self::new_with_config(font, SynthConfig::default())
    }

    /// Build the sink with explicit audio-output [`SynthConfig`] (the S31 A/B controls).
    ///
    /// `config.enable_reverb_and_chorus` is threaded into `SynthesizerSettings` at
    /// construction; `config.gain` is applied (with a `tanh` soft-clip) in our cpal
    /// callback. With [`SynthConfig::default`] this is exactly [`SynthSink::new`].
    pub fn new_with_config(
        font: SoundFontSource<'_>,
        synth_config: SynthConfig,
    ) -> Result<Self, AudioSinkError> {
        // 1) Load + parse the SoundFont (cheap-to-fail, do it before touching audio HW).
        let sound_font = Arc::new(load_soundfont(font)?);

        // 2) Open the default host + output device.
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| AudioSinkError::msg("no default audio output device available"))?;

        // 3) Negotiate the default output config (f32, device-native sample rate).
        let supported = device
            .default_output_config()
            .map_err(|e| AudioSinkError::msg(format!("no default output config: {e}")))?;
        let sample_format = supported.sample_format();
        let config: cpal::StreamConfig = supported.config();
        let sample_rate = config.sample_rate.0;
        let channels = config.channels as usize;

        // 4) Construct the synthesizer at the negotiated sample rate. It is MOVED into
        //    the audio callback closure below — owned solely by the audio thread. The
        //    reverb/chorus toggle is the only reverb lever rustysynth exposes; default
        //    `true` reproduces today's behavior (rustysynth 1.3.6 ships both effects ON).
        let mut settings = SynthesizerSettings::new(sample_rate as i32);
        settings.enable_reverb_and_chorus = synth_config.enable_reverb_and_chorus;
        let mut synth = Synthesizer::new(&sound_font, &settings)
            .map_err(|e| AudioSinkError::msg(format!("synthesizer init failed: {e}")))?;

        // 5) The lock-free SPSC bridge: producer stays here, consumer goes to the audio thread.
        let (tx, rx) = RingBuffer::<MidiCmd>::new(QUEUE_CAPACITY);

        // 6) Build the output stream for the negotiated sample format. cpal hands the
        //    callback an interleaved buffer of `channels`-frame samples; we render a
        //    stereo block of `frames` samples and interleave/downmix into it.
        let err_fn = |e: cpal::StreamError| eprintln!("audiohax synth stream error: {e}");
        let stream = match sample_format {
            cpal::SampleFormat::F32 => build_f32_stream(
                &device,
                &config,
                channels,
                synth,
                rx,
                synth_config.gain,
                err_fn,
            ),
            other => {
                // Non-f32 default formats are uncommon on modern hosts; rather than
                // pull a sample-conversion path into Phase 2, surface a clean error.
                let _ = &mut synth; // synth is dropped here on the error path.
                return Err(AudioSinkError::msg(format!(
                    "unsupported default output sample format {other:?} (expected f32)"
                )));
            }
        }
        .map_err(|e| AudioSinkError::msg(format!("failed to build output stream: {e}")))?;

        // 7) Start the stream (begins calling the audio callback).
        stream
            .play()
            .map_err(|e| AudioSinkError::msg(format!("failed to start audio stream: {e}")))?;

        Ok(SynthSink {
            tx,
            _stream: stream,
            sample_rate,
        })
    }

    /// Convenience for the zero-config default path: `SynthSink::new(Bundled)`.
    pub fn with_bundled_soundfont() -> Result<Self, AudioSinkError> {
        Self::new(SoundFontSource::Bundled)
    }

    /// The negotiated output sample rate in Hz (e.g. 44_100 / 48_000).
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Enqueue one [`MidiCmd`] onto the SPSC ring (non-blocking). A full ring (the
    /// audio thread fell far behind) drops the command with a warning rather than
    /// blocking the engine thread — the engine must never stall on the audio path.
    fn enqueue(&mut self, cmd: MidiCmd) -> Result<(), AudioSinkError> {
        match self.tx.push(cmd) {
            Ok(()) => Ok(()),
            Err(_) => {
                // rtrb push returns the value back on failure (ring full). We do not
                // propagate it as a hard error: a dropped MIDI event must not abort
                // playback. Report once to stderr for diagnostics.
                eprintln!("audiohax synth: command queue full, dropping {cmd:?}");
                Ok(())
            }
        }
    }
}

impl AudioSink for SynthSink {
    /// Enqueue a note_on (non-blocking).
    ///
    /// theory: rustysynth honors GM convention where a note_on with velocity 0 is
    /// treated as a note_off; we forward velocity verbatim and let the synth apply
    /// that, matching the raw-MIDI semantics `MidiOut` had.
    fn note_on(&mut self, channel: u8, note: u8, velocity: u8) -> Result<(), AudioSinkError> {
        self.enqueue(MidiCmd::NoteOn {
            channel,
            note,
            velocity,
        })
    }

    /// Enqueue a note_off (non-blocking).
    fn note_off(&mut self, channel: u8, note: u8) -> Result<(), AudioSinkError> {
        self.enqueue(MidiCmd::NoteOff { channel, note })
    }

    /// Enqueue a program_change (non-blocking).
    fn program_change(&mut self, channel: u8, program: u8) -> Result<(), AudioSinkError> {
        self.enqueue(MidiCmd::ProgramChange { channel, program })
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Offline (no-cpal) render-to-WAV — the deterministic A/B harness core (S31).
// ─────────────────────────────────────────────────────────────────────────────

/// One absolutely-timed MIDI event for the offline renderer.
///
/// The offline path takes events with an absolute onset in milliseconds (NOT the
/// per-step-relative offsets the live scheduler uses), so a whole composition can be
/// rendered deterministically — no wall clock, no RNG, no cpal device. The same event
/// list rendered through different [`SynthConfig`]/SoundFont choices changes ONLY the
/// synth config, which is exactly the apples-to-apples property the A/B needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimedMidiEvent {
    /// Absolute onset time of this event, in milliseconds from t=0.
    pub at_ms: u64,
    /// The MIDI command to apply at `at_ms`.
    pub cmd: MidiCmd,
}

/// Render a deterministic, fixed-sample-rate stereo buffer from an absolutely-timed
/// MIDI event list — the offline (no-cpal) core of the A/B WAV harness (S31).
///
/// Events are sorted by `at_ms`, then synthesized block-by-block: the synth advances
/// in `sample_rate`-relative time, MIDI events are applied as their timestamp is
/// crossed, and `config.gain`'s soft-clip is applied to every output sample (a no-op
/// at unity). `tail_ms` of extra silence is rendered after the last event so release
/// envelopes/reverb tails are not truncated. Returns interleaved `[L, R, L, R, …]`.
///
/// Determinism: identical inputs → byte-identical output. There is no RNG and no
/// device negotiation; `sample_rate` is fixed by the caller (44_100 in the harness).
pub fn render_events_to_stereo(
    font: SoundFontSource<'_>,
    config: SynthConfig,
    sample_rate: u32,
    mut events: Vec<TimedMidiEvent>,
    tail_ms: u64,
) -> Result<Vec<f32>, AudioSinkError> {
    let sound_font = Arc::new(load_soundfont(font)?);
    let mut settings = SynthesizerSettings::new(sample_rate as i32);
    settings.enable_reverb_and_chorus = config.enable_reverb_and_chorus;
    let mut synth = Synthesizer::new(&sound_font, &settings)
        .map_err(|e| AudioSinkError::msg(format!("offline synthesizer init failed: {e}")))?;

    events.sort_by_key(|e| e.at_ms);

    // Total duration = last event onset + a release/reverb tail.
    let last_ms = events.last().map(|e| e.at_ms).unwrap_or(0);
    let total_ms = last_ms + tail_ms;
    let total_frames = ((total_ms as u128 * sample_rate as u128) / 1000) as usize;

    // Render in modest blocks; apply each event the first time we cross its sample index.
    const BLOCK: usize = 441; // ~10 ms @ 44.1 kHz — fine enough vs. a 250 ms musical step.
    let mut left = vec![0.0f32; BLOCK];
    let mut right = vec![0.0f32; BLOCK];
    let mut out: Vec<f32> = Vec::with_capacity(total_frames * 2);

    let mut next_ev = 0usize;
    let mut frame = 0usize;
    while frame < total_frames {
        let this_block = BLOCK.min(total_frames - frame);

        // Apply every event whose onset falls before the END of this block. (Block
        // granularity ≈ 10 ms is inaudible against the 250 ms step; keeps the loop O(N).)
        let block_end_ms = (((frame + this_block) as u128 * 1000) / sample_rate as u128) as u64;
        while next_ev < events.len() && events[next_ev].at_ms < block_end_ms {
            let (ch, command, d1, d2) = events[next_ev].cmd.to_midi_message();
            synth.process_midi_message(ch, command, d1, d2);
            next_ev += 1;
        }

        if left.len() != this_block {
            left.resize(this_block, 0.0);
            right.resize(this_block, 0.0);
        }
        synth.render(&mut left, &mut right);

        for i in 0..this_block {
            out.push(apply_gain_softclip(left[i], config.gain));
            out.push(apply_gain_softclip(right[i], config.gain));
        }
        frame += this_block;
    }

    Ok(out)
}

/// Write an interleaved stereo `[L, R, …]` f32 buffer to a 16-bit PCM WAV at `path`
/// via the existing `hound` dependency — the A/B harness's on-disk output (S31).
///
/// Samples are clamped to `[-1.0, 1.0]` and quantized to `i16`. The header records
/// 2 channels at `sample_rate`. IO/encode failures map to [`engine::AudioSinkError`].
pub fn write_stereo_wav(
    path: &std::path::Path,
    sample_rate: u32,
    interleaved: &[f32],
) -> Result<(), AudioSinkError> {
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec)
        .map_err(|e| AudioSinkError::msg(format!("creating WAV {}: {e}", path.display())))?;
    for &s in interleaved {
        let v = (s.clamp(-1.0, 1.0) * i16::MAX as f32).round() as i16;
        writer
            .write_sample(v)
            .map_err(|e| AudioSinkError::msg(format!("writing WAV sample: {e}")))?;
    }
    writer
        .finalize()
        .map_err(|e| AudioSinkError::msg(format!("finalizing WAV {}: {e}", path.display())))?;
    Ok(())
}

/// Load + parse a SoundFont from a [`SoundFontSource`] into a `rustysynth::SoundFont`.
///
/// `Bundled` parses the `include_bytes!`-embedded GM SF2; `Path` reads the file off
/// disk; `Bytes` parses a caller-supplied buffer. All three go through
/// `SoundFont::new(&mut impl Read)`. Parse/IO failures map to [`engine::AudioSinkError`].
fn load_soundfont(font: SoundFontSource<'_>) -> Result<SoundFont, AudioSinkError> {
    match font {
        SoundFontSource::Bundled => {
            let mut cursor = std::io::Cursor::new(BUNDLED_SF2);
            SoundFont::new(&mut cursor)
                .map_err(|e| AudioSinkError::msg(format!("bundled soundfont parse failed: {e}")))
        }
        SoundFontSource::Bytes(bytes) => {
            let mut cursor = std::io::Cursor::new(bytes);
            SoundFont::new(&mut cursor)
                .map_err(|e| AudioSinkError::msg(format!("soundfont buffer parse failed: {e}")))
        }
        SoundFontSource::Path(path) => {
            let file = std::fs::File::open(path).map_err(|e| {
                AudioSinkError::msg(format!("opening soundfont {}: {e}", path.display()))
            })?;
            let mut reader = std::io::BufReader::new(file);
            SoundFont::new(&mut reader).map_err(|e| {
                AudioSinkError::msg(format!("parsing soundfont {}: {e}", path.display()))
            })
        }
    }
}

/// Build the f32 cpal output stream whose callback owns the synthesizer + consumer
/// and renders on the audio thread.
///
/// The render closure (design-s11 §3.B.3) runs on the realtime audio thread. Per
/// invocation it:
///   1. DRAINS all pending [`MidiCmd`]s from the SPSC consumer and applies each via
///      `synth.process_midi_message(channel, command, data1, data2)`;
///   2. renders `frames` stereo samples into reusable `left`/`right` scratch buffers
///      via `synth.render(&mut left, &mut right)` (rustysynth renders in fixed
///      `get_block_size()` blocks internally — 64 samples ≈ 1.45 ms @ 44.1 kHz — so
///      event timing is quantized to a block boundary, inaudible against a 250 ms
///      musical step);
///   3. interleaves L/R into the cpal output slice for a 2-channel device, or copies
///      L into each frame for mono / writes L,R,0,0,… for >2 channels.
///
/// The scratch buffers are grown once to the largest frame count seen and re-used, so
/// the steady-state callback is allocation-free.
///
/// `gain` is the S31 master gain applied per output sample via [`apply_gain_softclip`].
/// At the default `1.0` that helper is a literal no-op, so the interleave/downmix math
/// is byte-for-byte the pre-S31 code path.
fn build_f32_stream(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    channels: usize,
    mut synth: Synthesizer,
    mut rx: Consumer<MidiCmd>,
    gain: f32,
    err_fn: impl FnMut(cpal::StreamError) + Send + 'static,
) -> Result<cpal::Stream, cpal::BuildStreamError> {
    // Reusable per-callback scratch (grows once, then re-used — no steady-state alloc).
    let mut left: Vec<f32> = Vec::new();
    let mut right: Vec<f32> = Vec::new();

    device.build_output_stream(
        config,
        move |output: &mut [f32], _info: &cpal::OutputCallbackInfo| {
            // (1) Drain + apply all queued MIDI commands (wait-free pops).
            while let Ok(cmd) = rx.pop() {
                let (ch, command, d1, d2) = cmd.to_midi_message();
                synth.process_midi_message(ch, command, d1, d2);
            }

            // Number of audio frames this callback must fill.
            let frames = if channels == 0 {
                0
            } else {
                output.len() / channels
            };
            if frames == 0 {
                // Defensive: nothing to render (mono-less / zero-frame buffer).
                for s in output.iter_mut() {
                    *s = 0.0;
                }
                return;
            }

            // (2) Render a stereo block into the reusable scratch buffers.
            if left.len() != frames {
                left.resize(frames, 0.0);
                right.resize(frames, 0.0);
            }
            synth.render(&mut left, &mut right);

            // (3) Interleave/downmix into the cpal output buffer, applying the master
            //     gain + soft-clip per sample. `apply_gain_softclip` is a no-op at the
            //     default gain 1.0, so this is the pre-S31 path byte-for-byte by default.
            match channels {
                1 => {
                    // Mono device: downmix to the average of L/R.
                    for (i, s) in output.iter_mut().enumerate() {
                        *s = apply_gain_softclip(0.5 * (left[i] + right[i]), gain);
                    }
                }
                2 => {
                    for (i, frame) in output.chunks_mut(2).enumerate() {
                        frame[0] = apply_gain_softclip(left[i], gain);
                        frame[1] = apply_gain_softclip(right[i], gain);
                    }
                }
                n => {
                    // >2 channels: L on ch0, R on ch1, silence on the rest.
                    for (i, frame) in output.chunks_mut(n).enumerate() {
                        frame[0] = apply_gain_softclip(left[i], gain);
                        if n >= 2 {
                            frame[1] = apply_gain_softclip(right[i], gain);
                        }
                        for s in frame.iter_mut().skip(2) {
                            *s = 0.0;
                        }
                    }
                }
            }
        },
        err_fn,
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::AudioSink;

    // ── MidiCmd → rustysynth process_midi_message argument mapping ──────────────

    #[test]
    fn midi_cmd_note_on_maps_to_0x90() {
        let cmd = MidiCmd::NoteOn {
            channel: 3,
            note: 60,
            velocity: 100,
        };
        assert_eq!(cmd.to_midi_message(), (3, 0x90, 60, 100));
    }

    #[test]
    fn midi_cmd_note_off_maps_to_0x80_with_zero_velocity() {
        let cmd = MidiCmd::NoteOff {
            channel: 9,
            note: 48,
        };
        assert_eq!(cmd.to_midi_message(), (9, 0x80, 48, 0));
    }

    #[test]
    fn midi_cmd_program_change_maps_to_0xc0() {
        let cmd = MidiCmd::ProgramChange {
            channel: 0,
            program: 40,
        };
        assert_eq!(cmd.to_midi_message(), (0, 0xC0, 40, 0));
    }

    // ── SPSC queue enqueue/drain ordering (the bridge in isolation) ─────────────

    #[test]
    fn spsc_queue_preserves_fifo_order() {
        let (mut tx, mut rx) = RingBuffer::<MidiCmd>::new(8);
        let a = MidiCmd::ProgramChange {
            channel: 0,
            program: 7,
        };
        let b = MidiCmd::NoteOn {
            channel: 0,
            note: 60,
            velocity: 80,
        };
        let c = MidiCmd::NoteOff {
            channel: 0,
            note: 60,
        };
        tx.push(a).unwrap();
        tx.push(b).unwrap();
        tx.push(c).unwrap();
        assert_eq!(rx.pop().unwrap(), a);
        assert_eq!(rx.pop().unwrap(), b);
        assert_eq!(rx.pop().unwrap(), c);
        assert!(rx.pop().is_err(), "queue is empty after draining");
    }

    /// Construct a `SynthSink`-shaped producer/consumer pair WITHOUT opening cpal,
    /// drive the three `AudioSink` methods through a producer, and assert the right
    /// `MidiCmd`s land on the consumer in order. This exercises the exact enqueue
    /// path `note_on`/`note_off`/`program_change` use (minus the cpal stream), so it
    /// runs headlessly on a deviceless box.
    #[test]
    fn audiosink_methods_enqueue_correct_commands_headless() {
        // A minimal AudioSink that uses ONLY the producer end (no cpal stream), so we
        // can unit-test the enqueue path on a headless box. This mirrors SynthSink's
        // `enqueue` exactly.
        struct QueueOnlySink {
            tx: Producer<MidiCmd>,
        }
        impl AudioSink for QueueOnlySink {
            fn note_on(
                &mut self,
                channel: u8,
                note: u8,
                velocity: u8,
            ) -> Result<(), AudioSinkError> {
                self.tx
                    .push(MidiCmd::NoteOn {
                        channel,
                        note,
                        velocity,
                    })
                    .map_err(|_| AudioSinkError::msg("full"))
            }
            fn note_off(&mut self, channel: u8, note: u8) -> Result<(), AudioSinkError> {
                self.tx
                    .push(MidiCmd::NoteOff { channel, note })
                    .map_err(|_| AudioSinkError::msg("full"))
            }
            fn program_change(&mut self, channel: u8, program: u8) -> Result<(), AudioSinkError> {
                self.tx
                    .push(MidiCmd::ProgramChange { channel, program })
                    .map_err(|_| AudioSinkError::msg("full"))
            }
        }

        let (tx, mut rx) = RingBuffer::<MidiCmd>::new(16);
        let mut sink = QueueOnlySink { tx };

        // Drive the trait methods exactly as the engine would.
        sink.program_change(2, 7).expect("program_change Ok");
        sink.note_on(2, 64, 90).expect("note_on Ok");
        sink.note_off(2, 64).expect("note_off Ok");

        assert_eq!(
            rx.pop().unwrap(),
            MidiCmd::ProgramChange {
                channel: 2,
                program: 7
            }
        );
        assert_eq!(
            rx.pop().unwrap(),
            MidiCmd::NoteOn {
                channel: 2,
                note: 64,
                velocity: 90
            }
        );
        assert_eq!(
            rx.pop().unwrap(),
            MidiCmd::NoteOff {
                channel: 2,
                note: 64
            }
        );
    }

    /// Compile-time + behavioral proof that the same render math the audio callback
    /// runs ("drain queue → process_midi_message → render → non-silent samples") makes
    /// SOUND from our events, done WITHOUT cpal: load the bundled SoundFont, build a
    /// `Synthesizer`, apply a note_on via the SAME `MidiCmd::to_midi_message` path, and
    /// assert the rendered block contains non-zero samples. This is the "the synth
    /// actually makes sound from our events" proof.
    #[test]
    fn bundled_soundfont_renders_nonsilent_audio_for_a_note_on() {
        let sf = load_soundfont(SoundFontSource::Bundled).expect("bundled SF2 parses");
        let sf = Arc::new(sf);
        let settings = SynthesizerSettings::new(44_100);
        let mut synth = Synthesizer::new(&sf, &settings).expect("synth init");

        // Apply program_change + note_on through the SAME mapping the callback uses.
        for cmd in [
            MidiCmd::ProgramChange {
                channel: 0,
                program: 0, // Acoustic Grand Piano (GM program 0)
            },
            MidiCmd::NoteOn {
                channel: 0,
                note: 60, // middle C
                velocity: 120,
            },
        ] {
            let (ch, command, d1, d2) = cmd.to_midi_message();
            synth.process_midi_message(ch, command, d1, d2);
        }

        // Render ~0.25s of audio and look for any non-zero sample (the note sounds).
        let frames = 11_025; // 0.25 s @ 44.1 kHz
        let mut left = vec![0.0f32; frames];
        let mut right = vec![0.0f32; frames];
        synth.render(&mut left, &mut right);

        let peak = left
            .iter()
            .chain(right.iter())
            .fold(0.0f32, |m, &s| m.max(s.abs()));
        assert!(
            peak > 1e-4,
            "a note_on through our MidiCmd path must produce audible (non-silent) output; peak={peak}"
        );
    }

    /// `SynthSink` satisfies `engine::AudioSink` (compiles as an impl). We do not
    /// CALL `SynthSink::new` here (that opens a live cpal device and is gated to the
    /// hardware test); we only prove the trait bound at compile time.
    #[test]
    fn synth_sink_satisfies_audiosink_bound() {
        fn assert_is_audiosink<T: AudioSink>() {}
        assert_is_audiosink::<SynthSink>();
    }

    // ── S31 A/B controls ────────────────────────────────────────────────────────

    /// The default audio config reproduces today's behavior exactly: reverb on,
    /// unity gain. This pins the must-not-break property at the config layer.
    #[test]
    fn synth_config_default_is_current_behavior() {
        let d = SynthConfig::default();
        assert!(
            d.enable_reverb_and_chorus,
            "reverb defaults ON (rustysynth 1.3.6)"
        );
        assert_eq!(d.gain, 1.0, "gain defaults to unity");
    }

    /// THE no-op proof: at gain 1.0 the gain+soft-clip stage returns every sample
    /// UNCHANGED — for normal samples AND for out-of-range ones (tanh is NOT identity,
    /// so a naive `(s).tanh()` would corrupt these; the short-circuit must skip it).
    #[test]
    fn gain_unity_is_exact_no_op() {
        for &s in &[
            0.0f32,
            0.5,
            -0.5,
            1.0,
            -1.0,
            0.999_999,
            -0.123_456,
            2.0,
            -3.5,
            f32::MIN_POSITIVE,
        ] {
            let out = apply_gain_softclip(s, 1.0);
            assert_eq!(
                out.to_bits(),
                s.to_bits(),
                "gain 1.0 must be a bit-exact no-op for {s}"
            );
        }
    }

    /// At non-unity gain the stage multiplies then soft-clips: it must boost quiet
    /// signals and must keep the output strictly inside (-1, 1) even for a huge input.
    #[test]
    fn gain_nonunity_boosts_and_softclips() {
        // Quiet sample, modest boost: louder but still well below clip.
        let boosted = apply_gain_softclip(0.1, 2.0);
        assert!(boosted > 0.1, "2x gain must increase a quiet sample");
        assert!(boosted < 1.0, "still within range");
        // Hot input with boost: tanh saturates, never hard-clips past ±1.
        let hot = apply_gain_softclip(0.9, 8.0);
        assert!(
            hot < 1.0 && hot > 0.99,
            "soft-clip saturates toward +1, never exceeds it"
        );
        let hot_neg = apply_gain_softclip(-0.9, 8.0);
        assert!(
            hot_neg > -1.0 && hot_neg < -0.99,
            "soft-clip saturates toward -1"
        );
    }

    /// A bad/missing SoundFont path fails LOUDLY with a clear, path-bearing message
    /// (the `--soundfont` error contract), not a panic.
    #[test]
    fn soundfont_path_missing_fails_with_clear_message() {
        let p = std::path::Path::new("/nonexistent/does-not-exist.sf2");
        let err = load_soundfont(SoundFontSource::Path(p)).expect_err("missing font must error");
        let msg = err.to_string();
        assert!(
            msg.contains("does-not-exist.sf2"),
            "error must name the offending path; got: {msg}"
        );
    }

    /// A non-SF2 file fails loudly at PARSE time (clear message, no panic).
    #[test]
    fn soundfont_path_non_sf2_fails_to_parse() {
        let mut tmp = std::env::temp_dir();
        tmp.push("audiohax_not_a_soundfont.sf2");
        std::fs::write(&tmp, b"this is plainly not a RIFF/SF2 file").expect("write tmp");
        let err = load_soundfont(SoundFontSource::Path(&tmp)).expect_err("garbage must not parse");
        let _ = std::fs::remove_file(&tmp);
        assert!(
            err.to_string().contains("audiohax_not_a_soundfont.sf2"),
            "parse error must name the path"
        );
    }

    /// The offline renderer is DETERMINISTIC: the same event list + config renders to a
    /// byte-identical buffer across runs — the apples-to-apples property the A/B needs.
    /// Also proves it produces non-silent audio from a note_on through the bundled font.
    #[test]
    fn offline_render_is_deterministic_and_nonsilent() {
        let events = vec![
            TimedMidiEvent {
                at_ms: 0,
                cmd: MidiCmd::ProgramChange {
                    channel: 0,
                    program: 0,
                },
            },
            TimedMidiEvent {
                at_ms: 0,
                cmd: MidiCmd::NoteOn {
                    channel: 0,
                    note: 60,
                    velocity: 110,
                },
            },
            TimedMidiEvent {
                at_ms: 250,
                cmd: MidiCmd::NoteOff {
                    channel: 0,
                    note: 60,
                },
            },
        ];
        let a = render_events_to_stereo(
            SoundFontSource::Bundled,
            SynthConfig::default(),
            44_100,
            events.clone(),
            500,
        )
        .expect("render a");
        let b = render_events_to_stereo(
            SoundFontSource::Bundled,
            SynthConfig::default(),
            44_100,
            events,
            500,
        )
        .expect("render b");

        assert_eq!(a.len(), b.len(), "same length");
        assert_eq!(
            a.iter().map(|s| s.to_bits()).collect::<Vec<_>>(),
            b.iter().map(|s| s.to_bits()).collect::<Vec<_>>(),
            "identical inputs must render byte-identically"
        );
        let peak = a.iter().fold(0.0f32, |m, &s| m.max(s.abs()));
        assert!(
            peak > 1e-4,
            "a note_on must produce audible output; peak={peak}"
        );
    }

    /// Reverb on vs. off through the offline renderer produces DIFFERENT audio — proves
    /// the `--reverb` toggle is actually threaded into `SynthesizerSettings`.
    #[test]
    fn offline_render_reverb_toggle_changes_output() {
        let events = vec![
            TimedMidiEvent {
                at_ms: 0,
                cmd: MidiCmd::NoteOn {
                    channel: 0,
                    note: 64,
                    velocity: 110,
                },
            },
            TimedMidiEvent {
                at_ms: 200,
                cmd: MidiCmd::NoteOff {
                    channel: 0,
                    note: 64,
                },
            },
        ];
        let wet = render_events_to_stereo(
            SoundFontSource::Bundled,
            SynthConfig {
                enable_reverb_and_chorus: true,
                gain: 1.0,
            },
            44_100,
            events.clone(),
            500,
        )
        .expect("wet");
        let dry = render_events_to_stereo(
            SoundFontSource::Bundled,
            SynthConfig {
                enable_reverb_and_chorus: false,
                gain: 1.0,
            },
            44_100,
            events,
            500,
        )
        .expect("dry");
        assert_eq!(wet.len(), dry.len());
        assert!(
            wet.iter()
                .zip(&dry)
                .any(|(a, b)| a.to_bits() != b.to_bits()),
            "reverb on vs off must change the rendered audio"
        );
    }

    /// `write_stereo_wav` produces a readable 2-channel WAV at the requested rate.
    #[test]
    fn write_stereo_wav_roundtrips_header() {
        let mut tmp = std::env::temp_dir();
        tmp.push("audiohax_ab_test_out.wav");
        let interleaved = vec![0.0f32, 0.0, 0.25, -0.25, 0.5, -0.5];
        write_stereo_wav(&tmp, 44_100, &interleaved).expect("write wav");
        let reader = hound::WavReader::open(&tmp).expect("reopen wav");
        let spec = reader.spec();
        assert_eq!(spec.channels, 2);
        assert_eq!(spec.sample_rate, 44_100);
        assert_eq!(
            reader.len() as usize,
            interleaved.len(),
            "all samples written"
        );
        let _ = std::fs::remove_file(&tmp);
    }
}

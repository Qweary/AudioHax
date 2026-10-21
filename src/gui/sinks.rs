//! The live sinks the window plays through, and MIDI port enumeration.
//!
//! Lead ruling L-3 (carried) and L-11: ANY GUI sink silences everything on Stop and on Drop, as
//! the CLI's `MidiOut` does, and that includes closing the window mid-play. Two layers do it:
//!
//! * [`SilencingSink`] wraps any [`AudioSink`], remembers every note it switched on and not yet
//!   off, and on [`SilencingSink::silence`] / `Drop` sends a note-off for each. It wraps both
//!   the in-process synth and the MIDI port.
//! * [`GuiMidiSink`] additionally sends the MIDI "panic" (CC 123 All Notes Off + CC 120 All Sound
//!   Off on all 16 channels, the exact bytes `MidiOut::all_sound_off` sends) on Drop, so an
//!   EXTERNAL synth is never left sounding.
//!
//! The in-process synth (`synth_sink::SynthSink`, read-only this session) has no `Drop` of its
//! own; dropping it drops its cpal stream, which stops the audio device outright. The note-offs
//! [`SilencingSink`] sends first let held notes enter their release before that happens.
//!
//! MIDI enumeration lives here because `midi_output.rs` is bin-private and not writable this
//! session (SH HL-8, L-11). It is a thin enumerate-and-connect: the window connects with the port
//! handle from the SAME enumeration the user picked from (SH N-2), never an index resolved later.

use crate::engine::{AudioSink, AudioSinkError};
use midir::{MidiOutput, MidiOutputConnection, MidiOutputPort};
use std::collections::BTreeSet;

/// The client name the window registers with the OS MIDI system.
const MIDI_CLIENT_NAME: &str = "audiohax_gui";

/// Wraps a sink and silences every note it started, on [`SilencingSink::silence`] and on `Drop`.
pub struct SilencingSink<S: AudioSink> {
    inner: S,
    held: BTreeSet<(u8, u8)>,
}

impl<S: AudioSink> SilencingSink<S> {
    /// Wrap `inner`.
    pub fn new(inner: S) -> Self {
        SilencingSink {
            inner,
            held: BTreeSet::new(),
        }
    }

    /// Notes switched on and not yet off, as `(channel, note)`.
    pub fn held(&self) -> impl Iterator<Item = &(u8, u8)> {
        self.held.iter()
    }

    /// Send a note-off for every held note (errors ignored: this runs on Stop and in `Drop`).
    pub fn silence(&mut self) {
        for (channel, note) in std::mem::take(&mut self.held) {
            let _ = self.inner.note_off(channel, note);
        }
    }

    /// Borrow the wrapped sink.
    pub fn inner(&self) -> &S {
        &self.inner
    }
}

impl<S: AudioSink> AudioSink for SilencingSink<S> {
    fn note_on(&mut self, channel: u8, note: u8, velocity: u8) -> Result<(), AudioSinkError> {
        // A note-on with velocity 0 is a note-off in MIDI (and in rustysynth).
        if velocity == 0 {
            self.held.remove(&(channel, note));
        } else {
            self.held.insert((channel, note));
        }
        self.inner.note_on(channel, note, velocity)
    }

    fn note_off(&mut self, channel: u8, note: u8) -> Result<(), AudioSinkError> {
        self.held.remove(&(channel, note));
        self.inner.note_off(channel, note)
    }

    fn program_change(&mut self, channel: u8, program: u8) -> Result<(), AudioSinkError> {
        self.inner.program_change(channel, program)
    }
}

impl<S: AudioSink> Drop for SilencingSink<S> {
    fn drop(&mut self) {
        self.silence();
    }
}

/// The MIDI "panic" bytes: for each channel 0..16, CC 123 (All Notes Off) then CC 120 (All Sound
/// Off). The same 32 messages, in the same order, as `midi_output::all_sound_off_messages` (which
/// is bin-private, so it cannot be called from the library).
pub fn all_sound_off_messages() -> Vec<[u8; 3]> {
    let mut msgs = Vec::with_capacity(32);
    for ch in 0u8..16 {
        let status = 0xB0 | (ch & 0x0F);
        msgs.push([status, 123, 0]);
        msgs.push([status, 120, 0]);
    }
    msgs
}

/// An external MIDI destination the user picked, carried to the worker with the very handle the
/// enumeration produced (SH N-2).
pub struct MidiTarget {
    output: MidiOutput,
    port: MidiOutputPort,
    /// The port's name (raw; sanitise before display).
    pub name: String,
}

/// One port as the output selector shows it.
pub struct MidiPortEntry {
    /// The enumeration index (shown as `0: name`, like `--list-midi-ports`).
    pub index: usize,
    /// Raw name from the OS (sanitise before display).
    pub name: String,
    port: MidiOutputPort,
}

/// The ports from ONE enumeration, plus the `MidiOutput` that produced them.
pub struct MidiPorts {
    output: Option<MidiOutput>,
    /// The ports, in the order midir reported them. Empty is a valid, displayable result.
    pub entries: Vec<MidiPortEntry>,
}

impl MidiPorts {
    /// Enumerate the output ports now. An empty list is not an error.
    pub fn enumerate() -> Result<Self, String> {
        let output = MidiOutput::new(MIDI_CLIENT_NAME).map_err(|e| e.to_string())?;
        let mut entries = Vec::new();
        for (index, port) in output.ports().into_iter().enumerate() {
            // A port that vanished between listing and naming is skipped, not an error.
            if let Ok(name) = output.port_name(&port) {
                entries.push(MidiPortEntry { index, name, port });
            }
        }
        Ok(MidiPorts {
            output: Some(output),
            entries,
        })
    }

    /// Take the target for entry `i`, consuming this enumeration's `MidiOutput` (connecting
    /// consumes it in midir). The selector re-enumerates afterwards.
    pub fn take_target(&mut self, i: usize) -> Option<MidiTarget> {
        let entry = self.entries.iter().find(|e| e.index == i)?;
        let port = entry.port.clone();
        let name = entry.name.clone();
        let output = self.output.take()?;
        Some(MidiTarget { output, port, name })
    }
}

/// The external-MIDI sink: raw note/program bytes to a connection, the panic on Drop.
pub struct GuiMidiSink {
    conn: MidiOutputConnection,
}

impl GuiMidiSink {
    /// Connect to the target the user picked.
    pub fn connect(target: MidiTarget) -> Result<Self, String> {
        let conn = target
            .output
            .connect(&target.port, "audiohax_gui_conn")
            .map_err(|e| e.to_string())?;
        Ok(GuiMidiSink { conn })
    }

    fn send(&mut self, bytes: &[u8]) -> Result<(), AudioSinkError> {
        self.conn
            .send(bytes)
            .map_err(|e| AudioSinkError::msg(e.to_string()))
    }
}

impl AudioSink for GuiMidiSink {
    fn note_on(&mut self, channel: u8, note: u8, velocity: u8) -> Result<(), AudioSinkError> {
        self.send(&[0x90 | (channel & 0x0F), note, velocity])
    }

    fn note_off(&mut self, channel: u8, note: u8) -> Result<(), AudioSinkError> {
        self.send(&[0x80 | (channel & 0x0F), note, 0])
    }

    fn program_change(&mut self, channel: u8, program: u8) -> Result<(), AudioSinkError> {
        self.send(&[0xC0 | (channel & 0x0F), program])
    }
}

impl Drop for GuiMidiSink {
    fn drop(&mut self) {
        for msg in all_sound_off_messages() {
            let _ = self.conn.send(&msg);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// Records every call, so the silencing can be checked without a device.
    #[derive(Clone, Default)]
    struct Recorder(Arc<Mutex<Vec<String>>>);

    impl AudioSink for Recorder {
        fn note_on(&mut self, c: u8, n: u8, v: u8) -> Result<(), AudioSinkError> {
            self.0
                .lock()
                .map_err(|_| AudioSinkError::msg("poisoned"))?
                .push(format!("on {c} {n} {v}"));
            Ok(())
        }
        fn note_off(&mut self, c: u8, n: u8) -> Result<(), AudioSinkError> {
            self.0
                .lock()
                .map_err(|_| AudioSinkError::msg("poisoned"))?
                .push(format!("off {c} {n}"));
            Ok(())
        }
        fn program_change(&mut self, c: u8, p: u8) -> Result<(), AudioSinkError> {
            self.0
                .lock()
                .map_err(|_| AudioSinkError::msg("poisoned"))?
                .push(format!("pc {c} {p}"));
            Ok(())
        }
    }

    #[test]
    fn drop_releases_every_held_note_and_only_those() {
        let log = Recorder::default();
        {
            let mut s = SilencingSink::new(log.clone());
            s.program_change(0, 5).expect("pc");
            s.note_on(0, 60, 100).expect("on");
            s.note_on(1, 64, 90).expect("on");
            s.note_on(2, 67, 80).expect("on");
            s.note_off(1, 64).expect("off");
            s.note_on(2, 67, 0).expect("vel-0 is an off");
            assert_eq!(s.held().count(), 1);
        } // Drop
        let calls = log.0.lock().expect("log").clone();
        assert_eq!(calls.last().map(String::as_str), Some("off 0 60"));
        assert_eq!(calls.iter().filter(|c| c.starts_with("off")).count(), 2);
    }

    #[test]
    fn stop_silences_then_drop_sends_nothing_more() {
        let log = Recorder::default();
        let n_after_stop;
        {
            let mut s = SilencingSink::new(log.clone());
            s.note_on(3, 50, 70).expect("on");
            s.silence();
            n_after_stop = log.0.lock().expect("log").len();
        }
        assert_eq!(log.0.lock().expect("log").len(), n_after_stop);
    }

    #[test]
    fn panic_bytes_match_the_cli_all_sound_off() {
        let m = all_sound_off_messages();
        assert_eq!(m.len(), 32);
        for (i, msg) in m.iter().enumerate() {
            let ch = (i / 2) as u8;
            assert_eq!(msg[0], 0xB0 | ch);
            assert_eq!(msg[1], if i % 2 == 0 { 123 } else { 120 });
            assert_eq!(msg[2], 0);
        }
    }

    #[test]
    fn enumeration_never_panics_and_empty_is_fine() {
        // On a CI box with no MIDI system this may be an error or an empty list; both are fine.
        if let Ok(mut ports) = MidiPorts::enumerate() {
            assert!(ports.take_target(usize::MAX).is_none());
        }
    }
}

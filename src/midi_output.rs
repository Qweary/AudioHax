use midir::{MidiOutput, MidiOutputConnection};
// WS-4 S12: brings `MidiOutput::create_virtual` into scope on Unix (Linux ALSA /
// macOS CoreMIDI). midir 0.8 gates the whole `os::unix` module by `#[cfg(unix)]`
// internally — there is NO `virtual` cargo feature to enable (design-s12 §6.3).
#[cfg(unix)]
use midir::os::unix::VirtualOutput;
use std::error::Error;
use std::thread;
use std::time::Duration;

/// The actionable message shown when no usable MIDI output destination exists or a
/// selector matches nothing — points the operator at the smooth `--midi-virtual`
/// path (Unix) or an external port (loopMIDI/IAC/Qsynth).
const NO_PORTS_MSG: &str = "No MIDI output ports available. Use `--midi-virtual` \
    (Linux/macOS) to create one, or create a virtual MIDI port (loopMIDI/IAC) and run \
    FluidSynth/Qsynth, then select it with `--midi-port`.";

pub struct MidiOut {
    conn: MidiOutputConnection,
}

impl MidiOut {
    /// Connect to the first existing port (or first name-substring match). RETAINED for
    /// back-compat/fallback; the runtime path now uses [`open_selector`] (substring OR
    /// numeric index). `#[allow(dead_code)]` because S12 promoted `midi-out` into the
    /// default feature set, which surfaces this otherwise-unreferenced kept method in the
    /// default build.
    ///
    /// [`open_selector`]: MidiOut::open_selector
    #[allow(dead_code)]
    pub fn open_first(port_name_hint: Option<&str>) -> Result<Self, Box<dyn Error>> {
        let midi_out = MidiOutput::new("acoustic_art_out")?;
        let ports = midi_out.ports();
        if ports.is_empty() {
            return Err(NO_PORTS_MSG.into());
        }
        // choose the first matching or first port
        let mut chosen = None;
        if let Some(hint) = port_name_hint {
            for p in &ports {
                let name = midi_out.port_name(p)?;
                if name.contains(hint) {
                    chosen = Some(p.clone());
                    break;
                }
            }
        }
        let port = chosen.unwrap_or_else(|| ports[0].clone());
        let conn = midi_out.connect(&port, "acoustic_art_conn")?;
        Ok(MidiOut { conn })
    }

    /// List the available MIDI output ports as `(index, name)` pairs, in the order
    /// midir reports them. The `index` is the 0-based selector accepted by
    /// [`MidiOut::open_selector`]. Opens a transient [`MidiOutput`] to query; does not
    /// connect to any port. Static (no `&self`). Returns an empty `Vec` when no ports
    /// exist (which is itself a valid, displayable result — distinct from an error).
    pub fn list_ports() -> Result<Vec<(usize, String)>, Box<dyn Error>> {
        let midi_out = MidiOutput::new("acoustic_art_out")?;
        let ports = midi_out.ports();
        let mut out = Vec::with_capacity(ports.len());
        for (i, p) in ports.iter().enumerate() {
            out.push((i, midi_out.port_name(p)?));
        }
        Ok(out)
    }

    /// Connect to an EXISTING output port chosen by `selector`:
    ///   * if `selector` parses as a `usize`, treat it as a 0-based port index;
    ///   * otherwise treat it as a case-sensitive name substring (as [`open_first`]);
    ///   * if `selector` is `None`, fall back to the first available port.
    ///
    /// Errors (with the same actionable message family as [`open_first`]) when no ports
    /// exist, when the index is out of range, or when the substring matches nothing.
    ///
    /// [`open_first`]: MidiOut::open_first
    pub fn open_selector(selector: Option<&str>) -> Result<Self, Box<dyn Error>> {
        let midi_out = MidiOutput::new("acoustic_art_out")?;
        let ports = midi_out.ports();
        if ports.is_empty() {
            return Err(NO_PORTS_MSG.into());
        }

        let port = match selector {
            // Numeric → index branch.
            Some(s) if s.parse::<usize>().is_ok() => {
                let idx = s.parse::<usize>().unwrap_or(0);
                match ports.get(idx) {
                    Some(p) => p.clone(),
                    None => {
                        return Err(format!(
                            "MIDI port index {idx} is out of range (only {} port(s) available; \
                             run `--list-midi-ports`).",
                            ports.len()
                        )
                        .into());
                    }
                }
            }
            // Non-numeric → substring branch.
            Some(s) => {
                let mut chosen = None;
                for p in &ports {
                    if midi_out.port_name(p)?.contains(s) {
                        chosen = Some(p.clone());
                        break;
                    }
                }
                match chosen {
                    Some(p) => p,
                    None => {
                        return Err(format!(
                            "No MIDI output port matched '{s}' (run `--list-midi-ports` to see \
                             available ports)."
                        )
                        .into());
                    }
                }
            }
            // None → first port.
            None => ports[0].clone(),
        };

        let conn = midi_out.connect(&port, "acoustic_art_conn")?;
        Ok(MidiOut { conn })
    }

    /// Create a virtual MIDI output port named `name` that a DAW / Qsynth / FluidSynth
    /// can subscribe to — no pre-existing port required. Supported on Unix (Linux ALSA /
    /// macOS CoreMIDI). `create_virtual` CONSUMES the [`MidiOutput`] and yields the
    /// [`MidiOutputConnection`] directly (no separate `.connect()`).
    #[cfg(unix)]
    pub fn open_virtual(name: &str) -> Result<Self, Box<dyn Error>> {
        let midi_out = MidiOutput::new("acoustic_art_out")?;
        let conn = midi_out.create_virtual(name)?;
        Ok(MidiOut { conn })
    }

    /// Windows stub: midir does not support creating virtual ports on Windows. Returns
    /// an actionable error pointing the operator at loopMIDI + `--midi-port`. Exists so
    /// the `main.rs` selection branch is identical across platforms and only the body
    /// differs by cfg.
    #[cfg(not(unix))]
    pub fn open_virtual(name: &str) -> Result<Self, Box<dyn Error>> {
        let _ = name; // name is meaningful only on the unix path
        Err(
            "Virtual MIDI ports are not supported on Windows. Install loopMIDI, create a \
             port, then run with `--output midi --midi-port <loopMIDI port name>`."
                .into(),
        )
    }

    pub fn program_change(&mut self, channel: u8, program: u8) -> Result<(), Box<dyn Error>> {
        let status = 0xC0 | (channel & 0x0F);
        self.conn.send(&[status, program])?;
        Ok(())
    }

    pub fn note_on(&mut self, channel: u8, note: u8, velocity: u8) -> Result<(), Box<dyn Error>> {
        let status = 0x90 | (channel & 0x0F);
        self.conn.send(&[status, note, velocity])?;
        Ok(())
    }

    pub fn note_off(&mut self, channel: u8, note: u8) -> Result<(), Box<dyn Error>> {
        let status = 0x80 | (channel & 0x0F);
        self.conn.send(&[status, note, 0])?;
        Ok(())
    }

    /// Emit a thorough "MIDI panic" across the connection: for every channel 0..16,
    /// send CC 123 (All Notes Off) AND CC 120 (All Sound Off). CC 123 releases held
    /// notes (they enter their release tail); CC 120 cuts any sound still in its
    /// release tail. Sending both guarantees nothing keeps sounding in an EXTERNAL
    /// synth (Qsynth/FluidSynth/DAW) after this process goes away.
    ///
    /// Send errors are returned (non-fatal-friendly) so callers can decide; `Drop`
    /// ignores them. The exact bytes are produced by the pure [`all_sound_off_messages`]
    /// so they can be unit-tested without opening a real MIDI port.
    pub fn all_sound_off(&mut self) -> Result<(), Box<dyn Error>> {
        for msg in all_sound_off_messages() {
            self.conn.send(&msg)?;
        }
        Ok(())
    }

    /// Play a simple arpeggio for a chord on a channel. Pre-existing helper, unused by
    /// the engine driver (the adapter schedules note_on/note_off directly);
    /// `#[allow(dead_code)]` because S12's default-feature promotion now surfaces it.
    #[allow(dead_code)]
    pub fn play_chord_arpeggio(
        &mut self,
        channel: u8,
        notes: &[u8],
        velocity: u8,
        note_len_ms: u64,
    ) -> Result<(), Box<dyn Error>> {
        for &n in notes {
            self.note_on(channel, n, velocity)?;
            thread::sleep(Duration::from_millis(note_len_ms));
            self.note_off(channel, n)?;
        }
        Ok(())
    }
}

/// PURE byte generator for the "MIDI panic": the exact wire bytes [`MidiOut::all_sound_off`]
/// sends, with no I/O. Returns 32 messages = 16 channels (0..16) × 2 CCs, in this order
/// per channel: CC 123 (All Notes Off) = `[0xB0 | ch, 123, 0]`, then CC 120 (All Sound
/// Off) = `[0xB0 | ch, 120, 0]`. Status nibble `0xB0` is Control Change; the low nibble
/// carries the channel. Pure + deterministic so it can be unit-tested headlessly.
pub fn all_sound_off_messages() -> Vec<[u8; 3]> {
    let mut msgs = Vec::with_capacity(32);
    for ch in 0u8..16 {
        let status = 0xB0 | (ch & 0x0F);
        msgs.push([status, 123, 0]); // All Notes Off
        msgs.push([status, 120, 0]); // All Sound Off
    }
    msgs
}

/// Best-effort flush on ANY scope exit (normal return, early `?`, or a graceful break
/// out of the playback loop after a Ctrl-C). `Drop` cannot return a `Result`, so a send
/// failure here is logged and swallowed — by the time we are dropping, the connection
/// may already be torn down, and a noisy panic would be worse than a missed flush.
impl Drop for MidiOut {
    fn drop(&mut self) {
        if let Err(e) = self.all_sound_off() {
            eprintln!("MIDI all-sound-off on shutdown failed (non-fatal): {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Byte-level correctness of the "MIDI panic" sent on shutdown ─────────────
    // These pin the exact wire bytes of all_sound_off_messages so a stuck-note
    // regression (an external synth left sounding after an abrupt exit) is caught
    // without opening a real MIDI port. 16 channels × 2 Control-Change messages.

    #[test]
    fn all_sound_off_yields_exactly_32_messages() {
        assert_eq!(all_sound_off_messages().len(), 32);
    }

    #[test]
    fn every_message_is_a_control_change() {
        for msg in all_sound_off_messages() {
            assert_eq!(msg[0] & 0xF0, 0xB0, "status nibble must be Control Change");
        }
    }

    #[test]
    fn cc_numbers_are_only_123_and_120_and_both_present_per_channel() {
        let msgs = all_sound_off_messages();
        // Only CC 123 (All Notes Off) and CC 120 (All Sound Off) may appear.
        for msg in &msgs {
            assert!(
                msg[1] == 123 || msg[1] == 120,
                "unexpected CC number {}",
                msg[1]
            );
        }
        // BOTH CCs must be present for EVERY channel 0..16.
        for ch in 0u8..16 {
            let status = 0xB0 | ch;
            assert!(
                msgs.contains(&[status, 123, 0]),
                "missing CC 123 for channel {ch}"
            );
            assert!(
                msgs.contains(&[status, 120, 0]),
                "missing CC 120 for channel {ch}"
            );
        }
    }

    #[test]
    fn every_data_byte_is_zero() {
        for msg in all_sound_off_messages() {
            assert_eq!(msg[2], 0, "Control Change data byte must be 0");
        }
    }

    #[test]
    fn channel_nibble_covers_all_16_channels() {
        let mut seen = [false; 16];
        for msg in all_sound_off_messages() {
            seen[(msg[0] & 0x0F) as usize] = true;
        }
        assert!(
            seen.iter().all(|&s| s),
            "channel nibble must span all of 0..=15"
        );
    }
}

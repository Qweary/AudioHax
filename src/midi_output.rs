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

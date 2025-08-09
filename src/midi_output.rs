use midir::{MidiOutput, MidiOutputConnection};
use std::error::Error;
use std::thread;
use std::time::Duration;

pub struct MidiOut {
    conn: MidiOutputConnection,
}

impl MidiOut {
    pub fn open_first(port_name_hint: Option<&str>) -> Result<Self, Box<dyn Error>> {
        let midi_out = MidiOutput::new("acoustic_art_out")?;
        let ports = midi_out.ports();
        if ports.is_empty() {
            return Err("No MIDI output ports available. Create a virtual MIDI port (loopMIDI/IAC/virtual) and run FluidSynth/Qsynth.".into());
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

    /// Play a simple arpeggio for a chord on a channel
    pub fn play_chord_arpeggio(&mut self, channel: u8, notes: &[u8], velocity: u8, note_len_ms: u64) -> Result<(), Box<dyn Error>> {
        for &n in notes {
            self.note_on(channel, n, velocity)?;
            thread::sleep(Duration::from_millis(note_len_ms));
            self.note_off(channel, n)?;
        }
        Ok(())
    }
}

mod mapping_loader;
mod chord_engine;
mod midi_output;

use mapping_loader::{load_mappings, lookup_range_map};
use chord_engine::ChordEngine;
use midi_output::MidiOut;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load mappings
    let mappings = load_mappings("assets/mappings.json")?;
    println!("Mappings loaded.");

    // --- Simulated image-derived features (replace with real analysis)
    let hue: f32 = 100.0; // 0..360
    let edge_complexity: f32 = 0.8; // 0.0..1.0
    let brightness_drop: f32 = 0.3; // 0.0..1.0

    // Lookup mode from hue map
    let mode = lookup_range_map(&mappings.global.hue_to_mode, hue)
        .unwrap_or_else(|| "Ionian".to_string());
    println!("Chosen mode from hue {} -> {}", hue, mode);

    // Build chord engine
    let engine = ChordEngine::new(mappings);

    // Pick progression
    let progression = engine.pick_progression(&mode);
    println!("Picked progression (Roman): {:?}", progression);

    // Generate chords (root = C4 = 60) - later make root dependent on global analysis
    let chords = engine.generate_chords(&progression, 60, &mode, edge_complexity, brightness_drop);

    println!("Generated chords:");
    for ch in &chords {
        println!("{:?}", ch);
    }

    // If user passes "play" arg, open MIDI and play chords
    let args: Vec<String> = env::args().collect();
    if args.iter().any(|a| a == "play") {
        let mut midi = MidiOut::open_first(None)?;
        // set program for channel 0 to Acoustic Grand Piano (program 0)
        midi.program_change(0, 0)?;
        for ch in chords {
            println!("Playing chord {}", ch.name);
            // simple velocity mapping
            midi.play_chord_arpeggio(0, &ch.notes, 90, 250)?;
        }
    } else {
        println!("Run with `cargo run --release -- play` to send chords to a MIDI port.");
    }

    Ok(())
}

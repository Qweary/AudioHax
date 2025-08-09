mod mapping_loader;
mod chord_engine;
mod midi_output;

// NEW MODULES
mod image_source;
mod image_analysis;

use mapping_loader::{load_mappings, lookup_range_map};
use chord_engine::ChordEngine;
use midi_output::MidiOut;
use std::env;

// NEW IMPORTS
use image_source::{load_image_from_source, ImageSource};
use image_analysis::{analyze_global, analyze_scan_bar};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load mappings
    let mappings = load_mappings("assets/mappings.json")?;
    println!("Mappings loaded.");

    // --- Image selection ---
    // Change this to pick other sources:
    // ImageSource::UserPath("path/to/image.jpg".to_string())
    // ImageSource::CameraIndex(0)
    let src = ImageSource::Preselected("example.jpg".to_string());

    // Load image
    let img = load_image_from_source(&src)?;
    println!("Image loaded from source.");

    // --- Run image analysis ---
    let global_features = analyze_global(&img)?;
    println!("Global features: {:?}", global_features);

    // For demo: analyze scan bar into 6 vertical sections
    let scan_features = analyze_scan_bar(&img, 6, true)?;
    println!("Scan bar features: {:?}", scan_features);

    // --- Feature extraction for chord engine ---
    // Use avg_hue from global features
    let hue: f32 = global_features.avg_hue; // 0..360

    // Map edge_density (0..1) to "edge_complexity"
    let edge_complexity: f32 = global_features.edge_density;

    // Use first scan bar's brightness as "baseline" and compute drop from global brightness
    let first_bar_brightness = scan_features.first().map(|b| b.avg_brightness).unwrap_or(global_features.avg_brightness);
    let brightness_drop: f32 = ((global_features.avg_brightness - first_bar_brightness) / 100.0).abs();

    println!("Derived hue: {}", hue);
    println!("Derived edge_complexity: {}", edge_complexity);
    println!("Derived brightness_drop: {}", brightness_drop);

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

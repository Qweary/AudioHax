mod mapping_loader;
mod chord_engine;
mod midi_output;
mod image_source;
mod image_analysis;

use mapping_loader::{load_mappings, lookup_range_map};
use chord_engine::ChordEngine;
use midi_output::MidiOut;
use std::env;
use image_source::{load_image_from_source, ImageSource};
use image_analysis::{analyze_global, analyze_scan_bar, draw_scan_bar_overlay};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load mappings
    let mappings = load_mappings("assets/mappings.json")?;
    println!("Mappings loaded.");

    // --- CLI arg parsing ---
    let args: Vec<String> = env::args().collect();
    let instrument_count: usize = args.iter()
        .position(|a| a == "--instruments")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(4); // default to 4 instruments

    println!("Instrument count set to {}", instrument_count);

    // --- Image selection ---
    let src = ImageSource::Preselected("example.jpg".to_string());
    let img = load_image_from_source(&src)?;
    println!("Image loaded from source.");

    // --- Run image analysis ---
    let global_features = analyze_global(&img)?;
    println!("Global features: {:?}", global_features);

    // Analyze scan bar into user-specified instrument count
    let scan_features = analyze_scan_bar(&img, instrument_count, true)?;
    println!("Scan bar features: {:?}", scan_features);

    // Draw overlay for debug/display
    let overlay_img = draw_scan_bar_overlay(&img, instrument_count, true)?;
    opencv::highgui::imshow("Scan Bar Overlay", &overlay_img)?;
    opencv::highgui::wait_key(0)?; // wait for key press before continuing

    // --- Feature extraction for chord engine ---
    let hue: f32 = global_features.avg_hue;
    let edge_complexity: f32 = global_features.edge_density;
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
    let progression = engine.pick_progression(&mode);
    println!("Picked progression (Roman): {:?}", progression);

    // Generate chords
    let chords = engine.generate_chords(&progression, 60, &mode, edge_complexity, brightness_drop);
    println!("Generated chords: {:?}", chords);

    // Per-instrument note mapping from scan features
    for (i, section) in scan_features.iter().enumerate() {
        println!("Instrument {} section features: {:?}", i + 1, section);
        // TODO: map section features to actual instrument note events
        // Example: use hue delta to choose chord tone, brightness to set velocity, edge density for rhythm
    }

    // Play if requested
    if args.iter().any(|a| a == "play") {
        let mut midi = MidiOut::open_first(None)?;
        for (i, ch) in chords.iter().enumerate() {
            let channel = i % 16;
            midi.program_change(channel as u8, (i * 5 % 128) as u8)?; // rotate instrument patches
            midi.play_chord_arpeggio(channel as u8, &ch.notes, 90, 250)?;
        }
    } else {
        println!("Run with `cargo run --release -- play` to send chords to a MIDI port.");
    }

    Ok(())
}

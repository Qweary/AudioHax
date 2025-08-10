// src/main.rs
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
    // If user passed an image path as first argument, treat it as a UserPath:
    let src = if let Some(img_path) = args.get(1) {
        // If they explicitly passed "play" (no image param), fall back to example
        if img_path == "play" {
            ImageSource::UserPath("assets/images/example.jpg".to_string())
        } else {
            ImageSource::UserPath(img_path.clone())
        }
    } else {
        // default: use the provided example.jpg in assets/
        ImageSource::UserPath("assets/example.jpg".to_string())
    };

    let img = load_image_from_source(&src)?;
    println!("Image loaded from source.");

    // --- Run image analysis ---
    let global_features = analyze_global(&img)?;
    println!("Global features: {:?}", global_features);

    // Analyze scan bar into user-specified instrument count (vertical = true default)
    let scan_features = analyze_scan_bar(&img, instrument_count, true)?;
    println!("Scan bar features: {:?}", scan_features);

    // Draw overlay for debug/display and save it
    let overlay_img = draw_scan_bar_overlay(&img, instrument_count, true)?;
    // Save overlay to a file we can inspect.
    // If your OpenCV build supports imwrite, this should work.
    let overlay_path = "assets/overlay.png";
    match opencv::imgcodecs::imwrite(overlay_path, &overlay_img, &opencv::core::Vector::new()) {
        Ok(_) => println!("Wrote overlay image to {}", overlay_path),
        Err(e) => println!("Warning: failed to write overlay image: {}", e),
    }
    // Try to show it if GUI support is present; ignore errors (headless friendly)
    if let Err(e) = (|| -> opencv::Result<()> {
        opencv::highgui::named_window("Scan Bar Overlay", opencv::highgui::WINDOW_AUTOSIZE)?;
        opencv::highgui::imshow("Scan Bar Overlay", &overlay_img)?;
        opencv::highgui::wait_key(1)?;
        Ok(())
    })() {
        println!("Note: could not show overlay with highgui (this may be normal): {}", e);
    }

    // --- Feature extraction for chord engine ---
    let hue: f32 = global_features.avg_hue;
    let edge_complexity: f32 = global_features.edge_density;
    let first_bar_brightness = scan_features.first().map(|b| b.avg_brightness).unwrap_or(global_features.avg_brightness);
    let brightness_drop: f32 = ((global_features.avg_brightness - first_bar_brightness) / 100.0).abs();

    println!("Derived hue: {}", hue);
    println!("Derived edge_complexity: {}", edge_complexity);
    println!("Derived brightness_drop: {}", brightness_drop);

    // Lookup mode from hue map (mapping_loader::lookup_range_map expects hue)
    let mode = lookup_range_map(&mappings.global.hue_to_mode, hue)
        .unwrap_or_else(|| "Ionian".to_string());
    println!("Chosen mode from hue {} -> {}", hue, mode);

    // Build chord engine
    let engine = ChordEngine::new(mappings);
    let progression = engine.pick_progression(&mode);
    println!("Picked progression (Roman): {:?}", progression);

    // Generate chords (root 60 = Middle C)
    let chords = engine.generate_chords(&progression, 60, &mode, edge_complexity, brightness_drop);
    println!("Generated chords: {:?}", chords);

    // Per-instrument note mapping from scan features (debug print)
    for (i, section) in scan_features.iter().enumerate() {
        println!("Instrument {} section features: {:?}", i + 1, section);
        // TODO: map section features to actual instrument note events (fine granularity)
    }

    // --- Send to MIDI if requested ---
    // Program accepts literal arg "play" (no dashes). Example: `cargo run -- play`
    if args.iter().any(|a| a == "play") {
        // Try to get preferred port name from environment or default to "AudioHaxOut"
        let preferred = std::env::var("AUDIOHAX_MIDI_PORT").ok();
        let preferred_ref = preferred.as_deref().or(Some("AudioHaxOut"));

        println!("Attempting to open MIDI port (preferred = {:?})", preferred_ref);

        // Open MIDI with a preference, falling back to first available.
        let mut midi = MidiOut::open_first(preferred_ref)?;

        // Play chords (each chord on its own channel, rotate channels)
        for (i, ch) in chords.iter().enumerate() {
            let channel = (i % 16) as u8;
            // pick a program/patch heuristically
            let prog = ((i * 5) % 128) as u8;
            midi.program_change(channel as u8, prog)?;
            // Play chord notes with moderate velocity
            midi.play_chord_arpeggio(channel, &ch.notes, 90, 350)?;
        }
        println!("Finished sending MIDI events.");
    } else {
        println!("Run with `cargo run -- play` to send chords to a MIDI port.");
        println!("Or set env AUDIOHAX_MIDI_PORT to select a specific port name.");
    }

    Ok(())
}

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
use std::sync::{Arc, Barrier, Mutex};
use std::thread;
use std::time::Duration;
use image_source::{load_image_from_source, ImageSource};
use image_analysis::{analyze_global, analyze_scan_bar, draw_scan_bar_overlay, scan_image, draw_scan_bar_overlay_for_rect};

// Bring MatTraitConst into scope so .cols()/.rows() are available
use opencv::prelude::MatTraitConst;
use opencv::core;

fn clamp_u8(v: i32) -> u8 {
    if v < 0 { 0 } else if v > 127 { 127 } else { v as u8 }
}

/// Simple mapping: convert ScanBarFeatures -> (midi_note, velocity)
/// - base: base MIDI pitch (C3 = 48)
/// - instrument_offset: spacing per instrument (in semitones)
fn map_features_to_note_velocity(f: &image_analysis::ScanBarFeatures, instrument_idx: usize) -> (u8, u8) {
    let base: i32 = 48; // C3
    let instr_offset = (instrument_idx as i32) * 4; // small separation per instrument
    // brightness: 0..100 -> 0..24 semitones
    let brightness_offset = ((f.avg_brightness.clamp(0.0, 100.0) / 100.0) * 24.0).round() as i32;
    let note = clamp_u8(base + instr_offset + brightness_offset);
    // velocity from saturation: map to 30..120 (so it's not too quiet)
    let vel = ((f.avg_saturation.clamp(0.0, 100.0) / 100.0) * 90.0 + 30.0).round() as i32;
    let vel = if vel < 1 { 1 } else if vel > 127 { 127 } else { vel as u8 };
    (note, vel)
}

/// Play the scanned steps concurrently with worker threads coordinated by a Barrier.
/// - `steps`: Vec<steps> where each step is Vec<ScanBarFeatures> for each instrument
/// - `ms_per_step`: duration of each step in milliseconds
/// - `preferred_midi_port_hint`: optional preferred MIDI port name fragment
fn play_scanned_steps_concurrent(
    steps: Vec<Vec<image_analysis::ScanBarFeatures>>,
    ms_per_step: u64,
    preferred_midi_port_hint: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    if steps.is_empty() {
        println!("No steps to play.");
        return Ok(());
    }
    let num_steps = steps.len();
    let num_instruments = steps[0].len();
    if num_instruments == 0 {
        println!("No instruments found in steps.");
        return Ok(());
    }

    // Validate that each step has the same instrument count
    for (i, step) in steps.iter().enumerate() {
        if step.len() != num_instruments {
            return Err(format!("Step {} has inconsistent instrument count ({} != {})", i, step.len(), num_instruments).into());
        }
    }

    println!("Opening MIDI port...");
    let mut midi = MidiOut::open_first(preferred_midi_port_hint)?;
    println!("MIDI opened.");

    // choose programs/patches for channels 0..(num_instruments-1)
    for i in 0..num_instruments {
        let channel = (i % 16) as u8;
        let program = ((i * 7) % 128) as u8; // simple distribution
        midi.program_change(channel, program)?;
    }

    // Shared container where each worker writes its computed (note, vel) per step.
    // We'll keep a per-instrument slot holding the value for the *current* step.
    let shared_results: Arc<Mutex<Vec<Option<(u8, u8)>>>> = Arc::new(Mutex::new(vec![None; num_instruments]));
    // Barrier to synchronize workers + coordinator (count = workers + coordinator)
    let barrier = Arc::new(Barrier::new(num_instruments + 1));

    // Spawn worker threads
    let mut handles = Vec::new();
    for inst_idx in 0..num_instruments {
        let steps_clone = steps.clone(); // cheap for small number of steps; okay for now
        let br = barrier.clone();
        let results = shared_results.clone();

        let handle = thread::spawn(move || {
            // Worker iterates all steps, computes its (note, vel), writes into shared_results, then wait.
            for step_idx in 0..num_steps {
                let features = &steps_clone[step_idx][inst_idx];
                let (note, vel) = map_features_to_note_velocity(features, inst_idx);
                {
                    let mut guard = results.lock().unwrap();
                    guard[inst_idx] = Some((note, vel));
                }
                // signal ready (1st barrier)
                br.wait();
                // wait for coordinator to send notes and release (2nd barrier)
                br.wait();
                // next iteration
            }
            // Worker thread done
        });
        handles.push(handle);
    }

    // Coordinator loop: for each step, wait for workers to compute, then send MIDI for all, sleep, then send offs, and release workers
    for step_idx in 0..num_steps {
        // wait for workers to write their per-instrument results
        barrier.wait();

        // read results and send note_on for all instruments as quickly as possible
        let mut notes_to_off: Vec<(u8,u8,u8)> = Vec::with_capacity(num_instruments); // (channel,note,vel)
        {
            let guard = shared_results.lock().unwrap();
            for inst_idx in 0..num_instruments {
                if let Some((note, vel)) = guard[inst_idx] {
                    let channel = (inst_idx % 16) as u8;
                    // Send note_on
                    if let Err(e) = midi.note_on(channel, note, vel) {
                        eprintln!("MIDI note_on error for inst {}: {}", inst_idx, e);
                    } else {
                        notes_to_off.push((channel, note, vel));
                    }
                } else {
                    // missing value - skip
                }
            }
        }

        // Sleep for the step duration (notes sound for this long)
        thread::sleep(Duration::from_millis(ms_per_step));

        // Send note_off for all that were turned on
        for (channel, note, _vel) in notes_to_off.iter() {
            if let Err(e) = midi.note_off(*channel, *note) {
                eprintln!("MIDI note_off error: {}", e);
            }
        }

        // Clear shared_results for next step
        {
            let mut guard = shared_results.lock().unwrap();
            for slot in guard.iter_mut() { *slot = None; }
        }

        // Release workers to next iteration
        barrier.wait();
    }

    // Join workers
    for h in handles {
        if let Err(e) = h.join() {
            eprintln!("Worker thread panicked: {:?}", e);
        }
    }

    println!("Completed playback of {} steps.", num_steps);
    Ok(())
}

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

    // runtime scan/play parameters (can be exposed to CLI later)
    let bar_thickness_frac: f32 = 0.10; // 10%
    let num_steps: usize = 40;
    let ms_per_step: u64 = 250;

    println!("Scan bar thickness = {:.2}, steps = {}, ms/step = {}", bar_thickness_frac, num_steps, ms_per_step);

    // --- Image selection ---
    let src = if let Some(img_path) = args.get(1) {
        if img_path == "play" {
            ImageSource::UserPath("assets/images/example.jpg".to_string())
        } else {
            ImageSource::UserPath(img_path.clone())
        }
    } else {
        ImageSource::UserPath("assets/images/example.jpg".to_string())
    };

    let img = load_image_from_source(&src)?;
    println!("Image loaded from source.");

    // --- Quick global and static analysis (compat) ---
    let global_features = analyze_global(&img)?;
    println!("Global features: {:?}", global_features);

    let static_scan_features = analyze_scan_bar(&img, instrument_count, true)?;
    println!("Static scan-bar features (compat): {:?}", static_scan_features);

    // --- Moving scan bar (new) ---
    let steps = scan_image(&img, instrument_count, bar_thickness_frac, num_steps, None)?;
    println!("Completed scanning image. Steps: {}", steps.len());

    if !steps.is_empty() {
        println!("Step 0 features: {:?}", steps.first().unwrap());
        println!("Step {} features: {:?}", steps.len() / 2, &steps[steps.len() / 2]);
        println!("Step {} features: {:?}", steps.len() - 1, steps.last().unwrap());
    }

    // Save overlays for first/middle/last steps for visualization
    let (width, height) = (img.cols(), img.rows());
    let vertical_default = width > height;
    let bar_w = if vertical_default {
        ((width as f32) * bar_thickness_frac).max(1.0).round() as i32
    } else {
        width
    };
    let bar_h = if !vertical_default {
        ((height as f32) * bar_thickness_frac).max(1.0).round() as i32
    } else {
        height
    };

    let step_indices = vec![0usize, steps.len() / 2, steps.len().saturating_sub(1)];
    for &si in &step_indices {
        let x0 = if vertical_default {
            let travel = (width - bar_w).max(0);
            if steps.len() == 1 { 0 } else { ((si as f32) * (travel as f32) / ((steps.len() - 1) as f32)).round() as i32 }
        } else { 0 };
        let y0 = if !vertical_default {
            let travel = (height - bar_h).max(0);
            if steps.len() == 1 { 0 } else { ((si as f32) * (travel as f32) / ((steps.len() - 1) as f32)).round() as i32 }
        } else { 0 };

        let bar_rect = core::Rect::new(x0, y0, if vertical_default { bar_w } else { width }, if vertical_default { height } else { bar_h });
        let overlay = draw_scan_bar_overlay_for_rect(&img, bar_rect, instrument_count, vertical_default)?;
        let out_name = format!("assets/overlay_step_{}.png", si);
        match opencv::imgcodecs::imwrite(&out_name, &overlay, &opencv::core::Vector::new()) {
            Ok(_) => println!("Wrote overlay for step {} to {}", si, out_name),
            Err(e) => println!("Warning: failed to write overlay for step {}: {}", si, e),
        }
    }

    // Playback requested?
    if args.iter().any(|a| a == "play") {
        // Try to get preferred port name from environment or default to "AudioHaxOut"
        let preferred = std::env::var("AUDIOHAX_MIDI_PORT").ok();
        let preferred_ref = preferred.as_deref().or(Some("AudioHaxOut"));

        println!("Attempting concurrent playback (preferred MIDI = {:?})", preferred_ref);

        // Kick off concurrent playback (coordinator + workers inside the function)
        play_scanned_steps_concurrent(steps, ms_per_step, preferred_ref)?;
    } else {
        println!("Run with `cargo run -- play` to send chords to a MIDI port.");
        println!("Or set env AUDIOHAX_MIDI_PORT to select a specific port name.");
    }

    Ok(())
}

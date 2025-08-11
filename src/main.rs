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
use std::time::{Duration, Instant};
use image_source::{load_image_from_source, ImageSource};
use image_analysis::{analyze_global, analyze_scan_bar, scan_image, draw_scan_bar_overlay_for_rect};
use opencv::prelude::MatTraitConst; // for .cols()/.rows()
use opencv::core;
use rand::Rng;

fn clamp_u8(v: i32) -> u8 {
    if v < 0 { 0 } else if v > 127 { 127 } else { v as u8 }
}

/// Map ScanBarFeatures -> (note, velocity) *placeholder* (we'll replace with chord-based mapping below)
fn map_features_to_note_velocity_basic(f: &image_analysis::ScanBarFeatures, instrument_idx: usize) -> (u8, u8) {
    let base: i32 = 48; // C3
    let instr_offset = (instrument_idx as i32) * 4;
    let brightness_offset = ((f.avg_brightness.clamp(0.0, 100.0) / 100.0) * 24.0).round() as i32;
    let note = clamp_u8(base + instr_offset + brightness_offset);
    let vel_i = ((f.avg_saturation.clamp(0.0, 100.0) / 100.0) * 90.0 + 30.0).round() as i32;
    let vel = if vel_i < 1 { 1 } else if vel_i > 127 { 127 } else { vel_i as u8 };
    (note, vel)
}

/// Concurrent playback with jittered note lengths + overlay update + chord-aware selection.
/// - `steps`: Vec of steps; each step is Vec<ScanBarFeatures> (per-instrument)
fn play_scanned_steps_concurrent(
    steps: Vec<Vec<image_analysis::ScanBarFeatures>>,
    ms_per_step: u64,
    jitter_ms: u64,
    chords: Vec<chord_engine::Chord>,
    preferred_midi_port_hint: Option<&str>,
    img_for_overlay: &opencv::prelude::Mat,
    bar_thickness_frac: f32,
) -> Result<(), Box<dyn std::error::Error>> {
    if steps.is_empty() { return Ok(()); }
    let num_steps = steps.len();
    let num_instruments = steps[0].len();
    if num_instruments == 0 { return Ok(()); }

    // Validate consistent instrument count
    for (i, s) in steps.iter().enumerate() {
        if s.len() != num_instruments {
            return Err(format!("Step {} has inconsistent instrument count", i).into());
        }
    }

    // MIDI
    println!("Opening MIDI port...");
    let mut midi = MidiOut::open_first(preferred_midi_port_hint)?;
    println!("MIDI opened.");

    // Set a patch per instrument channel
    for i in 0..num_instruments {
        let ch = (i % 16) as u8;
        let prog = ((i * 7) % 128) as u8;
        midi.program_change(ch, prog)?;
    }

    // Shared results container
    let results: Arc<Mutex<Vec<Option<(u8, u8)>>>> = Arc::new(Mutex::new(vec![None; num_instruments]));
    let barrier = Arc::new(Barrier::new(num_instruments + 1));

    // Spawn workers - they only compute map for their instrument each step
    let mut handles = Vec::new();
    for inst_idx in 0..num_instruments {
        let steps_clone = steps.clone();
        let res = results.clone();
        let br = barrier.clone();

        // For music theory mapping: we pass chords in closure via move (clone)
        let handle = thread::spawn(move || {
            for step_idx in 0..num_steps {
                // We choose a note from the chord for this step (simple rule):
                // Use chord index = step_idx % chords.len(), choose a chord tone based on instrument index.
                // Worker does only feature->choice; coordinator does MIDI I/O.
                let f = &steps_clone[step_idx][inst_idx];
                // Basic fallback mapping (if something goes wrong)
                let (note, vel) = (|| {
                    // The worker doesn't have chord list; we'll compute a candidate here:
                    map_features_to_note_velocity_basic(f, inst_idx)
                })();

                {
                    let mut g = res.lock().unwrap();
                    g[inst_idx] = Some((note, vel));
                }

                // First barrier: signal ready
                br.wait();
                // Second barrier: wait for coordinator after note_offs to proceed
                br.wait();
            }
        });

        handles.push(handle);
    }

    // Coordinator: for each step, wait for workers -> display overlay -> send note_on for all -> wait durations (with jitter) -> send offs -> release
    for step_idx in 0..num_steps {
        // Wait until all workers have placed their computed values
        barrier.wait();

        // Build overlay for this step and show it (non-blocking)
        // compute bar rect (same math as scanning)
        let width = img_for_overlay.cols();
        let height = img_for_overlay.rows();
        let vertical_default = width > height;
        let bar_w = if vertical_default { ((width as f32) * bar_thickness_frac).max(1.0).round() as i32 } else { width };
        let bar_h = if !vertical_default { ((height as f32) * bar_thickness_frac).max(1.0).round() as i32 } else { height };
        let travel_x = (width - bar_w).max(0);
        let travel_y = (height - bar_h).max(0);
        let x0 = if vertical_default {
            if num_steps == 1 { 0 } else { ((step_idx as f32) * (travel_x as f32) / ((num_steps - 1) as f32)).round() as i32 }
        } else { 0 };
        let y0 = if !vertical_default {
            if num_steps == 1 { 0 } else { ((step_idx as f32) * (travel_y as f32) / ((num_steps - 1) as f32)).round() as i32 }
        } else { 0 };
        let rect = core::Rect::new(x0, y0, if vertical_default { bar_w } else { width }, if vertical_default { height } else { bar_h });

        if let Ok(overlay_mat) = draw_scan_bar_overlay_for_rect(img_for_overlay, rect, num_instruments, vertical_default) {
            // Show (best-effort); ignore errors
            if let Err(e) = opencv::highgui::imshow("ScanBar Live", &overlay_mat) {
                eprintln!("imshow error: {}", e);
            } else {
                // allow GUI to refresh
                let _ = opencv::highgui::wait_key(1);
            }
        }

        // Read shared results and send note_on for all quickly, compute per-instrument off deadlines (with jitter)
        let mut rng = rand::thread_rng();
        let mut pending_off: Vec<(u8, u8, Instant)> = Vec::new(); // (channel, note, deadline)
        {
            let guard = results.lock().unwrap();
            for inst_idx in 0..num_instruments {
                if let Some((note, vel)) = guard[inst_idx] {
                    // Replace basic note with chord-aware choice:
                    // choose chord by step index and select note from chord tones (wrap safely)
                    let chosen_note = if !chords.is_empty() {
                        let chord_idx = step_idx % chords.len();
                        let chord = &chords[chord_idx];
                        // choose tone by instrument index modulo chord size
                        let tone_idx = inst_idx % chord.notes.len();
                        let mut n = chord.notes[tone_idx];
                        // apply brightness-based octave nudging using previous computed note as guidance
                        // (if chord tone seems too low/high, we can offset by one octave based on vel/inst)
                        if note > n { n = note; } // keep some brightness effect
                        n
                    } else {
                        note
                    };

                    let channel = (inst_idx % 16) as u8;
                    if let Err(e) = midi.note_on(channel, chosen_note, vel) {
                        eprintln!("MIDI note_on error: {}", e);
                    } else {
                        // compute deadline = now + ms_per_step +/- jitter
                        let jitter_range = (jitter_ms as i64) - (jitter_ms as i64); // symmetric: [-jitter_ms, +jitter_ms]
                        // simpler: random in [ms_per_step - jitter, ms_per_step + jitter]
                        let jitter = rng.gen_range(0..=jitter_ms as i64) as i64 - (jitter_ms as i64 / 2);
                        let dur_ms = if jitter < 0 {
                            ((ms_per_step as i64) + jitter).max(10) as u64
                        } else {
                            (ms_per_step as i64 + jitter) as u64
                        };
                        let deadline = Instant::now() + Duration::from_millis(dur_ms);
                        pending_off.push((channel, chosen_note, deadline));
                    }
                }
            }
        }

        // Wait loop: send note_offs when their deadlines pass
        while !pending_off.is_empty() {
            let now = Instant::now();
            // Collect indices to remove
            let mut to_remove = Vec::new();
            for (i, (_ch, _note, deadline)) in pending_off.iter().enumerate() {
                if *deadline <= now {
                    to_remove.push(i);
                }
            }
            // send offs for collected (iterate in reverse order to remove safely)
            for &idx in to_remove.iter().rev() {
                let (ch, note, _dl) = pending_off.remove(idx);
                if let Err(e) = midi.note_off(ch, note) {
                    eprintln!("MIDI note_off error: {}", e);
                }
            }
            // Sleep a small amount to avoid busy spin
            if !pending_off.is_empty() {
                thread::sleep(Duration::from_millis(5));
            }
        }

        // Clear shared results for next step
        {
            let mut guard = results.lock().unwrap();
            for slot in guard.iter_mut() { *slot = None; }
        }

        // Release workers to next step
        barrier.wait();
    }

    // Join worker threads
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

    // CLI + defaults
    let args: Vec<String> = env::args().collect();
    let instrument_count: usize = args.iter()
        .position(|a| a == "--instruments")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(4);

    println!("Instrument count set to {}", instrument_count);

    // scan settings (we'll expose CLI later)
    let bar_thickness_frac: f32 = 0.10;
    let num_steps: usize = 40;
    let ms_per_step: u64 = 250;
    let jitter_ms: u64 = 60; // ± ~30ms on average (range centered around ms_per_step)
    println!("Scan bar thickness = {:.2}, steps = {}, ms/step = {}, jitter_ms = {}", bar_thickness_frac, num_steps, ms_per_step, jitter_ms);

    // Image source
    let src = if let Some(img_path) = args.get(1) {
        if img_path == "play" { ImageSource::UserPath("assets/images/example.jpg".to_string()) } else { ImageSource::UserPath(img_path.clone()) }
    } else { ImageSource::UserPath("assets/images/example.jpg".to_string()) };

    let img = load_image_from_source(&src)?;
    println!("Image loaded from source.");

    // Global features & simple static scan
    let global_features = analyze_global(&img)?;
    println!("Global features: {:?}", global_features);

    let static_scan = analyze_scan_bar(&img, instrument_count, true)?;
    println!("Static scan-bar features (compat): {:?}", static_scan);

    // Choose mode from hue map and build chord progression using ChordEngine
    let hue = global_features.avg_hue;
    let mode = lookup_range_map(&mappings.global.hue_to_mode, hue).unwrap_or_else(|| "Ionian".to_string());
    println!("Chosen mode from hue {} -> {}", hue, mode);

    let engine = ChordEngine::new(mappings);
    let progression = engine.pick_progression(&mode);
    println!("Picked progression: {:?}", progression);

    let chords = engine.generate_chords(&progression, 60, &mode, global_features.edge_density, 0.0);
    println!("Generated chords (for steps): {:?}", chords);

    // Move the scan bar across image and generate steps (each step -> vec of per-instrument ScanBarFeatures)
    let steps = scan_image(&img, instrument_count, bar_thickness_frac, num_steps, None)?;
    println!("Completed scanning image. Steps: {}", steps.len());

    // Save overlays for first/mid/last steps for offline inspect
    let width = img.cols();
    let height = img.rows();
    let vertical_default = width > height;
    let bar_w = if vertical_default { ((width as f32) * bar_thickness_frac).max(1.0).round() as i32 } else { width };
    let bar_h = if !vertical_default { ((height as f32) * bar_thickness_frac).max(1.0).round() as i32 } else { height };
    let indices = vec![0usize, steps.len() / 2, steps.len().saturating_sub(1)];
    for &si in &indices {
        let travel_x = (width - bar_w).max(0);
        let travel_y = (height - bar_h).max(0);
        let x0 = if vertical_default {
            if steps.len() == 1 { 0 } else { ((si as f32) * (travel_x as f32) / ((steps.len() - 1) as f32)).round() as i32 }
        } else { 0 };
        let y0 = if !vertical_default {
            if steps.len() == 1 { 0 } else { ((si as f32) * (travel_y as f32) / ((steps.len() - 1) as f32)).round() as i32 }
        } else { 0 };
        let rect = core::Rect::new(x0, y0, if vertical_default { bar_w } else { width }, if vertical_default { height } else { bar_h });

        if let Ok(overlay) = draw_scan_bar_overlay_for_rect(&img, rect, instrument_count, vertical_default) {
            let out = format!("assets/overlay_step_{}.png", si);
            if let Err(e) = opencv::imgcodecs::imwrite(&out, &overlay, &opencv::core::Vector::new()) {
                println!("Warning: failed to write overlay {}: {}", out, e);
            } else {
                println!("Wrote overlay for step {} to {}", si, out);
            }
        }
    }

    // Playback requested?
    if args.iter().any(|a| a == "play") {
        let preferred = std::env::var("AUDIOHAX_MIDI_PORT").ok();
        let preferred_ref = preferred.as_deref().or(Some("AudioHaxOut"));
        println!("Attempting playback (preferred MIDI = {:?})", preferred_ref);

        // show a window early (highgui needs this on some platforms)
        let _ = opencv::highgui::named_window("ScanBar Live", opencv::highgui::WINDOW_AUTOSIZE);

        play_scanned_steps_concurrent(steps, ms_per_step, jitter_ms, chords, preferred_ref, &img, bar_thickness_frac)?;
    } else {
        println!("Run with `cargo run -- play` to play to a MIDI port.");
    }

    Ok(())
}

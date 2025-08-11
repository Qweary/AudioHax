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
use opencv::prelude::MatTraitConst; // needed for .cols()/.rows()
use opencv::core;
use rand::Rng;

fn clamp_u8(v: i32) -> u8 {
    if v < 0 { 0 } else if v > 127 { 127 } else { v as u8 }
}

/// Single instrument action for a step: one or more note events (arpeggio or single note)
#[derive(Clone, Debug)]
struct InstrumentAction {
    /// sequence of (note, velocity, hold_ms, offset_ms) where offset_ms is relative to step start
    events: Vec<(u8, u8, u64, u64)>,
}

/// Scheduled MIDI event (time-based)
#[derive(Clone, Debug)]
struct ScheduledEvent {
    at: Instant,
    on: bool,         // true = note_on, false = note_off
    channel: u8,
    note: u8,
    vel: u8,          // used only for note_on (note_off vel ignored)
}

fn parse_cli_arg<T: std::str::FromStr>(args: &[String], key: &str, default: T) -> T {
    args.iter()
        .position(|a| a == key)
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse::<T>().ok())
        .unwrap_or(default)
}

/// Map simple features -> velocity (0..127)
fn velocity_from_saturation(sat: f32) -> u8 {
    let v = ((sat.clamp(0.0, 100.0) / 100.0) * 90.0 + 30.0).round() as i32;
    clamp_u8(v)
}

/// Worker mapping: decides per-instrument InstrumentAction given its ScanBarFeatures and chord context.
/// - `ms_per_step` used to size arpeggio durations.
/// - arpeggio if edge_density > edge_threshold.
fn worker_decide_action(
    f: &image_analysis::ScanBarFeatures,
    inst_idx: usize,
    step_idx: usize,
    chords: &Vec<chord_engine::Chord>,
    ms_per_step: u64,
    edge_threshold: f32,
) -> InstrumentAction {

    let mut events: Vec<(u8,u8,u64,u64)> = Vec::new();
    // choose chord for this step (wrap)
    let chord = if !chords.is_empty() {
        &chords[step_idx % chords.len()]
    } else {
        // fallback: simple major triad C
        &chord_engine::Chord { name: "C".to_string(), notes: vec![60,64,67] }
    };

    // velocity from saturation
    let vel = velocity_from_saturation(f.avg_saturation);

    // decide arpeggio vs single note
    if f.edge_density > edge_threshold {
        // make a short arpeggio. create sequence of chord tones across 1-2 octaves.
        // number of notes depends on ms_per_step (keep each note >= 30ms)
        let max_notes = (ms_per_step / 40).max(2) as usize; // heuristic
        let mut ar_notes: Vec<u8> = Vec::new();
        // build ascending arpeggio picking chord tones and then octave-up tones
        for rep in 0..2 {
            for &n in &chord.notes {
                let base = n;
                let note = if rep == 0 { base } else { base.saturating_add(12) };
                ar_notes.push(note);
                if ar_notes.len() >= max_notes { break; }
            }
            if ar_notes.len() >= max_notes { break; }
        }
        // offset each note evenly across ms_per_step
        let per_note = ((ms_per_step as f32) / (ar_notes.len() as f32)).round() as u64;
        let mut offset = 0u64;
        for &note in &ar_notes {
            // hold slightly shorter than spacing so there's small separation
            let hold = per_note.saturating_sub(10).max(10);
            events.push((note, vel, hold, offset));
            offset += per_note;
        }
    } else {
        // single chord-tone choice: instrument index picks tone from chord
        let tone_idx = inst_idx % chord.notes.len();
        let mut note = chord.notes[tone_idx];
        // brightness influence (raise by up to one octave for bright bars)
        let brightness_norm = (f.avg_brightness.clamp(0.0, 100.0) / 100.0) as f32;
        if brightness_norm > 0.75 {
            note = note.saturating_add(12);
        } else if brightness_norm < 0.25 {
            note = note.saturating_sub(12);
        }
        // default hold time nearly full step
        let hold = (ms_per_step as f32 * 0.9).round() as u64;
        events.push((note, vel, hold, 0));
    }

    InstrumentAction { events }
}

/// Play steps concurrently: workers populate InstrumentAction for their instrument each step,
/// then coordinator collects them and schedules all MIDI note_on/note_off events (single-threaded).
/// jitter_percent is applied to each event's duration (±percent).
fn play_scanned_steps_concurrent(
    steps: Vec<Vec<image_analysis::ScanBarFeatures>>,
    ms_per_step: u64,
    jitter_percent: f32,
    chords: Arc<Vec<chord_engine::Chord>>,
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

    // MIDI open once
    println!("Opening MIDI port...");
    let mut midi = MidiOut::open_first(preferred_midi_port_hint)?;
    println!("MIDI opened.");

    // set initial programs for channels
    for i in 0..num_instruments {
        let ch = (i % 16) as u8;
        let prog = ((i * 7) % 128) as u8;
        midi.program_change(ch, prog)?;
    }

    // shared actions (one slot per instrument)
    let actions: Arc<Mutex<Vec<Option<InstrumentAction>>>> =
        Arc::new(Mutex::new(vec![None; num_instruments]));
    let barrier = Arc::new(Barrier::new(num_instruments + 1));

    // parameters for workers
    let edge_threshold = 0.30_f32;

    // spawn workers
    let mut handles = Vec::new();
    for inst_idx in 0..num_instruments {
        let steps_clone = steps.clone();
        let acts = actions.clone();
        let bar = barrier.clone();
        let chords_cl = chords.clone();
        let ms_per_step_local = ms_per_step;
        let h_threshold = edge_threshold;

        let handle = thread::spawn(move || {
            for step_idx in 0..num_steps {
                let f = &steps_clone[step_idx][inst_idx];
                let action = worker_decide_action(f, inst_idx, step_idx, &*chords_cl, ms_per_step_local, h_threshold);
                {
                    let mut g = acts.lock().unwrap();
                    g[inst_idx] = Some(action);
                }
                // signal ready
                bar.wait();
                // wait for coordinator to process this step and release
                bar.wait();
            }
        });

        handles.push(handle);
    }

    // Coordinator loop per step: gather InstrumentAction for this step, build scheduled events (all instruments),
    // execute them in time order (single-threaded), then release workers.
    for step_idx in 0..num_steps {
        // wait for workers to compute this step
        barrier.wait();

        // show overlay for this step
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

        if let Ok(overlay) = draw_scan_bar_overlay_for_rect(img_for_overlay, rect, num_instruments, vertical_default) {
            let _ = opencv::highgui::imshow("ScanBar Live", &overlay);
            let _ = opencv::highgui::wait_key(1);
        }

        // Collect actions snapshot
        let snapshot: Vec<Option<InstrumentAction>> = {
            let guard = actions.lock().unwrap();
            guard.clone()
        };

        // Build scheduled events
        let mut events: Vec<ScheduledEvent> = Vec::new();
        let t0 = Instant::now();
        let mut rng = rand::thread_rng();

        for inst_idx in 0..num_instruments {
            if let Some(action) = &snapshot[inst_idx] {
                let channel = (inst_idx % 16) as u8;
                for (note, vel, hold_ms, offset_ms) in &action.events {
                    // apply jitter_percent on hold_ms
                    let jitter = rng.gen_range(-(jitter_percent*100.0) as i32 ..= (jitter_percent*100.0) as i32) as f32 / 100.0;
                    let base_hold = *hold_ms as f32;
                    let hold_ms_f = (base_hold * (1.0 + jitter)).max(8.0).round() as u64;

                    let start_instant = t0 + Duration::from_millis(*offset_ms) ;
                    let on_event = ScheduledEvent { at: start_instant, on: true, channel, note: *note, vel: *vel };
                    let off_event = ScheduledEvent { at: start_instant + Duration::from_millis(hold_ms_f), on: false, channel, note: *note, vel: 0 };
                    events.push(on_event);
                    events.push(off_event);
                }
            }
        }

        // Sort events by time
        events.sort_by_key(|e| e.at);

        // Execute scheduled events (single-threaded)
        for ev in events {
            let now = Instant::now();
            if ev.at > now {
                let sleep_dur = ev.at - now;
                // sleep until event time (coarse)
                thread::sleep(sleep_dur);
            }
            if ev.on {
                if let Err(e) = midi.note_on(ev.channel, ev.note, ev.vel) {
                    eprintln!("MIDI note_on error: {}", e);
                }
            } else {
                if let Err(e) = midi.note_off(ev.channel, ev.note) {
                    eprintln!("MIDI note_off error: {}", e);
                }
            }
        }

        // clear actions slots
        {
            let mut guard = actions.lock().unwrap();
            for slot in guard.iter_mut() {
                *slot = None;
            }
        }

        // release workers to next step
        barrier.wait();
    }

    // join workers
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

    // CLI
    let args: Vec<String> = env::args().collect();

    let instrument_count: usize = parse_cli_arg(&args, "--instruments", 4usize);
    let bar_thickness_frac: f32 = parse_cli_arg(&args, "--thickness", 0.10f32);
    let num_steps: usize = parse_cli_arg(&args, "--steps", 40usize);
    let ms_per_step: u64 = parse_cli_arg(&args, "--ms-per-step", 250u64);
    let jitter_percent: f32 = parse_cli_arg(&args, "--jitter-percent", 15.0f32); // percent (e.g., 15 -> ±15%)

    println!("Instrument count: {}", instrument_count);
    println!("Scan bar thickness = {:.2}, steps = {}, ms/step = {}, jitter% = {}",
             bar_thickness_frac, num_steps, ms_per_step, jitter_percent);

    // Image source (first arg unless it's "play" or flags)
    // If first arg looks like a path and doesn't start with "--", use it
    let maybe_img = args.get(1).filter(|s| !s.starts_with("--")).cloned();
    let src = if let Some(p) = maybe_img {
        if p == "play" {
            ImageSource::UserPath("assets/images/example.jpg".to_string())
        } else {
            ImageSource::UserPath(p)
        }
    } else {
        ImageSource::UserPath("assets/images/example.jpg".to_string())
    };

    let img = load_image_from_source(&src)?;
    println!("Image loaded from source.");

    // Global features
    let global_features = analyze_global(&img)?;
    println!("Global features: {:?}", global_features);

    // static scan compatibility (per-instrument bar avg)
    let static_scan = analyze_scan_bar(&img, instrument_count, true)?;
    println!("Static scan-bar features (compat): {:?}", static_scan);

    // pick mode and make chord progression
    let hue = global_features.avg_hue;
    let mode = lookup_range_map(&mappings.global.hue_to_mode, hue).unwrap_or_else(|| "Ionian".to_string());
    println!("Chosen mode from hue {} -> {}", hue, mode);

    let engine = ChordEngine::new(mappings);
    let progression = engine.pick_progression(&mode);
    println!("Picked progression: {:?}", progression);

    let chords_vec = engine.generate_chords(&progression, 60, &mode, global_features.edge_density, 0.0);
    println!("Generated chords: {:?}", chords_vec);
    let chords_arc = Arc::new(chords_vec);

    // Run full scan producing steps (each step -> per-instrument ScanBarFeatures)
    let steps = scan_image(&img, instrument_count, bar_thickness_frac, num_steps, None)?;
    println!("Completed scanning image. Steps: {}", steps.len());

    // Save overlays for inspection: first / mid / last
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

    // Playback?
    if args.iter().any(|a| a == "play") {
        let preferred = std::env::var("AUDIOHAX_MIDI_PORT").ok();
        let preferred_ref = preferred.as_deref().or(Some("AudioHaxOut"));
        println!("Attempting playback (preferred MIDI = {:?})", preferred_ref);

        let _ = opencv::highgui::named_window("ScanBar Live", opencv::highgui::WINDOW_AUTOSIZE);

        play_scanned_steps_concurrent(
            steps,
            ms_per_step,
            jitter_percent,
            chords_arc,
            preferred_ref,
            &img,
            bar_thickness_frac,
        )?;
    } else {
        println!("Run with `cargo run -- play` to play to a MIDI port (add CLI flags e.g. --jitter-percent 20 --thickness 0.08).");
    }

    Ok(())
}

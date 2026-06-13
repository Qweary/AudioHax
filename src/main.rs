// src/main.rs
mod image_analysis;
mod image_source;
mod midi_output;

use audiohax::{chord_engine, mapping_loader};
use chord_engine::ChordEngine;
use image_analysis::{
    analyze_global, analyze_scan_bar, draw_scan_bar_overlay_for_rect, scan_image,
};
use image_source::{load_image_from_source, ImageSource};
use mapping_loader::{load_mappings, lookup_range_map};
use midi_output::MidiOut;
use opencv::core;
use opencv::prelude::MatTraitConst; // needed for .cols()/.rows()
use rand::Rng;
use std::env;
use std::sync::{Arc, Barrier, Mutex};
use std::thread;
use std::time::{Duration, Instant};

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
    on: bool, // true = note_on, false = note_off
    channel: u8,
    note: u8,
    vel: u8, // used only for note_on (note_off vel ignored)
}

fn parse_cli_arg<T: std::str::FromStr>(args: &[String], key: &str, default: T) -> T {
    args.iter()
        .position(|a| a == key)
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse::<T>().ok())
        .unwrap_or(default)
}

/// Thin playback adapter for one instrument on one step: extract → lookup → realize → map.
///
/// Makes NO musical decision. All musical logic — voicing, dynamics, rhythm,
/// articulation, orchestration roles — lives in `chord_engine.rs`. This function
/// only: (1) looks up THIS step's `StepPlan` (wrapping by `step_idx`), (2) projects
/// the image-domain `ScanBarFeatures` into the plain-scalar `PerfFeatures` the lib
/// consumes (no OpenCV/image type crosses into the lib), (3) calls the single pure
/// entry point `chord_engine::realize_step`, and (4) maps each returned `NoteEvent`
/// onto the existing `(note, velocity, hold_ms, offset_ms)` tuple the coordinator
/// schedules.
///
/// `_edge_threshold` is retained for call-site compatibility but unused — the worker
/// no longer branches on edge density itself; `realize_step` owns that decision.
fn worker_decide_action(
    f: &image_analysis::ScanBarFeatures,
    inst_idx: usize,
    step_idx: usize,
    num_instruments: usize,
    plan: &[chord_engine::StepPlan],
    ms_per_step: u64,
    _edge_threshold: f32,
) -> InstrumentAction {
    // 1) Look up THIS step's plan, wrapping like the old chords[step_idx % len].
    //    Empty-plan guard: emit no events (a silent step) — the minimal safe choice
    //    that never panics and makes no musical decision here.
    if plan.is_empty() {
        return InstrumentAction { events: Vec::new() };
    }
    let step = &plan[step_idx % plan.len()];

    // 2) Project the image features into the plain-scalar PerfFeatures.
    //    ScanBarFeatures fields are all f32 and units already match PerfFeatures
    //    (saturation/brightness 0..=100, edge_density 0..=1) — no cast needed.
    let features = chord_engine::PerfFeatures {
        saturation: f.avg_saturation,
        brightness: f.avg_brightness,
        edge_density: f.edge_density,
    };

    // 3) Call the single pure entry point and map NoteEvent -> the existing tuple.
    let note_events =
        chord_engine::realize_step(step, inst_idx, num_instruments, &features, ms_per_step);
    let events = note_events
        .into_iter()
        .map(|e| (e.note, e.velocity, e.hold_ms, e.offset_ms))
        .collect();
    InstrumentAction { events }
}

/// Play steps concurrently: workers populate InstrumentAction for their instrument each step,
/// then coordinator collects them and schedules all MIDI note_on/note_off events (single-threaded).
/// jitter_percent is applied to each event's duration (±percent).
fn play_scanned_steps_concurrent(
    steps: Vec<Vec<image_analysis::ScanBarFeatures>>,
    ms_per_step: u64,
    jitter_percent: f32,
    plan: Arc<Vec<chord_engine::StepPlan>>,
    preferred_midi_port_hint: Option<&str>,
    img_for_overlay: &opencv::prelude::Mat,
    bar_thickness_frac: f32,
) -> Result<(), Box<dyn std::error::Error>> {
    if steps.is_empty() {
        return Ok(());
    }
    let num_steps = steps.len();
    let num_instruments = steps[0].len();
    if num_instruments == 0 {
        return Ok(());
    }

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
        let plan_cl = plan.clone();
        let ms_per_step_local = ms_per_step;
        let h_threshold = edge_threshold;

        let handle = thread::spawn(move || {
            for step_idx in 0..num_steps {
                let f = &steps_clone[step_idx][inst_idx];
                let action = worker_decide_action(
                    f,
                    inst_idx,
                    step_idx,
                    num_instruments,
                    &plan_cl,
                    ms_per_step_local,
                    h_threshold,
                );
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
        let travel_x = (width - bar_w).max(0);
        let travel_y = (height - bar_h).max(0);
        let x0 = if vertical_default {
            if num_steps == 1 {
                0
            } else {
                ((step_idx as f32) * (travel_x as f32) / ((num_steps - 1) as f32)).round() as i32
            }
        } else {
            0
        };
        let y0 = if !vertical_default {
            if num_steps == 1 {
                0
            } else {
                ((step_idx as f32) * (travel_y as f32) / ((num_steps - 1) as f32)).round() as i32
            }
        } else {
            0
        };
        let rect = core::Rect::new(
            x0,
            y0,
            if vertical_default { bar_w } else { width },
            if vertical_default { height } else { bar_h },
        );

        if let Ok(overlay) =
            draw_scan_bar_overlay_for_rect(img_for_overlay, rect, num_instruments, vertical_default)
        {
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
                    let jitter = rng.gen_range(
                        -(jitter_percent * 100.0) as i32..=(jitter_percent * 100.0) as i32,
                    ) as f32
                        / 100.0;
                    let base_hold = *hold_ms as f32;
                    let hold_ms_f = (base_hold * (1.0 + jitter)).max(8.0).round() as u64;

                    let start_instant = t0 + Duration::from_millis(*offset_ms);
                    let on_event = ScheduledEvent {
                        at: start_instant,
                        on: true,
                        channel,
                        note: *note,
                        vel: *vel,
                    };
                    let off_event = ScheduledEvent {
                        at: start_instant + Duration::from_millis(hold_ms_f),
                        on: false,
                        channel,
                        note: *note,
                        vel: 0,
                    };
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
    println!(
        "Scan bar thickness = {:.2}, steps = {}, ms/step = {}, jitter% = {}",
        bar_thickness_frac, num_steps, ms_per_step, jitter_percent
    );

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
    let mode =
        lookup_range_map(&mappings.global.hue_to_mode, hue).unwrap_or_else(|| "Ionian".to_string());
    println!("Chosen mode from hue {} -> {}", hue, mode);

    let engine = ChordEngine::new(mappings);
    let progression = engine.pick_progression(&mode);
    println!("Picked progression: {:?}", progression);

    let chords_vec =
        engine.generate_chords(&progression, 60, &mode, global_features.edge_density, 0.0);
    println!("Generated chords: {:?}", chords_vec);
    // Plan the phrases once from the generated chords and share the PLAN (not the
    // raw chords). plan_phrases runs voice_lead_sequence internally, so the shared
    // plan carries the voice-led chords — no separate voice-leading call needed.
    let plan_vec = engine.plan_phrases(&chords_vec); // Vec<chord_engine::StepPlan>
    let plan_arc = Arc::new(plan_vec); // Arc<Vec<StepPlan>>

    // Run full scan producing steps (each step -> per-instrument ScanBarFeatures)
    let steps = scan_image(&img, instrument_count, bar_thickness_frac, num_steps, None)?;
    println!("Completed scanning image. Steps: {}", steps.len());

    // Save overlays for inspection: first / mid / last
    let width = img.cols();
    let height = img.rows();
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
    let indices = vec![0usize, steps.len() / 2, steps.len().saturating_sub(1)];
    for &si in &indices {
        let travel_x = (width - bar_w).max(0);
        let travel_y = (height - bar_h).max(0);
        let x0 = if vertical_default {
            if steps.len() == 1 {
                0
            } else {
                ((si as f32) * (travel_x as f32) / ((steps.len() - 1) as f32)).round() as i32
            }
        } else {
            0
        };
        let y0 = if !vertical_default {
            if steps.len() == 1 {
                0
            } else {
                ((si as f32) * (travel_y as f32) / ((steps.len() - 1) as f32)).round() as i32
            }
        } else {
            0
        };
        let rect = core::Rect::new(
            x0,
            y0,
            if vertical_default { bar_w } else { width },
            if vertical_default { height } else { bar_h },
        );

        if let Ok(overlay) =
            draw_scan_bar_overlay_for_rect(&img, rect, instrument_count, vertical_default)
        {
            let out = format!("assets/overlay_step_{}.png", si);
            if let Err(e) = opencv::imgcodecs::imwrite(&out, &overlay, &opencv::core::Vector::new())
            {
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
            plan_arc,
            preferred_ref,
            &img,
            bar_thickness_frac,
        )?;
    } else {
        println!("Run with `cargo run -- play` to play to a MIDI port (add CLI flags e.g. --jitter-percent 20 --thickness 0.08).");
    }

    Ok(())
}

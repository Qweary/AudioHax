//! S123 — Shielding `s123-SH` W-3 / T14: the GUI worker's post-panic correctness, as a forcing
//! test.
//!
//! The window renders many pieces on one worker thread and catches panics there. The seed,
//! expression and cadence registers are `thread_local!`, and the event dump is process-global.
//! This file pins what `s123-SH` § 3.4 measured by hand:
//!
//! 1. A render that PANICS (injected through the note callback, after `compose()` has set the
//!    registers) at `Planned`, and at `ExpressionVerified` with the event dump ARMED, is caught;
//!    afterwards the dump is disarmed and a plain render AND a plain `prepare_play` on the SAME
//!    thread equal a fresh thread's (events and samples / timeline).
//! 2. The three registers poisoned directly (seed + counter, expression profile, cadence figure)
//!    leave the next render on that thread equal to a fresh thread's.
//!
//! A register added later without a reset in `compose()` would make (2) fail here, on the panic
//! path no golden exercises. (The window additionally replaces its worker thread after any caught
//! panic — `gui::job` W-2 — so its own correctness does not rest on this convention.)
//!
//! Library-only (no `gui` feature needed): it guards `render.rs`'s contract that the GUI relies on.
#![cfg(all(feature = "synth", feature = "pure-analysis"))]

use audiohax::cli::{PipelineArgs, Toggle, V2Spec};
use audiohax::mapping_loader::{parse_mappings, MappingTable};
use audiohax::render::{
    prepare_play, render, DumpLabel, ImageInput, RenderError, RenderNote, RenderRequest,
};
use audiohax::synth_sink::{SoundFontSource, TimedMidiEvent};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Mutex;

/// The event dump is process-global: serialise this file's tests so one test's armed render
/// cannot be observed by another's `is_armed()` check.
static SERIAL: Mutex<()> = Mutex::new(());

fn mappings() -> MappingTable {
    parse_mappings(include_str!("../assets/mappings.json")).expect("mappings parse")
}

/// A small structured picture (gradient + checker) generated here, never committed.
fn picture() -> Vec<u8> {
    let (w, h) = (96u32, 64u32);
    let img = image::ImageBuffer::from_fn(w, h, |x, y| {
        let block = ((x / 8) + (y / 8)) % 2 == 0;
        image::Rgb([
            ((x * 255) / w) as u8,
            ((y * 255) / h) as u8,
            if block { 220 } else { 30 },
        ])
    });
    let mut out = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(img)
        .write_to(&mut out, image::ImageFormat::Png)
        .expect("encode");
    out.into_inner()
}

/// What the window renders: CLI defaults, image-derived seed, no dump.
fn plain_request() -> RenderRequest {
    RenderRequest {
        seed: None,
        loop_cue: false,
        expression: Toggle::Off,
        v2: None,
        pipeline: PipelineArgs::default(),
        reverb: Toggle::On,
        gain: 1.0,
        event_dump: None,
    }
}

/// Everything armed: explicit seed, expression + every v2 term, and the event dump.
fn armed_request() -> RenderRequest {
    RenderRequest {
        seed: Some(99),
        expression: Toggle::On,
        v2: Some(V2Spec {
            e3: true,
            e2: true,
            ta: true,
        }),
        event_dump: Some(DumpLabel {
            cue: "s123-w3".into(),
            image: "<generated>".into(),
        }),
        ..plain_request()
    }
}

type Snapshot = (Vec<TimedMidiEvent>, Vec<f32>, Vec<TimedMidiEvent>);

/// A plain render and a plain `prepare_play` on the CURRENT thread.
fn plain_snapshot(bytes: &[u8], m: &MappingTable) -> Snapshot {
    let req = plain_request();
    let out = render(
        &req,
        ImageInput::Bytes(bytes),
        m,
        SoundFontSource::Bundled,
        &mut |_| Ok(()),
    )
    .expect("plain render");
    let plan = prepare_play(&req, ImageInput::Bytes(bytes), m, &mut |_| Ok(())).expect("prepare");
    (out.events, out.interleaved, plan.timeline)
}

/// The same snapshot on a brand-new thread (fresh thread-local registers).
fn fresh_snapshot(bytes: &[u8]) -> Snapshot {
    let b = bytes.to_vec();
    std::thread::spawn(move || plain_snapshot(&b, &mappings()))
        .join()
        .expect("fresh thread")
}

fn note_name(n: &RenderNote<'_>) -> &'static str {
    match n {
        RenderNote::ImageLoaded { .. } => "ImageLoaded",
        RenderNote::ImageScanned { .. } => "ImageScanned",
        RenderNote::GlobalFeatures(_) => "GlobalFeatures",
        RenderNote::Planned { .. } => "Planned",
        RenderNote::EventDumpArming => "EventDumpArming",
        RenderNote::ExpressionVerified { .. } => "ExpressionVerified",
        RenderNote::V2TermVerified { .. } => "V2TermVerified",
        RenderNote::EventDumpJson(_) => "EventDumpJson",
        RenderNote::EventDumpWritten { .. } => "EventDumpWritten",
        RenderNote::LoopFolded { .. } => "LoopFolded",
        RenderNote::EngineMode(_) => "EngineMode",
    }
}

/// Inject a panic at the note named `at` inside an armed render, catch it on THIS thread, then
/// check the same-thread snapshot equals a fresh thread's.
fn panic_then_compare(at: &'static str, expect_dump_armed_at_panic: bool) {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let bytes = picture();
    let m = mappings();
    let fresh = fresh_snapshot(&bytes);

    let mut armed_at_panic = None;
    let caught = catch_unwind(AssertUnwindSafe(|| {
        let mut notes = |n: RenderNote<'_>| -> Result<(), RenderError> {
            if note_name(&n) == at {
                armed_at_panic = Some(audiohax::expression_dump::is_armed());
                panic!("injected panic at {at}");
            }
            Ok(())
        };
        let _ = render(
            &armed_request(),
            ImageInput::Bytes(&bytes),
            &m,
            SoundFontSource::Bundled,
            &mut notes,
        );
    }));
    assert!(caught.is_err(), "the injected panic at {at} must unwind");
    assert_eq!(
        armed_at_panic,
        Some(expect_dump_armed_at_panic),
        "dump state when {at} fired"
    );
    assert!(
        !audiohax::expression_dump::is_armed(),
        "the dump must be disarmed after the unwind ({at})"
    );

    let same = plain_snapshot(&bytes, &m);
    assert_eq!(
        same.0, fresh.0,
        "render events differ after a panic at {at}"
    );
    assert_eq!(
        same.1, fresh.1,
        "render samples differ after a panic at {at}"
    );
    assert_eq!(
        same.2, fresh.2,
        "prepare_play timeline differs after a panic at {at}"
    );
}

#[test]
fn panic_at_planned_leaves_the_thread_clean() {
    panic_then_compare("Planned", false);
}

#[test]
fn panic_at_expression_verified_with_the_dump_armed_leaves_the_thread_clean() {
    panic_then_compare("ExpressionVerified", true);
}

#[test]
fn directly_poisoned_registers_do_not_leak_into_the_next_render() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let bytes = picture();
    let m = mappings();
    let fresh = fresh_snapshot(&bytes);

    audiohax::seed::set_composition_seed(Some(7));
    for _ in 0..5 {
        let _ = audiohax::seed::next_seed_call();
    }
    audiohax::composition::expression_register::set(
        audiohax::composition::ExpressionProfile::e1_entrance_destack(),
    );
    audiohax::chord_engine::set_cadence_development(audiohax::chord_engine::CadenceFigure {
        shape: Some(audiohax::chord_engine::select_cadence_shape(0.9, 2.0)),
        drive: 0.8,
    });

    let same = plain_snapshot(&bytes, &m);
    // Control: the poison is POTENT — a render that really used seed 7 differs from the plain
    // one — so equality below is evidence that compose() reset it, not that it never mattered.
    let seeded = render(
        &RenderRequest {
            seed: Some(7),
            ..plain_request()
        },
        ImageInput::Bytes(&bytes),
        &m,
        SoundFontSource::Bundled,
        &mut |_| Ok(()),
    )
    .expect("seeded render");
    assert_ne!(
        seeded.events, fresh.0,
        "control: seed 7 must change the piece"
    );
    assert_eq!(same.0, fresh.0, "render events leaked a poisoned register");
    assert_eq!(same.1, fresh.1, "render samples leaked a poisoned register");
    assert_eq!(
        same.2, fresh.2,
        "prepare_play timeline leaked a poisoned register"
    );
}

//! S123 — security review `s123-SH` W-3 / T14: the GUI worker's post-panic correctness, as a
//! forcing test.
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

/// The expression register's poison: the base expression profile. It is EXACTLY what `compose()`
/// writes for `expression: On` with no v2 term (`src/render.rs`), which is what makes the
/// potency control below a control for this very value.
fn expression_poison() -> audiohax::composition::ExpressionProfile {
    audiohax::composition::ExpressionProfile::e1_entrance_destack()
}

/// The cadence register's poison: a figure the test picture's own analysis does not produce.
fn cadence_poison() -> audiohax::chord_engine::CadenceFigure {
    audiohax::chord_engine::CadenceFigure {
        shape: Some(audiohax::chord_engine::select_cadence_shape(0.9, 2.0)),
        drive: 0.8,
    }
}

/// Potency control for the expression poison (fix round; `s123-SH2` H-2). If `compose()` stopped
/// resetting the expression register, a plain render after the poison would compose under this
/// profile. A render that asks for it (`expression: On`, no v2 term) shows that doing so changes
/// the piece, so equality in `directly_poisoned_registers_do_not_leak_into_the_next_render` is
/// evidence of the reset, not of a poison that never mattered.
#[test]
fn expression_poison_is_potent() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let bytes = picture();
    let fresh = fresh_snapshot(&bytes);
    let with_expression = std::thread::spawn({
        let bytes = bytes.clone();
        move || {
            render(
                &RenderRequest {
                    expression: Toggle::On,
                    ..plain_request()
                },
                ImageInput::Bytes(&bytes),
                &mappings(),
                SoundFontSource::Bundled,
                &mut |_| Ok(()),
            )
            .expect("expression render")
        }
    })
    .join()
    .expect("thread");
    assert_ne!(
        with_expression.events, fresh.0,
        "control: the expression poison (base profile) must change the piece"
    );
}

/// Potency control for the cadence poison (fix round; `s123-SH2` H-2). No request makes
/// `compose()` write this figure, so the control writes it itself, from the note callback at
/// `Planned`: that is AFTER `compose()` has set the register from the picture and BEFORE the
/// timeline is realised, which is the window a missing reset would leave the poison in. The piece
/// changes, so the poison is potent on the path that reads it.
#[test]
fn cadence_poison_is_potent() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let bytes = picture();
    let fresh = fresh_snapshot(&bytes);
    let poisoned = std::thread::spawn({
        let bytes = bytes.clone();
        move || {
            let mut own_figure = None;
            let out = render(
                &plain_request(),
                ImageInput::Bytes(&bytes),
                &mappings(),
                SoundFontSource::Bundled,
                &mut |n: RenderNote<'_>| {
                    if matches!(n, RenderNote::Planned { .. }) {
                        own_figure = Some(audiohax::chord_engine::cadence_development());
                        audiohax::chord_engine::set_cadence_development(cadence_poison());
                    }
                    Ok(())
                },
            )
            .expect("poisoned render");
            (out, own_figure)
        }
    })
    .join()
    .expect("thread");
    let (out, own_figure) = poisoned;
    let own_figure = own_figure.expect("Planned fired");
    assert_ne!(
        own_figure,
        cadence_poison(),
        "the picture's own figure must differ from the poison, or the control proves nothing"
    );
    assert_ne!(
        out.events, fresh.0,
        "control: the cadence poison must change the piece"
    );
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
    audiohax::composition::expression_register::set(expression_poison());
    audiohax::chord_engine::set_cadence_development(cadence_poison());

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
    // The other two poisons are proven potent by `expression_poison_is_potent` and
    // `cadence_poison_is_potent` below (security review `s123-SH2` H-2 / § 8 item 2).
    assert_eq!(same.0, fresh.0, "render events leaked a poisoned register");
    assert_eq!(same.1, fresh.1, "render samples leaked a poisoned register");
    assert_eq!(
        same.2, fresh.2,
        "prepare_play timeline leaked a poisoned register"
    );
}

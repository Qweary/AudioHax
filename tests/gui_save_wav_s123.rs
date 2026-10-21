//! S123 Test Engineer (Specialist 5): Save WAV writes the SAME samples `render()` returns
//! (the Architect's S123 minimum, `s123-01` § 4; lead ruling L-12).
//!
//! The builder's `job::tests::render_then_save_writes_the_same_samples_render_returned` compares
//! the file with the buffer the SAME job produced, through a re-typed copy of the i16
//! conversion. These tests compare against an INDEPENDENT library call — `render()` with the
//! window's request, the bundled font and the embedded mappings, written by the library's own
//! `synth_sink::write_stereo_wav` (the CLI's writer) — byte for byte, through BOTH of the
//! window's save routes: Save WAV with no current render (`JobKind::Render { save_to }`) and
//! Save WAV after Render (`JobKind::Save`).
//!
//! Files are written under `CARGO_TARGET_TMPDIR`, never `/tmp`.
#![cfg(feature = "gui")]

use audiohax::gui::embedded_mappings;
use audiohax::gui::job::{Controller, Event, GuardedImage, JobKind};
use audiohax::gui::state::Controls;
use audiohax::render::{render, ImageInput, RenderOutput, RenderRequest};
use audiohax::synth_sink::{write_stereo_wav, SoundFontSource};
use image::{ImageBuffer, ImageFormat, Rgb};
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

fn scratch(name: &str) -> PathBuf {
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("gui-save-wav-s123")
        .join(format!("{name}-{}", std::process::id()));
    std::fs::create_dir_all(&d).expect("scratch dir");
    d
}

fn picture() -> Vec<u8> {
    let (w, h) = (88u32, 56u32);
    let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_fn(w, h, |x, y| {
        Rgb([
            (x * 3 % 256) as u8,
            ((x * y) % 256) as u8,
            if (x / 11 + y / 7) % 2 == 0 { 230 } else { 20 },
        ])
    });
    let mut out = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(img)
        .write_to(&mut out, ImageFormat::Png)
        .expect("encode");
    out.into_inner()
}

/// The window's request for a non-default set of controls (so a request the window built
/// differently from the one it saved would show).
fn request() -> RenderRequest {
    Controls {
        seed: Some(1234),
        instruments: 3,
        reverb: false,
        gain: 0.8,
    }
    .request()
}

/// The library's own render + writer: what the CLI's `render --wav` path writes.
fn reference(png: &[u8], dir: &Path) -> (RenderOutput, Vec<u8>) {
    let out = render(
        &request(),
        ImageInput::Bytes(png),
        &embedded_mappings().expect("mappings"),
        SoundFontSource::Bundled,
        &mut |_| Ok(()),
    )
    .expect("library render");
    let p = dir.join("reference.wav");
    write_stereo_wav(&p, out.sample_rate, &out.interleaved).expect("library writer");
    let bytes = std::fs::read(&p).expect("read reference");
    (out, bytes)
}

fn drain(c: &mut Controller) -> Vec<Event> {
    let t0 = Instant::now();
    let mut all = Vec::new();
    // Let the job reach the worker before testing `is_busy`.
    loop {
        all.extend(c.poll());
        if !c.is_busy() {
            return all;
        }
        assert!(t0.elapsed() < Duration::from_secs(120), "worker timed out");
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// Save WAV with no current render: the window renders then saves in ONE job. The file is
/// byte-identical to the library's render written by the library's writer, and the buffer the
/// window keeps is sample-identical to the library's.
#[test]
fn save_with_no_current_render_writes_the_librarys_bytes() {
    let d = scratch("render-save");
    let png = picture();
    let (lib, lib_bytes) = reference(&png, &d);
    let target = d.join("from-window.wav");
    let mut c = Controller::new(embedded_mappings().expect("mappings"), || {});
    c.submit(JobKind::Render {
        request: request(),
        image: GuardedImage::admit(png).expect("admit"),
        save_to: Some(target.clone()),
    });
    let events = drain(&mut c);
    let kept = events
        .iter()
        .find_map(|e| match e {
            Event::Rendered { audio, saved_to } => {
                assert_eq!(saved_to.as_deref(), Some(target.as_path()));
                Some(audio.clone())
            }
            Event::Failed(u) => panic!("job failed: {u:?}"),
            _ => None,
        })
        .expect("Rendered event");
    assert_eq!(kept.sample_rate, lib.sample_rate);
    assert_eq!(kept.interleaved, lib.interleaved, "kept buffer == render()");
    let written = std::fs::read(&target).expect("saved file");
    assert_eq!(written.len(), lib_bytes.len(), "file length");
    assert!(
        written == lib_bytes,
        "saved WAV bytes differ from the library's"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// Save WAV after Render: the window writes the buffer it kept. Byte-identical to the library's
/// render + writer, and the file replaces an existing one (no partial file left beside it).
#[test]
fn save_after_render_writes_the_librarys_bytes() {
    let d = scratch("save-job");
    let png = picture();
    let (lib, lib_bytes) = reference(&png, &d);
    let target = d.join("kept.wav");
    std::fs::write(&target, b"an older file the user is replacing").expect("seed");
    let mut c = Controller::new(embedded_mappings().expect("mappings"), || {});
    c.submit(JobKind::Save {
        audio: Arc::new(lib),
        path: target.clone(),
    });
    let events = drain(&mut c);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::Saved(p) if p == &target)),
        "{events:?}"
    );
    let written = std::fs::read(&target).expect("saved file");
    assert_eq!(written.len(), lib_bytes.len(), "file length");
    assert!(
        written == lib_bytes,
        "saved WAV bytes differ from the library's"
    );
    let leftovers: Vec<_> = std::fs::read_dir(&d)
        .expect("dir")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.contains("partial"))
        .collect();
    assert!(leftovers.is_empty(), "{leftovers:?}");
    let _ = std::fs::remove_dir_all(&d);
}

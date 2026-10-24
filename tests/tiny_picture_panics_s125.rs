//! S125 (D-45, first half): the two pre-existing panic classes in the pure analyser.
//!
//! Security review `s123-SH-file-ingestion-bounds.md` § 1 HL-1 named two classes, and the S124
//! blind review (`s124-06B` F-PRE-1) reproduced them from the CLI:
//!
//! * (a) a 1×1 picture whose one pixel survives the Otsu threshold panics inside `imageproc
//!   0.23.1`'s union-find (`assertion failed: i < self.count`), reached through
//!   `analyze_global_pure` → `shape_complexity_pure` → `connected_components`;
//! * (b) a 1- or 2-px-wide picture tall enough that a zero-width cell of the 3×3 saliency grid
//!   (`analyze_regions_pure`) is at least 2 px tall panics in `to_gray` (`Image index (0, 1) out
//!   of bounds (0, N)`), because `image 0.24`'s pixel iterator steps to row 1 of a 0-wide image.
//!
//! Every test here PANICS on the unmodified `369938d` source and passes after the cure. Each one
//! collects all of its cases under `catch_unwind` first, so a failure names every case that still
//! panics rather than only the first one.
//!
//! The pinned outcome (the S125 cure): neither class gets a new error. (a) counts the lone
//! foreground pixel as one component; (b) gives the empty cell an empty grey image, exactly as
//! the zero-HEIGHT cells of the landscape mirrors already got. Both then flow on to the outcome
//! the library already gives the same-sized pictures that never panicked: at the default request
//! that is the existing `EmptyImage("analyze_section_pure")` refusal (CLI: `Error:
//! EmptyImage("analyze_section_pure")`, exit 1, instead of a panic and exit 101).
#![cfg(all(feature = "pure-analysis", feature = "synth"))]

use std::io::Cursor;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::process::Command;

use audiohax::cli::{PipelineArgs, Toggle};
use audiohax::mapping_loader::{load_mappings, MappingTable};
use audiohax::pure_analysis::{analyze_global_pure, understand_image_pure, AnalysisError};
use audiohax::render::{prepare_play, ImageInput, RenderError, RenderNote, RenderRequest};
use image::{ImageFormat, Rgb, RgbImage};

// ── fixtures ────────────────────────────────────────────────────────────────────────────────

/// Picture contents used throughout (the sweep in `tiny_picture_sweep_s125.rs` uses the same).
#[derive(Clone, Copy, Debug)]
enum Fill {
    Black,
    White,
    Grey,
    Red,
    /// Deterministic pseudo-random colours (a fixed LCG; no `rand` dependency).
    Noise,
}

fn picture(w: u32, h: u32, fill: Fill) -> RgbImage {
    let mut state: u32 = 0x5EED_0125;
    RgbImage::from_fn(w, h, |_, _| match fill {
        Fill::Black => Rgb([0, 0, 0]),
        Fill::White => Rgb([255, 255, 255]),
        Fill::Grey => Rgb([128, 128, 128]),
        Fill::Red => Rgb([220, 20, 20]),
        Fill::Noise => {
            let mut c = [0u8; 3];
            for v in c.iter_mut() {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                *v = (state >> 24) as u8;
            }
            Rgb(c)
        }
    })
}

fn png(img: &RgbImage) -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(img.clone())
        .write_to(&mut out, ImageFormat::Png)
        .expect("encode fixture png");
    out.into_inner()
}

fn manifest(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)
}

fn mappings() -> MappingTable {
    load_mappings(
        manifest("assets/mappings.json")
            .to_str()
            .expect("utf-8 path"),
    )
    .expect("assets/mappings.json loads")
}

/// The CLI's default request (`PipelineArgs::default()`: 4 instruments, thickness 0.10, 40
/// steps), with a fixed seed.
fn default_request() -> RenderRequest {
    RenderRequest {
        seed: Some(1),
        loop_cue: false,
        expression: Toggle::Off,
        v2: None,
        pipeline: PipelineArgs::default(),
        reverb: Toggle::On,
        gain: 1.0,
        event_dump: None,
    }
}

/// Run `f` for every case, catching panics; return `"<case>: <panic message>"` for each one
/// that panicked. The default panic hook still prints, which is what a failing run should show.
fn panicking_cases<T>(cases: &[(u32, u32, Fill)], f: impl Fn(&RgbImage) -> T) -> Vec<String> {
    let mut bad = Vec::new();
    for &(w, h, fill) in cases {
        let img = picture(w, h, fill);
        if let Err(p) = catch_unwind(AssertUnwindSafe(|| f(&img))) {
            let msg = p
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_else(|| "<non-string panic>".into());
            bad.push(format!("{w}x{h} {fill:?}: {msg}"));
        }
    }
    bad
}

/// Class (a): 1×1 pictures whose pixel is foreground after the Otsu threshold.
const CLASS_A: &[(u32, u32, Fill)] = &[
    (1, 1, Fill::White),
    (1, 1, Fill::Grey),
    (1, 1, Fill::Red),
    (1, 1, Fill::Noise),
];

/// Class (b): 1- and 2-px-wide portrait pictures (sizes from `s123-SH` § 3.1 and `s124-06B`).
fn class_b() -> Vec<(u32, u32, Fill)> {
    let mut v = Vec::new();
    for &(w, h) in &[(1, 4), (1, 16), (1, 40), (2, 40), (1, 50), (2, 100), (2, 6)] {
        for fill in [Fill::Black, Fill::White, Fill::Noise] {
            v.push((w, h, fill));
        }
    }
    v
}

// ── class (a) ───────────────────────────────────────────────────────────────────────────────

/// (a) at its true site: `analyze_global_pure` (whose `shape_complexity_pure` calls
/// `connected_components`) must not panic on a 1×1 foreground picture.
#[test]
fn class_a_global_features_do_not_panic_on_a_bright_1x1() {
    let bad = panicking_cases(CLASS_A, |img| analyze_global_pure(img).map(|_| ()));
    assert!(bad.is_empty(), "still panicking:\n{}", bad.join("\n"));
}

/// (a) the defined result: one foreground pixel is ONE connected component, so
/// `shape_complexity` = 1/1000 (the module's `count / 1000` proxy). A 1×1 BLACK picture never
/// panicked and stays at 0 components.
#[test]
fn class_a_counts_a_lone_foreground_pixel_as_one_component() {
    let white = analyze_global_pure(&picture(1, 1, Fill::White)).expect("1x1 white analyses");
    assert_eq!(
        white.shape_complexity.to_bits(),
        (1.0f32 / 1000.0).to_bits()
    );
    let black = analyze_global_pure(&picture(1, 1, Fill::Black)).expect("1x1 black analyses");
    assert_eq!(black.shape_complexity.to_bits(), 0.0f32.to_bits());
}

// ── class (b) ───────────────────────────────────────────────────────────────────────────────

/// (b) at its true site: `understand_image_pure` (whose 3×3 `analyze_regions_pure` grid has
/// zero-width cells when the picture is under 3 px wide) must not panic.
#[test]
fn class_b_understanding_does_not_panic_on_1_and_2_px_wide_pictures() {
    let bad = panicking_cases(&class_b(), |img| understand_image_pure(img).map(|_| ()));
    assert!(bad.is_empty(), "still panicking:\n{}", bad.join("\n"));
}

/// (b) the defined result is finite: no NaN or infinity leaks out of the empty cells.
#[test]
fn class_b_understanding_is_finite() {
    let mut bad = Vec::new();
    for (w, h, fill) in class_b() {
        let r = catch_unwind(|| understand_image_pure(&picture(w, h, fill)));
        if let Ok(Ok(u)) = r {
            let s = format!("{u:?}");
            if s.contains("NaN") || s.contains("inf") {
                bad.push(format!("{w}x{h} {fill:?}: {s}"));
            }
        } else {
            bad.push(format!("{w}x{h} {fill:?}: did not return Ok"));
        }
    }
    assert!(bad.is_empty(), "non-finite or failing:\n{}", bad.join("\n"));
}

// ── both classes, through the library entry point the CLI and the window use ────────────────

/// Both classes through `render::prepare_play` with the default request: no panic, and the
/// outcome is the library's EXISTING refusal for a picture too small to scan,
/// `EmptyImage("analyze_section_pure")` — the same refusal the landscape mirror of each picture
/// (and a 1×1 black picture) already got on `369938d`.
#[test]
fn both_classes_return_the_existing_empty_image_refusal_through_prepare_play() {
    let m = mappings();
    let req = default_request();
    let mut cases: Vec<(u32, u32, Fill)> = CLASS_A.to_vec();
    cases.extend(class_b());
    let mut bad = Vec::new();
    for (w, h, fill) in cases {
        let bytes = png(&picture(w, h, fill));
        let r = catch_unwind(AssertUnwindSafe(|| {
            prepare_play(&req, ImageInput::Bytes(&bytes), &m, &mut |_: RenderNote<
                '_,
            >|
             -> Result<
                (),
                RenderError,
            > {
                Ok(())
            })
            .map(|_| ())
        }));
        match r {
            Err(_) => bad.push(format!("{w}x{h} {fill:?}: PANIC")),
            Ok(Ok(())) => bad.push(format!("{w}x{h} {fill:?}: rendered (expected the refusal)")),
            Ok(Err(RenderError::Analysis(AnalysisError::EmptyImage("analyze_section_pure")))) => {}
            Ok(Err(e)) => bad.push(format!("{w}x{h} {fill:?}: other error {e:?}")),
        }
    }
    assert!(bad.is_empty(), "unexpected outcomes:\n{}", bad.join("\n"));
}

/// What the CLI user sees: `audiohax render <1x40.png>` and `<1x1 white.png>` exit 1 with the
/// existing `Error: EmptyImage("analyze_section_pure")` line, not a panic (exit 101).
#[test]
fn cli_render_refuses_instead_of_panicking() {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("tiny-picture-panics-s125")
        .join(format!("{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    for (w, h, fill) in [
        (1, 40, Fill::Noise),
        (2, 40, Fill::White),
        (1, 1, Fill::White),
    ] {
        let img = dir.join(format!("{w}x{h}.png"));
        std::fs::write(&img, png(&picture(w, h, fill))).expect("write fixture");
        let wav = dir.join(format!("{w}x{h}.wav"));
        let out = Command::new(env!("CARGO_BIN_EXE_audiohax"))
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .arg("render")
            .arg(&img)
            .arg("--seed")
            .arg("1")
            .arg("--wav")
            .arg(&wav)
            .output()
            .expect("spawn audiohax");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            !stderr.contains("panicked"),
            "{w}x{h}: the CLI panicked:\n{stderr}"
        );
        assert_eq!(out.status.code(), Some(1), "{w}x{h}: stderr:\n{stderr}");
        assert!(
            stderr.contains("Error: EmptyImage(\"analyze_section_pure\")"),
            "{w}x{h}: stderr:\n{stderr}"
        );
        assert!(
            !wav.exists(),
            "{w}x{h}: a refused render must not write a WAV"
        );
    }
}

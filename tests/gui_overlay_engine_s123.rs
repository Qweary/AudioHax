//! S123 Test Engineer (Specialist 5): the scan-bar overlay against the ENGINE'S OWN geometry
//! (UI `s123-UI` § 4, criterion A4; build report HL-1).
//!
//! The builder's A4 test (`overlay::tests::a4_matches_an_independent_transcription_of_the_analyzer`)
//! compares `ScanGeometry` with a second hand transcription of the analyzer's arithmetic. Both
//! transcriptions come from the same reading of `pure_analysis.rs`, so a shared misreading, or a
//! later change to the analyzer, passes it. These tests ask the analyzer itself: a one-pixel
//! white line on a black picture lights exactly the scan steps (and voice sections) whose crops
//! contain it, read back through `engine::FeatureSource::scan_bar_features` — the same rows the
//! composer reads. And the playhead's three cases (bar / coda / un-reached) are checked against
//! `prepare_play`'s and `render`'s own step counts and against the engine's zero-bar padding.
#![cfg(feature = "gui")]

use audiohax::engine::FeatureSource;
use audiohax::gui::embedded_mappings;
use audiohax::gui::overlay::{Playhead, PxRect, ScanGeometry};
use audiohax::gui::state::Controls;
use audiohax::pure_analysis::{load_pure_image_bytes, PureAnalysisSource, PureImage};
use audiohax::render::{prepare_play, render, ImageInput, RenderRequest};
use audiohax::synth_sink::SoundFontSource;
use image::{ImageBuffer, ImageFormat, Rgb, RgbImage};
use std::io::Cursor;

// ── fixtures ────────────────────────────────────────────────────────────────────────────────

/// Black picture with ONE white line: a column at `x` (vertical line) or a row at `y`.
fn line_image(w: u32, h: u32, column: Option<u32>, row: Option<u32>) -> PureImage {
    let img = RgbImage::from_fn(w, h, |x, y| {
        if Some(x) == column || Some(y) == row {
            Rgb([255, 255, 255])
        } else {
            Rgb([0, 0, 0])
        }
    });
    PureImage::from_rgb(img)
}

fn structured_png(w: u32, h: u32) -> Vec<u8> {
    let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_fn(w, h, |x, y| {
        let block = ((x / 8) + (y / 8)) % 2 == 0;
        Rgb([
            ((x * 255) / w.max(1)) as u8,
            ((y * 255) / h.max(1)) as u8,
            if block { 220 } else { 30 },
        ])
    });
    let mut out = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(img)
        .write_to(&mut out, ImageFormat::Png)
        .expect("encode");
    out.into_inner()
}

/// Which (step, voice) cells of the ANALYZER read any lit pixel.
fn lit_cells(img: &PureImage, voices: usize, thickness: f32, steps: usize) -> Vec<Vec<bool>> {
    let src = PureAnalysisSource::extract(img, voices, thickness, steps, None).expect("extract");
    (0..src.step_count())
        .map(|s| {
            src.scan_bar_features(s, voices)
                .iter()
                .map(|f| f.avg_brightness > 0.0)
                .collect()
        })
        .collect()
}

fn contains_x(r: PxRect, x: u32) -> bool {
    x >= r.x && x < r.x + r.w
}
fn contains_y(r: PxRect, y: u32) -> bool {
    y >= r.y && y < r.y + r.h
}

fn geometry(w: u32, h: u32, thickness: f32, steps: usize, voices: usize) -> ScanGeometry {
    ScanGeometry {
        width: w,
        height: h,
        thickness,
        steps,
        voices,
    }
}

// Picture shapes: landscape, portrait, SQUARE (the analyzer sweeps it top→bottom), and one
// pixel wider than tall (the smallest landscape).
const SHAPES: [(u32, u32); 4] = [(60, 24), (24, 60), (36, 36), (37, 36)];

// ── A4: rectangles, from the analyzer itself ──────────────────────────────────────────────

/// For every pixel line ACROSS the sweep, the steps whose drawn bar covers that line are exactly
/// the steps whose analyzer crop read it. Covers orientation (square ⇒ horizontal bar), bar
/// thickness, the flush-at-both-ends travel, and one-step scans.
#[test]
fn drawn_bar_covers_exactly_the_pixels_each_step_read() {
    for (w, h) in SHAPES {
        for thickness in [0.1f32, 0.33] {
            for steps in [1usize, 5, 9] {
                let g = geometry(w, h, thickness, steps, 1);
                let vertical = w > h;
                assert_eq!(g.vertical(), vertical, "{w}x{h}: orientation");
                let sweep = if vertical { w } else { h };
                for p in 0..sweep {
                    let img = if vertical {
                        line_image(w, h, Some(p), None)
                    } else {
                        line_image(w, h, None, Some(p))
                    };
                    let lit = lit_cells(&img, 1, thickness, steps);
                    assert_eq!(lit.len(), g.scan_steps(), "{w}x{h} steps={steps}");
                    for (s, row) in lit.iter().enumerate() {
                        let r = g.scan_rect(s);
                        let drawn = if vertical {
                            contains_x(r, p)
                        } else {
                            contains_y(r, p)
                        };
                        assert_eq!(
                            drawn, row[0],
                            "{w}x{h} t={thickness} steps={steps}: line at {p}, step {s}: \
                             overlay draws {r:?} (covers={drawn}) but the analyzer read={}",
                            row[0]
                        );
                    }
                }
            }
        }
    }
}

/// The per-voice stripes the overlay draws inside the bar are exactly the analyzer's voice
/// sections (the last one absorbing the remainder), checked with a line ALONG the sweep.
#[test]
fn drawn_voice_stripes_are_the_analyzers_sections() {
    for (w, h) in SHAPES {
        for voices in [1usize, 3, 4, 5] {
            let g = geometry(w, h, 0.33, 4, voices);
            let vertical = w > h;
            let across = if vertical { h } else { w };
            for p in 0..across {
                let img = if vertical {
                    line_image(w, h, None, Some(p))
                } else {
                    line_image(w, h, Some(p), None)
                };
                let lit = lit_cells(&img, voices, 0.33, 4);
                for (s, row) in lit.iter().enumerate() {
                    let stripes = g.voice_stripes(g.scan_rect(s));
                    assert_eq!(stripes.len(), voices);
                    for (v, read) in row.iter().enumerate() {
                        let drawn = if vertical {
                            contains_y(stripes[v], p)
                        } else {
                            contains_x(stripes[v], p)
                        };
                        assert_eq!(
                            drawn, *read,
                            "{w}x{h} voices={voices}: line at {p}, step {s}, voice {v}: \
                             overlay {:?} covers={drawn}, analyzer read={read}",
                            stripes[v]
                        );
                    }
                }
            }
        }
    }
}

// ── the three playhead cases, against the engine's counts ──────────────────────────────────

fn request_with_steps(steps: usize) -> RenderRequest {
    let mut r = Controls {
        seed: Some(5),
        ..Controls::default()
    }
    .request();
    r.pipeline.steps = steps;
    r
}

/// `S` as the overlay counts it equals the plan's `source_step_count` (what the engine
/// actually scanned), including the `--steps 0` edge the analyzer reads as one step.
#[test]
fn overlay_scan_count_is_the_plans_source_step_count() {
    let png = structured_png(96, 64);
    let m = embedded_mappings().expect("mappings");
    for steps in [0usize, 1, 7, 40] {
        let req = request_with_steps(steps);
        let plan = prepare_play(&req, ImageInput::Bytes(&png), &m, &mut |_| Ok(())).expect("plan");
        let g = geometry(
            96,
            64,
            req.pipeline.thickness,
            steps,
            req.pipeline.instruments,
        );
        assert_eq!(
            g.scan_steps(),
            plan.source_step_count,
            "--steps {steps}: overlay S vs engine S"
        );
    }
}

/// Case 1 vs case 2 (coda): over the WHOLE composed piece, the overlay shows the bar exactly on
/// the steps that read real picture rows, and the coda exactly on the steps the engine pads with
/// neutral zero-bars. The fixture forces a coda (10 scan positions, a longer piece).
#[test]
fn coda_is_shown_on_exactly_the_engines_zero_bar_steps() {
    let png = structured_png(96, 64);
    let m = embedded_mappings().expect("mappings");
    let req = request_with_steps(10);
    let plan = prepare_play(&req, ImageInput::Bytes(&png), &m, &mut |_| Ok(())).expect("plan");
    assert!(
        plan.total_steps > plan.source_step_count,
        "precondition: the piece ({}) must outrun the scan ({}) for a coda to exist",
        plan.total_steps,
        plan.source_step_count
    );
    let img = load_pure_image_bytes(&png).expect("decode");
    let voices = req.pipeline.instruments;
    let src = PureAnalysisSource::extract(&img, voices, req.pipeline.thickness, 10, None)
        .expect("extract");
    let g = geometry(96, 64, req.pipeline.thickness, 10, voices);
    let mut codas = 0;
    for step in 0..plan.total_steps {
        let zero_row = src.scan_bar_features(step, voices).iter().all(|f| {
            f.avg_brightness == 0.0
                && f.avg_saturation == 0.0
                && f.avg_hue == 0.0
                && f.edge_density == 0.0
                && f.texture_laplacian_var == 0.0
        });
        let coda = g.playhead(step) == Playhead::Coda;
        assert_eq!(
            coda, zero_row,
            "step {step} of {}: overlay coda={coda}, engine zero-bars={zero_row}",
            plan.total_steps
        );
        if let Playhead::Bar(r) = g.playhead(step) {
            assert_eq!(
                r,
                g.scan_rect(step),
                "step {step}: the bar is this step's rect"
            );
        }
        codas += usize::from(coda);
    }
    assert_eq!(codas, plan.total_steps - plan.source_step_count);
    // A coda piece leaves nothing un-reached.
    assert_eq!(g.unreached(plan.total_steps), None);
}

/// Case 3 (un-reached tail): when the piece is SHORTER than the scan, the hatched region starts
/// exactly at the trailing edge of the last bar the piece reads, no heard step's bar enters it,
/// and the first unheard bar does. The piece length comes from the engine — from `render()`'s
/// summary (what the window uses after Render) and `prepare_play()` (after Play), which must
/// agree or the hatch would move between the two.
#[test]
fn unreached_region_starts_where_the_last_heard_bar_ends() {
    for (w, h) in [(96u32, 64u32), (64, 96), (72, 72)] {
        let png = structured_png(w, h);
        let m = embedded_mappings().expect("mappings");
        let req = request_with_steps(80);
        let plan = prepare_play(&req, ImageInput::Bytes(&png), &m, &mut |_| Ok(())).expect("plan");
        assert!(
            plan.total_steps < plan.source_step_count,
            "{w}x{h}: precondition: piece ({}) shorter than the scan ({})",
            plan.total_steps,
            plan.source_step_count
        );
        let g = geometry(w, h, req.pipeline.thickness, 80, req.pipeline.instruments);
        let un = g.unreached(plan.total_steps).expect("an un-reached tail");
        let last_heard = g.scan_rect(plan.total_steps - 1);
        let first_unheard = g.scan_rect(plan.total_steps);
        if g.vertical() {
            assert_eq!(un.x, last_heard.x + last_heard.w, "{w}x{h}: hatch start");
            assert_eq!((un.x + un.w, un.y, un.h), (w, 0, h), "{w}x{h}: to the edge");
            for s in 0..plan.total_steps {
                let r = g.scan_rect(s);
                assert!(
                    r.x + r.w <= un.x,
                    "{w}x{h}: heard step {s} {r:?} enters {un:?}"
                );
            }
            assert!(first_unheard.x + first_unheard.w > un.x);
        } else {
            assert_eq!(un.y, last_heard.y + last_heard.h, "{w}x{h}: hatch start");
            assert_eq!((un.y + un.h, un.x, un.w), (h, 0, w), "{w}x{h}: to the edge");
            for s in 0..plan.total_steps {
                let r = g.scan_rect(s);
                assert!(
                    r.y + r.h <= un.y,
                    "{w}x{h}: heard step {s} {r:?} enters {un:?}"
                );
            }
            assert!(first_unheard.y + first_unheard.h > un.y);
        }
        // Every composed step is a bar (no coda in a case-3 piece).
        for step in 0..plan.total_steps {
            assert!(matches!(g.playhead(step), Playhead::Bar(_)));
        }
        // Render's count (used for the hatch after Render) equals Play's.
        if (w, h) == (96, 64) {
            let out = render(
                &req,
                ImageInput::Bytes(&png),
                &m,
                SoundFontSource::Bundled,
                &mut |_| Ok(()),
            )
            .expect("render");
            assert_eq!(out.summary.total_steps, plan.total_steps);
        }
    }
}

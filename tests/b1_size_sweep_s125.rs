//! S125 lane 05 (Test Engineer): `s125-SH5` Part B, B-1 as written, and the data for B-2.
//!
//! B-1: no panic from any picture size. Sides `w, h ∈ {1..=20, 31, 32, 33, 40, 50, 64, 100}`
//! (every pair, 729), contents {black, white, mid-grey, two-tone split, seeded noise, 1-px
//! checker}, through `render::render` AND `render::prepare_play`, for every request in
//! `instruments ∈ {1, 2, 3, 4, 16}` × `thickness ∈ {0.01, 0.1, 1.0}` × `steps ∈ {1, 2, 40, 200}`
//! (the CLI default request, 4 / 0.10 / 40, is one of the 60). 729 × 6 × 60 × 2 = 524,880 calls.
//!
//! The test is `#[ignore]` and is driven from outside by environment variables, so that it can be
//! sharded over processes and so that an ABORT (which no `catch_unwind` can catch) costs one shard
//! and is still visible (the shard's TSV stops early and the process exit status says so):
//!
//! * `S125_B1_OUT`     — the TSV to write (one row per call, flushed per row). Required.
//! * `S125_B1_SHARD`   — this shard's index (default 0); `S125_B1_SHARDS` — shard count (default 1).
//!   A shard takes every `SHARDS`-th (size, content) cell.
//! * `S125_B1_ENTRY`   — `prepare`, `render` or `both` (default `both`).
//! * `S125_B1_MAX_SIDE` — optional: skip every size with a side above this (used for the DEBUG
//!   pass, where a 100×100 cell costs minutes; the cell numbering, and so the sharding, is unchanged).
//!
//! Each row is `w h content instruments thickness steps entry outcome`, where outcome is
//! `OK <hash>` (a hash of the full timeline, and for `render` of the samples as bit patterns
//! too), `ERR <Debug of the error>`, or `PANIC <message>`. The hash is the std SipHash with
//! fixed keys, so the same toolchain gives the same hash in two trees (B-2: run this file
//! unchanged against the `369938d` library and diff the TSVs).
//!
//! The test fails if any call panicked. It writes only the TSV the caller names.
#![cfg(all(feature = "pure-analysis", feature = "synth"))]

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::{BufWriter, Cursor, Write};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::Path;

use audiohax::cli::{PipelineArgs, Toggle};
use audiohax::mapping_loader::load_mappings;
use audiohax::render::{prepare_play, render, ImageInput, RenderError, RenderNote, RenderRequest};
use audiohax::synth_sink::SoundFontSource;
use image::{ImageFormat, Rgb, RgbImage};

const SIDES: [u32; 27] = [
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 31, 32, 33, 40, 50, 64,
    100,
];
const CONTENTS: [&str; 6] = ["black", "white", "grey", "split", "noise", "checker"];
const INSTRUMENTS: [usize; 5] = [1, 2, 3, 4, 16];
const THICKNESS: [f32; 3] = [0.01, 0.1, 1.0];
const STEPS: [usize; 4] = [1, 2, 40, 200];

fn picture(w: u32, h: u32, content: &str) -> RgbImage {
    let mut state: u32 = 0x5EED_0125 ^ (w << 16) ^ h;
    RgbImage::from_fn(w, h, |x, y| match content {
        "black" => Rgb([0, 0, 0]),
        "white" => Rgb([255, 255, 255]),
        "grey" => Rgb([128, 128, 128]),
        // Two-tone split: left half dark red, right half light blue (a 1-px-wide picture is all
        // the left tone).
        "split" => {
            if x < w / 2 {
                Rgb([150, 20, 20])
            } else {
                Rgb([170, 200, 250])
            }
        }
        "checker" => {
            if (x + y) % 2 == 0 {
                Rgb([0, 0, 0])
            } else {
                Rgb([255, 255, 255])
            }
        }
        _ => {
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
        .expect("encode png");
    out.into_inner()
}

fn request(instruments: usize, thickness: f32, steps: usize) -> RenderRequest {
    RenderRequest {
        seed: Some(1),
        loop_cue: false,
        expression: Toggle::Off,
        v2: None,
        pipeline: PipelineArgs {
            instruments,
            thickness,
            steps,
            ..PipelineArgs::default()
        },
        reverb: Toggle::On,
        gain: 1.0,
        event_dump: None,
    }
}

fn panic_text(p: &(dyn std::any::Any + Send)) -> String {
    p.downcast_ref::<String>()
        .cloned()
        .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
        .unwrap_or_else(|| "<non-string panic>".into())
        .replace(['\n', '\t'], " ")
}

fn outcome<T>(
    r: std::thread::Result<Result<T, RenderError>>,
    hash: impl Fn(&T) -> u64,
) -> (bool, String) {
    match r {
        Ok(Ok(v)) => (false, format!("OK {:016x}", hash(&v))),
        Ok(Err(e)) => (false, format!("ERR {e:?}").replace(['\n', '\t'], " ")),
        Err(p) => (true, format!("PANIC {}", panic_text(p.as_ref()))),
    }
}

#[test]
#[ignore = "B-1 full sweep; driven by S125_B1_* environment variables (see the module doc)"]
fn b1_full_size_sweep() {
    let out_path = std::env::var_os("S125_B1_OUT").expect("S125_B1_OUT is required");
    let shard: usize = std::env::var("S125_B1_SHARD").map_or(0, |s| s.parse().expect("shard"));
    let shards: usize = std::env::var("S125_B1_SHARDS").map_or(1, |s| s.parse().expect("shards"));
    let max_side: u32 =
        std::env::var("S125_B1_MAX_SIDE").map_or(u32::MAX, |s| s.parse().expect("max side"));
    let entry = std::env::var("S125_B1_ENTRY").unwrap_or_else(|_| "both".into());
    let (do_prepare, do_render) = match entry.as_str() {
        "prepare" => (true, false),
        "render" => (false, true),
        "both" => (true, true),
        other => panic!("S125_B1_ENTRY={other}"),
    };
    let m = load_mappings(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets/mappings.json")
            .to_str()
            .expect("utf-8 path"),
    )
    .expect("mappings load");

    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let mut out = BufWriter::new(std::fs::File::create(&out_path).expect("create TSV"));
    let (mut calls, mut panics) = (0usize, 0usize);
    let mut cell = 0usize;
    for &w in &SIDES {
        for &h in &SIDES {
            for content in CONTENTS {
                cell += 1;
                if (cell - 1) % shards != shard || w.max(h) > max_side {
                    continue;
                }
                let bytes = png(&picture(w, h, content));
                for &inst in &INSTRUMENTS {
                    for &th in &THICKNESS {
                        for &steps in &STEPS {
                            let req = request(inst, th, steps);
                            if do_prepare {
                                let r = catch_unwind(AssertUnwindSafe(|| {
                                    prepare_play(
                                        &req,
                                        ImageInput::Bytes(&bytes),
                                        &m,
                                        &mut |_: RenderNote<'_>| -> Result<(), RenderError> {
                                            Ok(())
                                        },
                                    )
                                }));
                                let (p, o) = outcome(r, |plan| {
                                    let mut hs = DefaultHasher::new();
                                    format!("{:?}", plan.timeline).hash(&mut hs);
                                    (
                                        plan.total_steps,
                                        plan.grid_ms_per_step,
                                        plan.instrument_count,
                                    )
                                        .hash(&mut hs);
                                    plan.programs.hash(&mut hs);
                                    plan.source_step_count.hash(&mut hs);
                                    hs.finish()
                                });
                                calls += 1;
                                panics += usize::from(p);
                                writeln!(
                                    out,
                                    "{w}\t{h}\t{content}\t{inst}\t{th}\t{steps}\tprepare\t{o}"
                                )
                                .expect("row");
                                out.flush().expect("flush");
                            }
                            if do_render {
                                let r = catch_unwind(AssertUnwindSafe(|| {
                                    render(
                                        &req,
                                        ImageInput::Bytes(&bytes),
                                        &m,
                                        SoundFontSource::Bundled,
                                        &mut |_: RenderNote<'_>| -> Result<(), RenderError> {
                                            Ok(())
                                        },
                                    )
                                }));
                                let (p, o) = outcome(r, |ro| {
                                    let mut hs = DefaultHasher::new();
                                    ro.sample_rate.hash(&mut hs);
                                    for s in &ro.interleaved {
                                        s.to_bits().hash(&mut hs);
                                    }
                                    format!("{:?}", ro.events).hash(&mut hs);
                                    hs.finish()
                                });
                                calls += 1;
                                panics += usize::from(p);
                                writeln!(
                                    out,
                                    "{w}\t{h}\t{content}\t{inst}\t{th}\t{steps}\trender\t{o}"
                                )
                                .expect("row");
                                out.flush().expect("flush");
                            }
                        }
                    }
                }
            }
        }
    }
    std::panic::set_hook(prev);
    println!("B1 shard={shard}/{shards} entry={entry} calls={calls} panics={panics}");
    assert_eq!(
        panics, 0,
        "{panics} of {calls} calls panicked (see the TSV)"
    );
}

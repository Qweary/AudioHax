//! S125 (D-45): a size sweep over tiny and thin pictures through `render::prepare_play`.
//!
//! IGNORED by default (it makes several thousand calls). Run it explicitly:
//!
//! ```text
//! cargo test --locked --test tiny_picture_sweep_s125 -- --ignored --nocapture
//! ```
//!
//! Set `S125_SWEEP_OUT=<file>` to also write one TSV row per call
//! (`w h fill voices steps outcome`). The test fails if any call panics.
//!
//! A sweep is not a proof: it samples sizes 1..=20 on each side plus a few long thin shapes, three
//! contents and a few voices/steps values. It cannot show that no other size panics.
#![cfg(all(feature = "pure-analysis", feature = "synth"))]

use std::io::{Cursor, Write};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::Path;

use audiohax::cli::{PipelineArgs, Toggle};
use audiohax::mapping_loader::load_mappings;
use audiohax::render::{prepare_play, ImageInput, RenderError, RenderNote, RenderRequest};
use image::{ImageFormat, Rgb, RgbImage};

fn picture(w: u32, h: u32, fill: &str) -> RgbImage {
    let mut state: u32 = 0x5EED_0125;
    RgbImage::from_fn(w, h, |_, _| match fill {
        "black" => Rgb([0, 0, 0]),
        "white" => Rgb([255, 255, 255]),
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

#[test]
#[ignore = "slow sweep; run with --ignored (see the module doc)"]
fn tiny_picture_sweep() {
    let m = load_mappings(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets/mappings.json")
            .to_str()
            .expect("utf-8 path"),
    )
    .expect("mappings load");

    let mut sizes: Vec<(u32, u32)> = Vec::new();
    for w in 1..=20 {
        for h in 1..=20 {
            sizes.push((w, h));
        }
    }
    sizes.extend([
        (1, 40),
        (2, 40),
        (1, 50),
        (2, 100),
        (40, 1),
        (40, 2),
        (50, 1),
        (100, 2),
    ]);

    // Silence the default hook; the payload is still recorded per case.
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));

    let mut out = std::env::var_os("S125_SWEEP_OUT")
        .map(|p| std::fs::File::create(p).expect("create sweep output"));
    let (mut ok, mut err, mut panics) = (0usize, 0usize, Vec::new());
    for &(w, h) in &sizes {
        for fill in ["black", "white", "noise"] {
            let mut bytes = Cursor::new(Vec::new());
            image::DynamicImage::ImageRgb8(picture(w, h, fill))
                .write_to(&mut bytes, ImageFormat::Png)
                .expect("encode");
            let bytes = bytes.into_inner();
            for voices in [1usize, 4, 16] {
                for steps in [1usize, 40] {
                    let req = RenderRequest {
                        seed: Some(1),
                        loop_cue: false,
                        expression: Toggle::Off,
                        v2: None,
                        pipeline: PipelineArgs {
                            instruments: voices,
                            steps,
                            ..PipelineArgs::default()
                        },
                        reverb: Toggle::On,
                        gain: 1.0,
                        event_dump: None,
                    };
                    let r = catch_unwind(AssertUnwindSafe(|| {
                        prepare_play(
                            &req,
                            ImageInput::Bytes(&bytes),
                            &m,
                            &mut |_: RenderNote<'_>| -> Result<(), RenderError> { Ok(()) },
                        )
                        .map(|_| ())
                    }));
                    let outcome = match r {
                        Ok(Ok(())) => {
                            ok += 1;
                            "OK".to_string()
                        }
                        Ok(Err(e)) => {
                            err += 1;
                            format!("ERR {e:?}")
                        }
                        Err(p) => {
                            let msg = p
                                .downcast_ref::<String>()
                                .cloned()
                                .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
                                .unwrap_or_else(|| "<non-string panic>".into());
                            panics.push(format!("{w}x{h} {fill} v{voices} s{steps}: {msg}"));
                            format!("PANIC {msg}")
                        }
                    };
                    if let Some(f) = out.as_mut() {
                        writeln!(f, "{w}\t{h}\t{fill}\tv{voices}\ts{steps}\t{outcome}")
                            .expect("write sweep row");
                    }
                }
            }
        }
    }
    std::panic::set_hook(prev);

    println!(
        "SWEEP calls={} ok={ok} err={err} panic={}",
        ok + err + panics.len(),
        panics.len()
    );
    for p in &panics {
        println!("PANIC-CASE {p}");
    }
    assert!(panics.is_empty(), "{} calls panicked", panics.len());
}

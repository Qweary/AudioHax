//! S125 FINAL SEAL, lane 05Z (§5 Test Engineer): "the size the user is told is the size the music is
//! made from" (§13 MUST-FIX 1 / M-1; SH8's "printed = analysed"), on EVERY surface that states a
//! shrunk size, at sizes where round-half-up and truncation DISAGREE.
//!
//! The existing tests pin the resize's output size against `shrunk_size` (`m1_…`, `apply_…`), and
//! the CLI notice's text only at sizes that divide exactly (160×100 in `sk10_…`), and the window's
//! headline only at 6000×4000 (`sk4_…`) and 120×80. So a surface that computed its own size with a
//! different rule (e.g. `side * pct / 100`, truncating) would print a size the music was NOT made
//! from, and nothing would turn red. This file closes that:
//!
//! * library: over every picture from 16×16 to 160×160 (step 1 on one side, a spread on the other)
//!   and every level that fits, the resize's output = `shrunk_size` = the size parsed back out of
//!   the CLI notice text (`shrink::cli_notice`);
//! * window (gui builds): the same sizes, parsed back out of the notice headline
//!   (`gui::text::shrink_notice_headline`) and the level row line (`gui::text::shrink_row_line`);
//! * CLI process: `render --shrink 75` on a 71×78 picture (round-half-up gives 53×59; truncation
//!   gives 53×58) prints 53x59, and its WAV equals the library's render at 75 % (so the music WAS
//!   made from the printed size, because the library's resize is pinned to `shrunk_size`).
#![cfg(all(feature = "pure-analysis", feature = "synth"))]

use audiohax::pure_analysis::PureImage;
use audiohax::shrink::{apply, cli_notice, fits, shrunk_size, Shrink};
use image::{Rgb, RgbImage};
use std::path::PathBuf;
use std::process::Command;

fn pic(w: u32, h: u32) -> PureImage {
    PureImage::from_rgb(RgbImage::from_fn(w, h, |x, y| {
        Rgb([(x * 7) as u8, (y * 5) as u8, ((x ^ y) * 3) as u8])
    }))
}

/// Parse "(WxH, from …" out of the CLI notice: the size the user is told.
fn told_by_cli(notice: &str) -> (u32, u32) {
    let open = notice.find("picture (").expect("notice shape") + "picture (".len();
    let rest = &notice[open..];
    let comma = rest.find(", from").expect("notice shape");
    let (w, h) = rest[..comma].split_once('x').expect("WxH");
    (w.parse().expect("w"), h.parse().expect("h"))
}

/// The sizes: one side 16..=160 step 1, the other from a spread that includes odd sides.
fn sizes() -> Vec<(u32, u32)> {
    let mut v = Vec::new();
    for a in 16..=160u32 {
        for b in [16u32, 21, 33, 50, 71, 78, 99, 101, 127, 160] {
            v.push((a, b));
            v.push((b, a));
        }
    }
    v
}

/// The sizes where truncation and round-half-up disagree at some level must be in the set, or the
/// test could not see the defect it exists for.
#[test]
fn the_size_set_contains_rounding_cases() {
    let disagree = sizes()
        .into_iter()
        .filter(|&(w, h)| {
            [Shrink::Percent75, Shrink::Percent50, Shrink::Percent25]
                .into_iter()
                .any(|s| shrunk_size(w, h, s) != (w * s.percent() / 100, h * s.percent() / 100))
        })
        .count();
    assert!(disagree > 1000, "only {disagree} rounding-sensitive sizes");
    assert_eq!(shrunk_size(71, 78, Shrink::Percent75), (53, 59));
    assert_eq!(
        (71 * 75 / 100, 78 * 75 / 100),
        (53, 58),
        "truncation differs here"
    );
}

/// Library + CLI notice: what the resize makes = `shrunk_size` = what the notice tells the user.
#[test]
fn the_cli_notice_tells_the_size_the_resize_makes() {
    // The resize itself on a subsample (it allocates); the notice arithmetic on every size.
    for (i, (w, h)) in sizes().into_iter().enumerate() {
        for s in [Shrink::Percent75, Shrink::Percent50, Shrink::Percent25] {
            if !fits(w, h, s) {
                continue;
            }
            let to = shrunk_size(w, h, s);
            let notice = cli_notice(s, (w, h), to);
            assert_eq!(told_by_cli(&notice), to, "{w}x{h} {s:?}: {notice}");
            if i % 7 == 0 {
                let out = apply(pic(w, h), s).expect("fits");
                assert_eq!((out.width(), out.height()), to, "{w}x{h} {s:?}: resize");
            }
        }
    }
}

/// The window's two surfaces that state a shrunk size: the notice headline ("Shrunk to 75% (53 ×
/// 59).") and the level row ("75% · 53 × 59"), against the resize's output.
#[cfg(feature = "gui")]
#[test]
fn the_window_tells_the_size_the_resize_makes() {
    use audiohax::gui::text::{shrink_notice_headline, shrink_row_line};
    let parse = |s: &str, after: &str| -> (u32, u32) {
        let at = s.find(after).expect("shape") + after.len();
        let rest = s[at..].trim_end_matches(['.', ')']);
        let (w, h) = rest.split_once(" × ").expect("W × H");
        (w.trim().parse().expect("w"), h.trim().parse().expect("h"))
    };
    for (i, (w, h)) in sizes().into_iter().enumerate() {
        for s in [Shrink::Percent75, Shrink::Percent50, Shrink::Percent25] {
            if !fits(w, h, s) {
                continue;
            }
            let to = shrunk_size(w, h, s);
            let head = shrink_notice_headline(s, w, h).expect("a shrink has a notice");
            assert_eq!(parse(&head, "% ("), to, "{w}x{h} {s:?}: {head}");
            let row = shrink_row_line(s, w, h);
            assert_eq!(parse(&row, " · "), to, "{w}x{h} {s:?}: {row}");
            if i % 7 == 0 {
                let out = apply(pic(w, h), s).expect("fits");
                assert_eq!((out.width(), out.height()), to, "{w}x{h} {s:?}: resize");
            }
        }
    }
    assert_eq!(
        shrink_notice_headline(Shrink::Percent75, 71, 78).as_deref(),
        Some("Shrunk to 75% (53 × 59).")
    );
}

/// The CLI process, at the size §13 found an aspect-fitting resize gets wrong: the notice says
/// 53x59, and the WAV is the library's 75 % render of the same picture.
#[test]
fn the_cli_prints_53x59_for_71x78_at_75_and_renders_that_piece() {
    use audiohax::cli::{Cli, Command as CliCommand};
    use audiohax::render::{render_shrunk, ImageInput, RenderRequest};
    use audiohax::synth_sink::SoundFontSource;
    use clap::Parser;
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("shrink-printed-s125")
        .join(format!("cli-{}", std::process::id()));
    std::fs::create_dir_all(&d).expect("dir");
    let img = RgbImage::from_fn(71, 78, |x, y| {
        Rgb([
            (x * 3) as u8,
            (y * 3) as u8,
            if (x / 8 + y / 8) % 2 == 0 { 200 } else { 40 },
        ])
    });
    let p = d.join("pic.png");
    img.save(&p).expect("save");
    let wav = d.join("out.wav");
    let out = Command::new(env!("CARGO_BIN_EXE_audiohax"))
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args([
            "render", "--seed", "7", "--steps", "4", "--shrink", "75", "--wav",
        ])
        .arg(&wav)
        .arg(&p)
        .output()
        .expect("spawn audiohax");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "{stderr}");
    assert_eq!(told_by_cli(&stderr), (53, 59), "{stderr}");
    assert!(stderr.contains("(53x59, from 71x78)"), "{stderr}");

    // The library's 75 % render, from the same request the CLI builds.
    let argv = [
        "audiohax", "render", "--seed", "7", "--steps", "4", "--shrink", "75", "x.png",
    ];
    let (req, shrink) = match Cli::try_parse_from(argv).expect("parses").command {
        CliCommand::Render(r) => (RenderRequest::from(&r), r.shrink.shrink),
        other => panic!("not render: {other:?}"),
    };
    assert_eq!(shrink, Shrink::Percent75);
    let bytes = std::fs::read(&p).expect("read");
    let m = audiohax::mapping_loader::default_mappings().expect("mappings");
    let lib = render_shrunk(
        &req,
        shrink,
        ImageInput::Bytes(&bytes),
        &m,
        SoundFontSource::Bundled,
        &mut |_| Ok(()),
    )
    .expect("render");
    let lib_wav = d.join("lib.wav");
    audiohax::synth_sink::write_stereo_wav(&lib_wav, lib.sample_rate, &lib.interleaved)
        .expect("write");
    assert_eq!(
        std::fs::read(&wav).expect("cli wav"),
        std::fs::read(&lib_wav).expect("lib wav"),
        "the CLI's 75 % piece is not the library's 75 % piece"
    );
    let _ = std::fs::remove_dir_all(&d);
}

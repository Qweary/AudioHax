//! S125 FINAL SEAL, lane 05Z (§5 Test Engineer): the CLI's picture policy pinned BOTH WAYS against
//! the pre-S125 decode itself (operator `s125-00-OPERATOR-RULINGS.md` § A.5, Q6 "a"; lead ruling
//! L-80: "`369938d`'s own acceptance set minus WebP").
//!
//! `369938d`'s CLI decoded with `image::open` (format from the EXTENSION, default `Limits`). The
//! `image` crate is the same locked 0.24.9 here, so `image::open` in this process IS `369938d`'s
//! decode, and it is the oracle: for every payload × every file name in the matrix below,
//! `load_pure_image` (the CLI's path arm) must give
//!
//! * the same PIXELS when `image::open` decodes the file (nothing it accepted is refused), and
//! * the same ERROR TEXT when `image::open` refuses it (nothing it refused is accepted, and the
//!   words do not change),
//!
//! with exactly one exception, operator Q1 "a": a file named `.webp` (any case), or a regular file
//! whose first bytes are `RIFF....WEBP`, is refused by the WebP sentence (or "a RIFF file" when it
//! starts with `RIFF` but is not WebP) and never reaches a WebP decoder.
//!
//! The named-row tests in `picture_gate_s125.rs` pin the findings one by one; this file pins the
//! RULE over a matrix of 18 payloads × 27 names (486 cases), including mis-named files, upper-case
//! extensions, no extension and the extensions of formats this build cannot decode (AVIF). It also
//! pins the 512 MiB decode-allocation edge BEHAVIOURALLY (a header-only claim of exactly 2^29
//! bytes passes the limit; 2^29 + 1 row does not), which until now only a constant assert held.
//! Files are written only under `CARGO_TARGET_TMPDIR`. Every picture is small (≤ 64 px a side).
#![cfg(all(feature = "pure-analysis", feature = "synth"))]

use audiohax::pure_analysis::{load_pure_image, AnalysisError, PureImageSource};
use image::{DynamicImage, ImageEncoder, ImageFormat, Rgb, RgbImage};
use std::io::Cursor;
use std::path::PathBuf;
use std::time::{Duration, Instant};

const WEBP_MSG: &str =
    "WebP pictures are not supported yet. Save the picture as PNG or JPEG and try again";
const RIFF_MSG: &str = "this is a RIFF file (sound, video or SoundFont), not a picture";

fn scratch(name: &str) -> PathBuf {
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("cli-open-matrix-s125")
        .join(format!("{name}-{}", std::process::id()));
    std::fs::create_dir_all(&d).expect("scratch dir");
    d
}

fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("hex"))
        .collect()
}

fn structured(w: u32, h: u32) -> RgbImage {
    RgbImage::from_fn(w, h, |x, y| {
        Rgb([
            (x * 255 / w) as u8,
            (y * 255 / h) as u8,
            if (x / 4 + y / 4) % 2 == 0 { 220 } else { 30 },
        ])
    })
}

fn write(img: DynamicImage, format: ImageFormat) -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    img.write_to(&mut out, format)
        .unwrap_or_else(|e| panic!("encode {format:?}: {e}"));
    out.into_inner()
}

/// A minimal Radiance HDR (`#?RADIANCE`, flat RGBE scanlines) of `w × h`.
fn hdr(w: u32, h: u32) -> Vec<u8> {
    let mut v = format!("#?RADIANCE\nFORMAT=32-bit_rle_rgbe\n\n-Y {h} +X {w}\n").into_bytes();
    for y in 0..h {
        for x in 0..w {
            v.extend_from_slice(&[(x * 9) as u8 | 1, (y * 7) as u8 | 1, 90, 128]);
        }
    }
    v
}

/// The payloads: one real picture per format this build reads (all ≤ 64 px a side), plus
/// non-pictures, a truncated picture, a WebP and a non-WebP RIFF.
fn payloads() -> Vec<(&'static str, Vec<u8>)> {
    let rgb = structured(37, 23);
    let dynrgb = DynamicImage::ImageRgb8(rgb.clone());
    let png = write(dynrgb.clone(), ImageFormat::Png);
    vec![
        ("png", png.clone()),
        ("jpeg", write(dynrgb.clone(), ImageFormat::Jpeg)),
        ("gif", write(dynrgb.clone(), ImageFormat::Gif)),
        ("bmp", write(dynrgb.clone(), ImageFormat::Bmp)),
        (
            "ico",
            write(
                DynamicImage::ImageRgba8(dynrgb.to_rgba8()),
                ImageFormat::Ico,
            ),
        ),
        ("tiff", write(dynrgb.clone(), ImageFormat::Tiff)),
        ("tga", write(dynrgb.clone(), ImageFormat::Tga)),
        ("ppm", write(dynrgb.clone(), ImageFormat::Pnm)),
        (
            "pgm",
            write(
                DynamicImage::ImageLuma8(dynrgb.to_luma8()),
                ImageFormat::Pnm,
            ),
        ),
        (
            "farbfeld",
            write(
                DynamicImage::ImageRgba16(dynrgb.to_rgba16()),
                ImageFormat::Farbfeld,
            ),
        ),
        (
            "exr",
            write(
                DynamicImage::ImageRgb32F(dynrgb.to_rgb32f()),
                ImageFormat::OpenExr,
            ),
        ),
        ("qoi", write(dynrgb.clone(), ImageFormat::Qoi)),
        ("hdr", hdr(29, 17)),
        ("truncated-png", png[..png.len() / 2].to_vec()),
        ("text", b"plain text, not a picture\n".to_vec()),
        ("empty", Vec::new()),
        // SH5 B01: a valid 16x16-class VP8L WebP; SH5 M01: a RIFF/WAVE header.
        (
            "webp",
            hex("5249464618000000574542505650384c0c0000002f3fc00b002860810adcff00"),
        ),
        ("riff-wave", hex("524946460400000057415645")),
    ]
}

/// The file names: every extension `ImageFormat::from_extension` knows, some in upper case, plus
/// an unknown extension and none at all.
const NAMES: [&str; 27] = [
    "x.png",
    "x.PNG",
    "x.jpg",
    "x.jpeg",
    "x.JPG",
    "x.gif",
    "x.bmp",
    "x.ico",
    "x.tif",
    "x.tiff",
    "x.tga",
    "x.ppm",
    "x.pgm",
    "x.pbm",
    "x.pam",
    "x.ff",
    "x.farbfeld",
    "x.exr",
    "x.qoi",
    "x.hdr",
    "x.dds",
    "x.avif",
    "x.webp",
    "x.WEBP",
    "x.Webp",
    "x.txt",
    "noext",
];

/// A result, printable (`PureImage` has no `Debug`).
fn show(r: &Result<audiohax::pure_analysis::PureImage, AnalysisError>) -> String {
    match r {
        Ok(img) => format!("Ok({}x{})", img.width(), img.height()),
        Err(e) => format!("Err({e:?})"),
    }
}

fn is_webp_name(name: &str) -> bool {
    name.to_ascii_lowercase().ends_with(".webp")
}

/// What the CLI must say for a WebP-route case (operator Q1 "a"), from the bytes. A name with
/// no known extension is NOT on the WebP route: it is refused by `ImageFormat::from_path` before
/// any byte is read, exactly as `image::open` refused it.
fn webp_route_words(name: &str, bytes: &[u8]) -> Option<&'static str> {
    if ImageFormat::from_path(name).is_err() {
        return None;
    }
    let riff = bytes.starts_with(b"RIFF");
    let webp = riff && bytes.get(8..12) == Some(b"WEBP".as_slice());
    if webp {
        Some(WEBP_MSG)
    } else if is_webp_name(name) {
        Some(if riff { RIFF_MSG } else { WEBP_MSG })
    } else {
        None
    }
}

/// THE RULE, over the whole matrix: the CLI path arm is `image::open` (pixels or words), except
/// the WebP route. Also non-vacuity: every correctly named real picture of a decodable format
/// must have been ACCEPTED by both, and mis-named and junk rows must have been REFUSED by both.
#[test]
fn the_cli_accepts_and_refuses_exactly_what_image_open_did_except_webp() {
    let d = scratch("matrix");
    let (mut accepted, mut refused, mut webp_rows) = (0usize, 0usize, 0usize);
    let mut deviations = Vec::new();
    for (label, bytes) in payloads() {
        for name in NAMES {
            let p = d.join(format!("{label}--{name}"));
            std::fs::write(&p, &bytes).expect("write");
            let before = image::open(&p).map(|i| i.to_rgb8());
            let t0 = Instant::now();
            let after = load_pure_image(&PureImageSource::UserPath(p.clone()));
            let took = t0.elapsed();
            let case = format!("{label} as {name}");
            if let Some(words) = webp_route_words(name, &bytes) {
                webp_rows += 1;
                match &after {
                    Err(AnalysisError::Decode(s)) if s == &format!("{}: {words}", p.display()) => {}
                    other => deviations.push(format!("{case}: WebP route gave {}", show(other))),
                }
                assert!(
                    took < Duration::from_millis(50),
                    "{case}: WebP route took {took:?}"
                );
                continue;
            }
            match (&before, &after) {
                (Ok(want), Ok(got)) => {
                    accepted += 1;
                    if got.as_rgb() != want {
                        deviations.push(format!("{case}: decoded to other pixels"));
                    }
                }
                (Err(e), Err(AnalysisError::Decode(s))) => {
                    refused += 1;
                    if s != &format!("{}: {e}", p.display()) {
                        deviations.push(format!("{case}: words changed: before {e:?}, now {s:?}"));
                    }
                }
                (Ok(_), Err(e)) => {
                    deviations.push(format!("{case}: image::open read it, now refused: {e:?}"))
                }
                (Err(e), Ok(_)) => deviations.push(format!(
                    "{case}: image::open refused it ({e}), now accepted"
                )),
                (Err(e), other) => {
                    deviations.push(format!("{case}: before {e}, now {}", show(other)))
                }
            }
        }
    }
    eprintln!("matrix: {accepted} accepted by both, {refused} refused by both, {webp_rows} WebP-route rows");
    assert!(
        deviations.is_empty(),
        "{} deviation(s):\n{}",
        deviations.len(),
        deviations.join("\n")
    );
    // Non-vacuity: the 14 correctly named real pictures (and their upper-case and alias names)
    // were accepted; the junk was refused; the WebP route was exercised by name and by bytes.
    assert!(
        accepted >= 20,
        "only {accepted} rows decoded: the matrix tests nothing"
    );
    assert!(refused >= 300, "only {refused} rows refused");
    // 3 WebP names x 18 payloads, plus the WebP bytes under the 22 other KNOWN extensions.
    assert_eq!(webp_rows, 3 * 18 + 22, "WebP route rows");
    let _ = std::fs::remove_dir_all(&d);
}

/// Every correctly named real picture renders on the CLI arm, by name (so a regression names the
/// format, not a matrix count). AVIF and DDS are refused by `image::open` in this build (no AVIF
/// decoder; no DDS encoder to build one), so they are rows of the matrix only.
#[test]
fn every_format_the_cli_read_before_still_reads() {
    let d = scratch("by-name");
    let named: [(&str, &str); 14] = [
        ("png", "x.png"),
        ("jpeg", "x.jpg"),
        ("gif", "x.gif"),
        ("bmp", "x.bmp"),
        ("ico", "x.ico"),
        ("tiff", "x.tif"),
        ("tga", "x.tga"),
        ("ppm", "x.ppm"),
        ("pgm", "x.pgm"),
        ("farbfeld", "x.ff"),
        ("exr", "x.exr"),
        ("qoi", "x.qoi"),
        ("hdr", "x.hdr"),
        ("png", "x.PNG"),
    ];
    let all = payloads();
    for (label, name) in named {
        let bytes = &all.iter().find(|(l, _)| *l == label).expect("payload").1;
        let p = d.join(name);
        std::fs::write(&p, bytes).expect("write");
        let want = image::open(&p)
            .unwrap_or_else(|e| panic!("{label} as {name}: image::open refused it: {e}"))
            .to_rgb8();
        let got = load_pure_image(&PureImageSource::UserPath(p.clone()))
            .unwrap_or_else(|e| panic!("{label} as {name}: the CLI refused it: {e:?}"));
        assert_eq!(got.as_rgb(), &want, "{label} as {name}");
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// A PNG whose IHDR CLAIMS `w × h` greyscale 8-bit (1 byte a pixel, so `total_bytes = w·h`),
/// valid CRC, 1×1 of real data: a header-only claim.
fn png_l8_claiming(w: u32, h: u32) -> Vec<u8> {
    let mut v = Vec::new();
    image::codecs::png::PngEncoder::new(&mut v)
        .write_image(&[0], 1, 1, image::ColorType::L8)
        .expect("encode");
    v[16..20].copy_from_slice(&w.to_be_bytes());
    v[20..24].copy_from_slice(&h.to_be_bytes());
    let mut c = crc32fast::Hasher::new();
    c.update(&v[12..29]);
    v[29..33].copy_from_slice(&c.finalize().to_be_bytes());
    v
}

/// The 512 MiB decode-allocation limit, at its exact edge, behaviourally (lead ruling L-80:
/// "the decoder keeps the `image` crate's default limits exactly as `369938d` used them"). A claim
/// of exactly 2^29 bytes (32768 × 16384 grey) passes the limit and fails later, on the missing
/// data; one more row (32768 × 16385) is refused by the limit itself. Both in `image::open`'s
/// words. A smaller or larger CLI limit moves one of the two rows. Neither row decodes any pixels,
/// so the test touches no large memory.
#[test]
fn the_cli_decode_limit_is_512_mib_at_its_exact_edge() {
    let d = scratch("edge");
    let limit_words = "Memory limit exceeded";
    for (w, h, over) in [(32_768u32, 16_384u32, false), (32_768, 16_385, true)] {
        let p = d.join(format!("claims-{w}x{h}.png"));
        std::fs::write(&p, png_l8_claiming(w, h)).expect("write");
        let before = image::open(&p).expect_err("a header-only claim never decodes");
        let got = match load_pure_image(&PureImageSource::UserPath(p.clone())) {
            Err(AnalysisError::Decode(s)) => s,
            other => panic!("{w}x{h}: expected a Decode refusal, got {}", show(&other)),
        };
        assert_eq!(got, format!("{}: {before}", p.display()), "{w}x{h}");
        assert_eq!(
            got.ends_with(limit_words),
            over,
            "{w}x{h} ({} bytes): refused by the limit = {}, want {over}",
            u64::from(w) * u64::from(h),
            got.ends_with(limit_words)
        );
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// A folder named like a picture: refused with the plain folder sentence, before any decoder
/// (kept beyond `369938d`, which garbled it; 02R3 § Words).
#[test]
fn a_folder_named_like_a_picture_is_refused_plainly() {
    let d = scratch("folder");
    let f = d.join("holiday.png");
    std::fs::create_dir_all(&f).expect("mkdir");
    let got = match load_pure_image(&PureImageSource::UserPath(f.clone())) {
        Err(AnalysisError::Decode(s)) => s,
        other => panic!("expected a Decode refusal, got {}", show(&other)),
    };
    assert_eq!(
        got,
        format!(
            "{}: this is not a picture file (it is a folder, a device or a pipe)",
            f.display()
        )
    );
    let _ = std::fs::remove_dir_all(&d);
}

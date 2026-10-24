//! S125 ITEM 1b (D-45): forcing tests for the ONE library picture gate (`src/picture_gate.rs`),
//! as specified by Shielding in `s125-SH5-prerelease-spec.md` Part A and its FORCING TESTS table
//! (rows W*, F*, R*), under the per-front-end policies of ROUND 3 (operator
//! `s125-00-OPERATOR-RULINGS.md` § A.2/A.5, lead ruling L-80):
//!
//! * the CLI (`Policy::CLI`, reached through `load_pure_image`, the path arm, whose route is
//!   `picture_gate::open_and_decode`): exactly what `image::open` accepted at `369938d` (the
//!   format from the extension, the file streamed, the `image` crate's default limits), minus WebP.
//!   Wherever it is safe to run in-process, a CLI row is pinned against `image::open` ITSELF, the
//!   pre-S125 decode;
//! * the window (`Policy::WINDOW`, reached through `load_pure_image_bytes` and `gui::ingest`):
//!   PNG, JPEG, GIF, BMP, TIFF and the S123 caps, unchanged.
//!
//! WebP is refused by both. Every fixture is built here from bytes (the WebP and "other format"
//! rows are the exact bytes of SH5's generated files, embedded as hex). No committed fixture.
//! Files are written only under `CARGO_TARGET_TMPDIR`.
//!
//! "Refused fast" (SH5): the gate must refuse BEFORE any decoder allocates. In-process, each
//! refusal row asserts < 50 ms (500 ms where a real decoder runs first). A huge header with no
//! data is refused on the CLI by the decoder's default allocation limit, as at `369938d`; the
//! process-level behaviour under memory caps is in the lane report.
#![cfg(all(feature = "pure-analysis", feature = "synth"))]

use image::ImageEncoder;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use audiohax::picture_gate::{
    admit, allowed_format, check_dimensions, decode, open_and_decode, Admitted, PictureRefusal,
    Policy, MAX_IMAGE_BYTES, MAX_PIXELS, MAX_SIDE,
};
use audiohax::pure_analysis::{
    load_pure_image, load_pure_image_bytes, AnalysisError, PureImage, PureImageSource,
};
use image::{ImageFormat, Rgb, RgbImage};

// ── fixtures ────────────────────────────────────────────────────────────────────────────────

fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("hex"))
        .collect()
}

/// SH5's WebP fixtures, byte for byte (`fx/webp/<id>-*.webp`).
fn webp(id: &str) -> Vec<u8> {
    hex(match id {
        "B01" => "5249464618000000574542505650384c0c0000002f3fc00b002860810adcff00",
        "B03" => "524946463600000057454250565038202a0000009001009d012a1000100002c04c25a00274ba00039800fee4017ff041714fb51cf0ffe0375f16cbe00000",
        "H01" => "524946462400000057454250565038580a00000010000000feff00feff00414c50480500000001e87f440400",
        "H02" => "524946462400000057454250565038580a00000010000000fe3f00fe3f00414c50480500000001e87f440400",
        "H03" => "5249464618000000574542505650384c0c0000002fffffff0f2860810adcff00",
        "H05" => "524946462a00000057454250565038580a000000000000003f00003f00005650384c0c0000002fffffff0f2860810adcff00",
        "H06" => "524946465000000057454250565038580a000000020000003f00003f0000414e494d06000000000000000000414e4d46240000000000000000003f00003f0000640000005650384c0c0000002fffffff0f2860810adcff00",
        "M01" => "524946460400000057415645",
        "M02" => "5249464600000000",
        "M03" => "524946462800000057454250494343500800000000000000000000005650384c0c0000002f0fc003002860810adcff00",
        "M04" => "52494646140000005745425041424344080000000000000000000000",
        "M05" => "52494646f0ffffff574542505650384cf0ffffff2f0fc003002860810adcff00",
        "M06" => "5249464618000000574542505650384c0c0000002e0fc003002860810adcff00",
        "M07" => "524946461200000057454250565038580600000010000000ffff",
        "M08" => "524946462600000057454250565038580a000000000000003f00003f000045584946080000000000000000000000",
        "M09" => "524946462a00000057454250565038580a000000c10000003f00003f00005650384c0c0000002f3fc00f002860810adcff00",
        "P01" => "524946462600000057454250565038201a0000001002009d012aff3fff3f00000000000000000000000000000000",
        "P02" => "524946462600000057454250565038201a0000001102009d012a4000400000000000000000000000000000000000",
        "P03" => "524946462600000057454250565038201a0000001002009d012b4000400000000000000000000000000000000000",
        "P04" => "524946462600000057454250565038201a0000001002009d012a40c040c000000000000000000000000000000000",
        other => panic!("no fixture {other}"),
    })
}

/// A RIFF/WEBP container with the given chunks (each padded to even, as RIFF requires).
fn riff_webp(chunks: &[(&[u8; 4], Vec<u8>)], size_delta: i64) -> Vec<u8> {
    let mut body = b"WEBP".to_vec();
    for (id, data) in chunks {
        body.extend_from_slice(*id);
        body.extend_from_slice(&(data.len() as u32).to_le_bytes());
        body.extend_from_slice(data);
        if data.len() % 2 == 1 {
            body.push(0);
        }
    }
    let size = (body.len() as i64 + size_delta) as u32;
    let mut out = b"RIFF".to_vec();
    out.extend_from_slice(&size.to_le_bytes());
    out.extend(body);
    out
}

/// The `VP8 ` chunk payload of SH5's B03 (a PIL-encoded lossy 16×16).
fn b03_vp8_payload() -> Vec<u8> {
    let b = webp("B03");
    let len = u32::from_le_bytes([b[16], b[17], b[18], b[19]]) as usize;
    b[20..20 + len].to_vec()
}

/// A `VP8X` payload: flags, 3 reserved bytes, canvas width−1 and height−1 as u24.
fn vp8x(w: u32, h: u32, flags: u8) -> Vec<u8> {
    let mut v = vec![flags, 0, 0, 0];
    v.extend_from_slice(&(w - 1).to_le_bytes()[..3]);
    v.extend_from_slice(&(h - 1).to_le_bytes()[..3]);
    v
}

/// SH5's "other format" fixtures (`fx/other/O0n-*`), byte for byte, with their extensions.
fn other(id: &str) -> (Vec<u8>, &'static str) {
    let (h, ext) = match id {
        "O01" => ("000002000000000000000000ffffffff180000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000", "tga"),
        "O02" => ("6661726266656c64773594007735940000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000", "ff"),
        "O03" => ("716f696600061a8000061a80030000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000", "qoi"),
        "O04" => ("50360a3130303030303030303020313030303030303030300a3235350a00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000", "ppm"),
        "O05" => ("233f52414449414e43450a464f524d41543d33322d6269745f726c655f726762650a0a2d592031303030303030202b5820313030303030300a00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000", "hdr"),
        "O06" => ("444453207c00000007100000ffff0000ffff0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000020000000040000004458543100000000000000000000000000000000000000000010000000000000000000000000000000000000", "dds"),
        "O07" => ("424d00000000000000003600000028000000307500003075000001001800000000000000000000000000000000000000000000000000", "bmp"),
        other => panic!("no fixture {other}"),
    };
    (hex(h), ext)
}

/// A PNG whose IHDR CLAIMS `width × height` (valid CRC) with a 1×1 IDAT: header-only claims.
fn png_claiming(width: u32, height: u32, colour_type: u8) -> Vec<u8> {
    let mut v = png(&RgbImage::from_pixel(1, 1, Rgb([0, 0, 0])));
    v[16..20].copy_from_slice(&width.to_be_bytes());
    v[20..24].copy_from_slice(&height.to_be_bytes());
    v[25] = colour_type;
    let mut h = crc32fast::Hasher::new();
    h.update(&v[12..29]);
    v[29..33].copy_from_slice(&h.finalize().to_be_bytes());
    v
}

fn png(img: &RgbImage) -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(img.clone())
        .write_to(&mut out, ImageFormat::Png)
        .expect("encode png");
    out.into_inner()
}

fn structured(w: u32, h: u32) -> RgbImage {
    RgbImage::from_fn(w, h, |x, y| {
        Rgb([
            (x * 255 / w) as u8,
            (y * 255 / h) as u8,
            if (x / 8 + y / 8) % 2 == 0 { 220 } else { 30 },
        ])
    })
}

fn scratch(name: &str) -> PathBuf {
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("picture-gate-s125")
        .join(format!("{name}-{}", std::process::id()));
    std::fs::create_dir_all(&d).expect("scratch dir");
    d
}

fn decode_err(r: Result<PureImage, AnalysisError>) -> String {
    match r {
        Ok(img) => panic!("admitted a {}x{} picture", img.width(), img.height()),
        Err(AnalysisError::Decode(s)) => s,
        Err(other) => panic!("expected a Decode refusal, got {other:?}"),
    }
}

/// Refuse in-process through `load_pure_image_bytes`, within 50 ms; return the text.
fn refused_fast(bytes: &[u8]) -> String {
    let t0 = Instant::now();
    let s = decode_err(load_pure_image_bytes(bytes));
    let took = t0.elapsed();
    assert!(took < Duration::from_millis(50), "took {took:?}: {s}");
    s
}

const WEBP_MSG: &str =
    "WebP pictures are not supported yet. Save the picture as PNG or JPEG and try again";
const RIFF_MSG: &str = "this is a RIFF file (sound, video or SoundFont), not a picture";

/// Encode `img` in `format` (the CLI-only formats the `image` crate can write).
fn encode(img: &RgbImage, format: ImageFormat) -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(img.clone())
        .write_to(&mut out, format)
        .expect("encode");
    out.into_inner()
}

/// The CLI arm: write `bytes` as `name` and load it through `load_pure_image`.
fn cli_load(d: &Path, name: &str, bytes: &[u8]) -> Result<PureImage, AnalysisError> {
    let p = d.join(name);
    std::fs::write(&p, bytes).expect("write");
    load_pure_image(&PureImageSource::UserPath(p))
}

/// L-80: the CLI refuses `name` (written from `bytes`) with EXACTLY the words `image::open` (the
/// pre-S125 CLI decode) gives, within `limit`. Only for files whose `image::open` refusal is
/// itself cheap (a header the default limits or the decoder refuse).
fn cli_refuses_as_before(d: &Path, name: &str, bytes: &[u8], limit: Duration) -> String {
    let p = d.join(name);
    std::fs::write(&p, bytes).expect("write");
    let before = image::open(&p).expect_err("image::open refused it before S125");
    let t0 = Instant::now();
    let s = decode_err(load_pure_image(&PureImageSource::UserPath(p.clone())));
    assert!(t0.elapsed() < limit, "{name} took {:?}", t0.elapsed());
    assert_eq!(s, format!("{}: {before}", p.display()), "{name}");
    s
}

// ── W rows: WebP (A-W1), refused by BOTH policies ───────────────────────────────────────────

/// W1–W14, W17–W23: every SH5 WebP fixture is refused fast as WebP, before any WebP decoder,
/// through the window's arm AND through the CLI's, named `.webp` and named `.png`.
#[test]
fn w_rows_every_webp_fixture_is_refused_fast_by_both_policies() {
    let d = scratch("w-rows");
    for id in [
        "B01", "B03", "H01", "H02", "H03", "H05", "H06", "M03", "M04", "M05", "M06", "M07", "M08",
        "M09", "P01", "P02", "P03", "P04",
    ] {
        let b = webp(id);
        let s = refused_fast(&b);
        assert_eq!(s, format!("<in-memory image>: {WEBP_MSG}"), "{id}");
        for ext in ["webp", "png"] {
            let t0 = Instant::now();
            let s = decode_err(cli_load(&d, &format!("{id}.{ext}"), &b));
            assert!(t0.elapsed() < Duration::from_millis(50), "{id}.{ext} slow");
            assert!(s.ends_with(WEBP_MSG), "{id}.{ext}: {s}");
        }
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// W15, W16: a RIFF file that is not WebP is "not a picture", not "WebP", under both policies
/// (the CLI: named `.webp`, so the WebP decoder would have read it). ROUND 3 (key A SHOULD-2): a
/// file named `.webp` that does NOT start with `RIFF` (text, empty) is not called "a RIFF file";
/// it gets the WebP sentence, and no WebP decoder runs.
#[test]
fn w15_w16_a_riff_that_is_not_webp_is_not_a_picture() {
    let d = scratch("w15");
    for id in ["M01", "M02"] {
        let s = refused_fast(&webp(id));
        assert_eq!(s, format!("<in-memory image>: {RIFF_MSG}"), "{id}");
        let s = decode_err(cli_load(&d, &format!("{id}.webp"), &webp(id)));
        assert!(s.ends_with(RIFF_MSG), "{id}: {s}");
    }
    for (name, bytes) in [
        ("text.webp", b"plain text, not a picture\n".to_vec()),
        ("empty.webp", Vec::new()),
    ] {
        let s = decode_err(cli_load(&d, name, &bytes));
        assert!(s.ends_with(WEBP_MSG), "{name}: {s}");
        assert!(!s.contains("RIFF"), "{name}: {s}");
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// W8, W9: the dimension-independent VP8L bombs. Their refusal depends only on the first 12
/// bytes, so a file of H08's and H09's SIZE with a valid VP8L 16×16 header is used (the measured
/// originals are run through the CLI in the lane report).
#[test]
fn w8_w9_large_lossless_webp_is_refused_fast_whatever_its_size() {
    for len in [2_262_940usize, 9_052_060] {
        let mut v = riff_webp(&[(b"VP8L", hex("2f0fc003002860810adcff00"))], 0);
        v.resize(len, 0);
        let riff_size = (len - 8) as u32;
        v[4..8].copy_from_slice(&riff_size.to_le_bytes());
        let s = refused_fast(&v);
        assert!(s.ends_with(WEBP_MSG), "{len}: {s}");
        let d = scratch("w8-w9");
        let t0 = Instant::now();
        let s = decode_err(cli_load(&d, "big.webp", &v));
        assert!(t0.elapsed() < Duration::from_millis(50), "{len} slow");
        assert!(s.ends_with(WEBP_MSG), "{len}: {s}");
        let _ = std::fs::remove_dir_all(&d);
    }
}

/// W10, W24, W25: lossy-WebP variants built from B03 (canvas ≠ frame; unknown chunk appended;
/// `riff_size` claiming more than the file). Each is refused as WebP by both policies.
#[test]
fn w10_w24_w25_lossy_variants_are_refused_fast() {
    let mut big_frame = b03_vp8_payload();
    big_frame[6..8].copy_from_slice(&128u16.to_le_bytes());
    big_frame[8..10].copy_from_slice(&128u16.to_le_bytes());
    let w10 = riff_webp(&[(b"VP8X", vp8x(64, 64, 0)), (b"VP8 ", big_frame)], 0);
    let w24 = riff_webp(&[(b"VP8 ", b03_vp8_payload()), (b"ABCD", vec![0; 4])], 0);
    let w25 = riff_webp(&[(b"VP8 ", b03_vp8_payload())], 100);
    let d = scratch("w10");
    for (name, v) in [("W10", w10), ("W24", w24), ("W25", w25)] {
        assert!(refused_fast(&v).ends_with(WEBP_MSG), "{name}");
        let s = decode_err(cli_load(&d, &format!("{name}.webp"), &v));
        assert!(s.ends_with(WEBP_MSG), "{name}: {s}");
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// W26, ROUND 3 (L-80). The window follows the SNIFFED format (A-P3): a PNG's bytes are a PNG
/// whatever the name. The CLI follows the file EXTENSION, exactly as `image::open` did at
/// `369938d`, and never runs the WebP decoder: a WebP named `x.png` is refused as WebP (its first
/// bytes say so), and a PNG named `x.webp` is refused, as `369938d` refused it (it went to the WebP
/// decoder). A PNG named `x.jpg` is refused by the JPEG decoder, in its own words, as before.
#[test]
fn w26_webp_is_refused_whatever_its_name_and_the_cli_keeps_the_extension() {
    let d = scratch("w26");
    let as_png = d.join("x.png");
    std::fs::write(&as_png, webp("B01")).expect("write");
    let s = decode_err(load_pure_image(&PureImageSource::UserPath(as_png.clone())));
    assert_eq!(s, format!("{}: {WEBP_MSG}", as_png.display()));

    let a_png = png(&structured(64, 48));
    let img = load_pure_image_bytes(&a_png).expect("the window sniffs a PNG");
    assert_eq!((img.width(), img.height()), (64, 48));
    let as_webp = d.join("x.webp");
    std::fs::write(&as_webp, &a_png).expect("write");
    assert!(
        image::open(&as_webp).is_err(),
        "369938d's decode refused it"
    );
    let s = decode_err(load_pure_image(&PureImageSource::UserPath(as_webp.clone())));
    assert_eq!(s, format!("{}: {WEBP_MSG}", as_webp.display()));

    cli_refuses_as_before(&d, "x.jpg", &a_png, Duration::from_millis(500));
    let _ = std::fs::remove_dir_all(&d);
}

// ── F rows: the two policies' sizes and formats ─────────────────────────────────────────────

/// F1, F2: the greyscale PNG pixel-bomb HEADERS (20000², 12000²). The window refuses them from
/// the header. The CLI has no pixel cap (operator A.2/A.5): it does exactly what `image::open` did
/// with the same file (the 65535² RGBA claim is refused by the default decode limit, fast).
#[test]
fn f1_f2_png_pixel_headers_window_refuses_cli_admits() {
    let s = refused_fast(&png_claiming(20_000, 20_000, 0));
    assert_eq!(
        s,
        "<in-memory image>: the picture is 20000×20000 (400,000,000 pixels); the limit is \
         50,000,000 pixels"
    );
    let s = refused_fast(&png_claiming(12_000, 12_000, 0));
    assert!(s.contains("144,000,000 pixels"), "{s}");
    // The CLI: the same refusal, in the same words, as `image::open` gave before S125.
    let d = scratch("f1-f2");
    for (w, h, ct) in [
        (20_000u32, 20_000u32, 0u8),
        (12_000, 12_000, 0),
        (65_535, 65_535, 6),
    ] {
        let s = cli_refuses_as_before(
            &d,
            &format!("claims-{w}x{h}.png"),
            &png_claiming(w, h, ct),
            Duration::from_millis(500),
        );
        if ct == 6 {
            assert!(s.ends_with("Memory limit exceeded"), "{s}");
        }
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// F3, F4: the window's cap boundaries, checked by the gate's header step alone (no decode).
#[test]
fn f3_f4_window_cap_boundaries() {
    let w = Policy::WINDOW;
    let a = admit(&png_claiming(8160, 6120, 2), None, &w).expect("49.94 MP is admitted");
    assert_eq!(
        (a.width, a.height, a.format),
        (8160, 6120, ImageFormat::Png)
    );
    assert!(matches!(
        admit(&png_claiming(7072, 7072, 2), None, &w),
        Err(PictureRefusal::TooManyPixels {
            pixels: 50_013_184,
            ..
        })
    ));
    assert!(matches!(
        admit(&png_claiming(MAX_SIDE + 1, 16, 2), None, &w),
        Err(PictureRefusal::SideTooLarge { .. })
    ));
    assert!(admit(&png_claiming(MAX_SIDE, 16, 2), None, &w).is_ok());
    assert_eq!(MAX_PIXELS, 50_000_000);
    assert_eq!(MAX_IMAGE_BYTES, 268_435_456);
}

/// A real picture past every window limit renders through the CLI arm and is refused by the
/// window's: 32769×16 (a side over 32,768).
#[test]
fn a_picture_past_the_window_limits_loads_on_the_cli_and_is_refused_by_the_window() {
    let d = scratch("wide");
    let wide = png(&RgbImage::from_fn(MAX_SIDE + 1, 16, |x, y| {
        Rgb([(x % 251) as u8, (y * 9) as u8, 90])
    }));
    let img = cli_load(&d, "wide.png", &wide).expect("the CLI loads it");
    assert_eq!((img.width(), img.height()), (MAX_SIDE + 1, 16));
    let s = refused_fast(&wide);
    assert!(
        s.contains("no side may be longer than 32,768 pixels"),
        "{s}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// F5 under ROUND 2: formats outside the window's five load through the CLI arm (TGA, QOI and PNM,
/// written here as real 64×48 pictures; Farbfeld is covered by O02's header row, because the
/// `image` crate cannot write RGB8 Farbfeld) and are refused by the window's arm.
#[test]
fn f5_cli_reads_the_formats_the_window_refuses() {
    let d = scratch("f5");
    let pic = structured(64, 48);
    for (format, ext, name) in [
        (ImageFormat::Tga, "tga", "Tga"),
        (ImageFormat::Qoi, "qoi", "Qoi"),
        (ImageFormat::Pnm, "ppm", "Pnm"),
    ] {
        let bytes = encode(&pic, format);
        let img = cli_load(&d, &format!("x.{ext}"), &bytes).expect(name);
        assert_eq!((img.width(), img.height()), (64, 48), "{name}");
        assert_eq!(img.as_rgb().as_raw(), pic.as_raw(), "{name}: lossless");
        // The window refuses it: by type, or (TGA, which has no signature) as unrecognised.
        let s = refused_fast(&bytes);
        let want = if format == ImageFormat::Tga {
            "The image format could not be determined".to_string()
        } else {
            format!("{name} pictures are not supported")
        };
        assert!(s.contains(&want), "{name}: {s}");
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// F5 header rows (SH5's O01–O07, byte for byte): the window refuses each by type (BMP, which
/// is one of its five, by its pixel cap). The CLI refuses none by policy: each goes to its own
/// decoder, which refuses it exactly as `image::open` did at `369938d` (the default allocation
/// limit, or the decoder's own header check), fast.
#[test]
fn f5_header_rows_window_refuses_cli_refuses_as_before() {
    let d = scratch("f5-headers");
    for (id, window_says) in [
        ("O01", "Tga pictures are not supported"),
        ("O02", "Farbfeld pictures are not supported"),
        ("O03", "Qoi pictures are not supported"),
        ("O04", "Pnm pictures are not supported"),
        ("O05", "Hdr pictures are not supported"),
        ("O06", "Dds pictures are not supported"),
        ("O07", "the picture is 30000×30000 (900,000,000 pixels)"),
    ] {
        let (bytes, ext) = other(id);
        // The extension's format, as the CLI's path arm passes it (TGA has no signature).
        let fallback = ImageFormat::from_extension(ext).map(Ok);
        let window = admit(&bytes, fallback, &Policy::WINDOW)
            .expect_err("the window refuses it")
            .to_string();
        assert!(window.contains(window_says), "{id}: {window}");
        // The CLI: the decoder's own refusal, word for word what `image::open` said.
        let s = cli_refuses_as_before(
            &d,
            &format!("{id}.{ext}"),
            &bytes,
            Duration::from_millis(500),
        );
        assert!(
            !s.contains("not supported. Save"),
            "{id}: refused by policy: {s}"
        );
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// F6: a decode whose size differs from the admitted header is refused (A-C5's backstop) by the
/// gate's bytes route (`admit` + `decode`), whatever the policy handed to it. The window uses that
/// route; the CLI's route (`open_and_decode`) is `image::open`'s and has no header pre-pass.
#[test]
fn f6_decoded_size_must_equal_the_header_on_the_bytes_route() {
    let bytes = png(&structured(64, 48));
    for policy in [Policy::CLI, Policy::WINDOW] {
        let ok = admit(&bytes, None, &policy).expect("admit");
        assert!(decode(&bytes, &ok, &policy).is_ok());
        let lie = Admitted { height: 49, ..ok };
        assert_eq!(
            decode(&bytes, &lie, &policy).map(|_| ()),
            Err(PictureRefusal::SizeMismatch {
                header_width: 64,
                header_height: 49,
                width: 64,
                height: 48,
            })
        );
    }
}

/// The window's list is exactly PNG, JPEG, GIF, BMP, TIFF (A-C3); the CLI's is every format but
/// WebP; WebP is in neither.
#[test]
fn the_two_format_policies() {
    let all = [
        ImageFormat::Png,
        ImageFormat::Jpeg,
        ImageFormat::Gif,
        ImageFormat::WebP,
        ImageFormat::Pnm,
        ImageFormat::Tiff,
        ImageFormat::Tga,
        ImageFormat::Dds,
        ImageFormat::Bmp,
        ImageFormat::Ico,
        ImageFormat::Hdr,
        ImageFormat::OpenExr,
        ImageFormat::Farbfeld,
        ImageFormat::Avif,
        ImageFormat::Qoi,
    ];
    let window: Vec<_> = all
        .iter()
        .copied()
        .filter(|f| Policy::WINDOW.allows(*f))
        .collect();
    assert_eq!(
        window,
        vec![
            ImageFormat::Png,
            ImageFormat::Jpeg,
            ImageFormat::Gif,
            ImageFormat::Tiff,
            ImageFormat::Bmp
        ]
    );
    for f in all {
        assert_eq!(Policy::WINDOW.allows(f), allowed_format(f), "{f:?}");
        assert_eq!(Policy::CLI.allows(f), f != ImageFormat::WebP, "{f:?}");
    }
}

/// No smallest-side floor in either library policy (L-60): a 1×1 picture passes the gate. (What
/// happens to it next is the analyser's business, pinned by `tiny_picture_panics_s125.rs`.)
#[test]
fn the_library_policies_have_no_floor() {
    for policy in [Policy::CLI, Policy::WINDOW] {
        assert!(admit(&png(&structured(1, 1)), None, &policy).is_ok());
        assert!(check_dimensions(1, 1, &policy).is_ok());
    }
    let d = scratch("floor");
    let p = d.join("one.png");
    std::fs::write(&p, png(&structured(1, 1))).expect("write");
    let one = open_and_decode(&p, &Policy::CLI).expect("the CLI's route has no floor");
    assert_eq!((one.width(), one.height()), (1, 1));
    let _ = std::fs::remove_dir_all(&d);
    let floored = Policy {
        min_side: 16,
        ..Policy::WINDOW
    };
    assert!(matches!(
        check_dimensions(15, 15, &floored),
        Err(PictureRefusal::TooSmall { .. })
    ));
}

// ── R rows: the read ────────────────────────────────────────────────────────────────────────

/// R1 under ROUND 3. The window's read (regular files only, ≤ 256 MiB) is unchanged and pinned
/// by `gui::ingest`'s own tests; here it is pinned through the gate. The CLI's route streams the
/// file to its decoder as `image::open` did (no byte cap, nothing read into memory first), and
/// refuses a folder, an endless device and a missing path in the same words as before, at once.
#[test]
fn r1_reads_under_each_policy() {
    use audiohax::picture_gate::read_bounded_with_cap;
    let d = scratch("r1");
    let not_a_file = "this is not a picture file (it is a folder, a device or a pipe)";
    // Window.
    #[cfg(unix)]
    assert_eq!(
        read_bounded_with_cap(Path::new("/dev/zero"), MAX_IMAGE_BYTES),
        Err(PictureRefusal::NotARegularFile)
    );
    let sparse = d.join("sparse.png");
    let f = std::fs::File::create(&sparse).expect("create");
    f.set_len(MAX_IMAGE_BYTES + 1).expect("sparse");
    assert_eq!(
        read_bounded_with_cap(&sparse, MAX_IMAGE_BYTES),
        Err(PictureRefusal::FileTooBig {
            cap: MAX_IMAGE_BYTES
        })
    );
    // CLI: no byte cap; the PNG decoder reads the stream and refuses it in its own words, as
    // `image::open` did, without reading the 256 MiB into memory first (so: fast).
    let t0 = Instant::now();
    let s = decode_err(load_pure_image(&PureImageSource::UserPath(sparse.clone())));
    assert!(
        t0.elapsed() < Duration::from_millis(500),
        "the sparse file was read"
    );
    assert_eq!(
        s,
        format!(
            "{}: {}",
            sparse.display(),
            image::open(&sparse).expect_err("refused before")
        )
    );
    // CLI: a folder, an endless device, a missing path.
    let check = |p: &Path, want: &str| {
        let t0 = Instant::now();
        let s = decode_err(load_pure_image(&PureImageSource::UserPath(p.to_path_buf())));
        assert!(
            t0.elapsed() < Duration::from_millis(500),
            "{} slow",
            p.display()
        );
        assert_eq!(s, format!("{}: {want}", p.display()));
    };
    check(&d, not_a_file);
    #[cfg(unix)]
    check(
        Path::new("/dev/zero"),
        &image::open("/dev/zero")
            .expect_err("refused before")
            .to_string(),
    );
    check(
        &d.join("nope.png"),
        "No such file or directory (os error 2)",
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// `369938d` rendered a picture piped through a FIFO; the CLI arm still loads it, byte for byte
/// the same picture as the regular file. A FIFO carrying something that is not a picture is
/// refused by the extension's decoder, in its own words, as at `369938d`.
#[cfg(unix)]
#[test]
fn a_picture_through_a_fifo_still_loads_on_the_cli() {
    let d = scratch("fifo");
    let bytes = png(&structured(64, 48));
    for (name, payload, want_ok) in [
        ("p.png", bytes.clone(), true),
        ("junk.png", vec![b'x'; 5000], false),
    ] {
        let fifo = d.join(name);
        let st = Command::new("mkfifo").arg(&fifo).status().expect("mkfifo");
        assert!(st.success());
        let writer = {
            let fifo = fifo.clone();
            std::thread::spawn(move || {
                let mut f = std::fs::OpenOptions::new()
                    .write(true)
                    .open(&fifo)
                    .expect("open fifo for writing");
                let _ = f.write_all(&payload);
            })
        };
        let r = load_pure_image(&PureImageSource::UserPath(fifo.clone()));
        let _ = writer.join();
        if want_ok {
            let img = r.expect("a PNG through a FIFO loads");
            assert_eq!(img.as_rgb().as_raw(), structured(64, 48).as_raw());
        } else {
            assert_eq!(
                decode_err(r),
                format!(
                    "{}: Format error decoding Png: Invalid PNG signature.",
                    fifo.display()
                )
            );
        }
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// The two error texts the identity goldens pin (`r_missing_image`, `r_corrupt_image`) are
/// produced verbatim by the gated CLI path: a missing file, and a non-picture named `.png` (the
/// extension fallback gives the PNG decoder's own words, exactly as `image::open` did).
#[test]
fn errors_pinned_by_the_identity_goldens_keep_their_exact_text() {
    let d = scratch("pinned");
    let missing = d.join("nope.png");
    assert_eq!(
        decode_err(load_pure_image(&PureImageSource::UserPath(missing.clone()))),
        format!(
            "{}: No such file or directory (os error 2)",
            missing.display()
        )
    );
    let notapng = d.join("notapng.png");
    std::fs::write(&notapng, b"not a png\n").expect("write");
    assert_eq!(
        decode_err(load_pure_image(&PureImageSource::UserPath(notapng.clone()))),
        format!(
            "{}: Format error decoding Png: Invalid PNG signature.",
            notapng.display()
        )
    );
    // The in-memory arm keeps `image::load_from_memory`'s words for unknown bytes.
    assert_eq!(
        decode_err(load_pure_image_bytes(b"not a png\n")),
        "<in-memory image>: The image format could not be determined"
    );
    let _ = std::fs::remove_dir_all(&d);
}

// ── CLI binary ──────────────────────────────────────────────────────────────────────────────

/// Through the CLI binary: WebP and RIFF refusals exit 1, one line, no panic text, no WAV; a TGA
/// (a format the window refuses) renders.
#[test]
fn cli_render_refuses_webp_and_renders_a_tga() {
    let d = scratch("cli");
    let run = |img: &Path, wav: &Path| {
        Command::new(env!("CARGO_BIN_EXE_audiohax"))
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .arg("render")
            .arg(img)
            .args(["--seed", "1", "--steps", "4", "--wav"])
            .arg(wav)
            .output()
            .expect("spawn audiohax")
    };
    for (name, bytes, want) in [
        ("w1.webp", webp("B01"), WEBP_MSG),
        ("w3.webp", webp("H01"), WEBP_MSG),
        ("w6.webp", webp("H05"), WEBP_MSG),
        ("w26.png", webp("B01"), WEBP_MSG),
        ("w15.webp", webp("M01"), RIFF_MSG),
    ] {
        let img = d.join(name);
        std::fs::write(&img, &bytes).expect("write");
        let wav = d.join(format!("{name}.wav"));
        let out = run(&img, &wav);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(1), "{name}: {stderr}");
        assert!(!stderr.contains("panicked"), "{name}: {stderr}");
        assert!(stderr.contains(want), "{name}: {stderr}");
        assert_eq!(stderr.lines().count(), 1, "{name}: one line: {stderr}");
        assert!(!wav.exists(), "{name}: no WAV");
    }
    let tga = d.join("x.tga");
    std::fs::write(&tga, encode(&structured(64, 48), ImageFormat::Tga)).expect("write");
    let wav = d.join("tga.wav");
    let out = run(&tga, &wav);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(wav.exists(), "the TGA rendered");
    let _ = std::fs::remove_dir_all(&d);
}

/// ROUND 3 (L-80; SH7-1, SH7-S1, key A MUST-1/SHOULD-1, key B MUST-1/2/3): the CLI decodes with
/// the `image` crate's default limits, as `image::open` did at `369938d`. So every small file whose
/// header claims a huge picture is refused cleanly and fast, in `image::open`'s own words, at any
/// memory cap (the limit is checked before the decoder allocates); every real picture decodes to
/// `image::open`'s pixels; and the GIFs whose frame is larger than their screen render again, as
/// they did at `369938d` (round 2 refused them).
#[test]
fn the_cli_refuses_huge_claims_by_the_default_limits_and_decodes_what_369938d_decoded() {
    let d = scratch("round3");
    let fast = Duration::from_millis(500);
    let refused: Vec<(&str, Vec<u8>)> = vec![
        // SH7-1 / X02: a 66-byte RLE8 BMP claiming 30000² (aborted round 2 under a 4 GiB cap).
        ("x02-rle8-30000.bmp", bmp_rle8_eob(30_000)),
        // key A MUST-1: a 24-bit BMP claiming 65535² over 54 bytes of pixels (aborted at 8 GiB).
        ("bmp-lie-65535.bmp", bmp_rgb24_claiming(65_535, 65_535)),
        // key B MUST-2: the same lie, 40000² and 40000×−40000 (top-down).
        ("bmp-lie-40000.bmp", bmp_rgb24_claiming(40_000, 40_000)),
        (
            "bmp-lie-40000-topdown.bmp",
            bmp_rgb24_claiming(40_000, -40_000),
        ),
        // key A SHOULD-1 / key B MUST-1: valid GIFs whose logical SCREEN is huge, frame tiny.
        ("gif-screen-30000.gif", gif(30_000, 30_000, 1, 1)),
        ("gif-screen-65535.gif", gif(65_535, 65_535, 1, 1)),
        (
            "gif-screen-and-frame-65535.gif",
            gif(65_535, 65_535, 65_535, 65_535),
        ),
        // `s125-05`'s frame bomb: a 16×16 screen, a 65535² first frame.
        ("gif-frame-bomb.gif", gif_frame_bomb()),
        // PNG IHDR claims; SH5's TGA, PPM and HDR claims.
        ("png-claims-65535-rgba.png", png_claiming(65_535, 65_535, 6)),
        ("o01.tga", other("O01").0),
        ("o04.ppm", other("O04").0),
        ("o05.hdr", other("O05").0),
    ];
    for (name, bytes) in &refused {
        let s = cli_refuses_as_before(&d, name, bytes, fast);
        assert!(!s.contains("panicked"), "{name}: {s}");
    }
    // A mid-size screen lie (12000², 576 MB of RGBA, over the 512 MiB default) is refused too
    // (key B MUST-1's `gif_mid`, which round 2 rendered after 309 s).
    cli_refuses_as_before(&d, "gif-screen-12000.gif", &gif(12_000, 12_000, 8, 8), fast);

    // Real pictures at the formats' highest compression decode, pixel for pixel as `image::open`.
    let grey = image::GrayImage::from_pixel(4000, 4000, image::Luma([77]));
    let mut solid_png = Vec::new();
    image::codecs::png::PngEncoder::new_with_quality(
        &mut solid_png,
        image::codecs::png::CompressionType::Best,
        image::codecs::png::FilterType::NoFilter,
    )
    .write_image(grey.as_raw(), 4000, 4000, image::ColorType::L8)
    .expect("encode");
    let solid = RgbImage::from_pixel(2000, 2000, Rgb([10, 200, 30]));
    for (name, bytes) in [
        ("solid.png", solid_png),
        ("solid.jpg", encode(&solid, ImageFormat::Jpeg)),
        ("solid.qoi", encode(&solid, ImageFormat::Qoi)),
        // SH7-S1 (N03's shape): a 16×16 screen with a 3000×16 first frame (full pixel data).
        // `369938d` rendered it; round 2 refused it; round 3 renders it again.
        ("frame-wider-than-screen.gif", gif_full(16, 16, 3000, 16)),
        // An 8×8 frame on a 64×64 screen (an ordinary small GIF shape).
        ("small-frame.gif", gif_full(64, 64, 8, 8)),
    ] {
        let p = d.join(name);
        std::fs::write(&p, &bytes).expect("write");
        let before = image::open(&p)
            .unwrap_or_else(|e| panic!("{name}: {e}"))
            .to_rgb8();
        let gated = load_pure_image(&PureImageSource::UserPath(p))
            .unwrap_or_else(|e| panic!("{name}: {e:?}"));
        assert_eq!(gated.as_rgb(), &before, "{name}");
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// SH7's X01–X03 shape: an RLE8 BMP (BITMAPINFOHEADER, 8 bpp, 2-colour palette) claiming
/// `side × side`, whose data is one run then the end-of-bitmap escape (66 bytes).
fn bmp_rle8_eob(side: i32) -> Vec<u8> {
    let data: [u8; 4] = [1, 0, 0, 1]; // a run of 1 pixel of colour 0; end of bitmap
    let off = 14 + 40 + 8;
    let mut v = b"BM".to_vec();
    v.extend_from_slice(&((off + data.len()) as u32).to_le_bytes());
    v.extend_from_slice(&[0, 0, 0, 0]);
    v.extend_from_slice(&(off as u32).to_le_bytes());
    v.extend_from_slice(&40u32.to_le_bytes());
    v.extend_from_slice(&side.to_le_bytes());
    v.extend_from_slice(&side.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&8u16.to_le_bytes());
    v.extend_from_slice(&1u32.to_le_bytes()); // BI_RLE8
    v.extend_from_slice(&(data.len() as u32).to_le_bytes());
    v.extend_from_slice(&[0; 8]);
    v.extend_from_slice(&2u32.to_le_bytes()); // colours used
    v.extend_from_slice(&0u32.to_le_bytes());
    v.extend_from_slice(&[0, 0, 0, 0, 255, 255, 255, 0]);
    v.extend_from_slice(&data);
    v
}

/// key A's `bmp_lie_*` shape: an uncompressed 24-bit BMP whose header claims `w × h` over 54
/// bytes of pixels (108 bytes in all). A negative height is a top-down BMP.
fn bmp_rgb24_claiming(w: i32, h: i32) -> Vec<u8> {
    let mut v = b"BM".to_vec();
    v.extend_from_slice(&108u32.to_le_bytes());
    v.extend_from_slice(&[0, 0, 0, 0]);
    v.extend_from_slice(&54u32.to_le_bytes());
    v.extend_from_slice(&40u32.to_le_bytes());
    v.extend_from_slice(&w.to_le_bytes());
    v.extend_from_slice(&h.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&24u16.to_le_bytes());
    v.extend_from_slice(&[0; 24]); // BI_RGB, sizes, resolution, colours
    v.extend_from_slice(&[0x7f; 54]);
    v
}

/// Like [`gif`], but the frame carries ALL its pixels (a checkerboard of the two colours), as an
/// uncompressed LZW stream: 3-bit codes, a clear code before every two pixels, so the code size
/// never grows (every GIF decoder reads this).
fn gif_full(sw: u16, sh: u16, fw: u16, fh: u16) -> Vec<u8> {
    let (clear, end) = (4u32, 5u32);
    let mut codes = Vec::new();
    let n = usize::from(fw) * usize::from(fh);
    for i in 0..n {
        if i % 2 == 0 {
            codes.push(clear);
        }
        let (x, y) = (i % usize::from(fw), i / usize::from(fw));
        codes.push(((x / 4 + y / 4) % 2) as u32);
    }
    codes.push(end);
    let mut data = Vec::new();
    let (mut acc, mut bits) = (0u32, 0u32);
    for c in codes {
        acc |= c << bits;
        bits += 3;
        while bits >= 8 {
            data.push((acc & 0xff) as u8);
            acc >>= 8;
            bits -= 8;
        }
    }
    if bits > 0 {
        data.push((acc & 0xff) as u8);
    }
    let mut v = b"GIF89a".to_vec();
    v.extend_from_slice(&sw.to_le_bytes());
    v.extend_from_slice(&sh.to_le_bytes());
    v.extend_from_slice(&[0x80, 0, 0, 0, 0, 0, 255, 255, 255, 0x2C, 0, 0, 0, 0]);
    v.extend_from_slice(&fw.to_le_bytes());
    v.extend_from_slice(&fh.to_le_bytes());
    v.extend_from_slice(&[0, 2]); // no local table; LZW minimum code size 2
    for block in data.chunks(255) {
        v.push(block.len() as u8);
        v.extend_from_slice(block);
    }
    v.extend_from_slice(&[0, 0x3B]);
    v
}

/// A GIF89a with a `sw × sh` logical screen (2-colour global table) and one `fw × fh` frame at
/// (0, 0) holding a token LZW stream.
fn gif(sw: u16, sh: u16, fw: u16, fh: u16) -> Vec<u8> {
    let mut v = b"GIF89a".to_vec();
    v.extend_from_slice(&sw.to_le_bytes());
    v.extend_from_slice(&sh.to_le_bytes());
    v.extend_from_slice(&[0x80, 0, 0, 0, 0, 0, 255, 255, 255, 0x2C, 0, 0, 0, 0]);
    v.extend_from_slice(&fw.to_le_bytes());
    v.extend_from_slice(&fh.to_le_bytes());
    v.extend_from_slice(&[0, 2, 2, 0x44, 0x01, 0, 0x3B]);
    v
}

/// `s125-05`'s GIF frame bomb: a 16×16 screen whose first frame claims 65535².
fn gif_frame_bomb() -> Vec<u8> {
    let mut v = b"GIF89a".to_vec();
    v.extend_from_slice(&16u16.to_le_bytes());
    v.extend_from_slice(&16u16.to_le_bytes());
    v.extend_from_slice(&[0x80, 0, 0, 0, 0, 0, 255, 255, 255, 0x2C, 0, 0, 0, 0]);
    v.extend_from_slice(&65_535u16.to_le_bytes());
    v.extend_from_slice(&65_535u16.to_le_bytes());
    v.extend_from_slice(&[0, 2, 2, 0x44, 0x01, 0, 0x3B]);
    v
}

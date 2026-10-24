//! S125 ITEM 1b (`s125-SH5` A-P2 and the FORCING TESTS preamble): the window and the library share
//! ONE gate. The window's numbers are the library's, its allowlist is the library's, and every
//! gate refusal reaches the window's `job::run_load` as a plain refusal, never as `Crashed`. The
//! window keeps its own 16 px floor (B-4) and its own wording (pinned by the S123 window tests).
#![cfg(feature = "gui")]

use audiohax::gui::ingest::{self, Refusal};
use audiohax::gui::job::{run_load, LoadSource};
use audiohax::gui::text::ErrorKind;
use audiohax::picture_gate;
use image::ImageFormat;

fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("hex"))
        .collect()
}

#[test]
fn the_windows_numbers_and_allowlist_are_the_librarys() {
    assert_eq!(ingest::MAX_IMAGE_BYTES, picture_gate::MAX_IMAGE_BYTES);
    assert_eq!(ingest::MAX_SIDE, picture_gate::MAX_SIDE);
    assert_eq!(ingest::MAX_PIXELS, picture_gate::MAX_PIXELS);
    assert_eq!(
        ingest::DISPLAY_DECODE_MAX_ALLOC,
        picture_gate::DECODE_MAX_ALLOC
    );
    assert_eq!(ingest::MIN_SIDE, 16, "the window keeps its own floor");
    for f in [
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
    ] {
        // SH5 A-P2: the window's allowlist ⊆ the library's (here, equal).
        assert_eq!(
            ingest::allowed_format(f),
            picture_gate::allowed_format(f),
            "{f:?}"
        );
    }
}

#[test]
fn gate_refusals_reach_the_window_as_plain_refusals_never_crashed() {
    for (name, bytes) in [
        // SH5 B01 (valid VP8L 64×48), H01 (VP8X 65535² + ALPH; today's CLI abort), H05 (canvas
        // 64×64 around a VP8L 16384²).
        (
            "B01",
            hex("5249464618000000574542505650384c0c0000002f3fc00b002860810adcff00"),
        ),
        (
            "H01",
            hex("524946462400000057454250565038580a00000010000000feff00feff00414c50480500000001e87f440400"),
        ),
        (
            "H05",
            hex("524946462a00000057454250565038580a000000000000003f00003f00005650384c0c0000002fffffff0f2860810adcff00"),
        ),
    ] {
        let e = match run_load(LoadSource::Bytes {
            bytes: bytes.clone(),
            name: name.into(),
        }) {
            Ok(_) => panic!("{name} was admitted"),
            Err(e) => e,
        };
        assert_ne!(e.kind, ErrorKind::Crashed, "{name}");
        // Unchanged from S123: the window's probe calls this UnsupportedFormat("WebP").
        assert_eq!(
            ingest::probe(&bytes),
            Err(Refusal::UnsupportedFormat("WebP".into())),
            "{name}"
        );
    }
}

#[test]
fn the_window_floor_is_the_windows_and_runs_first() {
    // Floor before side, as in S123: 10×40000 is "too small" in the window.
    assert_eq!(
        ingest::check_dimensions(10, 40_000),
        Err(Refusal::TooSmall {
            width: 10,
            height: 40_000
        })
    );
}

/// ROUND 2 (operator A.2/A.4, L-69): the pictures the CLI now reads are still refused by the
/// window, which is unchanged: real TGA/QOI/PNM pictures by type, a 32769×16 PNG by its side cap.
#[test]
fn the_window_still_refuses_what_only_the_cli_reads() {
    fn encode(w: u32, h: u32, format: ImageFormat) -> Vec<u8> {
        let img = image::RgbImage::from_fn(w, h, |x, y| image::Rgb([x as u8, y as u8, 90]));
        let mut out = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(img)
            .write_to(&mut out, format)
            .expect("encode");
        out.into_inner()
    }
    for (format, name) in [
        (ImageFormat::Tga, "Tga"),
        (ImageFormat::Qoi, "Qoi"),
        (ImageFormat::Pnm, "Pnm"),
    ] {
        let bytes = encode(64, 48, format);
        // TGA has no signature, so the window (which sniffs only) cannot even name it.
        let want = if format == ImageFormat::Tga {
            Refusal::NotAnImage("the picture format could not be determined".into())
        } else {
            Refusal::UnsupportedFormat(name.into())
        };
        assert_eq!(ingest::probe(&bytes), Err(want), "{name}");
        let e = match run_load(LoadSource::Bytes {
            bytes,
            name: name.into(),
        }) {
            Ok(_) => panic!("{name} was admitted by the window"),
            Err(e) => e,
        };
        assert_ne!(e.kind, ErrorKind::Crashed, "{name}");
    }
    let wide = encode(picture_gate::MAX_SIDE + 1, 16, ImageFormat::Png);
    assert_eq!(
        ingest::probe(&wide),
        Err(Refusal::SideTooLarge {
            width: picture_gate::MAX_SIDE + 1,
            height: 16
        })
    );
    assert_eq!(
        ingest::WINDOW_POLICY,
        picture_gate::Policy {
            min_side: 16,
            ..picture_gate::Policy::WINDOW
        }
    );
    assert_eq!(
        picture_gate::Policy::WINDOW.max_side,
        Some(picture_gate::MAX_SIDE)
    );
    assert_eq!(
        picture_gate::Policy::WINDOW.max_pixels,
        Some(picture_gate::MAX_PIXELS)
    );
    assert_eq!(
        picture_gate::Policy::WINDOW.max_file_bytes,
        Some(picture_gate::MAX_IMAGE_BYTES)
    );
    assert_eq!(
        picture_gate::Policy::WINDOW.decode_max_alloc,
        Some(picture_gate::DECODE_MAX_ALLOC)
    );
}

//! S125 lane 05 (Test Engineer): rows the ITEM 1b forcing tests do not reach.
//!
//! ADOPTED into ITEM 1b ROUND 2 by lane 02 (lead ruling L-73), RE-AIMED to the per-front-end
//! policies of operator A.2/A.4 / L-69 (the CLI has no size or decode caps; the window keeps
//! S123's), then in ROUND 3 to operator A.5 / L-80 (the CLI is `image::open` as at `369938d`,
//! minus WebP: no size cap, the `image` crate's default decode limits). Each re-aimed test says
//! what changed. Lane 05's original is
//! `~/.cache/audiohax-s125-05/proposed/picture_gate_bounds_s125.rs` (`d3f78306…101c`).
//!
//! * The caps at their EXACT boundaries (a `>` that became `>=`, or a limit loosened by one, is
//!   invisible to tests that only probe far from the edge): 50,000,000 pixels admitted and
//!   50,000,001 refused; a read of exactly the byte cap admitted and one byte more refused, on
//!   both of the read's checks (the path's size and the capped read itself).
//! * The decode limits are LOAD-BEARING for GIF: `image 0.24.9`'s GIF decoder takes its
//!   dimensions from the logical screen, but allocates the FIRST FRAME at the frame's own size
//!   (`GifDecoder::read_image`, the "frame does not match the logical screen" arm), and only the
//!   explicit `Limits` stop it. A 16×16 screen holding a 65535×65535 frame passes every header cap
//!   (and the window's 16 px floor); without the gate's `decode_limits` it asks for 17 GiB.
//! * Byte-identity of the gated decode for every allowed format on BOTH arms (SH5 A.4
//!   "Byte-identity"): the gate's pixels equal what `image::open` / `image::load_from_memory`
//!   gave before S125 for PNG, JPEG, GIF, BMP and TIFF.
//! * The CLI's `play` path reaches the same gate (`prepare_play` with `ImageInput::Path`).
//!
//! Fixtures are built here from bytes; files only under `CARGO_TARGET_TMPDIR`.
#![cfg(all(feature = "pure-analysis", feature = "synth"))]

use std::io::Cursor;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use audiohax::cli::{PipelineArgs, Toggle};
use audiohax::mapping_loader::load_mappings;
use audiohax::picture_gate::{
    admit, check_dimensions, read_bounded_with_cap, read_capped, PictureRefusal, Policy,
    MAX_IMAGE_BYTES,
};
use audiohax::pure_analysis::{
    load_pure_image, load_pure_image_bytes, AnalysisError, PureImageSource,
};
use audiohax::render::{prepare_play, ImageInput, RenderError, RenderNote, RenderRequest};
use image::{ImageFormat, Rgb, RgbImage};

fn scratch(name: &str) -> PathBuf {
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("picture-gate-bounds-s125")
        .join(format!("{name}-{}", std::process::id()));
    std::fs::create_dir_all(&d).expect("scratch dir");
    d
}

fn encode(img: &RgbImage, format: ImageFormat) -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(img.clone())
        .write_to(&mut out, format)
        .expect("encode");
    out.into_inner()
}

/// A PNG whose IHDR CLAIMS `width × height` (valid CRC) with a 1×1 IDAT.
fn png_claiming(width: u32, height: u32) -> Vec<u8> {
    let mut v = encode(
        &RgbImage::from_pixel(1, 1, Rgb([0, 0, 0])),
        ImageFormat::Png,
    );
    v[16..20].copy_from_slice(&width.to_be_bytes());
    v[20..24].copy_from_slice(&height.to_be_bytes());
    let mut h = crc32fast::Hasher::new();
    h.update(&v[12..29]);
    v[29..33].copy_from_slice(&h.finalize().to_be_bytes());
    v
}

fn structured(w: u32, h: u32) -> RgbImage {
    RgbImage::from_fn(w, h, |x, y| {
        Rgb([
            (x * 255 / w) as u8,
            (y * 255 / h) as u8,
            if (x / 4 + y / 4) % 2 == 0 { 200 } else { 40 },
        ])
    })
}

/// A GIF89a whose logical screen is `sw × sh` (2-colour global table) and whose first frame
/// descriptor claims `fw × fh` at (0, 0), with a token LZW stream.
fn gif_screen_and_frame(sw: u16, sh: u16, fw: u16, fh: u16) -> Vec<u8> {
    let mut v = b"GIF89a".to_vec();
    v.extend_from_slice(&sw.to_le_bytes());
    v.extend_from_slice(&sh.to_le_bytes());
    v.extend_from_slice(&[0x80, 0, 0]); // global colour table, 2 entries; bg 0; aspect 0
    v.extend_from_slice(&[0, 0, 0, 255, 255, 255]);
    v.push(0x2C);
    v.extend_from_slice(&0u16.to_le_bytes());
    v.extend_from_slice(&0u16.to_le_bytes());
    v.extend_from_slice(&fw.to_le_bytes());
    v.extend_from_slice(&fh.to_le_bytes());
    v.push(0); // no local table, not interlaced
    v.extend_from_slice(&[2, 2, 0x44, 0x01, 0]); // LZW min code size 2; one 2-byte sub-block
    v.push(0x3B);
    v
}

// ── exact boundaries ────────────────────────────────────────────────────────────────────────

/// MAX_PIXELS is inclusive: exactly 50,000,000 pixels is admitted, 50,000,001 is refused.
/// (10000×5000 = 50,000,000; 2217×22553 = 50,000,001, both sides under MAX_SIDE.)
///
/// RE-AIMED (L-69): the cap is the WINDOW's now, so the boundary is pinned on `Policy::WINDOW`;
/// the CLI policy has no pixel cap at all (pinned last).
#[test]
fn the_pixel_cap_is_exact() {
    let w = Policy::WINDOW;
    let at = admit(&png_claiming(10_000, 5_000), None, &w).expect("exactly 50 MP is admitted");
    assert_eq!((at.width, at.height), (10_000, 5_000));
    assert_eq!(
        admit(&png_claiming(2_217, 22_553), None, &w),
        Err(PictureRefusal::TooManyPixels {
            width: 2_217,
            height: 22_553,
            pixels: 50_000_001,
            max_pixels: 50_000_000,
        })
    );
    assert!(check_dimensions(2_217, 22_553, &Policy::CLI).is_ok());
}

/// The read cap is inclusive on BOTH checks. The path-size check: a file of exactly `cap`
/// bytes is read, `cap + 1` is refused. The capped read alone (a source that reports no size):
/// `cap` bytes pass, `cap + 1` are refused.
#[test]
fn the_byte_cap_is_exact_on_both_read_checks() {
    let d = scratch("bytecap");
    let p = d.join("x.bin");
    std::fs::write(&p, vec![7u8; 1000]).expect("write");
    assert_eq!(read_bounded_with_cap(&p, 1000).map(|b| b.len()), Ok(1000));
    assert_eq!(
        read_bounded_with_cap(&p, 999),
        Err(PictureRefusal::FileTooBig { cap: 999 })
    );
    assert_eq!(
        read_capped(Cursor::new(vec![7u8; 1000]), 1000).map(|b| b.len()),
        Ok(1000)
    );
    assert_eq!(
        read_capped(Cursor::new(vec![7u8; 1001]), 1000),
        Err(PictureRefusal::FileTooBig { cap: 1000 })
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// The capped read bounds MEMORY, not only the verdict: from a source that never ends (a file
/// that grows after both size checks), it reads at most `cap + 1` bytes and refuses. The source
/// here panics if asked for more than `cap + 64` bytes, so a read without the `take` fails
/// instead of filling memory.
#[test]
fn the_capped_read_never_reads_past_cap_plus_one() {
    struct Endless {
        served: u64,
        limit: u64,
    }
    impl std::io::Read for Endless {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            assert!(
                self.served <= self.limit,
                "read {} bytes past the cap",
                self.served
            );
            let n = buf.len().min(16);
            buf[..n].fill(7);
            self.served += n as u64;
            Ok(n)
        }
    }
    let cap = 1000;
    assert_eq!(
        read_capped(
            Endless {
                served: 0,
                limit: cap + 64
            },
            cap
        ),
        Err(PictureRefusal::FileTooBig { cap })
    );
}

/// The production constant at its edge. RE-AIMED (L-69; ROUND 3): the byte cap is the WINDOW's
/// (`read_bounded_with_cap` at `MAX_IMAGE_BYTES`, the window's read): a sparse file of EXACTLY
/// 256 MiB is read, 256 MiB + 1 is refused. The CLI arm (`load_pure_image`, no byte cap) streams
/// it to the PNG decoder and fails with the PNG decoder's own words, as the original test asserted.
#[test]
fn a_file_of_exactly_the_byte_cap_is_read() {
    let d = scratch("bytecap-prod");
    let p = d.join("exact.png");
    std::fs::File::create(&p)
        .expect("create")
        .set_len(MAX_IMAGE_BYTES)
        .expect("sparse");
    assert_eq!(
        read_bounded_with_cap(&p, MAX_IMAGE_BYTES).map(|b| b.len() as u64),
        Ok(MAX_IMAGE_BYTES)
    );
    let over = d.join("over.png");
    std::fs::File::create(&over)
        .expect("create")
        .set_len(MAX_IMAGE_BYTES + 1)
        .expect("sparse");
    assert_eq!(
        read_bounded_with_cap(&over, MAX_IMAGE_BYTES),
        Err(PictureRefusal::FileTooBig {
            cap: MAX_IMAGE_BYTES
        })
    );
    match load_pure_image(&PureImageSource::UserPath(p.clone())) {
        Err(AnalysisError::Decode(s)) => assert_eq!(
            s,
            format!(
                "{}: Format error decoding Png: Invalid PNG signature.",
                p.display()
            )
        ),
        Err(other) => panic!("expected the PNG decoder's refusal, got {other:?}"),
        Ok(_) => panic!("a file of zeros decoded as a picture"),
    }
    let _ = std::fs::remove_dir_all(&d);
}

// ── GIF: the frame is not the screen ────────────────────────────────────────────────────────

/// A 16×16 GIF screen with a 65535×65535 first frame passes every header cap, and `image`
/// would allocate the frame (17 GiB).
///
/// RE-AIMED and RENAMED (L-73; was `…_is_stopped_by_the_decode_limits`), and in ROUND 3 back to
/// lane 05's original mechanism on the CLI (L-80): every front-end refuses it BY ITS DECODE
/// LIMITS. The CLI decodes with the `image` crate's default limits, as `image::open` did at
/// `369938d` (and refuses in `image::open`'s words); the window keeps S123's behaviour exactly
/// (its header step admits the 16×16 screen, and its decode limit refuses the frame, as
/// `tests/gui_sealA_s123.rs` pins). Remove either front-end's limits and its arm of this test
/// aborts the process.
#[test]
fn a_gif_frame_larger_than_its_screen_is_refused_on_every_front_end() {
    let bytes = gif_screen_and_frame(16, 16, 65_535, 65_535);
    let a = admit(&bytes, None, &Policy::WINDOW).expect("the window's header step: a 16×16 screen");
    assert_eq!((a.format, a.width, a.height), (ImageFormat::Gif, 16, 16));
    // The CLI arm, through a file: exactly `image::open`'s refusal.
    let d = scratch("gif-cli");
    let p = d.join("frame-bomb.gif");
    std::fs::write(&p, &bytes).expect("write");
    let before = image::open(&p).expect_err("image::open refused it before S125");
    let t0 = Instant::now();
    match load_pure_image(&PureImageSource::UserPath(p.clone())) {
        Err(AnalysisError::Decode(s)) => assert_eq!(s, format!("{}: {before}", p.display())),
        Err(other) => panic!("expected a Decode refusal, got {other:?}"),
        Ok(img) => panic!("admitted a {}x{} picture", img.width(), img.height()),
    }
    assert!(t0.elapsed() < Duration::from_millis(500));
    let _ = std::fs::remove_dir_all(&d);
    // The window's in-memory arm.
    let t0 = Instant::now();
    match load_pure_image_bytes(&bytes) {
        Err(AnalysisError::Decode(s)) => {
            assert!(s.starts_with("<in-memory image>: "), "{s}");
            assert!(!s.contains("WebP"), "{s}");
        }
        Err(other) => panic!("expected a Decode refusal, got {other:?}"),
        Ok(img) => panic!("admitted a {}x{} picture", img.width(), img.height()),
    }
    assert!(
        t0.elapsed() < Duration::from_millis(500),
        "took {:?}",
        t0.elapsed()
    );
}

/// The window path: the same GIF is not `Crashed`, and its display decode is refused.
#[cfg(feature = "gui")]
#[test]
fn a_gif_frame_larger_than_its_screen_is_a_plain_refusal_in_the_window() {
    use audiohax::gui::job::{run_load, LoadSource};
    use audiohax::gui::text::ErrorKind;
    let bytes = gif_screen_and_frame(16, 16, 65_535, 65_535);
    match run_load(LoadSource::Bytes {
        bytes,
        name: "frame-bomb.gif".into(),
    }) {
        Ok(_) => panic!("admitted"),
        Err(e) => assert_ne!(e.kind, ErrorKind::Crashed, "{e:?}"),
    }
}

// ── byte-identity of the gated decode, every allowed format, both arms ──────────────────────

/// SH5 A.4: for an admitted picture the decoder, the bytes and the pixels are unchanged. For each
/// allowed format, the gate's pixels equal `image::load_from_memory` (in-memory arm, as before
/// S125) and `image::open` (path arm, as before S125: format from the extension).
#[test]
fn the_gated_decode_is_pixel_identical_to_the_pre_s125_decode_for_every_allowed_format() {
    let d = scratch("identity");
    let img = structured(37, 23);
    for (format, ext) in [
        (ImageFormat::Png, "png"),
        (ImageFormat::Jpeg, "jpg"),
        (ImageFormat::Gif, "gif"),
        (ImageFormat::Bmp, "bmp"),
        (ImageFormat::Tiff, "tiff"),
    ] {
        let bytes = encode(&img, format);
        let before_mem = image::load_from_memory(&bytes).expect("decode").to_rgb8();
        let gated_mem = load_pure_image_bytes(&bytes).unwrap_or_else(|e| panic!("{ext}: {e:?}"));
        assert_eq!(gated_mem.as_rgb(), &before_mem, "{ext}: in-memory arm");

        let p = d.join(format!("x.{ext}"));
        std::fs::write(&p, &bytes).expect("write");
        let before_path = image::open(&p).expect("open").to_rgb8();
        let gated_path = load_pure_image(&PureImageSource::UserPath(p.clone()))
            .unwrap_or_else(|e| panic!("{ext}: {e:?}"));
        assert_eq!(gated_path.as_rgb(), &before_path, "{ext}: path arm");
    }
    // EXTENDED (L-69, as lane 05 proposed): the formats the CLI reads again, on the CLI's path
    // arm, against `image::open` (the window refuses them; pinned in `picture_gate_s125.rs`).
    for (format, ext) in [
        (ImageFormat::Tga, "tga"),
        (ImageFormat::Pnm, "ppm"),
        (ImageFormat::Qoi, "qoi"),
    ] {
        let bytes = encode(&img, format);
        let p = d.join(format!("x.{ext}"));
        std::fs::write(&p, &bytes).expect("write");
        let before_path = image::open(&p).expect("open").to_rgb8();
        let gated_path = load_pure_image(&PureImageSource::UserPath(p.clone()))
            .unwrap_or_else(|e| panic!("{ext}: {e:?}"));
        assert_eq!(gated_path.as_rgb(), &before_path, "{ext}: CLI path arm");
    }
    let _ = std::fs::remove_dir_all(&d);
}

// ── the CLI's `play` path ───────────────────────────────────────────────────────────────────

/// `audiohax play` reaches the picture through `prepare_play(ImageInput::Path)`; the gate refuses
/// there exactly as on `render` (a WebP, and a header pixel bomb), before any audio exists.
///
/// RE-AIMED (L-69; ROUND 3, L-80): the CLI has no pixel cap; the header-only 20000² RGB PNG (1.2 GB
/// of pixels) is refused by the decoder's default allocation limit, in `image::open`'s words, as
/// at `369938d`.
#[test]
fn prepare_play_on_a_path_is_gated() {
    let d = scratch("play");
    let m = load_mappings(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("assets/mappings.json")
            .to_str()
            .expect("utf-8"),
    )
    .expect("mappings");
    let req = RenderRequest {
        seed: Some(1),
        loop_cue: false,
        expression: Toggle::Off,
        v2: None,
        pipeline: PipelineArgs::default(),
        reverb: Toggle::On,
        gain: 1.0,
        event_dump: None,
    };
    let webp = {
        let mut v = b"RIFF".to_vec();
        v.extend_from_slice(&24u32.to_le_bytes());
        v.extend_from_slice(b"WEBPVP8L");
        v.extend_from_slice(&12u32.to_le_bytes());
        v.extend_from_slice(&[
            0x2f, 0x3f, 0xc0, 0x0b, 0x00, 0x28, 0x60, 0x81, 0x0a, 0xdc, 0xff, 0x00,
        ]);
        v
    };
    for (name, bytes, want) in [
        ("w.webp", webp, "WebP pictures are not supported yet"),
        (
            "bomb.png",
            png_claiming(20_000, 20_000),
            "Memory limit exceeded",
        ),
    ] {
        let p = d.join(name);
        std::fs::write(&p, &bytes).expect("write");
        let t0 = Instant::now();
        let r = prepare_play(
            &req,
            ImageInput::Path(&p),
            &m,
            &mut |_: RenderNote<'_>| -> Result<(), RenderError> { Ok(()) },
        );
        match r {
            Err(RenderError::Analysis(AnalysisError::Decode(s))) => {
                assert!(s.contains(want), "{name}: {s}")
            }
            Err(other) => panic!("{name}: expected a gate refusal, got {other:?}"),
            Ok(_) => panic!("{name}: admitted"),
        }
        assert!(t0.elapsed() < Duration::from_millis(50), "{name} slow");
    }
    let _ = std::fs::remove_dir_all(&d);
}

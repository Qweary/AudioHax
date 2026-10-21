//! S123 Test Engineer (Specialist 5): ingest bounds that the builder's unit tests in
//! `src/gui/ingest.rs` / `src/gui/job.rs` do not reach.
//!
//! The builder tests `probe` / `read_bounded` as functions. These tests drive the WINDOW'S load
//! path (`gui::job::run_load`, the function the worker runs for File ▸ Open, drag-and-drop and
//! "Try a sample") end to end, and they pin the exact cap boundaries (security review
//! `s123-SH` § 4.1 B-1/B-2, § 5 T3–T9, T12, T13).
//!
//! Filesystem fixtures are written under `CARGO_TARGET_TMPDIR` (inside the cargo target dir),
//! never `/tmp` and never inside the repository. No large file is materialised: the 256 MiB
//! case is a sparse file.
#![cfg(feature = "gui")]

use audiohax::gui::ingest::{
    check_dimensions, probe, Refusal, MAX_IMAGE_BYTES, MAX_PIXELS, MAX_SIDE, MIN_SIDE,
};
use audiohax::gui::job::{run_load, LoadSource};
use audiohax::gui::text::ErrorKind;
use image::{ImageBuffer, ImageFormat, Rgb};
use std::io::Cursor;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

// ── fixtures ────────────────────────────────────────────────────────────────────────────────

fn scratch(name: &str) -> PathBuf {
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("gui-ingest-bounds-s123")
        .join(format!("{name}-{}", std::process::id()));
    std::fs::create_dir_all(&d).expect("scratch dir");
    d
}

fn encode(width: u32, height: u32, format: ImageFormat) -> Vec<u8> {
    let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_fn(width, height, |x, y| {
        Rgb([
            (x * 7 % 256) as u8,
            (y * 11 % 256) as u8,
            ((x + y) % 2 * 200) as u8,
        ])
    });
    let mut out = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(img)
        .write_to(&mut out, format)
        .expect("encode fixture");
    out.into_inner()
}

/// A tiny PNG whose IHDR CLAIMS `width x height` (valid CRC), with a 1×1 IDAT. Any decoder
/// that trusted the header would allocate for the claimed size; the data cannot decode.
fn png_claiming(width: u32, height: u32) -> Vec<u8> {
    let mut v = encode(1, 1, ImageFormat::Png);
    v[16..20].copy_from_slice(&width.to_be_bytes());
    v[20..24].copy_from_slice(&height.to_be_bytes());
    let mut h = crc32fast::Hasher::new();
    h.update(&v[12..29]);
    v[29..33].copy_from_slice(&h.finalize().to_be_bytes());
    v
}

/// Run `f` on a helper thread and fail if it has not returned within `limit`. A blocked open
/// (a FIFO with no writer) would otherwise hang the suite; a leaked blocked thread does not stop
/// the test process from exiting.
fn within<T: Send + 'static>(limit: Duration, f: impl FnOnce() -> T + Send + 'static) -> T {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(f());
    });
    rx.recv_timeout(limit)
        .unwrap_or_else(|_| panic!("did not return within {limit:?} (blocked or unbounded read)"))
}

fn load_err(path: PathBuf) -> audiohax::gui::text::UserError {
    match run_load(LoadSource::Path(path.clone())) {
        Ok(_) => panic!("{} was admitted; it must be refused", path.display()),
        Err(e) => e,
    }
}

// ── B-2: the exact cap boundaries ──────────────────────────────────────────────────────────

/// Pins all three B-2 bounds at their exact edges, both orientations. The new cells are the
/// pixel cap's own edge: 50,000,000 px is ADMITTED (the cap is "more than 50 M is refused") and
/// 50,000,001 px (2217 × 22553, both sides inside the side bounds) is REFUSED with the product
/// reported in full. The builder's T8 pair (49,939,200 / 50,013,184) leaves a 74,000-pixel gap
/// in which `>` vs `>=` and an off-by-one cap are both invisible.
#[test]
fn dimension_caps_hold_at_their_exact_edges() {
    assert_eq!((MIN_SIDE, MAX_SIDE, MAX_PIXELS), (16, 32_768, 50_000_000));
    // The pixel cap, exactly.
    for (w, h) in [(10_000u32, 5_000u32), (5_000, 10_000)] {
        assert_eq!(u64::from(w) * u64::from(h), 50_000_000);
        assert_eq!(check_dimensions(w, h), Ok(()), "{w}x{h} is exactly the cap");
        let p = probe(&png_claiming(w, h)).expect("exactly 50 MP is admitted by the probe");
        assert_eq!((p.width, p.height), (w, h));
    }
    for (w, h) in [(2_217u32, 22_553u32), (22_553, 2_217)] {
        assert_eq!(u64::from(w) * u64::from(h), 50_000_001);
        assert_eq!(
            check_dimensions(w, h),
            Err(Refusal::TooManyPixels {
                width: w,
                height: h,
                pixels: 50_000_001
            })
        );
        assert!(matches!(
            probe(&png_claiming(w, h)),
            Err(Refusal::TooManyPixels { .. })
        ));
    }
    // The side floor and ceiling, each against the OTHER bound's extreme.
    assert!(matches!(
        check_dimensions(15, 32_768),
        Err(Refusal::TooSmall { .. })
    ));
    assert!(matches!(
        check_dimensions(32_768, 15),
        Err(Refusal::TooSmall { .. })
    ));
    assert_eq!(check_dimensions(16, 32_768), Ok(()));
    assert_eq!(check_dimensions(32_768, 16), Ok(()));
    assert!(matches!(
        check_dimensions(16, 32_769),
        Err(Refusal::SideTooLarge { .. })
    ));
    assert!(matches!(
        check_dimensions(32_769, 16),
        Err(Refusal::SideTooLarge { .. })
    ));
}

/// Header-probe-before-decode, through the window's load path: a 143-byte file whose header
/// claims 50,013,184 px (or 100,000²) is refused AS a size refusal. If any decode ran first,
/// the bogus pixel data would fail as "not a picture" (or try to allocate gigabytes) instead.
#[test]
fn header_bombs_are_refused_by_size_at_load_before_any_decode() {
    for (w, h, want) in [
        (7_072u32, 7_072u32, ErrorKind::TooManyPixels),
        (100_000, 100_000, ErrorKind::SideTooLarge),
        (2_217, 22_553, ErrorKind::TooManyPixels),
    ] {
        let t0 = Instant::now();
        let r = run_load(LoadSource::Bytes {
            bytes: png_claiming(w, h),
            name: "bomb.png".into(),
        });
        let took = t0.elapsed();
        match r {
            Err(e) => assert_eq!(e.kind, want, "{w}x{h}: {e:?}"),
            Ok(_) => panic!("{w}x{h} header bomb was admitted"),
        }
        assert!(took < Duration::from_millis(500), "{w}x{h} took {took:?}");
    }
}

// ── the load path on hostile or broken files ───────────────────────────────────────────────

/// T4 through the load path: a 0-byte file named `.png` is "not a picture", and nothing panics.
#[test]
fn zero_byte_file_is_not_a_picture() {
    let d = scratch("zero");
    let p = d.join("empty.png");
    std::fs::write(&p, b"").expect("write");
    let e = load_err(p);
    assert_eq!(e.kind, ErrorKind::NotAPicture, "{e:?}");
    let _ = std::fs::remove_dir_all(&d);
}

/// T5 through the load path: text renamed `.png`, and a real PNG cut inside its IHDR.
#[test]
fn non_image_and_header_truncated_png_are_not_pictures() {
    let d = scratch("text");
    let text = d.join("notes.png");
    std::fs::write(&text, b"this is a shopping list, not a picture\n").expect("write");
    assert_eq!(load_err(text).kind, ErrorKind::NotAPicture);
    let cut = d.join("cut-in-header.png");
    std::fs::write(&cut, &encode(64, 64, ImageFormat::Png)[..20]).expect("write");
    assert_eq!(load_err(cut).kind, ErrorKind::NotAPicture);
    let _ = std::fs::remove_dir_all(&d);
}

/// T6 through the load path: a PNG truncated in its pixel data passes the header probe, but the
/// window refuses it AT LOAD (the display decode fails), as "not a picture". It never becomes a
/// loaded picture the user could press Render on.
#[test]
fn data_truncated_png_is_refused_at_load() {
    let d = scratch("trunc");
    let p = d.join("half.png");
    let full = encode(64, 64, ImageFormat::Png);
    std::fs::write(&p, &full[..full.len() / 2]).expect("write");
    let e = load_err(p);
    assert_eq!(e.kind, ErrorKind::NotAPicture, "{e:?}");
    let _ = std::fs::remove_dir_all(&d);
}

/// T12 through the load path: a sparse file one byte over the REAL 256 MiB cap is refused as
/// "can't open that file" with the byte cap in the details, i.e. by B-1's bounded read and not
/// by a later stage that read it whole.
#[test]
fn sparse_file_over_the_real_byte_cap_is_refused_by_the_read() {
    let d = scratch("sparse");
    let p = d.join("big.png");
    let f = std::fs::File::create(&p).expect("create");
    f.set_len(MAX_IMAGE_BYTES + 1).expect("set_len (sparse)");
    drop(f);
    let e = within(Duration::from_secs(30), move || load_err(p));
    assert_eq!(e.kind, ErrorKind::CannotRead, "{e:?}");
    assert!(e.details.contains("268435456"), "{e:?}");
    let _ = std::fs::remove_dir_all(&d);
}

/// The in-memory route (`LoadSource::Bytes`) has its own copy of the byte cap: a buffer one
/// byte over is refused before the probe sees it.
#[test]
fn in_memory_bytes_over_the_cap_are_refused_before_the_probe() {
    let bytes = vec![0u8; (MAX_IMAGE_BYTES + 1) as usize]; // calloc'd; pages are never touched
    let e = match run_load(LoadSource::Bytes {
        bytes,
        name: "big".into(),
    }) {
        Err(e) => e,
        Ok(_) => panic!("admitted"),
    };
    assert_eq!(e.kind, ErrorKind::CannotRead, "{e:?}");
    assert!(e.details.contains("larger than"), "{e:?}");
}

/// T13 through the load path (unix): `/dev/zero`, a directory, a FIFO with no writer, and a
/// symlink to `/dev/zero` are each refused PROMPTLY as "not a regular file". A narrower check
/// (refusing only directories, say) would read 256 MiB of zeros from `/dev/zero` and block
/// forever on the FIFO.
#[cfg(unix)]
#[test]
fn non_regular_files_are_refused_promptly_through_the_load_path() {
    let d = scratch("nonreg");
    let fifo = d.join("pipe.png");
    let made_fifo = std::process::Command::new("mkfifo")
        .arg(&fifo)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    let link = d.join("zero-link.png");
    std::os::unix::fs::symlink("/dev/zero", &link).expect("symlink");
    let mut cases = vec![PathBuf::from("/dev/zero"), d.clone(), link];
    if made_fifo {
        cases.push(fifo);
    } else {
        eprintln!("mkfifo unavailable: FIFO case skipped");
    }
    for p in cases {
        let shown = p.display().to_string();
        let e = within(Duration::from_secs(2), move || load_err(p));
        assert_eq!(e.kind, ErrorKind::CannotRead, "{shown}: {e:?}");
        assert!(e.details.contains("not a regular file"), "{shown}: {e:?}");
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// T11's one missing format: Radiance HDR (named by the security review's B-3, absent from the
/// builder's list) is kept away from the decoder — refused as unsupported, or not recognised at
/// all.
#[test]
fn radiance_hdr_is_not_admitted() {
    // `DynamicImage::write_to` cannot encode HDR in image 0.24; the codec's encoder can.
    let pixels = vec![image::Rgb([0.25f32, 0.5, 0.75]); 32 * 32];
    let mut bytes = Vec::new();
    image::codecs::hdr::HdrEncoder::new(&mut bytes)
        .encode(&pixels, 32, 32)
        .expect("encode HDR");
    assert!(
        bytes.starts_with(b"#?RADIANCE"),
        "fixture is a Radiance file"
    );
    let r = probe(&bytes);
    assert!(
        matches!(
            r,
            Err(Refusal::UnsupportedFormat(_)) | Err(Refusal::NotAnImage(_))
        ),
        "HDR gave {r:?}"
    );
    let e = match run_load(LoadSource::Bytes {
        bytes,
        name: "scene.hdr".into(),
    }) {
        Err(e) => e,
        Ok(_) => panic!("HDR was admitted"),
    };
    assert!(
        matches!(e.kind, ErrorKind::UnsupportedType | ErrorKind::NotAPicture),
        "{e:?}"
    );
}

//! S123 seal, Key A (Quality Gate): the header probe answers BEFORE any full decode, for EVERY
//! format on the window's allowlist, not only PNG.
//!
//! `tests/gui_ingest_bounds_s123.rs` proves the header-bomb refusal for PNG only. This file
//! extends it to JPEG, GIF, BMP and TIFF. Each fixture is a small real picture whose header is
//! patched to claim 20000 x 20000 (400 MP, every side under the side cap), so the only correct
//! answer is the PIXEL-CAP refusal, which `probe` can give only from the header. A full decode
//! would either allocate for 400 MP or fail as a damaged picture, and both are caught here:
//! the refusal kind must be the size refusal, and a counting global allocator (this test binary
//! only) records the largest single allocation the window's load path makes.
//!
//! Fixtures are built in memory; nothing is written to disk.
#![cfg(feature = "gui")]

use audiohax::gui::ingest::{probe, Refusal};
use audiohax::gui::job::{run_load, LoadSource};
use audiohax::gui::text::ErrorKind;
use image::{ImageBuffer, ImageFormat, Rgb};
use std::alloc::{GlobalAlloc, Layout, System};
use std::io::Cursor;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Any single allocation larger than this is refused (null ⇒ abort), so a regression fails fast.
const REFUSE_ABOVE: usize = 1 << 30;

struct Counting;
static LARGEST: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        LARGEST.fetch_max(layout.size(), Ordering::SeqCst);
        if layout.size() > REFUSE_ABOVE {
            return std::ptr::null_mut();
        }
        // SAFETY: forwarded unchanged to the system allocator.
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: `ptr` came from `System.alloc` with this layout.
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        LARGEST.fetch_max(new_size, Ordering::SeqCst);
        if new_size > REFUSE_ABOVE {
            return std::ptr::null_mut();
        }
        // SAFETY: forwarded unchanged to the system allocator.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

static SERIAL: Mutex<()> = Mutex::new(());

const CLAIM: u32 = 20_000;

fn encode(format: ImageFormat) -> Vec<u8> {
    let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_fn(32, 32, |x, y| {
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

fn png_claiming(w: u32, h: u32) -> Vec<u8> {
    let mut v = encode(ImageFormat::Png);
    v[16..20].copy_from_slice(&w.to_be_bytes());
    v[20..24].copy_from_slice(&h.to_be_bytes());
    let mut c = crc32fast::Hasher::new();
    c.update(&v[12..29]);
    v[29..33].copy_from_slice(&c.finalize().to_be_bytes());
    v
}

/// Patch the first SOFn frame header: `FF Cn len(2) P(1) Y(2) X(2)`, big-endian.
fn jpeg_claiming(w: u16, h: u16) -> Vec<u8> {
    let mut v = encode(ImageFormat::Jpeg);
    let at = (0..v.len() - 1)
        .find(|&i| v[i] == 0xFF && matches!(v[i + 1], 0xC0..=0xC2))
        .expect("a SOF marker");
    v[at + 5..at + 7].copy_from_slice(&h.to_be_bytes());
    v[at + 7..at + 9].copy_from_slice(&w.to_be_bytes());
    v
}

/// Patch the GIF logical screen: width/height, u16 little-endian, at bytes 6..10.
fn gif_claiming(w: u16, h: u16) -> Vec<u8> {
    let mut v = encode(ImageFormat::Gif);
    assert_eq!(&v[..3], b"GIF");
    v[6..8].copy_from_slice(&w.to_le_bytes());
    v[8..10].copy_from_slice(&h.to_le_bytes());
    v
}

/// Patch BITMAPINFOHEADER width/height (i32 little-endian at bytes 18 and 22).
fn bmp_claiming(w: i32, h: i32) -> Vec<u8> {
    let mut v = encode(ImageFormat::Bmp);
    assert_eq!(&v[..2], b"BM");
    v[18..22].copy_from_slice(&w.to_le_bytes());
    v[22..26].copy_from_slice(&h.to_le_bytes());
    v
}

/// Patch the first IFD's ImageWidth (256), ImageLength (257) and RowsPerStrip (278), so the
/// strip count stays consistent and the header itself is well-formed.
fn tiff_claiming(w: u32, h: u32) -> Vec<u8> {
    let mut v = encode(ImageFormat::Tiff);
    assert_eq!(&v[..4], b"II*\0", "the encoder writes little-endian TIFF");
    let rd16 = |v: &[u8], o: usize| u16::from_le_bytes([v[o], v[o + 1]]);
    let ifd = u32::from_le_bytes([v[4], v[5], v[6], v[7]]) as usize;
    let n = rd16(&v, ifd) as usize;
    let mut patched = 0;
    for i in 0..n {
        let e = ifd + 2 + 12 * i;
        let tag = rd16(&v, e);
        let ty = rd16(&v, e + 2);
        let val = match tag {
            256 => w,
            257 | 278 => h,
            _ => continue,
        };
        match ty {
            3 => v[e + 8..e + 10].copy_from_slice(&(val as u16).to_le_bytes()),
            4 => v[e + 8..e + 12].copy_from_slice(&val.to_le_bytes()),
            other => panic!("tag {tag} has type {other}"),
        }
        if tag != 278 {
            patched += 1;
        }
    }
    assert_eq!(patched, 2, "both dimension tags patched");
    v
}

fn refused_from_the_header(bytes: Vec<u8>, what: &str) {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    // The probe alone, on the raw bytes: the pixel-cap refusal with the CLAIMED size.
    assert_eq!(
        probe(&bytes),
        Err(Refusal::TooManyPixels {
            width: CLAIM,
            height: CLAIM,
            pixels: u64::from(CLAIM) * u64::from(CLAIM),
        }),
        "{what}: probe must answer from the header"
    );
    // The window's load path, with the allocation watched.
    LARGEST.store(0, Ordering::SeqCst);
    let t0 = Instant::now();
    let r = run_load(LoadSource::Bytes {
        bytes,
        name: format!("bomb.{what}"),
    });
    let took = t0.elapsed();
    let largest = LARGEST.load(Ordering::SeqCst);
    eprintln!("{what}: refused in {took:?}; largest single allocation {largest} B");
    let e = r.expect_err("a 400 MP header must not load");
    assert_eq!(e.kind, ErrorKind::TooManyPixels, "{what}: {e:?}");
    assert!(took < Duration::from_millis(500), "{what}: took {took:?}");
    assert!(
        largest < 8 << 20,
        "{what}: largest single allocation {largest} B; a decoder ran"
    );
}

#[test]
fn png_header_bomb_is_refused_from_the_header() {
    refused_from_the_header(png_claiming(CLAIM, CLAIM), "png");
}

#[test]
fn jpeg_header_bomb_is_refused_from_the_header() {
    refused_from_the_header(jpeg_claiming(CLAIM as u16, CLAIM as u16), "jpeg");
}

#[test]
fn gif_header_bomb_is_refused_from_the_header() {
    refused_from_the_header(gif_claiming(CLAIM as u16, CLAIM as u16), "gif");
}

#[test]
fn bmp_header_bomb_is_refused_from_the_header() {
    refused_from_the_header(bmp_claiming(CLAIM as i32, CLAIM as i32), "bmp");
}

#[test]
fn tiff_header_bomb_is_refused_from_the_header() {
    refused_from_the_header(tiff_claiming(CLAIM, CLAIM), "tiff");
}

/// Control: the same five fixtures UNPATCHED load, so a refusal above is about the claimed size
/// and not about a fixture the decoders cannot read at all.
#[test]
fn control_the_unpatched_fixtures_load() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    for f in [
        ImageFormat::Png,
        ImageFormat::Jpeg,
        ImageFormat::Gif,
        ImageFormat::Bmp,
        ImageFormat::Tiff,
    ] {
        let r = run_load(LoadSource::Bytes {
            bytes: encode(f),
            name: "ok".into(),
        });
        let loaded = r.unwrap_or_else(|e| panic!("{f:?} fixture must load: {e:?}"));
        assert_eq!(loaded.image.probe().format, f);
        assert_eq!((loaded.display.width, loaded.display.height), (32, 32));
    }
}

/// T-3 (security review `s123-SH`; lead ruling L-3): at most one job alive, abandoned ones
/// included. `controller_delivers_only_the_wanted_result_after_an_abandon` (src/gui/job.rs) only
/// counts the results that arrive, which stays 1 even if the second job is started beside the
/// first, because the abandoned job's result is dropped either way. Here the Controller's own
/// state is read: it changes only inside `poll()`, so with no poll between the submissions the
/// first job is still the running one, and the second MUST be waiting in the single pending slot.
#[test]
fn t3_a_job_submitted_while_an_abandoned_one_runs_waits_in_the_one_pending_slot() {
    use audiohax::gui::job::{Controller, Event, GuardedImage, JobKind};
    use audiohax::gui::state::Controls;
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let mappings = audiohax::gui::embedded_mappings().expect("embedded mappings");
    let mut c = Controller::new(mappings, || {});
    let img = GuardedImage::admit(encode(ImageFormat::Png)).expect("admit");
    let req = Controls::default().request();
    let render = |image: &GuardedImage| JobKind::Render {
        request: req.clone(),
        image: image.clone(),
        save_to: None,
    };
    c.submit(render(&img));
    assert!(c.is_busy());
    c.abandon();
    c.submit(render(&img));
    assert!(
        c.waiting_on_abandoned(),
        "T-3: a job submitted while an abandoned one runs must WAIT, not start beside it"
    );
    c.submit(render(&img)); // replaces the pending one
    assert!(c.waiting_on_abandoned());
    let t0 = Instant::now();
    let mut rendered = 0;
    loop {
        for e in c.poll() {
            match e {
                Event::Rendered { .. } => rendered += 1,
                Event::Failed(u) => panic!("job failed: {u:?}"),
                _ => {}
            }
        }
        if !c.is_busy() {
            break;
        }
        assert!(
            t0.elapsed() < Duration::from_secs(300),
            "jobs did not finish"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(rendered, 1, "only the last submission's result arrives");
}

/// GIF is the one admitted format with TWO sets of dimensions: the logical screen (what `probe`
/// reads and caps) and each frame's own descriptor (what the decoder sizes its frame buffer
/// from). A 32 x 32 screen with a 60000 x 60000 first frame passes the probe. This checks that
/// neither the window's display decode (`run_load`) nor the library's analysis decode
/// (`run_render`, `image::load_from_memory` with the crate's default limits) acts on the frame's
/// claim: both must fail cleanly with no large allocation, and nothing may abort.
fn gif_frame_claiming(w: u16, h: u16) -> Vec<u8> {
    let mut v = encode(ImageFormat::Gif);
    let mut i = 13usize; // header (6) + logical screen descriptor (7)
    if v[10] & 0x80 != 0 {
        i += 3 * (1usize << ((v[10] & 0x07) + 1)); // global colour table
    }
    loop {
        match v[i] {
            0x21 => {
                i += 2; // extension introducer + label
                while v[i] != 0 {
                    i += 1 + v[i] as usize; // data sub-blocks
                }
                i += 1;
            }
            0x2C => break,
            other => panic!("unexpected GIF block 0x{other:02x} at {i}"),
        }
    }
    // Image descriptor: 2C, left(2), top(2), width(2), height(2), flags(1); little-endian.
    v[i + 5..i + 7].copy_from_slice(&w.to_le_bytes());
    v[i + 7..i + 9].copy_from_slice(&h.to_le_bytes());
    v
}

#[test]
fn gif_frame_larger_than_its_screen_is_refused_with_no_large_allocation() {
    use audiohax::gui::job::{run_render, GuardedImage};
    use audiohax::gui::state::Controls;
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let bytes = gif_frame_claiming(60_000, 60_000);
    let p = probe(&bytes).expect("the logical screen is 32 x 32, so the probe admits it");
    assert_eq!((p.width, p.height), (32, 32));

    LARGEST.store(0, Ordering::SeqCst);
    let r = run_load(LoadSource::Bytes {
        bytes: bytes.clone(),
        name: "frame-bomb.gif".into(),
    });
    let largest_load = LARGEST.load(Ordering::SeqCst);
    eprintln!(
        "gif frame bomb, run_load: {:?}; largest single allocation {largest_load} B",
        r.as_ref().map(|_| ()).map_err(|e| e.kind)
    );
    assert!(r.is_err(), "a 60000² frame must not load as a picture");
    assert!(
        largest_load < 64 << 20,
        "run_load allocated {largest_load} B"
    );

    let image = GuardedImage::admit(bytes).expect("admit");
    let mappings = audiohax::gui::embedded_mappings().expect("embedded mappings");
    LARGEST.store(0, Ordering::SeqCst);
    let r = run_render(&Controls::default().request(), &image, &mappings);
    let largest_render = LARGEST.load(Ordering::SeqCst);
    eprintln!(
        "gif frame bomb, run_render: {:?}; largest single allocation {largest_render} B",
        r.as_ref().map(|_| ()).map_err(|e| e.kind)
    );
    assert!(r.is_err(), "the library must refuse the 60000² frame");
    assert!(
        largest_render < 64 << 20,
        "run_render allocated {largest_render} B"
    );
}

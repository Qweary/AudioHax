//! S123 fix round, lead ruling L-16 (`s123-SH2` H-1 / N-1): the two crafted WebP files are
//! refused by the window's load path with NO large allocation.
//!
//! The unit tests in `src/gui/ingest.rs` check the refusal and its speed. Speed cannot show the
//! other half of the reviewer's condition, "no large allocation", so this file installs a
//! counting global allocator (its own test binary, so nothing else is affected) and records the
//! largest single request made while the window's load path handles each file.
//!
//! The allocator also REFUSES any single request over 1 GiB. A refused allocation aborts the
//! process, which is a test failure, but a safe one: if WebP were ever re-admitted unguarded, the
//! 52-byte file's 17 GiB request fails here at once instead of driving the machine into swap.
#![cfg(feature = "gui")]

use audiohax::gui::ingest::{probe, Refusal};
use audiohax::gui::job::{run_load, LoadSource};
use audiohax::gui::text::ErrorKind;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

/// Any single allocation larger than this is refused (returns null ⇒ the process aborts).
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

/// The two tests share the counter, so they run one at a time.
static SERIAL: Mutex<()> = Mutex::new(());

/// `s123-SH2` H-1's 38-byte lossless (`VP8L`) WebP declaring 16383 x 16383 (round 1: 1.86 s and
/// about 2.0 GiB inside `probe`). The reviewer's bytes, copied; nothing is committed as a file.
const WEBP_VP8L_16383: [u8; 38] = [
    0x52, 0x49, 0x46, 0x46, 0x1e, 0x00, 0x00, 0x00, 0x57, 0x45, 0x42, 0x50, 0x56, 0x50, 0x38, 0x4c,
    0x11, 0x00, 0x00, 0x00, 0x2f, 0xfe, 0xbf, 0xff, 0x0f, 0x07, 0x50, 0xad, 0xa2, 0x14, 0xb4, 0xff,
    0x81, 0x88, 0xe8, 0x7f, 0x00, 0x00,
];

/// `s123-SH2` H-1's 52-byte extended (`VP8X` + `ALPH`) WebP declaring a 65535 x 65535 canvas
/// (round 1: the process aborted on a 17 GiB allocation).
const WEBP_VP8X_ALPH_65535: [u8; 52] = [
    0x52, 0x49, 0x46, 0x46, 0x2c, 0x00, 0x00, 0x00, 0x57, 0x45, 0x42, 0x50, 0x56, 0x50, 0x38, 0x58,
    0x0a, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0xfe, 0xff, 0x00, 0xfe, 0xff, 0x00, 0x41, 0x4c,
    0x50, 0x48, 0x0d, 0x00, 0x00, 0x00, 0x01, 0x07, 0x50, 0xad, 0xa2, 0x14, 0xb4, 0xff, 0x81, 0x88,
    0xe8, 0x7f, 0x00, 0x00,
];

/// The largest single allocation made by the window's load path for `bytes`, in bytes, after
/// checking it was refused as an unsupported type within 50 ms.
fn load_and_measure(bytes: &[u8], what: &str) -> usize {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let input = bytes.to_vec();
    LARGEST.store(0, Ordering::SeqCst);
    let t0 = Instant::now();
    let r = run_load(LoadSource::Bytes {
        bytes: input,
        name: "crafted.webp".into(),
    });
    let took = t0.elapsed();
    let largest = LARGEST.load(Ordering::SeqCst);
    let e = r.expect_err("a WebP must not load");
    assert_eq!(e.kind, ErrorKind::UnsupportedType, "{what}: {e:?}");
    assert!(took.as_millis() < 50, "{what}: took {took:?}");
    assert_eq!(
        probe(bytes),
        Err(Refusal::UnsupportedFormat("WebP".into())),
        "{what}"
    );
    largest
}

#[test]
fn the_38_byte_vp8l_webp_is_refused_with_no_large_allocation() {
    let largest = load_and_measure(&WEBP_VP8L_16383, "VP8L 16383²");
    assert!(
        largest < 1 << 20,
        "largest single allocation {largest} B; the decoder was reached"
    );
}

#[test]
fn the_52_byte_vp8x_alpha_webp_is_refused_with_no_large_allocation() {
    let largest = load_and_measure(&WEBP_VP8X_ALPH_65535, "VP8X+ALPH 65535²");
    assert!(
        largest < 1 << 20,
        "largest single allocation {largest} B; the decoder was reached"
    );
}

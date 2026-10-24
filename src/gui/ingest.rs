//! The picture's way into the window (the security review `s123-SH` § 4.1 and § 4.9).
//!
//! Every byte the window hands to the library passes through here first, in this order:
//!
//! 1. [`read_bounded`] — regular files only, at most [`MAX_IMAGE_BYTES`] read (B-1).
//! 2. [`probe`] — header-only format sniff and dimensions, BEFORE any decode: the format
//!    allowlist (B-3) and the three dimension caps (B-2).
//! 3. The SAME byte buffer then goes to the library as `ImageInput::Bytes` (never the
//!    path variant, whose decoder is chosen by file extension), and to
//!    [`decode_for_display`] for the on-screen picture (D-1).
//!
//! All of this runs on the worker thread (see [`super::job`]); none of it runs on the UI thread.
//!
//! S125 (D-45, `s125-SH5` A-P2, lead ruling L-61): the read, the format and size checks and the
//! limited decode are no longer duplicated here. They are the library's ONE picture gate,
//! [`crate::picture_gate`], which the CLI goes through too, under its own policy. The window passes
//! [`WINDOW_POLICY`] (its formats and limits, unchanged since S123, plus its own 16 px floor),
//! re-exports the gate's numbers, and maps the gate's refusals onto [`Refusal`] so the window's
//! wording is unchanged. The library's in-memory decode, which the window uses, applies the same
//! window limits.

use crate::picture_gate::{self as gate, Admitted, PictureRefusal, Policy};
use image::ImageFormat;
use std::io::Read;
use std::path::Path;

/// B-1: the most bytes read from a picture file into memory (256 MiB). It bounds the read
/// before the format is known (a video renamed `.jpg`); it is NOT the decompression-bomb
/// defence, which is [`MAX_PIXELS`]. B-2: the largest side admitted (stops absurd headers
/// early; admits ordinary panoramas). B-2: the largest pixel count admitted (50 MP; about
/// 1.25 GiB and 75 s of analysis on the measured laptop, SH § 3.2). All three are the library
/// gate's, so the window and the CLI share one set of numbers (SH5 A-P2).
pub use crate::picture_gate::{MAX_IMAGE_BYTES, MAX_PIXELS, MAX_SIDE};

/// B-2: the smallest side admitted. Both known analyzer panic classes are at sides of 1–2 px
/// (SH HL-1), and 16 makes every Voices value the window offers (1..=16) renderable. This floor
/// is the WINDOW's product bound (SH5 B-4); the library gate has none (lead ruling L-60).
pub const MIN_SIDE: u32 = 16;

/// D-1: the long side of the ON-SCREEN copy of the picture. Display only; the engine always gets
/// the full original bytes.
pub const DISPLAY_MAX_SIDE: u32 = 2048;

/// B-2: the allocation limit for any decode the window performs itself (512 MiB; the library
/// gate's [`gate::DECODE_MAX_ALLOC`]).
pub const DISPLAY_DECODE_MAX_ALLOC: u64 = gate::DECODE_MAX_ALLOC;

/// The window's policy for the one library gate: [`Policy::WINDOW`] (its five formats and its
/// upper limits, unchanged since S123) plus its own [`MIN_SIDE`] floor. The CLI has its own
/// policy ([`Policy::CLI`], operator A.2/A.4); the window's does not follow it.
pub const WINDOW_POLICY: Policy = Policy {
    min_side: MIN_SIDE,
    ..Policy::WINDOW
};

/// Why a picture was refused before the library saw it.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Refusal {
    /// Not a regular file (a directory, FIFO, device such as `/dev/zero`).
    #[error("not a regular file")]
    NotARegularFile,
    /// More than [`MAX_IMAGE_BYTES`] bytes.
    #[error("file is larger than {cap} bytes")]
    FileTooBig {
        /// The cap that was exceeded.
        cap: u64,
    },
    /// The file could not be opened or read.
    #[error("could not read the file: {0}")]
    Io(String),
    /// The bytes are not a picture format the `image` crate recognises, or the header is bad.
    #[error("not a readable picture: {0}")]
    NotAnImage(String),
    /// A real picture format, but not on the allowlist (B-3).
    #[error("unsupported picture type: {0}")]
    UnsupportedFormat(String),
    /// A side is under [`MIN_SIDE`].
    #[error("picture is {width}x{height}; the smallest side must be at least {MIN_SIDE} px")]
    TooSmall {
        /// Width in pixels.
        width: u32,
        /// Height in pixels.
        height: u32,
    },
    /// A side is over [`MAX_SIDE`].
    #[error("picture is {width}x{height}; no side may exceed {MAX_SIDE} px")]
    SideTooLarge {
        /// Width in pixels.
        width: u32,
        /// Height in pixels.
        height: u32,
    },
    /// More than [`MAX_PIXELS`] pixels.
    #[error("picture is {width}x{height} ({pixels} pixels); the limit is {MAX_PIXELS}")]
    TooManyPixels {
        /// Width in pixels.
        width: u32,
        /// Height in pixels.
        height: u32,
        /// `width * height`, computed in `u64`.
        pixels: u64,
    },
    /// The on-screen decode failed (the header was fine but the data is not).
    #[error("could not decode the picture: {0}")]
    DisplayDecode(String),
}

/// B-1: read a picture file into memory, refusing anything that is not a regular file and
/// reading at most [`MAX_IMAGE_BYTES`] + 1 bytes.
pub fn read_bounded(path: &Path) -> Result<Vec<u8>, Refusal> {
    read_bounded_with_cap(path, MAX_IMAGE_BYTES)
}

/// [`read_bounded`] with an explicit cap (the test seam; the window always uses the constant).
///
/// Three checks bound the read. The path's reported type and size are checked FIRST, so a
/// non-regular or oversize file is refused without opening or reading it (`s123-SH2` N-5). The
/// OPENED handle is then checked again (`read_opened`), so a path swapped for something else
/// between the check and the open is refused before any read. The `take(cap + 1)` read
/// (`read_capped`) stays as the memory guard against a file that grows after both checks.
///
/// S125: the body is the library gate's [`gate::read_bounded_with_cap`] (SH5 A-C6), so the CLI
/// reads under the same three checks.
pub fn read_bounded_with_cap(path: &Path, cap: u64) -> Result<Vec<u8>, Refusal> {
    gate::read_bounded_with_cap(path, cap).map_err(refusal_from_gate)
}

/// The opened handle's own type and size (fstat), then the capped read (the gate's).
#[cfg_attr(not(test), allow(dead_code))]
fn read_opened(file: std::fs::File, cap: u64) -> Result<Vec<u8>, Refusal> {
    gate::read_opened(file, cap).map_err(refusal_from_gate)
}

/// Read at most `cap + 1` bytes from `source`, then refuse if it held more than `cap`. The
/// `take` bounds MEMORY: however much `source` could yield, no more than `cap + 1` is read.
/// (The gate's.)
#[cfg_attr(not(test), allow(dead_code))]
fn read_capped(source: impl Read, cap: u64) -> Result<Vec<u8>, Refusal> {
    gate::read_capped(source, cap).map_err(refusal_from_gate)
}

/// The window's wording for a library gate refusal. Every value here is exactly the
/// [`Refusal`] the window produced before S125 for the same input (the window's tests pin
/// them), so moving the checks into the library changed no message the window shows.
fn refusal_from_gate(r: PictureRefusal) -> Refusal {
    match r {
        PictureRefusal::NotARegularFile => Refusal::NotARegularFile,
        PictureRefusal::FileTooBig { cap } => Refusal::FileTooBig { cap },
        PictureRefusal::Io(e) => Refusal::Io(e),
        PictureRefusal::Unrecognised(_) => {
            Refusal::NotAnImage("the picture format could not be determined".into())
        }
        PictureRefusal::RiffNotAPicture => {
            Refusal::NotAnImage("a RIFF file (sound, video or SoundFont), not a picture".into())
        }
        PictureRefusal::WebP => Refusal::UnsupportedFormat(format!("{:?}", ImageFormat::WebP)),
        PictureRefusal::UnsupportedFormat(f) => Refusal::UnsupportedFormat(format!("{f:?}")),
        PictureRefusal::Image(e) => Refusal::NotAnImage(e),
        PictureRefusal::TooSmall { width, height, .. } => Refusal::TooSmall { width, height },
        PictureRefusal::SideTooLarge { width, height, .. } => {
            Refusal::SideTooLarge { width, height }
        }
        PictureRefusal::TooManyPixels {
            width,
            height,
            pixels,
            ..
        } => Refusal::TooManyPixels {
            width,
            height,
            pixels,
        },
        // Only the decode produces this; `decode_for_display` reports it as a display decode
        // failure. Here (a header check) it cannot occur, so it reads as not a usable picture.
        r @ PictureRefusal::SizeMismatch { .. } => Refusal::NotAnImage(r.to_string()),
    }
}

/// A picture that passed every header check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Probe {
    /// Width in pixels (from the header).
    pub width: u32,
    /// Height in pixels (from the header).
    pub height: u32,
    /// The sniffed format (from the bytes, never the file name).
    pub format: ImageFormat,
}

/// B-3: the picture formats the window accepts. Everything else the `image` crate's default
/// features would decode (OpenEXR, HDR, DDS, TGA, PNM, Farbfeld, QOI, ICO, AVIF…) is refused,
/// which removes most of the reachable decoder surface with no loss to a non-technical user.
///
/// WebP is NOT accepted (lead ruling L-16, `s123-SH2` H-1 / N-1). In `image 0.24` the WebP
/// decoder decodes the whole frame in its constructor, so [`probe`]'s `into_dimensions` is not a
/// header-only read for WebP: a 52-byte extended WebP declaring a 65535² canvas makes it request
/// about 17 GiB, and an allocation failure aborts the process (no `catch_unwind` can catch it).
/// Because this check runs BEFORE `into_dimensions`, a WebP file never reaches that decoder.
/// Re-admitting WebP needs a hand-parsed RIFF/VP8/VP8L/VP8X size check first (L-16 (a)), and
/// even that cannot bound a lossless WebP (`s125-SH5` HL-1).
///
/// S125: this IS the library gate's allowlist ([`gate::allowed_format`]), which the CLI now
/// uses too (SH5 A-P2/A-C3).
pub fn allowed_format(format: ImageFormat) -> bool {
    gate::allowed_format(format)
}

/// B-2: `width × height` in `u64`. A `u32` product would wrap at 65536² (to 0), which the side
/// cap happens to stop today; computing it in one tested place keeps the pixel cap honest if
/// the order of the checks ever changes. (The gate's [`gate::pixel_count`].)
pub fn pixel_count(width: u32, height: u32) -> u64 {
    gate::pixel_count(width, height)
}

/// B-2: the three dimension caps, in the order floor ([`MIN_SIDE`]), side, pixels: the gate's
/// [`gate::check_dimensions`] with the window's floor.
pub fn check_dimensions(width: u32, height: u32) -> Result<(), Refusal> {
    gate::check_dimensions(width, height, &WINDOW_POLICY).map_err(refusal_from_gate)
}

/// B-2 + B-3: sniff the format FROM THE BYTES, apply the allowlist, and only then read the
/// dimensions from the header and apply the caps. For every format on the allowlist the
/// dimension read is header-only (`s123-SH2` § 4: 0 ms and ≤ 11 MiB for PNG, JPEG, GIF, TIFF
/// and BMP); that is exactly why WebP is off the allowlist (see [`allowed_format`]).
///
/// S125: this is the library gate's [`gate::admit`] under [`WINDOW_POLICY`] (the window's
/// formats, limits and [`MIN_SIDE`] floor), with no extension fallback.
pub fn probe(bytes: &[u8]) -> Result<Probe, Refusal> {
    let Admitted {
        format,
        width,
        height,
    } = gate::admit(bytes, None, &WINDOW_POLICY).map_err(refusal_from_gate)?;
    Ok(Probe {
        width,
        height,
        format,
    })
}

/// The on-screen copy of a picture: RGBA8, long side at most [`DISPLAY_MAX_SIDE`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DisplayImage {
    /// Display width in pixels.
    pub width: u32,
    /// Display height in pixels.
    pub height: u32,
    /// `width * height * 4` bytes, RGBA, unmultiplied.
    pub rgba: Vec<u8>,
}

/// D-1: decode the SAME guarded bytes for display, with explicit limits (B-2's "any decode the
/// GUI performs itself"), then shrink so the long side is at most [`DISPLAY_MAX_SIDE`]. Call only
/// after [`probe`] accepted the bytes. Runs on the worker.
///
/// S125: the decode is the library gate's [`gate::decode`] (the same limits: each side at most
/// [`MAX_SIDE`], at most [`DISPLAY_DECODE_MAX_ALLOC`]), which also refuses a picture whose
/// decoded size differs from its header (SH5 A-C5).
pub fn decode_for_display(bytes: &[u8], probe: &Probe) -> Result<DisplayImage, Refusal> {
    let admitted = Admitted {
        format: probe.format,
        width: probe.width,
        height: probe.height,
    };
    let decoded = gate::decode(bytes, &admitted, &WINDOW_POLICY).map_err(|r| match r {
        PictureRefusal::Image(e) => Refusal::DisplayDecode(e),
        other => Refusal::DisplayDecode(other.to_string()),
    })?;
    let (w, h) = (decoded.width(), decoded.height());
    let shown = if w.max(h) > DISPLAY_MAX_SIDE {
        // `thumbnail` keeps the aspect ratio and fits inside the box.
        decoded.thumbnail(DISPLAY_MAX_SIDE, DISPLAY_MAX_SIDE)
    } else {
        decoded
    };
    let rgba = shown.to_rgba8();
    Ok(DisplayImage {
        width: rgba.width(),
        height: rgba.height(),
        rgba: rgba.into_raw(),
    })
}

#[cfg(test)]
pub(crate) mod fixtures {
    //! Picture fixtures generated in the test, never committed (SH § 5).

    use image::{ImageBuffer, ImageFormat, Rgb};
    use std::io::Cursor;

    /// Encode a solid-colour RGB picture in `format`.
    pub fn solid(width: u32, height: u32, rgb: [u8; 3], format: ImageFormat) -> Vec<u8> {
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_pixel(width, height, Rgb(rgb));
        let mut out = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(img)
            .write_to(&mut out, format)
            .expect("encode fixture");
        out.into_inner()
    }

    /// A small structured RGB picture (gradient plus blocks) so the analyzer has edges to find.
    pub fn structured(width: u32, height: u32, format: ImageFormat) -> Vec<u8> {
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_fn(width, height, |x, y| {
            let block = ((x / 8) + (y / 8)) % 2 == 0;
            let r = ((x * 255) / width.max(1)) as u8;
            let g = ((y * 255) / height.max(1)) as u8;
            let b = if block { 220 } else { 30 };
            Rgb([r, g, b])
        });
        let mut out = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(img)
            .write_to(&mut out, format)
            .expect("encode fixture");
        out.into_inner()
    }

    /// CRC-32 (IEEE), for patching a PNG chunk.
    fn crc32(data: &[u8]) -> u32 {
        let mut h = crc32fast::Hasher::new();
        h.update(data);
        h.finalize()
    }

    /// A 143-byte-ish PNG whose IHDR CLAIMS `width x height`, with a tiny (wrong) IDAT. The
    /// header is valid, the data is not: a decoder that trusted the header would allocate for
    /// the claimed size. `probe` must answer from the header without decoding (SH T7–T9).
    pub fn png_with_dims(width: u32, height: u32) -> Vec<u8> {
        let mut v = solid(1, 1, [128, 128, 128], ImageFormat::Png);
        // IHDR data starts at byte 16 (8 signature + 4 length + 4 "IHDR").
        v[16..20].copy_from_slice(&width.to_be_bytes());
        v[20..24].copy_from_slice(&height.to_be_bytes());
        // CRC covers the chunk type + data: bytes 12..29; CRC sits at 29..33.
        let crc = crc32(&v[12..29]);
        v[29..33].copy_from_slice(&crc.to_be_bytes());
        v
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::*;
    use super::*;
    use std::time::{Duration, Instant};

    fn scratch_dir(name: &str) -> std::path::PathBuf {
        crate::gui::test_scratch_dir(&format!("ingest-{name}"))
    }

    #[test]
    fn t1_one_by_one_white_is_too_small() {
        let b = solid(1, 1, [255, 255, 255], ImageFormat::Png);
        assert_eq!(
            probe(&b),
            Err(Refusal::TooSmall {
                width: 1,
                height: 1
            })
        );
    }

    #[test]
    fn t2_thin_portraits_are_too_small() {
        for (w, h) in [(1, 50), (2, 100)] {
            let b = solid(w, h, [200, 10, 10], ImageFormat::Png);
            assert_eq!(
                probe(&b),
                Err(Refusal::TooSmall {
                    width: w,
                    height: h
                })
            );
        }
    }

    #[test]
    fn t3_sixteen_is_the_floor() {
        for (w, h) in [(15, 16), (16, 15)] {
            let b = solid(w, h, [90, 90, 90], ImageFormat::Png);
            assert_eq!(
                probe(&b),
                Err(Refusal::TooSmall {
                    width: w,
                    height: h
                })
            );
        }
        let ok = solid(16, 16, [90, 90, 90], ImageFormat::Png);
        let p = probe(&ok).expect("16x16 admitted");
        assert_eq!((p.width, p.height), (16, 16));
    }

    #[test]
    fn t4_t5_empty_and_text_are_not_images() {
        assert!(matches!(probe(&[]), Err(Refusal::NotAnImage(_))));
        assert!(matches!(
            probe(b"this is not a picture, it is text renamed .png"),
            Err(Refusal::NotAnImage(_))
        ));
    }

    #[test]
    fn t6_truncated_png_passes_the_header_probe() {
        let full = structured(64, 64, ImageFormat::Png);
        let half = &full[..full.len() / 2];
        let p = probe(half).expect("header is intact");
        assert_eq!((p.width, p.height), (64, 64));
        // …and the display decode fails cleanly (no panic).
        assert!(matches!(
            decode_for_display(half, &p),
            Err(Refusal::DisplayDecode(_))
        ));
    }

    #[test]
    fn t7_huge_header_is_refused_without_decoding() {
        let b = png_with_dims(100_000, 100_000);
        let t0 = Instant::now();
        let r = probe(&b);
        let took = t0.elapsed();
        assert_eq!(
            r,
            Err(Refusal::SideTooLarge {
                width: 100_000,
                height: 100_000
            })
        );
        assert!(
            took.as_millis() < 50,
            "probe took {took:?}; it must not decode"
        );
    }

    #[test]
    fn t8_pixel_cap_boundary() {
        // The cap's own edge (strengthened in the S123 fix round: the old pair straddled the cap
        // by 74,000 px, so `>` → `>=` survived it). 10000 x 5000 is EXACTLY the cap: admitted.
        let p = probe(&png_with_dims(10_000, 5_000)).expect("exactly 50,000,000 px admitted");
        assert_eq!((p.width, p.height), (10_000, 5_000));
        // 2217 x 22553 = 50,000,001: one pixel over (both sides under the side cap).
        assert_eq!(
            probe(&png_with_dims(2217, 22_553)),
            Err(Refusal::TooManyPixels {
                width: 2217,
                height: 22_553,
                pixels: 50_000_001
            })
        );
        // The original pair, kept: 7072² = 50,013,184 refused; 8160 x 6120 = 49,939,200 admitted.
        assert!(matches!(
            probe(&png_with_dims(7072, 7072)),
            Err(Refusal::TooManyPixels { .. })
        ));
        assert!(probe(&png_with_dims(8160, 6120)).is_ok());
    }

    #[test]
    fn t9_side_cap_boundary() {
        assert!(matches!(
            probe(&png_with_dims(32_769, 16)),
            Err(Refusal::SideTooLarge { .. })
        ));
        assert!(probe(&png_with_dims(32_768, 1500)).is_ok());
    }

    #[test]
    fn pixel_count_is_computed_in_u64() {
        // Strengthened in the S123 fix round (TODO-6): the old `pixel_product_is_u64` asserted
        // only that check_dimensions refused, which the side cap guarantees whatever the product
        // type, so it could not fail. `pixel_count` is the function the cap uses; a u32 product
        // wraps 65536² to 0 and 65537 x 65535 to 0 as well.
        assert_eq!(pixel_count(65_536, 65_536), 4_294_967_296);
        assert_eq!(pixel_count(65_537, 65_535), 4_294_967_295);
        assert_eq!(pixel_count(u32::MAX, u32::MAX), 18_446_744_065_119_617_025);
        assert_eq!(pixel_count(10_000, 5_000), MAX_PIXELS);
        // …and check_dimensions reports that same count.
        assert_eq!(
            check_dimensions(20_000, 20_000),
            Err(Refusal::TooManyPixels {
                width: 20_000,
                height: 20_000,
                pixels: pixel_count(20_000, 20_000)
            })
        );
    }

    #[test]
    fn t10_jpeg_saved_as_png_is_sniffed_from_the_bytes() {
        let dir = scratch_dir("t10");
        let path = dir.join("actually-a-jpeg.png");
        std::fs::write(&path, structured(64, 48, ImageFormat::Jpeg)).expect("write fixture");
        let bytes = read_bounded(&path).expect("read");
        let p = probe(&bytes).expect("sniffed");
        assert_eq!(p.format, ImageFormat::Jpeg);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn t11_formats_off_the_allowlist_are_refused() {
        for f in [
            ImageFormat::Tga,
            ImageFormat::Qoi,
            ImageFormat::Pnm,
            ImageFormat::Farbfeld,
            ImageFormat::OpenExr,
            ImageFormat::Ico,
        ] {
            let b = {
                // OpenEXR needs float pixels, Farbfeld RGBA16; the others take RGB8.
                let mut out = std::io::Cursor::new(Vec::new());
                let img = if f == ImageFormat::OpenExr {
                    image::DynamicImage::ImageRgb32F(image::ImageBuffer::from_pixel(
                        32,
                        32,
                        image::Rgb([0.5f32, 0.5, 0.5]),
                    ))
                } else if f == ImageFormat::Farbfeld {
                    image::DynamicImage::ImageRgba16(image::ImageBuffer::from_pixel(
                        32,
                        32,
                        image::Rgba([1000u16, 2000, 3000, 65535]),
                    ))
                } else {
                    image::DynamicImage::ImageRgb8(image::ImageBuffer::from_pixel(
                        32,
                        32,
                        image::Rgb([10u8, 20, 30]),
                    ))
                };
                img.write_to(&mut out, f).expect("encode");
                out.into_inner()
            };
            let r = probe(&b);
            // TGA has no magic number, so the sniffer may not recognise it at all; either
            // refusal keeps it away from the decoder.
            assert!(
                matches!(
                    r,
                    Err(Refusal::UnsupportedFormat(_)) | Err(Refusal::NotAnImage(_))
                ),
                "{f:?} gave {r:?}"
            );
        }
        for f in [
            ImageFormat::Png,
            ImageFormat::Jpeg,
            ImageFormat::Gif,
            ImageFormat::Bmp,
            ImageFormat::Tiff,
        ] {
            let b = solid(32, 32, [10, 20, 30], f);
            assert!(probe(&b).is_ok(), "{f:?} should be accepted");
        }
        // L-16: WebP is refused before its decoder is reached (see the two forcing tests below).
        assert!(!allowed_format(ImageFormat::WebP));
    }

    /// `s123-SH2` H-1's 38-byte lossless (`VP8L`) WebP declaring 16383 x 16383. Through the
    /// round-1 probe it took 1.86 s and about 2.0 GiB before being refused. Bytes copied from the
    /// reviewer's crafted file (`webp-268mp-lossless.webp`); nothing is committed as a fixture.
    pub(crate) const WEBP_VP8L_16383: [u8; 38] = [
        0x52, 0x49, 0x46, 0x46, 0x1e, 0x00, 0x00, 0x00, 0x57, 0x45, 0x42, 0x50, 0x56, 0x50, 0x38,
        0x4c, 0x11, 0x00, 0x00, 0x00, 0x2f, 0xfe, 0xbf, 0xff, 0x0f, 0x07, 0x50, 0xad, 0xa2, 0x14,
        0xb4, 0xff, 0x81, 0x88, 0xe8, 0x7f, 0x00, 0x00,
    ];

    /// `s123-SH2` H-1's 52-byte extended (`VP8X` + `ALPH`) WebP declaring a 65535 x 65535 canvas.
    /// Through the round-1 probe it ABORTED the process on a 17 GiB allocation. Bytes copied from
    /// the reviewer's crafted file (`webp-alph-vp8l-4gp.webp`).
    pub(crate) const WEBP_VP8X_ALPH_65535: [u8; 52] = [
        0x52, 0x49, 0x46, 0x46, 0x2c, 0x00, 0x00, 0x00, 0x57, 0x45, 0x42, 0x50, 0x56, 0x50, 0x38,
        0x58, 0x0a, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0xfe, 0xff, 0x00, 0xfe, 0xff, 0x00,
        0x41, 0x4c, 0x50, 0x48, 0x0d, 0x00, 0x00, 0x00, 0x01, 0x07, 0x50, 0xad, 0xa2, 0x14, 0xb4,
        0xff, 0x81, 0x88, 0xe8, 0x7f, 0x00, 0x00,
    ];

    fn assert_webp_refused_promptly(bytes: &[u8], what: &str) {
        let t0 = Instant::now();
        let r = probe(bytes);
        let took = t0.elapsed();
        assert_eq!(
            r,
            Err(Refusal::UnsupportedFormat("WebP".into())),
            "{what}: must be refused by the allowlist, before any WebP decode"
        );
        assert!(took.as_millis() < 50, "{what}: probe took {took:?}");
    }

    #[test]
    fn webp_vp8l_bomb_is_refused_before_any_decode() {
        assert_webp_refused_promptly(&WEBP_VP8L_16383, "38-byte VP8L 16383²");
    }

    #[test]
    fn webp_vp8x_alpha_bomb_is_refused_before_any_decode() {
        assert_webp_refused_promptly(&WEBP_VP8X_ALPH_65535, "52-byte VP8X+ALPH 65535²");
    }

    #[test]
    fn t12_byte_cap_reads_at_most_cap_plus_one() {
        let dir = scratch_dir("t12");
        let path = dir.join("sparse.bin");
        let f = std::fs::File::create(&path).expect("create");
        // A sparse file one byte over a small cap (uses no disk). The real cap is exercised
        // by the same code path; the constant is checked separately.
        f.set_len(4097).expect("set_len");
        drop(f);
        assert_eq!(
            read_bounded_with_cap(&path, 4096),
            Err(Refusal::FileTooBig { cap: 4096 })
        );
        assert!(read_bounded_with_cap(&path, 4097).is_ok());
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(MAX_IMAGE_BYTES, 268_435_456);
    }

    #[cfg(unix)]
    #[test]
    fn n5_an_oversize_file_is_refused_by_its_size_without_reading_it() {
        // `s123-SH2` N-5. The file is made UNREADABLE: if the window opened and read it before
        // checking the size, the answer would be an I/O error. Refused by size, it never opens it.
        use std::os::unix::fs::PermissionsExt;
        let dir = scratch_dir("n5");
        let path = dir.join("big-and-unreadable.bin");
        let f = std::fs::File::create(&path).expect("create");
        f.set_len(4097).expect("set_len");
        drop(f);
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).expect("chmod");
        let readable_anyway = std::fs::File::open(&path).is_ok(); // running as root
        let r = read_bounded_with_cap(&path, 4096);
        let at_cap = read_bounded_with_cap(&path, 4097);
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644))
            .expect("chmod back");
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(r, Err(Refusal::FileTooBig { cap: 4096 }));
        if !readable_anyway {
            // Control: at a cap it does not exceed, the same file IS opened, and fails to open.
            assert!(matches!(at_cap, Err(Refusal::Io(_))), "{at_cap:?}");
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_file_that_reports_size_zero_is_still_refused_when_its_content_is_over_the_cap() {
        // A `/proc` file reports length 0 but yields content when read, which is the "file grew
        // after the size checks" case. The verdict here comes from `read_capped`'s length check;
        // the `take` memory bound is pinned by `the_read_cap_reads_no_more_than_cap_plus_one_byte`.
        let path = Path::new("/proc/self/maps");
        let reported = std::fs::metadata(path).map(|m| m.len()).unwrap_or(1);
        if reported != 0 {
            return; // not the /proc this test expects; nothing to show
        }
        assert_eq!(
            read_bounded_with_cap(path, 16),
            Err(Refusal::FileTooBig { cap: 16 })
        );
    }

    /// A source that can yield `left` bytes and counts how many it actually handed out.
    struct CountingSource {
        left: u64,
        served: u64,
    }

    impl Read for CountingSource {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let n = (buf.len() as u64).min(self.left) as usize;
            buf[..n].fill(0x55);
            self.left -= n as u64;
            self.served += n as u64;
            Ok(n)
        }
    }

    #[test]
    fn the_read_cap_reads_no_more_than_cap_plus_one_byte() {
        // `s123-06` MA11: the old test stayed green with the `take` removed, because the length
        // check decided its verdict. This one measures what `take` bounds: bytes READ.
        let cap = 16;
        let mut src = CountingSource {
            left: 1 << 20,
            served: 0,
        };
        assert_eq!(read_capped(&mut src, cap), Err(Refusal::FileTooBig { cap }));
        assert_eq!(src.served, cap + 1, "read past the cap");
        // Control: a source at exactly the cap is read whole and admitted.
        let mut ok = CountingSource {
            left: cap,
            served: 0,
        };
        assert_eq!(read_capped(&mut ok, cap).map(|b| b.len() as u64), Ok(cap));
    }

    #[cfg(unix)]
    #[test]
    fn an_opened_handle_that_is_not_a_regular_file_is_refused_before_any_read() {
        // `s123-06` N-B: the path check runs before `open`; a path swapped between the two is
        // caught by the fstat of the OPENED handle. `/dev/zero` reports size 0, so without that
        // check it would be read up to the cap and refused as too big instead.
        let t0 = Instant::now();
        let zero = std::fs::File::open("/dev/zero").expect("open /dev/zero");
        assert_eq!(read_opened(zero, 4096), Err(Refusal::NotARegularFile));
        let dir = scratch_dir("fstat");
        let handle = std::fs::File::open(&dir).expect("open a directory read-only");
        assert_eq!(read_opened(handle, 4096), Err(Refusal::NotARegularFile));
        let _ = std::fs::remove_dir_all(&dir);
        assert!(t0.elapsed() < Duration::from_secs(5));
        // Control: a regular file's handle is read.
        let dir = scratch_dir("fstat-ok");
        let path = dir.join("ok.bin");
        std::fs::write(&path, [1u8, 2, 3]).expect("write");
        let f = std::fs::File::open(&path).expect("open");
        assert_eq!(read_opened(f, 4096), Ok(vec![1, 2, 3]));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn t12b_real_cap_sparse_file() {
        let dir = scratch_dir("t12b");
        let path = dir.join("sparse-256mib-plus-1.bin");
        let f = std::fs::File::create(&path).expect("create");
        f.set_len(MAX_IMAGE_BYTES + 1).expect("set_len");
        drop(f);
        assert_eq!(
            read_bounded(&path),
            Err(Refusal::FileTooBig {
                cap: MAX_IMAGE_BYTES
            })
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn t13_non_regular_files_are_refused_promptly() {
        let t0 = Instant::now();
        assert_eq!(
            read_bounded(Path::new("/dev/zero")),
            Err(Refusal::NotARegularFile)
        );
        let dir = scratch_dir("t13");
        assert_eq!(read_bounded(&dir), Err(Refusal::NotARegularFile));
        let fifo = dir.join("fifo");
        let made = std::process::Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if made {
            assert_eq!(read_bounded(&fifo), Err(Refusal::NotARegularFile));
        }
        assert!(t0.elapsed().as_millis() < 1000);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn display_copy_is_downscaled_to_the_display_cap() {
        let b = structured(2600, 1300, ImageFormat::Png);
        let p = probe(&b).expect("admitted");
        let d = decode_for_display(&b, &p).expect("decoded");
        assert!(d.width.max(d.height) <= DISPLAY_MAX_SIDE);
        assert_eq!(d.rgba.len(), (d.width * d.height * 4) as usize);
        // Aspect kept (2:1).
        assert_eq!(d.width, 2 * d.height);
    }

    #[test]
    fn small_pictures_are_shown_at_their_own_size() {
        let b = structured(64, 48, ImageFormat::Png);
        let p = probe(&b).expect("admitted");
        let d = decode_for_display(&b, &p).expect("decoded");
        assert_eq!((d.width, d.height), (64, 48));
    }

    #[test]
    fn a_soundfont_is_not_a_picture() {
        // S-1: a dropped .sf2 (RIFF/sfbk) is refused as "not a picture".
        let mut sf2 = b"RIFF".to_vec();
        sf2.extend_from_slice(&100u32.to_le_bytes());
        sf2.extend_from_slice(b"sfbkLIST");
        sf2.resize(108, 0);
        assert!(matches!(probe(&sf2), Err(Refusal::NotAnImage(_))));
        // The same holds for any non-WebP RIFF file, e.g. a WAV the user drops by mistake.
        let mut wav = b"RIFF".to_vec();
        wav.extend_from_slice(&100u32.to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        wav.resize(108, 0);
        assert!(matches!(probe(&wav), Err(Refusal::NotAnImage(_))));
    }
}

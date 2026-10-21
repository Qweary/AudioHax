//! The picture's way into the window (Shielding `s123-SH` § 4.1 and § 4.9).
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
//! The library's own decode uses the `image` crate's default limits; that is safe only because
//! [`probe`] has already refused anything over the pixel cap on the same bytes (SH B-2).

use image::ImageFormat;
use std::io::{Cursor, Read};
use std::path::Path;

/// B-1: the most bytes read from a picture file into memory (256 MiB). It bounds the read
/// before the format is known (a video renamed `.jpg`); it is NOT the decompression-bomb
/// defence, which is [`MAX_PIXELS`].
pub const MAX_IMAGE_BYTES: u64 = 268_435_456;

/// B-2: the smallest side admitted. Both known analyzer panic classes are at sides of 1–2 px
/// (SH HL-1), and 16 makes every Voices value the window offers (1..=16) renderable.
pub const MIN_SIDE: u32 = 16;

/// B-2: the largest side admitted (stops absurd headers early; admits ordinary panoramas).
pub const MAX_SIDE: u32 = 32_768;

/// B-2: the largest pixel count admitted (50 MP). At about 25 B/px of analysis this caps one
/// job at about 1.25 GiB and about 75 s on the measured laptop (SH § 3.2). It is also the time
/// bound: the analysis cost is linear in pixels and cannot be interrupted.
pub const MAX_PIXELS: u64 = 50_000_000;

/// D-1: the long side of the ON-SCREEN copy of the picture. Display only; the engine always gets
/// the full original bytes.
pub const DISPLAY_MAX_SIDE: u32 = 2048;

/// B-2: the allocation limit for any decode the window performs itself (512 MiB).
pub const DISPLAY_DECODE_MAX_ALLOC: u64 = 512 * 1024 * 1024;

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
pub fn read_bounded_with_cap(path: &Path, cap: u64) -> Result<Vec<u8>, Refusal> {
    let meta = std::fs::metadata(path).map_err(|e| Refusal::Io(e.to_string()))?;
    if !meta.is_file() {
        return Err(Refusal::NotARegularFile);
    }
    let file = std::fs::File::open(path).map_err(|e| Refusal::Io(e.to_string()))?;
    let mut bytes = Vec::new();
    file.take(cap.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|e| Refusal::Io(e.to_string()))?;
    if bytes.len() as u64 > cap {
        return Err(Refusal::FileTooBig { cap });
    }
    Ok(bytes)
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
pub fn allowed_format(format: ImageFormat) -> bool {
    matches!(
        format,
        ImageFormat::Png
            | ImageFormat::Jpeg
            | ImageFormat::Gif
            | ImageFormat::WebP
            | ImageFormat::Bmp
            | ImageFormat::Tiff
    )
}

/// B-2: the three dimension caps, with the pixel count computed in `u64` (a `u32` product
/// overflows at 65536²).
pub fn check_dimensions(width: u32, height: u32) -> Result<(), Refusal> {
    if width.min(height) < MIN_SIDE {
        return Err(Refusal::TooSmall { width, height });
    }
    if width.max(height) > MAX_SIDE {
        return Err(Refusal::SideTooLarge { width, height });
    }
    let pixels = u64::from(width) * u64::from(height);
    if pixels > MAX_PIXELS {
        return Err(Refusal::TooManyPixels {
            width,
            height,
            pixels,
        });
    }
    Ok(())
}

/// B-2 + B-3: sniff the format FROM THE BYTES and read the dimensions from the header, without
/// decoding any pixel data, then apply the allowlist and the caps. Measured cost ≤ 86 µs (SH § 3.2).
pub fn probe(bytes: &[u8]) -> Result<Probe, Refusal> {
    let reader = image::io::Reader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| Refusal::NotAnImage(e.to_string()))?;
    let format = reader
        .format()
        .ok_or_else(|| Refusal::NotAnImage("the picture format could not be determined".into()))?;
    if !allowed_format(format) {
        return Err(Refusal::UnsupportedFormat(format!("{format:?}")));
    }
    let (width, height) = reader
        .into_dimensions()
        .map_err(|e| Refusal::NotAnImage(e.to_string()))?;
    check_dimensions(width, height)?;
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
pub fn decode_for_display(bytes: &[u8], probe: &Probe) -> Result<DisplayImage, Refusal> {
    let mut reader = image::io::Reader::new(Cursor::new(bytes));
    reader.set_format(probe.format);
    let mut limits = image::io::Limits::default();
    limits.max_image_width = Some(MAX_SIDE);
    limits.max_image_height = Some(MAX_SIDE);
    limits.max_alloc = Some(DISPLAY_DECODE_MAX_ALLOC);
    reader.limits(limits);
    let decoded = reader
        .decode()
        .map_err(|e| Refusal::DisplayDecode(e.to_string()))?;
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
    use std::time::Instant;

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
        // 7072 x 7072 = 50,013,184 > 50 M.
        assert!(matches!(
            probe(&png_with_dims(7072, 7072)),
            Err(Refusal::TooManyPixels { .. })
        ));
        // 8160 x 6120 = 49,939,200 <= 50 M: admitted (NOT decoded here).
        let p = probe(&png_with_dims(8160, 6120)).expect("49.94 MP admitted");
        assert_eq!((p.width, p.height), (8160, 6120));
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
    fn pixel_product_is_u64() {
        // 65536 x 65536 overflows u32; the side cap fires first, but the product must not wrap.
        assert!(check_dimensions(65_536, 65_536).is_err());
        assert!(check_dimensions(20_000, 20_000).is_err()); // 400 MP, sides under the cap
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
        assert!(allowed_format(ImageFormat::WebP));
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
    }
}

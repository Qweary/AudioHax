//! src/picture_gate.rs — the ONE picture gate (S125, D-45; `s125-SH5` Part A; lead rulings
//! L-60/L-61/L-80; the operator's `s125-00-OPERATOR-RULINGS.md` § A.2/A.4/A.5).
//!
//! Every user picture the library decodes passes through here, from inside
//! [`crate::pure_analysis::load_pure_image`] and [`crate::pure_analysis::load_pure_image_bytes`]
//! (SH5 A-P1), and the window's header check (`gui::ingest::probe`) calls the same [`admit`]
//! (A-P2). What differs between front-ends is a [`Policy`] handed to the gate, never a second
//! gate.
//!
//! The two policies and their routes:
//!
//! * [`Policy::CLI`] (operator A.5, Q6 "a": "keep exactly what the CLI accepted before … except
//!   WebP; it refuses nothing new"; lead ruling L-80), through [`open_and_decode`]: the read and the
//!   decode `image::open` did at `369938d`, step for step (the file streamed through a `BufReader`,
//!   the format from the file extension, the `image` crate's DEFAULT [`image::io::Limits`]), except
//!   that a WebP picture is refused before any WebP decoder exists (operator Q1 "a").
//! * [`Policy::WINDOW`] (`s123-SH` B-1…B-3, unchanged since S123), through [`read_bounded_with_cap`],
//!   [`admit`] and [`decode`], on the SAME bytes throughout: PNG, JPEG, GIF, BMP, TIFF, sniffed from
//!   the bytes (A-P3); each side ≤ [`MAX_SIDE`], ≤ [`MAX_PIXELS`], file ≤ [`MAX_IMAGE_BYTES`], decode
//!   ≤ [`DECODE_MAX_ALLOC`], and a decoded size that must equal the header's (A-C5). The window adds
//!   its own 16 px floor (`gui::ingest::MIN_SIDE`).
//!
//! Byte-identity: for a picture that decodes today, the codec, the bytes and the pixels are
//! unchanged (same `load_inner`, same limits on the CLI), and every error text a picture produced
//! before is produced verbatim (the `image` crate's own message is carried through).

use image::{DynamicImage, ImageError, ImageFormat};
use std::io::{BufReader, Cursor, Read, Seek, SeekFrom};
use std::path::Path;

/// The window's byte cap on a picture file (256 MiB). It bounds the read before the format is
/// known; it is NOT the decompression-bomb defence, which is [`MAX_PIXELS`] (`s123-SH` B-1).
pub const MAX_IMAGE_BYTES: u64 = 268_435_456;

/// The window's longest side, in pixels (`s123-SH` B-2).
pub const MAX_SIDE: u32 = 32_768;

/// The window's most pixels (50 MP; about 1.25 GiB and 75 s of analysis on the measured laptop,
/// `s123-SH` § 3.2).
pub const MAX_PIXELS: u64 = 50_000_000;

/// The decoder allocation limit (512 MiB): the `image` crate's own default (`Limits::default()`
/// in `image 0.24.9`), which the window sets explicitly and which the CLI had at `369938d` through
/// `image::open`. Both policies carry it; a test pins that the CLI's limits equal the crate's
/// default exactly.
pub const DECODE_MAX_ALLOC: u64 = 512 * 1024 * 1024;

/// How many leading bytes of a file are looked at to tell a WebP (`RIFF....WEBP`) from anything
/// else (the CLI's route only; see [`open_and_decode`]).
const RIFF_HEAD_BYTES: u64 = 12;

/// Which picture formats a front-end accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Formats {
    /// Every format the `image` crate reads, except WebP (the CLI; operator A.2/A.5, L-80).
    AllButWebP,
    /// PNG, JPEG, GIF, BMP and TIFF (the window; `s123-SH` B-3).
    WindowFive,
}

/// One front-end's rules for the one gate. `None` means "no such limit".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Policy {
    /// The formats admitted. WebP is refused under every policy (SH5 A-W1).
    pub formats: Formats,
    /// The smallest side admitted (0 = no floor).
    pub min_side: u32,
    /// The longest side admitted.
    pub max_side: Option<u32>,
    /// The most pixels admitted.
    pub max_pixels: Option<u64>,
    /// The most bytes read from a file. `Some` (the window's B-1): regular files only, read into
    /// memory at most this many bytes, then checked and decoded from that buffer. `None` (the
    /// CLI): the file is not read into memory first; it is streamed to the decoder, as `image::open`
    /// did at `369938d` (see [`open_and_decode`]).
    pub max_file_bytes: Option<u64>,
    /// The decoder's allocation limit.
    pub decode_max_alloc: Option<u64>,
}

impl Policy {
    /// The CLI (operator A.2: "I would rather give more options to the command line version and
    /// have the mismatch than restrict options just so two ways of using it match"; A.5, Q6 "a",
    /// the option Neo rendered as "keep exactly what the CLI accepted before … every picture it read
    /// before still reads, except WebP; it refuses nothing new"; lead ruling L-80).
    ///
    /// Exactly `369938d`'s acceptance set minus WebP: every format the `image` crate reads except
    /// WebP, no side, pixel or file-size limit, no floor, and the decoder's allocation limit at the
    /// `image` crate's own default ([`DECODE_MAX_ALLOC`], `max_alloc` only, as `image::open` used
    /// it). That default limit is what refused, cleanly, every small file whose header claims a
    /// huge picture at `369938d`, so the CLI refuses them cleanly again at any memory cap; a
    /// picture that decoded at `369938d` decodes identically. Its route is [`open_and_decode`].
    pub const CLI: Policy = Policy {
        formats: Formats::AllButWebP,
        min_side: 0,
        max_side: None,
        max_pixels: None,
        max_file_bytes: None,
        decode_max_alloc: Some(DECODE_MAX_ALLOC),
    };

    /// The window (`s123-SH` B-1…B-3, as shipped in S123/S124). Its 16 px floor is added by
    /// `gui::ingest`; the library's in-memory arm uses this policy with no floor.
    pub const WINDOW: Policy = Policy {
        formats: Formats::WindowFive,
        min_side: 0,
        max_side: Some(MAX_SIDE),
        max_pixels: Some(MAX_PIXELS),
        max_file_bytes: Some(MAX_IMAGE_BYTES),
        decode_max_alloc: Some(DECODE_MAX_ALLOC),
    };

    /// Whether this policy admits `format`. WebP is never admitted.
    pub fn allows(&self, format: ImageFormat) -> bool {
        match self.formats {
            Formats::AllButWebP => format != ImageFormat::WebP,
            Formats::WindowFive => allowed_format(format),
        }
    }

    /// The decoder limits for this policy.
    pub fn decode_limits(&self) -> image::io::Limits {
        let mut limits = image::io::Limits::no_limits();
        limits.max_image_width = self.max_side;
        limits.max_image_height = self.max_side;
        limits.max_alloc = self.decode_max_alloc;
        limits
    }
}

/// Why the gate refused a picture. Each variant's `Display` is one plain sentence fragment; the
/// caller adds the file name in front.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PictureRefusal {
    /// Not a regular file (a folder; for the window also a device such as `/dev/zero` or a pipe).
    #[error("this is not a picture file (it is a folder, a device or a pipe)")]
    NotARegularFile,
    /// More than the byte cap.
    #[error("the file is larger than {} ({cap} bytes), the most AudioHax reads", mib(*.cap))]
    FileTooBig {
        /// The cap that was exceeded.
        cap: u64,
    },
    /// The file could not be opened or read. The text is the operating system's, verbatim.
    #[error("{0}")]
    Io(String),
    /// The bytes did not identify a format and no fallback format was given. The text is the
    /// `image` crate's, verbatim.
    #[error("{0}")]
    Unrecognised(String),
    /// A WebP file (SH5 A-W1: every WebP is refused for the first release, on every front-end).
    #[error("WebP pictures are not supported yet. Save the picture as PNG or JPEG and try again")]
    WebP,
    /// A RIFF file that is not WebP (a WAV, an AVI, a SoundFont). Given only when the bytes start
    /// with `RIFF`.
    #[error("this is a RIFF file (sound, video or SoundFont), not a picture")]
    RiffNotAPicture,
    /// A real picture format, but not one this front-end accepts (the window, SH5 A-C3).
    #[error("{0:?} pictures are not supported. Save the picture as PNG or JPEG and try again")]
    UnsupportedFormat(ImageFormat),
    /// The `image` crate refused the header or the data. The text is the crate's, verbatim, so a
    /// picture that failed before fails with the same words.
    #[error("{0}")]
    Image(String),
    /// A side is under the policy's floor (the window's 16 px; the CLI has none).
    #[error(
        "the picture is {width}×{height} pixels; each side must be at least {min_side} pixels"
    )]
    TooSmall {
        /// Width in pixels.
        width: u32,
        /// Height in pixels.
        height: u32,
        /// The floor that was not met.
        min_side: u32,
    },
    /// A side is over the policy's longest side.
    #[error("the picture is {width}×{height} pixels; no side may be longer than {} pixels", group(u64::from(*.max_side)))]
    SideTooLarge {
        /// Width in pixels.
        width: u32,
        /// Height in pixels.
        height: u32,
        /// The limit.
        max_side: u32,
    },
    /// More pixels than the policy allows.
    #[error("the picture is {width}×{height} ({} pixels); the limit is {} pixels", group(*.pixels), group(*.max_pixels))]
    TooManyPixels {
        /// Width in pixels.
        width: u32,
        /// Height in pixels.
        height: u32,
        /// `width * height`, in `u64`.
        pixels: u64,
        /// The limit.
        max_pixels: u64,
    },
    /// The decoded picture is not the size its header declared.
    #[error("the picture's header says {header_width}×{header_height} pixels but its contents are {width}×{height}; the file is damaged or unusual, so it was not used")]
    SizeMismatch {
        /// Width from the header.
        header_width: u32,
        /// Height from the header.
        header_height: u32,
        /// Decoded width.
        width: u32,
        /// Decoded height.
        height: u32,
    },
}

/// `1234567` → `"1,234,567"`.
fn group(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// A byte count as whole MiB when it is one (`268435456` → `"256 MiB"`).
fn mib(n: u64) -> String {
    if n.is_multiple_of(1024 * 1024) {
        format!("{} MiB", n / (1024 * 1024))
    } else {
        format!("{} bytes", group(n))
    }
}

/// A picture that passed every header check. `width`/`height` are the header's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Admitted {
    /// The format the decoder will use (sniffed from the bytes; see [`admit`]).
    pub format: ImageFormat,
    /// Width in pixels, from the header.
    pub width: u32,
    /// Height in pixels, from the header.
    pub height: u32,
}

/// The window's read: a regular file, at most `cap` bytes.
///
/// The path's reported type and size are checked first, so a non-regular or oversize file is
/// refused without opening it (`s123-SH2` N-5). The OPENED handle is checked again, so a path
/// swapped between the check and the open is refused before any read. The `take(cap + 1)` read
/// bounds memory against a file that grows after both checks. (Moved here from `gui::ingest` in
/// S125, SH5 A-C6.)
pub fn read_bounded_with_cap(path: &Path, cap: u64) -> Result<Vec<u8>, PictureRefusal> {
    let meta = std::fs::metadata(path).map_err(|e| PictureRefusal::Io(e.to_string()))?;
    if !meta.is_file() {
        return Err(PictureRefusal::NotARegularFile);
    }
    if meta.len() > cap {
        return Err(PictureRefusal::FileTooBig { cap });
    }
    let file = std::fs::File::open(path).map_err(|e| PictureRefusal::Io(e.to_string()))?;
    read_opened(file, cap)
}

/// The opened handle's own type and size (fstat), then the capped read.
pub fn read_opened(file: std::fs::File, cap: u64) -> Result<Vec<u8>, PictureRefusal> {
    let meta = file
        .metadata()
        .map_err(|e| PictureRefusal::Io(e.to_string()))?;
    if !meta.is_file() {
        return Err(PictureRefusal::NotARegularFile);
    }
    if meta.len() > cap {
        return Err(PictureRefusal::FileTooBig { cap });
    }
    read_capped(file, cap)
}

/// Read at most `cap + 1` bytes from `source`, then refuse if it held more than `cap`.
pub fn read_capped(source: impl Read, cap: u64) -> Result<Vec<u8>, PictureRefusal> {
    let mut bytes = Vec::new();
    source
        .take(cap.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|e| PictureRefusal::Io(e.to_string()))?;
    if bytes.len() as u64 > cap {
        return Err(PictureRefusal::FileTooBig { cap });
    }
    Ok(bytes)
}

/// The window's five formats (`s123-SH` B-3; [`Formats::WindowFive`]). WebP is not one of them:
/// in `image 0.24` the WebP decoder decodes in its constructor, so no dimension read is
/// header-only for WebP, and a valid 16×16 lossless WebP can cost GiBs (SH5 H07–H09).
pub fn allowed_format(format: ImageFormat) -> bool {
    matches!(
        format,
        ImageFormat::Png
            | ImageFormat::Jpeg
            | ImageFormat::Gif
            | ImageFormat::Bmp
            | ImageFormat::Tiff
    )
}

/// `width × height` in `u64` (a `u32` product wraps at 65536²).
pub fn pixel_count(width: u32, height: u32) -> u64 {
    u64::from(width) * u64::from(height)
}

/// The policy's dimension checks, in this order: the floor, then the longest side, then the
/// pixel count (the window's `s123` order). The CLI policy has none of them.
pub fn check_dimensions(width: u32, height: u32, policy: &Policy) -> Result<(), PictureRefusal> {
    if width.min(height) < policy.min_side {
        return Err(PictureRefusal::TooSmall {
            width,
            height,
            min_side: policy.min_side,
        });
    }
    if let Some(max_side) = policy.max_side {
        if width.max(height) > max_side {
            return Err(PictureRefusal::SideTooLarge {
                width,
                height,
                max_side,
            });
        }
    }
    if let Some(max_pixels) = policy.max_pixels {
        let pixels = pixel_count(width, height);
        if pixels > max_pixels {
            return Err(PictureRefusal::TooManyPixels {
                width,
                height,
                pixels,
                max_pixels,
            });
        }
    }
    Ok(())
}

/// Admit `bytes` as a picture under `policy`, or refuse, WITHOUT decoding any pixels.
///
/// * The format is sniffed from the bytes. Only when the bytes carry no recognised signature is
///   `fallback` used (no library route passes one since round 3 of S125: the window passes
///   `None`, and the CLI's route is [`open_and_decode`]). `fallback` can never override a sniffed
///   format, so the gate always keys on the format the decoder will use (SH5 A-P3).
/// * WebP is refused before any WebP decoder exists, under every policy; then the policy's
///   formats.
/// * The dimensions come from `into_dimensions`.
/// * Then [`check_dimensions`].
pub fn admit(
    bytes: &[u8],
    fallback: Option<Result<ImageFormat, ImageError>>,
    policy: &Policy,
) -> Result<Admitted, PictureRefusal> {
    let format = match image::guess_format(bytes) {
        Ok(f) => f,
        Err(sniff_error) => match fallback {
            Some(Ok(f)) => f,
            Some(Err(e)) => return Err(PictureRefusal::Image(e.to_string())),
            None => return Err(PictureRefusal::Unrecognised(sniff_error.to_string())),
        },
    };
    if format == ImageFormat::WebP {
        // `image 0.24` guesses WebP from the 4-byte "RIFF" prefix alone. A RIFF file that is
        // not "RIFF....WEBP" is a sound, video or SoundFont file; anything else is refused as
        // WebP (the sniff or the fallback said WebP).
        return Err(webp_refusal(bytes));
    }
    if !policy.allows(format) {
        return Err(PictureRefusal::UnsupportedFormat(format));
    }
    let (width, height) = image::io::Reader::with_format(Cursor::new(bytes), format)
        .into_dimensions()
        .map_err(|e| PictureRefusal::Image(e.to_string()))?;
    check_dimensions(width, height, policy)?;
    Ok(Admitted {
        format,
        width,
        height,
    })
}

/// The refusal for a file the WebP decoder would read: "a RIFF file" only when the bytes start
/// with `RIFF` and are not `RIFF....WEBP` (a WAV, an AVI, a SoundFont); otherwise the WebP
/// sentence (the bytes say WebP, or the file's name does).
fn webp_refusal(head: &[u8]) -> PictureRefusal {
    if head.starts_with(b"RIFF") && head.get(8..12) != Some(b"WEBP".as_slice()) {
        PictureRefusal::RiffNotAPicture
    } else {
        PictureRefusal::WebP
    }
}

/// Whether `head` is the start of a WebP file (`RIFF....WEBP`).
fn is_webp(head: &[u8]) -> bool {
    head.starts_with(b"RIFF") && head.get(8..12) == Some(b"WEBP".as_slice())
}

/// The CLI's whole route for a picture file ([`Policy::CLI`]; lead ruling L-80): `369938d`'s
/// `image::open`, step for step, except that WebP is refused before any WebP decoder exists.
///
/// 1. `File::open` (the operating system's words on failure, as `image::open` gave them). A folder
///    is refused ([`PictureRefusal::NotARegularFile`]); `369938d` refused it too, in other words.
/// 2. The format is the file EXTENSION's (`ImageFormat::from_path`, the `image` crate's words on
///    failure), exactly as `image::open` chose it. The bytes are not sniffed: a file named for one
///    format and holding another is refused by that format's decoder, as at `369938d`.
/// 3. WebP: a file named `.webp` is refused before any WebP decoder exists; and a REGULAR file of
///    any name whose first bytes are `RIFF....WEBP` is refused as WebP (operator Q1 "a"). For that
///    one check the first [`RIFF_HEAD_BYTES`] bytes of a regular file are read and the handle is
///    rewound to the start. Nothing is read ahead from a pipe or device, so the decoder sees exactly
///    the stream `369938d`'s decoder saw.
/// 4. The decode: `image::io::Reader::with_format(BufReader::new(file), format)` with the policy's
///    limits, which for [`Policy::CLI`] are the `image` crate's defaults. That is the same
///    `load_inner(BufReader<File>, Limits::default(), format)` that `image::open` ran, on the same
///    stream: a picture `369938d` decoded decodes to the same pixels, and a file `369938d` refused
///    is refused with the same words (its default allocation limit is what refused every small
///    file whose header claims a huge picture).
pub fn open_and_decode(path: &Path, policy: &Policy) -> Result<DynamicImage, PictureRefusal> {
    let io = |e: std::io::Error| PictureRefusal::Io(e.to_string());
    let mut file = std::fs::File::open(path).map_err(io)?;
    let meta = file.metadata().map_err(io)?;
    if meta.is_dir() {
        return Err(PictureRefusal::NotARegularFile);
    }
    let format = ImageFormat::from_path(path).map_err(|e| PictureRefusal::Image(e.to_string()))?;
    if format == ImageFormat::WebP || meta.is_file() {
        let mut head = Vec::new();
        (&mut file)
            .take(RIFF_HEAD_BYTES)
            .read_to_end(&mut head)
            .map_err(io)?;
        if format == ImageFormat::WebP {
            return Err(webp_refusal(&head));
        }
        if is_webp(&head) {
            return Err(PictureRefusal::WebP);
        }
        file.seek(SeekFrom::Start(0)).map_err(io)?;
    }
    if !policy.allows(format) {
        return Err(PictureRefusal::UnsupportedFormat(format));
    }
    let mut reader = image::io::Reader::with_format(BufReader::new(file), format);
    reader.limits(policy.decode_limits());
    reader
        .decode()
        .map_err(|e| PictureRefusal::Image(e.to_string()))
}

/// Decode admitted bytes with the policy's limits, in the admitted format, then refuse if the
/// decoded size is not the header's (SH5 A-C5).
pub fn decode(
    bytes: &[u8],
    admitted: &Admitted,
    policy: &Policy,
) -> Result<DynamicImage, PictureRefusal> {
    let mut reader = image::io::Reader::with_format(Cursor::new(bytes), admitted.format);
    reader.limits(policy.decode_limits());
    let decoded = reader
        .decode()
        .map_err(|e| PictureRefusal::Image(e.to_string()))?;
    let (width, height) = (decoded.width(), decoded.height());
    if (width, height) != (admitted.width, admitted.height) {
        return Err(PictureRefusal::SizeMismatch {
            header_width: admitted.width,
            header_height: admitted.height,
            width,
            height,
        });
    }
    Ok(decoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grouping_and_mib() {
        assert_eq!(group(0), "0");
        assert_eq!(group(999), "999");
        assert_eq!(group(1000), "1,000");
        assert_eq!(group(400_000_000), "400,000,000");
        assert_eq!(mib(MAX_IMAGE_BYTES), "256 MiB");
        assert_eq!(mib(1000), "1,000 bytes");
    }

    #[test]
    fn dimension_order_is_floor_then_side_then_pixels() {
        let floored = Policy {
            min_side: 16,
            ..Policy::WINDOW
        };
        assert!(matches!(
            check_dimensions(10, 40_000, &floored),
            Err(PictureRefusal::TooSmall { .. })
        ));
        assert!(matches!(
            check_dimensions(10, 40_000, &Policy::WINDOW),
            Err(PictureRefusal::SideTooLarge { .. })
        ));
        assert!(matches!(
            check_dimensions(7072, 7072, &Policy::WINDOW),
            Err(PictureRefusal::TooManyPixels { .. })
        ));
        assert!(check_dimensions(8160, 6120, &Policy::WINDOW).is_ok());
        assert!(check_dimensions(1, 1, &Policy::WINDOW).is_ok());
    }

    #[test]
    fn the_cli_policy_has_no_size_check() {
        for (w, h) in [
            (1, 1),
            (100_000, 100_000),
            (u32::MAX, u32::MAX),
            (1, u32::MAX),
        ] {
            assert!(check_dimensions(w, h, &Policy::CLI).is_ok(), "{w}x{h}");
        }
    }

    /// L-80: the CLI decodes with EXACTLY the `image` crate's default limits, the ones
    /// `image::open` used at `369938d` (512 MiB `max_alloc`, no side limit). If the crate's default
    /// or [`DECODE_MAX_ALLOC`] ever moves, this fails.
    #[test]
    fn the_cli_decodes_with_the_image_crates_default_limits() {
        assert_eq!(Policy::CLI.decode_limits(), image::io::Limits::default());
        let l = Policy::CLI.decode_limits();
        assert_eq!(
            (l.max_image_width, l.max_image_height, l.max_alloc),
            (None, None, Some(512 * 1024 * 1024))
        );
    }

    /// key A SHOULD-2 (`s125-06A`): "a RIFF file" only for bytes that start with `RIFF`.
    #[test]
    fn a_riff_refusal_needs_riff_bytes() {
        assert_eq!(webp_refusal(b"RIFF\0\0\0\0WEBPVP8L"), PictureRefusal::WebP);
        assert_eq!(
            webp_refusal(b"RIFF\x04\0\0\0WAVE"),
            PictureRefusal::RiffNotAPicture
        );
        assert_eq!(webp_refusal(b"RIFF"), PictureRefusal::RiffNotAPicture);
        assert_eq!(
            webp_refusal(b"plain text, named x.webp"),
            PictureRefusal::WebP
        );
        assert_eq!(webp_refusal(b""), PictureRefusal::WebP);
        assert!(is_webp(b"RIFF\0\0\0\0WEBP"));
        assert!(!is_webp(b"RIFF\0\0\0\0WAVE"));
        assert!(!is_webp(b"RIFF\0\0\0\0WEB"));
    }

    #[test]
    fn webp_is_refused_by_every_policy_and_only_the_window_has_a_format_list() {
        for p in [Policy::CLI, Policy::WINDOW] {
            assert!(!p.allows(ImageFormat::WebP));
        }
        for f in [
            ImageFormat::Tga,
            ImageFormat::Qoi,
            ImageFormat::Pnm,
            ImageFormat::OpenExr,
            ImageFormat::Ico,
            ImageFormat::Hdr,
            ImageFormat::Dds,
            ImageFormat::Farbfeld,
        ] {
            assert!(Policy::CLI.allows(f), "{f:?}");
            assert!(!Policy::WINDOW.allows(f), "{f:?}");
        }
    }
}

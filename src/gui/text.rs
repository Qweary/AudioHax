//! Words the window shows: display sanitising (SH N-1), the suggested WAV name (SH V-1), and the
//! plain-language error wording (UI `s123-UI` § 7, SH B-4).
//!
//! Every error the user sees is ONE plain sentence saying what happened and what to do; the raw
//! engine text goes under a "Details" disclosure, never in the headline.

use super::ingest::{Refusal, MAX_PIXELS, MAX_SIDE};
use crate::pure_analysis::AnalysisError;
use crate::render::RenderError;

/// N-1: the most characters of a MIDI port name shown in a label.
pub const PORT_NAME_MAX: usize = 64;
/// N-1: the most characters of a file name shown in a label (the window title included).
pub const FILE_NAME_MAX: usize = 80;
/// V-1: the most characters of a suggested file stem.
pub const SUGGESTED_STEM_MAX: usize = 100;

/// Is `c` a control or bidi-override character that must never be drawn as-is (SH N-1)?
fn is_unsafe_char(c: char) -> bool {
    c.is_control() // C0, DEL and C1
        || matches!(
            c,
            '\u{061C}' | '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}'
        )
}

/// N-1: replace control and bidi characters with U+FFFD and truncate to `max` characters with an
/// ellipsis. Ordinary names come back unchanged, byte for byte. Show the full sanitised string
/// ([`sanitize_full`]) in a hover tooltip.
pub fn sanitize(s: &str, max: usize) -> String {
    let clean = sanitize_full(s);
    if clean.chars().count() <= max {
        clean
    } else {
        let mut out: String = clean.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

/// N-1 without truncation (for tooltips). Still capped at 4096 characters so a hostile 10 KB
/// name cannot blow up a tooltip's layout.
pub fn sanitize_full(s: &str) -> String {
    s.chars()
        .take(4096)
        .map(|c| if is_unsafe_char(c) { '\u{FFFD}' } else { c })
        .collect()
}

/// V-1: the suggested name for Save WAV, built from the picture's file STEM only: path
/// separators, `:`, control and bidi characters removed, at most [`SUGGESTED_STEM_MAX`]
/// characters, then `.wav`. No picture name (the sample, a dropped buffer) or an empty stem ⇒
/// `audiohax.wav`.
pub fn suggest_wav_name(image_name: Option<&str>) -> String {
    let stem = image_name
        .map(|n| {
            // Take the last path component by hand so BOTH separators count on every OS.
            let last = n.rsplit(['/', '\\']).next().unwrap_or("");
            match last.rfind('.') {
                Some(0) | None => last.to_string(),
                Some(i) => last[..i].to_string(),
            }
        })
        .unwrap_or_default();
    let cleaned: String = stem
        .chars()
        .filter(|c| !is_unsafe_char(*c) && !matches!(c, '/' | '\\' | ':'))
        .take(SUGGESTED_STEM_MAX)
        .collect();
    let cleaned = cleaned.trim().trim_matches('.').to_string();
    if cleaned.is_empty() {
        "audiohax.wav".to_string()
    } else {
        format!("{cleaned}.wav")
    }
}

/// Every user-facing failure the window can report, one row of UI § 7 each.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// The picture is over the pixel cap.
    TooManyPixels,
    /// A side is over the side cap.
    SideTooLarge,
    /// The picture is too small to make music from (side cap, or `ZeroBars`).
    TooSmall,
    /// Not a picture / damaged / unreadable format.
    NotAPicture,
    /// A picture type AudioHax does not open.
    UnsupportedType,
    /// The file is too large to read, or is not a regular file, or could not be read.
    CannotRead,
    /// More voices than the picture can supply (`EmptyImage` from the analyzer).
    TooManyVoices,
    /// The SoundFont failed to load or synthesise.
    SoundBank,
    /// No audio output device.
    NoSpeakers,
    /// No MIDI ports.
    NoMidiPorts,
    /// The chosen MIDI port is gone.
    MidiGone,
    /// The WAV could not be written.
    SaveFailed,
    /// The Open dialog returned at once with no file: probably no dialog could be shown
    /// (`s123-SH2` N-2; heuristic, see `app::DIALOG_FAILURE_THRESHOLD`).
    NoFileDialog,
    /// `--loop` with non-unity Level.
    LoopNeedsUnity,
    /// `--loop` cannot fold.
    LoopRingsTooLong,
    /// Expression detail without Expression.
    ExpressionFirst,
    /// The engine panicked; the panic was caught.
    Crashed,
    /// Anything else (raw text in Details).
    Other,
}

/// A failure, ready to show: what kind, and the raw engine text for "Details".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserError {
    /// Which row of UI § 7.
    pub kind: ErrorKind,
    /// The raw text, verbatim (developer prose is fine here; it is behind "Details").
    pub details: String,
}

impl UserError {
    /// Build one.
    pub fn new(kind: ErrorKind, details: impl Into<String>) -> Self {
        UserError {
            kind,
            details: details.into(),
        }
    }

    /// The one plain sentence for the headline.
    pub fn headline(&self) -> String {
        headline(self.kind)
    }
}

/// UI § 7: the headline for each kind. The numbers come from the security review's caps
/// (`s123-SH` B-1, B-2).
pub fn headline(kind: ErrorKind) -> String {
    match kind {
        ErrorKind::TooManyPixels => format!(
            "This image is too large to use — the limit is about {} megapixels. Try a smaller \
             version.",
            MAX_PIXELS / 1_000_000
        ),
        ErrorKind::SideTooLarge => format!(
            "This image is too big in one direction — the limit is {MAX_SIDE}×{MAX_SIDE} pixels. \
             Resize it and try again."
        ),
        ErrorKind::TooSmall => {
            "This picture is too small to turn into music. Open a larger one.".to_string()
        }
        ErrorKind::NotAPicture => "AudioHax couldn't read this file as a picture. It may be \
             damaged, or in a format AudioHax can't open — PNG and JPEG work best."
            .to_string(),
        ErrorKind::UnsupportedType => "AudioHax can't open this kind of picture. PNG, JPEG, GIF, \
             BMP and TIFF work."
            .to_string(),
        ErrorKind::CannotRead => "AudioHax couldn't open that file. Choose a picture file from \
             your computer (not a folder), under 256 MB."
            .to_string(),
        ErrorKind::TooManyVoices => "You've asked for more voices than this picture can supply. \
             Lower the Voices setting, or open a larger picture."
            .to_string(),
        ErrorKind::SoundBank => "AudioHax couldn't load its sound bank, so it couldn't make the \
             sound for this piece."
            .to_string(),
        ErrorKind::NoSpeakers => "AudioHax couldn't reach your speakers or sound card. You can \
             still save a WAV file instead."
            .to_string(),
        ErrorKind::NoMidiPorts => "No MIDI destinations were found. Choose 'This computer's \
             speakers', or set up a virtual MIDI port and try again."
            .to_string(),
        ErrorKind::MidiGone => "That MIDI destination is no longer available. Pick another from \
             the list, or refresh."
            .to_string(),
        ErrorKind::SaveFailed => "AudioHax couldn't save the file there. Check you can write to \
             that folder, or choose another location."
            .to_string(),
        // `s123-SH2` N-2, the reviewer's wording verbatim.
        ErrorKind::NoFileDialog => "AudioHax couldn't open a file dialog on this desktop. You can \
             drag a picture onto the window instead."
            .to_string(),
        ErrorKind::LoopNeedsUnity => {
            "Seamless loop needs the Level at 1.0. Set Level to 1.0 and try again.".to_string()
        }
        ErrorKind::LoopRingsTooLong => "This sound rings out too long to loop seamlessly. Try \
             turning off reverb or using a shorter setting."
            .to_string(),
        ErrorKind::ExpressionFirst => {
            "Turn Expression on first — Expression detail builds on it.".to_string()
        }
        ErrorKind::Crashed => "This picture couldn't be turned into music — something went wrong \
             inside AudioHax. The window is still fine; try another picture."
            .to_string(),
        ErrorKind::Other => "Something went wrong. The details below say what.".to_string(),
    }
}

/// B-4: a refusal from [`super::ingest`] as a user error.
pub fn from_refusal(r: &Refusal) -> UserError {
    let kind = match r {
        Refusal::NotARegularFile | Refusal::FileTooBig { .. } | Refusal::Io(_) => {
            ErrorKind::CannotRead
        }
        Refusal::NotAnImage(_) | Refusal::DisplayDecode(_) => ErrorKind::NotAPicture,
        Refusal::UnsupportedFormat(_) => ErrorKind::UnsupportedType,
        Refusal::TooSmall { .. } => ErrorKind::TooSmall,
        Refusal::SideTooLarge { .. } => ErrorKind::SideTooLarge,
        Refusal::TooManyPixels { .. } => ErrorKind::TooManyPixels,
    };
    UserError::new(kind, r.to_string())
}

/// B-4: a library error as a user error. The raw `Display` text is kept verbatim as details.
pub fn from_render_error(e: &RenderError) -> UserError {
    let raw = e.to_string();
    let kind = match e {
        RenderError::Analysis(AnalysisError::EmptyImage(_)) => ErrorKind::TooManyVoices,
        RenderError::Analysis(AnalysisError::ZeroBars) => ErrorKind::TooSmall,
        RenderError::Analysis(AnalysisError::Decode(_)) => ErrorKind::NotAPicture,
        RenderError::Synth(_) => ErrorKind::SoundBank,
        RenderError::Refused(s) if s.starts_with("--loop requires unity") => {
            ErrorKind::LoopNeedsUnity
        }
        RenderError::Refused(s) if s.starts_with("--loop:") => ErrorKind::LoopRingsTooLong,
        RenderError::Refused(s) if s.starts_with("--v2 requires") => ErrorKind::ExpressionFirst,
        RenderError::Refused(_) | RenderError::Dump(_) | RenderError::Io(_) => ErrorKind::Other,
    };
    UserError::new(kind, raw)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t20_sanitize_bounds_and_replaces() {
        let long = "x".repeat(10_000);
        let s = sanitize(&long, PORT_NAME_MAX);
        assert_eq!(s.chars().count(), PORT_NAME_MAX);
        assert!(s.ends_with('…'));

        let bidi = "evil\u{202E}gnp.exe";
        assert_eq!(sanitize(bidi, 80), "evil\u{FFFD}gnp.exe");

        let ctl = "a\nb\tc\x07d\u{0085}e";
        assert_eq!(sanitize(ctl, 80), "a\u{FFFD}b\u{FFFD}c\u{FFFD}d\u{FFFD}e");

        for ordinary in [
            "Midi Through:Midi Through Port-0 14:0",
            "Qsynth1",
            "Ünïcödé 音",
        ] {
            assert_eq!(sanitize(ordinary, PORT_NAME_MAX), ordinary);
        }
        for c in [
            '\u{061C}', '\u{200E}', '\u{200F}', '\u{202A}', '\u{2066}', '\u{2069}',
        ] {
            assert_eq!(sanitize(&c.to_string(), 8), "\u{FFFD}");
        }
    }

    #[test]
    fn tooltip_form_is_capped_too() {
        assert_eq!(sanitize_full(&"y".repeat(10_000)).chars().count(), 4096);
    }

    #[test]
    fn t19_suggested_names() {
        let n = suggest_wav_name(Some("../../a\u{202E}gnp.exe.jpg"));
        assert_eq!(n, "agnp.exe.wav");
        assert!(!n.contains('/') && !n.contains('\\'));

        let long = format!("{}.png", "z".repeat(300));
        let n = suggest_wav_name(Some(&long));
        assert_eq!(n.len(), SUGGESTED_STEM_MAX + 4);
        assert!(n.ends_with(".wav"));

        assert_eq!(suggest_wav_name(Some("")), "audiohax.wav");
        // A dot-file name: its stem is the whole name (as `Path::file_stem`), and the leading
        // dot is trimmed so the suggestion is never a hidden file.
        assert_eq!(suggest_wav_name(Some(".png")), "png.wav");
        assert_eq!(suggest_wav_name(Some("...")), "audiohax.wav");
        assert_eq!(suggest_wav_name(None), "audiohax.wav");
        assert_eq!(
            suggest_wav_name(Some("C:\\pics\\sunset.jpeg")),
            "sunset.wav"
        );
        assert_eq!(
            suggest_wav_name(Some("/srv/pictures/My Photo.png")),
            "My Photo.wav"
        );
        assert_eq!(suggest_wav_name(Some("a:b.png")), "ab.wav");
    }

    #[test]
    fn every_kind_has_a_plain_one_line_headline() {
        use ErrorKind::*;
        for k in [
            TooManyPixels,
            SideTooLarge,
            TooSmall,
            NotAPicture,
            UnsupportedType,
            CannotRead,
            TooManyVoices,
            SoundBank,
            NoSpeakers,
            NoMidiPorts,
            MidiGone,
            SaveFailed,
            NoFileDialog,
            LoopNeedsUnity,
            LoopRingsTooLong,
            ExpressionFirst,
            Crashed,
            Other,
        ] {
            let h = headline(k);
            assert!(!h.is_empty());
            assert!(!h.contains('\n'));
            // No developer prose in a headline (UI L4 / SH B-4).
            for jargon in ["analyze_section_pure", "EmptyImage", "ZeroBars", "--", "S1"] {
                assert!(!h.contains(jargon), "{k:?} headline has {jargon:?}: {h}");
            }
        }
    }

    #[test]
    fn the_caps_in_the_headlines_are_the_ingestion_caps() {
        assert!(headline(ErrorKind::TooManyPixels).contains("about 50 megapixels"));
        assert!(headline(ErrorKind::SideTooLarge).contains("32768×32768"));
    }

    #[test]
    fn the_formats_the_window_names_are_the_formats_it_opens() {
        // L-16 / UI2 F10: WebP left the allowlist, so no sentence may still offer it, and every
        // format the refusal sentence names must really be accepted.
        use super::super::ingest::allowed_format;
        use image::ImageFormat;
        let h = headline(ErrorKind::UnsupportedType);
        assert!(!h.to_lowercase().contains("webp"), "{h}");
        for (word, format) in [
            ("PNG", ImageFormat::Png),
            ("JPEG", ImageFormat::Jpeg),
            ("GIF", ImageFormat::Gif),
            ("BMP", ImageFormat::Bmp),
            ("TIFF", ImageFormat::Tiff),
        ] {
            assert!(h.contains(word), "{word} missing from: {h}");
            assert!(allowed_format(format), "{word} named but not accepted");
        }
        assert!(!allowed_format(ImageFormat::WebP));
        // The Open dialog's filter offers no WebP either.
        assert!(!super::super::app::OPEN_DIALOG_EXTENSIONS.contains(&"webp"));
    }

    #[test]
    fn library_errors_map_to_the_ui_rows_and_keep_the_raw_text() {
        let e = RenderError::Analysis(AnalysisError::EmptyImage("analyze_section_pure"));
        let u = from_render_error(&e);
        assert_eq!(u.kind, ErrorKind::TooManyVoices);
        assert_eq!(u.details, "empty image passed to analyze_section_pure");

        let e = RenderError::Analysis(AnalysisError::Decode("bad".into()));
        assert_eq!(from_render_error(&e).kind, ErrorKind::NotAPicture);

        let req = crate::render::RenderRequest {
            seed: None,
            loop_cue: true,
            expression: crate::cli::Toggle::Off,
            v2: None,
            pipeline: crate::cli::PipelineArgs::default(),
            reverb: crate::cli::Toggle::On,
            gain: 2.0,
            event_dump: None,
        };
        let e = req.validate().expect_err("loop at gain 2 is refused");
        assert_eq!(from_render_error(&e).kind, ErrorKind::LoopNeedsUnity);
    }

    #[test]
    fn refusals_map_to_the_ui_rows() {
        assert_eq!(
            from_refusal(&Refusal::TooSmall {
                width: 1,
                height: 1
            })
            .kind,
            ErrorKind::TooSmall
        );
        assert_eq!(
            from_refusal(&Refusal::TooManyPixels {
                width: 9000,
                height: 9000,
                pixels: 81_000_000
            })
            .kind,
            ErrorKind::TooManyPixels
        );
        assert_eq!(
            from_refusal(&Refusal::NotARegularFile).kind,
            ErrorKind::CannotRead
        );
    }
}

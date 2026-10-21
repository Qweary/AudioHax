//! S123 Test Engineer (Specialist 5): display-string sanitising (Shielding `s123-SH` N-1) and
//! the suggested Save-WAV name (V-1), at the edges the builder's T19/T20 do not touch: the exact
//! truncation boundary, character-vs-byte counting, the WHOLE C0/C1 and bidi lists, characters
//! that must NOT be touched, and names that sanitise to nothing.
#![cfg(feature = "gui")]

use audiohax::gui::text::{
    sanitize, sanitize_full, suggest_wav_name, FILE_NAME_MAX, PORT_NAME_MAX, SUGGESTED_STEM_MAX,
};

const FFFD: char = '\u{FFFD}';

/// N-1's list: C0 (incl. DEL), C1, and the bidi controls U+061C, U+200E, U+200F,
/// U+202A–U+202E, U+2066–U+2069.
fn n1_listed() -> Vec<char> {
    let mut v: Vec<char> = (0u32..0x20)
        .chain([0x7F])
        .chain(0x80..0xA0)
        .filter_map(char::from_u32)
        .collect();
    v.extend(['\u{061C}', '\u{200E}', '\u{200F}']);
    v.extend('\u{202A}'..='\u{202E}');
    v.extend('\u{2066}'..='\u{2069}');
    v
}

/// A name of EXACTLY the limit is shown whole; one character more is cut to the limit with an
/// ellipsis as its last character. (The builder tests only a 10,000-character name, which
/// cannot tell `<=` from `<`.)
#[test]
fn a_name_exactly_at_the_limit_is_shown_whole() {
    for max in [PORT_NAME_MAX, FILE_NAME_MAX] {
        let exact = "p".repeat(max);
        assert_eq!(sanitize(&exact, max), exact, "max={max}");
        let over = "p".repeat(max + 1);
        let shown = sanitize(&over, max);
        assert_eq!(shown.chars().count(), max, "max={max}");
        assert!(shown.ends_with('…'));
        assert_eq!(&shown[..max - 1], &over[..max - 1]);
    }
}

/// Limits count CHARACTERS: 64 CJK characters (192 bytes) is within the 64-character port
/// limit, and a longer CJK name is cut on a character boundary.
#[test]
fn limits_count_characters_not_bytes() {
    let exact = "音".repeat(PORT_NAME_MAX);
    assert_eq!(exact.len(), 3 * PORT_NAME_MAX);
    assert_eq!(sanitize(&exact, PORT_NAME_MAX), exact);
    let over = "音".repeat(PORT_NAME_MAX * 3);
    let shown = sanitize(&over, PORT_NAME_MAX);
    assert_eq!(shown.chars().count(), PORT_NAME_MAX);
    assert!(shown.starts_with("音音音"));
}

/// Every character on N-1's list is replaced, wherever it sits in a name.
#[test]
fn every_listed_control_and_bidi_character_is_replaced() {
    for c in n1_listed() {
        let name = format!("ab{c}cd");
        let want = format!("ab{FFFD}cd");
        assert_eq!(sanitize(&name, 80), want, "U+{:04X}", c as u32);
        assert_eq!(sanitize_full(&name), want, "U+{:04X}", c as u32);
    }
}

/// N-1 says "ordinary names come back unchanged, byte for byte". The characters NEXT TO each
/// listed range are ordinary and must survive: e.g. U+202F (narrow no-break space, used in
/// French text), U+2065 (unassigned), U+061B (Arabic semicolon), U+00A0 (no-break space),
/// U+2060 (word joiner) and a ZWJ emoji sequence. An over-wide range would eat them.
#[test]
fn neighbours_of_the_listed_ranges_are_left_alone() {
    // Sanity anchor: a listed character IS replaced (so "unchanged" below is not vacuous).
    assert_eq!(sanitize("\u{001F}", 8), FFFD.to_string());
    for name in [
        "Port\u{202F}A",
        "x\u{2065}y",
        "\u{061B}x",
        "a\u{00A0}b",
        "a\u{2060}b",
        "Synth \u{1F469}\u{200D}\u{1F4BB}",
        "\u{200D}\u{200C}",
        "~ ¡ ÿ",
    ] {
        assert_eq!(sanitize(name, PORT_NAME_MAX), name, "{name:?}");
    }
}

/// V-1: a picture whose stem sanitises to NOTHING (only bidi/control characters, or separators)
/// gets the default name; controls inside a stem are dropped, not kept.
#[test]
fn suggested_name_from_a_hostile_stem() {
    assert_eq!(
        suggest_wav_name(Some("\u{202E}\u{2066}.png")),
        "audiohax.wav"
    );
    assert_eq!(
        suggest_wav_name(Some("\u{0007}\u{0085}.jpg")),
        "audiohax.wav"
    );
    assert_eq!(suggest_wav_name(Some("dir/:.png")), "audiohax.wav");
    assert_eq!(suggest_wav_name(Some("line\nbreak.png")), "linebreak.wav");
    let n = suggest_wav_name(Some("ev\u{202E}il\u{200F}.gnp.jpg"));
    assert_eq!(n, "evil.gnp.wav");
    for c in n1_listed() {
        let n = suggest_wav_name(Some(&format!("a{c}b.png")));
        assert_eq!(n, "ab.wav", "U+{:04X}", c as u32);
    }
}

/// V-1's 100 is a CHARACTER limit on the stem: exactly 100 survives whole, 101 is cut to 100,
/// and a long multi-byte stem is cut on a character boundary.
#[test]
fn suggested_stem_limit_is_exact_and_counts_characters() {
    let s100 = "s".repeat(SUGGESTED_STEM_MAX);
    assert_eq!(
        suggest_wav_name(Some(&format!("{s100}.png"))),
        format!("{s100}.wav")
    );
    let s101 = "s".repeat(SUGGESTED_STEM_MAX + 1);
    assert_eq!(
        suggest_wav_name(Some(&format!("{s101}.png"))),
        format!("{s100}.wav")
    );
    let cjk = "音".repeat(300);
    let n = suggest_wav_name(Some(&format!("{cjk}.png")));
    let stem = n.strip_suffix(".wav").expect(".wav");
    assert_eq!(stem.chars().count(), SUGGESTED_STEM_MAX);
    assert!(stem.chars().all(|c| c == '音'));
}

/// N-1 WIRING: the window title the running window sends for a picture whose file name carries
/// a right-to-left override is the SANITISED name (the builder's tests check `sanitize` itself,
/// not that the title goes through it). Headless: the title leaves each frame as a
/// `ViewportCommand::Title` in the frame's output.
#[test]
fn window_title_shows_the_sanitised_file_name() {
    use audiohax::gui::app::AudioHaxApp;
    use audiohax::gui::state::Phase;
    use eframe::egui;
    use std::time::{Duration, Instant};

    let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("gui-text-s123")
        .join(format!("title-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch");
    let raw_name = "holiday\u{202E}gnp.exe.png";
    let path = dir.join(raw_name);
    let img = image::RgbImage::from_fn(48, 32, |x, y| {
        image::Rgb([(x * 5) as u8, (y * 7) as u8, 90])
    });
    image::DynamicImage::ImageRgb8(img)
        .save_with_format(&path, image::ImageFormat::Png)
        .expect("write png");

    let ctx = egui::Context::default();
    let mut app = AudioHaxApp::new_headless(&ctx);
    let mut titles: Vec<String> = Vec::new();
    let mut frame = |app: &mut AudioHaxApp| {
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| app.draw(ui));
        out.textures_delta.clear();
        for vo in out.viewport_output.values() {
            for cmd in &vo.commands {
                if let egui::ViewportCommand::Title(t) = cmd {
                    titles.push(t.clone());
                }
            }
        }
    };
    frame(&mut app);
    app.open_path(path);
    let t0 = Instant::now();
    while app.phase() != Phase::ImageLoaded {
        frame(&mut app);
        assert!(
            t0.elapsed() < Duration::from_secs(60),
            "load timed out in {:?}",
            app.phase()
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    frame(&mut app);
    let title = titles.last().expect("a Title command was sent").clone();
    assert_eq!(title, "AudioHax — holiday\u{FFFD}gnp.exe.png");
    assert!(titles.iter().all(|t| !t.contains('\u{202E}')), "{titles:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

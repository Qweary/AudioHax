//! S125 ITEM 3: the window's half of "Shrink picture" that a public-API test can reach — the
//! verbatim strings (UI4 SK4), the floor constant shared with the window's admission floor, the
//! suggested Save WAV name (UI4 § 5.6), and admission (SK19: a shrink opens no new way in). The
//! interaction tests (notice, stale hint, lock, reset on open, overlay, Save WAV bytes) are in
//! `src/gui/app.rs`'s test module, which can drive the window headlessly.
#![cfg(feature = "gui")]

use audiohax::gui::app::{stale_hint, STALE_BOTH_HINT, STALE_PRESET_HINT, STALE_SHRINK_HINT};
use audiohax::gui::ingest::MIN_SIDE;
use audiohax::gui::job::GuardedImage;
use audiohax::gui::text::{
    picture_readout, shrink_caption, shrink_closed_text, shrink_notice_headline, shrink_row_line,
    suggest_wav_name, suggest_wav_name_shrunk, SHRINK_DISABLED_CAPTION, SHRINK_HOVER, SHRINK_LABEL,
    SHRINK_NOTICE_BODY, SHRINK_WAIT_POINTER, SUGGESTED_STEM_MAX,
};
use audiohax::shrink::{Shrink, SHRINK_MIN_SIDE};
use image::{ImageFormat, Rgb, RgbImage};
use std::io::Cursor;

/// UI4 § 4: one floor; the shrink floor and the window's admission floor cannot drift apart.
#[test]
fn the_shrink_floor_is_the_windows_floor() {
    assert_eq!(SHRINK_MIN_SIDE, MIN_SIDE);
}

/// SK4: every window string, character for character as UI4 §§ 2, 5 write it.
#[test]
fn sk4_window_strings_are_verbatim() {
    assert_eq!(SHRINK_LABEL, "Shrink picture");
    assert_eq!(
        SHRINK_HOVER,
        "Make the music from a smaller copy of the picture. A smaller picture means a shorter \
         wait, and different music."
    );
    assert_eq!(SHRINK_DISABLED_CAPTION, "Too small to shrink this far.");
    assert_eq!(
        SHRINK_NOTICE_BODY,
        "Less detail means different music: this piece won't match the full-size one. Notes, \
         tempo and length can all change."
    );
    assert_eq!(
        SHRINK_WAIT_POINTER,
        "Shrink picture (above the picture) shortens the wait next time. It changes the music."
    );
    assert_eq!(STALE_PRESET_HINT, "Re-render to hear the new preset.");
    assert_eq!(
        STALE_SHRINK_HINT,
        "Re-render to hear the music from the chosen picture size."
    );
    assert_eq!(STALE_BOTH_HINT, "Re-render to hear your changes.");
    // The four rows for UI4's example picture, first lines and captions.
    let rows: Vec<(String, &str)> = Shrink::ALL
        .iter()
        .map(|s| (shrink_row_line(*s, 6000, 4000), shrink_caption(*s)))
        .collect();
    assert_eq!(
        rows,
        vec![
            (
                "Full size · 6000 × 4000".to_string(),
                "The whole picture. The longest wait."
            ),
            (
                "75% · 4500 × 3000".to_string(),
                "A shorter wait. Different music."
            ),
            (
                "50% · 3000 × 2000".to_string(),
                "A much shorter wait. Different music."
            ),
            (
                "25% · 1500 × 1000".to_string(),
                "The shortest wait. Different music."
            ),
        ]
    );
    let closed: Vec<&str> = Shrink::ALL.iter().map(|s| shrink_closed_text(*s)).collect();
    assert_eq!(closed, vec!["Full size", "75%", "50%", "25%"]);
    assert_eq!(
        shrink_notice_headline(Shrink::Percent50, 6000, 4000).as_deref(),
        Some("Shrunk to 50% (3000 × 2000).")
    );
    assert_eq!(shrink_notice_headline(Shrink::Full, 6000, 4000), None);
    assert_eq!(
        picture_readout(Some("marina-sunset.jpg"), 6000, 4000),
        "marina-sunset.jpg · 6000 × 4000"
    );
    assert_eq!(picture_readout(None, 6000, 4000), "6000 × 4000");
    assert_eq!(picture_readout(Some("  "), 64, 48), "64 × 48");
    // `×` is U+00D7 and `·` is U+00B7, each with a space either side.
    assert!(shrink_row_line(Shrink::Percent75, 40, 30).contains(" \u{00B7} 30 \u{00D7} 23"));
    // No string says faster, speed, small change, or a number of seconds (SK4, H4).
    let mut all: Vec<String> = vec![
        SHRINK_LABEL.into(),
        SHRINK_HOVER.into(),
        SHRINK_DISABLED_CAPTION.into(),
        SHRINK_NOTICE_BODY.into(),
        SHRINK_WAIT_POINTER.into(),
        STALE_SHRINK_HINT.into(),
        STALE_BOTH_HINT.into(),
    ];
    for s in Shrink::ALL {
        all.push(shrink_caption(s).into());
        all.push(shrink_row_line(s, 6000, 4000));
        all.extend(shrink_notice_headline(s, 6000, 4000));
    }
    for s in &all {
        let lower = s.to_lowercase();
        for banned in ["faster", "speed", "small change", "second", " s "] {
            assert!(!lower.contains(banned), "{s:?} contains {banned:?}");
        }
    }
}

/// UI4 § 5.3: which hint shows for what differs.
#[test]
fn stale_hint_picks_by_what_differs() {
    assert_eq!(stale_hint(false, false), None);
    assert_eq!(stale_hint(true, false), Some(STALE_PRESET_HINT));
    assert_eq!(stale_hint(false, true), Some(STALE_SHRINK_HINT));
    assert_eq!(stale_hint(true, true), Some(STALE_BOTH_HINT));
}

/// UI4 § 5.6 / SK15: `-shrunk-<pct>` after the cleaned, capped stem; unchanged at Full size.
#[test]
fn suggested_wav_names_carry_the_shrink() {
    assert_eq!(
        suggest_wav_name_shrunk(Some("marina-sunset.jpg"), Shrink::Percent50),
        "marina-sunset-shrunk-50.wav"
    );
    assert_eq!(
        suggest_wav_name_shrunk(None, Shrink::Percent50),
        "audiohax-shrunk-50.wav"
    );
    assert_eq!(
        suggest_wav_name_shrunk(Some("a.b.png"), Shrink::Percent25),
        "a.b-shrunk-25.wav"
    );
    for name in [Some("marina-sunset.jpg"), None, Some("dir/:.png")] {
        assert_eq!(
            suggest_wav_name_shrunk(name, Shrink::Full),
            suggest_wav_name(name)
        );
    }
    // The cap still bounds the user-derived part; the suffix comes after it.
    let long = format!("{}.png", "x".repeat(500));
    let n = suggest_wav_name_shrunk(Some(&long), Shrink::Percent75);
    assert_eq!(
        n,
        format!("{}-shrunk-75.wav", "x".repeat(SUGGESTED_STEM_MAX))
    );
}

fn png(w: u32, h: u32) -> Vec<u8> {
    let img = RgbImage::from_pixel(w, h, Rgb([10, 120, 230]));
    let mut out = Cursor::new(Vec::new());
    img.write_to(&mut out, ImageFormat::Png).expect("png");
    out.into_inner()
}

/// SK19 / SH5 C-3: the shrink is a property of an ADMITTED picture (`GuardedImage::with_shrink`
/// exists only on one), so a picture the window refuses today is still refused, whatever the
/// shrink; and a shrunk handle keeps the original picture's probe, which is what every cap reads.
#[test]
fn sk19_admission_is_the_original_pictures() {
    // Too small to open (the window's floor) — no shrink can be asked for it.
    assert!(GuardedImage::admit(png(15, 40)).is_err());
    // A side over the window's cap (32,768) — refused at the header, as before.
    assert!(GuardedImage::admit(png(32_769, 16)).is_err());
    // Admitted: the probe stays the original's at every level.
    let g = GuardedImage::admit(png(64, 48)).expect("admit");
    for s in Shrink::ALL {
        let shrunk = g.clone().with_shrink(s);
        assert_eq!(shrunk.probe(), g.probe());
        assert_eq!(shrunk.bytes(), g.bytes());
        assert_eq!(shrunk.shrink(), s);
    }
}

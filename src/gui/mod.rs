//! src/gui/ — the AudioHax desktop window (S123, egui/eframe, behind the `gui` feature).
//!
//! The window is a FRONT-END over [`crate::render`]: it builds a [`RenderRequest`] from four
//! controls, hands the guarded picture bytes to `render()` / `prepare_play()` on ONE worker
//! thread, and draws the picture with the scan-bar overlay. It adds no music logic of its own.
//!
//! Layout of this module (every piece that can be tested without a display is a plain function
//! outside the egui drawing code):
//!
//! * [`ingest`] — the picture's way in: bounded read, header probe, dimension caps, format
//!   allowlist, display decode (Shielding `s123-SH` B-1, B-2, B-3, D-1).
//! * [`job`] — the single worker thread, `catch_unwind` around every job, generation counter,
//!   at most one job alive (SH T-1, T-2, T-3, C-1..C-3, W-1, W-2).
//! * [`sinks`] — the live sinks the window plays through, each silencing itself on Stop and on
//!   Drop (lead rulings L-3, L-11), and MIDI port enumeration (L-11, SH N-2).
//! * [`state`] — the six interaction states and the button-enable matrix (UI `s123-UI` § 3), the
//!   Voices bound derived from the picture (L-14), and the four Everyday controls read from
//!   [`crate::options_schema`].
//! * [`overlay`] — the scan-bar geometry, mirrored exactly from the analyzer, and the
//!   three-case playhead (UI § 4).
//! * [`text`] — display sanitising, the suggested WAV name, and the plain-language error
//!   wording (UI § 7, SH N-1, V-1, B-4).
//! * [`save`] — the atomic WAV write and the save-dialog defaults (SH V-1..V-4).
//! * [`theme`] — the operator palette, dark default and light optional (UI § 10.2, D-41).
//! * [`app`] — the eframe application: panels, menus, transport, overlay painting.
//!
//! Self-contained by construction: the mappings and the sample picture are compiled in
//! ([`EMBEDDED_MAPPINGS_JSON`], [`SAMPLE_IMAGE`]), and the SoundFont is the bundled one
//! ([`gui_font_source`]), so a double-clicked binary reads nothing from its working directory
//! (lead ruling L-8, SH M-1).

pub mod app;
pub mod ingest;
pub mod job;
pub mod overlay;
pub mod save;
pub mod sinks;
pub mod state;
pub mod text;
pub mod theme;

use crate::mapping_loader::{parse_mappings, MappingTable};
use crate::synth_sink::SoundFontSource;

/// `assets/mappings.json`, compiled in (lead ruling L-8). The GUI performs NO working-directory
/// read of the mappings (SH M-1); the file in the repository stays the single source of truth, so
/// a GUI render equals a CLI render of the same inputs.
pub const EMBEDDED_MAPPINGS_JSON: &str = include_str!("../../assets/mappings.json");

/// The "Try a sample" picture: `assets/images/example.jpg` (the CLI's own default image),
/// compiled in so it works from any directory.
pub const SAMPLE_IMAGE: &[u8] = include_bytes!("../../assets/images/example.jpg");

/// The display name used for the sample picture.
pub const SAMPLE_NAME: &str = "example.jpg (sample)";

/// Parse the compiled-in mappings with the library's own parser. A failure here is a build
/// defect (the shipped file does not parse), caught by a unit test.
pub fn embedded_mappings() -> Result<MappingTable, serde_json::Error> {
    parse_mappings(EMBEDDED_MAPPINGS_JSON)
}

/// The SoundFont every GUI render and every GUI live-synth play uses: the bundled one, and only
/// the bundled one (lead ruling L-13, SH S-1). This is the ONE place the GUI chooses a font, so
/// the GUI's font choice is tested here (S122 `s122-06P1` B-1: a front-end dropping or swapping
/// the font is invisible to forcing test 2, so the GUI path is pinned by its own test).
pub fn gui_font_source() -> SoundFontSource<'static> {
    SoundFontSource::Bundled
}

/// Run the window. Blocks until the window closes. Called from `src/bin/audiohax-gui.rs`.
pub fn run() -> Result<(), eframe::Error> {
    job::install_panic_hook();
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("AudioHax")
            // Fits a 1920×1080 screen at 133 % scaling with room for the title bar and a task
            // bar (measured: 1280×800 at 1.33 pushed the transport below the screen edge).
            .with_inner_size([1120.0, 720.0])
            .with_min_inner_size([960.0, 600.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    // `audiohax-gui <picture>` opens that picture at start ("Open with…" from a file manager).
    // It goes through the same bounded read and checks as File ▸ Open.
    let initial = std::env::args_os().nth(1).map(std::path::PathBuf::from);
    eframe::run_native(
        "AudioHax",
        options,
        Box::new(move |cc| {
            let mut window = app::AudioHaxApp::new(&cc.egui_ctx);
            if let Some(path) = initial {
                window.open_path(path);
            }
            Ok(Box::new(window))
        }),
    )
}

/// Test-only scratch directory: next to the test executable, i.e. inside the cargo target dir
/// (never `/tmp`, never inside the repository).
#[cfg(test)]
pub(crate) fn test_scratch_dir(name: &str) -> std::path::PathBuf {
    let base = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(std::env::temp_dir);
    let dir = base
        .join("audiohax-gui-test-scratch")
        .join(format!("{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_mappings_parse_with_the_library_parser() {
        // T16: the GUI's mappings source is `parse_mappings(include_str!(…))`, with no
        // filesystem access at run time. A parse failure would be a build defect.
        let table = embedded_mappings();
        assert!(
            table.is_ok(),
            "embedded mappings.json failed to parse: {table:?}"
        );
    }

    #[test]
    fn embedded_mappings_are_the_repository_file_byte_for_byte() {
        // The compiled-in text IS the repo file (read here only by the TEST, from the crate
        // root, never by the GUI at run time).
        let on_disk =
            std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mappings.json"))
                .expect("test reads the repo copy");
        assert_eq!(EMBEDDED_MAPPINGS_JSON, on_disk);
    }

    #[test]
    fn gui_font_is_the_bundled_soundfont_only() {
        // B-1 route chosen: TEST the GUI path's font choice (no change to src/render.rs).
        assert!(matches!(gui_font_source(), SoundFontSource::Bundled));
    }

    #[test]
    fn sample_picture_is_compiled_in_and_is_a_jpeg() {
        assert!(SAMPLE_IMAGE.len() > 1000);
        assert_eq!(&SAMPLE_IMAGE[..2], &[0xFF, 0xD8], "JPEG SOI marker");
    }
}

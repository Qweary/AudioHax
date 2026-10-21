// src/bin/audiohax-gui.rs — the AudioHax desktop window (S123).
//
// Built only with `--features gui` (`required-features` in Cargo.toml). Everything lives in
// `audiohax::gui`; this file only starts it. On Windows a release build is a GUI-subsystem
// program (no console window when double-clicked); panics are therefore recorded by the GUI's
// own panic hook and shown in the window (Shielding s123-SH C-3).
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

fn main() -> anyhow::Result<()> {
    audiohax::gui::run().map_err(|e| anyhow::anyhow!("the AudioHax window could not start: {e}"))
}

//! S124 Test Engineer (Specialist 5): the window WRITES its per-viewer settings with
//! `std::fs::write` (`app.rs` `write_setting`), on the UI thread. If a FIFO sits at a settings path
//! (`gui-more-open`, `gui-preset`) the write blocks in `open()` until a reader appears, so a click on
//! "More" freezes the window. The READ side is bounded (metadata check first); the write side is
//! not. Needs a FIFO in the user's own config dir, so LOW. One test per file (it sets
//! `XDG_CONFIG_HOME` for the process).
#![cfg(all(feature = "gui", target_os = "linux"))]

use audiohax::gui::app::AudioHaxApp;
use eframe::egui;
use egui::accesskit::{Action, ActionRequest, Role, TreeId};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

#[test]
fn f5_a_fifo_at_a_settings_path_does_not_freeze_a_click_on_more() {
    let xdg = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("s124-fifo-w-{}", std::process::id()));
    std::fs::create_dir_all(&xdg).expect("xdg");
    std::env::set_var("XDG_CONFIG_HOME", &xdg);
    let dir = directories::ProjectDirs::from("", "", "AudioHax")
        .expect("dirs")
        .config_dir()
        .to_path_buf();
    std::fs::create_dir_all(&dir).expect("cfg");
    std::fs::write(dir.join("gui-tour-seen"), "1\n").expect("tour");
    let fifo = dir.join("gui-more-open");
    let _ = std::fs::remove_file(&fifo);
    assert!(std::process::Command::new("mkfifo")
        .arg(&fifo)
        .status()
        .expect("mkfifo")
        .success());
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut app = AudioHaxApp::new(&ctx);
        let mut run = |events: Vec<egui::Event>| {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::Vec2::new(1280.0, 800.0),
                )),
                events,
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| app.draw(ui));
            out.textures_delta.clear();
            out.platform_output
                .accesskit_update
                .map(|u| u.nodes)
                .unwrap_or_default()
        };
        run(vec![]);
        let nodes = run(vec![]);
        let more = nodes
            .iter()
            .find(|(_, n)| n.role() == Role::Button && n.label() == Some("More"))
            .expect("More")
            .0;
        run(vec![egui::Event::AccessKitActionRequest(ActionRequest {
            action: Action::Click,
            target_tree: TreeId::ROOT,
            target_node: more,
            data: None,
        })]);
        let _ = tx.send(());
    });
    let done = rx.recv_timeout(Duration::from_secs(15));
    if done.is_err() {
        // Unblock the stuck writer so the process can exit.
        let _ = std::fs::read(&fifo);
    }
    assert!(
        done.is_ok(),
        "a click on More blocked for 15 s writing to a FIFO settings file"
    );
}

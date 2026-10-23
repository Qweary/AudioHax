//! S124 Test Engineer (Specialist 5): what the window REMEMBERS across launches (`s124-UI` § 1.3,
//! § 2.4; AD2 second half) and that the remembered state is bounded input (a hostile file in the
//! config dir cannot hang or redirect the window).
//!
//! `AudioHaxApp::new` reads and writes the per-viewer config dir (`directories::ProjectDirs`, which
//! on Linux honours `XDG_CONFIG_HOME`). This file points `XDG_CONFIG_HOME` at a private scratch dir
//! — process-wide state, which is why this file holds exactly ONE test (cargo runs each test file
//! as its own process). It never touches the real user's config.
#![cfg(all(feature = "gui", target_os = "linux"))]

use audiohax::gui::app::AudioHaxApp;
use eframe::egui;
use egui::accesskit::{Action, ActionRequest, Node, NodeId, Role, TreeId};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

fn frame(
    ctx: &egui::Context,
    app: &mut AudioHaxApp,
    events: Vec<egui::Event>,
) -> Vec<(NodeId, Node)> {
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
}

fn has_slider(nodes: &[(NodeId, Node)], label: &str) -> bool {
    nodes
        .iter()
        .any(|(_, n)| n.role() == Role::Slider && n.label() == Some(label))
}

/// Launch a window with `AudioHaxApp::new` (the persisting constructor) on its own thread, with a
/// 20 s guard (a FIFO in the config dir must not hang it), draw two frames, optionally click
/// "More", and report (preset id, Scan-width-slider-drawn-on-first-frames).
fn launch(click_more: bool) -> (String, bool) {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut app = AudioHaxApp::new(&ctx);
        frame(&ctx, &mut app, vec![]);
        let nodes = frame(&ctx, &mut app, vec![]);
        let open = has_slider(&nodes, "Scan width");
        if click_more {
            let more = nodes
                .iter()
                .find(|(_, n)| n.role() == Role::Button && n.label() == Some("More"))
                .expect("More header")
                .0;
            let ev = egui::Event::AccessKitActionRequest(ActionRequest {
                action: Action::Click,
                target_tree: TreeId::ROOT,
                target_node: more,
                data: None,
            });
            frame(&ctx, &mut app, vec![ev]);
            frame(&ctx, &mut app, vec![]);
        }
        let _ = tx.send((app.preset_id().to_string(), open));
    });
    rx.recv_timeout(Duration::from_secs(20))
        .expect("the window did not finish launching within 20 s (blocked on a config file?)")
}

#[test]
fn more_state_and_preset_persist_and_hostile_config_files_are_ignored() {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let xdg = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("s124-persist-{}-{nanos}", std::process::id()));
    std::fs::create_dir_all(&xdg).expect("xdg");
    std::env::set_var("XDG_CONFIG_HOME", &xdg);
    let dir = directories::ProjectDirs::from("", "", "AudioHax")
        .expect("dirs")
        .config_dir()
        .to_path_buf();
    assert!(
        dir.starts_with(&xdg),
        "the config dir is the scratch dir: {}",
        dir.display()
    );
    std::fs::create_dir_all(&dir).expect("cfg");
    std::fs::write(dir.join("gui-tour-seen"), "1\n").expect("skip the tour");

    // AD2: a fresh config is CLOSED; a click opens it; the next launch starts OPEN; a click closes
    // it; the launch after that starts CLOSED.
    let (id, open) = launch(true);
    assert_eq!(id, "default", "P2: first launch selects the default preset");
    assert!(!open, "fresh config: More closed");
    let (_, open) = launch(true);
    assert!(open, "More's open state survived the restart");
    let (_, open) = launch(false);
    assert!(!open, "More's closed state survived the restart");

    // § 1.3: a stored preset id that is no longer shipped falls back to the default, no error.
    std::fs::write(dir.join("gui-preset"), "layer-reversed\n").expect("write");
    let (id, _) = launch(false);
    assert_eq!(id, "default");

    // Bounded input: an over-long "open" file is ignored (the reader takes ≤ 64 B regular files),
    // and a FIFO at either settings path neither hangs the launch nor opens anything.
    std::fs::write(
        dir.join("gui-more-open"),
        format!("open{}", " ".repeat(100)),
    )
    .expect("write");
    let (_, open) = launch(false);
    assert!(!open, "an over-long settings file is not read");
    for name in ["gui-preset", "gui-more-open"] {
        let p = dir.join(name);
        std::fs::remove_file(&p).expect("rm");
        assert!(std::process::Command::new("mkfifo")
            .arg(&p)
            .status()
            .expect("mkfifo")
            .success());
    }
    let (id, open) = launch(false);
    assert_eq!(id, "default");
    assert!(!open);
}

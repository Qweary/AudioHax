//! S123 fix round (Rust Implementer): the window behaviours the Test Engineer could not reach
//! headlessly (`s123-05` TODO-2 … TODO-5), lead ruling L-20 as drawn, and the UI2 F1 focus ring.
//!
//! Like the Test Engineer's files, these run the real window with egui's `Context::run_ui`,
//! accesskit on and no display, and act the way a screen reader does (accesskit `Click`, `Focus`,
//! `SetValue`). Three thin `#[doc(hidden)]` seams make the rest reachable without a native dialog
//! or a MIDI port: `save_chosen_path` (the path the Save dialog would have returned),
//! `select_midi_output` (the Output selector's choice) and `scan_bar_shown` (read-only).
//!
//! Files are written under `CARGO_TARGET_TMPDIR` (inside the cargo target dir), never `/tmp` and
//! never the repository.
#![cfg(feature = "gui")]

use audiohax::gui::app::AudioHaxApp;
use audiohax::gui::save::window_confirms_overwrite;
use audiohax::gui::state::{Controls, Phase, MIDI_ROOM_LEVEL_NOTICE};
use audiohax::gui::text::ErrorKind;
use audiohax::gui::theme::Palette;
use audiohax::render::{render, ImageInput};
use audiohax::synth_sink::{write_stereo_wav, SoundFontSource};
use eframe::egui;
use egui::accesskit::{Action, ActionData, ActionRequest, Node, NodeId, Role, TreeId};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

// ── harness ─────────────────────────────────────────────────────────────────────────────────

struct Harness {
    ctx: egui::Context,
    app: AudioHaxApp,
    pending: Vec<egui::Event>,
    shapes: Vec<egui::epaint::ClippedShape>,
}

impl Harness {
    fn new() -> Self {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let app = AudioHaxApp::new_headless(&ctx);
        let mut h = Harness {
            ctx,
            app,
            pending: Vec::new(),
            shapes: Vec::new(),
        };
        h.frame();
        h
    }

    /// One frame at 1280×800; returns the accesskit nodes and keeps the painted shapes.
    fn frame(&mut self) -> Vec<(NodeId, Node)> {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::Vec2::new(1280.0, 800.0),
            )),
            events: std::mem::take(&mut self.pending),
            ..Default::default()
        };
        let app = &mut self.app;
        let mut out = self.ctx.run_ui(input, |ui| app.draw(ui));
        out.textures_delta.clear(); // no renderer
        self.shapes = out.shapes;
        out.platform_output
            .accesskit_update
            .map(|u| u.nodes)
            .unwrap_or_default()
    }

    /// Frames until `what` holds for the phase, then one more for a fresh tree.
    fn wait_for(&mut self, what: impl Fn(Phase) -> bool) -> Vec<(NodeId, Node)> {
        let t0 = Instant::now();
        loop {
            self.frame();
            if what(self.app.phase()) {
                return self.frame();
            }
            assert!(
                t0.elapsed() < Duration::from_secs(120),
                "timed out in {:?}",
                self.app.phase()
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn action(&mut self, id: NodeId, action: Action, data: Option<ActionData>) {
        self.pending
            .push(egui::Event::AccessKitActionRequest(ActionRequest {
                action,
                target_tree: TreeId::ROOT,
                target_node: id,
                data,
            }));
    }

    fn key(&mut self, key: egui::Key) {
        self.pending.push(egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        });
    }

    fn open(&mut self, path: PathBuf) -> Vec<(NodeId, Node)> {
        self.app.open_path(path);
        self.wait_for(|p| p == Phase::ImageLoaded)
    }

    fn idle(&mut self) -> Vec<(NodeId, Node)> {
        self.wait_for(|p| matches!(p, Phase::ImageLoaded | Phase::Stopped))
    }
}

fn label(n: &Node) -> String {
    n.label().unwrap_or_default().to_string()
}

/// The single node with this role whose label satisfies `pick`.
fn one(nodes: &[(NodeId, Node)], role: Role, pick: impl Fn(&str) -> bool) -> &(NodeId, Node) {
    let hits: Vec<_> = nodes
        .iter()
        .filter(|(_, n)| n.role() == role && pick(&label(n)))
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "{role:?}: {} matches among {:?}",
        hits.len(),
        nodes
            .iter()
            .map(|(_, n)| (n.role(), label(n)))
            .collect::<Vec<_>>()
    );
    hits[0]
}

/// The text a node shows: buttons carry it as the accesskit label, plain labels as the value.
fn shown_text(n: &Node) -> String {
    match n.value() {
        Some(v) if label(n).is_empty() => v.to_string(),
        _ => label(n),
    }
}

fn any_label(nodes: &[(NodeId, Node)], text: &str) -> bool {
    nodes.iter().any(|(_, n)| shown_text(n) == text)
}

fn scratch(name: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("gui-fix-round-s123")
        .join(format!("{name}-{}", std::process::id()));
    std::fs::create_dir_all(&d).expect("scratch dir");
    d
}

/// A small structured PNG (gradient + checker), written to `dir`.
fn picture_file(dir: &Path) -> (PathBuf, Vec<u8>) {
    let (w, h) = (96u32, 64u32);
    let img = image::ImageBuffer::from_fn(w, h, |x, y| {
        let block = ((x / 8) + (y / 8)) % 2 == 0;
        image::Rgb([
            ((x * 255) / w) as u8,
            ((y * 255) / h) as u8,
            if block { 220 } else { 30 },
        ])
    });
    let mut out = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(img)
        .write_to(&mut out, image::ImageFormat::Png)
        .expect("encode");
    let bytes = out.into_inner();
    let path = dir.join("picture.png");
    std::fs::write(&path, &bytes).expect("write picture");
    (path, bytes)
}

/// The WAV bytes the library itself makes for `controls` (independent of the window).
fn library_wav(controls: Controls, picture: &[u8], dir: &Path, name: &str) -> Vec<u8> {
    let m = audiohax::gui::embedded_mappings().expect("mappings");
    let out = render(
        &controls.request(),
        ImageInput::Bytes(picture),
        &m,
        SoundFontSource::Bundled,
        &mut |_| Ok(()),
    )
    .expect("library render");
    let p = dir.join(name);
    write_stereo_wav(&p, out.sample_rate, &out.interleaved).expect("library wav");
    std::fs::read(&p).expect("read library wav")
}

// ── L-20 / UI2 F3 / TODO-4: Room and Level under a MIDI output ─────────────────────────────

/// Room's "On"/"Off" and the Level slider, as drawn: (on live, off live, level live).
fn room_and_level(nodes: &[(NodeId, Node)]) -> (bool, bool, bool) {
    let live = |n: &(NodeId, Node)| !n.1.is_disabled();
    let on = nodes
        .iter()
        .find(|(_, n)| label(n) == "On")
        .expect("Room On");
    let off = nodes
        .iter()
        .find(|(_, n)| label(n) == "Off")
        .expect("Room Off");
    let level = one(nodes, Role::Slider, |l| l.starts_with("Level"));
    (live(on), live(off), live(level))
}

#[test]
fn l20_room_and_level_stay_live_under_midi_with_the_designers_notice() {
    let dir = scratch("l20-live");
    let mut h = Harness::new();
    let (path, _) = picture_file(&dir);
    let nodes = h.open(path);
    assert_eq!(room_and_level(&nodes), (true, true, true), "speakers");
    assert!(
        !any_label(&nodes, MIDI_ROOM_LEVEL_NOTICE),
        "no notice with the speakers"
    );

    h.app.select_midi_output(0, "Test synth".into());
    let nodes = h.frame();
    let nodes = if nodes.is_empty() { h.frame() } else { nodes };
    assert_eq!(
        room_and_level(&nodes),
        (true, true, true),
        "MIDI output: still live (L-20)"
    );
    assert!(
        any_label(&nodes, MIDI_ROOM_LEVEL_NOTICE),
        "the one-line notice, verbatim"
    );
    assert!(!nodes
        .iter()
        .any(|(_, n)| shown_text(n).contains("set by your MIDI synth while")));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn l20_save_wav_under_midi_uses_the_room_and_level_shown() {
    // L-20's ground, end to end: under a MIDI output, the Room and Level the user sets are what
    // Save WAV renders with.
    let dir = scratch("l20-save");
    let mut h = Harness::new();
    let (path, picture) = picture_file(&dir);
    h.open(path);
    h.app.select_midi_output(0, "Test synth".into());
    let nodes = h.frame();
    let off = nodes
        .iter()
        .find(|(_, n)| label(n) == "Off")
        .expect("Room Off")
        .0;
    let level = one(&nodes, Role::Slider, |l| l.starts_with("Level")).0;
    h.action(off, Action::Click, None);
    h.action(level, Action::SetValue, Some(ActionData::NumericValue(0.8)));
    h.frame();
    h.frame();
    let target = dir.join("under-midi");
    h.app.save_chosen_path(target.clone());
    h.frame();
    h.idle();
    assert!(h.app.shown_error().is_none(), "{:?}", h.app.shown_error());
    let saved = std::fs::read(target.with_extension("wav")).expect("saved, with .wav added");
    let want = Controls {
        reverb: false,
        gain: 0.8,
        ..Controls::default()
    };
    assert_eq!(
        saved,
        library_wav(want, &picture, &dir, "lib-off-0.8.wav"),
        "Save WAV = the library's render with Room Off and Level 0.8"
    );
    assert_ne!(
        saved,
        library_wav(Controls::default(), &picture, &dir, "lib-default.wav"),
        "control: the defaults give a different file"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// ── N-1 / TODO-3: a hostile MIDI port name in the output label ──────────────────────────────

#[test]
fn n1_a_hostile_midi_port_name_is_sanitised_in_the_output_label() {
    let mut h = Harness::new();
    let hostile = format!("Synth\u{202E}lave\u{0007}{}", "x".repeat(300));
    h.app.select_midi_output(7, hostile);
    let nodes = h.frame();
    let (_, node) = one(&nodes, Role::Button, |l| l.starts_with("Output: MIDI 7: "));
    let shown = label(node);
    let name = shown.trim_start_matches("Output: MIDI 7: ");
    assert!(
        !shown.contains('\u{202E}') && !shown.contains('\u{0007}'),
        "{shown:?}"
    );
    assert!(name.starts_with("Synth\u{FFFD}lave\u{FFFD}"), "{shown:?}");
    assert_eq!(name.chars().count(), audiohax::gui::text::PORT_NAME_MAX);
    assert!(name.ends_with('…'));
}

// ── A9 / TODO-5: the V key ──────────────────────────────────────────────────────────────────

#[test]
fn a9_v_toggles_the_scan_bar_only_when_nothing_else_has_the_keyboard() {
    let dir = scratch("a9-v");
    let mut h = Harness::new();
    let (path, _) = picture_file(&dir);
    let nodes = h.open(path);
    assert!(h.app.scan_bar_shown(), "on by default");
    h.key(egui::Key::V);
    h.frame();
    assert!(!h.app.scan_bar_shown(), "V hides it");
    h.key(egui::Key::V);
    h.frame();
    assert!(h.app.scan_bar_shown(), "V shows it again");

    // With the Variation field focused, V belongs to the field.
    let field = one(&nodes, Role::TextInput, |_| true).0;
    h.action(field, Action::Focus, None);
    h.frame();
    h.frame();
    h.key(egui::Key::V);
    h.frame();
    assert!(
        h.app.scan_bar_shown(),
        "V typed into a text field leaves the bar alone"
    );
    h.key(egui::Key::Escape);
    h.frame();

    // With a dialog open, V does nothing either.
    h.app.open_path(dir.join("missing.png"));
    h.wait_for(|p| p != Phase::Loading);
    assert!(h.app.shown_error().is_some(), "an error dialog is open");
    let before = h.app.scan_bar_shown();
    h.key(egui::Key::V);
    h.frame();
    assert_eq!(h.app.scan_bar_shown(), before, "V under a dialog");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a9_space_with_no_picture_does_nothing() {
    let mut h = Harness::new();
    h.key(egui::Key::Space);
    h.frame();
    h.frame();
    assert_eq!(h.app.phase(), Phase::NoImage);
    assert!(h.app.shown_error().is_none());
}

// ── UI2 F1: the focus ring ──────────────────────────────────────────────────────────────────

/// The focus rings painted this frame: 2 px `color.focus` strokes drawn OUTSIDE a rect.
fn focus_rings(shapes: &[egui::epaint::ClippedShape], p: &Palette) -> Vec<egui::Rect> {
    shapes
        .iter()
        .filter_map(|c| match &c.shape {
            egui::Shape::Rect(r)
                if r.stroke == p.focus_stroke() && r.stroke_kind == egui::StrokeKind::Outside =>
            {
                Some(r.rect)
            }
            _ => None,
        })
        .collect()
}

#[test]
fn f1_the_focus_ring_is_drawn_outside_the_focused_play_button() {
    let dir = scratch("f1");
    let mut h = Harness::new();
    let (path, _) = picture_file(&dir);
    let nodes = h.open(path);
    let p = Palette::DARK;
    assert!(
        focus_rings(&h.shapes, &p).is_empty(),
        "no ring before focus"
    );
    let (play_id, play) = one(&nodes, Role::Button, |l| l.starts_with("▶"));
    let play_id = *play_id;
    let bounds = play.bounds().expect("Play has bounds");
    h.action(play_id, Action::Focus, None);
    h.frame();
    h.frame();
    let rings = focus_rings(&h.shapes, &p);
    assert_eq!(rings.len(), 1, "exactly one ring: {rings:?}");
    let ring = rings[0];
    // Outside the button by the offset, on every side (accesskit bounds are in points here).
    let o = f64::from(Palette::FOCUS_RING_OFFSET);
    assert!(
        (f64::from(ring.min.x) - (bounds.x0 - o)).abs() < 0.51,
        "{ring:?} vs {bounds:?}"
    );
    assert!(
        (f64::from(ring.max.x) - (bounds.x1 + o)).abs() < 0.51,
        "{ring:?} vs {bounds:?}"
    );
    assert!(
        (f64::from(ring.min.y) - (bounds.y0 - o)).abs() < 0.51,
        "{ring:?} vs {bounds:?}"
    );
    assert!(
        (f64::from(ring.max.y) - (bounds.y1 + o)).abs() < 0.51,
        "{ring:?} vs {bounds:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// ── TODO-2: Save WAV's decisions, after the dialog ──────────────────────────────────────────

#[test]
fn save_after_changing_the_controls_saves_the_new_piece_not_the_last_render() {
    let dir = scratch("save-rerender");
    let mut h = Harness::new();
    let (path, picture) = picture_file(&dir);
    let nodes = h.open(path);
    let render_id = one(&nodes, Role::Button, |l| l == "Render").0;
    h.action(render_id, Action::Click, None);
    h.frame();
    assert_eq!(h.app.phase(), Phase::Rendering);
    let nodes = h.idle();
    assert!(h.app.rendered_audio().is_some());
    // Change Voices 4 → 3 after the render.
    let voices = one(&nodes, Role::Slider, |l| l.starts_with("Voices")).0;
    h.action(
        voices,
        Action::SetValue,
        Some(ActionData::NumericValue(3.0)),
    );
    h.frame();
    h.frame();
    let target = dir.join("after-change.wav");
    h.app.save_chosen_path(target.clone());
    h.frame();
    assert_eq!(
        h.app.phase(),
        Phase::Rendering,
        "the changed piece is rendered first"
    );
    h.idle();
    let saved = std::fs::read(&target).expect("saved");
    let three = Controls {
        instruments: 3,
        ..Controls::default()
    };
    assert_eq!(saved, library_wav(three, &picture, &dir, "lib-3.wav"));
    assert_ne!(
        saved,
        library_wav(Controls::default(), &picture, &dir, "lib-4.wav"),
        "control: the stale 4-voice render would differ"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn v2_an_existing_file_is_replaced_only_after_the_window_asks() {
    let dir = scratch("save-v2");
    let mut h = Harness::new();
    let (path, picture) = picture_file(&dir);
    h.open(path);
    let target = dir.join("existing.wav");
    std::fs::write(&target, b"the user's previous file").expect("seed");
    h.app.save_chosen_path(target.clone());
    let nodes = h.frame();
    if window_confirms_overwrite() {
        // Linux / BSD: the window asks; nothing is written until Replace.
        h.frame();
        assert_eq!(
            std::fs::read(&target).expect("read"),
            b"the user's previous file",
            "not replaced before the answer"
        );
        let nodes = if any_label(&nodes, "Replace") {
            nodes
        } else {
            h.frame()
        };
        let replace = one(&nodes, Role::Button, |l| l == "Replace").0;
        h.action(replace, Action::Click, None);
        h.frame();
    }
    h.idle();
    assert_eq!(
        std::fs::read(&target).expect("read"),
        library_wav(Controls::default(), &picture, &dir, "lib.wav"),
        "replaced with the piece"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn v4_saving_over_the_open_picture_is_refused() {
    let dir = scratch("save-v4");
    let mut h = Harness::new();
    let (path, picture) = picture_file(&dir);
    h.open(path.clone());
    h.app.save_chosen_path(path.clone());
    h.frame();
    let e = h.app.shown_error().expect("refused with a message");
    assert_eq!(e.kind, ErrorKind::SaveFailed);
    assert_eq!(h.app.phase(), Phase::ImageLoaded, "no job started");
    assert_eq!(
        std::fs::read(&path).expect("read"),
        picture,
        "picture untouched"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

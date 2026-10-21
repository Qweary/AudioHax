//! S123 Test Engineer (Specialist 5): the Voices bound (lead ruling L-14, UI H7) and the
//! button-enable matrix (UI `s123-UI` § 3) AS THE WINDOW DRAWS IT.
//!
//! The builder's `state::tests::matrix_matches_ui_section_3` checks the `enablement` function.
//! It cannot see whether the drawn widgets consult it. These tests run the real window headlessly
//! (egui `Context::run_ui`, accesskit on, no display) and read each control's disabled flag from
//! the accessibility tree, then press buttons the way a screen reader does (accesskit `Click`),
//! which egui honours only for ENABLED widgets.
#![cfg(feature = "gui")]

use audiohax::gui::app::AudioHaxApp;
use audiohax::gui::ingest::MIN_SIDE;
use audiohax::gui::state::{voices_max, Controls, EverydaySpecs, Phase};
use eframe::egui;
use egui::accesskit::{Action, ActionRequest, Node, NodeId, Role, TreeId};
use std::time::{Duration, Instant};

// ── the Voices bound ────────────────────────────────────────────────────────────────────────

/// L-14: Voices max = min(16, min(width, height)), whichever side is the smaller one, never 0.
/// (The builder's cases put the small side in `width` only, so `min(16, width)` also passes them.)
#[test]
fn voices_bound_takes_the_smaller_side_on_either_axis() {
    for (w, h, want) in [
        (900u32, 12u32, 12u32),
        (12, 900, 12),
        (15, 15, 15),
        (16, 16, 16),
        (17, 17, 16),
        (4000, 3000, 16),
        (1, 900, 1),
        (900, 1, 1),
    ] {
        assert_eq!(voices_max(16, w, h), want, "{w}x{h}");
    }
    assert_eq!(voices_max(16, 0, 0), 1, "never offers a 0-voice slider");
    assert_eq!(voices_max(0, 640, 480), 1);
    let mut c = Controls {
        instruments: 0,
        ..Controls::default()
    };
    c.clamp_voices(0);
    assert_eq!(c.instruments, 1);
    c.instruments = 40;
    c.clamp_voices(voices_max(16, 900, 12));
    assert_eq!(c.instruments, 12);
}

/// The coupling L-14 relies on: the schema's Voices maximum equals the admission floor, so for
/// EVERY admitted picture the derived bound is the full 16. If either number moves, the window's
/// per-picture clamp starts to matter and this test says so.
#[test]
fn schema_voices_max_equals_the_admission_floor() {
    let specs = EverydaySpecs::from_schema().expect("everyday rows");
    assert_eq!(specs.voices_schema_max(), 16);
    assert_eq!(specs.voices_schema_max(), MIN_SIDE);
    assert_eq!(
        voices_max(specs.voices_schema_max(), MIN_SIDE, MIN_SIDE),
        16
    );
}

// ── the window, headless ────────────────────────────────────────────────────────────────────

struct Harness {
    ctx: egui::Context,
    app: AudioHaxApp,
    pending: Vec<egui::Event>,
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
        };
        h.frame();
        h
    }

    /// One frame at 1280×800; returns the accesskit nodes.
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
        out.platform_output
            .accesskit_update
            .map(|u| u.nodes)
            .unwrap_or_default()
    }

    fn wait_for(&mut self, what: impl Fn(Phase) -> bool) -> Vec<(NodeId, Node)> {
        let t0 = Instant::now();
        loop {
            let nodes = self.frame();
            if what(self.app.phase()) {
                return nodes;
            }
            assert!(
                t0.elapsed() < Duration::from_secs(120),
                "timed out in {:?}",
                self.app.phase()
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn click(&mut self, id: NodeId) {
        self.pending
            .push(egui::Event::AccessKitActionRequest(ActionRequest {
                action: Action::Click,
                target_tree: TreeId::ROOT,
                target_node: id,
                data: None,
            }));
    }
}

/// The node for a control, by role and label prefix (exactly one must match).
fn find<'a>(nodes: &'a [(NodeId, Node)], role: Role, label: &str) -> &'a (NodeId, Node) {
    let hits: Vec<_> = nodes
        .iter()
        .filter(|(_, n)| n.role() == role && n.label().unwrap_or_default().starts_with(label))
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "{role:?} {label:?}: {} matches among {:?}",
        hits.len(),
        nodes
            .iter()
            .map(|(_, n)| (n.role(), n.label().unwrap_or_default().to_string()))
            .collect::<Vec<_>>()
    );
    hits[0]
}

/// Is each transport control enabled, as DRAWN?
fn transport(nodes: &[(NodeId, Node)]) -> [(&'static str, bool); 4] {
    let live = |role, label| !find(nodes, role, label).1.is_disabled();
    [
        ("Render", live(Role::Button, "Render")),
        ("Play", live(Role::Button, "▶")),
        ("Stop", live(Role::Button, "⏹ Stop")),
        ("Save WAV", live(Role::Button, "Save WAV")),
    ]
}

/// UI § 3, as drawn: no image ⇒ Render/Play/Stop/Save and the Voices slider are greyed (and a
/// press on Render does nothing); picture loaded ⇒ Render/Play/Save live, Stop greyed, Voices
/// live with max 16; rendering ⇒ all four and Voices greyed; after the render ⇒ back to the
/// loaded set, with the piece kept as data.
#[test]
fn transport_and_voices_enablement_as_drawn_follows_ui_section_3() {
    let mut h = Harness::new();
    let nodes = h.frame();
    assert_eq!(h.app.phase(), Phase::NoImage);
    assert_eq!(
        transport(&nodes),
        [
            ("Render", false),
            ("Play", false),
            ("Stop", false),
            ("Save WAV", false)
        ],
        "no image"
    );
    assert!(find(&nodes, Role::Slider, "Voices").1.is_disabled());
    // A press on a greyed Render is inert.
    let render_id = find(&nodes, Role::Button, "Render").0;
    h.click(render_id);
    h.frame();
    h.frame();
    assert_eq!(
        h.app.phase(),
        Phase::NoImage,
        "greyed Render must not start a job"
    );

    h.app.try_sample();
    let nodes = h.wait_for(|p| p == Phase::ImageLoaded);
    let nodes = if nodes.is_empty() { h.frame() } else { nodes };
    assert_eq!(
        transport(&nodes),
        [
            ("Render", true),
            ("Play", true),
            ("Stop", false),
            ("Save WAV", true)
        ],
        "picture loaded"
    );
    let voices = &find(&nodes, Role::Slider, "Voices").1;
    assert!(!voices.is_disabled());
    assert_eq!(
        voices.max_numeric_value(),
        Some(16.0),
        "Voices max for the sample"
    );

    // Press Render the way a screen reader does.
    let render_id = find(&nodes, Role::Button, "Render").0;
    h.click(render_id);
    h.frame();
    assert_eq!(h.app.phase(), Phase::Rendering, "Render started");
    let nodes = h.frame();
    // A render of the sample takes far longer than two frames; if it has already finished, the
    // drawn "rendering" state was never observed and the test must not pass silently.
    assert_eq!(h.app.phase(), Phase::Rendering, "rendering frame observed");
    {
        assert_eq!(
            transport(&nodes),
            [
                ("Render", false),
                ("Play", false),
                ("Stop", false),
                ("Save WAV", false)
            ],
            "rendering (Stop stays greyed: render() is not cancellable)"
        );
        assert!(find(&nodes, Role::Slider, "Voices").1.is_disabled());
    }
    let nodes = h.wait_for(|p| p == Phase::ImageLoaded);
    let nodes = if nodes.is_empty() { h.frame() } else { nodes };
    assert_eq!(
        transport(&nodes),
        [
            ("Render", true),
            ("Play", true),
            ("Stop", false),
            ("Save WAV", true)
        ],
        "after render"
    );
    assert!(
        h.app.rendered_audio().is_some(),
        "the piece is kept as data"
    );
}

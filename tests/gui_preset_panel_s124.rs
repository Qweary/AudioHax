//! S124 Test Engineer (Specialist 5): the preset panel, the "More" disclosure and the A6
//! "modified from preset" marker AS THE WINDOW DRAWS THEM (`s124-UI` § 1.7 P2/P6/P8, § 2.6
//! AD1/AD2/AD3/AD5).
//!
//! The implementer's `app.rs` tests set private fields and look for text. These run the real
//! window headlessly (egui `Context::run_ui`, accesskit on, no display) and drive it ONLY the way a
//! screen reader does: accesskit `Click` on buttons and `SetValue` on sliders (egui honours both
//! only for ENABLED widgets), reading each control's state back from the accessibility tree.
//! Controls are told apart by their accesskit `labelled_by` relation, not by tree order (the
//! update's node order is not the drawing order).
#![cfg(feature = "gui")]

use audiohax::gui::app::AudioHaxApp;
use audiohax::gui::state::Phase;
use eframe::egui;
use egui::accesskit::{Action, ActionData, ActionRequest, Node, NodeId, Role, TreeId};
use std::collections::HashMap;
use std::time::{Duration, Instant};

struct Harness {
    ctx: egui::Context,
    app: AudioHaxApp,
    pending: Vec<egui::Event>,
}

type Nodes = Vec<(NodeId, Node)>;

impl Harness {
    /// A fresh window (no config dir touched) with the bundled sample picture loaded.
    fn loaded() -> Self {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let app = AudioHaxApp::new_headless(&ctx);
        let mut h = Harness {
            ctx,
            app,
            pending: Vec::new(),
        };
        h.frame();
        h.app.try_sample();
        h.wait_for(|p| p == Phase::ImageLoaded);
        h
    }

    fn frame(&mut self) -> Nodes {
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
        out.textures_delta.clear();
        out.platform_output
            .accesskit_update
            .map(|u| u.nodes)
            .unwrap_or_default()
    }

    /// Two frames (the action lands in the first, its effect is drawn in the second); the nodes
    /// of the last.
    fn settle(&mut self) -> Nodes {
        self.frame();
        let n = self.frame();
        if n.is_empty() {
            self.frame()
        } else {
            n
        }
    }

    fn wait_for(&mut self, what: impl Fn(Phase) -> bool) -> Nodes {
        let t0 = Instant::now();
        loop {
            let nodes = self.frame();
            if what(self.app.phase()) {
                return nodes;
            }
            assert!(
                t0.elapsed() < Duration::from_secs(300),
                "timed out in {:?}",
                self.app.phase()
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn act(&mut self, id: NodeId, action: Action, data: Option<ActionData>) {
        self.pending
            .push(egui::Event::AccessKitActionRequest(ActionRequest {
                action,
                target_tree: TreeId::ROOT,
                target_node: id,
                data,
            }));
    }

    fn click(&mut self, id: NodeId) {
        self.act(id, Action::Click, None);
    }

    fn set_value(&mut self, id: NodeId, v: f64) {
        self.act(id, Action::SetValue, Some(ActionData::NumericValue(v)));
    }
}

/// A node's words: its label, else its value.
fn text(n: &Node) -> String {
    n.label()
        .map(|s| s.to_string())
        .or_else(|| n.value().map(|s| s.to_string()))
        .unwrap_or_default()
}

fn all<'a>(nodes: &'a Nodes, role: Role, words: &str) -> Vec<&'a (NodeId, Node)> {
    nodes
        .iter()
        .filter(|(_, n)| n.role() == role && text(n) == words)
        .collect()
}

fn one<'a>(nodes: &'a Nodes, role: Role, words: &str) -> &'a (NodeId, Node) {
    let hits = all(nodes, role, words);
    assert_eq!(
        hits.len(),
        1,
        "{role:?} {words:?}: {} matches; tree: {:?}",
        hits.len(),
        nodes
            .iter()
            .filter(|(_, n)| !text(n).is_empty())
            .map(|(_, n)| (n.role(), text(n)))
            .collect::<Vec<_>>()
    );
    hits[0]
}

fn any_text(nodes: &Nodes, needle: &str) -> bool {
    nodes.iter().any(|(_, n)| text(n).contains(needle))
}

/// The button `words` whose accesskit `labelled_by` is the Label reading `label`.
fn labelled<'a>(nodes: &'a Nodes, label: &str, words: &str) -> &'a (NodeId, Node) {
    let lid = one(nodes, Role::Label, label).0;
    let hits: Vec<_> = nodes
        .iter()
        .filter(|(_, n)| text(n) == words && n.labelled_by().contains(&lid))
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "{words:?} labelled by {label:?}: {}",
        hits.len()
    );
    hits[0]
}

/// The "●" marker drawn in the same row as the Label (or Slider label) `row`.
fn marker_for(nodes: &Nodes, row: &str) -> Option<NodeId> {
    let parent: HashMap<NodeId, NodeId> = nodes
        .iter()
        .flat_map(|(id, n)| n.children().iter().map(move |c| (*c, *id)))
        .collect();
    let row_label = nodes
        .iter()
        .find(|(_, n)| n.role() == Role::Label && text(n) == row)?
        .0;
    let row_parent = parent.get(&row_label)?;
    nodes
        .iter()
        .find(|(id, n)| text(n) == "●" && parent.get(id) == Some(row_parent))
        .map(|(id, _)| *id)
}

fn markers(nodes: &Nodes) -> usize {
    nodes.iter().filter(|(_, n)| text(n) == "●").count()
}

fn open_more(h: &mut Harness) -> Nodes {
    let nodes = h.settle();
    let more = one(&nodes, Role::Button, "More").0;
    h.click(more);
    h.settle()
}

// ── AD1 / AD2 (first half) ──────────────────────────────────────────────────────────────────

/// AD2: "More" is CLOSED on a fresh window (none of its rows drawn); one click opens it; a second
/// click closes it again. AD1: open, it holds Expression, Expression detail, Scan width, Scan
/// detail and Seamless loop — and no sound bank (L-31), no mappings loader (L-33), and no second
/// copy of an Everyday control.
#[test]
fn ad1_ad2_more_is_closed_fresh_and_holds_exactly_the_advanced_rows() {
    let mut h = Harness::loaded();
    let nodes = h.settle();
    for row in ["Scan width", "Scan detail"] {
        assert!(
            all(&nodes, Role::Slider, row).is_empty(),
            "{row} drawn while More is closed"
        );
    }
    for row in ["Expression detail", "Seamless loop"] {
        assert!(
            all(&nodes, Role::Label, row).is_empty(),
            "{row} drawn while More is closed"
        );
    }

    let nodes = open_more(&mut h);
    one(&nodes, Role::Slider, "Scan width");
    one(&nodes, Role::Slider, "Scan detail");
    for row in ["Expression", "Expression detail", "Seamless loop"] {
        one(&nodes, Role::Label, row);
    }
    for every in ["Voices", "Level"] {
        one(&nodes, Role::Slider, every);
    }
    for absent in [
        "sound bank",
        "Sound bank",
        ".sf2",
        "mappings",
        "Mappings",
        "Load your own",
    ] {
        assert!(
            !any_text(&nodes, absent),
            "{absent:?} must not be in the window"
        );
    }

    let more = one(&nodes, Role::Button, "More").0;
    h.click(more);
    let nodes = h.settle();
    assert!(
        all(&nodes, Role::Slider, "Scan width").is_empty(),
        "a second click closes More"
    );
}

// ── AD3 ─────────────────────────────────────────────────────────────────────────────────────

/// AD3, as drawn: the three Expression-detail terms are DISABLED (not hidden) with the reason
/// shown until Expression is On; turning it On (a screen-reader click) enables them and removes
/// the reason. Seamless loop's "On" is enabled at Level 1.0, disabled with its reason when Level
/// is moved to 0.5 (a slider SetValue), and enabled again when Level returns to 1.0.
#[test]
fn ad3_dependents_are_disabled_with_their_reason_until_the_condition_holds() {
    let mut h = Harness::loaded();
    let nodes = open_more(&mut h);
    let v2_reason = "Turn Expression on first — Expression detail builds on it.";
    for term in ["e3", "e2", "ta"] {
        assert!(
            labelled(&nodes, "Expression detail", term).1.is_disabled(),
            "{term} off"
        );
    }
    assert!(any_text(&nodes, v2_reason), "the reason is shown inline");

    let on = labelled(&nodes, "Expression", "On").0;
    h.click(on);
    let nodes = h.settle();
    for term in ["e3", "e2", "ta"] {
        assert!(
            !labelled(&nodes, "Expression detail", term).1.is_disabled(),
            "{term} live"
        );
    }
    assert!(
        !any_text(&nodes, v2_reason),
        "reason gone once Expression is on"
    );

    let loop_reason = "Seamless loop needs Level at 1.0.";
    assert!(!labelled(&nodes, "Seamless loop", "On").1.is_disabled());
    assert!(!any_text(&nodes, loop_reason));
    let level = one(&nodes, Role::Slider, "Level").0;
    h.set_value(level, 0.5);
    let nodes = h.settle();
    assert_eq!(
        one(&nodes, Role::Slider, "Level").1.numeric_value(),
        Some(0.5),
        "SetValue landed"
    );
    assert!(
        labelled(&nodes, "Seamless loop", "On").1.is_disabled(),
        "loop greyed off unity"
    );
    assert!(any_text(&nodes, loop_reason), "its reason is shown");
    // Re-read the id: the slider's id changes when its ● appears (finding F-6).
    let level = one(&nodes, Role::Slider, "Level").0;
    h.set_value(level, 1.0);
    let nodes = h.settle();
    let lv = one(&nodes, Role::Slider, "Level").1.numeric_value();
    assert!(
        !labelled(&nodes, "Seamless loop", "On").1.is_disabled(),
        "live again at 1.0 (Level reads {lv:?}; modified={})",
        any_text(&nodes, "(modified)")
    );
}

/// AD5: a disabled dependent "exposes its reason to accesskit as the control's description, so a
/// screen-reader user learns WHY". Today the reason is a separate visible label next to the
/// control; the disabled buttons themselves carry no accesskit description.
#[test]
fn f4_ad5_a_disabled_dependent_exposes_its_reason_as_its_description() {
    let mut h = Harness::loaded();
    let nodes = open_more(&mut h);
    let e3 = &labelled(&nodes, "Expression detail", "e3").1;
    assert!(e3.is_disabled());
    assert!(
        e3.description()
            .is_some_and(|d| d.contains("Turn Expression on first")),
        "description = {:?}",
        e3.description()
    );
}

// ── P8 / A6 ─────────────────────────────────────────────────────────────────────────────────

/// P8 / A6, as drawn and driven by accesskit only:
/// * fresh: no "(modified)", no "Reset to preset", no ● marker;
/// * Level set to 1.0 (its preset value) is NOT modified (the compare is numeric);
/// * Level set to 0.5 shows "(modified)", "Reset to preset" and exactly one ●, in Level's row;
/// * "Reset to preset" clears all three and puts Level back to 1.0;
/// * with Level AND Scan detail changed, clicking the ● in Scan detail's row resets only Scan
///   detail (Level stays 0.5 and the panel still reads "(modified)").
#[test]
fn p8_a6_modified_marker_reset_all_and_reset_one_as_drawn() {
    let mut h = Harness::loaded();
    let nodes = open_more(&mut h);
    assert!(!any_text(&nodes, "(modified)"));
    assert!(all(&nodes, Role::Button, "Reset to preset").is_empty());
    assert_eq!(markers(&nodes), 0);

    let level = one(&nodes, Role::Slider, "Level").0;
    h.set_value(level, 1.0);
    let nodes = h.settle();
    assert!(
        !any_text(&nodes, "(modified)"),
        "1.0 vs the preset's 1.0 is not modified"
    );

    h.set_value(level, 0.5);
    let nodes = h.settle();
    assert!(any_text(&nodes, "(modified)"));
    let reset = one(&nodes, Role::Button, "Reset to preset").0;
    assert_eq!(markers(&nodes), 1, "one ● for one changed control");
    assert!(
        marker_for(&nodes, "Level").is_some(),
        "the ● is in Level's row"
    );

    h.click(reset);
    let nodes = h.settle();
    assert!(
        !any_text(&nodes, "(modified)"),
        "Reset to preset clears (modified)"
    );
    assert!(
        all(&nodes, Role::Button, "Reset to preset").is_empty(),
        "and hides itself"
    );
    assert_eq!(markers(&nodes), 0);
    assert_eq!(
        one(&nodes, Role::Slider, "Level").1.numeric_value(),
        Some(1.0)
    );

    let level = one(&nodes, Role::Slider, "Level").0;
    let detail = one(&nodes, Role::Slider, "Scan detail").0;
    h.set_value(level, 0.5);
    h.set_value(detail, 9.0);
    let nodes = h.settle();
    assert_eq!(markers(&nodes), 2);
    let dot = marker_for(&nodes, "Scan detail").expect("a ● in Scan detail's row");
    h.click(dot);
    let nodes = h.settle();
    assert_eq!(
        one(&nodes, Role::Slider, "Scan detail").1.numeric_value(),
        Some(40.0)
    );
    assert_eq!(
        one(&nodes, Role::Slider, "Level").1.numeric_value(),
        Some(0.5)
    );
    assert_eq!(markers(&nodes), 1);
    assert!(any_text(&nodes, "(modified)"));
}

// ── P6 ──────────────────────────────────────────────────────────────────────────────────────

/// P6: during a render the preset pill, "Reset to preset" and the "More" controls are disabled
/// (the saved piece is what the controls showed); they come back when the render ends.
#[test]
fn p6_the_preset_panel_locks_while_rendering() {
    let mut h = Harness::loaded();
    let nodes = open_more(&mut h);
    let level = one(&nodes, Role::Slider, "Level").0;
    h.set_value(level, 0.5);
    let nodes = h.settle();
    assert!(
        !one(&nodes, Role::Button, "Default").1.is_disabled(),
        "live when idle"
    );
    assert!(!one(&nodes, Role::Button, "Reset to preset").1.is_disabled());

    let render = one(&nodes, Role::Button, "Render").0;
    h.click(render);
    h.frame();
    assert_eq!(h.app.phase(), Phase::Rendering, "Render started");
    let nodes = h.frame();
    assert_eq!(
        h.app.phase(),
        Phase::Rendering,
        "the rendering frame was observed"
    );
    assert!(
        one(&nodes, Role::Button, "Default").1.is_disabled(),
        "preset pill locked"
    );
    assert!(
        one(&nodes, Role::Button, "Reset to preset").1.is_disabled(),
        "reset locked"
    );
    assert!(
        one(&nodes, Role::Slider, "Scan detail").1.is_disabled(),
        "More locked"
    );

    h.wait_for(|p| p == Phase::ImageLoaded);
    let nodes = h.settle();
    assert!(
        !one(&nodes, Role::Button, "Default").1.is_disabled(),
        "unlocked after"
    );
}

// ── P2 ──────────────────────────────────────────────────────────────────────────────────────

/// P2: on a fresh window the default preset is selected and the untouched window's Render is the
/// library's render of the CLI's all-defaults request on the same picture with the embedded
/// mappings, sample for sample. CONTROL: the window rendered at Level 0.5 differs (so the
/// comparison can fail).
#[test]
fn p2_the_untouched_window_renders_the_cli_default_piece() {
    use audiohax::render::{render, ImageInput, RenderRequest};
    use audiohax::synth_sink::SoundFontSource;
    let (cli, _) = audiohax::presets::parse_with_explicit(["audiohax", "render"]).expect("parse");
    let audiohax::cli::Command::Render(args) = cli.command else {
        panic!("render")
    };
    let lib = render(
        &RenderRequest::from(&args),
        ImageInput::Bytes(audiohax::gui::SAMPLE_IMAGE),
        &audiohax::mapping_loader::default_mappings().expect("embedded"),
        SoundFontSource::Bundled,
        &mut |_| Ok(()),
    )
    .expect("library render");

    let mut h = Harness::loaded();
    assert_eq!(h.app.preset_id(), "default");
    let nodes = h.settle();
    let render_btn = one(&nodes, Role::Button, "Render").0;
    h.click(render_btn);
    h.frame();
    h.wait_for(|p| p == Phase::ImageLoaded);
    let (win, rate) = h.app.rendered_audio().expect("rendered");
    assert_eq!(rate, lib.sample_rate);
    assert!(
        win == lib.interleaved.as_slice(),
        "window default == CLI default request"
    );

    let nodes = h.settle();
    let level = one(&nodes, Role::Slider, "Level").0;
    h.set_value(level, 0.5);
    let nodes = h.settle();
    let render_btn = one(&nodes, Role::Button, "Render").0;
    h.click(render_btn);
    h.frame();
    h.wait_for(|p| p == Phase::ImageLoaded);
    let (win2, _) = h.app.rendered_audio().expect("rendered");
    assert!(
        win2 != lib.interleaved.as_slice(),
        "CONTROL: Level 0.5 renders differently"
    );
}

/// A keyboard user (S123 A9: "sliders arrow-adjust") focuses a slider and presses ← twice. The
/// first press moves the value off the preset, which draws the ● marker IN FRONT of the slider in
/// the same row; that shifts the slider's auto-generated egui id (measured: Level's accesskit id
/// changes the moment it is modified, the other sliders' ids do not), so keyboard focus — held by
/// id — is lost and the second press does nothing. The same id change ends a mouse drag one frame
/// after it leaves the preset value. Probed on Level (Everyday) and Scan detail (More).
#[test]
fn f6_a_slider_keeps_focus_after_its_first_change() {
    let mut failures = Vec::new();
    for (row, want_after_two) in [("Level", 0.90), ("Scan detail", 38.0)] {
        let mut h = Harness::loaded();
        let nodes = open_more(&mut h);
        let id = one(&nodes, Role::Slider, row).0;
        let start = one(&nodes, Role::Slider, row)
            .1
            .numeric_value()
            .expect("value");
        h.act(id, Action::Focus, None);
        h.settle();
        for _ in 0..2 {
            h.pending.push(egui::Event::Key {
                key: egui::Key::ArrowLeft,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            });
            h.settle();
        }
        let nodes = h.settle();
        let now = one(&nodes, Role::Slider, row)
            .1
            .numeric_value()
            .expect("value");
        assert!(now < start, "{row}: the first ← moved it ({start} → {now})");
        if (now - want_after_two).abs() >= 1e-3 {
            failures.push(format!(
                "{row}: two ← should reach {want_after_two}, got {now}"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "focus lost after the first change: {failures:?}"
    );
}

/// CONTROL for F-6 (passes today): the same two ← presses DO reach their target when the slider
/// is already modified (its ● already drawn, so no id shift happens between the presses). This
/// shows the keyboard drive works headlessly and isolates F-6's cause to the marker appearing.
#[test]
fn f6_control_two_presses_land_when_the_marker_is_already_drawn() {
    for (row, from, want) in [("Level", 0.5, 0.40), ("Scan detail", 20.0, 18.0)] {
        let mut h = Harness::loaded();
        let nodes = open_more(&mut h);
        let id = one(&nodes, Role::Slider, row).0;
        h.set_value(id, from);
        let nodes = h.settle();
        let id = one(&nodes, Role::Slider, row).0;
        h.act(id, Action::Focus, None);
        h.settle();
        for _ in 0..2 {
            h.pending.push(egui::Event::Key {
                key: egui::Key::ArrowLeft,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            });
            h.settle();
        }
        let nodes = h.settle();
        let now = one(&nodes, Role::Slider, row)
            .1
            .numeric_value()
            .expect("value");
        assert!(
            (now - want).abs() < 1e-3,
            "{row}: {from} - 2 steps = {want}, got {now}"
        );
    }
}

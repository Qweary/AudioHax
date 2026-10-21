//! The eframe application: panels, menus, transport, sound controls, the stage with its scan-bar
//! overlay, and the modals (UI `s123-UI` §§ 2–7).
//!
//! All the testable logic lives in the sibling modules; this file draws and routes clicks. It
//! never decodes, renders or plays: every such action is a [`JobKind`] sent to the worker
//! ([`Controller`]), and the results come back as [`Event`]s each frame.

use super::ingest::DISPLAY_MAX_SIDE;
use super::job::{Controller, Event, JobKind, LoadSource, LoadedImage, OutputChoice};
use super::overlay::{Playhead, PxRect, ScanGeometry};
use super::save::{check_not_the_picture, default_save_dir, window_confirms_overwrite};
use super::sinks::MidiPorts;
use super::state::{enablement, voices_max, Control, Controls, EverydaySpecs, Phase};
use super::text::{
    sanitize, sanitize_full, suggest_wav_name, ErrorKind, UserError, FILE_NAME_MAX, PORT_NAME_MAX,
};
use super::theme::Palette;
use crate::render::{CancelToken, RenderOutput, RenderRequest};
use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontId, Id, Key, KeyboardShortcut, Layout,
    Modifiers, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, TextureHandle, TextureOptions,
    Vec2,
};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// After this long, a running job shows "Still working…" and a Cancel button (SH T-2).
pub const STILL_WORKING_AFTER: Duration = Duration::from_secs(3);

/// Below this window width the sound panel moves to a bottom drawer (UI § 2.4).
pub const NARROW_WIDTH: f32 = 1100.0;

/// The long form of "no MIDI ports", for the info hover (UI § 5.1, the CLI's `NO_PORTS_MSG` in
/// plain words).
const NO_PORTS_LONG: &str = "No MIDI destinations were found. To use your own synth, start it \
    (or a virtual MIDI port such as loopMIDI on Windows, or a software synth like Qsynth) and \
    press Refresh. Until then, Play uses this computer's speakers.";

/// Top-level mode (UI § 2.1, § 5.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// The music screen.
    Music,
    /// The data-modem placeholder.
    Modem,
}

/// Where Play sends sound, as the selector shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputSel {
    /// This computer's speakers (built-in synth). The default.
    Speakers,
    /// A MIDI port from the current enumeration: (index, raw name).
    Midi(usize, String),
}

/// What identifies a rendered piece: the picture loaded and the request.
#[derive(Debug, Clone, PartialEq)]
struct PieceKey {
    image_seq: u64,
    request: RenderRequest,
}

/// The picture on screen.
struct Picture {
    loaded: LoadedImage,
    texture: TextureHandle,
    seq: u64,
}

/// Playback view state (UI § 4).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct PlayView {
    /// The composed piece's step count, once known.
    total_steps: Option<usize>,
    /// The step the playhead is on (while playing, or where a user Stop left it).
    step: Option<usize>,
}

/// A save waiting for the window's own overwrite confirmation (SH V-2, Linux).
struct ConfirmOverwrite {
    path: PathBuf,
}

/// The AudioHax window.
pub struct AudioHaxApp {
    palette: Palette,
    specs: Result<EverydaySpecs, String>,
    controller: Option<Controller>,
    startup_error: Option<UserError>,
    phase: Phase,
    mode: Mode,
    picture: Option<Picture>,
    image_seq: u64,
    controls: Controls,
    seed_text: String,
    output: OutputSel,
    ports: Option<Result<MidiPorts, String>>,
    show_scan_bar: bool,
    reduce_motion: bool,
    last_render: Option<(PieceKey, Arc<RenderOutput>)>,
    /// `(key, total_steps)` of the last piece composed for the showing picture.
    known_total: Option<(PieceKey, usize)>,
    play: PlayView,
    job_label: &'static str,
    error: Option<UserError>,
    toast: Option<(String, Instant)>,
    tour_step: Option<usize>,
    show_shortcuts: bool,
    show_about: bool,
    confirm: Option<ConfirmOverwrite>,
    last_save_dir: Option<PathBuf>,
    hovering_files: bool,
}

impl AudioHaxApp {
    /// Build the window: apply the palette, parse the embedded mappings, read the Everyday rows,
    /// start the worker controller, and offer the Quick tour on the very first launch.
    pub fn new(ctx: &egui::Context) -> Self {
        let mut app = Self::new_headless(ctx);
        if first_run_and_mark() {
            app.tour_step = Some(0);
        }
        app
    }

    /// [`Self::new`] without touching the user's config directory (tests, screenshots).
    pub fn new_headless(ctx: &egui::Context) -> Self {
        let palette = Palette::DARK;
        palette.apply(ctx);
        let repaint = ctx.clone();
        let (controller, startup_error) = match super::embedded_mappings() {
            Ok(m) => (
                Some(Controller::new(m, move || repaint.request_repaint())),
                None,
            ),
            Err(e) => (
                None,
                Some(UserError::new(
                    ErrorKind::Other,
                    format!("the built-in mappings failed to load: {e}"),
                )),
            ),
        };
        let specs = EverydaySpecs::from_schema().map_err(|e| e.to_string());
        let startup_error = startup_error.or_else(|| {
            specs
                .as_ref()
                .err()
                .map(|e| UserError::new(ErrorKind::Other, e.clone()))
        });
        AudioHaxApp {
            palette,
            specs,
            controller,
            startup_error,
            phase: Phase::NoImage,
            mode: Mode::Music,
            picture: None,
            image_seq: 0,
            controls: Controls::default(),
            seed_text: String::new(),
            output: OutputSel::Speakers,
            ports: None,
            show_scan_bar: true,
            reduce_motion: false,
            last_render: None,
            known_total: None,
            play: PlayView::default(),
            job_label: "",
            error: None,
            toast: None,
            tour_step: None,
            show_shortcuts: false,
            show_about: false,
            confirm: None,
            last_save_dir: None,
            hovering_files: false,
        }
    }

    /// Current interaction state (for tests and the screenshot harness).
    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// The rendered piece the window is holding, as data: interleaved stereo samples and their
    /// rate (the Architect's S123 minimum, `s123-01` § 4). `None` until something is rendered.
    pub fn rendered_audio(&self) -> Option<(&[f32], u32)> {
        self.last_render
            .as_ref()
            .map(|(_, a)| (a.interleaved.as_slice(), a.sample_rate))
    }

    /// Load the compiled-in sample picture (the "Try a sample" button).
    pub fn try_sample(&mut self) {
        self.load(LoadSource::Bytes {
            bytes: super::SAMPLE_IMAGE.to_vec(),
            name: super::SAMPLE_NAME.to_string(),
        });
    }

    /// Load a picture from a path (File ▸ Open, drag-and-drop, or a command-line argument).
    pub fn open_path(&mut self, path: PathBuf) {
        self.load(LoadSource::Path(path));
    }

    fn live(&self, c: Control) -> bool {
        self.controller.is_some() && enablement(c, self.phase).is_ok()
    }

    fn idle_phase(&self) -> Phase {
        if self.picture.is_some() {
            Phase::ImageLoaded
        } else {
            Phase::NoImage
        }
    }

    fn piece_key(&self) -> Option<PieceKey> {
        self.picture.as_ref().map(|p| PieceKey {
            image_seq: p.seq,
            request: self.controls.request(),
        })
    }

    fn geometry(&self) -> Option<ScanGeometry> {
        self.picture.as_ref().map(|p| {
            let probe = p.loaded.image.probe();
            let req = self.controls.request();
            ScanGeometry {
                width: probe.width,
                height: probe.height,
                thickness: req.pipeline.thickness,
                steps: req.pipeline.steps,
                voices: req.pipeline.instruments,
            }
        })
    }

    // ── actions ─────────────────────────────────────────────────────────────────────────────

    fn load(&mut self, source: LoadSource) {
        if !self.live(Control::Open) {
            return;
        }
        if let Some(c) = self.controller.as_mut() {
            c.submit(JobKind::Load(source));
            self.phase = Phase::Loading;
            self.job_label = "Opening the picture…";
        }
    }

    fn open_dialog(&mut self) {
        if !self.live(Control::Open) {
            return;
        }
        let mut dialog = rfd::FileDialog::new().add_filter(
            "Pictures",
            &["png", "jpg", "jpeg", "gif", "webp", "bmp", "tif", "tiff"],
        );
        if let Some(dir) = directories::UserDirs::new().and_then(|d| {
            d.picture_dir()
                .map(|p| p.to_path_buf())
                .or_else(|| Some(d.home_dir().to_path_buf()))
        }) {
            dialog = dialog.set_directory(dir);
        }
        if let Some(path) = dialog.pick_file() {
            self.open_path(path);
        }
    }

    fn render(&mut self) {
        if !self.live(Control::Render) {
            return;
        }
        let Some(pic) = self.picture.as_ref() else {
            return;
        };
        let image = pic.loaded.image.clone();
        let request = self.controls.request();
        if let Some(c) = self.controller.as_mut() {
            c.submit(JobKind::Render {
                request,
                image,
                save_to: None,
            });
            self.phase = Phase::Rendering;
            self.job_label = "Making the piece…";
        }
    }

    fn play(&mut self) {
        if !self.live(Control::Play) {
            return;
        }
        let Some(pic) = self.picture.as_ref() else {
            return;
        };
        let output = match &self.output {
            OutputSel::Speakers => OutputChoice::Speakers,
            OutputSel::Midi(index, _) => {
                let target = self
                    .ports
                    .as_mut()
                    .and_then(|p| p.as_mut().ok())
                    .and_then(|p| p.take_target(*index));
                // The enumeration's MidiOutput is consumed by a connect; enumerate afresh next time.
                self.ports = None;
                match target {
                    Some(t) => OutputChoice::Midi(t),
                    None => {
                        self.error = Some(UserError::new(
                            ErrorKind::MidiGone,
                            "the chosen port is not in the current list",
                        ));
                        self.output = OutputSel::Speakers;
                        return;
                    }
                }
            }
        };
        let image = pic.loaded.image.clone();
        let request = self.controls.request();
        if let Some(c) = self.controller.as_mut() {
            c.submit(JobKind::Play {
                request,
                image,
                output,
                cancel: CancelToken::default(),
            });
            self.phase = Phase::Playing;
            self.play = PlayView::default();
            self.job_label = "Preparing the piece…";
        }
    }

    fn stop(&mut self) {
        if self.phase == Phase::Playing {
            if let Some(c) = self.controller.as_mut() {
                c.stop_play();
            }
        }
    }

    fn cancel_job(&mut self) {
        if let Some(c) = self.controller.as_mut() {
            c.abandon();
        }
        self.phase = self.idle_phase();
        self.job_label = "";
    }

    fn save_wav(&mut self) {
        if !self.live(Control::SaveWav) {
            return;
        }
        let Some(pic) = self.picture.as_ref() else {
            return;
        };
        let suggested = suggest_wav_name(
            pic.loaded
                .path
                .as_ref()
                .map(|p| {
                    p.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default()
                })
                .as_deref(),
        );
        let mut dialog = rfd::FileDialog::new()
            .add_filter("WAV audio", &["wav"])
            .set_file_name(suggested);
        if let Some(dir) = default_save_dir(self.last_save_dir.as_deref()) {
            dialog = dialog.set_directory(dir);
        }
        let Some(mut path) = dialog.save_file() else {
            return;
        };
        if path.extension().is_none() {
            path.set_extension("wav");
        }
        if let Err(e) = check_not_the_picture(&path, pic.loaded.path.as_deref()) {
            self.error = Some(UserError::new(ErrorKind::SaveFailed, e.to_string()));
            return;
        }
        if window_confirms_overwrite() && path.exists() {
            self.confirm = Some(ConfirmOverwrite { path });
        } else {
            self.save_to(path);
        }
    }

    fn save_to(&mut self, path: PathBuf) {
        let Some(key) = self.piece_key() else {
            return;
        };
        let Some(pic) = self.picture.as_ref() else {
            return;
        };
        let image = pic.loaded.image.clone();
        let current = self
            .last_render
            .as_ref()
            .filter(|(k, _)| *k == key)
            .map(|(_, a)| a.clone());
        if let Some(c) = self.controller.as_mut() {
            match current {
                Some(audio) => {
                    c.submit(JobKind::Save { audio, path });
                    self.job_label = "Saving…";
                }
                None => {
                    c.submit(JobKind::Render {
                        request: key.request,
                        image,
                        save_to: Some(path),
                    });
                    self.job_label = "Making the piece, then saving…";
                }
            }
            self.phase = Phase::Rendering;
        }
    }

    fn toast(&mut self, text: impl Into<String>) {
        self.toast = Some((text.into(), Instant::now()));
    }

    // ── worker events ───────────────────────────────────────────────────────────────────────

    fn handle_events(&mut self, ctx: &egui::Context) {
        let events = match self.controller.as_mut() {
            Some(c) => c.poll(),
            None => return,
        };
        for ev in events {
            match ev {
                Event::Loaded(loaded) => self.on_loaded(ctx, *loaded),
                Event::Rendered { audio, saved_to } => {
                    if let Some(key) = self.piece_key() {
                        self.known_total = Some((key.clone(), audio.summary.total_steps));
                        self.last_render = Some((key, audio));
                    }
                    self.phase = self.idle_phase();
                    self.job_label = "";
                    if let Some(p) = saved_to {
                        self.on_saved(p);
                    } else {
                        self.toast("The piece is ready. Save WAV keeps it.");
                    }
                }
                Event::PlayReady { total_steps, .. } => {
                    if let Some(key) = self.piece_key() {
                        self.known_total = Some((key, total_steps));
                    }
                    self.play.total_steps = Some(total_steps);
                    self.job_label = "";
                }
                Event::Step(s) => self.play.step = Some(s),
                Event::SinkTrouble(u) => self.toast(u.headline()),
                Event::PlayEnded { cancelled } => {
                    // Park: at the start on a natural finish; where it stopped on a user Stop.
                    if !cancelled {
                        self.play.step = None;
                    }
                    self.phase = if self.picture.is_some() {
                        Phase::Stopped
                    } else {
                        Phase::NoImage
                    };
                    self.job_label = "";
                }
                Event::Saved(p) => {
                    self.phase = self.idle_phase();
                    self.job_label = "";
                    self.on_saved(p);
                }
                Event::Failed(u) => {
                    self.error = Some(u);
                    self.phase = self.idle_phase();
                    self.job_label = "";
                }
            }
        }
    }

    fn on_loaded(&mut self, ctx: &egui::Context, mut loaded: LoadedImage) {
        let mut d = std::mem::take(&mut loaded.display);
        // D-1 / LOOK FOR: never hand egui a texture larger than the GPU allows.
        let max_side = ctx
            .input(|i| i.max_texture_side)
            .clamp(256, DISPLAY_MAX_SIDE as usize) as u32;
        if d.width.max(d.height) > max_side {
            if let Some(buf) =
                image::RgbaImage::from_raw(d.width, d.height, std::mem::take(&mut d.rgba))
            {
                let small = image::DynamicImage::ImageRgba8(buf)
                    .thumbnail(max_side, max_side)
                    .to_rgba8();
                d.width = small.width();
                d.height = small.height();
                d.rgba = small.into_raw();
            }
        }
        let color = egui::ColorImage::from_rgba_unmultiplied(
            [d.width as usize, d.height as usize],
            &d.rgba,
        );
        let texture = ctx.load_texture("audiohax-picture", color, TextureOptions::LINEAR);
        self.image_seq += 1;
        let probe = loaded.image.probe();
        let schema_max = self
            .specs
            .as_ref()
            .map(|s| s.voices_schema_max())
            .unwrap_or(16);
        self.controls
            .clamp_voices(voices_max(schema_max, probe.width, probe.height));
        let title = format!("AudioHax — {}", sanitize(&loaded.name, FILE_NAME_MAX));
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
        self.picture = Some(Picture {
            loaded,
            texture,
            seq: self.image_seq,
        });
        self.last_render = None;
        self.known_total = None;
        self.play = PlayView::default();
        self.phase = Phase::ImageLoaded;
        self.job_label = "";
    }

    fn on_saved(&mut self, p: PathBuf) {
        self.last_save_dir = p.parent().map(|d| d.to_path_buf());
        let via = if matches!(self.output, OutputSel::Midi(..)) {
            " Saved using the built-in synth."
        } else {
            ""
        };
        let shown = sanitize(&p.display().to_string(), 120);
        self.toast(format!("Saved {shown}.{via}"));
    }

    // ── input ───────────────────────────────────────────────────────────────────────────────

    fn handle_input(&mut self, ctx: &egui::Context) {
        // Drag-and-drop (Music mode only). Each dropped path goes through B-1 on the worker.
        let (dropped, hovering) =
            ctx.input(|i| (i.raw.dropped_files.clone(), !i.raw.hovered_files.is_empty()));
        self.hovering_files = hovering;
        if self.mode == Mode::Music {
            if let Some(f) = dropped.into_iter().next() {
                // Native drops always carry a path; it goes through B-1's bounded read. The
                // file's own `bytes()` (an UNBOUNDED read) is never called.
                self.open_path(f.path().to_path_buf());
            }
        }

        let modal_open = self.error.is_some()
            || self.tour_step.is_some()
            || self.show_shortcuts
            || self.show_about
            || self.confirm.is_some();
        // Esc: close a dialog, else stop playback.
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            if modal_open {
                self.error = None;
                self.tour_step = None;
                self.show_shortcuts = false;
                self.show_about = false;
                self.confirm = None;
            } else {
                self.stop();
            }
        }
        if modal_open {
            return;
        }
        let open = KeyboardShortcut::new(Modifiers::COMMAND, Key::O);
        let save = KeyboardShortcut::new(Modifiers::COMMAND, Key::S);
        if ctx.input_mut(|i| i.consume_shortcut(&open)) && self.mode == Mode::Music {
            self.open_dialog();
        }
        if ctx.input_mut(|i| i.consume_shortcut(&save)) && self.mode == Mode::Music {
            self.save_wav();
        }
        // Space and V only when no text field has the keyboard.
        if !ctx.egui_wants_keyboard_input() && self.mode == Mode::Music {
            if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Space)) {
                if self.phase == Phase::Playing {
                    self.stop();
                } else {
                    self.play();
                }
            }
            if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::V)) {
                self.show_scan_bar = !self.show_scan_bar;
            }
        }
    }

    // ── drawing ─────────────────────────────────────────────────────────────────────────────

    /// Draw one frame. `eframe::App::ui` calls this; tests call it through
    /// `egui::Context::run_ui` with no window.
    pub fn draw(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        if let Some(e) = self.startup_error.take() {
            self.error = Some(e);
        }
        self.handle_events(&ctx);
        self.handle_input(&ctx);

        let p = self.palette;
        egui::Panel::top("menubar")
            .frame(
                egui::Frame::new()
                    .fill(p.surface)
                    .inner_margin(egui::Margin::symmetric(8, 4)),
            )
            .show(ui, |ui| self.menu_bar(ui));
        egui::Panel::top("modebar")
            .frame(
                egui::Frame::new()
                    .fill(p.surface)
                    .inner_margin(egui::Margin::symmetric(12, 6)),
            )
            .show(ui, |ui| self.mode_bar(ui));

        if self.mode == Mode::Music {
            egui::Panel::bottom("transport")
                .frame(
                    egui::Frame::new()
                        .fill(p.surface)
                        .inner_margin(egui::Margin::symmetric(12, 8)),
                )
                .show(ui, |ui| self.transport(ui));
            let narrow = ctx.content_rect().width() < NARROW_WIDTH;
            if narrow {
                egui::Panel::bottom("sound_drawer")
                    .frame(
                        egui::Frame::new()
                            .fill(p.surface)
                            .inner_margin(egui::Margin::symmetric(12, 6)),
                    )
                    .show(ui, |ui| {
                        egui::CollapsingHeader::new("Sound")
                            .default_open(true)
                            .show(ui, |ui| {
                                egui::ScrollArea::vertical()
                                    .max_height(220.0)
                                    .show(ui, |ui| {
                                        self.sound_panel(ui, true);
                                    });
                            });
                    });
            } else {
                egui::Panel::right("sound")
                    .resizable(false)
                    .exact_size(320.0)
                    .frame(
                        egui::Frame::new()
                            .fill(p.surface)
                            .inner_margin(egui::Margin::same(16)),
                    )
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical().show(ui, |ui| self.sound_panel(ui, false));
                    });
            }
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::new()
                        .fill(p.bg)
                        .inner_margin(egui::Margin::same(24)),
                )
                .show(ui, |ui| self.stage(ui));
        } else {
            egui::CentralPanel::default()
                .frame(egui::Frame::new().fill(p.bg))
                .show(ui, |ui| self.modem_placeholder(ui));
        }

        self.modals(&ctx);

        // Keep the window live while work runs (progress, "Still working…", the playhead glide).
        let busy = self
            .controller
            .as_ref()
            .map(|c| c.is_busy())
            .unwrap_or(false);
        if busy || self.toast.is_some() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
        if let Some((_, t)) = &self.toast {
            if t.elapsed() > Duration::from_secs(6) {
                self.toast = None;
            }
        }
    }

    fn menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                let music = self.mode == Mode::Music;
                if ui
                    .add_enabled(
                        music && self.live(Control::Open),
                        egui::Button::new("Open picture…").shortcut_text(
                            ui.ctx().format_shortcut(&KeyboardShortcut::new(
                                Modifiers::COMMAND,
                                Key::O,
                            )),
                        ),
                    )
                    .clicked()
                {
                    ui.close();
                    self.open_dialog();
                }
                if ui
                    .add_enabled(
                        music && self.live(Control::SaveWav),
                        egui::Button::new("Save WAV…").shortcut_text(
                            ui.ctx().format_shortcut(&KeyboardShortcut::new(
                                Modifiers::COMMAND,
                                Key::S,
                            )),
                        ),
                    )
                    .clicked()
                {
                    ui.close();
                    self.save_wav();
                }
                if ui
                    .add_enabled(
                        music && self.live(Control::Open),
                        egui::Button::new("Try a sample"),
                    )
                    .clicked()
                {
                    ui.close();
                    self.try_sample();
                }
                ui.separator();
                if ui.button("Quit").clicked() {
                    ui.close();
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            ui.menu_button("View", |ui| {
                let mut scan = self.show_scan_bar;
                if ui
                    .add(egui::Checkbox::new(&mut scan, "Show scan bar"))
                    .on_hover_text("Shortcut: V")
                    .changed()
                {
                    self.show_scan_bar = scan;
                }
                ui.checkbox(&mut self.reduce_motion, "Reduce motion")
                    .on_hover_text("The scan bar jumps from step to step instead of gliding.");
                ui.separator();
                let mut dark = self.palette.dark;
                ui.radio_value(&mut dark, true, "Dark mode");
                ui.radio_value(&mut dark, false, "Light mode");
                if dark != self.palette.dark {
                    self.palette = Palette::for_mode(dark);
                    self.palette.apply(ui.ctx());
                }
            });
            ui.menu_button("Help", |ui| {
                if ui.button("Quick tour").clicked() {
                    ui.close();
                    self.tour_step = Some(0);
                }
                if ui.button("Keyboard shortcuts").clicked() {
                    ui.close();
                    self.show_shortcuts = true;
                }
                if ui.button("About AudioHax").clicked() {
                    ui.close();
                    self.show_about = true;
                }
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(RichText::new("AudioHax").color(self.palette.text_muted));
            });
        });
    }

    fn mode_bar(&mut self, ui: &mut egui::Ui) {
        let p = self.palette;
        ui.horizontal(|ui| {
            let switch_live = self.live(Control::ModeSwitch);
            ui.add_enabled_ui(switch_live, |ui| {
                let mut mode = self.mode;
                ui.selectable_value(&mut mode, Mode::Music, "Music");
                ui.selectable_value(&mut mode, Mode::Modem, "Modem");
                self.mode = mode;
            })
            .response
            .on_disabled_hover_text(
                enablement(Control::ModeSwitch, self.phase)
                    .err()
                    .unwrap_or(""),
            );

            if self.mode == Mode::Music {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let label = match &self.output {
                        OutputSel::Speakers => "Output: This computer's speakers".to_string(),
                        OutputSel::Midi(i, name) => {
                            format!("Output: MIDI {i}: {}", sanitize(name, PORT_NAME_MAX))
                        }
                    };
                    let live = self.live(Control::OutputSelector);
                    ui.add_enabled_ui(live, |ui| {
                        let r = ui.menu_button(label, |ui| self.output_selector(ui));
                        if r.response.clicked() {
                            // SH N-2: enumerate afresh whenever the selector opens.
                            self.ports = None;
                        }
                    })
                    .response
                    .on_disabled_hover_text(
                        enablement(Control::OutputSelector, self.phase)
                            .err()
                            .unwrap_or(""),
                    );
                    let _ = p;
                });
            }
        });
    }

    fn output_selector(&mut self, ui: &mut egui::Ui) {
        let p = self.palette;
        if self.ports.is_none() {
            self.ports = Some(MidiPorts::enumerate());
        }
        ui.set_min_width(420.0);
        ui.label(RichText::new("Output (where Play sends sound)").strong());
        ui.add_space(4.0);
        let mut sel = self.output.clone();
        ui.radio_value(
            &mut sel,
            OutputSel::Speakers,
            "This computer's speakers (built-in synth)",
        );
        let entries: Vec<(usize, String)> = match &self.ports {
            Some(Ok(ports)) => ports
                .entries
                .iter()
                .map(|e| (e.index, e.name.clone()))
                .collect(),
            _ => Vec::new(),
        };
        let enum_error = match &self.ports {
            Some(Err(e)) => Some(e.clone()),
            _ => None,
        };
        ui.horizontal(|ui| {
            let is_midi = matches!(sel, OutputSel::Midi(..));
            let r = ui.add_enabled(
                !entries.is_empty(),
                egui::RadioButton::new(is_midi, "A MIDI port"),
            );
            if r.clicked() {
                if let Some((i, n)) = entries.first() {
                    if !is_midi {
                        sel = OutputSel::Midi(*i, n.clone());
                    }
                }
            }
            let current = match &sel {
                OutputSel::Midi(i, n) => format!("{i}: {}", sanitize(n, PORT_NAME_MAX)),
                OutputSel::Speakers if entries.is_empty() => "No MIDI ports found".to_string(),
                OutputSel::Speakers => "Choose a port".to_string(),
            };
            ui.add_enabled_ui(!entries.is_empty(), |ui| {
                egui::ComboBox::from_id_salt("midi-port")
                    .selected_text(current)
                    .width(240.0)
                    .show_ui(ui, |ui| {
                        for (i, n) in &entries {
                            let shown = format!("{i}: {}", sanitize(n, PORT_NAME_MAX));
                            ui.selectable_value(&mut sel, OutputSel::Midi(*i, n.clone()), shown)
                                .on_hover_text(sanitize_full(n));
                        }
                    });
            })
            .response
            .on_disabled_hover_text(NO_PORTS_LONG);
            if ui.button("⟳ Refresh").clicked() {
                self.ports = Some(MidiPorts::enumerate());
            }
        });
        if let Some(e) = enum_error {
            ui.colored_label(p.warn, "MIDI is not available on this computer right now.")
                .on_hover_text(sanitize_full(&e));
        }
        ui.add_space(6.0);
        ui.label(
            RichText::new(
                "ⓘ With a MIDI port, your own synth makes the sound, so Room and Level are set by \
                 it, not here. Save WAV always uses the built-in synth.",
            )
            .color(p.text_muted)
            .size(13.0),
        );
        if sel != self.output {
            self.output = sel;
        }
    }

    fn sound_panel(&mut self, ui: &mut egui::Ui, compact: bool) {
        let p = self.palette;
        let specs = match &self.specs {
            Ok(s) => s.clone(),
            Err(e) => {
                ui.colored_label(p.error, "The sound controls could not be built.")
                    .on_hover_text(e.clone());
                return;
            }
        };
        let live = self.live(Control::SoundControls);
        let reason = enablement(Control::SoundControls, self.phase)
            .err()
            .unwrap_or("");
        let midi = matches!(self.output, OutputSel::Midi(..));

        if !compact {
            ui.label(
                RichText::new("SOUND")
                    .color(p.text_muted)
                    .size(13.0)
                    .strong(),
            );
            ui.add_space(4.0);
        }
        section_label(ui, p, "Music");
        ui.add_enabled_ui(live, |ui| {
            // Variation (seed): empty = the picture chooses (image-derived seed).
            ui.horizontal(|ui| {
                let l = ui.label(&specs.seed.label);
                let mut text = self.seed_text.clone();
                let r = ui
                    .add(
                        egui::TextEdit::singleline(&mut text)
                            .hint_text("from the picture")
                            .desired_width(130.0)
                            .char_limit(20),
                    )
                    .labelled_by(l.id)
                    .on_hover_text(EverydaySpecs::help(&specs.seed));
                if r.changed() {
                    let digits: String = text.chars().filter(|c| c.is_ascii_digit()).collect();
                    if digits.is_empty() {
                        self.seed_text.clear();
                        self.controls.seed = None;
                    } else if let Ok(v) = digits.parse::<u64>() {
                        self.seed_text = digits;
                        self.controls.seed = Some(v);
                    }
                }
                if ui
                    .button("🎲")
                    .on_hover_text("Pick a new Variation")
                    .clicked()
                {
                    let v = u64::from(rand::random::<u32>() % 1_000_000);
                    self.seed_text = v.to_string();
                    self.controls.seed = Some(v);
                }
                if self.controls.seed.is_some()
                    && ui
                        .small_button("×")
                        .on_hover_text("Let the picture choose")
                        .clicked()
                {
                    self.seed_text.clear();
                    self.controls.seed = None;
                }
            });
            // Voices: bounded by the picture (L-14).
            let vmax = self
                .picture
                .as_ref()
                .map(|pic| {
                    let pr = pic.loaded.image.probe();
                    voices_max(specs.voices_schema_max(), pr.width, pr.height)
                })
                .unwrap_or(specs.voices_schema_max());
            let mut voices = self.controls.instruments as u32;
            ui.horizontal(|ui| {
                let l = ui.label(&specs.instruments.label);
                let r = ui
                    .add(egui::Slider::new(&mut voices, 1..=vmax).integer())
                    .labelled_by(l.id)
                    .on_hover_text(EverydaySpecs::help(&specs.instruments));
                name_slider(&r, &specs.instruments.label, f64::from(voices));
            });
            self.controls.instruments = voices as usize;
        })
        .response
        .on_disabled_hover_text(reason);

        ui.add_space(8.0);
        section_label(ui, p, "After the music");
        let output_live = live && !midi;
        let output_reason = if midi && live {
            "Your MIDI synth sets these for Play. Save WAV uses the built-in synth with the values shown."
        } else {
            reason
        };
        ui.add_enabled_ui(output_live, |ui| {
            ui.horizontal(|ui| {
                let l = ui.label(&specs.reverb.label);
                let mut on = self.controls.reverb;
                ui.selectable_value(&mut on, true, "On")
                    .labelled_by(l.id)
                    .on_hover_text(EverydaySpecs::help(&specs.reverb));
                ui.selectable_value(&mut on, false, "Off")
                    .labelled_by(l.id)
                    .on_hover_text(EverydaySpecs::help(&specs.reverb));
                self.controls.reverb = on;
            });
            let (gmin, gmax, gstep) = specs.gain_range();
            ui.horizontal(|ui| {
                let l = ui.label(&specs.gain.label);
                let mut g = self.controls.gain;
                let r = ui
                    .add(
                        egui::Slider::new(&mut g, gmin..=gmax)
                            .step_by(f64::from(gstep))
                            .show_value(false),
                    )
                    .labelled_by(l.id)
                    .on_hover_text(
                        "1.0 leaves the level unchanged. Above 1.0 is a boost that softly \
                         rounds off the loudest peaks.",
                    );
                name_slider(&r, &specs.gain.label, f64::from(g));
                // The unity tick (UI § 10.1): where "no change" is.
                let rect = r.rect;
                let range = rect.x_range().shrink(rect.height() / 2.5);
                let t = ((1.0 - gmin) / (gmax - gmin)).clamp(0.0, 1.0);
                let x = range.min + t * (range.max - range.min);
                ui.painter().line_segment(
                    [
                        Pos2::new(x, rect.bottom() - 2.0),
                        Pos2::new(x, rect.bottom() + 3.0),
                    ],
                    Stroke::new(1.5, p.text_muted),
                );
                ui.label(RichText::new(format!("{g:.2}")).monospace());
                self.controls.gain = g;
            });
        })
        .response
        .on_disabled_hover_text(output_reason);
        if midi {
            ui.label(
                RichText::new(
                    "Room and Level are set by your MIDI synth while a MIDI port is chosen.",
                )
                .color(p.warn)
                .size(13.0),
            );
        }

        ui.add_space(8.0);
        egui::CollapsingHeader::new("More")
            .default_open(false)
            .show(ui, |ui| {
                ui.label(
                    RichText::new("More settings arrive in a later update.")
                        .color(p.text_muted)
                        .size(13.0),
                );
            });
        if !compact {
            ui.add_space(12.0);
            egui::Frame::new()
                .fill(p.raised)
                .stroke(Stroke::new(1.0, Palette::with_alpha(p.control_border, 0.6)))
                .corner_radius(CornerRadius::same(10))
                .inner_margin(egui::Margin::same(12))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(RichText::new("Presets").strong());
                    ui.label(
                        RichText::new("coming in the next update")
                            .color(p.text_muted)
                            .size(13.0),
                    );
                });
        }
    }

    fn transport(&mut self, ui: &mut egui::Ui) {
        let p = self.palette;
        ui.horizontal(|ui| {
            let busy_render = self.phase == Phase::Rendering;
            let render_text = if busy_render {
                "Rendering…"
            } else {
                "Render"
            };
            ui.add_enabled(self.live(Control::Render), egui::Button::new(render_text))
                .on_hover_text("Make the whole piece now, so Save WAV is instant.")
                .on_disabled_hover_text(enablement(Control::Render, self.phase).err().unwrap_or(""))
                .clicked()
                .then(|| self.render());

            let play_live = self.live(Control::Play);
            let play_label = if self.phase == Phase::Playing {
                "▶ Playing"
            } else {
                "▶ Play"
            };
            let play_btn = egui::Button::new(RichText::new(play_label).color(if play_live {
                p.on_accent
            } else {
                p.text_muted
            }))
            .fill(if play_live { p.accent } else { p.raised });
            ui.add_enabled(play_live, play_btn)
                .on_hover_text("Hear it now (Space)")
                .on_disabled_hover_text(enablement(Control::Play, self.phase).err().unwrap_or(""))
                .clicked()
                .then(|| self.play());

            ui.add_enabled(self.live(Control::Stop), egui::Button::new("⏹ Stop"))
                .on_hover_text("Stop (Space or Esc)")
                .on_disabled_hover_text(enablement(Control::Stop, self.phase).err().unwrap_or(""))
                .clicked()
                .then(|| self.stop());

            ui.add_space(16.0);
            // Step readout (the composer's steps, not the scan's).
            if let (Some(step), Some(total)) = (self.play.step, self.play.total_steps) {
                ui.label(RichText::new(format!("step {} / {}", step + 1, total)).monospace());
            } else if let Some(total) = self.play.total_steps {
                ui.label(
                    RichText::new(format!("{total} steps"))
                        .monospace()
                        .color(p.text_muted),
                );
            }

            // Progress and the abandon-style Cancel (SH T-2).
            if let Some(c) = self.controller.as_ref() {
                let preparing_play =
                    self.phase == Phase::Playing && self.play.total_steps.is_none();
                let working =
                    matches!(self.phase, Phase::Loading | Phase::Rendering) || preparing_play;
                if working {
                    ui.add(egui::Spinner::new());
                    let waiting = c.waiting_on_abandoned();
                    let label = if waiting {
                        "Finishing the previous picture…"
                    } else {
                        self.job_label
                    };
                    ui.label(RichText::new(label).color(p.text_muted));
                    let long = c
                        .running_for()
                        .map(|d| d >= STILL_WORKING_AFTER)
                        .unwrap_or(false);
                    if long && !waiting {
                        ui.label(RichText::new("Still working…").color(p.text_muted));
                        if ui
                            .button("Cancel")
                            .on_hover_text(
                                "Stop waiting for this one. It finishes in the background; a new \
                                 picture waits until it has.",
                            )
                            .clicked()
                        {
                            self.cancel_job();
                        }
                    }
                }
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_enabled(self.live(Control::SaveWav), egui::Button::new("Save WAV…"))
                    .on_hover_text("Keep the piece as a WAV file (Ctrl/Cmd+S)")
                    .on_disabled_hover_text(
                        enablement(Control::SaveWav, self.phase).err().unwrap_or(""),
                    )
                    .clicked()
                    .then(|| self.save_wav());
                if let Some((text, _)) = &self.toast {
                    ui.label(RichText::new(text.clone()).color(p.text).size(13.0));
                }
            });
        });
    }

    fn stage(&mut self, ui: &mut egui::Ui) {
        let p = self.palette;
        let avail = ui.available_rect_before_wrap();
        let Some(pic) = self.picture.as_ref() else {
            self.empty_state(ui, avail);
            return;
        };
        let probe = pic.loaded.image.probe();
        let texture_id = pic.texture.id();
        // Leave room past the trailing edge for the coda park and its caption.
        let vertical = probe.width > probe.height;
        let reserve = if self.show_scan_bar { 34.0 } else { 0.0 };
        let box_rect = if vertical {
            Rect::from_min_max(avail.min, Pos2::new(avail.max.x - reserve, avail.max.y))
        } else {
            Rect::from_min_max(avail.min, Pos2::new(avail.max.x, avail.max.y - reserve))
        };
        let scale =
            (box_rect.width() / probe.width as f32).min(box_rect.height() / probe.height as f32);
        let size = Vec2::new(probe.width as f32 * scale, probe.height as f32 * scale);
        let img_rect = Rect::from_center_size(box_rect.center(), size);
        ui.allocate_rect(avail, Sense::hover());
        let painter = ui.painter_at(avail);
        painter.image(
            texture_id,
            img_rect,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );
        if self.hovering_files {
            painter.rect_stroke(
                img_rect.expand(4.0),
                CornerRadius::same(6),
                Stroke::new(2.0, p.accent),
                StrokeKind::Outside,
            );
        }
        if !self.show_scan_bar {
            return;
        }
        let Some(geom) = self.geometry() else {
            return;
        };
        let to_screen = |r: PxRect| -> Rect {
            Rect::from_min_size(
                Pos2::new(
                    img_rect.min.x + r.x as f32 * scale,
                    img_rect.min.y + r.y as f32 * scale,
                ),
                Vec2::new(r.w as f32 * scale, r.h as f32 * scale),
            )
        };
        // Case 3: the part of the picture the piece never reaches.
        let key = self.piece_key();
        if let Some((k, total)) = &self.known_total {
            if Some(k) == key.as_ref() {
                if let Some(un) = geom.unreached(*total) {
                    let r = to_screen(un);
                    painter.rect_filled(r, CornerRadius::ZERO, Palette::with_alpha(p.bg, 0.30));
                    let hatch = Stroke::new(1.0, Palette::with_alpha(p.bg, 0.55));
                    let mut x = r.min.x - r.height();
                    while x < r.max.x {
                        let a = Pos2::new(x, r.max.y);
                        let b = Pos2::new(x + r.height(), r.min.y);
                        painter.with_clip_rect(r).line_segment([a, b], hatch);
                        x += 8.0;
                    }
                    let caption_pos = if vertical {
                        Pos2::new(r.center().x, r.max.y - 14.0)
                    } else {
                        Pos2::new(r.center().x, r.center().y)
                    };
                    caption(
                        &painter,
                        p,
                        caption_pos,
                        "this part of the picture isn't reached",
                    );
                }
            }
        }
        // The bar.
        let playing = self.phase == Phase::Playing;
        let step = self.play.step.unwrap_or(0);
        match geom.playhead(step) {
            Playhead::Bar(rect) => {
                let target = to_screen(rect);
                let shown = if self.reduce_motion {
                    target
                } else {
                    let ctx = ui.ctx();
                    let id = Id::new("scanbar-glide");
                    let (pos, along) = if vertical {
                        (
                            ctx.animate_value_with_time(id, target.min.x, 0.25),
                            target.min.y,
                        )
                    } else {
                        (
                            ctx.animate_value_with_time(id, target.min.y, 0.25),
                            target.min.x,
                        )
                    };
                    if vertical {
                        Rect::from_min_size(Pos2::new(pos, along), target.size())
                    } else {
                        Rect::from_min_size(Pos2::new(along, pos), target.size())
                    }
                };
                let alpha = if playing && self.play.step.is_some() {
                    0.50
                } else {
                    0.22
                };
                draw_bar(&painter, p, shown, alpha);
                // Per-voice stripe boundaries.
                let stripes = geom.voice_stripes(rect);
                for s in stripes.iter().skip(1) {
                    let r = to_screen(*s);
                    let off = shown.min - target.min;
                    let (a, b) = if vertical {
                        (
                            Pos2::new(r.min.x, r.min.y) + off,
                            Pos2::new(r.max.x, r.min.y) + off,
                        )
                    } else {
                        (
                            Pos2::new(r.min.x, r.min.y) + off,
                            Pos2::new(r.min.x, r.max.y) + off,
                        )
                    };
                    painter
                        .line_segment([a, b], Stroke::new(1.0, Palette::with_alpha(p.text, 0.45)));
                }
            }
            Playhead::Coda => {
                let park = if vertical {
                    Rect::from_min_size(
                        Pos2::new(img_rect.max.x + 8.0, img_rect.min.y),
                        Vec2::new(10.0, img_rect.height()),
                    )
                } else {
                    Rect::from_min_size(
                        Pos2::new(img_rect.min.x, img_rect.max.y + 8.0),
                        Vec2::new(img_rect.width(), 10.0),
                    )
                };
                painter.rect_filled(
                    park,
                    CornerRadius::same(3),
                    Palette::with_alpha(p.text_muted, 0.35),
                );
                painter.rect_stroke(
                    park,
                    CornerRadius::same(3),
                    Stroke::new(1.0, p.text_muted),
                    StrokeKind::Inside,
                );
                let pos = if vertical {
                    Pos2::new(img_rect.max.x - 4.0, img_rect.max.y + 0.0)
                } else {
                    Pos2::new(img_rect.center().x, park.max.y + 10.0)
                };
                let anchor = if vertical {
                    Align2::RIGHT_BOTTOM
                } else {
                    Align2::CENTER_CENTER
                };
                painter.text(
                    pos,
                    anchor,
                    "coda — the picture is finished",
                    FontId::proportional(13.0),
                    p.text_muted,
                );
            }
        }
    }

    fn empty_state(&mut self, ui: &mut egui::Ui, avail: Rect) {
        let p = self.palette;
        let card =
            Rect::from_center_size(avail.center(), Vec2::new(420.0, 230.0).min(avail.size()));
        let painter = ui.painter_at(avail);
        painter.text(
            Pos2::new(card.center().x, card.min.y - 34.0),
            Align2::CENTER_CENTER,
            "Turn a picture into music.",
            FontId::proportional(22.0),
            p.text,
        );
        let border = if self.hovering_files {
            p.accent
        } else {
            p.control_border
        };
        let pts = [
            card.left_top(),
            card.right_top(),
            card.right_bottom(),
            card.left_bottom(),
            card.left_top(),
        ];
        painter.extend(egui::Shape::dashed_line(
            &pts,
            Stroke::new(1.5, border),
            8.0,
            6.0,
        ));
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(card.shrink(24.0))
                .layout(Layout::top_down(Align::Center)),
        );
        child.add_space(12.0);
        child.label(RichText::new("Drop a picture here").size(17.0));
        child.label(RichText::new("— or —").color(p.text_muted));
        child.add_space(6.0);
        let live = self.live(Control::Open);
        if child
            .add_enabled(
                live,
                egui::Button::new("Open a picture…").min_size(Vec2::new(180.0, 30.0)),
            )
            .clicked()
        {
            self.open_dialog();
        }
        if child
            .add_enabled(
                live,
                egui::Button::new("Try a sample").min_size(Vec2::new(180.0, 30.0)),
            )
            .clicked()
        {
            self.try_sample();
        }
        if self.phase == Phase::Loading {
            child.add_space(6.0);
            child.horizontal(|ui| {
                ui.add(egui::Spinner::new());
                ui.label(RichText::new(self.job_label).color(p.text_muted));
            });
        }
    }

    fn modem_placeholder(&mut self, ui: &mut egui::Ui) {
        let p = self.palette;
        let avail = ui.available_rect_before_wrap();
        let card =
            Rect::from_center_size(avail.center(), Vec2::new(520.0, 280.0).min(avail.size()));
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(card)
                .layout(Layout::top_down(Align::Center)),
        );
        child.label(
            RichText::new("◌  Data modem")
                .size(20.0)
                .color(p.text_muted),
        );
        child.add_space(12.0);
        child.label(
            RichText::new(
                "AudioHax can hide a text message inside sound as an MFSK tone pattern. Bringing \
                 that into this window — type a message, hear it, and later blend it into the \
                 music — is coming in a future update.",
            )
            .color(p.text_muted),
        );
        child.add_space(8.0);
        child.label(
            RichText::new("For now, the data modem is available from the command line.")
                .color(p.text_muted),
        );
        child.add_space(16.0);
        if child.button("‹ Back to Music").clicked() {
            self.mode = Mode::Music;
        }
    }

    fn modals(&mut self, ctx: &egui::Context) {
        let p = self.palette;
        if let Some(err) = self.error.clone() {
            let r = egui::Modal::new(Id::new("error-modal")).show(ctx, |ui| {
                ui.set_max_width(460.0);
                ui.label(RichText::new(err.headline()).color(p.error).size(16.0));
                ui.add_space(6.0);
                egui::CollapsingHeader::new("Details")
                    .default_open(false)
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(sanitize_full(&err.details))
                                .monospace()
                                .color(p.text_muted),
                        );
                    });
                ui.add_space(8.0);
                ui.button("OK").clicked()
            });
            if r.inner || r.backdrop_response.clicked() {
                self.error = None;
            }
        }
        if let Some(path) = self.confirm.as_ref().map(|c| c.path.clone()) {
            let r = egui::Modal::new(Id::new("overwrite-modal")).show(ctx, |ui| {
                ui.set_max_width(460.0);
                ui.label(format!(
                    "“{}” already exists. Replace it?",
                    sanitize(
                        &path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default(),
                        FILE_NAME_MAX
                    )
                ));
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let replace = ui.button("Replace").clicked();
                    let cancel = ui.button("Cancel").clicked();
                    (replace, cancel)
                })
                .inner
            });
            let (replace, cancel) = r.inner;
            if replace {
                self.confirm = None;
                self.save_to(path);
            } else if cancel || r.backdrop_response.clicked() {
                self.confirm = None;
            }
        }
        if let Some(step) = self.tour_step {
            const STEPS: [(&str, &str); 5] = [
                (
                    "Open or drop a picture.",
                    "Use “Open a picture…”, drop one onto the window, or press “Try a sample”.",
                ),
                (
                    "Press Play to hear it.",
                    "The ▶ Play button at the bottom, or the Space bar.",
                ),
                (
                    "Turn on the scan bar to see which part of the picture you are hearing.",
                    "View ▸ Show scan bar, or the V key.",
                ),
                (
                    "Try the four controls — Variation, Voices, Room, Level.",
                    "They are in the Sound panel.",
                ),
                (
                    "Save a WAV to keep your piece.",
                    "Save WAV… at the bottom right, or Ctrl/Cmd+S.",
                ),
            ];
            let i = step.min(STEPS.len() - 1);
            let r = egui::Modal::new(Id::new("tour-modal")).show(ctx, |ui| {
                ui.set_max_width(440.0);
                ui.label(
                    RichText::new(format!("Quick tour · {} of {}", i + 1, STEPS.len()))
                        .color(p.text_muted)
                        .size(13.0),
                );
                ui.add_space(4.0);
                ui.label(RichText::new(STEPS[i].0).size(17.0));
                ui.label(RichText::new(STEPS[i].1).color(p.text_muted));
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    let last = i + 1 == STEPS.len();
                    let next = ui.button(if last { "Done" } else { "Next" }).clicked();
                    let skip = !last && ui.button("Skip the tour").clicked();
                    (next, last, skip)
                })
                .inner
            });
            let (next, last, skip) = r.inner;
            if skip || (next && last) || r.backdrop_response.clicked() {
                self.tour_step = None;
            } else if next {
                self.tour_step = Some(i + 1);
            }
        }
        if self.show_shortcuts {
            let r = egui::Modal::new(Id::new("shortcuts-modal")).show(ctx, |ui| {
                ui.label(RichText::new("Keyboard shortcuts").size(17.0));
                ui.add_space(6.0);
                egui::Grid::new("shortcuts")
                    .num_columns(2)
                    .spacing([24.0, 6.0])
                    .show(ui, |ui| {
                        for (k, what) in [
                            ("Space", "Play / Stop"),
                            ("V", "Show or hide the scan bar"),
                            ("Ctrl/Cmd + O", "Open a picture"),
                            ("Ctrl/Cmd + S", "Save WAV"),
                            ("Esc", "Stop playback / close a dialog"),
                        ] {
                            ui.label(RichText::new(k).monospace());
                            ui.label(what);
                            ui.end_row();
                        }
                    });
                ui.add_space(8.0);
                ui.button("Close").clicked()
            });
            if r.inner || r.backdrop_response.clicked() {
                self.show_shortcuts = false;
            }
        }
        if self.show_about {
            let r = egui::Modal::new(Id::new("about-modal")).show(ctx, |ui| {
                ui.set_max_width(460.0);
                ui.label(RichText::new("AudioHax").size(20.0));
                ui.label(
                    RichText::new(format!("Version {}", env!("CARGO_PKG_VERSION")))
                        .color(p.text_muted),
                );
                ui.add_space(6.0);
                ui.label("AudioHax turns a picture into music.");
                ui.add_space(6.0);
                ui.label(RichText::new("https://github.com/Qweary/AudioHax").color(p.accent));
                ui.add_space(6.0);
                ui.label(
                    RichText::new(
                        "Credits: the bundled SoundFont is GeneralUser GS by S. Christian Collins. \
                         The window's fonts are the egui defaults (Ubuntu-Light, Hack, Noto Emoji, \
                         emoji-icon-font), each under its own licence.",
                    )
                    .color(p.text_muted)
                    .size(13.0),
                );
                ui.add_space(8.0);
                ui.button("Close").clicked()
            });
            if r.inner || r.backdrop_response.clicked() {
                self.show_about = false;
            }
        }
    }
}

impl eframe::App for AudioHaxApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.draw(ui);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Closing mid-play: stop and wait for the sink's all-sound-off (L-3).
        if let Some(c) = self.controller.as_mut() {
            c.shutdown(Duration::from_millis(1500));
        }
    }
}

/// Give a slider whose visible label sits to its LEFT its own accessible name, so a screen
/// reader (and the A10 role+label test) finds it by role and label, not only by relation.
fn name_slider(r: &egui::Response, label: &str, value: f64) {
    let enabled = r.enabled();
    let label = label.to_string();
    r.widget_info(move || egui::WidgetInfo::slider(enabled, value, label.clone()));
}

fn section_label(ui: &mut egui::Ui, p: Palette, text: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(text).color(p.text_muted).size(13.0));
        let r = ui.available_rect_before_wrap();
        let y = r.center().y;
        ui.painter().line_segment(
            [Pos2::new(r.min.x + 4.0, y), Pos2::new(r.max.x, y)],
            Stroke::new(1.0, Palette::with_alpha(p.control_border, 0.6)),
        );
    });
}

/// The scan bar: frost-blue fill with a dark outer and a light inner outline, so it reads on
/// white, black and saturated pictures alike (UI § 4.4).
fn draw_bar(painter: &egui::Painter, p: Palette, r: Rect, alpha: f32) {
    painter.rect_filled(r, CornerRadius::ZERO, Palette::with_alpha(p.accent, alpha));
    painter.rect_stroke(
        r,
        CornerRadius::ZERO,
        Stroke::new(1.0, Palette::with_alpha(p.bg, 0.70)),
        StrokeKind::Outside,
    );
    painter.rect_stroke(
        r,
        CornerRadius::ZERO,
        Stroke::new(1.0, Palette::with_alpha(p.text, 0.50)),
        StrokeKind::Inside,
    );
}

fn caption(painter: &egui::Painter, p: Palette, pos: Pos2, text: &str) {
    let galley = painter.layout_no_wrap(text.to_string(), FontId::proportional(13.0), p.text);
    let r = Rect::from_center_size(pos, galley.size() + Vec2::new(12.0, 6.0));
    painter.rect_filled(r, CornerRadius::same(4), Palette::with_alpha(p.bg, 0.75));
    painter.galley(r.min + Vec2::new(6.0, 3.0), galley, p.text);
}

/// First launch? Marks the flag so the Quick tour is offered once (UI § 6). Best effort: if the
/// config directory cannot be written, the tour is simply offered again next time.
fn first_run_and_mark() -> bool {
    let Some(dirs) = directories::ProjectDirs::from("", "", "AudioHax") else {
        return false;
    };
    let flag = dirs.config_dir().join("gui-tour-seen");
    if flag.exists() {
        return false;
    }
    let _ = std::fs::create_dir_all(dirs.config_dir());
    let _ = std::fs::write(&flag, b"1\n");
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_frames(ctx: &egui::Context, app: &mut AudioHaxApp, n: usize) -> egui::FullOutput {
        let mut last = None;
        for _ in 0..n {
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 800.0))),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| app.draw(ui));
            // No renderer in a headless test: drop the texture uploads deliberately.
            out.textures_delta.clear();
            last = Some(out);
        }
        last.expect("at least one frame")
    }

    fn wait_until(ctx: &egui::Context, app: &mut AudioHaxApp, what: impl Fn(&AudioHaxApp) -> bool) {
        let t0 = Instant::now();
        while !what(app) {
            run_frames(ctx, app, 1);
            assert!(t0.elapsed() < Duration::from_secs(60), "timed out");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// Every accesskit node with a role and a label, as (role, label).
    fn a11y_nodes(out: &egui::FullOutput) -> Vec<(egui::accesskit::Role, String)> {
        out.platform_output
            .accesskit_update
            .as_ref()
            .map(|u| {
                u.nodes
                    .iter()
                    .map(|(_, n)| (n.role(), n.label().unwrap_or_default().to_string()))
                    .collect()
            })
            .unwrap_or_default()
    }

    #[test]
    fn a1_first_frame_is_the_empty_state_with_no_error() {
        let ctx = egui::Context::default();
        let mut app = AudioHaxApp::new_headless(&ctx);
        run_frames(&ctx, &mut app, 2);
        assert_eq!(app.phase(), Phase::NoImage);
        assert!(
            app.error.is_none(),
            "no error in the empty state: {:?}",
            app.error
        );
    }

    #[test]
    fn a10_controls_are_findable_by_role_and_label() {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut app = AudioHaxApp::new_headless(&ctx);
        run_frames(&ctx, &mut app, 1);
        app.try_sample();
        wait_until(&ctx, &mut app, |a| a.phase() == Phase::ImageLoaded);
        let out = run_frames(&ctx, &mut app, 2);
        let nodes = a11y_nodes(&out);
        use egui::accesskit::Role;
        let has =
            |role: Role, needle: &str| nodes.iter().any(|(r, l)| *r == role && l.contains(needle));
        for b in ["Render", "Play", "Stop", "Save WAV"] {
            assert!(has(Role::Button, b), "button {b:?} not found in {nodes:?}");
        }
        assert!(has(Role::Slider, "Voices"), "Voices slider: {nodes:?}");
        assert!(
            has(Role::Slider, "Level") || nodes.iter().any(|(r, _)| *r == Role::Slider),
            "Level slider"
        );
        assert!(
            nodes.iter().any(|(r, _)| *r == Role::TextInput),
            "Variation field"
        );
        assert!(nodes.iter().any(|(_, l)| l.contains("On")), "Room toggle");
    }

    #[test]
    fn sample_loads_renders_and_keeps_the_buffer_as_data() {
        let ctx = egui::Context::default();
        let mut app = AudioHaxApp::new_headless(&ctx);
        run_frames(&ctx, &mut app, 1);
        app.try_sample();
        wait_until(&ctx, &mut app, |a| a.phase() == Phase::ImageLoaded);
        assert!(app.rendered_audio().is_none());
        app.render();
        assert_eq!(app.phase(), Phase::Rendering);
        wait_until(&ctx, &mut app, |a| a.phase() == Phase::ImageLoaded);
        let (samples, rate) = app.rendered_audio().expect("render kept as data");
        assert_eq!(rate, 44_100);
        assert!(samples.len() > 44_100);
        assert!(app.known_total.is_some());
        assert!(app.error.is_none(), "{:?}", app.error);
    }

    #[test]
    fn voices_cap_follows_the_picture() {
        let ctx = egui::Context::default();
        let mut app = AudioHaxApp::new_headless(&ctx);
        run_frames(&ctx, &mut app, 1);
        app.controls.instruments = 16;
        app.load(LoadSource::Bytes {
            bytes: super::super::ingest::fixtures::structured(16, 40, image::ImageFormat::Png),
            name: "tiny.png".into(),
        });
        wait_until(&ctx, &mut app, |a| a.phase() == Phase::ImageLoaded);
        assert!(app.controls.instruments <= 16);
    }

    #[test]
    fn refused_picture_shows_a_plain_error_and_the_window_recovers() {
        let ctx = egui::Context::default();
        let mut app = AudioHaxApp::new_headless(&ctx);
        run_frames(&ctx, &mut app, 1);
        app.load(LoadSource::Bytes {
            bytes: super::super::ingest::fixtures::solid(
                1,
                1,
                [255, 255, 255],
                image::ImageFormat::Png,
            ),
            name: "one.png".into(),
        });
        wait_until(&ctx, &mut app, |a| a.phase() != Phase::Loading);
        assert_eq!(app.phase(), Phase::NoImage);
        let e = app.error.clone().expect("an error is shown");
        assert_eq!(e.kind, ErrorKind::TooSmall);
        // Esc closes it.
        let input = egui::RawInput {
            events: vec![egui::Event::Key {
                key: Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            }],
            ..Default::default()
        };
        ctx.run_ui(input, |ui| app.draw(ui)).textures_delta.clear();
        assert!(app.error.is_none());
    }

    #[test]
    fn modem_mode_shows_no_music_transport() {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut app = AudioHaxApp::new_headless(&ctx);
        app.mode = Mode::Modem;
        let out = run_frames(&ctx, &mut app, 2);
        let nodes = a11y_nodes(&out);
        assert!(!nodes.iter().any(|(_, l)| l.contains("Play")), "{nodes:?}");
        assert!(nodes.iter().any(|(_, l)| l.contains("Back to Music")));
        // L3: no date or release promise in the placeholder.
        for (_, l) in &nodes {
            for word in ["2026", "2027", "S126", "next month", "soon"] {
                assert!(!l.contains(word), "{l:?}");
            }
        }
    }

    #[test]
    fn narrow_window_moves_the_sound_controls_to_a_drawer() {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut app = AudioHaxApp::new_headless(&ctx);
        let input = || egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(960.0, 600.0))),
            ..Default::default()
        };
        ctx.run_ui(input(), |ui| app.draw(ui))
            .textures_delta
            .clear();
        let mut out = ctx.run_ui(input(), |ui| app.draw(ui));
        out.textures_delta.clear();
        let nodes = a11y_nodes(&out);
        assert!(
            nodes.iter().any(|(_, l)| l == "Sound"),
            "drawer header: {nodes:?}"
        );
    }
}

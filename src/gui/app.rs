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
use super::state::{
    any_modified, enablement, is_modified, loop_enablement, output_notice, request_with,
    reset_knob, v2_enablement, voices_max, AdvancedControls, AdvancedSpecs, Baseline, Control,
    Controls, EverydaySpecs, Knob, OutputKind, Phase, KNOBS,
};
use super::text::{
    sanitize, sanitize_full, suggest_wav_name, ErrorKind, UserError, FILE_NAME_MAX, PORT_NAME_MAX,
};
use super::theme::{focus_ring, selected_outline, Palette};
use crate::render::{CancelToken, RenderOutput, RenderRequest};
use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontId, Id, Key, KeyboardShortcut, Layout,
    Modifiers, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, TextStyle, TextureHandle,
    TextureOptions, Vec2,
};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// After this long, a running job shows "Still working…" and a Cancel button (SH T-2).
pub const STILL_WORKING_AFTER: Duration = Duration::from_secs(3);

/// The file extensions the Open dialog offers: the formats [`super::ingest::allowed_format`]
/// accepts. No WebP (lead ruling L-16). The bytes are sniffed anyway; this only filters the list.
pub const OPEN_DIALOG_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "bmp", "tif", "tiff"];

/// `s123-SH2` N-2: `rfd` returns `None` both when the person cancels the Open dialog and when no
/// dialog could be shown at all (a Linux desktop with no portal FileChooser and no `zenity`). A
/// `None` this fast is taken to be the second case. It is a heuristic: a person who cancels within
/// this time sees the message too.
pub const DIALOG_FAILURE_THRESHOLD: Duration = Duration::from_millis(150);

/// N-2: did the Open dialog probably fail to appear, given that it returned no file after
/// `elapsed`?
pub fn dialog_probably_failed(elapsed: Duration) -> bool {
    elapsed < DIALOG_FAILURE_THRESHOLD
}

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

/// What identifies a rendered piece: the picture loaded, the preset, and the request.
#[derive(Debug, Clone, PartialEq)]
struct PieceKey {
    image_seq: u64,
    /// S124: the preset the piece was made with (its mappings are part of the piece).
    preset: &'static str,
    request: RenderRequest,
}

/// S124 (`s124-UI` § 6): the Save fallback shown when no Save window could be opened.
pub const SAVE_NO_DIALOG_HEADLINE: &str =
    "AudioHax couldn't open a Save window on this system. You \
    can still keep your piece — type or paste a full file path below and AudioHax will save there.";

/// S124 (`s124-UI` § 1.4 / P9): the quiet note under a shelf of one preset.
pub const ONE_PRESET_NOTE: &str = "More presets are coming.";

/// S124 (`s124-UI` § 1.5 / P5): the hint shown while the held render was made with another preset.
pub const STALE_PRESET_HINT: &str = "Re-render to hear the new preset.";

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
    // ── S124 ──
    /// The Advanced controls behind "More".
    advanced: AdvancedControls,
    /// The Advanced rows read from the schema (L-31: no sound bank).
    adv_specs: Result<AdvancedSpecs, String>,
    /// The selected preset's id (always one; `s124-UI` § 1.3).
    preset_id: &'static str,
    /// What the selected preset defines for every control (A6).
    baseline: Baseline,
    /// Is "More" open (persisted per viewer, `s124-UI` § 2.4)?
    more_open: bool,
    /// Write the preset choice and the "More" state to the config dir (only the real window).
    persist: bool,
    /// The typed-path Save fallback, while shown (`s124-UI` § 6).
    save_fallback: Option<String>,
}

impl AudioHaxApp {
    /// Build the window: apply the palette, parse the embedded mappings, read the Everyday rows,
    /// start the worker controller, and offer the Quick tour on the very first launch.
    pub fn new(ctx: &egui::Context) -> Self {
        let mut app = Self::new_headless(ctx);
        if first_run_and_mark() {
            app.tour_step = Some(0);
        }
        // S124 (`s124-UI` § 1.3, § 2.4): the last preset and the "More" state, per viewer. A stored
        // id that is no longer shipped falls back to the default without an error.
        app.persist = true;
        if let Some(id) = read_setting(PRESET_SETTING_FILE) {
            if let Ok(found) = crate::presets::find(&id) {
                app.select_preset(&found.id);
            }
        }
        app.more_open = read_setting(MORE_SETTING_FILE).as_deref() == Some("open");
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
        let adv_specs = AdvancedSpecs::from_schema().map_err(|e| e.to_string());
        let (preset_id, baseline) = match crate::presets::default_preset() {
            Ok(d) => (
                d.id.as_str(),
                preset_baseline(d).unwrap_or_else(|_| default_baseline()),
            ),
            Err(_) => ("default", default_baseline()),
        };
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
            advanced: baseline.advanced,
            adv_specs,
            preset_id,
            baseline,
            more_open: false,
            persist: false,
            save_fallback: None,
        }
    }

    /// The request the controls describe: the Everyday four plus "More" (S124).
    fn request(&self) -> RenderRequest {
        request_with(&self.controls, &self.advanced)
    }

    /// The selected preset's id.
    ///
    /// TEST SEAM (S124): read-only.
    #[doc(hidden)]
    pub fn preset_id(&self) -> &str {
        self.preset_id
    }

    /// Select a preset (the panel's click, or a restored choice). Every control takes the preset's
    /// value EXCEPT Variation, which is kept with the picture so the same picture can be heard
    /// under another preset (`s124-UI` § 1.5, P7). An id not in the manifest changes nothing.
    ///
    /// Slice 1 (L-28): every shipped preset uses the built-in mappings (the manifest refuses any
    /// other), which are the mappings the worker was started with, so a switch needs no new table.
    pub fn select_preset(&mut self, id: &str) {
        let Ok(preset) = crate::presets::find(id) else {
            return;
        };
        let Ok(base) = preset_baseline(preset) else {
            return;
        };
        let seed = self.controls.seed;
        self.preset_id = preset.id.as_str();
        self.baseline = base;
        self.controls = Controls {
            seed,
            ..base.controls
        };
        self.advanced = base.advanced;
        if let Some(pic) = self.picture.as_ref() {
            let pr = pic.loaded.image.probe();
            let schema_max = self
                .specs
                .as_ref()
                .map(|s| s.voices_schema_max())
                .unwrap_or(16);
            self.controls
                .clamp_voices(voices_max(schema_max, pr.width, pr.height));
        }
        if self.persist {
            write_setting(PRESET_SETTING_FILE, self.preset_id);
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

    /// Is the scan-bar overlay shown (View ▸ Show scan bar, the V key)?
    ///
    /// TEST SEAM (fix round, `s123-05` TODO-5): read-only.
    #[doc(hidden)]
    pub fn scan_bar_shown(&self) -> bool {
        self.show_scan_bar
    }

    /// The error the window is showing, if any.
    ///
    /// TEST SEAM (fix round): read-only.
    #[doc(hidden)]
    pub fn shown_error(&self) -> Option<&UserError> {
        self.error.as_ref()
    }

    /// Choose a MIDI port as the output by its enumeration index and raw OS name, as picking it in
    /// the Output selector does. Only the SELECTION is set; no port is opened, and a Play with a
    /// name that is not in a real enumeration fails as "port gone", exactly as a vanished port
    /// does.
    ///
    /// TEST SEAM (fix round, `s123-05` TODO-3/TODO-4): a headless test cannot create a MIDI port,
    /// so this lets it put the window in the "MIDI output" state and read what is drawn.
    #[doc(hidden)]
    pub fn select_midi_output(&mut self, index: usize, raw_name: String) {
        self.output = OutputSel::Midi(index, raw_name);
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
            preset: self.preset_id,
            request: self.request(),
        })
    }

    fn geometry(&self) -> Option<ScanGeometry> {
        self.picture.as_ref().map(|p| {
            let probe = p.loaded.image.probe();
            let req = self.request();
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
        let mut dialog = rfd::FileDialog::new().add_filter("Pictures", OPEN_DIALOG_EXTENSIONS);
        if let Some(dir) = directories::UserDirs::new().and_then(|d| {
            d.picture_dir()
                .map(|p| p.to_path_buf())
                .or_else(|| Some(d.home_dir().to_path_buf()))
        }) {
            dialog = dialog.set_directory(dir);
        }
        let asked = Instant::now();
        match dialog.pick_file() {
            Some(path) => self.open_path(path),
            None => {
                if dialog_probably_failed(asked.elapsed()) {
                    self.error = Some(UserError::new(
                        ErrorKind::NoFileDialog,
                        format!(
                            "the file dialog returned no file after {:?}, under the {:?} \
                             threshold for a person cancelling it",
                            asked.elapsed(),
                            DIALOG_FAILURE_THRESHOLD
                        ),
                    ));
                }
            }
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
        let request = self.request();
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
        let request = self.request();
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
            .set_file_name(suggested.clone());
        let dir = default_save_dir(self.last_save_dir.as_deref());
        if let Some(dir) = dir.clone() {
            dialog = dialog.set_directory(dir);
        }
        let asked = Instant::now();
        let Some(path) = dialog.save_file() else {
            // S124 (`s124-UI` § 6): a dialog that returned at once probably never appeared (the same
            // N-2 heuristic as Open). Offer the typed-path fallback so Save is not a dead button. A
            // person who cancels within the threshold sees it too, and can press Cancel.
            if dialog_probably_failed(asked.elapsed()) {
                let prefill = dir
                    .map(|d| d.join(&suggested))
                    .unwrap_or_else(|| PathBuf::from(&suggested));
                self.save_fallback = Some(prefill.display().to_string());
            }
            return;
        };
        self.save_chosen_path(path);
    }

    /// The typed-path Save fallback (`s124-UI` § 6): the designer's headline, a path field
    /// pre-filled with the default folder and the suggested name, Save and Cancel. Save goes
    /// through [`Self::save_chosen_path`], so `.wav` is added, the open picture is refused (V-4)
    /// and an existing file is confirmed (V-2) exactly as after a dialog.
    fn save_fallback_panel(&mut self, ui: &mut egui::Ui) {
        let p = self.palette;
        let Some(mut text) = self.save_fallback.clone() else {
            return;
        };
        ui.label(RichText::new(SAVE_NO_DIALOG_HEADLINE).color(p.warn));
        let mut save = false;
        let mut cancel = false;
        ui.horizontal(|ui| {
            let r = ui.add(
                egui::TextEdit::singleline(&mut text)
                    .desired_width(520.0)
                    .char_limit(4096),
            );
            focus_ring(ui, &r, &p);
            let enter = r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
            let b = ui.add_enabled(self.live(Control::SaveWav), egui::Button::new("Save"));
            focus_ring(ui, &b, &p);
            save = (b.clicked() || enter) && !text.trim().is_empty();
            let c = ui.button("Cancel");
            focus_ring(ui, &c, &p);
            cancel = c.clicked();
        });
        if cancel {
            self.save_fallback = None;
        } else if save {
            self.save_fallback = None;
            self.save_chosen_path(PathBuf::from(text.trim()));
        } else {
            self.save_fallback = Some(text);
        }
    }

    /// Show the typed-path Save fallback with `prefill`, as a failed Save dialog does.
    ///
    /// TEST SEAM (S124): a headless test cannot make `rfd` fail.
    #[doc(hidden)]
    pub fn show_save_fallback(&mut self, prefill: String) {
        self.save_fallback = Some(prefill);
    }

    /// Is the typed-path Save fallback showing?
    ///
    /// TEST SEAM (S124): read-only.
    #[doc(hidden)]
    pub fn save_fallback_shown(&self) -> bool {
        self.save_fallback.is_some()
    }

    /// Everything Save WAV does AFTER the dialog returned `path`: add `.wav` if there is no
    /// extension, refuse the open picture's own file (V-4), ask before replacing an existing file
    /// where the window confirms itself (V-2), else save (re-rendering first when the controls
    /// changed since the last Render).
    ///
    /// TEST SEAM (fix round, `s123-05` TODO-2): this is the code path the dialog feeds, split out
    /// unchanged so a headless test can hand it the path a dialog would have returned.
    #[doc(hidden)]
    pub fn save_chosen_path(&mut self, mut path: PathBuf) {
        if !self.live(Control::SaveWav) {
            return;
        }
        let Some(pic) = self.picture.as_ref() else {
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
            if self.save_fallback.is_some() {
                egui::Panel::bottom("save_fallback")
                    .frame(
                        egui::Frame::new()
                            .fill(p.raised)
                            .inner_margin(egui::Margin::symmetric(12, 8)),
                    )
                    .show(ui, |ui| self.save_fallback_panel(ui));
            }
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
                                visible_scroll_bars(ui);
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
                        visible_scroll_bars(ui);
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
                for (value, text) in [(Mode::Music, "Music"), (Mode::Modem, "Modem")] {
                    let r = ui.selectable_value(&mut mode, value, text);
                    selected_outline(ui, &r, mode == value, &p);
                    focus_ring(ui, &r, &p);
                }
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
                        focus_ring(ui, &r.response, &p);
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
        let output_kind = match self.output {
            OutputSel::Speakers => OutputKind::Speakers,
            OutputSel::Midi(..) => OutputKind::Midi,
        };

        if !compact {
            ui.label(
                RichText::new("SOUND")
                    .color(p.text_muted)
                    .size(13.0)
                    .strong(),
            );
            ui.add_space(4.0);
        }
        // S124 (`s124-UI` § 1.1, P1): the preset surface is the FIRST group, above "Music".
        self.preset_panel(ui);
        ui.add_space(8.0);
        section_label(ui, p, "Music");
        ui.add_enabled_ui(live, |ui| {
            // Variation (seed): empty = the picture chooses (image-derived seed).
            ui.horizontal(|ui| {
                self.knob_marker(ui, Knob::Seed);
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
                // UI2 F7: the glyph at heading size, and a hover text that says what it does.
                let dice = ui
                    .button(RichText::new("🎲").text_style(TextStyle::Heading))
                    .on_hover_text("Pick a different variation");
                focus_ring(ui, &dice, &p);
                if dice.clicked() {
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
                self.knob_marker(ui, Knob::Instruments);
                let l = ui.label(&specs.instruments.label);
                let r = ui
                    .add(egui::Slider::new(&mut voices, 1..=vmax).integer())
                    .labelled_by(l.id)
                    .on_hover_text(EverydaySpecs::help(&specs.instruments));
                name_slider(&r, &specs.instruments.label, f64::from(voices));
                focus_ring(ui, &r, &p);
            });
            self.controls.instruments = voices as usize;
        })
        .response
        .on_disabled_hover_text(reason);

        ui.add_space(8.0);
        section_label(ui, p, "After the music");
        // L-20 / UI2 F3: Room and Level follow the same enablement as the other sound controls,
        // whatever the output, because `render()` applies them to Save WAV. A MIDI output adds
        // a one-line notice below them instead of greying them.
        ui.add_enabled_ui(live, |ui| {
            ui.horizontal(|ui| {
                self.knob_marker(ui, Knob::Reverb);
                let l = ui.label(&specs.reverb.label);
                let mut on = self.controls.reverb;
                for (value, text) in [(true, "On"), (false, "Off")] {
                    let r = ui
                        .selectable_value(&mut on, value, text)
                        .labelled_by(l.id)
                        .on_hover_text(EverydaySpecs::help(&specs.reverb));
                    selected_outline(ui, &r, on == value, &p);
                    focus_ring(ui, &r, &p);
                }
                self.controls.reverb = on;
            });
            let (gmin, gmax, gstep) = specs.gain_range();
            ui.horizontal(|ui| {
                self.knob_marker(ui, Knob::Gain);
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
                focus_ring(ui, &r, &p);
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
        .on_disabled_hover_text(reason);
        if let Some(notice) = output_notice(output_kind) {
            ui.label(RichText::new(notice).color(p.warn).size(13.0));
        }

        ui.add_space(8.0);
        // S124 (`s124-UI` § 2): the Advanced disclosure, closed on a fresh config, its state kept
        // per viewer. Holds the schema's Advanced rows except the sound bank (L-31).
        let more = egui::CollapsingHeader::new("More")
            .default_open(self.more_open)
            .show(ui, |ui| self.advanced_controls(ui, live, reason));
        focus_ring(ui, &more.header_response, &p);
        if more.header_response.clicked() {
            self.more_open = !self.more_open;
            if self.persist {
                write_setting(
                    MORE_SETTING_FILE,
                    if self.more_open { "open" } else { "closed" },
                );
            }
        }
    }

    /// S124 (`s124-UI` § 1): the preset panel. Pills for up to five presets, a drop-down for
    /// more; the selected preset's one-line description under it; "default" marks the default;
    /// "(modified)" and "Reset to preset" while any control differs from the preset (A6); locked
    /// with the sound controls while rendering or playing (P6).
    fn preset_panel(&mut self, ui: &mut egui::Ui) {
        let p = self.palette;
        let manifest = match crate::presets::manifest() {
            Ok(m) => m,
            Err(e) => {
                ui.colored_label(p.error, "The presets could not be read.")
                    .on_hover_text(e.to_string());
                return;
            }
        };
        let live = self.live(Control::SoundControls);
        let reason = enablement(Control::SoundControls, self.phase)
            .err()
            .unwrap_or("");
        let modified = any_modified(&self.controls, &self.advanced, &self.baseline);
        let mut chosen: &'static str = self.preset_id;
        let mut reset_all = false;
        // `s124-UI2` C3: the "default" tag marks the way back, so it shows only when the shelf
        // holds somewhere else to be (at one preset it would only repeat the pill's name).
        let tag_default = |id: &str| id == manifest.default && manifest.presets.len() > 1;
        egui::Frame::new()
            .fill(p.raised)
            .stroke(Stroke::new(1.0, Palette::with_alpha(p.control_border, 0.6)))
            .corner_radius(CornerRadius::same(10))
            .inner_margin(egui::Margin::same(12))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Preset").strong());
                    if modified {
                        ui.label(RichText::new("(modified)").color(p.text_muted));
                    }
                });
                ui.add_enabled_ui(live, |ui| {
                    if manifest.presets.len() <= 5 {
                        ui.horizontal_wrapped(|ui| {
                            for pr in &manifest.presets {
                                let r = ui
                                    .selectable_value(&mut chosen, pr.id.as_str(), &pr.name)
                                    .on_hover_text(&pr.description);
                                selected_outline(ui, &r, chosen == pr.id, &p);
                                focus_ring(ui, &r, &p);
                                if tag_default(&pr.id) {
                                    ui.label(
                                        RichText::new("default").color(p.text_muted).size(12.0),
                                    );
                                }
                            }
                        });
                    } else {
                        let shown = manifest
                            .presets
                            .iter()
                            .find(|x| x.id == chosen)
                            .map(|x| x.name.as_str())
                            .unwrap_or("");
                        let r = egui::ComboBox::from_id_salt("preset-combo")
                            .selected_text(shown)
                            .show_ui(ui, |ui| {
                                for pr in &manifest.presets {
                                    let tag = if tag_default(&pr.id) {
                                        " · default"
                                    } else {
                                        ""
                                    };
                                    ui.selectable_value(
                                        &mut chosen,
                                        pr.id.as_str(),
                                        format!("{}{tag}", pr.name),
                                    );
                                    ui.label(
                                        RichText::new(&pr.description)
                                            .color(p.text_muted)
                                            .size(12.0),
                                    );
                                }
                            });
                        focus_ring(ui, &r.response, &p);
                    }
                })
                .response
                .on_disabled_hover_text(reason);
                if let Some(pr) = manifest.presets.iter().find(|x| x.id == self.preset_id) {
                    ui.label(
                        RichText::new(&pr.description)
                            .color(p.text_muted)
                            .size(13.0),
                    );
                }
                if manifest.presets.len() == 1 {
                    ui.label(
                        // `s124-UI2` C2: `text_muted`, AA on the `raised` card in both palettes
                        // (`text_faint` is a `bg`-only token: 3.69 : 1 on dark `raised`).
                        RichText::new(ONE_PRESET_NOTE)
                            .color(p.text_muted)
                            .size(12.0),
                    );
                }
                if modified {
                    let b = ui
                        .add_enabled(live, egui::Button::new("Reset to preset"))
                        .on_hover_text("Put every control back to this preset's value.")
                        .on_disabled_hover_text(reason);
                    focus_ring(ui, &b, &p);
                    reset_all = b.clicked();
                }
            });
        if chosen != self.preset_id {
            self.select_preset(chosen);
        }
        if reset_all {
            for k in KNOBS {
                reset_knob(*k, &mut self.controls, &mut self.advanced, &self.baseline);
            }
            self.sync_seed_text();
        }
    }

    /// Keep the Variation text field in step with the seed after a reset.
    fn sync_seed_text(&mut self) {
        self.seed_text = self
            .controls
            .seed
            .map(|s| s.to_string())
            .unwrap_or_default();
    }

    /// A6 (`s124-UI` § 1.6): the ● before a control's label while it differs from the preset;
    /// clicking it puts that one control back.
    ///
    /// The marker's slot is ALWAYS allocated — one fixed-size rect, one egui auto id — whether or
    /// not the ● shows (lead ruling L-42, finding F-6). egui holds keyboard focus and a drag by the
    /// widget's id, and a row's later widgets take their auto ids in order; a marker that only
    /// allocated while modified shifted the control's id the moment it left its preset value, so
    /// a keyboard user's second ← (or a mouse drag's next frame) went nowhere. The fixed slot also
    /// keeps the label from jumping sideways when the ● appears.
    fn knob_marker(&mut self, ui: &mut egui::Ui, knob: Knob) {
        let modified = is_modified(knob, &self.controls, &self.advanced, &self.baseline);
        let size = Vec2::new(MARKER_SLOT_WIDTH, ui.spacing().interact_size.y);
        let sense = if modified {
            Sense::click()
        } else {
            Sense::hover()
        };
        let (rect, r) = ui.allocate_exact_size(size, sense);
        if !modified {
            return;
        }
        let p = self.palette;
        let enabled = ui.is_enabled();
        r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, "●"));
        // The painter of a disabled `Ui` already fades what it paints.
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            "●",
            TextStyle::Small.resolve(ui.style()),
            p.accent_2,
        );
        let name = crate::presets::find(self.preset_id)
            .map(|x| x.name.clone())
            .unwrap_or_else(|_| self.preset_id.to_string());
        let r = r.on_hover_text(format!("Changed from the {name} preset — click to reset"));
        focus_ring(ui, &r, &p);
        if r.clicked() {
            reset_knob(knob, &mut self.controls, &mut self.advanced, &self.baseline);
            if knob == Knob::Seed {
                self.sync_seed_text();
            }
        }
    }

    /// S124 (`s124-UI` § 2.1–2.2): the Advanced controls inside "More", in the Music then "After
    /// the music" split. Dependents are disabled with their reason inline, never refused later.
    fn advanced_controls(&mut self, ui: &mut egui::Ui, live: bool, reason: &str) {
        let p = self.palette;
        let specs = match &self.adv_specs {
            Ok(s) => s.clone(),
            Err(e) => {
                ui.colored_label(p.error, "These settings could not be built.")
                    .on_hover_text(e.clone());
                return;
            }
        };
        section_label(ui, p, "Music");
        ui.add_enabled_ui(live, |ui| {
            // Expression.
            ui.horizontal(|ui| {
                self.knob_marker(ui, Knob::Expression);
                let l = ui.label(&specs.expression.label);
                let mut on = self.advanced.expression;
                for (value, text) in [(true, "On"), (false, "Off")] {
                    let r = ui
                        .selectable_value(&mut on, value, text)
                        .labelled_by(l.id)
                        .on_hover_text(EverydaySpecs::help(&specs.expression));
                    selected_outline(ui, &r, on == value, &p);
                    focus_ring(ui, &r, &p);
                }
                self.advanced.expression = on;
            });
            // Expression detail: live only with Expression on.
            let v2_ok = v2_enablement(&self.advanced);
            ui.horizontal(|ui| {
                self.knob_marker(ui, Knob::V2);
                let l = ui.label(&specs.v2.label);
                ui.add_enabled_ui(v2_ok.is_ok(), |ui| {
                    let mut spec = self.advanced.v2.unwrap_or_default();
                    // The three terms, each toggled on its own (`V2_TERMS` order).
                    for (term, flag) in [
                        ("e3", &mut spec.e3),
                        ("e2", &mut spec.e2),
                        ("ta", &mut spec.ta),
                    ] {
                        let r = ui
                            .selectable_label(*flag, term)
                            .labelled_by(l.id)
                            .on_hover_text(EverydaySpecs::help(&specs.v2));
                        describe_disabled(ui, &r, v2_ok);
                        selected_outline(ui, &r, *flag, &p);
                        focus_ring(ui, &r, &p);
                        if r.clicked() {
                            *flag = !*flag;
                        }
                    }
                    self.advanced.v2 = if spec.is_empty() { None } else { Some(spec) };
                })
                .response
                .on_disabled_hover_text(v2_ok.err().unwrap_or(""));
            });
            if let Err(why) = v2_ok {
                ui.label(RichText::new(why).color(p.text_muted).size(12.0));
            }
            // Scan width.
            let (tmin, tmax, tstep) = AdvancedSpecs::range(&specs.thickness, (0.01, 1.0, 0.01));
            ui.horizontal(|ui| {
                self.knob_marker(ui, Knob::Thickness);
                let l = ui.label(&specs.thickness.label);
                let mut t = self.advanced.thickness;
                let r = ui
                    .add(
                        egui::Slider::new(&mut t, (tmin as f32)..=(tmax as f32))
                            .step_by(tstep)
                            .fixed_decimals(2),
                    )
                    .labelled_by(l.id)
                    .on_hover_text(EverydaySpecs::help(&specs.thickness));
                name_slider(&r, &specs.thickness.label, f64::from(t));
                focus_ring(ui, &r, &p);
                self.advanced.thickness = t;
            });
            // Scan detail.
            let (smin, smax, _) = AdvancedSpecs::range(&specs.steps, (1.0, 200.0, 1.0));
            ui.horizontal(|ui| {
                self.knob_marker(ui, Knob::Steps);
                let l = ui.label(&specs.steps.label);
                let mut st = self.advanced.steps as u32;
                let r = ui
                    .add(egui::Slider::new(&mut st, (smin as u32)..=(smax as u32)).integer())
                    .labelled_by(l.id)
                    .on_hover_text(EverydaySpecs::help(&specs.steps));
                name_slider(&r, &specs.steps.label, f64::from(st));
                focus_ring(ui, &r, &p);
                self.advanced.steps = st as usize;
            });
        })
        .response
        .on_disabled_hover_text(reason);

        ui.add_space(6.0);
        section_label(ui, p, "After the music");
        ui.add_enabled_ui(live, |ui| {
            let loop_ok = loop_enablement(&self.controls);
            ui.horizontal(|ui| {
                self.knob_marker(ui, Knob::Loop);
                let l = ui.label(&specs.loop_cue.label);
                ui.add_enabled_ui(loop_ok.is_ok(), |ui| {
                    let mut on = self.advanced.loop_cue;
                    for (value, text) in [(true, "On"), (false, "Off")] {
                        let r = ui
                            .selectable_value(&mut on, value, text)
                            .labelled_by(l.id)
                            .on_hover_text(EverydaySpecs::help(&specs.loop_cue));
                        describe_disabled(ui, &r, loop_ok);
                        selected_outline(ui, &r, on == value, &p);
                        focus_ring(ui, &r, &p);
                    }
                    self.advanced.loop_cue = on;
                })
                .response
                .on_disabled_hover_text(loop_ok.err().unwrap_or(""));
            });
            if let Err(why) = loop_ok {
                ui.label(RichText::new(why).color(p.text_muted).size(12.0));
            }
        })
        .response
        .on_disabled_hover_text(reason);
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
            let r = ui
                .add_enabled(self.live(Control::Render), egui::Button::new(render_text))
                .on_hover_text("Make the whole piece now, so Save WAV is instant.")
                .on_disabled_hover_text(
                    enablement(Control::Render, self.phase).err().unwrap_or(""),
                );
            focus_ring(ui, &r, &p);
            r.clicked().then(|| self.render());

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
            let r = ui
                .add_enabled(play_live, play_btn)
                .on_hover_text("Hear it now (Space)")
                .on_disabled_hover_text(enablement(Control::Play, self.phase).err().unwrap_or(""));
            focus_ring(ui, &r, &p);
            r.clicked().then(|| self.play());

            let r = ui
                .add_enabled(self.live(Control::Stop), egui::Button::new("⏹ Stop"))
                .on_hover_text("Stop (Space or Esc)")
                .on_disabled_hover_text(enablement(Control::Stop, self.phase).err().unwrap_or(""));
            focus_ring(ui, &r, &p);
            r.clicked().then(|| self.stop());

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

            // S124 (`s124-UI` § 1.5, P5): the held render was made with another preset.
            let stale = self
                .last_render
                .as_ref()
                .is_some_and(|(k, _)| k.preset != self.preset_id);
            if stale && matches!(self.phase, Phase::ImageLoaded | Phase::Stopped) {
                ui.label(RichText::new(STALE_PRESET_HINT).color(p.text_muted));
            }

            // Progress and the abandon-style Cancel (SH T-2).
            if let Some(c) = self.controller.as_ref() {
                let preparing_play =
                    self.phase == Phase::Playing && self.play.total_steps.is_none();
                let working =
                    matches!(self.phase, Phase::Loading | Phase::Rendering) || preparing_play;
                if working {
                    ui.add(egui::Spinner::new().color(p.accent));
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
                        let r = ui.button("Cancel").on_hover_text(
                            "Stop waiting for this one. It finishes in the background; a new \
                             picture waits until it has.",
                        );
                        focus_ring(ui, &r, &p);
                        if r.clicked() {
                            self.cancel_job();
                        }
                    }
                }
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let r = ui
                    .add_enabled(self.live(Control::SaveWav), egui::Button::new("Save WAV…"))
                    .on_hover_text("Keep the piece as a WAV file (Ctrl/Cmd+S)")
                    .on_disabled_hover_text(
                        enablement(Control::SaveWav, self.phase).err().unwrap_or(""),
                    );
                focus_ring(ui, &r, &p);
                r.clicked().then(|| self.save_wav());
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
                // UI2 F5: idle tint 28 % (was 22 %, faint on a saturated picture); 50 % sounding.
                let alpha = if playing && self.play.step.is_some() {
                    0.50
                } else {
                    0.28
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
        let r = child.add_enabled(
            live,
            egui::Button::new("Open a picture…").min_size(Vec2::new(180.0, 30.0)),
        );
        focus_ring(&child, &r, &p);
        if r.clicked() {
            self.open_dialog();
        }
        let r = child.add_enabled(
            live,
            egui::Button::new("Try a sample").min_size(Vec2::new(180.0, 30.0)),
        );
        focus_ring(&child, &r, &p);
        if r.clicked() {
            self.try_sample();
        }
        if self.phase == Phase::Loading {
            child.add_space(6.0);
            child.horizontal(|ui| {
                ui.add(egui::Spinner::new().color(p.accent));
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
        // UI2 F6: the backdrop is the palette's ground, not egui's stock black.
        let backdrop = Palette::with_alpha(p.bg, 0.6);
        if let Some(err) = self.error.clone() {
            let r = egui::Modal::new(Id::new("error-modal"))
                .backdrop_color(backdrop)
                .show(ctx, |ui| {
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
            let r = egui::Modal::new(Id::new("overwrite-modal"))
                .backdrop_color(backdrop)
                .show(ctx, |ui| {
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
            let r = egui::Modal::new(Id::new("tour-modal"))
                .backdrop_color(backdrop)
                .show(ctx, |ui| {
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
            let r = egui::Modal::new(Id::new("shortcuts-modal"))
                .backdrop_color(backdrop)
                .show(ctx, |ui| {
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
            let r = egui::Modal::new(Id::new("about-modal"))
                .backdrop_color(backdrop)
                .show(ctx, |ui| {
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
/// S124: a preset's control baseline (its settings over clap's defaults).
fn preset_baseline(
    preset: &crate::presets::Preset,
) -> Result<Baseline, crate::presets::PresetError> {
    let args = crate::presets::preset_render_args(preset)?;
    Ok(Baseline::from_request(&RenderRequest::from(&args)))
}

/// The CLI defaults as a baseline (used only if the manifest cannot be read).
fn default_baseline() -> Baseline {
    Baseline {
        controls: Controls::default(),
        advanced: AdvancedControls::default(),
    }
}

/// F-6 (L-42): the width of the A6 marker's fixed slot, in points — wide enough for the small ●.
const MARKER_SLOT_WIDTH: f32 = 12.0;

/// AD5 (`s124-UI` § 2.6; lead ruling L-42, finding F-4): a disabled dependent exposes WHY it is
/// disabled to `accesskit` as the control's description, so a screen-reader user hears the same
/// reason the inline label shows. Nothing is written while the control is enabled.
fn describe_disabled(ui: &egui::Ui, r: &egui::Response, enablement: Result<(), &str>) {
    if let Err(why) = enablement {
        let _ = ui
            .ctx()
            .accesskit_node_builder(r.id, |node| node.set_description(why));
    }
}

/// S124: the per-viewer file holding the selected preset's id.
const PRESET_SETTING_FILE: &str = "gui-preset";
/// S124: the per-viewer file holding "More"'s open/closed state.
const MORE_SETTING_FILE: &str = "gui-more-open";

/// Read a tiny per-viewer setting from the config dir: a regular file of at most 64 bytes of
/// UTF-8, trimmed. Anything else reads as absent (never an error, never a large read).
fn read_setting(name: &str) -> Option<String> {
    let dirs = directories::ProjectDirs::from("", "", "AudioHax")?;
    let path = dirs.config_dir().join(name);
    let meta = std::fs::metadata(&path).ok()?;
    if !meta.is_file() || meta.len() > 64 {
        return None;
    }
    std::fs::read_to_string(&path)
        .ok()
        .map(|s| s.trim().to_string())
}

/// Write a tiny per-viewer setting; failures are ignored (the default applies next launch).
///
/// The path gets the same regular-file check before `open()` that [`read_setting`] gives it (lead
/// ruling L-42, finding F-5): this runs on the UI thread, and opening a FIFO for writing blocks
/// until a reader appears, so a pipe (or a device, or a folder) at a settings path would freeze a
/// click on "More". Anything there that is not a regular file is left alone and nothing is
/// written; an absent file is created. Residual, as for `read_setting` and
/// `mapping_loader::read_mappings_file`: a regular file swapped for a FIFO between the check and
/// the `open()` still blocks (that needs someone writing in the viewer's own config folder during
/// the click; the cure is a non-blocking open, which needs `libc` as a direct dependency).
fn write_setting(name: &str, value: &str) {
    if let Some(dirs) = directories::ProjectDirs::from("", "", "AudioHax") {
        write_setting_in(dirs.config_dir(), name, value);
    }
}

/// [`write_setting`] in an explicit folder (so the check can be tested without the viewer's
/// config folder).
fn write_setting_in(dir: &std::path::Path, name: &str, value: &str) {
    let path = dir.join(name);
    match std::fs::metadata(&path) {
        Ok(meta) if meta.is_file() => {}
        Ok(_) => return,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let _ = std::fs::create_dir_all(dir);
        }
        Err(_) => return,
    }
    let _ = std::fs::write(&path, format!("{value}\n"));
}

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

/// `s124-UI2` C1: solid scroll bars for the Sound panel. egui's default style floats the bar and
/// paints it fully transparent until the pointer is over the area, so an overflowing panel (the
/// "More" rows at the default window size) gave no sign that anything was below. A solid
/// (non-floating) bar is painted opaque whenever the content overflows, and only then. Applies to
/// `ui` and the widgets added to it after this call.
fn visible_scroll_bars(ui: &mut egui::Ui) {
    ui.style_mut().spacing.scroll = egui::style::ScrollStyle::solid();
}

#[cfg(test)]
mod tests {
    use super::*;

    /// L-42 / F-5: `write_setting` writes a regular file (and creates an absent one) but leaves a
    /// FIFO or a folder at the path alone — returning at once rather than blocking in `open()`.
    #[test]
    fn write_setting_never_opens_what_is_not_a_regular_file() {
        let base = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .unwrap_or_else(std::env::temp_dir);
        let dir = base
            .join("audiohax-settings-test-scratch")
            .join(format!("w-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        write_setting_in(&dir, "fresh", "open");
        assert_eq!(
            std::fs::read_to_string(dir.join("fresh")).expect("created"),
            "open\n"
        );
        write_setting_in(&dir, "fresh", "closed");
        assert_eq!(
            std::fs::read_to_string(dir.join("fresh")).expect("rewritten"),
            "closed\n"
        );
        std::fs::create_dir_all(dir.join("a-folder")).expect("dir");
        write_setting_in(&dir, "a-folder", "open");
        assert!(dir.join("a-folder").is_dir(), "a folder is left alone");
        #[cfg(unix)]
        {
            let fifo = dir.join("a-pipe");
            let made = std::process::Command::new("mkfifo")
                .arg(&fifo)
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            if made {
                let (tx, rx) = std::sync::mpsc::channel();
                let d = dir.clone();
                std::thread::spawn(move || {
                    write_setting_in(&d, "a-pipe", "open");
                    let _ = tx.send(());
                });
                let done = rx.recv_timeout(Duration::from_secs(10));
                if done.is_err() {
                    // Unblock the stuck writer so the test process can exit.
                    let _ = std::fs::read(&fifo);
                }
                assert!(done.is_ok(), "writing a setting blocked on a FIFO");
            } else {
                eprintln!("mkfifo unavailable: the FIFO case was not checked");
            }
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

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
    fn the_window_clamps_voices_to_the_pictures_bound() {
        // Renamed and strengthened in the fix round (`s123-05` TODO-6). The old
        // `voices_cap_follows_the_picture` asked for 16 voices on a 16-px picture and asserted
        // `<= 16`, which held with or without any clamp. Every admitted picture derives a bound
        // of exactly 16 (16-px floor = schema max, pinned by
        // `schema_voices_max_equals_the_admission_floor`), so a clamp can only be seen from a
        // value ABOVE 16. TWO paths enforce it: `on_loaded`'s `clamp_voices` call, and the Voices
        // slider itself (egui's default `SliderClamping::Always` clamps the bound value every
        // frame the sound panel is drawn). Removing either one alone leaves this green; removing
        // both leaves 40 (fix-round controls M07 and M07b).
        let ctx = egui::Context::default();
        let mut app = AudioHaxApp::new_headless(&ctx);
        run_frames(&ctx, &mut app, 1);
        app.controls.instruments = 40;
        app.load(LoadSource::Bytes {
            bytes: super::super::ingest::fixtures::structured(16, 40, image::ImageFormat::Png),
            name: "tiny.png".into(),
        });
        wait_until(&ctx, &mut app, |a| a.phase() == Phase::ImageLoaded);
        assert_eq!(app.controls.instruments, 16);
    }

    #[test]
    fn n2_only_a_dialog_that_returns_at_once_counts_as_failed() {
        assert!(dialog_probably_failed(Duration::ZERO));
        assert!(dialog_probably_failed(Duration::from_millis(149)));
        assert!(!dialog_probably_failed(Duration::from_millis(150)));
        assert!(!dialog_probably_failed(Duration::from_secs(4)));
        assert_eq!(DIALOG_FAILURE_THRESHOLD, Duration::from_millis(150));
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

    // ── S124 ─────────────────────────────────────────────────────────────────────────────────

    /// Every accesskit node's label OR value text (egui puts a plain label's words in its
    /// `TextRun` child's value, not in a label), as (role, text).
    fn a11y_text(out: &egui::FullOutput) -> Vec<(egui::accesskit::Role, String)> {
        out.platform_output
            .accesskit_update
            .as_ref()
            .map(|u| {
                u.nodes
                    .iter()
                    .map(|(_, n)| {
                        let t = n
                            .label()
                            .map(|l| l.to_string())
                            .or_else(|| n.value().map(|v| v.to_string()))
                            .unwrap_or_default();
                        (n.role(), t)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn s124_loaded(more_open: bool) -> (egui::Context, AudioHaxApp) {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut app = AudioHaxApp::new_headless(&ctx);
        app.more_open = more_open;
        run_frames(&ctx, &mut app, 1);
        app.try_sample();
        wait_until(&ctx, &mut app, |a| a.phase() == Phase::ImageLoaded);
        (ctx, app)
    }

    /// P1/P2/P9 + AD1 + L-31/L-33: the preset shelf shows the one `default` preset with its
    /// description and the "more are coming" note; "More" holds the five Advanced controls and
    /// no sound-bank or mappings picker.
    #[test]
    fn s124_preset_panel_and_more_show_what_the_manifest_and_schema_hold() {
        let (ctx, mut app) = s124_loaded(true);
        let out = run_frames(&ctx, &mut app, 2);
        let nodes = a11y_text(&out);
        let any = |needle: &str| nodes.iter().any(|(_, l)| l.contains(needle));
        assert_eq!(app.preset_id(), "default");
        assert!(any("Default"), "preset pill: {nodes:?}");
        assert!(any("AudioHax as it ships"), "description: {nodes:?}");
        assert!(any(ONE_PRESET_NOTE), "one-preset note: {nodes:?}");
        assert!(!any("(modified)"), "untouched controls are not modified");
        use egui::accesskit::Role;
        for slider in ["Scan width", "Scan detail"] {
            assert!(
                nodes
                    .iter()
                    .any(|(r, l)| *r == Role::Slider && l.contains(slider)),
                "{slider}: {nodes:?}"
            );
        }
        for label in ["Expression", "Expression detail", "Seamless loop"] {
            assert!(any(label), "{label}: {nodes:?}");
        }
        for absent in ["sound bank", ".sf2", "mapping file", "Load a mapping"] {
            assert!(
                !any(absent),
                "{absent} must not be in the window: {nodes:?}"
            );
        }
        assert!(
            !any("coming in the next update"),
            "the S123 placeholder is gone"
        );
        // The untouched window's request is still the CLI default render.
        assert_eq!(
            app.request(),
            request_with(&Controls::default(), &AdvancedControls::default())
        );
    }

    /// Every painted shape, with `Shape::Vec` flattened.
    fn painted(out: &egui::FullOutput) -> Vec<egui::epaint::Shape> {
        fn flat(s: &egui::epaint::Shape, acc: &mut Vec<egui::epaint::Shape>) {
            match s {
                egui::epaint::Shape::Vec(v) => v.iter().for_each(|x| flat(x, acc)),
                other => acc.push(other.clone()),
            }
        }
        let mut acc = Vec::new();
        for c in &out.shapes {
            flat(&c.shape, &mut acc);
        }
        acc
    }

    /// Opaque, bar-shaped rectangles painted within 40 px of the window's right edge: a vertical
    /// scroll bar's track or handle (a slider rail is horizontal; a separator is a line).
    fn scroll_bars_at_right_edge(out: &egui::FullOutput, width: f32) -> usize {
        painted(out)
            .iter()
            .filter(|s| match s {
                egui::epaint::Shape::Rect(r) => {
                    let w = r.rect.width();
                    w > 2.0
                        && w <= 12.0
                        && r.rect.height() > 40.0
                        && r.rect.max.x > width - 40.0
                        && r.fill.a() > 0
                }
                _ => false,
            })
            .count()
    }

    /// Run `n` frames at `w`×`h` with no pointer anywhere (nothing is hovered).
    fn frames_at(
        ctx: &egui::Context,
        app: &mut AudioHaxApp,
        w: f32,
        h: f32,
        n: usize,
    ) -> egui::FullOutput {
        let mut last = None;
        for _ in 0..n {
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(w, h))),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| app.draw(ui));
            out.textures_delta.clear();
            last = Some(out);
        }
        last.expect("at least one frame")
    }

    /// `s124-UI2` C1: at the window's own default size (1120×720) with "More" open, the Sound
    /// panel overflows and its scroll bar is PAINTED with nothing hovered, so the rows below the
    /// fold are announced. Same for the narrow window's drawer. CONTROL: a window tall enough
    /// that nothing overflows paints no bar (the detector is not matching some other shape).
    #[test]
    fn s124_the_sound_panel_scroll_bar_shows_without_hover() {
        let (ctx, mut app) = s124_loaded(true);
        let out = frames_at(&ctx, &mut app, 1120.0, 720.0, 3);
        assert!(
            scroll_bars_at_right_edge(&out, 1120.0) >= 1,
            "1120×720, More open: no visible scroll bar"
        );
        let out = frames_at(&ctx, &mut app, 960.0, 600.0, 3);
        assert!(
            scroll_bars_at_right_edge(&out, 960.0) >= 1,
            "narrow drawer: no visible scroll bar"
        );
        // A fresh window (a bar that was shown animates out over several frames).
        let (ctx, mut app) = s124_loaded(false);
        let out = frames_at(&ctx, &mut app, 1120.0, 2400.0, 10);
        assert_eq!(
            scroll_bars_at_right_edge(&out, 1120.0),
            0,
            "CONTROL: nothing overflows, so no bar"
        );
    }

    /// `s124-UI2` C2: "More presets are coming." is painted in `text_muted` (AA on the `raised`
    /// card in both palettes, `theme.rs` `a8_*`), not `text_faint` (3.69 : 1 on dark `raised`).
    #[test]
    fn s124_the_one_preset_note_is_painted_in_text_muted() {
        let (ctx, mut app) = s124_loaded(false);
        let out = run_frames(&ctx, &mut app, 2);
        let p = app.palette;
        let colours: Vec<Color32> = painted(&out)
            .iter()
            .filter_map(|s| match s {
                egui::epaint::Shape::Text(t) if t.galley.text() == ONE_PRESET_NOTE => {
                    Some(t.override_text_color.unwrap_or_else(|| {
                        t.galley
                            .job
                            .sections
                            .first()
                            .map(|s| s.format.color)
                            .unwrap_or(t.fallback_color)
                    }))
                }
                _ => None,
            })
            .collect();
        assert_eq!(colours, vec![p.text_muted], "the note's colour");
        assert_ne!(p.text_muted, p.text_faint);
    }

    /// `s124-UI2` C3: with one preset on the shelf, the pill "Default" is not followed by a
    /// redundant "default" tag (the tag marks the way back only when there is somewhere else).
    #[test]
    fn s124_a_one_preset_shelf_shows_no_default_tag() {
        let (ctx, mut app) = s124_loaded(false);
        let out = run_frames(&ctx, &mut app, 2);
        let nodes = a11y_text(&out);
        assert!(
            nodes.iter().any(|(_, l)| l.contains("Default")),
            "the pill: {nodes:?}"
        );
        assert!(
            !nodes
                .iter()
                .any(|(_, l)| l == "default" || l.contains("· default")),
            "no default tag on a one-preset shelf: {nodes:?}"
        );
    }

    /// A6: a control off its preset value shows "(modified)" and "Reset to preset".
    #[test]
    fn s124_a_changed_control_marks_the_preset_modified() {
        let (ctx, mut app) = s124_loaded(false);
        app.controls.gain = 1.5;
        let out = run_frames(&ctx, &mut app, 2);
        let nodes = a11y_text(&out);
        let any = |needle: &str| nodes.iter().any(|(_, l)| l.contains(needle));
        assert!(any("(modified)"), "{nodes:?}");
        assert!(any("Reset to preset"), "{nodes:?}");
    }

    /// P7: choosing a preset keeps the Variation and puts every other control at the preset's
    /// value; an id that is not shipped changes nothing.
    #[test]
    fn s124_selecting_a_preset_keeps_the_variation() {
        let (_ctx, mut app) = s124_loaded(false);
        app.controls.seed = Some(42);
        app.controls.gain = 2.0;
        app.advanced.steps = 9;
        app.select_preset("default");
        assert_eq!(app.controls.seed, Some(42));
        assert_eq!(app.controls.gain, 1.0);
        assert_eq!(app.advanced.steps, 40);
        app.controls.gain = 0.5;
        app.select_preset("no-such-preset");
        assert_eq!(app.controls.gain, 0.5);
        assert_eq!(app.preset_id(), "default");
    }

    /// UI § 6: the Save fallback shows the designer's headline and a path field.
    #[test]
    fn s124_save_fallback_shows_the_designers_words() {
        let (ctx, mut app) = s124_loaded(false);
        assert!(!app.save_fallback_shown());
        app.show_save_fallback("/nonexistent-dir/piece.wav".into());
        let out = run_frames(&ctx, &mut app, 2);
        let nodes = a11y_text(&out);
        assert!(
            nodes
                .iter()
                .any(|(_, l)| l.contains("couldn't open a Save window")),
            "{nodes:?}"
        );
        assert!(app.save_fallback_shown());
    }
}

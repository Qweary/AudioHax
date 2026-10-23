//! The window's interaction states, its button-enable matrix (UI `s123-UI` § 3), the four
//! Everyday controls (read from [`crate::options_schema`], not hand-kept), and the request those
//! controls build.
//!
//! Nothing here draws. The drawing code asks [`enablement`] whether a control is live and, when it
//! is not, shows the returned reason on hover (disabled controls stay visible, so the layout never
//! jumps).

use crate::cli::{PipelineArgs, Toggle, V2Spec};
use crate::options_schema::{render_options_schema, OptionSpec, SchemaError, Tier};
use crate::render::RenderRequest;

/// The six interaction states of UI § 3, plus `Loading` (a picture is being read and checked on
/// the worker; the wireframe has no row for it, and it behaves like `Rendering` except that Open
/// stays live so a slow picture can be replaced).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// No picture yet: the empty-state invitation.
    NoImage,
    /// A picture is being read, checked and decoded for display.
    Loading,
    /// A picture is loaded; nothing is running.
    ImageLoaded,
    /// `render()` is running (Render, or Save WAV needing a render).
    Rendering,
    /// `prepare_play()` + `play()` are running.
    Playing,
    /// Playback finished or was stopped (same enable set as `ImageLoaded`).
    Stopped,
}

/// The controls whose enablement UI § 3 specifies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    /// File ▸ Open, "Try a sample", drag-and-drop.
    Open,
    /// The Music | Modem switch.
    ModeSwitch,
    /// The output selector (speakers / MIDI port).
    OutputSelector,
    /// Variation, Voices, Room, Level. Room and Level stay live whatever the output is
    /// (lead ruling L-20, `s123-UI2` F3): see [`output_notice`].
    SoundControls,
    /// Render.
    Render,
    /// ▶ Play.
    Play,
    /// ⏹ Stop.
    Stop,
    /// Save WAV.
    SaveWav,
}

/// Is `control` live in `phase`? `Ok(())` if so; otherwise the plain reason to show on hover.
pub fn enablement(control: Control, phase: Phase) -> Result<(), &'static str> {
    use Control::*;
    use Phase::*;
    const NEED_PICTURE: &str = "Open a picture first.";
    const BUSY: &str = "Wait for the current piece to finish.";
    const PLAYING: &str = "Stop playback first.";
    let idle = matches!(phase, ImageLoaded | Stopped);
    match control {
        Open => match phase {
            Playing => Err(PLAYING),
            _ => Ok(()),
        },
        ModeSwitch | OutputSelector => match phase {
            Rendering | Loading => Err(BUSY),
            Playing => Err(PLAYING),
            _ => Ok(()),
        },
        SoundControls => match phase {
            NoImage => Err(NEED_PICTURE),
            Loading | Rendering => Err(BUSY),
            Playing => {
                Err("Locked while playing, so the piece stays what you hear. Stop to change it.")
            }
            _ => Ok(()),
        },
        Render | Play | SaveWav => {
            if idle {
                Ok(())
            } else {
                match phase {
                    NoImage => Err(NEED_PICTURE),
                    Playing => Err(PLAYING),
                    _ => Err(BUSY),
                }
            }
        }
        Stop => match phase {
            Playing => Ok(()),
            _ => Err("Nothing is playing."),
        },
    }
}

/// L-20 / `s123-UI2` F3, the designer's words verbatim: the one-line notice shown under Room and
/// Level while a MIDI port is the output.
pub const MIDI_ROOM_LEVEL_NOTICE: &str =
    "For Play through MIDI, your synth sets Room and Level. Save WAV still uses the values here.";

/// Where Play sends sound, as far as the sound controls care.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputKind {
    /// The built-in synth: Room and Level shape what Play sounds like.
    Speakers,
    /// An external MIDI synth: Room and Level do not reach it, but `render()` still applies them
    /// to Save WAV (`src/render.rs`: `SynthConfig { enable_reverb_and_chorus: req.reverb…, gain:
    /// req.gain }`).
    Midi,
}

/// L-20: Room and Level are live exactly when the other sound controls are; the output choice
/// never greys them, because they still shape Save WAV. With a MIDI output the window shows
/// [`MIDI_ROOM_LEVEL_NOTICE`] instead. Returns that notice, or `None`.
pub fn output_notice(output: OutputKind) -> Option<&'static str> {
    match output {
        OutputKind::Speakers => None,
        OutputKind::Midi => Some(MIDI_ROOM_LEVEL_NOTICE),
    }
}

/// L-14: the Voices maximum for a picture is `min(schema max, min(width, height))`. With the
/// 16 px floor (SH B-2) this is the schema's 16 for every admitted picture; it is still DERIVED,
/// so a later change to either bound stays honest.
pub fn voices_max(schema_max: u32, width: u32, height: u32) -> u32 {
    schema_max.min(width.min(height)).max(1)
}

/// The four Everyday rows of [`crate::options_schema::GUI_TIERS`], as the window draws them.
#[derive(Debug, Clone, PartialEq)]
pub struct EverydaySpecs {
    /// `seed` → "Variation".
    pub seed: OptionSpec,
    /// `instruments` → "Voices".
    pub instruments: OptionSpec,
    /// `reverb` → "Room (reverb)".
    pub reverb: OptionSpec,
    /// `gain` → "Level".
    pub gain: OptionSpec,
}

/// Why the Everyday rows could not be read.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum EverydayError {
    /// The schema itself failed its forcing check.
    #[error(transparent)]
    Schema(#[from] SchemaError),
    /// The Everyday tier is not exactly the four rows this window draws.
    #[error(
        "the window draws seed/instruments/reverb/gain as Everyday, but the schema's Everyday \
         rows are {0:?}"
    )]
    Mismatch(Vec<String>),
}

impl EverydaySpecs {
    /// Read the four Everyday rows from the schema. Fails if the Everyday tier is not exactly
    /// `seed`, `instruments`, `reverb`, `gain` (a retiering must be reflected in this window).
    pub fn from_schema() -> Result<Self, EverydayError> {
        let schema = render_options_schema()?;
        let everyday: Vec<OptionSpec> = schema
            .into_iter()
            .filter(|o| o.tier == Tier::Everyday)
            .collect();
        let ids: Vec<String> = everyday.iter().map(|o| o.id.clone()).collect();
        let take = |id: &str| everyday.iter().find(|o| o.id == id).cloned();
        match (
            everyday.len(),
            take("seed"),
            take("instruments"),
            take("reverb"),
            take("gain"),
        ) {
            (4, Some(seed), Some(instruments), Some(reverb), Some(gain)) => Ok(EverydaySpecs {
                seed,
                instruments,
                reverb,
                gain,
            }),
            _ => Err(EverydayError::Mismatch(ids)),
        }
    }

    /// The schema's Voices maximum (16 today).
    pub fn voices_schema_max(&self) -> u32 {
        self.instruments.bounds.map(|b| b.max as u32).unwrap_or(16)
    }

    /// The schema's Level range `(min, max, step)`.
    pub fn gain_range(&self) -> (f32, f32, f32) {
        self.gain
            .bounds
            .map(|b| (b.min as f32, b.max as f32, b.step as f32))
            .unwrap_or((0.0, 4.0, 0.05))
    }

    /// The help text a non-technical reader sees: the designer's `gui_help`, else clap's.
    pub fn help(spec: &OptionSpec) -> &str {
        spec.gui_help.as_deref().unwrap_or(&spec.help)
    }
}

/// The values of the four Everyday controls.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Controls {
    /// Variation: `None` ⇒ the picture chooses (the image-derived seed, S91).
    pub seed: Option<u64>,
    /// Voices.
    pub instruments: usize,
    /// Room: reverb + chorus on/off.
    pub reverb: bool,
    /// Level (master gain; 1.0 is unity and a no-op).
    pub gain: f32,
}

impl Default for Controls {
    /// The CLI's defaults, so an untouched window renders what `audiohax render` renders.
    fn default() -> Self {
        let p = PipelineArgs::default();
        Controls {
            seed: None,
            instruments: p.instruments,
            reverb: true,
            gain: 1.0,
        }
    }
}

impl Controls {
    /// Clamp Voices to the picture's derived maximum (L-14).
    pub fn clamp_voices(&mut self, max: u32) {
        self.instruments = self.instruments.clamp(1, max.max(1) as usize);
    }

    /// The request the window renders and plays. Every other field is the CLI default:
    /// expression off, no v2, no loop, default scan width and detail. W-1: `event_dump` is
    /// ALWAYS `None`; the window never arms the process-global event dump.
    pub fn request(&self) -> RenderRequest {
        RenderRequest {
            seed: self.seed,
            loop_cue: false,
            expression: Toggle::Off,
            v2: None,
            pipeline: PipelineArgs {
                instruments: self.instruments,
                ..PipelineArgs::default()
            },
            reverb: if self.reverb { Toggle::On } else { Toggle::Off },
            gain: self.gain,
            event_dump: None,
        }
    }
}

impl Controls {
    /// S124: the four Everyday values a request carries (a preset's baseline, `s124-UI` § 1.6).
    pub fn from_request(r: &RenderRequest) -> Self {
        Controls {
            seed: r.seed,
            instruments: r.pipeline.instruments,
            reverb: r.reverb.is_on(),
            gain: r.gain,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// S124 — the Advanced disclosure ("More", `s124-UI` § 2) and the preset baseline (§ 1.6)
// ─────────────────────────────────────────────────────────────────────────────

/// The Advanced rows the window draws behind "More": every `Advanced` row of
/// [`crate::options_schema::GUI_TIERS`] EXCEPT `soundfont` (lead ruling L-31: no sound-bank picker
/// until the operator answers and the `rustysynth` odd-chunk check has run, L-13).
#[derive(Debug, Clone, PartialEq)]
pub struct AdvancedSpecs {
    /// `expression` → "Expression".
    pub expression: OptionSpec,
    /// `v2` → "Expression detail" (requires `expression=on`).
    pub v2: OptionSpec,
    /// `thickness` → "Scan width".
    pub thickness: OptionSpec,
    /// `steps` → "Scan detail".
    pub steps: OptionSpec,
    /// `loop` → "Seamless loop" (requires `gain=1`).
    pub loop_cue: OptionSpec,
}

/// The ids of the schema's Advanced tier this window knows: the five it draws plus `soundfont`,
/// which it deliberately does not draw (L-31).
pub const ADVANCED_IDS: &[&str] = &[
    "expression",
    "v2",
    "thickness",
    "steps",
    "loop",
    "soundfont",
];

impl AdvancedSpecs {
    /// Read the Advanced rows from the schema. Fails if the Advanced tier is not exactly
    /// [`ADVANCED_IDS`] (a retiering must be reflected in this window, as for the Everyday four).
    pub fn from_schema() -> Result<Self, EverydayError> {
        let schema = render_options_schema()?;
        let advanced: Vec<OptionSpec> = schema
            .into_iter()
            .filter(|o| o.tier == Tier::Advanced)
            .collect();
        let mut ids: Vec<String> = advanced.iter().map(|o| o.id.clone()).collect();
        let take = |id: &str| advanced.iter().find(|o| o.id == id).cloned();
        let mut want: Vec<String> = ADVANCED_IDS.iter().map(|s| s.to_string()).collect();
        ids.sort();
        want.sort();
        match (
            ids == want,
            take("expression"),
            take("v2"),
            take("thickness"),
            take("steps"),
            take("loop"),
        ) {
            (true, Some(expression), Some(v2), Some(thickness), Some(steps), Some(loop_cue)) => {
                Ok(AdvancedSpecs {
                    expression,
                    v2,
                    thickness,
                    steps,
                    loop_cue,
                })
            }
            _ => Err(EverydayError::Mismatch(ids)),
        }
    }

    /// A row's `(min, max, step)` GUI bounds, or the fallback.
    pub fn range(spec: &OptionSpec, fallback: (f64, f64, f64)) -> (f64, f64, f64) {
        spec.bounds
            .map(|b| (b.min, b.max, b.step))
            .unwrap_or(fallback)
    }
}

/// The values of the Advanced controls. Defaults are the CLI's, so an untouched "More" changes
/// nothing about the request.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdvancedControls {
    /// Expression (`--expression on|off`).
    pub expression: bool,
    /// Expression detail (`--v2`); `None` = no term.
    pub v2: Option<V2Spec>,
    /// Scan width (`--thickness`).
    pub thickness: f32,
    /// Scan detail (`--steps`).
    pub steps: usize,
    /// Seamless loop (`--loop`).
    pub loop_cue: bool,
}

impl Default for AdvancedControls {
    fn default() -> Self {
        let p = PipelineArgs::default();
        AdvancedControls {
            expression: false,
            v2: None,
            thickness: p.thickness,
            steps: p.steps,
            loop_cue: false,
        }
    }
}

impl AdvancedControls {
    /// The Advanced values a request carries (a preset's baseline).
    pub fn from_request(r: &RenderRequest) -> Self {
        AdvancedControls {
            expression: r.expression.is_on(),
            v2: r.v2,
            thickness: r.pipeline.thickness,
            steps: r.pipeline.steps,
            loop_cue: r.loop_cue,
        }
    }
}

/// The request the window renders and plays: the Everyday four ([`Controls::request`]) plus the
/// Advanced values. A dependent control that is DISABLED does not reach the request (UI `s122-UI`
/// § 4, `s124-UI` § 2.2: a dependency is shown as disabled-with-reason, never as a refusal after
/// the press): Expression detail only with Expression on, Seamless loop only at Level 1.0 — the
/// same two conditions [`crate::render::RenderRequest::validate`] / `compose` refuse on.
pub fn request_with(controls: &Controls, adv: &AdvancedControls) -> RenderRequest {
    let mut r = controls.request();
    r.expression = if adv.expression {
        Toggle::On
    } else {
        Toggle::Off
    };
    r.v2 = if adv.expression { adv.v2 } else { None };
    r.pipeline.thickness = adv.thickness;
    r.pipeline.steps = adv.steps;
    r.loop_cue = adv.loop_cue && controls.gain == 1.0;
    r
}

/// Is "Expression detail" live? `Err` carries the designer's inline reason (`s124-UI` § 2.2).
pub fn v2_enablement(adv: &AdvancedControls) -> Result<(), &'static str> {
    if adv.expression {
        Ok(())
    } else {
        Err("Turn Expression on first — Expression detail builds on it.")
    }
}

/// Is "Seamless loop" live? Numeric compare against unity (`gain=1` ≡ `1.0`).
pub fn loop_enablement(controls: &Controls) -> Result<(), &'static str> {
    if controls.gain == 1.0 {
        Ok(())
    } else {
        Err("Seamless loop needs Level at 1.0.")
    }
}

/// The values a preset defines for every control (`s124-UI` § 1.6, A6): what "modified" is
/// measured against and what "Reset to preset" restores.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Baseline {
    /// The Everyday four.
    pub controls: Controls,
    /// The Advanced values.
    pub advanced: AdvancedControls,
}

impl Baseline {
    /// A preset's baseline from its resolved request (the preset's settings over clap's
    /// defaults). The seed baseline is always `None` (a preset never pins a seed).
    pub fn from_request(r: &RenderRequest) -> Self {
        Baseline {
            controls: Controls {
                seed: None,
                ..Controls::from_request(r)
            },
            advanced: AdvancedControls::from_request(r),
        }
    }
}

/// One control the window marks as changed from the preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Knob {
    /// Variation.
    Seed,
    /// Voices.
    Instruments,
    /// Room.
    Reverb,
    /// Level.
    Gain,
    /// Expression.
    Expression,
    /// Expression detail.
    V2,
    /// Scan width.
    Thickness,
    /// Scan detail.
    Steps,
    /// Seamless loop.
    Loop,
}

/// Every knob, in the panel's order.
pub const KNOBS: &[Knob] = &[
    Knob::Seed,
    Knob::Instruments,
    Knob::Reverb,
    Knob::Gain,
    Knob::Expression,
    Knob::V2,
    Knob::Thickness,
    Knob::Steps,
    Knob::Loop,
];

/// Two GUI floats are "the same value" when they differ by less than half the finest slider step
/// in the schema (0.01). NUMERIC, never string: a preset's `1.0` and a slider's `1` are equal
/// (`s124-UI` § 1.6).
fn same_f32(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.005
}

/// Does `knob` differ from the baseline?
pub fn is_modified(knob: Knob, c: &Controls, a: &AdvancedControls, base: &Baseline) -> bool {
    let (bc, ba) = (&base.controls, &base.advanced);
    match knob {
        Knob::Seed => c.seed != bc.seed,
        Knob::Instruments => c.instruments != bc.instruments,
        Knob::Reverb => c.reverb != bc.reverb,
        Knob::Gain => !same_f32(c.gain, bc.gain),
        Knob::Expression => a.expression != ba.expression,
        Knob::V2 => a.v2 != ba.v2,
        Knob::Thickness => !same_f32(a.thickness, ba.thickness),
        Knob::Steps => a.steps != ba.steps,
        Knob::Loop => a.loop_cue != ba.loop_cue,
    }
}

/// Is any knob off the baseline?
pub fn any_modified(c: &Controls, a: &AdvancedControls, base: &Baseline) -> bool {
    KNOBS.iter().any(|k| is_modified(*k, c, a, base))
}

/// Put one knob back to the baseline.
pub fn reset_knob(knob: Knob, c: &mut Controls, a: &mut AdvancedControls, base: &Baseline) {
    let (bc, ba) = (&base.controls, &base.advanced);
    match knob {
        Knob::Seed => c.seed = bc.seed,
        Knob::Instruments => c.instruments = bc.instruments,
        Knob::Reverb => c.reverb = bc.reverb,
        Knob::Gain => c.gain = bc.gain,
        Knob::Expression => a.expression = ba.expression,
        Knob::V2 => a.v2 = ba.v2,
        Knob::Thickness => a.thickness = ba.thickness,
        Knob::Steps => a.steps = ba.steps,
        Knob::Loop => a.loop_cue = ba.loop_cue,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matrix_matches_ui_section_3() {
        use Control::*;
        use Phase::*;
        let all = [
            Open,
            ModeSwitch,
            OutputSelector,
            SoundControls,
            Render,
            Play,
            Stop,
            SaveWav,
        ];
        let live = |c, p| enablement(c, p).is_ok();
        // no image
        for c in all {
            let want = matches!(c, Open | ModeSwitch | OutputSelector);
            assert_eq!(live(c, NoImage), want, "{c:?} in NoImage");
        }
        // image loaded and stopped: everything but Stop
        for p in [ImageLoaded, Stopped] {
            for c in all {
                assert_eq!(live(c, p), c != Stop, "{c:?} in {p:?}");
            }
        }
        // rendering: only Open (UI § 3 row 1)
        for c in all {
            assert_eq!(live(c, Rendering), c == Open, "{c:?} in Rendering");
        }
        // playing: only Stop
        for c in all {
            assert_eq!(live(c, Playing), c == Stop, "{c:?} in Playing");
        }
    }

    #[test]
    fn disabled_controls_always_carry_a_reason() {
        use Control::*;
        use Phase::*;
        for p in [NoImage, Loading, ImageLoaded, Rendering, Playing, Stopped] {
            for c in [
                Open,
                ModeSwitch,
                OutputSelector,
                SoundControls,
                Render,
                Play,
                Stop,
                SaveWav,
            ] {
                if let Err(reason) = enablement(c, p) {
                    assert!(!reason.is_empty());
                }
            }
        }
    }

    #[test]
    fn voices_bound_derives_from_the_picture() {
        assert_eq!(voices_max(16, 640, 480), 16);
        assert_eq!(voices_max(16, 16, 5000), 16);
        // Below the admission floor (never reached through the window) it still derives, from
        // whichever side is smaller (strengthened in the fix round, TODO-6: with the small side
        // only ever in `width`, `min(16, width)` passed this test too).
        assert_eq!(voices_max(16, 12, 900), 12);
        assert_eq!(voices_max(16, 900, 12), 12);
        assert_eq!(voices_max(16, 900, 9), 9);
        assert_eq!(voices_max(8, 640, 480), 8);
        let mut c = Controls {
            instruments: 16,
            ..Controls::default()
        };
        c.clamp_voices(12);
        assert_eq!(c.instruments, 12);
    }

    #[test]
    fn l20_room_and_level_are_never_greyed_by_the_output_choice() {
        // The output choice adds a notice and changes no enablement: `enablement` takes no
        // output at all, so every phase gives Room/Level the same answer under either output.
        assert_eq!(output_notice(OutputKind::Speakers), None);
        assert_eq!(
            output_notice(OutputKind::Midi),
            Some(MIDI_ROOM_LEVEL_NOTICE)
        );
        assert_eq!(
            MIDI_ROOM_LEVEL_NOTICE,
            "For Play through MIDI, your synth sets Room and Level. Save WAV still uses the \
             values here."
        );
        assert!(enablement(Control::SoundControls, Phase::ImageLoaded).is_ok());
        assert!(enablement(Control::SoundControls, Phase::Stopped).is_ok());
    }

    #[test]
    fn everyday_rows_are_the_four_the_window_draws() {
        let e = EverydaySpecs::from_schema().expect("schema has exactly the four Everyday rows");
        assert_eq!(e.seed.label, "Variation");
        assert_eq!(e.instruments.label, "Voices");
        assert_eq!(e.gain.label, "Level");
        assert_eq!(e.voices_schema_max(), 16);
        // `steps` ("Scan detail") is NOT an Everyday control (brief § 3: verify vs GUI_TIERS).
        let schema = render_options_schema().expect("schema");
        let steps = schema.iter().find(|o| o.id == "steps").expect("steps row");
        assert_eq!(steps.tier, Tier::Advanced);
    }

    #[test]
    fn w1_every_request_the_window_builds_leaves_the_dump_disarmed() {
        // T15.
        for c in [
            Controls::default(),
            Controls {
                seed: Some(7),
                instruments: 16,
                reverb: false,
                gain: 2.5,
            },
            Controls {
                seed: Some(u64::MAX),
                instruments: 1,
                reverb: true,
                gain: 0.0,
            },
        ] {
            let r = c.request();
            assert_eq!(r.event_dump, None);
            assert!(!r.loop_cue);
            assert_eq!(r.expression, Toggle::Off);
            assert_eq!(r.v2, None);
        }
    }

    #[test]
    fn untouched_controls_build_the_cli_default_request() {
        // The CLI's `render` defaults, field by field.
        use clap::Parser;
        let cli =
            crate::cli::Cli::try_parse_from(["audiohax", "render", "x.png", "--wav", "o.wav"])
                .expect("parse");
        let crate::cli::Command::Render(args) = cli.command else {
            panic!("render subcommand");
        };
        let mut cli_req = RenderRequest::from(&args);
        cli_req.event_dump = None;
        assert_eq!(Controls::default().request(), cli_req);
    }

    // ── S124 ─────────────────────────────────────────────────────────────────────────────────

    #[test]
    fn advanced_rows_are_the_five_drawn_plus_the_sound_bank_left_out() {
        let a = AdvancedSpecs::from_schema().expect("schema Advanced tier is the known six");
        assert_eq!(a.expression.label, "Expression");
        assert_eq!(a.v2.label, "Expression detail");
        assert_eq!(a.thickness.label, "Scan width");
        assert_eq!(a.steps.label, "Scan detail");
        assert_eq!(a.loop_cue.label, "Seamless loop");
        // L-31: the window has no field for the sound bank at all.
        assert!(ADVANCED_IDS.contains(&"soundfont"));
    }

    #[test]
    fn untouched_more_and_default_preset_build_the_cli_default_request() {
        use clap::Parser;
        let cli =
            crate::cli::Cli::try_parse_from(["audiohax", "render", "x.png", "--wav", "o.wav"])
                .expect("parse");
        let crate::cli::Command::Render(args) = cli.command else {
            panic!("render subcommand");
        };
        let mut cli_req = RenderRequest::from(&args);
        cli_req.event_dump = None;
        assert_eq!(
            request_with(&Controls::default(), &AdvancedControls::default()),
            cli_req
        );
        // The default preset's baseline IS the untouched window (P2).
        let d = crate::presets::default_preset().expect("default preset");
        let preq = RenderRequest::from(&crate::presets::preset_render_args(d).expect("args"));
        let base = Baseline::from_request(&preq);
        assert_eq!(base.controls, Controls::default());
        assert_eq!(base.advanced, AdvancedControls::default());
        assert!(!any_modified(
            &Controls::default(),
            &AdvancedControls::default(),
            &base
        ));
    }

    #[test]
    fn disabled_dependents_never_reach_the_request() {
        let c = Controls::default();
        let a = AdvancedControls {
            expression: false,
            v2: Some("all".parse().expect("v2")),
            loop_cue: true,
            ..AdvancedControls::default()
        };
        assert!(v2_enablement(&a).is_err());
        let r = request_with(&c, &a);
        assert_eq!(
            r.v2, None,
            "Expression detail is off while Expression is off"
        );
        assert!(r.loop_cue, "loop at unity Level");
        r.validate().expect("never a refusal");
        let louder = Controls { gain: 1.5, ..c };
        assert!(loop_enablement(&louder).is_err());
        let r = request_with(&louder, &a);
        assert!(!r.loop_cue, "Seamless loop is off while Level is not 1.0");
        r.validate().expect("never a refusal");
        let on = AdvancedControls {
            expression: true,
            ..a
        };
        assert_eq!(request_with(&c, &on).v2, on.v2);
        assert_eq!(request_with(&c, &on).expression, Toggle::On);
    }

    #[test]
    fn a6_modified_is_numeric_and_reset_restores_the_preset_value() {
        let base = Baseline {
            controls: Controls::default(),
            advanced: AdvancedControls::default(),
        };
        let mut c = Controls::default();
        let mut a = AdvancedControls::default();
        // A preset's 1.0 against a slider's 1 (and float noise below the finest step): not modified.
        c.gain = 1.0000001;
        a.thickness = 0.1000001;
        assert!(!any_modified(&c, &a, &base));
        c.gain = 1.05;
        a.steps = 12;
        assert!(is_modified(Knob::Gain, &c, &a, &base));
        assert!(is_modified(Knob::Steps, &c, &a, &base));
        assert!(!is_modified(Knob::Reverb, &c, &a, &base));
        reset_knob(Knob::Gain, &mut c, &mut a, &base);
        assert!(!is_modified(Knob::Gain, &c, &a, &base));
        assert!(any_modified(&c, &a, &base), "steps still changed");
        for k in KNOBS {
            reset_knob(*k, &mut c, &mut a, &base);
        }
        assert!(!any_modified(&c, &a, &base));
        assert_eq!(KNOBS.len(), 9, "four Everyday + five Advanced");
    }
}

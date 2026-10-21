//! The window's interaction states, its button-enable matrix (UI `s123-UI` § 3), the four
//! Everyday controls (read from [`crate::options_schema`], not hand-kept), and the request those
//! controls build.
//!
//! Nothing here draws. The drawing code asks [`enablement`] whether a control is live and, when it
//! is not, shows the returned reason on hover (disabled controls stay visible, so the layout never
//! jumps).

use crate::cli::{PipelineArgs, Toggle};
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
    /// Variation, Voices, Room, Level.
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
    #[error("the window draws seed/instruments/reverb/gain as Everyday, but the schema's Everyday rows are {0:?}")]
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
        // Below the admission floor (never reached through the window) it still derives.
        assert_eq!(voices_max(16, 12, 900), 12);
        assert_eq!(voices_max(8, 640, 480), 8);
        let mut c = Controls {
            instruments: 16,
            ..Controls::default()
        };
        c.clamp_voices(12);
        assert_eq!(c.instruments, 12);
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
}

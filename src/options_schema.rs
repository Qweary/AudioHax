//! src/options_schema.rs — the anti-drift OPTIONS SEAM (S122 Phase 1, `s121-01` § 5.3 / § 3.5).
//!
//! A front-end (the planned egui GUI) must not hand-maintain its own list of render options: a
//! hand-kept list drifts from the CLI the first time someone adds a flag. Instead the options are
//! DATA, derived from the one grammar that already exists — clap's [`crate::cli::Cli`] — and joined
//! with exactly ONE piece of GUI-side curation, the [`GUI_TIERS`] table.
//!
//! * **Single source for the vocabulary.** Every option's id, type, default and help text are read
//!   off clap at runtime (`CommandFactory`). Help is the clap doc comment, verbatim. A front-end that
//!   needs plainer words for a non-technical reader gets them from the row's `gui_help`, which sits
//!   NEXT TO the clap help and never replaces it (editing `cli.rs` doc comments would change
//!   `--help`, a CLI behaviour change).
//! * **One curation table.** [`GUI_TIERS`] has one row per `render` argument: tier, group, label,
//!   optional GUI help, optional GUI bounds, dependencies, and the REASON for the tier. Retiering is
//!   a one-place edit.
//! * **Forcing, not remembering.** [`render_options_schema`] returns an error when a clap arg has no
//!   row or a row names an arg that no longer exists, and the error text is the exact line to add or
//!   delete. The suite's unit test `every_render_arg_has_exactly_one_row` turns that into a red
//!   build. A second, integration-level forcing test (`tests/options_schema_inert_s122.rs`) renders
//!   a fixture at default vs non-default for every `Everyday`/`Advanced` row and fails if the output
//!   bytes are identical — so a knob that does nothing cannot ship as a live control.
//!
//! **Bounds are GUI metadata, NOT clap ranges (S122 measurement).** `s121-01` § 5.3 asked for the
//! bounds to live in clap. Measured on the Phase-0 release binary, every value clap parses today for
//! `--thickness`, `--gain`, `--steps`, `--ms-per-step` and `--jitter-percent` renders successfully
//! (negatives, `NaN` and `inf` included), and `--instruments` renders up to `min(width, height)` of
//! the image, so any static clap range would reject invocations that work today. The bounds below
//! are therefore the GUI's slider range only; the CLI keeps accepting what it accepts. See
//! `Swaram/expression-s113/s122-02P1-options-schema-report.md` § 5 for the evidence.
//!
//! **Presets (S123) attach ABOVE this table, not inside it.** A preset is a named map from option
//! `id` (the clap long name, the one identifier across CLI, schema and GUI) to a clap value string.
//! It is applied by turning each pair back into CLI tokens and parsing them with clap, so a preset
//! can only ever name values the CLI would accept, and validated against [`render_options_schema`]
//! (an id with no `OptionSpec` is a stale preset). Nothing here needs to change for that layer to
//! exist; `Tier` deliberately has no `Preset` variant.
//!
//! Pure: depends only on `clap` + `serde` (both non-optional), so this module builds and unit-tests
//! under `cargo test --lib --no-default-features`.

use clap::{ArgAction, CommandFactory};
use std::any::TypeId;
use std::path::PathBuf;

use crate::cli::{Cli, V2Spec};

/// Which disclosure level a front-end shows an option at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum Tier {
    /// Shown by default. Every `Everyday` row is forced live by the inert-knob test.
    Everyday,
    /// Shown behind "More". Every `Advanced` row is forced live by the inert-knob test.
    Advanced,
    /// Never rendered as a control (I/O, diagnostics, measured-inert knobs, the picture itself).
    Hidden,
}

/// GUI-only slider/spin bounds. NOT enforced by the CLI (see the module doc).
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
pub struct Bounds {
    /// Lowest value the GUI offers.
    pub min: f64,
    /// Highest value the GUI offers.
    pub max: f64,
    /// Slider step.
    pub step: f64,
}

/// The control a front-end draws, derived from the clap argument's TYPE (not curated).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum OptionKind {
    /// A presence flag (`ArgAction::SetTrue`), e.g. `--loop`.
    Bool,
    /// An integer value (`usize` / `u64`). Bounds, when present, are the row's GUI bounds.
    Int {
        /// GUI minimum, if the row sets one.
        min: Option<i64>,
        /// GUI maximum, if the row sets one.
        max: Option<i64>,
    },
    /// A floating-point value (`f32`). Bounds, when present, are the row's GUI bounds.
    Float {
        /// GUI minimum, if the row sets one.
        min: Option<f64>,
        /// GUI maximum, if the row sets one.
        max: Option<f64>,
        /// GUI slider step, if the row sets one.
        step: Option<f64>,
    },
    /// One of clap's `ValueEnum` possible values, in clap's order.
    Enum(Vec<String>),
    /// A comma-separated subset of the `--v2` terms ([`V2_TERMS`]).
    TermSet(Vec<String>),
    /// A filesystem path. `save` is true for an output the user names (WAV, event dump).
    FilePick {
        /// Comma-separated accepted extensions, from the row (empty = any).
        accept: String,
        /// `true` for a save dialog, `false` for an open dialog.
        save: bool,
    },
}

/// One `render` option, as a front-end consumes it: clap's facts joined with its `GUI_TIERS` row.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct OptionSpec {
    /// The clap long name (`"ms-per-step"`), or the arg id for a positional (`"image"`). The ONE
    /// identifier across CLI, schema, presets and GUI.
    pub id: String,
    /// `true` for a positional argument (`image`).
    pub positional: bool,
    /// `true` for a top-level `global = true` argument (`--config`, `--json`).
    pub global: bool,
    /// The user-facing label. The row's label; the `id` when the row sets none.
    pub label: String,
    /// The clap doc comment's summary, verbatim (what `-h` prints).
    pub help: String,
    /// The full clap doc comment, verbatim (what `--help` prints), when it is longer than `help`.
    pub long_help: Option<String>,
    /// Plain-language help from the UI Designer, for a front-end to show INSTEAD of `help` where
    /// the designer judged the clap text unusable for a non-technical reader. `None` ⇒ use `help`.
    pub gui_help: Option<String>,
    /// The control to draw.
    pub kind: OptionKind,
    /// clap's default value(s), comma-joined, as clap would print them. `None` ⇒ no default
    /// (the option is absent unless given).
    pub default: Option<String>,
    /// Disclosure tier.
    pub tier: Tier,
    /// The designer's group (`"Picture"`, `"Music"`, `"Output"`), or `None`.
    pub group: Option<String>,
    /// Preconditions, each `"<id>=<value>"` (e.g. `"expression=on"`). They MIRROR an existing
    /// refusal in the render path; they do not re-implement it. A front-end disables the control
    /// with the reason while a precondition is unmet.
    pub requires: Vec<String>,
    /// GUI-only bounds (not enforced by the CLI).
    pub bounds: Option<Bounds>,
    /// Why the row has this tier. Recorded so a later retiering can see what it overrides.
    pub reason: String,
}

/// One curation row. The ONLY GUI-side data; everything else in an [`OptionSpec`] comes from clap.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TierRow {
    /// The clap long name (or positional id) this row curates.
    pub id: &'static str,
    /// Disclosure tier.
    pub tier: Tier,
    /// Designer group id, plain string.
    pub group: Option<&'static str>,
    /// Designer label; `None` ⇒ the id.
    pub label: Option<&'static str>,
    /// Designer help override; `None` ⇒ the clap doc comment.
    pub gui_help: Option<&'static str>,
    /// Accepted extensions for a path argument.
    pub accept: Option<&'static str>,
    /// `true` when a path argument is an output the user names.
    pub save: bool,
    /// GUI-only bounds.
    pub bounds: Option<Bounds>,
    /// Preconditions, `"<id>=<value>"`.
    pub requires: &'static [&'static str],
    /// Why this tier.
    pub reason: &'static str,
}

/// The `--v2` terms a GUI may offer, in ladder order. Mirrors `V2Spec::from_str`; the unit test
/// `v2_terms_cover_all` fails if `all` arms anything these three do not.
pub const V2_TERMS: &[&str] = &["e3", "e2", "ta"];

/// Shorthand for a row; keeps each row to one line-group in [`GUI_TIERS`].
const fn row(id: &'static str, tier: Tier, reason: &'static str) -> TierRow {
    TierRow {
        id,
        tier,
        group: None,
        label: None,
        gui_help: None,
        accept: None,
        save: false,
        bounds: None,
        requires: &[],
        reason,
    }
}

/// THE curation table: one row per `render` argument, globals included. PROVISIONAL tiers and
/// labels from the UI Designer (`s122-UI-schema-tiering.md`, adopted by lead ruling S122-P1 (1)).
/// Labels, groups and `gui_help` are the designer's strings; tiers are the designer's except where a
/// measurement overrides (none did). To retier, edit the row here and nothing else.
pub const GUI_TIERS: &[TierRow] = &[
    // ── Picture ──────────────────────────────────────────────────────────────────────────────
    TierRow {
        group: Some("Picture"),
        accept: Some(".png,.jpg,.jpeg"),
        ..row(
            "image",
            Tier::Hidden,
            "THE input object, surfaced as Open/Drop, not as a schema control (designer row 1; the \
             designer's tier is '—', which this three-tier enum records as Hidden).",
        )
    },
    // ── Music ────────────────────────────────────────────────────────────────────────────────
    TierRow {
        group: Some("Music"),
        label: Some("Variation"),
        gui_help: Some("Same picture + same Variation = the same piece every time."),
        ..row(
            "seed",
            Tier::Everyday,
            "Determinism is AudioHax's signature property; a dice/lock is the one newcomer-legible \
             reproducibility control (designer row 3). Live: forced by the inert-knob test.",
        )
    },
    TierRow {
        group: Some("Music"),
        label: Some("Voices"),
        bounds: Some(Bounds { min: 1.0, max: 16.0, step: 1.0 }),
        ..row(
            "instruments",
            Tier::Everyday,
            "Audible, intuitive count (designer row 4). GUI bound 1..=16 is the s121-01 §5.3 / \
             designer proposal and is GUI-ONLY: the CLI renders up to min(width, height) of the \
             image (measured S122-P1), and 0 is refused by the analyzer (ZeroBars).",
        )
    },
    TierRow {
        group: Some("Music"),
        label: Some("Expression"),
        gui_help: Some("Shape phrase dynamics (quieter openings)."),
        ..row(
            "expression",
            Tier::Advanced,
            "A musical-quality axis; an expert toggle once presets fold the common case (designer \
             row 7). Gates --v2.",
        )
    },
    TierRow {
        group: Some("Music"),
        label: Some("Expression detail"),
        requires: &["expression=on"],
        ..row(
            "v2",
            Tier::Advanced,
            "3-rung ladder of deltas, deeply technical (designer row 8). Requires expression=on: \
             mirrors the refusal in render.rs compose() ('--v2 requires `--expression on`'). The \
             designer marked the clap help as jargon, acceptable as Advanced help, never as a label; \
             no replacement string was supplied, so gui_help is None.",
        )
    },
    TierRow {
        group: Some("Music"),
        label: Some("Scan width"),
        bounds: Some(Bounds { min: 0.01, max: 1.0, step: 0.01 }),
        ..row(
            "thickness",
            Tier::Advanced,
            "How much image each step reads; niche (designer row 10). GUI bound 0.01..=1.0 is \
             GUI-ONLY: measured, values >= 1.0 render identically to 1.0 and 0/negative/NaN render \
             identically to 0.001, so the CLI accepts and clamps everything.",
        )
    },
    TierRow {
        group: Some("Music"),
        label: Some("Scan detail"),
        bounds: Some(Bounds { min: 1.0, max: 200.0, step: 1.0 }),
        ..row(
            "steps",
            Tier::Advanced,
            "RELABELLED per s121-01 §3.5: it sets feature-extraction sampling, NOT the piece's step \
             count (re-measured S122-P1: steps 1..200 change the bytes while the piece stays 36 / 42 \
             / 48 steps on three images). Live, so not Hidden. The designer marked the clap help \
             misleading; no replacement string was supplied, so gui_help is None (OWED). GUI bound \
             1..=200 is GUI-ONLY and provisional; the CLI renders 0..=10000 (0 == 1).",
        )
    },
    TierRow {
        label: Some("ms-per-step"),
        ..row(
            "ms-per-step",
            Tier::Hidden,
            "MEASURED INERT ON render: byte-identical WAV at 0 / 100 / 1000 vs 250 over 8 images x 4 \
             configs (S122-P1 re-measure of s121-01 §3.5). The composer takes the grid from the \
             plan. Would read as a dead 'tempo' slider. Do NOT promote without making it live, which \
             changes the sound and is not authorised.",
        )
    },
    TierRow {
        label: Some("jitter-percent"),
        ..row(
            "jitter-percent",
            Tier::Hidden,
            "MEASURED INERT ON render: byte-identical WAV at 0 / 50 vs 15 over 8 images x 4 configs \
             (S122-P1 re-measure of s121-01 §3.5). render has no jitter (deterministic). Do NOT \
             promote without making it live, which changes the sound and is not authorised.",
        )
    },
    // ── Output ───────────────────────────────────────────────────────────────────────────────
    TierRow {
        group: Some("Output"),
        label: Some("Room (reverb)"),
        ..row(
            "reverb",
            Tier::Everyday,
            "Audible, binary, newcomer-safe (designer row 5).",
        )
    },
    TierRow {
        group: Some("Output"),
        label: Some("Level"),
        bounds: Some(Bounds { min: 0.0, max: 4.0, step: 0.05 }),
        ..row(
            "gain",
            Tier::Everyday,
            "Volume, in the 'after the music' group (designer row 6). GUI bound 0..=4 is GUI-ONLY \
             and provisional: the CLI renders every f32 it parses, negatives, NaN and inf included.",
        )
    },
    TierRow {
        group: Some("Output"),
        label: Some("Seamless loop"),
        requires: &["gain=1"],
        ..row(
            "loop",
            Tier::Advanced,
            "Niche export mode (designer row 15). Requires unity gain: mirrors \
             RenderRequest::validate() (refuses when loop && gain != 1.0, exact compare). It can \
             ALSO refuse data-dependently (tail still above -90 dBFS), which `requires` cannot \
             express.",
        )
    },
    TierRow {
        group: Some("Output"),
        label: Some("Instrument sound bank (.sf2)"),
        accept: Some(".sf2"),
        ..row(
            "soundfont",
            Tier::Advanced,
            "Power feature; file-pick (designer row 9). Ignored under MIDI out is a PLAY-path \
             property and play is not in this schema (noted for S124).",
        )
    },
    TierRow {
        group: Some("Output"),
        label: Some("Export WAV…"),
        accept: Some(".wav"),
        save: true,
        ..row(
            "wav",
            Tier::Hidden,
            "I/O destination, not a setting; becomes a Save action (designer row 2).",
        )
    },
    TierRow {
        label: Some("Export event dump"),
        accept: Some(".json"),
        save: true,
        ..row(
            "event-dump",
            Tier::Hidden,
            "Diagnostic; developer surface only, under Details (designer row 14).",
        )
    },
    // ── Globals (reach `render` via `global = true`) ─────────────────────────────────────────
    TierRow {
        accept: Some(".toml"),
        ..row(
            "config",
            Tier::Hidden,
            "Config-file path, not a music control (designer g1). Note: main.rs never loads it today.",
        )
    },
    row(
        "json",
        Tier::Hidden,
        "Machine-output mode, not a music control (designer g2). Note: render --wav ignores it today.",
    ),
];

/// Why the schema could not be built. Each message is written for the developer who must fix it.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum SchemaError {
    /// clap has no `render` subcommand (the grammar changed shape).
    #[error("clap grammar has no `{0}` subcommand")]
    NoSubcommand(String),
    /// The table and the grammar disagree: clap args with no row (`missing`) and/or rows naming
    /// an arg clap no longer has (`stale`). Both lists are reported at once.
    #[error("{}{}", missing_rows_message(.missing), stale_rows_message(.stale))]
    Mismatch {
        /// clap args with no `GUI_TIERS` row.
        missing: Vec<String>,
        /// `GUI_TIERS` rows with no clap arg.
        stale: Vec<String>,
    },
    /// A row is listed twice.
    #[error(
        "GUI_TIERS lists `{0}` more than once in src/options_schema.rs; delete the duplicate row"
    )]
    DuplicateRow(String),
    /// A clap arg's value type has no `OptionKind` mapping.
    #[error(
        "render arg `{0}` has a value type src/options_schema.rs::kind_of does not map to an \
         OptionKind; add a mapping there"
    )]
    UnsupportedType(String),
}

fn missing_rows_message(ids: &[String]) -> String {
    if ids.is_empty() {
        return String::new();
    }
    let mut s = String::from(
        "render arg(s) with NO row in GUI_TIERS (src/options_schema.rs). Decide whether the GUI \
         shows each one and add ONE line per arg to GUI_TIERS, e.g.:\n",
    );
    for id in ids {
        s.push_str(&format!(
            "    row(\"{id}\", Tier::Hidden, \"<why this tier — Everyday/Advanced rows must change \
             the rendered bytes>\"),\n"
        ));
    }
    s
}

fn stale_rows_message(ids: &[String]) -> String {
    if ids.is_empty() {
        return String::new();
    }
    let mut s = String::from(
        "GUI_TIERS (src/options_schema.rs) has row(s) for arg(s) the `render` CLI no longer has. \
         Delete these rows:\n",
    );
    for id in ids {
        s.push_str(&format!("    the row with id \"{id}\"\n"));
    }
    s
}

/// The id an arg is known by everywhere: its long name, or its clap id for a positional.
fn arg_id(arg: &clap::Arg) -> String {
    arg.get_long()
        .map(str::to_string)
        .unwrap_or_else(|| arg.get_id().as_str().to_string())
}

/// `true` for clap's own generated args (`--help`, `--version`), which are not render options.
fn is_generated(arg: &clap::Arg) -> bool {
    matches!(
        arg.get_action(),
        ArgAction::Help | ArgAction::HelpShort | ArgAction::HelpLong | ArgAction::Version
    )
}

/// The `render` subcommand's user-facing args, globals propagated, clap-generated args removed.
fn render_args() -> Result<Vec<clap::Arg>, SchemaError> {
    let mut cmd = Cli::command();
    // `build()` propagates `global = true` args (`--config`, `--json`) into the subcommands.
    cmd.build();
    let render = cmd
        .find_subcommand("render")
        .ok_or_else(|| SchemaError::NoSubcommand("render".to_string()))?;
    Ok(render
        .get_arguments()
        .filter(|a| !is_generated(a))
        .cloned()
        .collect())
}

/// Map a clap arg's type to the control to draw.
fn kind_of(arg: &clap::Arg, row: &TierRow) -> Result<OptionKind, SchemaError> {
    if matches!(arg.get_action(), ArgAction::SetTrue) {
        return Ok(OptionKind::Bool);
    }
    let pv = arg.get_possible_values();
    if !pv.is_empty() {
        return Ok(OptionKind::Enum(
            pv.iter().map(|v| v.get_name().to_string()).collect(),
        ));
    }
    let t = arg.get_value_parser().type_id();
    let b = row.bounds;
    if t == TypeId::of::<usize>() || t == TypeId::of::<u64>() {
        Ok(OptionKind::Int {
            min: b.map(|b| b.min as i64),
            max: b.map(|b| b.max as i64),
        })
    } else if t == TypeId::of::<f32>() || t == TypeId::of::<f64>() {
        Ok(OptionKind::Float {
            min: b.map(|b| b.min),
            max: b.map(|b| b.max),
            step: b.map(|b| b.step),
        })
    } else if t == TypeId::of::<V2Spec>() {
        Ok(OptionKind::TermSet(
            V2_TERMS.iter().map(|s| s.to_string()).collect(),
        ))
    } else if t == TypeId::of::<PathBuf>() {
        Ok(OptionKind::FilePick {
            accept: row.accept.unwrap_or("").to_string(),
            save: row.save,
        })
    } else {
        Err(SchemaError::UnsupportedType(arg_id(arg)))
    }
}

/// Check `GUI_TIERS` against the clap grammar, both directions. `Ok(())` ⇔ every `render` arg has
/// exactly one row and every row names a live arg. This is what forcing test 1 asserts.
pub fn check_gui_tiers() -> Result<(), SchemaError> {
    let args = render_args()?;
    let ids: Vec<String> = args.iter().map(arg_id).collect();
    for (i, r) in GUI_TIERS.iter().enumerate() {
        if GUI_TIERS[..i].iter().any(|p| p.id == r.id) {
            return Err(SchemaError::DuplicateRow(r.id.to_string()));
        }
    }
    let missing: Vec<String> = ids
        .iter()
        .filter(|id| !GUI_TIERS.iter().any(|r| r.id == id.as_str()))
        .cloned()
        .collect();
    let stale: Vec<String> = GUI_TIERS
        .iter()
        .filter(|r| !ids.iter().any(|id| id == r.id))
        .map(|r| r.id.to_string())
        .collect();
    if !missing.is_empty() || !stale.is_empty() {
        return Err(SchemaError::Mismatch { missing, stale });
    }
    Ok(())
}

/// The `render` options as data, in clap's declaration order: each clap arg joined with its
/// [`GUI_TIERS`] row. Fails (rather than guessing) when the table and the grammar disagree.
pub fn render_options_schema() -> Result<Vec<OptionSpec>, SchemaError> {
    check_gui_tiers()?;
    let mut out = Vec::new();
    for arg in render_args()? {
        let id = arg_id(&arg);
        let row = GUI_TIERS
            .iter()
            .find(|r| r.id == id)
            .ok_or_else(|| SchemaError::Mismatch {
                missing: vec![id.clone()],
                stale: Vec::new(),
            })?;
        let help = arg.get_help().map(|h| h.to_string()).unwrap_or_default();
        let long_help = arg
            .get_long_help()
            .map(|h| h.to_string())
            .filter(|h| *h != help);
        let defaults: Vec<String> = arg
            .get_default_values()
            .iter()
            .map(|v| v.to_string_lossy().to_string())
            .collect();
        out.push(OptionSpec {
            positional: arg.is_positional(),
            global: arg.is_global_set(),
            label: row.label.unwrap_or(row.id).to_string(),
            help,
            long_help,
            gui_help: row.gui_help.map(str::to_string),
            kind: kind_of(&arg, row)?,
            default: if defaults.is_empty() {
                None
            } else {
                Some(defaults.join(","))
            },
            tier: row.tier,
            group: row.group.map(str::to_string),
            requires: row.requires.iter().map(|s| s.to_string()).collect(),
            bounds: row.bounds,
            reason: row.reason.to_string(),
            id,
        });
    }
    Ok(out)
}

/// [`render_options_schema`] as pretty JSON, for a front-end that consumes the schema as a file.
pub fn render_options_schema_json() -> Result<String, SchemaError> {
    let schema = render_options_schema()?;
    // Serializing plain strings / numbers / enums cannot fail; map defensively anyway (no unwrap).
    serde_json::to_string_pretty(&schema)
        .map_err(|e| SchemaError::UnsupportedType(format!("serde_json: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    /// FORCING TEST 1. Fails if any `render` arg has no `GUI_TIERS` row, or a row names an arg
    /// that no longer exists. The failure message is the exact line(s) to add or delete.
    #[test]
    fn every_render_arg_has_exactly_one_row() {
        if let Err(e) = check_gui_tiers() {
            panic!("\n\nOPTIONS-SCHEMA FORCING TEST 1 FAILED\n{e}\n");
        }
    }

    /// The enumeration the UI Designer counted independently (15 render args + 2 globals). If this
    /// changes, forcing test 1 already told you which row to add; update this count with it.
    #[test]
    fn render_arg_count_matches_designer_enumeration() {
        let s = render_options_schema().expect("schema builds");
        let globals = s.iter().filter(|o| o.global).count();
        assert_eq!(s.len() - globals, 15, "render args (non-global)");
        assert_eq!(globals, 2, "globals reaching render");
        let mut ids: Vec<&str> = s.iter().map(|o| o.id.as_str()).collect();
        ids.sort_unstable();
        assert_eq!(
            ids,
            vec![
                "config",
                "event-dump",
                "expression",
                "gain",
                "image",
                "instruments",
                "jitter-percent",
                "json",
                "loop",
                "ms-per-step",
                "reverb",
                "seed",
                "soundfont",
                "steps",
                "thickness",
                "v2",
                "wav"
            ]
        );
    }

    /// The mechanism really fires: a synthetic missing arg and a synthetic stale row both produce
    /// a message naming the one line to act on. (Checks the message builders, which forcing test 1
    /// can only exercise when it is red.)
    #[test]
    fn forcing_messages_name_the_line() {
        let m = SchemaError::Mismatch {
            missing: vec!["new-thing".into()],
            stale: vec!["old-thing".into()],
        }
        .to_string();
        assert!(
            m.contains("row(\"new-thing\", Tier::Hidden,"),
            "missing-row message must show the line to add: {m}"
        );
        assert!(m.contains("src/options_schema.rs"), "{m}");
        assert!(m.contains("\"old-thing\"") && m.contains("Delete"), "{m}");
    }

    /// The two measured-inert knobs (s121-01 §3.5, re-measured S122-P1) must never be silently
    /// exposed. Promoting either needs it made live first (a sound change, not authorised).
    #[test]
    fn inert_knobs_stay_hidden() {
        for id in ["ms-per-step", "jitter-percent"] {
            let r = GUI_TIERS.iter().find(|r| r.id == id).expect("row exists");
            assert_eq!(r.tier, Tier::Hidden, "{id} is measured inert on render");
            assert!(
                r.reason.contains("INERT"),
                "{id} must carry its recorded reason"
            );
        }
    }

    /// I/O args stay Hidden (s121-01 §5.3).
    #[test]
    fn io_args_stay_hidden() {
        for id in ["wav", "event-dump", "config", "json"] {
            let r = GUI_TIERS.iter().find(|r| r.id == id).expect("row exists");
            assert_eq!(r.tier, Tier::Hidden, "{id} is I/O, not a setting");
        }
    }

    /// Every row records a reason, and every visible row has a group.
    #[test]
    fn rows_are_complete() {
        for r in GUI_TIERS {
            assert!(!r.reason.trim().is_empty(), "{} has no reason", r.id);
            if r.tier != Tier::Hidden {
                assert!(r.group.is_some(), "visible row {} has no group", r.id);
            }
            if let Some(b) = r.bounds {
                assert!(b.min <= b.max && b.step > 0.0, "{} bounds malformed", r.id);
            }
        }
    }

    /// `V2_TERMS` mirrors `V2Spec`: each term parses, and together they arm exactly what `all`
    /// arms. A fourth term that `all` sets but `V2_TERMS` lacks fails here.
    #[test]
    fn v2_terms_cover_all() {
        let all: V2Spec = "all".parse().expect("all parses");
        let joined: V2Spec = V2_TERMS.join(",").parse().expect("terms parse");
        assert_eq!(all, joined);
        for t in V2_TERMS {
            assert!(t.parse::<V2Spec>().is_ok(), "{t}");
        }
    }

    /// Kinds come from clap's types, help from clap's doc comments, defaults from clap.
    #[test]
    fn kinds_help_and_defaults_come_from_clap() {
        let s = render_options_schema().expect("schema builds");
        let get = |id: &str| s.iter().find(|o| o.id == id).expect(id).clone();
        assert_eq!(get("loop").kind, OptionKind::Bool);
        assert_eq!(
            get("reverb").kind,
            OptionKind::Enum(vec!["on".into(), "off".into()])
        );
        assert!(matches!(
            get("instruments").kind,
            OptionKind::Int {
                min: Some(1),
                max: Some(16)
            }
        ));
        assert!(matches!(
            get("seed").kind,
            OptionKind::Int {
                min: None,
                max: None
            }
        ));
        assert!(matches!(get("gain").kind, OptionKind::Float { .. }));
        assert_eq!(
            get("v2").kind,
            OptionKind::TermSet(vec!["e3".into(), "e2".into(), "ta".into()])
        );
        assert_eq!(
            get("soundfont").kind,
            OptionKind::FilePick {
                accept: ".sf2".into(),
                save: false
            }
        );
        assert_eq!(get("instruments").default.as_deref(), Some("4"));
        assert_eq!(get("ms-per-step").default.as_deref(), Some("250"));
        assert_eq!(get("seed").default, None);
        // clap's derive renders a one-sentence doc comment without its trailing period; the
        // schema carries what clap renders, so `--help` and the GUI show the same string.
        assert_eq!(
            get("instruments").help,
            "Number of instruments in the ensemble"
        );
        assert!(get("image").positional);
        assert!(get("config").global && get("json").global);
        assert_eq!(get("v2").requires, vec!["expression=on".to_string()]);
        // The designer's help sits NEXT TO clap's, it does not replace it.
        let seed = get("seed");
        assert!(seed.help.starts_with("S41: deterministic composition seed"));
        assert!(seed.gui_help.is_some());
    }

    /// Every `requires` entry names a live option and a value clap would accept for it.
    #[test]
    fn requires_name_live_options_with_valid_values() {
        let s = render_options_schema().expect("schema builds");
        for o in &s {
            for req in &o.requires {
                let (id, val) = req.split_once('=').expect("requires is id=value");
                let target = s
                    .iter()
                    .find(|t| t.id == id)
                    .unwrap_or_else(|| panic!("{}: requires names unknown option {id}", o.id));
                let argv = ["audiohax", "render", &format!("--{id}"), val];
                assert!(
                    Cli::try_parse_from(argv).is_ok(),
                    "{}: requires value {val} not accepted for {}",
                    o.id,
                    target.id
                );
            }
        }
    }

    #[test]
    fn schema_serializes_to_json() {
        let j = render_options_schema_json().expect("json");
        // Visible with `-- --nocapture`: the schema exactly as a front-end would receive it.
        println!("{j}");
        assert!(j.contains("\"id\": \"ms-per-step\""));
        assert!(j.contains("\"tier\": \"Hidden\""));
    }
}

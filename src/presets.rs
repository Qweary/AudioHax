//! src/presets.rs — the PRESET layer (S124 slice 1; lead rulings L-28…L-33, design `s124-01` § 2.3 / § 3).
//!
//! A preset is a named, curated starting point for the sound: a mappings table plus, optionally, a
//! set of option values, and (in the FORMAT, not yet in slice 1) a sound bank, a voice-count policy,
//! a picture pre-pass, a steps mode and a seed policy. The presets live in ONE manifest compiled into
//! the binary ([`MANIFEST_JSON`], `assets/presets/manifest.json`); nothing is read from disk, and the
//! manifest names no file path and no output option (L-32, `s124-SH` PB-1…PB-3).
//!
//! **Slice 1 ships only `default`** (L-28): the embedded mappings, no settings, the bundled sound
//! bank, image-derived seed. `--preset default` therefore renders byte-for-byte what no flag renders
//! from a directory holding the shipped `assets/mappings.json` (L-29).
//!
//! **Refuse, never ignore.** The manifest format can already say everything `s124-09` § 5 asks a
//! preset to carry (sound bank, voice count, picture pre-pass, steps mode, seed policy,
//! provenance). Whatever slice 1 cannot HONOUR is refused when the manifest is loaded
//! ([`PresetError::Unsupported`]), with the reason; it is never parsed and then dropped. Every type
//! is `deny_unknown_fields`, so a misspelt key is a refusal too (the file is ours; `mappings.json`'s
//! permissive policy, L-32, does not apply here).
//!
//! **Precedence** (L-30): what the user typed > the preset > clap's defaults. The CLI learns what was
//! typed from clap's `ArgMatches::value_source` ([`ExplicitArgs`]); re-parsing preset tokens together
//! with the user's would collide with clap's refusal of a repeated flag (`s124-01` H5).
//! `RenderRequest::overlay_preset` (`src/render.rs`) applies the table field by field.
//!
//! **Listing** (L-30): `--preset`'s values are clap possible values ([`preset_value_parser`]), so
//! `render --help` / `play --help` list them and an unknown name gets clap's own error.
//!
//! Pure: `clap`, `serde`, `serde_json`, `thiserror` only, so this module builds and unit-tests under
//! `cargo test --lib --no-default-features`.

use crate::cli::{Cli, Command, RenderArgs};
use crate::mapping_loader::MappingTable;
use clap::{ArgAction, CommandFactory, FromArgMatches};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

/// The preset manifest, compiled in.
pub const MANIFEST_JSON: &str = include_str!("../assets/presets/manifest.json");

/// The only manifest version this build reads.
pub const MANIFEST_VERSION: u32 = 1;

/// The option ids (clap long names) a preset's `settings` may carry. Everything else is refused at
/// manifest load with the reason from [`not_presettable_reason`].
///
/// `seed` is NOT here: a preset's seed is its [`SeedPolicy`] (image-derived), and pinning a number
/// would give every picture the same seed (`s124-09` § 5 P6, `s124-12` § Q4 item 5).
pub const PRESET_SETTING_IDS: &[&str] = &[
    "instruments",
    "thickness",
    "steps",
    "expression",
    "v2",
    "reverb",
    "gain",
];

/// Why an option id may not appear in a preset's `settings`.
pub fn not_presettable_reason(id: &str) -> &'static str {
    match id {
        "seed" => {
            "the seed is the preset's `seed` policy (image-derived); a pinned number would give \
             every picture the same seed"
        }
        "ms-per-step" | "jitter-percent" => {
            "measured inert on render; a preset carrying it would claim an effect it does not have"
        }
        "loop" => "a render-only output mode that `play` cannot honour",
        "soundfont" => "a file path; a preset's sound bank is its `soundfont` field",
        "wav" | "event-dump" | "image" | "config" | "json" | "preset" | "mappings" => {
            "input/output, not music; a preset names no file and no output"
        }
        _ => "not a render option a preset may set",
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// The manifest format
// ─────────────────────────────────────────────────────────────────────────────

/// The whole manifest.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// Must equal [`MANIFEST_VERSION`].
    pub manifest_version: u32,
    /// The id of the preset a front-end selects first.
    pub default: String,
    /// The presets, in display order.
    pub presets: Vec<Preset>,
}

/// One preset. Every field is REQUIRED (a manifest states each policy explicitly).
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preset {
    /// ASCII kebab-case; the `--preset` value; stable forever.
    pub id: String,
    /// What the window shows.
    pub name: String,
    /// One plain sentence.
    pub description: String,
    /// The mappings table.
    pub mappings: MappingsSource,
    /// Option values, clap long id → clap value string (ids from [`PRESET_SETTING_IDS`]).
    pub settings: BTreeMap<String, String>,
    /// The sound bank the preset was heard on (`s124-09` § 5 P2).
    pub soundfont: SoundFontChoice,
    /// The voice-count policy (`s124-09` § 5 P3).
    pub voices: VoicePolicy,
    /// Picture operations applied before analysis, in order (`s124-09` § 5 P4).
    pub picture: Vec<PictureOp>,
    /// How many scan steps (`s124-09` § 5 P5).
    pub steps: StepsMode,
    /// Where the composition seed comes from (`s124-09` § 5 P6).
    pub seed: SeedPolicy,
    /// Who certified it, and on what (`s124-09` § 5 P8).
    pub provenance: Provenance,
}

/// Where a preset's mappings come from.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum MappingsSource {
    /// A whole mappings file compiled in under this name (`"default"` = the built-in mappings).
    /// JSON: `{"builtin": "default"}`.
    Builtin(String),
    /// A JSON merge patch over a builtin. JSON: `{"patched": {"base": …, "merge_patch": {…}}}`.
    /// NOT honoured in slice 1 (refused).
    Patched {
        /// The builtin it patches.
        base: String,
        /// RFC 7386 merge patch.
        merge_patch: serde_json::Value,
    },
}

/// The sound bank.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum SoundFontChoice {
    /// The bundled GeneralUser GS font. JSON: `"bundled"`.
    Bundled,
    /// Another font, by name and content hash. NOT honoured in slice 1 (L-31: no non-bundled bank
    /// until the operator answers; L-13: the `rustysynth` odd-chunk check first).
    Named {
        /// Display name.
        name: String,
        /// sha256 of the `.sf2`.
        sha256: String,
    },
}

/// The voice-count policy.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum VoicePolicy {
    /// The user's (or the default) voice count. JSON: `"inherit"`.
    Inherit,
    /// Locked to this count. NOT honoured in slice 1.
    Fixed(u32),
    /// Limited to a range. NOT honoured in slice 1.
    Range {
        /// Fewest voices.
        min: u32,
        /// Most voices.
        max: u32,
    },
}

/// One picture operation, applied before analysis.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields, tag = "op", rename_all = "snake_case")]
pub enum PictureOp {
    /// Shift luma by `amount` (−1..1), as `ffmpeg -vf eq=brightness=<amount>` does (`s124-12` § Q4
    /// item 3). NOT honoured in slice 1.
    LumaOffset {
        /// The offset.
        amount: f32,
    },
}

/// How many scan steps a preset renders with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepsMode {
    /// `--steps` as given (default 40). The identity value.
    EngineDefault,
    /// `--steps` = the composed piece's own step count. NOT honoured in slice 1.
    Matched,
}

/// Where the composition seed comes from.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum SeedPolicy {
    /// From the picture (S91), unless the user gives `--seed`. JSON: `"image"`.
    Image,
    /// A pinned seed. NOT honoured (refused): every picture would draw the same seed.
    Fixed(u64),
}

/// Certification state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PresetStatus {
    /// Accepted by the ear and shipped.
    Certified,
    /// Not yet accepted. NOT honoured in slice 1 (no surface shows or hides a candidate).
    Candidate,
}

/// Provenance: the trail back to what was heard.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    /// Certification state.
    pub status: PresetStatus,
    /// What certified it (plain text; not a path the program reads).
    pub record: String,
    /// The slug the preset descends from.
    #[serde(default)]
    pub lineage: Option<String>,
    /// What the certifying render was made with.
    #[serde(default)]
    pub heard_on: Option<HeardOn>,
}

/// What a certifying render was made with (record only).
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HeardOn {
    /// Sound bank name.
    pub soundfont: String,
    /// Seed used, if pinned for the listen.
    #[serde(default)]
    pub seed: Option<u64>,
    /// `--steps` used.
    #[serde(default)]
    pub steps: Option<String>,
    /// sha256 of the binary.
    #[serde(default)]
    pub binary_sha256: Option<String>,
    /// sha256 of the mappings file.
    #[serde(default)]
    pub mappings_sha256: Option<String>,
}

/// Why a manifest or a preset was refused.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum PresetError {
    /// The manifest text is not a manifest (JSON syntax, an unknown key, a wrong type).
    #[error("the preset manifest does not parse: {0}")]
    Parse(String),
    /// A manifest-level problem.
    #[error("the preset manifest is invalid: {0}")]
    Manifest(String),
    /// No preset has this id.
    #[error("unknown preset `{0}`")]
    Unknown(String),
    /// A `settings` key that a preset may not set.
    #[error("preset `{id}`: setting `{setting}` is not presettable: {why}")]
    NotPresettable {
        /// Preset id.
        id: String,
        /// The offending setting.
        setting: String,
        /// Why.
        why: &'static str,
    },
    /// A field this build can express but cannot honour yet (L-28: refused, never ignored).
    #[error("preset `{id}`: `{field}` is not supported by this build: {why}")]
    Unsupported {
        /// Preset id.
        id: String,
        /// The field, with its value.
        field: String,
        /// Why.
        why: &'static str,
    },
    /// A preset whose values are malformed.
    #[error("preset `{id}`: {msg}")]
    Invalid {
        /// Preset id.
        id: String,
        /// What is wrong.
        msg: String,
    },
    /// The preset's mappings were refused by the mappings validation.
    #[error("preset `{id}`: {msg}")]
    Mappings {
        /// Preset id.
        id: String,
        /// The mappings error's text.
        msg: String,
    },
}

// ─────────────────────────────────────────────────────────────────────────────
// Loading and validation
// ─────────────────────────────────────────────────────────────────────────────

/// Parse and structurally validate a manifest text ([`validate_manifest`]). Does NOT call clap, so it
/// is safe inside [`manifest`]'s one-time initialisation (clap's grammar itself reads the manifest
/// for `--preset`'s possible values).
pub fn parse_manifest(json: &str) -> Result<Manifest, PresetError> {
    let m: Manifest = serde_json::from_str(json).map_err(|e| PresetError::Parse(e.to_string()))?;
    validate_manifest(&m)?;
    Ok(m)
}

/// The embedded manifest, parsed and validated once. An `Err` here means the EMBEDDED file is
/// malformed — a build defect that the unit test `embedded_manifest_parses_and_validates` turns
/// into a red build. It is returned, never panicked on: with an invalid manifest the CLI offers no
/// `--preset` value at all and the window shows the error.
pub fn manifest() -> Result<&'static Manifest, PresetError> {
    static MANIFEST: OnceLock<Result<Manifest, PresetError>> = OnceLock::new();
    MANIFEST
        .get_or_init(|| parse_manifest(MANIFEST_JSON))
        .as_ref()
        .map_err(Clone::clone)
}

/// Preset ids in manifest order (empty if the manifest is invalid).
pub fn preset_ids() -> Vec<&'static str> {
    manifest()
        .map(|m| m.presets.iter().map(|p| p.id.as_str()).collect())
        .unwrap_or_default()
}

/// The preset with this id.
pub fn find(id: &str) -> Result<&'static Preset, PresetError> {
    manifest()?
        .presets
        .iter()
        .find(|p| p.id == id)
        .ok_or_else(|| PresetError::Unknown(id.to_string()))
}

/// The manifest's default preset.
pub fn default_preset() -> Result<&'static Preset, PresetError> {
    find(&manifest()?.default)
}

/// clap value parser for `--preset`: the manifest's ids, each carrying the preset's `name` as its
/// help. An unknown id gets clap's own `invalid value … [possible values: …]` error, which is also
/// how the presets are listed (L-30, `s124-01` § 4.3).
pub fn preset_value_parser() -> clap::builder::PossibleValuesParser {
    let values: Vec<clap::builder::PossibleValue> = manifest()
        .map(|m| {
            m.presets
                .iter()
                .map(|p| clap::builder::PossibleValue::new(p.id.as_str()).help(p.name.as_str()))
                .collect()
        })
        .unwrap_or_default();
    clap::builder::PossibleValuesParser::new(values)
}

/// `true` for an id of ASCII lowercase letters, digits and single inner hyphens.
fn is_kebab(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 32
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        && !id.starts_with('-')
        && !id.ends_with('-')
        && !id.contains("--")
}

/// The structural checks: version, unique kebab-case ids, plain one-line name and description,
/// `default` exists and is the canonical identity preset, every `settings` key presettable, and
/// every field slice 1 cannot honour refused (L-28). Calls no clap.
pub fn validate_manifest(m: &Manifest) -> Result<(), PresetError> {
    if m.manifest_version != MANIFEST_VERSION {
        return Err(PresetError::Manifest(format!(
            "manifest_version {} is not {MANIFEST_VERSION}",
            m.manifest_version
        )));
    }
    if m.presets.is_empty() {
        return Err(PresetError::Manifest("it holds no preset".into()));
    }
    let mut seen = BTreeSet::new();
    for p in &m.presets {
        if !is_kebab(&p.id) {
            return Err(PresetError::Invalid {
                id: p.id.escape_debug().to_string(),
                msg: "the id must be 1-32 characters of a-z, 0-9 and inner single hyphens".into(),
            });
        }
        if !seen.insert(p.id.as_str()) {
            return Err(PresetError::Manifest(format!(
                "the id `{}` appears twice",
                p.id
            )));
        }
        validate_preset(p)?;
    }
    let default = m
        .presets
        .iter()
        .find(|p| p.id == m.default)
        .ok_or_else(|| {
            PresetError::Manifest(format!(
                "`default` names `{}`, which is not a preset",
                m.default.escape_debug()
            ))
        })?;
    // The byte-identity anchor (L-29): the default preset is the plain embedded default, forced.
    let canonical = default.mappings == MappingsSource::Builtin("default".into())
        && default.settings.is_empty()
        && default.soundfont == SoundFontChoice::Bundled
        && default.voices == VoicePolicy::Inherit
        && default.picture.is_empty()
        && default.steps == StepsMode::EngineDefault
        && default.seed == SeedPolicy::Image;
    if !canonical {
        return Err(PresetError::Invalid {
            id: default.id.clone(),
            msg: "the default preset must be exactly the built-in mappings with no settings, the \
                  bundled sound bank, inherited voices, no picture operation, the default steps \
                  and the image seed (it is the byte-identity anchor)"
                .into(),
        });
    }
    Ok(())
}

/// One preset's structural checks (see [`validate_manifest`]).
fn validate_preset(p: &Preset) -> Result<(), PresetError> {
    let id = || p.id.clone();
    let one_line = |what: &str, s: &str| -> Result<(), PresetError> {
        if s.trim().is_empty() || s.chars().any(char::is_control) {
            return Err(PresetError::Invalid {
                id: id(),
                msg: format!("the {what} must be one non-empty line of plain text"),
            });
        }
        Ok(())
    };
    one_line("name", &p.name)?;
    one_line("description", &p.description)?;
    for key in p.settings.keys() {
        if !PRESET_SETTING_IDS.contains(&key.as_str()) {
            return Err(PresetError::NotPresettable {
                id: id(),
                setting: key.escape_debug().to_string(),
                why: not_presettable_reason(key),
            });
        }
    }
    check_settings_render_together(p)?;
    let unsupported = |field: String, why: &'static str| PresetError::Unsupported {
        id: id(),
        field,
        why,
    };
    match &p.mappings {
        MappingsSource::Builtin(name) if name == "default" => {}
        MappingsSource::Builtin(name) => {
            return Err(unsupported(
                format!("mappings.builtin = {}", name.escape_debug()),
                "this build embeds only the `default` mappings",
            ))
        }
        MappingsSource::Patched { .. } => {
            return Err(unsupported(
                "mappings.patched".into(),
                "merge-patched mappings arrive with the first non-default preset",
            ))
        }
    }
    if let SoundFontChoice::Named { name, .. } = &p.soundfont {
        return Err(unsupported(
            format!("soundfont = {}", name.escape_debug()),
            "only the bundled sound bank ships until the sound-bank question is answered",
        ));
    }
    if p.voices != VoicePolicy::Inherit {
        return Err(unsupported(
            format!("voices = {:?}", p.voices),
            "voice-count locks are not built yet",
        ));
    }
    if !p.picture.is_empty() {
        return Err(unsupported(
            format!("picture = {:?}", p.picture),
            "picture operations are not built yet",
        ));
    }
    if p.steps != StepsMode::EngineDefault {
        return Err(unsupported(
            "steps = matched".into(),
            "matched scan steps are not built yet",
        ));
    }
    if let SeedPolicy::Fixed(n) = p.seed {
        return Err(unsupported(
            format!("seed = {n}"),
            "a pinned seed gives every picture the same seed",
        ));
    }
    if p.provenance.status != PresetStatus::Certified {
        return Err(unsupported(
            "provenance.status = candidate".into(),
            "no surface in this build shows or hides a candidate preset",
        ));
    }
    Ok(())
}

/// Settings that are each valid but can never render TOGETHER are refused when the manifest is
/// loaded (lead ruling L-42, finding F-2), so a preset can never ship that fails at every render
/// naming a flag the user did not type. Today the one such pair among the presettable ids is
/// `v2` without `expression = on`: `render::compose` refuses "--v2 requires `--expression on`".
/// Reads the values with [`clap::ValueEnum::from_str`] only — no command is built, so this is
/// safe inside [`manifest`]'s one-time initialisation.
fn check_settings_render_together(p: &Preset) -> Result<(), PresetError> {
    use crate::cli::Toggle;
    if p.settings.contains_key("v2") {
        let expression_on = p
            .settings
            .get("expression")
            .and_then(|v| <Toggle as clap::ValueEnum>::from_str(v, false).ok())
            .is_some_and(Toggle::is_on);
        if !expression_on {
            return Err(PresetError::Invalid {
                id: p.id.clone(),
                msg: "setting `v2` needs `expression` = `on` in the same preset (a render refuses \
                      v2 terms with expression off, so this preset could never play)"
                    .into(),
            });
        }
    }
    Ok(())
}

/// The checks that need clap's grammar (so they cannot run inside [`manifest`]'s initialisation):
/// the preset's `settings` parse as `render` options ([`preset_render_args`]); each names an
/// `Everyday` or `Advanced` row of the options schema (PB-1); and each numeric value lies inside
/// that row's GUI bounds (PB-2 — clap alone accepts NaN, negatives and infinities on five flags).
/// The unit test `every_preset_passes_the_cli_checks` runs this over the embedded manifest.
pub fn validate_against_cli(p: &Preset) -> Result<(), PresetError> {
    use crate::options_schema::{render_options_schema, Tier};
    preset_render_args(p)?;
    let schema = render_options_schema().map_err(|e| PresetError::Invalid {
        id: p.id.clone(),
        msg: format!("the options schema could not be built: {e}"),
    })?;
    for (key, value) in &p.settings {
        let row =
            schema
                .iter()
                .find(|o| o.id == *key)
                .ok_or_else(|| PresetError::NotPresettable {
                    id: p.id.clone(),
                    setting: key.clone(),
                    why: not_presettable_reason(key),
                })?;
        if !matches!(row.tier, Tier::Everyday | Tier::Advanced) {
            return Err(PresetError::NotPresettable {
                id: p.id.clone(),
                setting: key.clone(),
                why: "a Hidden option (input/output or measured inert)",
            });
        }
        if let Some(b) = row.bounds {
            let v: f64 = value.parse().map_err(|_| PresetError::Invalid {
                id: p.id.clone(),
                msg: format!(
                    "setting `{key}` = `{}` is not a number",
                    value.escape_debug()
                ),
            })?;
            if !(v.is_finite() && v >= b.min && v <= b.max) {
                return Err(PresetError::Invalid {
                    id: p.id.clone(),
                    msg: format!(
                        "setting `{key}` = {v} is outside {}..={} (the window's bounds)",
                        b.min, b.max
                    ),
                });
            }
        }
    }
    Ok(())
}

/// The preset's `settings` parsed BY CLAP as a `render` invocation: `["audiohax", "render",
/// "--<id>", "<value>", …]`. The values are validated by the very grammar the CLI uses. With empty
/// settings this is clap's all-defaults `RenderArgs`.
pub fn preset_render_args(p: &Preset) -> Result<RenderArgs, PresetError> {
    let mut argv: Vec<String> = vec!["audiohax".into(), "render".into()];
    for (k, v) in &p.settings {
        if !PRESET_SETTING_IDS.contains(&k.as_str()) {
            return Err(PresetError::NotPresettable {
                id: p.id.clone(),
                setting: k.clone(),
                why: not_presettable_reason(k),
            });
        }
        argv.push(format!("--{k}"));
        argv.push(v.clone());
    }
    let cli = Cli::try_parse_from_argv(&argv).map_err(|e| PresetError::Invalid {
        id: p.id.clone(),
        msg: format!("its settings do not parse as options: {}", e.kind()),
    })?;
    match cli.command {
        Command::Render(args) => Ok(args),
        _ => Err(PresetError::Invalid {
            id: p.id.clone(),
            msg: "its settings did not parse as a render".into(),
        }),
    }
}

/// Small private shim so this module does not need `clap::Parser` in scope at every call.
trait TryParseArgv: Sized {
    fn try_parse_from_argv(argv: &[String]) -> Result<Self, clap::Error>;
}

impl TryParseArgv for Cli {
    fn try_parse_from_argv(argv: &[String]) -> Result<Self, clap::Error> {
        let m = Cli::command().try_get_matches_from(argv)?;
        Cli::from_arg_matches(&m)
    }
}

/// The preset's mappings, through the mappings validation chokepoint (L-32). `builtin: "default"`
/// is exactly [`crate::mapping_loader::default_mappings`].
pub fn resolve_mappings(p: &Preset) -> Result<MappingTable, PresetError> {
    match &p.mappings {
        MappingsSource::Builtin(name) if name == "default" => {
            crate::mapping_loader::default_mappings().map_err(|e| PresetError::Mappings {
                id: p.id.clone(),
                msg: e.to_string(),
            })
        }
        other => Err(PresetError::Unsupported {
            id: p.id.clone(),
            field: format!("mappings = {other:?}"),
            why: "this build embeds only the `default` mappings",
        }),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// What the user typed (L-30 precedence)
// ─────────────────────────────────────────────────────────────────────────────

/// The clap LONG names (or positional ids) the user actually typed for the chosen subcommand
/// (`ArgMatches::value_source == CommandLine`). Typed-equal-to-default still counts as typed:
/// `--preset X --instruments 4` gives 4 even if X says 6. The window and the TUI use
/// [`ExplicitArgs::none`] (nothing typed; their controls carry the user's adjustments).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExplicitArgs(BTreeSet<String>);

impl ExplicitArgs {
    /// Nothing typed.
    pub fn none() -> Self {
        ExplicitArgs::default()
    }

    /// Was the option with this clap long name typed?
    pub fn contains(&self, long: &str) -> bool {
        self.0.contains(long)
    }

    /// Read the typed set off top-level matches built by [`Cli`]'s grammar. Reads only
    /// `value_source`, never removes a value, so it may run before or after
    /// `Cli::from_arg_matches` (the `_mut` variant REMOVES values; do not call this after it).
    pub fn from_matches(m: &clap::ArgMatches) -> Self {
        let mut set = BTreeSet::new();
        let Some((name, sub)) = m.subcommand() else {
            return ExplicitArgs(set);
        };
        let mut cmd = Cli::command();
        cmd.build();
        let Some(sc) = cmd.find_subcommand(name) else {
            return ExplicitArgs(set);
        };
        for arg in sc.get_arguments() {
            if matches!(
                arg.get_action(),
                ArgAction::Help | ArgAction::HelpShort | ArgAction::HelpLong | ArgAction::Version
            ) {
                continue;
            }
            let id = arg.get_id().as_str();
            if sub.value_source(id) == Some(clap::parser::ValueSource::CommandLine) {
                set.insert(
                    arg.get_long()
                        .map(str::to_string)
                        .unwrap_or_else(|| id.to_string()),
                );
            }
        }
        ExplicitArgs(set)
    }
}

/// Parse `argv` exactly as the CLI does (`Cli::command().try_get_matches_from`, then
/// `from_arg_matches`), and read the typed set from the SAME matches. `main.rs` does the same with
/// `get_matches()` (clap's own `parse()` body, so every clap message is unchanged).
pub fn parse_with_explicit<I, T>(argv: I) -> Result<(Cli, ExplicitArgs), clap::Error>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let m = Cli::command().try_get_matches_from(argv)?;
    let explicit = ExplicitArgs::from_matches(&m);
    let cli = Cli::from_arg_matches(&m)?;
    Ok((cli, explicit))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A copy of the shipped manifest with `edit` applied to its first preset.
    fn manifest_with(edit: impl FnOnce(&mut serde_json::Value)) -> Result<Manifest, PresetError> {
        let mut v: serde_json::Value = serde_json::from_str(MANIFEST_JSON).expect("json");
        edit(&mut v["presets"][0]);
        parse_manifest(&serde_json::to_string(&v).expect("ser"))
    }

    /// A second, non-default preset appended to the shipped manifest.
    fn with_second(edit: impl FnOnce(&mut serde_json::Value)) -> Result<Manifest, PresetError> {
        let mut v: serde_json::Value = serde_json::from_str(MANIFEST_JSON).expect("json");
        let mut p = v["presets"][0].clone();
        p["id"] = serde_json::json!("second");
        edit(&mut p);
        v["presets"].as_array_mut().expect("array").push(p);
        parse_manifest(&serde_json::to_string(&v).expect("ser"))
    }

    /// PB-4 / T-7: the embedded manifest parses and validates; it holds exactly `default` (L-28),
    /// which is the canonical identity preset; and its mappings are the embedded default's.
    #[test]
    fn embedded_manifest_parses_and_validates() {
        let m = manifest().expect("embedded manifest is valid");
        assert_eq!(m.default, "default");
        assert_eq!(
            preset_ids(),
            vec!["default"],
            "slice 1 ships only `default` (L-28)"
        );
        let d = default_preset().expect("default");
        assert_eq!(d.name, "Default");
        assert!(d.settings.is_empty());
        let t = resolve_mappings(d).expect("default mappings");
        let e = crate::mapping_loader::default_mappings().expect("embedded");
        assert_eq!(t.composition, e.composition);
    }

    /// PB-1 / PB-2 / PB-4: every embedded preset passes the clap-side checks.
    #[test]
    fn every_preset_passes_the_cli_checks() {
        for p in &manifest().expect("manifest").presets {
            validate_against_cli(p).unwrap_or_else(|e| panic!("{e}"));
        }
    }

    /// L-42 / F-2: `v2` without `expression = on` is refused at manifest load (with expression
    /// absent, `off`, or not a toggle value); with `expression = on` the same preset loads.
    #[test]
    fn v2_without_expression_on_is_refused_at_load() {
        for settings in [
            serde_json::json!({"v2": "e3"}),
            serde_json::json!({"v2": "e3", "expression": "off"}),
            serde_json::json!({"v2": "all", "expression": "ON"}),
        ] {
            let got = with_second(|p| p["settings"] = settings.clone());
            match got {
                Err(PresetError::Invalid { id, msg }) => {
                    assert_eq!(id, "second");
                    assert!(msg.contains("`v2` needs `expression` = `on`"), "{msg}");
                }
                other => panic!("{settings}: expected a refusal, got {other:?}"),
            }
        }
        let m =
            with_second(|p| p["settings"] = serde_json::json!({"v2": "e3", "expression": "on"}))
                .expect("v2 with expression on loads");
        let second = m.presets.iter().find(|p| p.id == "second").expect("second");
        validate_against_cli(second).expect("and passes the clap-side checks");
    }

    /// The default preset's `RenderArgs` are clap's all-defaults `RenderArgs` (the identity).
    #[test]
    fn default_preset_render_args_are_clap_defaults() {
        let d = default_preset().expect("default");
        let (cli, _) = parse_with_explicit(["audiohax", "render"]).expect("parse");
        let Command::Render(defaults) = cli.command else {
            panic!("render")
        };
        assert_eq!(preset_render_args(d).expect("args"), defaults);
    }

    /// L-28: every field the format can express but slice 1 cannot honour is REFUSED, with the
    /// field named — on a non-default preset, so the canonical-default check does not mask it.
    #[test]
    fn unhonoured_fields_are_refused_not_ignored() {
        let cases: Vec<(&str, Box<dyn FnOnce(&mut serde_json::Value)>)> = vec![
            (
                "soundfont",
                Box::new(|p| {
                    p["soundfont"] =
                        serde_json::json!({"named": {"name": "MuseScore_General", "sha256": "x"}})
                }),
            ),
            (
                "voices",
                Box::new(|p| p["voices"] = serde_json::json!({"fixed": 4})),
            ),
            (
                "voices",
                Box::new(|p| p["voices"] = serde_json::json!({"range": {"min": 1, "max": 4}})),
            ),
            (
                "picture",
                Box::new(|p| {
                    p["picture"] = serde_json::json!([{"op": "luma_offset", "amount": -0.3}])
                }),
            ),
            (
                "steps",
                Box::new(|p| p["steps"] = serde_json::json!("matched")),
            ),
            (
                "seed",
                Box::new(|p| p["seed"] = serde_json::json!({"fixed": 114})),
            ),
            (
                "mappings.patched",
                Box::new(|p| {
                    p["mappings"] =
                        serde_json::json!({"patched": {"base": "default", "merge_patch": {}}})
                }),
            ),
            (
                "mappings.builtin",
                Box::new(|p| p["mappings"] = serde_json::json!({"builtin": "edit-a"})),
            ),
            (
                "provenance.status",
                Box::new(|p| p["provenance"]["status"] = serde_json::json!("candidate")),
            ),
        ];
        for (field, edit) in cases {
            match with_second(edit) {
                Err(PresetError::Unsupported { field: f, .. }) => {
                    assert!(f.starts_with(field), "{field}: got field {f}")
                }
                other => panic!("{field}: expected Unsupported, got {other:?}"),
            }
        }
    }

    /// PB-1 / H13: a preset naming an I/O option, an inert knob, the seed, or a non-option is
    /// refused with the reason; a misspelt manifest key is refused too (deny_unknown_fields).
    #[test]
    fn non_presettable_settings_and_unknown_keys_are_refused() {
        for id in [
            "wav",
            "event-dump",
            "soundfont",
            "image",
            "config",
            "json",
            "preset",
            "mappings",
            "ms-per-step",
            "jitter-percent",
            "loop",
            "seed",
            "nonsense",
        ] {
            match with_second(|p| p["settings"] = serde_json::json!({ id: "1" })) {
                Err(PresetError::NotPresettable { setting, why, .. }) => {
                    assert_eq!(setting, id);
                    assert_eq!(why, not_presettable_reason(id));
                }
                other => panic!("{id}: expected NotPresettable, got {other:?}"),
            }
        }
        assert!(matches!(
            manifest_with(|p| p["descripton"] = serde_json::json!("typo")),
            Err(PresetError::Parse(_))
        ));
        // Every field is required: a missing policy is a refusal, not a silent default.
        assert!(matches!(
            manifest_with(|p| {
                p.as_object_mut().expect("obj").remove("seed");
            }),
            Err(PresetError::Parse(_))
        ));
    }

    /// PB-2 / H13: a preset value outside the GUI bounds (clap would accept NaN) is refused.
    #[test]
    fn out_of_bounds_values_are_refused() {
        for (k, v) in [
            ("gain", "NaN"),
            ("gain", "-1"),
            ("steps", "100000"),
            ("instruments", "0"),
        ] {
            let m = with_second(|p| p["settings"] = serde_json::json!({ k: v }))
                .expect("structural ok");
            let p = m.presets.iter().find(|p| p.id == "second").expect("second");
            assert!(
                matches!(validate_against_cli(p), Err(PresetError::Invalid { .. })),
                "{k}={v}"
            );
        }
        // A value clap rejects outright.
        let m = with_second(|p| p["settings"] = serde_json::json!({"expression": "maybe"}))
            .expect("ok");
        let p = m.presets.iter().find(|p| p.id == "second").expect("second");
        assert!(validate_against_cli(p).is_err());
        // In-bounds values pass.
        let m = with_second(|p| p["settings"] = serde_json::json!({"gain": "0.8", "instruments": "6", "expression": "on", "v2": "e3"})).expect("ok");
        let p = m.presets.iter().find(|p| p.id == "second").expect("second");
        validate_against_cli(p).expect("in bounds");
    }

    /// The default preset must stay the identity: giving it a setting, or any other identity-
    /// breaking value, is refused.
    #[test]
    fn the_default_preset_is_forced_canonical() {
        assert!(matches!(
            manifest_with(|p| p["settings"] = serde_json::json!({"gain": "0.8"})),
            Err(PresetError::Invalid { .. })
        ));
    }

    /// Manifest-level checks: version, duplicate ids, bad ids, a missing default.
    #[test]
    fn manifest_level_checks() {
        let mut v: serde_json::Value = serde_json::from_str(MANIFEST_JSON).expect("json");
        v["manifest_version"] = serde_json::json!(2);
        assert!(parse_manifest(&v.to_string()).is_err());
        assert!(with_second(|p| p["id"] = serde_json::json!("default")).is_err());
        for bad in ["", "Upper", "a b", "-x", "x-", "a--b", "x\u{1b}"] {
            assert!(
                with_second(|p| p["id"] = serde_json::json!(bad)).is_err(),
                "{bad:?}"
            );
        }
        let mut v: serde_json::Value = serde_json::from_str(MANIFEST_JSON).expect("json");
        v["default"] = serde_json::json!("nope");
        assert!(parse_manifest(&v.to_string()).is_err());
        assert!(with_second(|p| p["name"] = serde_json::json!("two\nlines")).is_err());
    }

    /// T-4: the typed set. Typed-equal-to-default counts; untyped does not; it is read from the
    /// same matches the struct is built from (no `_mut` removal hazard).
    #[test]
    fn explicit_args_are_what_was_typed() {
        let (cli, ex) = parse_with_explicit([
            "audiohax",
            "render",
            "x.png",
            "--wav",
            "o.wav",
            "--instruments",
            "4",
            "--ms-per-step",
            "250",
        ])
        .expect("parse");
        assert!(matches!(cli.command, Command::Render(_)));
        for typed in ["instruments", "ms-per-step", "wav", "image"] {
            assert!(ex.contains(typed), "{typed} was typed: {ex:?}");
        }
        for untyped in [
            "steps",
            "gain",
            "expression",
            "preset",
            "seed",
            "json",
            "config",
        ] {
            assert!(!ex.contains(untyped), "{untyped} was not typed: {ex:?}");
        }
        let (_, ex) = parse_with_explicit(["audiohax", "play", "--gain", "1.0"]).expect("play");
        assert!(ex.contains("gain") && !ex.contains("instruments"));
        let (_, ex) = parse_with_explicit(["audiohax", "--json", "render"]).expect("global");
        assert!(ex.contains("json"));
        assert_eq!(ExplicitArgs::none(), ExplicitArgs::default());
    }

    /// L-30 / LD-2 / T-9: `--preset` and `--mappings` are mutually exclusive (clap's refusal), on
    /// both `render` and `play`; each alone parses; an unknown preset is clap's invalid-value error
    /// listing the possible values.
    #[test]
    fn preset_and_mappings_flags() {
        for sub in ["render", "play"] {
            let e = parse_with_explicit([
                "audiohax",
                sub,
                "--preset",
                "default",
                "--mappings",
                "m.json",
            ])
            .expect_err("conflict");
            assert_eq!(e.kind(), clap::error::ErrorKind::ArgumentConflict, "{sub}");
            let (cli, ex) =
                parse_with_explicit(["audiohax", sub, "--preset", "default"]).expect("preset");
            assert!(ex.contains("preset"));
            let p = match cli.command {
                Command::Render(r) => r.presets,
                Command::Play(p) => p.presets,
                _ => panic!("{sub}"),
            };
            assert_eq!(p.preset.as_deref(), Some("default"));
            assert_eq!(p.mappings, None);
            let (cli, _) =
                parse_with_explicit(["audiohax", sub, "--mappings", "m.json"]).expect("mappings");
            let p = match cli.command {
                Command::Render(r) => r.presets,
                Command::Play(p) => p.presets,
                _ => panic!("{sub}"),
            };
            assert_eq!(p.mappings.as_deref(), Some(std::path::Path::new("m.json")));
            let e =
                parse_with_explicit(["audiohax", sub, "--preset", "nope"]).expect_err("unknown");
            assert_eq!(e.kind(), clap::error::ErrorKind::InvalidValue);
            assert!(e.to_string().contains("[possible values: default]"), "{e}");
        }
        // Not global: the other subcommands do not take them.
        assert!(parse_with_explicit(["audiohax", "analyze", "--preset", "default"]).is_err());
        assert!(parse_with_explicit(["audiohax", "tui", "--mappings", "m.json"]).is_err());
    }
}

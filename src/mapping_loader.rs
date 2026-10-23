use serde::Deserialize;
use std::collections::HashMap;
use std::fs;

/// S13 feature-normalization contract (Option-NORM-MAP, design-s13-diversity §0).
///
/// The engine seam carries RAW physical scalars (edge density ~0..0.05, texture
/// Laplacian variance ~0..2000, etc.). The music layer normalizes each into a
/// usable 0..1 "knob" by dividing by the calibrated `range_max` declared here and
/// clamping — so a real photo's edge density of 0.005..0.036 lands in a usable
/// 0.10..0.72 ACTIVITY band instead of clustering near zero. These divisors are
/// the empirical max-of-six measured set (a touch generous so a busier-than-sample
/// image still lands in range; the `clamp` protects the top). They live in JSON so
/// re-tuning the calibration never needs a recompile — do NOT bake them as `const`.
///
/// Derives `Clone` so `rebuild_mapping_table` (engine.rs) can deep-copy the table
/// with a single `.clone()` of this struct (design-s13 §5 coordination note).
#[derive(Debug, Deserialize, Clone)]
pub struct FeatureNormalization {
    /// `range_max` for `edge_density` → `edge_activity` (raw 0..~0.05 → 0..1).
    pub edge_density_max: f32,
    /// `range_max` for `texture_laplacian_var` → `texture` (raw 0..~2000 → 0..1).
    pub texture_laplacian_var_max: f32,
    /// `range_max` for `shape_complexity` → `complexity` (raw 0..~2.0 → 0..1).
    pub shape_complexity_max: f32,
    /// `range_max` for `hue_spread` → `colorfulness` (already ~0..1; identity).
    pub hue_spread_max: f32,
    /// `range_max` for `avg_brightness` → `brightness01` (raw 0..100 → 0..1).
    pub avg_brightness_max: f32,
    /// `range_max` for `avg_saturation` → `saturation01` (raw 0..100 → 0..1).
    pub avg_saturation_max: f32,
}

impl FeatureNormalization {
    /// Normalize a raw feature against `range_max` into a 0..1 knob.
    ///
    /// theory: a deterministic, calibrated `clamp(raw / range_max, 0, 1)`. The
    /// clamp guarantees a busier-than-calibration image cannot push a knob past 1
    /// (it saturates at full activity) and a degenerate negative cannot go below 0.
    /// A zero/negative `range_max` is treated as "no calibration" and returns 0 so
    /// a misconfigured map fails safe (no activity) rather than dividing by zero.
    pub fn normalize(raw: f32, range_max: f32) -> f32 {
        if range_max <= 0.0 {
            return 0.0;
        }
        (raw / range_max).clamp(0.0, 1.0)
    }
}

#[derive(Debug, Deserialize)]
pub struct DominantSubTrigger {
    pub edge_complexity_threshold: f32,
    pub substitutions: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ModalInterchangeTrigger {
    pub brightness_drop_threshold: f32,
    pub borrowed_chords: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct CadenceTrigger {
    pub stillness_threshold: f32,
    pub high_motion_cadence: String,
    pub low_motion_cadence: String,
}

#[derive(Debug, Deserialize)]
pub struct GlobalMapping {
    pub hue_to_mode: HashMap<String, String>,
    pub saturation_to_harmonic_complexity: HashMap<String, String>,
    pub brightness_to_tempo_bpm: HashMap<String, u32>,
    /// S13: calibrated raw-feature → 0..1 normalization divisors (design-s13 §0).
    pub feature_normalization: FeatureNormalization,
    pub dominant_substitution_trigger: DominantSubTrigger,
    pub modal_interchange_trigger: ModalInterchangeTrigger,
    pub cadence_trigger: CadenceTrigger,
    pub progression_families: HashMap<String, Vec<String>>,
}

/// S15 Slice 1: the loader's deserialize target for the optional `composition` block.
///
/// Shape mirrors `composition::PlanMappings` exactly; it deserializes here (the loader owns
/// JSON schema) and `composition.rs` adapts it via `From<CompositionMappings>` (the planner
/// owns the planner type). The structural enums/tables it nests
/// (`SelectTable`/`FormSpec`/…) are DEFINED in `composition.rs` (the musical/structural
/// authority) and re-used here so there is one definition, not two.
///
/// The block is OPTIONAL on `MappingTable` (`#[serde(default)]`), so the OLD `mappings.json`
/// (no `composition` key) still parses with the new loader — the slice-1 back-compat floor.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct CompositionMappings {
    /// → a `FormSpec.id` from `form_catalogue`.
    pub form: crate::composition::SelectTable,
    /// → a `Character` variant name; slice 1 pinned "ballad".
    pub character: crate::composition::SelectTable,
    /// → a `Meter` name; slice 1 pinned "four4".
    pub meter: crate::composition::SelectTable,
    /// → a key-scheme id; slice 1 pinned "home_only".
    pub key_scheme: crate::composition::SelectTable,
    /// → "absent" | "fragment" | "second_theme".
    pub theme_behaviour: crate::composition::SelectTable,
    /// S17 → an `OrchestrationProfile.id` from `texture_catalogue`. `#[serde(default)]` so an
    /// OLD `composition` block (no `texture` axis) still deserializes — the loader's
    /// back-compat floor; an absent axis yields an empty `SelectTable` (planner → identity).
    #[serde(default)]
    pub texture: crate::composition::SelectTable,
    /// The form vocabulary (the 6 slice-1 FormSpec rows).
    pub form_catalogue: Vec<crate::composition::FormSpec>,
    /// S17 — the orchestration-profile vocabulary. `#[serde(default)]` (back-compat floor):
    /// absent → empty catalogue → planner falls back to `OrchestrationProfile::identity()`.
    #[serde(default)]
    pub texture_catalogue: Vec<crate::composition::OrchestrationProfile>,
    /// S20 — the figuration vocabulary. `#[serde(default)]` back-compat floor: absent → empty
    /// catalogue → unresolved figuration handle falls back to the block bed. This is the struct
    /// that actually deserializes `assets/mappings.json`; the `From<CompositionMappings>` impl
    /// carries it onto `PlanMappings`.
    #[serde(default)]
    pub figuration_catalogue: Vec<crate::composition::FigurationSpec>,
    /// S34 — the bass-pattern vocabulary. `#[serde(default)]` back-compat floor: absent → empty
    /// catalogue → an unresolved `bass_pattern` handle falls back to the byte-stable sustained
    /// Bass arm. This is the struct that actually deserializes `assets/mappings.json`; the
    /// `From<CompositionMappings>` impl carries it onto `PlanMappings`.
    #[serde(default)]
    pub bass_pattern_catalogue: Vec<crate::composition::BassPatternSpec>,
    /// S22 — the affect weights + per-character tempo windows. `#[serde(default)]` back-compat
    /// floor: absent → `AffectMappings::default()` (legacy Ballad window). Carried onto
    /// `PlanMappings` by the `From<CompositionMappings>` impl in composition.rs.
    #[serde(default)]
    pub affect: crate::composition::AffectMappings,
    /// S23 — the prominence SelectTable. `#[serde(default)]` back-compat floor: absent →
    /// empty → planner falls to uniform (byte-stable). Carried onto `PlanMappings` by the
    /// `From<CompositionMappings>` impl in composition.rs.
    #[serde(default)]
    pub prominence: crate::composition::SelectTable,
    /// S23 — the prominence-profile vocabulary. `#[serde(default)]` empty Vec.
    #[serde(default)]
    pub prominence_catalogue: Vec<crate::composition::ProminenceProfile>,
    /// S24 — the key-scheme vocabulary. `#[serde(default)]` empty Vec (back-compat floor):
    /// absent → only "home_only" reachable → byte-stable. Carried onto `PlanMappings` by the
    /// `From<CompositionMappings>` impl in composition.rs.
    #[serde(default)]
    pub key_scheme_catalogue: Vec<crate::composition::KeyScheme>,
    /// S40 / Slice-2 — the optional per-image home block (Finding #1). `#[serde(default)]`
    /// back-compat floor: absent → `None` → the planner returns `home_root_midi = 60`
    /// byte-for-byte (today's behavior). When present, the dominant hue selects a chromatic
    /// pitch class (Music Theory single-writes the `hue_to_pc` cuts) which is seated into the
    /// Theory-owned safe register band `[lo,hi]`. Carried onto `PlanMappings` by the
    /// `From<CompositionMappings>` impl in composition.rs.
    #[serde(default)]
    pub home_root: Option<HomeRootMap>,
    /// S64 — the per-image phrase/cadence scheme selector (→ a `PhraseScheme` id). `#[serde(default)]`
    /// back-compat floor: absent → empty `SelectTable` → `""` → the LEGACY `PhraseScheme::Period4`
    /// (byte-stable). Carried onto `PlanMappings` by the `From<CompositionMappings>` impl in
    /// composition.rs.
    #[serde(default)]
    pub phrase_scheme: crate::composition::SelectTable,
    /// S74b/B5 — the per-image ENSEMBLE selector (→ an `EnsembleSpec` id from `ensemble_catalogue`).
    /// `#[serde(default)]` back-compat floor: absent → empty `SelectTable` → `""` → no ensemble →
    /// the adapter falls every voice to `legacy_program(i)=(i*7)%128` (byte-identical to pre-B5).
    /// Carried onto `PlanMappings` by the `From<CompositionMappings>` impl in composition.rs.
    #[serde(default)]
    pub instrumentation: crate::composition::SelectTable,
    /// S74b/B5 — the ensemble palette vocabulary (id → voice-indexed GM programs). `#[serde(default)]`
    /// back-compat floor: absent → empty catalogue → an unresolved ensemble id → per-voice floor.
    /// Carried onto `PlanMappings` by the `From<CompositionMappings>` impl in composition.rs.
    #[serde(default)]
    pub ensemble_catalogue: Vec<crate::composition::EnsembleSpec>,
    /// S98/B2-a — the PER-SEAT eligibility pools that make instrumentation selection per-voice
    /// (`GRAIN_instrumentation` 1 → `num_instruments`). `#[serde(default)]` back-compat floor:
    /// absent → empty pools → `select_programs_per_voice` falls back to the whole-image
    /// `ensemble_catalogue` path, byte-identically. NOT carried onto `PlanMappings`: this axis is
    /// consumed by `instrumentation::select_programs_per_voice` before the plan is seated, and the
    /// planner never reads it.
    #[serde(default)]
    pub instrumentation_pools: crate::instrumentation::InstrumentationPools,
}

/// S40 / Slice-2 (Finding #1) — the per-image home block (schema Option S1, range-map shape).
///
/// `dominant_hue` (degrees) selects a chromatic pitch class via the `hue_to_pc` range map, and
/// that pitch class is seated into the Theory-owned safe register band `[band.lo, band.hi]`.
/// This is the FROZEN deserialize contract shared with the parallel Music Theory JSON lane:
/// the `band` / `hue_to_pc` field names and shapes must not drift.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct HomeRootMap {
    /// The Theory-owned safe register band the chosen pitch class is seated into. The band must
    /// span exactly 12 semitones (`hi - lo == 11`) so every pitch class has exactly one
    /// representative in-band; the shipped default is `[57,68]` (A3 … G#4).
    pub band: HomeBand,
    /// The hue→pitch-class range map: `"lo-hi"` (hue degrees) → `"pc"` (the chromatic pitch
    /// class `0..=11` as a string, matching the existing `HashMap<String,String>` range-map
    /// convention used by `global.hue_to_mode`). Music Theory single-writes the cuts.
    pub hue_to_pc: HashMap<String, String>,
}

/// S40 / Slice-2 — the safe register band a seated home pitch class lands within. Theory-owned
/// (register-safety guard); `hi` must not exceed 68 without re-deriving headroom margins.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct HomeBand {
    /// Lowest MIDI note number of the safe band (inclusive).
    pub lo: u8,
    /// Highest MIDI note number of the safe band (inclusive). Must equal `lo + 11`.
    pub hi: u8,
}

#[derive(Debug, Deserialize)]
pub struct MappingTable {
    pub global: GlobalMapping,
    /// S15 Slice 1: the optional `composition` block (form catalogue + selection tables).
    /// Absent in the legacy `mappings.json` → `None` (back-compat floor). When present, the
    /// engine's compose path builds a `CompositionPlanner` from it.
    #[serde(default)]
    pub composition: Option<CompositionMappings>,
}

/// The legacy, UNCHECKED path loader: `fs::read_to_string` + [`parse_mappings`]. Kept for tests and
/// tools that load the repository file by its crate-root path. No front-end calls it any more: the
/// CLI, the TUI and the window load through [`resolve_cli_mappings`] / [`default_mappings`], which
/// bound the read and run [`validate_mappings`] (S124, lead ruling L-32).
pub fn load_mappings(path: &str) -> Result<MappingTable, Box<dyn std::error::Error>> {
    let data = fs::read_to_string(path)?;
    let mappings: MappingTable = parse_mappings(&data)?;
    Ok(mappings)
}

// ─────────────────────────────────────────────────────────────────────────────
// S124 — the embedded default, the bounded reader, and the ONE validation chokepoint
// (lead rulings L-28, L-29, L-32; the security review `s124-SH` MB-1…MB-8).
// ─────────────────────────────────────────────────────────────────────────────

/// `assets/mappings.json`, compiled in. The ONE embedded copy of the default mappings: the window's
/// `gui::EMBEDDED_MAPPINGS_JSON` re-exports it, the `default` preset names it, and the CLI and TUI
/// fall back to it when no `./assets/mappings.json` exists (L-29).
pub const DEFAULT_MAPPINGS_JSON: &str = include_str!("../assets/mappings.json");

/// The working-directory path the CLI and the TUI read when no `--mappings` is given and the file
/// exists (L-29, `s124-01` § 5.2 option B). Relative to the current directory, exactly as before.
pub const CWD_MAPPINGS_PATH: &str = "assets/mappings.json";

/// MB-1: the largest mappings file read, in bytes (1 MiB). The shipped file is 77,171 bytes.
pub const MAX_MAPPINGS_BYTES: u64 = 1_048_576;

/// MB-3′ (lead ruling L-41, amending L-32; `s124-SH` addendum A.1): the tempo window every BPM
/// value in a mappings table must lie in. The floor is 1 BPM, inclusive: the planner already plays
/// any tempo below 1 as 1 (`bpm.max(1.0)`), so 1 is the smallest floor at which the number written
/// is the number played. Duration is bounded by the 600 s piece cap (`render::MAX_PIECE_MS`), not by
/// this floor. The ONE place the floor is stated.
pub const MIN_BPM: f64 = 1.0;
/// MB-3: see [`MIN_BPM`].
pub const MAX_BPM: f64 = 400.0;
/// MB-6 / L-32: the most sections a form may have (the shipped maximum is 5).
pub const MAX_SECTIONS_PER_FORM: usize = 32;
/// MB-7: the highest MIDI program number (a MIDI data byte is 7 bits).
pub const MAX_MIDI_PROGRAM: u8 = 127;

/// Where a mappings table came from. Every [`MappingsError`] names it (MB-8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MappingsOrigin {
    /// [`DEFAULT_MAPPINGS_JSON`].
    Builtin,
    /// A file on disk (`--mappings FILE`, or `./assets/mappings.json`).
    File(std::path::PathBuf),
}

impl std::fmt::Display for MappingsOrigin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MappingsOrigin::Builtin => write!(f, "The built-in mappings"),
            // MB-8: a path is shown escaped, so a file name cannot put terminal escapes on the
            // user's console.
            MappingsOrigin::File(p) => write!(
                f,
                "The mappings file `{}`",
                p.display().to_string().escape_debug()
            ),
        }
    }
}

/// Why a mappings table was refused. Every message names the file, says what is wrong, and says
/// that nothing was rendered (MB-8).
///
/// `Debug` is implemented BY HAND as the `Display` text, so that when `main.rs` returns one of
/// these from `fn main() -> Result<(), Box<dyn Error>>` the user sees the sentence on stderr, not a
/// Rust struct dump (MB-8: no `Os { … }` / `Kind(…)` output).
#[derive(thiserror::Error)]
pub enum MappingsError {
    /// The path names a directory, a device, a pipe or anything else that is not a regular file.
    #[error("{origin} is not a regular file (a folder, device or pipe cannot be a mappings file). Nothing was rendered.")]
    NotARegularFile {
        /// The file.
        origin: MappingsOrigin,
    },
    /// The file is larger than [`MAX_MAPPINGS_BYTES`].
    #[error("{origin} is {len} bytes; the limit is 1 MiB ({max} bytes). Nothing was rendered.")]
    TooLarge {
        /// The file.
        origin: MappingsOrigin,
        /// Its size (at least `max + 1`).
        len: u64,
        /// The limit.
        max: u64,
    },
    /// The file is not UTF-8 text.
    #[error("{origin} is not UTF-8 text. Nothing was rendered.")]
    NotUtf8 {
        /// The file.
        origin: MappingsOrigin,
    },
    /// The file could not be opened or read.
    #[error("{origin} could not be read: {detail}. Nothing was rendered.")]
    Io {
        /// The file.
        origin: MappingsOrigin,
        /// The operating system's reason (its `Display` text, never `Debug`).
        detail: String,
    },
    /// The text is not a mappings document (JSON syntax, a missing field, a wrong type). serde's
    /// message carries the line and column.
    ///
    /// MB-8: serde's message can quote the file's own text (an unknown enum variant is echoed
    /// verbatim), so it is shown through `escape_controls`: a file cannot put terminal escape
    /// sequences on the user's console. The field is deliberately NOT named `source` (thiserror
    /// would make it `Error::source()`, and a chain printer would echo the raw text after all).
    #[error(
        "{origin} is not a valid mappings file: {}. Nothing was rendered.",
        escape_controls(&.error.to_string())
    )]
    Parse {
        /// The file.
        origin: MappingsOrigin,
        /// serde's error (with line and column). Never printed unescaped.
        error: serde_json::Error,
    },
    /// The document parsed but a value is outside its bound (MB-3, MB-4, MB-6, MB-7).
    #[error("{origin}: `{field}` {problem}. Nothing was rendered.")]
    Invalid {
        /// The file.
        origin: MappingsOrigin,
        /// The JSON path of the offending value, e.g. `composition.affect.energy_tempo.bpm_at_zero`.
        field: String,
        /// What is wrong, with the value and the bound.
        problem: String,
    },
}

/// MB-8: `text` with every character that can drive or reorder a terminal written as its Rust
/// escape: control characters (C0, DEL, C1 — so ESC, BEL and the 8-bit CSI: `\u{1b}`, `\u{7}`,
/// `\u{9b}`, `\n`) and the bidi and format characters (overrides and isolates U+202A–202E and
/// U+2066–2069, zero-width and direction marks, U+061C, U+FEFF, U+2028/2029: `\u{202e}` …).
/// The set is the one `str::escape_debug` escapes (as the file-name and field escaping here
/// do, the security review's SH3-2), less two things: quotes and backslashes are kept as
/// written, and so is a combining mark (`char::escape_debug` would escape the accent in a
/// decomposed `é` or the virama inside a Hindi word). An ordinary message reads unchanged.
/// Used on text a mappings file can put on a terminal: serde's error message, and a mode name
/// a front end prints (`play`'s "Engine mode" line, the TUI's mode).
pub fn escape_controls(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if c.is_control() || !kept_as_written(c) {
            out.extend(c.escape_debug());
        } else {
            out.push(c);
        }
    }
    out
}

/// Whether [`escape_controls`] keeps `c` as written: a quote or backslash, or a character that
/// `str::escape_debug` leaves alone when it is not the first in its string (`str::escape_debug`
/// escapes a combining mark only at the start; it is probed second so it never is).
fn kept_as_written(c: char) -> bool {
    if matches!(c, '"' | '\'' | '\\') {
        return true;
    }
    let mut probe = String::with_capacity(5);
    probe.push(' ');
    probe.push(c);
    probe.escape_debug().eq([' ', c])
}

impl std::fmt::Debug for MappingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

/// MB-1: read a mappings file with bounds. The PATH's metadata is checked first (regular file,
/// size ≤ [`MAX_MAPPINGS_BYTES`]) so a FIFO, a device or a directory is refused before `open()` can
/// block on it; then the OPENED handle is checked again and at most `max + 1` bytes are read; then
/// the bytes must be UTF-8. Symlinks are followed (the user named the path); the TARGET must be a
/// regular file of bounded size.
///
/// Known residual (no `libc` dependency, so no `O_NONBLOCK`): a regular file swapped for a FIFO
/// BETWEEN the path check and `open()` still blocks in `open()`. That needs write access to the
/// directory during the call (`s124-SH` § 4, MB-16).
pub fn read_mappings_file(path: &std::path::Path) -> Result<String, MappingsError> {
    use std::io::Read;
    let origin = || MappingsOrigin::File(path.to_path_buf());
    let io = |e: std::io::Error| MappingsError::Io {
        origin: origin(),
        detail: e.to_string(),
    };
    let check = |meta: &fs::Metadata| -> Result<(), MappingsError> {
        if !meta.is_file() {
            return Err(MappingsError::NotARegularFile { origin: origin() });
        }
        if meta.len() > MAX_MAPPINGS_BYTES {
            return Err(MappingsError::TooLarge {
                origin: origin(),
                len: meta.len(),
                max: MAX_MAPPINGS_BYTES,
            });
        }
        Ok(())
    };
    check(&fs::metadata(path).map_err(io)?)?;
    let file = fs::File::open(path).map_err(io)?;
    check(&file.metadata().map_err(io)?)?;
    let mut bytes = Vec::new();
    file.take(MAX_MAPPINGS_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(io)?;
    if bytes.len() as u64 > MAX_MAPPINGS_BYTES {
        return Err(MappingsError::TooLarge {
            origin: origin(),
            len: bytes.len() as u64,
            max: MAX_MAPPINGS_BYTES,
        });
    }
    String::from_utf8(bytes).map_err(|_| MappingsError::NotUtf8 { origin: origin() })
}

/// MB-2: THE chokepoint. Parse `json` with [`parse_mappings`] (so the table is exactly the one the
/// unchecked loader builds), then run [`validate_mappings`]. Every front-end's mappings — the
/// embedded default, `./assets/mappings.json`, `--mappings FILE`, a preset's — come through here.
pub fn load_checked(json: &str, origin: MappingsOrigin) -> Result<MappingTable, MappingsError> {
    let table = parse_mappings(json).map_err(|error| MappingsError::Parse {
        origin: origin.clone(),
        error,
    })?;
    // The typed parse accepted the text, so it is valid JSON and this second parse cannot fail on
    // syntax; it exists only to see every number as written (MB-4), including the ones the typed
    // schema ignores.
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|error| MappingsError::Parse {
            origin: origin.clone(),
            error,
        })?;
    validate_mappings(&value, &table).map_err(|(field, problem)| MappingsError::Invalid {
        origin,
        field,
        problem,
    })?;
    Ok(table)
}

/// [`read_mappings_file`] then [`load_checked`].
pub fn load_mappings_file(path: &std::path::Path) -> Result<MappingTable, MappingsError> {
    let text = read_mappings_file(path)?;
    load_checked(&text, MappingsOrigin::File(path.to_path_buf()))
}

/// The embedded default ([`DEFAULT_MAPPINGS_JSON`]) through [`load_checked`]. A failure is a build
/// defect, which the unit test `embedded_default_passes_validation` turns into a red build.
pub fn default_mappings() -> Result<MappingTable, MappingsError> {
    load_checked(DEFAULT_MAPPINGS_JSON, MappingsOrigin::Builtin)
}

/// The CLI's and the TUI's mappings when no preset is chosen (lead ruling L-29):
/// `Some(file)` (`--mappings FILE`) ⇒ that file; else `./assets/mappings.json` if it exists (the
/// pre-S124 behaviour, now bounded and validated); else the embedded default. "Exists" means
/// ANYTHING is at that path — read with `symlink_metadata`, which does not follow a link — so only
/// an empty path falls back. An unreadable or non-regular file there, or a symlink whose target is
/// missing, is reported rather than silently replaced by the built-in table (lead ruling L-42:
/// L-29's "if present" read strictly).
pub fn resolve_cli_mappings(file: Option<&std::path::Path>) -> Result<MappingTable, MappingsError> {
    resolve_cli_mappings_with_origin(file).map(|(table, _)| table)
}

/// [`resolve_cli_mappings`], also saying which source was used (so a later refusal, e.g. the
/// piece-length cap, can name the file).
pub fn resolve_cli_mappings_with_origin(
    file: Option<&std::path::Path>,
) -> Result<(MappingTable, MappingsOrigin), MappingsError> {
    let from_file = |path: &std::path::Path| {
        load_mappings_file(path).map(|t| (t, MappingsOrigin::File(path.to_path_buf())))
    };
    if let Some(path) = file {
        return from_file(path);
    }
    resolve_present_or_builtin(std::path::Path::new(CWD_MAPPINGS_PATH))
}

/// The no-`--mappings` arm of [`resolve_cli_mappings_with_origin`], over an explicit path (so it
/// can be tested without changing the process's working directory): whatever is AT `path` —
/// including a dangling symlink — goes through [`load_mappings_file`] and is refused loudly if it
/// is not a good mappings file; only `NotFound` on the path ITSELF (not on a link's target) falls
/// back to the embedded default.
fn resolve_present_or_builtin(
    path: &std::path::Path,
) -> Result<(MappingTable, MappingsOrigin), MappingsError> {
    match fs::symlink_metadata(path) {
        Ok(_) => load_mappings_file(path).map(|t| (t, MappingsOrigin::File(path.to_path_buf()))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            default_mappings().map(|t| (t, MappingsOrigin::Builtin))
        }
        Err(e) => Err(MappingsError::Io {
            origin: MappingsOrigin::File(path.to_path_buf()),
            detail: e.to_string(),
        }),
    }
}

/// The value bounds every mappings table must satisfy before anything is composed (lead ruling
/// L-32; `s124-SH` MB-3, MB-4, MB-6, MB-7). `value` is the document as JSON (to see every number
/// as written, MB-4) and `table` the typed parse of the SAME text. On failure returns
/// `(json_path, problem)`.
///
/// * MB-4: every number, anywhere in the document, is finite as an `f32`. serde turns a JSON
///   number above `f32::MAX` (e.g. `1e39`) into `inf` without an error, which is how NaN reached
///   the tempo grid.
/// * MB-3: every BPM value is in [`MIN_BPM`]`..=`[`MAX_BPM`] — `global.brightness_to_tempo_bpm.*`,
///   `composition.affect.energy_tempo.bpm_at_zero` / `bpm_at_one`, and each
///   `composition.affect.character_tempo.<name>.bpm_min` / `bpm_max` — and `bpm_min ≤ bpm_max`
///   (an inverted window panics `f32::clamp`).
/// * MB-6: every `composition.form_catalogue[i].sections` has at most
///   [`MAX_SECTIONS_PER_FORM`] entries (the measured multiplier behind the abort and the
///   superlinear compose).
/// * MB-7: every MIDI program (`composition.ensemble_catalogue[i].programs[j]`,
///   `composition.instrumentation_pools.roles[i].entries[j].program`) is at most 127.
///
/// Unknown keys stay ALLOWED (L-32, `s124-SH` HL-9: the shipped file carries keys no code reads).
/// Map keys are checked in sorted order, so the first violation reported is deterministic.
pub fn validate_mappings(
    value: &serde_json::Value,
    table: &MappingTable,
) -> Result<(), (String, String)> {
    check_numbers_finite(value, &mut String::new())?;

    let bpm = |field: String, v: f64| -> Result<(), (String, String)> {
        if (MIN_BPM..=MAX_BPM).contains(&v) {
            Ok(())
        } else {
            Err((
                field,
                format!(
                    "is {} BPM; a tempo must be between {MIN_BPM} and {MAX_BPM} BPM",
                    show_number(v)
                ),
            ))
        }
    };

    let mut b2t: Vec<(&String, &u32)> = table.global.brightness_to_tempo_bpm.iter().collect();
    b2t.sort();
    for (k, v) in b2t {
        bpm(
            format!("global.brightness_to_tempo_bpm.{}", k.escape_debug()),
            f64::from(*v),
        )?;
    }

    let Some(comp) = table.composition.as_ref() else {
        return Ok(());
    };
    if let Some(et) = comp.affect.energy_tempo.as_ref() {
        bpm(
            "composition.affect.energy_tempo.bpm_at_zero".into(),
            f64::from(et.bpm_at_zero),
        )?;
        bpm(
            "composition.affect.energy_tempo.bpm_at_one".into(),
            f64::from(et.bpm_at_one),
        )?;
    }
    let mut ct: Vec<_> = comp.affect.character_tempo.iter().collect();
    ct.sort_by(|a, b| a.0.cmp(b.0));
    for (name, w) in ct {
        let base = format!("composition.affect.character_tempo.{}", name.escape_debug());
        bpm(format!("{base}.bpm_min"), f64::from(w.bpm_min))?;
        bpm(format!("{base}.bpm_max"), f64::from(w.bpm_max))?;
        if w.bpm_min > w.bpm_max {
            return Err((
                base,
                format!(
                    "has bpm_min ({}) greater than bpm_max ({})",
                    w.bpm_min, w.bpm_max
                ),
            ));
        }
    }

    for (i, form) in comp.form_catalogue.iter().enumerate() {
        if form.sections.len() > MAX_SECTIONS_PER_FORM {
            return Err((
                format!("composition.form_catalogue[{i}].sections"),
                format!(
                    "(form `{}`) has {} sections; the limit is {MAX_SECTIONS_PER_FORM}",
                    form.id.escape_debug(),
                    form.sections.len()
                ),
            ));
        }
    }

    let program = |field: String, p: u8| -> Result<(), (String, String)> {
        if p <= MAX_MIDI_PROGRAM {
            Ok(())
        } else {
            Err((
                field,
                format!("is {p}, which is not a MIDI program (0-{MAX_MIDI_PROGRAM})"),
            ))
        }
    };
    for (i, ens) in comp.ensemble_catalogue.iter().enumerate() {
        for (j, p) in ens.programs.iter().enumerate() {
            program(
                format!("composition.ensemble_catalogue[{i}].programs[{j}]"),
                *p,
            )?;
        }
    }
    for (i, role) in comp.instrumentation_pools.roles.iter().enumerate() {
        for (j, e) in role.entries.iter().enumerate() {
            program(
                format!("composition.instrumentation_pools.roles[{i}].entries[{j}].program"),
                e.program,
            )?;
        }
    }
    Ok(())
}

/// A number as a person reads it: plain for ordinary magnitudes, scientific (`1e39`) for huge or
/// tiny ones, so an error never prints forty digits.
fn show_number(x: f64) -> String {
    let a = x.abs();
    if a != 0.0 && !(1e-3..1e6).contains(&a) {
        format!("{x:e}")
    } else {
        format!("{x}")
    }
}

/// MB-4: walk every number in `v`; refuse one that is not finite once read as an `f32`. `path` is
/// the JSON path so far (`a.b[3].c`).
fn check_numbers_finite(v: &serde_json::Value, path: &mut String) -> Result<(), (String, String)> {
    use serde_json::Value;
    match v {
        Value::Number(n) => {
            if let Some(x) = n.as_f64() {
                if !(x as f32).is_finite() {
                    return Err((
                        path.clone(),
                        format!(
                            "is {}, which is too large to be a number here (not finite)",
                            show_number(x)
                        ),
                    ));
                }
            }
            Ok(())
        }
        Value::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                let len = path.len();
                path.push_str(&format!("[{i}]"));
                check_numbers_finite(item, path)?;
                path.truncate(len);
            }
            Ok(())
        }
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            for k in keys {
                let len = path.len();
                if !path.is_empty() {
                    path.push('.');
                }
                path.push_str(&k.escape_debug().to_string());
                check_numbers_finite(&map[k], path)?;
                path.truncate(len);
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

/// Parse a `mappings.json` document already in memory (S122 Phase 0: the GUI / embedded-
/// mappings entry point, which has no path to read). [`load_mappings`] is exactly
/// `fs::read_to_string` + this, so both entry points parse identically.
pub fn parse_mappings(json: &str) -> Result<MappingTable, serde_json::Error> {
    serde_json::from_str(json)
}

/// Deep-copy a [`MappingTable`] by hand-rebuilding it from its public fields (all public,
/// plain `HashMap`/`Vec`/scalar data + the optional `composition` block, which derives
/// `Clone`).
///
/// NOTE(s9/s15): `ChordEngine::new` consumes a `MappingTable` BY VALUE, but the engine and
/// the `CompositionPlanner` each must keep their own copy across re-derivations — and
/// `MappingTable` derives only `Deserialize`, not `Clone`. Rather than add `#[derive(Clone)]`
/// (the nested triggers do not all derive it), this reconstructs a fresh table from the
/// public fields (lossless — all plain data). Moved here from `engine.rs` in S15 so
/// `composition.rs` can also build a transient `ChordEngine` from the shared table.
pub fn rebuild_mapping_table(t: &MappingTable) -> MappingTable {
    MappingTable {
        global: GlobalMapping {
            hue_to_mode: t.global.hue_to_mode.clone(),
            saturation_to_harmonic_complexity: t.global.saturation_to_harmonic_complexity.clone(),
            brightness_to_tempo_bpm: t.global.brightness_to_tempo_bpm.clone(),
            feature_normalization: t.global.feature_normalization.clone(),
            dominant_substitution_trigger: DominantSubTrigger {
                edge_complexity_threshold: t
                    .global
                    .dominant_substitution_trigger
                    .edge_complexity_threshold,
                substitutions: t.global.dominant_substitution_trigger.substitutions.clone(),
            },
            modal_interchange_trigger: ModalInterchangeTrigger {
                brightness_drop_threshold: t
                    .global
                    .modal_interchange_trigger
                    .brightness_drop_threshold,
                borrowed_chords: t.global.modal_interchange_trigger.borrowed_chords.clone(),
            },
            cadence_trigger: CadenceTrigger {
                stillness_threshold: t.global.cadence_trigger.stillness_threshold,
                high_motion_cadence: t.global.cadence_trigger.high_motion_cadence.clone(),
                low_motion_cadence: t.global.cadence_trigger.low_motion_cadence.clone(),
            },
            progression_families: t.global.progression_families.clone(),
        },
        // S15: the optional composition block derives Clone, so copy it directly.
        composition: t.composition.clone(),
    }
}

/// Helper: given hue (0..360) find mapping entry key like "91-150" and return value
pub fn lookup_range_map(map: &HashMap<String, String>, value: f32) -> Option<String> {
    for (key, val) in map.iter() {
        if let Some((a, b)) = parse_range(key) {
            if value >= a as f32 && value <= b as f32 {
                return Some(val.clone());
            }
        }
    }
    None
}

fn parse_range(s: &str) -> Option<(i32, i32)> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() == 2 {
        if let (Ok(a), Ok(b)) = (parts[0].trim().parse(), parts[1].trim().parse()) {
            return Some((a, b));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composition::BassPatternKind;

    /// S34 — the loader parses `assets/mappings.json`'s NEW `bass_pattern_catalogue` through the
    /// `CompositionMappings` mirror: every row deserializes, the walking/pedal kinds resolve, and
    /// the `density`/`pedal_degree` defaults + overrides land. (mapping_loader-lane parse witness.)
    #[test]
    fn s34_bass_pattern_catalogue_parses_via_loader() {
        let mt = load_mappings("assets/mappings.json").expect("mappings load");
        let cm = mt.composition.expect("composition block present");
        let find = |id: &str| -> &crate::composition::BassPatternSpec {
            cm.bass_pattern_catalogue
                .iter()
                .find(|b| b.id == id)
                .unwrap_or_else(|| panic!("bass_pattern row `{id}` present"))
        };
        assert_eq!(find("sustained").kind, BassPatternKind::Sustained);
        assert_eq!(find("walking").kind, BassPatternKind::Walking);
        // S102/FC-7: amended 2 -> 1 alongside the catalogue — a config-mirror round-trip
        // assertion, not a musical pin. See the twin in `composition.rs`. The musical evidence
        // for the value lives in
        // `chord_engine::tests::s102_walking_bass_is_not_the_busiest_voice_at_shipped_density`,
        // which reads the catalogue and asserts the FC-7 texture property the value exists to
        // secure (it fails at density 2 and 4).
        assert_eq!(find("walking").density, 1);
        assert_eq!(find("walking_q").density, 4);
        assert_eq!(find("pedal").kind, BassPatternKind::Pedal);
        assert_eq!(find("pedal").pedal_degree, 1);
        assert_eq!(find("pedal_dom").pedal_degree, 5);
    }

    /// S34 — an UNKNOWN bass-pattern kind is REJECTED by serde (the closed-enum safety): a
    /// `composition` block whose catalogue carries a `"kind": "boogie"` row fails to deserialize.
    #[test]
    fn s34_unknown_bass_pattern_kind_rejected() {
        let json = r#"{
            "global": { "hue_to_mode": {}, "saturation_to_harmonic_complexity": {},
              "brightness_to_tempo_bpm": {},
              "feature_normalization": { "edge_density_max": 0.05, "texture_laplacian_var_max": 2000.0,
                "shape_complexity_max": 2.0, "hue_spread_max": 1.0, "avg_brightness_max": 100.0,
                "avg_saturation_max": 100.0 },
              "dominant_substitution_trigger": { "edge_complexity_threshold": 0.5, "substitutions": [] },
              "modal_interchange_trigger": { "brightness_drop_threshold": 0.5, "borrowed_chords": [] },
              "cadence_trigger": { "stillness_threshold": 0.5, "high_motion_cadence": "x", "low_motion_cadence": "y" },
              "progression_families": {} },
            "composition": {
              "form": { "default": "f", "rules": [] }, "character": { "default": "ballad", "rules": [] },
              "meter": { "default": "four4", "rules": [] }, "key_scheme": { "default": "home_only", "rules": [] },
              "theme_behaviour": { "default": "absent", "rules": [] },
              "form_catalogue": [],
              "bass_pattern_catalogue": [ { "id": "bad", "kind": "boogie" } ]
            }
        }"#;
        let res: Result<MappingTable, _> = serde_json::from_str(json);
        assert!(
            res.is_err(),
            "an unknown bass-pattern kind must be rejected by serde at load time"
        );
    }

    /// S122 — `parse_mappings` (in-memory) and `load_mappings` (path) parse the shipped table
    /// identically, and `parse_mappings` surfaces a malformed document as an error.
    #[test]
    fn s122_parse_mappings_matches_load_mappings() {
        let path = "assets/mappings.json";
        let text = std::fs::read_to_string(path).expect("read shipped mappings");
        let parsed = parse_mappings(&text).expect("parse shipped mappings");
        let loaded = load_mappings(path).expect("load shipped mappings");
        assert_eq!(parsed.composition, loaded.composition);
        assert_eq!(parsed.global.hue_to_mode, loaded.global.hue_to_mode);
        assert_eq!(
            parsed.global.progression_families,
            loaded.global.progression_families
        );
        assert!(parse_mappings("{ not json").is_err());
    }

    // ── S124: embedded default, bounded reader, validation (L-32; s124-SH § 5.3) ────────────

    /// Scratch directory beside the test executable (inside the cargo target dir; never `/tmp`,
    /// never inside the repository).
    fn scratch(name: &str) -> std::path::PathBuf {
        let base = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .unwrap_or_else(std::env::temp_dir);
        let dir = base
            .join("audiohax-mappings-test-scratch")
            .join(format!("{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("scratch dir");
        dir
    }

    /// The shipped document as a JSON value, for building hostile variants.
    fn shipped_value() -> serde_json::Value {
        serde_json::from_str(DEFAULT_MAPPINGS_JSON).expect("shipped json")
    }

    /// Validate a mutated copy of the shipped document; return the user-facing error text.
    fn refusal(mutate: impl FnOnce(&mut serde_json::Value)) -> String {
        let mut v = shipped_value();
        mutate(&mut v);
        let text = serde_json::to_string(&v).expect("serialize");
        match load_checked(&text, MappingsOrigin::File("hostile.json".into())) {
            Ok(_) => panic!("the hostile table was ACCEPTED"),
            Err(e) => e.to_string(),
        }
    }

    /// H10 / MB-2: the embedded default passes validation, and it is the repository file byte
    /// for byte (moved here from the window's test, so the LIB pins the bytes the CLI falls back to).
    #[test]
    fn embedded_default_passes_validation() {
        let on_disk =
            std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mappings.json"))
                .expect("test reads the repo copy");
        assert_eq!(DEFAULT_MAPPINGS_JSON, on_disk);
        let checked = default_mappings().expect("the shipped mappings pass validation");
        // Same parser, same table: the checked load builds exactly the unchecked one.
        let unchecked = parse_mappings(DEFAULT_MAPPINGS_JSON).expect("parse");
        assert_eq!(checked.composition, unchecked.composition);
        assert_eq!(checked.global.hue_to_mode, unchecked.global.hue_to_mode);
        assert_eq!(
            checked.global.brightness_to_tempo_bpm,
            unchecked.global.brightness_to_tempo_bpm
        );
    }

    /// H1 / MB-3: an inverted tempo window is refused for EVERY character (today it panics
    /// `f32::clamp`), and the message names the path and both values.
    #[test]
    fn inverted_tempo_window_is_refused_for_every_character() {
        let v = shipped_value();
        let names: Vec<String> = v["composition"]["affect"]["character_tempo"]
            .as_object()
            .expect("character_tempo")
            .keys()
            .cloned()
            .collect();
        assert_eq!(names.len(), 10, "the shipped file has ten characters");
        for name in names {
            let msg = refusal(|v| {
                let w = &mut v["composition"]["affect"]["character_tempo"][&name];
                w["bpm_min"] = serde_json::json!(200.0);
                w["bpm_max"] = serde_json::json!(50.0);
            });
            assert!(
                msg.contains(&format!("composition.affect.character_tempo.{name}"))
                    && msg.contains("bpm_min (200)")
                    && msg.contains("bpm_max (50)")
                    && msg.contains("hostile.json")
                    && msg.contains("Nothing was rendered"),
                "{msg}"
            );
        }
    }

    /// H2 / MB-4: a number too large for an `f32` (serde would make it `inf`) is refused by path.
    #[test]
    fn a_number_that_would_become_infinite_is_refused() {
        let msg = refusal(|v| {
            v["composition"]["affect"]["energy_tempo"]["bpm_at_zero"] = serde_json::json!(1e39)
        });
        assert!(
            msg.contains("composition.affect.energy_tempo.bpm_at_zero")
                && msg.contains("is 1e39")
                && msg.contains("finite"),
            "{msg}"
        );
        // Also inside a key no code reads (the walk sees the whole document).
        let msg = refusal(|v| v["composition"]["_note_hostile"] = serde_json::json!(-3.5e38));
        assert!(msg.contains("composition._note_hostile"), "{msg}");
    }

    /// H5 / MB-3: tempi outside 20–400 BPM, on the energy curve and on the legacy brightness map.
    #[test]
    fn out_of_range_tempi_are_refused() {
        for bad in [0.001, 0.5, MIN_BPM - 0.1, MAX_BPM + 0.5, 1e30] {
            let msg = refusal(|v| {
                v["composition"]["affect"]["energy_tempo"]["bpm_at_one"] = serde_json::json!(bad)
            });
            assert!(
                msg.contains("energy_tempo.bpm_at_one")
                    && msg.contains(&format!("between {MIN_BPM} and {MAX_BPM}")),
                "{bad}: {msg}"
            );
        }
        for bad in [0u64, 4_294_967_295] {
            let msg = refusal(|v| {
                v["global"]["brightness_to_tempo_bpm"]["0-30"] = serde_json::json!(bad)
            });
            assert!(
                msg.contains("global.brightness_to_tempo_bpm.0-30"),
                "{bad}: {msg}"
            );
        }
        // The bounds themselves are accepted.
        for ok in [MIN_BPM, MAX_BPM] {
            let mut v = shipped_value();
            v["composition"]["affect"]["energy_tempo"]["bpm_at_one"] = serde_json::json!(ok);
            let text = serde_json::to_string(&v).expect("ser");
            assert!(load_checked(&text, MappingsOrigin::Builtin).is_ok(), "{ok}");
        }
    }

    /// H3 / MB-6: 33 sections and 10,000 sections are refused, naming the form; 32 is accepted.
    #[test]
    fn too_many_sections_are_refused() {
        for n in [33usize, 10_000] {
            let msg = refusal(|v| {
                let form = &mut v["composition"]["form_catalogue"][0];
                let one = form["sections"][0].clone();
                form["sections"] = serde_json::Value::Array(vec![one; n]);
            });
            assert!(
                msg.contains("composition.form_catalogue[0].sections")
                    && msg.contains(&format!("has {n} sections"))
                    && msg.contains("limit is 32"),
                "{msg}"
            );
        }
        let mut v = shipped_value();
        let one = v["composition"]["form_catalogue"][0]["sections"][0].clone();
        v["composition"]["form_catalogue"][0]["sections"] = serde_json::Value::Array(vec![one; 32]);
        let text = serde_json::to_string(&v).expect("ser");
        assert!(load_checked(&text, MappingsOrigin::Builtin).is_ok());
    }

    /// H6 / MB-7: MIDI programs above 127, in an ensemble and in a pool.
    #[test]
    fn programs_above_127_are_refused() {
        let msg = refusal(|v| {
            v["composition"]["ensemble_catalogue"][0]["programs"] =
                serde_json::json!([255, 200, 128, 240])
        });
        assert!(
            msg.contains("composition.ensemble_catalogue[0].programs[0]")
                && msg.contains("is 255")
                && msg.contains("not a MIDI program"),
            "{msg}"
        );
        let msg = refusal(|v| {
            v["composition"]["instrumentation_pools"]["roles"][0]["entries"][0]["program"] =
                serde_json::json!(255)
        });
        assert!(
            msg.contains("instrumentation_pools.roles[0].entries[0].program"),
            "{msg}"
        );
    }

    /// H9: parse errors keep serde's line and column and name the file.
    #[test]
    fn parse_errors_name_the_file_and_the_position() {
        for text in ["{ not json", "\u{feff}{}", "{} trailing", "NaN"] {
            let e = load_checked(text, MappingsOrigin::File("bad.json".into()))
                .expect_err("not a mappings document");
            let msg = e.to_string();
            assert!(
                msg.contains("`bad.json` is not a valid mappings file")
                    && msg.contains("line 1")
                    && msg.contains("Nothing was rendered"),
                "{msg}"
            );
        }
    }

    /// H8 / MB-1: directory, device, FIFO, oversize, non-UTF-8, missing, empty — each refused with
    /// its own sentence, promptly, and none with Rust `Debug` output (H14).
    #[test]
    fn bounded_reader_refuses_what_is_not_a_small_text_file() {
        let dir = scratch("bounded-reader");
        let no_debug = |m: &str| {
            assert!(!m.contains("Os {") && !m.contains("Kind("), "{m}");
        };
        // directory
        let m = read_mappings_file(&dir).expect_err("dir").to_string();
        assert!(m.contains("is not a regular file"), "{m}");
        no_debug(&m);
        // a device
        #[cfg(unix)]
        {
            let m = read_mappings_file(std::path::Path::new("/dev/zero"))
                .expect_err("device")
                .to_string();
            assert!(m.contains("is not a regular file"), "{m}");
        }
        // a FIFO: refused from the path's metadata, before open() could block on it
        #[cfg(unix)]
        {
            let fifo = dir.join("pipe.json");
            let _ = std::fs::remove_file(&fifo);
            let made = std::process::Command::new("mkfifo")
                .arg(&fifo)
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            if made {
                let t = std::time::Instant::now();
                let m = read_mappings_file(&fifo).expect_err("fifo").to_string();
                assert!(m.contains("is not a regular file"), "{m}");
                assert!(t.elapsed() < std::time::Duration::from_secs(2));
            }
        }
        // 1 MiB + 1 byte
        let big = dir.join("big.json");
        std::fs::write(&big, vec![b' '; (MAX_MAPPINGS_BYTES + 1) as usize]).expect("write");
        let m = read_mappings_file(&big).expect_err("big").to_string();
        assert!(
            m.contains("1048577 bytes") && m.contains("limit is 1 MiB"),
            "{m}"
        );
        // exactly 1 MiB is read (and then fails as JSON, not as size)
        let edge = dir.join("edge.json");
        std::fs::write(&edge, vec![b' '; MAX_MAPPINGS_BYTES as usize]).expect("write");
        assert!(read_mappings_file(&edge).is_ok());
        // non-UTF-8
        let bin = dir.join("bin.json");
        std::fs::write(&bin, [0xFFu8, 0xFE, 0x00, 0x7B]).expect("write");
        let m = read_mappings_file(&bin).expect_err("bin").to_string();
        assert!(m.contains("is not UTF-8 text"), "{m}");
        // missing
        let m = read_mappings_file(&dir.join("nope.json"))
            .expect_err("missing")
            .to_string();
        assert!(
            m.contains("could not be read") && m.contains("nope.json"),
            "{m}"
        );
        no_debug(&m);
        // empty: read fine, refused as a document
        let empty = dir.join("empty.json");
        std::fs::write(&empty, b"").expect("write");
        let m = load_mappings_file(&empty).expect_err("empty").to_string();
        assert!(m.contains("is not a valid mappings file"), "{m}");
        // the shipped file through the whole file path
        assert!(load_mappings_file(std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/mappings.json"
        )))
        .is_ok());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// SH2-1 / MB-8: `escape_controls` escapes ESC, BEL, the 8-bit CSI, DEL and a newline, and
    /// keeps quotes, backslashes, backticks and non-ASCII letters exactly as written.
    #[test]
    fn escape_controls_escapes_control_characters_and_keeps_plain_text() {
        assert_eq!(
            escape_controls("a\u{1b}[2Jb\u{7}c\u{9b}d\u{7f}e\nf"),
            "a\\u{1b}[2Jb\\u{7}c\\u{9b}d\\u{7f}e\\nf"
        );
        let plain = "unknown variant `q\"u\\o'te é`, expected `x`";
        assert_eq!(escape_controls(plain), plain);
    }

    /// SH3-2 / MB-8: the bidi and format characters `str::escape_debug` escapes are escaped too
    /// (overrides and isolates, zero-width and direction marks, the Arabic letter mark, the BOM,
    /// line and paragraph separators), so a file cannot reorder or hide text in a message.
    /// Ordinary text stays as written: non-ASCII letters, a combining accent, a Devanagari virama,
    /// an emoji with its presentation selector, quotes, backslashes and backticks.
    #[test]
    fn escape_controls_escapes_bidi_and_format_characters() {
        for c in [
            '\u{202a}', '\u{202b}', '\u{202c}', '\u{202d}', '\u{202e}', '\u{2066}', '\u{2067}',
            '\u{2068}', '\u{2069}', '\u{200b}', '\u{200e}', '\u{200f}', '\u{61c}', '\u{feff}',
            '\u{2028}', '\u{2029}',
        ] {
            let shown = escape_controls(&format!("a{c}b"));
            assert_eq!(
                shown,
                format!("a\\u{{{:x}}}b", c as u32),
                "U+{:04X} must be escaped",
                c as u32
            );
        }
        let plain = "Dorian é e\u{301} हिन्दी ✔\u{fe0f} \"q\" 'q' a\\b `k` Ünïcödé";
        assert_eq!(escape_controls(plain), plain);
    }

    /// H14 / MB-8: a file name carrying a terminal escape is shown escaped.
    #[test]
    fn a_file_name_cannot_put_escapes_on_the_console() {
        let e = MappingsError::NotUtf8 {
            origin: MappingsOrigin::File("evil\u{1b}[31m.json".into()),
        };
        let m = e.to_string();
        assert!(!m.contains('\u{1b}'), "{m:?}");
        assert!(m.contains("\\u{1b}[31m"), "{m}");
        // Debug is the sentence, not a struct dump.
        assert_eq!(format!("{e:?}"), m);
    }

    /// L-29: with no `--mappings`, a missing `./assets/mappings.json` falls back to the embedded
    /// table. (The cwd of a unit test is the crate root, where the file exists; the fallback is
    /// exercised through the NotFound arm by the same function with an explicit path in the
    /// CLI-level checks of the report. Here: an explicit file wins, and is validated.)
    #[test]
    fn an_explicit_file_is_loaded_through_the_chokepoint() {
        let dir = scratch("explicit");
        let p = dir.join("hostile.json");
        let mut v = shipped_value();
        v["composition"]["affect"]["character_tempo"]["march"]["bpm_min"] = serde_json::json!(200);
        std::fs::write(&p, serde_json::to_string(&v).expect("ser")).expect("write");
        let m = resolve_cli_mappings(Some(&p))
            .expect_err("validated")
            .to_string();
        assert!(m.contains("character_tempo.march"), "{m}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// L-42 / F-3: with no `--mappings`, whatever is AT the cwd path is loaded and refused loudly
    /// if bad — a dangling symlink and a directory included — and only an empty path falls back
    /// to the embedded default. CONTROL: a symlink to a good file loads that file.
    #[test]
    fn a_present_but_broken_cwd_file_is_reported_not_replaced() {
        let dir = scratch("present-or-builtin");
        let at = dir.join("mappings.json");
        let _ = std::fs::remove_file(&at);
        // nothing there ⇒ the built-in table
        let (_, origin) = resolve_present_or_builtin(&at).expect("absent ⇒ built-in");
        assert_eq!(origin, MappingsOrigin::Builtin);
        #[cfg(unix)]
        {
            // a dangling symlink ⇒ reported, naming the path
            std::os::unix::fs::symlink(dir.join("missing-target.json"), &at).expect("symlink");
            let m = resolve_present_or_builtin(&at)
                .expect_err("a dangling link is reported")
                .to_string();
            assert!(m.contains("could not be read"), "{m}");
            assert!(m.contains("mappings.json"), "{m}");
            assert!(!m.contains("Os {"), "{m}");
            // CONTROL: the same link to a good file loads it (from the file, not the built-in)
            let good = dir.join("good.json");
            std::fs::write(&good, DEFAULT_MAPPINGS_JSON).expect("write");
            std::fs::remove_file(&at).expect("unlink");
            std::os::unix::fs::symlink(&good, &at).expect("symlink");
            let (_, origin) = resolve_present_or_builtin(&at).expect("a good link loads");
            assert_eq!(origin, MappingsOrigin::File(at.clone()));
            std::fs::remove_file(&at).expect("unlink");
        }
        // a directory at the path ⇒ reported
        std::fs::create_dir_all(&at).expect("dir");
        let m = resolve_present_or_builtin(&at)
            .expect_err("a directory is reported")
            .to_string();
        assert!(m.contains("is not a regular file"), "{m}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

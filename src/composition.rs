//! src/composition.rs — S15 Slice 1 the COMPOSER layer (spec-s15-slice1-build §1–§3).
//!
//! Pure-Rust, `--no-default-features`-clean: NO image type, NO OpenCV, NO pixel math.
//! It reads perceptual scalars (an [`ImageUnderstanding`], the image-free mirror of the
//! analysis — same boundary discipline as `engine::GlobalFeatures`) and emits STRUCTURE:
//! a [`CompositionPlan`] of concrete [`Section`]s the time cursor walks ONCE, 0→`total_steps`,
//! with NO modulo loop. The per-section harmony is filled by the EXISTING `chord_engine`
//! craft (`pick_progression` → `generate_chords` → `plan_phrases`); the returning theme's
//! motif is resolved by `chord_engine::resolve_motif` at PLAN-BUILD time (the one place
//! contour → `MotifNote` happens — Music Theory owns that fn). This module never makes a
//! per-note musical decision; it picks form/theme structure and delegates the music craft.
//!
//! Slice-1 Section invariants are LOCKED (spec §1.2): every concrete `Section` carries
//! `key_offset_semitones == 0`, `ms_per_step == base_ms_per_step`, `mode == home_mode`,
//! `variation ∈ {Identity, Fragmented}`, `character == Ballad`, `meter == Four4`. The
//! planner never lets a non-zero / non-home value leak in; modulation / meter / the other
//! characters ship as schema (default-pinned) and are realized in later stages.

use crate::chord_engine::{self, ChordEngine, MotifArchetype, StepPlan};
use crate::mapping_loader::{rebuild_mapping_table, CompositionMappings, MappingTable};

/// Local mirror of the (private) `chord_engine::EDGE_ACTIVITY_RANGE_MAX` (== 0.05). The
/// planner stores `edge_activity` already-normalized (0..1) but `generate_chords` wants the
/// RAW edge density (~0..0.05), so it multiplies back through this. KEEP IN SYNC with
/// `chord_engine`/`feature_normalization.edge_density_max` (spec §1.1).
const EDGE_ACTIVITY_RANGE_MAX: f32 = 0.05;

// ─────────────────────────────────────────────────────────────────────────────
// §1.1 ImageUnderstanding — the planner's input (image-free mirror)
// ─────────────────────────────────────────────────────────────────────────────

/// Whole-image perceptual understanding — the COMPOSER'S input. Computed once per image,
/// whole-image, all plain values. Image-free (no `Mat`, no pixel type) — same discipline as
/// [`crate::engine::GlobalFeatures`]. Slice 1 only READS the subset the form/theme ladders
/// need; the rest are present (so later stages fill VALUES, not TYPES) and default to the
/// whole-image / sentinel value. The planner treats a default/sentinel field as "condition
/// not met" so a ladder rule reading a not-yet-extracted knob falls through to the default.
#[derive(Debug, Clone, PartialEq)]
pub struct ImageUnderstanding {
    // ── Energy (0..1; the dead S13 features re-exposed via pure_analysis) ──
    /// Visual activity, `clamp(global.edge_density / 0.05, 0, 1)`.
    pub edge_activity: f32,
    /// Texture, `clamp(global.texture_laplacian_var / 2000, 0, 1)`.
    pub texture: f32,
    /// Shape complexity, `clamp(global.shape_complexity / 2, 0, 1)`.
    pub complexity: f32,
    // ── Palette ──
    /// Dominant hue 0..360 — slice 1 `== global.avg_hue` (argmax upgrade is Stage 8).
    pub dominant_hue: f32,
    /// Mass of the dominant hue — slice 1 default `1.0`.
    pub dominant_hue_mass: f32,
    /// Secondary hue — slice 1 default `== dominant_hue`.
    pub secondary_hue: f32,
    /// Palette bimodality 0..1 — slice 1 default `0.0`.
    pub palette_bimodality: f32,
    /// Colorfulness `== global.hue_spread`.
    pub colorfulness: f32,
    /// Value key 0..1 toward dark — slice 1 `clamp(1 - avg_brightness/100, 0, 1)`.
    pub value_key: f32,
    /// Mirror of `global.avg_brightness`, 0..100.
    pub avg_brightness: f32,
    /// Mirror of `global.avg_saturation`, 0..100.
    pub avg_saturation: f32,
    // ── Composition balance ──
    /// Visual-mass centroid (x, y) — slice 1 default `(0.5, 0.5)`.
    pub mass_centroid: (f32, f32),
    /// Quadrant contrast 0..1 — slice 1 default `0.0`.
    pub quadrant_contrast: f32,
    /// Aspect ratio `== global.aspect_ratio` (w/h).
    pub aspect_ratio: f32,
    /// Vertical (upper-mass) emphasis 0..1 — slice 1 default `0.5`.
    pub vertical_emphasis: f32,
    // ── Subject / region-saliency (defaults = whole-image; saliency is Stage 9) ──
    /// Subject size — slice 1 default `1.0`.
    pub subject_size: f32,
    /// Subject hue — slice 1 default `== dominant_hue`.
    pub subject_hue: f32,
    /// Subject saturation — slice 1 default `== avg_saturation`.
    pub subject_saturation: f32,
    /// Foreground/background contrast — slice 1 default `0.0`.
    pub fg_bg_contrast: f32,
    /// Energy (edge activity) in the salient subject region, 0..1. NEW S18.
    pub subject_energy: f32,
    /// Energy in the foreground band (the non-subject central/edge-mid cells), 0..1. NEW S18.
    pub foreground_energy: f32,
    /// Energy in the background band (the corner cells minus the subject), 0..1. NEW S18.
    pub background_energy: f32,
    /// NEW S22 — the planner-computed arousal composite (0..1). NOT extracted from pixels and
    /// NOT deserialized; `pure_analysis::understand_image_pure` and `neutral()` leave it at the
    /// `-1.0` sentinel ("not yet computed"), and the planner overwrites it via `affect_composite`
    /// before the character/tempo ladders run. `Knob::Arousal` reads this. Keeping it off the
    /// pixel producer holds the module boundary (`pure_analysis.rs` writes the sentinel, never
    /// a real value). The `-1.0` sentinel is below any real 0..1 value, so a `Ge`/`Gt` ladder
    /// rule reading an unfilled composite never spuriously fires.
    pub affect_arousal: f32,
    /// NEW S22 — the planner-computed valence composite (0..1). Same sentinel discipline.
    pub affect_valence: f32,
}

impl ImageUnderstanding {
    /// A neutral, whole-image understanding (all energy zero, all balance at its
    /// slice-1 default). Used by tests and as a no-op planner input.
    pub fn neutral() -> Self {
        ImageUnderstanding {
            edge_activity: 0.0,
            texture: 0.0,
            complexity: 0.0,
            dominant_hue: 0.0,
            dominant_hue_mass: 1.0,
            secondary_hue: 0.0,
            palette_bimodality: 0.0,
            colorfulness: 0.0,
            value_key: 0.0,
            avg_brightness: 50.0,
            avg_saturation: 50.0,
            mass_centroid: (0.5, 0.5),
            quadrant_contrast: 0.0,
            aspect_ratio: 1.0,
            vertical_emphasis: 0.5,
            subject_size: 1.0,
            subject_hue: 0.0,
            subject_saturation: 50.0,
            fg_bg_contrast: 0.0,
            subject_energy: 0.0,
            foreground_energy: 0.0,
            background_energy: 0.0,
            affect_arousal: -1.0,
            affect_valence: -1.0,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// §S22 Affect — valence/arousal composite + per-character tempo windows
// ─────────────────────────────────────────────────────────────────────────────

/// The affect composite — the image's valence/arousal coordinates, each 0..1 (0.5 neutral),
/// derived purely from the perceptual scalars already on `ImageUnderstanding`. NO new image
/// extraction. Computed ONCE per plan in `composition.rs` (the planner's module). Pure: no
/// pixels, no RNG, no clock.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Affect {
    /// Arousal / energy, 0 (calm) .. 1 (energetic). Saturation-led.
    pub arousal: f32,
    /// Valence / mood, 0 (dark/tense) .. 1 (bright/pleasant). Brightness-led.
    pub valence: f32,
}

/// One character's tempo window (BPM), loaded from `affect.character_tempo.<character>`.
#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize)]
pub struct CharacterTempo {
    pub bpm_min: f32,
    pub bpm_max: f32,
}

/// The `affect` mapping block: composite weights + per-character tempo windows. All fields
/// `#[serde(default)]` so a partial/absent block still parses. NEW S22.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct AffectMappings {
    /// Weight per ImageUnderstanding field name (snake_case JSON keys) for the arousal blend.
    #[serde(default)]
    pub arousal_weights: std::collections::HashMap<String, f32>,
    /// Weight per ImageUnderstanding field name for the valence blend. The `fg_bg_contrast`
    /// term is fed through the `0.5 + 0.5*x` fluency transform INSIDE `affect_composite`
    /// (NOT pre-transformed in JSON).
    #[serde(default)]
    pub valence_weights: std::collections::HashMap<String, f32>,
    /// Per-character tempo windows, keyed by lowercase character name ("ballad","scherzo",…).
    #[serde(default)]
    pub character_tempo: std::collections::HashMap<String, CharacterTempo>,
}

impl Default for AffectMappings {
    /// The no-`affect`-block floor: empty weight maps (the composite then degenerates to a
    /// neutral 0.5/0.5 — harmless, since with no `affect` block the character ladder is also
    /// empty and the plan stays Ballad) AND the SINGLE legacy `ballad:{56,96}` tempo window,
    /// so `character_tempo_bpm(raw, Ballad, default)` == the old `clamp(56,96)` byte-for-byte.
    fn default() -> Self {
        let mut character_tempo = std::collections::HashMap::new();
        character_tempo.insert(
            "ballad".to_string(),
            CharacterTempo {
                bpm_min: 56.0,
                bpm_max: 96.0,
            },
        );
        AffectMappings {
            arousal_weights: std::collections::HashMap::new(),
            valence_weights: std::collections::HashMap::new(),
            character_tempo,
        }
    }
}

/// Pure. Weighted blend of EXISTING `ImageUnderstanding` scalars under the JSON weights.
/// The two HSV scalars (`avg_saturation`, `avg_brightness`) are 0..100 and divided by 100;
/// the rest are already 0..1. Output each clamped to 0..1.
///
/// AROUSAL = 0.45*(avg_saturation/100) + 0.25*colorfulness + 0.20*edge_activity + 0.10*complexity
/// VALENCE = 0.70*(avg_brightness/100) + 0.20*(avg_saturation/100) + 0.10*(0.5 + 0.5*fg_bg_contrast)
///
/// The weights come from `weights.arousal_weights` / `weights.valence_weights` keyed by the
/// snake_case field name. For each weighted field the term is `weight * normalized_field`,
/// where normalization is: `avg_saturation`→/100, `avg_brightness`→/100, `fg_bg_contrast`→
/// fluency transform (`0.5 + 0.5*x`), all others→identity. Sum, then clamp 0..1. When a weight
/// map is EMPTY (the default floor / no-affect-block path) the corresponding axis returns the
/// neutral 0.5 (an empty blend has no terms; seed it to 0.5 so a `Ge`/`Le` rule reads "neutral").
fn affect_composite(u: &ImageUnderstanding, weights: &AffectMappings) -> Affect {
    /// Normalize one knob field for the affect blend (mirrors the §3.2 field-name table).
    fn normalized_field(name: &str, u: &ImageUnderstanding) -> f32 {
        match name {
            "avg_saturation" => u.avg_saturation / 100.0,
            "avg_brightness" => u.avg_brightness / 100.0,
            "fg_bg_contrast" => 0.5 + 0.5 * u.fg_bg_contrast,
            "colorfulness" => u.colorfulness,
            "edge_activity" => u.edge_activity,
            "complexity" => u.complexity,
            // Any other field falls back to its raw value (no §4 weight row uses these).
            _ => 0.0,
        }
    }
    /// Sum `weight * normalized_field` over a weight map; an EMPTY map → neutral 0.5.
    fn blend(map: &std::collections::HashMap<String, f32>, u: &ImageUnderstanding) -> f32 {
        if map.is_empty() {
            return 0.5;
        }
        let sum: f32 = map
            .iter()
            .map(|(name, w)| w * normalized_field(name, u))
            .sum();
        sum.clamp(0.0, 1.0)
    }
    Affect {
        arousal: blend(&weights.arousal_weights, u),
        valence: blend(&weights.valence_weights, u),
    }
}

/// Clamp the raw brightness→BPM into the selected character's window from
/// `affect.character_tempo.<character>`. An ABSENT window (character name not in the map)
/// means "no clamp" — return `raw_bpm` unchanged (the legacy flat-path behaviour, which never
/// clamped). With the default `AffectMappings` (no-affect-block floor), the only window present
/// is `ballad:{56,96}`, so `character_tempo_bpm(raw, Ballad, default)` == the old
/// `clamp(56,96)` byte-for-byte. Pure. Replaces the hard clamp at composition.rs:727.
fn character_tempo_bpm(raw_bpm: f32, character: Character, affect: &AffectMappings) -> f32 {
    let key = character_tempo_key(character);
    match affect.character_tempo.get(key) {
        Some(w) => raw_bpm.clamp(w.bpm_min, w.bpm_max),
        None => raw_bpm,
    }
}

/// The lowercase JSON key for a [`Character`] in `affect.character_tempo` (matches the §4(b)
/// rows). Total over the closed enum.
fn character_tempo_key(character: Character) -> &'static str {
    match character {
        Character::Ballad => "ballad",
        Character::Hymn => "hymn",
        Character::Nocturne => "nocturne",
        Character::Drone => "drone",
        Character::March => "march",
        Character::Lament => "lament",
        Character::Waltz => "waltz",
        Character::Scherzo => "scherzo",
        Character::Lilt => "lilt",
        Character::Gigue => "gigue",
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// §1.5 The closed enums + §1.4 serde mapping structs
// ─────────────────────────────────────────────────────────────────────────────

/// A section's thematic role. Closed enum (serde rejects unknown variant).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ThematicRole {
    Statement,
    Contrast,
    Return,
    Development,
    Coda,
}

/// How a section varies its theme. Slice 1 USES only `Identity` + `Fragmented`; the rest
/// ship as schema (later stages).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ThemeVariation {
    Identity,
    Transposed,
    Reharmonized,
    Augmented,
    Diminished,
    Ornamented,
    Fragmented,
    Inverted,
    Retrograde,
}

/// The cadence strength closing a section.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum CadenceStrength {
    Half,
    Imperfect,
    Perfect,
    Deceptive,
    Plagal,
}

/// Time signature family. Slice 1 always `Four4`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum Meter {
    Four4,
    Three4,
    Six8,
    Two4,
}

/// Expressive character family. Slice 1 always `Ballad`; the rest ship as schema
/// (default-pinned), realized in later stages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum Character {
    Ballad,
    Hymn,
    Nocturne,
    Drone,
    March,
    Lament,
    Waltz,
    Scherzo,
    Lilt,
    Gigue,
}

/// The layer vocabulary — closed (mechanism), mirrors `chord_engine::OrchestralRole`
/// 1:1. serde-safe (rejects an unknown layer name). NEW S17. The role-assignment bridge
/// (`LayerRole` → `OrchestralRole`) lives in the realizer module that owns `OrchestralRole`;
/// this module only DEFINES the structural vocabulary the planner attaches per section.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum LayerRole {
    Bass,
    HarmonicFill,
    Melody,
    CounterMelody,
    Pad,
}

/// One named orchestration/texture profile — pure STRUCTURE, no note content. The planner
/// attaches one per [`Section`] (selected by the `texture` [`SelectTable`]); the realizer's
/// role-assignment + Pad branch read it. Adding a profile is a JSON row in `mappings.json`,
/// not a Rust edit (the `FormSpec` discipline). NEW S17.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct OrchestrationProfile {
    /// Stable id, e.g. `"identity"` / `"pad_bed"`.
    pub id: String,
    /// Which roles sound, in inst-index order; the realizer's `assign_role` maps instruments
    /// onto this list. serde rejects an unknown [`LayerRole`]. Empty == the identity sentinel.
    pub layers: Vec<LayerRole>,
    /// 0..1 density bias the realizer's edge-activity bands MAY shift by. Default `0.5` ==
    /// no-op (slice 1 does NOT wire this into the bands; reserved schema).
    #[serde(default = "half_f32")]
    pub density: f32,
    /// How many chord tones the Pad holds simultaneously (`0` == no pad). Default `0`.
    #[serde(default)]
    pub pad_voices: u8,
    /// NEW S20 — id of a `figuration_catalogue` row this profile's Pad animates with, or None
    /// for the S17 block bed. `#[serde(default)]` (== None) so EVERY old profile parses unchanged
    /// → byte-identical to S18. The planner resolves this handle (into `figuration_resolved`);
    /// the realizer reads the RESOLVED spec, never this raw handle.
    #[serde(default)]
    pub figuration: Option<String>,
    /// NEW S20 — the RESOLVED figuration spec for this section, filled by the planner from
    /// `figuration` against `figuration_catalogue`. NOT loaded from JSON (`#[serde(skip)]` →
    /// always `None` at deserialize); the planner sets it. The realizer reads THIS, never the
    /// raw `figuration` handle. `#[serde(skip)]` keeps mappings.json byte-shape unchanged and
    /// keeps `PartialEq`/`Clone` total.
    #[serde(skip)]
    pub figuration_resolved: Option<FigurationSpec>,
    /// NEW S23 — the RESOLVED per-layer prominence for this section, filled by the planner
    /// from the `prominence` `SelectTable`. NOT loaded from JSON (`#[serde(skip)]` → always
    /// empty at deserialize). EMPTY == the uniform/identity sentinel: the realizer takes its
    /// byte-stable legacy path. The realizer reads THIS.
    #[serde(skip)]
    pub prominence: Vec<LayerProminence>,
}

/// serde default for [`OrchestrationProfile::density`] — the no-op `0.5` midpoint.
fn half_f32() -> f32 {
    0.5
}

impl OrchestrationProfile {
    /// The behaviour-neutral profile: today's role split (the realizer's `assign_role`
    /// delegates to `instrument_role` under it), no pad. The byte-freeze anchor — every
    /// default Section literal carries this, so the realizer is byte-identical under it.
    pub fn identity() -> Self {
        OrchestrationProfile {
            id: "identity".to_string(),
            layers: Vec::new(),
            density: 0.5,
            pad_voices: 0,
            figuration: None,
            figuration_resolved: None,
            prominence: Vec::new(),
        }
    }

    /// `true` iff this is the behaviour-neutral profile: no pad AND no explicit layer split
    /// (the realizer reads this to take the byte-stable legacy `instrument_role` path).
    pub fn is_identity(&self) -> bool {
        self.pad_voices == 0 && self.layers.is_empty()
    }
}

/// One layer's resolved prominence weight for a section — the saliency "who is foreground"
/// signal. `role` reuses the EXISTING planner layer vocabulary ([`LayerRole`]); the realizer
/// bridges it to `OrchestralRole` via the existing `to_orchestral_role`. NEW S23.
#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize)]
pub struct LayerProminence {
    /// Which layer this weight applies to. serde rejects an unknown [`LayerRole`] name; the
    /// §2.6(d) JSON strings "Melody"/"CounterMelody"/"HarmonicFill"/"Pad"/"Bass" parse 1:1
    /// ([`LayerRole`] is `#[serde(rename_all = "PascalCase")]`).
    pub role: LayerRole,
    /// 0..1 prominence; 0.5 == neutral (every nudge is a no-op at exactly 0.5). 1.0 ==
    /// fully foreground (Melody louder/higher/freer); 0.0 == fully recessive.
    pub weight: f32,
}

/// One named prominence profile — pure structure. Selected by the `prominence` [`SelectTable`];
/// the planner copies its `layers` onto the section's [`OrchestrationProfile::prominence`].
/// Adding a profile is a JSON row, NOT a Rust edit (the `FigurationSpec`/`FormSpec`
/// discipline). NEW S23.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct ProminenceProfile {
    pub id: String,
    pub layers: Vec<LayerProminence>,
}

/// One named accompaniment-figuration pattern — pure STRUCTURE, no note content. Animates a
/// held chord into a bounded rhythmic burst within ONE step. Lives as a row in
/// `figuration_catalogue`; an [`OrchestrationProfile`] references it BY ID. Adding a pattern is a
/// JSON row, NOT a Rust edit (the `FormSpec`/`OrchestrationProfile` content-as-data discipline). NEW S20.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct FigurationSpec {
    /// Stable id, e.g. "alberti" / "block" (block == the no-op sustained bed: empty `onsets`).
    pub id: String,
    /// Per-step onset template, in TIME ORDER (ascending `at`). 2..=4 entries (the bounded
    /// burst). Empty == a block bed (no-op). serde rejects a malformed `FigurationOnset` entry.
    #[serde(default)]
    pub onsets: Vec<FigurationOnset>,
    /// How many DISTINCT inner chord tones the figure draws from (Alberti = 3). The mapper
    /// clamps this to the seated inner-tone count at realize time. Default 1.
    #[serde(default = "one_u8")]
    pub voices: u8,
}

/// One onset of a figure: WHEN it sounds (fraction of step_ms), WHICH seated inner-voice index
/// it sounds (cycled modulo the seated voice count), and how long it holds. NEW S20.
#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize)]
pub struct FigurationOnset {
    /// Onset time as a fraction of step_ms, 0.0..1.0 (0.0 == downbeat, 0.25 == the off-beat).
    pub at: f32,
    /// Seated inner-voice index this onset sounds; cycled modulo the seated voice count.
    pub tone: u8,
    /// Hold as a fraction of the GAP to the next onset (0.0..1.0), the in-step articulation.
    /// Default 1.0 (legato: fill the whole gap up to the per-onset cap).
    #[serde(default = "one_f32")]
    pub hold_frac: f32,
}

/// serde default for [`FigurationSpec::voices`].
fn one_u8() -> u8 {
    1
}

/// serde default for [`FigurationOnset::hold_frac`].
fn one_f32() -> f32 {
    1.0
}

/// One section's role in a FORM TEMPLATE — pure structure, no music content. The planner
/// expands these into concrete [`Section`]s. Loaded from `mappings.json`.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct SectionTemplate {
    /// "A" / "B" / "A'" / "T" / "V1" …
    pub label: String,
    /// Closed enum (serde rejects unknown variant).
    pub role: ThematicRole,
    /// Relative weight; scaled to fill `total_steps`.
    pub rel_len: f32,
    /// Which theme slot this section states/recalls (or `None`).
    pub theme: Option<usize>,
    /// Slice-1 set: `{Identity, Fragmented}`.
    pub variation: ThemeVariation,
    /// The cadence closing this section.
    pub boundary_cadence: CadenceStrength,
}

/// A FORM = an ordered section-template list + a stable id handle. THE FORM VOCABULARY
/// LIVES HERE, IN `mappings.json`. Adding a form is a JSON row, not a Rust enum edit.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct FormSpec {
    /// "rounded_binary" / "ternary_aba" / "aaba" / …
    pub id: String,
    /// The ordered section templates.
    pub sections: Vec<SectionTemplate>,
}

/// Closed handle naming a selectable [`ImageUnderstanding`] knob. New knob → enum variant +
/// a getter arm in [`Knob::read`]. serde rejects unknown variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Knob {
    EdgeActivity,
    Texture,
    Complexity,
    Colorfulness,
    ValueKey,
    AvgBrightness,
    AvgSaturation,
    DominantHue,
    PaletteBimodality,
    QuadrantContrast,
    VerticalEmphasis,
    AspectRatio,
    SubjectSize,
    FgBgContrast,
    SubjectEnergy,
    ForegroundEnergy,
    BackgroundEnergy,
    /// NEW S22 — the planner-computed arousal composite (0..1). Reads the runtime-only
    /// `affect_arousal` field the planner fills via `affect_composite` (NOT a pixel field).
    Arousal,
    /// NEW S22 — the planner-computed valence composite (0..1). Same discipline.
    Valence,
}

impl Knob {
    /// Read this knob's scalar value out of an [`ImageUnderstanding`]. The ONLY place a
    /// `Knob` is mapped to a field — a new knob adds exactly one arm here.
    pub fn read(self, u: &ImageUnderstanding) -> f32 {
        match self {
            Knob::EdgeActivity => u.edge_activity,
            Knob::Texture => u.texture,
            Knob::Complexity => u.complexity,
            Knob::Colorfulness => u.colorfulness,
            Knob::ValueKey => u.value_key,
            Knob::AvgBrightness => u.avg_brightness,
            Knob::AvgSaturation => u.avg_saturation,
            Knob::DominantHue => u.dominant_hue,
            Knob::PaletteBimodality => u.palette_bimodality,
            Knob::QuadrantContrast => u.quadrant_contrast,
            Knob::VerticalEmphasis => u.vertical_emphasis,
            Knob::AspectRatio => u.aspect_ratio,
            Knob::SubjectSize => u.subject_size,
            Knob::FgBgContrast => u.fg_bg_contrast,
            Knob::SubjectEnergy => u.subject_energy,
            Knob::ForegroundEnergy => u.foreground_energy,
            Knob::BackgroundEnergy => u.background_energy,
            Knob::Arousal => u.affect_arousal,
            Knob::Valence => u.affect_valence,
        }
    }
}

/// The comparison operator of a [`Predicate`]. Closed op set — NOT an expression language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CmpOp {
    Lt,
    Le,
    Gt,
    Ge,
    InRange,
}

/// A single threshold/range test over one [`ImageUnderstanding`] knob.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct Predicate {
    /// The knob this predicate reads.
    pub knob: Knob,
    /// `Lt | Le | Gt | Ge | InRange`.
    pub op: CmpOp,
    /// The threshold (or lower bound for `InRange`).
    pub lo: f32,
    /// The upper bound — used only by `InRange` (`lo..=hi`).
    #[serde(default)]
    pub hi: f32,
}

impl Predicate {
    /// Evaluate this predicate against `u`. Deterministic, pure.
    pub fn holds(&self, u: &ImageUnderstanding) -> bool {
        let v = self.knob.read(u);
        match self.op {
            CmpOp::Lt => v < self.lo,
            CmpOp::Le => v <= self.lo,
            CmpOp::Gt => v > self.lo,
            CmpOp::Ge => v >= self.lo,
            CmpOp::InRange => v >= self.lo && v <= self.hi,
        }
    }
}

/// One rule of a [`SelectTable`]: ALL predicates must hold (AND); rules are tried in order,
/// first match wins.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct SelectRule {
    /// ALL must hold (AND).
    pub when: Vec<Predicate>,
    /// The id this rule selects on a match.
    pub pick: String,
}

/// One axis's "default + ordered conditional departures." `pick`/`default` are string ids.
///
/// Derives `Default` (empty `default` id + no rules) so an axis can be `#[serde(default)]` on
/// a parent mapping struct — an absent axis yields a table whose `select` returns `""`, which
/// matches no catalogue id, so the consumer falls back to its own neutral default (S17).
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
pub struct SelectTable {
    /// The id chosen when no rule matches.
    pub default: String,
    /// Ordered first-match-wins rules.
    #[serde(default)]
    pub rules: Vec<SelectRule>,
}

impl SelectTable {
    /// Pure scan: the `pick` of the FIRST rule whose every [`Predicate`] holds against `u`,
    /// else `default`. Deterministic given `(u, self)`.
    pub fn select(&self, u: &ImageUnderstanding) -> String {
        for rule in &self.rules {
            if rule.when.iter().all(|p| p.holds(u)) {
                return rule.pick.clone();
            }
        }
        self.default.clone()
    }
}

/// Curated plan-selection tables over the [`ImageUnderstanding`] knobs. Loaded from
/// `mappings.json` (`composition` block). Each axis: a default id + ordered first-match-wins
/// rules. `character_overlays` / `key_schemes` ship as schema later; slice 1 omits or
/// default-pins them.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct PlanMappings {
    /// → a `FormSpec.id` from `form_catalogue`.
    pub form: SelectTable,
    /// → a `Character` variant name; slice 1 pinned "ballad".
    pub character: SelectTable,
    /// → a `Meter` name; slice 1 pinned "four4".
    pub meter: SelectTable,
    /// → a key-scheme id; slice 1 pinned "home_only".
    pub key_scheme: SelectTable,
    /// → "absent" | "fragment" | "second_theme".
    pub theme_behaviour: SelectTable,
    /// → an `OrchestrationProfile.id` from `texture_catalogue`. NEW S17 — parallel to
    /// `form`/`form_catalogue`. `#[serde(default)]` so an OLD `mappings.json` (no `texture`
    /// axis) still parses: the absent default yields an empty `SelectTable` → planner falls
    /// back to `OrchestrationProfile::identity()` (no pad — honest degradation).
    #[serde(default)]
    pub texture: SelectTable,
    /// The form vocabulary.
    pub form_catalogue: Vec<FormSpec>,
    /// The orchestration-profile vocabulary. NEW S17 — parallel to `form_catalogue`.
    /// `#[serde(default)]` so an old mappings.json parses (empty → planner uses identity).
    #[serde(default)]
    pub texture_catalogue: Vec<OrchestrationProfile>,
    /// NEW S20 — the figuration vocabulary, parallel to `form_catalogue`/`texture_catalogue`.
    /// `#[serde(default)]` (empty Vec) so an OLD mappings.json with no `figuration_catalogue`
    /// key parses; an unresolved profile handle then falls back to the block bed.
    #[serde(default)]
    pub figuration_catalogue: Vec<FigurationSpec>,
    /// NEW S22 — the affect weights + per-character tempo windows (§3.1). `#[serde(default)]`
    /// so an OLD mappings.json (no `affect` key) parses → `AffectMappings::default()`, which
    /// ships the legacy `ballad:{56,96}` window → the compose-path tempo is bit-identical.
    #[serde(default)]
    pub affect: AffectMappings,
    /// NEW S23 — selects a `prominence_catalogue` id from the saliency knobs (subject_size,
    /// fg_bg_contrast). `#[serde(default)]` empty `SelectTable` → "" → uniform (byte-stable
    /// legacy realization).
    #[serde(default)]
    pub prominence: SelectTable,
    /// NEW S23 — the prominence-profile vocabulary (id → per-layer weights). Parallel to
    /// `texture_catalogue`/`figuration_catalogue`. `#[serde(default)]` empty Vec.
    #[serde(default)]
    pub prominence_catalogue: Vec<ProminenceProfile>,
}

impl From<CompositionMappings> for PlanMappings {
    /// Adapt the loader's plain serde mirror into the planner's `PlanMappings`. The two have
    /// identical shape; this exists so `composition.rs` owns the planner type and
    /// `mapping_loader.rs` owns only the schema-present deserialize target.
    fn from(c: CompositionMappings) -> Self {
        PlanMappings {
            form: c.form,
            character: c.character,
            meter: c.meter,
            key_scheme: c.key_scheme,
            theme_behaviour: c.theme_behaviour,
            texture: c.texture,
            form_catalogue: c.form_catalogue,
            texture_catalogue: c.texture_catalogue,
            figuration_catalogue: c.figuration_catalogue,
            affect: c.affect,
            prominence: c.prominence,
            prominence_catalogue: c.prominence_catalogue,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// §1.2 / §1.3 The plan + its parts
// ─────────────────────────────────────────────────────────────────────────────

// NOTE (S15 cross-file): the motif-note type is `chord_engine::MotifNote` (Music Theory owns
// it; `resolve_motif` returns `Vec<chord_engine::MotifNote>` and `ThemeSeed.motif` stores it).
// `composition.rs` does NOT define its own `MotifNote` — there is one definition, in
// `chord_engine`, re-used here.

/// A returning-theme seed. The motif is KEY-RELATIVE (degree+duration) so a section could
/// transpose it by `key_offset` (slice 1 stays home, so it never does).
#[derive(Debug, Clone, PartialEq)]
pub struct ThemeSeed {
    /// Theme slot id (index into `CompositionPlan.themes`).
    pub id: usize,
    /// The EXPANDED concrete motif the realizer reads — degree+duration, key-relative.
    /// Produced at PLAN-BUILD time by `chord_engine::resolve_motif`. The archetype is NOT
    /// stored on the seed in slice 1 — resolution is one-way at build.
    pub motif: Vec<chord_engine::MotifNote>,
}

/// The piece's structural key + tempo SPINE — computed once, section-stable.
#[derive(Debug, Clone, PartialEq)]
pub struct KeyTempoPlan {
    /// Tonal home (from dominant-hue lookup; seeds, then offsets apply).
    pub home_root_midi: u8,
    /// The home mode name.
    pub home_mode: String,
    /// Base tempo (brightness→BPM, clamped by character window).
    pub base_ms_per_step: u64,
    /// `section_index → key_offset`; slice 1 ALL ZEROS.
    pub key_scheme: Vec<i8>,
    /// `section_index → ms_per_step`; slice 1 all `== base_ms_per_step`.
    pub tempo_scheme: Vec<u64>,
}

/// One section — a span of steps with a local identity and a theme ref. The unit the time
/// cursor walks; the per-step realizer is parameterized by the CURRENT section.
#[derive(Debug, Clone, PartialEq)]
pub struct Section {
    /// "A" / "B" / "A'" — carried to the snapshot/observer.
    pub label: String,
    /// How many global steps this section spans.
    pub step_len: usize,
    /// This section's thematic role.
    pub thematic_role: ThematicRole,
    /// Slice 1: ALWAYS 0 (home key) — modulation is Stage 5.
    pub key_offset_semitones: i8,
    /// Slice 1: `== key_tempo.base_ms_per_step` (section-stable).
    pub ms_per_step: u64,
    /// Slice 1: `== key_tempo.home_mode` (no modal plan yet).
    pub mode: String,
    /// Roman numerals for this section (filled by chord_engine).
    pub progression: Vec<String>,
    /// Index into `themes[]` this section states/recalls, or `None`.
    pub theme: Option<usize>,
    /// Slice 1: `Identity` or `Fragmented` only.
    pub variation: ThemeVariation,
    /// The cadence closing this section.
    pub boundary_cadence: CadenceStrength,
    /// Local density bias, 0..1; slice 1 default 0.5 (no-op).
    pub density: f32,
    /// NEW S17 — the selected orchestration profile for this section. The default paths
    /// (`legacy_default_section` / `single_section_default` consumers / the planner's
    /// identity fallback) carry [`OrchestrationProfile::identity()`], so the realizer is
    /// byte-stable under it; the compose path attaches a non-identity (`pad_voices > 0`)
    /// profile selected by the `texture` [`SelectTable`].
    pub orchestration: OrchestrationProfile,
    /// The section's own FILLED phrase plan (`chord_engine` output). This is where the
    /// per-section `StepPlan`s live so the realizer reads the section's own steps, never
    /// `plan[step_idx % len]`.
    pub steps: Vec<StepPlan>,
}

/// The up-front architectural plan for one piece — computed ONCE by [`CompositionPlanner`]
/// from an [`ImageUnderstanding`], then DRIVES per-step realization.
#[derive(Debug, Clone, PartialEq)]
pub struct CompositionPlan {
    /// The selected `FormSpec.id`.
    pub form: String,
    /// Closed enum — slice 1 always `Character::Ballad`.
    pub character: Character,
    /// Closed enum — slice 1 always `Meter::Four4`.
    pub meter: Meter,
    /// The key + tempo spine.
    pub key_tempo: KeyTempoPlan,
    /// The EXPANDED, concrete ordered sections — THIS IS THE PIECE.
    pub sections: Vec<Section>,
    /// Returning theme(s); a section with `theme: None` is valid.
    pub themes: Vec<ThemeSeed>,
    /// `== sum of section.step_len`; the time cursor's N.
    pub total_steps: usize,
}

impl CompositionPlan {
    /// Resolve the global step index `step_idx` (0..`total_steps`) to `(section, step_in_section)`
    /// by walking section boundaries with NO modulo — the death of `plan[step_idx % len]`.
    /// Returns `None` when there are no sections or `step_idx >= total_steps` (the engine
    /// never advances past `total_steps`, so the latter is a guard, not a hot path).
    pub fn locate(&self, step_idx: usize) -> Option<(&Section, usize)> {
        let mut acc = 0usize;
        for section in &self.sections {
            if section.step_len == 0 {
                continue;
            }
            if step_idx < acc + section.step_len {
                return Some((section, step_idx - acc));
            }
            acc += section.step_len;
        }
        None
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// §1.5 StepContext — the plan-relative per-step context (BORROWED, zero-copy)
// ─────────────────────────────────────────────────────────────────────────────

/// The plan-relative context for one scan step — WHICH section, its theme/key/tempo, and the
/// step's offset within the section. Threaded into the realizer so realization is DRIVEN BY
/// the plan. BORROWED (zero-copy) — operator decision 6.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StepContext<'a> {
    /// The section this step belongs to.
    pub section: &'a Section,
    /// The step's offset within `section`.
    pub step_in_section: usize,
    /// Resolved from `section.theme` against `plan.themes`.
    pub theme: Option<&'a ThemeSeed>,
    /// The piece's key + tempo spine.
    pub key_tempo: &'a KeyTempoPlan,
}

impl<'a> StepContext<'a> {
    /// The behaviour-neutral default: one section, no theme, home key, identity variation.
    /// Under it the kernel does EXACTLY what it does today — no transposition, no theme, home
    /// mode, same `ms_per_step`. Used by `engine_equivalence.rs` and the legacy flat path.
    pub fn single_section_default(
        section: &'a Section,
        key_tempo: &'a KeyTempoPlan,
    ) -> StepContext<'a> {
        StepContext {
            section,
            step_in_section: 0,
            theme: None,
            key_tempo,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// §3 The planner — selects form/theme, expands sections, delegates the music craft
// ─────────────────────────────────────────────────────────────────────────────

/// Base step budget PER SECTION before image scaling — sized so a 3-section form lands
/// near the legacy single-image scan length, deterministic and modest.
const BASE_STEPS_PER_SECTION: usize = 8;

/// Builds a [`CompositionPlan`] from an [`ImageUnderstanding`] by running the `SelectTable`
/// ladders, expanding the chosen [`FormSpec`], and delegating per-section harmony +
/// theme-motif resolution to `chord_engine`. It does NOT duplicate `chord_engine` craft.
pub struct CompositionPlanner {
    plan_mappings: PlanMappings,
}

impl CompositionPlanner {
    /// Construct from the loaded [`PlanMappings`].
    pub fn new(plan_mappings: PlanMappings) -> Self {
        CompositionPlanner { plan_mappings }
    }

    /// Read-only access to the underlying plan mappings.
    pub fn plan_mappings(&self) -> &PlanMappings {
        &self.plan_mappings
    }

    /// Deterministic given `(understanding, plan_mappings)` EXCEPT the delegated
    /// `pick_progression` `thread_rng` (the documented S9 boundary; the equivalence net never
    /// calls this path). Builds the full [`CompositionPlan`] per spec §3.
    ///
    /// `mappings` is the SAME `MappingTable` the engine holds — used to drive
    /// `pick_progression`/`generate_chords`/`plan_phrases` and the home root/tempo lookups, so
    /// the section harmony matches what `set_features_global` would derive.
    pub fn plan(&self, u: &ImageUnderstanding, mappings: &MappingTable) -> CompositionPlan {
        // S22: compute the affect composite once and seat it on a local working copy so the
        // character/tempo ladders read the real arousal/valence (the input `u` is borrowed `&`,
        // and the pixel producer left the affect fields at the -1.0 sentinel).
        let affect = affect_composite(u, &self.plan_mappings.affect);
        let mut u = u.clone();
        u.affect_arousal = affect.arousal;
        u.affect_valence = affect.valence;
        let u = &u; // shadow back to a shared borrow for the rest of plan (minimal blast radius)

        // 1) form id (first-match-wins, else default).
        let form_id = self.plan_mappings.form.select(u);
        // 2) character / meter / key_scheme / theme_behaviour. Slice 1: pinned.
        let character = parse_character(&self.plan_mappings.character.select(u));
        let meter = parse_meter(&self.plan_mappings.meter.select(u));
        let _key_scheme_id = self.plan_mappings.key_scheme.select(u); // slice 1: "home_only"
        let theme_behaviour = self.plan_mappings.theme_behaviour.select(u);

        // 3) lookup the chosen form; fall back to the default id, then to a minimal 1-section
        //    form so the planner is total (never panics on a malformed catalogue).
        let form_spec = lookup_form(&self.plan_mappings.form_catalogue, &form_id)
            .or_else(|| {
                lookup_form(
                    &self.plan_mappings.form_catalogue,
                    &self.plan_mappings.form.default,
                )
            })
            .cloned()
            .unwrap_or_else(fallback_form);

        // 4) KeyTempoPlan — home root + mode from the SAME paths set_features_global uses, so
        //    section harmony matches the legacy derivation. base_ms_per_step from
        //    brightness→BPM, clamped to a Ballad window.
        let home_mode =
            crate::mapping_loader::lookup_range_map(&mappings.global.hue_to_mode, u.dominant_hue)
                .unwrap_or_else(|| "Ionian".to_string());
        let home_root_midi = 60; // C4 seed (EngineConfig.root_midi default); offsets are all 0.
        let raw_bpm = interp_tempo_bpm(&mappings.global.brightness_to_tempo_bpm, u.avg_brightness);
        // Per-character tempo window (de-caps the legacy Ballad 56..96 clamp): the chosen
        // character selects the window; brightness positions BPM within it. Absent window → no clamp.
        let bpm = character_tempo_bpm(raw_bpm, character, &self.plan_mappings.affect);
        let base_ms_per_step = (60_000.0 / bpm.max(1.0)).round() as u64;

        // 5) total_steps from a base budget × section count, image-influenced (edge_activity
        //    lengthens busier images modestly), deterministic.
        let n_sections = form_spec.sections.len().max(1);
        let activity_bonus = (u.edge_activity * BASE_STEPS_PER_SECTION as f32).round() as usize;
        let steps_per_section = BASE_STEPS_PER_SECTION + activity_bonus;
        let total_steps = steps_per_section * n_sections;

        // 6) Themes — choose archetype + range/length from the image (hue + edge_activity),
        //    resolve via chord_engine::resolve_motif, gated by theme_behaviour. A form that
        //    references a theme slot (theme: Some) needs at least one theme present.
        let needs_theme = form_spec.sections.iter().any(|s| s.theme.is_some());
        let themes = if theme_behaviour == "absent" || !needs_theme {
            Vec::new()
        } else {
            let archetype = pick_archetype(u);
            // Range in scale degrees from edge_activity (calm = narrow, busy = wider), length
            // in steps from complexity (richer image = a slightly longer subject).
            let range_degrees = (2.0 + u.edge_activity * 5.0).round() as u8; // 2..=7
            let length_steps = (3.0 + u.complexity * 5.0).round() as usize; // 3..=8
            let motif = chord_engine::resolve_motif(archetype, range_degrees, length_steps);
            vec![ThemeSeed { id: 0, motif }]
        };

        // 7) Expand form_spec.sections → Vec<Section>, scaling rel_len to fill total_steps.
        let chord_engine = ChordEngine::new(rebuild_mapping_table(mappings));
        let rel_total: f32 = form_spec.sections.iter().map(|s| s.rel_len.max(0.0)).sum();
        let rel_total = if rel_total <= 0.0 {
            n_sections as f32
        } else {
            rel_total
        };

        // Brightness drop / raw scalars exactly as set_features_global feeds them, so the
        // section harmony matches the legacy single-section derivation.
        let brightness_drop = (0.5 - u.avg_brightness / 100.0).clamp(0.0, 1.0) * 2.0;

        // S17: select the orchestration profile ONCE per plan over the whole-image knobs
        // (`texture` SelectTable), then look it up in `texture_catalogue`. An absent/unmatched
        // id falls back to the byte-stable identity profile. Slice 1 selects once per plan (no
        // saliency knob); section-conditioned selection is a later slice.
        let texture_id = self.plan_mappings.texture.select(u);
        let mut orchestration =
            lookup_orchestration(&self.plan_mappings.texture_catalogue, &texture_id)
                .cloned()
                .unwrap_or_else(OrchestrationProfile::identity);
        // S20: resolve the figuration handle ONCE per plan against the catalogue. An unresolved /
        // None handle leaves `figuration_resolved == None` → the realizer takes the block bed.
        orchestration.figuration_resolved = orchestration
            .figuration
            .as_deref()
            .and_then(|id| lookup_figuration(&self.plan_mappings.figuration_catalogue, id))
            .cloned();
        // S23: resolve saliency → prominence ONCE per plan, immediately after the figuration
        // resolve. The `prominence` SelectTable picks a catalogue id from the saliency knobs
        // (subject_size, fg_bg_contrast); an absent/unmatched/`uniform` id leaves `prominence`
        // empty → the realizer takes its byte-stable uniform path. `orchestration.clone()` per
        // section (below) deep-clones this Vec onto each Section — no section-loop edit needed.
        let prom_id = self.plan_mappings.prominence.select(u);
        orchestration.prominence =
            lookup_prominence(&self.plan_mappings.prominence_catalogue, &prom_id)
                .map(|p| p.layers.clone())
                .unwrap_or_default();

        let mut sections: Vec<Section> = Vec::with_capacity(form_spec.sections.len());
        let mut assigned = 0usize;
        for (i, tpl) in form_spec.sections.iter().enumerate() {
            // Distribute total_steps by rel_len; the LAST section absorbs the rounding
            // remainder so the section lengths sum EXACTLY to total_steps (cursor invariant).
            let step_len = if i + 1 == form_spec.sections.len() {
                total_steps.saturating_sub(assigned)
            } else {
                let share =
                    ((tpl.rel_len.max(0.0) / rel_total) * total_steps as f32).round() as usize;
                share.min(total_steps.saturating_sub(assigned))
            };
            assigned += step_len;

            // Fill this section's harmony via the EXISTING chord_engine craft.
            let progression = chord_engine.pick_progression(&home_mode);
            let chords = chord_engine.generate_chords(
                &progression,
                home_root_midi,
                &home_mode,
                u.edge_activity * EDGE_ACTIVITY_RANGE_MAX, // raw edge density
                brightness_drop,
                u.avg_saturation, // raw 0..100
                u.colorfulness,   // raw hue_spread ~0..1
            );
            let steps = chord_engine.plan_phrases(&chords);

            sections.push(Section {
                label: tpl.label.clone(),
                step_len,
                thematic_role: tpl.role,
                key_offset_semitones: 0, // LOCKED slice 1
                ms_per_step: base_ms_per_step,
                mode: home_mode.clone(),
                progression,
                theme: if themes.is_empty() { None } else { tpl.theme },
                // LOCKED slice-1 variation set {Identity, Fragmented}: anything else clamps
                // to Identity so a later-stage variation can never leak into a slice-1 plan.
                variation: clamp_variation_slice1(tpl.variation),
                boundary_cadence: tpl.boundary_cadence,
                density: 0.5,
                orchestration: orchestration.clone(),
                steps,
            });
        }

        let key_scheme = vec![0i8; sections.len()];
        let tempo_scheme = vec![base_ms_per_step; sections.len()];

        CompositionPlan {
            form: form_spec.id.clone(),
            character,
            meter,
            key_tempo: KeyTempoPlan {
                home_root_midi,
                home_mode,
                base_ms_per_step,
                key_scheme,
                tempo_scheme,
            },
            sections,
            themes,
            total_steps,
        }
    }
}

/// Choose a melodic archetype from the image. Slice-1 ACTIVE subset is the original four
/// (Arch, Descent, Ascent, NeighborTurn); hue picks the broad shape and edge_activity tips
/// busy images toward more motion. Build-time only.
fn pick_archetype(u: &ImageUnderstanding) -> MotifArchetype {
    // Hue quadrant → broad melodic intent; a busy image (edge_activity high) leans active.
    let hue = u.dominant_hue.rem_euclid(360.0);
    if u.edge_activity >= 0.6 {
        return MotifArchetype::Ascent;
    }
    match hue {
        h if h < 90.0 => MotifArchetype::Arch,
        h if h < 180.0 => MotifArchetype::NeighborTurn,
        h if h < 270.0 => MotifArchetype::Descent,
        _ => MotifArchetype::Arch,
    }
}

/// Clamp a template variation into the slice-1 active set `{Identity, Fragmented}`. Any
/// later-stage variation collapses to `Identity` (a no-op) — the LOCKED invariant guard.
fn clamp_variation_slice1(v: ThemeVariation) -> ThemeVariation {
    match v {
        ThemeVariation::Fragmented => ThemeVariation::Fragmented,
        _ => ThemeVariation::Identity,
    }
}

/// Look up a `FormSpec` by id in the catalogue.
fn lookup_form<'a>(catalogue: &'a [FormSpec], id: &str) -> Option<&'a FormSpec> {
    catalogue.iter().find(|f| f.id == id)
}

/// Look up an [`OrchestrationProfile`] by id in the texture catalogue (S17). Mirrors
/// [`lookup_form`]; an absent/unmatched id returns `None` so the planner falls back to
/// [`OrchestrationProfile::identity()`].
fn lookup_orchestration<'a>(
    catalogue: &'a [OrchestrationProfile],
    id: &str,
) -> Option<&'a OrchestrationProfile> {
    catalogue.iter().find(|p| p.id == id)
}

/// Look up a [`FigurationSpec`] by id in the figuration catalogue (S20). Mirrors
/// [`lookup_orchestration`]; an absent/unmatched id returns `None` so the planner leaves
/// `figuration_resolved == None` and the realizer falls back to the block bed.
fn lookup_figuration<'a>(catalogue: &'a [FigurationSpec], id: &str) -> Option<&'a FigurationSpec> {
    catalogue.iter().find(|f| f.id == id)
}

/// Look up a [`ProminenceProfile`] by id in the prominence catalogue (S23). Mirrors
/// [`lookup_figuration`]; an absent/unmatched id returns `None` so the planner leaves
/// `OrchestrationProfile::prominence` empty and the realizer falls back to the byte-stable
/// uniform path.
fn lookup_prominence<'a>(
    catalogue: &'a [ProminenceProfile],
    id: &str,
) -> Option<&'a ProminenceProfile> {
    catalogue.iter().find(|p| p.id == id)
}

/// A minimal single-section fallback form so the planner is total when the catalogue is
/// empty/malformed. One Statement section, no theme, identity, perfect close.
fn fallback_form() -> FormSpec {
    FormSpec {
        id: "rounded_binary".to_string(),
        sections: vec![SectionTemplate {
            label: "A".to_string(),
            role: ThematicRole::Statement,
            rel_len: 1.0,
            theme: None,
            variation: ThemeVariation::Identity,
            boundary_cadence: CadenceStrength::Perfect,
        }],
    }
}

/// Parse a `Character` variant name (slice-1 pinned "ballad"); unknown → `Ballad`.
fn parse_character(s: &str) -> Character {
    match s {
        "ballad" => Character::Ballad,
        "hymn" => Character::Hymn,
        "nocturne" => Character::Nocturne,
        "drone" => Character::Drone,
        "march" => Character::March,
        "lament" => Character::Lament,
        "waltz" => Character::Waltz,
        "scherzo" => Character::Scherzo,
        "lilt" => Character::Lilt,
        "gigue" => Character::Gigue,
        _ => Character::Ballad,
    }
}

/// Parse a `Meter` variant name (slice-1 pinned "four4"); unknown → `Four4`.
fn parse_meter(s: &str) -> Meter {
    match s {
        "four4" => Meter::Four4,
        "three4" => Meter::Three4,
        "six8" => Meter::Six8,
        "two4" => Meter::Two4,
        _ => Meter::Four4,
    }
}

/// Continuous brightness(0..100) → BPM over the JSON anchor map — a local copy of the engine
/// helper (composition.rs cannot reach the engine's private fn, and duplicating the tiny
/// interpolator keeps the module boundary clean). Degenerate map → 240 BPM (legacy 250 ms).
fn interp_tempo_bpm(map: &std::collections::HashMap<String, u32>, brightness: f32) -> f32 {
    let mut anchors: Vec<(f32, f32)> = map
        .iter()
        .filter_map(|(k, v)| {
            let mut it = k.split('-');
            let lo: f32 = it.next()?.trim().parse().ok()?;
            let hi: f32 = it.next()?.trim().parse().ok()?;
            Some(((lo + hi) * 0.5, *v as f32))
        })
        .collect();
    anchors.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    if anchors.is_empty() {
        return 60_000.0 / 250.0;
    }
    if brightness <= anchors[0].0 {
        return anchors[0].1;
    }
    if brightness >= anchors[anchors.len() - 1].0 {
        return anchors[anchors.len() - 1].1;
    }
    for w in anchors.windows(2) {
        let (x0, y0) = w[0];
        let (x1, y1) = w[1];
        if brightness >= x0 && brightness <= x1 {
            let t = (brightness - x0) / (x1 - x0);
            return y0 + t * (y1 - y0);
        }
    }
    anchors[anchors.len() - 1].1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u_with(complexity: f32, edge: f32) -> ImageUnderstanding {
        ImageUnderstanding {
            complexity,
            edge_activity: edge,
            ..ImageUnderstanding::neutral()
        }
    }

    fn mappings() -> MappingTable {
        crate::mapping_loader::load_mappings("assets/mappings.json").expect("mappings load")
    }

    /// SelectTable first-match-wins: a rule that holds beats the default; none holding →
    /// default. Pure, no music.
    #[test]
    fn select_table_first_match_wins() {
        let table = SelectTable {
            default: "d".to_string(),
            rules: vec![
                SelectRule {
                    when: vec![Predicate {
                        knob: Knob::Complexity,
                        op: CmpOp::Ge,
                        lo: 0.9,
                        hi: 0.0,
                    }],
                    pick: "high".to_string(),
                },
                SelectRule {
                    when: vec![Predicate {
                        knob: Knob::Complexity,
                        op: CmpOp::Ge,
                        lo: 0.4,
                        hi: 0.0,
                    }],
                    pick: "mid".to_string(),
                },
            ],
        };
        assert_eq!(table.select(&u_with(0.95, 0.0)), "high");
        assert_eq!(table.select(&u_with(0.5, 0.0)), "mid");
        assert_eq!(table.select(&u_with(0.1, 0.0)), "d");
    }

    /// S18 Slice 2: the texture axis selects the CounterMelody-bearing `pad_bed_counter`
    /// profile when the foreground is busy AND a real subject/ground stratification exists
    /// (both predicates AND), else falls back to the `pad_bed` default. RNG-free, no planner —
    /// directly over the loaded `texture` SelectTable.
    #[test]
    fn texture_selects_pad_bed_counter_on_busy_foreground_subject() {
        let pm: PlanMappings = mappings()
            .composition
            .clone()
            .expect("composition block present")
            .into();
        let texture = &pm.texture;

        // Busy foreground (≥0.35) AND real subject (fg_bg_contrast ≥0.20) → pad_bed_counter.
        let counter = ImageUnderstanding {
            foreground_energy: 0.5,
            fg_bg_contrast: 0.3,
            ..ImageUnderstanding::neutral()
        };
        assert_eq!(texture.select(&counter), "pad_bed_counter");

        // Quiet foreground → the rule does NOT fire; default pad_bed (no counter).
        let quiet = ImageUnderstanding {
            foreground_energy: 0.1,
            fg_bg_contrast: 0.3,
            ..ImageUnderstanding::neutral()
        };
        assert_eq!(texture.select(&quiet), "pad_bed");

        // Busy foreground but no real subject (low contrast) → rule does NOT fire; default.
        let no_subject = ImageUnderstanding {
            foreground_energy: 0.5,
            fg_bg_contrast: 0.05,
            ..ImageUnderstanding::neutral()
        };
        assert_eq!(texture.select(&no_subject), "pad_bed");

        // The catalogue carries the new profile with a CounterMelody layer.
        let prof = pm
            .texture_catalogue
            .iter()
            .find(|p| p.id == "pad_bed_counter")
            .expect("pad_bed_counter profile present");
        assert!(
            prof.layers.contains(&LayerRole::CounterMelody),
            "pad_bed_counter carries a CounterMelody layer"
        );
    }

    /// S17 §11.7 back-compat ride-along: a texture axis whose `default`/rules name no profile
    /// present in `texture_catalogue` resolves to the identity profile (the `lookup_orchestration`
    /// → identity fallback). An OLD mappings.json with an empty/absent texture catalogue → no pad.
    #[test]
    fn texture_unknown_id_falls_back_to_identity() {
        // An empty catalogue (the absent-axis / old-mappings shape).
        assert!(super::lookup_orchestration(&[], "pad_bed").is_none());
        // A non-empty catalogue that does not carry the requested id.
        let cat = vec![OrchestrationProfile::identity()];
        assert!(super::lookup_orchestration(&cat, "pad_bed_counter").is_none());
        // The found case still works.
        assert!(super::lookup_orchestration(&cat, "identity").is_some());
        // The consumer's `.unwrap_or_else(identity)` therefore yields identity (no pad).
        let resolved = super::lookup_orchestration(&[], "pad_bed")
            .cloned()
            .unwrap_or_else(OrchestrationProfile::identity);
        assert!(
            resolved.is_identity(),
            "unknown texture id degrades to identity (no pad)"
        );
    }

    /// InRange predicate uses lo..=hi inclusive.
    #[test]
    fn predicate_in_range_inclusive() {
        let p = Predicate {
            knob: Knob::AvgBrightness,
            op: CmpOp::InRange,
            lo: 30.0,
            hi: 70.0,
        };
        assert!(p.holds(&ImageUnderstanding {
            avg_brightness: 30.0,
            ..ImageUnderstanding::neutral()
        }));
        assert!(p.holds(&ImageUnderstanding {
            avg_brightness: 70.0,
            ..ImageUnderstanding::neutral()
        }));
        assert!(!p.holds(&ImageUnderstanding {
            avg_brightness: 71.0,
            ..ImageUnderstanding::neutral()
        }));
    }

    /// The plan's sections sum EXACTLY to total_steps — the non-looping cursor invariant
    /// (no step is unreachable, none double-counted).
    #[test]
    fn plan_sections_sum_to_total_steps() {
        let pm: PlanMappings = mappings()
            .composition
            .clone()
            .expect("composition block present")
            .into();
        let planner = CompositionPlanner::new(pm);
        let plan = planner.plan(&u_with(0.2, 0.3), &mappings());
        let sum: usize = plan.sections.iter().map(|s| s.step_len).sum();
        assert_eq!(
            sum, plan.total_steps,
            "section lengths must tile total_steps"
        );
        assert!(plan.total_steps > 0, "a piece has steps");
    }

    /// locate() walks boundaries with NO modulo: each global step maps to exactly one
    /// (section, offset), and the offset is always < that section's step_len.
    #[test]
    fn locate_is_modulo_free_and_total() {
        let pm: PlanMappings = mappings()
            .composition
            .clone()
            .expect("composition block present")
            .into();
        let planner = CompositionPlanner::new(pm);
        let plan = planner.plan(&u_with(0.4, 0.5), &mappings());
        for step in 0..plan.total_steps {
            let (section, off) = plan.locate(step).expect("every in-range step locates");
            assert!(off < section.step_len, "offset within section bounds");
        }
        assert!(
            plan.locate(plan.total_steps).is_none(),
            "the cursor never advances past total_steps"
        );
    }

    /// LOCKED slice-1 Section invariants: every section is home-key, base-tempo, home-mode,
    /// variation ∈ {Identity, Fragmented}; the plan is Ballad / Four4.
    #[test]
    fn slice1_section_invariants_hold() {
        let pm: PlanMappings = mappings()
            .composition
            .clone()
            .expect("composition block present")
            .into();
        let planner = CompositionPlanner::new(pm);
        let plan = planner.plan(&u_with(0.5, 0.5), &mappings());
        assert_eq!(plan.character, Character::Ballad);
        assert_eq!(plan.meter, Meter::Four4);
        for s in &plan.sections {
            assert_eq!(s.key_offset_semitones, 0, "home key only in slice 1");
            assert_eq!(
                s.ms_per_step, plan.key_tempo.base_ms_per_step,
                "section tempo is the base tempo"
            );
            assert_eq!(
                s.mode, plan.key_tempo.home_mode,
                "section mode is home mode"
            );
            assert!(
                matches!(
                    s.variation,
                    ThemeVariation::Identity | ThemeVariation::Fragmented
                ),
                "slice-1 variation set is {{Identity, Fragmented}}"
            );
        }
    }

    /// single_section_default builds a no-theme, home-key context — the equivalence-net
    /// neutral point.
    #[test]
    fn step_context_default_is_neutral() {
        let section = Section {
            label: "A".to_string(),
            step_len: 2,
            thematic_role: ThematicRole::Statement,
            key_offset_semitones: 0,
            ms_per_step: 200,
            mode: "Ionian".to_string(),
            progression: vec![],
            theme: None,
            variation: ThemeVariation::Identity,
            boundary_cadence: CadenceStrength::Perfect,
            density: 0.5,
            orchestration: OrchestrationProfile::identity(),
            steps: vec![],
        };
        let kt = KeyTempoPlan {
            home_root_midi: 60,
            home_mode: "Ionian".to_string(),
            base_ms_per_step: 200,
            key_scheme: vec![0],
            tempo_scheme: vec![200],
        };
        let ctx = StepContext::single_section_default(&section, &kt);
        assert!(ctx.theme.is_none());
        assert_eq!(ctx.key_tempo.home_root_midi, 60);
        assert_eq!(ctx.section.key_offset_semitones, 0);
    }
}

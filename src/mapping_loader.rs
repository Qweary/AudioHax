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

#[derive(Debug, Deserialize)]
pub struct InstrumentSectionMapping {
    pub edge_density_to_rhythm: HashMap<String, String>,
    pub line_orientation_to_interval: HashMap<String, String>,
    pub contrast_to_articulation: HashMap<String, String>,
    pub color_shift_to_chord_extension: HashMap<String, Vec<String>>,
    pub texture_to_modal_color: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct FineDetailMapping {
    pub pixel_y_position_to_pitch: String,
    pub pixel_brightness_to_velocity: String,
    pub local_jaggedness_to_chromaticism: HashMap<String, i32>,
    pub shape_to_ostinato: HashMap<String, String>,
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
}

#[derive(Debug, Deserialize)]
pub struct MappingTable {
    pub global: GlobalMapping,
    pub instrument_section: InstrumentSectionMapping,
    pub fine_detail: FineDetailMapping,
    /// S15 Slice 1: the optional `composition` block (form catalogue + selection tables).
    /// Absent in the legacy `mappings.json` → `None` (back-compat floor). When present, the
    /// engine's compose path builds a `CompositionPlanner` from it.
    #[serde(default)]
    pub composition: Option<CompositionMappings>,
}

pub fn load_mappings(path: &str) -> Result<MappingTable, Box<dyn std::error::Error>> {
    let data = fs::read_to_string(path)?;
    let mappings: MappingTable = serde_json::from_str(&data)?;
    Ok(mappings)
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
        instrument_section: InstrumentSectionMapping {
            edge_density_to_rhythm: t.instrument_section.edge_density_to_rhythm.clone(),
            line_orientation_to_interval: t.instrument_section.line_orientation_to_interval.clone(),
            contrast_to_articulation: t.instrument_section.contrast_to_articulation.clone(),
            color_shift_to_chord_extension: t
                .instrument_section
                .color_shift_to_chord_extension
                .clone(),
            texture_to_modal_color: t.instrument_section.texture_to_modal_color.clone(),
        },
        fine_detail: FineDetailMapping {
            pixel_y_position_to_pitch: t.fine_detail.pixel_y_position_to_pitch.clone(),
            pixel_brightness_to_velocity: t.fine_detail.pixel_brightness_to_velocity.clone(),
            local_jaggedness_to_chromaticism: t
                .fine_detail
                .local_jaggedness_to_chromaticism
                .clone(),
            shape_to_ostinato: t.fine_detail.shape_to_ostinato.clone(),
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

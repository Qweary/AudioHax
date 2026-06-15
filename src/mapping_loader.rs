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

#[derive(Debug, Deserialize)]
pub struct MappingTable {
    pub global: GlobalMapping,
    pub instrument_section: InstrumentSectionMapping,
    pub fine_detail: FineDetailMapping,
}

pub fn load_mappings(path: &str) -> Result<MappingTable, Box<dyn std::error::Error>> {
    let data = fs::read_to_string(path)?;
    let mappings: MappingTable = serde_json::from_str(&data)?;
    Ok(mappings)
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

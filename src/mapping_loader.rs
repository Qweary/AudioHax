use serde::Deserialize;
use std::collections::HashMap;
use std::fs;

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

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
        assert_eq!(find("walking").density, 2);
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
}

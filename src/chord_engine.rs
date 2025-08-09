use rand::seq::SliceRandom;
use rand::thread_rng;
use std::collections::HashMap;
use crate::mapping_loader::{MappingTable, lookup_range_map};

/// Basic major scale degrees offsets (Ionian) in semitones from root
const IONIAN: [i8;7] = [0, 2, 4, 5, 7, 9, 11];
/// Natural minor (Aeolian)
const AEOLIAN: [i8;7] = [0, 2, 3, 5, 7, 8, 10];

/// Represent a chord as root MIDI note and a vec of intervals (semitones)
#[derive(Debug, Clone)]
pub struct Chord {
    pub name: String,
    pub notes: Vec<u8>, // actual MIDI note numbers
}

pub struct ChordEngine {
    pub mappings: MappingTable,
}

impl ChordEngine {
    pub fn new(mappings: MappingTable) -> Self {
        Self { mappings }
    }

    /// Choose a progression family based on mode (simple heuristic)
    pub fn pick_progression(&self, mode: &str) -> Vec<String> {
        let warm = vec!["Ionian","Lydian","Mixolydian"];
        let cool = vec!["Dorian","Aeolian","Phrygian"];

        let family_key = if warm.contains(&mode) {
            "warm"
        } else if cool.contains(&mode) {
            "cool"
        } else {
            "neutral"
        };

        if let Some(choices) = self.mappings.global.progression_families.get(family_key) {
            let mut rng = thread_rng();
            if let Some(p) = choices.choose(&mut rng) {
                // split chord progression string into parts "I-vi-IV-V"
                return p.split('-').map(|s| s.to_string()).collect();
            }
        }
        // fallback
        vec!["I".to_string(), "V".to_string(), "vi".to_string(), "IV".to_string()]
    }

    /// Generate chords (in MIDI note numbers) given root (C=60) and mode.
    /// root_midi: MIDI note number for tonic (e.g., 60 = C4)
    pub fn generate_chords(
        &self,
        progression: &[String],
        root_midi: u8,
        mode: &str,
        edge_complexity: f32,
        brightness_drop: f32,
    ) -> Vec<Chord> {
        // choose scale degrees offsets based on mode (simple)
        let scale = if mode == "Ionian" || mode == "Lydian" || mode == "Mixolydian" {
            IONIAN
        } else {
            AEOLIAN
        };

        // helper to convert Roman numeral (basic) to scale degree index
        let mut chords: Vec<Chord> = Vec::new();

        for (i, sym) in progression.iter().enumerate() {
            // simple modal interchange: if brightness drop is significant and chord is "IV", make it iv
            let sym_mod = if brightness_drop > self.mappings.global.modal_interchange_trigger.brightness_drop_threshold && sym == "IV" {
                "iv".to_string()
            } else { sym.clone() };

            // possible secondary dominant insertion: if edge_complexity high and mapping allows, insert V/V before progression chord
            if edge_complexity > self.mappings.global.dominant_substitution_trigger.edge_complexity_threshold {
                // heuristic: insert a V of the next chord
                if i + 1 < progression.len() {
                    let next = &progression[i + 1];
                    // compute V of next and insert (very simplified)
                    let v_chord = self.roman_to_chord("V", root_midi, &scale, "V");
                    chords.push(v_chord);
                }
            }

            let chord = self.roman_to_chord(&sym_mod, root_midi, &scale, mode);
            chords.push(chord);
        }

        chords
    }

    /// Convert a simple Roman numeral to a Chord (very simplified)
    fn roman_to_chord(&self, roman: &str, root_midi: u8, scale: &[i8;7], _mode: &str) -> Chord {
        // map some roman numerals to scale degree index (major/minor context simple mapping)
        // I -> degree 0, ii -> 1, III -> 2, IV -> 3, V -> 4, vi -> 5, vii° -> 6
        let lower = roman.to_lowercase();
        let degree = if lower.starts_with('i') && lower.len() == 1 { 0 } 
                     else if lower.starts_with("ii") { 1 }
                     else if lower.starts_with('i') && lower.len() == 3 && lower.chars().nth(1) == Some('i') { 2 } // III fallback
                     else if lower.starts_with('i') && lower.ends_with('v') && lower.len() == 2 { 4 } // iv/IV fallback rough
                     else if lower == "iv" { 3 }
                     else if lower == "v" { 4 }
                     else if lower == "vi" { 5 }
                     else if lower == "vii" { 6 }
                     else if lower == "iii" { 2 }
                     else { 0 };

        // root pitch of chord
        let root_semitone = scale[degree as usize];
        let root_note = (root_midi as i16 + root_semitone as i16) as u8;

        // triad (root, 3rd, 5th) using scale steps (simple)
        let third_degree = (degree + 2) % 7;
        let fifth_degree = (degree + 4) % 7;
        let third = (root_midi as i16 + scale[third_degree as usize] as i16) as u8;
        let fifth = (root_midi as i16 + scale[fifth_degree as usize] as i16) as u8;

        Chord {
            name: roman.to_string(),
            notes: vec![root_note, third, fifth],
        }
    }
}

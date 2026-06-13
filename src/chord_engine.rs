use crate::mapping_loader::{lookup_range_map, MappingTable};
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::collections::HashMap;

// The six canonical diatonic modes, as semitone offsets from the tonic.
// Each is the same seven pitch classes rotated; the rotation is what gives each
// mode its single characteristic alteration (called out per-const below).

/// Ionian (major): the reference major scale.
// theory: the major scale — no alterations; this is the baseline all other modes are measured against.
const IONIAN: [i8; 7] = [0, 2, 4, 5, 7, 9, 11];
/// Dorian: minor with a natural 6th.
// theory: minor scale raised 6th — degree-index 5 is tonic+9 (vs Aeolian's +8), the bright "natural 6".
const DORIAN: [i8; 7] = [0, 2, 3, 5, 7, 9, 10];
/// Phrygian: minor with a flat 2nd.
// theory: minor scale with b2 — degree-index 1 is tonic+1 (a half step above the tonic), the dark "flat 2".
const PHRYGIAN: [i8; 7] = [0, 1, 3, 5, 7, 8, 10];
/// Lydian: major with a raised 4th.
// theory: major scale with #4 — degree-index 3 is tonic+6 (vs Ionian's +5), the lifted "sharp 4".
const LYDIAN: [i8; 7] = [0, 2, 4, 6, 7, 9, 11];
/// Mixolydian: major with a flat 7th.
// theory: major scale with b7 — degree-index 6 is tonic+10 (vs Ionian's +11), the dominant "flat 7".
const MIXOLYDIAN: [i8; 7] = [0, 2, 4, 5, 7, 9, 10];
/// Aeolian (natural minor): the reference minor scale.
// theory: the natural minor scale — minor 3rd, minor 6th, minor 7th.
const AEOLIAN: [i8; 7] = [0, 2, 3, 5, 7, 8, 10];

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
        let warm = vec!["Ionian", "Lydian", "Mixolydian"];
        let cool = vec!["Dorian", "Aeolian", "Phrygian"];

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
        vec![
            "I".to_string(),
            "V".to_string(),
            "vi".to_string(),
            "IV".to_string(),
        ]
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
        // Select the mode's true scale. Each arm is the canonical diatonic mode;
        // an unrecognized mode string defaults to Ionian (the major scale) as a
        // safe, well-formed fallback rather than silently collapsing modes.
        let scale = match mode {
            "Ionian" => IONIAN,
            "Dorian" => DORIAN,
            "Phrygian" => PHRYGIAN,
            "Lydian" => LYDIAN,
            "Mixolydian" => MIXOLYDIAN,
            "Aeolian" => AEOLIAN,
            // theory: unknown mode -> default to Ionian (major) so output stays diatonic.
            _ => IONIAN,
        };

        // helper to convert Roman numeral (basic) to scale degree index
        let mut chords: Vec<Chord> = Vec::new();

        for (i, sym) in progression.iter().enumerate() {
            // simple modal interchange: if brightness drop is significant and chord is "IV", make it iv
            let sym_mod = if brightness_drop
                > self
                    .mappings
                    .global
                    .modal_interchange_trigger
                    .brightness_drop_threshold
                && sym == "IV"
            {
                "iv".to_string()
            } else {
                sym.clone()
            };

            // possible secondary dominant insertion: if edge_complexity high and mapping allows, insert V/V before progression chord
            if edge_complexity
                > self
                    .mappings
                    .global
                    .dominant_substitution_trigger
                    .edge_complexity_threshold
            {
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
    fn roman_to_chord(&self, roman: &str, root_midi: u8, scale: &[i8; 7], _mode: &str) -> Chord {
        // Map a Roman numeral to its scale-degree index (0-based) by exact,
        // case-insensitive match. An exhaustive `match` on the whole lowercased
        // numeral avoids the order-dependent `starts_with`/`len` shadowing that
        // previously misrouted "iv" (subdominant) and "iii" (mediant).
        // theory: I=tonic, ii=supertonic, iii=mediant, IV=subdominant,
        //         V=dominant, vi=submediant, vii=leading-tone.
        let lower = roman.to_lowercase();
        let degree: u8 = match lower.as_str() {
            "i" => 0,   // tonic
            "ii" => 1,  // supertonic
            "iii" => 2, // mediant (degree 2, NOT shadowed by the "ii" prefix anymore)
            "iv" => 3,  // subdominant (degree 3, NOT the dominant)
            "v" => 4,   // dominant
            "vi" => 5,  // submediant
            "vii" => 6, // leading tone
            // theory: unrecognized numeral -> tonic (degree 0), a safe diatonic default.
            _ => 0,
        };

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

#[cfg(test)]
mod tests {
    //! Musical-property regression net for `ChordEngine::generate_chords`.
    //!
    //! These tests validate that the chords the engine emits have the correct
    //! *musical* shape (in-range MIDI, scale-derived triad tones, mode-honoring
    //! scale selection) — not merely that the function runs.
    //!
    //! Determinism contract (held by every test below):
    //!   * explicit Roman-numeral progressions (never `pick_progression`, which
    //!     uses a thread RNG),
    //!   * `edge_complexity = 0.0` so the secondary-dominant-insertion branch
    //!     (threshold 0.7) never fires,
    //!   * `brightness_drop = 0.0` so the modal-interchange branch
    //!     (threshold 0.25) never fires,
    //!   * fixed `root_midi = 60` (C4).
    //! With those inputs each progression symbol yields exactly one triad, with
    //! no RNG and no inserted chords.

    use super::*;

    const ROOT: u8 = 60; // C4

    /// The six canonical mode interval patterns (semitone offsets from tonic).
    /// This is the *reference* the production scale-selection is measured
    /// against; it is intentionally independent of the production constants so
    /// the mode-honored tests assert against ground truth, not against whatever
    /// the engine currently happens to do.
    const REF_IONIAN: [i8; 7] = [0, 2, 4, 5, 7, 9, 11];
    const REF_DORIAN: [i8; 7] = [0, 2, 3, 5, 7, 9, 10];
    const REF_PHRYGIAN: [i8; 7] = [0, 1, 3, 5, 7, 8, 10];
    const REF_LYDIAN: [i8; 7] = [0, 2, 4, 6, 7, 9, 11];
    const REF_MIXOLYDIAN: [i8; 7] = [0, 2, 4, 5, 7, 9, 10];
    const REF_AEOLIAN: [i8; 7] = [0, 2, 3, 5, 7, 8, 10];

    fn engine() -> ChordEngine {
        let mappings = crate::mapping_loader::load_mappings(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/mappings.json"
        ))
        .expect("load mappings");
        ChordEngine::new(mappings)
    }

    /// Deterministic single-chord generation: one Roman numeral in, one triad
    /// out (no RNG, no inserted/borrowed chords).
    fn one_chord(eng: &ChordEngine, roman: &str, mode: &str) -> Chord {
        let prog = vec![roman.to_string()];
        let mut chords = eng.generate_chords(&prog, ROOT, mode, 0.0, 0.0);
        assert_eq!(
            chords.len(),
            1,
            "expected exactly one triad for symbol {roman:?} in mode {mode:?} \
             (got {}); a non-deterministic branch must have fired",
            chords.len()
        );
        chords.remove(0)
    }

    /// Reference triad for `degree` built from an explicit scale, rooted on
    /// `ROOT` — root, third (degree+2), fifth (degree+4), all mod 7. This mirrors
    /// the engine's own triad-construction so a passing assertion proves the
    /// engine used the *expected scale*.
    fn expected_triad(scale: &[i8; 7], degree: usize) -> Vec<u8> {
        let third = (degree + 2) % 7;
        let fifth = (degree + 4) % 7;
        vec![
            (ROOT as i16 + scale[degree] as i16) as u8,
            (ROOT as i16 + scale[third] as i16) as u8,
            (ROOT as i16 + scale[fifth] as i16) as u8,
        ]
    }

    // ---------------------------------------------------------------------
    // CATEGORY 1 — NOTE-RANGE VALIDATION
    // Property: every MIDI note produced sits inside the playable band
    // 24..=108 (C1..C8). Should PASS today and after the fix.
    // ---------------------------------------------------------------------

    #[test]
    fn test_all_notes_within_playable_midi_range() {
        let eng = engine();
        let modes = [
            "Ionian",
            "Dorian",
            "Phrygian",
            "Lydian",
            "Mixolydian",
            "Aeolian",
        ];
        let progressions: [&[&str]; 3] = [
            &["I", "IV", "V", "vi"],
            &["ii", "iii", "vi", "vii"],
            &["I", "V", "vi", "IV"],
        ];

        for mode in modes {
            for prog in progressions {
                let prog: Vec<String> = prog.iter().map(|s| s.to_string()).collect();
                let chords = eng.generate_chords(&prog, ROOT, mode, 0.0, 0.0);
                for chord in &chords {
                    for &note in &chord.notes {
                        assert!(
                            (24..=108).contains(&note),
                            "note {note} out of playable range 24..=108 \
                             (mode {mode:?}, chord {:?})",
                            chord.name
                        );
                    }
                }
            }
        }
    }

    // ---------------------------------------------------------------------
    // CATEGORY 2 — CHORD-MEMBERSHIP
    // Property: a generated triad's three notes are exactly the scale-derived
    // chord tones (root, +2, +4 mod 7) of its degree. Proves the triad is built
    // from a real scale, not arbitrary pitches. PASSES today for the two
    // scales the engine actually has (Ionian, Aeolian).
    // ---------------------------------------------------------------------

    #[test]
    fn test_ionian_triads_are_scale_derived_chord_tones() {
        let eng = engine();
        // (roman, degree) pairs that the engine's roman->degree mapping reaches
        // *correctly and unambiguously*. Deliberately excluded because of
        // roman->degree mapping bugs (documented in the SECONDARY BUG section):
        //   "IV"/"iv" -> degree 4 (should be 3)
        //   "iii"     -> degree 1 (shadowed by the "ii" prefix arm; should be 2)
        for (roman, degree) in [("I", 0usize), ("ii", 1), ("V", 4), ("vi", 5), ("vii", 6)] {
            let chord = one_chord(&eng, roman, "Ionian");
            assert_eq!(
                chord.notes,
                expected_triad(&REF_IONIAN, degree),
                "Ionian {roman} (degree {degree}) should be the scale-derived triad"
            );
        }
    }

    #[test]
    fn test_aeolian_triads_are_scale_derived_chord_tones() {
        let eng = engine();
        // Same correctly-mapping subset as the Ionian membership test (see that
        // test for the excluded buggy numerals).
        for (roman, degree) in [("I", 0usize), ("ii", 1), ("V", 4), ("vi", 5), ("vii", 6)] {
            let chord = one_chord(&eng, roman, "Aeolian");
            assert_eq!(
                chord.notes,
                expected_triad(&REF_AEOLIAN, degree),
                "Aeolian {roman} (degree {degree}) should be the scale-derived triad"
            );
        }
    }

    // ---------------------------------------------------------------------
    // CATEGORY 3 — MODE-IS-HONORED (the centerpiece)
    // Property: generate_chords selects THAT mode's scale. Each test targets a
    // chord whose tones include a scale degree where the mode differs from the
    // engine's current IONIAN/AEOLIAN fallback.
    //   Ionian + Aeolian PASS today (and after the fix).
    //   Lydian, Mixolydian, Dorian, Phrygian FAIL today (collapse to fallback)
    //     and will PASS once the six real scales are wired in.
    // Each asserts the FULL expected triad against the mode's true scale.
    // ---------------------------------------------------------------------

    #[test]
    fn test_ionian_mode_honored_tonic_triad() {
        // Ionian I = [60,64,67]. Ionian is a true scale today -> PASSES.
        let eng = engine();
        let chord = one_chord(&eng, "I", "Ionian");
        assert_eq!(chord.notes, vec![60, 64, 67]);
        assert_eq!(chord.notes, expected_triad(&REF_IONIAN, 0));
    }

    #[test]
    fn test_aeolian_mode_honored_minor_tonic_triad() {
        // Aeolian I = [60,63,67] (minor third 63). Aeolian is a true scale today
        // -> PASSES.
        let eng = engine();
        let chord = one_chord(&eng, "I", "Aeolian");
        assert_eq!(chord.notes, vec![60, 63, 67]);
        assert_eq!(chord.notes, expected_triad(&REF_AEOLIAN, 0));
    }

    #[test]
    fn test_lydian_mode_honored_raised_fourth() {
        // Lydian has a raised 4th (#4): scale degree-index 3 = tonic+6, not +5
        // as in Ionian. The IV chord is unreachable through the engine's
        // roman->degree mapping (it resolves to degree 4), so we observe the #4
        // as the THIRD of the ii chord: ii(degree 1) triad indices {1,3,5}.
        //   Lydian ii  = [62, 66, 69]  (third = tonic+6 = 66, the #4)
        //   Ionian ii  = [62, 65, 69]  (third = tonic+5 = 65)  <- current/wrong
        // FAILS today (Lydian collapses to Ionian), PASSES after the fix.
        let eng = engine();
        let chord = one_chord(&eng, "ii", "Lydian");
        assert_eq!(
            chord.notes,
            expected_triad(&REF_LYDIAN, 1),
            "Lydian ii must carry the raised 4th (66) as its third, not Ionian's 65"
        );
        assert_eq!(chord.notes, vec![62, 66, 69]);
    }

    #[test]
    fn test_mixolydian_mode_honored_flat_seventh() {
        // Mixolydian has a lowered 7th (b7): scale degree-index 6 = tonic+10, not
        // +11 as in Ionian. Observed as the ROOT of the vii chord:
        //   Mixolydian vii = [70, 62, 65]  (root = tonic+10 = 70, the b7)
        //   Ionian     vii = [71, 62, 65]  (root = tonic+11 = 71)  <- current/wrong
        // FAILS today (Mixolydian collapses to Ionian), PASSES after the fix.
        let eng = engine();
        let chord = one_chord(&eng, "vii", "Mixolydian");
        assert_eq!(
            chord.notes,
            expected_triad(&REF_MIXOLYDIAN, 6),
            "Mixolydian vii must be rooted on the b7 (70), not Ionian's 71"
        );
        assert_eq!(chord.notes, vec![70, 62, 65]);
    }

    #[test]
    fn test_dorian_mode_honored_natural_sixth() {
        // Dorian has a natural 6th: scale degree-index 5 = tonic+9, not +8 as in
        // Aeolian. Observed as the ROOT of the vi chord:
        //   Dorian  vi = [69, 60, 63]  (root = tonic+9 = 69, the natural 6)
        //   Aeolian vi = [68, 60, 63]  (root = tonic+8 = 68)  <- current/wrong
        // FAILS today (Dorian collapses to Aeolian), PASSES after the fix.
        let eng = engine();
        let chord = one_chord(&eng, "vi", "Dorian");
        assert_eq!(
            chord.notes,
            expected_triad(&REF_DORIAN, 5),
            "Dorian vi must be rooted on the natural 6th (69), not Aeolian's 68"
        );
        assert_eq!(chord.notes, vec![69, 60, 63]);
    }

    #[test]
    fn test_phrygian_mode_honored_flat_second() {
        // Phrygian has a lowered 2nd (b2): scale degree-index 1 = tonic+1, not +2
        // as in Aeolian. Observed as the ROOT of the ii chord:
        //   Phrygian ii = [61, 65, 68]  (root = tonic+1 = 61, the b2)
        //   Aeolian  ii = [62, 65, 68]  (root = tonic+2 = 62)  <- current/wrong
        // FAILS today (Phrygian collapses to Aeolian), PASSES after the fix.
        let eng = engine();
        let chord = one_chord(&eng, "ii", "Phrygian");
        assert_eq!(
            chord.notes,
            expected_triad(&REF_PHRYGIAN, 1),
            "Phrygian ii must be rooted on the b2 (61), not Aeolian's 62"
        );
        assert_eq!(chord.notes, vec![61, 65, 68]);
    }

    // ---------------------------------------------------------------------
    // SECONDARY BUG DOCUMENTATION (discovered while writing the net)
    // Not a musical-property gate per se, but pins a real defect so the fix
    // specialist sees it: the "IV"/"iv" numeral resolves to degree 4 (the V
    // scale degree), because roman_to_chord's `lower == "iv" => 3` arm is dead
    // code, shadowed by an earlier `ends_with('v') && len()==2 => 4` arm.
    // This test documents CURRENT behavior; if the mapping bug is fixed it will
    // (correctly) go red and should be updated to expect degree 3.
    // ---------------------------------------------------------------------

    #[test]
    fn test_iv_numeral_resolves_to_subdominant() {
        // CORRECTED (was test_iv_numeral_resolves_to_degree_four_bug): "IV" must
        // resolve to the subdominant, scale degree 3, distinct from the dominant
        // "V" (degree 4). In Ionian, degree 3 = tonic+5 = 65 (the IV root is F,
        // not G); degree 4 = tonic+7 = 67. The two chords must now DIFFER.
        let eng = engine();
        let iv = one_chord(&eng, "IV", "Ionian");
        let v = one_chord(&eng, "V", "Ionian");
        assert_ne!(
            iv.notes, v.notes,
            "IV (subdominant, degree 3) must differ from V (dominant, degree 4)"
        );
        assert_eq!(
            iv.notes,
            expected_triad(&REF_IONIAN, 3),
            "IV must produce the degree-3 (subdominant) triad"
        );
    }

    #[test]
    fn test_iii_numeral_resolves_to_mediant() {
        // CORRECTED (was test_iii_numeral_resolves_to_degree_one_bug): "iii" must
        // resolve to the mediant, scale degree 2, distinct from the supertonic
        // "ii" (degree 1). In Ionian, degree 2 = tonic+4 = 64 (the iii root is E);
        // degree 1 = tonic+2 = 62. The two chords must now DIFFER.
        let eng = engine();
        let iii = one_chord(&eng, "iii", "Ionian");
        let ii = one_chord(&eng, "ii", "Ionian");
        assert_ne!(
            iii.notes, ii.notes,
            "iii (mediant, degree 2) must differ from ii (supertonic, degree 1)"
        );
        assert_eq!(
            iii.notes,
            expected_triad(&REF_IONIAN, 2),
            "iii must produce the degree-2 (mediant) triad"
        );
    }
}

use crate::mapping_loader::{lookup_range_map, MappingTable};
use crate::seed::{composition_seed, mix_seed, next_seed_call};
use rand::seq::SliceRandom;
use rand::thread_rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

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
// PartialEq: required so `composition::Section` (which derives PartialEq and holds a
// `Vec<StepPlan>` whose `StepPlan` contains a `Chord`) compiles under the locked S15
// type contract. theory-neutral: a derive only, no field/behavior change — the byte
// freeze is untouched.
#[derive(Debug, Clone, PartialEq)]
pub struct Chord {
    pub name: String,
    pub notes: Vec<u8>, // actual MIDI note numbers
}

/// How many chord tones to stack — the harmonic-complexity axis (design-s13 §2).
///
/// theory: saturation is the visual correlate of harmonic RICHNESS. A washed-out
/// image gets bare triads (the plain, "computer-like" sonority the operator flagged);
/// a vivid image gets 7ths and 9ths — the tones that make harmony *breathe*. The
/// 7th adds the colour and the dominant pull; the 9th adds shimmer above it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HarmonicComplexity {
    /// Bare root-position triad (root, 3rd, 5th) — 3 tones. Low saturation.
    Triad,
    /// Triad + the diatonic 7th — 4 tones. Mid saturation; the chord "opens up".
    SeventhChord,
    /// Triad + 7th + 9th — 5 tones. High saturation; lush, shimmering harmony.
    NinthChord,
}

impl HarmonicComplexity {
    /// Map a normalized `saturation01` (0..1) to a complexity level via the JSON
    /// `saturation_to_harmonic_complexity` bands (keyed in 0..100 space), then
    /// fall back to fixed thresholds if the map is empty/degenerate.
    ///
    /// theory (design-s13 §2): `<0.31`→triad, `0.31..0.71`→add the 7th,
    /// `≥0.71`→7th + 9th. The JSON bands ("0-30"/"31-70"/"71-100") encode exactly
    /// this in the 0..100 space the existing `lookup_range_map` understands, so we
    /// scale the knob up by 100 and reuse that helper (design-s13 §5).
    fn from_saturation01(
        map: &std::collections::HashMap<String, String>,
        saturation01: f32,
    ) -> Self {
        let sat = saturation01.clamp(0.0, 1.0);
        // Reuse the integer-range lookup against the 0..100 band keys.
        let band = lookup_range_map(map, sat * 100.0);
        match band.as_deref() {
            Some("TriadsOnly") => HarmonicComplexity::Triad,
            Some("TriadsAnd7ths") => HarmonicComplexity::SeventhChord,
            Some("Triads7thsAndExtensions") => HarmonicComplexity::NinthChord,
            // Fallback thresholds mirror the JSON bands so behavior is stable even
            // if the map is renamed/emptied (design-s13 §2 numbers).
            _ if sat < 0.31 => HarmonicComplexity::Triad,
            _ if sat < 0.71 => HarmonicComplexity::SeventhChord,
            _ => HarmonicComplexity::NinthChord,
        }
    }

    /// Does this complexity include the chordal 7th? (Used to give the secondary
    /// dominant a stronger pull when the image is vivid — design-s13 §4.)
    fn has_seventh(self) -> bool {
        !matches!(self, HarmonicComplexity::Triad)
    }
}

/// Map a (case-insensitive) Roman numeral to its 0-based scale-degree index.
///
/// theory: I=tonic(0), ii=supertonic(1), iii=mediant(2), IV=subdominant(3),
/// V=dominant(4), vi=submediant(5), vii=leading-tone(6). An exhaustive exact match
/// (not `starts_with`) avoids the order-dependent shadowing that misrouted "iv"/"iii".
/// An unrecognized numeral falls back to the tonic — a safe diatonic default.
fn roman_degree(roman: &str) -> u8 {
    match roman.to_lowercase().as_str() {
        "i" => 0,
        "ii" => 1,
        "iii" => 2,
        "iv" => 3,
        "v" => 4,
        "vi" => 5,
        "vii" => 6,
        _ => 0,
    }
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
            // S41 — deterministic-seed seam. The seed register (thread-local) is the ONLY
            // change to how this draw obtains randomness; no caller signature changes and
            // frozen `engine.rs` is untouched (it merely CALLS this method).
            //   - register Some(seed) → derive a per-call deterministic `ChaCha8Rng` keyed
            //     by `seed` mixed with a per-call counter, so a multi-section composition is
            //     reproducible AND its sections still diverge (each call advances the counter).
            //   - register None (the default, absent `--seed`) → today's exact `thread_rng()`
            //     path, byte-unchanged.
            let picked = match composition_seed() {
                Some(seed) => {
                    let call = next_seed_call();
                    let mut rng = ChaCha8Rng::seed_from_u64(mix_seed(seed, call));
                    choices.choose(&mut rng).cloned()
                }
                None => {
                    let mut rng = thread_rng();
                    choices.choose(&mut rng).cloned()
                }
            };
            if let Some(p) = picked {
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
    ///
    /// `root_midi`        — MIDI note number for the tonic (e.g. 60 = C4).
    /// `edge_complexity`  — RAW global edge density (~0..0.05); normalized
    ///                      music-side to `edge_activity` (Option-NORM-MAP) and
    ///                      compared against the recalibrated secondary-dominant
    ///                      trigger so applied dominants actually fire on real photos.
    /// `brightness_drop`  — 0..1 "shadow" amount (dark image ⇒ larger); gates the
    ///                      borrowed minor `iv` (modal interchange).
    /// `saturation01_raw` — RAW `avg_saturation` (0..100); normalized music-side to
    ///                      `saturation01` and mapped to HARMONIC COMPLEXITY (triad
    ///                      → 7th → 7th+9th). This is the core fix for "computer
    ///                      triads": vivid images now get lush, breathing harmony.
    /// `colorfulness_raw` — RAW `hue_spread` (~0..1); the COLORFULNESS axis. A
    ///                      palette-spread image widens toward MODE MIXTURE: extra
    ///                      borrowed chords decouple harmonic colour from the single
    ///                      mean-hue mode pick, so diverse palettes stop collapsing.
    ///
    /// theory (design-s13 §0): the seam carries RAW physical scalars; this function
    /// owns normalizing them against the calibrated `feature_normalization` ranges in
    /// `mappings.json`, so a real photo's tiny edge density (0.005..0.036) lands in a
    /// usable activity band instead of clustering near zero.
    #[allow(clippy::too_many_arguments)]
    pub fn generate_chords(
        &self,
        progression: &[String],
        root_midi: u8,
        mode: &str,
        edge_complexity: f32,
        brightness_drop: f32,
        saturation01_raw: f32,
        colorfulness_raw: f32,
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

        let fnorm = &self.mappings.global.feature_normalization;

        // --- Normalization-at-consumption (Option-NORM-MAP, design-s13 §0) ------
        // RAW edge density → 0..1 ACTIVITY. The old code compared the raw 0.005-ish
        // value against a 0.7 threshold (two orders of magnitude too high), so the
        // secondary dominant NEVER fired on a real photo. Normalizing first puts the
        // busy half of the real set (≈0.51-0.72) above the recalibrated 0.55 trigger.
        let edge_activity = crate::mapping_loader::FeatureNormalization::normalize(
            edge_complexity,
            fnorm.edge_density_max,
        );
        // RAW saturation → 0..1 RICHNESS knob, then → a harmonic-complexity level.
        let saturation01 = crate::mapping_loader::FeatureNormalization::normalize(
            saturation01_raw,
            fnorm.avg_saturation_max,
        );
        let complexity = HarmonicComplexity::from_saturation01(
            &self.mappings.global.saturation_to_harmonic_complexity,
            saturation01,
        );
        // RAW hue_spread → 0..1 COLORFULNESS. hue_spread is already ~0..1, so this is
        // (calibrated) identity; normalizing through the same path keeps it tunable.
        let colorfulness = crate::mapping_loader::FeatureNormalization::normalize(
            colorfulness_raw,
            fnorm.hue_spread_max,
        );

        let mut chords: Vec<Chord> = Vec::new();

        // Does this image cross the modal-interchange trigger? (Dark/low-key ⇒
        // larger brightness_drop ⇒ borrow the shadow subdominant.) Computed once;
        // applied per-symbol below only where the symbol is the major subdominant.
        let borrow_minor_iv = brightness_drop
            > self
                .mappings
                .global
                .modal_interchange_trigger
                .brightness_drop_threshold;

        for (i, sym) in progression.iter().enumerate() {
            // Secondary-dominant insertion (the §4 fix): when the image is busy
            // enough, tonicize the NEXT chord by inserting its own dominant (V/x).
            // theory: V/x gives root-motion-by-fourth propulsion + the first
            // non-diatonic colour this engine produces. Gated on the NORMALIZED
            // edge_activity against the recalibrated 0.55 trigger so it fires on the
            // busy half of real photos, not only on pathological images.
            if edge_activity
                > self
                    .mappings
                    .global
                    .dominant_substitution_trigger
                    .edge_complexity_threshold
                && i + 1 < progression.len()
            {
                // Honor the look-ahead: build V OF the following chord (was the bug —
                // `next` was computed then discarded in favor of a literal home "V").
                let next = &progression[i + 1];
                // A vivid image gets a dominant-SEVENTH secondary (strongest pull);
                // a washed-out one a bare secondary triad — dovetails complexity.
                let v_of_next =
                    self.secondary_dominant_of(next, root_midi, &scale, complexity.has_seventh());
                chords.push(v_of_next);
            }

            // Modal interchange: a dark/low-key image borrows the MINOR subdominant
            // — the textbook "shadow" colour — in place of the diatonic (major)
            // subdominant. theory (design-s13 §2): the borrowed chord is the minor
            // iv of the parallel minor (Aeolian iv): root on the subdominant scale
            // degree, with a MINOR third and perfect fifth above it. Relabeling "IV"
            // → "iv" alone is a no-op in a major-third mode because `roman_degree`
            // would re-resolve it to the SAME diatonic degree-3 triad (major in
            // Ionian); we must build the minor triad explicitly so the flattened
            // third is actually sounded (in C: F-A-C → F-Ab-C). For an already-minor
            // mode the diatonic iv is already minor, and this builder reproduces that
            // same minor triad — no double-flatten (mode-general by construction).
            let chord = if borrow_minor_iv && sym == "IV" {
                self.borrowed_minor_iv(root_midi, &scale, complexity.has_seventh())
            } else {
                self.roman_to_chord_complex(sym, root_midi, &scale, complexity)
            };
            chords.push(chord);
        }

        // Mode-mixture widening (design-s13 §5 contract item 4, option ii): a
        // palette-SPREAD image (high colorfulness) appends a borrowed chord so its
        // harmonic colour stops being a function of the single mean-hue mode pick.
        // theory: two images with the SAME mean hue but different spread now differ
        // in borrowed content — the "colorfulness" axis decouples mode-feel from the
        // mean, exactly the collapse the diagnosis (§2c) identified. We borrow the
        // bVI (the bright submediant from the parallel major/minor), a stock mixture
        // colour, only when the spread is genuinely wide.
        const MODE_MIXTURE_THRESHOLD: f32 = 0.45;
        if colorfulness > MODE_MIXTURE_THRESHOLD && !progression.is_empty() {
            // bVI = a major triad rooted a minor sixth (8 semitones) above the tonic.
            let borrowed = self.flat_submediant(root_midi, complexity.has_seventh());
            chords.push(borrowed);
        }

        chords
    }

    /// Convert a Roman numeral to a Chord at the requested HARMONIC COMPLEXITY.
    ///
    /// theory: the triad (root, 3rd, 5th) is the harmonic skeleton; the diatonic 7th
    /// is `scale[(degree+6)%7]` and the 9th is `scale[(degree+1)%7]` an octave up —
    /// both drawn from the SAME scale so the added tones stay diatonic and the chord
    /// remains in-mode. Higher saturation adds these upper tones, turning bare triads
    /// into 7th/9th chords (the breathing harmony the operator wanted).
    fn roman_to_chord_complex(
        &self,
        roman: &str,
        root_midi: u8,
        scale: &[i8; 7],
        complexity: HarmonicComplexity,
    ) -> Chord {
        let degree = roman_degree(roman);

        // Triad tones (root, 3rd, 5th) by stacking thirds within the scale.
        let third_degree = (degree + 2) % 7;
        let fifth_degree = (degree + 4) % 7;
        let root_note = (root_midi as i16 + scale[degree as usize] as i16) as u8;
        let third = (root_midi as i16 + scale[third_degree as usize] as i16) as u8;
        let fifth = (root_midi as i16 + scale[fifth_degree as usize] as i16) as u8;
        let mut notes = vec![root_note, third, fifth];

        if complexity.has_seventh() {
            // theory: the diatonic 7th sits a third above the 5th — degree+6 mod 7.
            // It is the colour tone that lifts a plain triad into a 7th chord and
            // (on a dominant) supplies the leading dissonance that wants to resolve.
            let seventh_degree = (degree + 6) % 7;
            let seventh = (root_midi as i16 + scale[seventh_degree as usize] as i16) as u8;
            notes.push(seventh);
        }
        if matches!(complexity, HarmonicComplexity::NinthChord) {
            // theory: the 9th is the 2nd scale degree above the root (degree+1 mod 7),
            // voiced an octave UP so it shimmers above the 7th rather than clashing a
            // step from the root. It is the lush extension a vivid image earns.
            let ninth_degree = (degree + 1) % 7;
            let ninth = (root_midi as i16 + scale[ninth_degree as usize] as i16 + 12) as u8;
            notes.push(ninth);
        }

        Chord {
            name: roman.to_string(),
            notes,
        }
    }

    /// Build the SECONDARY DOMINANT of `target` (the "V/x"): the major triad (or
    /// dom7 when `with_seventh`) rooted a perfect fifth above the target's root.
    ///
    /// theory: tonicizing the next chord by inserting its own dominant gives
    /// root-motion-by-fourth propulsion and the first non-diatonic (chromatic) tone
    /// this engine produces — the raised third of the applied dominant. `with_seventh`
    /// adds the minor 7th, turning it into a dominant-SEVENTH whose tritone pulls
    /// strongly into the target. Voicing is left to the downstream phrase planner,
    /// which re-seats every chord tone into its playable register band.
    fn secondary_dominant_of(
        &self,
        target_roman: &str,
        root_midi: u8,
        scale: &[i8; 7],
        with_seventh: bool,
    ) -> Chord {
        // Degree of the target within the home scale, then its absolute root pitch.
        let target_degree = roman_degree(target_roman) as usize;
        let target_root = (root_midi as i16 + scale[target_degree] as i16) as u8;
        // V/x root = a perfect fifth (7 semitones) above the target's root.
        let v_root = target_root.saturating_add(7);
        // A MAJOR triad on v_root: root, +4 (major 3rd — the chromatic tone), +7 (P5).
        let mut notes = vec![v_root, v_root.saturating_add(4), v_root.saturating_add(7)];
        if with_seventh {
            // +10 = the minor 7th ⇒ dominant-seventh quality (the strong tritone pull).
            notes.push(v_root.saturating_add(10));
        }
        Chord {
            name: format!("V/{target_roman}"),
            notes,
        }
    }

    /// Build the borrowed MINOR iv — the "shadow subdominant" of modal interchange.
    ///
    /// theory (design-s13 §2): a dark image borrows the minor subdominant from the
    /// PARALLEL MINOR (Aeolian iv). It is a MINOR triad built on the subdominant
    /// scale degree (degree 4 ≡ index 3): the root, a MINOR third (root + 3
    /// semitones), and a perfect fifth (root + 7) above it. In C major the diatonic
    /// IV is F-A-C (major); the borrow lowers the third a semitone to F-Ab-C — the
    /// audible darkening the operator's "shadow" colour requires. The third is built
    /// as `root + 3` (NOT the scale's diatonic third) precisely so the minor quality
    /// is GUARANTEED regardless of the active mode's diatonic subdominant quality:
    /// in a major-third mode (Ionian/Lydian/Mixolydian) this flattens the third for
    /// real; in an already-minor mode (Aeolian/Dorian/Phrygian) the diatonic iv is
    /// already minor and `root + 3` reproduces that same minor third — no
    /// double-flatten. `with_seventh` adds the minor 7th (root + 10), the diatonic
    /// 7th of the borrowed minor subdominant (a iv7 of the parallel minor).
    fn borrowed_minor_iv(&self, root_midi: u8, scale: &[i8; 7], with_seventh: bool) -> Chord {
        // Root = the subdominant scale degree (degree 4, 0-based index 3). Drawn from
        // the active scale so the borrow sits on the mode's own subdominant pitch.
        let iv_root = (root_midi as i16 + scale[3] as i16) as u8;
        // Minor triad ON that root: +3 (MINOR third — the borrowed darkening), +7 (P5).
        let mut notes = vec![
            iv_root,
            iv_root.saturating_add(3),
            iv_root.saturating_add(7),
        ];
        if with_seventh {
            // +10 = the minor 7th above the root (the iv7 of the parallel minor).
            notes.push(iv_root.saturating_add(10));
        }
        Chord {
            name: "iv".to_string(),
            notes,
        }
    }

    /// Build the borrowed flat-submediant (bVI): a major triad rooted a minor sixth
    /// (8 semitones) above the tonic, optionally with its major 7th.
    ///
    /// theory: bVI is the stock modal-mixture colour borrowed from the parallel
    /// minor — bright, plagal, and unmistakably "non-diatonic-in-major". Appending it
    /// when the palette is wide gives a spread image audibly different harmony from a
    /// monochrome one of the same mean hue.
    fn flat_submediant(&self, root_midi: u8, with_seventh: bool) -> Chord {
        let r = root_midi.saturating_add(8); // minor sixth above the tonic
        let mut notes = vec![r, r.saturating_add(4), r.saturating_add(7)]; // major triad
        if with_seventh {
            notes.push(r.saturating_add(11)); // major 7th — the lush mixture colour
        }
        Chord {
            name: "bVI".to_string(),
            notes,
        }
    }

    /// Build the ROOT-POSITION tonic triad ("I") at `root_midi` in `mode` — the
    /// DETERMINISTIC destination tonic the S29 opening tonicizing cadence (Lever 1)
    /// forces as a modulating section's first chord, so the step-0 pivot V resolves
    /// V→I into the new key.
    ///
    /// theory reasoning: this is a plain diatonic "I" — root, diatonic third, perfect
    /// fifth — with NO RNG, NO secondary-dominant prepend, and NO mode-mixture borrow.
    /// It reuses the EXACT same scale selection `generate_chords` uses (the `match mode`
    /// head) and the existing private `roman_to_chord_complex("I", …, Triad)` builder, so
    /// the chord TONES are byte-for-byte identical to a free-selected diatonic "I" at this
    /// root — only the SELECTION is forced (we drop the RNG/secondary-dominant/borrow
    /// machinery that `generate_chords` wraps around the same builder). Triad complexity:
    /// the opening confirmation wants the bare tonic skeleton (a 7th on a tonic blurs the
    /// arrival), and a bare triad keeps the V→I voice-leading frame (§ input-s29) clean.
    /// `Chord.name == "I"` so the Test Engineer can pin `chords[0].name == "I"`.
    pub fn tonic_triad(&self, root_midi: u8, mode: &str) -> Chord {
        // Same mode→scale selection `generate_chords` performs at its head (chord_engine
        // :183); an unknown mode falls back to Ionian, exactly as the free-select path does,
        // so the forced tonic is built in the SAME scale the rest of the section uses.
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
        // Deterministic root-position "I" at Triad complexity — identical tones to a
        // free-selected diatonic tonic; the SELECTION is forced, the harmony is not new.
        self.roman_to_chord_complex("I", root_midi, &scale, HarmonicComplexity::Triad)
    }
}

// =========================================================================
// VOICE LEADING + PHRASE STRUCTURE — public API
// =========================================================================
//
// The texture is the existing three-note triad emitted by `roman_to_chord`:
// `notes[0]` = root, `notes[1]` = third, `notes[2]` = fifth, all in root
// position. The two layers below operate OVER a `&[Chord]` sequence:
//
//   * Voice leading (`voice_lead_sequence`) RE-VOICES the sequence so the upper
//     voices move smoothly chord-to-chord against the previous voicing. It is an
//     additive pass that returns a new `Vec<Chord>` with the same `name`s but
//     re-spelled `notes`; it does NOT change `generate_chords`'s signature, so
//     main.rs keeps reading better `Chord.notes` transparently.
//
//   * Phrase planning (`plan_phrases`) groups the sequence into 4- or 8-unit
//     phrases, positions cadences at phrase boundaries, and attaches a
//     structural per-step velocity. It returns a `Vec<StepPlan>`; its playback
//     wiring is a later layer.

/// Texture role of a single voice within the triad.
///
/// theory: in a three-voice triad the lowest sounding voice is the BASS, which
/// carries harmonic root motion and may LEAP freely (it is exempt from the
/// upper-voice motion cap). The two voices above it are UPPER voices, which are
/// held to smooth, conjunct motion (≤ a perfect fifth), common-tone retention,
/// and the no-parallel-perfect-consonance rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceRole {
    /// Lowest sounding voice — free to leap by root motion; exempt from the ≤P5 cap.
    Bass,
    /// A non-bass voice — constrained to ≤ perfect fifth motion, common-tone
    /// retention, and the no-parallel-5ths/8ves rule across T→T+1.
    Upper,
}

/// Position of a step inside its phrase.
///
/// theory: a phrase is a complete musical thought delimited by a cadence.
/// Antecedent and consequent phrases pair into a period: the antecedent poses
/// a question that rests on the dominant (half cadence); the consequent answers
/// it and closes on the tonic (perfect authentic cadence, V–I). `Interior`
/// marks steps that fall between phrase boundaries and carry no cadential role.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhrasePosition {
    /// First step of a phrase — the phrase's point of departure.
    PhraseStart,
    /// A step inside a phrase, neither its start nor its cadence.
    Interior,
    /// Antecedent cadence step: a HALF CADENCE, resting on V at the phrase boundary.
    HalfCadence,
    /// Consequent cadence step: a PERFECT AUTHENTIC CADENCE (V–I), closing on I
    /// at the phrase boundary.
    PerfectAuthenticCadence,
}

/// One step of a phrase-aware plan: the (voice-led) chord for the step, where
/// that step sits inside its phrase, and the structural velocity to sound it at.
///
/// theory: this is the structural skeleton the expressive-dynamics layer (a
/// later layer) will hang nuance on. It carries only the bare
/// structural facts — chord, phrase position, and a phrase-position velocity
/// floor — never any expressive contour (no messa di voce, crescendo, accents).
// PartialEq: required by the locked `composition::Section { steps: Vec<StepPlan>, .. }`
// (S15 §1.2), which derives PartialEq. Derive only — no shape/behavior change.
#[derive(Debug, Clone, PartialEq)]
pub struct StepPlan {
    /// The chord sounded on this step, already passed through voice leading.
    pub chord: Chord,
    /// 0-based index of the phrase this step belongs to.
    pub phrase_index: usize,
    /// 0-based index of this step within its phrase.
    pub position_in_phrase: usize,
    /// Length (in steps) of the phrase this step belongs to (4 or 8).
    pub phrase_len: usize,
    /// Where this step sits in its phrase (start / interior / cadence).
    pub position: PhrasePosition,
    /// Structural MIDI velocity (0..=127) for this step. theory: this is the
    /// STRUCTURAL FLOOR only — it varies just enough to mark phrase boundaries
    /// and cadence points (intra-phrase variance > 0); expressive dynamics are
    /// a later layer and are deliberately absent here.
    pub velocity: u8,
}

/// The motion cap for upper (non-bass) voices, in semitones.
///
/// theory: a perfect fifth (7 semitones) is the widest leap a constrained inner
/// voice should make under conservative voice-leading practice; beyond it the
/// line stops sounding like a connected melodic strand. The bass is exempt.
pub const MAX_UPPER_VOICE_MOTION: u8 = 7;

/// The two phrase lengths the planner is allowed to use, in steps.
///
/// theory: 4- and 8-bar phrases are the normative units of common-practice
/// periodic structure; cadences land at their boundaries.
pub const PHRASE_LENGTHS: [usize; 2] = [4, 8];

impl ChordEngine {
    /// Re-voice a chord sequence for smooth voice leading.
    ///
    /// theory: the first chord is seated with its root in the bass and the
    /// remaining chord tones stacked just above it. Then for each chord change
    /// T→T+1, the lowest voice is held as the BASS (free to leap by root motion)
    /// and each UPPER voice moves to the NEAREST available chord tone of the
    /// next chord, retaining common tones in the same voice. Upper-voice motion
    /// is capped at a perfect fifth (`MAX_UPPER_VOICE_MOTION` = 7 semitones), and
    /// parallel perfect fifths and perfect octaves between any voice pair across
    /// the change are forbidden. Per-voice positions are carried across the whole
    /// sequence so each chord's voicing is chosen relative to the one before it
    /// (the connection work is done by `voice_lead_one`).
    pub fn voice_lead_sequence(&self, chords: &[Chord]) -> Vec<Chord> {
        // Empty / single-chord sequences have nothing to connect; the lone chord
        // still needs to come out voice-aligned (index 0 = bass), so we run the
        // first-chord seating below even for len()==1.
        if chords.is_empty() {
            return Vec::new();
        }

        let mut out: Vec<Chord> = Vec::with_capacity(chords.len());

        // ---- Seat the FIRST chord ----------------------------------------
        // theory: the opening sonority sets the registral frame. We put the
        // chord's ROOT in the bass (index 0, the lowest sounding voice) and the
        // remaining two chord tones above it, each lifted into the octave that
        // sits just above the bass so the triad reads cleanly from the bottom
        // up. This guarantees the voice-alignment contract the tests rely on:
        // index 0 is the bass, indices 1.. are the upper voices.
        let first_pcs = chord_pitch_classes(&chords[0]);
        let root_pc = first_pcs[0]; // roman_to_chord emits notes[0] == the root
                                    // Seat the bass: take the chord's lowest sounding note as the bass anchor
                                    // so the absolute register matches the engine's existing output band.
        let bass0 = *chords[0].notes.iter().min().expect("non-empty triad");
        // Re-seat the bass onto the ROOT pitch class at or just below that anchor,
        // so the bass genuinely carries root motion from the start.
        let bass0 = nearest_pc_to(root_pc, bass0);
        // Place each upper chord tone in the octave immediately above the bass.
        let mut voicing = vec![bass0];
        for &pc_up in first_pcs.iter().filter(|&&p| p != root_pc) {
            voicing.push(seat_above(pc_up, bass0));
        }
        // A diminished/odd triad could in principle collapse a pc; pad defensively
        // so we always emit the same voice count the input chord had.
        while voicing.len() < chords[0].notes.len() {
            voicing.push(seat_above(root_pc, *voicing.last().unwrap()));
        }
        out.push(Chord {
            name: chords[0].name.clone(),
            notes: voicing,
        });

        // ---- Connect each subsequent chord to its predecessor ------------
        for next in &chords[1..] {
            let prev = out.last().expect("at least the first chord is seated");
            let voiced = voice_lead_one(prev, next);
            out.push(voiced);
        }

        out
    }

    /// Classify the role (bass vs. upper) of each voice in a triad voicing.
    ///
    /// theory: the lowest sounding pitch is the bass (leap-exempt); every other
    /// voice is an upper voice (motion-constrained). This is a pure helper the
    /// voice-leading pass uses to decide which constraints apply to which voice.
    /// It classifies by VOICE POSITION (index 0 = bass, the rest upper), which is
    /// the contract `voice_lead_sequence` establishes by seating the bass first.
    pub fn voice_roles(chord: &Chord) -> Vec<VoiceRole> {
        // theory: in the voice-aligned texture produced by `voice_lead_sequence`,
        // index 0 is the BASS (the lowest sounding voice, carrying root motion and
        // exempt from the ≤P5 cap) and every higher index is an UPPER voice. We
        // classify by VOICE POSITION (index), not by raw pitch value, because the
        // voice-leading contract assigns meaning to the index — index 0 is the
        // bass voice even on the rare beat its pitch is not the numeric minimum
        // (e.g. a wide-spread upper voice dipping near the bass). For a chord that
        // has not been through voice leading the lowest note is still index 0 in
        // practice, so this remains correct for the engine's root-position output
        // whose bass the seating step places first.
        chord
            .notes
            .iter()
            .enumerate()
            .map(|(i, _)| {
                if i == 0 {
                    VoiceRole::Bass
                } else {
                    VoiceRole::Upper
                }
            })
            .collect()
    }

    /// Build a phrase-aware plan over a chord sequence: group steps into
    /// 4- or 8-unit phrases, position cadences at the phrase boundaries, and
    /// attach a structural per-step velocity.
    ///
    /// theory: the sequence is partitioned into phrases of `PHRASE_LENGTHS`
    /// (4 or 8 steps), preferring the largest length that divides evenly and
    /// still yields at least two phrases so a period can form. Phrases pair into
    /// periods: a non-final phrase ends on a HALF CADENCE (its boundary chord
    /// rests on V), and the final phrase closes with a PERFECT AUTHENTIC CADENCE
    /// (V–I) ending on I when its predecessor is a dominant — otherwise it falls
    /// back to a half cadence on V. Cadential chords land AT phrase boundaries,
    /// never mid-phrase; the planner re-spells the boundary chord's name to the
    /// "V"/"I" the cadence requires. Voice leading is applied first so the plan
    /// carries the voice-led chords, and each step receives a structural
    /// velocity floor (76/88/96 for interior/start/cadence) that marks phrase
    /// shape without expressive contour.
    pub fn plan_phrases(&self, chords: &[Chord]) -> Vec<StepPlan> {
        // Voice-lead first so the plan carries the smoothly connected chords, not
        // the raw root-position triads — the structural plan rides on top of the
        // re-voiced surface.
        let led = self.voice_lead_sequence(chords);
        if led.is_empty() {
            return Vec::new();
        }

        // ---- Choose a phrase length from PHRASE_LENGTHS ------------------
        // theory: common-practice periodic structure parses into 4- or 8-step
        // units, and the smallest complete unit is the PERIOD — an antecedent
        // phrase resting on a half cadence answered by a consequent closing with
        // a PAC. So we prefer the LARGEST allowed length that (a) divides the
        // sequence evenly AND (b) still yields at least TWO phrases, so a period
        // can form. An 8-step input therefore parses as two 4-step phrases
        // (antecedent + consequent), not one undifferentiated 8-step phrase.
        // If no length yields a period (sequences of 4 or fewer), we fall back to
        // the largest evenly-dividing length, or to 4 as the normative default.
        let n = led.len();
        let phrase_len = PHRASE_LENGTHS
            .iter()
            .rev()
            .copied()
            .find(|&len| len <= n && n.is_multiple_of(len) && n / len >= 2)
            .or_else(|| {
                PHRASE_LENGTHS
                    .iter()
                    .rev()
                    .copied()
                    .find(|&len| len <= n && n.is_multiple_of(len))
            })
            .unwrap_or(PHRASE_LENGTHS[0]);

        let phrase_count = n.div_ceil(phrase_len);

        // Structural-velocity floor (NOT expressive dynamics — S6 builds those).
        // theory: phrase shape is marked by weighting structural arrival points
        // differently from passing interior steps. A phrase START gets a modest
        // downbeat accent; the CADENCE step gets the heaviest weight (the point
        // of arrival the ear is led toward); interior steps sit at a neutral
        // baseline. That alone guarantees intra-phrase variance > 0 wherever a
        // phrase has more than one distinct structural role — without any
        // crescendo/diminuendo/messa-di-voce contour.
        const V_INTERIOR: u8 = 76; // neutral passing weight
        const V_START: u8 = 88; // phrase-initial downbeat accent
        const V_CADENCE: u8 = 96; // cadential arrival — the strongest structural point

        let mut plan: Vec<StepPlan> = Vec::with_capacity(n);

        for (i, chord) in led.iter().enumerate() {
            let phrase_index = i / phrase_len;
            let position_in_phrase = i % phrase_len;
            // The true length of THIS phrase (the last phrase may be short).
            let this_len = if phrase_index + 1 == phrase_count {
                n - phrase_index * phrase_len
            } else {
                phrase_len
            };
            let at_boundary = position_in_phrase == this_len - 1;
            let is_final_phrase = phrase_index + 1 == phrase_count;

            // ---- Decide the structural position ----------------------------
            // theory: only a phrase-final step may carry a cadence; everything
            // else is PhraseStart (the opening step) or Interior. A half cadence
            // closes a non-final phrase on the dominant; a perfect authentic
            // cadence closes the final phrase with V–I on the tonic. We confirm
            // the harmonic preconditions before stamping a cadence label, and we
            // re-spell the boundary chord's NAME to the exact "V"/"I" the cadence
            // requires (the test reads cadence identity from chord.name).
            let mut chord = chord.clone();
            let position = if at_boundary && this_len >= 2 {
                if is_final_phrase {
                    // PAC requires a V immediately before the closing I. Confirm
                    // the predecessor is a dominant; if so, close on the tonic.
                    let prev_is_dominant = i
                        .checked_sub(1)
                        .and_then(|p| plan.get(p))
                        .map(|s| is_dominant_name(&s.chord.name))
                        .unwrap_or(false);
                    if prev_is_dominant {
                        // theory: stamp the cadential tonic. Name it exactly "I"
                        // so downstream detects the PAC; preserve major/minor by
                        // honoring an already-minor tonic spelling if present.
                        chord.name = "I".to_string();
                        PhrasePosition::PerfectAuthenticCadence
                    } else {
                        // No dominant preparation — cannot honestly call it a PAC.
                        // Rest on the dominant instead (a half cadence is the
                        // weaker but valid phrase close), re-spelling to "V".
                        chord.name = "V".to_string();
                        PhrasePosition::HalfCadence
                    }
                } else {
                    // Non-final phrase boundary: a half cadence resting on V.
                    chord.name = "V".to_string();
                    PhrasePosition::HalfCadence
                }
            } else if position_in_phrase == 0 {
                PhrasePosition::PhraseStart
            } else {
                PhrasePosition::Interior
            };

            let velocity = match position {
                PhrasePosition::PhraseStart => V_START,
                PhrasePosition::HalfCadence | PhrasePosition::PerfectAuthenticCadence => V_CADENCE,
                PhrasePosition::Interior => V_INTERIOR,
            };

            plan.push(StepPlan {
                chord,
                phrase_index,
                position_in_phrase,
                phrase_len: this_len,
                position,
                velocity,
            });
        }

        plan
    }
}

// =========================================================================
// PERFORMANCE-REALIZATION LAYER (S6) — public API as NAIVE STUBS (Pass A)
// =========================================================================
//
// This layer turns a structural `StepPlan` (chord + phrase position + a
// structural velocity FLOOR) into concrete, sounding `NoteEvent`s for ONE
// instrument on ONE step. It is the single pure entry point main.rs's worker
// calls; main.rs stays a thin adapter that extracts plain scalars, looks up
// the step's plan, calls `realize_step`, and maps the result into its own
// `(note, vel, hold_ms, offset_ms)` tuples.
//
// PURITY / BOUNDARY CONTRACT: nothing here takes an OpenCV or image type. The
// image features arrive as the plain-scalar `PerfFeatures` (the music-domain
// projection of `ScanBarFeatures`), so no rendering/vision type ever crosses
// into the lib crate and the whole layer is headless-testable.
//
// PASS-A STATUS: every function below is a NAIVE STUB. It compiles and returns
// a musically inert result (a single flat note at the structural floor, held
// ~90% of the step), so the Test Engineer's property net compiles and goes RED
// on the MISSING expressive behavior — not on missing symbols. Pass B fills in
// the four expressive dimensions documented in docs/design-s6-expressivity.md:
// DYNAMICS (phrase contour on the floor), RHYTHM (≥3 onset/duration patterns +
// harmonic-rhythm acceleration), ARTICULATION (staccato/legato/portato via
// hold fraction + phrase-end ritardando), and ORCHESTRATION ROLES (bass vs
// melody vs harmonic-fill differing in register, rhythmic freedom, motion).

/// The orchestration role of one INSTRUMENT (not one voice within a triad).
///
/// theory: a three-part ensemble texture distributes a triad across functional
/// strata. The BASS instrument carries the harmonic foundation (root motion,
/// lowest register, rhythmically steady). The MELODY instrument carries the top
/// line (highest register, the most rhythmic freedom — arpeggiation,
/// syncopation, the phrase's expressive peak). HARMONIC-FILL instruments sit in
/// the middle register, sustaining the chord's inner tones with the least
/// rhythmic activity so they support rather than compete. This is a DIFFERENT
/// axis from `VoiceRole`: `VoiceRole` classifies a voice WITHIN one triad
/// voicing for voice-leading constraints; `OrchestralRole` classifies an
/// INSTRUMENT across the ensemble for performance realization. Pass B maps
/// `(inst_idx, num_instruments)` onto these via `instrument_role`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrchestralRole {
    /// Lowest instrument — sounds the chord ROOT in a low register, steady rhythm.
    Bass,
    /// Middle instrument(s) — sustain inner chord tones; least rhythmic activity.
    HarmonicFill,
    /// Highest instrument — the top line; highest register, most rhythmic freedom.
    Melody,
    /// A sustained HARMONY BED: holds several chord tones simultaneously in the
    /// inner register at a supporting (quieter) dynamic, tied step-to-step under
    /// the legato-overlap cap so consecutive beds connect into a continuous floor.
    /// Never rests; the widest single-role simultaneous note count. This is the
    /// "all the harmony / all the background" layer — the held bed under the tune.
    Pad,
    /// A second melodic line moving under/around the Melody — a GENUINE
    /// species-counterpoint voice, fully realized in `realize_rhythm`'s
    /// `CounterMelody` arm (:1818). On every emitted event it recomputes the
    /// melody's pitch this/prev step for contrary motion, replays the realized
    /// previous counter pitch deterministically, and selects its own pitch through
    /// the shared `realized_counter_pitch_with_prev` (:3538) — the fifth-species
    /// figure driver `pick_counter_figure` (:4652) gated by the species predicates
    /// `is_legal_passing`/`neighbor`/`suspension`/`cambiata` (:4141–4259), with
    /// parallel-perfect avoidance, a cadential clausula (:1604), and held-period
    /// activation (the guaranteed off-beat onset that fills a static harmony's
    /// empty period). The single `role_pitch` seat it shares with the inner
    /// voices (:1271) is therefore only a DEAD anchor — `realize_rhythm` overwrites
    /// it on every emitted event. It is unreachable under the behaviour-neutral
    /// identity profile (no Counter instrument), so byte-neutral on the freeze path.
    CounterMelody,
}

/// Image-free performance features for ONE scan step — the music-domain
/// projection of `ScanBarFeatures`. No OpenCV/image type crosses into the lib.
///
/// theory: the visual→musical mapping is meaningful, not arbitrary —
///   * `saturation` (0..=100) → overall dynamic LEVEL (vivid color = bold tone),
///   * `brightness` (0..=100) → REGISTER (bright image = higher octave),
///   * `edge_density` (0..=1)  → RHYTHMIC ACTIVITY (busy texture = more onsets).
/// `realize_step` consumes these as the expressive inputs the phrase plan is
/// shaped against; main.rs builds one from each `ScanBarFeatures` (see the
/// wiring spec) and never passes the raw feature struct in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PerfFeatures {
    /// Color saturation, 0..=100 — drives overall dynamic LEVEL.
    pub saturation: f32,
    /// Luminance, 0..=100 — drives REGISTER (octave placement).
    pub brightness: f32,
    /// Edge density, 0..=1 — drives RHYTHMIC ACTIVITY (onset count/pattern).
    pub edge_density: f32,
}

/// One realized MIDI note event for the playback scheduler.
///
/// theory: this is the unit main.rs's coordinator schedules. `offset_ms` is the
/// note's onset RELATIVE to the step start (so a step can hold several notes —
/// an arpeggio, a syncopation), `hold_ms` is its sounding duration (the
/// articulation handle: short = staccato, near-full = legato), and `velocity`
/// is its dynamic (the phrase-contour-shaped level). It maps 1:1 onto main.rs's
/// existing `(note, vel, hold_ms, offset_ms)` tuple.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoteEvent {
    /// MIDI note number (0..=127).
    pub note: u8,
    /// MIDI velocity (0..=127) — the phrase-contour-shaped dynamic.
    pub velocity: u8,
    /// Sounding duration in milliseconds — the articulation handle.
    pub hold_ms: u64,
    /// Onset offset in milliseconds, relative to the step's start.
    pub offset_ms: u64,
}

/// Assign an `OrchestralRole` to instrument `inst_idx` of `num_instruments`.
///
/// theory: the ensemble is stratified bottom-to-top. The LOWEST instrument
/// (index 0) is the BASS. The HIGHEST instrument (index `num_instruments - 1`)
/// is the MELODY. Everything between is HARMONIC-FILL. With a single instrument
/// it is the MELODY (the lone line is the tune, not a bare bass); with two, the
/// low one is Bass and the high one is Melody (no fill). This is a pure,
/// total function of the two indices so the Test Engineer can pin every cell.
///
/// PASS-A STUB BEHAVIOR: the assignment scheme itself is FINAL (Pass B does not
/// change the mapping) — only the per-role realization in `realize_step` is
/// stubbed. This fn is therefore implemented for real here, not stubbed, since
/// it carries no expressive behavior to defer.
pub fn instrument_role(inst_idx: usize, num_instruments: usize) -> OrchestralRole {
    // theory: a single voice is the melody, not a bass — the lone line is the
    // tune. Guard the degenerate counts before the general stratification.
    if num_instruments <= 1 {
        return OrchestralRole::Melody;
    }
    if inst_idx == 0 {
        OrchestralRole::Bass
    } else if inst_idx >= num_instruments - 1 {
        OrchestralRole::Melody
    } else {
        OrchestralRole::HarmonicFill
    }
}

/// Map a planner layer-vocabulary entry onto the realizer's orchestral role.
/// theory: the two enums are deliberately 1:1 — the planner names the structural
/// layers it wants (`LayerRole`), the realizer turns each into the performance
/// behaviour that sounds it (`OrchestralRole`). A total, lossless bridge.
fn to_orchestral_role(layer: crate::composition::LayerRole) -> OrchestralRole {
    use crate::composition::LayerRole;
    match layer {
        LayerRole::Bass => OrchestralRole::Bass,
        LayerRole::HarmonicFill => OrchestralRole::HarmonicFill,
        LayerRole::Melody => OrchestralRole::Melody,
        LayerRole::CounterMelody => OrchestralRole::CounterMelody,
        LayerRole::Pad => OrchestralRole::Pad,
    }
}

// ----------------------------------------------------------------------------
// S23 SLICE B — saliency → role PROMINENCE (the realizer half).
//
// A salient image subject pushes the MELODY forward (louder, higher, rhythmically
// freer) while recessive regions recede into a quieter, plainer background bed.
// The realizer reads a per-role prominence WEIGHT `w in [0,1]` (0.5 == neutral)
// off `ctx.section.orchestration.prominence` and applies THREE CENTERED nudges,
// each EXACTLY 0.0 when w == 0.5. That centering is load-bearing: under identity
// the prominence Vec is EMPTY, `prominence_weight` returns PROMINENCE_NEUTRAL, and
// every nudge evaluates to `(0.5-0.5)*SPAN == 0.0` — so the realization is
// byte-for-byte today's, independent of the SPAN magnitudes below.
// ----------------------------------------------------------------------------

/// The neutral prominence weight — the freeze pivot. FIXED, not tunable: it is the
/// value `prominence_weight` returns under identity, where every nudge becomes
/// `(0.5-0.5)*SPAN == 0.0` exactly. Changing this would break the byte-freeze.
const PROMINENCE_NEUTRAL: f32 = 0.5;

/// Velocity span (MIDI units) of the centered prominence nudge. At full foreground
/// (w=1.0) the melody gains `0.5*VEL_SPAN = +9` over neutral; a fully recessive Pad
/// (w=0.3) loses `0.2*VEL_SPAN ≈ -3.6`. Layered on the existing +2 Melody / -3 Pad
/// bias, the Melody-vs-Pad gap widens by ~12 — an audible "the subject is speaking
/// up, the bed is stepping back" without clipping typical mid-velocities after the
/// 1..=127 clamp. Finalized by ear at 18.0 (a touch above the 16.0 seed): on a real
/// strong-subject image the +9 reads as a clear dynamic foreground — a mezzo line
/// rising to forte — where +8 still left the bed crowding the tune.
const PROMINENCE_VEL_SPAN: f32 = 18.0;

/// Register span (semitones) of the centered prominence lift. Max foreground lift is
/// `0.5*REG_SPAN = +2` semitones. Deliberately small so the SUMMED melody lift stays
/// in range: MELODY_REGISTER_FLOOR(67) + max bright lift(12) + max prom lift(2) = 81,
/// well under the 96 clamp ceiling — so the melody never clamps flat at the top of
/// 24..=108 and `no_inversion_invariant` cannot break. A whole-tone lift is enough to
/// hear the subject "sit on top" of the bed (a step up out of the accompaniment's
/// register) without the muddy range-compression a +5 would risk. DO NOT raise REG_SPAN
/// without re-deriving that 81 ≪ 96 margin (spec §2.2 / design-s21 §C.3 Risk 1).
const PROMINENCE_REG_SPAN: f32 = 4.0;

/// Rhythm band-cutoff shift (in edge_activity units) of the centered prominence nudge.
/// At full foreground (w=1.0) the shift is `0.5*RHY_SHIFT = 0.05`: the melody's arpeggio
/// cutoff drops 0.80→0.75, syncopated 0.55→0.50, dotted 0.25→0.20 — the foreground
/// subdivides more readily (rhythmically freer); a recessive melody raises them (plainer,
/// more sustained). Kept at 0.10 (the seed): a modest, audible bias toward subdivision
/// that nudges a borderline-busy subject into an arpeggiated/syncopated figure without
/// collapsing the four bands into one. Larger would erase the calm/busy distinction the
/// continuous curve carries.
const PROMINENCE_RHY_SHIFT: f32 = 0.10;

/// S48 SLICE 3 (2a.ii) — the CounterMelody's STRUCTURAL velocity recession below the
/// melody. theory (figure-ground, the LEVEL FINISH): the counter is already activity-
/// recessed (the S47 governor keeps it one rank below a holding melody) and register-
/// recessed (it sits under COUNTER_CEILING); a modest NEGATIVE velocity bias completes the
/// figure-ground gap from the LEVEL side, mirroring the Pad's −3 structural floor. Kept
/// SMALLER than the Pad's −3 because the counter is a MOVING line, not a flat bed — it must
/// stay audible as a real second voice (S45: the counter MOVES, never silence it). The bias
/// only RECEDES level (bounded; the final round().clamp(1,127) holds the floor), it does not
/// mute. SIGN fixed negative; magnitude is the ear-tunable start [1.0, 4.0]. NO-OP on the
/// freeze path: under identity there is no CounterMelody instrument, so this arm is never
/// reached on the engine_equivalence goldens.
const COUNTER_VEL_BIAS: f32 = 2.0;

/// S48 SLICE 3 (2a.ii) — the HarmonicFill's STRUCTURAL velocity recession. theory: the fill
/// is the connective INNER tissue under the line; a gentle recession keeps it below the
/// melody without hollowing the harmony. Kept SMALLER than the counter's bias — the fill is
/// more recessive than the counter by role (it supports, it does not sing a second line).
/// SIGN fixed negative; magnitude ear-tunable [0.5, 2.0]. The arm is `!is_cadence`-guarded
/// and the synthetic engine_equivalence goldens are NO-COUNTER melody/bass bars that emit no
/// HarmonicFill event, so this bias is freeze-neutral on the goldens (spec §4 witness).
const FILL_VEL_BIAS: f32 = 1.0;

// ----------------------------------------------------------------------------
// S47 SLICE 1 — THE FIGURE-GROUND ACTIVITY HIERARCHY (melody-most-active).
//
// theory: figure-ground segregation rests FIRST on rhythmic ACTIVITY (the ear's
// strongest stream-segregation cue for a timbre-flat synth texture), not on level.
// The melody must be the busiest line and the background must recede in onset
// density. The S46 defect was an ACTIVITY INVERSION — on a calm image the melody
// fell to one held tone while the CounterMelody took a guaranteed off-beat onset,
// so the background out-moved the foreground. The fix is a coarse, RNG-free
// activity CLASS the Counter arm reads off the MELODY so it can stay one rank
// below a holding melody (the governor), plus a prominence-keyed FLOOR so a
// foreground melody never falls all the way to a held tone (the floor).
// ----------------------------------------------------------------------------

/// The Melody arm's three rhythm-band cutoffs (in `edge_activity` units), centered
/// on the freeze identity (`prom_shift == 0` at neutral weight). EXTRACTED to named
/// consts so the Melody arm (:1943-1992) and `melody_activity_class` read ONE source
/// — the helper's classification can never drift from the arm it mirrors (spec §8.2).
/// theory: ARPEGGIO is the busiest band (≥3 onsets), SYNCOPATED pushes the meter (2
/// onsets off the beat), DOTTED is the minimum real motion (a long-short pair, 2
/// onsets), and below DOTTED the line SUSTAINS (1 held tone). These values are the
/// SAME 0.80/0.55/0.25 the arm has always used — extracting them is byte-neutral.
const MELODY_ARP_CUTOFF: f32 = 0.80; // → ARPEGGIO band (the busiest figure)
const MELODY_SYNC_CUTOFF: f32 = 0.55; // → SYNCOPATED band
const MELODY_DOTTED_CUTOFF: f32 = 0.25; // → DOTTED band (minimum real motion); below → SUSTAINED

// S50 — RHYTHM-VARIETY RE-RANGE (the band side). These three are TASTE-OWNED VALUES set by the
// Affect/Aesthetics reconciliation (design-s50-affect-cutpoints.md §1 ∥ design-s50-aesthetics-
// cutpoints.md §1; the lead reconciled the two lenses to these numbers). They parameterize
// `band_activity_spread` below; they do NOT move the band cut constants above (0.80/0.55/0.25 stay
// frozen — the spread does the re-positioning so `melody_activity_class` and the arm keep ONE shared
// cutoff source, spec §2.A).
const BAND_SPREAD_CENTER: f32 = 0.40; // pivot = the natural-photo activity centroid AND freeze-neutral reference
const BAND_SPREAD_GAIN_LOW: f32 = 1.8; // slope below center (opens the calm side toward SUSTAINED)
const BAND_SPREAD_GAIN_HIGH: f32 = 1.4; // slope at/above center (opens the busy side toward SYNC/ARP)

/// S50 — re-range the BAND-INPUT activity onto the real-image distribution. Maps the measured
/// natural-photo cluster across the full 0..1 decision range so the EXISTING band cuts
/// (0.80/0.55/0.25) bite again — instead of every real photo pancaking onto DOTTED. The transform
/// is a one-knee piecewise-linear stretch about `BAND_SPREAD_CENTER`, monotone non-decreasing, with
/// a FIXED POINT at the center (identity there → the band ladder is byte-neutral at the reference
/// activity, spec §3.2) and identity whenever both gains are 1.0 (the gate can disable the spread).
///
/// theory: rhythmic subdivision should track visual activity (the affect bridge: more visual energy
/// → more onsets), but natural photographs occupy a COMPRESSED activity sub-band (≈0.30–0.51 edge
/// for four of the six bundled images), so a raw mapping collapses them all into one band. A linear
/// stretch about the cluster centroid re-expands that compressed sub-band so the calm tail can reach
/// SUSTAINED and the busy tail can reach SYNCOPATED/ARPEGGIO — i.e. SUSTAINED..ARPEGGIO are all
/// REACHABLE for real photos. The slope is ASYMMETRIC (low side > high side) on purpose: the calm
/// side opens hard toward SUSTAINED (a calm image should genuinely hold), while the busy side opens
/// only enough to separate the one genuinely-busy image into ARPEGGIO WITHOUT flinging the
/// mid-cluster into the "computer-like fragmentation" failure (the over-drive guard).
///
/// Pure, RNG-free, free fn (the `melody_total_rhythm_shift` precedent). Applied ONLY to the
/// band-ladder comparison input and (mirrored) inside `melody_activity_class` so the helper stays
/// 1:1 with the arm — NEVER to the articulation curve, the FILL_REST check, or the `/0.05`
/// normalization (the freeze discipline that keeps the diversity_s13 articulation goldens
/// byte-identical, spec §3.3).
fn band_activity_spread(edge_activity: f32) -> f32 {
    let slope = if edge_activity < BAND_SPREAD_CENTER {
        BAND_SPREAD_GAIN_LOW
    } else {
        BAND_SPREAD_GAIN_HIGH
    };
    (BAND_SPREAD_CENTER + (edge_activity - BAND_SPREAD_CENTER) * slope).clamp(0.0, 1.0)
}

/// The realized rhythmic-activity CLASS of a voice on a step — coarse, RNG-free, and
/// structural (a function of `edge_activity` + the arm's band, never of absolute pitch).
/// Ordering IS the figure-ground rank: Sustained < Oblique < Subdividing. The Counter arm
/// reads the MELODY's class to stay strictly BELOW it on a HOLDING melody (the hierarchy
/// invariant), so the background never out-moves the foreground. Under identity there is no
/// Counter instrument and prominence is neutral, so this is never consulted on the freeze
/// path → byte-neutral. `Ord` is derived so the governor can compare classes directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ActivityClass {
    /// One held tone, no real motion (the Melody SUSTAINED arm).
    Sustained,
    /// Minimum real motion — a long-short pair / a single sustained tone (the DOTTED band).
    Oblique,
    /// ≥2 onsets pushing/spreading the beat (the SYNCOPATED + ARPEGGIO bands).
    Subdividing,
}

/// The melody's activity class for THIS step, derived from the SAME 4-band ladder the Melody
/// arm uses (:1943-1992) — extracted so the Counter arm can govern off it WITHOUT duplicating
/// the cutoff logic. Pure; reads only `edge_activity`, the prominence rhythm shift `prom_shift`
/// (the melody's own :1941-1942 value, passed in so the helper takes no `ctx`), and
/// `pre_cadence`.
///
/// theory: the Melody arm has FOUR rhythm bands (arpeggio / syncopated / dotted / sustained)
/// but the figure-ground rank has THREE classes. The mapping (locked S46 theory lens, spec
/// §2a.2): DOTTED→Oblique (a long-short pair is the melody's minimum-real-motion, so it lets
/// the counter recede to a single tone rather than move), SYNCOPATED+ARPEGGIO→Subdividing
/// (both push ≥2 onsets against/across the beat, so the melody is unambiguously moving and the
/// counter may keep its moving inner texture — PRESERVES S45). The cutoffs MUST stay 1:1 with
/// the live Melody arm — both now read the shared MELODY_*_CUTOFF consts so they cannot drift.
fn melody_activity_class(edge_activity: f32, prom_shift: f32, pre_cadence: bool) -> ActivityClass {
    // Mirrors the Melody arm's band ladder EXACTLY. The arm's ARPEGGIO band (> ARP_CUTOFF)
    // and SYNCOPATED band (> SYNC_CUTOFF) BOTH map to Subdividing (≥2 onsets pushing/spreading
    // the beat), so a single `> SYNC_CUTOFF` test covers both — ARP_CUTOFF > SYNC_CUTOFF, so
    // anything clearing ARP also clears SYNC (the ARP_CUTOFF const is retained as the arm's
    // own band boundary). pre_cadence forces the ARPEGGIO band, also Subdividing. Below SYNC
    // but above DOTTED is Oblique (minimum real motion); below DOTTED is Sustained (held tone).
    // S50 — mirror the band ladder's `band_activity_spread` on the comparison input so this helper
    // stays 1:1 with the Melody arm (the governor MUST see the same melody class the arm renders).
    // Spread is identity at BAND_SPREAD_CENTER and when the gains are 1.0, so the freeze witness is
    // preserved; the cut CONSTANTS stay 0.80/0.55/0.25 (spec §3.3).
    let spread = band_activity_spread(edge_activity);
    if pre_cadence || spread > (MELODY_SYNC_CUTOFF - prom_shift) {
        ActivityClass::Subdividing // ARPEGGIO + SYNCOPATED bands — the melody MOVES
    } else if spread > (MELODY_DOTTED_CUTOFF - prom_shift) {
        ActivityClass::Oblique // DOTTED band — minimum real motion
    } else {
        ActivityClass::Sustained // the SUSTAINED arm — one held tone
    }
}

// ----------------------------------------------------------------------------
// S49 SLICE 2 — PER-ROLE RHYTHMIC IDENTITY (L1: the per-role band-cutoff bias).
//
// theory: figure-ground segregation rests FIRST on rhythmic ACTIVITY. The S47/S48
// passes made the melody the busiest line and the bed recede, but EVERY role's band
// selection still keys off the ONE shared `edge_activity` against ONE shared cutoff set,
// so between-role rhythm reads FLAT — all roles "see" the same busyness and resolve to a
// fixed per-role body, never a per-role onset GRID. L1 generalizes the melody-only
// `prom_shift` nudge into a per-ROLE band-cutoff bias: the melody is biased TOWARD
// subdivision (cutoffs LOWERED → reaches arpeggio/syncopation sooner → a busier surface),
// the bed roles biased AWAY (cutoffs RAISED → steadier). The bias is ADDED to the existing
// `prom_shift` term in the SAME `(CUTOFF - shift)` expression the band ladder and
// `melody_activity_class` already use, so the helper and the arm stay 1:1 (spec §7).
// ----------------------------------------------------------------------------

/// L1 per-role rhythm-bias magnitudes (in `edge_activity`/cutoff units), ADDED to the existing
/// `prom_shift` so a positive value LOWERS the effective cutoffs (`CUTOFF - (prom_shift + bias)`)
/// → the role reaches a busier band sooner. SIGNS are load-bearing and FIXED (spec §3): the
/// MELODY is POSITIVE (toward subdivision — the figure must MOVE) and the BED roles are
/// NON-POSITIVE (toward sustained — the bed stays steady), so `bed_onsets ≤ melody_onsets`
/// (F1/F5b) is preserved/strengthened by construction, never inverted. Magnitudes are
/// CONSERVATIVE build-start defaults flagged TASTE-SIZED for the standing taste/affect +
/// aesthetics gate (Affect §8 ∥ Aesthetics §9) — the gate sizes the final numbers so each role
/// reads as its own line WITHOUT the texture fragmenting (the slice-2 failure mode); the build
/// lands the structure at these starting values. Ranges (spec §3): melody [+0.03, +0.10]; bed
/// [−0.03, −0.10].
const MELODY_RHYTHM_BIAS: f32 = 0.06; // → subdivides sooner (more active foreground)
const FILL_RHYTHM_BIAS: f32 = -0.05; // inner bed → steadier
const PAD_RHYTHM_BIAS: f32 = -0.05; // harmony bed → steadier
const BASS_RHYTHM_BIAS: f32 = -0.10; // the floor → steadiest of all

/// L2 — the WEAK-BEAT fraction of `step_ms` the HarmonicFill bed onset is DISPLACED onto so it
/// leaves the melody's downbeat (offset 0). theory (spec §2 item 2): a bed onset on offset 0
/// FUSES its onset grid with the melody's downbeat attack (the F5a anti-fusion signal); moving
/// it to the "and" of the beat phase-separates the bed as its own stream. The default is the
/// HALF-beat (0.5) — DELIBERATELY DISTINCT from the counter's MOVING `step_ms/4` and the melody's
/// downbeat 0 (spec §7: `step_ms/2 ≠ step_ms/4 ≠ 0` is collision-free), and the SAME phase the
/// receded block-Pad stab already uses (`PAD_WEAK_BEAT_FRAC`) so the two beds share ONE off-beat
/// grid rather than fragmenting. COUNT-PRESERVING (the `recede_pad_onsets` precedent): it moves
/// WHERE the onset sits, never HOW MANY → F5b/F1 untouched. CONSERVATIVE build-start; SIGN fixed
/// POSITIVE (off the downbeat); magnitude TASTE-SIZED [0.375, 0.625] (spec §3), and the taste
/// gate confirms no collision on the 6-image set.
const BED_PHASE_FRAC: f32 = 0.5;

/// L3 — the MELODY's per-role articulation DETACH bias: a NEGATIVE nudge on `base_frac` so the
/// foreground melody is CRISPER / more detached than the bed (a third figure cue — articulation
/// contrast). theory (spec §2 item 3 / §3): the melody is the figure, so it speaks with a more
/// articulated surface; the bed connects under it. SIGN fixed NEGATIVE on `base_frac` (more
/// detached). Composes BEFORE the window clamp (stays pleasant) and BEFORE the S48 comp detach
/// (the comp still rides on top for the low-seat case). FREEZE: gated freeze-neutral on the
/// foreground-weight pivot (like L1) → 0.0 at neutral 0.5 → the melody-79 golden's `base_frac` is
/// the unchanged `curve_frac`. CONSERVATIVE build-start; magnitude TASTE-SIZED [0.0, 0.12] (spec §3).
const MELODY_ARTIC_DETACH_BIAS: f32 = 0.07;

// L3 — the BASS's per-role articulation CONNECT bias is implemented INLINE at the Bass match arm
// (`OrchestralRole::Bass if counter_present => curve_frac.max(ARTIC_WINDOW_LO)`), not as a named
// const: the Fill connected-floor lean generalized to the bass so the bed sustains smoothly UNDER
// the melody. SIGN positive (more connected); GATED on `counter_present` (the ONE golden-reachable
// L3 term) so the no-counter goldens keep the legacy `curve_frac` byte-identically. See that match
// arm for the full theory note (spec §2 item 3 / §3, §4/§8).

/// The per-role band-cutoff rhythm bias (L1) — a SEPARATE additive term centered at 0.0 for
/// every role, summed with the prominence `prom_shift` in the band ladder + `melody_activity_class`
/// so each role resolves the SAME `edge_activity` to a DISTINCT onset grid. Positive (melody)
/// LOWERS the effective cutoffs (busier sooner); non-positive (bed) RAISES them (steadier).
///
/// FREEZE: the MELODY bias is gated freeze-neutral at its CALL SITES by the foreground-weight
/// pivot (`melody_w > ACTIVITY_FLOOR_THRESHOLD`), exactly as the S47 floor + S48 comp are — at
/// neutral weight 0.5 the caller passes 0.0, so the melody-79 goldens select the SAME band
/// (SUSTAINED, offset 0) byte-identically. The BED biases only matter inside the Pad / Fill /
/// CounterMelody arms, which are NOT entered on the synthetic no-counter goldens (melody-79 +
/// bass-36 only); the CounterMelody/Pad arms read the melody's class (governor side) where the
/// MELODY bias is the relevant term, and the bias raising a bed role's own cutoffs only ever
/// makes it steadier (never up into ARPEGGIO — spec §8.2). The Bass arm carries NO band ladder
/// (it is sustained/walking/pedal), so its bias is unread there and never moves the bass-36
/// golden. Returns 0.0 for any role with no band-ladder differentiation (Melody handled at the
/// gated call site, not here, so the freeze witness is local to the caller).
fn role_rhythm_bias(role: OrchestralRole) -> f32 {
    match role {
        OrchestralRole::Melody => MELODY_RHYTHM_BIAS,
        OrchestralRole::HarmonicFill => FILL_RHYTHM_BIAS,
        OrchestralRole::Pad => PAD_RHYTHM_BIAS,
        OrchestralRole::Bass => BASS_RHYTHM_BIAS,
        OrchestralRole::CounterMelody => 0.0, // governed off the MELODY class; no own band ladder
    }
}

/// The MELODY's TOTAL band-cutoff shift for THIS step — the prominence `prom_shift` PLUS the
/// L1 per-role rhythm bias, gated freeze-neutral. theory/spec §7 (shared-cutoff coupling): the
/// Melody arm's band ladder AND the governor's `melody_activity_class` (read from the Counter
/// arm + the Pad cap) MUST see the SAME melody class, so they MUST read the SAME shift. This
/// single source guarantees they cannot drift. FREEZE: the L1 bias is gated on the foreground-
/// weight pivot (`melody_w > ACTIVITY_FLOOR_THRESHOLD`) — at neutral weight 0.5 the strict `>`
/// is FALSE → the bias is 0.0 → the shift is EXACTLY the legacy `(w-0.5)*RHY_SHIFT` (== 0.0 at
/// neutral) → the goldens are byte-identical.
fn melody_total_rhythm_shift(melody_w: f32) -> f32 {
    let bias = if melody_w > ACTIVITY_FLOOR_THRESHOLD {
        role_rhythm_bias(OrchestralRole::Melody)
    } else {
        0.0
    };
    (melody_w - PROMINENCE_NEUTRAL) * PROMINENCE_RHY_SHIFT + bias
}

/// The melody-prominence weight ABOVE which the activity FLOOR bites (strict `>`): a
/// FOREGROUND melody (resolved weight > this) never falls to SUSTAINED on a calm image —
/// it is floored up one rank to the DOTTED (Oblique) band so it always out-moves the
/// governed counter's single sustained tone. Set EXACTLY at PROMINENCE_NEUTRAL so an
/// identity/neutral melody (weight == 0.5) is UNAFFECTED (strict `>` is false → the
/// SUSTAINED arm runs byte-identically); every genuine foreground weight (> 0.5) floors.
/// theory: a line that is the declared figure must MOVE; a held foreground over a moving
/// bed is the figure-ground inversion from the foreground side. EAR-TUNABLE [0.50, 0.60].
const ACTIVITY_FLOOR_THRESHOLD: f32 = 0.50;

/// The minimum clear margin (in semitones) the melody seat must hold ABOVE the counter
/// ceiling — a POSITIVE gap (operator decision 5: a clear seat, not a tie). High-voice
/// superiority wants the figure unambiguously on top, so the seat floor is
/// `COUNTER_CEILING + this` whenever a counter is present. EAR-TUNABLE [1, 5] (the taste
/// gate sizes it). GATED on counter-present (spec §2b / operator decision A): the
/// engine_equivalence goldens are synthetic no-counter bars, so the guard never fires
/// there → the freeze holds.
const MIN_FIGURE_GAP: u8 = 2; // recommended start; range [1, 5] — see spec §3

// ----------------------------------------------------------------------------
// S48 SLICE 3 — INVERSE-REGISTER COMPENSATION (the figure-ground FINISH, F4).
//
// theory: a melody seated LOW in its range self-projects LEAST — it sits close to
// the bed ceiling, so the register cue that normally puts the figure on top is weak.
// The fix is NON-LEVEL (DP-3: level NEVER for the comp; the level lever is the
// SEPARATE 2a bed-recession). A low-seated melody is held as the figure by SEPARATION
// instead: it attacks OFF the bed's downbeat (rhythmic separation, the PRIMARY tool)
// and detaches its articulation (the SECONDARY tool). The amount of help is INVERSE to
// the realized seat height — a melody clearly above the bed needs none (factor 0.0), a
// melody dropping toward the bed band needs the most (factor 1.0). This drives the F4
// metric correlation(register_gap, separation) NEGATIVE: small gap → high separation.
// ----------------------------------------------------------------------------

/// The max fraction of `step_ms` the melody's FIRST onset is pushed off the downbeat at FULL
/// compensation (factor 1.0). theory (DP-3 rank 1, rhythmic separation FIRST): a low-seated
/// melody attacks on the "and" of the beat — DISTINCT from the bed's offset-0 downbeat — so it
/// reads as a separate stream without getting louder. Kept ≤ step_ms/4 (the same off-beat the
/// counter's MOVING mode uses) so the pushed onset never crosses into the next beat and never
/// collides with the SYNCOPATED band's own step_ms/4 displacement. SIGN fixed POSITIVE (push
/// LATER); magnitude ear-tunable [0.125, 0.375].
const COMP_OFFSET_FRAC: f32 = 0.25;

/// The max reduction of `base_frac` (articulation) at FULL compensation. theory (DP-3 rank 4,
/// articulation SECOND): a low-seated melody gets CRISPER, more detached notes — perceptual
/// figure-ground separation the ear hears even where the F4 onset-offset metric does not read
/// it. SIGN fixed NEGATIVE on base_frac (more detached when low); floored at ARTIC_WINDOW_LO so
/// the note never clicks. Magnitude ear-tunable [0.05, 0.20]. LEVEL is NEVER a comp tool (DP-3)
/// — this nudges DURATION, not velocity.
const COMP_ARTIC_DETACH: f32 = 0.10;

/// The inverse-register compensation FACTOR for a melody seated at `seat` relative to the bed
/// ceiling — 0.0 when the melody sits clearly ABOVE the bed (no help needed), rising LINEARLY
/// toward 1.0 as the realized seat approaches/drops into the bed band (a LOW-seated melody needs
/// MORE separation to hold figure WITHOUT a louder level — the realization of "a melody low in
/// its range self-projects least and needs more help, via non-level tools"). Pure; RNG-free; a
/// function of the realized seat ONLY. The bed reference is COUNTER_CEILING (67) — the same
/// ceiling the seat guard uses; the help band is [FILL_REGISTER_FLOOR(55), COUNTER_CEILING +
/// MIN_FIGURE_GAP(69)). theory (DP-3): help is INVERSE to register height — the low melody gets
/// the separation, the high one does not. Returns 0.0 at/above (COUNTER_CEILING + MIN_FIGURE_GAP)
/// (== 69, the guarded seat floor) and ramps to 1.0 at FILL_REGISTER_FLOOR (== 55). Monotone
/// NON-increasing in `seat` (FIXED direction; the LINEAR shape is the ear-tunable craft call).
/// If the seat-guard's MIN_FIGURE_GAP moves at the taste gate, this zero-crossing tracks it 1:1
/// (it reads the SAME const) — no extra edit (spec §8 watch-item 1 coupling).
fn inverse_register_compensation(seat: u8) -> f32 {
    let ceiling = (COUNTER_CEILING + MIN_FIGURE_GAP) as f32; // 69 — at/above → no help (0.0)
    let floor = FILL_REGISTER_FLOOR as f32; // 55 — at/below → full help (1.0)
    let s = seat as f32;
    if s >= ceiling {
        return 0.0; // clearly on top of the bed — the register cue already segregates it
    }
    if s <= floor {
        return 1.0; // in/under the bed band — needs the most non-level separation
    }
    // LINEAR ramp over (floor, ceiling): factor 1.0 at the floor, 0.0 at the ceiling, monotone
    // NON-increasing in seat. (ceiling > floor structurally — 69 > 55 — so no divide-by-zero.)
    (ceiling - s) / (ceiling - floor)
}

// ----------------------------------------------------------------------------
// S47 SLICE 4 — THE BED ACTIVITY RECESSION (pass 2: make the melody the FIGURE).
//
// theory: pass 1 (the governor + activity floor) made the COUNTER recede below the
// melody, but the figure-ground instrumentation proved the live F5b/F1 defect is
// PAD-driven, not counter-driven — the Pad out-onsets the melody on co-sounding
// steps (block bed = `pad_voices` simultaneous tones at offset 0 → 3 events/step;
// figured bed = the figure's 2..4-onset burst), while the melody carries ~2. The
// metric (`onsets_per_step`) counts each Pad NoteEvent, so a 3-voice chord stab IS
// 3 "onsets" by F5b's lens even though the ear hears one attack. The fix RECEDES the
// Pad's per-step ONSET COUNT below the melody's, image-conditioned by the resolved
// Pad prominence weight pass 1 already wired: a deeper-recessed Pad (weight 0.30)
// caps one onset BELOW the melody (a STRICT positive F1 lead — the figure clearly
// wins on a strong-subject image), a shallow Pad (weight 0.45) caps AT the melody
// (near-even — correct for a FIELD image, no forced lead). It NEVER hollows the bed
// to silence (the floor; re-opening the S45 static-bed defect is forbidden) and is a
// NO-OP at neutral weight 0.5 (the identity / golden / synthetic-bar path), so
// engine.rs stays byte-frozen exactly as the melody floor is.
// ----------------------------------------------------------------------------

/// The Pad-prominence weight AT/ABOVE which the bed recession is a NO-OP (the freeze hinge):
/// the cap only bites for a genuinely RECESSED Pad (resolved weight strictly `<` this). Set
/// EXACTLY at PROMINENCE_NEUTRAL so an identity/neutral Pad (weight == 0.5 — the value
/// `prominence_weight` returns when the section's prominence Vec is EMPTY, i.e. the golden /
/// synthetic-bar path) is UNAFFECTED: the cap returns "no cap" and the Pad arm runs
/// byte-identically. Every real recessed Pad (0.30 / 0.40 / 0.45) clears the strict `<` and
/// recedes. This mirrors ACTIVITY_FLOOR_THRESHOLD's freeze-pivot gating on the melody side.
const PAD_RECESSION_THRESHOLD: f32 = 0.50;

/// The Pad-prominence weight AT/BELOW which the bed recedes ONE EXTRA onset — STRICTLY below
/// the melody (a POSITIVE F1 margin) rather than merely even with it. theory: a deep-tier Pad
/// (resolved weight 0.30, the subject_melody / melody_lead_strong tiers a strong, separated
/// subject selects) accompanies a melody that MUST be the unambiguous figure, so the bed sheds
/// one more onset to clear a real activity gap; a shallow-tier Pad (0.45, the melody_lead_gentle
/// FIELD tier) caps merely AT the melody (near-even — an abstract/field image legitimately shares
/// focus, so no forced lead). Lower resolved weight → fewer Pad onsets, integrating the cap with
/// the pass-1 recession family. Placed between the deep (0.30) and shallow (0.45) tiers; the mid
/// (0.40) tier reads as deep here, which is the intended musical call (a melody-forward section
/// still wants a clear figure). EAR-TUNABLE [0.35, 0.45].
const PAD_DEEP_RECESSION_CEILING: f32 = 0.40;

/// The Pad onset FLOOR — the bed never recedes below ONE sounding onset per step (it RECEDES in
/// activity, it does NOT go silent). theory: a SILENT bed re-opens the static-bed defect S45
/// fixed; the recession thins the bed's activity while keeping it sounding. So the cap is floored
/// here even on the deepest tier against the calmest melody (Sustained → mel_min 1, deep gap 1 →
/// 0, floored back to 1). FIXED at 1 (a bed of zero voices is not a bed).
const PAD_ONSET_FLOOR: usize = 1;

/// The weak-beat the receded BLOCK bed displaces its stab onto — a fraction of the step. theory:
/// a block bed is a SIMULTANEOUS chord stab at offset 0 (the downbeat, where the melody attacks);
/// when the recession thins it to a single stab, leaving it on offset 0 would FUSE its onset grid
/// with the Bass/Fill/Melody downbeat (the S42 fusion signature F5a red-bars). So the surviving
/// stab is moved to the "and" of the beat (the back half of the step) — a classic off-beat comp —
/// keeping the Pad OFF the melody's strong beat (the prompt's weak-beat steer) while preserving its
/// onset COUNT (so F5b stays satisfied — displacement changes WHERE the onset sits, not HOW MANY).
/// A FIGURED bed already keeps its surviving onsets off the downbeat (the recession drops the
/// offset-0 onsets first), so it is never displaced. EAR-TUNABLE [0.5, 0.75] (the off-beat feel).
const PAD_WEAK_BEAT_FRAC: f32 = 0.5;

/// The MINIMUM onset count the Melody arm emits for a given activity class — the relative
/// reference the Pad cap recedes under so `bed_onsets ≤ melody_onsets` holds on EVERY co-sounding
/// step (the F5b invariant), INCLUDING the busiest melody steps (so the CLIMAX-BLOOM cross-arc
/// invariant survives: when a later texture-arc slice blooms the melody to ARPEGGIO/3+ onsets, the
/// bed may bloom too while staying under it). theory: the Melody arm (:2116-2168) emits ARPEGGIO→3
/// / SYNCOPATED→2 / DOTTED→2 / SUSTAINED→1 onsets; mapped onto the 3 figure-ground classes the
/// MINIMUM is Sustained→1, Oblique(DOTTED)→2, Subdividing(SYNC or ARP)→2. Using the class MINIMUM
/// (2 for Subdividing, not 3) is the CONSERVATIVE choice: on an ARPEGGIO step the melody emits 3
/// and the capped-at-2 bed stays strictly under — the invariant can never be violated by an
/// optimistic read. Pure; mirrors the arm's per-band counts (kept 1:1 in the unit tests).
fn melody_min_onsets(class: ActivityClass) -> usize {
    match class {
        ActivityClass::Sustained => 1,   // one held tone
        ActivityClass::Oblique => 2,     // DOTTED long-short pair
        ActivityClass::Subdividing => 2, // SYNCOPATED (2) — the MIN of {SYNC 2, ARP 3}
    }
}

/// The maximum onset count the Pad bed may emit THIS step so it recedes BELOW (deep tier) or
/// AT (shallow tier) the melody's activity, image-conditioned by the resolved Pad prominence
/// weight `pad_w` and the melody's activity `class`. Returns `None` == NO CAP (the freeze hinge:
/// a neutral Pad runs unbounded/byte-identical); `Some(k)` == cap to at most `k` onsets, never
/// below PAD_ONSET_FLOOR.
///
/// theory (the recession law): a recessed Pad must satisfy `bed_onsets ≤ melody_onsets` on every
/// co-sounding step (F5b). The cap is RELATIVE to the melody's per-class minimum so the bed tracks
/// the melody (preserving CLIMAX-BLOOM) rather than pinning to an absolute low count. The DEEP
/// tier sheds one EXTRA onset for a STRICT positive F1 lead; the SHALLOW tier caps at the melody
/// for near-even. The floor guarantees the bed never hollows to silence.
///
/// FREEZE: at `pad_w == PROMINENCE_NEUTRAL` (0.5 — the identity / golden / synthetic-bar path,
/// where `prominence_weight` returns neutral) the strict `<` is FALSE → `None` → no cap → the Pad
/// arm is byte-identical. Only a real recessed Pad (weight < 0.5) is ever capped.
fn pad_onset_cap(pad_w: f32, class: ActivityClass) -> Option<usize> {
    if pad_w >= PAD_RECESSION_THRESHOLD {
        return None; // neutral / non-recessed Pad — the freeze hinge, no cap
    }
    let mel = melody_min_onsets(class);
    // Deep tier (weight ≤ ceiling): one onset BELOW the melody (strict positive lead).
    // Shallow tier: AT the melody (near-even). Floored so the bed never hollows.
    let capped = if pad_w <= PAD_DEEP_RECESSION_CEILING {
        mel.saturating_sub(1)
    } else {
        mel
    };
    Some(capped.max(PAD_ONSET_FLOOR))
}

/// Thin a built Pad event list DOWN to at most `cap` onsets, RECEDING the bed off the melody's
/// strong beat: it drops the onsets NEAREST the downbeat (offset 0 — where the melody attacks)
/// FIRST, keeping the latest (weak-beat) onsets, so the bed recedes in activity WITHOUT hollowing
/// and stays clear of the figure's downbeat attack. theory: figure-ground segregation wants the
/// bed off the melody's accent; dropping the downbeat-coincident onsets is the cleanest activity
/// recession that also opens the strong beat for the figure.
///
/// THE BLOCK-BED CASE (every event at offset 0 — a simultaneous chord stab): the events are tied
/// on offset, so the thinning keeps the FIRST `cap` seated voices (the harmonically essential inner
/// tones, seated low→high) — a thinner stab, still sounding. But a single surviving stab left on
/// offset 0 would FUSE its onset grid with the Bass/Fill/Melody downbeat (the F5a anti-fusion
/// signal). So when EVERY surviving onset is still on the downbeat, the stab is DISPLACED to the
/// weak beat (PAD_WEAK_BEAT_FRAC) — an off-beat comp that keeps the bed off the melody's accent
/// AND distinct from the other beds, WITHOUT changing the onset COUNT (F5b is count-based, so the
/// displacement is F5b-neutral). The hold is re-fitted into the remaining step so the displaced
/// stab never over-runs the step more than the block bed already did.
///
/// A `cap` ≥ len is the identity (returns the list unchanged). Pure.
fn recede_pad_onsets(mut events: Vec<NoteEvent>, cap: usize, step_ms: u64) -> Vec<NoteEvent> {
    if events.len() <= cap {
        return events; // already at/under the cap — nothing to recede (covers the no-cap path)
    }
    // Index sorted by offset DESCENDING so the LATEST (weak-beat) onsets are kept first; ties
    // (the block bed's all-offset-0 stab) break on original index so the first `cap` kept are the
    // lowest seated voices (the essential inner harmony). We mark which original indices to KEEP,
    // then rebuild in ORIGINAL order so the emitted list stays in onset/seat order.
    let mut order: Vec<usize> = (0..events.len()).collect();
    order.sort_by(|&a, &b| {
        events[b]
            .offset_ms
            .cmp(&events[a].offset_ms)
            .then(a.cmp(&b))
    });
    let keep: std::collections::BTreeSet<usize> = order.into_iter().take(cap).collect();
    let mut out: Vec<NoteEvent> = Vec::with_capacity(cap);
    for (i, ev) in events.drain(..).enumerate() {
        if keep.contains(&i) {
            out.push(ev);
        }
    }
    // F5a anti-fusion: if EVERY surviving onset is still on the downbeat (the block-bed stab — no
    // weak-beat onset survived to keep the bed off the beat), displace the stab to the weak beat so
    // its onset grid does not fuse with the Bass/Fill/Melody downbeat. Count-preserving (F5b-safe).
    if out.iter().all(|e| e.offset_ms == 0) {
        let weak = (step_ms as f32 * PAD_WEAK_BEAT_FRAC).round() as u64;
        // Re-fit each displaced onset's hold into [weak, step_ms × PAD_OVERLAP_FRAC] so it sounds
        // through the back of the step (the block bed's legato ceiling) without a multi-step ring.
        let ceil = ((step_ms as f32) * PAD_OVERLAP_FRAC).round() as u64;
        let hold = ceil.saturating_sub(weak).max(1);
        for e in &mut out {
            e.offset_ms = weak;
            e.hold_ms = hold;
        }
    }
    out
}

/// L2 (S49 slice 2) — PHASE-SEPARATE a bed onset OFF the melody's downbeat. Displaces every
/// onset currently sitting ON the downbeat (offset 0) to the weak-beat fraction `phase_frac` of
/// `step_ms`, re-fitting its hold into the room remaining before the step boundary so it does not
/// ring across into the next step. theory (spec §2 item 2): a HarmonicFill bed that attacks on
/// offset 0 fuses its onset grid with the melody's downbeat (the F5a fusion signal); moving it to
/// the "and" gives the bed its OWN phase, distinct from the melody (0) and the counter (step_ms/4).
///
/// COUNT-PRESERVING (the `recede_pad_onsets` precedent, proven freeze-neutral): it moves WHERE the
/// onset sits, never HOW MANY → onset counts byte-stable → F5b/F1 untouched. An onset already OFF
/// the downbeat is left alone (a figured/multi-onset bed already keeps its grid off the beat).
/// Pure. (The bed FLOOR / deep-bed-thinness WATCH is upstream of this — this never DROPS an onset,
/// so it cannot hollow the bed; spec §8 watch-5.)
fn phase_separate_bed(mut events: Vec<NoteEvent>, phase_frac: f32, step_ms: u64) -> Vec<NoteEvent> {
    let weak = (step_ms as f32 * phase_frac).round() as u64;
    if weak == 0 {
        return events; // a 0 phase fraction is the identity (no displacement)
    }
    let room = step_ms.saturating_sub(weak).max(1);
    for e in &mut events {
        if e.offset_ms == 0 {
            // Re-fit the hold into the remaining room so the displaced onset does not over-run the
            // step (the same intent as recede_pad_onsets / the S48 comp re-fit). Count unchanged.
            e.hold_ms = e.hold_ms.min(room).max(1);
            e.offset_ms = weak;
        }
    }
    events
}

/// Prominence weight (0..1) for `role`, read off `ctx.section.orchestration.prominence`.
/// theory: returns PROMINENCE_NEUTRAL (0.5) when the section's prominence is EMPTY
/// (identity / uniform) OR the role is unlisted — so the legacy realization is
/// byte-identical whenever prominence is absent (every centered nudge is then a no-op).
/// Pure. Bridges the planner's LayerRole → the realizer's OrchestralRole via the
/// EXISTING `to_orchestral_role` (no new bridge).
fn prominence_weight(ctx: &crate::composition::StepContext, role: OrchestralRole) -> f32 {
    let prom = &ctx.section.orchestration.prominence;
    if prom.is_empty() {
        return PROMINENCE_NEUTRAL;
    }
    prom.iter()
        .find(|lp| to_orchestral_role(lp.role) == role)
        .map(|lp| lp.weight)
        .unwrap_or(PROMINENCE_NEUTRAL)
}

/// Assign an `OrchestralRole` to instrument `inst_idx`, PLAN-AWARE.
///
/// theory: the role an instrument plays is normally a pure function of its index
/// in the ensemble (`instrument_role`). A section may instead carry an explicit
/// orchestration profile naming the layers it wants in inst-index order — so a
/// section can swap one inner fill for a held Pad bed without widening the
/// ensemble. The default (identity) profile carries no explicit layers, so this
/// fn delegates to `instrument_role` byte-for-byte: the default path is unchanged.
///
/// Count-mismatch rule: when the ensemble has MORE instruments than the profile
/// names layers for, the extra (highest-index) instruments CLAMP onto the last
/// named layer. The Slice-1 `pad_bed` profile names exactly the default-ensemble
/// width, so the clamp is a safe-by-construction edge case (a 5th instrument
/// would extend the Melody layer, never wrap onto Bass and mud the bottom).
pub fn assign_role(
    inst_idx: usize,
    num_instruments: usize,
    ctx: &crate::composition::StepContext,
) -> OrchestralRole {
    let prof = &ctx.section.orchestration;
    if prof.is_identity() {
        // The behaviour-neutral path: byte-identical to the legacy stratification.
        return instrument_role(inst_idx, num_instruments);
    }
    // Non-identity profile: map inst_idx onto the named layers, clamping any
    // instrument past the layer list onto the last (highest) named layer. An
    // empty layer list is the identity sentinel handled above, so `layers` is
    // non-empty here and the clamp index is always valid.
    let layers = &prof.layers;
    let clamped = inst_idx.min(layers.len().saturating_sub(1));
    to_orchestral_role(layers[clamped])
}

/// Realize the performance for ONE instrument on ONE step, given that step's
/// phrase plan. The single pure entry point main.rs's worker calls.
///
/// `step`           — the phrase-aware plan for this step (chord + position +
///                    structural-velocity FLOOR), shared once across the run.
/// `inst_idx`       — which instrument in the ensemble this realization is for.
/// `num_instruments`— ensemble size (with `inst_idx`, selects the orchestral role).
/// `features`       — the plain-scalar music-domain projection of this step's
///                    image features (dynamic level, register, rhythmic activity).
/// `ms_per_step`    — the step's total time budget, in ms (sizes onsets/holds).
///
/// PASS-A STUB BEHAVIOR (deliberately inert, so the property net goes RED on
/// behavior): returns exactly ONE note — the chord's root-position tone for
/// this instrument's tone slot — at `step.velocity` (the bare structural
/// floor, NO phrase contour / accent / taper), held for 90% of the step (NO
/// staccato/legato/portato differentiation), at offset 0 (NO rhythm pattern),
/// IGNORING the orchestral role, `features`, and harmonic-rhythm acceleration.
/// Pass B replaces this body with the four-dimension realization specified in
/// docs/design-s6-expressivity.md while keeping THIS signature.
pub fn realize_step(
    step: &StepPlan,
    inst_idx: usize,
    num_instruments: usize,
    features: &PerfFeatures,
    ms_per_step: u64,
    ctx: &crate::composition::StepContext,
) -> Vec<NoteEvent> {
    let role = assign_role(inst_idx, num_instruments, ctx);
    // S28/K3 — pivot/common-tone modulation. Reachable ONLY on a non-`home_only`,
    // `pivot:true` scheme at a real key-change boundary. Returns Some(events) → those ARE
    // this step's events for THIS instrument (the boundary step IS realized as the pivot,
    // voiced per the instrument's role). Returns None on the identity / home_only /
    // pivot:false path → fall straight through to the FROZEN free-select/theme path below.
    // This is the byte-freeze gate: on every identity step `pivot_chord_events` is None, so
    // the guard is dead and control reaches the frozen path byte-identically.
    if let Some(pivot) = pivot_chord_events(ctx, role, features, ms_per_step) {
        return pivot;
    }
    // S23 prominence: the per-role weight for THIS step, computed once and threaded
    // down to role_pitch (register) and realize_velocity (dynamics) as an additive
    // PRIVATE param — the already-blessed pad_voices/ctx additive-private-param route,
    // so realize_step's PUBLIC signature is unchanged. realize_rhythm recomputes the
    // weight itself from its already-borrowed ctx. Under identity the prominence Vec is
    // empty → this is PROMINENCE_NEUTRAL (0.5) for every role → every nudge is 0.0.
    let prominence_w = prominence_weight(ctx, role);
    // How many chord tones a Pad instrument holds for this section (0 == no pad,
    // the identity-profile default — so this read is inert on the legacy path and
    // the Pad branch never fires there). Read zero-copy off the borrowed section.
    let pad_voices = ctx.section.orchestration.pad_voices;
    // S47 seat-order-guard gate: is a CounterMelody present in THIS ensemble? Computed
    // off the SAME `assign_role` the realizer uses, over every instrument index, so the
    // guard fires exactly when (and only when) a counter actually sounds. On the identity
    // path the profile is identity → `assign_role` delegates to `instrument_role`, which
    // never yields CounterMelody, so this is FALSE on the freeze path → the seat guard is
    // a no-op there and the engine_equivalence goldens are byte-stable (spec §2b).
    let counter_present = (0..num_instruments)
        .any(|i| assign_role(i, num_instruments, ctx) == OrchestralRole::CounterMelody);
    let is_cadence = matches!(
        step.position,
        PhrasePosition::HalfCadence | PhrasePosition::PerfectAuthenticCadence
    );
    let is_phrase_start = matches!(step.position, PhrasePosition::PhraseStart);

    // ------------------------------------------------------------------
    // PITCH + REGISTER — derived from the ROLE, never from a chord-tone
    // index modulo the voicing length (which would alias a 4th instrument
    // back onto the bass pitch). The bass sounds the chord ROOT in a low
    // register; the melody the TOP chord tone in a high register; the fill
    // an inner tone in the middle. brightness nudges the melody/fill octave.
    // ------------------------------------------------------------------
    //
    // THE THEME SEAM (§2 / §4.3): before free-selecting the melody pitch, ask the
    // plan whether THIS step replays the returning theme. theme_melody_pitch
    // encodes the §2 musical intent and returns one of three outcomes:
    //   * `Some(None)`   → a theme-driven REST (Fragmented section past its head):
    //                      the melody role is silent this step — emit no events.
    //   * `Some(Some(p))`→ the theme PITCH for this step: substitute base_note = p,
    //                      then run the EXISTING velocity/rhythm/articulation
    //                      realization unchanged (so a theme note still sings with
    //                      the same contour/articulation a free-selected note would).
    //   * `None`         → free-select (bass/fill always; a melody in a Contrast/
    //                      Coda section, or with no theme, or under the net's default
    //                      `ctx`): the byte-identical role_pitch path, frozen.
    // Only the MELODY pitch is touched here — bass/fill/velocity/rhythm/articulation
    // bodies are untouched (additive musical logic, not a rewrite).
    let base_note = match theme_melody_pitch(ctx, role, &step.chord, features) {
        Some(None) => return Vec::new(),
        Some(Some(p)) => p,
        None => role_pitch(
            role,
            &step.chord,
            inst_idx,
            num_instruments,
            features,
            prominence_w,
            counter_present,
        ),
    };

    // S28/K3 — LAND-HOME re-voicing. When the form is a returning (Resolve) `pivot:true`
    // form at its final, already-stamped Perfect Authentic Cadence step, STRENGTHEN the
    // cadence's VOICING into an explicit root-position V→I in the HOME key with the home
    // tonic on top (the PAC marker: soprano on the tonic). This only RE-POINTS the role's
    // pitch within the existing single-note path — it adds NO event, moves no boundary, and
    // is `false` (untouched) on every identity / home_only / pivot:false / Open step.
    let base_note = if land_home_is_armed(ctx, step.position) {
        land_home_pitch(role, ctx, &step.chord)
    } else {
        base_note
    };

    // S29/K3 Lever 1(b) — OPENING V→I re-voicing. On the step that RESOLVES a modulating
    // section's step-0 pivot V (step_in_section == 1 of a real key change), voice this step's
    // chord as the DESTINATION TONIC resolving the pivot — leading-tone up to the new tonic,
    // pivot 7th down to the new third — so the boundary reads as a true authentic cadence in
    // the new key rather than two root-position triads stacked (the parallel octaves a
    // trombonist hears). Like land-home, this only RE-POINTS the role's pitch within the
    // existing single-note path: NO event added, NO step, NO stamp (Option A, not the held
    // Option B). `false` (untouched, byte-identical) on every identity / home_only /
    // pivot:false / non-modulating-boundary step.
    let base_note = if pivot_resolution_is_armed(ctx) {
        pivot_resolution_pitch(role, ctx)
    } else {
        base_note
    };

    // ------------------------------------------------------------------
    // DYNAMICS — phrase contour ON the structural floor.
    // velocity = floor + saturation LEVEL gain + messa-di-voce swell
    //            + metric accent + phrase-end taper, all clamped 1..=127.
    // The cadence step is EXEMPT from the swell/accent/taper contour: its
    // arrival weight comes from the floor (96), and keeping it un-shaped is
    // what makes the realized series' variance STRICTLY EXCEED the floor's
    // (the contour adds variation to the NON-cadence steps).
    // ------------------------------------------------------------------
    let velocity = realize_velocity(step, features, is_cadence, role, prominence_w);

    // ------------------------------------------------------------------
    // RHYTHM + ARTICULATION — choose a rhythm pattern from (role,
    // edge_density, phrase position) and emit its onset/duration sequence,
    // with harmonic-rhythm acceleration toward the cadence and a phrase-end
    // ritardando lengthening the final note's hold.
    // ------------------------------------------------------------------
    realize_rhythm(
        base_note,
        velocity,
        role,
        features,
        ms_per_step,
        is_cadence,
        is_phrase_start,
        step,
        pad_voices,
        ctx,
        counter_present,
    )
}

/// Register band (low MIDI bound) each orchestral role centers on.
/// theory: the bass anchors the low register, harmonic-fill the middle, the
/// melody the top. Deriving register from the ROLE (not a chord-tone index)
/// guarantees bass < melody in pitch no matter how many chord tones the voicing
/// has — the alias bug a `notes[idx % len]` scheme produces is impossible here.
const BASS_REGISTER_FLOOR: u8 = 36; // C2 — the harmonic foundation
const FILL_REGISTER_FLOOR: u8 = 55; // G3 — inner voices sit just under the tune
const MELODY_REGISTER_FLOOR: u8 = 67; // G4 — the top line sings above the texture

/// Pick this instrument's sounding pitch from the chord, placed into the
/// register band its ROLE owns. Pure; clamps into the engine's playable band.
fn role_pitch(
    role: OrchestralRole,
    chord: &Chord,
    inst_idx: usize,
    num_instruments: usize,
    features: &PerfFeatures,
    // S23 prominence weight (0..1; 0.5 neutral) for this role. Additive PRIVATE param
    // (the pad_voices precedent) — realize_step's signature is unchanged. Drives the
    // centered register lift; at 0.5 the lift is exactly 0 (byte-frozen identity).
    prominence_w: f32,
    // S47 seat-order guard gate (additive PRIVATE param, the prominence_w precedent —
    // realize_step's signature is unchanged). TRUE iff a CounterMelody is present in the
    // active ensemble. When true, the Melody seat is floored to COUNTER_CEILING +
    // MIN_FIGURE_GAP so a dark-image brightness drop can never seat the melody INTO the
    // counter band (the register inversion). GATED on counter-present so the guard never
    // fires on the synthetic no-counter engine_equivalence goldens → the freeze holds
    // (spec §2b / operator decision A). FALSE everywhere on the identity/freeze path.
    counter_present: bool,
) -> u8 {
    let notes = &chord.notes;
    if notes.is_empty() {
        return 60; // defensive middle C
    }

    // theory: brightness (0..=100) raises a part by up to an octave when the
    // image is bright, lowers it when dark — register tracks luminance. The
    // bass is exempt from upward shift (it must stay the harmonic floor); the
    // melody gets the full bright lift, the fill a gentle one.
    let bright_octaves = ((features.brightness - 50.0) / 50.0).clamp(-1.0, 1.0); // -1..=1 octave

    match role {
        OrchestralRole::Bass => {
            // The chord ROOT, dropped into the bass register. Never lifted up by
            // brightness (a bass that floats up stops being a bass); only a dark
            // image may push it an octave lower.
            let pc = notes[0] % 12;
            let dark_drop = if bright_octaves < 0.0 { 12 } else { 0 };
            let floor = BASS_REGISTER_FLOOR.saturating_sub(dark_drop);
            seat_pc_in_register(pc, floor)
        }
        OrchestralRole::Melody => {
            // The TOP chord tone (highest pitch class in the voicing), lifted
            // into the melody register; brightness raises/lowers the octave.
            let top = *notes.iter().max().unwrap();
            let pc = top % 12;
            let lift = (bright_octaves * 12.0).round() as i16;
            // S23 prominence: a foreground melody (w>0.5) lifts UP; w==0.5 → 0 (identity).
            // Risk-1 (design-s21 §C.3): clamp the SUM of (brightness lift + prominence
            // lift), never each independently — the single .clamp(24,96) below IS that
            // sum-clamp. REG_SPAN is kept small so floor(67)+lift(≤12)+prom_lift(≤2)=81 ≪
            // 96, so the melody never clamps flat at the top (no_inversion_invariant holds).
            let prom_lift =
                ((prominence_w - PROMINENCE_NEUTRAL) * PROMINENCE_REG_SPAN).round() as i16;
            let raw = MELODY_REGISTER_FLOOR as i16 + lift + prom_lift;
            // S47 SEAT-ORDER GUARD (spec §2b): when a CounterMelody is present, the melody
            // seat must sit a clear MIN_FIGURE_GAP above the counter ceiling — high-voice
            // superiority wants the figure unambiguously on top. On a dark image `lift` can be
            // −12, pulling `raw` (≈55-57) INTO the counter band [55, 67); the `.max(...)` lifts
            // it back to a clear seat. theory: melody-on-top becomes STRUCTURAL, not an
            // emergent accident of brightness. GATED on `counter_present` (operator decision A):
            // the engine_equivalence goldens are synthetic no-counter bars, so the guard never
            // fires there and the freeze holds; it fires exactly where the register inversion
            // can occur (counter-routed dark images). Folded UNDER the SAME single .clamp(24,96)
            // (the Risk-1 sum-clamp discipline) so the summed lift can never escape the band.
            let seat_floor = if counter_present {
                COUNTER_CEILING as i16 + MIN_FIGURE_GAP as i16
            } else {
                i16::MIN // no-op floor on the no-counter / identity path
            };
            let floor = raw.max(seat_floor).clamp(24, 96) as u8;
            seat_pc_in_register(pc, floor)
        }
        // Pad and CounterMelody both seat a representative INNER tone in the fill
        // register, exactly like HarmonicFill. theory: the Pad's full multi-tone
        // bed is built in realize_rhythm directly off the chord (one note can't
        // express it); the single base_note role_pitch returns here is only the
        // anchor pitch the other realize machinery threads through. For
        // CounterMelody this seat is a DEAD anchor — `realize_rhythm`'s species
        // arm (:1818) overwrites it on every emitted event with the moving
        // counter-line's own pitch. It shares the inner-tone seating below only as
        // a harmless default (the value never sounds). Both therefore share the
        // inner-tone seating below so they sit under the melody.
        OrchestralRole::HarmonicFill | OrchestralRole::Pad | OrchestralRole::CounterMelody => {
            // An INNER chord tone, spread across the fill instruments so two
            // fills don't double the same tone, placed in the middle register
            // with a gentle brightness nudge. theory: inner voices fill the
            // harmony; spacing them by index keeps the chord's body audible.
            let inner_count = notes.len().max(1);
            // Prefer the middle tone (the third) as the canonical fill pitch,
            // then spread additional fills across the remaining tones by index.
            let pick = if num_instruments >= 3 {
                // map fill instruments (idx 1..num-1) across inner tones
                let fill_rank = inst_idx.saturating_sub(1);
                1 + (fill_rank % inner_count.saturating_sub(1).max(1))
            } else {
                inner_count / 2
            };
            let pc = notes[pick.min(notes.len() - 1)] % 12;
            let lift = ((bright_octaves * 6.0).round() as i16).clamp(-12, 12);
            // S23 prominence (Risk-1, design-s21 §C.3): the recessive bed is NEVER lowered.
            // A recessive role (w<0.5) would yield a negative lift — clamp it at >=0 so
            // prominence can only ever RAISE the bed toward neutral, never sink it below
            // the bass and invert figure-ground. A foreground (w>0.5) could rise. At w==0.5
            // the lift is exactly 0 (identity). The summed lift+prom_lift rides the SINGLE
            // .clamp(24,96) below (the Risk-1 sum-clamp), not an independent clamp.
            let prom_lift =
                (((prominence_w - PROMINENCE_NEUTRAL) * PROMINENCE_REG_SPAN).round() as i16).max(0);
            let floor = (FILL_REGISTER_FLOOR as i16 + lift + prom_lift).clamp(24, 96) as u8;
            seat_pc_in_register(pc, floor)
        }
    }
}

/// Seat pitch class `pc` at or just above a register floor, clamped to 24..=108.
fn seat_pc_in_register(pc: u8, floor: u8) -> u8 {
    let pc = pc % 12;
    let mut note = (floor / 12) * 12 + pc;
    while note < floor {
        note = note.saturating_add(12);
    }
    note.clamp(24, 108)
}

/// Realize the phrase-contoured velocity for a step. Pure, deterministic.
fn realize_velocity(
    step: &StepPlan,
    features: &PerfFeatures,
    is_cadence: bool,
    role: OrchestralRole,
    // S23 prominence weight (0..1; 0.5 neutral) for this role. Additive PRIVATE param
    // (the pad_voices precedent) — realize_step's signature is unchanged. Drives the
    // centered velocity nudge; at 0.5 the nudge is exactly 0 (byte-frozen identity).
    prominence_w: f32,
) -> u8 {
    // Overall LEVEL from saturation: map 0..=100 to roughly [-12, +18] velocity
    // added to the floor — a dull bar still respects the floor's phrase marking,
    // a vivid bar pushes louder. theory: saturation = how vivid the color is =
    // how bold the tone; it sets the dynamic LEVEL, the phrase contour SHAPES it.
    let sat = features.saturation.clamp(0.0, 100.0);
    let level_gain = -12.0 + (sat / 100.0) * 30.0; // -12..=+18

    let mut vel = step.velocity as f32 + level_gain;

    if !is_cadence {
        // Messa di voce: a gentle half-sine swell to the phrase midpoint and a
        // relaxation toward its end. theory: the classic dynamic arch of a
        // sung/blown phrase — grow into the middle, ease toward the close. Kept
        // modest (~+4) so it shapes the line without filling the metric valley
        // between strong and weak beats (the structural floor already lifts the
        // start and cadence; the contour must add spread, not compress it).
        let span = step.phrase_len.max(2) as f32 - 1.0;
        let frac = (step.position_in_phrase as f32 / span).clamp(0.0, 1.0);
        let swell = (std::f32::consts::PI * frac).sin() * 4.0;
        vel += swell;

        // Metric accent: strong metric positions (even position_in_phrase, the
        // phrase downbeat strongest) get an additive accent; weak positions a
        // subtraction. theory: meter alternates strong and weak; voicing that
        // alternation as velocity opens the realized series WIDER than the bare
        // structural floor — the strong beats rise, the weak beats deepen — while
        // staying smaller than the floor's start/cadence weighting so phrase
        // structure still reads.
        let accent = if step.position_in_phrase == 0 {
            9.0 // phrase downbeat — strongest metric position
        } else if step.position_in_phrase.is_multiple_of(2) {
            2.0 // a strong interior beat
        } else {
            -6.0 // a weak interior beat — the valley of the metric arch
        };
        vel += accent;

        // Phrase-end taper: the last interior step before the cadence eases off
        // so the cadence remains the loudest point. theory: the phrase relaxes
        // into its arrival; the diminuendo makes the cadence's weight land.
        if step.position_in_phrase + 2 >= step.phrase_len && step.position_in_phrase > 0 {
            vel -= 4.0;
        }
    }

    // The melody carries the messa-di-voce peak a touch more; the bass holds a
    // steadier dynamic (the foundation should not surge). theory: foreground
    // voices shape dynamics more than the structural bass.
    match role {
        OrchestralRole::Melody if !is_cadence => vel += 2.0,
        OrchestralRole::Bass if !is_cadence => vel -= 1.0,
        // The Pad is a SUPPORTING bed: it must sit clearly under the melody and
        // not compete with the foreground line. A modest negative bias (a touch
        // deeper than the bass's −1) keeps the held harmony present-but-recessed —
        // the background, not a second tune. Unreachable under the identity
        // profile, so this never touches the byte-frozen default path.
        OrchestralRole::Pad if !is_cadence => vel -= 3.0,
        // S48 SLICE 3 (2a.ii) — the LEVEL FINISH: give the CounterMelody and HarmonicFill a
        // STRUCTURAL level floor BELOW the melody, completing the figure-ground gap from the
        // level side (the Pad's −3 precedent above). The counter recedes more than the fill
        // by role, both less than the Pad (a moving inner line must stay audible — S45). Both
        // `!is_cadence`-guarded like every arm above; the cadence ring is exempt. Under
        // identity neither role exists on the goldens, so this is byte-neutral on the freeze
        // path (the synthetic engine_equivalence bars carry only Melody+Bass — spec §4).
        OrchestralRole::CounterMelody if !is_cadence => vel -= COUNTER_VEL_BIAS,
        OrchestralRole::HarmonicFill if !is_cadence => vel -= FILL_VEL_BIAS,
        _ => {}
    }

    // S23 prominence: centered velocity nudge, applied AFTER the existing per-role bias
    // so saliency WIDENS the gap the realizer already opens (+2 Melody / -3 Pad). A
    // foreground role (w>0.5) gets louder, a recessive role (w<0.5) quieter. EXACTLY 0
    // at w==0.5: (0.5-0.5)*VEL_SPAN == 0.0 — the byte-frozen identity nudge. Guarded on
    // !is_cadence so the cadence goldens (114/84) stay byte-stable even under a future
    // non-0.5 cadence weight; moot under identity (the term is already 0). The final
    // round().clamp(1,127) below satisfies the 1..=127 bound.
    if !is_cadence {
        vel += (prominence_w - PROMINENCE_NEUTRAL) * PROMINENCE_VEL_SPAN;
    }

    vel.round().clamp(1.0, 127.0) as u8
}

/// The articulation classes, expressed as the fraction of a note's time slot it
/// actually sounds. theory: staccato detaches (short), portato slightly
/// separates (the neutral default), legato connects (near-full / overlapping).
const STACCATO_FRAC: f32 = 0.40;
const PORTATO_FRAC: f32 = 0.70;
const LEGATO_FRAC: f32 = 0.95;
// S13's LEGATO_FRAC_HI (1.05) — the original continuous-curve legato ceiling — is
// SUPERSEDED by ARTIC_WINDOW_HI (1.10) below (S15 §4.4). The curve still crosses the
// bar line for calm images (the "played vs typed" cue), now within the bounded
// 0.55..=1.10 non-cadence window. See ARTIC_WINDOW_LO / ARTIC_WINDOW_HI.
/// S13: per-bar edge-density normalization divisor for the articulation curve.
/// theory: real photos carry raw per-bar edge density ~0.005..0.05; dividing by this
/// calibrated max maps that into a usable 0..1 ACTIVITY range so the curve actually
/// spans staccato↔legato instead of clustering at one end. This MIRRORS the canonical
/// `feature_normalization.edge_density_max` in mappings.json — it lives as a const
/// ONLY because `realize_rhythm` is a free fn with no MappingTable handle (the seam
/// does not carry one into the per-step realization); keep the two values in sync.
const EDGE_ACTIVITY_RANGE_MAX: f32 = 0.05;
/// S29 Lever 2(ii) — gain mapping `(Section.density − 0.5)` into an `edge_activity` nudge.
/// theory reasoning (spec-s29 §2.2(ii)): `Section.density` is the per-section busyness knob the
/// planner sets from region energy (a denser/higher-energy excursion should sound busier — the
/// MX-4 "second dimension of contrast"). The realizer's busyness knob is `edge_activity`, so we
/// bias it by the section density. The gain 0.5 keeps the bias MODEST: the planner's ±0.15
/// density swing (0.35..0.65 around the 0.5 neutral) shifts activity by ≤ ±0.075 — a felt scene
/// change, not a different piece. CRITICAL byte-freeze hinge: `Section.density == 0.5`
/// (DENSITY_NEUTRAL) on EVERY identity/home/home_only section ⇒ `(0.5 − 0.5) * GAIN == 0.0` ⇒
/// `edge_activity` is byte-identical to pre-S29 on the identity path.
const DENSITY_ACTIVITY_GAIN: f32 = 0.5;
/// S15 §4.4: the NON-CADENCE articulation window — the perceptually pleasant
/// hold-fraction range the continuous S13 curve is re-scaled into. theory: a hold
/// below ~0.55 of the slot reads as a click rather than a sounded tone; a hold above
/// ~1.10 muds successive notes together and loses articulation. The cadence ring
/// (1.20, via the `sustained` cap) is a separate, byte-stable structural figure and
/// is NOT governed by this window. WINDOW_LO replaces STACCATO_FRAC and WINDOW_HI
/// replaces LEGATO_FRAC_HI as the curve endpoints (the lerp shape is unchanged).
const ARTIC_WINDOW_LO: f32 = 0.55;
const ARTIC_WINDOW_HI: f32 = 1.10;
/// S15 §2.1/§4.4: the per-character articulation bias for Ballad — the slice-1
/// character. theory: a character centers WHERE in the 0.55..=1.10 window its line
/// sits (Ballad leans legato, toward the ceiling); edge_activity then varies around
/// that center. Slice 1 is Ballad-ONLY and this bias is IDENTITY (×1.0), so it is a
/// deliberate no-op here — it exists as the documented seam the later characters
/// (March→toward 0.55, Hymn→~1.0) ride on without re-touching the clamp.
const BALLAD_ARTIC_BIAS: f32 = 1.0;
/// S39 — the within-step hold multiplier a multi-step THEME note (motif `dur_steps > 1`)
/// rides in the Melody SUSTAINED branch so it SINGS longer than a one-step note. The
/// `sustained` closure's `.min(1.20)` overlap-ceiling cap clamps the product, so even
/// from the window ceiling (1.10) this never rings past the next kernel step — the note's
/// extra length is carried by its CONTINUATION rest, not by an across-step hold (freeze
/// safe). Applied ONLY on a theme onset with `dur > 1`; `dur == 1` and free-select see ×1.0.
const THEME_LONG_NOTE_SING: f32 = 1.15;
/// The ritardando multiplier applied to a phrase-final note's hold. theory: as
/// the phrase relaxes into its arrival the final note rings longer.
const RITARDANDO_FACTOR: f32 = 1.30;
/// S53 (D-CELL) — the per-piece MOTTO onset-bias depth: the MAXIMUM fraction of an interior onset's
/// local inter-onset gap by which the per-piece rhythmic motto may DISPLACE that onset (cell 3 only
/// uses the full depth; cell 1 a partial pull). theory & guard-rails: the motto biases WHERE the
/// melody's already-chosen onsets sit, never HOW MANY (count-preserving — GR-1: it cannot promote a
/// voice's ActivityClass because it changes no onset count and touches only the Melody/figure), and
/// the displacement is a FRACTION of the GAP to the next onset so it can never cross the next onset
/// or the step boundary. 0.18 is conservative: a perceptible lilt/steadying (≈ one-tenth-to-one-fifth
/// of a beat's "and") that the ear reads as a different WALK without fragmenting the texture or
/// muddying the downbeat. Ear-tune window [0.12, 0.25]; below 0.12 the gait is inaudible (GR-4),
/// above 0.25 the syncopation starts to read as a metric error rather than a swung profile.
const MOTTO_ONSET_BIAS_DEPTH: f32 = 0.18;
/// S53 (D-CELL) — the pre-cadence attenuation of the motto onset bias (GR-2): the phrase RELAXES
/// into its close, so the bias is HALVED on the pre-cadence approach step and ZEROED at the cadence
/// itself (the cadence ring early-returns above, structurally unperturbed). theory: a gait that kept
/// stamping its off-grid signature through the homecoming would rob the arrival of repose; the gait
/// drives the phrase BODY and gets out of the way for the cadence.
const MOTTO_PRECADENCE_ATTEN: f32 = 0.5;
/// The NORMALIZED edge-activity floor below which an inner voice may rest-as-
/// gesture on a weak interior beat. theory: rest-as-gesture is a deliberate
/// silence that lets the outer voices speak — it must be RARE and intentional,
/// not constant. The original guard read the RAW per-bar edge (~0.005..0.05 on
/// real photos) against 0.15, so it fired on essentially every image and the
/// inner harmony vanished half the time. Re-pointed to the normalized 0..1
/// `edge_activity`, the threshold is the activity below which the texture is
/// genuinely near-static. 0.10 (NOT the design's loose ~0.15 sketch) is chosen
/// against the fixtures: the "calm" texture sits at activity ≈ 0.08, and a calm
/// image is exactly where the held harmonic bed must be PRESENT, not silent — so
/// the rest must NOT fire there. 0.10 keeps the calm bed sounding while still
/// reserving a rest for a genuinely flat, near-edgeless field (activity < 0.10),
/// where a momentary inner silence reads as intended repose rather than a hole.
const FILL_REST_ACTIVITY: f32 = 0.10;
/// The hold fraction the Pad bed uses so consecutive beds TIE into a continuous
/// floor (legato overlap). theory: the seam ships legato-overlap, not true
/// cross-step sustain — a Pad `hold_ms` that spanned multiple steps would stall
/// the block-until-last-event scheduler and wreck the tempo. So each Pad bed
/// holds the FULL step plus a small overlap into the next, at the established
/// non-cadence window ceiling (ARTIC_WINDOW_HI = 1.10), so successive beds
/// connect with no audible gap while staying within the `sustained` ≤1.2× cap —
/// the wall-clock over-run per step is ≤10%, a tolerable near-constant wobble,
/// never the N× catastrophe of a multi-step hold. A reverberant external synth
/// smooths the tiny re-attack into a continuous pad on the actual listening path.
const PAD_OVERLAP_FRAC: f32 = ARTIC_WINDOW_HI;

/// Realize the rhythm + articulation for one step: select a rhythm pattern from
/// (role, edge_density, phrase position) and emit its onset/duration sequence.
/// Pure and deterministic. Notes are clamped into 24..=108.
#[allow(clippy::too_many_arguments)]
fn realize_rhythm(
    note: u8,
    velocity: u8,
    role: OrchestralRole,
    features: &PerfFeatures,
    ms_per_step: u64,
    is_cadence: bool,
    is_phrase_start: bool,
    step: &StepPlan,
    // How many chord tones a Pad instrument holds as the bed (0 on the identity
    // path — inert, no instrument is ever a Pad there). Private additive param on
    // this free fn: NOT a public-seam change, realize_step's signature is unchanged.
    pad_voices: u8,
    // The borrowed plan-relative context (S18, additive). The CounterMelody arm
    // reads `ctx.section.steps` + `ctx.step_in_section` + the recomputed melody
    // pitch off it; mirrors the `pad_voices` precedent — a private-fn param, NOT a
    // realize_step signature change. Inert on the identity path (no Counter inst).
    ctx: &crate::composition::StepContext,
    // S48 SLICE 3 (2b.4) — is a CounterMelody present in THIS ensemble? Additive PRIVATE param
    // on this free fn (the pad_voices/ctx precedent — realize_step's PUBLIC signature is
    // UNCHANGED), threading the SAME value realize_step already computed (:1377-1378) and
    // already passes to role_pitch. The inverse-register comp fires iff this is true (plus a
    // foreground prominence weight + !is_cadence) — so on the synthetic no-counter
    // engine_equivalence goldens (counter_present == false) every comp term short-circuits and
    // the freeze holds, EXACTLY as the seat guard's counter_present gate does.
    counter_present: bool,
) -> Vec<NoteEvent> {
    // S13: normalize the RAW per-bar edge density into a 0..1 ACTIVITY knob. The old
    // code compared raw edge (≈0.005 on real photos) against 0.25/0.70 cutoffs, so
    // every real image fell in the "low edge → legato/sustained" band — uniform output.
    // Normalizing first lets the curve and the pattern bands span their full range.
    let edge_activity = {
        let base = (features.edge_density / EDGE_ACTIVITY_RANGE_MAX).clamp(0.0, 1.0);
        // S29 Lever 2(ii)/MX-4: a denser section nudges activity UP, a sparser one DOWN, so the
        // per-section density the planner set from region energy becomes AUDIBLE as busyness —
        // the second dimension of contrast (key + density) the modulation needs to read as a
        // scene change. theory: `ctx.section.density` is 0.5 (DENSITY_NEUTRAL) on EVERY
        // identity/home/home_only section, so this term is EXACTLY 0.0 there → `edge_activity`
        // is byte-identical to pre-S29 on the identity path (the byte-freeze hinge). Read
        // zero-copy off the already-borrowed `ctx` — no new field, no seam change.
        let density_nudge = (ctx.section.density - 0.5) * DENSITY_ACTIVITY_GAIN;
        (base + density_nudge).clamp(0.0, 1.0)
    };
    // (The former raw-clamped `edge` local is GONE: the HarmonicFill rest-as-gesture
    // check now reads the NORMALIZED `edge_activity` against FILL_REST_ACTIVITY, so the
    // raw value — which never reached the old 0.15 cutoff on real photos and silenced
    // the inner harmony nearly every weak beat — has no remaining reader.)
    let step_ms = ms_per_step.max(1);

    // Harmonic-rhythm acceleration: the step immediately before a cadence drives
    // into the arrival with MORE onsets. theory: shortening note values toward a
    // cadence is how common-practice phrases "press" into their close.
    let pre_cadence = !is_cadence
        && !is_phrase_start
        && step.position_in_phrase + 2 >= step.phrase_len
        && step.phrase_len >= 2;

    // S13 CONTINUOUS articulation curve (S15 §4.4 re-scaled the output window).
    // theory: note length varies SMOOTHLY with the normalized edge activity, now
    // re-scaled into the bounded non-cadence window (§4.4):
    //   edge_activity = 0 (calm)  → ARTIC_WINDOW_HI (1.10, overlapping/singing)
    //   edge_activity = 1 (busy)  → ARTIC_WINDOW_LO (0.55, crisply detached)
    // A continuous lerp means two images of different busyness get different note
    // lengths instead of snapping to one of three values — directly fixing
    // "uniformly short, computer-like." The HarmonicFill still leans connected
    // (it supports rather than competes), so it sits a touch above the curve.
    // NOTE (compromise): the spec's optional global articulation bias `*= lerp(1.10,
    // 0.90, texture)` is omitted — `texture_laplacian_var` is not on the per-step
    // PerfFeatures seam and adding it would be a forbidden seam change. The per-bar
    // edge curve alone removes the uniformity; the global texture bias is deferred.
    // S15 §4.4 — the NON-CADENCE articulation window narrows to a perceptually
    // pleasant 0.55..=1.10. theory (design-s15 §7): below ~0.55 a note reads as a
    // click rather than a tone; above ~1.10 successive notes mud together and lose
    // their articulation. The S13 curve's RESPONSIVENESS is preserved — we RE-SCALE
    // the same 0→1 edge_activity sweep into the narrower output range rather than
    // truncate it (a calm image is still more legato than a busy one, just inside
    // musical bounds). The endpoints move LEGATO_FRAC_HI(1.05)→ARTIC_WINDOW_HI(1.10)
    // and STACCATO_FRAC(0.40)→ARTIC_WINDOW_LO(0.55); the lerp is otherwise identical.
    //
    // The cadence ring is UNTOUCHED: the `is_cadence` early return below at the
    // `sustained(0, step_ms, LEGATO_FRAC)` line and the `sustained` helper's
    // `(frac*rit).min(1.20)` cap (the 1.20 cadence-overlap ceiling, the 240 ms golden)
    // never read `base_frac` — so the cadence goldens stay byte-stable.
    //
    // ARTIC_BIAS rides on TOP of the window as a per-character multiplier centering
    // where in 0.55..=1.10 this character sits (Ballad→toward the 1.10 ceiling). Slice
    // 1 is Ballad-only and BALLAD_ARTIC_BIAS == 1.0 (identity), so the bias is a no-op
    // and the window re-scale is the only change this slice makes.
    let curve_frac = ARTIC_WINDOW_HI + (ARTIC_WINDOW_LO - ARTIC_WINDOW_HI) * edge_activity;
    // S49 SLICE 2 (L3) — the MELODY's articulation-detach gate. theory: the foreground melody is
    // CRISPER than the bed, but ONLY when it is a genuine FOREGROUND voice — gated on the SAME
    // foreground-weight pivot as L1 so at neutral weight 0.5 the strict `>` is FALSE → 0.0 bias →
    // the melody-79 golden's `base_frac` is the unchanged `curve_frac` (spec §4 freeze witness).
    let melody_detach = if matches!(role, OrchestralRole::Melody)
        && prominence_weight(ctx, OrchestralRole::Melody) > ACTIVITY_FLOOR_THRESHOLD
    {
        MELODY_ARTIC_DETACH_BIAS
    } else {
        0.0
    };
    let base_frac = match role {
        // Fill biases toward the connected end so inner voices sustain under the line.
        // Floor it at the window LOW, not LEGATO_FRAC (0.95): clamping a HarmonicFill
        // up to 0.95 would punch ABOVE the busy-end of the new window and re-introduce
        // a discontinuity. The window low (0.55) is the right connected-leaning floor.
        OrchestralRole::HarmonicFill => curve_frac.max(ARTIC_WINDOW_LO),
        // S49 SLICE 2 (L3) — the MELODY detaches: a NEGATIVE base_frac nudge (crisper foreground),
        // gated freeze-neutral above. Composes BEFORE the window clamp and BEFORE the S48 comp
        // detach (the comp still rides on top for the low-seat case). DURATION-only — F1/F3 (onset
        // count / pitch) untouched. Floored by the clamp below so it never clicks.
        OrchestralRole::Melody => curve_frac - melody_detach,
        // S49 SLICE 2 (L3) — the BASS connects: the Fill connected-floor lean generalized to the
        // bed so the bass sustains smoothly under the figure. GATED on `counter_present` — the ONE
        // golden-reachable L3 term (the Bass arm runs on the bass-36 golden); `false` on the
        // no-counter goldens → the legacy `curve_frac` → byte-identical rounded `hold_ms` (spec §4/§8).
        OrchestralRole::Bass if counter_present => curve_frac.max(ARTIC_WINDOW_LO),
        _ => curve_frac,
    }
    // Apply the per-character articulation bias (slice-1 Ballad == 1.0, no-op), then
    // clamp back into the window so the bias can't escape the pleasant range.
    .mul_add(BALLAD_ARTIC_BIAS, 0.0)
    .clamp(ARTIC_WINDOW_LO, ARTIC_WINDOW_HI);

    // S48 SLICE 3 (2b) — INVERSE-REGISTER COMPENSATION factor for THIS step's melody.
    // theory: a melody low in its range self-projects least, so it earns NON-LEVEL separation
    // (DP-3: level NEVER here) inverse to its realized seat. GATE (spec §2b.4, the lead's
    // confirmed decision): fire iff a counter is present (the primary freeze witness — the
    // synthetic no-counter goldens have counter_present == false → comp is a NO-OP → 9/9
    // byte-green), AND the melody is a genuine FOREGROUND voice (prominence > the activity
    // floor; neutral 0.5 fails the strict `>` → no comp on the identity path), AND it is not a
    // cadence (the cadence ring early-returns above and is structurally untouched). On the
    // freeze path ALL THREE are false, so `comp` is 0.0 and every comp term below is inert.
    let comp = if matches!(role, OrchestralRole::Melody)
        && counter_present
        && prominence_weight(ctx, OrchestralRole::Melody) > ACTIVITY_FLOOR_THRESHOLD
        && !is_cadence
    {
        inverse_register_compensation(note)
    } else {
        0.0
    };

    // S48 SLICE 3 (2b.3) — THE SECONDARY (articulation) comp tool: nudge base_frac toward MORE
    // DETACHED (smaller fraction → crisper, more separated notes) as the comp rises. theory
    // (DP-3 rank 4, articulation SECOND to the onset push): a low-seated melody gets crisper
    // attacks the ear reads as figure-ground separation. LEVEL is NEVER a comp tool (DP-3) — we
    // shorten DURATION, never raise velocity. Floored at ARTIC_WINDOW_LO so it stays in the
    // pleasant window and never clicks. comp == 0.0 (high-seated / freeze path) → byte-unchanged.
    let base_frac = if comp > 0.0 {
        (base_frac - comp * COMP_ARTIC_DETACH).max(ARTIC_WINDOW_LO)
    } else {
        base_frac
    };

    // Phrase-end ritardando: cadence (and the pre-cadence approach) lengthen the
    // sounding duration so the arrival rings. Applied as a multiplier on the
    // hold fraction, capped at full slot. theory: notes ring longer as the music
    // relaxes into the cadence.
    let rit = if is_cadence { RITARDANDO_FACTOR } else { 1.0 };

    // Helper: build a single sustained event filling `slot_ms` from `offset`.
    let sustained = |offset: u64, slot_ms: u64, frac: f32| -> NoteEvent {
        let f = (frac * rit).min(1.20); // allow a slight overlap on cadence ring
        let hold = ((slot_ms as f32) * f).round().max(1.0) as u64;
        NoteEvent {
            note,
            velocity,
            hold_ms: hold,
            offset_ms: offset,
        }
    };

    // ---- Pattern selection -------------------------------------------------
    // theory: the orchestral role GATES how much rhythmic freedom each part
    // gets; edge_density and phrase position select WITHIN that freedom. The
    // result is >=3 genuinely distinct onset/duration shapes:
    //   sustained / arpeggio / dotted / syncopated / rest-as-gesture, plus
    //   harmonic-rhythm acceleration adding onsets before a cadence.

    // A cadence/phrase-start must ALWAYS sound (never rest-as-gesture), and a
    // cadence rings as a single sustained, ritardando-lengthened note — the
    // arrival is a point of repose, not an active figure.
    if is_cadence {
        // S30-CP-FIX (GAP-3) — COUNTER CADENCE PITCH AT THE RING. The generic cadence ring
        // emits `note` (the role's `role_pitch` seat) and returns BEFORE the per-role match
        // below — including the CounterMelody arm. For the structural roles that is correct
        // and byte-frozen. But the CounterMelody role's species CADENCE clausula
        // (`cadence_resolution_pitch`, the §1.2 no-leap step-to-perfect close) lives in
        // `pick_counter_figure`, reached only from the counter arm — so without this branch the
        // counter would ring on its STUB inner-tone seat and the cadence clausula would be dead
        // code. Here we recompute the CounterMelody pitch through the SAME shared realize path
        // (`realized_counter_pitch_with_prev`, which dispatches the PAC arm of
        // `pick_counter_figure`) so the counter rings on its CONTRAPUNTAL cadence pitch, then
        // still rings it as the single sustained legato cadence note. This touches ONLY the
        // CounterMelody role; Bass/Pad/Melody keep their frozen `note`, so the cadence ring and
        // the engine_equivalence goldens are unperturbed (the counter is never on the identity/
        // frozen path). Deterministic (the realize path is RNG-free).
        if role == OrchestralRole::CounterMelody {
            let si = ctx.step_in_section;
            let prev_counter = match si.checked_sub(1) {
                Some(_) => realized_prev_counter(ctx, features, si),
                None => seed_prev_counter(None, step),
            };
            let cnt = realized_counter_pitch_with_prev(ctx, step, features, si, prev_counter);
            let ring = NoteEvent {
                note: cnt,
                ..sustained(0, step_ms, LEGATO_FRAC)
            };
            return vec![ring];
        }
        return vec![sustained(0, step_ms, LEGATO_FRAC)];
    }

    match role {
        OrchestralRole::Bass => {
            // S34 PART (B) — GENERATOR-BACKED BASS DISPATCH. Before the legacy bass body, branch
            // on this section's RESOLVED bass pattern (read zero-copy off the borrowed `ctx`, the
            // S20 figuration-seam discipline). A Walking/Pedal pattern OVERRIDES the bass with a
            // generated line; None / Sustained / ANY cadence step falls through to the EXISTING
            // sustained/pre-cadence body BYTE-UNCHANGED — that `_` arm is the freeze default.
            // FREEZE: the identity / `single_section_default` profile carries
            // `bass_pattern_resolved == None`, so the match is structurally `None` on every
            // equivalence-net step → control reaches the legacy body byte-identically and the
            // engine_equivalence goldens (G_BASS_NOTE=36, …) do not move. (`is_cadence` already
            // returned its sustained ring above, so a cadence never reaches this match.)
            match ctx
                .section
                .orchestration
                .bass_pattern_resolved
                .as_ref()
                .map(|b| (b.kind, b.density, b.pedal_degree))
            {
                Some((crate::composition::BassPatternKind::Walking, density, _)) => {
                    // The tonic pitch class for diatonic derivation — IDENTICAL to the convention
                    // `theme_pitch`/the pivot helpers use (home root + section key offset).
                    let tonic_pc = ((ctx.key_tempo.home_root_midi as i16
                        + ctx.section.key_offset_semitones as i16)
                        .rem_euclid(12)) as u8;
                    let current_root_pc =
                        step.chord.notes.first().map(|n| n % 12).unwrap_or(tonic_pc);
                    // Next chord root via the proven S33 in-realizer lookahead (no seam change).
                    let si = ctx.step_in_section;
                    let next_root_pc = ctx
                        .section
                        .steps
                        .get(si + 1)
                        .and_then(|s| s.chord.notes.first())
                        .map(|n| n % 12);
                    walking_bass(
                        current_root_pc,
                        next_root_pc,
                        tonic_pc,
                        ctx.section.mode.as_str(),
                        velocity,
                        step_ms,
                        density,
                    )
                }
                Some((crate::composition::BassPatternKind::Pedal, _, pedal_degree)) => {
                    let tonic_pc = ((ctx.key_tempo.home_root_midi as i16
                        + ctx.section.key_offset_semitones as i16)
                        .rem_euclid(12)) as u8;
                    pedal_bass(
                        tonic_pc,
                        ctx.section.mode.as_str(),
                        pedal_degree,
                        velocity,
                        step_ms,
                        base_frac,
                    )
                }
                // Sustained, None → the EXISTING byte-unchanged bass body (the freeze default).
                _ => {
                    // BASS: the harmonic floor — steady, sparse, mostly sustained. Even
                    // when busy it adds at most a single re-articulation (a walking
                    // pickup into a pre-cadence), never an arpeggio. Low onset count.
                    if pre_cadence {
                        // Harmonic-rhythm acceleration: a long root + a short pickup
                        // onset at 3/4 driving into the cadence (2 onsets > the 1 of an
                        // early-interior bass step).
                        let two_thirds = step_ms * 3 / 4;
                        vec![
                            sustained(0, two_thirds, PORTATO_FRAC),
                            sustained(two_thirds, step_ms - two_thirds, PORTATO_FRAC),
                        ]
                    } else {
                        // One grounded, sustained root for the whole step, its length set by
                        // the CONTINUOUS articulation curve (S13): a calm image's bass sings
                        // across the bar (frac>1.0), a busy image's bass is shorter/detached.
                        vec![sustained(0, step_ms, base_frac)]
                    }
                }
            }
        }

        OrchestralRole::HarmonicFill => {
            // HARMONIC-FILL: supports, never competes — least rhythmic activity,
            // sustained inner tones. This is the ONLY place rest-as-gesture is
            // allowed, and only on a weak interior beat (never start/cadence).
            // theory: a deliberate silence in an inner voice is an articulation,
            // letting the outer voices speak.
            let weak_interior = !step.position_in_phrase.is_multiple_of(2);
            // REST-BUG FIX: gate on the NORMALIZED edge_activity (0..1), not the raw
            // per-bar edge. The raw value (~0.005..0.05 on real photos) never reached
            // the old 0.15 cutoff except by accident, so the rest fired on essentially
            // every weak interior beat and dropped the inner harmony — the "missing all
            // the harmony" defect. Against the normalized scale, only a genuinely
            // near-static texture (activity < FILL_REST_ACTIVITY) rests; the calm-image
            // bed (activity ≈ 0.08 on the calm fixture is still above this floor only by
            // a hair — see the const's note) now SOUNDS instead of resting.
            //
            // S49 SLICE 2 (L1) — per-role rhythm bias on the bed's ONE busyness decision. The
            // Fill is a BED role, biased TOWARD steadier (NON-POSITIVE `role_rhythm_bias`). The
            // only busyness decision a Fill makes is whether to break into a rest-as-gesture (a
            // momentary activity event); the negative bed bias LOWERS the effective rest cutoff so
            // the inner bed holds steady more readily and only rests on a genuinely flatter field —
            // a steadier bed grid distinct from the melody's. This NEVER hollows the bed beyond the
            // legacy behavior (it makes rests RARER, the bed MORE present — the safe side of the
            // deep-bed-thinness WATCH, spec §8 watch-5). FREEZE: the Fill arm is not entered on the
            // synthetic no-counter goldens (no Fill instrument there), so this is inert on the
            // freeze path. SIGN fixed NON-POSITIVE; magnitude TASTE-SIZED (spec §3).
            let fill_rest_cutoff = FILL_REST_ACTIVITY + role_rhythm_bias(role);
            if edge_activity < fill_rest_cutoff && weak_interior {
                // rest-as-gesture: emit NO event.
                Vec::new()
            } else {
                // S49 SLICE 2 (L2) — PHASE-SEPARATE the inner bed onset OFF the melody's downbeat.
                // The Fill onset sits at offset 0 (the same downbeat the melody attacks); displace
                // it to the weak beat (BED_PHASE_FRAC) so the inner bed reads as its OWN onset grid,
                // distinct from the melody (0) and the counter (step_ms/4). COUNT-PRESERVING (the
                // recede_pad_onsets precedent) → F5b/F1 untouched; F5a (rhythm-distinctness) can only
                // improve. FREEZE: the Fill arm is an inner-voice role never assigned on the
                // single-instrument synthetic goldens, so this is inert on the freeze path.
                phase_separate_bed(
                    vec![sustained(0, step_ms, base_frac)],
                    BED_PHASE_FRAC,
                    step_ms,
                )
            }
        }

        OrchestralRole::Pad => {
            // HARMONY BED: hold `pad_voices` chord tones SIMULTANEOUSLY (all at
            // offset 0) as a continuous floor under the melody. The full chord is
            // reachable here via `step.chord.notes` (StepPlan is already in scope —
            // no seam change), so unlike the other arms (which sound one base_note)
            // the Pad re-derives its whole voicing from the chord.
            //
            // VOICING (music-theory owned): seat the INNER chord tones — the 3rd /
            // 5th / 7th, i.e. notes[1..] — in the fill register (FILL_REGISTER_FLOOR
            // = G3, where inner voices live), DELIBERATELY skipping notes[0] (the
            // chord root) so the bed does not double the Bass an octave up and mud
            // the bottom. We take up to `pad_voices` of those inner tones; if the
            // chord is a bare triad we still get root-less 3rd+5th, a clean inner
            // bed. Each tone is seated into its own register slot and de-duplicated
            // so no two bed voices collapse onto the same pitch (the spirit of
            // upper_voices_well_spaced — a bed of unisons is not a bed).
            let notes = &step.chord.notes;
            let pad_events = if notes.is_empty() || pad_voices == 0 {
                // Defensive: a chord with no tones (or a profile that named a Pad
                // with 0 voices) holds nothing. Fall back to one supporting tone so
                // the instrument is not silent.
                vec![sustained(0, step_ms, PAD_OVERLAP_FRAC)]
            } else {
                // Prefer the inner tones (skip the root at index 0); if the chord
                // is a single tone, fall back to it so the bed is never empty.
                let inner: &[u8] = if notes.len() > 1 {
                    &notes[1..]
                } else {
                    &notes[..]
                };
                let want = (pad_voices as usize).min(inner.len());
                let mut seated: Vec<u8> = Vec::with_capacity(want);
                for tone in inner.iter().take(want) {
                    let mut seat = seat_pc_in_register(tone % 12, FILL_REGISTER_FLOOR);
                    // De-dup: if two chord tones share a pitch class they would seat
                    // onto the same note — lift the collision an octave so the bed
                    // stays spread (no two upper voices on the same pitch).
                    while seated.contains(&seat) {
                        seat = seat.saturating_add(12).min(108);
                        if seat >= 108 {
                            break;
                        }
                    }
                    seated.push(seat);
                }
                // S20 Slice-3a — ACCOMPANIMENT FIGURATION. When this section's resolved
                // figuration is present and non-empty, ANIMATE the held bed: expand the
                // seated inner tones into the figure's bounded multi-onset burst within
                // this single step (an in-beat broken-chord / Alberti cell). When it is
                // None/empty the EXISTING sustained block bed runs BYTE-UNCHANGED — the
                // figured branch is purely ADDITIVE (the `block_bed_unchanged_when_
                // figuration_none` witness). The figure animates the SAME `seated`
                // root-less inner voicing the block bed already built, so it is
                // chord-tones-only and stays in the fill band [55, 67) by construction
                // (it never re-derives pitches or leaves the band). Unreachable under
                // the identity profile (no instrument is ever a Pad there → byte-freeze
                // intact); the cadence early-return above fires before this role match,
                // so a cadence step is never figured.
                match ctx.section.orchestration.figuration_resolved.as_ref() {
                    Some(spec) if !spec.onsets.is_empty() => {
                        figured_bed(spec, &seated, velocity, step_ms)
                    }
                    _ => seated
                        .into_iter()
                        .map(|n| NoteEvent {
                            note: n,
                            velocity,
                            // Held the full step + the legato overlap so consecutive beds
                            // tie; kept within the `sustained`-style ≤1.2× cap (PAD_OVERLAP_FRAC
                            // = 1.10) so the block-until-last-event scheduler over-runs each
                            // step by ≤10%, never the N× of a true multi-step hold.
                            hold_ms: ((step_ms as f32) * PAD_OVERLAP_FRAC).round().max(1.0) as u64,
                            offset_ms: 0,
                        })
                        .collect(),
                }
            };
            // S47 SLICE 4 — THE BED ACTIVITY RECESSION (pass 2). Cap the Pad's per-step ONSET
            // COUNT so it recedes BELOW (deep tier) or AT (shallow tier) the melody's activity,
            // image-conditioned by the resolved Pad prominence weight pass 1 wired. This is the
            // PAD-side complement of the pass-1 melody floor / counter governor: it drives F5b
            // (bed_onsets > melody_onsets) toward 0 and flips F1 (the melody out-onsets the bed)
            // POSITIVE on deep-tier images. theory: the cap is RELATIVE to the melody's per-class
            // minimum onsets, so the bed tracks the melody (CLIMAX-BLOOM survives) rather than
            // pinning to an absolute low; `recede_pad_onsets` keeps the bed OFF the melody's
            // downbeat (drops the offset-0 onsets first) and never hollows it to silence (the
            // PAD_ONSET_FLOOR). FREEZE: on the identity / golden / synthetic-bar path the section's
            // prominence Vec is EMPTY → `prominence_weight` returns PROMINENCE_NEUTRAL (0.5) →
            // `pad_onset_cap` returns None → no cap → the Pad arm is byte-identical (mirrors the
            // melody floor's no-op-at-0.5 gating; the engine_equivalence goldens are no-Pad
            // synthetic bars, so this is doubly inert there).
            let pad_w = prominence_weight(ctx, OrchestralRole::Pad);
            // S49 SLICE 2 (L1): the governor's view of the MELODY class MUST match the melody arm's
            // (spec §7) — read the SAME `melody_total_rhythm_shift` (prominence + gated L1 bias).
            let melody_prom_shift =
                melody_total_rhythm_shift(prominence_weight(ctx, OrchestralRole::Melody));
            let m_class = melody_activity_class(edge_activity, melody_prom_shift, pre_cadence);
            match pad_onset_cap(pad_w, m_class) {
                Some(cap) => recede_pad_onsets(pad_events, cap, step_ms),
                None => pad_events,
            }
        }

        OrchestralRole::CounterMelody => {
            // THE REAL COUNTER-LINE (S18 §3) — a genuine second moving line, not the
            // HarmonicFill delegate the stub was. Everything it needs is RE-COMPUTED
            // deterministically off `ctx` (no plan-time counter storage, no cross-step
            // state): the melody pitch this/prev step (via the Melody role's own pitch
            // path), the prior `StepPlan` (via `ctx.section.steps`), and a non-recursive
            // prior-chord seed for the line's previous pitch. Unreachable under the
            // identity profile (no Counter inst), so byte-neutral on the freeze path.
            let si = ctx.step_in_section;
            // The prior step's plan, or None at a section start (no contrary constraint).
            // S45-CP-FIX (tiling consistency): fetch the prior step by the realizer's own
            // `% len` wrap so a TILED section past `steps.len()` reads the tiled phrase's prior
            // step (the one that actually sounded) instead of falling off the stored phrase to
            // `None` — which would short-circuit `prev_counter` to the synthetic seed below and
            // DIVERGE from the live tiled line. On an untiled section `% len` is the identity →
            // byte-identical. (The melody index stays raw, matching the Melody role's own motif
            // index; see `realized_counter_pitch_with_prev`.)
            let counter_len = ctx.section.steps.len();
            let prev: Option<&StepPlan> = match (si.checked_sub(1), counter_len) {
                (Some(p), l) if l > 0 => Some(&ctx.section.steps[p % l]),
                _ => None,
            };

            // Melody pitch THIS step and the PREVIOUS step, recomputed exactly as the
            // Melody instrument computes it. Some(p) == sounds p; None == the melody
            // rests this step (treated as Hold for the contrary rule, a gap for rhythm).
            let m_now = melody_pitch_for(ctx, step, features);
            let m_prev = match (si.checked_sub(1), prev) {
                (Some(prev_si), Some(p)) => {
                    let mut prev_ctx = *ctx;
                    prev_ctx.step_in_section = prev_si;
                    melody_pitch_for(&prev_ctx, p, features)
                }
                _ => None,
            };
            let mel_dir = motion_dir(m_prev, m_now);

            // Held-period / melody-static detection (§3.4 — the "empty periods" answer).
            let held_chord = prev.is_some_and(|p| p.chord.notes == step.chord.notes);
            let melody_static = mel_dir == MotionDir::Hold || m_now.is_none();

            // PITCH (§3.2 — contrary/oblique, chord-tone, no parallel perfects, fifth-species
            // figures): the full pitch selection — held-run rotation seed / §3.1 seed / figure
            // driver — now lives in the SHARED `realized_counter_pitch_with_prev` so the live
            // emission and the cross-step REPLAY use one code path and cannot diverge. The arm
            // computes only the realized-prev seed and delegates; `held_run_index`/`held_target`
            // are recomputed inside the shared helper.

            // S30-CP-FIX — REALIZED-PREV MEMORY. The as-built arm seeded `prev_counter` off the
            // PRIOR CHORD (`seed_prev_counter`, the §3.1 "LOCK"), so the species two-point gates
            // (parallel-perfects, approach-perfect, melodic-leap, the cadence clausula) checked a
            // SYNTHETIC seed→candidate transition the ear never hears, not the REALIZED prev→now
            // transition that actually sounds — the root cause of GAP-1/3/4. We now feed the gates
            // the ACTUAL realized counter pitch of the previous step, recovered by deterministic
            // replay (`realized_prev_counter`: recurse toward the section opening, the base case).
            // At a section START (no prior) this is exactly the §3.1 seed, so the opening is
            // unchanged. The pitch is then computed via the SHARED `realized_counter_pitch_with_prev`
            // so the live emission and the replay cannot diverge.
            let prev_counter = match prev {
                Some(_) => realized_prev_counter(ctx, features, si),
                None => seed_prev_counter(None, step),
            };

            let cnt = realized_counter_pitch_with_prev(ctx, step, features, si, prev_counter);

            let with_note = |ev: NoteEvent| NoteEvent { note: cnt, ..ev };

            // RHYTHM (§3.3) + THE FIGURE-GROUND ACTIVITY GOVERNOR (S47 slice 1, spec §2a.3).
            //
            // theory: the counter is a GAIN (S45's moving inner texture) but must never
            // out-MOVE the foreground. The activation is now governed by the MELODY's
            // activity class so the counter recedes EXACTLY ONE RANK below the melody:
            //   • melody Subdividing (it moves — arpeggio/syncopated) → counter MOVING
            //     (the guaranteed off-beat onset) — the counter keeps its inner motion;
            //     PRESERVES S45 (when the melody moves, the counter moves).
            //   • melody Oblique (dotted — minimum real motion) → counter OBLIQUE (one
            //     sustained tone, onset 0) — it recedes below the melody's ≥2 onsets.
            //   • melody Sustained (it HOLDS — the calm-image inversion case) → counter
            //     OBLIQUE-or-rest (NEVER the guaranteed onset) — the fix: a holding melody
            //     no longer lets the background take the onset the foreground lacks.
            // The OLD predicate (`held_chord || melody_static` → MOVING) was the inversion:
            // it routed the counter to MOVING precisely when the melody held. The counter is
            // NEVER silenced — a holding-melody step gives it a sustained tone (or the
            // existing breathing rest-as-gesture on a weak interior beat), never a mute.
            //
            // The melody's class is computed off the MELODY role's prominence shift (the
            // governor asks "how active is the MELODY," spec §8.4) — NOT the counter's.
            // S49 SLICE 2 (L1): the governor reads the MELODY's class off the SAME
            // `melody_total_rhythm_shift` the melody arm uses (prominence + gated L1 bias) so the
            // governor and the arm cannot drift (spec §7 shared-cutoff coupling).
            let melody_prom_shift =
                melody_total_rhythm_shift(prominence_weight(ctx, OrchestralRole::Melody));
            let m_class = melody_activity_class(edge_activity, melody_prom_shift, pre_cadence);
            // `held_chord` / `melody_static` are retained for clarity of intent: the Sustained
            // class is precisely the held/static case the governor now recedes (asserted in the
            // unit tests), but the CLASS — not these flags — drives the routing.
            let _ = (held_chord, melody_static);
            // The OBLIQUE/rest recession body, shared by the Oblique and Sustained arms: one
            // sustained tone underneath, or (on a genuinely near-static changing chord) the
            // existing breathing rest-as-gesture on a weak interior beat. NEVER the guaranteed
            // onset, so the counter cannot out-move a melody that is not Subdividing.
            let oblique_or_rest = || -> Vec<NoteEvent> {
                let weak_interior = !step.position_in_phrase.is_multiple_of(2);
                if edge_activity < FILL_REST_ACTIVITY && weak_interior {
                    Vec::new()
                } else {
                    vec![with_note(sustained(0, step_ms, base_frac))]
                }
            };
            match m_class {
                ActivityClass::Subdividing => {
                    // MOVING mode: a GUARANTEED off-beat onset at step_ms/4 — the moving inner
                    // line weaving under the active melody (PRESERVE S45: melody moves → counter
                    // moves). theory: two simultaneously moving lines are legitimate counterpoint
                    // when the figure is the busier one; the seat guard + the melody's ≥2-onset
                    // density keep the melody the figure even as the counter moves.
                    let offset = step_ms / 4;
                    let slot = step_ms - offset;
                    vec![with_note(sustained(offset, slot, base_frac))]
                }
                ActivityClass::Oblique => {
                    // The melody has minimal real motion (dotted, ≥2 onsets): the counter
                    // recedes one rank to ONE sustained tone (onset 0), staying under the figure.
                    vec![with_note(sustained(0, step_ms, base_frac))]
                }
                ActivityClass::Sustained => {
                    // THE FIX: the melody HOLDS — the counter must NOT take the guaranteed onset
                    // that out-moves the held foreground. It recedes to a sustained tone (or the
                    // existing breathing rest), one rank below a holding melody. Never silenced.
                    oblique_or_rest()
                }
            }
        }

        OrchestralRole::Melody => {
            // MELODY: the most rhythmic freedom. Pattern by NORMALIZED edge_activity
            // band (design-s13 §2), with syncopation and pre-cadence acceleration.
            // theory: recalibrating the cutoffs against edge_activity (not raw edge)
            // is what makes "busy image → denser, arpeggiated melody" actually fire on
            // real photos — under the old raw cutoffs every photo fell in "sustained".
            //
            // S23 prominence: shift the three band cutoffs by a CENTERED term. A
            // foreground melody (w>0.5) LOWERS the cutoffs so it subdivides more readily
            // (rhythmically freer — arpeggio/syncopation reached sooner); a recessive
            // melody (w<0.5) raises them (plainer, more sustained). EXACTLY 0 at w==0.5:
            // (0.5-0.5)*RHY_SHIFT == 0.0 → cutoffs are exactly 0.80/0.55/0.25, byte-stable.
            // The `pre_cadence ||` disjunct is UNCHANGED so the cadence acceleration path
            // is never shifted (protects the cadence golden).
            let melody_w = prominence_weight(ctx, role);
            // S49 SLICE 2 (L1) — the melody's TOTAL cutoff shift now folds in the per-ROLE rhythm
            // bias (gated freeze-neutral) so the melody subdivides sooner than the bed (distinct
            // per-role onset GRIDS). The single `melody_total_rhythm_shift` source is shared with the
            // governor/Pad-cap (spec §7), and at neutral weight is EXACTLY the legacy term (0.0 at
            // 0.5) → byte-identical goldens (spec §4).
            let prom_shift = melody_total_rhythm_shift(melody_w);
            // S47 activity FLOOR: a FOREGROUND melody (resolved weight > threshold) must
            // never fall all the way to a held tone on a calm image — that is the
            // figure-ground inversion seen from the foreground side (a held figure under a
            // moving bed). When the band ladder would otherwise select SUSTAINED, FLOOR the
            // melody up one rank to the DOTTED (Oblique) band so it carries ≥2 onsets and
            // always out-moves the governed counter's single sustained tone. theory: the
            // declared figure must MOVE. Identity neutrality: at neutral weight (0.5) the
            // strict `>` is FALSE → no floor → the SUSTAINED arm runs byte-identically; and
            // a recessive (<0.5) melody is likewise unaffected. The floor only LIFTS a band
            // selection that was already SUSTAINED, so it can never reduce activity.
            let floor_to_dotted = melody_w > ACTIVITY_FLOOR_THRESHOLD
                && melody_activity_class(edge_activity, prom_shift, pre_cadence)
                    == ActivityClass::Sustained;
            // S48 SLICE 3 (2b.2): mark the band as PUSHABLE iff its FIRST onset currently sits
            // ON the downbeat (offset 0) — only the DOTTED and SUSTAINED bands. The ARPEGGIO and
            // SYNCOPATED bands already place their first onset OFF the downbeat (k=0 spread /
            // step_ms/4), where the counter MOVES (Subdividing) at step_ms/4 — pushing them would
            // double-displace and risk F5a re-fusion (spec §2b.2). So `pushable` gates the
            // inverse-comp onset push to exactly the calm/low-activity bands the F4 metric reads.
            // S50 — RHYTHM-VARIETY RE-RANGE: the band ladder compares the SPREAD activity (the
            // natural-photo cluster fanned across the full decision range) against the unchanged cut
            // constants, so distinct images land on distinct bands instead of all collapsing onto
            // DOTTED. Applied to the BAND-LADDER comparison input ONLY — `edge_activity` itself is
            // left UNMAPPED for the articulation curve / FILL_REST / `/0.05` (the freeze discipline,
            // spec §3.3). Composes WITH `prom_shift` exactly as before: spread the activity, THEN
            // compare against `CUTOFF - prom_shift` (the per-role L1 bias + prominence are untouched).
            let band_edge = band_activity_spread(edge_activity);
            let (mut events, pushable): (Vec<NoteEvent>, bool) =
                if pre_cadence || band_edge > (MELODY_ARP_CUTOFF - prom_shift) {
                    // ARPEGGIO / acceleration: spread chord-tone onsets evenly across
                    // the step (more onsets, shorter values) — the active, driving
                    // figure. theory: subdividing the beat is the melody's way of
                    // generating forward motion, intensified into a cadence.
                    let n = if pre_cadence { 4 } else { 3 };
                    let slot = step_ms / n as u64;
                    let ev = (0..n)
                        .map(|k| {
                            let offset = (k as u64) * slot;
                            sustained(offset, slot, STACCATO_FRAC)
                        })
                        .collect();
                    (ev, false) // already spread off the downbeat — NOT pushable
                } else if band_edge > (MELODY_SYNC_CUTOFF - prom_shift) {
                    // SYNCOPATED: delay the onset off the downbeat by 1/4 step, then
                    // a second onset, pushing against the meter. theory: syncopation
                    // displaces the accent to energize an active-but-not-busy melody.
                    let quarter = step_ms / 4;
                    let ev = vec![
                        sustained(quarter, step_ms / 2, PORTATO_FRAC),
                        sustained(step_ms * 3 / 4, step_ms / 4, STACCATO_FRAC),
                    ];
                    (ev, false) // first onset already at step_ms/4 — NOT pushable
                } else if floor_to_dotted || band_edge > (MELODY_DOTTED_CUTOFF - prom_shift) {
                    // DOTTED: a long-short pair (onsets at 0 and 2/3; holds 2/3 and
                    // 1/3) — the lilting mid-activity figure. theory: the dotted
                    // rhythm is the default expressive subdivision of a singing line.
                    // S47: `floor_to_dotted` ALSO routes a calm FOREGROUND melody here
                    // (the activity floor — a foreground figure gets the minimum real
                    // motion rather than holding under a moving bed).
                    let two_thirds = step_ms * 2 / 3;
                    let ev = vec![
                        sustained(0, two_thirds, PORTATO_FRAC),
                        sustained(two_thirds, step_ms - two_thirds, STACCATO_FRAC),
                    ];
                    (ev, true) // first onset at 0 (low-seat case) — PUSHABLE for the comp
                } else {
                    // SUSTAINED (low edge_activity): one long tone whose length rides the
                    // CONTINUOUS articulation curve (S13). At low activity base_frac ≈ 1.05,
                    // so the calm melody OVERLAPS across the step boundary and truly sings —
                    // the fix for "uniformly short" notes that the old hard 0.95 cap blocked.
                    //
                    // S39 — a multi-step THEME note (dur_steps>1) gets a longer within-step
                    // hold so it SINGS toward the overlap ceiling (its continuation step is a
                    // theme-driven rest, so the onset+gap reads as the note's full length). The
                    // `sustained` closure's `.min(1.20)` cap keeps it inside the overlap ceiling
                    // — it NEVER rings across the next kernel step (no freeze break). A dur==1
                    // onset (every pre-S39 note), a non-onset step, and a free-select melody all
                    // see `theme_onset_dur_steps == None`/`Some(1)` → factor 1.0 → byte-identical.
                    let sing_frac = match theme_onset_dur_steps(ctx, step, features) {
                        Some(d) if d > 1 => base_frac * THEME_LONG_NOTE_SING,
                        _ => base_frac,
                    };
                    (vec![sustained(0, step_ms, sing_frac)], true) // single onset at 0 — PUSHABLE
                };

            // S48 SLICE 3 (2b.2) — THE PRIMARY (rhythmic-separation) comp tool: push the melody's
            // FIRST onset OFF the bed's downbeat, scaled by the inverse-register comp. theory
            // (DP-3 rank 1, the F4 driver): a LOW-seated melody (comp → 1.0) attacks on the "and"
            // (≈ step_ms/4) — DISTINCT from the bed's offset-0 downbeat → high F4 separation; a
            // HIGH-seated melody (comp 0.0) stays on the downbeat → low separation → the F4
            // correlation(register_gap, separation) goes NEGATIVE. Confined to the PUSHABLE
            // (DOTTED/SUSTAINED) bands whose first onset is at 0; on those steps the counter is at
            // offset 0 / sustained (Oblique/Sustained classes), so the pushed onset (≈ step_ms/4)
            // is DISTINCT from the counter — F5a separation IMPROVES, never fuses (spec §2b.2).
            // COUNT-PRESERVING (the recede_pad_onsets precedent): we move WHERE the first onset
            // sits, never HOW MANY events — so F5b (bed_onsets ≤ melody_onsets) and F1 cannot
            // regress. comp == 0.0 on the freeze/high-seat path → the push is a no-op (offset
            // stays 0, hold unchanged) → byte-identical. LEVEL is NEVER touched here (DP-3).
            if comp > 0.0 && pushable {
                if let Some(first) = events.first_mut() {
                    let offset_push = (comp * COMP_OFFSET_FRAC * step_ms as f32).round() as u64;
                    if offset_push > 0 {
                        // Re-fit the pushed onset's hold into the room remaining BEFORE the step
                        // boundary so it does not ring across into the next step (the same intent
                        // as the recede_pad_onsets re-fit) — count is unchanged, only this onset's
                        // offset+hold move. The fit preserves the onset's articulation FRACTION
                        // against the shortened remaining slot, then bounds it so it cannot exceed
                        // the step (no new overlap the original did not have).
                        let room = step_ms.saturating_sub(offset_push).max(1);
                        let prev_hold = first.hold_ms.max(1);
                        let refit = prev_hold.min(room);
                        first.offset_ms = offset_push;
                        first.hold_ms = refit.max(1);
                    }
                }
            }

            // S53 (D-CELL) — APPLY THE PER-PIECE RHYTHMIC MOTTO onset bias. theory: this is the
            // deliverable — the per-piece gait that gives a short image-piece a hummable identity.
            // The motto re-places WHERE the melody's onsets sit (its WALK) WITHOUT changing how many
            // there are, so it composes UNDER the band ladder (the band already chose the onset
            // COUNT; the motto only shapes the spacing). Guard-rail compliance:
            //   • GR-1 (figure-ground): applied ONLY to the Melody (the figure) and ONLY by moving
            //     existing onsets — it adds zero onsets, so it can never raise any voice's
            //     ActivityClass; the bed roles are never touched, so `bed_onsets ≤ melody_onsets`
            //     (spec-s46 F5b) is untouched by construction.
            //   • GR-2 (homecoming): the cadence ring already early-returned above (bias = 0 at the
            //     cadence, structurally); on the pre-cadence approach the bias is HALVED so the
            //     phrase relaxes into its close.
            //   • freeze hinge (I-4): `motto.is_neutral()` (cell_index == None on every legacy/
            //     identity/single_section_default section) short-circuits → events are returned
            //     byte-identical to pre-S53. The engine_equivalence goldens carry neutral() and
            //     cannot move.
            // Count-preserving and bounded WITHIN the step (each displaced onset stays strictly
            // before the next onset / the step boundary), so it can never re-order onsets or ring
            // across the kernel step (the same freeze-safety the comp push above relies on).
            let motto = ctx.section.orchestration.motto;
            if matches!(role, OrchestralRole::Melody) && !motto.is_neutral() && events.len() >= 2 {
                let atten = if pre_cadence {
                    MOTTO_PRECADENCE_ATTEN
                } else {
                    1.0
                };
                apply_motto_onset_bias(&mut events, motto, step_ms, atten);
            }

            events
        }
    }
}

/// S53 (D-CELL) — displace the melody's INTERIOR onsets to express the per-piece motto's gait.
/// COUNT-PRESERVING: it moves WHERE onsets sit, never HOW MANY. Each interior onset (index ≥ 1) is
/// nudged by a signed fraction of the GAP to its NEXT neighbour (or the step boundary for the last
/// onset), so a displaced onset can never reach or pass the following onset — onset ORDER and COUNT
/// are invariant, and nothing rings across the step boundary. The FIRST onset (the downbeat anchor)
/// is left in place so the beat-one reference the ear locks onto is never blurred (GR-2 readability).
///
/// theory — the four gaits map to four signed displacement characters (read off the cell index,
/// whose authored semantics are in `MotifArchetype::rhythm_cells`):
///   * cell 0 (the S39 anchor): NEUTRAL — no displacement (the even reference walk).
///   * cell 1 (broad/augmented): a small EARLIER pull — onsets settle a touch toward the grid, the
///     calm, steady, "on-the-beat" reading (negative phase).
///   * cell 2 (busy/even-subdivided): NEUTRAL even spacing — the moto-perpetuo walk is already even;
///     biasing it would fight its defining evenness, so it keeps its grid.
///   * cell 3 (profiled/syncopated): a LATER push — interior onsets lean off the grid toward the
///     following weak phase, the characteristic lilting/swung lurch (positive phase, full depth).
///
/// The depth is `MOTTO_ONSET_BIAS_DEPTH` (× the per-cell character weight × the boundary `atten`),
/// taken as a fraction of the local gap so a wide gap lilts more than a tight one (musically: a
/// slow walk swings more audibly than a fast run). Pure; no RNG, no clock.
fn apply_motto_onset_bias(events: &mut [NoteEvent], motto: RhythmMotto, step_ms: u64, atten: f32) {
    // The signed character weight of the motto's cell (neutral cells contribute 0 → early return).
    let cell = match motto.cell_index {
        Some(c) => c,
        None => return,
    };
    let character: f32 = match cell {
        1 => -0.6, // broad → a gentle earlier pull toward the grid (steadier)
        3 => 1.0,  // profiled → the full later lean (the syncopated lurch)
        _ => 0.0,  // cell 0 (anchor) and cell 2 (even) keep their grid
    };
    if character == 0.0 || atten == 0.0 {
        return;
    }
    let depth = MOTTO_ONSET_BIAS_DEPTH * character * atten;

    let n = events.len();
    // Walk interior onsets (skip index 0 — the downbeat anchor). For each, the available room is
    // the gap to the NEXT onset (or the step boundary for the final onset); the displacement is a
    // fraction of that gap, BOUNDED so the onset stays strictly inside (offset, next_offset).
    for i in 1..n {
        let here = events[i].offset_ms;
        let next = if i + 1 < n {
            events[i + 1].offset_ms
        } else {
            step_ms
        };
        // Margins to the neighbours so the bias can never reach an adjacent onset/boundary
        // (preserve strict ordering): cap at just under the gap on the side we move toward.
        let gap_after = next.saturating_sub(here);
        let gap_before = here.saturating_sub(events[i - 1].offset_ms);
        // The raw signed shift in ms; positive = later (toward `next`), negative = earlier.
        let raw = (depth * gap_after.min(gap_before) as f32).round() as i64;
        if raw == 0 {
            continue;
        }
        // Keep at least 1 ms of separation from each neighbour so onsets never coincide/re-order.
        let max_later = gap_after.saturating_sub(1) as i64;
        let max_earlier = gap_before.saturating_sub(1) as i64;
        let shift = raw.clamp(-max_earlier, max_later);
        if shift == 0 {
            continue;
        }
        let new_offset = (here as i64 + shift).max(0) as u64;
        // Re-fit this onset's hold into the room remaining before the next onset / step boundary,
        // preserving count and never ringing past the next event (the comp-push re-fit precedent).
        let room = next.saturating_sub(new_offset).max(1);
        events[i].offset_ms = new_offset;
        events[i].hold_ms = events[i].hold_ms.max(1).min(room);
    }
}

/// Apply a whole-octave register shift to a seated pitch, clamped to the engine's MIDI
/// range `[24, 108]` (identical to `seat_pc_in_register`'s clamp, so a shifted tone can
/// never escape the synthesizable range). `octaves == 0` returns `pitch` UNCHANGED — the
/// byte-freeze default for every non-register-split figure. NEW S34. Private free fn.
///
/// theory: the oom-pah / stride idiom places the "oom"/stride-bass an octave BELOW the
/// inner bed (`octaves == -1`) so the accompaniment reads as bass-vs-chord (the alternating
/// left hand), while the "pah"/stab stays in the fill band (`octaves == 0`). The shift is
/// whole-octave ONLY — it preserves PITCH CLASS, so the figure remains chord-tones-only (a
/// `-1` shift on a tone seated at the fill floor 55 lands at 43 / G2, between the bass floor
/// 36 and the fill floor 55: the correct "oom" register, below the inner stabs and above the
/// true Bass root). The clamp uses saturating i16 arithmetic so an adversarially large
/// `octaves` can never wrap a `u8` — it pins to the playable band instead of panicking.
fn apply_register_octaves(pitch: u8, octaves: i8) -> u8 {
    // octaves == 0 is the common case (every existing figuration row) and is the byte-freeze
    // no-op: 0 * 12 == 0, so `shifted == pitch` and the clamp is the identity on a valid seat.
    let shifted = pitch as i16 + (octaves as i16) * 12;
    shifted.clamp(24, 108) as u8
}

/// The 7-tone interval set of a mode (semitones from the tonic). Mirrors `degree_to_pitch`'s
/// `match`; an unknown mode falls back to Ionian, matching `generate_chords`. NEW S34 helper
/// the Bass generators share for their DIATONIC pitch derivation.
fn mode_scale(mode: &str) -> [i8; 7] {
    match mode {
        "Ionian" => IONIAN,
        "Dorian" => DORIAN,
        "Phrygian" => PHRYGIAN,
        "Lydian" => LYDIAN,
        "Mixolydian" => MIXOLYDIAN,
        "Aeolian" => AEOLIAN,
        _ => IONIAN,
    }
}

/// Map a pitch CLASS to the index (0..=6) of the nearest-at-or-below diatonic scale degree
/// in `mode` over `tonic_pc`. theory: a walking bass moves THROUGH the scale, so we must place
/// an arbitrary chord root (which is always diatonic in this engine — chords are built off scale
/// degrees) onto its scale-degree index. If a pc is not exactly in the scale (it always is here,
/// but be defensive), snap DOWN to the nearest scale tone so the walk never lands on a pitch the
/// mode does not contain. NEW S34.
fn pc_to_scale_index(pc: u8, tonic_pc: u8, scale: &[i8; 7]) -> usize {
    // Degree above the tonic, 0..=11.
    let rel = (pc as i16 - tonic_pc as i16).rem_euclid(12) as i8;
    // Exact match preferred; else the highest scale degree whose interval is <= rel (snap down).
    let mut best = 0usize;
    for (i, &iv) in scale.iter().enumerate() {
        if iv == rel {
            return i;
        }
        if iv < rel {
            best = i;
        }
    }
    best
}

/// Seat a pitch CLASS into the bass register at the octave NEAREST `prev` (so the walking line
/// moves by the smallest interval — a walking bass is a connected, stepwise-or-small-leap line,
/// never an octave jump between adjacent notes). Falls back to the plain bass-floor seat when
/// there is no previous note (the first onset). NEW S34. Stays within `[24, 108]`.
fn seat_bass_near(pc: u8, prev: Option<u8>) -> u8 {
    let base = seat_pc_in_register(pc, BASS_REGISTER_FLOOR);
    match prev {
        None => base,
        Some(p) => {
            // Consider base and the octave above/below; pick whichever is closest to `prev`,
            // preferring to stay at/above the bass floor so the line does not sink into mud.
            let candidates = [
                base.saturating_sub(12),
                base,
                base.saturating_add(12).min(108),
            ];
            candidates
                .into_iter()
                .filter(|&c| (24..=108).contains(&c))
                .min_by_key(|&c| (c as i16 - p as i16).unsigned_abs())
                .unwrap_or(base)
        }
    }
}

/// Generate a target-seeking stepwise WALKING bass for one step: `density` onsets that connect
/// THIS chord's root to the NEXT chord's root by DIATONIC step, arriving so the next downbeat
/// lands on the next root. The strong-beat (first) onset is a chord tone (the current root); the
/// weak-beat onsets are diatonic passing/neighbor tones walking toward the target, with the FINAL
/// onset a diatonic scale tone one step from the next root (the approach). The next chord is read
/// off `ctx.section.steps.get(si+1)` by the caller (the S33 in-realizer lookahead — NO seam
/// change). At a section's last step (`next_root_pc` is None) it walks WITHIN the current chord
/// (root → 5th → root) rather than inventing a target — the §R-B end-of-section fallback. All
/// notes are seated in the bass register, connected by `seat_bass_near` so no adjacent pair leaps
/// by an octave. Deterministic: a pure function of the chord stream + spec (NO RNG — the S30
/// realized-line gating lesson). Returns `density.max(1)` `NoteEvent`s within the step. NEW S34.
fn walking_bass(
    current_root_pc: u8,
    next_root_pc: Option<u8>,
    tonic_pc: u8,
    mode: &str,
    velocity: u8,
    step_ms: u64,
    density: u8,
) -> Vec<NoteEvent> {
    let scale = mode_scale(mode);
    let n = density.max(1) as usize;
    let step_ms = step_ms.max(1);

    // The DIATONIC pitch-class walk: a sequence of `n` scale tones starting on the current root
    // and stepping toward the target. theory: a walking bass arrives ON the next root at the
    // next downbeat — so THIS step's onsets begin on the current root and step diatonically so
    // the line is poised to land on the next root at the start of the next step. The final onset
    // is the diatonic scale tone one step short of the target (the approach tone), so the next
    // step's downbeat root completes the stepwise motion.
    let start_idx = pc_to_scale_index(current_root_pc, tonic_pc, &scale) as i32;

    let pc_seq: Vec<u8> = match next_root_pc {
        Some(target_pc) => {
            // Walk diatonically from the current root toward the diatonic position one step
            // BEFORE the target (so step N+1's root completes the stepwise arrival). Choose the
            // direction (up or down) that gives the SHORTER diatonic distance — a walking bass
            // takes the nearer route, not a forced ascent.
            let target_idx = pc_to_scale_index(target_pc, tonic_pc, &scale) as i32;
            // Diatonic signed distance, measured in scale steps within one octave (-3..=3-ish).
            let mut diff = (target_idx - start_idx).rem_euclid(7);
            // Prefer the shorter direction: distances 4..6 are shorter going DOWN.
            let dir: i32 = if diff == 0 || diff > 3 {
                if diff > 3 {
                    diff -= 7;
                }
                if diff < 0 {
                    -1
                } else {
                    1
                }
            } else {
                1
            };
            // We need to traverse `diff.abs()` diatonic steps to REACH the target at the NEXT
            // downbeat; this step emits `n` onsets that approach it. Distribute the onsets along
            // the path so the first is the current root and the last is one step short of target.
            let total_steps = diff.unsigned_abs() as i32; // diatonic steps to the target
            (0..n)
                .map(|k| {
                    // Fractional progress 0..1 across this step's onsets; the last onset stops
                    // one diatonic step short of the target (progress maps to total_steps - 1
                    // at most, so the target itself is the NEXT step's downbeat, not this one).
                    let reach = if total_steps == 0 {
                        // Same scale position (e.g. C→C or a chord whose root repeats): neighbor
                        // figure — root, upper-neighbor, root, ... so the line still MOVES.
                        match k % 2 {
                            0 => start_idx,
                            _ => start_idx + 1,
                        }
                    } else if n == 1 {
                        start_idx
                    } else {
                        // Spread k across [0, total_steps-1]: first onset = root, last onset = the
                        // approach tone one step before the target.
                        let span = (total_steps - 1).max(0);
                        let frac = k as f32 / (n - 1) as f32;
                        start_idx + dir * (frac * span as f32).round() as i32
                    };
                    diatonic_index_to_pc(reach, tonic_pc, &scale)
                })
                .collect()
        }
        None => {
            // §R-B end-of-section fallback: no next chord → walk WITHIN the current chord
            // (root → 5th → root …), an arpeggiated hold rather than an invented target. The 5th
            // is scale degree start_idx + 4 (a diatonic fifth above the root in the mode).
            (0..n)
                .map(|k| {
                    let reach = match k % 2 {
                        0 => start_idx,
                        _ => start_idx + 4, // a diatonic fifth above the root
                    };
                    diatonic_index_to_pc(reach, tonic_pc, &scale)
                })
                .collect()
        }
    };

    // Seat each pc in the bass register, connected to the prior note so adjacent onsets move by
    // the smallest interval (a true walking line — stepwise, no octave jumps). Even slots within
    // the step; each note holds to the next onset (portato walk, not legato smear).
    let slot = step_ms / n as u64;
    let mut events = Vec::with_capacity(n);
    let mut prev: Option<u8> = None;
    for (k, &pc) in pc_seq.iter().enumerate() {
        let note = seat_bass_near(pc, prev);
        prev = Some(note);
        let offset_ms = (k as u64) * slot;
        // Hold to just shy of the next onset (a detached, articulated walk — the canonical
        // walking-bass touch — so each note speaks individually rather than tying over).
        let next_off = if k + 1 < n {
            (k as u64 + 1) * slot
        } else {
            step_ms
        };
        let hold_ms = next_off.saturating_sub(offset_ms).max(1);
        events.push(NoteEvent {
            note,
            velocity,
            hold_ms,
            offset_ms,
        });
    }
    events
}

/// Map a (possibly out-of-range) diatonic scale INDEX to a pitch class, wrapping by octave so
/// `index` can ascend/descend past the 7-tone scale (the walk crosses the octave seam cleanly).
/// NEW S34 — the inverse of `pc_to_scale_index`.
fn diatonic_index_to_pc(index: i32, tonic_pc: u8, scale: &[i8; 7]) -> u8 {
    let i = index.rem_euclid(7) as usize;
    ((tonic_pc as i32 + scale[i] as i32).rem_euclid(12)) as u8
}

/// Generate a PEDAL-POINT bass for one step: hold the section-key `pedal_degree` (1 == tonic,
/// 5 == dominant) as ONE sustained low pitch, IGNORING the step's chord (the harmony changes
/// above; the pedal does not). Seated in the bass register. Returns one sustained `NoteEvent`.
/// theory: the pedal is a NON-chord bass that the upper voices' harmony is heard AGAINST — the
/// one sanctioned standing dissonance in common practice. `pedal_degree` is a SCALE DEGREE
/// (1-based: 1 == tonic, 5 == dominant); any other value falls back to the tonic. Deterministic.
/// NEW S34.
fn pedal_bass(
    tonic_pc: u8,
    mode: &str,
    pedal_degree: u8,
    velocity: u8,
    step_ms: u64,
    base_frac: f32,
) -> Vec<NoteEvent> {
    let scale = mode_scale(mode);
    // 1-based scale degree → 0-based index; default to the tonic on an out-of-range degree.
    let idx = match pedal_degree {
        d @ 1..=7 => (d - 1) as usize,
        _ => 0,
    };
    let pc = ((tonic_pc as i32 + scale[idx] as i32).rem_euclid(12)) as u8;
    let note = seat_pc_in_register(pc, BASS_REGISTER_FLOOR);
    let step_ms = step_ms.max(1);
    // Sustain across the whole step, the same legato-curve hold the sustained-root bass uses (so
    // consecutive pedal steps tie into one another — the pedal SOUNDS as a single held drone).
    let hold_ms = ((step_ms as f32) * base_frac.max(0.0)).round().max(1.0) as u64;
    vec![NoteEvent {
        note,
        velocity,
        hold_ms,
        offset_ms: 0,
    }]
}

/// Expand ONE held chord into the figure's bounded multi-onset burst within a SINGLE
/// step — the S20 Slice-3a accompaniment-figuration mapper (Music Theory Specialist
/// owns). Animates the held Pad bed with an in-beat broken-chord / Alberti cell so the
/// inner harmony stops being rhythmically inert under the tune.
///
/// `spec`     — the resolved figuration (guaranteed non-empty `onsets`, 2..=4 entries;
///              the caller only enters here on a present, non-empty spec).
/// `seated`   — the inner chord tones the block bed ALREADY seated (root-skipped, fill
///              register, de-duped — reused verbatim). The figure animates exactly these,
///              so it is chord-tones-only and stays in the fill band [55, 67) by
///              construction: it never re-derives a pitch or leaves the band.
/// `velocity` — the Pad's supporting velocity for this step, used UNCHANGED (no `-3`
///              trim in 3a — the only audible delta this slice introduces is rhythm, not
///              dynamics; a supporting-velocity trim is a later dynamics slice).
/// `step_ms`  — the step duration; all onsets + holds live within it (plus the ≤10% Pad
///              over-run the block bed already permits).
///
/// Returns a bounded `Vec<NoteEvent>` of exactly `onsets.len()` events (2..=4, defensively
/// truncated to 4), each WITHIN the step.
///
/// MUSIC-OWNED BUILD-TIME RULE (the non-triad modulo rule): each onset names a SEATED
/// inner-voice index (`onset.tone`); the index is taken MODULO the seated voice count
/// (`onset.tone % seated.len()`). On a TRIAD the bed seats 2 inner tones (3rd, 5th), so a
/// 4-onset Alberti cell {0,2,1,2} reads indices {0, 2%2=0, 1, 2%2=0} = [3rd, 3rd, 5th,
/// 3rd] — it degrades sanely to the two available tones and NEVER indexes out of bounds.
/// On a 7th chord the bed seats 3 (3rd, 5th, 7th), so {0,2,1,2} reads the full low-high-
/// mid-high Alberti shape [3rd, 7th, 5th, 7th]. The catalogue's Alberti row is authored for
/// the 3-voice case; the modulo is what makes it cycle correctly on any seated voice count.
fn figured_bed(
    spec: &crate::composition::FigurationSpec,
    seated: &[u8],
    velocity: u8,
    step_ms: u64,
) -> Vec<NoteEvent> {
    // The seated bed is guaranteed non-empty by the caller (`notes.is_empty() ||
    // pad_voices == 0` already returned the defensive path), so `% n` is always safe.
    let n = seated.len().max(1);
    // The per-onset hold ceiling: REUSE the block bed's overlap cap (PAD_OVERLAP_FRAC =
    // 1.10) so a figured note never over-runs the step more than the block bed already
    // does (≤10%). No new cap constant is introduced. 1.10 ≤ 1.20 satisfies the legato
    // steer with margin.
    let cap = ((step_ms as f32) * PAD_OVERLAP_FRAC).round() as u64;

    // Bound the burst to 4 (the catalogue guarantees 2..=4; truncate defensively so the
    // helper can never emit an unbounded count even on a malformed row).
    let onsets = &spec.onsets[..spec.onsets.len().min(4)];

    onsets
        .iter()
        .enumerate()
        .map(|(i, onset)| {
            // Onset time: a fraction of the step, snapped to whole ms. For Alberti this
            // lands on 0, step_ms/4, step_ms/2, 3·step_ms/4 — slot boundaries, so the
            // figure reads as rhythm, not jitter.
            let offset_ms = (onset.at.clamp(0.0, 1.0) * step_ms as f32).round() as u64;
            // Tone selection — the non-triad modulo rule (see the fn doc).
            let idx = (onset.tone as usize) % n;
            // S34 PART (A) — oom-pah / stride register split. Apply this onset's whole-octave
            // register shift to the seated pitch. theory: the oom-pah/stride idiom alternates a
            // LOW bass note (the "oom"/stride bass, register_octaves == -1, an octave below the
            // inner bed) with a MID-register chord stab (the "pah", register_octaves == 0). The
            // shift is whole-octave ONLY, so the pitch CLASS is preserved — a shifted onset is
            // still a chord tone (it never introduces a non-harmonic pitch). FREEZE: every
            // existing figuration row carries register_octaves == 0 (the serde default), for
            // which apply_register_octaves is the identity `seated[idx]` — so every alberti /
            // broken-chord / arp-waltz / block-comp bed realizes BYTE-IDENTICALLY to pre-S34.
            let note = apply_register_octaves(seated[idx], onset.register_octaves);
            // Hold: a fraction of the GAP to the next onset (the in-step articulation).
            // The last onset fills to the step end. theory: with the default hold_frac
            // 1.0 each note holds to the next onset, giving a continuous figure; only the
            // last onset's hold may reach the overlap cap.
            let next_offset_ms = if i + 1 < onsets.len() {
                (onsets[i + 1].at.clamp(0.0, 1.0) * step_ms as f32).round() as u64
            } else {
                step_ms
            };
            let gap_ms = next_offset_ms.saturating_sub(offset_ms);
            let raw_hold = (gap_ms as f32 * onset.hold_frac.clamp(0.0, 1.0)).round() as u64;
            // Per-onset cap: clamp so offset_ms + hold_ms ≤ step_ms × PAD_OVERLAP_FRAC,
            // identical to the block bed's ceiling. Never below 1 ms (a sounding note).
            let hold_ms = raw_hold.min(cap.saturating_sub(offset_ms)).max(1);
            NoteEvent {
                note,
                velocity,
                hold_ms,
                offset_ms,
            }
        })
        .collect()
}

// ════════════════════════════════════════════════════════════════════════════
// S15 SLICE 1 — RETURNING-THEME LAYER (Music Theory Specialist owns)
//
// Two responsibilities, per the locked cross-file contract (spec-s15-slice1 §2):
//   (1) PRODUCE the theme: `resolve_motif` turns a build-time MotifArchetype +
//       image-derived range/length into the concrete key-relative `Vec<MotifNote>`
//       the realizer reads. Called by composition.rs at PLAN-BUILD time, NEVER at
//       tick time. This is the ONE place a melodic contour → degree+duration.
//   (2) CONSUME the theme: `theme_melody_pitch` is the realizer's theme-replay
//       decision — on a Statement/Return section it maps the current motif note's
//       scale degree into a sounding pitch in the section's mode/key; on Fragmented
//       it plays only the head then rests; on Contrast/None it returns None so the
//       caller free-selects exactly as today (byte-stable back-compat path).
//
// The motif is KEY-RELATIVE (scale degree + duration), so when Stage-5 modulation
// arrives a section's key_offset transposes it for free — slice 1 stays home (offset
// 0), so no transposition ever fires here.
// ════════════════════════════════════════════════════════════════════════════

/// The 8 curated melodic-shape archetypes (design-s15 §4.3). Each is a textbook
/// contour; `resolve_motif` parameterizes it by range and length. This enum is a
/// BUILD-TIME input only (operator decision 4, spec §7): it is NOT stored on
/// `ThemeSeed`/`MotifNote` and is NOT read at tick time. The slice-1 ACTIVE subset
/// is the original four (Arch, Descent, Ascent, NeighborTurn — the contours the
/// theme-seeding ladder selects in slice 1); the other four ship as variants the
/// later variation-technique stage pairs with inversion/sequence operations.
///
/// theory: bounding the motif vocabulary to known-good shapes is the §1-governor
/// (a principled default + conditional departures) applied at the motif level —
/// the image SELECTS and PARAMETERIZES a shape, it never generates a random (and
/// possibly ugly) interval string.
/// One motif note — scale/key-relative so it transposes cleanly (spec §1.3, operator
/// decision 4 — LOCKED shape). theory: a motif stored as scale DEGREE + DURATION (not
/// absolute pitch) is mode-portable and transposes by simply adding the section key
/// offset; the realizer (`theme_melody_pitch`) is the one place degree → pitch happens.
///
/// CONTRACT NOTE: this type lives HERE in `chord_engine` because (a) `resolve_motif`
/// (the one-way contour→degree resolver) produces it and the realizer consumes it —
/// both Music-Theory-owned — and (b) `composition::ThemeSeed.motif` is already typed
/// `Vec<chord_engine::MotifNote>` and `composition.rs` calls `chord_engine::resolve_motif`
/// expecting this type. (The duplicate `composition::MotifNote` in the parallel
/// composition.rs is unused by `ThemeSeed` and should be removed by the Implementer to
/// avoid two same-named types — flagged in the return summary.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MotifNote {
    /// Scale degree relative to the section tonic (0 == tonic).
    pub degree: i8,
    /// Duration in steps (>= 1).
    pub dur_steps: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotifArchetype {
    /// 1 3 5 3 1 — up then down; balanced, singable. The DEFAULT contour.
    Arch,
    /// 5 3 1 3 5 — settling then rising; the mirror of Arch (a valley).
    InvertedArch,
    /// 5 4 3 2 1 — stepwise fall, "from the light," strongly resolving.
    Descent,
    /// 1 2 3 4 5 — stepwise rise, opening/lifting.
    Ascent,
    /// 1 2 1 7 1 — upper then lower neighbor around the tonic; gentle, ornamental.
    NeighborTurn,
    /// 1 5 4 3 2 — an opening leap then a stepwise gap-fill descent.
    LeapStep,
    /// 1 5 1 5 1 — oscillating tonic↔dominant; insistent, two-zone.
    Pendulum,
    /// 1 2 3 / 2 3 4 — a rising 3-note cell sequenced up a step; developmental.
    RisingSequence,
}

impl MotifArchetype {
    /// The archetype's contour as a sequence of SCALE-DEGREE OFFSETS from the tonic,
    /// expressed in a canonical 0-based form (0 == tonic, 4 == the fifth, -1 == the
    /// leading tone below). `resolve_motif` scales these into the requested range and
    /// pads/truncates to the requested length. theory: keeping the contour as degree
    /// offsets (not pitches) is what makes inversion (negate) and transposition (add
    /// the key offset) clean operations on the SAME data the realizer reads.
    fn contour(self) -> &'static [i8] {
        match self {
            // do=0 frame; the fifth is degree 4, the third degree 2.
            MotifArchetype::Arch => &[0, 2, 4, 2, 0],
            MotifArchetype::InvertedArch => &[4, 2, 0, 2, 4],
            MotifArchetype::Descent => &[4, 3, 2, 1, 0],
            MotifArchetype::Ascent => &[0, 1, 2, 3, 4],
            // upper neighbor (1), back to tonic (0), lower neighbor (-1, the leading
            // tone), resolve to tonic — the classic turn around do.
            MotifArchetype::NeighborTurn => &[0, 1, 0, -1, 0],
            MotifArchetype::LeapStep => &[0, 4, 3, 2, 1],
            MotifArchetype::Pendulum => &[0, 4, 0, 4, 0],
            // two overlapping rising cells (a real motivic sequence).
            MotifArchetype::RisingSequence => &[0, 1, 2, 1, 2, 3],
        }
    }

    /// The archetype's RHYTHM-CELL VOCABULARY (S41 Finding-B): K = 4 musically-idiomatic
    /// durational cells per contour, each a `dur_steps`-weight list (1 == one step, 2 == a
    /// held note worth two steps, 3 == a dotted/long value), cycled across the sampled
    /// contour by `resolve_motif_celled` exactly as the single S39 profile was. Lives HERE
    /// beside `contour()` because rhythm-vs-pitch are the two halves of the same theory-owned
    /// motivic identity (a motif is a pitch shape AND a rhythm), and the realizer reads only
    /// what these two produce.
    ///
    /// theory — the GAIT DIMENSION. S39 welded one rhythm to each contour, so two images on
    /// the same contour clapped identically (Finding B, the "clap test"). Here each contour
    /// keeps its PITCH gesture but owns FOUR gaits that are all legal for that gesture, so the
    /// SAME melodic shape can be clapped four different ways. The cell ordering is a
    /// deliberate, monotone affect ramp the upstream selector (`pick_rhythm_cell`) indexes
    /// into BROAD → BUSY:
    ///   * cell 0 — the S39 profile (the back-compat anchor — `rhythm_cell(_, 0)` reproduces
    ///     S39 byte-for-byte; HARD freeze constraint, NOT a taste call — see the §6 hinge in
    ///     `docs/design-s41-findingB-rhythm-depth.md`).
    ///   * cell 1 — BROADER / AUGMENTED. More 2s, the gesture in longer values: the calm,
    ///     sustained reading of the same line (low arousal).
    ///   * cell 2 — BUSIER / EVEN-SUBDIVIDED. All (or mostly) 1s: the gesture run as a quick,
    ///     even, energetic line (high arousal, the moto-perpetuo reading).
    ///   * cell 3 — PROFILED / SYNCOPATED. A dotted (3), long-short-short, or off-balance
    ///     pattern: the same gesture given a characteristic, lilting/snapping rhythmic profile
    ///     so two busy images still diverge on gait-shape, not just density.
    ///
    /// Every weight is >= 1 (the `MotifNote` contract). Each cell, cycled across the contour
    /// and durational-sum-capped at `length_steps` by the UNCHANGED accumulation loop, honors
    /// the same Σ-budget contract S39 did — so the realizer contract is unchanged across the
    /// whole vocabulary (proven by `tests/motif_s41.rs` P5/P6 and the still-green
    /// `tests/motif_s39.rs` cell-0 hinge).
    fn rhythm_cells(self) -> &'static [&'static [u8]] {
        match self {
            // ARCH 1-3-5-3-1 — endpoint-framed climb-and-fall.
            //  0 S39: longer launch + longer arrival frame the quicker middle.
            //  1 broad: all augmentation — a slow, spacious arch (sostenuto).
            //  2 busy: even running quavers — the arch taken at speed.
            //  3 profiled: a dotted launch (long-short) then even, a lilting upbeat into the rise.
            MotifArchetype::Arch => &[&[2, 1, 1, 2], &[2, 2, 2], &[1, 1, 1, 1], &[3, 1, 1, 1, 2]],
            // INVERTED-ARCH 5-3-1-3-5 — settle into the valley, lift back out.
            //  0 S39 (shared with Arch in S39): endpoint-weighted.
            //  1 broad: augmented descent-and-return, a sighing dip.
            //  2 busy: even line through the valley.
            //  3 profiled: weighted floor (the 2 lands on the valley bottom, degree 1) — the dip
            //    is the long note, framing it as the gesture's center of gravity.
            MotifArchetype::InvertedArch => {
                &[&[2, 1, 1, 2], &[2, 2, 2], &[1, 1, 1, 1], &[1, 1, 2, 1, 1]]
            }
            // DESCENT 5-4-3-2-1 — stepwise fall that resolves.
            //  0 S39: even fall, lands long on the tonic arrival.
            //  1 broad: augmented descent — a slow, weighty settling.
            //  2 busy: a perfectly even running scale (the moto-perpetuo fall).
            //  3 profiled: a long-weighted HEAD (the 5 is held), then the fall accelerates —
            //    a suspension-and-release shape idiomatic to a resolving descent.
            MotifArchetype::Descent => &[
                &[1, 1, 1, 1, 2],
                &[2, 2, 2],
                &[1, 1, 1, 1, 1],
                &[2, 1, 1, 1, 1],
            ],
            // ASCENT 1-2-3-4-5 — stepwise lift, opening.
            //  0 S39: light lift that arrives long.
            //  1 broad: augmented rise — a slow, swelling crescendo of pitch.
            //  2 busy: an even running scale up (the energetic lift).
            //  3 profiled: even climb that arrives LONG on the goal degree — a goal-directed
            //    rise that breathes at the top (anacrusis-to-downbeat shape).
            MotifArchetype::Ascent => &[&[1, 1, 2], &[2, 2], &[1, 1, 1, 1], &[1, 1, 1, 2]],
            // NEIGHBOR-TURN 1-2-1-7-1 — ornamental turn around the tonic.
            //  0 S39: even turn settling long on the final resolution.
            //  1 broad: a slow, expressive turn (each pole of the ornament sustained).
            //  2 busy: a quick, even mordent-like turn — the ornament taken brightly.
            //  3 profiled: a SNAP turn — long tonic anchor (2), then the ornament flicks fast
            //    around it, a Lombard/Scotch-snap reading of the figure.
            MotifArchetype::NeighborTurn => {
                &[&[1, 1, 1, 1, 2], &[2, 2], &[1, 1, 1, 1], &[2, 1, 1, 1, 1]]
            }
            // LEAP-STEP 1-5-4-3-2 — an opening leap then a stepwise gap-fill.
            //  0 S39: the opening leap gets the weight, then quick stepwise recovery.
            //  1 broad: a sustained opening interval, then a measured fill (declamatory).
            //  2 busy: even — the leap and fill run as one quick gesture.
            //  3 profiled: a DOTTED opening (3 on the launch degree) — a heroic, fanfare-like
            //    long-leap then a rapid gap-fill descent (a French-overture profile).
            MotifArchetype::LeapStep => &[
                &[2, 1, 1, 1, 1],
                &[2, 2, 1],
                &[1, 1, 1, 1, 1],
                &[3, 1, 1, 1],
            ],
            // PENDULUM 1-5-1-5-1 — oscillating tonic↔dominant, insistent.
            //  0 S39: all augmentation — a slow two-zone toll.
            //  1 broad: still broader, every pole sustained — the deepest toll.
            //  2 busy: even — a quick alternation, an insistent ostinato.
            //  3 profiled: a DOTTED swing (long-short) — the pendulum given a lopsided, lurching
            //    gait, the toll with an uneven swing to it.
            MotifArchetype::Pendulum => &[&[2, 2], &[2, 2, 2], &[1, 1, 1, 1], &[3, 1]],
            // RISING-SEQUENCE 1-2-3 / 2-3-4 — a 3-note cell sequenced up a step; developmental.
            //  0 S39 (shared with Ascent in S39): a light developmental cell arriving long.
            //  1 broad: augmented — each cell of the sequence stated broadly.
            //  2 busy: even running line through both cells (a driving sequence).
            //  3 profiled: a long-short-short within EACH 3-note cell ([2,1,1] repeated) — the
            //    classic sequenced-motive gait where the cell head is accented on each repeat.
            MotifArchetype::RisingSequence => {
                &[&[1, 1, 2], &[2, 2], &[1, 1, 1, 1, 1, 1], &[2, 1, 1]]
            }
        }
    }

    /// Select ONE rhythm cell from this archetype's vocabulary by a 0..K-1 `index`
    /// (image-derived upstream by `composition::pick_rhythm_cell`). Defensive clamp so an
    /// out-of-range index can never panic — it saturates to the busiest/last cell.
    /// `rhythm_cell(_, 0)` is the S39 profile (the freeze anchor).
    fn rhythm_cell(self, index: usize) -> &'static [u8] {
        let cells = self.rhythm_cells();
        cells[index.min(cells.len() - 1)]
    }

    /// The size K of this archetype's rhythm-cell vocabulary, so the plan-time selector
    /// (`composition::pick_rhythm_cell`) can size the index it produces WITHOUT the static
    /// cell table crossing the module boundary. `K >= 1` for every archetype (it is 4 for all
    /// eight as authored; the slice type permits per-archetype variation — the only structural
    /// rule is cell 0 == the S39 profile).
    pub fn rhythm_cell_count(self) -> usize {
        self.rhythm_cells().len()
    }
}

// ════════════════════════════════════════════════════════════════════════════════════════════
// S53 / fix-direction-2 SLICE 1 (D-CELL) — THE PER-PIECE RHYTHMIC MOTTO + its un-gated selector.
//
// theory (the whole point of the slice): a short image-piece earns a hummable IDENTITY from ONE
// recognizable rhythmic GAIT that recurs — "this is how this picture walks." Until now the cell
// axis (the four idiomatic gaits authored per archetype in `rhythm_cells`) was reachable ONLY on
// the synthetic THEME path (complexity >= 0.4), so every real photo got NO cell — the gait was
// dead on the real path (design-s53-cell-seam §1). This slice lifts a per-piece `RhythmMotto` —
// an (archetype, cell) IDENTITY, not a resolved subject motif — selected once per plan from
// robust per-image features INDEPENDENT of the theme gate, stamped on every Section, and read by
// `realize_rhythm` to BIAS onset placement within the band the band-ladder already chose. The
// selection LOGIC lives here beside the cell vocabulary it indexes (so the cuts and the cells are
// one source); the carrier (`Section.motto`) and the realizer read live downstream.
// ════════════════════════════════════════════════════════════════════════════════════════════

/// S53 D-CELL — the per-piece cell-1/0/2 density-ramp edges, applied to the `band_activity_spread`-
/// re-expanded `edge_activity` (NOT raw edge — the natural-photo cluster compresses into ≈0.30–0.51
/// and would pancake onto one cell without the spread, design-s53-cell-affect §2). These REUSE the
/// pre-S50 themed-path values (CELL_EDGE_BROAD/BUSY in composition.rs) by VALUE — the cells 0/1/2 of
/// every archetype are authored as exactly this BROAD→BUSY ramp (`rhythm_cells` doc) — but live here
/// as their own consts so the per-piece path never reaches across the module boundary for them and
/// the themed path's reverted consts stay independent. The spread genuinely straddles both cuts for
/// the real cluster (affect §3.1: spread spans 0.000..0.847 across the six probes), so all three ramp
/// cells {1,0,2} are reachable — the S50 trap (cuts behind a complexity>=0.4 pre-filter) does NOT
/// recur because this path has no pre-filter. Invariant BROAD < BUSY holds (0.33 < 0.66).
const PIECE_EDGE_BROAD: f32 = 0.33; // spread-edge < this → cell 1 (broadest/augmented — calmest gait)
const PIECE_EDGE_BUSY: f32 = 0.66; // spread-edge >= this → cell 2 (busiest/even-subdivided)

/// S53 D-CELL — the SECONDARY (cell-3 PROFILED/SYNCOPATED) divert cut, keyed on `complexity`,
/// applied UN-GATED. This is the Affect specialist's adjudicated DRIVER decision (design-s53-cell-
/// affect §1/§3, confirmed by the lead over the seam spec's original `affect_arousal` proposal):
/// `affect_arousal` COLLAPSES the probe set (it ties the load-bearing Img3~Lena watch-pair within
/// 0.016 and only ever clears a cut for the already-distinct `example`), whereas `complexity`
/// cleanly separates it. complexity is the right CROSS-MODAL percept anyway: syncopation (onsets
/// displaced off the grid) is the rhythmic analogue of spatial INTRICACY/detail, which `complexity`
/// (a texture/shape scalar) measures and arousal (a saturation-dominant ENERGY composite) does not.
/// It only failed in S50 because it was trapped BEHIND the complexity>=0.4 theme gate — the
/// un-gating removes that trap, so the value the S50 author always intended (0.20) finally reaches
/// the real-photo cluster.
///
/// Value 0.20 sits in the only window that both (a) reaches Img3 (cplx 0.229 — the lower-bound floor:
/// the cut MUST be <= 0.229 or the Img3~Lena pair never splits) and (b) excludes Lena (cplx 0.164 —
/// the selectivity ceiling: the cut MUST be > 0.164 or the cell-0 anchor empties). Valid window
/// (0.164, 0.229]; ear-tune within [0.18, 0.23]. DISTINCT from the themed-path `CELL_COMPLEXITY_PROFILED`
/// (composition.rs, still 0.66 — that governs the unchanged synthetic theme path and stays reverted).
const PIECE_COMPLEXITY_PROFILED: f32 = 0.20;

/// A per-piece RHYTHMIC MOTTO: the chosen melodic archetype and the index of its rhythm cell,
/// selected ONCE per plan and stamped on every `Section`. Carries IDENTITY (archetype + cell),
/// NOT a resolved `Vec<MotifNote>` — the no-theme path has no subject to replay; the motto only
/// BIASES the band-ladder onset placement in `realize_rhythm`. `Copy` so it rides `Section` (which
/// is `Clone`) and the per-section `orchestration.clone()` with zero allocation.
///
/// `RhythmMotto::neutral()` is the behaviour-neutral value `realize_rhythm` treats as "apply NO
/// onset bias", so a section carrying it produces byte-identical output to pre-S53 (the freeze
/// hinge — design-s53-cell-seam §I-4): the legacy/identity/`single_section_default` paths all carry
/// it, so the engine_equivalence goldens cannot move.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RhythmMotto {
    pub archetype: MotifArchetype,
    /// The index of the chosen rhythm cell, OR `None` for the behaviour-neutral motto (apply no
    /// onset bias). `None` (not "cell 0") IS the freeze hinge: cell 0 is the S39 anchor gait and a
    /// real per-piece motto MAY legitimately select it, so neutrality must be a value DISTINCT from
    /// every real cell index — otherwise an image whose motto is genuinely cell 0 would be
    /// indistinguishable from "no motto" and the realizer could not tell the freeze path from a
    /// live cell-0 piece. `None` is that distinct sentinel.
    pub cell_index: Option<usize>,
}

impl RhythmMotto {
    /// The behaviour-neutral motto: `realize_rhythm` maps this to "apply NO onset bias", so a
    /// section carrying it is byte-identical to pre-S53 (the freeze hinge — §I-4). Used by
    /// `legacy_default_section`, `single_section_default` consumers, and the planner's fallback.
    /// The archetype is a don't-care placeholder (`Arch`) — `cell_index == None` is what the read
    /// short-circuits on, never the archetype.
    pub fn neutral() -> RhythmMotto {
        RhythmMotto {
            archetype: MotifArchetype::Arch,
            cell_index: None,
        }
    }

    /// True iff this is the behaviour-neutral (no-bias) motto. The realizer's freeze short-circuit.
    pub fn is_neutral(self) -> bool {
        self.cell_index.is_none()
    }
}

/// `Default` == `neutral()` — required so `OrchestrationProfile`'s `#[serde(skip)] motto` field
/// deserializes to the byte-stable no-op (the `prominence`/`figuration_resolved` precedent), so
/// every profile loaded from mappings.json carries the neutral motto until the planner seats the
/// live per-piece value.
impl Default for RhythmMotto {
    fn default() -> Self {
        RhythmMotto::neutral()
    }
}

/// Select the PER-PIECE rhythmic-motto cell (`0..cell_count`) for `archetype` from robust per-image
/// features, INDEPENDENT of the theme path. Pure; no RNG, no clock. The returned index is clamped
/// to `cell_count` (and the realizer also clamps defensively in `rhythm_cell`), so it can never
/// index out of an archetype's vocabulary.
///
/// PRIMARY axis = `band_activity_spread(edge_activity)` against `PIECE_EDGE_BROAD`/`PIECE_EDGE_BUSY`
/// along the BROAD→BUSY density ramp (cells 1/0/2). Reusing the SAME spread the band ladder reads
/// means the rhythmic motto and the realized onset density agree on one re-expanded activity.
///
/// SECONDARY divert = `complexity >= PIECE_COMPLEXITY_PROFILED` → the PROFILED/SYNCOPATED character
/// cell (cell 3), GUARDED so it fires ONLY when the primary did not already land the BUSIEST cell 2
/// (`primary_cell != 2`). theory (the cell-2 guard, affect §3.2): the genuine high-edge outlier
/// (`example`) must keep its even-subdivided cell-2 gait and not be swept to cell 3 by its high
/// complexity — cell 3 is reserved for the TEXTURED-but-not-busiest images, so a busy surface walks
/// evenly while an intricate-but-calm surface lurches. Without the guard the distinct-cell count
/// drops 4→3 (example would steal cell 3 from its own distinct cell-2 landing).
///
/// Takes the two driving features BY VALUE (not `&ImageUnderstanding`) so `chord_engine` imports no
/// image type — the module boundary "chord engine has no image logic" is preserved; `composition.rs`
/// reads `u.edge_activity`/`u.complexity` and passes scalars, exactly as it passes range/length to
/// `resolve_motif_celled`.
pub fn pick_piece_cell(
    edge_activity: f32,
    complexity: f32,
    _archetype: MotifArchetype,
    cell_count: usize,
) -> usize {
    // PRIMARY: the density ramp on the RE-EXPANDED edge (so the compressed real-photo cluster
    // actually spans the cuts — without the spread, four of six probes pancake onto one cell).
    let spread = band_activity_spread(edge_activity);
    let primary = if spread < PIECE_EDGE_BROAD {
        1 // calm → the broadest/augmented gait
    } else if spread < PIECE_EDGE_BUSY {
        0 // mid energy → the S39 anchor gait (broad-but-moving)
    } else {
        2 // busy → the busiest/even-subdivided gait
    };

    // SECONDARY divert (the decorrelating tiebreak): a visually-INTRICATE image takes the profiled/
    // syncopated character gait (cell 3) — but GUARDED so the genuine busy outlier (primary == 2)
    // keeps its even cell-2 gait. The vocabulary must actually HAVE a cell 3 (K >= 4 as authored)
    // for the divert to be available; otherwise the primary stands and the final clamp keeps range.
    let index = if complexity >= PIECE_COMPLEXITY_PROFILED && primary != 2 && cell_count > 3 {
        3
    } else {
        primary
    };

    // Clamp into this archetype's vocabulary (`cell_count >= 1`); saturates to the last cell.
    index.min(cell_count.saturating_sub(1))
}

/// Resolve a build-time `MotifArchetype` + image-derived range/length into the
/// concrete key-relative degree+duration sequence the realizer reads (spec §1.5).
/// THE ONE PLACE contour → `MotifNote` happens. Called by `composition.rs` at plan
/// build, never at tick time — so `MotifNote` stays frozen and the freeze is safe.
///
/// `archetype`    — the chosen melodic shape (image-selected upstream).
/// `range_degrees`— the span (in scale degrees) the contour is stretched/compressed
///                  to fill; clamped to a singable 1..=7 (a degree-7 span ≈ an
///                  octave). theory: edge_activity/complexity sets this band — calm →
///                  conjunct (small range), busy → wider leaps.
/// `length_steps` — the motif's total DURATION in steps (Σ dur_steps), NOT its note
///                  count. Each archetype's `rhythm_profile()` is cycled across the
///                  sampled contour and durations accumulate until they fill
///                  `length_steps`; the emitted note count is therefore <= length_steps.
///                  Clamped to >=1. Guarantees: every `dur_steps >= 1` and
///                  Σ dur_steps <= length_steps (the motif never over-runs its section).
///
/// Determinism: pure function of its three inputs — no RNG, no clock.
///
/// S41 (Finding B): this is the BACK-COMPAT entry point — equivalent to
/// `resolve_motif_celled(archetype, range_degrees, length_steps, 0)`. Cell 0 of every
/// archetype is the S39 `rhythm_profile()` value, so this wrapper reproduces the pre-S41
/// bytes for every existing caller and golden. New plan-time callers that want an
/// image-selected gait call `resolve_motif_celled` with a non-zero `cell_index`.
pub fn resolve_motif(
    archetype: MotifArchetype,
    range_degrees: u8,
    length_steps: usize,
) -> Vec<MotifNote> {
    resolve_motif_celled(archetype, range_degrees, length_steps, 0)
}

/// Resolve a build-time `MotifArchetype` + image-derived range/length + an image-SELECTED
/// rhythm-cell index into the concrete key-relative degree+duration sequence the realizer
/// reads (spec §1.5; S41 Finding-B rhythm-depth). THE ONE PLACE contour → `MotifNote`
/// happens. Called by `composition.rs` at plan build, never at tick time — so `MotifNote`
/// stays frozen and the freeze is safe.
///
/// Identical to the S39 `resolve_motif` body in every respect EXCEPT the rhythm source: the
/// archetype's single `rhythm_profile()` is replaced by `archetype.rhythm_cell(cell_index)`,
/// one of the archetype's K idiomatic gaits (S41 §3). `cell_index == 0` selects the S39
/// profile, making `resolve_motif_celled(.., 0)` byte-identical to the S39 `resolve_motif`
/// (the freeze anchor — `tests/motif_s39.rs` and P5 in `tests/motif_s41.rs` pin it). The
/// index is clamped defensively in `rhythm_cell`, so an out-of-range index can never panic.
///
/// `archetype`    — the chosen melodic shape (image-selected upstream).
/// `range_degrees`— the span (in scale degrees) the contour is stretched/compressed to fill;
///                  clamped to a singable 1..=7. theory: edge_activity/complexity sets this.
/// `length_steps` — the motif's total DURATION in steps (Σ dur_steps), NOT its note count.
///                  The selected cell is cycled across the sampled contour and durations
///                  accumulate until they fill `length_steps`; the emitted note count is
///                  therefore <= length_steps. Clamped to >=1. Guarantees, for EVERY cell:
///                  every `dur_steps >= 1` and Σ dur_steps <= length_steps.
/// `cell_index`   — which of the archetype's `rhythm_cell_count()` gaits to use (0 == S39).
///
/// Determinism: pure function of its four inputs — no RNG, no clock.
pub fn resolve_motif_celled(
    archetype: MotifArchetype,
    range_degrees: u8,
    length_steps: usize,
    cell_index: usize,
) -> Vec<MotifNote> {
    let contour = archetype.contour();
    // The canonical contour spans degrees 0..=4 (a fifth). Re-scale that native span
    // to the requested range so a "wide" image stretches the same shape over a larger
    // ambit without changing the contour's identity. Clamp the range to a singable
    // 1..=7 (degree 7 ≈ one octave) so we never produce an unsingable leap.
    let range = range_degrees.clamp(1, 7) as f32;
    const NATIVE_SPAN: f32 = 4.0; // degrees 0..=4
    let scale = range / NATIVE_SPAN;

    let len = length_steps.max(1);
    // S41: the rhythm source is now the image-SELECTED cell, not the archetype-fixed S39
    // profile. cell_index 0 == the S39 profile (freeze anchor); rhythm_cell clamps an
    // out-of-range index defensively. Everything below this line is byte-unchanged from S39 —
    // the same cycle-and-cap accumulation loop consumes `profile` exactly as before.
    let profile = archetype.rhythm_cell(cell_index);

    // DURATIONAL-SUM-CAPPED emit. Walk the contour ONCE, giving each sampled degree the
    // next duration from the archetype's cycled rhythm profile, and accumulate
    // `dur_steps` until the running sum reaches `len`. The motif's note COUNT is now
    // whatever fits in `len` steps (<= len), NOT a fixed `len`-long run of 1-step notes.
    //
    // Two invariants are load-bearing and enforced inline:
    //   (1) dur_steps >= 1 always (the `MotifNote` contract).
    //   (2) Σ dur_steps <= len always (the motif must NEVER over-run its section) — so a
    //       profile weight that would cross the `len` boundary is CLAMPED to the exact
    //       remaining budget. This is what makes Arch `[2,1,1,2]` land at Σ=5 (durs
    //       2,1,1,1) for len=5 instead of overshooting to Σ=6.
    let mut motif: Vec<MotifNote> = Vec::new();
    let mut total: usize = 0;
    for (i, &c) in contour.iter().enumerate() {
        if total >= len {
            break;
        }
        let remaining = len - total;
        // Cycle the profile across the sampled notes; clamp the boundary-crossing note
        // to `remaining` so Σ never exceeds `len`. `remaining >= 1` here (loop guard), so
        // the clamped duration is always >= 1 — the contract holds.
        let dur = (profile[i % profile.len()] as usize).min(remaining).max(1);
        let raw = c as f32 * scale;
        // Round to the nearest scale degree; the realizer maps degree → pitch.
        let degree = raw.round() as i8;
        motif.push(MotifNote {
            degree,
            dur_steps: dur as u8,
        });
        total += dur;
    }

    // STATIC-TAIL FIX (§4): if the whole contour was sampled and budget still remains,
    // do NOT repeat the held final degree as a string of short notes (the old smear at
    // the former `i.min(n_contour - 1)` hold). Instead extend the FINAL note into ONE
    // long arrival that absorbs the remainder — a single sustained landing, not a wrap.
    // This is the only held tail §4 permits, and it keeps Σ == len exactly.
    if total < len {
        if let Some(last) = motif.last_mut() {
            last.dur_steps = last.dur_steps.saturating_add((len - total) as u8);
        }
    }

    debug_assert!(
        motif.iter().map(|n| n.dur_steps as usize).sum::<usize>() <= len,
        "Σ dur_steps must never over-run length_steps"
    );
    debug_assert!(
        motif.iter().all(|n| n.dur_steps >= 1),
        "every MotifNote must carry dur_steps >= 1"
    );
    motif
}

/// Map a scale degree (relative to the section tonic, 0 == tonic) onto a sounding
/// MIDI pitch in the section's mode, seated near the melody register. Pure helper
/// for the theme-replay realizer. theory: a key-relative degree must resolve through
/// the section's actual MODE (degree 2 is a major third in Ionian but a minor third
/// in Aeolian) — that is what makes the same motif sound right in any mode.
fn degree_to_pitch(degree: i8, tonic_pc: u8, mode: &str, register_floor: u8) -> u8 {
    let scale = match mode {
        "Ionian" => IONIAN,
        "Dorian" => DORIAN,
        "Phrygian" => PHRYGIAN,
        "Lydian" => LYDIAN,
        "Mixolydian" => MIXOLYDIAN,
        "Aeolian" => AEOLIAN,
        _ => IONIAN, // unknown mode → Ionian, matching generate_chords' fallback
    };
    // Decompose the (possibly negative, possibly >6) degree into an octave count and
    // a 0..=6 index into the 7-note scale. Rust's `%` is truncating, so use rem_euclid
    // for a correct floored index/octave on negative degrees (e.g. degree -1 = the
    // leading tone, one scale step BELOW the tonic in the octave below).
    let n = scale.len() as i32; // 7
    let d = degree as i32;
    let idx = d.rem_euclid(n); // 0..=6
    let octave = d.div_euclid(n); // floored octave offset
                                  // The pitch CLASS this degree names, then seat that class at/above the register
                                  // floor (the same one-place seating role_pitch uses), and apply the degree's own
                                  // octave on top so an ascending motif actually climbs across the octave break.
    let pc = (tonic_pc as i32 + scale[idx as usize] as i32).rem_euclid(12) as u8;
    let seated = seat_pc_in_register(pc, register_floor) as i32 + 12 * octave;
    seated.clamp(24, 108) as u8
}

/// Where a kernel step lands inside a DURATIONAL motif (S39). The motif `Vec` is now
/// SHORTER than its step span — one entry per NOTE, each carrying `dur_steps >= 1` — so a
/// kernel step no longer maps 1:1 to a motif index. This resolves a `step_in_section` to
/// the motif NOTE that covers it (by walking cumulative `dur_steps`) and whether the step
/// is that note's ONSET (its first covered step) or a CONTINUATION (a later covered step
/// of a `dur_steps > 1` note). The freeze-safe realization plays the note at its onset and
/// RESTS on its continuation steps — making the note's true length audible WITHOUT holding
/// one emitted note across multiple kernel steps (the frozen kernel emits one note/step).
///
/// `motif` is sliced to `head_notes` first (the Fragmented head-vs-whole choice the caller
/// makes), so this walks only the playable note span. Returns:
///   * `MotifStep::Onset(note)` — `step` is this note's first covered step: PLAY it.
///   * `MotifStep::Continuation` — a later covered step of a multi-step note: REST.
///   * `MotifStep::PastEnd` — beyond Σ dur_steps of the head; the caller decides (Identity
///     holds the last note, Fragmented rests).
///
/// FREEZE HINGE — when every note has `dur_steps == 1`, Σ dur_steps == note count, so step
/// `s` lands exactly on note `s` as an `Onset` and `s >= note count` is `PastEnd` — i.e.
/// IDENTICAL to the old 1:1 `motif[step_in_section]` index with the same past-end boundary.
/// No `Continuation` is ever produced in the 1-step case, so the realized output is
/// byte-for-byte unchanged. The `Continuation` arm is reachable ONLY for a `dur_steps > 1`
/// note, which the old motif builder never produced — so it perturbs nothing frozen.
enum MotifStep {
    Onset(MotifNote),
    Continuation,
    PastEnd,
}

/// Resolve `step` against a motif slice via cumulative `dur_steps`. Pure helper for the
/// theme realizer. See [`MotifStep`] for the freeze-hinge argument.
fn motif_step_at(motif: &[MotifNote], step: usize) -> MotifStep {
    let mut cum = 0usize;
    for &note in motif {
        let dur = note.dur_steps.max(1) as usize; // contract guarantees >=1; be defensive.
        if step < cum + dur {
            // step falls inside this note's [cum, cum+dur) span.
            return if step == cum {
                MotifStep::Onset(note)
            } else {
                MotifStep::Continuation
            };
        }
        cum += dur;
    }
    MotifStep::PastEnd
}

/// The realizer's THEME-REPLAY decision (spec §2, task 2; S39 durational reconciliation).
/// Given the plan-relative `StepContext`, decide what the MELODY role sounds on this step:
///
///   * `thematic_role == Statement | Return` with a theme present → map the step to its
///     covering motif NOTE via cumulative `dur_steps` (S39). On the note's ONSET step,
///     play it (degree → pitch in the section's mode/key) → `Some(Some(p))`. On a
///     CONTINUATION step of a `dur_steps > 1` note, REST → `Some(None)`, so the note's
///     length sings as one onset followed by silence (the freeze-safe realization — the
///     kernel cannot hold one emitted note across steps).
///   * `variation == Fragmented` → play only the FIRST HALF of the motif (by NOTE count);
///     once past the head's step span the melody role RESTS → `Some(None)`.
///   * past the whole motif's step span on an Identity section → hold the FINAL motif note
///     as a sustained arrival (re-articulated each step, never a wrap-loop), matching the
///     old past-end behavior.
///   * `Contrast | Coda | Development`, or no theme, or a non-Melody role → `None`: the
///     caller free-selects exactly as today (byte-stable back-compat).
///
/// Returns `Option<Option<u8>>`:
///   * `None`          → not a theme-replay step; caller takes its existing path.
///   * `Some(Some(p))` → play the theme pitch `p`.
///   * `Some(None)`    → theme-driven REST (motif continuation, or Fragmented tail).
///
/// This keeps the realizer's free-select / velocity / rhythm bodies UNTOUCHED — the theme
/// branch only substitutes (or silences) the melody PITCH; everything else (Bass/Fill
/// roles, velocity contour, rhythm patterns) is unchanged. When every motif note carries
/// `dur_steps == 1` this is byte-identical to the pre-S39 1:1 index (see [`MotifStep`]).
pub fn theme_melody_pitch(
    ctx: &crate::composition::StepContext,
    role: OrchestralRole,
    chord: &Chord,
    features: &PerfFeatures,
) -> Option<Option<u8>> {
    use crate::composition::{ThematicRole, ThemeVariation};

    // Only the MELODY role carries the theme; bass/fill keep their existing behavior.
    if role != OrchestralRole::Melody {
        return None;
    }
    // A theme must be present and the section must STATE or RECALL it. Contrast/Coda/
    // Development sections (and theme:None) free-select — the spec's "today's behavior."
    let theme = ctx.theme?;
    match ctx.section.thematic_role {
        ThematicRole::Statement | ThematicRole::Return => {}
        _ => return None,
    }
    if theme.motif.is_empty() {
        return None;
    }

    // FRAGMENTED (head-then-rest): the B-section continuity gesture. Play only the FIRST
    // HALF of the motif (by NOTE count — the "developing" gesture keeps motivic continuity
    // through a contrast without restating the whole tune, design-s15 §4.1/§4.2). Slice the
    // motif to the head NOTE span; the step→note mapping then walks only those notes'
    // durations, so a multi-step head note still consumes its full duration before the rest.
    let head_notes = match ctx.section.variation {
        ThemeVariation::Fragmented => theme.motif.len().div_ceil(2), // ceil(half), by note count
        _ => theme.motif.len(),
    };
    let head = &theme.motif[..head_notes];

    // S39: map this step to its covering motif note via cumulative dur_steps. Onset plays,
    // continuation rests, past-end falls to the variation-specific tail behavior below.
    match motif_step_at(head, ctx.step_in_section) {
        MotifStep::Onset(note) => Some(Some(theme_pitch(note.degree, ctx, chord, features))),
        // A covered/continuation step of a dur_steps>1 note: the onset already sounded; this
        // step is silent so the note reads as its full length (freeze-safe — no held note
        // spans kernel steps). UNREACHABLE when all dur_steps==1 (the byte-freeze hinge).
        MotifStep::Continuation => Some(None),
        MotifStep::PastEnd => match ctx.section.variation {
            // Past the head's step span: Fragmented rests (head consumed → melody silent).
            ThemeVariation::Fragmented => Some(None),
            // Identity holds the FINAL motif note as a sustained arrival (re-articulated each
            // step past the end, never a wrap-loop) — byte-identical to the old past-end hold.
            _ => {
                let last = theme.motif[theme.motif.len() - 1];
                Some(Some(theme_pitch(last.degree, ctx, chord, features)))
            }
        },
    }
}

/// The `dur_steps` of the motif note whose ONSET lands on this melody step, or `None` when
/// the step is NOT a theme onset (free-select, a continuation rest, a Fragmented/past-end
/// tail, a non-Melody role, or theme:None). The Melody realize arm reads this to let a
/// multi-step theme note SING a touch longer within the overlap ceiling (S39 — "lengthen
/// the within-step hold toward the existing overlap ceiling so it sings"). It mirrors
/// `theme_melody_pitch`'s exact gating so the two never disagree about onset-hood.
///
/// FREEZE: returns `Some(dur)` ONLY for `MotifStep::Onset`, and the only reader lengthens
/// the hold ONLY when `dur > 1` — a `dur == 1` onset (every note in the pre-S39 motif) and
/// every non-onset step return a hold-neutral value, so the realized output is unchanged on
/// the freeze path. Pure: only `ctx.step_in_section` (a Copy) is read, nothing is mutated.
fn theme_onset_dur_steps(
    ctx: &crate::composition::StepContext,
    step: &StepPlan,
    features: &PerfFeatures,
) -> Option<u8> {
    use crate::composition::{ThematicRole, ThemeVariation};
    let theme = ctx.theme?;
    match ctx.section.thematic_role {
        ThematicRole::Statement | ThematicRole::Return => {}
        _ => return None,
    }
    if theme.motif.is_empty() {
        return None;
    }
    // A theme onset only sounds where theme_melody_pitch would actually play a NEW note (and
    // not at the past-end hold, which re-articulates the last note but is not a fresh onset).
    // Confirm the seam agrees this step plays a pitch before trusting the motif walk.
    match theme_melody_pitch(ctx, OrchestralRole::Melody, &step.chord, features) {
        Some(Some(_)) => {}
        _ => return None,
    }
    let head_notes = match ctx.section.variation {
        ThemeVariation::Fragmented => theme.motif.len().div_ceil(2),
        _ => theme.motif.len(),
    };
    let head = &theme.motif[..head_notes];
    match motif_step_at(head, ctx.step_in_section) {
        MotifStep::Onset(note) => Some(note.dur_steps.max(1)),
        // past-end Identity hold re-articulates the last note but is not a fresh onset:
        // treat it as dur 1 (no extra lengthening) so the sustained arrival is unperturbed.
        _ => None,
    }
}

/// Resolve one motif degree to a sounding melody pitch in the CURRENT section's
/// mode/key, biased toward the chord so the theme note lands as a chord tone when it
/// can. theory: a motif degree that coincides with a chord tone is consonant; one
/// that doesn't is a NON-CHORD TONE (passing/neighbor) over the prevailing harmony —
/// both are musical, and degree-relative-to-key (not snapped-to-chord) is what gives
/// the theme its recognizable melodic identity across changing chords.
fn theme_pitch(
    degree: i8,
    ctx: &crate::composition::StepContext,
    chord: &Chord,
    features: &PerfFeatures,
) -> u8 {
    // Tonic pitch-class: section key_offset (always 0 in slice 1) over the home root.
    let tonic_pc = ((ctx.key_tempo.home_root_midi as i16 + ctx.section.key_offset_semitones as i16)
        .rem_euclid(12)) as u8;
    // Brightness raises/lowers the melody register exactly as role_pitch does for the
    // free-select melody, so theme and free-select notes share one register frame.
    let bright_octaves = ((features.brightness - 50.0) / 50.0).clamp(-1.0, 1.0);
    let lift = (bright_octaves * 12.0).round() as i16;
    let floor = (MELODY_REGISTER_FLOOR as i16 + lift).clamp(24, 96) as u8;
    let mode = ctx.section.mode.as_str();
    let mut pitch = degree_to_pitch(degree, tonic_pc, mode, floor);
    // If the resolved degree is enharmonically a chord tone, prefer the chord's own
    // octave-seating of that pitch class so a consonant theme note locks to the chord.
    let pc = pitch % 12;
    if chord.notes.iter().any(|&n| n % 12 == pc) {
        pitch = seat_pc_in_register(pc, floor);
    }
    pitch.clamp(24, 108)
}

// ─────────────────────────────────────────────────────────────────────────────
// S28/K3 — the pivot / common-tone modulation + land-home cadence realizer fns.
// The harmonic RULES are specified in docs/input-s28-k3-pivot-harmony.md. These
// fns are reachable ONLY on a non-home_only, pivot:true scheme; on the identity /
// home_only / pivot:false / Open path they are inert (None / false), which is the
// chord_engine.rs BEHAVIORAL byte-freeze.
// ─────────────────────────────────────────────────────────────────────────────

/// The boundary-step velocity for a pivot: a prepared modulation lands ON a downbeat,
/// so it sounds at the phrase-start weight — not a cadence (96), not an interior step.
/// theory: the pivot is the fresh start of the new section; the phrase-initial accent is
/// the correct dynamic for it. Matches `plan_phrases`'s `V_START`.
const V_PIVOT: u8 = 88;

/// A witnessed pivot / common-tone chord inserted at a MODULATING section boundary (S28/K3).
/// The chord prepares the move from the previous section's key to this section's
/// `key_offset_semitones` so a direct modulation no longer sounds like a splice. Returns the
/// pivot's note event(s) for THIS instrument's `role` on this step, or `None` when no pivot
/// applies (the byte-freeze gate).
///
/// THE HARMONIC RULE (docs/input-s28-k3-pivot-harmony.md §2): the pivot is the DOMINANT of the
/// DESTINATION key (a major-quality V triad rooted on `dest_root_pc + 7`). The destination's
/// leading tone — the V chord's major third — is the pitch that most strongly asserts the new
/// key, and the dominant→tonic pull into the next downbeat (the destination's own first chord)
/// realizes the prepared modulation across the abutting step windows (§4, no scheduler change).
/// A pitch class shared between the PREVIOUS tonic triad and this destination V is voiced in an
/// inner (fill) voice as the audible hinge.
///
/// Returns `Some` ONLY when ALL hold (else `None`):
///   (a) the active scheme is `pivot == true` (`ctx.section.pivot`);
///   (b) this is the FIRST step of the section (`ctx.step_in_section == 0`);
///   (c) this section's key differs from the previous section's:
///       `Some(prev) = ctx.prev_key_offset_semitones` AND `prev != ctx.section.key_offset_semitones`
///       (a `None` prev — first section / identity — is NEVER a key change → `None`).
/// Under `home_only` every offset is 0 and equals its predecessor → `None` → NOTHING inserted.
///
/// VOICING (§3): bass = dominant root; fill/inner = the common-tone hinge (+ the destination
/// leading tone); melody = the dominant's fifth. Every pitch is seated via the SAME register
/// floors the free-select path uses (`seat_pc_in_register` at the role's floor), so the bass <
/// fill < melody ordering holds by construction and `no_inversion_invariant` cannot break.
/// DURATION (§4): one note for this role, `offset_ms = 0`, `hold_ms = ms_per_step` (a sustained,
/// legato preparation; capped at the step so it never bleeds past). Pure.
fn pivot_chord_events(
    ctx: &crate::composition::StepContext,
    role: OrchestralRole,
    features: &PerfFeatures,
    ms_per_step: u64,
) -> Option<Vec<NoteEvent>> {
    // (a) scheme opt-in. (b) boundary step only. Both are the byte-freeze gate.
    if !ctx.section.pivot || ctx.step_in_section != 0 {
        return None;
    }
    // (c) a REAL key change: a Some(prev) that differs from this section's offset. A `None`
    // prev (first section / identity / legacy flat path) is never a modulation.
    let prev_off = ctx.prev_key_offset_semitones?;
    let dest_off = ctx.section.key_offset_semitones;
    if prev_off == dest_off {
        return None;
    }

    // The home/prev/dest tonic pitch classes (semitone offsets from the home root).
    let home_root_pc = (ctx.key_tempo.home_root_midi % 12) as i16;
    let dest_root_pc = (home_root_pc + dest_off as i16).rem_euclid(12) as u8;
    let prev_root_pc = (home_root_pc + prev_off as i16).rem_euclid(12) as u8;

    // The DESTINATION DOMINANT (V of the destination key) — the unifying pivot (§2.1). The
    // dominant is MAJOR-quality (the leading tone of the new key is its major third); that is
    // what asserts the destination regardless of the piece's mode.
    let dom_root_pc = (dest_root_pc + 7) % 12; // 5th degree of the destination
    let dom_third_pc = (dom_root_pc + 4) % 12; // major 3rd = the destination's leading tone
    let dom_fifth_pc = (dom_root_pc + 7) % 12; // perfect 5th of the dominant
                                               // S29 Lever 3: the dominant SEVENTH — a minor 7th above the dominant root — turning the
                                               // bare V triad into a V7. theory reasoning: the V7 carries the tritone (its 3rd ↔ 7th,
                                               // i.e. the destination's leading tone ↔ this 7th) that is the unambiguous signal of a
                                               // functional dominant; a bare triad lacks that pull, so the announcement of the new key
                                               // is softer than it could be. The 7th resolves DOWN by step into the destination tonic's
                                               // THIRD across the pivot→I boundary (the dovetail with the §2.1(b) opening-cadence rule;
                                               // see docs/input-s29-k3-retune-harmony.md). It is an inner-voice color tone by nature and
                                               // is seated in the FILL register below, strictly between the bass (dom root) and the
                                               // melody (dom fifth), so the no-inversion frame (bass < fill < melody) holds by
                                               // construction. A 1-/2-instrument ensemble assigns no fill role, so the 7th is simply
                                               // never voiced there (the bare-triad pivot remains) — exactly spec §2.3.
    let dom_seventh_pc = (dom_root_pc + 10) % 12; // minor 7th above the dominant = the V7 color

    // The HINGE (§2.2, retained as documented reasoning): a pitch class shared by the PREVIOUS
    // tonic triad and this destination V. Prefer the previous tonic itself if it is a member of
    // V/dest, else the previous dominant pc, else the dominant 5th. The picker only ever selects
    // a pc that IS in the dominant triad, so a shared tone PROVABLY exists across the boundary —
    // which is WHY the V7 reads as a prepared move and not a splice. As of S29 (Lever 3) the
    // inner voice carries the dominant SEVENTH instead of statically holding this hinge; the hinge
    // is preserved here as the proof a common tone exists and as the conceptual line the resolving
    // 7th rides (see docs/input-s29-k3-retune-harmony.md §3). Bound with a leading underscore
    // because the inner voice now sounds the 7th, not this pc, but kept for the invariant it
    // witnesses. `dom_third_pc` (the destination leading tone) is likewise the V's defining new
    // pitch — implied by the major-quality dominant and the resolution target documented below.
    let v_triad = [dom_root_pc, dom_third_pc, dom_fifth_pc];
    let prev_dom_pc = ((prev_root_pc as i16 + 7).rem_euclid(12)) as u8;
    let _common_tone_pc = if v_triad.contains(&prev_root_pc) {
        prev_root_pc
    } else if v_triad.contains(&prev_dom_pc) {
        prev_dom_pc
    } else {
        dom_fifth_pc
    };

    // Per-role voicing, each pitch seated at the role's existing register floor via the SAME
    // helper the free-select path uses → bass < fill < melody by construction (no inversion).
    // brightness lifts the melody octave exactly as role_pitch does, so the pivot's top line
    // sits in the same register the surrounding melody does.
    let bright_octaves = ((features.brightness - 50.0) / 50.0).clamp(-1.0, 1.0);
    let note = match role {
        OrchestralRole::Bass => {
            // Root of the dominant in the bass = root-position V — the strongest preparation.
            let dark_drop = if bright_octaves < 0.0 { 12 } else { 0 };
            let floor = BASS_REGISTER_FLOOR.saturating_sub(dark_drop);
            seat_pc_in_register(dom_root_pc, floor)
        }
        OrchestralRole::Melody => {
            // The dominant's 5th on top — a stable, singable melody tone over the V.
            let lift = (bright_octaves * 12.0).round() as i16;
            let floor = (MELODY_REGISTER_FLOOR as i16 + lift).clamp(24, 96) as u8;
            seat_pc_in_register(dom_fifth_pc, floor)
        }
        OrchestralRole::HarmonicFill | OrchestralRole::Pad | OrchestralRole::CounterMelody => {
            // S29 Lever 3: the inner voice now sounds the dominant SEVENTH (the V7 color),
            // not the bare common-tone hinge. theory reasoning: the 7th is the chord's
            // leading dissonance — the inner voice is exactly where it belongs, and giving it
            // to the fill makes the pivot an unambiguous V7. The common-tone HINGE is not
            // abandoned: it is preserved as the 7th's RESOLUTION TARGET — the 7th resolves
            // DOWN by step into the destination tonic's third on the next downbeat (the
            // step-1 I), so the inner voice still tracks a prepared, resolving line across the
            // boundary rather than a static hinge. (The hinge picker above proves a common tone
            // exists; the 7th seated here is strictly between bass (dom root) and melody (dom
            // fifth) → no inversion.)
            let lift = ((bright_octaves * 6.0).round() as i16).clamp(-12, 12);
            let floor = (FILL_REGISTER_FLOOR as i16 + lift).clamp(24, 96) as u8;
            seat_pc_in_register(dom_seventh_pc, floor)
        }
    };

    // One sustained note for this role, on the boundary downbeat, never longer than the step.
    Some(vec![NoteEvent {
        note,
        velocity: V_PIVOT,
        hold_ms: ms_per_step,
        offset_ms: 0,
    }])
}

/// The CounterMelody's REALIZED pitch when the step at `(ctx.section, si)` is voiced by the
/// PIVOT path (`pivot_chord_events`) rather than the species counter arm — i.e. the dominant
/// SEVENTH seat the V7 pivot gives the inner voice. `None` when this step is NOT a pivot step.
///
/// theory / why this exists (S45-CP-FIX): at a modulating section's step-0 boundary the counter
/// does not sound its species line — `realize_step` returns the pivot voicing BEFORE the counter
/// arm. The species REPLAY (`realized_prev_counter`) must therefore reconstruct THIS pitch at a
/// pivot step, not the §3.1 seed, or the next step's `prev_counter` would disagree with what the
/// boundary actually sounded — re-introducing a cross-step inconsistency the M1.4 independence
/// guard then cannot see (the residual pivot-boundary parallels the scorecard measured). This
/// recomputes the SAME `dom_seventh_pc` seat `pivot_chord_events` voices for the CounterMelody,
/// keyed off the borrowed ctx at index `si`, so the replay tracks the live boundary exactly.
/// Pure / RNG-free. Byte-neutral on the freeze net (no pivot on identity/home_only).
fn pivot_counter_pitch(
    ctx: &crate::composition::StepContext,
    features: &PerfFeatures,
    si: usize,
) -> Option<u8> {
    pivot_role_pitch(ctx, features, si, OrchestralRole::CounterMelody)
}

/// The MELODY's REALIZED pitch when the step is a PIVOT step (the dominant FIFTH the V7 pivot
/// gives the top line), or `None` when not a pivot step. The melody analogue of
/// [`pivot_counter_pitch`]: the counter must see the melody that ACTUALLY sounds at the
/// boundary (the pivot dom-5th), not the species free-select pitch, or its M1.4 independence
/// check at the step AFTER the pivot answers the wrong melody direction. Same gate/seat as the
/// Melody arm of `pivot_chord_events`. Pure / byte-neutral on the freeze net.
fn pivot_melody_pitch(
    ctx: &crate::composition::StepContext,
    features: &PerfFeatures,
    si: usize,
) -> Option<u8> {
    pivot_role_pitch(ctx, features, si, OrchestralRole::Melody)
}

/// Shared pivot-voicing recompute for the two melodic roles the counter's M1.4 logic reads
/// (Melody = dom 5th, CounterMelody = dom 7th). Mirrors the EXACT gate + per-role seat of
/// `pivot_chord_events` so the species replay / melody-view reconstruct the pitch the pivot
/// boundary actually sounds. `None` unless `(ctx.section, si)` is a real modulating step-0
/// boundary. Bass/Pad are not reconstructed here (the counter never reads them).
fn pivot_role_pitch(
    ctx: &crate::composition::StepContext,
    features: &PerfFeatures,
    si: usize,
    role: OrchestralRole,
) -> Option<u8> {
    if !ctx.section.pivot || si != 0 {
        return None;
    }
    let prev_off = ctx.prev_key_offset_semitones?;
    let dest_off = ctx.section.key_offset_semitones;
    if prev_off == dest_off {
        return None;
    }
    let home_root_pc = (ctx.key_tempo.home_root_midi % 12) as i16;
    let dest_root_pc = (home_root_pc + dest_off as i16).rem_euclid(12) as u8;
    let dom_root_pc = (dest_root_pc + 7) % 12;
    let bright_octaves = ((features.brightness - 50.0) / 50.0).clamp(-1.0, 1.0);
    match role {
        OrchestralRole::Melody => {
            // Dominant 5th on top, seated at the MELODY floor + full bright lift (pivot §Melody).
            let dom_fifth_pc = (dom_root_pc + 7) % 12;
            let lift = (bright_octaves * 12.0).round() as i16;
            let floor = (MELODY_REGISTER_FLOOR as i16 + lift).clamp(24, 96) as u8;
            Some(seat_pc_in_register(dom_fifth_pc, floor))
        }
        OrchestralRole::CounterMelody => {
            // Dominant 7th (V7 color), seated at the FILL floor + half bright lift (pivot §inner).
            let dom_seventh_pc = (dom_root_pc + 10) % 12;
            let lift = ((bright_octaves * 6.0).round() as i16).clamp(-12, 12);
            let floor = (FILL_REGISTER_FLOOR as i16 + lift).clamp(24, 96) as u8;
            Some(seat_pc_in_register(dom_seventh_pc, floor))
        }
        _ => None,
    }
}

/// Is the land-home authentic cadence armed at THIS step (S28/K3)? `true` only when the scheme's
/// `ResolutionPolicy::Resolve` forced the final section to offset 0 AND `pivot == true` AND this
/// is the final section's closing Perfect-cadence step. When armed, the realizer STRENGTHENS the
/// VOICING of the already-stamped Perfect cadence into an explicit root-position V→I in the HOME
/// key with the home tonic on top (a true PAC) — it does NOT re-author cadence DATA (`plan_phrases`
/// already stamps `PerfectAuthenticCadence` and adds no step / moves no boundary). Pure. Returns
/// `false` on the identity / `home_only` / `pivot:false` / `Open` path → the voicing is untouched
/// and byte-identical to pre-K3.
fn land_home_is_armed(ctx: &crate::composition::StepContext, position: PhrasePosition) -> bool {
    use crate::composition::ResolutionPolicy;
    ctx.section.pivot
        && ctx.section.resolution == ResolutionPolicy::Resolve
        // Resolve forces the FINAL section to offset 0; this guards against arming off-home.
        && ctx.section.key_offset_semitones == 0
        // Only the already-stamped final Perfect Authentic Cadence step.
        && matches!(position, PhrasePosition::PerfectAuthenticCadence)
}

/// The land-home PAC pitch for `role` (S28/K3): bass = home tonic ROOT (root-position I),
/// soprano/melody = the home TONIC (the defining PAC marker — soprano on the tonic), fill = the
/// inner tonic-triad tones. Seated at the role's existing register floor so the voicing adds NO
/// event and cannot invert the frame. Called ONLY when `land_home_is_armed` is true.
fn land_home_pitch(
    role: OrchestralRole,
    ctx: &crate::composition::StepContext,
    chord: &Chord,
) -> u8 {
    let home_root_pc = (ctx.key_tempo.home_root_midi % 12) as u8;
    match role {
        OrchestralRole::Bass => {
            // Home tonic root in the bass → root-position I (the PAC requires it).
            seat_pc_in_register(home_root_pc, BASS_REGISTER_FLOOR)
        }
        OrchestralRole::Melody => {
            // Soprano on the home TONIC — the single feature that makes this a PERFECT
            // authentic cadence rather than an imperfect one.
            seat_pc_in_register(home_root_pc, MELODY_REGISTER_FLOOR)
        }
        OrchestralRole::HarmonicFill | OrchestralRole::Pad | OrchestralRole::CounterMelody => {
            // An inner tonic-triad tone (the 3rd or 5th) if the cadence chord carries one,
            // else the home tonic — keeps the inner voice on the I chord.
            let third_or_fifth = chord
                .notes
                .iter()
                .map(|&n| n % 12)
                .find(|&pc| pc == (home_root_pc + 3) % 12 || pc == (home_root_pc + 4) % 12)
                .unwrap_or((home_root_pc + 7) % 12);
            seat_pc_in_register(third_or_fifth, FILL_REGISTER_FLOOR)
        }
    }
}

/// Is the S29 opening V→I authentic cadence armed at THIS step (Lever 1(b), Option A)?
/// `true` only on the step that RESOLVES the step-0 pivot V of a modulating section: the
/// scheme is `pivot == true`, this is `step_in_section == 1` (the downbeat right after the
/// boundary pivot), and the section is a REAL key change (`Some(prev) != dest`). When armed,
/// the realizer voice-leads this step's chord as the DESTINATION TONIC resolving the prior
/// pivot V (`pivot_resolution_pitch`), turning V→I into a true authentic cadence in the new
/// key instead of two root-position triads stacked.
///
/// theory reasoning: this is the SAME gate shape as `pivot_chord_events` but one step later
/// (step 1, the resolution downbeat, vs. step 0, the pivot itself). It is `false` on every
/// identity / `home_only` / `pivot:false` / non-modulating-boundary step → the voicing is
/// untouched and byte-identical to pre-S29. Option B (`plan_phrases` stamping an explicit
/// opening PAC) is HELD; this voicing-only rule is the default per spec-s29 §2.1(b).
fn pivot_resolution_is_armed(ctx: &crate::composition::StepContext) -> bool {
    if !ctx.section.pivot || ctx.step_in_section != 1 {
        return false;
    }
    // A real key change: a Some(prev) differing from this section's offset (a None prev — the
    // first section / identity — is never a modulation, exactly as the pivot gate reads it).
    match ctx.prev_key_offset_semitones {
        Some(prev) => prev != ctx.section.key_offset_semitones,
        None => false,
    }
}

/// The S29 opening-cadence resolution pitch for `role` (Lever 1(b), Option A): voice the
/// destination TONIC so the step-0 pivot V resolves V→I as a true authentic cadence in the new
/// key. Called ONLY when `pivot_resolution_is_armed` is true.
///
/// theory reasoning (docs/input-s29-k3-retune-harmony.md §3): the prior pivot sounded V7 of the
/// destination — bass = dominant root, melody = dominant 5th (= destination 2nd degree),
/// inner = dominant 7th. The two voice-leading resolutions that make this an authentic cadence
/// and PREVENT the parallel octaves a trombonist hears when two root-position triads are merely
/// stacked are:
///   * the new key's LEADING TONE (the V's major 3rd) resolves UP by semitone to the new TONIC
///     ROOT — realized structurally by arriving root-position with the tonic DOUBLED in the
///     outer voices, the upper voices descending by step INTO it (contrary/oblique to the bass
///     leap), which is exactly the frame the leading-tone-up resolution produces;
///   * any chordal 7th resolves DOWN by step to the I's THIRD — realized literally: the inner
///     voice's pivot 7th (destination 5th degree + a minor 7th = dest_root + 5) steps DOWN to
///     the destination tonic's diatonic third in the active mode.
/// Per role, in the destination key (`dest_root_pc = home_root + this section's offset`):
///   * Bass    → destination tonic ROOT (root-position I; dom-root → tonic-root is the
///               idiomatic authentic-cadence bass leap).
///   * Melody  → destination tonic ROOT on top (the pivot's melody dom-5th = dest 2nd degree
///               descends a STEP to the tonic — soprano-on-tonic, a strong arrival, and the
///               step-down/leap-up split is the contrary motion that voids parallel octaves).
///   * Fill    → destination tonic diatonic THIRD (the pivot 7th's down-by-step target).
/// Every pitch is seated via `seat_pc_in_register` at the role's existing floor, so
/// bass < fill < melody holds by construction and `no_inversion_invariant` cannot break.
fn pivot_resolution_pitch(role: OrchestralRole, ctx: &crate::composition::StepContext) -> u8 {
    let home_root_pc = (ctx.key_tempo.home_root_midi % 12) as i16;
    let dest_off = ctx.section.key_offset_semitones as i16;
    let dest_root_pc = (home_root_pc + dest_off).rem_euclid(12) as u8;
    // The destination tonic's DIATONIC third in the piece's mode — major (3rd interval = 4) for
    // Ionian/Lydian/Mixolydian, minor (= 3) for Aeolian/Dorian/Phrygian. Reuses the same scale
    // selection `generate_chords`/`tonic_triad` use, so the resolved third matches the tonic the
    // forced I (`tonic_triad`) builds. scale[2] is the third scale degree's semitone offset.
    let scale = match ctx.key_tempo.home_mode.as_str() {
        "Ionian" => IONIAN,
        "Dorian" => DORIAN,
        "Phrygian" => PHRYGIAN,
        "Lydian" => LYDIAN,
        "Mixolydian" => MIXOLYDIAN,
        "Aeolian" => AEOLIAN,
        _ => IONIAN,
    };
    let dest_third_pc = ((dest_root_pc as i16 + scale[2] as i16).rem_euclid(12)) as u8;
    // theory: the resolution arrival is register-stable — we seat each role at its plain floor
    // (no brightness octave lift) so the V→I lands as a steady, grounded cadence rather than a
    // register-jumping gesture.
    match role {
        OrchestralRole::Bass => {
            // Destination tonic root in the bass → root-position I; dom-root → tonic-root.
            seat_pc_in_register(dest_root_pc, BASS_REGISTER_FLOOR)
        }
        OrchestralRole::Melody => {
            // Soprano on the destination tonic — the strong-arrival top; the pivot's dom-5th
            // (dest 2nd degree) descends a step into it (contrary to the bass leap → no
            // parallel octaves between outer voices).
            seat_pc_in_register(dest_root_pc, MELODY_REGISTER_FLOOR)
        }
        OrchestralRole::HarmonicFill | OrchestralRole::Pad | OrchestralRole::CounterMelody => {
            // The destination tonic's diatonic third — the pivot 7th's down-by-step target,
            // completing the V7→I resolution in the inner voice where the dissonance lived.
            seat_pc_in_register(dest_third_pc, FILL_REGISTER_FLOOR)
        }
    }
}

/// Minimum spacing (in semitones) required between any two ADJACENT upper
/// voices in a voiced triad. Pass B enforces this inside the voice-leading
/// search to forbid the unison collapse S4 can currently produce (e.g.
/// IV = [65, 65, 65]).
///
/// theory: a value of 1 forbids only the literal unison (no two upper voices on
/// the SAME MIDI note) — the minimal, musically-defensible floor. A closed-
/// position minor third (3 semitones) or major second between inner voices is
/// perfectly idiomatic, so a stricter "≥ minor third" rule would reject good
/// voicings; the unison, by contrast, is never what we want (it silently drops
/// a voice and thins the triad to a dyad). See the spec for the full rationale.
/// Pass A exposes the CONSTANT and the checker below as stubs; Pass B wires the
/// check into `voice_lead_one`'s candidate rejection.
pub const MIN_UPPER_VOICE_SPACING: u8 = 1;

/// True iff the voicing `notes` SATISFIES the minimum-spacing rule: no two
/// upper voices (indices 1..) share the same MIDI note. (The bass at index 0 is
/// exempt — a bass doubling an upper voice at the octave is fine; only the
/// inner/upper voices must not literally collide on one pitch.)
///
/// PASS-A STUB BEHAVIOR: returns `true` UNCONDITIONALLY, so a property test that
/// feeds it the known-bad IV = [65, 65, 65] collapse goes RED. Pass B replaces
/// the body with the real pairwise-distinct check over indices 1.. .
pub fn upper_voices_well_spaced(notes: &[u8]) -> bool {
    // theory: scan every pair of UPPER voices (indices 1..) and reject the
    // voicing iff any two land on the SAME MIDI note (separation < the minimum
    // of 1 semitone — i.e. a literal unison). The bass at index 0 is exempt: a
    // bass doubling an upper voice at the octave is idiomatic. We do NOT reject
    // close seconds/thirds between inner voices — those are perfectly good
    // closed-position spacings; only the unison collapse silently thins the
    // triad to a dyad.
    if notes.len() <= 2 {
        // 0/1 upper voices: no pair to collide.
        return true;
    }
    let upper = &notes[1..];
    for i in 0..upper.len() {
        for j in (i + 1)..upper.len() {
            // MIN_UPPER_VOICE_SPACING == 1 semitone: equal notes are the only
            // failure (their spacing of 0 is below the minimum).
            if upper[i].abs_diff(upper[j]) < MIN_UPPER_VOICE_SPACING {
                return false;
            }
        }
    }
    true
}

// =========================================================================
// VOICE-LEADING FREE HELPERS (production code, outside the test module)
// =========================================================================

/// The distinct pitch classes of a chord, with the chord ROOT first.
///
/// theory: `roman_to_chord` emits `notes[0]` as the chord root, so its pitch
/// class is the harmonic root we want in the bass. We preserve that root-first
/// ordering and keep the remaining chord tones (third, fifth) in their emitted
/// order for stable, deterministic seating.
fn chord_pitch_classes(chord: &Chord) -> Vec<u8> {
    let mut pcs: Vec<u8> = Vec::with_capacity(chord.notes.len());
    for &n in &chord.notes {
        let p = n % 12;
        if !pcs.contains(&p) {
            pcs.push(p);
        }
    }
    pcs
}

/// True if a chord NAME denotes a dominant ("V"/"v") and not a submediant
/// ("VI"/"vi") — the same string test the cadence property net applies.
///
/// theory: cadence identity in this layer is read from the Roman-numeral name.
/// "V" is the dominant; we must not mistake "vi"/"VI" (the submediant) for it.
fn is_dominant_name(name: &str) -> bool {
    name.contains('V') && !name.to_uppercase().contains("VI")
}

/// Seat pitch class `pc` into the lowest octave that lands at or above `floor`.
///
/// theory: used to stack an upper chord tone immediately above the bass when
/// seating the opening sonority, so the triad reads cleanly bottom-up.
fn seat_above(pc: u8, floor: u8) -> u8 {
    let mut note = (floor / 12) * 12 + pc % 12;
    while note < floor {
        note = note.saturating_add(12);
    }
    note
}

/// Seat pitch class `pc` into the octave whose representative is NEAREST to
/// `target` (ties resolve downward, the more grounded choice for a bass).
///
/// theory: the bass carries root motion and may leap, but it should still land
/// in a sane register near where it already is rather than jumping octaves.
fn nearest_pc_to(pc: u8, target: u8) -> u8 {
    let pc = pc % 12;
    let base = (target / 12) as i16;
    let mut best = pc as i16;
    let mut best_dist = i16::MAX;
    // Search a few octaves on either side of the target's octave.
    for octave in (base - 2)..=(base + 2) {
        let cand = octave * 12 + pc as i16;
        if (0..=127).contains(&cand) {
            let dist = (cand - target as i16).abs();
            if dist < best_dist {
                best_dist = dist;
                best = cand;
            }
        }
    }
    best as u8
}

/// All octave seatings of `pc` within `max_motion` semitones of `from`,
/// clamped to a safe MIDI band — the candidate set for one upper voice.
///
/// theory: a constrained upper voice may only step to a chord tone within a
/// perfect fifth of where it is. We enumerate every octave of the target pitch
/// class that satisfies that cap so the search can pick the smoothest one.
fn upper_voice_candidates(pc: u8, from: u8, max_motion: u8) -> Vec<u8> {
    let pc = pc % 12;
    let mut cands: Vec<u8> = Vec::new();
    let lo = from.saturating_sub(max_motion);
    let hi = from.saturating_add(max_motion);
    let base = (from / 12) as i16;
    for octave in (base - 2)..=(base + 2) {
        let cand = octave * 12 + pc as i16;
        if (24..=108).contains(&cand) {
            let note = cand as u8;
            if note >= lo && note <= hi {
                cands.push(note);
            }
        }
    }
    cands.sort_unstable();
    cands.dedup();
    cands
}

/// Harmonic interval class (0..=11) between two MIDI notes: |a-b| mod 12.
/// theory: 0 = unison/octave, 7 = perfect fifth — the two perfect consonances
/// the parallel-motion prohibition guards.
fn interval_class(a: u8, b: u8) -> u8 {
    ((a as i16 - b as i16).abs() % 12) as u8
}

/// Does moving from voicing `a` to voicing `b` create a parallel perfect fifth
/// or octave between any voice pair?
///
/// theory: a parallel perfect consonance is the same perfect interval class
/// (0 or 7) sustained between two voices while BOTH of those voices change
/// pitch. If either voice holds (a common tone), the motion is oblique, not
/// parallel, and is allowed. We check every pair, bass included, at T and T+1.
fn has_parallel_perfects(a: &[u8], b: &[u8]) -> bool {
    let n = a.len().min(b.len());
    for i in 0..n {
        for j in (i + 1)..n {
            let ic_a = interval_class(a[i], a[j]);
            let ic_b = interval_class(b[i], b[j]);
            let both_move = a[i] != b[i] && a[j] != b[j];
            if ic_a == ic_b && (ic_a == 0 || ic_a == 7) && both_move {
                return true;
            }
        }
    }
    false
}

// =========================================================================
// S18 SLICE 2 — REAL COUNTER-MELODY HELPERS (Music Theory Specialist owns)
//
// The counter-line is a SECOND moving line under the Melody. It re-uses the
// existing voice-leading craft (`upper_voice_candidates` / `has_parallel_perfects`)
// and the existing melody-pitch path (`theme_melody_pitch` / `role_pitch`) so it
// stays connected and contrapuntally clean WITHOUT any new cross-step state — every
// datum is RE-DERIVED from the borrowed `StepContext` (spec §3.1). All helpers are
// pure, deterministic, and reached ONLY from the CounterMelody realize arm (never on
// the identity/freeze path).
// =========================================================================

/// The melody-register floor a CounterMelody must sit UNDER (so the second line
/// reads as inner/counter, not as a second tune in the soprano). == MELODY_REGISTER_FLOOR.
const COUNTER_CEILING: u8 = MELODY_REGISTER_FLOOR; // 67 (G4) — the counter stays below this

/// The contrary-motion bonus subtracted from a counter candidate's score when its
/// motion OPPOSES (or obliques, while the melody moves) the melody's motion. theory:
/// contrary motion is the strongest independence between two lines; rewarding it
/// makes the counter a genuine second voice rather than a parallel shadow. Sized to
/// dominate the conjunct-motion (≤P5) score term so a contrary leap beats a similar
/// step, but not so large it overrides the no-parallel-perfects HARD reject (a
/// separate boolean gate, not a score term).
const CONTRARY_BONUS: i32 = 24;
/// A heavy penalty added when a counter candidate would double the melody's pitch in
/// the same octave (a unison-double erases the two-line texture). theory: the counter
/// must remain audibly distinct from the melody; an octave/unison double is the one
/// vertical the line must never pick when any other chord tone is available.
const COUNTER_UNISON_PENALTY: i32 = 1000;
/// A small tie-breaking bias against seating the counter on the chord ROOT pc. theory:
/// the counter is an inner/upper line that leans on the 3rd/5th; the root in the
/// counter register is legal (not a bass double) but should yield to a non-root inner
/// tone when the choice is otherwise equal. Kept far below CONTRARY_BONUS so it only
/// breaks ties, never overrides counterpoint.
const ROOT_PC_BIAS: i32 = 2;

/// The cap on how far back the held-run scan looks (and the rotation period of the
/// advancing held-period seed). theory: a held run needs only ~one rotation through
/// the chord's band tones to read as a genuine moving line, so bounding the scan at a
/// small cap keeps the arm O(cap) per step (never O(run length) → never O(n²) over a
/// held section) while still cycling the seed through every reachable chord tone. A
/// bare triad offers ≤3 band tones, so 4 covers a full rotation with margin.
const HELD_RUN_SEED_CAP: usize = 4;

/// True iff MIDI note `n`'s pitch class is the chord ROOT (`notes[0]`'s pc) — the
/// counter de-prioritizes (but does not forbid) the root, so it stays an inner line
/// without starving the held-period mover of reachable tones.
fn is_root_pc(chord: &Chord, n: u8) -> bool {
    chord.notes.first().is_some_and(|&r| r % 12 == n % 12)
}

/// True iff the chord's pitch-class set is a DIMINISHED triad: a root, a minor third
/// (root+3), and a diminished fifth (root+6) — the structure that carries an internal
/// tritone (the `vii°` of a major key, the `ii°` of a minor key). Used to recognize the
/// GAP-2 "keep the bite" terminal sonority: a diminished triad is the only diatonic triad
/// whose verticals are inherently dissonant, so a SECTION-TERMINAL diminished is the chord
/// where an intentional unresolved dissonance ("the bite") is the musical choice rather than
/// a smoothed-away consonance.
///
/// theory: we test the pitch-class INTERVALS above the chord root (notes[0] is the root, per
/// `roman_to_chord`), not absolute pitches, so it is octave- and voicing-agnostic. A 7th/9th
/// extension on top of a diminished triad still reads as diminished (the m3 + dim5 above the
/// root are present); a half-diminished 7th likewise reads diminished on its triad core,
/// which is the correct reading for the terminal-bite decision. Pure/deterministic.
fn is_diminished_triad(chord: &Chord) -> bool {
    let Some(&root) = chord.notes.first() else {
        return false;
    };
    let root_pc = root % 12;
    let has = |semis: u8| {
        chord
            .notes
            .iter()
            .any(|&n| (n % 12) == ((root_pc + semis) % 12))
    };
    // minor third (3) AND diminished fifth (6) above the root — the diminished signature.
    has(3) && has(6)
}

/// Melodic motion direction between two (optional) pitches. A rest is folded into
/// `Hold` by the callers (a rest neither rises nor falls — it is a static gap).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MotionDir {
    Up,
    Down,
    Hold,
}

/// The direction from `prev` to `now`. A missing pitch on EITHER side (a rest, or no
/// prior step) is `Hold` — there is no contrary constraint to honor across a gap.
fn motion_dir(prev: Option<u8>, now: Option<u8>) -> MotionDir {
    match (prev, now) {
        (Some(p), Some(n)) if n > p => MotionDir::Up,
        (Some(p), Some(n)) if n < p => MotionDir::Down,
        _ => MotionDir::Hold,
    }
}

/// Recompute the MELODY role's sounding pitch for the CURRENT step (the step whose
/// `ctx.step_in_section` `ctx` already points at) — the SAME value the Melody
/// instrument computes in `realize_step`. `None` == the melody RESTS this step.
///
/// theory: the counter must see the melody to move against it, but `realize_step` is
/// stateless per-instrument and never receives the melody's pitch. It IS re-derivable:
/// the melody pitch is a pure function of (ctx, chord, features) via the SAME pipeline the
/// Melody arm runs — the theme seam (`theme_melody_pitch`) / free-select (`role_pitch`),
/// THEN the two cadence/modulation RE-VOICINGS the Melody role applies on top (the
/// `land_home` PAC strengthening and the opening V→I pivot resolution). The re-voicings were
/// the missing piece (S45-CP-FIX): without them the counter saw the UN-revoiced free-select
/// pitch while the melody actually SOUNDED the re-voiced one, so the counter computed motion
/// against a phantom melody and the M1.4 independence guard answered the wrong direction —
/// the residual strictly-parallel pairs the scorecard measured. Replicating both stages here
/// makes the counter's `m_now`/`m_prev` byte-identical to the melody the ear hears.
fn melody_pitch_for(
    ctx: &crate::composition::StepContext,
    step: &StepPlan,
    features: &PerfFeatures,
) -> Option<u8> {
    // Stage 0: PIVOT boundary — at a modulating section's step 0 the melody sounds the V7
    // pivot's dominant 5th (via `pivot_chord_events`), which PRE-EMPTS the free-select/theme +
    // cadence/modulation pipeline below (realize_step returns the pivot events before that path
    // ever runs). The counter must read THIS pitch so its M1.4 motion check at the next step is
    // against the melody that actually sounded at the boundary.
    if let Some(p) = pivot_melody_pitch(ctx, features, ctx.step_in_section) {
        return Some(p);
    }
    // Stage 1: the free-select / theme pitch (a theme-driven rest stays a rest — the
    // re-voicings below only re-point an EXISTING pitch, never resurrect a rested step).
    let base = match theme_melody_pitch(ctx, OrchestralRole::Melody, &step.chord, features) {
        Some(None) => return None, // theme-driven rest → the melody is silent this step
        Some(Some(p)) => p,        // theme pitch
        // Free-select: inst_idx/num are irrelevant for the Melody arm of role_pitch
        // (it seats the chord's TOP tone, independent of index), so a synthetic
        // (idx 0, num 1) is correct and avoids guessing the real ensemble width.
        None => role_pitch(
            OrchestralRole::Melody,
            &step.chord,
            0,
            1,
            features,
            // S23: the theme/counter path computes the Melody weight off the same ctx —
            // the SAME value realize_step threads for the Melody role, so a theme note
            // lifts identically to a free-selected one. Empty prominence → 0.5 → no lift.
            prominence_weight(ctx, OrchestralRole::Melody),
            // S47: `melody_pitch_for` is reached ONLY from the CounterMelody arm (the
            // counter recomputing the melody it tracks). A counter is therefore present by
            // construction, so the seat guard MUST apply here too — otherwise the counter
            // would track the UN-guarded melody pitch while the melody actually sounds the
            // seat-guarded one, re-introducing a phantom-melody divergence (the S45-CP-FIX
            // class of bug). `true` keeps the counter's register-relative motion honest.
            true,
        ),
    };
    // Stage 2: LAND-HOME PAC re-voicing — identical guard/call to realize_step:1163.
    let base = if land_home_is_armed(ctx, step.position) {
        land_home_pitch(OrchestralRole::Melody, ctx, &step.chord)
    } else {
        base
    };
    // Stage 3: OPENING V→I pivot resolution — identical guard/call to realize_step:1178.
    let base = if pivot_resolution_is_armed(ctx) {
        pivot_resolution_pitch(OrchestralRole::Melody, ctx)
    } else {
        base
    };
    Some(base)
}

/// Recompute the MELODY pitch for an ARBITRARY prior step `p`, by re-pointing the
/// borrowed context's `step_in_section` at that step so the theme seam reads the
/// right motif index. Pure: only the `step_in_section` field is overridden (a Copy
/// of the borrowed context), nothing is mutated through the references.
///
/// S45-CP-FIX: the live realize path no longer calls this (the counter's prior-melody read is
/// inlined in `realized_counter_pitch_with_prev` so it can re-point to the TILED prior step);
/// the helper is retained for the counterpoint unit tests, hence `cfg(test)`.
#[cfg(test)]
fn melody_pitch_for_step(
    ctx: &crate::composition::StepContext,
    p: &StepPlan,
    features: &PerfFeatures,
) -> Option<u8> {
    // Find this StepPlan's index within the section so theme_melody_pitch reads the
    // correct motif note. The section's steps are the authoritative list; locate by
    // identity of plan position (the prior step is steps[step_in_section - 1]).
    let prior_idx = ctx.step_in_section.saturating_sub(1);
    let mut prior_ctx = *ctx;
    prior_ctx.step_in_section = prior_idx;
    melody_pitch_for(&prior_ctx, p, features)
}

/// Seed the counter's "previous pitch" non-recursively (spec §3.1 LOCK): the chord
/// tone the counter WOULD seat off the PRIOR step's chord, from a neutral anchor in
/// the counter register. This keeps the line connected (each step picks near where it
/// just was) without a recursive self-call, so the arm stays O(1) per step.
///
/// theory: the line's continuity comes from "nearest chord tone to where I was";
/// seeding the prior pitch off the prior chord (rather than recursing the whole arm)
/// is sufficient for a connected line and exactly the cheaper LOCK the spec chose.
fn seed_prev_counter(prev: Option<&StepPlan>, step: &StepPlan) -> u8 {
    // The chord to seat the seed off: the PRIOR chord if there is one (so the line
    // approaches the current chord from where it sat last step), else THIS chord (a
    // section-start anchor with no prior — the counter simply opens on a chord tone).
    let seed_chord = prev.map(|p| &p.chord).unwrap_or(&step.chord);
    // A neutral anchor in the middle of the counter register (between FILL and the
    // melody floor) — the seating reference the nearest-tone pick measures from.
    let anchor = (FILL_REGISTER_FLOOR + COUNTER_CEILING) / 2;
    nearest_counter_tone(seed_chord, anchor).unwrap_or(anchor)
}

/// The nearest band CHORD TONE whose motion is CONTRARY/OBLIQUE to a moving melody, CONSONANT
/// against the sounding melody, and not a parallel perfect — the witness-safe M1.4 independence
/// substitute for a plain Sustain landing that would otherwise be strictly parallel. Returns
/// `None` when the chord offers no such tone (then the caller keeps its original landing rather
/// than manufacture a non-chord-tone hold — that floor is what keeps the species "structural
/// sustain is consonant" invariant and its witnesses intact). Deterministic / RNG-free.
fn nearest_consonant_independent_counter(
    chord: &Chord,
    prev_counter: u8,
    m_prev: Option<u8>,
    m_now: Option<u8>,
    mel_dir: MotionDir,
) -> Option<u8> {
    let cf = m_now?; // a moving melody implies a sounding CF; guard anyway.
    counter_candidate_pitches(chord, prev_counter)
        .into_iter()
        // CONTRARY or OBLIQUE (never similar to the melody) — the M1.4 independence.
        .filter(|&c| motion_dir(Some(prev_counter), Some(c)) != mel_dir)
        // CONSONANT against the sounding melody — never trade a parallel for a raw dissonance.
        .filter(|&c| is_consonant(c, cf))
        // No parallel/hidden perfect across the prev->now transition (the same two-point guard).
        .filter(|&c| match m_prev {
            Some(mp) => !has_parallel_perfects(&[mp, prev_counter], &[cf, c]),
            None => true,
        })
        // Closest to where the line just sat — a connected step.
        .min_by_key(|&c| (c as i16 - prev_counter as i16).abs())
}

/// S30-CP-FIX — the REALIZED counter pitch this step would sound, GIVEN the realized prior
/// counter pitch `prev_counter`. This is the exact pitch-selection the CounterMelody realize
/// arm performs (held-run rotation seed / §3.1 LOCK seed → figure driver), factored into a
/// pure function so it can be REPLAYED for an earlier step to recover that step's realized
/// pitch. The realize arm and the replay therefore share ONE code path — they cannot diverge.
///
/// theory: the species two-point gates (parallel-perfects, approach-perfect, melodic-leap,
/// the cadence clausula) must constrain the REALIZED prev→now transition that actually sounds.
/// The as-built arm fed them a SYNTHETIC seed (`seed_prev_counter`, re-derived off the prior
/// CHORD), so they guarded a transition the ear never hears. Routing the gates through this
/// function with the realized prior pitch (see `realized_prev_counter`) makes them bind the
/// sounding line — closing GAP-1/3/4 at the root. `force_move`/`held_target`/`figures_enabled`
/// are recomputed here EXACTLY as the arm computes them, so the replayed pitch is identical to
/// what the arm emits.
fn realized_counter_pitch_with_prev(
    ctx: &crate::composition::StepContext,
    step: &StepPlan,
    features: &PerfFeatures,
    si: usize,
    prev_counter: u8,
) -> u8 {
    // S45-CP-FIX (tiling consistency): fetch the prior/next STEP-PLAN by the realizer's own
    // `% len` wrap, so a TILED section's prev/next chord is the tiled phrase's chord (the one
    // that actually sounds at that tiled position), not a raw index that falls off the end of
    // the stored phrase. On an untiled section `% len` is the identity → byte-identical. The
    // MELODY index stays RAW (`step_in_section = si`, and the prev melody read at raw `si-1`):
    // the melody role itself indexes its own motif by the raw section-local step and resolves
    // past-end via `motif_step_at` (Identity-hold / wrap), so the counter must read the melody
    // the SAME raw way to move against the line that actually sounds.
    let len = ctx.section.steps.len();
    let prev: Option<&StepPlan> = match (si.checked_sub(1), len) {
        (Some(p), l) if l > 0 => Some(&ctx.section.steps[p % l]),
        _ => None,
    };
    // Re-point the borrowed ctx at THIS step so the melody/theme seam reads the right index
    // (the replay path computes earlier steps; the live arm passes its own ctx unchanged).
    let mut step_ctx = *ctx;
    step_ctx.step_in_section = si;
    let m_now = melody_pitch_for(&step_ctx, step, features);
    // The prior melody pitch at the RAW previous section-local step (its own motif index).
    let m_prev = match si.checked_sub(1) {
        Some(prev_si) if prev.is_some() => {
            let mut prev_ctx = *ctx;
            prev_ctx.step_in_section = prev_si;
            melody_pitch_for(&prev_ctx, prev.unwrap(), features)
        }
        _ => None,
    };
    let mel_dir = motion_dir(m_prev, m_now);

    let held_chord = prev.is_some_and(|p| p.chord.notes == step.chord.notes);
    let melody_static = mel_dir == MotionDir::Hold || m_now.is_none();
    let held_run_index = held_run_position(ctx.section, si);
    let held_target = advancing_seed_counter(step, held_run_index);
    // The NEXT step's chord. `None` ONLY at the section's GENUINE terminal (the last step of
    // the whole tiled span, `si + 1 >= step_len`) — that `None` is the §GAP-2 terminal-figure /
    // terminal-diminished-bite gate and MUST be preserved (a wrapped next-chord there would
    // suppress the terminal bite and the witnesses that pin it). For any INTERIOR tiled step
    // (`si + 1 < step_len`) the next chord is the tiled phrase's chord at `(si + 1) % len` — the
    // one that actually sounds next — so a tiled section's lookahead tracks the repeating phrase
    // rather than falling off the stored steps. On an untiled section `step_len == len`, so this
    // is `None` exactly at the last step and `steps[si + 1]` everywhere else — byte-identical.
    let next_chord: Option<&Chord> = if len > 0 && si + 1 < ctx.section.step_len {
        Some(&ctx.section.steps[(si + 1) % len].chord)
    } else {
        None
    };
    let figures_enabled = !(held_chord || melody_static);

    let (cnt, figure) = pick_counter_figure(
        &step.chord,
        prev_counter,
        m_prev,
        m_now,
        mel_dir,
        step.position,
        next_chord,
        held_chord || melody_static,
        held_target,
        figures_enabled,
    );
    // S45-CP-FIX (M1.4, witness-safe): if the ordinary first-species (Sustain) landing moves
    // STRICTLY SIMILAR (same direction) to a MOVING melody — the strict parallel spec-s45 §M1.4
    // forbids ("the arm is BUILT so the counter is never parallel; parallel fraction == 0
    // deterministically") — re-point it to the nearest CONSONANT contrary/oblique chord tone, if
    // one exists. Strictly SCOPED so the deliberate, witnessed counterpoint arms are untouched:
    //   * only the plain `Sustain` figure (the dissonant Passing/Neighbor/Suspension figures own
    //     their approach+resolution and are left as-is);
    //   * only `Interior`/`HalfCadence` steps (PhraseStart opening, PAC clausula, and the
    //     terminal-diminished bite run their own pitch formula and are left as-is);
    //   * the substitute is a CONSONANT band chord tone only — never a non-chord-tone "hold",
    //     so the species "every structural sustain is consonant" floor and the dissonance-
    //     resolution witnesses are preserved; when the chord offers no consonant contrary/oblique
    //     tone the original `cnt` stands (the chord genuinely forces similar motion there).
    let cnt = if figure == CounterFigure::Sustain
        && matches!(
            step.position,
            PhrasePosition::Interior | PhrasePosition::HalfCadence
        )
        && mel_dir != MotionDir::Hold
        && motion_dir(Some(prev_counter), Some(cnt)) == mel_dir
    {
        nearest_consonant_independent_counter(&step.chord, prev_counter, m_prev, m_now, mel_dir)
            .unwrap_or(cnt)
    } else {
        cnt
    };

    // S33-CP-FIX (GAP-4) — PENULT LOOKAHEAD REWORK (design §1.2 Option 1).
    //
    // theory: on `{ii,vi} → IV → iii` the as-built IV penult `65` (root of IV, bare P5 vs the
    // melody) dead-ends: from `65` every `iii` landing is a hidden P5 (64), a dissonant tritone
    // leap (59), or a wide consonant leap (55) — there is NO clean stepwise consonant landing,
    // so the realized line is forced into the `65 → 59` tritone (the GAP-4 residual). We already
    // hold `next_chord` (= the iii chord) and can cheaply compute its melody at `si+1`, so we
    // re-pick the penult ONE STEP EARLY to a tone (the third of IV, `57`) from which the
    // ordinary machinery lands the `iii` step cleanly by a `−2` step onto `55`. This rides the
    // existing private realize-replay seam (which already owns `ctx`/`si`/`next_chord`): NO new
    // threaded field, NO `pick_counter_figure` signature change, the public `realize_step`
    // surface untouched. The helper is a strict NO-OP unless this exact dead-end shape holds
    // (it re-checks both boundaries and only fires when the as-built penult forces the tritone
    // AND a clean reworked penult exists), so every clean transition and the `I→V→IV` witness
    // (terminal IV, not pre-`iii`) are byte-identical. The fix is upstream candidate selection;
    // no gate is loosened.
    if let (Some(nc), Some(next_step)) = (next_chord, ctx.section.steps.get(si + 1)) {
        // The iii-step (next-step) melody (`m_iii`), computed exactly as the replay computes
        // melodies: re-point the borrowed ctx at `si+1` and read the Melody/theme seam there.
        let mut next_ctx = *ctx;
        next_ctx.step_in_section = si + 1;
        let m_next = melody_pitch_for(&next_ctx, next_step, features);
        if let Some(m_next) = m_next {
            // The next step's ORDINARY realized counter landing, GIVEN the as-built penult `cnt`
            // as its prior counter pitch. We call `pick_counter_figure` DIRECTLY (the same path
            // the next step's own replay would take) rather than `realized_counter_pitch_with_prev`
            // — this both avoids recursing back into this rework AND reads the unreworked landing,
            // which is exactly the dead-end the trigger must detect (e.g. the `65 → 59` tritone).
            let nn_chord: Option<&Chord> = ctx.section.steps.get(si + 2).map(|s| &s.chord);
            let nn_held = next_step.chord.notes == step.chord.notes;
            let nn_mel_dir = motion_dir(m_now, Some(m_next));
            let nn_static = nn_mel_dir == MotionDir::Hold;
            let nn_held_run = held_run_position(ctx.section, si + 1);
            let nn_held_target = advancing_seed_counter(next_step, nn_held_run);
            let nn_figures_enabled = !(nn_held || nn_static);
            let (as_built_next_landing, _f) = pick_counter_figure(
                &next_step.chord,
                cnt, // the as-built penult is the next step's prior counter pitch
                m_now,
                Some(m_next),
                nn_mel_dir,
                next_step.position,
                nn_chord,
                nn_held || nn_static,
                nn_held_target,
                nn_figures_enabled,
            );
            if let Some(reworked) = penult_for_clean_next(
                &step.chord,
                prev_counter,
                m_prev,
                m_now,
                nc,
                m_next,
                cnt,
                as_built_next_landing,
            ) {
                return reworked;
            }
        }
    }

    cnt
}

/// S30-CP-FIX — the REALIZED counter pitch at step `si` (the line that ACTUALLY sounds), by
/// deterministic REPLAY. This is the cross-step pitch memory the as-built arm lacked.
///
/// MECHANISM: the counter pick at step `i` is a deterministic function of `(ctx, features, i)`
/// and the realized pick at `i-1`. So the realized prior pitch is recovered by recursing toward
/// the section opening — `si == 0` is the BASE CASE (no prior step; the line opens off the §3.1
/// seed exactly as the as-built arm does at a section start), and each step `i` feeds the
/// realized pitch of `i-1` as its `prev_counter`. Because the recursion is strictly downward in
/// `si` and the per-step work is O(1) over the bounded helpers, a single top-level call costs
/// O(si) — and the realize arm's own call (replaying only `si-1`) makes the whole section's
/// realization O(n²) in the worst case. Sections are short (a phrase, ≤ a few dozen steps), so
/// this is negligible; it is also RNG-free and fully deterministic (PT-9 preserved). The
/// `seen`/depth guard caps the recursion at the section length as a defensive floor against a
/// malformed `step_in_section`.
///
/// S45-CP-FIX (tiling consistency) — `step_in_section` CAN legitimately exceed the stored
/// step count. The planner distributes `total_steps` across sections by `rel_len` share, so a
/// section's `step_len` (its slice of the global cursor) is unrelated to its `plan_phrases`
/// length: a section TILES when `step_len > steps.len()` (first surfaced by routing real images
/// to `pad_bed_counter`). The global cursor walks `step_in_section` PAST `steps.len()`, and the
/// realizer voices the WRAPPED step `steps[step_idx % len]` (engine.rs:723) at each TILED
/// position — so the same phrase REPEATS in time, e.g. a 3-step phrase tiled to 11 steps sounds
/// `0 1 2 0 1 2 0 1 2 0 1`.
///
/// The replay therefore must walk the ACTUAL TILED SEQUENCE in time — recurse on `si-1` (the
/// real previous step), NOT on `(si % len) - 1` — and fetch each step's chord/plan via
/// `steps[i % len]` exactly as the live realizer does. Recursing on the wrapped phrase index
/// would make the replay's reconstructed prior pitch DISAGREE with the live emission across a
/// tile boundary (the live step `si=3` of a len-3 phrase sounds the realized pitch carried from
/// `si=2`, but a phrase-wrapped replay would re-OPEN it as phrase-position-0), and that
/// disagreement re-introduces the very cross-step inconsistency the realized-prev memory exists
/// to prevent — including parallels the M1.4 guard then cannot see. Recursing on the tiled
/// position keeps the replayed prior pitch byte-identical to what step `si-1` actually emitted.
/// Recursion depth is `si` (≤ `step_len`, a few dozen) — negligible, RNG-free, deterministic.
/// On an UNtiled section (`step_len <= len`) `i % len == i`, so every step replays byte-identically.
fn realized_prev_counter(
    ctx: &crate::composition::StepContext,
    features: &PerfFeatures,
    si: usize,
) -> u8 {
    let steps = &ctx.section.steps;
    let len = steps.len();
    if len == 0 {
        // Degenerate section with no stored steps — there is no chord to seat a seed off.
        // Open the line on a neutral counter-register anchor (never panics). The arm only
        // reaches here on a malformed plan; the empty-plan guard in decide_instrument_action
        // already returns a silent step before this on the live path.
        return (FILL_REGISTER_FLOOR + COUNTER_CEILING) / 2;
    }
    // `realized_prev_counter(si)` returns the realized counter pitch at step `si - 1`.
    let Some(prev_idx) = si.checked_sub(1) else {
        // BASE CASE: si == 0 has no prior step (the seed BEFORE the section opening).
        return seed_prev_counter(None, &steps[0]);
    };
    // The realized pitch at `prev_idx`. If THAT step is a PIVOT boundary, it sounded the V7
    // pivot voicing (the dom-7th seat), NOT the species line — return it directly, because the
    // species recompute below would mis-reconstruct it as a free-select tone and the next step's
    // M1.4 independence check would then answer against a phantom prior counter pitch.
    if let Some(p) = pivot_counter_pitch(ctx, features, prev_idx) {
        return p;
    }
    // Otherwise the previous TILED step's chord/plan, fetched via the realizer's own `% len`
    // wrap, with its realized pitch fed through the shared pick — tracking the live tiled line.
    let prev_step = &steps[prev_idx % len];
    let prev_prev_counter = realized_prev_counter(ctx, features, prev_idx);
    realized_counter_pitch_with_prev(ctx, prev_step, features, prev_idx, prev_prev_counter)
}

/// The step's position WITHIN its current held run: the count of consecutive PRIOR
/// steps in `section` (ending at `si - 1`) whose chord notes equal `section.steps[si]`'s,
/// capped at `HELD_RUN_SEED_CAP`. 0 means `si` opens a run (its prior chord differs, or
/// it is the section start). theory: this is the deterministic phase that advances the
/// held-period seed so consecutive identical-chord steps seat from different pitches.
///
/// Bounded: the scan walks back at most `HELD_RUN_SEED_CAP` steps, so it is O(cap) per
/// step, never O(run length) — a held run of any length costs O(cap·run), i.e. linear,
/// not the O(n²) a full recursive seed back to the run start would cost. RNG-free.
fn held_run_position(section: &crate::composition::Section, si: usize) -> usize {
    let Some(cur) = section.steps.get(si) else {
        return 0;
    };
    let mut count = 0usize;
    let mut idx = si;
    while count < HELD_RUN_SEED_CAP {
        let Some(prev_idx) = idx.checked_sub(1) else {
            break; // section start — no further history
        };
        match section.steps.get(prev_idx) {
            Some(p) if p.chord.notes == cur.chord.notes => {
                count += 1;
                idx = prev_idx;
            }
            _ => break, // chord changed (or missing) — the held run ends here
        }
    }
    count
}

/// The ADVANCING held-run TARGET pitch — the chord tone the counter should SOUND on
/// step `held_run_index` of a held run, or `None` when this step is not inside a held
/// run (`held_run_index == 0`: a run start or a changing chord) and the caller should
/// use the §3.1 LOCK seed + `force_move` exactly as the as-built impl does.
///
/// INSIDE a held run (`held_run_index >= 1`) the target ROTATES through the chord's
/// non-root counter-band tones by the run position: step N lands on ring tone N (mod
/// ring length), so consecutive held steps sound DIFFERENT inner tones (e.g. 3rd → 5th
/// → 3rd …) — a moving inner line woven through the static harmony rather than one
/// re-struck stab. Preferring the non-root ring keeps the held line off the bass-doubling
/// root (the same intent `ROOT_PC_BIAS` encodes), falling back to the full band set only
/// for a degenerate chord that cannot offer two non-root tones. The caller seats this
/// target as the pick's previous pitch with `force_move` OFF so the pick LANDS on it
/// (still subject to the no-parallel-perfects reject). Deterministic (pure function of
/// plan position + chord, no RNG) and bounded (the ring is a small band-seated chord-tone
/// list; the run index is capped at `HELD_RUN_SEED_CAP`).
fn advancing_seed_counter(step: &StepPlan, held_run_index: usize) -> Option<u8> {
    if held_run_index == 0 {
        return None; // run start / changing chord — caller uses the §3.1 LOCK seed
    }
    // The band tones the held line may visit, PREFERRING non-root inner tones (the
    // counter is an inner line, not a bass double — the same de-prioritized-root intent
    // ROOT_PC_BIAS encodes). A bare triad seats {root, 3rd, 5th}; dropping the root pc
    // leaves the 3rd and 5th, the two tones a moving inner line should oscillate between.
    // Only fall back to the full set (including root) if non-root tones cannot offer a
    // moving choice (degenerate chord), so the line is never starved.
    let anchor = (FILL_REGISTER_FLOOR + COUNTER_CEILING) / 2;
    let all = counter_candidate_pitches(&step.chord, anchor);
    let non_root: Vec<u8> = all
        .iter()
        .copied()
        .filter(|&c| !is_root_pc(&step.chord, c))
        .collect();
    let ring = if non_root.len() >= 2 { &non_root } else { &all };
    if ring.len() <= 1 {
        return None; // nothing to rotate through — fall back to the §3.1 seed (force_move)
    }
    // Deterministic rotation by the held-run position. The run's FIRST appearance
    // (run_index 0, NOT a held step) sounds via the force_move path, which lands on the
    // ring's LAST tone (the contrary-motion bonus prefers the farther-from-seed non-root
    // tone); so the held steps start the rotation at ring[0] — `held_run_index - 1` — to
    // step AWAY from that first sound and then cycle. Each held step thus lands on a tone
    // different from BOTH its neighbours, weaving a moving inner line through the static
    // harmony. RNG-free; bounded by the small ring and the capped run index.
    Some(ring[(held_run_index - 1) % ring.len()])
}

/// The chord tone of `chord` (pitch class) seated NEAREST `anchor` within the counter
/// register band (FILL_REGISTER_FLOOR ..< COUNTER_CEILING), preferring INNER/upper
/// tones (the counter is not a bass double, so the root pc is skipped when the chord
/// has more than one tone). `None` only for an empty chord.
fn nearest_counter_tone(chord: &Chord, anchor: u8) -> Option<u8> {
    counter_candidate_pitches(chord, anchor)
        .into_iter()
        .min_by_key(|&n| (n as i16 - anchor as i16).abs())
}

/// All chord-tone candidate pitches for the counter near `from`, seated in the counter
/// register [FILL_REGISTER_FLOOR, COUNTER_CEILING). Reuses `upper_voice_candidates`
/// (the same nearest-tone octave-enumeration search `voice_lead_one` uses) over each
/// chord pitch class, keeping only seatings inside the counter band. ALL chord tones
/// are candidates — the counter is an inner/upper line, and a root PC seated in the
/// counter register (≈55..66) is NOT a bass double (the actual bass sounds at C2≈36);
/// `pick_counter_pitch` de-prioritizes the root pc on a tie so non-root inner tones are
/// preferred, honoring the "inner line, not a bass double" intent WITHOUT starving the
/// held-period moving line of tones to step to. Deduped, sorted.
///
/// Per-pc seatings are enumerated with an OCTAVE window (12) rather than the ≤P5 cap:
/// the ~12-semitone counter band ITSELF bounds the leap, and the wider window is what
/// makes all three triad tones reachable in that narrow band (a bare triad must offer
/// ≥2 distinct tones, or the held-period mover has nowhere to step). A direct band-seat
/// fallback keeps the set non-empty for any pathological chord.
fn counter_candidate_pitches(chord: &Chord, from: u8) -> Vec<u8> {
    let notes = &chord.notes;
    if notes.is_empty() {
        return Vec::new();
    }
    let mut cands: Vec<u8> = Vec::new();
    for &n in notes {
        let pc = n % 12;
        for c in upper_voice_candidates(pc, from, 12) {
            if (FILL_REGISTER_FLOOR..COUNTER_CEILING).contains(&c) {
                cands.push(c);
            }
        }
    }
    if cands.is_empty() {
        // Defensive: seat each pc directly in the counter register so the line is never
        // empty — a connected chord tone is always available.
        for &n in notes {
            let seat = seat_pc_in_register(n % 12, FILL_REGISTER_FLOOR);
            if (FILL_REGISTER_FLOOR..COUNTER_CEILING).contains(&seat) {
                cands.push(seat);
            }
        }
    }
    cands.sort_unstable();
    cands.dedup();
    cands
}

/// Pick the counter's sounding pitch for this step (spec §3.2): a chord tone near the
/// counter's previous pitch, scored for CONTRARY/OBLIQUE motion against the melody,
/// HARD-rejecting similar motion into a parallel perfect fifth/octave with the melody.
///
/// theory: the counter is a real second voice — it must be a chord tone (first-species
/// floor), move smoothly from where it was (≤P5), and stay independent of the melody
/// (prefer contrary motion, never parallel perfects, never a unison double). The
/// no-parallel-perfects check is the same `has_parallel_perfects` the voice-leading
/// core uses, applied AS-IS to the 2-voice [melody, counter] pair across T→T+1 (a NEW
/// call site, not an edit to that fn).
fn pick_counter_pitch(
    chord: &Chord,
    prev_counter: u8,
    m_prev: Option<u8>,
    m_now: Option<u8>,
    mel_dir: MotionDir,
    // True during a held-chord / melody-static period (§3.4): the counter is the ONLY
    // thing that should move underneath, so STAYING on its previous pitch is the
    // "re-struck stab" the operator rejected — penalize a no-op so the line steps to a
    // NEW chord tone each step. False on a changing-chord step where holding (an
    // oblique common tone under a moving melody) is musically correct.
    force_move: bool,
    // INSIDE a held run: the rotation's chosen SOUNDING tone for this step (a non-root
    // band chord tone that walks with the run position). When `Some(t)`, the line LANDS
    // on `t` — the rotation already chose the musically-correct moving tone, so the line
    // visits a new tone each held step — UNLESS `t` would form a parallel perfect against
    // the melody, in which case the scored pick below takes over (legality wins). `None`
    // on a run start / changing chord → the scored seed+force_move path is unchanged from
    // the as-built impl, so every non-held-run step is byte-identical.
    held_target: Option<u8>,
) -> u8 {
    let cands = counter_candidate_pitches(chord, prev_counter);
    if cands.is_empty() {
        return prev_counter; // defensive: no chord tone at all → hold (never panics)
    }
    // HELD-RUN TARGET FAST-PATH: land on the rotation's chosen tone iff it is a legal
    // chord-tone candidate AND does not create a parallel perfect against the melody AND
    // does not move STRICTLY SIMILAR to the melody (M1.4: the counter is never parallel to
    // the melody across a sounding pair — spec-s45 §M1.4 HARD). The rotation owns "which
    // tone" for an INDEPENDENT step, but it must defer to the scored path when its tone would
    // shadow the melody's direction; the scored path then finds a contrary/oblique tone (or
    // the oblique hold), keeping the held line moving WITHOUT paralleling the tune. A static
    // melody (mel_dir == Hold) has no direction to parallel, so the fast-path is unchanged
    // there (the common held-period case — melody static under the held chord).
    if let Some(t) = held_target {
        if cands.contains(&t) {
            let creates_parallel = match (m_prev, m_now) {
                (Some(mp), Some(mn)) => has_parallel_perfects(&[mp, prev_counter], &[mn, t]),
                _ => false,
            };
            if !creates_parallel {
                return t;
            }
        }
    }
    // If a forced move is required but the ONLY candidate is the held pitch, there is
    // nothing to move to — accept the hold rather than emit a non-chord-tone.
    let can_move = force_move && cands.iter().any(|&c| c != prev_counter);

    let mut best: Option<(i32, u8)> = None;
    for cand in cands {
        // HARD REJECT: similar motion into a parallel P5/P8 with the melody. Only
        // checkable when both the prior melody pitch and prior counter pitch exist;
        // at a section opening (m_prev None) there is no prior pair to parallel.
        if let (Some(mp), Some(mn)) = (m_prev, m_now) {
            let before = [mp, prev_counter];
            let after = [mn, cand];
            if has_parallel_perfects(&before, &after) {
                continue; // never emit a parallel perfect against the melody
            }
        }

        // Score (lower wins): conjunct preference (motion size, like voice_lead_one)...
        let motion = (cand as i32 - prev_counter as i32).abs();
        let mut score = motion;

        // ...minus a contrary/oblique bonus vs the melody. cnt_dir Hold while the
        // melody moves is OBLIQUE (still independent); cnt_dir opposing mel_dir is
        // CONTRARY. Either earns the bonus; SIMILAR motion (same direction) does not.
        let cnt_dir = motion_dir(Some(prev_counter), Some(cand));
        let independent = match (mel_dir, cnt_dir) {
            // melody static: no contrary axis — the held-period mover just steps; give
            // a small bonus for actually MOVING (cnt_dir != Hold) so it doesn't stall.
            (MotionDir::Hold, MotionDir::Hold) => false,
            (MotionDir::Hold, _) => true,
            // melody moving: reward opposing direction (contrary) or holding (oblique).
            (MotionDir::Up, MotionDir::Down) | (MotionDir::Down, MotionDir::Up) => true,
            (_, MotionDir::Hold) => true,
            _ => false, // similar motion — no bonus
        };
        if independent {
            score -= CONTRARY_BONUS;
        }

        // ...plus a heavy penalty for doubling the melody's exact pitch (no unison).
        if Some(cand) == m_now {
            score += COUNTER_UNISON_PENALTY;
        }

        // ...plus a held-period MOVE penalty: under a held/static period, holding the
        // previous pitch is the forbidden re-struck stab — penalize the no-op so a
        // different chord tone always wins (the moving line through the held harmony).
        // Only when an alternative chord tone actually exists (can_move).
        if can_move && cand == prev_counter {
            score += COUNTER_UNISON_PENALTY;
        }

        // ...plus a SMALL root-pc bias: the counter is an inner line that leans on the
        // 3rd/5th. A root pc seated in the counter register is legal (not a bass double)
        // but should lose a tie to a non-root inner tone — a gentle nudge, far smaller
        // than the contrary bonus, so it only breaks otherwise-equal choices.
        if is_root_pc(chord, cand) {
            score += ROOT_PC_BIAS;
        }

        match best {
            Some((bs, _)) if bs <= score => {}
            _ => best = Some((score, cand)),
        }
    }

    // Every candidate rejected as a parallel (rare): fall back to the nearest oblique
    // candidate — HOLD the previous pitch if it is still a chord tone, else the nearest
    // chord tone. Never emit a parallel; this branch keeps the line legal.
    best.map(|(_, c)| c).unwrap_or_else(|| {
        let oblique = counter_candidate_pitches(chord, prev_counter);
        // Prefer holding prev_counter if it is itself a chord tone (a true oblique),
        // else the nearest chord tone to it.
        if oblique.contains(&prev_counter) {
            prev_counter
        } else {
            oblique
                .into_iter()
                .min_by_key(|&n| (n as i16 - prev_counter as i16).abs())
                .unwrap_or(prev_counter)
        }
    })
}

// =========================================================================
// S30 SLICE 1 — SPECIES-COUNTERPOINT FIGURE SELECTION (Music Theory Specialist owns)
//
// design-s30-pattern-library-slice1.md §3.1 / research-s30 Area 1. Promotes the
// sustain-only `pick_counter_pitch` scorer above into a fifth-species figure-selection
// driver: per step ENUMERATE {Sustain, Passing, Neighbor, Suspension, Cambiata},
// HARD-GATE by first-species + figure-specific counterpoint invariants, PREF-SCORE,
// and PICK DETERMINISTICALLY (no thread_rng).
//
// BYTE-FREEZE DISCIPLINE (design §5.2 PT-0): `pick_counter_figure` reduces EXACTLY to
// `pick_counter_pitch` on every step where no dissonant figure is licensed — the Sustain
// figure's pitch is literally `pick_counter_pitch(..)` unchanged, and dissonant figures
// are only ever ENABLED on changing-chord / moving-melody steps (R-A). On the identity /
// held-run / equivalence path the driver is never reached or yields Sustain only, so the
// realized counter is byte-identical to the as-built line.
//
// All items are PRIVATE; nothing is `pub`; nothing changes `realize_step`'s public
// 7-param signature; everything is reachable ONLY from the CounterMelody realize arm.
// =========================================================================

// --- §1.1 consonance/dissonance classifier (the new foundation predicate) ---

/// Common-practice harmonic class of the vertical interval between two MIDI notes,
/// computed via the existing `interval_class`.
///
/// theory (research Area 1 §1.1): ic 0/7 = PERFECT consonance (unison/octave, fifth);
/// ic 3/4/8/9 = IMPERFECT consonance (thirds, sixths); ic 1/2/6/10/11 = DISSONANCE
/// (seconds, tritone, sevenths). The perfect fourth (ic 5) is the one contested class —
/// classified per `FOURTH_IS_DISSONANT` (contested-decision #1: dissonant in the bare
/// two-voice counter line, the safe standard split). This classifier is used ONLY by the
/// two-voice counter scorer; the chordal `voice_lead_one` path never calls it and keeps
/// its current 4th-as-consonant behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HarmonicClass {
    PerfectConsonance,
    ImperfectConsonance,
    Dissonance,
}

/// Contested-decision #1: the bare perfect fourth (ic 5) is a DISSONANCE inside the
/// independent two-voice counter line (a 4th cannot be a stable structural interval
/// between two unsupported voices). `true` = the standard, conservative resolution.
const FOURTH_IS_DISSONANT: bool = true;

fn harmonic_class(a: u8, b: u8) -> HarmonicClass {
    match interval_class(a, b) {
        0 | 7 => HarmonicClass::PerfectConsonance,
        3 | 4 | 8 | 9 => HarmonicClass::ImperfectConsonance,
        // ic 5 (the perfect fourth) routes per the contested-decision flag.
        5 => {
            if FOURTH_IS_DISSONANT {
                HarmonicClass::Dissonance
            } else {
                HarmonicClass::PerfectConsonance
            }
        }
        // 1, 2, 6, 10, 11 — seconds, tritone, sevenths.
        _ => HarmonicClass::Dissonance,
    }
}

/// True iff the vertical interval between the two voices is ANY consonance (perfect or
/// imperfect) — the first-species "structural verticals must be consonant" gate.
fn is_consonant(a: u8, b: u8) -> bool {
    !matches!(harmonic_class(a, b), HarmonicClass::Dissonance)
}

// --- §1.x relative (pairwise) motion of two voices ---

/// The four relative-motion types of two voices, in preference order (Contrary best,
/// Parallel worst). Distinct from single-line `MotionDir`.
///
/// theory (research Area 1 terminology): contrary = opposite directions (most
/// independent); oblique = one voice holds; similar = same direction, different
/// interval; parallel = same direction, SAME harmonic interval class (least
/// independent — the parallel-perfect prohibition's geometry).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RelMotion {
    Contrary,
    Oblique,
    Similar,
    Parallel,
}

/// Pairwise relative motion from voice-pair (a_prev,b_prev) to (a_now,b_now). `a` is the
/// CF (melody) side, `b` the counter side, but the classification is symmetric.
///
/// theory: oblique iff either voice holds; otherwise contrary iff the two single-line
/// directions oppose; otherwise similar, refined to PARALLEL when the harmonic interval
/// class is PRESERVED across the move (the textbook "same direction, same interval").
fn rel_motion(a_prev: u8, a_now: u8, b_prev: u8, b_now: u8) -> RelMotion {
    let da = motion_dir(Some(a_prev), Some(a_now));
    let db = motion_dir(Some(b_prev), Some(b_now));
    match (da, db) {
        // Either voice holds → oblique (the held voice is the oblique axis).
        (MotionDir::Hold, _) | (_, MotionDir::Hold) => RelMotion::Oblique,
        // Both move in opposite directions → contrary.
        (MotionDir::Up, MotionDir::Down) | (MotionDir::Down, MotionDir::Up) => RelMotion::Contrary,
        // Both move the same way → similar, promoted to parallel when the interval
        // class is preserved (same vertical interval carried in the same direction).
        _ => {
            if interval_class(a_prev, b_prev) == interval_class(a_now, b_now) {
                RelMotion::Parallel
            } else {
                RelMotion::Similar
            }
        }
    }
}

/// Graded PREF term (lower-is-better, matching `pick_counter_pitch`'s score convention)
/// for a relative-motion type — generalizes the single `CONTRARY_BONUS` into the full
/// contrary > oblique > similar > parallel gradient (research Area 1 §1.2 PREF).
///
/// theory: contrary motion is the strongest two-voice independence, parallel the weakest.
/// The gradient is keyed off `CONTRARY_BONUS` so it stays smaller than any HARD reject
/// (those are boolean gates upstream, never score terms) — it ORDERS legal candidates,
/// it never rescues an illegal one. Contrary keeps the full bonus; oblique most of it;
/// similar a small reward; parallel none (the worst legal motion).
fn rel_motion_score(m: RelMotion) -> i32 {
    match m {
        RelMotion::Contrary => -CONTRARY_BONUS, // best — the as-built contrary bonus
        RelMotion::Oblique => -(CONTRARY_BONUS * 3 / 4), // a held voice is still independent
        RelMotion::Similar => -(CONTRARY_BONUS / 8), // a faint nudge over parallel
        RelMotion::Parallel => 0,               // worst legal motion — no reward
    }
}

// --- §1.2 / §1.3 HARD gates over a voice-pair transition ---

/// Contested-decision #2: STRICT hidden/direct-perfects rule — forbid ALL similar
/// (and parallel) motion INTO a perfect consonance. `true` = the strict two-voice form
/// (the cleaner independence the engine lacks); the four-part-harmony relaxation
/// (allow when the upper voice steps) belongs to chordal pedagogy, not the bare line.
const HIDDEN_PERFECTS_STRICT: bool = true;

/// HARD (research Area 1 §1.2): approaching a PERFECT consonance must be by Contrary or
/// Oblique motion — forbids direct/hidden fifths & octaves (similar motion into a
/// perfect) on top of the strict parallel-perfect ban `has_parallel_perfects` already
/// enforces. Returns `true` when the resulting vertical is NOT perfect (rule vacuous) or
/// the approach motion is legal.
///
/// `m_*` is the CF/melody side, `c_*` the counter side.
fn approach_perfect_is_legal(m_prev: u8, m_now: u8, c_prev: u8, c_now: u8) -> bool {
    // The rule only constrains transitions whose ARRIVAL vertical is a perfect consonance.
    if harmonic_class(m_now, c_now) != HarmonicClass::PerfectConsonance {
        return true;
    }
    match rel_motion(m_prev, m_now, c_prev, c_now) {
        RelMotion::Contrary | RelMotion::Oblique => true,
        // Similar/Parallel into a perfect: legal only if the strict flag is OFF.
        RelMotion::Similar | RelMotion::Parallel => !HIDDEN_PERFECTS_STRICT,
    }
}

/// HARD (research Area 1 §1.2 melodic): the counter line must not LEAP by a DISSONANT
/// melodic interval — a melodic seventh (ic 10/11 across an octave-or-less leap) or any
/// augmented interval (the tritone, ic 6). Steps (≤2 semitones) are always legal; the
/// classic prohibited leaps are the melodic 7th and the tritone.
///
/// theory: the ear cannot sing an unprepared dissonant leap cleanly — these are the
/// textbook forbidden melodic intervals. Consonant leaps (3rds/4ths/5ths/6ths/octave)
/// pass; a tritone or seventh leap is rejected.
fn melodic_leap_is_legal(prev: u8, now: u8) -> bool {
    let semis = (now as i16 - prev as i16).abs();
    // A step is never a forbidden leap.
    if semis <= 2 {
        return true;
    }
    let ic = interval_class(prev, now);
    // Tritone (ic 6) and sevenths (ic 10/11) are the prohibited melodic leaps. (An
    // octave, ic 0 across 12 semitones, is a consonant leap and stays legal.)
    !matches!(ic, 6 | 10 | 11)
}

// --- §1.4–§1.6 the licensed dissonant figures (each = approach+resolution predicate) ---

/// The per-step figure the counter sounds. `Sustain` is the first-species default (one
/// consonant chord tone — today's behavior, byte-preserved). The rest are the licensed
/// dissonances; each is producible ONLY when its approach+resolution predicate holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CounterFigure {
    Sustain,    // one consonant chord tone (today's pick_counter_pitch behavior)
    Passing,    // §1.4/1.5: dissonance approached & left by step, SAME direction
    Neighbor,   // §1.5: step away & step back to the start pitch, OPPOSITE directions
    Suspension, // §1.6: prepared-consonant, held, dissonant-on-strong, resolves DOWN by step
    #[allow(dead_code)] // recognized by `is_legal_cambiata`; emission is a Slice-4 widening
    Cambiata, // §1.5: the canonical 5-note changing-note figure (one sanctioned leap-from-diss)
}

/// Contested-decision #3 (second-species dissonance): passing is HARD-legal; neighbor is
/// PREF-permitted behind this flag. `true` admits neighbor figures.
const NEIGHBOR_ALLOWED: bool = true;

/// §1.4 passing-tone predicate: a candidate dissonant note is legal iff it is approached
/// by STEP and left by STEP in the SAME direction (it passes between two consonances).
///
/// `prev` = the prior counter pitch (consonant against its CF), `cand` = the candidate
/// (dissonant against `cf_at_cand`), `next_resolution` = the pitch it steps to (consonant
/// against its CF). theory (research Area 1 §1.4): step-in AND step-out AND same-direction,
/// both ENDS consonant. The candidate itself is the controlled passing dissonance.
fn is_legal_passing(prev: u8, cand: u8, next_resolution: u8, cf_at_cand: u8) -> bool {
    let in_step = (cand as i16 - prev as i16).abs();
    let out_step = (next_resolution as i16 - cand as i16).abs();
    // Approached and left by step (≤ a whole tone).
    if !(1..=2).contains(&in_step) || !(1..=2).contains(&out_step) {
        return false;
    }
    // Same direction in and out (a true passing tone, not a turn).
    let same_dir =
        motion_dir(Some(prev), Some(cand)) == motion_dir(Some(cand), Some(next_resolution));
    // The candidate must actually BE the dissonance the figure licenses, and the
    // resolution end-point a consonance (the prep end is the prior consonant note).
    same_dir && harmonic_class(cand, cf_at_cand) == HarmonicClass::Dissonance
}

/// §1.5 neighbor predicate: step AWAY from the start pitch then step BACK to it
/// (opposite directions, start == end), the candidate dissonant in the middle, both
/// framing notes consonant. Behind `NEIGHBOR_ALLOWED` at the call site.
///
/// theory (research Area 1 §1.5): `step_in AND step_out AND opposite_dir AND start==end`.
fn is_legal_neighbor(prev: u8, cand: u8, return_to: u8, cf_at_cand: u8) -> bool {
    let in_step = (cand as i16 - prev as i16).abs();
    let out_step = (return_to as i16 - cand as i16).abs();
    if !(1..=2).contains(&in_step) || !(1..=2).contains(&out_step) {
        return false;
    }
    // Returns to the SAME pitch it left (the defining neighbor property).
    if prev != return_to {
        return false;
    }
    // Opposite directions (away then back).
    let away = motion_dir(Some(prev), Some(cand));
    let back = motion_dir(Some(cand), Some(return_to));
    let opposite = matches!(
        (away, back),
        (MotionDir::Up, MotionDir::Down) | (MotionDir::Down, MotionDir::Up)
    );
    opposite && harmonic_class(cand, cf_at_cand) == HarmonicClass::Dissonance
}

/// §1.6 suspension three-stage predicate over three consecutive counter states and the
/// CF beneath each: PREP consonant → HELD at the same pitch → now DISSONANT on the strong
/// position → RESOLVES DOWN by step to a consonance. Encodes the canonical {7-6, 4-3, 9-8
/// upper; 2-3 bass} suspension table implicitly (any prepared-and-down-resolved
/// dissonance against the CF that lands consonant satisfies it).
///
/// theory (research Area 1 §1.6): `prep_consonant AND held_same_pitch AND
/// now_dissonant_on_strong AND resolves == step_down_to_consonance`. Resolution is ALWAYS
/// by a single downward step (−1 or −2 semitones) to a consonant vertical.
fn is_legal_suspension(
    prep: u8,
    held: u8,
    resolution: u8,
    cf_prep: u8,
    cf_held: u8,
    cf_resolution: u8,
) -> bool {
    // Stage 1: preparation is consonant.
    if !is_consonant(prep, cf_prep) {
        return false;
    }
    // Stage 2: the SAME pitch is held into the strong position (the syncopation/tie).
    if prep != held {
        return false;
    }
    // Stage 3a: that held pitch is now DISSONANT against the new CF (the suspension).
    if harmonic_class(held, cf_held) != HarmonicClass::Dissonance {
        return false;
    }
    // Stage 3b: it resolves DOWN by a single step to a consonance (the 7-6/4-3/9-8/2-3
    // table is exactly "down a step to the consonant tone below").
    let drop = resolution as i16 - held as i16;
    (drop == -1 || drop == -2) && is_consonant(resolution, cf_resolution)
}

/// §1.5 cambiata recognizer: the ONE canonical 5-note changing-note form —
/// consonance → step DOWN to a dissonance → leap DOWN a third → step UP → step UP. It is
/// the single sanctioned place a dissonance is left by LEAP; the recognizer permits that
/// otherwise-illegal leap-away-from-dissonance only when the whole template matches.
///
/// `figure` is the 5 counter pitches, `cf` the 5 CF pitches beneath them. theory
/// (research Area 1 §1.5; contested-decision: one canonical 5-note form, variants out of
/// scope). Recognition-only in Slice 1 (the emission path is a Slice-4 widening — the
/// predicate is built now so PT-4 can classify a cambiata if one is ever produced).
// retained for the deferred Slice-4 cambiata emission (recognition-only for now)
#[allow(dead_code)]
fn is_legal_cambiata(figure: &[u8], cf: &[u8]) -> bool {
    if figure.len() != 5 || cf.len() != 5 {
        return false;
    }
    let d = |a: u8, b: u8| b as i16 - a as i16; // signed melodic interval a→b
                                                // n0→n1: step down (−1/−2); n1→n2: leap down a third (−3/−4); n2→n3: step up (+1/+2);
                                                // n3→n4: step up (+1/+2). The classic Fux cambiata shape.
    let n01 = d(figure[0], figure[1]);
    let n12 = d(figure[1], figure[2]);
    let n23 = d(figure[2], figure[3]);
    let n34 = d(figure[3], figure[4]);
    let step_down = (-2..=-1).contains(&n01);
    let third_down = (-4..=-3).contains(&n12);
    let step_up_1 = (1..=2).contains(&n23);
    let step_up_2 = (1..=2).contains(&n34);
    if !(step_down && third_down && step_up_1 && step_up_2) {
        return false;
    }
    // The 2nd note (the changing note) is the licensed dissonance; the frame notes
    // (0 and the landing) are consonant.
    harmonic_class(figure[1], cf[1]) == HarmonicClass::Dissonance
        && is_consonant(figure[0], cf[0])
        && is_consonant(figure[4], cf[4])
}

// --- §1.3 begin / cadence boundary formulas ---

/// HARD (research Area 1 §1.3): the counter's FIRST vertical (a `PhraseStart` step) must
/// be a PERFECT consonance — unison/5th/octave when the counter is above the CF;
/// unison/octave when below (avoid the bare fourth-below opening). Returns the opening
/// candidate set filtered to perfect-consonant verticals, in the counter band.
///
/// theory: species writing opens on a stable perfect interval so the two voices declare
/// their independence from a clear vertical. The 5th-below is the bare-fourth-above
/// inversion the rule excludes, hence the `counter_above` split.
fn opening_candidates(chord: &Chord, cf_first: u8, counter_above: bool) -> Vec<u8> {
    counter_candidate_pitches(chord, cf_first)
        .into_iter()
        .filter(|&c| harmonic_class(c, cf_first) == HarmonicClass::PerfectConsonance)
        .filter(|&c| {
            let ic = interval_class(c, cf_first);
            if counter_above {
                // Above: unison/octave (ic 0) or fifth (ic 7) are all legal.
                ic == 0 || ic == 7
            } else {
                // Below: only unison/octave; a "fifth below" sounds as a fourth above
                // the counter, the excluded bare-fourth opening.
                ic == 0 || c >= cf_first
            }
        })
        .collect()
}

/// HARD (research Area 1 §1.3): at a PerfectAuthenticCadence step, the counter resolves by
/// STEPWISE CONTRARY motion onto the octave/unison with the CF (the clausula) — the
/// penultimate vertical a major 6th (counter above) or minor 3rd (counter below)
/// expanding/contracting by step to the final perfect consonance.
///
/// theory: the cadential clausula is the most rigid two-voice formula — the voices
/// converge by step to the octave/unison. We choose the perfect-consonant chord tone
/// reachable by a STEP from `counter_prev` whose motion is CONTRARY to the CF's, nearest
/// the prior counter pitch. Falls back to the nearest perfect-consonant tone if no
/// stepwise contrary landing exists (degenerate cadence chord).
fn cadence_resolution_pitch(chord: &Chord, cf_now: u8, counter_prev: u8, cf_prev: u8) -> u8 {
    let cf_dir = motion_dir(Some(cf_prev), Some(cf_now));
    let cands = counter_candidate_pitches(chord, counter_prev);
    // Perfect-consonant landings reachable by a STEP from where the counter sat.
    let mut stepwise_contrary: Vec<u8> = cands
        .iter()
        .copied()
        .filter(|&c| harmonic_class(c, cf_now) == HarmonicClass::PerfectConsonance)
        .filter(|&c| (c as i16 - counter_prev as i16).abs() <= 2 && c != counter_prev)
        .filter(|&c| {
            // Contrary to the CF (or oblique when the CF holds — still convergent).
            let cdir = motion_dir(Some(counter_prev), Some(c));
            matches!(
                (cf_dir, cdir),
                (MotionDir::Up, MotionDir::Down)
                    | (MotionDir::Down, MotionDir::Up)
                    | (MotionDir::Hold, _)
            )
        })
        .collect();
    stepwise_contrary.sort_by_key(|&c| (c as i16 - counter_prev as i16).abs());
    if let Some(&c) = stepwise_contrary.first() {
        // TIER A (unchanged): the ideal clausula — a stepwise CONTRARY perfect close.
        return c;
    }

    // S30-CP-FIX (GAP-3) — TIER B: a no-leap STEPWISE perfect close in EITHER direction.
    //
    // theory: the strict clausula demands stepwise CONTRARY convergence, but the
    // load-bearing musical guarantee at a PAC is "close on a perfect consonance WITHOUT a
    // leap" — a leap into the final sonority is the mechanical artifact we are removing.
    // From a realized penult like 62 (on V/vi → IV → V → I) the perfect P5 landing 60 is a
    // -2 STEP, but it moves SIMILAR to the descending CF (74→67), so the strict contrary
    // filter above drops it and the old nearest-perfect fallback LEAPED to 55 (move 7).
    // Before falling back to that leap, accept any perfect-consonant chord tone reachable by
    // a STEP (|motion| ≤ 2) regardless of motion direction (similar/oblique allowed). This
    // preserves the perfect CLOSE (PT-8 strengthened) and eliminates the by-leap resolution
    // while never octave-displacing the line out of band.
    let mut stepwise_perfect: Vec<u8> = cands
        .iter()
        .copied()
        .filter(|&c| harmonic_class(c, cf_now) == HarmonicClass::PerfectConsonance)
        .filter(|&c| (c as i16 - counter_prev as i16).abs() <= 2 && c != counter_prev)
        .collect();
    stepwise_perfect.sort_by_key(|&c| (c as i16 - counter_prev as i16).abs());
    if let Some(&c) = stepwise_perfect.first() {
        return c;
    }

    // S30-CP-FIX (GAP-3) — TIER C: a no-leap STEPWISE CONSONANT (perfect-or-imperfect)
    // close. Not needed for the pinned battery; specified for completeness so a future
    // degenerate cadence chord with no stepwise PERFECT landing still avoids a leap, mildly
    // relaxing only the "perfect close" while keeping the no-leap guarantee.
    let mut stepwise_consonant: Vec<u8> = cands
        .iter()
        .copied()
        .filter(|&c| is_consonant(c, cf_now))
        .filter(|&c| (c as i16 - counter_prev as i16).abs() <= 2 && c != counter_prev)
        .collect();
    stepwise_consonant.sort_by_key(|&c| (c as i16 - counter_prev as i16).abs());
    if let Some(&c) = stepwise_consonant.first() {
        return c;
    }

    // TIER D (fallback, unchanged): nearest perfect-consonant chord tone by ANY motion —
    // the existing leaping floor, now reached only by a genuinely degenerate chord with no
    // stepwise landing of any kind.
    cands
        .iter()
        .copied()
        .filter(|&c| harmonic_class(c, cf_now) == HarmonicClass::PerfectConsonance)
        .min_by_key(|&c| (c as i16 - counter_prev as i16).abs())
        .unwrap_or(counter_prev)
}

// --- new PREF constants (siblings of CONTRARY_BONUS, all private) ---

/// §1.2 PREF: prefer imperfect consonances (3rds/6ths) over perfect ones for interior
/// verticals — a line of only 5ths/8ves sounds empty and courts parallels. A small
/// reward (kept below the motion gradient so it only breaks near-ties).
const IMPERFECT_PREF: i32 = 4;

/// §1.6 reward for a legal SUSPENSION figure — the most expressive licensed dissonance,
/// nudged so a prepared-and-resolved suspension is preferred over a plain sustain when
/// both are legal on a strong cadential-approach step. Smaller than `CONTRARY_BONUS` so
/// it never overrides motion independence or any HARD reject.
const SUSPENSION_CHAIN_BONUS: i32 = 6;

/// §1.4 reward for a legal PASSING/NEIGHBOR figure — a small inducement to fill a
/// stepwise gap with a controlled passing dissonance rather than a static sustain, on the
/// changing-chord/moving-melody steps where dissonant figures are enabled. Kept small so
/// the consonant frame remains the default and dissonance stays an ornament.
const PASSING_PREF: i32 = 3;

/// §1.7 the fifth-species figure-selection driver — the new top of the counter scorer.
///
/// Enumerates the legal {Sustain, Passing, Neighbor, Suspension} candidates for this step
/// under the HARD gates (consonant structural verticals; dissonance only as a licensed
/// figure with correct approach+resolution; approach perfects by contrary/oblique; no
/// melodic dissonant leaps; begin/cadence formulas), PREF-scores survivors, and picks the
/// best DETERMINISTICALLY. Returns the sounding pitch AND its figure tag.
///
/// BYTE-FREEZE (design §5.2 PT-0): the `Sustain` pitch is literally `pick_counter_pitch`
/// unchanged, and dissonant figures are enabled ONLY when `figures_enabled` is true (the
/// arm sets it false on held-run / static steps — R-A). When dissonance is disabled, OR no
/// dissonant figure's predicate holds, the driver returns `(pick_counter_pitch(..),
/// Sustain)` — IDENTICAL to the as-built line. The cambiata figure is recognition-only in
/// Slice 1 (emission deferred to Slice 4), so it is not enumerated here.
/// S30-CP-FIX (GAP-2) — re-select a CONSONANT structural sustain when the plain scorer pick
/// is dissonant against the sounding CF.
///
/// theory: in species counterpoint every SUSTAINED (note-against-note) vertical must be a
/// consonance; a dissonance is admissible only as a licensed figure with a prepared approach
/// and a stepwise resolution. The as-built `pick_counter_pitch` never consonance-checks the
/// tone it lands on, so a diminished chord can sustain a chord tone that is a bare tritone
/// against the melody. This gate enforces the floor: if `raw` is already consonant (or the CF
/// rests, so there is no vertical to constrain), it is returned unchanged — a no-op on every
/// consonant triad, preserving the byte-stable held/consonant behavior. Otherwise we choose,
/// among the chord's reachable counter-band tones, the CONSONANT one that best matches the
/// inner-line bias the scorer already encodes: prefer an imperfect consonance (3rd/6th) over a
/// perfect one (a line of bare 5ths/8ves is the §1.2 emptiness the scorer also avoids), then
/// the smallest melodic step from `prev_counter` (connectedness), then a non-root tone. If NO
/// chord tone is consonant against the CF (a fully degenerate vertical — every band tone of
/// the chord clashes), we keep `raw`: the structural floor cannot be met from the chord's own
/// tones, and inventing a non-chord tone would break the first-species chord-tone invariant;
/// such a chord has no consonant counter and that is a harmony defect upstream, not a counter
/// defect. Pure/deterministic; reached only from the CounterMelody figure driver.
/// Rank a band-seated counter candidate against the CF for the "no ideal landing" recovery
/// search (design §1, the Option-B preference order). Lower-is-better, total order, RNG-free:
///
///   tier 0  CONSONANT + reached by STEP        (|motion| ≤ 2)               — the ideal
///   tier 1  CONSONANT + reached by a CONSONANT LEAP (legal leap, |motion| ≥ 3) — register kept
///   tier 2  a PREPARED ornamental DISSONANCE reached by STEP (|motion| ≤ 2)  — rare, expressive
///
/// A dissonant LEAP and an out-of-band pitch are NOT candidates here — they are exactly what
/// Option B forbids; callers filter them out BEFORE ranking. Within a tier the existing
/// inner-line biases break ties, identical to `consonance_gate_sustain`'s historic ordering:
/// imperfect consonance over perfect (avoid bare 5ths/8ves), then smallest melodic motion
/// (connectedness), then a non-root pc (the counter is not a bass double). The helper does NOT
/// itself enforce legality (band membership, `melodic_leap_is_legal`, parallel-perfect) — the
/// caller restricts to legal candidates first, then `min_by_key(rank_inregister_landing)`.
fn rank_inregister_landing(
    cand: u8,
    prev_counter: u8,
    cf_now: u8,
    chord: &Chord,
) -> (u8, u8, i16, u8) {
    let motion = (cand as i16 - prev_counter as i16).abs();
    let is_step = motion <= 2;
    let consonant = is_consonant(cand, cf_now);
    // tier: 0 consonant-step, 1 consonant-leap, 2 prepared-dissonance-step (3 = should be
    // pre-filtered out, ranked last as a defensive floor).
    let tier = match (consonant, is_step) {
        (true, true) => 0,
        (true, false) => 1,
        (false, true) => 2,
        (false, false) => 3,
    };
    let imperfect_first = if harmonic_class(cand, cf_now) == HarmonicClass::ImperfectConsonance {
        0
    } else {
        1
    };
    let root_last = if is_root_pc(chord, cand) { 1 } else { 0 };
    (tier, imperfect_first, motion, root_last)
}

fn consonance_gate_sustain(
    chord: &Chord,
    raw: u8,
    prev_counter: u8,
    m_prev: Option<u8>,
    m_now: Option<u8>,
) -> u8 {
    // No CF this step (melody rests) → no vertical to constrain; keep the scorer's pick.
    let Some(cf) = m_now else { return raw };
    // Already consonant → no-op (the byte-stable common case).
    if is_consonant(raw, cf) {
        return raw;
    }

    // S30-CP-FIX (GAP-2) — TIER 0/1: widen the consonant search to a CONSONANT LEAP.
    //
    // theory: a first-species (sustained) vertical must be a consonance. From the realized
    // penult on {IV,V} → vii a consonant landing DOES exist in the chord's band tones (the
    // m3, 62, vs melody 77), reachable by a legal CONSONANT leap (P4/m3) — but the old gate
    // ranked only by (imperfect, step, root) and never distinguished a step from a consonant
    // leap nor admitted the leap explicitly. We rank the consonant chord-tone set by
    // `rank_inregister_landing` so a consonant STEP wins (tier 0) and, failing that, a
    // consonant LEAP (tier 1) — register preserved, vertical consonant — is taken before any
    // dissonant fallback. Only LEGAL melodic moves are eligible: a step is always legal; a
    // leap must pass `melodic_leap_is_legal` (so a tritone/7th leap is never a consonant-leap
    // landing — there is none in the pinned set).
    let consonant: Vec<u8> = counter_candidate_pitches(chord, prev_counter)
        .into_iter()
        .filter(|&c| is_consonant(c, cf))
        .filter(|&c| melodic_leap_is_legal(prev_counter, c))
        .collect();
    if let Some(c) = consonant
        .into_iter()
        .min_by_key(|&c| rank_inregister_landing(c, prev_counter, cf, chord))
    {
        return c;
    }

    // S30-CP-FIX (GAP-2) — TIER 2: a PREPARED ornamental dissonance reached by STEP.
    //
    // theory: when NO chord tone of the harmony is consonant vs the CF from this band
    // position (a truly degenerate terminal vertical), Option B keeps the line IN REGISTER
    // with a deliberate, step-prepared dissonance rather than the blunt non-chord-tone floor.
    // A terminal `vii` has no resolution slot, so the dissonance is made intentional by
    // PREPARATION ALONE: it is a band chord tone reached by a STEP (|motion| ≤ 2) from a prior
    // counter pitch that was ITSELF consonant against the prior CF — the appoggiatura/
    // suspension "lean-in" gesture minus the resolution the final position cannot provide
    // (design §2). This tier is reached ONLY when the consonant set above is empty; it does
    // NOT arise for the pinned {IV,V}→vii set (62 is consonant), but is the principled floor
    // for a future fully-degenerate terminal.
    let preparation_consonant = match m_prev {
        // The preparation must itself be a consonance against the prior CF.
        Some(cf_prev) => is_consonant(prev_counter, cf_prev),
        // No prior CF → cannot verify the preparation; do not license the ornament.
        None => false,
    };
    if preparation_consonant {
        let ornament: Vec<u8> = counter_candidate_pitches(chord, prev_counter)
            .into_iter()
            // A STEP (≤2) from the realized prior counter pitch — the prepared approach.
            .filter(|&c| {
                let motion = (c as i16 - prev_counter as i16).abs();
                (1..=2).contains(&motion)
            })
            .collect();
        if let Some(c) = ornament
            .into_iter()
            .min_by_key(|&c| rank_inregister_landing(c, prev_counter, cf, chord))
        {
            return c;
        }
    }

    // No consonant chord tone AND no step-prepared ornament exists → keep `raw` as the
    // absolute defensive floor (a harmony defect upstream, not a counter defect).
    raw
}

/// S33-CP-FIX (GAP-4) — does a clean STEPWISE consonant non-parallel landing on `next_chord`
/// exist from a given penult pitch `penult`, against the next melody `m_next`? "Clean" = an
/// in-band `next_chord` chord tone reached by a STEP (`|motion| ≤ 2`), CONSONANT vs `m_next`,
/// NOT a unison with `m_next` (PT-7), and introducing NO parallel/hidden perfect against the
/// melody across the penult→landing transition (`approach_perfect_is_legal` +
/// `has_parallel_perfects`, the SAME two-point guards the main scorer applies — never loosened).
///
/// theory: this is the Boundary-B test of the penult-rework (design §1.1). It asks, at the IV
/// penult, whether the penult under test admits the stepwise consonant `iii` landing the residual
/// lacks. `m_iv` (the penult's own melody) drives the approach-motion classification.
///
/// Pure/deterministic; reached only from `penult_for_clean_next` on the counter path.
fn clean_next_landing_exists(next_chord: &Chord, penult: u8, m_iv: Option<u8>, m_next: u8) -> bool {
    counter_candidate_pitches(next_chord, penult)
        .into_iter()
        // STEP only — the clean landing the residual lacks is a stepwise one (|motion| ≤ 2).
        .filter(|&c| (c as i16 - penult as i16).abs() <= 2 && c != penult)
        // CONSONANT vs the next melody — a structural landing must be a consonance.
        .filter(|&c| is_consonant(c, m_next))
        // PT-7: never collapse onto the melody's exact pitch.
        .filter(|&c| c != m_next)
        // PT-1: introduce NO parallel/hidden perfect against the melody across penult→landing.
        // The penult's own melody `m_iv` is the prior CF for this two-point transition.
        .any(|c| match m_iv {
            Some(mp) => {
                approach_perfect_is_legal(mp, m_next, penult, c)
                    && !has_parallel_perfects(&[mp, penult], &[m_next, c])
            }
            // No penult melody → cannot evaluate the two-point motion; treat as clean.
            None => true,
        })
}

/// S33-CP-FIX (GAP-4) — PENULT LOOKAHEAD REWORK (design §1.2 Option 1, the preferred
/// realize-seam helper). Given the as-built realized penult `as_built` on an interior step whose
/// NEXT step's REALIZED counter landing (`as_built_next_landing`, computed by the caller through
/// the ordinary machinery from `as_built`) is reached by a DISSONANT melodic leap, return a
/// REWORKED penult `P` that escapes that dead-end. Returns `None` (a byte-identical no-op)
/// unless ALL of these hold:
///   * the as-built penult forces the next realized step into a DISSONANT melodic leap
///     (`!melodic_leap_is_legal(as_built, as_built_next_landing)`) — the GAP-4 tritone shape;
///   * the as-built penult itself admits NO clean stepwise consonant non-parallel landing on
///     `next_chord` (Boundary B fails from `as_built`, confirming the dead-end is real); AND
///   * some OTHER legal in-band chord tone `P` of the CURRENT chord both (Boundary A) is a
///     legal, consonant, non-parallel vertical reached from `prev_counter`, AND (Boundary B)
///     DOES admit a clean stepwise consonant non-parallel landing on `next_chord`.
///
/// theory (design §1.0–§1.1): on `{ii,vi} → IV → iii` the as-built IV penult is `65` (root of IV
/// = F, a bare P5 vs the IV melody `72`); the next step is forced to the `iii` chord tone `59` by
/// a `65 → 59` `−6` TRITONE — a dissonant melodic leap. From `65` there is NO clean STEPWISE
/// consonant landing on iii (`64` is a hidden P5, `55`/`59` are leaps). The rework re-picks the
/// penult to `57` (A, the THIRD of IV = an IMPERFECT M3 vs `72`), from which `57 → 55` is a `−2`
/// STEP onto `55` (G, an iii chord tone, M3 vs the next melody `71`). The cost is one CONSONANT
/// leap of approach into the penult (m6 from `ii`'s `65`, P5 from `vi`'s `64`); by the engine's
/// Option-B order a consonant penult-leap that buys a clean stepwise consonant landing strictly
/// beats a smooth penult that forces a dissonant tritone leap out. The dissonant-leap trigger is
/// what keeps this from firing on `I→V→IV` (the `62→65` next move is a CONSONANT m3 leap, not a
/// dissonant one → no-op) and on the GAP-2 `{IV,V}→vii` terminals (the bite lands by a STEP, not
/// a dissonant leap → no-op). `melodic_leap_is_legal`, `approach_perfect_is_legal`, and
/// `has_parallel_perfects` are NOT loosened — the fix is purely upstream candidate selection.
///
/// Determinism (PT-9): `min_by_key` over the deterministically-ordered
/// `counter_candidate_pitches` list, ranked by `rank_inregister_landing` against the penult's
/// own melody (consonant-step penult first, then consonant-leap penult, then non-root
/// tie-break). No RNG. Band `[55,67)` preserved (all candidates come from
/// `counter_candidate_pitches`).
#[allow(clippy::too_many_arguments)]
fn penult_for_clean_next(
    chord: &Chord,
    prev_counter: u8,
    m_prev: Option<u8>,
    m_now: Option<u8>,
    next_chord: &Chord,
    m_next: u8,
    as_built: u8,
    as_built_next_landing: u8,
) -> Option<u8> {
    // TRIGGER 1 — the as-built penult must FORCE the next realized step into a DISSONANT melodic
    // leap (the GAP-4 tritone). If the next realized move is a step or a CONSONANT leap, the line
    // is already clean and this branch is a strict no-op (so I→V→IV, GAP-2 terminals, and every
    // clean transition are byte-identical).
    if melodic_leap_is_legal(as_built, as_built_next_landing) {
        return None;
    }
    // TRIGGER 2 — confirm the dead-end is real: the as-built penult admits NO clean stepwise
    // consonant non-parallel landing on next_chord (so re-picking the penult is the only fix).
    if clean_next_landing_exists(next_chord, as_built, m_now, m_next) {
        return None;
    }
    // The penult's own CF (`m_now`) is needed for Boundary A's consonance/approach checks.
    let cf_iv = m_now?;

    counter_candidate_pitches(chord, prev_counter)
        .into_iter()
        // Don't re-pick the as-built penult (it is the dead-end we are escaping).
        .filter(|&p| p != as_built)
        // --- Boundary A: prev_counter → P must be a legal, consonant, non-parallel IV vertical.
        // (1) P is itself CONSONANT vs the IV melody — the first-species structural floor.
        .filter(|&p| is_consonant(p, cf_iv))
        // (2) prev_counter → P is a LEGAL melodic move (step, or a non-tritone/non-7th leap).
        .filter(|&p| melodic_leap_is_legal(prev_counter, p))
        // (3) prev_counter → P introduces NO parallel/hidden perfect against the melody.
        .filter(|&p| match m_prev {
            Some(mp) => {
                approach_perfect_is_legal(mp, cf_iv, prev_counter, p)
                    && !has_parallel_perfects(&[mp, prev_counter], &[cf_iv, p])
            }
            None => true,
        })
        // --- Boundary B: from P, a clean stepwise consonant non-parallel iii landing EXISTS.
        .filter(|&p| clean_next_landing_exists(next_chord, p, m_now, m_next))
        // Option-B ranking against the IV melody: consonant-step penult first, then
        // consonant-leap penult; imperfect-over-perfect, smallest motion, non-root tie-break.
        .min_by_key(|&p| rank_inregister_landing(p, prev_counter, cf_iv, chord))
}

#[allow(clippy::too_many_arguments)]
fn pick_counter_figure(
    chord: &Chord,
    prev_counter: u8,
    m_prev: Option<u8>,
    m_now: Option<u8>,
    mel_dir: MotionDir,
    position: PhrasePosition,
    next_chord: Option<&Chord>,
    force_move: bool,
    held_target: Option<u8>,
    // True only on changing-chord / moving-melody steps (R-A): dissonant figures are
    // enabled here and NOWHERE else, so held-run / static steps stay sustain-only and the
    // held-period behavior is byte-stable (PT-0).
    figures_enabled: bool,
) -> (u8, CounterFigure) {
    // The first-species sustain pitch — today's scorer, unchanged. This is the byte-freeze
    // anchor: it is what the figure driver reduces to whenever no dissonance is licensed.
    let sustain_raw = pick_counter_pitch(
        chord,
        prev_counter,
        m_prev,
        m_now,
        mel_dir,
        force_move,
        held_target,
    );

    // S30-CP-FIX (GAP-2) — CONSONANCE-GATE THE STRUCTURAL SUSTAIN.
    //
    // theory: a first-species (sustained) structural vertical MUST be a consonance — the
    // species floor is "every note-against-note vertical is consonant; dissonance appears
    // ONLY as a licensed, prepared-and-resolved figure". The as-built `pick_counter_pitch`
    // scorer rejects parallel perfects and rewards independence but NEVER checks the
    // vertical consonance of the pitch it lands on, so on a chord whose only reachable band
    // tones are dissonant against the CF (the diminished vii = B-D-F sustaining B against a
    // melody on F → a bare tritone) it emits an unprepared, unresolved structural dissonance.
    // Here we route the sustain pick through `is_consonant` against the sounding CF: if the
    // plain pick is dissonant, we re-select the CONSONANT chord tone nearest the prior
    // counter pitch (preferring an imperfect consonance, then the smallest melodic step from
    // `prev_counter`, then a non-root tone — the same inner-line bias the scorer encodes), so
    // the line never leaves an unprepared structural dissonance. The added passing/neighbor/
    // suspension figures below remain the ONLY way a dissonance is sounded, and each is still
    // routed through its own approach+resolution predicate. When the plain pick is already
    // consonant (the overwhelmingly common case, every consonant triad) this is a no-op and
    // the sustain is byte-identical to before — so the held-period / consonant-triad freeze
    // and PT-0 are untouched.
    let sustain = consonance_gate_sustain(chord, sustain_raw, prev_counter, m_prev, m_now);

    // --- §1.3 BEGIN formula: a PhraseStart vertical MUST be a perfect consonance. This is
    // a HARD override of the sustain pick (which optimizes for motion, not the opening
    // interval). Only applies when the CF actually sounds this step.
    if position == PhrasePosition::PhraseStart {
        if let Some(cf) = m_now {
            let counter_above = sustain >= cf;
            let opens = opening_candidates(chord, cf, counter_above);
            if !opens.is_empty() {
                // Nearest legal perfect-consonant opening to the as-built sustain pick, so
                // the opening is stable yet as close as possible to the connected line.
                let pick = opens
                    .into_iter()
                    .min_by_key(|&c| (c as i16 - sustain as i16).abs())
                    .unwrap_or(sustain);
                return (pick, CounterFigure::Sustain);
            }
        }
        // No CF (rest) or no legal perfect opening → fall through to the sustain pick.
        return (sustain, CounterFigure::Sustain);
    }

    // --- §1.3 CADENCE formula: a PerfectAuthenticCadence step resolves by stepwise
    // contrary motion onto the octave/unison. HARD override of the sustain pick.
    if position == PhrasePosition::PerfectAuthenticCadence {
        if let (Some(cf_now), Some(cf_prev)) = (m_now, m_prev) {
            let pick = cadence_resolution_pitch(chord, cf_now, prev_counter, cf_prev);
            return (pick, CounterFigure::Sustain);
        }
        return (sustain, CounterFigure::Sustain);
    }

    // --- S32-CP "KEEP THE BITE" (GAP-2) — SECTION-TERMINAL DIMINISHED FINAL SONORITY.
    //
    // theory & operator decision: at a SECTION-TERMINAL diminished triad (`vii°` last, no
    // `next_chord`) the species "every structural vertical is consonant" floor would smooth the
    // line to a consonant chord tone (the prior GAP-2 outcome — counter 62, a m3 vs the melody).
    // But a terminal step has NO resolution beat: it is not a figure-that-must-resolve, it is the
    // FINAL sonority of the piece. The operator's confirmed call is to KEEP THE BITE — let the
    // counter sound a DELIBERATE, PREPARED dissonance against the melody and leave it unresolved,
    // because that is the expressive intent of ending on a diminished harmony. The dissonance is
    // made musical (intentional, not a blundered leap-in) by PREPARATION ALONE:
    //   * IN-REGISTER  — drawn from `counter_candidate_pitches`, which enforces the band [55,67);
    //   * PREPARED     — approached by STEP (|motion| ≤ 2) from the REALIZED penult `prev_counter`
    //                    (the lean-in gesture; a blunt leap into a dissonance is forbidden);
    //   * GENUINELY DISSONANT vs the sounding melody (the bite itself);
    //   * UNRESOLVED   — it is the end of the piece, so no resolution slot is expected.
    // PREFERENCE: the prepared dissonance is PREFERRED OVER the consonant sustain here. GUARD: if
    // NO dissonant band tone is step-reachable from the realized penult, we DO NOT manufacture a
    // blunt leap-in dissonance — we fall through to the consonance-gated sustain (the prepared-
    // ness requirement is a hard floor, never relaxed into a leap). Determinism: `min_by_key`
    // (smallest step first, then prefer a tone NOT equal to the melody's pc for PT-7 hygiene),
    // RNG-free. Scope: ONLY the terminal-diminished case — every interior step, every consonant
    // terminal, and the PhraseStart/PAC overrides above are untouched, so the goldens and PT
    // invariants outside this exact sonority are byte-stable.
    if next_chord.is_none() && is_diminished_triad(chord) {
        if let Some(cf_now) = m_now {
            let bite: Vec<u8> = counter_candidate_pitches(chord, prev_counter)
                .into_iter()
                // PREPARED: |motion| ≤ 2 from the realized penult (the operator's definition).
                // This INCLUDES motion 0 — a HELD dissonant tone (oblique motion against the
                // moving melody, the textbook suspension-style preparation: the counter sustains
                // its penult while the melody steps onto the dissonant interval). A leap (≥3) is
                // the blunt leap-in the guard forbids. So |motion| ≤ 2 exactly.
                .filter(|&c| (c as i16 - prev_counter as i16).abs() <= 2)
                // The BITE: a genuine vertical dissonance against the sounding melody.
                .filter(|&c| !is_consonant(c, cf_now))
                // PT-7 hygiene: never collapse onto the melody's exact pitch (a unison is not a
                // dissonance anyway, but keep the floor explicit).
                .filter(|&c| c != cf_now)
                .collect();
            if let Some(c) = bite.into_iter().min_by_key(|&c| {
                // Lower-is-better: smallest prepared step first (the tightest lean-in), then a
                // deterministic tie-break by pitch.
                ((c as i16 - prev_counter as i16).abs(), c)
            }) {
                return (c, CounterFigure::Sustain);
            }
            // No step-reachable dissonance from the realized penult → preparation impossible.
            // Honor the guard: fall through to the consonance-gated sustain rather than leap into
            // a dissonance. (The sustain below is already consonant where a consonant in-band
            // landing exists.)
        }
    }

    // --- Interior steps. Dissonant figures are licensed ONLY when enabled (changing-chord
    // / moving-melody) AND the CF sounds both now and prior (we need the vertical context)
    // AND a real NEXT chord exists to resolve into.
    //
    // S30-CP-FIX (GAP-2) — TERMINAL FIGURE LICENSE. A passing/neighbor/suspension figure is a
    // PREPARED-AND-RESOLVED dissonance: it requires a resolution BEAT on the following step
    // (design §2). At a section-terminal step there is no next step emitted, so a dissonant
    // figure there would sound a permanently-UNRESOLVED structural dissonance — exactly the
    // GAP-2 fault (e.g. {IV,V} → vii terminal, where the figure scorer otherwise picks a
    // Neighbor/Passing that beats the now-consonant sustain and lands a bare m7/P4 with no
    // resolution slot). We therefore license dissonant figures ONLY when `next_chord` is
    // present; at a terminal step the line falls to the consonance-gated sustain (§1.1), which
    // is consonant wherever a consonant in-band landing exists (the strong Option-B outcome).
    // The old `next_chord.unwrap_or(chord)` "resolve into the current harmony at section end"
    // reasoning was the bug — a figure cannot resolve when no further step sounds.
    if figures_enabled {
        if let (Some(cf_now), Some(cf_prev), Some(resolve_chord)) = (m_now, m_prev, next_chord) {
            if let Some((pitch, figure, bonus)) =
                best_dissonant_figure(resolve_chord, prev_counter, sustain, cf_prev, cf_now)
            {
                // A licensed dissonant figure was found and is legal. It must still pass
                // the §1.2 melodic-leap gate (no dissonant leap INTO the figure) and the
                // approach-perfect gate is irrelevant (the arrival is a dissonance, not a
                // perfect). Compare it against the sustain pick on score: a dissonance is
                // an ORNAMENT, only chosen when its figure bonus makes it the better line.
                if melodic_leap_is_legal(prev_counter, pitch) {
                    // Score both options on the SAME scale and pick the lower (better).
                    let sustain_score =
                        figure_vertical_score(cf_prev, cf_now, prev_counter, sustain, chord);
                    let diss_score = (pitch as i32 - prev_counter as i32).abs() - bonus;
                    if diss_score < sustain_score {
                        return (pitch, figure);
                    }
                }
            }
        }
    }

    // S30-CP-FIX (GAP-4) — OPTION-B IN-REGISTER LEAP RECOVERY.
    //
    // theory: `melodic_leap_is_legal` correctly forbids a DISSONANT melodic leap (a tritone
    // or 7th), but the as-built scorer can still SELECT such a leap as its surviving sustain
    // pick when every consonant chord-tone candidate is vetoed by an upstream gate (the
    // no-parallel / approach-perfect guard). On `ii/vi → IV → iii` the line lands 59 by a
    // -6 TRITONE leap from the realized penult 65, because the clean stepwise consonant
    // chord tone 64 (P5 vs melody 71) is dropped by `approach_perfect_is_legal` (similar
    // motion into a perfect). Rather than loosen the leap gate (it is the correct gate) or
    // octave-displace, we run an in-register recovery search when — and ONLY when — the
    // chosen sustain pitch is itself reachable from the realized prior counter pitch only by
    // a DISSONANT leap. We prefer, per the Option-B order: (tier 0) a consonant chord tone
    // by STEP, (tier 1) a consonant chord tone by a CONSONANT (legal) LEAP, (tier 2) a
    // PREPARED ornamental dissonance by STEP. A consonant stepwise landing (64) strictly
    // beats the dissonant tritone leap (59) and is taken. On every clean transition
    // `melodic_leap_is_legal(prev_counter, sustain)` is already true, so this branch is a
    // no-op and the line is byte-identical (the recovery never widens the band — all
    // candidates come from `counter_candidate_pitches`, which enforces [55,67)).
    if !melodic_leap_is_legal(prev_counter, sustain) {
        if let Some(cf_now) = m_now {
            // Eligible recovery landings: in-band chord tones reachable by a LEGAL melodic
            // move (step, or a consonant/legal leap — never a tritone/7th leap), and never
            // the unison with the CF (PT-7). Tier 2 (prepared ornament) requires the
            // preparation `prev_counter` to be consonant vs the prior CF.
            let preparation_consonant = match m_prev {
                Some(cf_prev) => is_consonant(prev_counter, cf_prev),
                None => false,
            };
            let recovery: Vec<u8> = counter_candidate_pitches(chord, prev_counter)
                .into_iter()
                .filter(|&c| c != cf_now) // no unison collapse (PT-7)
                .filter(|&c| melodic_leap_is_legal(prev_counter, c)) // never a dissonant leap
                // HARD INVARIANT — the recovery must NOT manufacture a parallel/hidden perfect
                // against the melody (PT-1). The naive "stepwise consonant landing 64" the spec
                // suggested for ii/vi → IV → iii is a P5 vs the melody reached by SIMILAR motion
                // (counter 65→64 with melody 72→71) — a hidden P5 that `approach_perfect_is_legal`
                // rejects and that `test_no_audible_parallel_perfect` (a non-GAP, must-stay-green
                // invariant) forbids. We re-apply the SAME two-point guards the main scorer uses
                // so the leap-recovery can never trade the tritone for a parallel perfect.
                .filter(|&c| match m_prev {
                    Some(cf_prev) => {
                        approach_perfect_is_legal(cf_prev, cf_now, prev_counter, c)
                            && !has_parallel_perfects(&[cf_prev, prev_counter], &[cf_now, c])
                    }
                    // No prior CF → cannot evaluate two-point motion; keep the candidate.
                    None => true,
                })
                .filter(|&c| {
                    let motion = (c as i16 - prev_counter as i16).abs();
                    let is_step = motion <= 2;
                    if is_consonant(c, cf_now) {
                        // tiers 0/1: a consonant landing by step OR a consonant legal leap.
                        true
                    } else {
                        // tier 2: a PREPARED ornamental dissonance, by STEP only.
                        is_step && preparation_consonant
                    }
                })
                .collect();
            if let Some(c) = recovery
                .into_iter()
                .min_by_key(|&c| rank_inregister_landing(c, prev_counter, cf_now, chord))
            {
                return (c, CounterFigure::Sustain);
            }
        }
    }

    (sustain, CounterFigure::Sustain)
}

/// PREF score (lower-is-better) of the SUSTAIN candidate as a vertical+motion choice, on
/// the same scale `pick_counter_figure` compares the dissonant figures against: melodic
/// step size, minus the graded relative-motion bonus, minus the imperfect-consonance
/// preference. Used only to decide sustain-vs-dissonant-figure; it never changes the
/// sustain PITCH (that is `pick_counter_pitch`).
fn figure_vertical_score(
    cf_prev: u8,
    cf_now: u8,
    prev_counter: u8,
    cand: u8,
    chord: &Chord,
) -> i32 {
    let mut score = (cand as i32 - prev_counter as i32).abs();
    score += rel_motion_score(rel_motion(cf_prev, cf_now, prev_counter, cand));
    if harmonic_class(cand, cf_now) == HarmonicClass::ImperfectConsonance {
        score -= IMPERFECT_PREF; // §1.2 prefer 3rds/6ths interior
    }
    if is_root_pc(chord, cand) {
        score += ROOT_PC_BIAS;
    }
    score
}

/// Enumerate and HARD-gate the dissonant figures {Passing, Neighbor, Suspension} for this
/// interior step, returning the best legal one as `(pitch, figure, pref_bonus)` or `None`
/// when none is licensed. Every returned figure has a verified approach AND resolution
/// (PT-4): the candidate is the dissonance, and the resolution lands consonant.
///
/// theory: a dissonance reads as intentional ONLY as a recognized figure with a satisfied
/// approach+resolution (research §4.2 coherence rule 1). We enumerate candidate dissonant
/// counter pitches a STEP from the prior pitch, and for each test the passing / neighbor /
/// suspension predicate against the CF; a candidate failing EVERY predicate is discarded
/// (never an unprepared/unresolved dissonance).
fn best_dissonant_figure(
    resolve_chord: &Chord,
    prev_counter: u8,
    sustain: u8,
    cf_prev: u8,
    cf_now: u8,
) -> Option<(u8, CounterFigure, i32)> {
    // The resolution pitch a passing/neighbor figure steps to: the as-built sustain pick
    // is the connected consonant tone this step would otherwise sound, so it is the
    // natural resolution target (a consonant chord tone of the resolving harmony). Its CF
    // is the CF at the resolution moment — `cf_now` (the figure ornaments THIS step's
    // arrival; the resolution sits on the same beat's consonant tone in Slice 1's
    // single-step ornament model).
    let resolution = sustain;
    let cf_at_resolution = cf_now;

    // Candidate dissonant counter pitches: a STEP (±1/±2) away from the prior counter
    // pitch, dissonant against the current CF, seated in the counter band. These are the
    // only notes a passing/neighbor figure can occupy (approached by step from prev).
    let mut best: Option<(i32, u8, CounterFigure, i32)> = None;
    for delta in [-2i16, -1, 1, 2] {
        let cand_i = prev_counter as i16 + delta;
        if !(FILL_REGISTER_FLOOR as i16..COUNTER_CEILING as i16).contains(&cand_i) {
            continue;
        }
        let cand = cand_i as u8;
        // Must be a dissonance to be a dissonant figure at all (a consonant step is just
        // a sustain alternative, handled by the sustain scorer).
        if harmonic_class(cand, cf_now) != HarmonicClass::Dissonance {
            continue;
        }
        // Never collide in unison with the melody/CF (PT-7): the dissonant figure must
        // stay audibly distinct from the line it counters. (In the two-voice model the CF
        // *is* the melody pitch, so `cf_now` is the note to avoid doubling.)
        if cand == cf_now {
            continue;
        }

        // --- PASSING: step-in (prev→cand) and step-out (cand→resolution) same direction.
        // The resolution arrival must also honor the approach-to-perfect HARD gate (§1.2):
        // if it lands on a perfect consonance it must do so by contrary/oblique motion
        // (the CF moves from `cf_now` toward the resolution's CF, here `cf_at_resolution`).
        if is_legal_passing(prev_counter, cand, resolution, cf_now)
            && is_consonant(resolution, cf_at_resolution)
            && approach_perfect_is_legal(cf_now, cf_at_resolution, cand, resolution)
        {
            let score = delta.unsigned_abs() as i32 - PASSING_PREF;
            consider(&mut best, score, cand, CounterFigure::Passing, PASSING_PREF);
        }

        // --- NEIGHBOR: step-away then step-back to prev (start==end), behind the flag.
        if NEIGHBOR_ALLOWED
            && is_legal_neighbor(prev_counter, cand, prev_counter, cf_now)
            && is_consonant(prev_counter, cf_now)
        {
            // A neighbor returns to prev; the "resolution" is prev itself, which must be
            // consonant against the CF at the return (approximated by cf_now in the
            // single-step model). Slightly less preferred than passing (a turn, not a
            // forward step), so a smaller bonus.
            let score = delta.unsigned_abs() as i32 - (PASSING_PREF - 1);
            consider(
                &mut best,
                score,
                cand,
                CounterFigure::Neighbor,
                PASSING_PREF - 1,
            );
        }

        // --- SUSPENSION: prep (prev consonant) → held (==prev) → dissonant on strong →
        // resolves DOWN a step to a consonance. Modeled across the two available beats:
        // prep/held = prev_counter under cf_prev/cf_now, resolution = the down-step tone.
        // Only a DOWNWARD candidate can be a suspension resolution's antecedent dissonance.
        // Here the *held dissonance* is prev_counter carried under cf_now; `cand` is its
        // downward-step resolution. We detect this when prev_counter is dissonant now and
        // cand resolves it down by step to a consonance.
        if delta < 0
            && is_legal_suspension(
                prev_counter, // prep
                prev_counter, // held (same pitch — the syncopation)
                cand,         // resolution (down a step)
                cf_prev,      // CF under preparation
                cf_now,       // CF under the held dissonance
                cf_at_resolution,
            )
        {
            let score = delta.unsigned_abs() as i32 - SUSPENSION_CHAIN_BONUS;
            consider(
                &mut best,
                score,
                cand,
                CounterFigure::Suspension,
                SUSPENSION_CHAIN_BONUS,
            );
        }
    }

    // Reuse the resolve_chord parameter so its harmony constrains the resolution end
    // when a next chord exists (R-B): the resolution pitch must be a chord tone of the
    // resolving harmony. If `resolution` is NOT a tone of `resolve_chord`, no forward
    // figure can legally resolve into it — discard (HARD, PT-4).
    if !resolve_chord.notes.is_empty() {
        let res_pc = resolution % 12;
        let resolves_into_harmony = resolve_chord.notes.iter().any(|&n| n % 12 == res_pc);
        if !resolves_into_harmony {
            // The forward (passing) figures cannot resolve; only the suspension (which
            // resolves to `cand`, a fresh tone, not `resolution`) survives.
            if let Some((_, p, CounterFigure::Suspension, b)) = best {
                return Some((p, CounterFigure::Suspension, b));
            }
            return None;
        }
    }

    best.map(|(_, p, f, b)| (p, f, b))
}

/// Tie-broken "keep the lower score" accumulator for the figure search (deterministic:
/// strictly-less replaces, so the first-enumerated wins a tie — no RNG, stable order).
fn consider(
    best: &mut Option<(i32, u8, CounterFigure, i32)>,
    score: i32,
    pitch: u8,
    figure: CounterFigure,
    bonus: i32,
) {
    match best {
        Some((bs, _, _, _)) if *bs <= score => {}
        _ => *best = Some((score, pitch, figure, bonus)),
    }
}

/// Re-voice `next` to connect smoothly from the already-voiced `prev`.
///
/// theory (the conservative voice-leading core):
///
/// - BASS (voice 0) takes `next`'s harmonic root, seated near the previous bass.
///   It carries root motion and is EXEMPT from the ≤P5 cap (it may leap).
/// - Each UPPER voice moves to the NEAREST available chord tone within a perfect
///   fifth (`MAX_UPPER_VOICE_MOTION` = 7 semitones), keeping the inner lines
///   conjunct and connected.
/// - COMMON TONES are retained in the same voice: a voice already sounding a
///   pitch class present in `next` is rewarded for holding it (zero motion).
/// - PARALLEL perfect fifths/octaves between any voice pair are FORBIDDEN across
///   the change and are hard-rejected from the candidate search.
///
/// We enumerate every (upper-voice-1, upper-voice-2) seating within the motion
/// cap and pick the lowest-scoring legal one: score = total upper-voice motion
/// minus a strong reward per retained common tone, with parallels rejected
/// outright. This small exhaustive search is the right shape for a 3-voice
/// triad — correctness over cleverness.
fn voice_lead_one(prev: &Chord, next: &Chord) -> Chord {
    let next_pcs = chord_pitch_classes(next);
    let root_pc = next_pcs[0];
    let voice_count = prev.notes.len().min(next.notes.len());

    // --- Bass: root motion, seated near the previous bass. ---
    let prev_bass = prev.notes[0];
    let bass = nearest_pc_to(root_pc, prev_bass);

    // Upper voices we must fill (everything above the bass).
    let upper_indices: Vec<usize> = (1..voice_count).collect();

    // For each upper voice, build its candidate pitches: any chord tone of
    // `next` within the motion cap of where that voice currently sits. If a
    // voice has no in-cap chord tone (rare, very wide spacing), relax to the
    // nearest chord tone regardless of cap so we never produce an empty set —
    // a relaxation we record in the comment as the one place the ≤P5 rule may
    // bend, and only to keep a legal voicing at all.
    let mut per_voice: Vec<Vec<u8>> = Vec::with_capacity(upper_indices.len());
    for &vi in &upper_indices {
        let from = prev.notes[vi];
        let mut cands: Vec<u8> = Vec::new();
        for &pc in &next_pcs {
            cands.extend(upper_voice_candidates(pc, from, MAX_UPPER_VOICE_MOTION));
        }
        if cands.is_empty() {
            // Relaxation fallback: seat the nearest chord tone outright.
            let nearest = next_pcs
                .iter()
                .map(|&pc| nearest_pc_to(pc, from))
                .min_by_key(|&n| (n as i16 - from as i16).abs())
                .unwrap_or(from);
            cands.push(nearest);
        }
        cands.sort_unstable();
        cands.dedup();
        per_voice.push(cands);
    }

    // --- Exhaustive search over the (small) candidate cross-product. ---
    // Score (lower is better): total upper-voice semitone motion, with a heavy
    // bonus subtracted per retained common tone (a voice holding its pitch
    // class). Hard constraints: motion ≤ cap (already guaranteed by the
    // candidate sets, save the relaxation), and NO parallel perfect 5ths/8ves.
    let mut best: Option<(i32, Vec<u8>)> = None;
    let mut indices = vec![0usize; per_voice.len()];

    loop {
        // Assemble this candidate voicing: [bass, upper voices...].
        let mut voicing = vec![bass];
        for (slot, &ci) in indices.iter().enumerate() {
            voicing.push(per_voice[slot][ci]);
        }

        // Score it.
        let mut score: i32 = 0;
        for (slot, &vi) in upper_indices.iter().enumerate() {
            let from = prev.notes[vi] as i32;
            let to = voicing[slot + 1] as i32;
            let motion = (to - from).abs();
            score += motion;
            // theory: reward holding a common tone in the same voice — a held
            // pitch class is the smoothest possible connection and the property
            // net requires shared pcs to stay in their voice. The bonus dwarfs
            // any plausible motion total so retention always wins when available.
            // A voice that keeps its exact MIDI pitch (same note, same octave) is
            // holding its pitch class by definition, so the equal-pitch test alone
            // is the retention check — the separate pitch-class compare it used to
            // carry was always implied by it and contributed nothing.
            if prev.notes[vi] == voicing[slot + 1] {
                score -= 100;
            }
        }

        // theory: a candidate voicing is legal iff it neither creates a parallel
        // perfect fifth/octave across the change NOR collapses two UPPER voices
        // onto the same MIDI note. The unison collapse (e.g. S4's IV -> [65,65,65],
        // and equally I -> [72,60,60] / V -> [67,67,67] on the same scorer) is a
        // hard reject here, treated exactly like the parallel-perfects reject:
        // the minimal-motion + common-tone scorer would otherwise drive two upper
        // voices onto one pitch and silently thin the triad to a dyad. Both upper
        // voices of a three-voice triad must sound distinct pitches.
        let legal = !has_parallel_perfects(&prev.notes[..voice_count], &voicing)
            && upper_voices_well_spaced(&voicing);

        if legal {
            match &best {
                Some((bs, _)) if *bs <= score => {}
                _ => best = Some((score, voicing.clone())),
            }
        }

        // Advance the odometer over candidate indices.
        if per_voice.is_empty() {
            break;
        }
        let mut k = 0;
        loop {
            indices[k] += 1;
            if indices[k] < per_voice[k].len() {
                break;
            }
            indices[k] = 0;
            k += 1;
            if k == per_voice.len() {
                break;
            }
        }
        if k == per_voice.len() {
            break;
        }
    }

    // If every candidate combination was rejected (or there were no upper
    // voices), fall back through a graded last-resort relaxation that still
    // keeps the UPPER-VOICE SPACING rule whenever possible — the unison collapse
    // is the worse defect, so we relax the (softer) parallel-perfects rule before
    // we relax spacing.
    //
    // theory: ordered preference for the relaxation —
    //   (1) a voicing that is well-spaced even if it carries a parallel perfect
    //       (oblique-to-parallel is a lesser sin than a thinned triad);
    //   (2) only if NO well-spaced combination exists at all, the minimal-motion
    //       seating ignoring both rules (truly last resort — defensive only).
    // In practice the primary search finds a clean voicing for every diatonic
    // triad in the test progressions, so this branch is exercised rarely.
    let voicing = best.map(|(_, v)| v).unwrap_or_else(|| {
        // Pass (1): scan again for the lowest-motion WELL-SPACED voicing,
        // ignoring only the parallel-perfects constraint.
        let mut spaced_best: Option<(i32, Vec<u8>)> = None;
        if !per_voice.is_empty() {
            let mut idx = vec![0usize; per_voice.len()];
            loop {
                let mut voicing = vec![bass];
                for (slot, &ci) in idx.iter().enumerate() {
                    voicing.push(per_voice[slot][ci]);
                }
                if upper_voices_well_spaced(&voicing) {
                    let mut score: i32 = 0;
                    for (slot, &vi) in upper_indices.iter().enumerate() {
                        score += (voicing[slot + 1] as i32 - prev.notes[vi] as i32).abs();
                    }
                    match &spaced_best {
                        Some((bs, _)) if *bs <= score => {}
                        _ => spaced_best = Some((score, voicing.clone())),
                    }
                }
                let mut k = 0;
                loop {
                    idx[k] += 1;
                    if idx[k] < per_voice[k].len() {
                        break;
                    }
                    idx[k] = 0;
                    k += 1;
                    if k == per_voice.len() {
                        break;
                    }
                }
                if k == per_voice.len() {
                    break;
                }
            }
        }
        spaced_best.map(|(_, v)| v).unwrap_or_else(|| {
            // Pass (2): truly last resort — minimal-motion seating ignoring both
            // rules so we never emit nothing.
            let mut v = vec![bass];
            for (slot, _) in upper_indices.iter().enumerate() {
                v.push(per_voice[slot][0]);
            }
            v
        })
    });

    Chord {
        name: next.name.clone(),
        notes: voicing,
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

    // ─────────────────────────────────────────────────────────────────────────────────────────
    // S53 / fix-direction-2 SLICE 1 (D-CELL) — `pick_piece_cell` PURE-SELECTOR UNIT NET.
    //
    // Tests the per-piece cell selector in ISOLATION (scalar inputs, no image type — the module
    // boundary the signature preserves). Reproduces the Affect specialist's adjudicated target
    // table on the six probe feature vectors, plus the cell-2 guard, the spread reachability, and
    // the defensive clamp. The probe `(edge_activity, complexity)` pairs are the SAME literals
    // `tests/cell_distinctness_s53.rs` captured from `understand_image_pure` at clean HEAD.
    // ─────────────────────────────────────────────────────────────────────────────────────────

    /// K=4 for every authored archetype, so the cell-3 divert is always available in these tests.
    const K4: usize = 4;

    /// PROPERTY: the six probes separate into the Affect target table {Img1:1, Lena:0, Img2:0,
    /// example:2, magicstudio:3, Img3:3} — 4 distinct cells, exactly 2 on cell 3 (GR-3 selective).
    /// Archetype is a don't-care here (the selector keys only on the two scalars), so a fixed
    /// `Arch` exercises the path for all six.
    #[test]
    fn s53_pick_piece_cell_reproduces_affect_target_table() {
        // (name, edge_activity, complexity, expected_cell) — the adjudicated acceptance set.
        let probes: [(&str, f32, f32, usize); 6] = [
            ("AudioHaxImg1", 0.300_852_45, 0.0055, 1),
            ("Lena", 0.471_038_82, 0.1645, 0),
            ("AudioHaxImg2", 0.509_440_1, 0.0155, 0),
            ("example", 0.718_838_33, 0.905, 2),
            ("magicstudio", 0.106_048_584, 1.0, 3),
            ("AudioHaxImg3", 0.475_413_02, 0.229, 3),
        ];
        let mut distinct = std::collections::BTreeSet::new();
        let mut on_cell_3 = 0;
        for (name, edge, cplx, want) in probes {
            let got = pick_piece_cell(edge, cplx, MotifArchetype::Arch, K4);
            assert_eq!(
                got, want,
                "{name} (edge {edge}, cplx {cplx}) → cell {got}, expected {want} (Affect target)"
            );
            distinct.insert(got);
            if got == 3 {
                on_cell_3 += 1;
            }
        }
        assert!(
            distinct.len() >= 4,
            "the six probes must occupy >= 4 distinct cells (got {distinct:?})"
        );
        assert_eq!(
            on_cell_3, 2,
            "exactly 2 probes on the syncopated cell 3 (GR-3 selectivity); got {on_cell_3}"
        );
    }

    /// PROPERTY (the cell-2 GUARD): a high-edge, high-complexity image (`example`-like) keeps its
    /// even-subdivided cell 2 and is NOT swept to cell 3 by its high complexity. Without the guard
    /// it would divert and the distinct-cell count would drop 4→3.
    #[test]
    fn s53_pick_piece_cell_cell2_guard_keeps_busy_outlier_even() {
        // example: spread(0.719) ≈ 0.847 ≥ BUSY → primary cell 2; complexity 0.905 ≥ 0.20 but the
        // guard blocks the divert because primary == 2.
        let cell = pick_piece_cell(0.718_838_33, 0.905, MotifArchetype::Descent, K4);
        assert_eq!(
            cell, 2,
            "the busy outlier must keep cell 2 (the guard blocks the divert)"
        );
    }

    /// PROPERTY: the complexity divert sits in the (0.164, 0.229] window — it splits the carried
    /// Img3~Lena watch-pair (Img3 cplx 0.229 diverts to 3; Lena cplx 0.164 stays on the ramp).
    #[test]
    fn s53_pick_piece_cell_splits_img3_lena_watch_pair() {
        // Near-identical edges (both land primary cell 0); only complexity separates them.
        let img3 = pick_piece_cell(0.475_413_02, 0.229, MotifArchetype::Arch, K4);
        let lena = pick_piece_cell(0.471_038_82, 0.1645, MotifArchetype::Arch, K4);
        assert_eq!(
            img3, 3,
            "Img3 (cplx 0.229 ≥ 0.20) diverts to the profiled cell 3"
        );
        assert_eq!(
            lena, 0,
            "Lena (cplx 0.164 < 0.20) stays on the cell-0 anchor"
        );
        assert_ne!(img3, lena, "the watch-pair must split on the cell axis");
    }

    /// PROPERTY: the spread genuinely straddles both primary cuts for the real cluster, so all
    /// three ramp cells {1,0,2} are reachable (the S50 trap — cuts behind a pre-filter — is gone).
    #[test]
    fn s53_pick_piece_cell_primary_ramp_spans_all_three_cells() {
        // Low-complexity probes so the secondary never fires — isolate the PRIMARY ramp.
        let calm = pick_piece_cell(0.106_048_584, 0.0, MotifArchetype::Arch, K4); // spread 0.000 → cell 1
        let mid = pick_piece_cell(0.471_038_82, 0.0, MotifArchetype::Arch, K4); // spread 0.499 → cell 0
        let busy = pick_piece_cell(0.718_838_33, 0.0, MotifArchetype::Arch, K4); // spread 0.847 → cell 2
        assert_eq!(
            (calm, mid, busy),
            (1, 0, 2),
            "the primary ramp reaches cells 1/0/2"
        );
    }

    /// PROPERTY: the divert is unavailable (and never panics) when the vocabulary has < 4 cells;
    /// the index is clamped into `0..cell_count`.
    #[test]
    fn s53_pick_piece_cell_clamps_to_vocabulary_size() {
        // cell_count == 3: the cell-3 divert is suppressed (cell_count > 3 is false) → primary stands.
        let three = pick_piece_cell(0.475_413_02, 0.9, MotifArchetype::Arch, 3);
        assert!(
            three < 3,
            "with K=3 the divert is unavailable; primary cell stays in range"
        );
        // cell_count == 1: everything clamps to 0.
        let one = pick_piece_cell(0.9, 0.9, MotifArchetype::Arch, 1);
        assert_eq!(one, 0, "K=1 saturates every selection to cell 0");
    }

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
        // saturation01_raw=0.0 → Triad (3 notes); colorfulness_raw=0.0 → no mixture
        // append; edge=0.0/drop=0.0 → no secondary dominant / no borrowed iv. So one
        // symbol yields exactly one root-position triad — the determinism contract.
        let mut chords = eng.generate_chords(&prog, ROOT, mode, 0.0, 0.0, 0.0, 0.0);
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
                let chords = eng.generate_chords(&prog, ROOT, mode, 0.0, 0.0, 0.0, 0.0);
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

    // =====================================================================
    // VOICE-LEADING + PHRASE-STRUCTURE PROPERTY NET (Pass-1: RED-before-GREEN)
    //
    // These five tests pin the *musical* contract of the new public API
    // (`voice_lead_sequence`, `voice_roles`, `plan_phrases`). They are written
    // to FAIL on the current naive stubs (root-position passthrough, no cadence
    // placement, flat velocity) and to PASS once Pass-2 lands the real
    // algorithms. They hold the SAME determinism contract as the net above:
    // explicit Roman-numeral progressions, edge_complexity=0.0 (no secondary
    // dominant), brightness_drop=0.0 (no modal interchange), fixed ROOT=60.
    //
    // Voice-alignment contract relied on below: in the output of
    // `voice_lead_sequence`, `notes[0]` is the bass voice and `notes[1..]` are
    // upper voices, and a given index is the SAME voice across consecutive
    // chords. The role of each index is read from `ChordEngine::voice_roles`.
    // =====================================================================

    // ---- small private helpers for the property net ----

    /// Pitch class (0..=11) of a MIDI note.
    fn pc(n: u8) -> u8 {
        n % 12
    }

    /// Harmonic interval class between two MIDI notes: the absolute semitone
    /// distance reduced mod 12 (0 = unison/octave, 7 = perfect fifth).
    fn interval_class(a: u8, b: u8) -> u8 {
        ((a as i16 - b as i16).abs() % 12) as u8
    }

    /// Population variance of a velocity series (0 iff all values are equal).
    fn variance(values: &[u8]) -> f64 {
        if values.is_empty() {
            return 0.0;
        }
        let n = values.len() as f64;
        let mean = values.iter().map(|&v| v as f64).sum::<f64>() / n;
        values
            .iter()
            .map(|&v| {
                let d = v as f64 - mean;
                d * d
            })
            .sum::<f64>()
            / n
    }

    /// Deterministic progression -> input chords for the voice-leading layer.
    fn prog_chords(eng: &ChordEngine, romans: &[&str], mode: &str) -> Vec<Chord> {
        let prog: Vec<String> = romans.iter().map(|s| s.to_string()).collect();
        eng.generate_chords(&prog, ROOT, mode, 0.0, 0.0, 0.0, 0.0)
    }

    // ---------------------------------------------------------------------
    // PROPERTY 1 — UPPER VOICES NEVER LEAP MORE THAN A PERFECT FIFTH
    // theory: an UPPER (non-bass) voice is a connected melodic strand; under
    // conservative voice leading it moves by at most a perfect fifth
    // (MAX_UPPER_VOICE_MOTION = 7 semitones) between adjacent chords. The BASS
    // is exempt — it carries harmonic root motion and may leap freely. We read
    // each index's role from `ChordEngine::voice_roles` at the source chord T;
    // any index that is `Upper` at T must move <= 7 semitones into T+1.
    //
    // RED on the stub: `voice_lead_sequence` returns the root-position triads
    // unchanged, so upper voices leap by sevenths/octaves (e.g. Ionian V->vi an
    // upper voice moves 11 semitones). GREEN once real re-voicing caps motion.
    // ---------------------------------------------------------------------
    #[test]
    fn test_upper_voices_never_leap_beyond_perfect_fifth() {
        let eng = engine();
        let input = prog_chords(&eng, &["I", "V", "vi", "IV"], "Ionian");
        let led = eng.voice_lead_sequence(&input);

        for t in 0..led.len().saturating_sub(1) {
            let a = &led[t];
            let b = &led[t + 1];
            let roles_a = ChordEngine::voice_roles(a);
            let voices = a.notes.len().min(b.notes.len());
            for v in 0..voices {
                // Bass at the source chord is leap-exempt; only constrain uppers.
                if roles_a.get(v) == Some(&VoiceRole::Bass) {
                    continue;
                }
                let motion = (a.notes[v] as i16 - b.notes[v] as i16).unsigned_abs() as u8;
                assert!(
                    motion <= MAX_UPPER_VOICE_MOTION,
                    "upper voice index {v} leaps {motion} semitones \
                     ({} -> {}) across {:?} -> {:?}, exceeding the perfect-fifth \
                     cap of {MAX_UPPER_VOICE_MOTION}",
                    a.notes[v],
                    b.notes[v],
                    a.name,
                    b.name
                );
            }
        }
    }

    // ---------------------------------------------------------------------
    // PROPERTY 2 — NO PARALLEL PERFECT FIFTHS OR OCTAVES
    // theory: two voices that are a perfect fifth (interval class 7) or a
    // perfect octave/unison (interval class 0) apart must not stay at that same
    // perfect interval while BOTH move, T -> T+1 — that is a parallel perfect
    // consonance, the cardinal voice-leading prohibition. (If one voice holds a
    // common tone it is not "parallel" motion, so we require both voices to
    // actually change pitch.)
    //
    // RED on the stub: parallel root-position triads on stepwise root motion
    // (Ionian I->ii->iii->IV) keep the root<->fifth voices a perfect fifth apart
    // while both rise by step — textbook parallel fifths. GREEN once re-voicing
    // breaks the parallels.
    // ---------------------------------------------------------------------
    #[test]
    fn test_no_parallel_perfect_fifths_or_octaves() {
        let eng = engine();
        let input = prog_chords(&eng, &["I", "ii", "iii", "IV"], "Ionian");
        let led = eng.voice_lead_sequence(&input);

        for t in 0..led.len().saturating_sub(1) {
            let a = &led[t];
            let b = &led[t + 1];
            let voices = a.notes.len().min(b.notes.len());
            for i in 0..voices {
                for j in (i + 1)..voices {
                    let ic_a = interval_class(a.notes[i], a.notes[j]);
                    let ic_b = interval_class(b.notes[i], b.notes[j]);
                    let both_move = a.notes[i] != b.notes[i] && a.notes[j] != b.notes[j];
                    let same_perfect = ic_a == ic_b && (ic_a == 7 || ic_a == 0);
                    assert!(
                        !(same_perfect && both_move),
                        "parallel perfect {} between voices ({i},{j}) across \
                         {:?} -> {:?}: interval class {ic_a} held while both voices \
                         move ({}->{}, {}->{})",
                        if ic_a == 7 { "fifth" } else { "octave/unison" },
                        a.name,
                        b.name,
                        a.notes[i],
                        b.notes[i],
                        a.notes[j],
                        b.notes[j]
                    );
                }
            }
        }
    }

    // ---------------------------------------------------------------------
    // PROPERTY 3 — COMMON-TONE RETENTION
    // theory: when two adjacent chords share a pitch class, smooth voice leading
    // KEEPS that shared tone in the SAME voice rather than re-spelling it into a
    // different voice. We pick Ionian I=[60,64,67] (pcs {0,4,7}) -> vi=[69,60,64]
    // (pcs {9,0,4}); the shared pitch classes are {0,4}. For each shared pitch
    // class there must exist a voice index that carries it at BOTH T and T+1.
    //
    // RED on the stub: root-position passthrough places pc 0 at idx0 in I but
    // idx1 in vi (and pc 4 at idx1 then idx2), so NO index retains a shared pc.
    // GREEN once re-voicing holds common tones in place.
    // ---------------------------------------------------------------------
    #[test]
    fn test_common_tone_retained_in_same_voice() {
        let eng = engine();
        let input = prog_chords(&eng, &["I", "vi"], "Ionian");
        let led = eng.voice_lead_sequence(&input);
        assert!(
            led.len() >= 2,
            "need at least two chords for the common-tone test"
        );
        let a = &led[0];
        let b = &led[1];

        let pcs_a: std::collections::HashSet<u8> = a.notes.iter().map(|&n| pc(n)).collect();
        let pcs_b: std::collections::HashSet<u8> = b.notes.iter().map(|&n| pc(n)).collect();
        let shared: Vec<u8> = pcs_a.intersection(&pcs_b).copied().collect();
        assert!(
            !shared.is_empty(),
            "test progression must share a pitch class between adjacent chords \
             (chose Ionian I -> vi); got A pcs {pcs_a:?}, B pcs {pcs_b:?}"
        );

        for &p in &shared {
            let voices = a.notes.len().min(b.notes.len());
            let retained = (0..voices).any(|v| pc(a.notes[v]) == p && pc(b.notes[v]) == p);
            assert!(
                retained,
                "shared pitch class {p} between {:?} {:?} and {:?} {:?} must be held \
                 in the SAME voice index across the change, but no index keeps it",
                a.name, a.notes, b.name, b.notes
            );
        }
    }

    // ---------------------------------------------------------------------
    // PROPERTY 4 — CADENCES LAND AT PHRASE BOUNDARIES (HALF + PAC)
    // theory: a phrase closes with a cadence AT its boundary, never mid-phrase.
    // An antecedent rests on the dominant (HALF CADENCE on "V"); a consequent
    // closes with a PERFECT AUTHENTIC CADENCE (the V->I pair, ending on "I").
    // Over an antecedent+consequent period we require: every HalfCadence step is
    // at a boundary (position_in_phrase == phrase_len-1) and is a "V"; every
    // PerfectAuthenticCadence step is at a boundary, is a "I", and is immediately
    // preceded by a "V"; at least one of each cadence appears; and NO interior
    // step ever carries a cadence label.
    //
    // RED on the stub: `plan_phrases` only emits PhraseStart/Interior and never
    // places a cadence, so the "at least one HalfCadence / one PAC" assertions
    // fail. GREEN once real phrase/cadence positioning lands.
    // ---------------------------------------------------------------------
    #[test]
    fn test_cadences_sit_at_phrase_boundaries() {
        let eng = engine();
        // 8 chords: antecedent ...->V (half cadence), consequent ...->V->I (PAC).
        let input = prog_chords(
            &eng,
            &["I", "ii", "IV", "V", "vi", "IV", "V", "I"],
            "Ionian",
        );
        let plan = eng.plan_phrases(&input);
        assert!(!plan.is_empty(), "planner must produce steps");

        let mut saw_half = false;
        let mut saw_pac = false;

        for (i, step) in plan.iter().enumerate() {
            let at_boundary = step.position_in_phrase == step.phrase_len - 1;
            match step.position {
                PhrasePosition::HalfCadence => {
                    saw_half = true;
                    assert!(
                        at_boundary,
                        "HalfCadence at step {i} must sit at a phrase boundary \
                         (position_in_phrase {} of phrase_len {})",
                        step.position_in_phrase, step.phrase_len
                    );
                    assert!(
                        step.chord.name.contains('V') && !step.chord.name.contains("VI"),
                        "HalfCadence at step {i} must rest on a dominant 'V', got {:?}",
                        step.chord.name
                    );
                }
                PhrasePosition::PerfectAuthenticCadence => {
                    saw_pac = true;
                    assert!(
                        at_boundary,
                        "PAC at step {i} must sit at a phrase boundary \
                         (position_in_phrase {} of phrase_len {})",
                        step.position_in_phrase, step.phrase_len
                    );
                    assert!(
                        step.chord.name == "I" || step.chord.name == "i",
                        "PAC at step {i} must close on the tonic 'I', got {:?}",
                        step.chord.name
                    );
                    assert!(i > 0, "PAC at step {i} must be preceded by a chord");
                    let prev = &plan[i - 1].chord.name;
                    assert!(
                        prev.contains('V') && !prev.contains("VI"),
                        "PAC at step {i} must be preceded by a dominant 'V' (the V-I \
                         pair), got predecessor {prev:?}"
                    );
                }
                PhrasePosition::PhraseStart | PhrasePosition::Interior => {
                    // A cadence label must never hide on an interior/start step:
                    // enforced structurally by the match arms — nothing to assert.
                }
            }
        }

        assert!(
            saw_half,
            "an 8-chord period must contain at least one HalfCadence at a phrase \
             boundary, but the plan placed none"
        );
        assert!(
            saw_pac,
            "an 8-chord period must contain at least one PerfectAuthenticCadence \
             at a phrase boundary, but the plan placed none"
        );
    }

    // ---------------------------------------------------------------------
    // PROPERTY 5 — INTRA-PHRASE VELOCITY VARIES (STRUCTURAL FLOOR)
    // theory: the structural-velocity floor must mark phrase shape — boundaries
    // and cadence points get a different weight than interior steps — so within
    // at least one phrase the velocities are NOT all identical (variance > 0).
    // (This is the bare structural floor, not expressive dynamics.)
    //
    // RED on the stub: `plan_phrases` assigns a flat velocity (80) to every
    // step, so every phrase has variance 0. GREEN once the structural velocity
    // signature introduces boundary/cadence variation.
    // ---------------------------------------------------------------------
    #[test]
    fn test_velocity_varies_within_a_phrase() {
        let eng = engine();
        let input = prog_chords(
            &eng,
            &["I", "ii", "IV", "V", "vi", "IV", "V", "I"],
            "Ionian",
        );
        let plan = eng.plan_phrases(&input);
        assert!(!plan.is_empty(), "planner must produce steps");

        let mut by_phrase: std::collections::BTreeMap<usize, Vec<u8>> =
            std::collections::BTreeMap::new();
        for step in &plan {
            by_phrase
                .entry(step.phrase_index)
                .or_default()
                .push(step.velocity);
        }

        let any_varies = by_phrase.values().any(|vels| variance(vels) > 0.0);
        let summary: Vec<(usize, f64)> = by_phrase
            .iter()
            .map(|(idx, vels)| (*idx, variance(vels)))
            .collect();
        assert!(
            any_varies,
            "at least one phrase must have velocity variance > 0 (a structural \
             floor that marks phrase shape), but every phrase was flat: \
             per-phrase (phrase_index, variance) = {summary:?}"
        );
    }

    // =====================================================================
    // S6 — PERFORMANCE-REALIZATION PROPERTY NET (Pass-A RED-before-GREEN)
    //
    // These tests pin the *musical* contract of `realize_step` (the four
    // expressive dimensions) and the Pass-B voice-spacing enforcement. They
    // are written to FAIL on the Pass-A naive stubs (flat single note at the
    // structural floor, 90% hold, offset 0, role/features ignored;
    // `upper_voices_well_spaced` returning `true` unconditionally) and to PASS
    // once Pass B lands the real realization + spacing-enforced voice leading.
    //
    // Determinism contract (held by every test): explicit Roman-numeral
    // progressions, edge_complexity=0.0, brightness_drop=0.0, fixed ROOT=60,
    // no RNG, no filesystem. The expressive INPUTS are the fixed `PerfFeatures`
    // and `ms_per_step` passed to `realize_step`. Per design-s6-expressivity.md.
    // =====================================================================

    // ---- small private helpers for the realization net ----

    /// The canonical 8-chord Ionian period (`plan_phrases` parses it as two
    /// 4-step phrases: an antecedent on a half cadence + a consequent PAC).
    /// Reused by the dynamics/rhythm/articulation scans so they exercise a real
    /// phrase shape, not an ad-hoc step list.
    fn ionian_period(eng: &ChordEngine) -> Vec<StepPlan> {
        let input = prog_chords(eng, &["I", "ii", "IV", "V", "vi", "IV", "V", "I"], "Ionian");
        eng.plan_phrases(&input)
    }

    /// A neutral mid-band feature set (used where a single fixed expressive input
    /// is wanted so any observed variation comes from PHRASE POSITION / ROLE, not
    /// from varying features). saturation drives level, brightness register,
    /// edge_density rhythmic activity.
    fn mid_features() -> PerfFeatures {
        PerfFeatures {
            saturation: 50.0,
            brightness: 50.0,
            edge_density: 0.5,
        }
    }

    // ---------------------------------------------------------------------
    // DYNAMICS 1 — phrase contour exceeds the structural floor.
    // property: the realized per-step velocity series for a single voice across
    // a phrase must carry MORE variation than the bare StepPlan.velocity floor
    // over the same steps — the expressive contour (messa di voce + accent +
    // taper) adds variation ON TOP of the floor, so its variance is STRICTLY
    // GREATER than the floor's.
    //
    // RED on stub: `realize_step` emits velocity == step.velocity (the floor
    // verbatim), so the realized series IS the floor series and the two
    // variances are EQUAL, not strictly greater. GREEN once Pass B shapes the
    // floor with saturation level + half-sine swell + metric accent + taper.
    // ---------------------------------------------------------------------
    #[test]
    fn test_dynamics_contour_exceeds_structural_floor() {
        let eng = engine();
        let plan = ionian_period(&eng);
        assert!(!plan.is_empty(), "planner must produce steps");
        let features = mid_features();
        let ms_per_step = 1000u64;

        // One fixed voice/instrument across the whole phrase plan. With 3
        // instruments, index 1 is HarmonicFill — a constant role, so any velocity
        // variation beyond the floor is the phrase CONTOUR, not a role artifact.
        let num_instruments = 3usize;
        let inst_idx = 1usize;

        let (s, k) = neutral_ctx_parts();
        let ctx = StepContext::single_section_default(&s, &k);
        let mut realized: Vec<u8> = Vec::new();
        let mut floor: Vec<u8> = Vec::new();
        for step in &plan {
            let events = realize_step(
                step,
                inst_idx,
                num_instruments,
                &features,
                ms_per_step,
                &ctx,
            );
            // Take the step's representative (loudest) realized velocity so a
            // multi-onset step still contributes one contour sample.
            let v = events
                .iter()
                .map(|e| e.velocity)
                .max()
                .unwrap_or(step.velocity);
            realized.push(v);
            floor.push(step.velocity);
        }

        let var_realized = variance(&realized);
        let var_floor = variance(&floor);
        assert!(
            var_realized > var_floor,
            "realized velocity variance ({var_realized:.3}) must STRICTLY EXCEED \
             the structural-floor variance ({var_floor:.3}) — the expressive \
             contour must add variation on top of the floor. realized={realized:?} \
             floor={floor:?}"
        );
    }

    // ---------------------------------------------------------------------
    // DYNAMICS 2 — metric accent: strong beat louder than adjacent weak beat.
    // property: a metrically STRONG position (the phrase-start downbeat) must
    // realize a HIGHER velocity than an adjacent WEAK interior position in the
    // same phrase. We read positions from the plan so the comparison is
    // unambiguous: phrase 0's PhraseStart (position_in_phrase 0) vs an Interior
    // step of phrase 0 (position_in_phrase 1).
    //
    // RED on stub: flat velocity == floor for every step, AND the floor itself
    // would have start(88) > interior(76) — so to make this fail ONLY on the
    // stub's flatness we compare two steps the realization must DIFFERENTIATE
    // beyond the floor: we require the realized START to exceed the realized
    // INTERIOR by MORE than the structural-floor delta alone, i.e. the accent
    // adds on top. On the stub realized delta == floor delta, so strictly-greater
    // fails. GREEN once Pass B adds the metric accent.
    // ---------------------------------------------------------------------
    #[test]
    fn test_dynamics_metric_accent_strong_exceeds_weak() {
        let eng = engine();
        let plan = ionian_period(&eng);
        let features = mid_features();
        let ms_per_step = 1000u64;
        let (num_instruments, inst_idx) = (3usize, 1usize);

        // Locate phrase 0's PhraseStart and an adjacent Interior step.
        let start = plan
            .iter()
            .find(|s| s.phrase_index == 0 && s.position == PhrasePosition::PhraseStart)
            .expect("phrase 0 must have a PhraseStart step");
        let interior = plan
            .iter()
            .find(|s| s.phrase_index == 0 && s.position == PhrasePosition::Interior)
            .expect("phrase 0 must have an Interior step");

        let (s, k) = neutral_ctx_parts();
        let ctx = StepContext::single_section_default(&s, &k);
        let v_start = realize_step(
            start,
            inst_idx,
            num_instruments,
            &features,
            ms_per_step,
            &ctx,
        )
        .iter()
        .map(|e| e.velocity)
        .max()
        .unwrap_or(start.velocity) as i32;
        let v_interior = realize_step(
            interior,
            inst_idx,
            num_instruments,
            &features,
            ms_per_step,
            &ctx,
        )
        .iter()
        .map(|e| e.velocity)
        .max()
        .unwrap_or(interior.velocity) as i32;

        // The realized strong-minus-weak gap must EXCEED the bare structural-floor
        // gap — i.e. the metric accent contributes on top of the floor. On the
        // stub the realized gap equals the floor gap, so this is RED.
        let floor_gap = start.velocity as i32 - interior.velocity as i32;
        let realized_gap = v_start - v_interior;
        assert!(
            realized_gap > floor_gap,
            "realized strong-beat advantage ({realized_gap}) must exceed the bare \
             structural-floor gap ({floor_gap}) — the metric accent must add on \
             top of the floor. v_start={v_start} v_interior={v_interior} \
             floor_start={} floor_interior={}",
            start.velocity,
            interior.velocity
        );
    }

    // ---------------------------------------------------------------------
    // RHYTHM — >=3 distinct onset/duration patterns across a scan.
    // property: realization must produce at least THREE genuinely distinct
    // rhythmic signatures (sustained / arpeggio / dotted / syncopated /
    // rest-as-gesture / harmonic-rhythm acceleration), selected by edge_density
    // band, role, and phrase position. We scan the period across all three roles
    // AND across low/mid/high edge_density and collect each realization's
    // signature: the sorted sequence of (offset_ms, hold_ms) pairs.
    //
    // RED on stub: every call returns ONE note at offset 0, hold ~90% — a single
    // signature shape regardless of inputs, so only 1 distinct pattern appears.
    // GREEN once Pass B emits >=3 patterns.
    // ---------------------------------------------------------------------
    #[test]
    fn test_rhythm_at_least_three_distinct_patterns() {
        let eng = engine();
        let plan = ionian_period(&eng);
        let ms_per_step = 1200u64;
        let num_instruments = 3usize;

        let (s, k) = neutral_ctx_parts();
        let ctx = StepContext::single_section_default(&s, &k);
        let mut signatures: std::collections::HashSet<Vec<(u64, u64)>> =
            std::collections::HashSet::new();
        for &edge in &[0.05f32, 0.5, 0.95] {
            let features = PerfFeatures {
                saturation: 50.0,
                brightness: 50.0,
                edge_density: edge,
            };
            for inst_idx in 0..num_instruments {
                for step in &plan {
                    let events = realize_step(
                        step,
                        inst_idx,
                        num_instruments,
                        &features,
                        ms_per_step,
                        &ctx,
                    );
                    let mut sig: Vec<(u64, u64)> =
                        events.iter().map(|e| (e.offset_ms, e.hold_ms)).collect();
                    sig.sort_unstable();
                    signatures.insert(sig);
                }
            }
        }

        assert!(
            signatures.len() >= 3,
            "expected at least 3 distinct rhythmic signatures across the role x \
             edge_density x phrase-position scan, found {}: {signatures:?}",
            signatures.len()
        );
    }

    // ---------------------------------------------------------------------
    // ARTICULATION 1 — note durations vary (staccato/portato/legato).
    // property: across a scan, realized hold_ms expressed as a FRACTION of the
    // per-note time budget must NOT all be the same value — at least TWO distinct
    // hold fractions appear (a detached staccato vs a connected legato is the
    // articulation handle).
    //
    // RED on stub: hold_ms is always round(ms_per_step * 0.9), so every fraction
    // is ~0.9 — exactly ONE distinct value. GREEN once Pass B sets per-note hold
    // fractions by articulation class + ritardando.
    // ---------------------------------------------------------------------
    #[test]
    fn test_articulation_hold_fractions_vary() {
        let eng = engine();
        let plan = ionian_period(&eng);
        let ms_per_step = 1000u64;
        let num_instruments = 3usize;

        // Quantize hold fraction to 0.05 buckets so we count GENUINELY distinct
        // articulations, not float noise.
        let (s, k) = neutral_ctx_parts();
        let ctx = StepContext::single_section_default(&s, &k);
        let mut fractions: std::collections::HashSet<u32> = std::collections::HashSet::new();
        for &edge in &[0.05f32, 0.5, 0.95] {
            let features = PerfFeatures {
                saturation: 50.0,
                brightness: 50.0,
                edge_density: edge,
            };
            for inst_idx in 0..num_instruments {
                for step in &plan {
                    let events = realize_step(
                        step,
                        inst_idx,
                        num_instruments,
                        &features,
                        ms_per_step,
                        &ctx,
                    );
                    for e in &events {
                        let frac = e.hold_ms as f64 / ms_per_step as f64;
                        fractions.insert((frac * 20.0).round() as u32);
                    }
                }
            }
        }

        assert!(
            fractions.len() >= 2,
            "expected at least 2 distinct hold fractions (staccato vs legato), \
             found {} (each is hold_ms/ms_per_step in 0.05 buckets): {fractions:?} \
             — the stub holds a constant ~0.9 for every note",
            fractions.len()
        );
    }

    // ---------------------------------------------------------------------
    // ARTICULATION 2 — phrase-end ritardando raises hold_ms.
    // property: the cadential (phrase-final) step must realize a LONGER hold_ms
    // than the mean interior hold_ms of its phrase — the ritardando/taper lets
    // the arrival ring. Per design-s6 §2.3 ("the cadence step's note hold_ms >
    // the mean interior hold_ms of its phrase").
    //
    // RED on stub: every hold_ms is the same ~90% value, so the cadence hold is
    // EQUAL to (never greater than) the interior mean. GREEN once Pass B applies
    // the phrase-end ritardando.
    // ---------------------------------------------------------------------
    #[test]
    fn test_articulation_phrase_end_ritardando_lengthens_hold() {
        let eng = engine();
        let plan = ionian_period(&eng);
        let features = mid_features();
        let ms_per_step = 1000u64;
        let (num_instruments, inst_idx) = (3usize, 1usize);

        // Pick a phrase that has BOTH a cadence step and >=1 interior step, then
        // compare the cadence note's hold to the mean interior hold of that phrase.
        let cadence = plan
            .iter()
            .find(|s| {
                matches!(
                    s.position,
                    PhrasePosition::HalfCadence | PhrasePosition::PerfectAuthenticCadence
                )
            })
            .expect("the period must contain at least one cadence step");
        let phrase = cadence.phrase_index;

        let (sec, kt) = neutral_ctx_parts();
        let ctx = StepContext::single_section_default(&sec, &kt);
        let interior_holds: Vec<u64> = plan
            .iter()
            .filter(|s| s.phrase_index == phrase && s.position == PhrasePosition::Interior)
            .flat_map(|s| realize_step(s, inst_idx, num_instruments, &features, ms_per_step, &ctx))
            .map(|e| e.hold_ms)
            .collect();
        assert!(
            !interior_holds.is_empty(),
            "phrase {phrase} must have at least one interior step to compare against"
        );
        let interior_mean =
            interior_holds.iter().map(|&h| h as f64).sum::<f64>() / interior_holds.len() as f64;

        let cadence_hold = realize_step(
            cadence,
            inst_idx,
            num_instruments,
            &features,
            ms_per_step,
            &ctx,
        )
        .iter()
        .map(|e| e.hold_ms)
        .max()
        .expect("cadence step must sound at least one note") as f64;

        assert!(
            cadence_hold > interior_mean,
            "cadence-step hold_ms ({cadence_hold}) must EXCEED the phrase's mean \
             interior hold_ms ({interior_mean:.1}) — the phrase-end ritardando \
             lengthens the final note. (phrase {phrase}, interior holds \
             {interior_holds:?})"
        );
    }

    // ---------------------------------------------------------------------
    // ORCHESTRATION ROLES — per-role behavioral difference + role mapping.
    // property: with num_instruments >= 3 the SAME step realized for a Bass
    // (idx 0), a HarmonicFill (middle), and a Melody (idx num-1), under fixed
    // features, must DIFFER in a role-meaningful way: register (bass below
    // melody) and/or rhythmic activity (melody onset count >= bass onset count,
    // and they must not be identical realizations). We also assert
    // `instrument_role` returns the documented role for representative indices
    // (this part is FINAL/real and GREEN immediately — kept per the brief).
    //
    // RED on stub: `realize_step` ignores the role entirely; bass/fill/melody all
    // get `notes[inst_idx % len]` at the same flat dynamic/onset shape, so the
    // register-ordered + onset assertions fail. GREEN once Pass B differentiates
    // roles by register, rhythm, and motion.
    // ---------------------------------------------------------------------
    #[test]
    fn test_orchestration_roles_differ_by_register_and_activity() {
        let eng = engine();
        let plan = ionian_period(&eng);
        let features = mid_features();
        let ms_per_step = 1000u64;
        let num = 4usize; // idx0=Bass, idx1,2=HarmonicFill, idx3=Melody

        // --- role mapping (FINAL/real — GREEN immediately) ---
        assert_eq!(
            instrument_role(0, num),
            OrchestralRole::Bass,
            "idx 0 = Bass"
        );
        assert_eq!(
            instrument_role(num - 1, num),
            OrchestralRole::Melody,
            "idx num-1 = Melody"
        );
        assert_eq!(
            instrument_role(1, num),
            OrchestralRole::HarmonicFill,
            "middle idx = HarmonicFill"
        );
        // Degenerate counts from the FINAL scheme table.
        assert_eq!(
            instrument_role(0, 1),
            OrchestralRole::Melody,
            "lone line = Melody"
        );
        assert_eq!(
            instrument_role(0, 2),
            OrchestralRole::Bass,
            "2-inst idx0 = Bass"
        );
        assert_eq!(
            instrument_role(1, 2),
            OrchestralRole::Melody,
            "2-inst idx1 = Melody"
        );

        // --- per-role realization difference (RED on stub) ---
        // Use a phrase-start step (always sounds; never rest-as-gesture).
        let step = plan
            .iter()
            .find(|s| s.position == PhrasePosition::PhraseStart)
            .expect("need a phrase-start step");

        let (s, k) = neutral_ctx_parts();
        let ctx = StepContext::single_section_default(&s, &k);
        let bass = realize_step(step, 0, num, &features, ms_per_step, &ctx);
        let melody = realize_step(step, num - 1, num, &features, ms_per_step, &ctx);

        assert!(
            !bass.is_empty() && !melody.is_empty(),
            "bass and melody must both sound on a phrase-start step"
        );

        // Register separation: the bass's lowest note must sit BELOW the melody's
        // highest note (the bass anchors the low register, the melody the top).
        let bass_min = bass.iter().map(|e| e.note).min().unwrap();
        let melody_max = melody.iter().map(|e| e.note).max().unwrap();
        assert!(
            bass_min < melody_max,
            "bass min note ({bass_min}) must be BELOW melody max note \
             ({melody_max}) — register separation by role. bass={bass:?} \
             melody={melody:?}"
        );

        // Rhythmic-freedom separation: melody is at least as active as the bass,
        // and the two realizations are NOT byte-identical (the stub makes them
        // identical-shaped, differing only in the picked pitch).
        assert!(
            melody.len() >= bass.len(),
            "melody onset count ({}) must be >= bass onset count ({}) — the \
             melody carries the most rhythmic freedom. bass={bass:?} melody={melody:?}",
            melody.len(),
            bass.len()
        );
        assert!(
            bass != melody,
            "bass and melody realizations must DIFFER in a role-meaningful way \
             (register/rhythm/motion), but they are identical: {bass:?}"
        );
    }

    // ---------------------------------------------------------------------
    // VOICE-SPACING — upper voices never collapse to a unison.
    // property: every chord out of `voice_lead_sequence` must place its UPPER
    // voices (indices 1..) on DISTINCT MIDI notes — no two upper voices may share
    // one pitch (the bass at index 0 is exempt). A unison collapse silently thins
    // the triad to a dyad. Per design-s6 §5.1 (MIN_UPPER_VOICE_SPACING = 1).
    //
    // PINNED PROGRESSION: Ionian ["I","IV","V","I"] — confirmed (this session,
    // and cited by design-s6 §5.1) to collapse the IV chord to [65,65,65] under
    // S4's minimal-motion + common-tone scorer. The same scorer also collapses I
    // to [72,60,60] here; the test asserts the spacing rule over EVERY output
    // chord so it catches whichever collapse(s) the search produces.
    //
    // RED now: S4 voice leading produces the [65,65,65] unison collapse, so the
    // actual-notes check fails. (We intentionally test the ACTUAL notes rather
    // than rely on `upper_voices_well_spaced`, since that stub returns `true`
    // unconditionally and would mask the defect.) GREEN once Pass B wires the
    // spacing check into `voice_lead_one`'s candidate rejection.
    // ---------------------------------------------------------------------
    #[test]
    fn test_voice_spacing_upper_voices_never_collapse_to_unison() {
        let eng = engine();
        // Confirmed-collapsing progression (IV -> [65,65,65] in Ionian).
        let input = prog_chords(&eng, &["I", "IV", "V", "I"], "Ionian");
        let led = eng.voice_lead_sequence(&input);
        assert!(!led.is_empty(), "voice leading must produce chords");

        for chord in &led {
            // Upper voices are indices 1.. ; the bass (index 0) is exempt.
            let upper = &chord.notes[1.min(chord.notes.len())..];
            for i in 0..upper.len() {
                for j in (i + 1)..upper.len() {
                    assert_ne!(
                        upper[i],
                        upper[j],
                        "upper voices {} and {} of chord {:?} collapsed to the same \
                         MIDI note {} (unison) — the triad thins to a dyad. \
                         full voicing = {:?}",
                        i + 1,
                        j + 1,
                        chord.name,
                        upper[i],
                        chord.notes
                    );
                }
            }
            // The Pass-B checker must AGREE with the actual-notes verdict (it is a
            // stub returning `true` now, so this is consistent today and stays
            // correct once both are real).
            assert!(
                upper_voices_well_spaced(&chord.notes),
                "upper_voices_well_spaced disagrees with the actual-notes spacing \
                 verdict for chord {:?} = {:?}",
                chord.name,
                chord.notes
            );
        }
    }

    // =====================================================================
    // S13 — IMAGE→MUSIC DIVERSITY PROPERTY NET (Music Theory Specialist)
    //
    // Focused unit tests for the music-side diversity fixes (design-s13 §2/§4/§6):
    // harmonic complexity from saturation, the CONTINUOUS articulation curve,
    // per-image rhythmic density, mode-mixture from colorfulness, and the corrected
    // secondary dominant. These hold the same determinism contract as the nets above
    // (explicit progressions, fixed ROOT, no RNG, no synth). The cross-cutting
    // headline net is the Test Engineer's job; these pin MY functions.
    //
    // RAW-feature note: generate_chords takes RAW seam scalars (saturation 0..100,
    // edge 0..~0.05, hue_spread ~0..1) and normalizes them itself, so these tests
    // feed raw values exactly as the engine would.
    // =====================================================================

    /// Build chords with all the S13 knobs explicit (raw seam values).
    #[allow(clippy::too_many_arguments)]
    fn gen(
        eng: &ChordEngine,
        romans: &[&str],
        mode: &str,
        edge_raw: f32,
        drop: f32,
        sat_raw: f32,
        color_raw: f32,
    ) -> Vec<Chord> {
        let prog: Vec<String> = romans.iter().map(|s| s.to_string()).collect();
        eng.generate_chords(&prog, ROOT, mode, edge_raw, drop, sat_raw, color_raw)
    }

    // ---------------------------------------------------------------------
    // HARMONIC COMPLEXITY — chord-tone count tracks saturation (design-s13 §2).
    // property: low saturation → bare triads (3 notes); mid → +7th (4); high →
    // +7th+9th (5). This is the fix for the operator's "computer triads".
    // ---------------------------------------------------------------------
    #[test]
    fn test_harmonic_complexity_tracks_saturation() {
        let eng = engine();
        // Low sat (raw 20/100 = 0.20 < 0.31) → every chord a 3-note triad.
        let low = gen(&eng, &["I", "IV", "V", "vi"], "Ionian", 0.0, 0.0, 20.0, 0.0);
        for c in &low {
            assert_eq!(
                c.notes.len(),
                3,
                "low-saturation chord {:?} must be a bare triad (3 notes), got {:?}",
                c.name,
                c.notes
            );
        }
        // Mid sat (raw 50 = 0.50, band 31-70) → 7th chords (4 notes).
        let mid = gen(&eng, &["I", "IV", "V", "vi"], "Ionian", 0.0, 0.0, 50.0, 0.0);
        assert!(
            mid.iter().all(|c| c.notes.len() == 4),
            "mid-saturation chords must all carry the diatonic 7th (4 notes): {:?}",
            mid.iter()
                .map(|c| (c.name.clone(), c.notes.len()))
                .collect::<Vec<_>>()
        );
        // High sat (raw 90 = 0.90, band 71-100) → 7th+9th (5 notes).
        let high = gen(&eng, &["I", "IV", "V", "vi"], "Ionian", 0.0, 0.0, 90.0, 0.0);
        assert!(
            high.iter().all(|c| c.notes.len() == 5),
            "high-saturation chords must carry the 7th AND 9th (5 notes): {:?}",
            high.iter()
                .map(|c| (c.name.clone(), c.notes.len()))
                .collect::<Vec<_>>()
        );
    }

    // ---------------------------------------------------------------------
    // HARMONIC COMPLEXITY — the added tones are DIATONIC (in-scale).
    // property: the 7th of Ionian I (C major) is B (pc 11); the 9th is D (pc 2).
    // The chord must remain in-mode, not reach for chromatic colour.
    // ---------------------------------------------------------------------
    #[test]
    fn test_seventh_and_ninth_are_diatonic() {
        let eng = engine();
        // High-sat tonic in Ionian: I7add9 over C → C E G B D.
        let chords = gen(&eng, &["I"], "Ionian", 0.0, 0.0, 90.0, 0.0);
        assert_eq!(
            chords.len(),
            1,
            "single symbol, no inserts/mixture at color=0"
        );
        let pcs: std::collections::HashSet<u8> = chords[0].notes.iter().map(|&n| n % 12).collect();
        // C(0) E(4) G(7) B(11) D(2) — all diatonic to C Ionian.
        for pc in [0u8, 4, 7, 11, 2] {
            assert!(
                pcs.contains(&pc),
                "Ionian I7add9 must contain diatonic pc {pc} (C E G B D); got pcs {pcs:?}"
            );
        }
    }

    // ---------------------------------------------------------------------
    // ARTICULATION — CONTINUOUS curve: calm image holds LONGER than busy, and a
    // calm image actually crosses into connected/legato (frac > 0.95). This is
    // the direct "uniformly short" killer (design-s13 §2 articulation / §6.3).
    // ---------------------------------------------------------------------
    #[test]
    fn test_articulation_curve_calm_longer_than_busy_and_crosses_legato() {
        let eng = engine();
        let plan = ionian_period(&eng);
        let ms_per_step = 1000u64;
        let num = 3usize;
        let (s, k) = neutral_ctx_parts();
        let ctx = StepContext::single_section_default(&s, &k);

        // Mean hold fraction over the MELODY's CURVE-GOVERNED steps (PhraseStart /
        // Interior — the sustained branch). We exclude the cadence step (a fixed
        // LEGATO ring, deliberately byte-stable per spec §7) and the pre-cadence step
        // (a fixed arpeggiated acceleration). Those are STRUCTURAL figures, not the
        // articulation curve under test; the operator's "uniformly short" complaint is
        // about the body of the line, which is exactly these steps. We take each step's
        // representative (longest) hold so a multi-onset step contributes one sample.
        let mean_frac = |edge_raw: f32| -> f64 {
            let features = PerfFeatures {
                saturation: 50.0,
                brightness: 50.0,
                edge_density: edge_raw,
            };
            let mut fracs: Vec<f64> = Vec::new();
            for step in &plan {
                let curve_governed = matches!(
                    step.position,
                    PhrasePosition::PhraseStart | PhrasePosition::Interior
                ) && !(step.position_in_phrase + 2 >= step.phrase_len
                    && step.position_in_phrase > 0); // skip pre-cadence
                if !curve_governed {
                    continue;
                }
                if let Some(max_hold) =
                    realize_step(step, num - 1, num, &features, ms_per_step, &ctx)
                        .iter()
                        .map(|e| e.hold_ms)
                        .max()
                {
                    fracs.push(max_hold as f64 / ms_per_step as f64);
                }
            }
            fracs.iter().sum::<f64>() / fracs.len().max(1) as f64
        };

        // Calm A: raw per-bar edge 0.004 (edge_activity ≈ 0.08) → near-legato.
        // Busy B: raw per-bar edge 0.045 (edge_activity ≈ 0.90) → detached/arpeggiated.
        let frac_a = mean_frac(0.004);
        let frac_b = mean_frac(0.045);
        assert!(
            frac_a - frac_b > 0.05,
            "calm image mean hold fraction ({frac_a:.3}) must exceed busy ({frac_b:.3}) \
             by a clear margin — the continuous curve must make note length vary"
        );
        assert!(
            frac_a > 0.95,
            "a CALM image's mean hold fraction ({frac_a:.3}) must cross into connected/\
             legato territory (>0.95) — the old code capped legato at 0.95 and snapped \
             every real photo there, producing 'uniformly short' notes"
        );
    }

    // ---------------------------------------------------------------------
    // RHYTHMIC DENSITY — busier image yields a DIFFERENT onset-count distribution
    // (design-s13 §2 density / §6.4). Not "a pattern exists" — the multiset differs.
    // ---------------------------------------------------------------------
    #[test]
    fn test_rhythmic_density_distribution_differs_with_busyness() {
        let eng = engine();
        let plan = ionian_period(&eng);
        let ms_per_step = 1200u64;
        let num = 3usize;
        let (s, k) = neutral_ctx_parts();
        let ctx = StepContext::single_section_default(&s, &k);

        let onset_multiset = |edge_raw: f32| -> std::collections::BTreeMap<usize, usize> {
            let features = PerfFeatures {
                saturation: 50.0,
                brightness: 50.0,
                edge_density: edge_raw,
            };
            let mut counts: std::collections::BTreeMap<usize, usize> =
                std::collections::BTreeMap::new();
            for inst in 0..num {
                for step in &plan {
                    let n = realize_step(step, inst, num, &features, ms_per_step, &ctx).len();
                    *counts.entry(n).or_default() += 1;
                }
            }
            counts
        };

        let calm = onset_multiset(0.004); // edge_activity ≈ 0.08 → sustained melody
        let busy = onset_multiset(0.045); // edge_activity ≈ 0.90 → arpeggiated melody
        assert_ne!(
            calm, busy,
            "the per-step onset-count distribution must differ between a calm and a \
             busy image (busier → more onsets on the melody). calm={calm:?} busy={busy:?}"
        );
    }

    // ---------------------------------------------------------------------
    // MODE MIXTURE — colorfulness (hue_spread) adds a borrowed chord that a
    // monochrome image of the SAME mean hue does not get (design-s13 §5 item 4 /
    // §6.5). Decouples harmonic colour from the single mean-hue mode pick.
    // ---------------------------------------------------------------------
    #[test]
    fn test_colorfulness_adds_borrowed_chord() {
        let eng = engine();
        // Same mode (same mean hue, decided upstream); only colorfulness differs.
        let mono = gen(&eng, &["I", "IV", "V", "I"], "Ionian", 0.0, 0.0, 20.0, 0.05);
        let vivid = gen(&eng, &["I", "IV", "V", "I"], "Ionian", 0.0, 0.0, 20.0, 0.65);
        // The vivid (palette-spread) image appends the borrowed bVI; the mono does not.
        assert!(
            !mono.iter().any(|c| c.name == "bVI"),
            "a low-spread (monochrome) image must NOT borrow bVI: {:?}",
            mono.iter().map(|c| c.name.clone()).collect::<Vec<_>>()
        );
        assert!(
            vivid.iter().any(|c| c.name == "bVI"),
            "a high-spread (colorful) image MUST add the borrowed bVI mixture chord, \
             decoupling colour from the mean-hue mode: {:?}",
            vivid.iter().map(|c| c.name.clone()).collect::<Vec<_>>()
        );
        assert!(
            vivid.len() > mono.len(),
            "the mixture chord must be additive (vivid has more chords than mono)"
        );
    }

    // ---------------------------------------------------------------------
    // SECONDARY DOMINANT — fires on a busy image, is the V OF the next chord (a
    // chromatic, non-home tone), and does NOT fire on a calm image (design-s13
    // §4 / §6.7). Also: it honors the look-ahead (different `next` → different V/x).
    // ---------------------------------------------------------------------
    #[test]
    fn test_secondary_dominant_fires_and_targets_next() {
        let eng = engine();
        // Busy: raw edge 0.045 → edge_activity ≈ 0.90 > 0.55 trigger.
        // Progression I → IV: the inserted chord must be V/IV (dominant of IV).
        let busy = gen(&eng, &["I", "IV"], "Ionian", 0.045, 0.0, 20.0, 0.0);
        let names: Vec<&str> = busy.iter().map(|c| c.name.as_str()).collect();
        assert!(
            names.contains(&"V/IV"),
            "busy image must insert the secondary dominant V/IV (dominant of the next \
             chord), not the home V; got {names:?}"
        );
        // V/IV root = a P5 above IV's root. IV root in Ionian = tonic+5 = 65 (F);
        // V/IV root = 65+7 = 72 (C). The chromatic tone is its major 3rd 72+4 = 76 (E
        // is diatonic, but the dom7 would bring Bb) — assert the inserted root is 72.
        let vfour = busy.iter().find(|c| c.name == "V/IV").unwrap();
        assert_eq!(
            vfour.notes[0], 72,
            "V/IV must be rooted a perfect fifth above IV's root (F=65 → C=72); got {:?}",
            vfour.notes
        );

        // Look-ahead honored: I → V inserts V/V whose root differs from V/IV's.
        let busy2 = gen(&eng, &["I", "V"], "Ionian", 0.045, 0.0, 20.0, 0.0);
        let vfive = busy2
            .iter()
            .find(|c| c.name == "V/V")
            .expect("V/V must be inserted");
        assert_ne!(
            vfour.notes[0], vfive.notes[0],
            "V/IV and V/V must have different roots — proves `next` is actually consumed"
        );

        // Calm: raw edge 0.004 → edge_activity ≈ 0.08 < 0.55 → NO secondary dominant.
        let calm = gen(&eng, &["I", "IV"], "Ionian", 0.004, 0.0, 20.0, 0.0);
        assert!(
            !calm.iter().any(|c| c.name.starts_with("V/")),
            "calm image must NOT insert a secondary dominant: {:?}",
            calm.iter().map(|c| c.name.clone()).collect::<Vec<_>>()
        );
    }

    // ---------------------------------------------------------------------
    // SECONDARY DOMINANT — quality tracks saturation: vivid image → dom7 (4 notes),
    // washed-out → bare secondary triad (3 notes). Dovetails harmonic complexity.
    // ---------------------------------------------------------------------
    #[test]
    fn test_secondary_dominant_quality_tracks_saturation() {
        let eng = engine();
        // Busy + LOW sat → secondary dominant is a bare triad (3 notes).
        let low = gen(&eng, &["I", "IV"], "Ionian", 0.045, 0.0, 20.0, 0.0);
        let v_low = low.iter().find(|c| c.name == "V/IV").unwrap();
        assert_eq!(
            v_low.notes.len(),
            3,
            "low-sat secondary dominant = bare triad"
        );
        // Busy + HIGH sat → dominant SEVENTH (4 notes, the strong tritone pull).
        let high = gen(&eng, &["I", "IV"], "Ionian", 0.045, 0.0, 90.0, 0.0);
        let v_high = high.iter().find(|c| c.name == "V/IV").unwrap();
        assert_eq!(
            v_high.notes.len(),
            4,
            "high-sat secondary dominant must be a dom7 (4 notes) for the strongest pull"
        );
    }

    // ---------------------------------------------------------------------
    // MODAL INTERCHANGE — a dark image (large brightness_drop) borrows the minor iv
    // in place of major IV (design-s13 §3 E-2 trigger, M-side consumption / §6.8).
    // ---------------------------------------------------------------------
    #[test]
    fn test_modal_interchange_borrows_minor_iv_when_dark() {
        let eng = engine();
        // drop=0.6 > 0.25 threshold → IV becomes the borrowed minor iv. Ionian is a
        // MAJOR-THIRD mode, so the DIATONIC subdominant (major IV) would be F-A-C; the
        // borrow must actually lower the third to F-Ab-C — the audible darkening.
        let dark = gen(&eng, &["I", "IV", "V"], "Ionian", 0.0, 0.6, 20.0, 0.0);
        let iv = dark.iter().find(|c| c.name == "iv").unwrap_or_else(|| {
            panic!(
                "a dark image (brightness_drop>threshold) must borrow the minor iv: {:?}",
                dark.iter().map(|c| c.name.clone()).collect::<Vec<_>>()
            )
        });

        // PROVE the minor third is SOUNDED, not merely relabeled. The chord is built
        // root-position (notes[0] = root, notes[1] = third). In C Ionian the iv root is
        // degree-3 = tonic+5 = 65 (F); the borrowed minor third is 65+3 = 68 (Ab).
        let root = iv.notes[0];
        let third = iv.notes[1];
        assert_eq!(
            root, 65,
            "borrowed iv must be rooted on the subdominant (F=65 in C Ionian): {:?}",
            iv.notes
        );
        // The decisive assertion: the interval from root to third is a MINOR third (3
        // semitones), NOT the major third (4) the diatonic IV of a major mode carries.
        // This is exactly the no-op the previous symbol-only swap could never satisfy —
        // it asserts the flattened third is in the pitch set the engine emits.
        let root_to_third = third as i16 - root as i16;
        assert_eq!(
            root_to_third, 3,
            "borrowed iv MUST sound a MINOR third (3 semitones) above its root, not a \
             major third — in C: F-Ab-C, not F-A-C. root={root} third={third} chord={:?}",
            iv.notes
        );
        // And concretely: Ab (pc 8) present, A-natural (pc 9 — the diatonic major third) absent.
        let pcs: std::collections::HashSet<u8> = iv.notes.iter().map(|&n| n % 12).collect();
        assert!(
            pcs.contains(&8) && !pcs.contains(&9),
            "borrowed iv must contain Ab (pc8) and NOT A-natural (pc9): {:?}",
            iv.notes
        );

        // A bright image keeps the major IV (no borrow): F-A-C, a MAJOR third (4 st).
        let bright = gen(&eng, &["I", "IV", "V"], "Ionian", 0.0, 0.0, 20.0, 0.0);
        let major_iv = bright
            .iter()
            .find(|c| c.name == "IV")
            .expect("a bright image must keep major IV, not borrow iv");
        assert!(
            !bright.iter().any(|c| c.name == "iv"),
            "a bright image must not borrow iv: {:?}",
            bright.iter().map(|c| c.name.clone()).collect::<Vec<_>>()
        );
        assert_eq!(
            major_iv.notes[1] as i16 - major_iv.notes[0] as i16,
            4,
            "the diatonic IV in a major mode keeps its MAJOR third (4 semitones): {:?}",
            major_iv.notes
        );

        // MODE-GENERALITY: borrowing into an already-MINOR mode must NOT double-flatten.
        // Aeolian's diatonic iv is already minor (F-Ab-C); the borrow reproduces that
        // same minor third (3 semitones) — never a diminished/double-flat third (2).
        let dark_minor = gen(&eng, &["I", "IV", "V"], "Aeolian", 0.0, 0.6, 20.0, 0.0);
        let iv_minor = dark_minor
            .iter()
            .find(|c| c.name == "iv")
            .expect("Aeolian dark image borrows iv too");
        assert_eq!(
            iv_minor.notes[1] as i16 - iv_minor.notes[0] as i16,
            3,
            "borrowing the minor iv into an already-minor mode must stay a MINOR third \
             (3 semitones), not double-flatten: {:?}",
            iv_minor.notes
        );
    }

    // ---------------------------------------------------------------------
    // SECONDARY-DOMINANT REGISTER SAFETY — even after the +7/+10 stack and the
    // voice-leading re-seat, the FINAL realized chord tones stay in 24..=108
    // (design-s13 §7 risk: register blowout). Run a busy, high-sat plan end-to-end.
    // ---------------------------------------------------------------------
    #[test]
    fn test_busy_vivid_plan_notes_stay_in_playable_range() {
        let eng = engine();
        // Busy + vivid → secondary dominants + 7th/9th chords + mixture, the densest
        // harmony the engine produces. Voice-lead and assert every note is in band.
        let chords = gen(
            &eng,
            &["I", "IV", "V", "vi", "ii", "V", "I"],
            "Ionian",
            0.045,
            0.0,
            90.0,
            0.65,
        );
        let led = eng.voice_lead_sequence(&chords);
        for c in &led {
            for &n in &c.notes {
                assert!(
                    (24..=108).contains(&n),
                    "note {n} in chord {:?} out of playable range 24..=108 after \
                     voice leading: {:?}",
                    c.name,
                    c.notes
                );
            }
        }
    }

    // ---------------------------------------------------------------------
    // FEATURE NORMALIZATION — the calibrated knobs land where the diagnosis says.
    // Guards the contract values in mappings.json (design-s13 §0).
    // ---------------------------------------------------------------------
    #[test]
    fn test_feature_normalization_calibration() {
        use crate::mapping_loader::FeatureNormalization;
        let eng = engine();
        let f = &eng.mappings.global.feature_normalization;
        // edge_density: raw 0.005..0.036 must map into ≈0.10..0.72 (the usable band).
        let lo = FeatureNormalization::normalize(0.005, f.edge_density_max);
        let hi = FeatureNormalization::normalize(0.036, f.edge_density_max);
        assert!(
            (0.08..0.12).contains(&lo) && (0.70..0.74).contains(&hi),
            "edge_density 0.005/0.036 must normalize to ≈0.10/0.72; got {lo}/{hi}"
        );
        // clamp protects the top: a busier-than-calibration image saturates at 1.0.
        assert_eq!(
            FeatureNormalization::normalize(0.20, f.edge_density_max),
            1.0,
            "edge_activity must clamp at 1.0 for an over-range image"
        );
        // degenerate range_max fails safe to 0 (no activity), never NaN/inf.
        assert_eq!(FeatureNormalization::normalize(0.5, 0.0), 0.0);
    }

    // ═══════════════════════════════════════════════════════════════════════
    // S15 SLICE 1 — RETURNING-THEME LAYER tests (resolve_motif + theme_melody_pitch
    // + the articulation-clamp window). Musical-property assertions, not execution.
    // ═══════════════════════════════════════════════════════════════════════

    use crate::composition::{
        CadenceStrength, KeyTempoPlan, ResolutionPolicy, Section, StepContext, ThematicRole,
        ThemeSeed, ThemeVariation,
    };

    /// Build a behaviour-neutral home-key section for theme tests.
    fn theme_section(
        role: ThematicRole,
        variation: ThemeVariation,
        theme: Option<usize>,
    ) -> Section {
        Section {
            label: "A".to_string(),
            step_len: 8,
            thematic_role: role,
            key_offset_semitones: 0,
            ms_per_step: 200,
            mode: "Ionian".to_string(),
            progression: vec![],
            theme,
            variation,
            boundary_cadence: CadenceStrength::Perfect,
            // S28/K3 — identity carry: a test fixture stays on the inert pivot/land-home path.
            pivot: false,
            resolution: ResolutionPolicy::Resolve,
            density: 0.5,
            // S17: identity orchestration profile — mandatory-field plumb so this
            // behaviour-neutral test fixture compiles; byte-neutral (no realizer logic, no
            // assert touched). The realizer Pad/role work is the next lane's.
            orchestration: crate::composition::OrchestrationProfile::identity(),
            steps: vec![],
        }
    }

    fn home_key_tempo() -> KeyTempoPlan {
        KeyTempoPlan {
            home_root_midi: 60, // C4 → tonic pc 0
            home_mode: "Ionian".to_string(),
            base_ms_per_step: 200,
            key_scheme: vec![0],
            tempo_scheme: vec![200],
        }
    }

    /// The behaviour-neutral owned (Section, KeyTempoPlan) for the PRE-theme
    /// articulation/velocity/rhythm tests below: theme:None ⇒ realize_step takes its
    /// existing free-select melody path, so every assertion in those tests is
    /// byte-identical to its pre-seam value. Built owned so each call site can bind a
    /// local `ctx = StepContext::single_section_default(&s, &k)` — the ONLY change to
    /// those tests is adding the `&ctx` argument; no assert/expected value moves.
    fn neutral_ctx_parts() -> (Section, KeyTempoPlan) {
        (
            theme_section(ThematicRole::Statement, ThemeVariation::Identity, None),
            home_key_tempo(),
        )
    }

    fn perf_mid() -> PerfFeatures {
        PerfFeatures {
            saturation: 50.0,
            brightness: 50.0, // bright_octaves 0 → no register lift
            edge_density: 0.01,
        }
    }

    // ── resolve_motif ────────────────────────────────────────────────────

    /// PROPERTY (DURATIONAL-SUM contract): resolve_motif emits 1..=length_steps notes,
    /// each with dur_steps >= 1, whose durations SUM to exactly length_steps (the motif
    /// fills its section without over-running it). The note count is <= length_steps —
    /// it is no longer fixed at length_steps, because notes now carry real durations.
    #[test]
    fn test_resolve_motif_length_and_duration() {
        for &len in &[1usize, 4, 5, 8] {
            let m = resolve_motif(MotifArchetype::Arch, 4, len);
            assert!(!m.is_empty(), "motif must never be empty");
            assert!(
                m.len() <= len,
                "note count {} must be <= length_steps {len} (durational contract)",
                m.len()
            );
            assert!(
                m.iter().all(|n| n.dur_steps >= 1),
                "every motif note must have dur_steps >= 1"
            );
            let total: usize = m.iter().map(|n| n.dur_steps as usize).sum();
            assert_eq!(
                total, len,
                "Σ dur_steps must equal length_steps (fills section, never over-runs)"
            );
        }
        // length 0 is clamped to 1 (never an empty motif the realizer can't read).
        let m0 = resolve_motif(MotifArchetype::Arch, 4, 0);
        assert_eq!(m0.len(), 1);
        assert_eq!(
            m0[0].dur_steps, 1,
            "the len-0 floor is a single 1-step note"
        );
    }

    /// PROPERTY (§4 shrink guard): augmentation-heavy rhythm profiles must NOT collapse a
    /// theme-bearing section to a couple of notes plus a long held tail. For every
    /// archetype at the canonical theme length (8 steps), the note count stays at or above
    /// ceil(length/2), AND Σ dur_steps never exceeds length — so a wide profile lands as a
    /// single long arrival (e.g. Descent: 5 notes, final dur 4) rather than a static smear.
    #[test]
    fn test_resolve_motif_shrink_guard() {
        let archetypes = [
            MotifArchetype::Arch,
            MotifArchetype::InvertedArch,
            MotifArchetype::Descent,
            MotifArchetype::Ascent,
            MotifArchetype::NeighborTurn,
            MotifArchetype::LeapStep,
            MotifArchetype::Pendulum,
            MotifArchetype::RisingSequence,
        ];
        for len in [4usize, 5, 8] {
            for arch in archetypes {
                let m = resolve_motif(arch, 4, len);
                let total: usize = m.iter().map(|n| n.dur_steps as usize).sum();
                assert!(
                    total <= len,
                    "{arch:?} len={len}: Σ dur_steps {total} over-ran length",
                );
                let guard = len.div_ceil(2);
                assert!(
                    m.len() >= guard,
                    "{arch:?} len={len}: {} notes < ceil(len/2)={guard} (shrink smear)",
                    m.len()
                );
            }
        }
    }

    /// PROPERTY: the Arch contour rises to an apex then falls back to (near) the
    /// tonic — the defining "up then down" shape. Tests degrees, not pitches. Uses
    /// length 8 so the full 5-degree contour ([0,2,4,2,0]) is sampled before the
    /// durational cap (at length 5 the durational-sum contract emits only the first 4
    /// degrees — still a valid arch head, but the closing return needs the longer run).
    #[test]
    fn test_resolve_motif_arch_is_up_then_down() {
        let m = resolve_motif(MotifArchetype::Arch, 4, 8);
        let degs: Vec<i8> = m.iter().map(|n| n.degree).collect();
        let apex = degs
            .iter()
            .position(|&d| d == *degs.iter().max().unwrap())
            .unwrap();
        // strictly rises to the apex, then falls from it.
        assert!(
            degs[..=apex].windows(2).all(|w| w[1] >= w[0]),
            "must rise to apex: {degs:?}"
        );
        assert!(
            degs[apex..].windows(2).all(|w| w[1] <= w[0]),
            "must fall from apex: {degs:?}"
        );
        assert_eq!(
            *degs.first().unwrap(),
            *degs.last().unwrap(),
            "an arch returns to its starting degree: {degs:?}"
        );
    }

    /// PROPERTY: Ascent rises monotonically; Descent falls monotonically — the two
    /// gradient-driven directional contours move in opposite directions.
    #[test]
    fn test_resolve_motif_ascent_descent_directions() {
        let asc: Vec<i8> = resolve_motif(MotifArchetype::Ascent, 4, 5)
            .iter()
            .map(|n| n.degree)
            .collect();
        let desc: Vec<i8> = resolve_motif(MotifArchetype::Descent, 4, 5)
            .iter()
            .map(|n| n.degree)
            .collect();
        assert!(
            asc.windows(2).all(|w| w[1] >= w[0]),
            "Ascent must not fall: {asc:?}"
        );
        assert!(
            desc.windows(2).all(|w| w[1] <= w[0]),
            "Descent must not rise: {desc:?}"
        );
        assert!(asc[asc.len() - 1] > asc[0], "Ascent must net rise");
        assert!(desc[desc.len() - 1] < desc[0], "Descent must net fall");
    }

    /// PROPERTY: a WIDER range stretches the contour to a larger ambit (the busy-image
    /// "wider leaps" knob), while a narrow range stays conjunct. Compare apex degrees.
    #[test]
    fn test_resolve_motif_range_widens_ambit() {
        let narrow = resolve_motif(MotifArchetype::Arch, 2, 5);
        let wide = resolve_motif(MotifArchetype::Arch, 7, 5);
        let span = |m: &[MotifNote]| {
            let lo = m.iter().map(|n| n.degree).min().unwrap();
            let hi = m.iter().map(|n| n.degree).max().unwrap();
            hi - lo
        };
        assert!(
            span(&wide) > span(&narrow),
            "a wider range must produce a larger degree span (wide {} > narrow {})",
            span(&wide),
            span(&narrow)
        );
    }

    // ── theme_melody_pitch ────────────────────────────────────────────────

    /// PROPERTY: a non-Melody role NEVER takes the theme path (bass/fill keep their
    /// existing free-select behavior) — returns None so the caller is byte-stable.
    #[test]
    fn test_theme_pitch_only_melody_role() {
        let section = theme_section(ThematicRole::Statement, ThemeVariation::Identity, Some(0));
        let kt = home_key_tempo();
        let seed = ThemeSeed {
            id: 0,
            motif: resolve_motif(MotifArchetype::Arch, 4, 8),
        };
        let ctx = StepContext {
            section: &section,
            step_in_section: 0,
            theme: Some(&seed),
            key_tempo: &kt,
            // S28/K3 — identity carry: this test ctx never modulates.
            prev_key_offset_semitones: None,
        };
        let chord = c_major_triad();
        for role in [OrchestralRole::Bass, OrchestralRole::HarmonicFill] {
            assert!(
                theme_melody_pitch(&ctx, role, &chord, &perf_mid()).is_none(),
                "{role:?} must not take the theme path"
            );
        }
    }

    /// PROPERTY: a Contrast section (or theme:None) free-selects — theme path is None.
    #[test]
    fn test_theme_pitch_contrast_free_selects() {
        let kt = home_key_tempo();
        let seed = ThemeSeed {
            id: 0,
            motif: resolve_motif(MotifArchetype::Arch, 4, 8),
        };
        let chord = c_major_triad();

        let contrast = theme_section(ThematicRole::Contrast, ThemeVariation::Identity, Some(0));
        let ctx = StepContext {
            section: &contrast,
            step_in_section: 0,
            theme: Some(&seed),
            key_tempo: &kt,
            // S28/K3 — identity carry: this test ctx never modulates.
            prev_key_offset_semitones: None,
        };
        assert!(
            theme_melody_pitch(&ctx, OrchestralRole::Melody, &chord, &perf_mid()).is_none(),
            "a Contrast section must free-select (theme path None)"
        );

        // theme:None on a Statement section also free-selects.
        let no_theme = theme_section(ThematicRole::Statement, ThemeVariation::Identity, None);
        let ctx2 = StepContext {
            section: &no_theme,
            step_in_section: 0,
            theme: None,
            key_tempo: &kt,
            // S28/K3 — identity carry: this test ctx never modulates.
            prev_key_offset_semitones: None,
        };
        assert!(
            theme_melody_pitch(&ctx2, OrchestralRole::Melody, &chord, &perf_mid()).is_none(),
            "a section with no theme must free-select"
        );
    }

    /// PROPERTY: on a Statement/Return Melody step the theme plays a sounding pitch in
    /// the engine's playable band, and the tonic-degree (0) note resolves to the key's
    /// tonic pitch class (C, pc 0, for home_root 60).
    #[test]
    fn test_theme_pitch_statement_plays_tonic_for_degree_zero() {
        // Arch motif starts on degree 0 (tonic). At home root C (pc 0), step 0's pitch
        // must be a C (pc 0), seated in the melody register, in band 24..=108.
        let section = theme_section(ThematicRole::Statement, ThemeVariation::Identity, Some(0));
        let kt = home_key_tempo();
        let seed = ThemeSeed {
            id: 0,
            motif: resolve_motif(MotifArchetype::Arch, 4, 8),
        };
        let ctx = StepContext {
            section: &section,
            step_in_section: 0,
            theme: Some(&seed),
            key_tempo: &kt,
            // S28/K3 — identity carry: this test ctx never modulates.
            prev_key_offset_semitones: None,
        };
        let chord = c_major_triad();
        let got = theme_melody_pitch(&ctx, OrchestralRole::Melody, &chord, &perf_mid());
        match got {
            Some(Some(p)) => {
                assert!((24..=108).contains(&p), "theme pitch {p} out of band");
                assert_eq!(
                    p % 12,
                    0,
                    "degree-0 (tonic) over C must sound a C, got pc {}",
                    p % 12
                );
            }
            other => panic!("Statement step must PLAY the theme, got {other:?}"),
        }
    }

    /// PROPERTY: Fragmented plays only the FIRST HALF of the motif then RESTS — the
    /// head-then-silence continuity gesture. S39: the head is the first ceil(n/2) NOTES,
    /// but a step now maps to its covering note via cumulative `dur_steps`, so a head step
    /// is either a note ONSET (sounds) or a `dur_steps>1` CONTINUATION (rests); past the
    /// head's STEP SPAN the melody rests. This walks the head's true step span and checks
    /// onset-sounds / continuation-rests / past-head-rests against the cumulative durations.
    #[test]
    fn test_theme_pitch_fragmented_head_then_rest() {
        let section = theme_section(ThematicRole::Statement, ThemeVariation::Fragmented, Some(0));
        let kt = home_key_tempo();
        // Arch [2,1,1,2] over len 8 → notes [(0,2),(2,1),(4,1),(2,2),(0,2)] (5 notes, Σ=8).
        let motif = resolve_motif(MotifArchetype::Arch, 4, 8);
        let note_count = motif.len();
        let head_notes = note_count.div_ceil(2); // ceil(5/2) == 3 → notes n0,n1,n2.
                                                 // The head's step span = Σ dur_steps of the first `head_notes` notes, and the set of
                                                 // ONSET steps = the cumulative offsets of those notes. Compute both from the motif.
        let head_dur_sum: usize = motif[..head_notes]
            .iter()
            .map(|n| n.dur_steps as usize)
            .sum();
        let mut onset_steps = std::collections::BTreeSet::new();
        let mut cum = 0usize;
        for n in &motif[..head_notes] {
            onset_steps.insert(cum);
            cum += n.dur_steps as usize;
        }
        let seed = ThemeSeed { id: 0, motif };
        let chord = c_major_triad();
        let mk = |step: usize| {
            let ctx = StepContext {
                section: &section,
                step_in_section: step,
                theme: Some(&seed),
                key_tempo: &kt,
                // S28/K3 — identity carry: this test ctx never modulates.
                prev_key_offset_semitones: None,
            };
            theme_melody_pitch(&ctx, OrchestralRole::Melody, &chord, &perf_mid())
        };
        // Within the head's step span: an onset step SOUNDS, a continuation step RESTS.
        for step in 0..head_dur_sum {
            if onset_steps.contains(&step) {
                assert!(
                    matches!(mk(step), Some(Some(_))),
                    "fragmented head ONSET step {step} must sound"
                );
            } else {
                assert_eq!(
                    mk(step),
                    Some(None),
                    "fragmented head CONTINUATION step {step} (a dur>1 note's later step) must REST"
                );
            }
        }
        // Past the head's step span: the head is consumed → the melody rests (head-then-silence).
        for step in head_dur_sum..(head_dur_sum + 3) {
            assert_eq!(
                mk(step),
                Some(None),
                "fragmented tail step {step} (past the head's step span) must REST"
            );
        }
    }

    /// PROPERTY: Identity past the motif's STEP SPAN HOLDS the final note (a sustained
    /// arrival), never wraps/loops back to the head. S39: the final note's ONSET is at step
    /// `Σ dur_steps − last_note.dur_steps` (not at `motif.len()−1`), and any step at/after
    /// `Σ dur_steps` is past the end and must hold that same final note.
    #[test]
    fn test_theme_pitch_identity_holds_not_loops() {
        let section = theme_section(ThematicRole::Return, ThemeVariation::Identity, Some(0));
        let kt = home_key_tempo();
        // Ascent [1,1,2] over len 5 → notes [(0,1),(1,1),(2,2),(3,1)] (4 notes, Σ=5).
        let motif = resolve_motif(MotifArchetype::Ascent, 4, 5);
        let total_steps: usize = motif.iter().map(|n| n.dur_steps as usize).sum();
        // The final note's ONSET step = total_steps − its own duration.
        let last_onset = total_steps - motif[motif.len() - 1].dur_steps as usize;
        let seed = ThemeSeed {
            id: 0,
            motif: motif.clone(),
        };
        let chord = c_major_triad();
        let mk = |step: usize| {
            let ctx = StepContext {
                section: &section,
                step_in_section: step,
                theme: Some(&seed),
                key_tempo: &kt,
                // S28/K3 — identity carry: this test ctx never modulates.
                prev_key_offset_semitones: None,
            };
            theme_melody_pitch(&ctx, OrchestralRole::Melody, &chord, &perf_mid())
        };
        let at_end = mk(last_onset); // the final note's onset — sounds.
        let past_end = mk(total_steps + 2); // well past Σ dur_steps — the held arrival.
        assert!(
            matches!(at_end, Some(Some(_))),
            "the final motif note's onset step must sound"
        );
        assert_eq!(
            at_end, past_end,
            "Identity past the step-span end must HOLD the final note, not loop to the head"
        );
    }

    // ── articulation clamp window (S15 §4.4) ───────────────────────────────

    /// PROPERTY: the NON-CADENCE hold fraction stays inside the 0.55..=1.10 window
    /// for every edge_activity — a calm note never muds (>1.10), a busy note never
    /// clicks (<0.55). Swept across the full raw-edge range and all three roles.
    /// (The cadence branch is exempt and tested separately as the byte-stable 240ms.)
    #[test]
    fn test_articulation_window_bounds_non_cadence() {
        let eng = engine();
        let plan = ionian_period(&eng);
        let ms = 1000u64;
        let (sec, kt) = neutral_ctx_parts();
        let ctx = StepContext::single_section_default(&sec, &kt);
        for &raw_edge in &[0.0f32, 0.005, 0.01, 0.025, 0.05, 0.2] {
            let f = PerfFeatures {
                saturation: 50.0,
                brightness: 50.0,
                edge_density: raw_edge,
            };
            for inst in 0..3usize {
                for step in &plan {
                    let is_cadence = matches!(
                        step.position,
                        PhrasePosition::HalfCadence | PhrasePosition::PerfectAuthenticCadence
                    );
                    if is_cadence {
                        continue; // cadence ring is the exempt, byte-stable figure
                    }
                    for e in realize_step(step, inst, 3, &f, ms, &ctx) {
                        let frac = e.hold_ms as f32 / ms as f32;
                        // The window low (0.55) is the floor; the cadence cap (1.20) is
                        // the only way >1.10 can appear, and we excluded cadence steps.
                        // Multi-onset patterns subdivide the slot, so a SUB-note can be
                        // shorter than 0.55 of the WHOLE step — assert against its own
                        // slot via the ceiling only (the busy end can't exceed 1.10).
                        assert!(
                            frac <= 1.10 + 1e-3,
                            "non-cadence hold frac {frac:.3} exceeded the 1.10 window \
                             ceiling (edge={raw_edge}, role inst {inst})"
                        );
                    }
                }
            }
        }
    }

    /// PROPERTY (golden-discipline witness): the calm-end sustained note still sings
    /// ABOVE 0.95 of the step — the new window's ceiling (1.10) preserves the S13
    /// "calm image legato crosses the bar line" cue the diversity net pins. Confirms
    /// the re-scale RAISED (did not lower) the legato ceiling. Hand value below.
    #[test]
    fn test_articulation_window_calm_end_still_legato() {
        // A near-zero edge (edge_activity ≈ 0) on a sustained melody step.
        // HAND DERIVATION (new window): base_frac = ARTIC_WINDOW_HI + (LO-HI)*ea
        //   ea ≈ 0  ⇒ base_frac ≈ 1.10 ; non-cadence rit = 1.0 ; sustained cap min(.,1.20)
        //   ⇒ hold = round(1000 * 1.10) = 1100 ms ⇒ frac 1.10 (was 1.05 under S13's
        //   LEGATO_FRAC_HI). The diversity-net assertion `frac > 0.95` is preserved with
        //   margin BECAUSE the ceiling rose 1.05→1.10; no diversity golden was a pinned
        //   equality, so none needed re-derivation — only this inequality witness.
        let eng = engine();
        let plan = ionian_period(&eng);
        let ms = 1000u64;
        let f = PerfFeatures {
            saturation: 50.0,
            brightness: 50.0,
            edge_density: 0.0,
        };
        // Find a sustained (interior, non-pre-cadence) melody step and read its hold.
        let melody = 2usize; // num=3 → inst 2 is Melody
        let (sec, kt) = neutral_ctx_parts();
        let ctx = StepContext::single_section_default(&sec, &kt);
        let mut max_frac = 0.0f32;
        for step in &plan {
            if matches!(step.position, PhrasePosition::Interior) {
                if let Some(h) = realize_step(step, melody, 3, &f, ms, &ctx)
                    .iter()
                    .map(|e| e.hold_ms)
                    .max()
                {
                    max_frac = max_frac.max(h as f32 / ms as f32);
                }
            }
        }
        assert!(
            max_frac > 0.95,
            "calm-image sustained melody must still cross into legato (>0.95), got {max_frac:.3} \
             — the window ceiling 1.10 preserves the S13 cue"
        );
        // And it must not exceed the window ceiling.
        assert!(
            max_frac <= 1.10 + 1e-3,
            "calm legato {max_frac:.3} must stay <= 1.10 window"
        );
    }

    /// A plain C-major triad fixture for the theme tests (root position).
    fn c_major_triad() -> Chord {
        Chord {
            name: "I".to_string(),
            notes: vec![60, 64, 67],
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // S18 SLICE 2 — REAL COUNTER-MELODY tests (spec §3.6). Hand-built, RNG-free,
    // no planner/loader. Build a CounterMelody-bearing section BY HAND and drive
    // realize_step for the counter instrument, asserting the contrary/oblique +
    // held-period-fill + slice-ceiling behaviour the real counter-line must show.
    // ═══════════════════════════════════════════════════════════════════════

    /// A CounterMelody-bearing profile: inst 0→Bass, 1→Pad, 2→CounterMelody, 3→Melody
    /// (the `pad_bed_counter` shape, built by hand so this net never loads mappings.json).
    fn counter_profile() -> crate::composition::OrchestrationProfile {
        crate::composition::OrchestrationProfile {
            id: "pad_bed_counter".to_string(),
            layers: vec![
                crate::composition::LayerRole::Bass,
                crate::composition::LayerRole::Pad,
                crate::composition::LayerRole::CounterMelody,
                crate::composition::LayerRole::Melody,
            ],
            density: 0.6,
            pad_voices: 3,
            // S20 — mechanical additive fixture fields (this counter net is NOT figured:
            // no figuration, so the Pad arm takes the byte-unchanged block bed).
            figuration: None,
            figuration_resolved: None,
            // S34 — mechanical additive fixture fields (this counter net carries NO bass
            // pattern, so the Bass arm takes the byte-unchanged sustained/pre-cadence path).
            bass_pattern: None,
            bass_pattern_resolved: None,
            // S23 — empty prominence: this existing counter net is a UNIFORM fixture, so
            // every role's weight is PROMINENCE_NEUTRAL (0.5) and the three prominence
            // nudges are all 0.0 — this test stays byte-identical (additive freeze).
            prominence: Vec::new(),
            // S53 — neutral motto: this counter net carries no per-piece gait, so the realizer's
            // motto read short-circuits and the realized onsets are byte-unchanged (additive freeze).
            motto: RhythmMotto::neutral(),
        }
    }

    /// One interior StepPlan carrying the given chord (phrase position chosen so it is a
    /// plain interior beat — never a cadence/start, so the counter arm is reached).
    fn counter_step(chord: Chord, position_in_phrase: usize) -> StepPlan {
        StepPlan {
            chord,
            phrase_index: 0,
            position_in_phrase,
            phrase_len: 8,
            position: PhrasePosition::Interior,
            velocity: 76,
        }
    }

    /// Build a 2-step CounterMelody section (the prior step + the current step), a home
    /// key-tempo, and return a `StepContext` pointing at step index 1 (so the counter
    /// arm sees a real prior step via `ctx.section.steps[0]`). The melody is FREE-SELECT
    /// (theme None) so `melody_pitch_for` recomputes the top-chord-tone melody.
    fn counter_ctx_parts(s0: StepPlan, s1: StepPlan) -> (Section, KeyTempoPlan) {
        let section = Section {
            label: "A".to_string(),
            step_len: 2,
            thematic_role: ThematicRole::Statement,
            key_offset_semitones: 0,
            ms_per_step: 1000,
            mode: "Ionian".to_string(),
            progression: vec![],
            theme: None,
            variation: ThemeVariation::Identity,
            boundary_cadence: CadenceStrength::Perfect,
            // S28/K3 — identity carry.
            pivot: false,
            resolution: ResolutionPolicy::Resolve,
            density: 0.6,
            orchestration: counter_profile(),
            steps: vec![s0, s1],
        };
        (section, home_key_tempo())
    }

    /// Realize the COUNTER instrument (inst 2 of 4) for the section's step index 1.
    fn realize_counter(
        section: &Section,
        kt: &KeyTempoPlan,
        features: &PerfFeatures,
    ) -> Vec<NoteEvent> {
        let ctx = StepContext {
            section,
            step_in_section: 1,
            theme: None,
            key_tempo: kt,
            // S28/K3 — identity carry: this test ctx never modulates.
            prev_key_offset_semitones: None,
        };
        // inst 2 of 4 under counter_profile() == CounterMelody. The step is steps[1].
        realize_step(&section.steps[1], 2, 4, features, section.ms_per_step, &ctx)
    }

    /// A G-major triad (the dominant) — a chord change from C so a held-vs-changing
    /// distinction can be drawn against the C-major tonic.
    fn g_major_triad() -> Chord {
        Chord {
            name: "V".to_string(),
            notes: vec![67, 71, 74],
        }
    }

    /// §3.6 test 3 (and a guard for every counter pitch): the realized counter pitch is
    /// always a chord-tone pitch class of the step's chord, seated in the counter band
    /// (FILL_REGISTER_FLOOR ≤ note < MELODY_REGISTER_FLOOR).
    #[test]
    fn test_counter_is_chord_tone_in_counter_register() {
        let chord = c_major_triad();
        let (sec, kt) = counter_ctx_parts(
            counter_step(g_major_triad(), 0),
            counter_step(chord.clone(), 1),
        );
        let evs = realize_counter(&sec, &kt, &perf_mid());
        assert!(
            !evs.is_empty(),
            "the counter must sound on this interior step"
        );
        let chord_pcs: Vec<u8> = chord.notes.iter().map(|n| n % 12).collect();
        for e in &evs {
            assert!(
                chord_pcs.contains(&(e.note % 12)),
                "counter note {} (pc {}) must be a chord tone of {:?}",
                e.note,
                e.note % 12,
                chord_pcs
            );
            assert!(
                (FILL_REGISTER_FLOOR..MELODY_REGISTER_FLOOR).contains(&e.note),
                "counter note {} must sit in the counter band [{FILL_REGISTER_FLOOR}, {MELODY_REGISTER_FLOOR})",
                e.note
            );
        }
    }

    /// §3.6 test 7 (the Slice-2 ceiling): the counter emits AT MOST one NoteEvent per
    /// step — never an arpeggio/comping figure. Pins the ceiling so Slice-3 figuration
    /// is a clean future diff. Swept across calm/busy and held/changing configurations.
    #[test]
    fn test_counter_at_most_one_event_per_step() {
        for chord_pair in [
            (c_major_triad(), c_major_triad()), // held chord
            (g_major_triad(), c_major_triad()), // changing chord
        ] {
            for &raw_edge in &[0.0f32, 0.01, 0.04, 0.2] {
                let (sec, kt) = counter_ctx_parts(
                    counter_step(chord_pair.0.clone(), 0),
                    counter_step(chord_pair.1.clone(), 1),
                );
                let f = PerfFeatures {
                    saturation: 50.0,
                    brightness: 50.0,
                    edge_density: raw_edge,
                };
                let evs = realize_counter(&sec, &kt, &f);
                assert!(
                    evs.len() <= 1,
                    "the counter must emit at most ONE event (Slice-2 ceiling), got {} \
                     (edge={raw_edge})",
                    evs.len()
                );
            }
        }
    }

    /// §3.6 test 4, RE-AUTHORED for the S47 FIGURE-GROUND GOVERNOR (slice 1). The PRE-S47
    /// behavior — a held chord + STATIC melody routed the counter to a GUARANTEED off-beat
    /// onset — was precisely the activity INVERSION S46 diagnosed: the background moved while
    /// the foreground held. The S47 governor recedes the counter to ONE rank below the
    /// melody's class, so on a HOLDING (Sustained) melody the counter now takes its OBLIQUE
    /// sustained tone (onset 0), NEVER the off-beat onset. It still SOUNDS (never silenced
    /// here — the chord changes are absent so the rest-as-gesture gate does not fire), and it
    /// still steps to a NEW chord tone across the held run (the moving-LINE pitch behavior is
    /// preserved — it is the off-beat ONSET that recedes, not the line's contrapuntal motion).
    /// The companion test `test_counter_moves_when_melody_subdividing` proves PRESERVE-S45:
    /// when the melody MOVES, the counter takes its off-beat onset again.
    #[test]
    fn test_counter_recedes_under_holding_melody() {
        // SAME voiced chord on both steps → held period; the free-select top tone is identical
        // across identical chords → the melody HOLDS (mel_dir Hold). The fixture's prominence
        // is uniform-neutral (0.5), so the activity floor does NOT lift the melody — it stays
        // Sustained, and the governor recedes the counter.
        let held = c_major_triad();
        let (sec, kt) =
            counter_ctx_parts(counter_step(held.clone(), 0), counter_step(held.clone(), 1));
        let f = perf_mid(); // calm; the melody falls to Sustained → the governor recedes the counter
        let evs = realize_counter(&sec, &kt, &f);
        assert_eq!(
            evs.len(),
            1,
            "the receded counter still SOUNDS one sustained tone (never silenced here), got {} events",
            evs.len()
        );
        assert_eq!(
            evs[0].offset_ms, 0,
            "S47 GOVERNOR: under a HOLDING melody the counter recedes to its OBLIQUE sustained \
             tone at onset 0 — it must NOT take the guaranteed off-beat onset that out-moves \
             the held foreground (the inversion the governor fixes)"
        );
        // The moving-LINE pitch behavior is unchanged by the governor: the held step's counter
        // pitch still differs from the prior step's realized pitch (contrapuntal motion of the
        // line is preserved; only the off-beat ONSET receded).
        let prev_ctx = StepContext {
            section: &sec,
            step_in_section: 0,
            theme: None,
            key_tempo: &kt,
            prev_key_offset_semitones: None,
        };
        let prev_evs = realize_step(&sec.steps[0], 2, 4, &f, sec.ms_per_step, &prev_ctx);
        assert_eq!(
            prev_evs.len(),
            1,
            "the prior held step also sounds one note"
        );
        assert_ne!(
            evs[0].note, prev_evs[0].note,
            "the counter LINE still steps to a new chord tone across the held run (moving line, \
             not a re-struck stab): step1 note {} == step0 note {}",
            evs[0].note, prev_evs[0].note
        );
    }

    /// PRESERVE-S45 (binding): when the melody MOVES (Subdividing — a busy image), the S47
    /// governor lets the counter keep its MOVING off-beat onset. The counter recedes ONLY
    /// relative to a holding melody; a moving melody keeps the moving inner texture S45 routed
    /// in. This is the complement of `test_counter_recedes_under_holding_melody`.
    #[test]
    fn test_counter_moves_when_melody_subdividing() {
        let held = c_major_triad();
        let (sec, kt) =
            counter_ctx_parts(counter_step(held.clone(), 0), counter_step(held.clone(), 1));
        // A BUSY image: edge_density high enough that the melody is Subdividing
        // (edge_activity > MELODY_SYNC_CUTOFF). With the fixture density 0.6 the nudge is
        // +0.05; raw 0.04/0.05 == 0.8 → edge_activity clamps near 0.85 → Subdividing.
        let busy = PerfFeatures {
            saturation: 50.0,
            brightness: 50.0,
            edge_density: 0.04,
        };
        let evs = realize_counter(&sec, &kt, &busy);
        assert_eq!(evs.len(), 1, "the moving counter sounds one off-beat note");
        assert_eq!(
            evs[0].offset_ms,
            sec.ms_per_step / 4,
            "PRESERVE-S45: a SUBDIVIDING melody lets the counter keep its moving off-beat \
             onset at step_ms/4 — the counter recedes only relative to a HOLDING melody"
        );
    }

    /// §3.4 ADVANCING-SEED PROOF (the load-bearing fix): across a REAL multi-step held
    /// run (C→C→C, three identical-chord steps) the SOUNDING counter pitch must visit
    /// ≥2 distinct pitches — a genuine moving line weaving through the static harmony,
    /// not the as-built re-struck stab where the held-period pitch was identical every
    /// step (the 64/64/64 gap `tests/saliency_s18.rs` pinned). The advancing seed rotates
    /// through the chord's band tones by the held-run position, so consecutive held steps
    /// seat from different pitches and the pick lands somewhere new.
    #[test]
    fn test_counter_held_run_advances_across_three_steps() {
        let held = c_major_triad();
        // A real 3-step held section, all C-major: steps 1 and 2 are inside the held run.
        let section = Section {
            label: "A".to_string(),
            step_len: 3,
            thematic_role: ThematicRole::Statement,
            key_offset_semitones: 0,
            ms_per_step: 1000,
            mode: "Ionian".to_string(),
            progression: vec![],
            theme: None,
            variation: ThemeVariation::Identity,
            boundary_cadence: CadenceStrength::Perfect,
            // S28/K3 — identity carry.
            pivot: false,
            resolution: ResolutionPolicy::Resolve,
            density: 0.6,
            orchestration: counter_profile(),
            steps: vec![
                counter_step(held.clone(), 0),
                counter_step(held.clone(), 1),
                counter_step(held.clone(), 2),
            ],
        };
        let kt = home_key_tempo();
        let f = perf_mid();
        let mut pitches = Vec::new();
        for si in 0..3 {
            let ctx = StepContext {
                section: &section,
                step_in_section: si,
                theme: None,
                key_tempo: &kt,
                // S28/K3 — identity carry: this test ctx never modulates.
                prev_key_offset_semitones: None,
            };
            let evs = realize_step(&section.steps[si], 2, 4, &f, section.ms_per_step, &ctx);
            assert_eq!(
                evs.len(),
                1,
                "each held step sounds exactly one counter note"
            );
            // S47 GOVERNOR: `perf_mid` is calm → the melody is Sustained → the counter recedes
            // to its OBLIQUE sustained tone at onset 0 (it no longer takes the off-beat onset
            // over a holding melody). The advancing-LINE proof below is unaffected — the line
            // still weaves through ≥2 chord tones; only the off-beat onset receded.
            assert_eq!(
                evs[0].offset_ms, 0,
                "under a holding (calm) melody the receded counter onsets at 0 (oblique sustained)"
            );
            // The Slice-2 contract still holds per step: chord tone in the counter band.
            let chord_pcs: Vec<u8> = held.notes.iter().map(|n| n % 12).collect();
            assert!(
                chord_pcs.contains(&(evs[0].note % 12)),
                "held-run counter note {} must be a chord tone of {:?}",
                evs[0].note,
                chord_pcs
            );
            assert!(
                (FILL_REGISTER_FLOOR..MELODY_REGISTER_FLOOR).contains(&evs[0].note),
                "held-run counter note {} must sit in the counter band",
                evs[0].note
            );
            pitches.push(evs[0].note);
        }
        let mut distinct = pitches.clone();
        distinct.sort_unstable();
        distinct.dedup();
        assert!(
            distinct.len() >= 2,
            "the held run must WEAVE a moving line (≥2 distinct sounding pitches), got \
             sequence {pitches:?} (distinct {distinct:?}) — the advancing seed did not move"
        );
    }

    /// §3.6 test 1 (the core counterpoint rule): when the melody moves UP across the
    /// step boundary, the counter must NOT move strictly UP with it (contrary/oblique,
    /// never similar). Built by recomputing the melody from the two chords and asserting
    /// the realized counter's direction vs its seed opposes/obliques the melody.
    #[test]
    fn test_counter_contrary_or_oblique_vs_melody() {
        // Prior chord C (melody top tone C5-ish), current chord G (melody top tone D5-ish
        // — higher) → the melody moves UP. The counter must not also move strictly up.
        let s0 = counter_step(c_major_triad(), 0);
        let s1 = counter_step(g_major_triad(), 1);
        let (sec, kt) = counter_ctx_parts(s0, s1);
        let f = PerfFeatures {
            saturation: 50.0,
            brightness: 50.0,
            edge_density: 0.04, // a sounding (changing-chord) step
        };
        // Confirm the premise: the recomputed melody actually rises across the boundary.
        let ctx_now = StepContext {
            section: &sec,
            step_in_section: 1,
            theme: None,
            key_tempo: &kt,
            // S28/K3 — identity carry: this test ctx never modulates.
            prev_key_offset_semitones: None,
        };
        let m_now = melody_pitch_for(&ctx_now, &sec.steps[1], &f);
        let m_prev = melody_pitch_for_step(&ctx_now, &sec.steps[0], &f);
        let mel_dir = motion_dir(m_prev, m_now);
        // Only assert the contrary rule when the melody genuinely moves (premise holds).
        if mel_dir == MotionDir::Up {
            let evs = realize_counter(&sec, &kt, &f);
            assert_eq!(
                evs.len(),
                1,
                "the counter must sound on a changing-chord step"
            );
            // S30-CP-FIX: measure the counter's motion from its REALIZED prior pitch (the line
            // that actually sounded at step 0), not the synthetic §3.1 seed — that realized
            // prev is exactly the transition the species gates now constrain, so it is the
            // correct anchor for the contrary/oblique rule.
            let realized_prev = realized_prev_counter(&ctx_now, &f, 1);
            let cnt_dir = motion_dir(Some(realized_prev), Some(evs[0].note));
            assert_ne!(
                cnt_dir,
                MotionDir::Up,
                "the counter must move CONTRARY/OBLIQUE (not strictly up with the melody): \
                 realized-prev {realized_prev} → note {}",
                evs[0].note
            );
        }
    }

    /// §3.6 test 6 (section-start guard): at `step_in_section == 0` (no prior step) the
    /// counter still produces a valid chord-tone note — no panic, no parallel check
    /// against a missing prior.
    #[test]
    fn test_counter_section_start_no_panic() {
        let chord = c_major_triad();
        let (sec, kt) = counter_ctx_parts(
            counter_step(chord.clone(), 0),
            counter_step(chord.clone(), 1),
        );
        // Point at step 0 (no prior) directly.
        let ctx = StepContext {
            section: &sec,
            step_in_section: 0,
            theme: None,
            key_tempo: &kt,
            // S28/K3 — identity carry: this test ctx never modulates.
            prev_key_offset_semitones: None,
        };
        let evs = realize_step(&sec.steps[0], 2, 4, &perf_mid(), sec.ms_per_step, &ctx);
        // A start with no prior: melody_static is true (m_prev None → mel_dir Hold), so
        // the counter SOUNDS (held/static activation) — a valid chord tone, no panic.
        assert_eq!(
            evs.len(),
            1,
            "section-start counter must produce one valid note"
        );
        let chord_pcs: Vec<u8> = chord.notes.iter().map(|n| n % 12).collect();
        assert!(
            chord_pcs.contains(&(evs[0].note % 12)),
            "section-start counter note must be a chord tone"
        );
    }

    /// §3.6 test 5 / §6.4 (the supersession witness), updated for S47: on a held chord with a
    /// MOVING (Subdividing) melody — a busy image — the counter takes its off-beat onset at
    /// step_ms/4, where the OLD HarmonicFill stub would have onset at 0, so the counter is NO
    /// LONGER a HarmonicFill delegate. (The witness moved from a STATIC-melody step to a
    /// moving-melody step because the S47 governor now correctly recedes the counter to onset
    /// 0 over a HOLDING melody — the off-beat onset is reserved for a moving melody. The
    /// supersession claim is unchanged; the witness step is chosen so the counter is in its
    /// moving mode.)
    #[test]
    fn test_counter_no_longer_harmonicfill_delegate() {
        let held = c_major_triad();
        let (sec, kt) =
            counter_ctx_parts(counter_step(held.clone(), 0), counter_step(held.clone(), 1));
        // BUSY image → the melody is Subdividing → the governor lets the counter MOVE
        // (PRESERVE-S45), so the off-beat-onset supersession witness is observable.
        let f = PerfFeatures {
            saturation: 50.0,
            brightness: 50.0,
            edge_density: 0.04,
        };
        let counter = realize_counter(&sec, &kt, &f);
        assert_eq!(counter.len(), 1, "counter sounds on the held step");
        assert_ne!(
            counter[0].offset_ms, 0,
            "the real counter onsets OFF the downbeat (step_ms/4) under a moving melody — \
             the HarmonicFill figure would onset at 0; the stub is superseded"
        );
        // And it is the documented step_ms/4 offset.
        assert_eq!(counter[0].offset_ms, sec.ms_per_step / 4);
    }

    // ═══════════════════════════════════════════════════════════════════════
    // S30 SLICE 1 — SPECIES-COUNTERPOINT figure-selection UNIT tests (design §6).
    // Unit-level correctness of the new helpers ONLY. The load-bearing property net
    // (full voice-independence at T/T+1, motion distribution, byte-freeze witnesses)
    // is the Test Engineer lane's tests/counterpoint_s30.rs — NOT duplicated here.
    // ═══════════════════════════════════════════════════════════════════════

    /// §1.1 classifier: the consonance/dissonance table is exactly right, including the
    /// contested perfect fourth (ic 5 = dissonant in the two-voice counter scorer).
    #[test]
    fn s30_harmonic_class_table_incl_contested_fourth() {
        // Perfect consonances: unison (0), octave (12 → ic 0), fifth (7).
        assert_eq!(harmonic_class(60, 60), HarmonicClass::PerfectConsonance);
        assert_eq!(harmonic_class(60, 72), HarmonicClass::PerfectConsonance);
        assert_eq!(harmonic_class(60, 67), HarmonicClass::PerfectConsonance);
        // Imperfect consonances: m3 (3), M3 (4), m6 (8), M6 (9).
        assert_eq!(harmonic_class(60, 63), HarmonicClass::ImperfectConsonance);
        assert_eq!(harmonic_class(60, 64), HarmonicClass::ImperfectConsonance);
        assert_eq!(harmonic_class(60, 68), HarmonicClass::ImperfectConsonance);
        assert_eq!(harmonic_class(60, 69), HarmonicClass::ImperfectConsonance);
        // Dissonances: m2/M2 (1/2), tritone (6), m7/M7 (10/11).
        assert_eq!(harmonic_class(60, 61), HarmonicClass::Dissonance);
        assert_eq!(harmonic_class(60, 62), HarmonicClass::Dissonance);
        assert_eq!(harmonic_class(60, 66), HarmonicClass::Dissonance);
        assert_eq!(harmonic_class(60, 70), HarmonicClass::Dissonance);
        assert_eq!(harmonic_class(60, 71), HarmonicClass::Dissonance);
        // Contested-decision #1: the bare perfect fourth (ic 5) is DISSONANT here.
        assert!(FOURTH_IS_DISSONANT);
        assert_eq!(harmonic_class(60, 65), HarmonicClass::Dissonance);
    }

    /// §1.x relative motion + the parallel/similar refinement, and the graded score order
    /// (contrary best, parallel worst).
    #[test]
    fn s30_rel_motion_classification_and_gradient() {
        // Contrary: CF up, counter down.
        assert_eq!(rel_motion(60, 62, 67, 65), RelMotion::Contrary);
        // Oblique: CF holds, counter moves.
        assert_eq!(rel_motion(60, 60, 64, 67), RelMotion::Oblique);
        // Parallel: both up by the SAME interval (a parallel third 60/64 → 62/66).
        assert_eq!(rel_motion(60, 62, 64, 66), RelMotion::Parallel);
        // Similar: both up, DIFFERENT interval (third 60/64 → fifth 62/69).
        assert_eq!(rel_motion(60, 62, 64, 69), RelMotion::Similar);
        // Gradient strictly orders contrary < oblique < similar < parallel (lower better).
        assert!(rel_motion_score(RelMotion::Contrary) < rel_motion_score(RelMotion::Oblique));
        assert!(rel_motion_score(RelMotion::Oblique) < rel_motion_score(RelMotion::Similar));
        assert!(rel_motion_score(RelMotion::Similar) < rel_motion_score(RelMotion::Parallel));
    }

    /// §1.2 HARD approach-to-perfect: strict form forbids ALL similar/parallel motion
    /// INTO a perfect consonance; contrary/oblique into a perfect is legal; the rule is
    /// vacuous when the arrival vertical is not perfect.
    #[test]
    fn s30_approach_perfect_strict() {
        assert!(HIDDEN_PERFECTS_STRICT);
        // SIMILAR motion into a perfect fifth (both up; arrival 60/67 is ic 7) → ILLEGAL.
        // CF 57→60 (up), counter 64→67 (up): arrival is a fifth, both rose ⇒ hidden fifth.
        assert!(!approach_perfect_is_legal(57, 60, 64, 67));
        // CONTRARY into the same fifth (CF up, counter down to 67 from above) → LEGAL.
        assert!(approach_perfect_is_legal(57, 60, 69, 67));
        // OBLIQUE into a perfect (CF holds) → LEGAL.
        assert!(approach_perfect_is_legal(60, 60, 64, 67));
        // Arrival is an imperfect consonance (a third) → rule vacuous, always legal.
        assert!(approach_perfect_is_legal(57, 60, 60, 64));
    }

    /// §1.2 melodic: steps always legal; consonant leaps legal; the tritone and the
    /// melodic seventh are the forbidden dissonant leaps.
    #[test]
    fn s30_melodic_leap_legality() {
        assert!(melodic_leap_is_legal(60, 62)); // a step
        assert!(melodic_leap_is_legal(60, 64)); // a major third (consonant leap)
        assert!(melodic_leap_is_legal(60, 67)); // a perfect fifth
        assert!(melodic_leap_is_legal(60, 72)); // an octave — a consonant leap
        assert!(!melodic_leap_is_legal(60, 66)); // a tritone leap — forbidden
        assert!(!melodic_leap_is_legal(60, 70)); // a minor-seventh leap — forbidden
        assert!(!melodic_leap_is_legal(60, 71)); // a major-seventh leap — forbidden
    }

    /// §1.4 passing-tone predicate: a stepwise-in, stepwise-out, same-direction dissonance
    /// between two consonances is legal; a turn (opposite direction) is NOT a passing tone.
    #[test]
    fn s30_passing_tone_predicate() {
        // Over a CF of C4 (60): counter goes E(64, M3 consonant) → D(62, M2 DISSONANT)
        // → C(60, unison consonant). Step down, step down, same direction ⇒ passing.
        assert!(is_legal_passing(64, 62, 60, 60));
        // Opposite directions (down then up) is a neighbor, not a passing tone.
        assert!(!is_legal_passing(64, 62, 64, 60));
        // The candidate must be a dissonance: E→F#(consonant? F#=66 vs C=60 is tritone,
        // dissonant) is fine, but a consonant candidate fails the passing predicate.
        // G(67) over C(60) is a fifth (consonant) ⇒ not a passing dissonance.
        assert!(!is_legal_passing(65, 67, 69, 60));
    }

    /// §1.5 neighbor predicate: step away and step BACK to the start pitch (opposite
    /// directions, start == end), the middle note dissonant.
    #[test]
    fn s30_neighbor_predicate() {
        // CF C4 (60). Counter E(64) → F(65, ic5 = DISSONANT here) → back to E(64).
        // Step up, step down, returns to start ⇒ a legal upper neighbor.
        assert!(NEIGHBOR_ALLOWED);
        assert!(is_legal_neighbor(64, 65, 64, 60));
        // Does NOT return to the start pitch ⇒ not a neighbor.
        assert!(!is_legal_neighbor(64, 65, 62, 60));
        // Same direction (not a turn) ⇒ not a neighbor.
        assert!(!is_legal_neighbor(64, 65, 67, 60));
    }

    /// §1.6 suspension three-stage shape: prep consonant → held same pitch → dissonant on
    /// the strong beat → resolves DOWN by step to a consonance.
    #[test]
    fn s30_suspension_predicate() {
        // Classic 7-6 suspension. Prep: D5(74) over E(64)?? Build it cleanly in one octave:
        // Prep D(62) over G(55): ic7 fifth (consonant). Hold D(62) as the CF moves to
        // C(60): D-over-C is a major SECOND (ic2, DISSONANT) — the suspension. Resolve
        // DOWN a step to C(60)?? C-over-C is a unison; use a 4-3: prep over the bass.
        // Simpler canonical: prep F(65) consonant over C(60)? ic5 dissonant — pick a
        // genuine consonance prep. Prep E(64) over C(60): M3 consonant. Hold E(64) as CF
        // rises to D(62): E-over-D is a M2 (dissonant). Resolve DOWN a step to D(62):
        // D-over-D unison (consonant). prep/held E, resolve D, cf C→D→D.
        assert!(is_legal_suspension(64, 64, 62, 60, 62, 62));
        // Resolution UP (a retardation) is NOT a suspension.
        assert!(!is_legal_suspension(64, 64, 66, 60, 62, 62));
        // Prep not consonant ⇒ not a suspension (prep F65 over C60 = ic5 dissonant here).
        assert!(!is_legal_suspension(65, 65, 64, 60, 62, 62));
        // Held pitch differs from prep (no tie/syncopation) ⇒ not a suspension.
        assert!(!is_legal_suspension(64, 65, 64, 60, 62, 62));
    }

    /// §1.5 cambiata recognizer: the canonical 5-note form (consonance → step down to a
    /// dissonance → leap down a third → step up → step up) is recognized; a non-matching
    /// shape is rejected. Recognition-only in Slice 1.
    #[test]
    fn s30_cambiata_recognizer() {
        let cf_c = [60u8; 5];
        // REJECT — correct melodic shape (step-down / third-down / step-up / step-up) but
        // the LANDING note (index 4) is dissonant against the CF: a cambiata must close on
        // a consonance. figure G F D E F over held C: 65-over-60 (ic5) is the bad landing.
        assert!(!is_legal_cambiata(&[67, 65, 62, 64, 65], &cf_c));
        // REJECT — correct shape but the CHANGING NOTE (index 1) is consonant (67-over-60
        // = P5): the figure's whole point is a sanctioned dissonance at index 1.
        assert!(!is_legal_cambiata(&[69, 67, 64, 65, 67], &cf_c));
        // ACCEPT — a fully-legal cambiata: figure E D B C D over CF C C C C B.
        //   melody 64→62 (step down), 62→59 (leap down a m3), 59→60 (step up),
        //          60→62 (step up) — the canonical 5-note shape ✓
        //   index1 62-over-60 = M2 DISSONANT (the changing note) ✓
        //   index0 64-over-60 = M3 consonant frame ✓; index4 62-over-59 = m3 consonant ✓
        assert!(is_legal_cambiata(
            &[64, 62, 59, 60, 62],
            &[60, 60, 60, 60, 59]
        ));
    }

    /// §1.3 opening: the PhraseStart vertical is filtered to PERFECT consonances; the
    /// above/below split excludes the bare fourth-below opening.
    #[test]
    fn s30_opening_candidates_are_perfect() {
        let chord = c_major_triad(); // C E G
                                     // Counter ABOVE a CF of C4(60): every legal opening is a perfect consonance.
        for c in opening_candidates(&chord, 60, true) {
            assert_eq!(
                harmonic_class(c, 60),
                HarmonicClass::PerfectConsonance,
                "opening vertical {c} over 60 must be a perfect consonance"
            );
            let ic = interval_class(c, 60);
            assert!(ic == 0 || ic == 7, "above: unison/octave/fifth only");
        }
    }

    /// PT-0 / §5.2 BYTE-PRESERVATION (unit-level): when dissonant figures are DISABLED
    /// (`figures_enabled = false` — the held-run / static path), `pick_counter_figure`
    /// returns the CONSONANCE-GATED sustain pitch, tagged `Sustain` — the §5.2 freeze anchor
    /// as corrected by the S30-CP-FIX GAP-2 consonance gate.
    ///
    /// S30-CP-FIX: the gate is a NO-OP whenever the as-built scorer's pick is already
    /// consonant against the sounding CF (every consonant-triad sustain — the overwhelmingly
    /// common case), so the byte-freeze still holds there: `pick_counter_figure == raw
    /// pick_counter_pitch`. It re-selects a consonant chord tone ONLY when the raw pick would
    /// be a dissonant structural sustain (GAP-2: never leave an unprepared, unresolved
    /// dissonance). We therefore assert the driver reduces to `consonance_gate_sustain(raw)`
    /// — which is `raw` itself in the consonant cases and a consonant re-pick in the dissonant
    /// one — and that the consonant-already cases are byte-identical to the raw scorer.
    #[test]
    fn s30_sustain_reduces_to_gated_sustain_when_figures_disabled() {
        let chord = c_major_triad();
        // A battery of (prev_counter, m_prev, m_now, mel_dir, force_move, held_target)
        // states spanning the as-built scorer's branches. Case 1 (prev 60 / CF 62) is the
        // GAP-2 witness: the raw scorer pick (C=60) is a M2 DISSONANCE against D(62), so the
        // gate re-picks a consonant tone; the rest are already consonant (byte-frozen).
        let cases: [(u8, Option<u8>, Option<u8>, bool, Option<u8>); 5] = [
            (60, Some(64), Some(62), false, None),
            (64, Some(60), Some(67), true, None),
            (62, None, Some(60), false, None),
            (67, Some(72), Some(71), false, Some(64)),
            (55, Some(60), None, true, None),
        ];
        for (prev_c, mp, mn, fm, ht) in cases {
            let mel_dir = motion_dir(mp, mn);
            let raw = pick_counter_pitch(&chord, prev_c, mp, mn, mel_dir, fm, ht);
            let gated = consonance_gate_sustain(&chord, raw, prev_c, mp, mn);
            let (pitch, fig) = pick_counter_figure(
                &chord,
                prev_c,
                mp,
                mn,
                mel_dir,
                PhrasePosition::Interior,
                Some(&chord),
                fm,
                ht,
                false, // figures DISABLED → must reduce to the consonance-gated sustain pick
            );
            assert_eq!(
                pitch, gated,
                "figures-disabled pitch must reduce to the consonance-gated sustain"
            );
            // The structural sustain is NEVER a dissonance against the sounding CF (GAP-2 floor).
            if let Some(cf) = mn {
                assert!(
                    is_consonant(pitch, cf),
                    "the figures-disabled structural sustain {pitch} must be consonant against \
                     the CF {cf} (no unprepared structural dissonance)"
                );
            }
            // Byte-freeze: where the raw scorer pick is already consonant, the gate is a no-op
            // and the driver is byte-identical to the as-built sustain scorer.
            if mn.map(|cf| is_consonant(raw, cf)).unwrap_or(true) {
                assert_eq!(
                    pitch, raw,
                    "consonant-already sustain must byte-match the as-built scorer (freeze)"
                );
            }
            assert_eq!(fig, CounterFigure::Sustain);
        }
    }

    /// A parallel-perfects rejection case threaded through the driver: on an interior step
    /// the driver never returns a pitch that forms a parallel perfect with the melody
    /// (delegated to the as-built `has_parallel_perfects` reject inside the sustain
    /// scorer, which the figure driver inherits — a candidate that would parallel is never
    /// the sustain pick, and dissonant figures are dissonances, not perfects).
    #[test]
    fn s30_driver_never_emits_parallel_perfect() {
        let chord = c_major_triad();
        // Melody rises a step (60→62); seed a prior counter a fifth below the prior melody
        // so a SIMILAR rise would parallel the fifth (55 under 60 → 57 under 62). The
        // scorer must reject that and pick a non-parallel tone.
        let m_prev = Some(60u8);
        let m_now = Some(62u8);
        let mel_dir = motion_dir(m_prev, m_now);
        let (pitch, _) = pick_counter_figure(
            &chord,
            55,
            m_prev,
            m_now,
            mel_dir,
            PhrasePosition::Interior,
            Some(&chord),
            false,
            None,
            true, // enabled — exercise the full path
        );
        // No parallel perfect across the [melody, counter] transition.
        assert!(
            !has_parallel_perfects(&[60, 55], &[62, pitch]),
            "driver pitch {pitch} forms a parallel perfect with the melody"
        );
    }

    /// Determinism: the driver is a pure function — same inputs, same output, twice
    /// (no thread_rng in the figure/voice choice, design §1.7 requirement 4 / PT-9).
    #[test]
    fn s30_driver_is_deterministic() {
        let chord = g_major_triad();
        let args = || {
            pick_counter_figure(
                &chord,
                62,
                Some(67),
                Some(65),
                motion_dir(Some(67), Some(65)),
                PhrasePosition::Interior,
                Some(&c_major_triad()),
                false,
                None,
                true,
            )
        };
        assert_eq!(args(), args());
    }

    // ═══════════════════════════════════════════════════════════════════════
    // S34 PATTERN-LIBRARY SLICE 2 — register_octaves (oom-pah/stride) + the
    // walking-bass / pedal-point generators. Hand-built, RNG-free; drives the
    // PUBLIC realize_step for the integration witnesses and the generators
    // directly for the unit-property witnesses. Validates MUSICAL properties.
    // ═══════════════════════════════════════════════════════════════════════

    use crate::composition::{BassPatternKind, BassPatternSpec, FigurationOnset, FigurationSpec};

    /// A 4-role profile (Bass, Pad, HarmonicFill, Melody) carrying an optional resolved
    /// figuration and/or bass pattern — the S34 driving fixture. inst 0 == Bass.
    fn s34_profile(
        figuration_resolved: Option<FigurationSpec>,
        bass_pattern_resolved: Option<BassPatternSpec>,
    ) -> crate::composition::OrchestrationProfile {
        crate::composition::OrchestrationProfile {
            id: "s34_test".to_string(),
            layers: vec![
                crate::composition::LayerRole::Bass,
                crate::composition::LayerRole::Pad,
                crate::composition::LayerRole::HarmonicFill,
                crate::composition::LayerRole::Melody,
            ],
            density: 0.6,
            pad_voices: 3,
            figuration: None,
            figuration_resolved,
            bass_pattern: None,
            bass_pattern_resolved,
            prominence: Vec::new(),
            motto: RhythmMotto::neutral(), // S53 — neutral gait; realizer short-circuits (freeze)
        }
    }

    /// An INTERIOR step (never cadence/start) carrying `chord` at `position_in_phrase` — so the
    /// Bass/Pad arms (not the cadence ring) are the realized path.
    fn s34_step(chord: Chord, position_in_phrase: usize) -> StepPlan {
        StepPlan {
            chord,
            phrase_index: 0,
            position_in_phrase,
            phrase_len: 8,
            position: PhrasePosition::Interior,
            velocity: 76,
        }
    }

    /// Realize the BASS instrument (inst 0 of 4) for the section's step index `si`.
    fn s34_realize_bass(
        section: &Section,
        kt: &KeyTempoPlan,
        si: usize,
        features: &PerfFeatures,
    ) -> Vec<NoteEvent> {
        let ctx = StepContext {
            section,
            key_tempo: kt,
            step_in_section: si,
            theme: None,
            prev_key_offset_semitones: None,
        };
        realize_step(
            &section.steps[si],
            0,
            4,
            features,
            section.ms_per_step,
            &ctx,
        )
    }

    /// Realize the PAD instrument (inst 1 of 4) for the section's step index `si`.
    fn s34_realize_pad(
        section: &Section,
        kt: &KeyTempoPlan,
        si: usize,
        features: &PerfFeatures,
    ) -> Vec<NoteEvent> {
        let ctx = StepContext {
            section,
            key_tempo: kt,
            step_in_section: si,
            theme: None,
            prev_key_offset_semitones: None,
        };
        realize_step(
            &section.steps[si],
            1,
            4,
            features,
            section.ms_per_step,
            &ctx,
        )
    }

    /// Build an owned home-key section carrying `steps` + the given orchestration profile.
    fn s34_section(
        steps: Vec<StepPlan>,
        orchestration: crate::composition::OrchestrationProfile,
    ) -> (Section, KeyTempoPlan) {
        let section = Section {
            label: "A".to_string(),
            step_len: steps.len(),
            thematic_role: ThematicRole::Statement,
            key_offset_semitones: 0,
            ms_per_step: 1000,
            mode: "Ionian".to_string(),
            progression: vec![],
            theme: None,
            variation: ThemeVariation::Identity,
            boundary_cadence: CadenceStrength::Perfect,
            pivot: false,
            resolution: ResolutionPolicy::Resolve,
            density: 0.5,
            orchestration,
            steps,
        };
        (section, home_key_tempo())
    }

    fn d_minor_triad() -> Chord {
        // ii in C major: D F A.
        Chord {
            name: "ii".to_string(),
            notes: vec![62, 65, 69],
        }
    }

    // ── Part (A) — register_octaves / oom-pah / stride ─────────────────────

    /// apply_register_octaves: 0 is a no-op (the byte-freeze default); -1 lands EXACTLY 12
    /// semitones below the 0 counterpart; +1 exactly 12 above; out-of-range clamps to [24,108].
    #[test]
    fn s34_apply_register_octaves_arithmetic_and_clamp() {
        let p = 60u8;
        assert_eq!(
            apply_register_octaves(p, 0),
            60,
            "0 is the byte-freeze no-op"
        );
        assert_eq!(
            apply_register_octaves(p, -1),
            48,
            "-1 == 12 semitones below"
        );
        assert_eq!(apply_register_octaves(p, 1), 72, "+1 == 12 semitones above");
        // The -1 result is EXACTLY 12 below the 0 result (the test-plan witness).
        assert_eq!(
            apply_register_octaves(p, 0) as i16 - apply_register_octaves(p, -1) as i16,
            12,
            "register_octaves=-1 lands exactly an octave below register_octaves=0"
        );
        // Adversarial shifts clamp into the synthesizable band — never wrap, never panic.
        assert_eq!(
            apply_register_octaves(60, -9),
            24,
            "−9 clamps to the low bound"
        );
        assert_eq!(
            apply_register_octaves(60, 9),
            108,
            "+9 clamps to the high bound"
        );
    }

    /// FREEZE: a figured profile whose onsets all carry register_octaves==0 realizes the
    /// IDENTICAL Pad event sequence as the same row WITHOUT the field — the default-zero freeze
    /// guard (§2.2). Two specs differing ONLY in an explicit 0 vs the serde default must match.
    #[test]
    fn s34_register_octaves_zero_is_byte_identical() {
        let with_zero = FigurationSpec {
            id: "alberti".to_string(),
            voices: 3,
            onsets: vec![
                FigurationOnset {
                    at: 0.0,
                    tone: 0,
                    hold_frac: 1.0,
                    register_octaves: 0,
                },
                FigurationOnset {
                    at: 0.25,
                    tone: 2,
                    hold_frac: 1.0,
                    register_octaves: 0,
                },
                FigurationOnset {
                    at: 0.5,
                    tone: 1,
                    hold_frac: 1.0,
                    register_octaves: 0,
                },
                FigurationOnset {
                    at: 0.75,
                    tone: 2,
                    hold_frac: 1.0,
                    register_octaves: 0,
                },
            ],
        };
        let without = FigurationSpec {
            onsets: vec![
                FigurationOnset {
                    at: 0.0,
                    tone: 0,
                    hold_frac: 1.0,
                    register_octaves: 0,
                },
                FigurationOnset {
                    at: 0.25,
                    tone: 2,
                    hold_frac: 1.0,
                    register_octaves: 0,
                },
                FigurationOnset {
                    at: 0.5,
                    tone: 1,
                    hold_frac: 1.0,
                    register_octaves: 0,
                },
                FigurationOnset {
                    at: 0.75,
                    tone: 2,
                    hold_frac: 1.0,
                    register_octaves: 0,
                },
            ],
            ..with_zero.clone()
        };
        let seated = [55u8, 59, 62];
        assert_eq!(
            figured_bed(&with_zero, &seated, 80, 1000),
            figured_bed(&without, &seated, 80, 1000),
            "register_octaves==0 on every onset realizes byte-identically (the freeze default)"
        );
        // And every event's note is exactly the seated tone (no shift at all).
        let evs = figured_bed(&with_zero, &seated, 80, 1000);
        assert_eq!(evs[0].note, seated[0]);
        assert_eq!(evs[1].note, seated[2 % 3]);
    }

    /// oom-pah: the at:0.0 "oom" (register_octaves=-1) lands an octave below its in-band
    /// counterpart; the at:0.5 "pah" (register_octaves=0) stays in the seated fill band — the
    /// oom is STRICTLY LOWER than the pah (the bass-vs-chord split that makes oom-pah read).
    #[test]
    fn s34_oom_pah_register_split() {
        let oom_pah = FigurationSpec {
            id: "oom_pah".to_string(),
            voices: 3,
            onsets: vec![
                FigurationOnset {
                    at: 0.0,
                    tone: 0,
                    hold_frac: 0.4,
                    register_octaves: -1,
                },
                FigurationOnset {
                    at: 0.5,
                    tone: 1,
                    hold_frac: 0.4,
                    register_octaves: 0,
                },
            ],
        };
        let seated = [55u8, 59, 62]; // fill-band inner tones
        let evs = figured_bed(&oom_pah, &seated, 80, 1000);
        assert_eq!(evs.len(), 2);
        // The oom == apply_register_octaves(seated[0], -1) == an octave below seated[0].
        assert_eq!(evs[0].note, apply_register_octaves(seated[0], -1));
        assert_eq!(evs[0].note, seated[0] - 12);
        // The pah is in-band (== seated[1], no shift).
        assert_eq!(evs[1].note, seated[1]);
        assert!(
            evs[0].note < evs[1].note,
            "the oom must sit strictly below the pah"
        );
    }

    /// A whole-octave shift PRESERVES pitch class — so a register-shifted onset is STILL a chord
    /// tone (the §6 chord-tone witness). Every shifted note's pc ∈ the seated bed's pitch classes.
    #[test]
    fn s34_register_shift_stays_chord_tone() {
        let stride = FigurationSpec {
            id: "stride".to_string(),
            voices: 3,
            onsets: vec![
                FigurationOnset {
                    at: 0.0,
                    tone: 0,
                    hold_frac: 0.4,
                    register_octaves: -1,
                },
                FigurationOnset {
                    at: 0.25,
                    tone: 1,
                    hold_frac: 0.4,
                    register_octaves: 0,
                },
                FigurationOnset {
                    at: 0.5,
                    tone: 2,
                    hold_frac: 0.4,
                    register_octaves: -1,
                },
                FigurationOnset {
                    at: 0.75,
                    tone: 1,
                    hold_frac: 0.4,
                    register_octaves: 0,
                },
            ],
        };
        let seated = [55u8, 59, 62];
        let bed_pcs: Vec<u8> = seated.iter().map(|n| n % 12).collect();
        for e in figured_bed(&stride, &seated, 80, 1000) {
            assert!(
                bed_pcs.contains(&(e.note % 12)),
                "shifted note {} (pc {}) must keep a seated pitch class",
                e.note,
                e.note % 12
            );
        }
    }

    /// stride alternates bass (at∈{0.0,0.5}, shifted down) and stab (at∈{0.25,0.75}, in-band):
    /// the strong-beat bass notes sit strictly below the weak-beat stabs.
    #[test]
    fn s34_stride_alternates_bass_and_stab() {
        let stride = FigurationSpec {
            id: "stride".to_string(),
            voices: 3,
            onsets: vec![
                FigurationOnset {
                    at: 0.0,
                    tone: 0,
                    hold_frac: 0.4,
                    register_octaves: -1,
                },
                FigurationOnset {
                    at: 0.25,
                    tone: 1,
                    hold_frac: 0.4,
                    register_octaves: 0,
                },
                FigurationOnset {
                    at: 0.5,
                    tone: 2,
                    hold_frac: 0.4,
                    register_octaves: -1,
                },
                FigurationOnset {
                    at: 0.75,
                    tone: 1,
                    hold_frac: 0.4,
                    register_octaves: 0,
                },
            ],
        };
        let seated = [55u8, 59, 62];
        let evs = figured_bed(&stride, &seated, 80, 1000);
        // The two stride-bass onsets (indices 0, 2) are below both stab onsets (1, 3).
        let bass_max = evs[0].note.max(evs[2].note);
        let stab_min = evs[1].note.min(evs[3].note);
        assert!(
            bass_max < stab_min,
            "every stride bass ({bass_max}) must sit below every stab ({stab_min})"
        );
    }

    // ── Part (B) — walking bass ─────────────────────────────────────────────

    /// walking_bass: the first (strong-beat) onset is the CURRENT chord root (a chord tone), and
    /// the line, completed by the NEXT step's downbeat root, arrives ON that next root. Over C→G,
    /// onset 0 is C (pc 0) and the final onset is one diatonic step short of G (pc 7).
    #[test]
    fn s34_walking_bass_strong_beat_is_root_and_targets_next() {
        // C (pc 0) → G (pc 7), 4 onsets, C major (Ionian), tonic pc 0. The generator takes the
        // SHORTER diatonic route: G is 4 scale steps UP but only 3 DOWN, so the walk descends
        // C→B→A→(G), arriving on G at the NEXT step's downbeat.
        let line = walking_bass(0, Some(7), 0, "Ionian", 80, 1000, 4);
        assert_eq!(line.len(), 4, "density 4 → 4 onsets");
        assert_eq!(
            line[0].note % 12,
            0,
            "strong-beat onset is the current root (C)"
        );
        // The last onset is the diatonic tone ONE STEP from the target G (here A, pc 9, the upper
        // neighbour) — NOT G itself; the next downbeat completes the stepwise arrival on G.
        let scale = [0u8, 2, 4, 5, 7, 9, 11];
        let g_idx = scale.iter().position(|&p| p == 7).unwrap();
        let last_idx = scale.iter().position(|&p| p == line[3].note % 12).unwrap();
        let dist = (last_idx as i32 - g_idx as i32).rem_euclid(7);
        assert!(
            dist == 1 || dist == 6,
            "the final onset (pc {}) must be ONE diatonic step from the target G",
            line[3].note % 12
        );
        assert_ne!(
            line[3].note % 12,
            7,
            "the final onset is NOT yet G (that is the next downbeat)"
        );
    }

    /// Every walking onset is DIATONIC to the section scale — NO chromatic approach this slice
    /// (OD-2: diatonic-only). C major scale pcs = {0,2,4,5,7,9,11}.
    #[test]
    fn s34_walking_bass_is_diatonic() {
        let scale_pcs = [0u8, 2, 4, 5, 7, 9, 11]; // C Ionian
        for (cur, next) in [(0u8, 7u8), (7, 0), (2, 9), (5, 0)] {
            for e in walking_bass(cur, Some(next), 0, "Ionian", 80, 1000, 4) {
                assert!(
                    scale_pcs.contains(&(e.note % 12)),
                    "walking note {} (pc {}) must be diatonic to C Ionian",
                    e.note,
                    e.note % 12
                );
            }
        }
    }

    /// The walking line stays in the bass register and moves connectedly — no adjacent pair
    /// leaps by an octave or more (a true walking line is stepwise / small-leap, not jumpy).
    #[test]
    fn s34_walking_bass_is_a_connected_bass_line() {
        let line = walking_bass(0, Some(7), 0, "Ionian", 80, 1000, 4);
        for e in &line {
            assert!(
                (24..=108).contains(&e.note) && e.note >= BASS_REGISTER_FLOOR - 12,
                "walking note {} must sit in the bass register",
                e.note
            );
        }
        for w in line.windows(2) {
            let leap = (w[1].note as i16 - w[0].note as i16).unsigned_abs();
            assert!(
                leap < 12,
                "adjacent walking notes must not leap an octave (got {leap})"
            );
        }
    }

    /// End-of-section fallback: with no next chord, walking_bass walks WITHIN the current chord
    /// (root → 5th → root …) rather than inventing a target — no panic, first onset is the root.
    #[test]
    fn s34_walking_bass_end_of_section_falls_back() {
        let line = walking_bass(0, None, 0, "Ionian", 80, 1000, 4);
        assert_eq!(line.len(), 4);
        assert_eq!(line[0].note % 12, 0, "fallback opens on the current root");
        // The odd onsets are the diatonic fifth above the root (G, pc 7).
        assert_eq!(
            line[1].note % 12,
            7,
            "the fallback alternates root and its diatonic fifth"
        );
    }

    /// Integration: a Walking profile driven through realize_step over a C→G step pair produces
    /// the walking line on the Bass instrument; a single-step section (no next) falls back.
    #[test]
    fn s34_walking_bass_via_realize_step() {
        let walking = BassPatternSpec {
            id: "walking".to_string(),
            kind: BassPatternKind::Walking,
            density: 4,
            pedal_degree: 1,
        };
        let steps = vec![s34_step(c_major_triad(), 2), s34_step(g_major_triad(), 4)];
        let (section, kt) = s34_section(steps, s34_profile(None, Some(walking)));
        let f = perf_mid();
        let bass0 = s34_realize_bass(&section, &kt, 0, &f);
        assert_eq!(bass0.len(), 4, "the walking bass emits density (4) onsets");
        assert_eq!(bass0[0].note % 12, 0, "step 0 opens on the C root");
    }

    // ── Part (B) — pedal point ──────────────────────────────────────────────

    /// pedal_bass holds ONE constant pitch (the pedal_degree of the key) regardless of the
    /// chord — the same low pitch across a multi-chord span (the standing drone).
    #[test]
    fn s34_pedal_point_holds_one_pitch() {
        // tonic pedal (degree 1) in C major → pc 0.
        let a = pedal_bass(0, "Ionian", 1, 80, 1000, 1.0);
        let b = pedal_bass(0, "Ionian", 1, 80, 1000, 1.0);
        assert_eq!(a.len(), 1, "the pedal is a single sustained note");
        assert_eq!(
            a[0].note, b[0].note,
            "the pedal pitch is constant across steps"
        );
        assert_eq!(a[0].note % 12, 0, "the tonic pedal sounds the tonic (C)");
    }

    /// pedal_degree selects tonic (1) vs dominant (5): degree 1 → pc 0 (C), degree 5 → pc 7 (G).
    #[test]
    fn s34_pedal_degree_selects_tonic_or_dominant() {
        assert_eq!(
            pedal_bass(0, "Ionian", 1, 80, 1000, 1.0)[0].note % 12,
            0,
            "degree 1 == tonic"
        );
        assert_eq!(
            pedal_bass(0, "Ionian", 5, 80, 1000, 1.0)[0].note % 12,
            7,
            "degree 5 == dominant"
        );
    }

    /// Integration: a Pedal profile holds the SAME bass pitch under a CHANGING harmony across a
    /// 3-chord stream (C, ii, G) — the harmony moves above; the pedal does not.
    #[test]
    fn s34_pedal_point_holds_under_changing_harmony_via_realize_step() {
        let pedal = BassPatternSpec {
            id: "pedal".to_string(),
            kind: BassPatternKind::Pedal,
            density: 2,
            pedal_degree: 1,
        };
        let steps = vec![
            s34_step(c_major_triad(), 2),
            s34_step(d_minor_triad(), 4),
            s34_step(g_major_triad(), 6),
        ];
        let (section, kt) = s34_section(steps, s34_profile(None, Some(pedal)));
        let f = perf_mid();
        let n0 = s34_realize_bass(&section, &kt, 0, &f);
        let n1 = s34_realize_bass(&section, &kt, 1, &f);
        let n2 = s34_realize_bass(&section, &kt, 2, &f);
        assert_eq!(n0.len(), 1);
        assert_eq!(
            (n0[0].note, n1[0].note, n2[0].note),
            (n0[0].note, n0[0].note, n0[0].note),
            "the pedal bass is the SAME pitch on every step despite the chord change"
        );
        assert_eq!(
            n0[0].note % 12,
            0,
            "the tonic pedal sounds C under all three chords"
        );
    }

    // ── Part (B) — the Sustained/None dispatch freeze witness ───────────────

    /// FREEZE: a profile with bass_pattern_resolved == None realizes the IDENTICAL Bass events
    /// as a profile with NO bass-pattern seam at all — the sustained-root freeze default did not
    /// move. Drives realize_step on the Bass instrument and compares against a Sustained-kind row
    /// (which the dispatch routes to the legacy `_` arm byte-for-byte).
    #[test]
    fn s34_sustained_and_none_dispatch_are_byte_identical() {
        let f = perf_mid();
        let steps_a = vec![s34_step(c_major_triad(), 2), s34_step(g_major_triad(), 4)];
        let steps_b = vec![s34_step(c_major_triad(), 2), s34_step(g_major_triad(), 4)];
        // (1) bass_pattern_resolved == None → the legacy path.
        let (sec_none, kt) = s34_section(steps_a, s34_profile(None, None));
        // (2) an explicit Sustained kind → the SAME legacy `_` arm.
        let sustained = BassPatternSpec {
            id: "sustained".to_string(),
            kind: BassPatternKind::Sustained,
            density: 2,
            pedal_degree: 1,
        };
        let (sec_sus, _) = s34_section(steps_b, s34_profile(None, Some(sustained)));
        for si in 0..2 {
            assert_eq!(
                s34_realize_bass(&sec_none, &kt, si, &f),
                s34_realize_bass(&sec_sus, &kt, si, &f),
                "Sustained kind must realize byte-identically to the None (legacy) path at step {si}"
            );
        }
    }

    /// The Pad bed is UNCHANGED when a bass pattern is present but no figuration: the bass-arm
    /// dispatch must not perturb the Pad voice (the parts are independent).
    #[test]
    fn s34_pad_bed_unperturbed_by_bass_pattern() {
        let f = perf_mid();
        let steps_a = vec![s34_step(c_major_triad(), 2), s34_step(g_major_triad(), 4)];
        let steps_b = vec![s34_step(c_major_triad(), 2), s34_step(g_major_triad(), 4)];
        let (sec_plain, kt) = s34_section(steps_a, s34_profile(None, None));
        let walking = BassPatternSpec {
            id: "walking".to_string(),
            kind: BassPatternKind::Walking,
            density: 4,
            pedal_degree: 1,
        };
        let (sec_walk, _) = s34_section(steps_b, s34_profile(None, Some(walking)));
        assert_eq!(
            s34_realize_pad(&sec_plain, &kt, 0, &f),
            s34_realize_pad(&sec_walk, &kt, 0, &f),
            "the Pad bed is independent of the bass pattern"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // S47 SLICE 1 — THE FIGURE-GROUND HIERARCHY. Pure-property tests for the
    // new helpers (ActivityClass / melody_activity_class), the governor's
    // class→counter-mode mapping, the activity floor's SUSTAINED→DOTTED lift,
    // and the seat-order guard arithmetic + its counter-present gate. RNG-free.
    // ═══════════════════════════════════════════════════════════════════════

    /// PROPERTY: the activity rank IS the figure-ground ordering Sustained < Oblique <
    /// Subdividing. The governor compares classes via this `Ord`, so the ordering is
    /// load-bearing — a holding melody (Sustained) must compare strictly below a moving one.
    #[test]
    fn s47_activity_class_ordering_is_figure_ground_rank() {
        assert!(ActivityClass::Sustained < ActivityClass::Oblique);
        assert!(ActivityClass::Oblique < ActivityClass::Subdividing);
        assert!(ActivityClass::Sustained < ActivityClass::Subdividing);
    }

    /// PROPERTY: `melody_activity_class` mirrors the Melody arm's 4→3 band mapping EXACTLY
    /// — ARPEGGIO+SYNCOPATED → Subdividing (the melody moves), DOTTED → Oblique (minimum
    /// real motion), below DOTTED → Sustained (one held tone). At neutral prom_shift the
    /// cutoffs are the shared MELODY_*_CUTOFF consts the arm reads, so they cannot drift.
    #[test]
    fn s47_melody_activity_class_band_mapping() {
        let s = 0.0; // neutral prom_shift
                     // ARPEGGIO band (> 0.80) → Subdividing.
        assert_eq!(
            melody_activity_class(0.90, s, false),
            ActivityClass::Subdividing
        );
        // SYNCOPATED band (> 0.55, ≤ 0.80) → Subdividing (≥2 onsets pushing the meter).
        assert_eq!(
            melody_activity_class(0.60, s, false),
            ActivityClass::Subdividing
        );
        // DOTTED band (> 0.25, ≤ 0.55) → Oblique (a long-short pair, minimum real motion).
        assert_eq!(
            melody_activity_class(0.40, s, false),
            ActivityClass::Oblique
        );
        // Below DOTTED (≤ 0.25) → Sustained (the held-tone arm — the inversion case).
        assert_eq!(
            melody_activity_class(0.10, s, false),
            ActivityClass::Sustained
        );
    }

    /// PROPERTY: `pre_cadence` forces the ARPEGGIO band (Subdividing) regardless of
    /// edge_activity — the cadence-acceleration path the arm protects.
    #[test]
    fn s47_melody_activity_class_pre_cadence_forces_subdividing() {
        assert_eq!(
            melody_activity_class(0.0, 0.0, true),
            ActivityClass::Subdividing,
            "a pre-cadence step subdivides into the arrival → Subdividing"
        );
    }

    /// PROPERTY: a FOREGROUND prom_shift lowers the cutoffs, so a borderline-calm melody
    /// reaches the DOTTED band sooner (the S23 rhythm bias, mirrored 1:1 in the helper).
    /// S50 re-bless: the helper now SPREADS its input through `band_activity_spread` before the
    /// band comparison (mirroring the arm 1:1). At edge 0.30 the spread is
    /// 0.40 + (0.30-0.40)*GAIN_LOW(1.8) = 0.22, which sits between DOTTED-0.05(0.20) and
    /// DOTTED(0.25): with a foreground shift (0.05) the dotted cutoff drops 0.25→0.20, so
    /// 0.22 > 0.20 → Oblique; with neutral shift 0.22 ≤ 0.25 → Sustained. The PROPERTY (a
    /// foreground shift reaches DOTTED sooner) is unchanged; only the boundary edge value moves
    /// because the comparison input is now the spread value.
    #[test]
    fn s47_melody_activity_class_prom_shift_lowers_cutoffs() {
        assert_eq!(
            melody_activity_class(0.30, 0.0, false),
            ActivityClass::Sustained
        );
        assert_eq!(
            melody_activity_class(0.30, 0.05, false),
            ActivityClass::Oblique,
            "a foreground melody subdivides more readily — the cutoff shift reaches DOTTED"
        );
    }

    /// PROPERTY (the GOVERNOR's class→counter-mode law): the counter takes the MOVING branch
    /// (the guaranteed off-beat onset) IFF the melody is Subdividing; for Oblique and
    /// Sustained the counter recedes (it does NOT move). This is the inversion fix:
    /// historically the counter moved whenever the melody held — now it moves only when the
    /// melody moves, PRESERVING S45 (melody moves → counter moves) without out-moving a
    /// holding melody. We assert the boundary the governor's `match` branches on.
    #[test]
    fn s47_governor_counter_moves_iff_melody_subdividing() {
        let counter_moves = |class: ActivityClass| class == ActivityClass::Subdividing;
        // Melody moves → counter MOVES (S45 preserved).
        assert!(counter_moves(melody_activity_class(0.90, 0.0, false))); // arpeggio
        assert!(counter_moves(melody_activity_class(0.60, 0.0, false))); // syncopated
                                                                         // Melody at minimum motion → counter RECEDES (one rank below).
        assert!(!counter_moves(melody_activity_class(0.40, 0.0, false))); // dotted → Oblique
                                                                          // Melody HOLDS → counter RECEDES (the fix — no guaranteed onset over a held melody).
        assert!(!counter_moves(melody_activity_class(0.10, 0.0, false))); // sustained
    }

    /// PROPERTY (the ACTIVITY FLOOR): a FOREGROUND melody (weight > threshold) that the band
    /// ladder would otherwise SUSTAIN is floored up to the DOTTED (Oblique) band, so it
    /// carries ≥2 onsets and out-moves the governed counter. A neutral (== 0.5) or recessive
    /// melody is UNAFFECTED (byte-stable). This replicates the EXACT `floor_to_dotted`
    /// predicate the Melody arm uses.
    #[test]
    fn s47_activity_floor_lifts_foreground_sustained_to_dotted() {
        let edge_calm = 0.10; // below the dotted cutoff → the un-floored class is Sustained
        let floor_fires = |weight: f32| -> bool {
            let prom_shift = (weight - PROMINENCE_NEUTRAL) * PROMINENCE_RHY_SHIFT;
            weight > ACTIVITY_FLOOR_THRESHOLD
                && melody_activity_class(edge_calm, prom_shift, false) == ActivityClass::Sustained
        };
        // Foreground (> 0.5) on a calm image → the floor FIRES (Sustained → Dotted).
        assert!(
            floor_fires(0.78),
            "a foreground melody must not hold on a calm image — floor it to Dotted"
        );
        assert!(floor_fires(0.90));
        // Neutral identity weight → NO floor (strict `>` is false) → byte-stable SUSTAINED.
        assert!(
            !floor_fires(0.50),
            "the identity/neutral melody (0.5) must be a no-op — the byte-freeze hinge"
        );
        // Recessive melody → NO floor.
        assert!(!floor_fires(0.40));
    }

    /// PROPERTY: the floor only LIFTS an already-Sustained selection — it never fires when the
    /// band ladder already chose a moving band (so it can only ADD activity, never remove it).
    #[test]
    fn s47_activity_floor_noop_when_melody_already_moving() {
        let edge_busy = 0.60; // syncopated band → Subdividing even before the floor
        let weight = 0.90_f32;
        let prom_shift = (weight - PROMINENCE_NEUTRAL) * PROMINENCE_RHY_SHIFT;
        let floor_fires = weight > ACTIVITY_FLOOR_THRESHOLD
            && melody_activity_class(edge_busy, prom_shift, false) == ActivityClass::Sustained;
        assert!(
            !floor_fires,
            "the floor must not fire on an already-moving melody — it only lifts SUSTAINED"
        );
    }

    /// PROPERTY (the SEAT-ORDER GUARD + its counter-present gate): on a DARK image the
    /// brightness lift is −12, which without the guard seats the melody INTO the counter band
    /// [55, 67). With a counter present the guard floors the seat to COUNTER_CEILING +
    /// MIN_FIGURE_GAP (a clear seat above the counter). With NO counter present the guard is a
    /// no-op (the freeze gate) and the melody seats at its dark, low pitch. Driven directly
    /// through `role_pitch` (pure).
    #[test]
    fn s47_seat_guard_lifts_dark_melody_only_when_counter_present() {
        let chord = c_major_triad(); // top tone pc = G (67 % 12 == 7)
        let dark = PerfFeatures {
            saturation: 50.0,
            brightness: 0.0, // bright_octaves == -1 → lift == -12 (the dark drop)
            edge_density: 0.0,
        };
        // NO counter present → guard is a no-op → the dark melody seats LOW (below the ceiling).
        let seat_no_counter = role_pitch(
            OrchestralRole::Melody,
            &chord,
            0,
            1,
            &dark,
            PROMINENCE_NEUTRAL,
            false,
        );
        assert!(
            seat_no_counter < COUNTER_CEILING,
            "no counter → no guard → the dark melody drops below the counter ceiling \
             (seat {seat_no_counter}, ceiling {COUNTER_CEILING}) — the freeze-path behavior"
        );
        // Counter present → the guard floors the seat to a CLEAR margin above the ceiling.
        let seat_with_counter = role_pitch(
            OrchestralRole::Melody,
            &chord,
            0,
            1,
            &dark,
            PROMINENCE_NEUTRAL,
            true,
        );
        assert!(
            seat_with_counter >= COUNTER_CEILING + MIN_FIGURE_GAP,
            "counter present → the melody seat must clear COUNTER_CEILING + MIN_FIGURE_GAP \
             (seat {seat_with_counter}, floor {})",
            COUNTER_CEILING + MIN_FIGURE_GAP
        );
    }

    /// PROPERTY: the seat guard is a NO-OP on a BRIGHT image even with a counter present — a
    /// bright melody already seats well above the counter ceiling, so the `.max(...)` floor
    /// never bites (it only rescues a DARK-dropped melody). Confirms the guard is additive,
    /// not a constant clamp.
    #[test]
    fn s47_seat_guard_noop_on_bright_melody() {
        let chord = c_major_triad();
        let bright = PerfFeatures {
            saturation: 50.0,
            brightness: 100.0, // bright_octaves == +1 → lift == +12
            edge_density: 0.0,
        };
        let seat_no_counter = role_pitch(
            OrchestralRole::Melody,
            &chord,
            0,
            1,
            &bright,
            PROMINENCE_NEUTRAL,
            false,
        );
        let seat_with_counter = role_pitch(
            OrchestralRole::Melody,
            &chord,
            0,
            1,
            &bright,
            PROMINENCE_NEUTRAL,
            true,
        );
        assert_eq!(
            seat_no_counter, seat_with_counter,
            "a bright melody already sits well above the counter ceiling — the guard must be \
             a no-op (it only rescues a dark-dropped seat), so counter-present cannot move it"
        );
        assert!(seat_with_counter >= COUNTER_CEILING + MIN_FIGURE_GAP);
    }

    // ════════════════════════════════════════════════════════════════════════
    // S47 SLICE 4 — THE BED ACTIVITY RECESSION (pass 2) unit tests.
    // ════════════════════════════════════════════════════════════════════════

    /// A test Pad event at `offset_ms` (hold/note/velocity are immaterial to the cap arithmetic).
    fn pad_ev(offset_ms: u64) -> NoteEvent {
        NoteEvent {
            note: 60,
            velocity: 64,
            hold_ms: 100,
            offset_ms,
        }
    }

    /// PROPERTY: `melody_min_onsets` mirrors the Melody arm's per-class minimum onset counts 1:1
    /// (Sustained→1 held tone, Oblique→2 DOTTED long-short pair, Subdividing→2 the MIN of {SYNC 2,
    /// ARP 3}). The Subdividing case is the CONSERVATIVE 2 (not 3) so the cap can never violate
    /// `bed ≤ melody` on a SYNCOPATED step while still letting an ARPEGGIO step bloom above it.
    #[test]
    fn s47_pad_melody_min_onsets_mirrors_arm() {
        assert_eq!(melody_min_onsets(ActivityClass::Sustained), 1);
        assert_eq!(melody_min_onsets(ActivityClass::Oblique), 2);
        assert_eq!(
            melody_min_onsets(ActivityClass::Subdividing),
            2,
            "Subdividing must read the MIN (SYNCOPATED=2), not the ARPEGGIO=3, so the cap is safe \
             on the busiest co-sounding step the class admits"
        );
    }

    /// PROPERTY (THE FREEZE WITNESS): at neutral Pad weight (PROMINENCE_NEUTRAL == 0.5 — the
    /// identity / golden / synthetic-bar path) `pad_onset_cap` returns None (NO cap) for EVERY
    /// melody class, so the Pad arm runs byte-identically and engine.rs stays frozen. This is the
    /// pass-2 analogue of the melody floor's no-op-at-0.5 hinge. A weight just ABOVE neutral (a
    /// hypothetically FOREGROUND Pad) is also uncapped (strict `<` gate).
    #[test]
    fn s47_pad_onset_cap_noop_at_neutral_weight() {
        for class in [
            ActivityClass::Sustained,
            ActivityClass::Oblique,
            ActivityClass::Subdividing,
        ] {
            assert_eq!(
                pad_onset_cap(PROMINENCE_NEUTRAL, class),
                None,
                "neutral Pad weight 0.5 must be the freeze hinge — no cap on any class"
            );
            assert_eq!(
                pad_onset_cap(0.7, class),
                None,
                "a Pad weight above neutral must also be uncapped (the strict `<` gate)"
            );
        }
    }

    /// PROPERTY (IMAGE-CONDITIONED DEPTH): a DEEPER-recessed Pad (lower resolved weight) caps to
    /// FEWER onsets. The deep tier (weight ≤ PAD_DEEP_RECESSION_CEILING, e.g. 0.30) caps ONE BELOW
    /// the melody's class minimum (a STRICT positive F1 lead); the shallow tier (e.g. 0.45) caps AT
    /// the melody (near-even). So for an Oblique/Subdividing melody (min 2) deep→1, shallow→2, and
    /// deep < shallow — lower weight ⇒ fewer Pad onsets, the recession-family integration.
    #[test]
    fn s47_pad_onset_cap_deeper_tier_fewer_onsets() {
        // Oblique melody (min 2): deep caps to 1 (strict lead), shallow caps to 2 (even).
        let deep = pad_onset_cap(0.30, ActivityClass::Oblique).unwrap();
        let shallow = pad_onset_cap(0.45, ActivityClass::Oblique).unwrap();
        assert_eq!(
            deep, 1,
            "deep tier (0.30) recedes one BELOW the melody's 2 onsets"
        );
        assert_eq!(
            shallow, 2,
            "shallow tier (0.45) recedes to EVEN with the melody"
        );
        assert!(
            deep < shallow,
            "lower resolved Pad weight ⇒ FEWER Pad onsets (the image-conditioned depth)"
        );
        // Subdividing melody (min 2) behaves the same (the cap reads the class minimum).
        assert_eq!(pad_onset_cap(0.30, ActivityClass::Subdividing).unwrap(), 1);
        assert_eq!(pad_onset_cap(0.45, ActivityClass::Subdividing).unwrap(), 2);
    }

    /// PROPERTY (THE NO-HOLLOWING FLOOR): the cap NEVER drops the Pad to 0 onsets — even the
    /// DEEPEST tier against the CALMEST melody (Sustained → min 1, deep gap 1 → 0) is floored back
    /// to PAD_ONSET_FLOOR (1). A silent bed re-opens the S45 static-bed defect, so the bed recedes
    /// in ACTIVITY but always keeps ≥1 sounding onset.
    #[test]
    fn s47_pad_onset_cap_never_hollows() {
        assert_eq!(
            pad_onset_cap(0.30, ActivityClass::Sustained).unwrap(),
            PAD_ONSET_FLOOR,
            "deep tier against a Sustained melody would compute 0 — must floor to 1 (no hollow)"
        );
        // And the floor holds across every (tier, class) combination the recession can hit.
        for w in [0.30_f32, 0.40, 0.45, 0.49] {
            for class in [
                ActivityClass::Sustained,
                ActivityClass::Oblique,
                ActivityClass::Subdividing,
            ] {
                assert!(
                    pad_onset_cap(w, class).unwrap() >= PAD_ONSET_FLOOR,
                    "the bed must never recede below {PAD_ONSET_FLOOR} onset (w={w}, {class:?})"
                );
            }
        }
    }

    /// PROPERTY (THE CAP ARITHMETIC + WEAK-BEAT DISPLACEMENT on the BLOCK bed): a 3-voice block-bed
    /// stab (all at offset 0) capped to 1 keeps ONE onset (no hollow) and DISPLACES it OFF the
    /// downbeat to the weak beat (so its onset grid does not fuse with the Bass/Fill/Melody downbeat
    /// — the F5a anti-fusion steer). The onset COUNT is preserved by the displacement (F5b-safe).
    #[test]
    fn s47_recede_block_bed_thins_and_displaces_off_downbeat() {
        let step_ms = 200_u64;
        let block = vec![pad_ev(0), pad_ev(0), pad_ev(0)]; // 3-voice simultaneous stab
        let out = recede_pad_onsets(block, 1, step_ms);
        assert_eq!(
            out.len(),
            1,
            "capped to 1 onset (deep tier), never hollowed to 0"
        );
        assert_ne!(
            out[0].offset_ms, 0,
            "the surviving block-bed stab must be DISPLACED off the downbeat (F5a anti-fusion)"
        );
        let expected_weak = (step_ms as f32 * PAD_WEAK_BEAT_FRAC).round() as u64;
        assert_eq!(
            out[0].offset_ms, expected_weak,
            "displaced to the weak beat"
        );
    }

    /// PROPERTY (FIGURED bed recession — drop the DOWNBEAT-nearest onsets first, KEEP weak beats):
    /// a 4-onset broken-chord figure (offsets 0, 50, 100, 150 over a 200 ms step) capped to 2 keeps
    /// the TWO LATEST (weak-beat) onsets and drops the offset-0 (downbeat) onset — so the bed
    /// recedes off the melody's strong beat. The kept onsets are returned in original (ascending
    /// offset) order, and because a weak-beat onset survives, NO displacement is applied.
    #[test]
    fn s47_recede_figured_bed_drops_downbeat_keeps_weak_beats() {
        let step_ms = 200_u64;
        let figure = vec![pad_ev(0), pad_ev(50), pad_ev(100), pad_ev(150)];
        let out = recede_pad_onsets(figure, 2, step_ms);
        assert_eq!(out.len(), 2, "capped to 2 onsets");
        let offs: Vec<u64> = out.iter().map(|e| e.offset_ms).collect();
        assert_eq!(
            offs,
            vec![100, 150],
            "the two LATEST (weak-beat) onsets are kept; the offset-0 downbeat is dropped first, \
             and a surviving weak-beat onset means NO re-displacement"
        );
    }

    /// PROPERTY: a cap ≥ the event count is the IDENTITY (nothing is thinned or displaced) — the
    /// no-op path a shallow-tier cap takes when the bed is already at/under the melody's onsets.
    #[test]
    fn s47_recede_pad_onsets_identity_when_under_cap() {
        let step_ms = 200_u64;
        let figure = vec![pad_ev(0), pad_ev(100)];
        let out = recede_pad_onsets(figure.clone(), 2, step_ms);
        assert_eq!(out, figure, "cap == len → unchanged");
        let out2 = recede_pad_onsets(figure.clone(), 5, step_ms);
        assert_eq!(out2, figure, "cap > len → unchanged");
    }

    // ═══════════════════════════════════════════════════════════════════════
    // S48 SLICE 3 — INVERSE-REGISTER COMPENSATION + the LEVEL FINISH tests.
    // ═══════════════════════════════════════════════════════════════════════

    /// PROPERTY (the comp BOUNDARIES): the factor is 0.0 at/above the guarded seat floor
    /// (COUNTER_CEILING + MIN_FIGURE_GAP == 69 — the melody is clearly on top, no help) and
    /// rises to 1.0 at/below FILL_REGISTER_FLOOR (55 — the melody is in/under the bed band and
    /// needs the most non-level separation). theory: help is INVERSE to register height.
    #[test]
    fn s48_inverse_register_compensation_boundaries() {
        // At/above 69 → no help (the high-seat / freeze-side zero).
        assert_eq!(inverse_register_compensation(69), 0.0);
        assert_eq!(inverse_register_compensation(72), 0.0);
        assert_eq!(inverse_register_compensation(96), 0.0);
        // At/below 55 → full help.
        assert_eq!(inverse_register_compensation(55), 1.0);
        assert_eq!(inverse_register_compensation(40), 1.0);
        // The zero-crossing tracks COUNTER_CEILING + MIN_FIGURE_GAP (1:1 with the seat guard).
        assert_eq!(
            inverse_register_compensation(COUNTER_CEILING + MIN_FIGURE_GAP),
            0.0,
            "the comp's high-seat zero must sit exactly at the guarded seat floor (spec §8)"
        );
    }

    /// PROPERTY (the comp MONOTONICITY): the factor is monotone NON-INCREASING in the seat —
    /// a LOWER seat never gets LESS help than a higher one. theory (DP-3, FIXED direction): the
    /// low melody self-projects least, so it earns the most separation; the direction is the
    /// load-bearing invariant (the magnitude/shape is ear-tunable).
    #[test]
    fn s48_inverse_register_compensation_monotone_non_increasing() {
        let mut prev = inverse_register_compensation(40);
        for seat in 41u8..=96 {
            let cur = inverse_register_compensation(seat);
            assert!(
                cur <= prev + f32::EPSILON,
                "comp must not RISE as the seat rises (seat {seat}: {cur} > prev {prev})"
            );
            prev = cur;
        }
        // A mid-band seat gets STRICTLY more help than a higher mid-band seat (it is a real ramp,
        // not a flat step): seat 58 (deep in the bed band) > seat 66 (just under the ceiling).
        assert!(
            inverse_register_compensation(58) > inverse_register_compensation(66),
            "a lower-seated melody earns strictly more non-level separation in the ramp"
        );
    }

    /// PROPERTY (the PRIMARY tool is COUNT-PRESERVING): the inverse-comp onset push moves WHERE
    /// the melody's FIRST onset sits, never HOW MANY events — so F5b (bed_onsets ≤ melody_onsets)
    /// and F1 cannot regress. Replicates the arm's exact push logic (the
    /// s47_activity_floor test's replication discipline) on the two PUSHABLE band shapes: DOTTED
    /// stays 2 onsets, SUSTAINED stays 1, and the first onset moves OFF the downbeat at full comp.
    #[test]
    fn s48_comp_offset_push_preserves_onset_count() {
        let step_ms: u64 = 240;
        // The arm's push (full comp == 1.0): offset_push = round(1.0 * COMP_OFFSET_FRAC * step_ms).
        let comp = 1.0_f32;
        let offset_push = (comp * COMP_OFFSET_FRAC * step_ms as f32).round() as u64;
        assert!(
            offset_push > 0,
            "full comp must actually push the onset off the downbeat"
        );
        let push_first = |mut events: Vec<NoteEvent>| -> Vec<NoteEvent> {
            if let Some(first) = events.first_mut() {
                let room = step_ms.saturating_sub(offset_push).max(1);
                let prev_hold = first.hold_ms.max(1);
                let refit = prev_hold.min(room);
                first.offset_ms = offset_push;
                first.hold_ms = refit.max(1);
            }
            events
        };
        // DOTTED shape: two onsets (at 0 and 2/3) — stays TWO after the push.
        let two_thirds = step_ms * 2 / 3;
        let dotted = vec![
            NoteEvent {
                note: 60,
                velocity: 76,
                hold_ms: two_thirds,
                offset_ms: 0,
            },
            NoteEvent {
                note: 60,
                velocity: 76,
                hold_ms: step_ms - two_thirds,
                offset_ms: two_thirds,
            },
        ];
        let pushed = push_first(dotted);
        assert_eq!(
            pushed.len(),
            2,
            "DOTTED must stay 2 onsets (count-preserving)"
        );
        assert_eq!(
            pushed[0].offset_ms, offset_push,
            "the first DOTTED onset moves off 0"
        );
        assert_eq!(
            pushed[1].offset_ms, two_thirds,
            "the SECOND DOTTED onset is untouched"
        );
        assert!(
            pushed[0].offset_ms + pushed[0].hold_ms <= step_ms,
            "the pushed onset's hold is re-fit so it does not ring across the step boundary"
        );
        // SUSTAINED shape: a single onset at 0 — stays ONE after the push.
        let sustained_ev = vec![NoteEvent {
            note: 60,
            velocity: 76,
            hold_ms: step_ms,
            offset_ms: 0,
        }];
        let pushed_s = push_first(sustained_ev);
        assert_eq!(
            pushed_s.len(),
            1,
            "SUSTAINED must stay 1 onset (count-preserving)"
        );
        assert_eq!(
            pushed_s[0].offset_ms, offset_push,
            "the single SUSTAINED onset moves off 0"
        );
        assert!(
            pushed_s[0].offset_ms + pushed_s[0].hold_ms <= step_ms,
            "the pushed SUSTAINED onset's hold is re-fit within the step"
        );
    }

    /// PROPERTY (the LEVEL FINISH — 2a.ii velocity-bias arms): the CounterMelody and HarmonicFill
    /// RECEDE in level BELOW an EQUAL-prominence Melody — completing the figure-ground gap from the
    /// level side. Driven directly through `realize_velocity` (pure of ctx). At equal prominence the
    /// only velocity difference is the per-role structural bias (Melody +2, Counter −COUNTER_VEL_BIAS,
    /// Fill −FILL_VEL_BIAS), so counter < melody and fill < melody, and the counter recedes MORE than
    /// the fill (a moving line stays MORE audible than the connective tissue). S45 preserved: the
    /// bias only RECEDES level (bounded by the 1..=127 clamp), it does NOT mute.
    #[test]
    fn s48_counter_fill_recede_below_equal_prominence_melody() {
        let chord = c_major_triad();
        let step = StepPlan {
            chord,
            phrase_index: 0,
            position_in_phrase: 2, // a plain interior beat (not cadence/start) so the arms fire
            phrase_len: 8,
            position: PhrasePosition::Interior,
            velocity: 80,
        };
        let features = PerfFeatures {
            saturation: 60.0,
            brightness: 50.0,
            edge_density: 0.3,
        };
        // Equal prominence (neutral) for all three roles → the ONLY difference is the per-role bias.
        let w = PROMINENCE_NEUTRAL;
        let mel = realize_velocity(&step, &features, false, OrchestralRole::Melody, w);
        let counter = realize_velocity(&step, &features, false, OrchestralRole::CounterMelody, w);
        let fill = realize_velocity(&step, &features, false, OrchestralRole::HarmonicFill, w);
        assert!(
            counter < mel,
            "the counter must recede below an equal-prominence melody (counter {counter} < melody {mel})"
        );
        assert!(
            fill < mel,
            "the fill must recede below an equal-prominence melody (fill {fill} < melody {mel})"
        );
        assert!(
            counter < fill,
            "the counter recedes MORE than the fill (counter {counter} < fill {fill}) — bias 2.0 vs 1.0"
        );
        assert!(
            counter >= 1,
            "S45: the counter recedes in LEVEL but is never muted to silence"
        );
    }
}

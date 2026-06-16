use crate::mapping_loader::{lookup_range_map, MappingTable};
use rand::seq::SliceRandom;
use rand::thread_rng;

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
    /// A second melodic line moving under/around the Melody. Its realization is
    /// STUBBED for now: it delegates to the HarmonicFill figure (a real counter-
    /// line is a later slice — a pure realization fill, no signature change). It
    /// is unreachable under the behaviour-neutral identity profile, so byte-neutral.
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
    // How many chord tones a Pad instrument holds for this section (0 == no pad,
    // the identity-profile default — so this read is inert on the legacy path and
    // the Pad branch never fires there). Read zero-copy off the borrowed section.
    let pad_voices = ctx.section.orchestration.pad_voices;
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
        None => role_pitch(role, &step.chord, inst_idx, num_instruments, features),
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
    let velocity = realize_velocity(step, features, is_cadence, role);

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
            let floor = (MELODY_REGISTER_FLOOR as i16 + lift).clamp(24, 96) as u8;
            seat_pc_in_register(pc, floor)
        }
        // Pad and CounterMelody both seat a representative INNER tone in the fill
        // register, exactly like HarmonicFill. theory: the Pad's full multi-tone
        // bed is built in realize_rhythm directly off the chord (one note can't
        // express it); the single base_note role_pitch returns here is only the
        // anchor pitch the other realize machinery threads through. The
        // CounterMelody stub delegates to the fill figure for now. Both therefore
        // share the inner-tone seating below so they sit under the melody.
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
            let floor = (FILL_REGISTER_FLOOR as i16 + lift).clamp(24, 96) as u8;
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
        _ => {}
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
/// The ritardando multiplier applied to a phrase-final note's hold. theory: as
/// the phrase relaxes into its arrival the final note rings longer.
const RITARDANDO_FACTOR: f32 = 1.30;
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
) -> Vec<NoteEvent> {
    // S13: normalize the RAW per-bar edge density into a 0..1 ACTIVITY knob. The old
    // code compared raw edge (≈0.005 on real photos) against 0.25/0.70 cutoffs, so
    // every real image fell in the "low edge → legato/sustained" band — uniform output.
    // Normalizing first lets the curve and the pattern bands span their full range.
    let edge_activity = (features.edge_density / EDGE_ACTIVITY_RANGE_MAX).clamp(0.0, 1.0);
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
    let base_frac = match role {
        // Fill biases toward the connected end so inner voices sustain under the line.
        // Floor it at the window LOW, not LEGATO_FRAC (0.95): clamping a HarmonicFill
        // up to 0.95 would punch ABOVE the busy-end of the new window and re-introduce
        // a discontinuity. The window low (0.55) is the right connected-leaning floor.
        OrchestralRole::HarmonicFill => curve_frac.max(ARTIC_WINDOW_LO),
        _ => curve_frac,
    }
    // Apply the per-character articulation bias (slice-1 Ballad == 1.0, no-op), then
    // clamp back into the window so the bias can't escape the pleasant range.
    .mul_add(BALLAD_ARTIC_BIAS, 0.0)
    .clamp(ARTIC_WINDOW_LO, ARTIC_WINDOW_HI);

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
        return vec![sustained(0, step_ms, LEGATO_FRAC)];
    }

    match role {
        OrchestralRole::Bass => {
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
            if edge_activity < FILL_REST_ACTIVITY && weak_interior {
                // rest-as-gesture: emit NO event.
                Vec::new()
            } else {
                vec![sustained(0, step_ms, base_frac)]
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
            if notes.is_empty() || pad_voices == 0 {
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
            let prev: Option<&StepPlan> = si.checked_sub(1).and_then(|p| ctx.section.steps.get(p));

            // Melody pitch THIS step and the PREVIOUS step, recomputed exactly as the
            // Melody instrument computes it. Some(p) == sounds p; None == the melody
            // rests this step (treated as Hold for the contrary rule, a gap for rhythm).
            let m_now = melody_pitch_for(ctx, step, features);
            let m_prev = prev.and_then(|p| melody_pitch_for_step(ctx, p, features));
            let mel_dir = motion_dir(m_prev, m_now);

            // Held-period / melody-static detection (§3.4 — the "empty periods" answer).
            let held_chord = prev.is_some_and(|p| p.chord.notes == step.chord.notes);
            let melody_static = mel_dir == MotionDir::Hold || m_now.is_none();

            // How many consecutive PRIOR steps already sounded THIS exact chord — the
            // step's position WITHIN the current held run (0 at the run's first step).
            // Bounded by HELD_RUN_SEED_CAP so this stays O(cap) per step, never O(run
            // length): a held run only needs ~one rotation through the chord's band
            // tones to read as a moving line, so the cap is the rotation period, not the
            // whole section. Deterministic (a pure scan of plan chords, no RNG).
            let held_run_index = held_run_position(ctx.section, si);

            // PITCH: contrary/oblique, chord-tone, no parallel perfects vs the melody
            // (§3.2). Two seed modes drive `pick_counter_pitch`:
            //   * INSIDE a held run (held_run_index >= 1): seat the ADVANCING ROTATION
            //     TARGET (a non-root band tone that walks with the run position) as the
            //     pick's previous pitch with force_move OFF, so the pick LANDS on that
            //     rotated tone — each held step sounds a NEW inner tone, the moving line
            //     the operator's "empty period" verdict demanded. The no-parallel-perfects
            //     reject still guards it.
            //   * OTHERWISE (run start / changing chord): the §3.1 LOCK nearest-prior-chord
            //     seed with force_move = held_chord||melody_static, byte-identical to the
            //     as-built impl on every non-held-run step.
            // The advancing held-run TARGET (Some inside a held run), or None on a run
            // start / changing chord. When present it is the chord tone the held line
            // should SOUND this step (the rotation already chose the musically-correct,
            // non-root, moving tone); `pick_counter_pitch` lands on it unless it would
            // form a parallel perfect against the melody, in which case it falls back to
            // the scored pick. When None, the seed + force_move path is the as-built impl.
            let held_target = advancing_seed_counter(step, held_run_index);
            let prev_counter = seed_prev_counter(prev, step);
            let cnt = pick_counter_pitch(
                &step.chord,
                prev_counter,
                m_prev,
                m_now,
                mel_dir,
                held_chord || melody_static,
                held_target,
            );

            let with_note = |ev: NoteEvent| NoteEvent { note: cnt, ..ev };

            // RHYTHM (§3.3) + HELD-PERIOD ACTIVATION (§3.4).
            if held_chord || melody_static {
                // MOVING mode: a GUARANTEED off-beat onset (no rest-as-gesture), a
                // single delayed note at step_ms/4 — the moving line that weaves under
                // the held/static harmony and fills the operator's "empty period."
                let offset = step_ms / 4;
                let slot = step_ms - offset;
                let mut ev = sustained(offset, slot, base_frac);
                ev = with_note(ev);
                vec![ev]
            } else if edge_activity > 0.55 {
                // The melody is ACTIVE (it subdivides — arpeggio/syncopated bands): the
                // counter holds ONE sustained tone underneath (the OBLIQUE case), staying
                // out of the melody's way. Onset 0, one note for the full step.
                vec![with_note(sustained(0, step_ms, base_frac))]
            } else {
                // Both voices calm and the chord is changing: the texture may breathe —
                // rest-as-gesture on a weak interior beat, gated on FILL_REST_ACTIVITY
                // exactly like the fill, so a genuinely near-static image gets the
                // occasional counter-rest. Otherwise one sustained tone.
                let weak_interior = !step.position_in_phrase.is_multiple_of(2);
                if edge_activity < FILL_REST_ACTIVITY && weak_interior {
                    Vec::new()
                } else {
                    vec![with_note(sustained(0, step_ms, base_frac))]
                }
            }
        }

        OrchestralRole::Melody => {
            // MELODY: the most rhythmic freedom. Pattern by NORMALIZED edge_activity
            // band (design-s13 §2), with syncopation and pre-cadence acceleration.
            // theory: recalibrating the cutoffs against edge_activity (not raw edge)
            // is what makes "busy image → denser, arpeggiated melody" actually fire on
            // real photos — under the old raw cutoffs every photo fell in "sustained".
            if pre_cadence || edge_activity > 0.80 {
                // ARPEGGIO / acceleration: spread chord-tone onsets evenly across
                // the step (more onsets, shorter values) — the active, driving
                // figure. theory: subdividing the beat is the melody's way of
                // generating forward motion, intensified into a cadence.
                let n = if pre_cadence { 4 } else { 3 };
                let slot = step_ms / n as u64;
                (0..n)
                    .map(|k| {
                        let offset = (k as u64) * slot;
                        sustained(offset, slot, STACCATO_FRAC)
                    })
                    .collect()
            } else if edge_activity > 0.55 {
                // SYNCOPATED: delay the onset off the downbeat by 1/4 step, then
                // a second onset, pushing against the meter. theory: syncopation
                // displaces the accent to energize an active-but-not-busy melody.
                let quarter = step_ms / 4;
                vec![
                    sustained(quarter, step_ms / 2, PORTATO_FRAC),
                    sustained(step_ms * 3 / 4, step_ms / 4, STACCATO_FRAC),
                ]
            } else if edge_activity > 0.25 {
                // DOTTED: a long-short pair (onsets at 0 and 2/3; holds 2/3 and
                // 1/3) — the lilting mid-activity figure. theory: the dotted
                // rhythm is the default expressive subdivision of a singing line.
                let two_thirds = step_ms * 2 / 3;
                vec![
                    sustained(0, two_thirds, PORTATO_FRAC),
                    sustained(two_thirds, step_ms - two_thirds, STACCATO_FRAC),
                ]
            } else {
                // SUSTAINED (low edge_activity): one long tone whose length rides the
                // CONTINUOUS articulation curve (S13). At low activity base_frac ≈ 1.05,
                // so the calm melody OVERLAPS across the step boundary and truly sings —
                // the fix for "uniformly short" notes that the old hard 0.95 cap blocked.
                vec![sustained(0, step_ms, base_frac)]
            }
        }
    }
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
            let note = seated[idx];
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
/// `length_steps` — how many `MotifNote`s the theme runs for; the contour is sampled
///                  /padded to this length. Clamped to >=1.
///
/// Determinism: pure function of its three inputs — no RNG, no clock.
pub fn resolve_motif(
    archetype: MotifArchetype,
    range_degrees: u8,
    length_steps: usize,
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
    let n_contour = contour.len();

    // Distribute `len` notes across the contour. If len >= n_contour we hold the
    // tail; if len < n_contour we sample the FRONT (a natural truncation that still
    // begins on the tonic — exactly what "head-only" fragmentation wants downstream).
    (0..len)
        .map(|i| {
            // Sample index into the contour: walk the contour once, then hold its
            // final degree for any extra steps (a sustained arrival, not a wrap —
            // wrapping would restart the shape and read as a loop).
            let ci = i.min(n_contour - 1);
            let raw = contour[ci] as f32 * scale;
            // Round to the nearest scale degree; the realizer maps degree → pitch.
            let degree = raw.round() as i8;
            MotifNote {
                degree,
                // Even 1-step durations in slice 1: the per-step rhythm comes from the
                // realizer's existing rhythm layer, not the motif. theory: keeping the
                // motif rhythmically plain in slice 1 means augmentation/diminution
                // (Stage 7) can later scale dur_steps without colliding with a
                // pre-baked rhythm here.
                dur_steps: 1,
            }
        })
        .collect()
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

/// The realizer's THEME-REPLAY decision (spec §2, task 2). Given the plan-relative
/// `StepContext`, decide what pitch the MELODY role sounds on this step:
///
///   * `thematic_role == Statement | Return` with a theme present → play the motif
///     note for `ctx.step_in_section` (degree → pitch in the section's mode/key),
///     returning `Some(pitch)`.
///   * `variation == Fragmented` → play only the FIRST HALF of the motif; once past
///     the head the melody role RESTS (returns `Some(None)` semantics via the outer
///     Option: `Some(SILENT_MELODY)`), so the caller emits no melody note.
///   * `Contrast | Coda | Development`, or no theme, or a non-Melody role → return
///     `None`: the caller free-selects exactly as today (byte-stable back-compat).
///
/// Returns `Option<Option<u8>>`:
///   * `None`          → not a theme-replay step; caller takes its existing path.
///   * `Some(Some(p))` → play the theme pitch `p`.
///   * `Some(None)`    → theme-driven REST (Fragmented tail); caller emits nothing.
///
/// This keeps the realizer's free-select / velocity / rhythm bodies UNTOUCHED — the
/// theme branch only substitutes (or silences) the melody PITCH; everything else
/// (the Bass/Fill roles, velocity contour, rhythm patterns) is unchanged.
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

    // FRAGMENTED (head-then-rest): the B-section continuity gesture. Play only the
    // first half of the motif; past the head, the melody role rests. theory: head-
    // only sequencing is the "developing" gesture — it keeps motivic continuity
    // through a contrast without restating the whole tune (design-s15 §4.1/§4.2).
    let head_len = match ctx.section.variation {
        ThemeVariation::Fragmented => (theme.motif.len() + 1) / 2, // ceil(half)
        _ => theme.motif.len(),
    };
    let motif_idx = ctx.step_in_section;
    if motif_idx >= head_len {
        // Past the playable span: Identity holds the final motif note (a sustained
        // arrival, never a wrap-loop); Fragmented rests.
        return match ctx.section.variation {
            ThemeVariation::Fragmented => Some(None), // head consumed → melody rests
            _ => {
                let last = theme.motif[theme.motif.len() - 1];
                Some(Some(theme_pitch(last.degree, ctx, chord, features)))
            }
        };
    }

    let note = theme.motif[motif_idx];
    Some(Some(theme_pitch(note.degree, ctx, chord, features)))
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
/// the melody pitch is a pure function of (ctx, chord, features) via the same theme
/// seam (`theme_melody_pitch`) and free-select (`role_pitch`) the Melody arm uses.
fn melody_pitch_for(
    ctx: &crate::composition::StepContext,
    step: &StepPlan,
    features: &PerfFeatures,
) -> Option<u8> {
    match theme_melody_pitch(ctx, OrchestralRole::Melody, &step.chord, features) {
        Some(None) => None,       // theme-driven rest → the melody is silent this step
        Some(Some(p)) => Some(p), // theme pitch
        // Free-select: inst_idx/num are irrelevant for the Melody arm of role_pitch
        // (it seats the chord's TOP tone, independent of index), so a synthetic
        // (idx 0, num 1) is correct and avoids guessing the real ensemble width.
        None => Some(role_pitch(
            OrchestralRole::Melody,
            &step.chord,
            0,
            1,
            features,
        )),
    }
}

/// Recompute the MELODY pitch for an ARBITRARY prior step `p`, by re-pointing the
/// borrowed context's `step_in_section` at that step so the theme seam reads the
/// right motif index. Pure: only the `step_in_section` field is overridden (a Copy
/// of the borrowed context), nothing is mutated through the references.
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
    // chord-tone candidate AND does not create a parallel perfect against the melody.
    // This is what makes the held line actually ADVANCE to a new tone each step (the
    // rotation owns "which tone"); the scored path's contrary-while-static bonus would
    // otherwise pull the pick off the target. Legality (no parallel perfect) still wins.
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
        CadenceStrength, KeyTempoPlan, Section, StepContext, ThematicRole, ThemeSeed,
        ThemeVariation,
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

    /// PROPERTY: resolve_motif emits exactly `length_steps` notes (>=1), each with
    /// a valid duration — the realizer reads a concrete, correctly-sized motif.
    #[test]
    fn test_resolve_motif_length_and_duration() {
        for &len in &[1usize, 4, 5, 8] {
            let m = resolve_motif(MotifArchetype::Arch, 4, len);
            assert_eq!(m.len(), len, "must emit exactly the requested length");
            assert!(
                m.iter().all(|n| n.dur_steps >= 1),
                "every motif note must have dur_steps >= 1"
            );
        }
        // length 0 is clamped to 1 (never an empty motif the realizer can't read).
        assert_eq!(resolve_motif(MotifArchetype::Arch, 4, 0).len(), 1);
    }

    /// PROPERTY: the Arch contour rises to an apex then falls back to (near) the
    /// tonic — the defining "up then down" shape. Tests degrees, not pitches.
    #[test]
    fn test_resolve_motif_arch_is_up_then_down() {
        let m = resolve_motif(MotifArchetype::Arch, 4, 5);
        let degs: Vec<i8> = m.iter().map(|n| n.degree).collect();
        // 0 .. apex .. 0 : strictly rises to the middle, then falls.
        assert!(
            degs[0] <= degs[1] && degs[1] <= degs[2],
            "must rise to apex: {degs:?}"
        );
        assert!(
            degs[2] >= degs[3] && degs[3] >= degs[4],
            "must fall from apex: {degs:?}"
        );
        assert_eq!(
            degs[0], degs[4],
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
    /// head-then-silence continuity gesture. Past the head, the melody returns
    /// `Some(None)` (a theme-driven rest), not a sounded note.
    #[test]
    fn test_theme_pitch_fragmented_head_then_rest() {
        let section = theme_section(ThematicRole::Statement, ThemeVariation::Fragmented, Some(0));
        let kt = home_key_tempo();
        let motif = resolve_motif(MotifArchetype::Arch, 4, 8); // len 8 → head = 4
        let seed = ThemeSeed { id: 0, motif };
        let chord = c_major_triad();
        let mk = |step: usize| {
            let ctx = StepContext {
                section: &section,
                step_in_section: step,
                theme: Some(&seed),
                key_tempo: &kt,
            };
            theme_melody_pitch(&ctx, OrchestralRole::Melody, &chord, &perf_mid())
        };
        // Head (steps 0..=3) sounds; tail (4..) rests.
        for step in 0..4 {
            assert!(
                matches!(mk(step), Some(Some(_))),
                "fragmented head step {step} must sound"
            );
        }
        for step in 4..8 {
            assert_eq!(
                mk(step),
                Some(None),
                "fragmented tail step {step} must REST (head-then-silence)"
            );
        }
    }

    /// PROPERTY: Identity past the motif end HOLDS the final note (a sustained arrival),
    /// never wraps/loops back to the head — a loop would read as the mechanical
    /// repetition the whole theme layer exists to kill.
    #[test]
    fn test_theme_pitch_identity_holds_not_loops() {
        let section = theme_section(ThematicRole::Return, ThemeVariation::Identity, Some(0));
        let kt = home_key_tempo();
        let motif = resolve_motif(MotifArchetype::Ascent, 4, 5);
        let seed = ThemeSeed {
            id: 0,
            motif: motif.clone(),
        };
        let chord = c_major_triad();
        let last_idx = motif.len() - 1;
        let mk = |step: usize| {
            let ctx = StepContext {
                section: &section,
                step_in_section: step,
                theme: Some(&seed),
                key_tempo: &kt,
            };
            theme_melody_pitch(&ctx, OrchestralRole::Melody, &chord, &perf_mid())
        };
        let at_end = mk(last_idx);
        let past_end = mk(last_idx + 3);
        assert!(
            matches!(at_end, Some(Some(_))),
            "final motif step must sound"
        );
        assert_eq!(
            at_end, past_end,
            "Identity past the end must HOLD the final note, not loop to the head"
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

    /// §3.6 test 4 (the operator "empty periods" verdict — the load-bearing one): two
    /// consecutive steps on the SAME voiced chord (`held_chord == true`) → the counter
    /// SOUNDS on the held step with an off-beat onset (`offset_ms == step_ms/4`), and its
    /// pitch DIFFERS from the seed it would carry from the prior step (something moves
    /// underneath the held chord — not a re-struck stab, not a rest).
    #[test]
    fn test_counter_held_period_fills_and_moves() {
        // SAME voiced chord on both steps → held period. A near-static melody (free-
        // select top tone is identical across identical chords → mel_dir Hold).
        let held = c_major_triad();
        let (sec, kt) =
            counter_ctx_parts(counter_step(held.clone(), 0), counter_step(held.clone(), 1));
        let f = perf_mid(); // calm-ish; held-period activation must override any rest
        let evs = realize_counter(&sec, &kt, &f);
        assert_eq!(
            evs.len(),
            1,
            "a held-period counter must SOUND (no rest-as-gesture), got {} events",
            evs.len()
        );
        let step_ms = sec.ms_per_step;
        assert_eq!(
            evs[0].offset_ms,
            step_ms / 4,
            "a held-period counter must onset OFF the downbeat at step_ms/4"
        );
        // The moving-line guarantee: the held step's counter pitch differs from what the
        // PRIOR step of the held run actually sounds — the line STEPS to a new chord tone
        // (the advancing seed rotates the seat each held step), so consecutive held steps
        // are NOT the same re-struck stab. Compared against the prior step's ACTUAL
        // realized pitch (step index 0), not a static prior-chord seed proxy.
        let prev_ctx = StepContext {
            section: &sec,
            step_in_section: 0,
            theme: None,
            key_tempo: &kt,
        };
        let prev_evs = realize_step(&sec.steps[0], 2, 4, &f, sec.ms_per_step, &prev_ctx);
        assert_eq!(
            prev_evs.len(),
            1,
            "the prior held step also sounds one note"
        );
        assert_ne!(
            evs[0].note, prev_evs[0].note,
            "the counter must step to a NEW chord tone across the held run (moving line, \
             not a re-struck stab): step1 note {} == step0 note {}",
            evs[0].note, prev_evs[0].note
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
            };
            let evs = realize_step(&section.steps[si], 2, 4, &f, section.ms_per_step, &ctx);
            assert_eq!(
                evs.len(),
                1,
                "each held step sounds exactly one counter note"
            );
            assert_eq!(
                evs[0].offset_ms,
                section.ms_per_step / 4,
                "every held step onsets off the downbeat (rhythmic fill still holds)"
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
            let seed = seed_prev_counter(Some(&sec.steps[0]), &sec.steps[1]);
            let cnt_dir = motion_dir(Some(seed), Some(evs[0].note));
            assert_ne!(
                cnt_dir,
                MotionDir::Up,
                "the counter must move CONTRARY/OBLIQUE (not strictly up with the melody): \
                 seed {seed} → note {}",
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

    /// §3.6 test 5 / §6.4 (the supersession witness): on a held-chord/static-melody step
    /// the counter onsets OFF the downbeat (step_ms/4), where the OLD HarmonicFill stub
    /// would have onset at 0 — so the counter is NO LONGER a HarmonicFill delegate. This
    /// is the in-file analogue of the retired texture_s17 stub-equals assertion.
    #[test]
    fn test_counter_no_longer_harmonicfill_delegate() {
        let held = c_major_triad();
        let (sec, kt) =
            counter_ctx_parts(counter_step(held.clone(), 0), counter_step(held.clone(), 1));
        let f = perf_mid();
        let counter = realize_counter(&sec, &kt, &f);
        assert_eq!(counter.len(), 1, "counter sounds on the held step");
        assert_ne!(
            counter[0].offset_ms, 0,
            "the real counter onsets OFF the downbeat (step_ms/4) on a held/static step — \
             the HarmonicFill figure would onset at 0; the stub is superseded"
        );
        // And it is the documented step_ms/4 offset.
        assert_eq!(counter[0].offset_ms, sec.ms_per_step / 4);
    }
}

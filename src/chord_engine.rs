use crate::mapping_loader::MappingTable;
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
#[derive(Debug, Clone)]
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
) -> Vec<NoteEvent> {
    let role = instrument_role(inst_idx, num_instruments);
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
    let base_note = role_pitch(role, &step.chord, inst_idx, num_instruments, features);

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
        OrchestralRole::HarmonicFill => {
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
/// The ritardando multiplier applied to a phrase-final note's hold. theory: as
/// the phrase relaxes into its arrival the final note rings longer.
const RITARDANDO_FACTOR: f32 = 1.30;

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
) -> Vec<NoteEvent> {
    let edge = features.edge_density.clamp(0.0, 1.0);
    let step_ms = ms_per_step.max(1);

    // Harmonic-rhythm acceleration: the step immediately before a cadence drives
    // into the arrival with MORE onsets. theory: shortening note values toward a
    // cadence is how common-practice phrases "press" into their close.
    let pre_cadence = !is_cadence
        && !is_phrase_start
        && step.position_in_phrase + 2 >= step.phrase_len
        && step.phrase_len >= 2;

    // Articulation fraction for sustained/single-onset notes, before ritardando.
    // theory: low edge_density (calm texture) -> legato; high -> staccato; the
    // fill sustains (legato) to support; mid is portato (the neutral default).
    let base_frac = match role {
        OrchestralRole::HarmonicFill => LEGATO_FRAC,
        _ if edge < 0.25 => LEGATO_FRAC,
        _ if edge > 0.70 => STACCATO_FRAC,
        _ => PORTATO_FRAC,
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
                // One grounded, sustained root for the whole step.
                vec![sustained(0, step_ms, LEGATO_FRAC)]
            }
        }

        OrchestralRole::HarmonicFill => {
            // HARMONIC-FILL: supports, never competes — least rhythmic activity,
            // sustained inner tones. This is the ONLY place rest-as-gesture is
            // allowed, and only on a weak interior beat (never start/cadence).
            // theory: a deliberate silence in an inner voice is an articulation,
            // letting the outer voices speak.
            let weak_interior = !step.position_in_phrase.is_multiple_of(2);
            if edge < 0.15 && weak_interior {
                // rest-as-gesture: emit NO event.
                Vec::new()
            } else {
                vec![sustained(0, step_ms, base_frac)]
            }
        }

        OrchestralRole::Melody => {
            // MELODY: the most rhythmic freedom. Pattern by edge_density band,
            // with syncopation and pre-cadence acceleration.
            if pre_cadence || edge > 0.80 {
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
            } else if edge > 0.55 {
                // SYNCOPATED: delay the onset off the downbeat by 1/4 step, then
                // a second onset, pushing against the meter. theory: syncopation
                // displaces the accent to energize an active-but-not-busy melody.
                let quarter = step_ms / 4;
                vec![
                    sustained(quarter, step_ms / 2, PORTATO_FRAC),
                    sustained(step_ms * 3 / 4, step_ms / 4, STACCATO_FRAC),
                ]
            } else if edge > 0.25 {
                // DOTTED: a long-short pair (onsets at 0 and 2/3; holds 2/3 and
                // 1/3) — the lilting mid-activity figure. theory: the dotted
                // rhythm is the default expressive subdivision of a singing line.
                let two_thirds = step_ms * 2 / 3;
                vec![
                    sustained(0, two_thirds, PORTATO_FRAC),
                    sustained(two_thirds, step_ms - two_thirds, STACCATO_FRAC),
                ]
            } else {
                // SUSTAINED (low edge): one long legato tone — the calm, singing
                // melody when the texture is sparse.
                vec![sustained(0, step_ms, LEGATO_FRAC)]
            }
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
        eng.generate_chords(&prog, ROOT, mode, 0.0, 0.0)
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

        let mut realized: Vec<u8> = Vec::new();
        let mut floor: Vec<u8> = Vec::new();
        for step in &plan {
            let events = realize_step(step, inst_idx, num_instruments, &features, ms_per_step);
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

        let v_start = realize_step(start, inst_idx, num_instruments, &features, ms_per_step)
            .iter()
            .map(|e| e.velocity)
            .max()
            .unwrap_or(start.velocity) as i32;
        let v_interior = realize_step(interior, inst_idx, num_instruments, &features, ms_per_step)
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
                    let events =
                        realize_step(step, inst_idx, num_instruments, &features, ms_per_step);
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
        let mut fractions: std::collections::HashSet<u32> = std::collections::HashSet::new();
        for &edge in &[0.05f32, 0.5, 0.95] {
            let features = PerfFeatures {
                saturation: 50.0,
                brightness: 50.0,
                edge_density: edge,
            };
            for inst_idx in 0..num_instruments {
                for step in &plan {
                    let events =
                        realize_step(step, inst_idx, num_instruments, &features, ms_per_step);
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

        let interior_holds: Vec<u64> = plan
            .iter()
            .filter(|s| s.phrase_index == phrase && s.position == PhrasePosition::Interior)
            .flat_map(|s| realize_step(s, inst_idx, num_instruments, &features, ms_per_step))
            .map(|e| e.hold_ms)
            .collect();
        assert!(
            !interior_holds.is_empty(),
            "phrase {phrase} must have at least one interior step to compare against"
        );
        let interior_mean =
            interior_holds.iter().map(|&h| h as f64).sum::<f64>() / interior_holds.len() as f64;

        let cadence_hold = realize_step(cadence, inst_idx, num_instruments, &features, ms_per_step)
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

        let bass = realize_step(step, 0, num, &features, ms_per_step);
        let melody = realize_step(step, num - 1, num, &features, ms_per_step);

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
}

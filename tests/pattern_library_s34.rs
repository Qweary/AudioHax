//! tests/pattern_library_s34.rs — the S34 PATTERN-LIBRARY SLICE-2 INTEGRATION NET.
//!
//! Slice 2 landed three generator-backed accompaniment mechanisms in `src/`:
//!   - Part (A) `register_octaves` on a `FigurationOnset`: a whole-octave register shift
//!     applied inside `chord_engine::figured_bed` (the oom-pah/stride bass split).
//!   - Part (B) the `BassPatternSpec`/`BassPatternKind` seam: a `walking_bass` /
//!     `pedal_bass` generator the Bass arm of `realize_rhythm` dispatches to off the
//!     section's RESOLVED `bass_pattern_resolved`.
//!
//! The in-`src/` unit tests already exercise the helpers directly. THIS net is the
//! end-to-end integration witness: it drives the PUBLIC `realize_step` over hand-built
//! RNG-free `Section`/`StepContext` fixtures (the `tests/figuration_s20.rs` /
//! `chord_engine.rs::s34_*` discipline) and asserts the MUSICAL PROPERTIES the planner→
//! realizer must hold — not "it produced notes", but: the walking line arrives on the
//! next root, every walking tone is diatonic, the pedal holds one pitch under changing
//! harmony, a `-1` register shift lands exactly 12 semitones below the `0` counterpart and
//! keeps its pitch class, and the default (no-pattern / register_octaves==0) compose path
//! is byte-identical to pre-S34.
//!
//! DETERMINISTIC + HEADLESS — no `thread_rng`, no disk. The Bass/Pad arms are pure
//! functions of the fixture. Run under DEFAULT features:  cargo test --test pattern_library_s34

use audiohax::chord_engine::RhythmMotto;
use audiohax::chord_engine::{
    realize_step, Chord, NoteEvent, PerfFeatures, PhrasePosition, StepPlan,
};
use audiohax::composition::{
    BassPatternKind, BassPatternSpec, CadenceStrength, FigurationOnset, FigurationSpec,
    KeyTempoPlan, LayerRole, OrchestrationProfile, ResolutionPolicy, Section, StepContext,
    ThematicRole, ThemeVariation,
};

// ─────────────────────────────────────────────────────────────────────────────
// Constants + shared fixtures (RNG-free, hand-built — the SyntheticSource discipline).
// ─────────────────────────────────────────────────────────────────────────────

const MS_PER_STEP: u64 = 1000;
// The pad/fill band [FILL_REGISTER_FLOOR, MELODY_REGISTER_FLOOR); restated as literals to
// keep this an integration (public-surface) test (module-private constants in chord_engine).
const FILL_FLOOR: u8 = 55;
const FILL_CEIL: u8 = 67;
// The bass register floor; the bass band seats AT or above 36 and stays below the fill floor.
// A walking/pedal bass note must sit comfortably in this band (well below the pad bed at 55).
const BASS_CEIL: u8 = 54;
// The engine's synthesizable MIDI clamp (seat_pc_in_register / apply_register_octaves share it).
const MIDI_LO: u8 = 24;
const MIDI_HI: u8 = 108;

fn chord(name: &str, notes: Vec<u8>) -> Chord {
    Chord {
        name: name.to_string(),
        notes,
    }
}

/// I in C major: C E G — root pc 0.
fn c_major_triad() -> Chord {
    chord("I", vec![60, 64, 67])
}
/// V in C major: G B D — root pc 7.
fn g_major_triad() -> Chord {
    chord("V", vec![67, 71, 74])
}
/// ii in C major: D F A — root pc 2.
fn d_minor_triad() -> Chord {
    chord("ii", vec![62, 65, 69])
}
/// IV in C major: F A C — root pc 5.
fn f_major_triad() -> Chord {
    chord("IV", vec![65, 69, 72])
}
/// C major-7 — three inner tones (E,G,B) so the figured bed seats a full 3-voice cell.
fn c_major7() -> Chord {
    chord("Imaj7", vec![60, 64, 67, 71])
}

fn key_tempo() -> KeyTempoPlan {
    KeyTempoPlan {
        home_root_midi: 60, // C → tonic pc 0
        home_mode: "Ionian".to_string(),
        base_ms_per_step: MS_PER_STEP,
        key_scheme: vec![0],
        tempo_scheme: vec![MS_PER_STEP],
    }
}

fn perf() -> PerfFeatures {
    PerfFeatures {
        saturation: 60.0,
        brightness: 55.0,
        edge_density: 0.03,
    }
}

/// An INTERIOR step (never cadence/phrase-start) so the Bass/Pad GENERATOR arms — not the
/// cadence ring — are the realized path. Low `position_in_phrase` keeps it off `pre_cadence`.
fn interior_step(c: Chord, position_in_phrase: usize) -> StepPlan {
    StepPlan {
        chord: c,
        phrase_index: 0,
        position_in_phrase,
        phrase_len: 8,
        position: PhrasePosition::Interior,
        velocity: 80,
    }
}

/// A 4-instrument orchestration profile (inst 0→Bass, 1→Pad, 2→HarmonicFill, 3→Melody)
/// carrying the RESOLVED figuration and/or bass pattern the planner would have stamped.
fn profile(
    figuration_resolved: Option<FigurationSpec>,
    bass_pattern_resolved: Option<BassPatternSpec>,
) -> OrchestrationProfile {
    OrchestrationProfile {
        id: "s34_net".to_string(),
        layers: vec![
            LayerRole::Bass,
            LayerRole::Pad,
            LayerRole::HarmonicFill,
            LayerRole::Melody,
        ],
        density: 0.6,
        pad_voices: 3,
        figuration: None,
        figuration_resolved,
        bass_pattern: None,
        bass_pattern_resolved,
        prominence: Vec::new(),
        motto: RhythmMotto::neutral(),
    }
}

/// Build an owned home-key C-major section carrying `steps` + `prof`.
fn section(steps: Vec<StepPlan>, prof: OrchestrationProfile) -> Section {
    Section {
        label: "A".to_string(),
        step_len: steps.len(),
        thematic_role: ThematicRole::Statement,
        key_offset_semitones: 0,
        ms_per_step: MS_PER_STEP,
        mode: "Ionian".to_string(),
        progression: vec![],
        theme: None,
        variation: ThemeVariation::Identity,
        boundary_cadence: CadenceStrength::Perfect,
        pivot: false,
        resolution: ResolutionPolicy::Resolve,
        density: 0.6,
        orchestration: prof,
        steps,
    }
}

/// Realize instrument `inst` (0=Bass, 1=Pad) on step `si` of `sec`, through the PUBLIC
/// `realize_step` — the only public surface the generators are observable through.
fn realize(sec: &Section, si: usize, inst: usize) -> Vec<NoteEvent> {
    let kt = key_tempo();
    let ctx = StepContext {
        section: sec,
        step_in_section: si,
        theme: None,
        key_tempo: &kt,
        prev_key_offset_semitones: None,
    };
    realize_step(&sec.steps[si], inst, 4, &perf(), MS_PER_STEP, &ctx)
}

fn realize_bass(sec: &Section, si: usize) -> Vec<NoteEvent> {
    realize(sec, si, 0)
}
fn realize_pad(sec: &Section, si: usize) -> Vec<NoteEvent> {
    realize(sec, si, 1)
}

/// The 7 diatonic pitch classes of C-major (Ionian over tonic pc 0) — the legal walking set.
const C_MAJOR_PCS: [u8; 7] = [0, 2, 4, 5, 7, 9, 11];

// ═════════════════════════════════════════════════════════════════════════════
// PART (A) — register_octaves / oom-pah / stride, via the public Pad arm.
// ═════════════════════════════════════════════════════════════════════════════

/// A figuration whose two onsets name the SAME seated index but differ in `register_octaves`
/// (0 vs -1). Identical otherwise, so the only realized delta is the octave shift.
fn split_figuration() -> FigurationSpec {
    FigurationSpec {
        id: "oom_pah_test".to_string(),
        onsets: vec![
            // "oom": tone 0, dropped an octave.
            FigurationOnset {
                at: 0.0,
                tone: 0,
                hold_frac: 0.4,
                register_octaves: -1,
            },
            // "pah": tone 0 again (SAME seated pitch), in-band (no shift) — the 12-semitone
            // reference for the oom.
            FigurationOnset {
                at: 0.5,
                tone: 0,
                hold_frac: 0.4,
                register_octaves: 0,
            },
        ],
        voices: 3,
    }
}

/// PROPERTY: a `register_octaves: -1` onset realizes EXACTLY 12 semitones below the
/// `register_octaves: 0` onset that names the SAME seated tone. The shift is whole-octave,
/// so the shifted tone keeps its pitch CLASS (still a chord tone), and the "oom" is strictly
/// LOWER than the in-band "pah". This is the headline register-shift property end-to-end.
#[test]
fn register_octaves_minus_one_is_exactly_an_octave_below() {
    let sec = section(
        vec![interior_step(c_major_triad(), 1)],
        profile(Some(split_figuration()), None),
    );
    let evs = realize_pad(&sec, 0);
    assert_eq!(
        evs.len(),
        2,
        "the 2-onset split figure emits exactly 2 events"
    );
    // onset order is preserved: [0] is the at=0.0 oom (-1), [1] is the at=0.5 pah (0).
    let oom = evs[0].note;
    let pah = evs[1].note;
    assert_eq!(
        oom + 12,
        pah,
        "the register_octaves:-1 oom ({oom}) must be EXACTLY 12 semitones below the \
         register_octaves:0 pah ({pah})"
    );
    assert!(oom < pah, "the oom must be strictly lower than the pah");
    // The shift preserves pitch class (a chord tone of the current chord, root-skipped).
    assert_eq!(
        oom % 12,
        pah % 12,
        "a whole-octave shift preserves the pitch class"
    );
}

/// PROPERTY: register_octaves==0 is a NO-OP. A figure whose onsets are all in-band realizes
/// the in-band note for that seated index, and the at=0.5 pah here (tone 0, octave 0) sits in
/// the pad band [55,67) — proving the unshifted figured bed is unmoved by the new field.
#[test]
fn register_octaves_zero_stays_in_band() {
    let sec = section(
        vec![interior_step(c_major_triad(), 1)],
        profile(Some(split_figuration()), None),
    );
    let evs = realize_pad(&sec, 0);
    let pah = evs[1].note; // register_octaves: 0
    assert!(
        (FILL_FLOOR..FILL_CEIL).contains(&pah),
        "the register_octaves:0 onset stays in the pad band [{FILL_FLOOR},{FILL_CEIL}), got {pah}"
    );
}

/// PROPERTY: every register-shifted onset is still a chord tone — a whole-octave shift never
/// leaves the chord. The figured bed seats root-skipped inner tones; every emitted pc (shifted
/// or not) must be one of the current chord's root-skipped inner pitch classes.
#[test]
fn register_shift_stays_chord_tone() {
    let cur = c_major7(); // E G B inner tones → pcs {4,7,11}
    let sec = section(
        vec![interior_step(cur.clone(), 1)],
        profile(Some(split_figuration()), None),
    );
    let evs = realize_pad(&sec, 0);
    let inner_pcs: Vec<u8> = cur.notes[1..].iter().map(|n| n % 12).collect();
    for ev in &evs {
        assert!(
            inner_pcs.contains(&(ev.note % 12)),
            "shifted figured note {} (pc {}) must be a root-skipped inner chord tone of {:?} (pcs {:?})",
            ev.note,
            ev.note % 12,
            cur.notes,
            inner_pcs,
        );
    }
}

/// PROPERTY: an adversarial out-of-range register shift CLAMPS into the engine's [24,108] MIDI
/// range — never panics, never wraps. A `-9` shift on a fill-band seat (~55..67) would underflow
/// to a negative MIDI value without the clamp; assert the realized note is a valid clamped pitch.
#[test]
fn register_shift_clamped_to_midi_range() {
    let fig = FigurationSpec {
        id: "adversarial".to_string(),
        onsets: vec![
            FigurationOnset {
                at: 0.0,
                tone: 0,
                hold_frac: 0.4,
                register_octaves: -9, // would be far below 0 without the clamp
            },
            FigurationOnset {
                at: 0.5,
                tone: 1,
                hold_frac: 0.4,
                register_octaves: 9, // would be far above 108 without the clamp
            },
        ],
        voices: 3,
    };
    let sec = section(vec![interior_step(c_major7(), 1)], profile(Some(fig), None));
    let evs = realize_pad(&sec, 0); // must not panic
    assert_eq!(evs.len(), 2);
    for ev in &evs {
        assert!(
            (MIDI_LO..=MIDI_HI).contains(&ev.note),
            "register-shifted note {} must clamp into [{MIDI_LO},{MIDI_HI}]",
            ev.note
        );
    }
}

/// PROPERTY: the register-split figure emits exactly `onsets.len()` events, all within the step
/// plus the ≤10% Pad over-run cap (no unbounded burst from the new field).
#[test]
fn register_split_bounded_burst() {
    let fig = split_figuration();
    let n = fig.onsets.len();
    let sec = section(
        vec![interior_step(c_major_triad(), 1)],
        profile(Some(fig), None),
    );
    let evs = realize_pad(&sec, 0);
    assert_eq!(evs.len(), n, "the figure emits exactly onsets.len() events");
    assert!(
        (2..=4).contains(&evs.len()),
        "the burst stays bounded 2..=4"
    );
    let cap = ((MS_PER_STEP as f32) * 1.2).round() as u64;
    for ev in &evs {
        assert!(
            ev.offset_ms + ev.hold_ms <= cap,
            "onset (off {} + hold {}) must stay within the legato cap {cap}",
            ev.offset_ms,
            ev.hold_ms
        );
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// PART (B) — walking bass, via the public Bass arm.
// ═════════════════════════════════════════════════════════════════════════════

fn walking(density: u8) -> BassPatternSpec {
    BassPatternSpec {
        id: "walking".to_string(),
        kind: BassPatternKind::Walking,
        density,
        pedal_degree: 1,
    }
}

/// PROPERTY: across a 2-chord section (C → G), the walking line OPENS on the current chord's
/// root (the strong-beat onset is a chord tone), and the NEXT step's downbeat ARRIVES on the
/// next chord's root pc — the target-seeking arrival the walking bass exists to produce.
#[test]
fn walking_bass_arrives_on_next_root() {
    let sec = section(
        vec![
            interior_step(c_major_triad(), 1), // root pc 0
            interior_step(g_major_triad(), 2), // root pc 7
        ],
        profile(None, Some(walking(4))),
    );
    let step0 = realize_bass(&sec, 0);
    let step1 = realize_bass(&sec, 1);
    assert!(!step0.is_empty() && !step1.is_empty());
    // Strong-beat onset of step 0 is the C root (pc 0).
    assert_eq!(
        step0[0].note % 12,
        0,
        "the walking line's strong beat opens on the current chord root (C, pc 0)"
    );
    // The NEXT downbeat lands on the next chord's root (G, pc 7) — the arrival.
    assert_eq!(
        step1[0].note % 12,
        7,
        "the next step's downbeat ARRIVES on the next chord root (G, pc 7)"
    );
}

/// PROPERTY: `density` controls the onset count — a density-2 walk emits 2 onsets, a density-4
/// walk emits 4, over the same chord pair.
#[test]
fn walking_bass_density_controls_onset_count() {
    for d in [2u8, 4u8] {
        let sec = section(
            vec![
                interior_step(c_major_triad(), 1),
                interior_step(g_major_triad(), 2),
            ],
            profile(None, Some(walking(d))),
        );
        let evs = realize_bass(&sec, 0);
        assert_eq!(
            evs.len(),
            d as usize,
            "a density-{d} walk emits {d} onsets, got {}",
            evs.len()
        );
    }
}

/// PROPERTY: every walking tone is DIATONIC to the section scale (C-major) — no chromatic
/// approach (OD-2: diatonic-only for Slice 2). Asserted over a multi-chord stream so the walk
/// crosses several targets.
#[test]
fn walking_tones_are_diatonic() {
    let sec = section(
        vec![
            interior_step(c_major_triad(), 1),
            interior_step(f_major_triad(), 2),
            interior_step(g_major_triad(), 3),
            interior_step(c_major_triad(), 4),
        ],
        profile(None, Some(walking(4))),
    );
    for si in 0..sec.steps.len() {
        for ev in realize_bass(&sec, si) {
            assert!(
                C_MAJOR_PCS.contains(&(ev.note % 12)),
                "walking tone {} (pc {}) at step {si} must be DIATONIC to C-major {C_MAJOR_PCS:?} \
                 (no chromatic approach — OD-2)",
                ev.note,
                ev.note % 12
            );
        }
    }
}

/// PROPERTY: the walking line stays in the BASS register — every onset sits below the pad bed
/// (≤ BASS_CEIL, well under the fill floor at 55). It is a bass line, not an inner figure.
#[test]
fn walking_line_stays_in_bass_register() {
    let sec = section(
        vec![
            interior_step(c_major_triad(), 1),
            interior_step(g_major_triad(), 2),
        ],
        profile(None, Some(walking(4))),
    );
    for si in 0..sec.steps.len() {
        for ev in realize_bass(&sec, si) {
            assert!(
                ev.note <= BASS_CEIL,
                "walking tone {} at step {si} must stay in the bass register (<= {BASS_CEIL})",
                ev.note
            );
        }
    }
}

/// PROPERTY: at a section's LAST step (no next chord) the walking arm does not invent a target
/// or panic — it falls back to a within-chord line that still OPENS on the current root. The
/// §R-B end-of-section fallback.
#[test]
fn walking_bass_end_of_section_falls_back() {
    let sec = section(
        vec![interior_step(c_major_triad(), 1)], // single step → next is None
        profile(None, Some(walking(4))),
    );
    let evs = realize_bass(&sec, 0); // must not panic
    assert_eq!(evs.len(), 4, "the fallback still emits density onsets");
    assert_eq!(
        evs[0].note % 12,
        0,
        "the end-of-section fallback opens on the current root (C)"
    );
    // Diatonic + in-register even on the fallback path.
    for ev in &evs {
        assert!(C_MAJOR_PCS.contains(&(ev.note % 12)));
        assert!(ev.note <= BASS_CEIL);
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// PART (B) — pedal point, via the public Bass arm.
// ═════════════════════════════════════════════════════════════════════════════

fn pedal(degree: u8) -> BassPatternSpec {
    BassPatternSpec {
        id: "pedal".to_string(),
        kind: BassPatternKind::Pedal,
        density: 2,
        pedal_degree: degree,
    }
}

/// PROPERTY: the pedal point holds a SINGLE constant pitch == the key's tonic (pedal_degree 1)
/// across a 3-chord span (C → ii → V) while the UPPER harmony changes. The harmony moves above;
/// the bass does not.
#[test]
fn pedal_point_holds_one_pitch_under_changing_harmony() {
    let sec = section(
        vec![
            interior_step(c_major_triad(), 1),
            interior_step(d_minor_triad(), 2),
            interior_step(g_major_triad(), 3),
        ],
        profile(None, Some(pedal(1))),
    );
    let b0 = realize_bass(&sec, 0);
    let b1 = realize_bass(&sec, 1);
    let b2 = realize_bass(&sec, 2);
    assert_eq!(b0.len(), 1, "the pedal is a single sustained note per step");
    assert_eq!(b1.len(), 1);
    assert_eq!(b2.len(), 1);
    // SAME pitch on every step despite the chord change.
    assert_eq!(
        (b0[0].note, b1[0].note, b2[0].note),
        (b0[0].note, b0[0].note, b0[0].note),
        "the pedal bass is the SAME pitch on every step despite C→ii→V"
    );
    // It is the TONIC (degree 1 → pc 0).
    assert_eq!(
        b0[0].note % 12,
        0,
        "the tonic pedal sounds the tonic (C, pc 0)"
    );
    // And the UPPER harmony genuinely changed (the Pad bed differs across the C and ii steps).
    let pad0 = realize_pad(&sec, 0);
    let pad1 = realize_pad(&sec, 1);
    assert_ne!(
        pad0, pad1,
        "the upper harmony must CHANGE above the held pedal (C vs ii Pad beds differ)"
    );
}

/// PROPERTY: `pedal_degree` selects which degree is pinned — degree 1 holds the tonic (pc 0),
/// degree 5 holds the dominant (pc 7).
#[test]
fn pedal_degree_selects_tonic_or_dominant() {
    let tonic_sec = section(
        vec![interior_step(c_major_triad(), 1)],
        profile(None, Some(pedal(1))),
    );
    let dom_sec = section(
        vec![interior_step(c_major_triad(), 1)],
        profile(None, Some(pedal(5))),
    );
    assert_eq!(
        realize_bass(&tonic_sec, 0)[0].note % 12,
        0,
        "pedal_degree 1 pins the tonic (C, pc 0)"
    );
    assert_eq!(
        realize_bass(&dom_sec, 0)[0].note % 12,
        7,
        "pedal_degree 5 pins the dominant (G, pc 7)"
    );
}

/// PROPERTY: the pedal sits in the BASS register (≤ BASS_CEIL) — a low standing drone.
#[test]
fn pedal_point_stays_in_bass_register() {
    let sec = section(
        vec![interior_step(c_major_triad(), 1)],
        profile(None, Some(pedal(1))),
    );
    let note = realize_bass(&sec, 0)[0].note;
    assert!(
        note <= BASS_CEIL,
        "the pedal pitch {note} must sit in the bass register (<= {BASS_CEIL})"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// FREEZE WITNESSES (integration level) — the default compose path did not move.
// ═════════════════════════════════════════════════════════════════════════════

/// FREEZE WITNESS: a Bass profile with `bass_pattern_resolved == None` realizes the IDENTICAL
/// Bass events as a profile with NO bass-pattern at all (and as an explicit `Sustained` kind,
/// which the dispatch routes to the SAME legacy `_` arm). The pre-S34 sustained bass did not move.
#[test]
fn bass_pattern_none_is_byte_identical_to_sustained() {
    let steps = || {
        vec![
            interior_step(c_major_triad(), 1),
            interior_step(g_major_triad(), 2),
        ]
    };
    let sec_none = section(steps(), profile(None, None));
    let sustained = BassPatternSpec {
        id: "sustained".to_string(),
        kind: BassPatternKind::Sustained,
        density: 2,
        pedal_degree: 1,
    };
    let sec_sus = section(steps(), profile(None, Some(sustained)));
    for si in 0..2 {
        assert_eq!(
            realize_bass(&sec_none, si),
            realize_bass(&sec_sus, si),
            "the explicit Sustained kind must realize byte-identically to the None (legacy) \
             sustained-root path at step {si}"
        );
    }
}

/// FREEZE WITNESS: a bass pattern is INDEPENDENT of the Pad bed. A Walking Bass profile must not
/// perturb the Pad voice — the same plain Pad bed realizes identically whether or not a walking
/// bass is present (the parts are independent voices, the §6/OD-1 collision concern aside).
#[test]
fn bass_pattern_does_not_perturb_pad_bed() {
    let steps = || {
        vec![
            interior_step(c_major_triad(), 1),
            interior_step(g_major_triad(), 2),
        ]
    };
    let sec_plain = section(steps(), profile(None, None));
    let sec_walk = section(steps(), profile(None, Some(walking(4))));
    assert_eq!(
        realize_pad(&sec_plain, 0),
        realize_pad(&sec_walk, 0),
        "the Pad bed must be independent of the bass pattern"
    );
}

/// FREEZE WITNESS: a Pad bed with figuration whose onsets all carry `register_octaves: 0`
/// realizes IDENTICALLY to the same figured bed (the §2.2 default-zero guarantee, observed at
/// the integration level). A zero shift is `note = seated[idx]` — byte-identical to the figure
/// without the field. Built two structurally-equal figures (both all-zero shift) and compared.
#[test]
fn register_octaves_zero_does_not_move_the_figured_bed() {
    let zero_fig = FigurationSpec {
        id: "alberti_zero".to_string(),
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
        voices: 3,
    };
    // A second figure identical in EVERY field — the all-zero-shift reference.
    let ref_fig = zero_fig.clone();
    let sec_a = section(
        vec![interior_step(c_major7(), 1)],
        profile(Some(zero_fig), None),
    );
    let sec_b = section(
        vec![interior_step(c_major7(), 1)],
        profile(Some(ref_fig), None),
    );
    let a = realize_pad(&sec_a, 0);
    let b = realize_pad(&sec_b, 0);
    assert_eq!(
        a, b,
        "two figured beds with register_octaves==0 throughout realize byte-identically"
    );
    // And every event sits in-band (a zero shift never leaves the pad band).
    for ev in &a {
        assert!(
            (FILL_FLOOR..FILL_CEIL).contains(&ev.note),
            "register_octaves==0 keeps the figured bed in the pad band [{FILL_FLOOR},{FILL_CEIL})"
        );
    }
}

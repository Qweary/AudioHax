//! tests/figuration_s20.rs — the S20 SLICE-3a FIGURATION PROPERTY NET. The cross-lane
//! proof that the two Slice-3a mechanisms — the figuration resolution seam (Implementer
//! lane: `composition.rs`/`mapping_loader.rs`/`mappings.json`) and the `figured_bed`
//! onset→NoteEvent mapper (Music Theory lane: `chord_engine.rs`) — actually DO what the
//! build spec (`docs/spec-s20-slice3a-build.md` §5/§6/§7) promises, observed only through
//! the PUBLIC surface (`realize_step` on a Pad-role step + the `texture` `SelectTable`).
//!
//! DETERMINISTIC + HEADLESS, in the same sense as `saliency_s18.rs` / `texture_s17.rs` /
//! `composition_s15.rs`: every fixture is a hand-built RNG-free `Section`/`StepContext`,
//! and the figured Pad arm is pure (no `thread_rng`, no disk). The selection tests load
//! `assets/mappings.json` ONLY to read the shipped gate ladder + catalogue (same discipline
//! as `composition_s15.rs`'s loader-backed selection tests).
//!
//! Run under DEFAULT features (the as-built quirk: `--no-default-features` drags in a
//! feature-gated bin):  cargo test --test figuration_s20

use audiohax::chord_engine::RhythmMotto;
use audiohax::chord_engine::{
    realize_step, Chord, NoteEvent, PerfFeatures, PhrasePosition, StepPlan,
};
use audiohax::composition::{
    CadenceStrength, FigurationOnset, FigurationSpec, ImageUnderstanding, KeyTempoPlan, LayerRole,
    OrchestrationProfile, PlanMappings, ResolutionPolicy, Section, StepContext, ThematicRole,
    ThemeVariation,
};

// ─────────────────────────────────────────────────────────────────────────────
// Shared fixtures — RNG-free, hand-built (the SyntheticSource determinism discipline).
// ─────────────────────────────────────────────────────────────────────────────

const MS_PER_STEP: u64 = 200;
// The pad/fill band [FILL_REGISTER_FLOOR, MELODY_REGISTER_FLOOR); module-private constants,
// re-stated here as literals per the spec to keep this an integration (public-surface) test.
const FILL_FLOOR: u8 = 55;
const FILL_CEIL: u8 = 67;
// The legato over-run cap the figured bed shares with the block bed (PAD_OVERLAP_FRAC = 1.10).
// The §6 test only requires the looser absolute ceiling of 1.2× — assert against that so the
// net passes whether the build uses 1.10 or up to 1.20.
const ABS_CAP_FRAC: f32 = 1.2;

fn chord(name: &str, notes: Vec<u8>) -> Chord {
    Chord {
        name: name.to_string(),
        notes,
    }
}

/// C major triad — root-skipped inner tones are E(64) and G(55-seated). pcs {E,G} = {4,7}.
fn c_major_triad() -> Chord {
    chord("I", vec![60, 64, 67]) // C E G
}

/// C major-7 — inner tones are E, G, B. Three seated voices → the full Alberti cell.
fn c_major7() -> Chord {
    chord("Imaj7", vec![60, 64, 67, 71]) // C E G B
}

/// The shipped Alberti figuration spec, hand-built to match the §4.1 catalogue row.
fn alberti() -> FigurationSpec {
    FigurationSpec {
        id: "alberti".to_string(),
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
    }
}

/// The `pad_figured` profile: inst 0→Bass, 1→Pad, 2→HarmonicFill, 3→Melody, 3 pad voices,
/// carrying the RESOLVED Alberti spec (as the planner would set it at build time). inst 1
/// is therefore the figured Pad bed.
fn pad_figured() -> OrchestrationProfile {
    OrchestrationProfile {
        id: "pad_figured".to_string(),
        layers: vec![
            LayerRole::Bass,
            LayerRole::Pad,
            LayerRole::HarmonicFill,
            LayerRole::Melody,
        ],
        density: 0.62,
        pad_voices: 3,
        figuration: Some("alberti".to_string()),
        figuration_resolved: Some(alberti()),
        bass_pattern: None,
        bass_pattern_resolved: None,
        prominence: Vec::new(),
        motto: RhythmMotto::neutral(),
    }
}

/// The shipped `pad_bed` profile (the S17 block bed): same layers, NO figuration → the
/// block-bed path. `figuration_resolved: None`.
fn pad_bed() -> OrchestrationProfile {
    OrchestrationProfile {
        id: "pad_bed".to_string(),
        layers: vec![
            LayerRole::Bass,
            LayerRole::Pad,
            LayerRole::HarmonicFill,
            LayerRole::Melody,
        ],
        density: 0.55,
        pad_voices: 3,
        figuration: None,
        figuration_resolved: None,
        bass_pattern: None,
        bass_pattern_resolved: None,
        prominence: Vec::new(),
        motto: RhythmMotto::neutral(),
    }
}

/// A `pad_figured`-shaped profile whose figuration handle named a NON-EXISTENT catalogue id,
/// so the planner left `figuration_resolved == None` — the unresolved-id → block-bed case.
fn pad_figured_unresolved() -> OrchestrationProfile {
    OrchestrationProfile {
        id: "pad_figured".to_string(),
        layers: vec![
            LayerRole::Bass,
            LayerRole::Pad,
            LayerRole::HarmonicFill,
            LayerRole::Melody,
        ],
        density: 0.62,
        pad_voices: 3,
        figuration: Some("nope".to_string()),
        figuration_resolved: None,
        bass_pattern: None,
        bass_pattern_resolved: None,
        prominence: Vec::new(),
        motto: RhythmMotto::neutral(),
    }
}

/// A held, NON-CADENCE interior step (the figured arm only fires off the cadence path).
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

fn key_tempo() -> KeyTempoPlan {
    KeyTempoPlan {
        home_root_midi: 60,
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

/// Realize the Pad instrument (inst 1 of 4) on one interior step under `profile`, via the
/// PUBLIC `realize_step` — the figured arm is reached exactly as `texture_s17`/`saliency_s18`
/// reach the realizer (a hand-built `Section`/`StepContext`, no loader, no RNG).
fn realize_pad(profile: OrchestrationProfile, step: StepPlan) -> Vec<NoteEvent> {
    let kt = key_tempo();
    let section = Section {
        label: "A".to_string(),
        step_len: 1,
        thematic_role: ThematicRole::Statement,
        key_offset_semitones: 0,
        ms_per_step: MS_PER_STEP,
        mode: "Ionian".to_string(),
        progression: vec![],
        theme: None,
        variation: ThemeVariation::Identity,
        boundary_cadence: CadenceStrength::Perfect,
        // K3 identity carry: keep this fixture on the byte-frozen non-modulating path.
        pivot: false,
        resolution: ResolutionPolicy::Resolve,
        density: 0.6,
        orchestration: profile,
        steps: vec![step],
    };
    let ctx = StepContext {
        section: &section,
        step_in_section: 0,
        theme: None,
        key_tempo: &kt,
        // K3 identity carry: None prev ⇒ never a modulating boundary ⇒ pivot path dead.
        prev_key_offset_semitones: None,
    };
    // inst 1 of 4 → the Pad layer under both `pad_figured` and `pad_bed`.
    realize_step(&section.steps[0], 1, 4, &perf(), MS_PER_STEP, &ctx)
}

// ═════════════════════════════════════════════════════════════════════════════
// PART 1 — THE FIGURED-BED MAPPER (§5/§6), observed via the public Pad arm.
// ═════════════════════════════════════════════════════════════════════════════

/// §6 `figuration_emits_bounded_burst`: a resolved Alberti figured bed emits EXACTLY
/// `onsets.len()` events, bounded 2..=4 — never an unbounded count, never a single block stab.
#[test]
fn figuration_emits_bounded_burst() {
    let evs = realize_pad(pad_figured(), interior_step(c_major_triad(), 1));
    assert_eq!(
        evs.len(),
        alberti().onsets.len(),
        "the figured bed emits exactly onsets.len() events"
    );
    assert!(
        (2..=4).contains(&evs.len()),
        "the burst is bounded 2..=4, got {}",
        evs.len()
    );
}

/// §6 `figuration_onsets_are_in_step`: every onset's `offset_ms + hold_ms` stays within the
/// legato cap (`step_ms × 1.2` absolute ceiling) — no onset runs past the cap, the last
/// onset never overhangs the step beyond the established Pad over-run.
#[test]
fn figuration_onsets_are_in_step() {
    let cap = ((MS_PER_STEP as f32) * ABS_CAP_FRAC).round() as u64;
    for c in [c_major_triad(), c_major7()] {
        let evs = realize_pad(pad_figured(), interior_step(c.clone(), 1));
        for ev in &evs {
            assert!(
                ev.offset_ms + ev.hold_ms <= cap,
                "onset (off {} + hold {} = {}) must stay within the legato cap {cap} on {:?}",
                ev.offset_ms,
                ev.hold_ms,
                ev.offset_ms + ev.hold_ms,
                c.name,
            );
        }
    }
}

/// §6 `figuration_tones_are_chord_tones_in_band`: every emitted note is a chord tone of the
/// CURRENT chord, seated in the pad band [55,67). On a C-major triad the root-skipped inner
/// tones are the 3rd (E) and 5th (G), so every figured note's pc ∈ {E,G}.
#[test]
fn figuration_tones_are_chord_tones_in_band() {
    let cur = c_major_triad();
    let evs = realize_pad(pad_figured(), interior_step(cur.clone(), 1));
    // The root is skipped by the bed; assert membership in the root-skipped inner pcs.
    let inner_pcs: Vec<u8> = cur.notes[1..].iter().map(|n| n % 12).collect();
    for ev in &evs {
        assert!(
            inner_pcs.contains(&(ev.note % 12)),
            "figured note {} (pc {}) must be a root-skipped inner chord tone of {:?} (pcs {:?})",
            ev.note,
            ev.note % 12,
            cur.notes,
            inner_pcs,
        );
        assert!(
            (FILL_FLOOR..FILL_CEIL).contains(&ev.note),
            "figured note {} must sit in the pad band [{FILL_FLOOR},{FILL_CEIL})",
            ev.note
        );
    }
}

/// §6 `figured_bed_off_beat`: at least one onset has a non-zero `offset_ms` — a real off-beat
/// onset, proving the bed ANIMATES (a broken-chord cell) rather than re-striking a block stab.
/// The Alberti ¼/½/¾ onsets guarantee it.
#[test]
fn figured_bed_off_beat() {
    let evs = realize_pad(pad_figured(), interior_step(c_major_triad(), 1));
    assert!(
        evs.iter().any(|ev| ev.offset_ms > 0),
        "at least one figured onset must be off the downbeat (offset_ms > 0); got offsets {:?}",
        evs.iter().map(|e| e.offset_ms).collect::<Vec<_>>()
    );
}

/// §6 `block_bed_unchanged_when_figuration_none` (BACK-COMPAT witness): with
/// `figuration_resolved == None` the Pad arm emits the ORIGINAL S17 block bed — every event
/// at offset 0, count == seated voices (== pad_voices on a chord with enough inner tones),
/// hold == round(step_ms × 1.10). Byte-identical to pre-S20.
#[test]
fn block_bed_unchanged_when_figuration_none() {
    // C major-7 has 3 inner tones (E,G,B), so pad_voices(3) all seat → 3 block events.
    let evs = realize_pad(pad_bed(), interior_step(c_major7(), 1));
    assert_eq!(
        evs.len(),
        3,
        "the block bed holds pad_voices(3) inner tones simultaneously, got {}",
        evs.len()
    );
    let expect_hold = ((MS_PER_STEP as f32) * 1.10).round() as u64;
    for ev in &evs {
        assert_eq!(
            ev.offset_ms, 0,
            "every block-bed voice is struck at the downbeat (offset 0)"
        );
        assert_eq!(
            ev.hold_ms, expect_hold,
            "the block-bed hold is round(step_ms × 1.10) = {expect_hold}"
        );
    }
}

/// §6 `unresolved_figuration_id_falls_to_block`: a profile whose `figuration` handle named a
/// catalogue id that does not exist resolved to `figuration_resolved: None` — so the Pad arm
/// takes the block bed (no panic, no figuration). Same emission as `pad_bed`.
#[test]
fn unresolved_figuration_id_falls_to_block() {
    let evs = realize_pad(pad_figured_unresolved(), interior_step(c_major7(), 1));
    assert_eq!(
        evs.len(),
        3,
        "an unresolved figuration id falls back to the block bed (3 simultaneous voices)"
    );
    let expect_hold = ((MS_PER_STEP as f32) * 1.10).round() as u64;
    for ev in &evs {
        assert_eq!(ev.offset_ms, 0, "unresolved → block bed, all at offset 0");
        assert_eq!(ev.hold_ms, expect_hold, "unresolved → block-bed hold");
    }
}

/// §6 `tone_index_cycles_modulo_seated` (pins §5.4 — the non-triad modulo rule): the Alberti
/// onset tone indices {0,2,1,2} are taken MODULO the seated voice count, never out of bounds.
///   * On a TRIAD (2 seated inner tones) they read {0, 0, 1, 0} → 3 of 4 onsets sound the
///     SAME inner tone (the degrade-to-2-tones case), 1 sounds the other.
///   * On a 7th chord (3 seated) they read {0, 2, 1, 2} → all three inner tones appear.
/// No event is ever silent or out of band; the membership is the OOB witness.
#[test]
fn tone_index_cycles_modulo_seated() {
    // TRIAD: 2 seated → only 2 distinct pitches reachable across the 4 onsets.
    let triad = c_major_triad();
    let triad_evs = realize_pad(pad_figured(), interior_step(triad.clone(), 1));
    let mut triad_pitches: Vec<u8> = triad_evs.iter().map(|e| e.note).collect();
    triad_pitches.sort_unstable();
    triad_pitches.dedup();
    assert!(
        triad_pitches.len() <= 2,
        "a triad seats 2 inner tones; the modulo cell can sound at most 2 distinct pitches, \
         got {triad_pitches:?}"
    );
    assert!(
        !triad_pitches.is_empty(),
        "the triad figure must still sound"
    );

    // 7th chord: 3 seated → the full Alberti cell can visit 3 distinct inner tones.
    let seventh = c_major7();
    let seventh_evs = realize_pad(pad_figured(), interior_step(seventh.clone(), 1));
    let mut seventh_pitches: Vec<u8> = seventh_evs.iter().map(|e| e.note).collect();
    seventh_pitches.sort_unstable();
    seventh_pitches.dedup();
    assert!(
        seventh_pitches.len() >= 2,
        "a 7th chord seats 3 inner tones; the Alberti cell visits ≥2 distinct pitches, \
         got {seventh_pitches:?}"
    );
    // No OOB: every pitch on either chord is a seated inner tone in band (the modulo never
    // indexes past the seated set).
    for (c, evs) in [(triad, triad_evs), (seventh, seventh_evs)] {
        let inner_pcs: Vec<u8> = c.notes[1..].iter().map(|n| n % 12).collect();
        for ev in &evs {
            assert!(
                inner_pcs.contains(&(ev.note % 12)) && (FILL_FLOOR..FILL_CEIL).contains(&ev.note),
                "modulo-cycled note {} on {:?} stays an in-band inner tone (no OOB)",
                ev.note,
                c.name
            );
        }
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// PART 2 — THE SALIENCY GATE LADDER (§4.3), over the shipped `texture` SelectTable.
// ═════════════════════════════════════════════════════════════════════════════
//
// Loads `assets/mappings.json` ONLY to read the shipped gate ladder + figuration catalogue —
// the same loader-backed selection discipline as `composition_s15.rs`'s selection tests.

fn plan_mappings() -> PlanMappings {
    audiohax::mapping_loader::load_mappings("assets/mappings.json")
        .expect("mappings load")
        .composition
        .clone()
        .expect("composition block present")
        .into()
}

/// §6 `texture_selects_pad_figured_on_salient_subject`: the gate ladder is first-match-wins.
///   * subject_energy ≥ 0.45 ∧ fg_bg_contrast ≥ 0.25 → "pad_figured", and its figuration
///     handle resolves to the alberti onsets in the catalogue.
///   * the unchanged S18 case (foreground_energy ≥ 0.35 ∧ fg_bg_contrast ≥ 0.20, no salient
///     subject) → "pad_bed_counter" — the S18 rule is NOT broken by the prepended rule.
///   * a calm image → the "pad_bed" default (NOT pad_figured).
#[test]
fn texture_selects_pad_figured_on_salient_subject() {
    let pm = plan_mappings();
    let texture = &pm.texture;

    // Salient subject crosses the new (first) rule → pad_figured.
    let salient = ImageUnderstanding {
        subject_energy: 0.5,
        fg_bg_contrast: 0.3,
        ..ImageUnderstanding::neutral()
    };
    assert_eq!(
        texture.select(&salient),
        "pad_figured",
        "a salient subject (subject_energy ≥0.45 ∧ fg_bg_contrast ≥0.25) selects pad_figured"
    );

    // The chosen profile's figuration handle resolves to the alberti onsets in the catalogue.
    let prof = pm
        .texture_catalogue
        .iter()
        .find(|p| p.id == "pad_figured")
        .expect("pad_figured profile present");
    let handle = prof
        .figuration
        .as_deref()
        .expect("pad_figured carries a figuration handle");
    let spec = pm
        .figuration_catalogue
        .iter()
        .find(|f| f.id == handle)
        .expect("the figuration handle resolves to a catalogue row");
    assert_eq!(
        spec.id, "alberti",
        "pad_figured's figuration handle resolves to the alberti row"
    );
    assert_eq!(
        spec.onsets.len(),
        4,
        "the alberti row carries the 4-onset cell, got {}",
        spec.onsets.len()
    );
    let ats: Vec<f32> = spec.onsets.iter().map(|o| o.at).collect();
    assert_eq!(
        ats,
        vec![0.0, 0.25, 0.5, 0.75],
        "the alberti onsets land on the quarter-step grid"
    );

    // The S18 rule is unbroken: a busy foreground with a real subject but NO salient-subject
    // energy still selects pad_bed_counter (the ladder's second rule), not pad_figured.
    let s18 = ImageUnderstanding {
        foreground_energy: 0.4,
        fg_bg_contrast: 0.25,
        subject_energy: 0.0,
        ..ImageUnderstanding::neutral()
    };
    assert_eq!(
        texture.select(&s18),
        "pad_bed_counter",
        "the unchanged S18 rule still selects pad_bed_counter (the ladder order is intact)"
    );

    // A calm image falls through to the pad_bed default — NOT pad_figured.
    let calm = ImageUnderstanding {
        subject_energy: 0.1,
        fg_bg_contrast: 0.05,
        foreground_energy: 0.1,
        ..ImageUnderstanding::neutral()
    };
    let calm_pick = texture.select(&calm);
    assert_eq!(
        calm_pick, "pad_bed",
        "a calm image selects the pad_bed default"
    );
    assert_ne!(
        calm_pick, "pad_figured",
        "a calm image must NOT select pad_figured"
    );
}

// Run under DEFAULT features (the integration harness builds the feature-gated bin, so
// `--no-default-features` cannot RUN this net):
//   cargo test --test figuration_s20

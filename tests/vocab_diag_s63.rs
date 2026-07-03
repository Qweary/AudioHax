//! S63 VOCABULARY-BREADTH DIAGNOSTIC (read-only; Music-Theory-owned craft).
//!
//! Operator report after listening across several images: the NOTES now differ per image
//! (S62 un-gate landed), BUT "the rhythms sound repetitive" and "the overall feel of the
//! accompaniment and melody feels like it's the same thing with variations." Operator theory,
//! verbatim: "our library of unique seeds to start with and then vary are lacking on all fronts."
//!
//! This harness INVENTORIES every generative vocabulary ("seed library") in the image->music
//! pipeline and measures, empirically across the six bundled images, three numbers for each:
//!   * SIZE               — how many distinct AUTHORED entries exist.
//!   * CORPUS UTILIZATION — how many distinct entries actually FIRE across the six images.
//!   * WITHIN-PIECE       — within ONE piece, does the element cycle/develop or loop one entry.
//!
//! It runs the REAL planner path (`CompositionPlanner::plan`) per image and inspects the
//! `CompositionPlan` — the same authoritative seam `motif_diag_s62.rs` used. Selector IDs that
//! aren't stored on the plan (prominence) are read back through the PUBLIC `planner.plan_mappings()`
//! SelectTables after re-seating the affect composite exactly as `plan()` does.
//!
//! Run: `cargo test --release vocab_diag_s63 -- --nocapture` (DEFAULT features).
//!
//! ── S63 CI-HARDENING AUDIT (Test Engineer pass) ─────────────────────────────────────────────
//! This harness is committed with the S63 build, so it runs in CI on Linux AND macOS. AUDIT
//! RESULT: it carries NO fractional/audio-magnitude assertions — every measured quantity is
//! PRINTED, and the only pre-existing failure modes are structural `expect`/`panic` on a failed
//! image load or a missing `composition` block (not FP-sensitive). So it was already free of the
//! S62.1 macOS-FP-fragility class (a per-image exact audio metric at a tight eps). To give it
//! actual CI regression TEETH (rather than shipping a pure printer), the `vocab_diag_s63_asserts`
//! test below adds ONLY cross-platform-safe EXACT-INTEGER / STRUCTURAL floors — catalogue
//! non-emptiness, per-piece motto non-neutrality + cell bounds, section-count floor, and
//! resolved-figuration catalogue-membership. NO fractional/ms/pitch/velocity value is ever pinned
//! (those diverge ~0.03 across platforms — S62.1); the structural facts (which id/cell/kind fired,
//! how many sections) are identical on every platform by construction.

use audiohax::chord_engine::MotifArchetype;
use audiohax::composition::{
    CompositionPlan, CompositionPlanner, ImageUnderstanding, PlanMappings,
};
use audiohax::mapping_loader::{load_mappings, MappingTable};
use audiohax::pure_analysis::{load_pure_image, understand_image_pure, PureImageSource};
use std::collections::BTreeSet;

const MAPPINGS_PATH: &str = "assets/mappings.json";

const IMAGES: &[&str] = &[
    "AudioHaxImg1.jpg",
    "AudioHaxImg2.jpg",
    "AudioHaxImg3.jpg",
    "example.jpg",
    "Lena.png",
    "magicstudio-art.jpg",
];

// ── Affect composite (identical to assets/mappings.json composition.affect weights). Re-seated so
// the affect-reading SelectTables (character/texture/prominence) evaluate the same way plan() does.
fn affect_arousal(u: &ImageUnderstanding) -> f32 {
    (0.45 * (u.avg_saturation / 100.0)
        + 0.25 * u.colorfulness
        + 0.20 * u.edge_activity
        + 0.10 * u.complexity)
        .clamp(0.0, 1.0)
}
fn affect_valence(u: &ImageUnderstanding) -> f32 {
    (0.70 * (u.avg_brightness / 100.0)
        + 0.20 * (u.avg_saturation / 100.0)
        + 0.10 * (0.5 + 0.5 * u.fg_bg_contrast))
        .clamp(0.0, 1.0)
}

// ── Faithful reconstruction of composition::pick_archetype (composition.rs:1784) — used only to
// LABEL the theme's contour class in the table (the concrete motif itself is read off the real plan).
fn pick_archetype_recon(u: &ImageUnderstanding, arousal: f32, valence: f32) -> MotifArchetype {
    let upper = u.vertical_emphasis >= 0.5;
    let warm = u.dominant_hue.rem_euclid(360.0) < 180.0;
    match (arousal >= 0.5, valence >= 0.5) {
        (true, true) => {
            if upper || warm {
                MotifArchetype::RisingSequence
            } else {
                MotifArchetype::Ascent
            }
        }
        (false, true) => {
            if upper || warm {
                MotifArchetype::Arch
            } else {
                MotifArchetype::InvertedArch
            }
        }
        (false, false) => {
            if upper || warm {
                MotifArchetype::LeapStep
            } else {
                MotifArchetype::Descent
            }
        }
        (true, false) => {
            if upper || warm {
                MotifArchetype::Pendulum
            } else {
                MotifArchetype::NeighborTurn
            }
        }
    }
}

fn understanding_for(name: &str) -> ImageUnderstanding {
    let img = load_pure_image(&PureImageSource::Preselected(name.to_string()))
        .unwrap_or_else(|e| panic!("load {name}: {e}"));
    understand_image_pure(img.as_rgb()).unwrap_or_else(|e| panic!("understand {name}: {e}"))
}

/// Re-seat the affect composite onto a working copy, exactly as `plan()` does, so the
/// affect-reading SelectTables (character/texture/prominence) select correctly.
fn seat_affect(u: &ImageUnderstanding) -> ImageUnderstanding {
    let mut w = u.clone();
    w.affect_arousal = affect_arousal(u);
    w.affect_valence = affect_valence(u);
    w
}

fn plan_mappings(m: &MappingTable) -> PlanMappings {
    m.composition
        .clone()
        .expect("composition block present in mappings.json")
        .into()
}

/// The mode->family bucket `pick_progression` uses (warm/cool/neutral).
fn progression_family(mode: &str) -> &'static str {
    let warm = ["Ionian", "Lydian", "Mixolydian"];
    let cool = ["Dorian", "Aeolian", "Phrygian"];
    if warm.contains(&mode) {
        "warm"
    } else if cool.contains(&mode) {
        "cool"
    } else {
        "neutral"
    }
}

fn distinct<T: Ord + Clone>(v: &[T]) -> usize {
    v.iter().cloned().collect::<BTreeSet<T>>().len()
}

// One row of collected per-image facts.
struct Row {
    name: String,
    form: String,
    character: String,
    mode: String,
    family: String,
    texture_id: String,
    prominence_id: String,
    bass_id: String,
    motto_cell: Option<usize>,
    archetype: String,
    theme_len_notes: usize,
    theme_sum_dur: usize,
    // per-section within-piece observations:
    figuration_ids: Vec<String>, // one per section (None -> "block/none")
    progression_sigs: Vec<String>, // one per section (roman-numeral join)
    key_offsets: Vec<i8>,
    section_roles: Vec<String>,
    n_sections: usize,
}

fn collect(planner: &CompositionPlanner, m: &MappingTable, name: &str) -> Row {
    let u = understanding_for(name);
    let seated = seat_affect(&u);
    let plan: CompositionPlan = planner.plan(&u, m);

    // Selector IDs the plan stores directly:
    let form = plan.form.clone();
    let character = format!("{:?}", plan.character);
    let mode = plan.key_tempo.home_mode.clone();
    let family = progression_family(&mode).to_string();

    // Texture profile id (uniform per piece — set once, cloned onto every section).
    let texture_id = plan
        .sections
        .first()
        .map(|s| s.orchestration.id.clone())
        .unwrap_or_else(|| "?".into());

    // Prominence id — not stored on the plan; select it back through the public SelectTable.
    let prominence_id = planner.plan_mappings().prominence.select(&seated);

    // Bass pattern id (uniform per piece).
    let bass_id = plan
        .sections
        .first()
        .and_then(|s| {
            s.orchestration
                .bass_pattern_resolved
                .as_ref()
                .map(|b| b.id.clone())
        })
        .unwrap_or_else(|| "sustained(default)".into());

    // Per-piece motto cell (S53) — read the engine-consumed value off section 0.
    let motto_cell = plan.sections.first().and_then(|s| s.motto().cell_index);

    // Theme archetype label + concrete motif shape (read off the real plan).
    let archetype = format!(
        "{:?}",
        pick_archetype_recon(&seated, seated.affect_arousal, seated.affect_valence)
    );
    let (theme_len_notes, theme_sum_dur) = plan
        .themes
        .first()
        .map(|t| {
            (
                t.motif.len(),
                t.motif.iter().map(|n| n.dur_steps as usize).sum::<usize>(),
            )
        })
        .unwrap_or((0, 0));

    // Per-section within-piece vocabularies.
    let figuration_ids: Vec<String> = plan
        .sections
        .iter()
        .map(|s| {
            s.orchestration
                .figuration_resolved
                .as_ref()
                .map(|f| f.id.clone())
                .unwrap_or_else(|| "block/none".into())
        })
        .collect();
    let progression_sigs: Vec<String> = plan
        .sections
        .iter()
        .map(|s| s.progression.join("-"))
        .collect();
    let key_offsets: Vec<i8> = plan
        .sections
        .iter()
        .map(|s| s.key_offset_semitones)
        .collect();
    let section_roles: Vec<String> = plan
        .sections
        .iter()
        .map(|s| format!("{:?}", s.thematic_role))
        .collect();
    let n_sections = plan.sections.len();

    Row {
        name: name.to_string(),
        form,
        character,
        mode,
        family,
        texture_id,
        prominence_id,
        bass_id,
        motto_cell,
        archetype,
        theme_len_notes,
        theme_sum_dur,
        figuration_ids,
        progression_sigs,
        key_offsets,
        section_roles,
        n_sections,
    }
}

#[test]
fn vocab_diag_s63_inventory() {
    let m = load_mappings(MAPPINGS_PATH).expect("mappings load");
    let planner = CompositionPlanner::new(plan_mappings(&m));
    let pm = planner.plan_mappings();

    // ── AUTHORED SIZES (catalogue lengths — the raw seed-library sizes). ──
    let size_forms = pm.form_catalogue.len();
    let size_texture = pm.texture_catalogue.len();
    let size_figuration = pm.figuration_catalogue.len();
    let size_bass = pm.bass_pattern_catalogue.len();
    let size_prominence = pm.prominence_catalogue.len();
    let size_keyscheme = pm.key_scheme_catalogue.len();
    let size_archetypes = 8usize; // MotifArchetype variants (chord_engine.rs:3287)
    let cells_per_arch = 4usize; // authored rhythm_cells() per archetype (all K==4)
    let warm = m
        .global
        .progression_families
        .get("warm")
        .map(|v| v.len())
        .unwrap_or(0);
    let cool = m
        .global
        .progression_families
        .get("cool")
        .map(|v| v.len())
        .unwrap_or(0);
    let neutral = m
        .global
        .progression_families
        .get("neutral")
        .map(|v| v.len())
        .unwrap_or(0);
    // Reachable characters (SelectTable default + rule picks).
    let mut char_reach: BTreeSet<String> = BTreeSet::new();
    char_reach.insert(pm.character.default.clone());
    for r in &pm.character.rules {
        char_reach.insert(r.pick.clone());
    }
    // Reachable texture / prominence / form ids in their SelectTables.
    let mut tex_reach: BTreeSet<String> = BTreeSet::new();
    tex_reach.insert(pm.texture.default.clone());
    for r in &pm.texture.rules {
        tex_reach.insert(r.pick.clone());
    }
    let mut form_reach: BTreeSet<String> = BTreeSet::new();
    form_reach.insert(pm.form.default.clone());
    for r in &pm.form.rules {
        form_reach.insert(r.pick.clone());
    }

    println!("\n================ S63 VOCABULARY-BREADTH DIAGNOSTIC ================\n");
    println!("AUTHORED SIZES (catalogue entries):");
    println!(
        "  form_catalogue          : {size_forms}   (reachable via `form` table: {})",
        form_reach.len()
    );
    println!(
        "  texture_catalogue       : {size_texture}   (reachable via `texture` table: {})",
        tex_reach.len()
    );
    println!("  figuration_catalogue    : {size_figuration}");
    println!("  bass_pattern_catalogue  : {size_bass}");
    println!("  prominence_catalogue    : {size_prominence}");
    println!("  key_scheme_catalogue    : {size_keyscheme}");
    println!("  MotifArchetype contours : {size_archetypes}  (x {cells_per_arch} rhythm cells = {} contour-gaits)", size_archetypes * cells_per_arch);
    println!(
        "  progression families    : warm {warm} / cool {cool} / neutral {neutral}  (total {})",
        warm + cool + neutral
    );
    println!(
        "  Character enum (defined) : 10   (reachable via `character` table: {})",
        char_reach.len()
    );
    println!("    reachable chars        : {char_reach:?}");

    // ── Collect per-image rows off the real planner. ──
    let rows: Vec<Row> = IMAGES.iter().map(|n| collect(&planner, &m, n)).collect();

    println!("\n---- PER-IMAGE SELECTIONS (real planner) ----");
    println!(
        "{:<20} {:>16} {:>9} {:>11} {:>16} {:>18} {:>6}",
        "image", "form", "character", "mode/fam", "texture", "prominence", "motto"
    );
    println!("{}", "-".repeat(104));
    for r in &rows {
        println!(
            "{:<20} {:>16} {:>9} {:>11} {:>16} {:>18} {:>6}",
            r.name,
            r.form,
            r.character,
            format!("{}/{}", r.mode, r.family),
            r.texture_id,
            r.prominence_id,
            r.motto_cell
                .map(|c| c.to_string())
                .unwrap_or_else(|| "none".into()),
        );
    }

    println!("\n---- THEME (pitched subject) per image ----");
    println!(
        "{:<20} {:>14} {:>7} {:>8}",
        "image", "archetype", "notes", "Sdur"
    );
    println!("{}", "-".repeat(52));
    for r in &rows {
        println!(
            "{:<20} {:>14} {:>7} {:>8}",
            r.name, r.archetype, r.theme_len_notes, r.theme_sum_dur
        );
    }

    // ── CORPUS UTILIZATION (distinct entries fired across the six images). ──
    let u_form = distinct(&rows.iter().map(|r| r.form.clone()).collect::<Vec<_>>());
    let u_char = distinct(&rows.iter().map(|r| r.character.clone()).collect::<Vec<_>>());
    let u_tex = distinct(
        &rows
            .iter()
            .map(|r| r.texture_id.clone())
            .collect::<Vec<_>>(),
    );
    let u_prom = distinct(
        &rows
            .iter()
            .map(|r| r.prominence_id.clone())
            .collect::<Vec<_>>(),
    );
    let u_bass = distinct(&rows.iter().map(|r| r.bass_id.clone()).collect::<Vec<_>>());
    let u_mode = distinct(&rows.iter().map(|r| r.mode.clone()).collect::<Vec<_>>());
    let u_family = distinct(&rows.iter().map(|r| r.family.clone()).collect::<Vec<_>>());
    let u_arch = distinct(&rows.iter().map(|r| r.archetype.clone()).collect::<Vec<_>>());
    let u_motto = distinct(&rows.iter().map(|r| r.motto_cell).collect::<Vec<_>>());
    // texture x figuration-arc identity, and (archetype, motto) motif identity:
    let u_arch_motto = distinct(
        &rows
            .iter()
            .map(|r| (r.archetype.clone(), r.motto_cell))
            .collect::<Vec<_>>(),
    );

    println!("\n---- CORPUS UTILIZATION (distinct entries fired / size) ----");
    println!(
        "  form            : {u_form}/{size_forms}   picks: {:?}",
        set_of(&rows, |r| r.form.clone())
    );
    println!(
        "  character       : {u_char}/10 (reach {})   picks: {:?}",
        char_reach.len(),
        set_of(&rows, |r| r.character.clone())
    );
    println!(
        "  mode            : {u_mode}/6    picks: {:?}",
        set_of(&rows, |r| r.mode.clone())
    );
    println!(
        "  progression fam : {u_family}/3    picks: {:?}",
        set_of(&rows, |r| r.family.clone())
    );
    println!(
        "  texture profile : {u_tex}/{size_texture}   picks: {:?}",
        set_of(&rows, |r| r.texture_id.clone())
    );
    println!(
        "  prominence      : {u_prom}/{size_prominence}    picks: {:?}",
        set_of(&rows, |r| r.prominence_id.clone())
    );
    println!(
        "  bass pattern    : {u_bass}/{size_bass}    picks: {:?}",
        set_of(&rows, |r| r.bass_id.clone())
    );
    println!(
        "  motif archetype : {u_arch}/{size_archetypes}    picks: {:?}",
        set_of(&rows, |r| r.archetype.clone())
    );
    println!(
        "  per-piece motto : {u_motto}/{cells_per_arch}    picks: {:?}",
        set_of(&rows, |r| r
            .motto_cell
            .map(|c| c.to_string())
            .unwrap_or_else(|| "none".into()))
    );
    println!("  (archetype,motto) motif identities : {u_arch_motto}/6 images");

    // ── WITHIN-PIECE VARIETY (does the element develop within one piece?). ──
    println!("\n---- WITHIN-PIECE VARIETY (per-image, across that piece's sections) ----");
    println!(
        "{:<20} {:>5} {:>28} {:>7} {:>10} {:>10}",
        "image", "secs", "figuration arc (per section)", "figDist", "motto", "keyoff"
    );
    println!("{}", "-".repeat(88));
    for r in &rows {
        let fig_distinct = distinct(&r.figuration_ids);
        let key_moves = r.key_offsets.iter().any(|&o| o != 0);
        println!(
            "{:<20} {:>5} {:>28} {:>7} {:>10} {:>10}",
            r.name,
            r.n_sections,
            r.figuration_ids.join(">"),
            format!("{fig_distinct}/{}", r.n_sections),
            format!(
                "cell {} fixed",
                r.motto_cell
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "none".into())
            ),
            if key_moves { "moves" } else { "static" },
        );
    }

    println!(
        "\n---- PROGRESSION within-piece (random draw per section from the piece's family) ----"
    );
    for r in &rows {
        let prog_distinct = distinct(&r.progression_sigs);
        println!(
            "{:<20} family={:<8} distinct progressions across {} sections: {}   {:?}",
            r.name, r.family, r.n_sections, prog_distinct, r.progression_sigs
        );
    }

    println!("\n---- SECTION ROLE templates (the form's within-piece contrast skeleton) ----");
    for r in &rows {
        println!(
            "{:<20} form={:<16} roles: {:?}",
            r.name, r.form, r.section_roles
        );
    }

    println!("\n=================================================================\n");
}

/// Distinct value set of a projection over the rows, as a sorted Vec for stable printing.
fn set_of<F: Fn(&Row) -> String>(rows: &[Row], f: F) -> Vec<String> {
    rows.iter()
        .map(&f)
        .collect::<BTreeSet<String>>()
        .into_iter()
        .collect()
}

/// S63 CI-HARDENING — EXACT-INTEGER / STRUCTURAL assertions only (see the module-doc audit note).
/// Every assertion here is platform-invariant by construction: it reads a catalogue length, a
/// section COUNT, a motto CELL INDEX (a small integer), or a figuration ID's catalogue membership
/// — never a fractional audio/composition magnitude (which would carry the S62.1 ~0.03 macOS FP
/// divergence). This is the CI regression guard the pure-printer diagnostic above lacked.
#[test]
fn vocab_diag_s63_asserts() {
    let m = load_mappings(MAPPINGS_PATH).expect("mappings load");
    let planner = CompositionPlanner::new(plan_mappings(&m));
    let pm = planner.plan_mappings();

    // 1) Every generative catalogue is authored (non-empty) — a zeroed catalogue is a build defect
    //    the diagnostic's SIZE column would otherwise only PRINT. Integer floors.
    assert!(
        !pm.form_catalogue.is_empty(),
        "form_catalogue must be authored"
    );
    assert!(
        !pm.texture_catalogue.is_empty(),
        "texture_catalogue must be authored"
    );
    assert!(
        !pm.figuration_catalogue.is_empty(),
        "figuration_catalogue must be authored"
    );
    assert!(
        !pm.bass_pattern_catalogue.is_empty(),
        "bass_pattern_catalogue must be authored"
    );
    assert!(
        !pm.prominence_catalogue.is_empty(),
        "prominence_catalogue must be authored"
    );

    let rows: Vec<Row> = IMAGES.iter().map(|n| collect(&planner, &m, n)).collect();
    assert_eq!(rows.len(), IMAGES.len(), "every image must yield a row");

    for r in &rows {
        // 2) Section-count FLOOR — a real multi-section form (Statement/Contrast/Return etc.), not
        //    a degenerate single block. Integer floor.
        assert!(
            r.n_sections >= 2,
            "{}: expected a multi-section plan, got {} section(s)",
            r.name,
            r.n_sections
        );
        // 3) The S53/S63 INTEGRATION invariant: the per-piece motto is NON-NEUTRAL on the real
        //    path (a concrete cell was seated), and its index is a bounded small integer 0..=3.
        //    This is what the accompaniment/bass interlock (interlock_h3_s63) reads; a neutral
        //    motto here would silently disable the S63 integration. Exact-integer.
        let cell = r.motto_cell.unwrap_or_else(|| {
            panic!(
                "{}: per-piece motto is neutral (integration disabled)",
                r.name
            )
        });
        assert!(
            cell <= 3,
            "{}: motto cell index {cell} out of the authored 0..=3 range",
            r.name
        );
        // 4) Every RESOLVED figuration id names a real catalogue entry (or the sentinel for an
        //    un-figured section) — structural membership, never a magnitude.
        for id in &r.figuration_ids {
            let ok = id == "block/none" || pm.figuration_catalogue.iter().any(|c| &c.id == id);
            assert!(
                ok,
                "{}: resolved figuration id `{id}` not in catalogue",
                r.name
            );
        }
    }

    // 5) Cross-piece motto spread is an INTEGER count (a distinct-cell tally). Floor at >= 2 so a
    //    full within-corpus collapse to one gait (the pre-S53 failure) trips CI, while staying far
    //    below the 6-image over-fit line (never require full 4/4 coverage).
    let distinct_motto = distinct(&rows.iter().map(|r| r.motto_cell).collect::<Vec<_>>());
    assert!(
        distinct_motto >= 2,
        "cross-piece motto collapsed to {distinct_motto} distinct cell(s) over the corpus"
    );
}

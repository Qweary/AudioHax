//! tests/interlock_h3_s63.rs — S63 INTEGRATION · the H3 ONSET-INTERLOCK property.
//!
//! WHAT S63 SLICE-1 CHANGED (uncommitted build under test): the accompaniment (Pad) figuration
//! is no longer an independent static onset table stacked under the melody — it now expresses the
//! per-piece `RhythmMotto` as the COMPLEMENT of the melody's gait (chord_engine::
//! `motto_complement_onsets`). Concretely, for a NON-syncopated motto (cells 0/1/2) the comp
//! VACATES the melody's downbeat and answers in its gaps; for the syncopated motto (cell 3) the
//! melody leaves the downbeat and the comp ANCHORS it. Either way the two lines INTERLOCK at the
//! one structurally-privileged position — the downbeat (offset 0 of a step) — instead of both
//! attacking it in parallel. This is the buildable proxy for the design doc's H3 "do the parts
//! fill each other's gaps?" (docs/design-s63-holistic-distinctness-aesthetics.md §2 I2 / §4 H3).
//!
//! ─────────────────────────────────────────────────────────────────────────────────────────────
//! THE PROPERTY (exact-integer / structural — NOT a fractional per-image audio pin) and WHY it is
//! both cross-platform-robust AND regression-sensitive:
//!
//!   Over each piece's FIGURED INTERIOR steps (a step whose section has a non-empty resolved
//!   figuration AND whose phrase-position is `Interior` — i.e. neither a PhraseStart section-
//!   opening block nor a cadence block, both of which legitimately hit the downbeat and are NOT
//!   part of the figured-comp interlock):
//!
//!     • NON-cell-3 pieces  →  the comp (Pad) emits ZERO onsets at the downbeat (offset_ms == 0).
//!       The melody OWNS the downbeat, the comp answers in the gaps.  ASSERTED `== 0` (strict).
//!     • cell-3 pieces      →  the comp ANCHORS the downbeat (a MAJORITY of figured interior steps
//!       carry a Pad onset at offset 0), because the syncopated melody has vacated it.
//!
//!   WHY CROSS-PLATFORM-ROBUST (the S62.1 lesson — a per-image exact audio metric at 0.005 eps
//!   red-barred macOS; here we pin NO fractional audio value):
//!     offset_ms == 0 arises ONLY from a figuration onset at fraction `at == 0.0` (`0.0 * step_ms`
//!     rounds to 0 on every platform, whatever the image-derived tempo/step_ms is). The property
//!     is an INTEGER COUNT of onsets landing exactly on 0 — it never compares a computed ms offset
//!     to a tolerance, so a cross-platform FP drift in step_ms (the S62.1 ~0.03 divergence) cannot
//!     move it. The one floored fraction below (disjointness) is floored GENEROUSLY at 0.50 and is
//!     computed from integer set-disjointness, not from an audio magnitude.
//!
//!   WHY REGRESSION-SENSITIVE (it FAILS if the comp were re-fused onto the melody's grid):
//!     `motto_complement_onsets` builds the non-cell-3 comp by FILTERING OUT the onsets within
//!     ~2% of the downbeat (chord_engine.rs `DOWNBEAT_EPS`). The SOURCE figures it filters
//!     (arp_waltz, broken_chord_up/wave, …) all BEGIN with an `at == 0.0` downbeat onset — this
//!     test asserts that source downbeat still exists (the `regression witness`). So if a future
//!     change re-fused the comp onto the static figure grid (`figured_bed(&spec.onsets, …)` — the
//!     pre-S63 stacked path), that at==0.0 onset would sound → the Pad WOULD hit offset 0 on every
//!     figured interior step → the strict `== 0` flips to a large positive and this test RED-BARS.
//!     The witness makes the flip explicit: the material HAS a downbeat; integration removed it
//!     from the comp; re-fusion puts it back.
//! ─────────────────────────────────────────────────────────────────────────────────────────────
//!
//! Render discipline mirrors `tests/variety_scorecard_s45.rs` exactly (frozen `src/engine.rs` is
//! never called; the public realizer `chord_engine::realize_step` is walked over the seeded plan,
//! per-role, the same way `engine::decide_step` does). Fixed SEED 42 → reproducible.
//!
//! Run: `cargo test --release interlock_h3_s63 -- --nocapture` (DEFAULT features).

use audiohax::chord_engine::{
    instrument_role, realize_step, NoteEvent, OrchestralRole, PerfFeatures, PhrasePosition,
};
use audiohax::composition::{
    CompositionPlan, CompositionPlanner, ImageUnderstanding, LayerRole, PlanMappings, Section,
    StepContext,
};
use audiohax::mapping_loader::{load_mappings, MappingTable};
use audiohax::pure_analysis::{load_pure_image, understand_image_pure, PureImageSource};
use audiohax::seed::set_composition_seed;
use std::collections::{BTreeMap, BTreeSet};

const MAPPINGS_PATH: &str = "assets/mappings.json";
const SEED: u64 = 42;
const NUM_INSTRUMENTS: usize = 4;
/// `EDGE_ACTIVITY_RANGE_MAX` — the same whole-image PerfFeatures projection variety_scorecard uses.
const EDGE_RANGE_MAX: f32 = 0.05;
/// The frozen `src/engine.rs` sha256 (byte-freeze keystone; asserted at the end).
const ENGINE_SHA256: &str = "e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261";

const IMAGES: [&str; 6] = [
    "example.jpg",
    "Lena.png",
    "AudioHaxImg1.jpg",
    "AudioHaxImg2.jpg",
    "AudioHaxImg3.jpg",
    "magicstudio-art.jpg",
];

// ── Fixtures ────────────────────────────────────────────────────────────────────────────────
fn mappings() -> MappingTable {
    load_mappings(MAPPINGS_PATH).expect("assets/mappings.json loads")
}
fn plan_mappings(m: &MappingTable) -> PlanMappings {
    m.composition
        .clone()
        .expect("composition block present in mappings.json")
        .into()
}
fn understand(name: &str) -> ImageUnderstanding {
    let img = load_pure_image(&PureImageSource::Preselected(name.to_string()))
        .unwrap_or_else(|e| panic!("load {name}: {e:?}"));
    understand_image_pure(img.as_rgb()).unwrap_or_else(|e| panic!("understand {name}: {e:?}"))
}
fn perf_for(u: &ImageUnderstanding) -> PerfFeatures {
    PerfFeatures {
        saturation: u.avg_saturation,
        brightness: u.avg_brightness,
        edge_density: (u.edge_activity * EDGE_RANGE_MAX).clamp(0.0, 1.0),
    }
}

// ── The public mirror of engine::decide_step's role assignment (identical to variety_scorecard). ──
fn role_of(section: &Section, inst_idx: usize) -> OrchestralRole {
    let layers = &section.orchestration.layers;
    if layers.is_empty() {
        return instrument_role(inst_idx, NUM_INSTRUMENTS);
    }
    let clamped = inst_idx.min(layers.len().saturating_sub(1));
    match layers[clamped] {
        LayerRole::Bass => OrchestralRole::Bass,
        LayerRole::HarmonicFill => OrchestralRole::HarmonicFill,
        LayerRole::Melody => OrchestralRole::Melody,
        LayerRole::CounterMelody => OrchestralRole::CounterMelody,
        LayerRole::Pad => OrchestralRole::Pad,
    }
}
fn role_name(r: OrchestralRole) -> &'static str {
    match r {
        OrchestralRole::Bass => "Bass",
        OrchestralRole::HarmonicFill => "HarmonicFill",
        OrchestralRole::Melody => "Melody",
        OrchestralRole::Pad => "Pad",
        OrchestralRole::CounterMelody => "CounterMelody",
    }
}

#[derive(Clone)]
struct Stamped {
    step: usize,
    ev: NoteEvent,
}

/// Walk the real realizer across the whole plan (mirrors engine::decide_step over the public
/// surface); collect per-role stamped events keyed by role name.
fn render(plan: &CompositionPlan, perf: &PerfFeatures) -> BTreeMap<&'static str, Vec<Stamped>> {
    let mut by_role: BTreeMap<&'static str, Vec<Stamped>> = BTreeMap::new();
    for step in 0..plan.total_steps {
        let (section, sis) = match plan.locate(step) {
            Some(s) => s,
            None => continue,
        };
        if section.steps.is_empty() {
            continue;
        }
        let theme = section
            .theme
            .and_then(|ti| plan.themes.iter().find(|t| t.id == ti));
        let ctx = StepContext::with_prev(
            section,
            sis,
            theme,
            &plan.key_tempo,
            plan.prev_section_offset(step),
        );
        let plan_step = &section.steps[sis % section.steps.len()];
        for inst_idx in 0..NUM_INSTRUMENTS {
            let role = role_of(section, inst_idx);
            let evs = realize_step(
                plan_step,
                inst_idx,
                NUM_INSTRUMENTS,
                perf,
                section.ms_per_step,
                &ctx,
            );
            let name = role_name(role);
            for ev in evs {
                by_role.entry(name).or_default().push(Stamped { step, ev });
            }
        }
    }
    by_role
}

/// Per-global-step metadata: is this an INTERIOR step (not a PhraseStart section-opening block,
/// not a cadence block), and is the section's figuration ACTIVE (resolved, non-empty onsets — the
/// state in which `figured_bed` runs the motto-complement rather than a sustained block)?
#[derive(Clone, Copy)]
struct StepMeta {
    is_interior: bool,
    fig_active: bool,
}
fn step_meta(plan: &CompositionPlan) -> BTreeMap<usize, StepMeta> {
    let mut mp = BTreeMap::new();
    for step in 0..plan.total_steps {
        let (section, sis) = match plan.locate(step) {
            Some(s) => s,
            None => continue,
        };
        if section.steps.is_empty() {
            continue;
        }
        let sp = &section.steps[sis % section.steps.len()];
        let is_interior = matches!(sp.position, PhrasePosition::Interior);
        let fig_active = section
            .orchestration
            .figuration_resolved
            .as_ref()
            .map(|f| !f.onsets.is_empty())
            .unwrap_or(false);
        mp.insert(
            step,
            StepMeta {
                is_interior,
                fig_active,
            },
        );
    }
    mp
}

/// Per-step offset SET for a role's stamped events (integer ms offsets within the step).
fn offsets_by_step(events: &[Stamped]) -> BTreeMap<usize, BTreeSet<u64>> {
    let mut mp: BTreeMap<usize, BTreeSet<u64>> = BTreeMap::new();
    for s in events {
        mp.entry(s.step).or_default().insert(s.ev.offset_ms);
    }
    mp
}

/// The per-image interlock tally over FIGURED INTERIOR steps.
struct Interlock {
    motto: Option<usize>,
    fnc: usize, // figured interior steps where the Pad sounded
    pad_at_downbeat: usize,
    mel_at_downbeat: usize,
    disjoint_steps: usize,
    /// Regression witness: figured sections whose STATIC figure has an at≈0.0 downbeat onset
    /// (the onset the complement removes for non-cell-3, and re-fusion would restore).
    src_sections_with_downbeat: usize,
    figured_sections: usize,
}

fn measure(plan: &CompositionPlan, perf: &PerfFeatures) -> Interlock {
    let meta = step_meta(plan);
    let streams = render(plan, perf);
    let mel = streams.get("Melody").cloned().unwrap_or_default();
    let pad = streams.get("Pad").cloned().unwrap_or_default();
    let mel_off = offsets_by_step(&mel);
    let pad_off = offsets_by_step(&pad);

    let motto = plan.sections.first().and_then(|s| s.motto().cell_index);

    let (mut fnc, mut pad_at0, mut mel_at0, mut disjoint) = (0usize, 0usize, 0usize, 0usize);
    for (step, mm) in &meta {
        if !mm.fig_active || !mm.is_interior {
            continue;
        }
        let po = pad_off.get(step).cloned().unwrap_or_default();
        if po.is_empty() {
            continue; // the Pad did not sound this step (nothing to interlock)
        }
        let mo = mel_off.get(step).cloned().unwrap_or_default();
        fnc += 1;
        if po.contains(&0) {
            pad_at0 += 1;
        }
        if mo.contains(&0) {
            mel_at0 += 1;
        }
        if po.is_disjoint(&mo) {
            disjoint += 1;
        }
    }

    // Regression witness: does the source figuration material carry a downbeat onset?
    let mut figured_sections = 0usize;
    let mut src_downbeat = 0usize;
    for s in &plan.sections {
        if let Some(f) = &s.orchestration.figuration_resolved {
            if f.onsets.is_empty() {
                continue;
            }
            figured_sections += 1;
            if f.onsets.iter().any(|o| o.at.clamp(0.0, 1.0) <= 0.02) {
                src_downbeat += 1;
            }
        }
    }

    Interlock {
        motto,
        fnc,
        pad_at_downbeat: pad_at0,
        mel_at_downbeat: mel_at0,
        disjoint_steps: disjoint,
        src_sections_with_downbeat: src_downbeat,
        figured_sections,
    }
}

#[test]
fn h3_onset_interlock() {
    let m = mappings();
    let pm = plan_mappings(&m);

    println!("\n================ S63 H3 ONSET-INTERLOCK (figured INTERIOR steps) ================");
    println!(
        "{:<20} {:>5} {:>4} {:>10} {:>10} {:>9} {:>12}",
        "image", "motto", "fnc", "PAD@down", "MEL@down", "disjoint", "srcDownbeat"
    );
    println!("{}", "-".repeat(78));

    // Non-vacuity accumulators: the property must actually be EXERCISED (not pass on an empty set).
    let mut noncell3_pieces_checked = 0usize;
    let mut cell3_pieces_checked = 0usize;
    let mut total_noncell3_fnc = 0usize;

    for name in IMAGES {
        let u = understand(name);
        let perf = perf_for(&u);
        let planner = CompositionPlanner::new(pm.clone());
        set_composition_seed(Some(SEED));
        let plan = planner.plan(&u, &m);
        let il = measure(&plan, &perf);

        println!(
            "{:<20} {:>5} {:>4} {:>10} {:>10} {:>9} {:>12}",
            name,
            il.motto
                .map(|c| c.to_string())
                .unwrap_or_else(|| "-".into()),
            il.fnc,
            il.pad_at_downbeat,
            il.mel_at_downbeat,
            format!("{}/{}", il.disjoint_steps, il.fnc),
            format!("{}/{}", il.src_sections_with_downbeat, il.figured_sections),
        );

        // Every corpus piece is now figured on this build; each must expose figured interior steps.
        assert!(
            il.fnc > 0,
            "{name}: expected figured interior steps to interlock over (S63 integrated all 6); \
             got 0 — the figured-comp path is not being exercised"
        );

        match il.motto {
            Some(3) => {
                cell3_pieces_checked += 1;
                // cell 3 (syncopated): the melody vacates the downbeat, the comp ANCHORS it.
                // FLOOR: a MAJORITY of figured interior steps carry a Pad downbeat onset. (Not
                // asserted `== fnc` because a `block`-onset middle section or a density-thinned
                // step can drop a step; the majority floor is the robust, regression-sensitive
                // form — if the anchor logic broke, pad_at_downbeat would collapse toward 0.)
                assert!(
                    il.pad_at_downbeat * 2 >= il.fnc,
                    "{name}: cell-3 comp must ANCHOR the downbeat on a majority of figured \
                     interior steps; got {}/{}",
                    il.pad_at_downbeat,
                    il.fnc
                );
            }
            Some(_) => {
                noncell3_pieces_checked += 1;
                total_noncell3_fnc += il.fnc;
                // ── PRIMARY EXACT-INTEGER PROPERTY ──
                // The comp answers in the melody's GAPS: it emits NO onset at the melody's
                // downbeat. Strict `== 0` (0.0*step_ms == 0 on every platform → no FP fragility).
                assert_eq!(
                    il.pad_at_downbeat, 0,
                    "{name}: non-cell-3 comp must VACATE the melody's downbeat on ALL figured \
                     interior steps (offset 0). Got {} onset(s) at the downbeat — the comp is \
                     re-fused onto the melody's grid (the pre-S63 STACKED regression), not \
                     interlocking. Expected 0.",
                    il.pad_at_downbeat
                );
                // Interlock is GENUINE (not a muted comp): the melody actually claims the downbeat
                // the comp vacated, on a healthy count of these steps.
                assert!(
                    il.mel_at_downbeat > 0,
                    "{name}: interlock requires the melody to OWN the vacated downbeat; it never \
                     claims offset 0 on the figured interior steps — the downbeat is dead, not \
                     interlocked"
                );
                // Generous disjointness FLOOR (>= 0.50): the pad onset-set and melody onset-set
                // are fully disjoint on at least half the figured interior steps. Integer set
                // disjointness (not an audio magnitude); floored loosely so a coincidental shared
                // non-downbeat offset never red-bars.
                assert!(
                    il.disjoint_steps * 2 >= il.fnc,
                    "{name}: expected the comp and melody onset grids to be disjoint on >= 50% of \
                     figured interior steps (interlock); got {}/{}",
                    il.disjoint_steps,
                    il.fnc
                );
            }
            None => panic!(
                "{name}: figured build must carry a non-neutral per-piece motto; got neutral \
                 (cell_index None) — the S63 integration is not wired"
            ),
        }

        // ── REGRESSION WITNESS (both branches) ──
        // The property above is only regression-sensitive if the SOURCE figuration actually has a
        // downbeat onset for the comp to have removed / anchored. Assert the material carries one
        // on at least one figured section (so `pad_at_downbeat == 0` is an ACTIVE removal, and a
        // re-fusion to `&spec.onsets` would demonstrably restore a downbeat onset).
        assert!(
            il.src_sections_with_downbeat > 0,
            "{name}: no figured section's source figure has a downbeat onset — the interlock \
             property would be vacuous (nothing for the comp to vacate/anchor)"
        );
    }

    // Non-vacuity: the primary property was exercised on real non-cell-3 pieces with real steps.
    assert!(
        noncell3_pieces_checked >= 1 && total_noncell3_fnc > 0,
        "the primary non-cell-3 interlock property was never exercised (checked {} pieces, {} \
         total figured interior steps)",
        noncell3_pieces_checked,
        total_noncell3_fnc
    );
    // And the corpus exercises BOTH interlock modes (downbeat vacate + downbeat anchor).
    assert!(
        cell3_pieces_checked >= 1,
        "the cell-3 (comp-anchors-downbeat) branch was never exercised on the corpus"
    );

    println!(
        "\nPASS: non-cell-3 pieces vacate the melody's downbeat in the comp (PAD@down == 0); \
         cell-3 pieces anchor it. {} non-cell-3 + {} cell-3 pieces checked over {} figured \
         interior steps.",
        noncell3_pieces_checked, cell3_pieces_checked, total_noncell3_fnc
    );

    // engine.rs byte-freeze keystone.
    let sha = sha256_engine();
    assert_eq!(
        sha, ENGINE_SHA256,
        "src/engine.rs changed — freeze violated"
    );
    println!("engine.rs sha256 = {sha} (FROZEN, unchanged)\n");
}

/// sha256 of `src/engine.rs` — the byte-freeze witness (no external crate; reuse the std hashless
/// approach via the shipped `sha2`-free path is unavailable, so shell out is disallowed in tests;
/// instead read the file and compare a rolling hash is unsafe. We compute a real sha256 with the
/// workspace's `sha2` dev-dependency if present; otherwise fall back to asserting the known bytes.)
fn sha256_engine() -> String {
    use std::io::Read;
    let mut f = std::fs::File::open("src/engine.rs").expect("open src/engine.rs");
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).expect("read src/engine.rs");
    sha256_hex(&buf)
}

// Minimal, self-contained SHA-256 (no external dependency) so the freeze witness never depends on
// a dev-dependency being wired for this test target.
fn sha256_hex(data: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let bitlen = (data.len() as u64) * 8;
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bitlen.to_be_bytes());
    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 64];
        for (i, wi) in w.iter_mut().enumerate().take(16) {
            *wi = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let mut v = h;
        for i in 0..64 {
            let s1 = v[4].rotate_right(6) ^ v[4].rotate_right(11) ^ v[4].rotate_right(25);
            let ch = (v[4] & v[5]) ^ ((!v[4]) & v[6]);
            let t1 = v[7]
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = v[0].rotate_right(2) ^ v[0].rotate_right(13) ^ v[0].rotate_right(22);
            let maj = (v[0] & v[1]) ^ (v[0] & v[2]) ^ (v[1] & v[2]);
            let t2 = s0.wrapping_add(maj);
            v[7] = v[6];
            v[6] = v[5];
            v[5] = v[4];
            v[4] = v[3].wrapping_add(t1);
            v[3] = v[2];
            v[2] = v[1];
            v[1] = v[0];
            v[0] = t1.wrapping_add(t2);
        }
        for i in 0..8 {
            h[i] = h[i].wrapping_add(v[i]);
        }
    }
    let mut out = String::with_capacity(64);
    for word in h {
        out.push_str(&format!("{word:08x}"));
    }
    out
}

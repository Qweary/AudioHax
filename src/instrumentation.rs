//! src/instrumentation.rs — image-feature → per-voice GM program selection (B5, S74b).
//!
//! Freeze-safe: depends ONLY on `crate::composition::{CompositionMappings, ImageUnderstanding}` —
//! no `engine` (frozen), no `chord_engine`, no `synth_sink`/`midi_output`, no OpenCV, no audio
//! stack. Pure and deterministic; builds and unit-tests headlessly under `--no-default-features`
//! (the exact profile the S73/S74b harnesses built under).
//!
//! The instrumentation axis is added exactly as the texture axis was in S17: the *palette*
//! (`ensemble_catalogue`) and the *selection rule* (`instrumentation` `SelectTable`) are BOTH DATA
//! in `assets/mappings.json`; this module only reads the knobs, runs the shared `SelectTable::select`,
//! resolves the picked id, and maps voice index → GM program. Adding/re-voicing an ensemble or
//! re-keying an image band is a JSON edit, no Rust change.

use crate::composition::ImageUnderstanding;
use crate::mapping_loader::CompositionMappings;

// ─────────────────────────────────────────────────────────────────────────────
// S98 / B2-a — PER-VOICE ensemble granularity (per-seat eligibility pools)
//
// Implements the §3 (Music Theory) design ruling `s98-musictheory-b2a-design-constraint.md`:
// per-seat eligibility POOLS keyed on the pure index function `instrument_role(i, n)`, with
// INDEPENDENT draws within each pool and ONE deterministic repair pass. NOT a global pool with
// rejection sampling (non-deterministic in effort — poison against sealed goldens).
//
// The load-bearing call, restated so it is not quietly reverted: **constrain REGISTER (C1) and
// ENVELOPE (C2); let FAMILY float.** Family purity is the safe-feeling constraint that would
// collapse the entire gain while the grain metric still read 4, leaving the "assembled from a
// bank" percept intact (§3 §4a). Family is used ONLY as a soft inner-voice separation nudge
// (C3a), never as a coherence gate.
//
// Freeze boundary held: this module still imports NO `engine` and NO `chord_engine`. The seat
// classifier below is a local re-derivation of `chord_engine::instrument_role`, and a unit test
// (`seat_matches_chord_engine_instrument_role`) asserts the two agree for n = 1..=8, so the
// duplication cannot silently drift.
// ─────────────────────────────────────────────────────────────────────────────

/// One voice's step-aggregated image-band features, supplied by the caller.
///
/// `scan_image` already splits the image into `instrument_count` bands and voice `i` already
/// sounds band `i` (§3 F6), so band `i` is the principled per-voice driver — no new extraction.
/// The caller (`main.rs`) aggregates band `i` over all scan steps and hands the mean here.
///
/// An EMPTY `&[BandFeatures]` slice means "no per-voice input available" and routes selection
/// back to the whole-image `ensemble_catalogue` path — the honest-degradation floor this codebase
/// uses everywhere else.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BandFeatures {
    /// Mean `avg_brightness` (0..100) of this voice's band, aggregated over all scan steps.
    pub brightness: f32,
}

/// The three orchestral seats a voice index can occupy, as a pure function of `(i, n)`.
///
/// Mirrors `chord_engine::instrument_role` exactly (asserted by test). Keyed on
/// `instrument_role` and NOT `assign_role` because selection happens BEFORE the plan is seated
/// (§3 F1/F4): `assign_role` needs a section context that does not exist yet, while
/// `instrument_role` is index-only and total. Under the identity profile they are the same
/// function, so nothing is lost on the freeze path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Seat {
    /// Voice 0 — harmonic foundation and metric anchor, tessitura ~MIDI 36..55.
    Bass,
    /// Inner voices — sustained inner chord tones, tessitura ~MIDI 48..72.
    HarmonicFill,
    /// Top voice — the tune, tessitura ~MIDI 60..84.
    Melody,
}

impl Seat {
    /// The `role` key this seat matches in the `instrumentation_pools.roles` JSON block.
    fn key(self) -> &'static str {
        match self {
            Seat::Bass => "bass",
            Seat::HarmonicFill => "harmonic_fill",
            Seat::Melody => "melody",
        }
    }
}

/// Seat for voice `i` of `n`. Pure index function; mirrors `chord_engine::instrument_role`.
///
/// A single voice is the melody, not a bass (the lone line is the tune) — that degenerate guard
/// is part of the mirrored contract.
pub fn seat_for(i: usize, n: usize) -> Seat {
    if n <= 1 {
        return Seat::Melody;
    }
    if i == 0 {
        Seat::Bass
    } else if i >= n - 1 {
        Seat::Melody
    } else {
        Seat::HarmonicFill
    }
}

/// Envelope class of a GM program — the C2 axis.
///
/// Not derivable from the program number, so it ships as DATA per pool entry. The distinction is
/// a functional contract with the realizer, not a taste call: the `Pad`/`HarmonicFill` roles tie
/// chord tones step-to-step "so consecutive beds connect into a continuous floor", which a
/// DECAYING program cannot honour — the note-off arrives long after the sound has died and the
/// harmony audibly leaks (§3 C2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Sustain {
    /// Bowed / blown / driven — holds until note-off.
    Sustaining,
    /// Plucked / struck — decays regardless of note-off.
    Decaying,
}

/// One eligible program for one seat. Pool ORDER encodes ascending spectral brightness (§3's
/// primary axis: visual luminance ↔ timbral brightness), so a pool index is a brightness position.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct PoolEntry {
    /// GM program 0..=127.
    pub program: u8,
    /// Envelope class — the C2 axis.
    pub sustain: Sustain,
}

/// One seat's eligibility pool: the programs that pass C1 (tessitura) and C2 (envelope) for that
/// seat, ordered by ascending spectral brightness.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct RolePool {
    /// `"bass"` | `"harmonic_fill"` | `"melody"`.
    pub role: String,
    /// Brightness-ordered eligible entries.
    pub entries: Vec<PoolEntry>,
}

/// The B2-a per-seat pool block (`composition.instrumentation_pools` in `mappings.json`).
///
/// `#[serde(default)]` at the use site: an OLD `mappings.json` with no such key yields empty
/// `roles`, which routes selection back to the whole-image `ensemble_catalogue` path — the
/// back-compat floor.
#[derive(Debug, Clone, PartialEq, Default, serde::Deserialize)]
pub struct InstrumentationPools {
    /// Per-seat eligibility pools.
    #[serde(default)]
    pub roles: Vec<RolePool>,
    /// Blend weight on the WHOLE-IMAGE brightness component of a seat's pool position, in 0..1.
    ///
    /// The per-seat position is `w * global + (1 - w) * band_rank_component`. The global term
    /// carries image-relatedness (a dark image draws the dark end of EVERY pool, so a dark photo
    /// still yields a dark quartet); the rank term carries per-voice differentiation. At `w = 1`
    /// the four draws collapse to whole-image scalars — that is precisely §6's pre-registered
    /// P-A hollow build — so `w` is deliberately < 1 and the rank term is deliberately never
    /// scaled to zero.
    #[serde(default = "default_global_weight")]
    pub global_weight: f32,
}

fn default_global_weight() -> f32 {
    0.5
}

impl InstrumentationPools {
    /// The pool for `seat`, or `None` when the block is absent/omits that seat.
    fn pool(&self, seat: Seat) -> Option<&[PoolEntry]> {
        self.roles
            .iter()
            .find(|r| r.role == seat.key())
            .map(|r| r.entries.as_slice())
            .filter(|e| !e.is_empty())
    }
}

/// GM's own 8-program family grouping (`program / 8`) — the C3a separation proxy.
///
/// §3 endorses GM's grouping as "an adequate, non-arbitrary proxy". Used ONLY for the soft
/// inner-voice nudge; it is NOT a coherence gate (see the family-float ruling above).
fn gm_family(program: u8) -> u8 {
    program / 8
}

/// Per-voice pool POSITION in 0..1, deviation-coded against the image mean.
///
/// This is §3's §4(b) mitigation and the single most important line in the build. Measured on the
/// nine-image corpus, absolute band brightness is tightly correlated across bands (spread 4.6..20
/// on a 0..100 scale) — four correlated inputs through four similar mappings would produce four
/// similar draws, i.e. timbral unison, which sounds WORSE than today. The band's RANK among its
/// siblings is not correlated in that way (8 distinct permutations across the 9 images).
///
/// Two components:
///   * `global` = whole-image brightness/100 — carries image-relatedness (B6 correspondence).
///   * `rank`   = `(rank_i + norm_i) / n`, where `rank_i` is the band's brightness rank among the
///     `n` bands (ties broken by band index) and `norm_i` is its min-max normalized brightness.
///     Strictly increasing in band brightness, and — because a rank is a PERMUTATION — distinct
///     for every pair of voices BY CONSTRUCTION. This is what separately *determines* v1 and v2,
///     which share the `HarmonicFill` seat and would otherwise return the same program (FINDING X).
fn seat_positions(u: &ImageUnderstanding, bands: &[BandFeatures]) -> Vec<f32> {
    let n = bands.len();
    if n == 0 {
        return Vec::new();
    }
    // Rank the bands by brightness; ties break by band index so the permutation is total and
    // deterministic (an exact brightness tie between two bands still yields distinct ranks).
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| {
        bands[a]
            .brightness
            .partial_cmp(&bands[b].brightness)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.cmp(&b))
    });
    let mut rank = vec![0usize; n];
    for (r, &band) in order.iter().enumerate() {
        rank[band] = r;
    }

    let lo = bands.iter().fold(f32::MAX, |m, b| m.min(b.brightness));
    let hi = bands.iter().fold(f32::MIN, |m, b| m.max(b.brightness));
    let span = hi - lo;

    let global = (u.avg_brightness / 100.0).clamp(0.0, 1.0);

    (0..n)
        .map(|i| {
            // Min-max normalized band brightness; a perfectly flat image degenerates to the rank
            // midpoint rather than dividing by zero (deviation-coding still guarantees spread,
            // because `rank` alone already separates the voices).
            let norm = if span > f32::EPSILON {
                (bands[i].brightness - lo) / span
            } else {
                0.5
            };
            let rank_component = (rank[i] as f32 + norm) / n as f32;
            (global + rank_component) * 0.5
        })
        .collect()
}

/// Resolve `pos` (0..1) to an index into a pool of length `len`.
fn pool_index(pos: f32, len: usize) -> usize {
    debug_assert!(len > 0);
    let idx = (pos.clamp(0.0, 1.0) * len as f32) as usize;
    idx.min(len - 1)
}

/// Deterministic outward scan from `from` for the first entry satisfying `ok`.
///
/// Tries `from`, then `from-1`, `from+1`, `from-2`, `from+2`, … — a bounded, deterministic repair
/// (§3: "nudging a colliding voice to its pool's next candidate rather than rejecting and
/// redrawing"). Rejection sampling is deliberately NOT used: it is non-deterministic in effort,
/// which is poison for a byte-reproducibility discipline with sealed goldens.
fn nearest<F: Fn(&PoolEntry) -> bool>(pool: &[PoolEntry], from: usize, ok: F) -> Option<usize> {
    if ok(&pool[from]) {
        return Some(from);
    }
    for d in 1..=pool.len() {
        if let Some(i) = from.checked_sub(d) {
            if ok(&pool[i]) {
                return Some(i);
            }
        }
        let i = from + d;
        if i < pool.len() && ok(&pool[i]) {
            return Some(i);
        }
    }
    None
}

/// The GM program (0..=127) legacy floor for voice `i` — the byte-for-byte reproduction of the
/// historical `main.rs` patch `((i*7)%128)`. Used for (a) an absent/empty `instrumentation` table,
/// (b) an unresolved ensemble id, and (c) overflow voices when an ensemble palette is shorter than
/// the runtime voice count. Keeping the floor HERE (one definition) means the control arm and any
/// old `mappings.json` stay byte-identical to the pre-B5 binary.
pub fn legacy_program(i: usize) -> u8 {
    ((i * 7) % 128) as u8
}

/// Resolve the per-voice GM program vector for one image, length == `num_instruments`.
///
/// Deterministic pure fn of `(understanding, composition-mappings, num_instruments)`:
///   1. run the `instrumentation` `SelectTable` over `u` → an ensemble id (or `""` if no table/rule);
///   2. look the id up in `ensemble_catalogue` → its voice-indexed `programs` array;
///   3. for voice `i`: `programs[i]` if in range, else `legacy_program(i)` (overflow/unresolved floor).
///
/// `comp` is the loaded `composition` block (`MappingTable.composition`, `Option`-unwrapped by the
/// caller). When `comp` has no `instrumentation` axis / empty catalogue, EVERY voice falls to
/// `legacy_program(i)` — so the output is byte-identical to today's `(i*7)%128` and the control arm.
/// Never panics on bad data (unknown id → floor; short array → floor for the tail;
/// `num_instruments == 0` → empty `Vec`).
pub fn select_programs(
    u: &ImageUnderstanding,
    comp: &CompositionMappings,
    num_instruments: usize,
) -> Vec<u8> {
    // 1. run the instrumentation SelectTable → ensemble id ("" when no rule matches AND the
    //    default is empty — the S17 honest-degradation floor).
    let id = comp.instrumentation.select(u);
    // 2. resolve the id → its voice-indexed programs array. `""` (or any id absent from the
    //    catalogue) resolves to None → the per-voice legacy floor.
    let palette: Option<&[u8]> = comp
        .ensemble_catalogue
        .iter()
        .find(|e| e.id == id)
        .map(|e| e.programs.as_slice());
    // 3. per voice i: programs[i] if in range, else legacy_program(i). Total: `.get(i)` clamps
    //    rather than index-panics, so a short palette floors only its tail.
    (0..num_instruments)
        .map(|i| match palette {
            Some(p) => p.get(i).copied().unwrap_or_else(|| legacy_program(i)),
            None => legacy_program(i),
        })
        .collect()
}

/// S98/B2-a — resolve the per-voice GM program vector with **one independent selection event per
/// voice** over the per-seat eligibility pools.
///
/// This is the B2-a selection path and it is what `main.rs` calls whenever per-voice band features
/// are available (i.e. on every real image). `GRAIN_instrumentation` moves 1 → `num_instruments`
/// because each voice's program is resolved by its OWN pool draw against its OWN band's
/// deviation-coded brightness — not by positionally reading one drawn catalogue row.
///
/// Falls back to the whole-image [`select_programs`] catalogue path — byte-identically — when
/// `bands` is empty or `composition.instrumentation_pools` is absent/empty. That fallback is a
/// CAPABILITY floor (no per-voice input exists), not a per-piece gate: every piece with band
/// features takes the per-voice path. Selection consumes no RNG and is a pure deterministic
/// function of `(u, comp, num_instruments, bands)`.
///
/// Constraint order (single deterministic repair pass, §3 §3):
///   1. **C1 tessitura** — enforced by POOL MEMBERSHIP (a pool contains only programs whose
///      practical range covers that seat), so it can never be violated downstream.
///   2. **C2 envelope** (hard) — at least one `HarmonicFill` seat must be SUSTAINING, unless
///      *every* voice is DECAYING. "All-decaying is fine, half-decaying is forbidden": the
///      all-decaying quartet is a real orchestration (`cool_mallets`) and stays reachable, while
///      a lone decaying program in a sustaining bed — which leaves the floor silent between
///      onsets and reads to the ear as a BUG — is repaired.
///   3. **C3a inner separation** (soft) — consecutive `HarmonicFill` seats nudged apart by GM
///      family, preserving each seat's envelope class so the repair cannot undo C2.
///   4. **C3b outer polarity** (soft) — v0 and vN-1 nudged apart when identical, likewise
///      envelope-preserving.
pub fn select_programs_per_voice(
    u: &ImageUnderstanding,
    comp: &CompositionMappings,
    num_instruments: usize,
    bands: &[BandFeatures],
) -> Vec<u8> {
    // Capability floor: no per-voice input, or no pool data → the whole-image catalogue path.
    if bands.len() != num_instruments || num_instruments == 0 {
        return select_programs(u, comp, num_instruments);
    }
    let pools = &comp.instrumentation_pools;
    if (0..num_instruments).any(|i| pools.pool(seat_for(i, num_instruments)).is_none()) {
        return select_programs(u, comp, num_instruments);
    }

    let positions = seat_positions(u, bands);
    let w = pools.global_weight.clamp(0.0, 1.0);
    let global = (u.avg_brightness / 100.0).clamp(0.0, 1.0);

    // ── 1. C1: one INDEPENDENT draw per voice, within that seat's tessitura-eligible pool ──
    let mut idx: Vec<usize> = Vec::with_capacity(num_instruments);
    for (i, &raw) in positions.iter().enumerate() {
        let pool = pools.pool(seat_for(i, num_instruments)).unwrap_or(&[]);
        // `seat_positions` blends global and rank at a fixed 0.5; re-blend at the configured
        // weight so `global_weight` is a live knob (w=1 collapses to whole-image — the hollow
        // build — and is therefore never the shipped value).
        let rank_component = raw * 2.0 - global;
        let pos = w * global + (1.0 - w) * rank_component;
        idx.push(pool_index(pos, pool.len()));
    }

    let entry = |i: usize, k: usize| -> &PoolEntry {
        &pools.pool(seat_for(i, num_instruments)).unwrap_or(&[])[k]
    };

    // ── 2. C2 (hard): the harmonic bed must be HELD by at least one sustaining fill seat,
    //       unless every voice decays (the coherent all-plucked/all-mallet ensemble). ──
    let fills: Vec<usize> = (0..num_instruments)
        .filter(|&i| seat_for(i, num_instruments) == Seat::HarmonicFill)
        .collect();
    let all_decaying = (0..num_instruments).all(|i| entry(i, idx[i]).sustain == Sustain::Decaying);
    let bed_held = fills
        .iter()
        .any(|&i| entry(i, idx[i]).sustain == Sustain::Sustaining);
    if !fills.is_empty() && !bed_held && !all_decaying {
        // Repair the FIRST fill seat to its nearest sustaining candidate — lowest index first, so
        // the repair is a stable function of the pool order.
        let i = fills[0];
        let pool = pools.pool(Seat::HarmonicFill).unwrap_or(&[]);
        if let Some(k) = nearest(pool, idx[i], |e| e.sustain == Sustain::Sustaining) {
            idx[i] = k;
        }
    }

    // ── 3. C3a (soft): consecutive fill seats should not share a GM family, or they fuse and a
    //       four-part texture collapses audibly to three. Envelope-class preserving. ──
    for pair in fills.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if gm_family(entry(a, idx[a]).program) == gm_family(entry(b, idx[b]).program) {
            let pool = pools.pool(Seat::HarmonicFill).unwrap_or(&[]);
            let want = entry(b, idx[b]).sustain;
            let fam_a = gm_family(entry(a, idx[a]).program);
            if let Some(k) = nearest(pool, idx[b], |e| {
                e.sustain == want && gm_family(e.program) != fam_a
            }) {
                idx[b] = k;
            }
        }
    }

    // ── 4. C3b (soft): bass and melody on one program erases the outer-voice contrast that makes
    //       four-part texture legible. Envelope-class preserving (so C2 survives the nudge). ──
    if num_instruments >= 2 {
        let (a, b) = (0usize, num_instruments - 1);
        if entry(a, idx[a]).program == entry(b, idx[b]).program {
            let pool = pools.pool(seat_for(b, num_instruments)).unwrap_or(&[]);
            let want = entry(b, idx[b]).sustain;
            let bass = entry(a, idx[a]).program;
            if let Some(k) = nearest(pool, idx[b], |e| {
                e.sustain == want && e.program != bass
            }) {
                idx[b] = k;
            }
        }
    }

    (0..num_instruments).map(|i| entry(i, idx[i]).program).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composition::SelectTable;

    /// The real shipped `composition` block (loads `assets/mappings.json`; headless, no
    /// pure-analysis/OpenCV needed — cargo runs tests with CWD == crate root).
    fn comp() -> CompositionMappings {
        crate::mapping_loader::load_mappings("assets/mappings.json")
            .expect("mappings.json loads")
            .composition
            .expect("composition block present")
    }

    /// Hand-build an `ImageUnderstanding` carrying only the 6 keying features (the rest neutral).
    fn u_with(hue: f32, bri: f32, edge: f32, cfl: f32, sat: f32, tex: f32) -> ImageUnderstanding {
        let mut u = ImageUnderstanding::neutral();
        u.dominant_hue = hue;
        u.avg_brightness = bri;
        u.edge_activity = edge;
        u.colorfulness = cfl;
        u.avg_saturation = sat;
        u.texture = tex;
        u
    }

    // ── The 6 corpus images at their S74b §2.1 / arch §3.2 spread-feature values ──
    fn u_img1() -> ImageUnderstanding {
        u_with(39.94, 29.30, 0.301, 0.011, 32.89, 0.615)
    }
    fn u_img2() -> ImageUnderstanding {
        u_with(36.64, 81.07, 0.509, 0.080, 30.13, 0.774)
    }
    fn u_img3() -> ImageUnderstanding {
        u_with(5.19, 65.68, 0.475, 0.423, 37.18, 0.550)
    }
    fn u_example() -> ImageUnderstanding {
        u_with(198.65, 49.88, 0.719, 0.686, 64.52, 0.979)
    }
    fn u_lena() -> ImageUnderstanding {
        u_with(354.15, 70.54, 0.471, 0.122, 51.90, 0.194)
    }
    fn u_magic() -> ImageUnderstanding {
        u_with(278.31, 45.34, 0.106, 0.287, 43.61, 0.164)
    }

    /// 6a.1 — THE 6 GOLDEN TRACES. Each corpus image lands on its ear-approved ensemble, pinning
    /// the S74b §3 program arrays to a headless assertion (this is the ear-approved sound as data).
    #[test]
    fn six_golden_program_arrays() {
        let c = comp();
        assert_eq!(
            select_programs(&u_img1(), &c, 4),
            vec![70, 60, 69, 71],
            "Img1 dark_reeds"
        );
        assert_eq!(
            select_programs(&u_img2(), &c, 4),
            vec![58, 61, 57, 56],
            "Img2 bright_brass"
        );
        assert_eq!(
            select_programs(&u_img3(), &c, 4),
            vec![32, 46, 6, 24],
            "Img3 warm_plucked"
        );
        assert_eq!(
            select_programs(&u_example(), &c, 4),
            vec![12, 11, 13, 9],
            "example cool_mallets"
        );
        assert_eq!(
            select_programs(&u_lena(), &c, 4),
            vec![42, 48, 41, 40],
            "Lena lush_strings"
        );
        assert_eq!(
            select_programs(&u_magic(), &c, 4),
            vec![89, 52, 8, 10],
            "magic ethereal_violet"
        );
    }

    /// 6a.2 — EMPTY-TABLE BYTE-FLOOR. With `instrumentation`/`ensemble_catalogue` cleared, every
    /// voice == `legacy_program(i) == (i*7)%128` → `[0,7,14,21]`. The control arm + old-mappings
    /// back-compat proof.
    #[test]
    fn empty_table_reproduces_legacy_floor() {
        let mut c = comp();
        c.instrumentation = SelectTable::default();
        c.ensemble_catalogue = Vec::new();
        assert_eq!(select_programs(&u_img1(), &c, 4), vec![0, 7, 14, 21]);
        // legacy_program itself is the (i*7)%128 byte-floor.
        assert_eq!(
            (0..4).map(legacy_program).collect::<Vec<u8>>(),
            vec![0, 7, 14, 21]
        );
    }

    /// 6a.3 — ORDER-SENSITIVITY GUARDS (the four §3.2 near-miss cases). Asserts on the resolved
    /// ensemble id directly so a re-ordering of the SelectTable rules is caught.
    #[test]
    fn order_sensitivity_near_misses() {
        let c = comp();
        // Img3: edge 0.475 < 0.50 misses R4 brass → falls through to R5 colorful → plucked.
        assert_eq!(c.instrumentation.select(&u_img3()), "warm_plucked");
        // Lena: misses R4 (edge<.50), R5 (cfl<.30), R6 (hue not <20) → R7 smooth+sat → strings.
        assert_eq!(c.instrumentation.select(&u_lena()), "lush_strings");
        // Img1: warm+dark trips R3 reeds BEFORE R4 brass (reeds must precede brass).
        assert_eq!(c.instrumentation.select(&u_img1()), "dark_reeds");
        // example/magic removed by the hue zones (R1/R2) before any warm splitter can mis-catch.
        assert_eq!(c.instrumentation.select(&u_example()), "cool_mallets");
        assert_eq!(c.instrumentation.select(&u_magic()), "ethereal_violet");
    }

    /// 6a.4 — N-GENERALITY / OVERFLOW. A 4-voice palette + 6 runtime voices → first 4 = ensemble,
    /// tail voices fall to the floor; `num_instruments == 0` → empty Vec (no panic).
    #[test]
    fn overflow_tail_falls_to_floor() {
        let c = comp();
        let got = select_programs(&u_img1(), &c, 6);
        assert_eq!(
            got,
            vec![70, 60, 69, 71, legacy_program(4), legacy_program(5)],
            "voices 4,5 must floor to (i*7)%128 = 28,35"
        );
        assert_eq!(got[4], 28);
        assert_eq!(got[5], 35);
        assert!(select_programs(&u_img1(), &c, 0).is_empty());
    }

    /// 6a.5 — TOTALITY / BAD DATA. An `instrumentation.default` pointing at an id ABSENT from the
    /// catalogue ⇒ per-voice floor, no panic.
    #[test]
    fn unknown_default_id_falls_to_floor() {
        let mut c = comp();
        c.instrumentation = SelectTable {
            default: "does_not_exist".to_string(),
            rules: Vec::new(),
        };
        assert_eq!(select_programs(&u_img1(), &c, 4), vec![0, 7, 14, 21]);
    }

    /// 6a.6 — DEFAULT ARM. A synthetic warm image matching NO splitter (mid-bright, mid-edge,
    /// mid-texture, warm-but-not-red hue) resolves to the `default` `warm_plucked`.
    #[test]
    fn default_arm_catches_uncategorized_warm() {
        let c = comp();
        let u = u_with(60.0, 50.0, 0.30, 0.20, 40.0, 0.50);
        assert_eq!(c.instrumentation.select(&u), "warm_plucked");
        assert_eq!(select_programs(&u, &c, 4), vec![32, 46, 6, 24]);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // S98 / B2-a — per-voice granularity
    // ─────────────────────────────────────────────────────────────────────────

    fn bands(v: [f32; 4]) -> Vec<BandFeatures> {
        v.iter().map(|&b| BandFeatures { brightness: b }).collect()
    }

    /// The local `seat_for` is a re-derivation of `chord_engine::instrument_role` kept so this
    /// module's freeze boundary (no `chord_engine` import) holds. Assert the two AGREE across
    /// every voice count the engine can be configured with, so the duplication cannot drift.
    #[test]
    fn seat_matches_chord_engine_instrument_role() {
        use crate::chord_engine::{instrument_role, OrchestralRole};
        for n in 1..=8 {
            for i in 0..n {
                let want = match instrument_role(i, n) {
                    OrchestralRole::Bass => Seat::Bass,
                    OrchestralRole::HarmonicFill => Seat::HarmonicFill,
                    OrchestralRole::Melody => Seat::Melody,
                    other => panic!("instrument_role returned an unmirrored role {other:?}"),
                };
                assert_eq!(seat_for(i, n), want, "seat mismatch at i={i} n={n}");
            }
        }
    }

    /// **B4/B5 PRESERVATION + the capability floor.** With NO band features the per-voice entry
    /// point must reproduce the whole-image catalogue path byte-for-byte — this is what keeps the
    /// six ear-approved golden program arrays valid, and it is a CAPABILITY fallback (no per-voice
    /// input exists), not a per-piece gate.
    #[test]
    fn empty_bands_reproduces_whole_image_path() {
        let c = comp();
        for u in [u_img1(), u_img2(), u_img3(), u_example(), u_lena(), u_magic()] {
            assert_eq!(
                select_programs_per_voice(&u, &c, 4, &[]),
                select_programs(&u, &c, 4),
                "empty bands must be byte-identical to the catalogue path"
            );
        }
    }

    /// Absent pool data (an OLD `mappings.json`) must also fall back byte-identically, even when
    /// band features ARE supplied. The back-compat floor.
    #[test]
    fn absent_pools_fall_back_to_catalogue_path() {
        let mut c = comp();
        c.instrumentation_pools = InstrumentationPools::default();
        assert_eq!(
            select_programs_per_voice(&u_img1(), &c, 4, &bands([10.0, 40.0, 70.0, 90.0])),
            vec![70, 60, 69, 71],
            "no pools ⇒ the shipped dark_reeds row, unchanged"
        );
    }

    /// **FINDING X — v1 and v2 must be separately DETERMINED, not merely separately called.**
    ///
    /// Both inner voices occupy the `HarmonicFill` seat and share a pool, and selection is
    /// deterministic with no RNG — so keyed on role ALONE they would return the same program by
    /// construction and realized grain would be 3, not 4. They are separated here because each is
    /// keyed on its OWN band's brightness RANK, and a rank is a permutation: v1 and v2 can never
    /// receive the same rank. This asserts the property holds even when the two bands are
    /// arbitrarily close, and even when they are EXACTLY EQUAL (ties break by band index).
    #[test]
    fn inner_voices_are_separately_determined() {
        let c = comp();
        let u = u_img1();
        for b in [
            [10.0, 40.0, 70.0, 90.0],
            [50.0, 50.001, 50.002, 50.003],
            [50.0, 50.0, 50.0, 50.0], // exact tie — rank still total via index tiebreak
            [90.0, 70.0, 40.0, 10.0],
        ] {
            let got = select_programs_per_voice(&u, &c, 4, &bands(b));
            assert_ne!(
                got[1], got[2],
                "v1/v2 collapsed to one program on bands {b:?} — realized grain would be 3, not 4"
            );
        }
    }

    /// **The anti-hollow property.** Different band structure over the SAME whole-image
    /// understanding must move the realized vector. If this fails, the four draws are keyed on
    /// whole-image scalars only — §6's pre-registered P-A hollow build.
    #[test]
    fn band_structure_changes_the_realized_vector() {
        let c = comp();
        let u = u_img1();
        let a = select_programs_per_voice(&u, &c, 4, &bands([10.0, 40.0, 70.0, 90.0]));
        let b = select_programs_per_voice(&u, &c, 4, &bands([90.0, 70.0, 40.0, 10.0]));
        assert_ne!(a, b, "band permutation must change the vector (else hollow)");
    }

    /// **C1 (hard) — tessitura eligibility is enforced by pool membership**, so no draw can seat a
    /// program outside its seat's range. The piccolo-bass guard: GM 72 is admissible on the
    /// MELODY seat and must never be reachable on the BASS seat.
    #[test]
    fn c1_tessitura_pool_membership_is_the_guard() {
        let c = comp();
        let p = &c.instrumentation_pools;
        let bass: Vec<u8> = p.pool(Seat::Bass).unwrap().iter().map(|e| e.program).collect();
        let mel: Vec<u8> = p.pool(Seat::Melody).unwrap().iter().map(|e| e.program).collect();
        assert!(!bass.contains(&72), "piccolo must not be reachable in the bass seat");
        assert!(mel.contains(&72), "piccolo is an ordinary melody-seat program");
        // And no draw, over a wide sweep of band structures, escapes its pool.
        for k in 0..64 {
            let b = bands([
                (k % 4) as f32 * 25.0,
                ((k / 4) % 4) as f32 * 25.0,
                ((k / 16) % 4) as f32 * 25.0,
                (k % 3) as f32 * 33.0,
            ]);
            let got = select_programs_per_voice(&u_img1(), &c, 4, &b);
            assert!(bass.contains(&got[0]), "v0 escaped the bass pool: {got:?}");
            assert!(mel.contains(&got[3]), "v3 escaped the melody pool: {got:?}");
        }
    }

    /// **C2 (hard) — "all-decaying is fine, half-decaying is forbidden."** The harmonic bed must
    /// be held by at least one SUSTAINING fill seat unless EVERY voice decays (the coherent
    /// all-mallet/all-plucked ensemble, which must stay reachable). A lone decaying program in a
    /// sustaining bed leaves the floor silent between onsets and reads to the ear as a bug.
    #[test]
    fn c2_harmonic_bed_is_never_half_decaying() {
        let c = comp();
        let p = &c.instrumentation_pools;
        let cls = |seat: Seat, prog: u8| -> Sustain {
            p.pool(seat)
                .unwrap()
                .iter()
                .find(|e| e.program == prog)
                .map(|e| e.sustain)
                .expect("realized program must come from its seat pool")
        };
        for k in 0..96 {
            let b = bands([
                (k % 5) as f32 * 20.0,
                ((k / 5) % 5) as f32 * 20.0,
                ((k / 25) % 5) as f32 * 20.0,
                ((k / 7) % 5) as f32 * 20.0,
            ]);
            let got = select_programs_per_voice(&u_img1(), &c, 4, &b);
            let bed_held = cls(Seat::HarmonicFill, got[1]) == Sustain::Sustaining
                || cls(Seat::HarmonicFill, got[2]) == Sustain::Sustaining;
            let all_dec = cls(Seat::Bass, got[0]) == Sustain::Decaying
                && cls(Seat::HarmonicFill, got[1]) == Sustain::Decaying
                && cls(Seat::HarmonicFill, got[2]) == Sustain::Decaying
                && cls(Seat::Melody, got[3]) == Sustain::Decaying;
            assert!(
                bed_held || all_dec,
                "C2 violated on bands {b:?}: realized {got:?} has a half-decaying bed"
            );
        }
    }

    /// **C3b (soft) — outer-voice polarity.** Bass and melody on one program erases the contrast
    /// that makes a four-part texture legible, so an exact outer collision is repaired.
    #[test]
    fn c3b_outer_voices_are_never_identical() {
        let c = comp();
        for k in 0..64 {
            let b = bands([
                (k % 4) as f32 * 30.0,
                ((k / 4) % 4) as f32 * 30.0,
                ((k / 16) % 4) as f32 * 30.0,
                (k % 5) as f32 * 24.0,
            ]);
            let got = select_programs_per_voice(&u_img1(), &c, 4, &b);
            assert_ne!(got[0], got[3], "outer voices collided on bands {b:?}");
        }
    }

    /// Determinism: the same inputs must produce the same vector, always. Selection consumes no
    /// RNG and the repair pass is a bounded deterministic scan (never rejection sampling, which
    /// is non-deterministic in effort and poison for a sealed-golden discipline).
    #[test]
    fn per_voice_selection_is_deterministic() {
        let c = comp();
        let b = bands([31.0, 62.0, 17.0, 88.0]);
        let first = select_programs_per_voice(&u_lena(), &c, 4, &b);
        for _ in 0..32 {
            assert_eq!(select_programs_per_voice(&u_lena(), &c, 4, &b), first);
        }
    }

    /// N-generality: the seat classifier and the pools are not hardcoded to 4 voices.
    #[test]
    fn per_voice_generalizes_over_voice_counts() {
        let c = comp();
        for n in 1..=6usize {
            let b: Vec<BandFeatures> = (0..n)
                .map(|i| BandFeatures {
                    brightness: 10.0 + i as f32 * 13.0,
                })
                .collect();
            let got = select_programs_per_voice(&u_img1(), &c, n, &b);
            assert_eq!(got.len(), n, "must fill exactly n voices");
        }
        assert!(select_programs_per_voice(&u_img1(), &c, 0, &[]).is_empty());
    }

    /// Every program of every ROUTED `ensemble_catalogue` row must be reachable in its own seat's
    /// pool — this is how the curated ensembles stay reachable under B2-a, rather than via a
    /// whole-image arm (which would make GRAIN read 1 on the pieces it fired for).
    #[test]
    fn shipped_ensembles_remain_reachable_through_pools() {
        let c = comp();
        let p = &c.instrumentation_pools;
        let routed: Vec<String> = c
            .instrumentation
            .rules
            .iter()
            .map(|r| r.pick.clone())
            .chain(std::iter::once(c.instrumentation.default.clone()))
            .collect();
        for e in &c.ensemble_catalogue {
            if !routed.contains(&e.id) {
                continue; // staged-but-unrouted rows are outside the reachable set
            }
            for (i, &prog) in e.programs.iter().enumerate() {
                let pool = p.pool(seat_for(i, e.programs.len())).unwrap();
                assert!(
                    pool.iter().any(|x| x.program == prog),
                    "{} voice {i} program {prog} unreachable in its seat pool",
                    e.id
                );
            }
        }
    }
}

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
}

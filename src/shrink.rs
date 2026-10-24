//! src/shrink.rs — "Shrink picture" / `--shrink` (S125 ITEM 3; operator D-51, D-56).
//!
//! The ONE size function the CLI and the window share (UI Designer `s125-UI4-shrink-final-spec.md`
//! § 1, § 4; Shielding `s125-SH5-prerelease-spec.md` Part C). Everything that states or uses a
//! shrunk size reads it from here: the window's level rows, its notice and its scan-bar overlay,
//! the CLI's notice and its refusal, and the resize itself.
//!
//! * **Levels.** A closed set of four ([`Shrink`]): the whole picture (the default), 75 %, 50 % and
//!   25 % of EACH side. No other value is offered anywhere (UI4 § 1 "Closed set, on both
//!   surfaces"; SH5 C-1: closed and integral, never a float).
//! * **Arithmetic.** [`shrunk_side`]: `(side · pct + 50) / 100` in `u64`, round half up, width and
//!   height separately (UI4 § 1; §13 `s125-13` § 2.2 keeps this rule).
//! * **Floor.** [`SHRINK_MIN_SIDE`] (16 px). A level whose smaller result side would fall under it
//!   is refused on the CLI and disabled in the window ([`check`], [`fits`]). Nothing is ever
//!   enlarged: every level is ≤ 100 %, and 100 % makes no resize call at all.
//! * **Resize.** [`apply`] (pure-analysis builds only): `image::imageops::thumbnail` (integer
//!   area-box averaging) on the decoded `RgbImage`, called with EXACTLY [`shrunk_size`]'s two numbers
//!   and checked afterwards, so the size the user is told is the size the music is made from
//!   (§13 MUST-FIX 1, `s125-13` M-1). The filter is §2's proposal with §13's sign-off (lead rulings
//!   L-57, L-64). Changing the filter, or the `image` crate it comes from (0.24.9 today), re-draws
//!   every shrunk piece; the golden test in `tests/shrink_s125.rs` turns such a change red.
//!
//! Order on every front-end (SH5 C-3): read → gate (the ORIGINAL picture's caps) → decode →
//! [`check`] → [`apply`] → analysis. A shrink never admits a picture the gate refused, and the
//! full-size buffer is dropped before the analysis starts.
//!
//! Pure except for [`apply`], which needs the `image` crate: the rest builds and unit-tests under
//! `--no-default-features` (the CLI grammar in `cli.rs` uses [`Shrink`]).

use clap::ValueEnum;

/// The smallest side a shrink may produce, in pixels. Equal to the window's admission floor
/// (`gui::ingest::MIN_SIDE`; a test keeps the two equal) and stated in the `--shrink` help text (a
/// test keeps that equal too). It covers the 16-voice ceiling and keeps every 1–2 px side (the
/// D-45 panic class) out of reach of any shrink.
pub const SHRINK_MIN_SIDE: u32 = 16;

/// How much to shrink the picture before the music is made from it: one of four closed levels.
///
/// clap reads the level as the plain integer `100`, `75`, `50` or `25` (`--shrink 50`). The
/// variants carry plain comments, not doc comments, because clap would print a variant's doc
/// comment as that value's help and change the `--help` layout (lead ruling L-54: the flag's own
/// doc comment is the only `--help` addition).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, ValueEnum)]
pub enum Shrink {
    // The whole picture. The default; no resize call is made.
    #[default]
    #[value(name = "100")]
    Full,
    // 75 % of each side.
    #[value(name = "75")]
    Percent75,
    // 50 % of each side.
    #[value(name = "50")]
    Percent50,
    // 25 % of each side.
    #[value(name = "25")]
    Percent25,
}

impl Shrink {
    /// Every level, largest first: the order the window lists them and clap prints them.
    pub const ALL: [Shrink; 4] = [
        Shrink::Full,
        Shrink::Percent75,
        Shrink::Percent50,
        Shrink::Percent25,
    ];

    /// The level as a percent of each side (100, 75, 50 or 25).
    pub const fn percent(self) -> u32 {
        match self {
            Shrink::Full => 100,
            Shrink::Percent75 => 75,
            Shrink::Percent50 => 50,
            Shrink::Percent25 => 25,
        }
    }

    /// `true` for the whole picture (no shrink, no resize call).
    pub const fn is_full(self) -> bool {
        matches!(self, Shrink::Full)
    }

    /// The level for a percent, if it is one of the four.
    pub fn from_percent(pct: u32) -> Option<Shrink> {
        Shrink::ALL.into_iter().find(|s| s.percent() == pct)
    }
}

/// One side after a shrink to `pct` percent: `(side · pct + 50) / 100`, integer, round half up,
/// computed in `u64` (UI4 § 1; SK3). For `pct ≤ 100` the result is never larger than `side`.
pub const fn shrunk_side(side: u32, pct: u32) -> u32 {
    ((side as u64 * pct as u64 + 50) / 100) as u32
}

/// The picture's size after `shrink`: each side through [`shrunk_side`]. [`Shrink::Full`] returns
/// the size unchanged.
pub const fn shrunk_size(width: u32, height: u32, shrink: Shrink) -> (u32, u32) {
    if shrink.is_full() {
        (width, height)
    } else {
        let pct = shrink.percent();
        (shrunk_side(width, pct), shrunk_side(height, pct))
    }
}

/// Can a `width × height` picture be shrunk to `shrink`? [`Shrink::Full`] always can; any other
/// level only when both result sides are at least [`SHRINK_MIN_SIDE`]. The window disables a level
/// exactly when this is `false`; the CLI refuses it ([`check`]) by the same test.
pub const fn fits(width: u32, height: u32, shrink: Shrink) -> bool {
    if shrink.is_full() {
        return true;
    }
    let (w, h) = shrunk_size(width, height, shrink);
    w >= SHRINK_MIN_SIDE && h >= SHRINK_MIN_SIDE
}

/// [`fits`], as a result: the size the picture will be analysed at, or the refusal.
pub fn check(width: u32, height: u32, shrink: Shrink) -> Result<(u32, u32), ShrinkError> {
    let to = shrunk_size(width, height, shrink);
    if fits(width, height, shrink) {
        Ok(to)
    } else {
        Err(ShrinkError::TooSmall {
            percent: shrink.percent(),
            from: (width, height),
            to,
        })
    }
}

/// Why a shrink could not be applied.
///
/// The `Display` text of [`ShrinkError::TooSmall`] and [`ShrinkError::Unsupported`] is the CLI's
/// error sentence (UI4 § 3.3), printed after `Error: `. The window never reaches either: it
/// disables an infeasible level and has no unsupported build, so it words these itself if it ever
/// did (`gui::text::from_render_error`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShrinkError {
    /// The shrink would take a side under [`SHRINK_MIN_SIDE`].
    TooSmall {
        /// The level asked for, in percent.
        percent: u32,
        /// The picture's size, `(width, height)`.
        from: (u32, u32),
        /// What the shrink would make it, `(width, height)`.
        to: (u32, u32),
    },
    /// This build's analysis cannot shrink (the `opencv` feature path, UI4 § 3.3).
    Unsupported,
    /// The resize returned a size other than the one asked for. Never expected (`imageops::thumbnail`
    /// returns exactly the requested size); checked so that a later crate change cannot make the
    /// printed size and the analysed size disagree in silence (§13 M-1).
    SizeMismatch {
        /// The size asked for.
        wanted: (u32, u32),
        /// The size the resize returned.
        got: (u32, u32),
    },
}

impl std::fmt::Display for ShrinkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShrinkError::TooSmall { percent, from, to } => write!(
                f,
                "--shrink {percent} would make this picture {}x{} (from {}x{}), and each side must \
                 stay at least {SHRINK_MIN_SIDE} px. Use a larger --shrink value, or leave \
                 --shrink off.",
                to.0, to.1, from.0, from.1
            ),
            ShrinkError::Unsupported => f.write_str("--shrink is not supported by this build."),
            ShrinkError::SizeMismatch { wanted, got } => write!(
                f,
                "internal: the shrunk picture came out {}x{} instead of {}x{}, so it was not used.",
                got.0, got.1, wanted.0, wanted.1
            ),
        }
    }
}

impl std::error::Error for ShrinkError {}

/// The CLI's one-line notice (UI4 § 3.2), printed on stderr after the shrink is known to be valid
/// and before the analysis starts. `from` is the picture's size, `to` the size the music is made
/// from. Sizes use an ASCII `x` (terminals and logs).
pub fn cli_notice(shrink: Shrink, from: (u32, u32), to: (u32, u32)) -> String {
    format!(
        "Note: --shrink {} makes the music from a smaller copy of the picture ({}x{}, from {}x{}). \
         Less detail means different music: this piece won't match the full-size one, and its \
         notes, tempo and length can all change.",
        shrink.percent(),
        to.0,
        to.1,
        from.0,
        from.1
    )
}

thread_local! {
    /// How many resizes [`apply`] has run on this thread (SK1's "no resize call at 100 %" hook).
    /// Thread-local, so parallel tests cannot count each other's resizes.
    static RESIZES: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// How many times [`apply`] has resized a picture on the calling thread.
///
/// TEST SEAM (SK1): lets a test show that the default path never calls the resizer. It changes
/// nothing about any output.
#[doc(hidden)]
pub fn resizes_on_this_thread() -> u64 {
    RESIZES.with(|c| c.get())
}

/// Shrink a decoded picture to `shrink` (SH5 C-3/C-4; §13's filter, L-64).
///
/// [`Shrink::Full`] returns `img` itself: no resize call is made, so the default path is the
/// unchanged decoded buffer, byte for byte. Any other level is refused by [`check`] if a side would
/// fall under [`SHRINK_MIN_SIDE`] (so the resizer is never called with a 0 side), then made with
/// `image::imageops::thumbnail` at exactly [`shrunk_size`]'s two numbers. `img` is dropped before
/// this returns, so the full-size buffer is gone before any analysis starts.
#[cfg(feature = "pure-analysis")]
pub fn apply(
    img: crate::pure_analysis::PureImage,
    shrink: Shrink,
) -> Result<crate::pure_analysis::PureImage, ShrinkError> {
    if shrink.is_full() {
        return Ok(img);
    }
    let (w, h) = check(img.width(), img.height(), shrink)?;
    // Integer area-box averaging: deterministic (no platform maths library), and the cheapest in
    // time and memory of the filters §13 measured (`s125-13` § 2.1). An exact-size call: NOT
    // `DynamicImage::thumbnail` / `resize`, which fit inside a box with their own rounding (M-1).
    let small = image::imageops::thumbnail(img.as_rgb(), w, h);
    drop(img);
    RESIZES.with(|c| c.set(c.get() + 1));
    if small.width() != w || small.height() != h {
        return Err(ShrinkError::SizeMismatch {
            wanted: (w, h),
            got: (small.width(), small.height()),
        });
    }
    Ok(crate::pure_analysis::PureImage::from_rgb(small))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sk3_the_arithmetic_table() {
        // UI4 SK3's four cases, plus §13's checks of UI4's examples.
        assert_eq!(shrunk_size(6000, 4000, Shrink::Percent50), (3000, 2000));
        assert_eq!(shrunk_side(21, 75), 16);
        assert_eq!(shrunk_side(20, 75), 15);
        assert_eq!(shrunk_size(101, 100, Shrink::Percent25), (25, 25));
        assert_eq!(shrunk_size(40, 30, Shrink::Percent75), (30, 23));
        assert_eq!(shrunk_size(40, 30, Shrink::Percent50), (20, 15));
        assert_eq!(shrunk_size(40, 30, Shrink::Percent25), (10, 8));
        assert_eq!(shrunk_size(6000, 4000, Shrink::Percent75), (4500, 3000));
        assert_eq!(shrunk_size(6000, 4000, Shrink::Percent25), (1500, 1000));
        // Round half UP: 2·25 = 50 → (50 + 50) / 100 = 1; 1·50 = 50 → 1.
        assert_eq!(shrunk_side(2, 25), 1);
        assert_eq!(shrunk_side(1, 50), 1);
        assert_eq!(shrunk_side(3, 50), 2);
        // §13 M-1's example: the exact-size rule gives 53×59 at 75 % (an aspect-fit gives 53×58).
        assert_eq!(shrunk_size(71, 78, Shrink::Percent75), (53, 59));
        // u64 arithmetic: no overflow at the largest u32 side.
        assert_eq!(shrunk_side(u32::MAX, 75), 3_221_225_471);
        assert_eq!(shrunk_side(u32::MAX, 25), 1_073_741_824);
    }

    #[test]
    fn full_is_the_identity_and_never_enlarges() {
        for (w, h) in [(1, 1), (16, 16), (40, 30), (6000, 4000), (u32::MAX, 1)] {
            assert_eq!(shrunk_size(w, h, Shrink::Full), (w, h));
            assert!(fits(w, h, Shrink::Full), "Full always fits {w}x{h}");
            for s in Shrink::ALL {
                let (a, b) = shrunk_size(w, h, s);
                assert!(a <= w && b <= h, "{s:?} enlarged {w}x{h} to {a}x{b}");
            }
        }
    }

    #[test]
    fn sk5_the_floor_disables_exactly_the_levels_under_16() {
        // 40×30: 75 % (30×23) fits; 50 % (20×15) and 25 % (10×8) do not.
        assert!(fits(40, 30, Shrink::Percent75));
        assert!(!fits(40, 30, Shrink::Percent50));
        assert!(!fits(40, 30, Shrink::Percent25));
        // 20×20: every shrink level fails (75 % → 15).
        for s in [Shrink::Percent75, Shrink::Percent50, Shrink::Percent25] {
            assert!(!fits(20, 20, s), "{s:?} on 20x20");
        }
        // 21×21: 75 % → 16 is allowed (≥ 16, not > 16).
        assert!(fits(21, 21, Shrink::Percent75));
        // SH5 S2/S3: extreme aspects, every level infeasible, never rounded to a 0/1-px side.
        for (w, h) in [(32768, 16), (16, 32768), (40, 16)] {
            for s in [Shrink::Percent75, Shrink::Percent50, Shrink::Percent25] {
                assert!(!fits(w, h, s), "{s:?} on {w}x{h}");
            }
        }
        // SH5 S1: a 16×16 picture: Full fits (nothing happens), every shrink is refused.
        assert!(fits(16, 16, Shrink::Full));
        assert!(!fits(16, 16, Shrink::Percent75));
    }

    #[test]
    fn no_level_that_fits_has_a_side_under_the_floor() {
        // Exhaustive over small pictures: the 1–2 px panic class is unreachable through a shrink.
        for w in 1..=80u32 {
            for h in 1..=80u32 {
                for s in [Shrink::Percent75, Shrink::Percent50, Shrink::Percent25] {
                    match check(w, h, s) {
                        Ok((a, b)) => assert!(a >= SHRINK_MIN_SIDE && b >= SHRINK_MIN_SIDE),
                        Err(ShrinkError::TooSmall { to, .. }) => {
                            assert!(to.0 < SHRINK_MIN_SIDE || to.1 < SHRINK_MIN_SIDE)
                        }
                        Err(e) => panic!("unexpected {e:?}"),
                    }
                }
            }
        }
    }

    #[test]
    fn sk4_the_cli_strings_are_verbatim() {
        // UI4 § 3.3, character for character.
        assert_eq!(
            check(40, 30, Shrink::Percent25).unwrap_err().to_string(),
            "--shrink 25 would make this picture 10x8 (from 40x30), and each side must stay at \
             least 16 px. Use a larger --shrink value, or leave --shrink off."
        );
        assert_eq!(
            ShrinkError::Unsupported.to_string(),
            "--shrink is not supported by this build."
        );
        // UI4 § 3.2.
        assert_eq!(
            cli_notice(Shrink::Percent50, (6000, 4000), (3000, 2000)),
            "Note: --shrink 50 makes the music from a smaller copy of the picture (3000x2000, \
             from 6000x4000). Less detail means different music: this piece won't match the \
             full-size one, and its notes, tempo and length can all change."
        );
        for s in [
            cli_notice(Shrink::Percent75, (1, 1), (1, 1)),
            check(40, 30, Shrink::Percent50).unwrap_err().to_string(),
        ] {
            for banned in ["faster", "speed", "small change", "seconds"] {
                assert!(!s.contains(banned), "{s:?} contains {banned:?}");
            }
        }
    }

    #[test]
    fn the_four_levels_round_trip() {
        assert_eq!(Shrink::default(), Shrink::Full);
        let pcts: Vec<u32> = Shrink::ALL.iter().map(|s| s.percent()).collect();
        assert_eq!(pcts, vec![100, 75, 50, 25]);
        for s in Shrink::ALL {
            assert_eq!(Shrink::from_percent(s.percent()), Some(s));
            // clap's value name is the percent, as an integer string.
            let pv = s.to_possible_value().expect("every level is offered");
            assert_eq!(pv.get_name(), s.percent().to_string());
            assert!(pv.get_help().is_none(), "no per-value help (L-54)");
        }
        assert_eq!(Shrink::from_percent(60), None);
        assert_eq!(Shrink::from_percent(0), None);
    }

    #[cfg(feature = "pure-analysis")]
    #[test]
    fn apply_makes_exactly_the_printed_size_and_full_makes_no_call() {
        use crate::pure_analysis::PureImage;
        let pic = |w: u32, h: u32| {
            PureImage::from_rgb(image::RgbImage::from_fn(w, h, |x, y| {
                image::Rgb([(x * 7) as u8, (y * 5) as u8, ((x ^ y) * 3) as u8])
            }))
        };
        let before = resizes_on_this_thread();
        let same = apply(pic(71, 78), Shrink::Full).expect("full");
        assert_eq!((same.width(), same.height()), (71, 78));
        assert_eq!(resizes_on_this_thread(), before, "Full made a resize call");
        assert_eq!(
            same.as_rgb().as_raw(),
            pic(71, 78).as_rgb().as_raw(),
            "Full returned other pixels"
        );
        // M-1: the analysed size IS `shrunk_size`, including §13's disagreeing example 71×78.
        for (w, h) in [(71, 78), (101, 100), (6000 / 50, 4000 / 50), (333, 17 * 4)] {
            for s in [Shrink::Percent75, Shrink::Percent50, Shrink::Percent25] {
                if !fits(w, h, s) {
                    continue;
                }
                let n = resizes_on_this_thread();
                let out = apply(pic(w, h), s).expect("fits");
                assert_eq!(
                    (out.width(), out.height()),
                    shrunk_size(w, h, s),
                    "{w}x{h} {s:?}"
                );
                assert_eq!(resizes_on_this_thread(), n + 1);
            }
        }
        // A refused level never reaches the resizer.
        let n = resizes_on_this_thread();
        assert!(matches!(
            apply(pic(40, 30), Shrink::Percent50),
            Err(ShrinkError::TooSmall { .. })
        ));
        assert_eq!(resizes_on_this_thread(), n);
    }
}

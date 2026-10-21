//! Scan-bar geometry and the playhead (UI `s123-UI` § 4).
//!
//! The geometry is the ANALYZER'S, mirrored line for line from
//! `pure_analysis::PureAnalysisSource::scan_steps` (the bar rectangle and its per-voice split),
//! so the bar drawn for step `s` covers exactly the pixels the music for step `s` read (UI A4).
//! It is pure arithmetic on picture pixels; [`super::app`] only scales it to the screen.
//!
//! Two engine facts the overlay must honour, both measured from the code rather than taken from
//! the wireframe's sketch (see the build report's headlines):
//!
//! * The scan is a VERTICAL bar moving left→right only when `width > height` (strictly). A square
//!   picture is scanned by a HORIZONTAL bar moving top→bottom.
//! * Step `s` of `S` places the bar's top-left at `round(s * (side - bar) / (S - 1))`, so the
//!   first bar sits flush at the leading edge and the last flush at the trailing edge. It is not
//!   centred at `(s + 0.5) / S`.
//!
//! And the three playhead cases (UI H3): a composed step `< S` reads its scan rectangle; a step
//! `≥ S` reads neutral zero-bars (the coda: the bar leaves the picture); and when the piece has
//! fewer steps than the scan, the tail of the picture is never swept.

/// An axis-aligned rectangle in PICTURE pixels (origin top-left).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PxRect {
    /// Left edge.
    pub x: u32,
    /// Top edge.
    pub y: u32,
    /// Width.
    pub w: u32,
    /// Height.
    pub h: u32,
}

/// Everything the scan geometry depends on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScanGeometry {
    /// Picture width in pixels (the ORIGINAL picture, not the on-screen copy).
    pub width: u32,
    /// Picture height in pixels.
    pub height: u32,
    /// `--thickness`, as the engine receives it (`PipelineArgs::thickness`).
    pub thickness: f32,
    /// `--steps` as requested (`0` behaves as `1`, as in the analyzer).
    pub steps: usize,
    /// Voices (`--instruments`).
    pub voices: usize,
}

impl ScanGeometry {
    /// `S`: the number of scan positions the analyzer produces (`steps`, with `0` read as `1`).
    /// This equals `PlayPlan::source_step_count` for every picture the window admits.
    pub fn scan_steps(&self) -> usize {
        self.steps.max(1)
    }

    /// The analyzer's orientation rule: a vertical bar sweeping left→right iff `width > height`.
    pub fn vertical(&self) -> bool {
        self.width > self.height
    }

    /// The bar's size along the sweep (`bar_w` for a vertical bar, `bar_h` for a horizontal one),
    /// exactly as the analyzer computes it.
    fn bar_extent(&self) -> i64 {
        let side = if self.vertical() {
            self.width
        } else {
            self.height
        } as i64;
        (((side as f32) * self.thickness).max(1.0).round() as i64).min(side)
    }

    /// The scan rectangle for scan position `s` (`s < S`), exactly as
    /// `PureAnalysisSource::scan_steps` computes `bar_rect`.
    pub fn scan_rect(&self, s: usize) -> PxRect {
        let width = self.width as i64;
        let height = self.height as i64;
        let vertical = self.vertical();
        let steps_count = self.scan_steps();
        let bar = self.bar_extent();
        let (bar_w, bar_h) = if vertical {
            (bar, height)
        } else {
            (width, bar)
        };
        let travel_x = (width - bar_w).max(0);
        let travel_y = (height - bar_h).max(0);
        let pos = |travel: i64| -> i64 {
            if steps_count == 1 {
                0
            } else {
                ((s as f32) * (travel as f32) / ((steps_count - 1) as f32)).round() as i64
            }
        };
        let x0 = if vertical { pos(travel_x) } else { 0 };
        let y0 = if vertical { 0 } else { pos(travel_y) };
        PxRect {
            x: x0 as u32,
            y: y0 as u32,
            w: if vertical { bar_w } else { width } as u32,
            h: if vertical { height } else { bar_h } as u32,
        }
    }

    /// The per-voice stripes of a scan rectangle, exactly as the analyzer splits it (the last
    /// stripe absorbs the remainder). Clipped to the picture, as the analyzer clips its crops.
    pub fn voice_stripes(&self, bar: PxRect) -> Vec<PxRect> {
        let nb = self.voices.max(1) as i64;
        let vertical = self.vertical();
        (0..self.voices.max(1))
            .map(|i| {
                let ii = i as i64;
                let last = i + 1 == self.voices.max(1);
                let r = if vertical {
                    let per_h = ((bar.h as i64) / nb).max(1);
                    let y_i = bar.y as i64 + ii * per_h;
                    let h = if last {
                        (bar.y as i64 + bar.h as i64) - y_i
                    } else {
                        per_h
                    }
                    .max(1);
                    PxRect {
                        x: bar.x,
                        y: y_i as u32,
                        w: bar.w,
                        h: h as u32,
                    }
                } else {
                    let per_w = ((bar.w as i64) / nb).max(1);
                    let x_i = bar.x as i64 + ii * per_w;
                    let w = if last {
                        (bar.x as i64 + bar.w as i64) - x_i
                    } else {
                        per_w
                    }
                    .max(1);
                    PxRect {
                        x: x_i as u32,
                        y: bar.y,
                        w: w as u32,
                        h: bar.h,
                    }
                };
                clip(r, self.width, self.height)
            })
            .collect()
    }

    /// Where the playhead is for COMPOSED step `step` (what `PlayEvent::Step` reports).
    pub fn playhead(&self, step: usize) -> Playhead {
        if step < self.scan_steps() {
            Playhead::Bar(self.scan_rect(step))
        } else {
            Playhead::Coda
        }
    }

    /// Case 3: when the piece has FEWER steps than the scan (`total_steps < S`), the part of the
    /// picture past the last heard bar is never swept. Returns that region, or `None` when every
    /// scan position is heard. `total_steps == 0` marks the whole picture.
    pub fn unreached(&self, total_steps: usize) -> Option<PxRect> {
        let s = self.scan_steps();
        if total_steps >= s {
            return None;
        }
        if total_steps == 0 {
            return Some(PxRect {
                x: 0,
                y: 0,
                w: self.width,
                h: self.height,
            });
        }
        let last = self.scan_rect(total_steps - 1);
        if self.vertical() {
            let from = last.x + last.w;
            (from < self.width).then_some(PxRect {
                x: from,
                y: 0,
                w: self.width - from,
                h: self.height,
            })
        } else {
            let from = last.y + last.h;
            (from < self.height).then_some(PxRect {
                x: 0,
                y: from,
                w: self.width,
                h: self.height - from,
            })
        }
    }
}

/// Clip a rectangle to the picture, as the analyzer clips its crops (`.max(1)` included).
fn clip(r: PxRect, width: u32, height: u32) -> PxRect {
    let w = r.w.min(width.saturating_sub(r.x)).max(1);
    let h = r.h.min(height.saturating_sub(r.y)).max(1);
    PxRect {
        x: r.x,
        y: r.y,
        w,
        h,
    }
}

/// The playhead for one composed step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Playhead {
    /// Case 1: the bar is over the real scan rectangle this step read.
    Bar(PxRect),
    /// Case 2: this step reads zero-bars (past the scan); the bar parks off the trailing edge.
    Coda,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn geom(width: u32, height: u32, steps: usize) -> ScanGeometry {
        ScanGeometry {
            width,
            height,
            thickness: 0.1,
            steps,
            voices: 4,
        }
    }

    #[test]
    fn orientation_is_strictly_wider_than_tall() {
        assert!(geom(641, 640, 40).vertical());
        assert!(!geom(640, 640, 40).vertical(), "square ⇒ horizontal bar");
        assert!(!geom(480, 640, 40).vertical());
    }

    #[test]
    fn bar_is_flush_at_both_ends_not_centred() {
        let g = geom(1000, 500, 40);
        let first = g.scan_rect(0);
        let last = g.scan_rect(39);
        assert_eq!(
            first,
            PxRect {
                x: 0,
                y: 0,
                w: 100,
                h: 500
            }
        );
        assert_eq!(last.x + last.w, 1000, "last bar ends at the trailing edge");
        // (s + 0.5)/S centring would put step 0's centre at 12.5 px; the engine's is at 50 px.
        assert_eq!(first.x + first.w / 2, 50);
    }

    #[test]
    fn portrait_sweeps_down() {
        let g = geom(300, 600, 10);
        let r = g.scan_rect(9);
        assert_eq!((r.x, r.w), (0, 300));
        assert_eq!(r.y + r.h, 600);
        assert_eq!(r.h, 60);
    }

    #[test]
    fn one_step_and_zero_steps_sit_at_the_origin() {
        assert_eq!(geom(800, 400, 1).scan_rect(0).x, 0);
        let g = geom(800, 400, 0);
        assert_eq!(g.scan_steps(), 1);
        assert_eq!(g.scan_rect(0).x, 0);
    }

    #[test]
    fn three_playhead_cases() {
        let g = geom(1000, 500, 40);
        assert!(matches!(g.playhead(0), Playhead::Bar(_)));
        assert!(matches!(g.playhead(39), Playhead::Bar(_)));
        assert_eq!(g.playhead(40), Playhead::Coda, "case 2: past the scan");
        assert_eq!(g.playhead(41), Playhead::Coda);
        // Case 3: a 36-step piece over a 40-position scan leaves the tail unswept.
        let u = g.unreached(36).expect("tail unreached");
        let last_heard = g.scan_rect(35);
        assert_eq!(u.x, last_heard.x + last_heard.w);
        assert_eq!(u.x + u.w, 1000);
        // Piece at least as long as the scan: nothing unreached.
        assert_eq!(g.unreached(40), None);
        assert_eq!(g.unreached(42), None);
        // Zero-step piece: the whole picture.
        assert_eq!(
            g.unreached(0),
            Some(PxRect {
                x: 0,
                y: 0,
                w: 1000,
                h: 500
            })
        );
    }

    #[test]
    fn stripes_split_like_the_analyzer() {
        let g = ScanGeometry {
            width: 1000,
            height: 501,
            thickness: 0.1,
            steps: 40,
            voices: 4,
        };
        let bar = g.scan_rect(3);
        let s = g.voice_stripes(bar);
        assert_eq!(s.len(), 4);
        assert_eq!(s[0].h, 125);
        assert_eq!(s[3].h, 501 - 3 * 125, "last stripe absorbs the remainder");
        assert_eq!(s[3].y + s[3].h, 501);
        for st in &s {
            assert_eq!((st.x, st.w), (bar.x, bar.w));
        }
    }

    #[test]
    fn thickness_extremes_clamp_like_the_analyzer() {
        let mut g = geom(1000, 500, 40);
        g.thickness = 5.0;
        assert_eq!(g.scan_rect(10).w, 1000);
        g.thickness = 0.0;
        assert_eq!(g.scan_rect(10).w, 1, "max(1.0)");
    }

    /// A4: the overlay's rectangles equal the analyzer's own. The analyzer does not expose its
    /// rectangles, so this re-derives them from an INDEPENDENT transcription of the analyzer's
    /// arithmetic (`pure_analysis.rs` `scan_steps`, the `bar_rect` block) and compares every step
    /// on a landscape, a portrait and a square fixture, several thicknesses and step counts.
    #[test]
    fn a4_matches_an_independent_transcription_of_the_analyzer() {
        fn engine_rect(w: u32, h: u32, t: f32, n: usize, s: usize) -> (i64, i64, i64, i64) {
            let width = w as i64;
            let height = h as i64;
            let vertical = width > height;
            let bar_w = if vertical {
                (((width as f32) * t).max(1.0).round() as i64).min(width)
            } else {
                width
            };
            let bar_h = if vertical {
                height
            } else {
                (((height as f32) * t).max(1.0).round() as i64).min(height)
            };
            let steps_count = if n == 0 { 1 } else { n };
            let travel_x = (width - bar_w).max(0);
            let travel_y = (height - bar_h).max(0);
            let x0 = if vertical {
                if steps_count == 1 {
                    0
                } else {
                    ((s as f32) * (travel_x as f32) / ((steps_count - 1) as f32)).round() as i64
                }
            } else {
                0
            };
            let y0 = if !vertical {
                if steps_count == 1 {
                    0
                } else {
                    ((s as f32) * (travel_y as f32) / ((steps_count - 1) as f32)).round() as i64
                }
            } else {
                0
            };
            (
                x0,
                y0,
                if vertical { bar_w } else { width },
                if vertical { height } else { bar_h },
            )
        }
        for (w, h) in [
            (640u32, 427u32),
            (427, 640),
            (500, 500),
            (16, 5000),
            (5000, 16),
        ] {
            for t in [0.01f32, 0.1, 0.33, 1.0] {
                for n in [1usize, 2, 40, 200] {
                    let g = ScanGeometry {
                        width: w,
                        height: h,
                        thickness: t,
                        steps: n,
                        voices: 4,
                    };
                    for s in 0..n {
                        let r = g.scan_rect(s);
                        let e = engine_rect(w, h, t, n, s);
                        assert_eq!(
                            (r.x as i64, r.y as i64, r.w as i64, r.h as i64),
                            e,
                            "{w}x{h} t={t} n={n} s={s}"
                        );
                    }
                }
            }
        }
    }
}
